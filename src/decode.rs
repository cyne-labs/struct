//! Decoding implementation for Struct (v0.3).
//!
//! Converts Struct text into structured data in standard Rust data structures.

use std::collections::BTreeMap;
use crate::schema::{
    parse_outside_schemas, parse_schema_line, ClassDef, FieldDef, FieldType, ListItemType,
    ListTarget, ParsedSchemaLine, RecordDef, ScalarType,
};
use crate::types::{Block, DecodedFile, StructError, Value, Warning};

/// Decodes a Struct document string with no outside schemas.
///
/// Stops on the first error encountered and returns line and column details.
pub fn decode(input: &str) -> Result<DecodedFile, StructError> {
    decode_with("", input)
}

/// Decodes a Struct document string with schemas supplied from outside the file.
///
/// `schemas` contains one or more `schema:name{...}` definition lines.
/// Errors inside `schemas` report line and column within `schemas` and mention outside schemas.
pub fn decode_with(schemas: &str, input: &str) -> Result<DecodedFile, StructError> {
    let outside_classes = parse_outside_schemas(schemas)?;
    let num_outside = outside_classes.len();

    let mut decoded = DecodedFile::new();
    let mut defined_classes: BTreeMap<String, ClassDef> = BTreeMap::new();
    for (k, v) in &outside_classes {
        defined_classes.insert(k.clone(), v.clone());
        decoded.schemas.insert(k.clone(), v.clone());
    }

    let mut current_block: Option<Block> = None;
    let mut current_class_def: Option<ClassDef> = None;

    if num_outside == 1 {
        let single_class = outside_classes.values().next().unwrap().clone();
        current_class_def = Some(single_class);
    }

    let mut stack: Vec<StackEntry> = Vec::new();

    let mut byte_pos = 0;
    let mut line_num = 1;
    let mut line_start_byte = 0;

    let len = input.len();

    while byte_pos < len {
        // 1. Skip Windows/Unix newlines for blank lines
        if input[byte_pos..].starts_with("\r\n") {
            byte_pos += 2;
            line_num += 1;
            line_start_byte = byte_pos;
            continue;
        }
        if input[byte_pos..].starts_with('\n') {
            byte_pos += 1;
            line_num += 1;
            line_start_byte = byte_pos;
            continue;
        }

        // 2. Check if remainder of physical line has only spaces and tabs
        let next_nl = match input[byte_pos..].find('\n') {
            Some(i) => byte_pos + i,
            None => len,
        };
        let phys_line = &input[byte_pos..next_nl];
        let phys_line_clean = phys_line.strip_suffix('\r').unwrap_or(phys_line);
        if phys_line_clean.chars().all(|c| c == ' ' || c == '\t') {
            byte_pos = next_nl;
            if byte_pos < len && input[byte_pos..].starts_with('\n') {
                byte_pos += 1;
            }
            line_num += 1;
            line_start_byte = byte_pos;
            continue;
        }

        // 3. Trim leading spaces and tabs on this physical line
        while byte_pos < len {
            let c = input[byte_pos..].chars().next().unwrap();
            if c == ' ' || c == '\t' {
                byte_pos += 1;
            } else {
                break;
            }
        }

        // 4. Check if schema line: starts with "schema:" AND contains NO '|' anywhere on this physical line
        let remaining_phys_line = &input[byte_pos..next_nl];
        let remaining_clean = remaining_phys_line.strip_suffix('\r').unwrap_or(remaining_phys_line);
        let trimmed_clean = remaining_clean.trim_end_matches(|c| c == ' ' || c == '\t');

        if trimmed_clean.starts_with("schema:") && !trimmed_clean.contains('|') {
            // It is a schema line
            unwind_stack(&mut stack, 0, &mut current_block);
            if let Some(b) = current_block.take() {
                decoded.blocks.push(b);
            }

            match parse_schema_line(trimmed_clean, line_num)? {
                ParsedSchemaLine::Define(class_def) => {
                    let name = class_def.name.clone();
                    if defined_classes.contains_key(&name) {
                        return Err(StructError::new(
                            format!("class '{}' defined twice", name),
                            line_num,
                            8,
                        ));
                    }
                    defined_classes.insert(name.clone(), class_def.clone());
                    decoded.schemas.insert(name.clone(), class_def.clone());
                    current_class_def = Some(class_def);
                    current_block = Some(Block {
                        class_name: name,
                        records: Vec::new(),
                    });
                }
                ParsedSchemaLine::Reuse(name) => {
                    let class_def = match defined_classes.get(&name) {
                        Some(def) => def.clone(),
                        None => {
                            return Err(StructError::new(
                                format!("reusing class '{}' that hasn't been defined yet", name),
                                line_num,
                                8,
                            ));
                        }
                    };
                    current_class_def = Some(class_def);
                    current_block = Some(Block {
                        class_name: name,
                        records: Vec::new(),
                    });
                }
            }

            byte_pos = next_nl;
            if byte_pos < len && input[byte_pos..].starts_with('\n') {
                byte_pos += 1;
            }
            line_num += 1;
            line_start_byte = byte_pos;
            continue;
        }

        // 5. Otherwise, it is a DATA LINE
        let class_def = match current_class_def.as_ref() {
            Some(cd) => cd.clone(),
            None => {
                if num_outside > 1 {
                    return Err(StructError::new(
                        "with several outside classes, data before the first schema line is an error",
                        line_num,
                        1,
                    ));
                } else {
                    return Err(StructError::new(
                        "data line appears before any schema line",
                        line_num,
                        1,
                    ));
                }
            }
        };

        // If current_block is None (e.g. single outside class initial block), create it now
        if current_block.is_none() {
            current_block = Some(Block {
                class_name: class_def.name.clone(),
                records: Vec::new(),
            });
        }

        // Count leading '>' depth markers
        let start_line_num = line_num;
        let mut depth = 0;
        while byte_pos < len && input[byte_pos..].starts_with('>') {
            depth += 1;
            byte_pos += 1;
        }

        // Parse fields on this data line (handling \r raw values that can span lines)
        let mut raw_fields: Vec<RawField> = Vec::new();

        loop {
            // Trim leading spaces and tabs before field value
            while byte_pos < len {
                let c = input[byte_pos..].chars().next().unwrap();
                if c == ' ' || c == '\t' {
                    byte_pos += 1;
                } else {
                    break;
                }
            }

            let field_col = get_col(input, line_start_byte, byte_pos);
            let field_line = line_num;

            if byte_pos >= len {
                return Err(StructError::new(
                    "data line must always end with an unescaped '|'",
                    field_line,
                    field_col,
                ));
            }

            // Check if field is raw: starts with \r
            if input[byte_pos..].starts_with(r"\r") {
                byte_pos += 2; // skip \r
                let mut raw_val = String::new();
                let mut closed = false;

                while byte_pos < len {
                    if input[byte_pos..].starts_with(r"\|") {
                        raw_val.push('|');
                        byte_pos += 2;
                    } else if input[byte_pos..].starts_with(r"\\") {
                        raw_val.push('\\');
                        byte_pos += 2;
                    } else if input[byte_pos..].starts_with('\\') {
                        let esc_col = get_col(input, line_start_byte, byte_pos);
                        let esc_c = input[byte_pos + 1..].chars().next().unwrap_or(' ');
                        return Err(StructError::new(
                            format!("invalid escape sequence '\\{}' inside raw value", esc_c),
                            line_num,
                            esc_col,
                        ));
                    } else if input[byte_pos..].starts_with('|') {
                        byte_pos += 1;
                        closed = true;
                        break;
                    } else if input[byte_pos..].starts_with("\r\n") {
                        raw_val.push('\n');
                        byte_pos += 2;
                        line_num += 1;
                        line_start_byte = byte_pos;
                    } else if input[byte_pos..].starts_with('\n') {
                        raw_val.push('\n');
                        byte_pos += 1;
                        line_num += 1;
                        line_start_byte = byte_pos;
                    } else {
                        let c = input[byte_pos..].chars().next().unwrap();
                        raw_val.push(c);
                        byte_pos += c.len_utf8();
                    }
                }

                if !closed {
                    return Err(StructError::new(
                        "raw value has no closing '|' before the end of the file",
                        field_line,
                        field_col,
                    ));
                }

                if raw_val.is_empty() {
                    return Err(StructError::new(
                        "empty raw value is an error",
                        field_line,
                        field_col,
                    ));
                }

                raw_fields.push(RawField {
                    value: raw_val,
                    is_raw: true,
                    is_null: false,
                    line: field_line,
                    col_start: field_col,
                });
            } else {
                // Non-raw field on the CURRENT physical line
                let start_content = byte_pos;
                let mut in_escape = false;
                let mut closed = false;
                let mut pipe_pos = byte_pos;

                while byte_pos < len {
                    if in_escape {
                        in_escape = false;
                        let c = input[byte_pos..].chars().next().unwrap();
                        byte_pos += c.len_utf8();
                        continue;
                    }
                    if input[byte_pos..].starts_with('\\') {
                        in_escape = true;
                        byte_pos += 1;
                        continue;
                    }
                    if input[byte_pos..].starts_with('|') {
                        pipe_pos = byte_pos;
                        byte_pos += 1;
                        closed = true;
                        break;
                    }
                    if input[byte_pos..].starts_with('\n') || input[byte_pos..].starts_with('\r') {
                        break;
                    }
                    let c = input[byte_pos..].chars().next().unwrap();
                    byte_pos += c.len_utf8();
                }

                if in_escape {
                    return Err(StructError::new(
                        "dangling backslash escape at end of line",
                        line_num,
                        get_col(input, line_start_byte, byte_pos),
                    ));
                }

                if !closed {
                    return Err(StructError::new(
                        "data line must always end with an unescaped '|'",
                        line_num,
                        get_col(input, line_start_byte, byte_pos),
                    ));
                }

                let raw_slice = &input[start_content..pipe_pos];
                let trimmed = raw_slice.trim_end_matches(|c| c == ' ' || c == '\t');
                if trimmed.is_empty() {
                    return Err(StructError::new(
                        "empty field is an error; blank text must be written as '~'",
                        line_num,
                        field_col,
                    ));
                }

                if trimmed == "~" {
                    raw_fields.push(RawField {
                        value: String::new(),
                        is_raw: false,
                        is_null: true,
                        line: line_num,
                        col_start: field_col,
                    });
                } else {
                    let mut unescaped = String::with_capacity(trimmed.len());
                    let mut chars = trimmed.char_indices().peekable();
                    while let Some((ci, c)) = chars.next() {
                        if c == '\\' {
                            match chars.next() {
                                Some((_, '|')) => unescaped.push('|'),
                                Some((_, '\\')) => unescaped.push('\\'),
                                Some((_, '~')) => unescaped.push('~'),
                                Some((_, '>')) => unescaped.push('>'),
                                Some((_, '#')) => unescaped.push('#'),
                                Some((_, 'r')) => {
                                    return Err(StructError::new(
                                        "invalid escape sequence '\\r'",
                                        line_num,
                                        field_col + ci,
                                    ));
                                }
                                Some((_, 'n')) => {
                                    return Err(StructError::new(
                                        "invalid escape sequence '\\n'",
                                        line_num,
                                        field_col + ci,
                                    ));
                                }
                                Some((_, 's')) => {
                                    return Err(StructError::new(
                                        "invalid escape sequence '\\s'",
                                        line_num,
                                        field_col + ci,
                                    ));
                                }
                                Some((_, invalid_esc)) => {
                                    return Err(StructError::new(
                                        format!("invalid escape sequence '\\{}'", invalid_esc),
                                        line_num,
                                        field_col + ci,
                                    ));
                                }
                                None => {
                                    return Err(StructError::new(
                                        "dangling backslash escape",
                                        line_num,
                                        field_col + ci,
                                    ));
                                }
                            }
                        } else {
                            unescaped.push(c);
                        }
                    }

                    raw_fields.push(RawField {
                        value: unescaped,
                        is_raw: false,
                        is_null: false,
                        line: line_num,
                        col_start: field_col,
                    });
                }
            }

            // Check if this was the last field on the physical line
            let mut check_pos = byte_pos;
            while check_pos < len {
                let c = input[check_pos..].chars().next().unwrap();
                if c == ' ' || c == '\t' {
                    check_pos += 1;
                } else {
                    break;
                }
            }
            if check_pos >= len || input[check_pos..].starts_with('\n') || input[check_pos..].starts_with("\r\n") {
                byte_pos = check_pos;
                if byte_pos < len && input[byte_pos..].starts_with("\r\n") {
                    byte_pos += 2;
                    line_num += 1;
                    line_start_byte = byte_pos;
                } else if byte_pos < len && input[byte_pos..].starts_with('\n') {
                    byte_pos += 1;
                    line_num += 1;
                    line_start_byte = byte_pos;
                }
                break;
            }
        }

        // Process data line according to depth
        if depth == 0 {
            unwind_stack(&mut stack, 0, &mut current_block);

            let expected_cols = class_def.num_columns();
            let actual_cols = raw_fields.len();
            if actual_cols < expected_cols {
                return Err(StructError::new(
                    format!("too few columns: expected {}, found {}", expected_cols, actual_cols),
                    start_line_num,
                    1,
                ));
            } else if actual_cols > expected_cols {
                return Err(StructError::new(
                    format!("too many columns: expected {}, found {}", expected_cols, actual_cols),
                    start_line_num,
                    1,
                ));
            }

            let mut col_cursor = 0;
            let record = parse_record_fields(&class_def.fields, &raw_fields, &mut col_cursor, start_line_num)?;

            stack.push(StackEntry {
                value: record,
                schema: StackItemSchema::Class(class_def),
                target_list_path: Vec::new(),
            });
        } else {
            // Child line at depth > 0
            if stack.is_empty() {
                return Err(StructError::new(
                    format!("depth increased by more than one level (from depth 0 to {})", depth),
                    start_line_num,
                    1,
                ));
            }

            if depth > stack.len() {
                return Err(StructError::new(
                    format!(
                        "depth increased by more than one level (from depth {} to {})",
                        stack.len() - 1,
                        depth
                    ),
                    start_line_num,
                    1,
                ));
            }

            unwind_stack(&mut stack, depth, &mut current_block);

            let parent_item = &stack[depth - 1];
            let list_targets = parent_item.schema.find_lists();

            if list_targets.is_empty() {
                return Err(StructError::new(
                    "parent record has no list fields to attach child line",
                    start_line_num,
                    1,
                ));
            }

            let (target_list, data_fields_start) = if list_targets.len() == 1 {
                let single_target = &list_targets[0];
                let item_cols = single_target.item_type.num_columns();

                let mut used_index = false;
                if raw_fields.len() == item_cols + 1 && !raw_fields[0].is_raw {
                    let candidate_index = &raw_fields[0].value;
                    if single_target.accepted_indices.iter().any(|idx| idx == candidate_index) {
                        used_index = true;
                        decoded.warnings.push(Warning {
                            message: format!(
                                "unneeded list index '{}' for parent with only one list",
                                candidate_index
                            ),
                            line: start_line_num,
                            column: depth + 1,
                        });
                    }
                }

                if used_index {
                    (single_target.clone(), 1)
                } else {
                    let actual_cols = raw_fields.len();
                    if actual_cols < item_cols {
                        return Err(StructError::new(
                            format!(
                                "too few columns for child item: expected {}, found {}",
                                item_cols, actual_cols
                            ),
                            start_line_num,
                            depth + 1,
                        ));
                    } else if actual_cols > item_cols {
                        return Err(StructError::new(
                            format!(
                                "too many columns for child item: expected {}, found {}",
                                item_cols, actual_cols
                            ),
                            start_line_num,
                            depth + 1,
                        ));
                    }
                    (single_target.clone(), 0)
                }
            } else {
                // 2 or more lists -> index is REQUIRED
                if raw_fields.is_empty() {
                    return Err(StructError::new("empty child line", start_line_num, depth + 1));
                }
                let index_field = &raw_fields[0];
                let index_str = &index_field.value;
                let matched_target = if index_field.is_raw {
                    None
                } else {
                    list_targets.iter().find(|t| {
                        t.canonical_index == *index_str || t.accepted_indices.iter().any(|a| a == index_str)
                    })
                };

                let target = match matched_target {
                    Some(t) => t.clone(),
                    None => {
                        return Err(validate_unmatched_index(
                            index_str,
                            parent_item.schema.fields(),
                            start_line_num,
                            depth + 1,
                        ));
                    }
                };

                let expected_cols = target.item_type.num_columns();
                let actual_cols = raw_fields.len() - 1;
                if actual_cols < expected_cols {
                    return Err(StructError::new(
                        format!(
                            "too few columns for child item: expected {}, found {}",
                            expected_cols, actual_cols
                        ),
                        start_line_num,
                        depth + 1,
                    ));
                } else if actual_cols > expected_cols {
                    return Err(StructError::new(
                        format!(
                            "too many columns for child item: expected {}, found {}",
                            expected_cols, actual_cols
                        ),
                        start_line_num,
                        depth + 1,
                    ));
                }

                (target, 1)
            };

            let item_fields = &raw_fields[data_fields_start..];
            let (parsed_item, item_schema) = match &target_list.item_type {
                ListItemType::Scalar(scalar_type) => {
                    let val = parse_scalar_value(&item_fields[0], *scalar_type, start_line_num)?;
                    (val, StackItemSchema::Scalar)
                }
                ListItemType::Enum(labels) => {
                    let val = parse_enum_value(&item_fields[0], labels, start_line_num)?;
                    (val, StackItemSchema::Enum)
                }
                ListItemType::Record(rec_def) => {
                    let mut col_cursor = 0;
                    let val = parse_record_fields(&rec_def.fields, item_fields, &mut col_cursor, start_line_num)?;
                    (val, StackItemSchema::Record(rec_def.clone()))
                }
            };

            stack.push(StackEntry {
                value: parsed_item,
                schema: item_schema,
                target_list_path: target_list.field_path,
            });
        }
    }

    unwind_stack(&mut stack, 0, &mut current_block);
    if let Some(b) = current_block {
        decoded.blocks.push(b);
    }

    Ok(decoded)
}

fn get_col(input: &str, line_start_byte: usize, byte_pos: usize) -> usize {
    if byte_pos <= line_start_byte {
        1
    } else {
        input[line_start_byte..byte_pos].chars().count() + 1
    }
}

#[derive(Debug, Clone)]
struct RawField {
    value: String,
    is_raw: bool,
    is_null: bool,
    #[allow(dead_code)]
    line: usize,
    col_start: usize,
}

struct StackEntry {
    value: Value,
    schema: StackItemSchema,
    target_list_path: Vec<usize>,
}

enum StackItemSchema {
    Class(ClassDef),
    Record(RecordDef),
    Scalar,
    Enum,
}

impl StackItemSchema {
    fn find_lists(&self) -> Vec<ListTarget> {
        match self {
            StackItemSchema::Class(c) => c.find_lists(),
            StackItemSchema::Record(r) => r.find_lists(),
            StackItemSchema::Scalar | StackItemSchema::Enum => Vec::new(),
        }
    }

    fn fields(&self) -> &[FieldDef] {
        match self {
            StackItemSchema::Class(c) => &c.fields,
            StackItemSchema::Record(r) => &r.fields,
            StackItemSchema::Scalar | StackItemSchema::Enum => &[],
        }
    }
}

fn validate_unmatched_index(
    index_str: &str,
    parent_fields: &[FieldDef],
    line_num: usize,
    col: usize,
) -> StructError {
    if index_str.is_empty()
        || index_str.starts_with('.')
        || index_str.ends_with('.')
        || index_str.contains("..")
        || !index_str.chars().all(|c| c.is_ascii_digit() || c == '.')
    {
        return StructError::new(
            "missing list index when parent has multiple lists",
            line_num,
            col,
        );
    }

    let parts: Vec<&str> = index_str.split('.').collect();
    let mut current_fields = parent_fields;

    for (level, part) in parts.iter().enumerate() {
        let idx = match part.parse::<usize>() {
            Ok(n) => n,
            Err(_) => {
                return StructError::new(
                    "missing list index when parent has multiple lists",
                    line_num,
                    col,
                );
            }
        };

        if idx >= current_fields.len() {
            return StructError::new(
                format!("list index '{}' is out of range", index_str),
                line_num,
                col,
            );
        }

        let field = &current_fields[idx];
        if level == parts.len() - 1 {
            if !field.has_list() {
                return StructError::new(
                    format!("index '{}' names a non-list field", index_str),
                    line_num,
                    col,
                );
            }
        } else {
            match &field.field_type {
                FieldType::Record(r) => {
                    if !r.has_list() {
                        return StructError::new(
                            format!("index '{}' names a non-list field", index_str),
                            line_num,
                            col,
                        );
                    }
                    current_fields = &r.fields;
                }
                _ => {
                    return StructError::new(
                        format!("index '{}' names a non-list field", index_str),
                        line_num,
                        col,
                    );
                }
            }
        }
    }

    StructError::new(
        format!("unknown list index '{}' for parent record", index_str),
        line_num,
        col,
    )
}

impl ListItemType {
    fn num_columns(&self) -> usize {
        match self {
            ListItemType::Scalar(_) | ListItemType::Enum(_) => 1,
            ListItemType::Record(r) => r.num_columns(),
        }
    }
}

fn unwind_stack(
    stack: &mut Vec<StackEntry>,
    target_len: usize,
    block_opt: &mut Option<Block>,
) {
    while stack.len() > target_len {
        let entry = stack.pop().unwrap();
        if stack.is_empty() {
            if let Some(b) = block_opt.as_mut() {
                b.records.push(entry.value);
            }
        } else {
            let parent_idx = stack.len() - 1;
            let parent_entry = &mut stack[parent_idx];
            add_item_to_parent_list(
                &mut parent_entry.value,
                &parent_entry.schema,
                &entry.target_list_path,
                entry.value,
            );
        }
    }
}

fn add_item_to_parent_list(
    parent: &mut Value,
    schema: &StackItemSchema,
    path: &[usize],
    item: Value,
) {
    if parent.is_null() {
        let field_defs = match schema {
            StackItemSchema::Class(c) => c.fields.clone(),
            StackItemSchema::Record(r) => r.fields.clone(),
            _ => Vec::new(),
        };
        let mut rec_fields = Vec::new();
        for f in field_defs {
            match &f.field_type {
                FieldType::List(_) => rec_fields.push((f.name.clone(), Value::List(Vec::new()))),
                _ => rec_fields.push((f.name.clone(), Value::Null)),
            }
        }
        *parent = Value::Record(rec_fields);
    }

    if let Value::Record(fields) = parent {
        let field_idx = path[0];
        let (_field_name, field_val) = &mut fields[field_idx];
        if path.len() == 1 {
            if let Value::List(items) = field_val {
                items.push(item);
            }
        } else {
            let sub_schema = match schema {
                StackItemSchema::Class(c) => match &c.fields[field_idx].field_type {
                    FieldType::Record(r) => StackItemSchema::Record(r.clone()),
                    _ => StackItemSchema::Record(RecordDef { fields: Vec::new() }),
                },
                StackItemSchema::Record(r) => match &r.fields[field_idx].field_type {
                    FieldType::Record(sub_r) => StackItemSchema::Record(sub_r.clone()),
                    _ => StackItemSchema::Record(RecordDef { fields: Vec::new() }),
                },
                _ => StackItemSchema::Record(RecordDef { fields: Vec::new() }),
            };
            add_item_to_parent_list(field_val, &sub_schema, &path[1..], item);
        }
    }
}

fn parse_record_fields(
    field_defs: &[FieldDef],
    raw_fields: &[RawField],
    cursor: &mut usize,
    line_num: usize,
) -> Result<Value, StructError> {
    let mut record_fields = Vec::new();
    let mut all_columns_null = true;
    let mut has_any_columns = false;
    let contains_list = field_defs.iter().any(|f| f.has_list());

    for field_def in field_defs {
        match &field_def.field_type {
            FieldType::Scalar(st) => {
                has_any_columns = true;
                if *cursor >= raw_fields.len() {
                    return Err(StructError::new("missing column for field", line_num, 1));
                }
                let raw_f = &raw_fields[*cursor];
                *cursor += 1;
                let val = parse_scalar_value(raw_f, *st, line_num)?;
                if !val.is_null() {
                    all_columns_null = false;
                }
                record_fields.push((field_def.name.clone(), val));
            }
            FieldType::Enum(labels) => {
                has_any_columns = true;
                if *cursor >= raw_fields.len() {
                    return Err(StructError::new("missing column for field", line_num, 1));
                }
                let raw_f = &raw_fields[*cursor];
                *cursor += 1;
                let val = parse_enum_value(raw_f, labels, line_num)?;
                if !val.is_null() {
                    all_columns_null = false;
                }
                record_fields.push((field_def.name.clone(), val));
            }
            FieldType::Record(sub_rec) => {
                let sub_val = parse_record_fields(&sub_rec.fields, raw_fields, cursor, line_num)?;
                if sub_rec.num_columns() > 0 {
                    has_any_columns = true;
                    if !sub_val.is_null() {
                        all_columns_null = false;
                    }
                }
                record_fields.push((field_def.name.clone(), sub_val));
            }
            FieldType::List(_) => {
                record_fields.push((field_def.name.clone(), Value::List(Vec::new())));
            }
        }
    }

    if !contains_list && has_any_columns && all_columns_null {
        Ok(Value::Null)
    } else {
        Ok(Value::Record(record_fields))
    }
}

fn parse_scalar_value(
    raw_field: &RawField,
    scalar_type: ScalarType,
    line_num: usize,
) -> Result<Value, StructError> {
    if raw_field.is_null {
        return Ok(Value::Null);
    }

    if raw_field.is_raw && scalar_type != ScalarType::Str {
        return Err(StructError::new(
            format!("raw value '\\r' is not allowed in {} field", scalar_type.as_str()),
            line_num,
            raw_field.col_start,
        ));
    }

    let text = &raw_field.value;

    match scalar_type {
        ScalarType::Str => Ok(Value::String(text.clone())),
        ScalarType::Int => {
            if text.starts_with('+') {
                return Err(StructError::new(
                    format!("leading '+' is not allowed in int '{}'", text),
                    line_num,
                    raw_field.col_start,
                ));
            }
            match text.parse::<i64>() {
                Ok(n) => Ok(Value::Int(n)),
                Err(e) => {
                    if e.kind() == &std::num::IntErrorKind::PosOverflow
                        || e.kind() == &std::num::IntErrorKind::NegOverflow
                    {
                        Err(StructError::new(
                            format!("integer '{}' is out of i64 range", text),
                            line_num,
                            raw_field.col_start,
                        ))
                    } else {
                        Err(StructError::new(
                            format!("type mismatch: '{}' is not a valid int (i64)", text),
                            line_num,
                            raw_field.col_start,
                        ))
                    }
                }
            }
        }
        ScalarType::Float => {
            if text == "NaN" {
                Ok(Value::Float(f64::NAN))
            } else if text == "inf" {
                Ok(Value::Float(f64::INFINITY))
            } else if text == "-inf" {
                Ok(Value::Float(f64::NEG_INFINITY))
            } else if text == "-0" {
                Ok(Value::Float(-0.0))
            } else if text == "0" {
                Ok(Value::Float(0.0))
            } else {
                match text.parse::<f64>() {
                    Ok(f) => Ok(Value::Float(f)),
                    Err(_) => Err(StructError::new(
                        format!("type mismatch: '{}' is not a valid float (f64)", text),
                        line_num,
                        raw_field.col_start,
                    )),
                }
            }
        }
        ScalarType::Bool => {
            if text == "1" {
                Ok(Value::Bool(true))
            } else if text == "0" {
                Ok(Value::Bool(false))
            } else {
                Err(StructError::new(
                    format!("type mismatch: '{}' is not a valid bool (must be 1 or 0)", text),
                    line_num,
                    raw_field.col_start,
                ))
            }
        }
    }
}

fn parse_enum_value(
    raw_field: &RawField,
    labels: &[String],
    line_num: usize,
) -> Result<Value, StructError> {
    if raw_field.is_null {
        return Ok(Value::Null);
    }

    if raw_field.is_raw {
        return Err(StructError::new(
            "raw value '\\r' is not allowed in enum field",
            line_num,
            raw_field.col_start,
        ));
    }

    let text = &raw_field.value;
    if labels.iter().any(|l| l == text) {
        Ok(Value::Enum(text.clone()))
    } else {
        Err(StructError::new(
            format!(
                "type mismatch: value '{}' is not in enum({})",
                text,
                labels.join(",")
            ),
            line_num,
            raw_field.col_start,
        ))
    }
}
