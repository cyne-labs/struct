//! Decoding implementation for Struct (v0.2).
//!
//! Converts Struct text into structured data in standard Rust data structures.

use std::collections::BTreeMap;
use crate::schema::{
    parse_schema_line, ClassDef, FieldDef, FieldType, ListItemType, ListTarget,
    ParsedSchemaLine, RecordDef, ScalarType,
};
use crate::types::{Block, DecodedFile, StructError, Value, Warning};

/// Decodes a Struct document string into a `DecodedFile`.
///
/// Returns the list of blocks in file order, any warnings generated,
/// and the registry of defined class schemas.
///
/// Stops on the first error encountered and returns line and column details.
pub fn decode(input: &str) -> Result<DecodedFile, StructError> {
    let mut decoded = DecodedFile::new();
    let mut defined_classes: BTreeMap<String, ClassDef> = BTreeMap::new();
    let mut current_block: Option<Block> = None;
    let mut current_class_def: Option<ClassDef> = None;

    // Stack for hierarchical child lines:
    let mut stack: Vec<StackEntry> = Vec::new();

    // Iterate through lines, preserving 1-based line numbers
    for (line_idx, raw_line) in input.split('\n').enumerate() {
        let line_num = line_idx + 1;
        // Strip CR if Windows line ending
        let clean_line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed_line = clean_line.trim_matches(|c| c == ' ' || c == '\t');

        // SPEC.md §1: Blank lines, and lines containing only spaces or tabs, are ignored.
        if trimmed_line.is_empty() {
            continue;
        }

        // Check if schema line
        if trimmed_line.starts_with("schema:") {
            // Unwind entire stack into current block before starting new block
            unwind_stack(&mut stack, 0, &mut current_block);

            // If there's an existing block, add it to decoded
            if let Some(b) = current_block.take() {
                decoded.blocks.push(b);
            }

            match parse_schema_line(trimmed_line, line_num)? {
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
            continue;
        }

        // Data line
        let class_def = match current_class_def.as_ref() {
            Some(cd) => cd,
            None => {
                // SPEC.md §1: A data line that appears before any schema line is an error.
                return Err(StructError::new(
                    "data line appears before any schema line",
                    line_num,
                    1,
                ));
            }
        };

        // Count leading '>' child markers
        let mut depth = 0;
        let mut chars_iter = trimmed_line.chars();
        while let Some('>') = chars_iter.clone().next() {
            depth += 1;
            chars_iter.next();
        }

        let content_after_depth = &trimmed_line[depth..];

        // SPEC.md §1: Each non-blank line is a data line: it always ends with an unescaped |
        let raw_fields = split_data_fields(content_after_depth, line_num, depth + 1)?;

        if depth == 0 {
            // Unwind entire stack into current block
            unwind_stack(&mut stack, 0, &mut current_block);

            let expected_cols = class_def.num_columns();
            let actual_cols = raw_fields.len();
            if actual_cols < expected_cols {
                return Err(StructError::new(
                    format!(
                        "too few columns: expected {}, found {}",
                        expected_cols, actual_cols
                    ),
                    line_num,
                    1,
                ));
            } else if actual_cols > expected_cols {
                return Err(StructError::new(
                    format!(
                        "too many columns: expected {}, found {}",
                        expected_cols, actual_cols
                    ),
                    line_num,
                    1,
                ));
            }

            // Parse depth 0 record
            let mut col_cursor = 0;
            let record = parse_record_fields(
                &class_def.fields,
                &raw_fields,
                &mut col_cursor,
                line_num,
            )?;

            stack.push(StackEntry {
                value: record,
                schema: StackItemSchema::Class(class_def.clone()),
                target_list_path: Vec::new(),
            });
        } else {
            // Depth >= 1: Child line
            // SPEC.md §5: Depth can rise by only one level at a time
            if depth > stack.len() {
                return Err(StructError::new(
                    format!(
                        "depth increased by more than one level (from depth {} to {})",
                        stack.len().saturating_sub(1),
                        depth
                    ),
                    line_num,
                    1,
                ));
            }

            // Unwind stack to depth (so stack[depth - 1] is the parent)
            unwind_stack(&mut stack, depth, &mut current_block);
            let parent_idx = depth - 1;

            // Identify target list on parent
            let parent_item = &stack[parent_idx];
            let list_targets = parent_item.schema.find_lists();

            if list_targets.is_empty() {
                return Err(StructError::new(
                    "parent record has no list fields to attach child line",
                    line_num,
                    1,
                ));
            }

            let (target_list, data_fields_start) = if list_targets.len() == 1 {
                // Exactly 1 list
                let target = &list_targets[0];
                let expected_cols = target.item_type.num_columns();

                // Check leniency: was an unneeded index provided?
                if raw_fields.len() == expected_cols + 1 {
                    let first_field_raw = raw_fields[0].raw.trim();
                    if target.accepted_indices.iter().any(|idx| idx == first_field_raw) {
                        decoded.warnings.push(Warning {
                            message: format!(
                                "unneeded list index '{}' on child line with exactly one list",
                                first_field_raw
                            ),
                            line: line_num,
                            column: depth + 1,
                        });
                        (target.clone(), 1)
                    } else {
                        let actual_cols = raw_fields.len();
                        if actual_cols < expected_cols {
                            return Err(StructError::new(
                                format!(
                                    "too few columns for child item: expected {}, found {}",
                                    expected_cols, actual_cols
                                ),
                                line_num,
                                depth + 1,
                            ));
                        } else {
                            return Err(StructError::new(
                                format!(
                                    "too many columns for child item: expected {}, found {}",
                                    expected_cols, actual_cols
                                ),
                                line_num,
                                depth + 1,
                            ));
                        }
                    }
                } else if raw_fields.len() == expected_cols {
                    (target.clone(), 0)
                } else {
                    let actual_cols = raw_fields.len();
                    if actual_cols < expected_cols {
                        return Err(StructError::new(
                            format!(
                                "too few columns for child item: expected {}, found {}",
                                expected_cols, actual_cols
                            ),
                            line_num,
                            depth + 1,
                        ));
                    } else {
                        return Err(StructError::new(
                            format!(
                                "too many columns for child item: expected {}, found {}",
                                expected_cols, actual_cols
                            ),
                            line_num,
                            depth + 1,
                        ));
                    }
                }
            } else {
                // 2 or more lists -> index is REQUIRED
                if raw_fields.is_empty() {
                    return Err(StructError::new("empty child line", line_num, depth + 1));
                }
                let index_str = raw_fields[0].raw.trim();
                let matched_target = list_targets.iter().find(|t| {
                    t.canonical_index == index_str || t.accepted_indices.iter().any(|a| a == index_str)
                });

                let target = match matched_target {
                    Some(t) => t.clone(),
                    None => {
                        return Err(validate_unmatched_index(
                            index_str,
                            parent_item.schema.fields(),
                            line_num,
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
                        line_num,
                        depth + 1,
                    ));
                } else if actual_cols > expected_cols {
                    return Err(StructError::new(
                        format!(
                            "too many columns for child item: expected {}, found {}",
                            expected_cols, actual_cols
                        ),
                        line_num,
                        depth + 1,
                    ));
                }

                (target, 1)
            };

            // Parse list item
            let item_fields = &raw_fields[data_fields_start..];
            let (parsed_item, item_schema) = match &target_list.item_type {
                ListItemType::Scalar(scalar_type) => {
                    let val = parse_scalar_value(
                        &item_fields[0],
                        *scalar_type,
                        line_num,
                    )?;
                    (val, StackItemSchema::Scalar)
                }
                ListItemType::Enum(labels) => {
                    let val = parse_enum_value(&item_fields[0], labels, line_num)?;
                    (val, StackItemSchema::Enum)
                }
                ListItemType::Record(rec_def) => {
                    let mut col_cursor = 0;
                    let val = parse_record_fields(
                        &rec_def.fields,
                        item_fields,
                        &mut col_cursor,
                        line_num,
                    )?;
                    (val, StackItemSchema::Record(rec_def.clone()))
                }
            };

            // Push this item onto the stack at depth with target_list_path
            stack.push(StackEntry {
                value: parsed_item,
                schema: item_schema,
                target_list_path: target_list.field_path,
            });
        }
    }

    // Unwind all remaining stack items into current block
    unwind_stack(&mut stack, 0, &mut current_block);
    if let Some(b) = current_block {
        decoded.blocks.push(b);
    }

    Ok(decoded)
}

/// Helper representing an item on the hierarchical tree stack.
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

/// Validates an unmatched index on a child line when parent has 2 or more lists.
/// Differentiates between:
/// - missing index (e.g. data value present where index was expected)
/// - index naming a non-list field
/// - index out of range
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

/// Unwinds the stack until its length is `target_len`.
/// Each popped child item is inserted into its parent at `target_list_path`.
/// If depth 0 is popped, it is pushed into `current_block.records`.
fn unwind_stack(
    stack: &mut Vec<StackEntry>,
    target_len: usize,
    block_opt: &mut Option<Block>,
) {
    while stack.len() > target_len {
        let entry = stack.pop().unwrap();
        if stack.is_empty() {
            // Depth 0 item
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

/// Recursively appends an item to the target list field in a parent Value.
/// If the parent was temporarily Value::Null (because its scalar columns were all ~),
/// it is reconstituted into a Value::Record using its schema so child items can attach.
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
            // Target list reached
            if let Value::List(items) = field_val {
                items.push(item);
            }
        } else {
            // Recurse into nested record
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

/// Raw representation of a field slice on a data line.
#[derive(Debug, Clone)]
struct RawField<'a> {
    raw: &'a str,
    col_start: usize,
}

/// Splits a line by unescaped `|` delimiters.
/// Enforces that line ends with an unescaped `|`.
fn split_data_fields<'a>(
    line: &'a str,
    line_num: usize,
    base_col: usize,
) -> Result<Vec<RawField<'a>>, StructError> {
    let mut fields = Vec::new();
    let mut start_idx = 0;
    let mut in_escape = false;

    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let mut has_trailing_pipe = false;

    for (i, c) in &chars {
        if in_escape {
            in_escape = false;
            continue;
        }
        if *c == '\\' {
            in_escape = true;
            continue;
        }
        if *c == '|' {
            fields.push(RawField {
                raw: &line[start_idx..*i],
                col_start: base_col + start_idx,
            });
            start_idx = i + 1;
            if *i == line.len() - 1 {
                has_trailing_pipe = true;
            }
        }
    }

    if in_escape {
        return Err(StructError::new(
            "dangling backslash escape at end of line",
            line_num,
            base_col + line.len(),
        ));
    }

    if !has_trailing_pipe {
        return Err(StructError::new(
            "data line must always end with an unescaped '|'",
            line_num,
            base_col + line.len(),
        ));
    }

    Ok(fields)
}

/// Recursively parses fields for a record from the raw field slice.
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
                let sub_val = parse_record_fields(
                    &sub_rec.fields,
                    raw_fields,
                    cursor,
                    line_num,
                )?;
                if sub_rec.num_columns() > 0 {
                    has_any_columns = true;
                    if !sub_val.is_null() {
                        all_columns_null = false;
                    }
                }
                record_fields.push((field_def.name.clone(), sub_val));
            }
            FieldType::List(_) => {
                // Lists occupy 0 columns on parent line
                record_fields.push((field_def.name.clone(), Value::List(Vec::new())));
            }
        }
    }

    // SPEC.md §3: A record can be null. A record whose columns are all ~ reads back as a null record.
    // A record that contains a list is never null.
    if !contains_list && has_any_columns && all_columns_null {
        Ok(Value::Null)
    } else {
        Ok(Value::Record(record_fields))
    }
}

/// Decodes and unescapes a single field string.
/// Returns Ok(None) for null (~), or Ok(Some(unescaped_text)).
fn unescape_field(raw_field: &RawField, line_num: usize) -> Result<Option<String>, StructError> {
    let s = raw_field.raw;
    let col_start = raw_field.col_start;

    // SPEC.md §3: Spaces next to each | are trimmed. Spaces inside a value are kept.
    // Untrimmed leading and trailing plain spaces are trimmed.
    let leading_spaces = s.chars().take_while(|c| *c == ' ' || *c == '\t').count();
    let trailing_spaces = s.chars().rev().take_while(|c| *c == ' ' || *c == '\t').count();

    if leading_spaces + trailing_spaces > s.len() {
        // Entirely plain whitespace -> empty field!
        // SPEC.md §3: An empty field is always an error. There's no empty string. Blank text is written ~.
        return Err(StructError::new(
            "empty field is an error; blank text must be written as '~'",
            line_num,
            col_start,
        ));
    }

    let trimmed = &s[leading_spaces..s.len() - trailing_spaces];
    if trimmed.is_empty() {
        return Err(StructError::new(
            "empty field is an error; blank text must be written as '~'",
            line_num,
            col_start,
        ));
    }

    // Check for null: exactly '~'
    if trimmed == "~" {
        return Ok(None);
    }

    // Unescape
    let mut out = String::with_capacity(trimmed.len());
    let mut chars = trimmed.char_indices().peekable();
    let base_col = col_start + leading_spaces;

    while let Some((i, c)) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some((_, '|')) => out.push('|'),
                Some((_, '\\')) => out.push('\\'),
                Some((_, 'n')) => out.push('\n'),
                Some((_, '~')) => out.push('~'),
                Some((_, 's')) => out.push(' '),
                Some((_, '>')) => out.push('>'),
                Some((_, '#')) => out.push('#'),
                Some((sub_i, invalid_esc)) => {
                    return Err(StructError::new(
                        format!("invalid escape sequence '\\{}'", invalid_esc),
                        line_num,
                        base_col + sub_i - 1,
                    ));
                }
                None => {
                    return Err(StructError::new(
                        "dangling backslash escape",
                        line_num,
                        base_col + i,
                    ));
                }
            }
        } else {
            out.push(c);
        }
    }

    Ok(Some(out))
}

/// Parses a scalar value for a field.
fn parse_scalar_value(
    raw_field: &RawField,
    scalar_type: ScalarType,
    line_num: usize,
) -> Result<Value, StructError> {
    let unescaped = unescape_field(raw_field, line_num)?;
    let text = match unescaped {
        None => return Ok(Value::Null),
        Some(t) => t,
    };

    match scalar_type {
        ScalarType::Str => Ok(Value::String(text)),
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
        ScalarType::Float => match text.parse::<f64>() {
            Ok(f) => Ok(Value::Float(f)),
            Err(_) => Err(StructError::new(
                format!("type mismatch: '{}' is not a valid float (f64)", text),
                line_num,
                raw_field.col_start,
            )),
        },
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

/// Parses an enum value for a field.
fn parse_enum_value(
    raw_field: &RawField,
    labels: &[String],
    line_num: usize,
) -> Result<Value, StructError> {
    let unescaped = unescape_field(raw_field, line_num)?;
    let text = match unescaped {
        None => return Ok(Value::Null),
        Some(t) => t,
    };

    if labels.iter().any(|l| l == &text) {
        Ok(Value::Enum(text))
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
