//! Schema parsing and type definitions for Struct (v0.2).
//!
//! Standard library only.

use crate::types::StructError;

/// Scalar types supported in Struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScalarType {
    /// UTF-8 text string (default when no type is specified).
    Str,
    /// 64-bit signed integer.
    Int,
    /// 64-bit floating point number.
    Float,
    /// Boolean flag (1 = true, 0 = false).
    Bool,
}

impl ScalarType {
    /// Returns the canonical type name in schema syntax.
    pub fn as_str(&self) -> &'static str {
        match self {
            ScalarType::Str => "str",
            ScalarType::Int => "int",
            ScalarType::Float => "float",
            ScalarType::Bool => "bool",
        }
    }
}

/// Item type allowed inside a list.
/// SPEC.md §2: "where T is a scalar type, an enum, or record{...}".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListItemType {
    /// A scalar item.
    Scalar(ScalarType),
    /// An enum item.
    Enum(Vec<String>),
    /// An inline record item.
    Record(RecordDef),
}

impl ListItemType {
    /// Formats this list item type for schema output.
    pub fn format_schema(&self) -> String {
        match self {
            ListItemType::Scalar(s) => s.as_str().to_string(),
            ListItemType::Enum(labels) => format!("enum({})", labels.join(",")),
            ListItemType::Record(rec) => rec.format_schema(),
        }
    }
}

/// Data type of a field in a class or record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    /// A scalar value.
    Scalar(ScalarType),
    /// An enum value.
    Enum(Vec<String>),
    /// A nested record flattened into columns.
    Record(RecordDef),
    /// A list of items written as child lines.
    List(Box<ListItemType>),
}

impl FieldType {
    /// Formats this field type for schema output.
    pub fn format_schema(&self) -> String {
        match self {
            FieldType::Scalar(s) => s.as_str().to_string(),
            FieldType::Enum(labels) => format!("enum({})", labels.join(",")),
            FieldType::Record(rec) => rec.format_schema(),
            FieldType::List(item) => format!("list[{}]", item.format_schema()),
        }
    }
}

/// Definition of a single field inside a class or record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDef {
    /// The field name.
    pub name: String,
    /// The field type.
    pub field_type: FieldType,
}

impl FieldDef {
    /// Formats this field definition for schema output.
    /// Omits `:str` for text fields, per SPEC.md minimal syntax.
    pub fn format_schema(&self) -> String {
        match &self.field_type {
            FieldType::Scalar(ScalarType::Str) => self.name.clone(),
            _ => format!("{}:{}", self.name, self.field_type.format_schema()),
        }
    }

    /// Returns true if this field is a list or contains a list at any depth.
    pub fn has_list(&self) -> bool {
        match &self.field_type {
            FieldType::List(_) => true,
            FieldType::Record(r) => r.has_list(),
            FieldType::Scalar(_) | FieldType::Enum(_) => false,
        }
    }
}

/// Definition of a record shape (a sequence of fields).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordDef {
    /// Fields comprising this record, in declaration order.
    pub fields: Vec<FieldDef>,
}

impl RecordDef {
    /// Returns the number of columns this record occupies on a data line.
    /// Lists do not occupy any columns (their items become child lines).
    pub fn num_columns(&self) -> usize {
        let mut count = 0;
        for f in &self.fields {
            match &f.field_type {
                FieldType::Scalar(_) | FieldType::Enum(_) => count += 1,
                FieldType::Record(r) => count += r.num_columns(),
                FieldType::List(_) => {}
            }
        }
        count
    }

    /// Formats this record as `record{...}` for schema output.
    pub fn format_schema(&self) -> String {
        let field_strs: Vec<String> = self.fields.iter().map(|f| f.format_schema()).collect();
        format!("record{{{}}}", field_strs.join(", "))
    }

    /// Returns true if this record has a list anywhere inside it, at any depth.
    pub fn has_list(&self) -> bool {
        self.fields.iter().any(|f| f.has_list())
    }

    /// Returns the name of the first field that is or contains a list.
    pub fn first_list_field_name(&self) -> Option<String> {
        for f in &self.fields {
            if f.has_list() {
                return Some(f.name.clone());
            }
        }
        None
    }

    /// Finds all child lists directly or indirectly contained in this record.
    pub fn find_lists(&self) -> Vec<ListTarget> {
        let mut out = Vec::new();
        find_lists_internal(self, Vec::new(), &mut out);

        // Assign canonical indices and accepted indices based on SPEC.md §5 rules
        let total_lists = out.len();
        for target in &mut out {
            if total_lists == 1 {
                // Rule 1: exactly one list anywhere -> no index
                target.canonical_index = String::new();
                // But leniency allows plain field index
                target.accepted_indices.push(target.field_path[0].to_string());
                if target.field_path.len() > 1 {
                    target.accepted_indices.push(target.field_path.iter().map(|n| n.to_string()).collect::<Vec<_>>().join("."));
                }
            } else {
                // Rule 2 & 3: two or more lists
                if target.field_path.len() == 1 {
                    target.canonical_index = target.field_path[0].to_string();
                    target.accepted_indices.push(target.canonical_index.clone());
                } else {
                    // List is inside a record field.
                    // Check if that record field contains only this list
                    let parent_field_idx = target.field_path[0];
                    let parent_field = &self.fields[parent_field_idx];
                    let sub_list_count = count_lists_in_field(parent_field);
                    let dotted = target.field_path.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(".");

                    if sub_list_count == 1 {
                        // SPEC.md §5: "If a record field contains only one list, its plain field number is enough (>1|)."
                        target.canonical_index = parent_field_idx.to_string();
                        target.accepted_indices.push(target.canonical_index.clone());
                        target.accepted_indices.push(dotted);
                    } else {
                        // Several lists inside one record field -> dotted index
                        target.canonical_index = dotted.clone();
                        target.accepted_indices.push(dotted);
                    }
                }
            }
        }

        out
    }
}

/// Helper to count all lists inside a single field.
fn count_lists_in_field(field: &FieldDef) -> usize {
    match &field.field_type {
        FieldType::List(_) => 1,
        FieldType::Record(r) => {
            let mut c = 0;
            for f in &r.fields {
                c += count_lists_in_field(f);
            }
            c
        }
        _ => 0,
    }
}

/// A target list found inside a record schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListTarget {
    /// 0-based field index path from root (e.g. [1, 2]).
    pub field_path: Vec<usize>,
    /// The canonical index string emitted when encoding child lines.
    pub canonical_index: String,
    /// Indices accepted during decoding (including dotted and plain variants).
    pub accepted_indices: Vec<String>,
    /// The list item type.
    pub item_type: ListItemType,
}

fn find_lists_internal(
    record: &RecordDef,
    current_path: Vec<usize>,
    out: &mut Vec<ListTarget>,
) {
    for (i, field) in record.fields.iter().enumerate() {
        let mut path = current_path.clone();
        path.push(i);
        match &field.field_type {
            FieldType::List(item) => {
                out.push(ListTarget {
                    field_path: path,
                    canonical_index: String::new(),
                    accepted_indices: Vec::new(),
                    item_type: (**item).clone(),
                });
            }
            FieldType::Record(sub) => {
                find_lists_internal(sub, path, out);
            }
            _ => {}
        }
    }
}

/// Definition of a class in Struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDef {
    /// The class name.
    pub name: String,
    /// Fields comprising this class.
    pub fields: Vec<FieldDef>,
}

impl ClassDef {
    /// Converts this class into a `RecordDef` for column counting and list discovery.
    pub fn as_record_def(&self) -> RecordDef {
        RecordDef {
            fields: self.fields.clone(),
        }
    }

    /// Returns the number of columns this class occupies on a depth-0 data line.
    pub fn num_columns(&self) -> usize {
        self.as_record_def().num_columns()
    }

    /// Finds all child lists directly or indirectly contained in this class.
    pub fn find_lists(&self) -> Vec<ListTarget> {
        self.as_record_def().find_lists()
    }

    /// Returns true if this class has a list anywhere inside it, at any depth.
    pub fn has_list(&self) -> bool {
        self.as_record_def().has_list()
    }

    /// Returns the name of the first field that is or contains a list.
    pub fn first_list_field_name(&self) -> Option<String> {
        self.as_record_def().first_list_field_name()
    }

    /// Formats this class definition line, e.g. `schema:claim{text, year:int}\n`.
    pub fn format_schema_line(&self) -> String {
        let field_strs: Vec<String> = self.fields.iter().map(|f| f.format_schema()).collect();
        format!("schema:{}{{{}}}\n", self.name, field_strs.join(", "))
    }
}

/// Checks whether a string is a valid Struct identifier.
/// SPEC.md §2: "made of letters, digits and _, and don't start with a digit."
pub fn is_valid_ident(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if first.is_ascii_digit() || (!first.is_alphabetic() && first != '_') {
        return false;
    }
    for c in chars {
        if !c.is_alphanumeric() && c != '_' {
            return false;
        }
    }
    true
}

/// Represents the parsed outcome of a `schema:` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedSchemaLine {
    /// A new class definition: `schema:name{...}`.
    Define(ClassDef),
    /// A reuse of an existing class: `schema:name`.
    Reuse(String),
}

/// Parses a schema line.
///
/// Enforces:
/// - Starts with `schema:`
/// - Does not contain `|`
/// - Valid class names and field names
/// - Valid type syntax
/// - No class references in lists
pub fn parse_schema_line(line_str: &str, line_num: usize) -> Result<ParsedSchemaLine, StructError> {
    if !line_str.starts_with("schema:") {
        return Err(StructError::new("schema line must start with 'schema:'", line_num, 1));
    }
    if line_str.contains('|') {
        let col = line_str.find('|').unwrap() + 1;
        return Err(StructError::new("schema line must never contain '|'", line_num, col));
    }

    let rest = &line_str["schema:".len()..];
    let trimmed_rest = rest.trim();
    if trimmed_rest.is_empty() {
        return Err(StructError::new("missing class name after 'schema:'", line_num, 8));
    }

    // Check if there is an opening brace `{`
    if let Some(brace_pos) = trimmed_rest.find('{') {
        // Class definition
        let name_part = trimmed_rest[..brace_pos].trim();
        if !is_valid_ident(name_part) {
            return Err(StructError::new(
                format!("invalid class name '{}'", name_part),
                line_num,
                8,
            ));
        }

        if !trimmed_rest.ends_with('}') {
            return Err(StructError::new("schema definition missing closing '}'", line_num, line_str.len()));
        }

        let inside = &trimmed_rest[brace_pos + 1..trimmed_rest.len() - 1].trim();
        let fields = parse_fields(inside, line_num, 8 + brace_pos + 1)?;

        Ok(ParsedSchemaLine::Define(ClassDef {
            name: name_part.to_string(),
            fields,
        }))
    } else {
        // Class reuse
        let name_part = trimmed_rest;
        if !is_valid_ident(name_part) {
            return Err(StructError::new(
                format!("invalid class name '{}' in reuse", name_part),
                line_num,
                8,
            ));
        }
        Ok(ParsedSchemaLine::Reuse(name_part.to_string()))
    }
}

/// Parses a comma-separated list of fields inside braces.
fn parse_fields(input: &str, line_num: usize, base_col: usize) -> Result<Vec<FieldDef>, StructError> {
    let mut fields = Vec::new();
    let parts = split_top_level_commas(input, line_num, base_col)?;

    let mut seen_names = std::collections::HashSet::new();

    for (part_str, part_col) in parts {
        let trimmed = part_str.trim();
        if trimmed.is_empty() {
            return Err(StructError::new("empty field definition in schema", line_num, base_col + part_col));
        }

        let f = parse_single_field(trimmed, line_num, base_col + part_col)?;
        if seen_names.contains(&f.name) {
            return Err(StructError::new(
                format!("duplicate field name '{}' in schema", f.name),
                line_num,
                base_col + part_col,
            ));
        }
        seen_names.insert(f.name.clone());
        fields.push(f);
    }

    Ok(fields)
}

/// Splits a string by commas, respecting nested braces `{}` and brackets `[]` and parens `()`.
fn split_top_level_commas(
    input: &str,
    line_num: usize,
    base_col: usize,
) -> Result<Vec<(&str, usize)>, StructError> {
    let mut parts = Vec::new();
    let mut depth_curly = 0;
    let mut depth_square = 0;
    let mut depth_paren = 0;
    let mut start_idx = 0;

    let chars: Vec<(usize, char)> = input.char_indices().collect();
    for (i, c) in &chars {
        match c {
            '{' => depth_curly += 1,
            '}' => {
                if depth_curly == 0 {
                    return Err(StructError::new("unmatched '}' in schema", line_num, base_col + i));
                }
                depth_curly -= 1;
            }
            '[' => depth_square += 1,
            ']' => {
                if depth_square == 0 {
                    return Err(StructError::new("unmatched ']' in schema", line_num, base_col + i));
                }
                depth_square -= 1;
            }
            '(' => depth_paren += 1,
            ')' => {
                if depth_paren == 0 {
                    return Err(StructError::new("unmatched ')' in schema", line_num, base_col + i));
                }
                depth_paren -= 1;
            }
            ',' if depth_curly == 0 && depth_square == 0 && depth_paren == 0 => {
                parts.push((&input[start_idx..*i], start_idx));
                start_idx = i + 1;
            }
            _ => {}
        }
    }

    if depth_curly > 0 {
        return Err(StructError::new("unclosed '{' in schema", line_num, base_col + input.len()));
    }
    if depth_square > 0 {
        return Err(StructError::new("unclosed '[' in schema", line_num, base_col + input.len()));
    }
    if depth_paren > 0 {
        return Err(StructError::new("unclosed '(' in schema", line_num, base_col + input.len()));
    }

    if start_idx < input.len() {
        parts.push((&input[start_idx..], start_idx));
    }

    Ok(parts)
}

/// Parses a single field definition string, e.g. `text` or `year:int` or `notes:list[str]`.
fn parse_single_field(s: &str, line_num: usize, col: usize) -> Result<FieldDef, StructError> {
    if let Some(colon_pos) = s.find(':') {
        let name_part = s[..colon_pos].trim();
        let type_part = s[colon_pos + 1..].trim();

        if !is_valid_ident(name_part) {
            return Err(StructError::new(
                format!("invalid field name '{}'", name_part),
                line_num,
                col,
            ));
        }

        let field_type = parse_field_type(type_part, line_num, col + colon_pos + 1)?;
        Ok(FieldDef {
            name: name_part.to_string(),
            field_type,
        })
    } else {
        let name_part = s.trim();
        if !is_valid_ident(name_part) {
            return Err(StructError::new(
                format!("invalid field name '{}'", name_part),
                line_num,
                col,
            ));
        }
        Ok(FieldDef {
            name: name_part.to_string(),
            field_type: FieldType::Scalar(ScalarType::Str),
        })
    }
}

/// Parses a field type string.
fn parse_field_type(type_str: &str, line_num: usize, col: usize) -> Result<FieldType, StructError> {
    let t = type_str.trim();
    if t == "str" {
        Ok(FieldType::Scalar(ScalarType::Str))
    } else if t == "int" {
        Ok(FieldType::Scalar(ScalarType::Int))
    } else if t == "float" {
        Ok(FieldType::Scalar(ScalarType::Float))
    } else if t == "bool" {
        Ok(FieldType::Scalar(ScalarType::Bool))
    } else if t.starts_with("enum(") && t.ends_with(')') {
        let inside = &t["enum(".len()..t.len() - 1].trim();
        let labels = parse_enum_labels(inside, line_num, col + 5)?;
        Ok(FieldType::Enum(labels))
    } else if t.starts_with("record{") && t.ends_with('}') {
        let inside = &t["record{".len()..t.len() - 1].trim();
        let fields = parse_fields(inside, line_num, col + 7)?;
        Ok(FieldType::Record(RecordDef { fields }))
    } else if t.starts_with("list[") && t.ends_with(']') {
        let inside = &t["list[".len()..t.len() - 1].trim();
        let item_type = parse_list_item_type(inside, line_num, col + 5)?;
        Ok(FieldType::List(Box::new(item_type)))
    } else {
        Err(StructError::new(
            format!("unknown or invalid type '{}'", type_str),
            line_num,
            col,
        ))
    }
}

/// Parses list item type.
/// SPEC.md §2: "where T is a scalar type, an enum, or record{...}".
/// SPEC.md §140: "No class references. A field can't refer to another class by name (list[note])."
fn parse_list_item_type(type_str: &str, line_num: usize, col: usize) -> Result<ListItemType, StructError> {
    let t = type_str.trim();
    if t == "str" {
        Ok(ListItemType::Scalar(ScalarType::Str))
    } else if t == "int" {
        Ok(ListItemType::Scalar(ScalarType::Int))
    } else if t == "float" {
        Ok(ListItemType::Scalar(ScalarType::Float))
    } else if t == "bool" {
        Ok(ListItemType::Scalar(ScalarType::Bool))
    } else if t.starts_with("enum(") && t.ends_with(')') {
        let inside = &t["enum(".len()..t.len() - 1].trim();
        let labels = parse_enum_labels(inside, line_num, col + 5)?;
        Ok(ListItemType::Enum(labels))
    } else if t.starts_with("record{") && t.ends_with('}') {
        let inside = &t["record{".len()..t.len() - 1].trim();
        let fields = parse_fields(inside, line_num, col + 7)?;
        Ok(ListItemType::Record(RecordDef { fields }))
    } else {
        // Any other item type, including class references like list[note]
        Err(StructError::new(
            format!("invalid list item type '{}' (class references are not allowed; use inline record{{...}} or scalar)", type_str),
            line_num,
            col,
        ))
    }
}

/// Parses comma-separated enum labels.
/// SPEC.md §2: "Labels follow the same rule as names: letters, digits and _, no spaces, and they don't start with a digit."
fn parse_enum_labels(input: &str, line_num: usize, col: usize) -> Result<Vec<String>, StructError> {
    if input.is_empty() {
        return Err(StructError::new("enum must have at least one label", line_num, col));
    }
    let parts: Vec<&str> = input.split(',').collect();
    let mut labels = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for p in parts {
        let trimmed = p.trim();
        if trimmed.is_empty() {
            return Err(StructError::new("empty enum label", line_num, col));
        }
        // SPEC.md: "no spaces"
        if p.contains(' ') || p.contains('\t') {
            return Err(StructError::new(
                format!("enum label '{}' must not contain spaces", p),
                line_num,
                col,
            ));
        }
        if !is_valid_ident(trimmed) {
            return Err(StructError::new(
                format!("invalid enum label '{}'", trimmed),
                line_num,
                col,
            ));
        }
        if seen.contains(trimmed) {
            return Err(StructError::new(
                format!("duplicate enum label '{}'", trimmed),
                line_num,
                col,
            ));
        }
        seen.insert(trimmed.to_string());
        labels.push(trimmed.to_string());
    }

    Ok(labels)
}
