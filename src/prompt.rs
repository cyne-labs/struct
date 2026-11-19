//! Prompt builder module for Struct (SPEC.md §10).
//!
//! Standard library only.

use crate::schema::{parse_schema_line, ClassDef, FieldDef, FieldType, ListItemType, ParsedSchemaLine, ScalarType};
use crate::types::StructError;

/// Parses outside schemas into an ordered list of class definitions.
fn parse_outside_schemas_ordered(schemas: &str) -> Result<Vec<ClassDef>, StructError> {
    let mut classes = Vec::new();
    let mut names = std::collections::HashSet::new();

    for (line_idx, raw_line) in schemas.split('\n').enumerate() {
        let line_num = line_idx + 1;
        let clean = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed = clean.trim_matches(|c| c == ' ' || c == '\t');
        if trimmed.is_empty() {
            continue;
        }
        match parse_schema_line(trimmed, line_num) {
            Ok(ParsedSchemaLine::Define(class_def)) => {
                if names.contains(&class_def.name) {
                    return Err(StructError::new(
                        format!("error in outside schemas: class '{}' defined twice", class_def.name),
                        line_num,
                        8,
                    ));
                }
                names.insert(class_def.name.clone());
                classes.push(class_def);
            }
            Ok(ParsedSchemaLine::Reuse(name)) => {
                return Err(StructError::new(
                    format!(
                        "error in outside schemas: class reuse 'schema:{}' is not allowed in outside schemas",
                        name
                    ),
                    line_num,
                    8,
                ));
            }
            Err(e) => {
                return Err(StructError::new(
                    format!("error in outside schemas: {}", e.message),
                    e.line,
                    e.column,
                ));
            }
        }
    }

    if classes.is_empty() {
        return Err(StructError::new(
            "error in outside schemas: at least one class definition is required",
            1,
            1,
        ));
    }

    Ok(classes)
}

fn field_has_record(f: &FieldDef) -> bool {
    match &f.field_type {
        FieldType::Record(_) => true,
        FieldType::List(item) => match &**item {
            ListItemType::Record(_) => true,
            _ => false,
        },
        _ => false,
    }
}

fn any_class_has_record(classes: &[ClassDef]) -> bool {
    classes.iter().any(|c| c.fields.iter().any(field_has_record))
}

fn any_class_has_list(classes: &[ClassDef]) -> bool {
    classes.iter().any(|c| c.has_list())
}

fn check_sub_parents_for_multiple_lists(fields: &[FieldDef]) -> bool {
    for f in fields {
        match &f.field_type {
            FieldType::List(item) => {
                if let ListItemType::Record(r) = &**item {
                    if r.find_lists().len() >= 2 {
                        return true;
                    }
                    if check_sub_parents_for_multiple_lists(&r.fields) {
                        return true;
                    }
                }
            }
            FieldType::Record(r) => {
                if check_sub_parents_for_multiple_lists(&r.fields) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn any_parent_has_two_or_more_lists(classes: &[ClassDef]) -> bool {
    for c in classes {
        if c.find_lists().len() >= 2 {
            return true;
        }
        if check_sub_parents_for_multiple_lists(&c.fields) {
            return true;
        }
    }
    false
}

fn field_has_text(f: &FieldDef) -> bool {
    match &f.field_type {
        FieldType::Scalar(ScalarType::Str) => true,
        FieldType::Record(r) => r.fields.iter().any(field_has_text),
        FieldType::List(item) => match &**item {
            ListItemType::Scalar(ScalarType::Str) => true,
            ListItemType::Record(r) => r.fields.iter().any(field_has_text),
            _ => false,
        },
        _ => false,
    }
}

fn any_class_has_text(classes: &[ClassDef]) -> bool {
    classes.iter().any(|c| c.fields.iter().any(field_has_text))
}

enum ExampleCol {
    Scalar(String),
    Enum(String),
}

fn collect_example_cols(fields: &[FieldDef], cols: &mut Vec<ExampleCol>) {
    for f in fields {
        match &f.field_type {
            FieldType::Scalar(s) => {
                let val = match s {
                    ScalarType::Str => "text",
                    ScalarType::Int => "7",
                    ScalarType::Float => "1.5",
                    ScalarType::Bool => "1",
                };
                cols.push(ExampleCol::Scalar(val.to_string()));
            }
            FieldType::Enum(labels) => {
                let val = labels.first().map(|s| s.as_str()).unwrap_or("");
                cols.push(ExampleCol::Enum(val.to_string()));
            }
            FieldType::Record(r) => {
                collect_example_cols(&r.fields, cols);
            }
            FieldType::List(_) => {
                // Lists are left out of the example.
            }
        }
    }
}

/// Builds the model instruction prompt from outside schemas (SPEC.md §10).
pub fn prompt(schemas: &str) -> Result<String, StructError> {
    let classes = parse_outside_schemas_ordered(schemas)?;

    let mut out = String::new();

    // 1. Schema lines first
    for c in &classes {
        out.push_str(&c.format_schema_line());
    }

    // 2. Always
    out.push_str("Reply with data only in Struct format: no prose, no code fences.\n");
    out.push_str("Never write a header row or column names. The first line is already data.\n");
    out.push_str("One record per line: values in schema order, separated by |, and every line ends with |.\n");
    out.push_str("~ means null. bool is 1 or 0. enum values must be one of the listed labels exactly.\n");
    out.push_str("Inside a value, write \\| for | and \\ for \\.\n");

    // 3. When any class has a record{...} field:
    if any_class_has_record(&classes) {
        out.push_str("A record{...} field adds its fields as extra columns, in order.\n");
    }

    // 4. When any class has a list:
    if any_class_has_list(&classes) {
        out.push_str("List items go on the lines below their parent, starting with one > per level: > for its items, >> for theirs.\n");
    }

    // 5. When any parent has two or more lists:
    if any_parent_has_two_or_more_lists(&classes) {
        out.push_str("When a parent has several lists, start each item with the list's field number and |, e.g. >2|item|. Fields count from 0 in schema order.\n");
    }

    // 6. When any class has a text field:
    if any_class_has_text(&classes) {
        out.push_str("For text with line breaks, or with spaces or tabs at its start or end, start the value with \\r, then write it exactly as it is, keeping every space, tab, indent and line break, until the closing |.\n");
    }

    // 7. When there are two or more classes:
    if classes.len() >= 2 {
        out.push_str("Start each block with schema:name on its own line.\n");
    }

    // 8. Always last: one example line
    let first_class = &classes[0];
    let mut cols = Vec::new();
    collect_example_cols(&first_class.fields, &mut cols);

    let last_scalar_idx = cols.iter().rposition(|c| matches!(c, ExampleCol::Scalar(_)));
    if let Some(idx) = last_scalar_idx {
        cols[idx] = ExampleCol::Scalar("~".to_string());
    }

    let col_strs: Vec<&str> = cols
        .iter()
        .map(|c| match c {
            ExampleCol::Scalar(s) => s.as_str(),
            ExampleCol::Enum(s) => s.as_str(),
        })
        .collect();

    out.push_str("Example: ");
    out.push_str(&col_strs.join("|"));
    out.push_str("|\n");

    Ok(out)
}
