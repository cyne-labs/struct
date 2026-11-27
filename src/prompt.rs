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
                    ScalarType::Bool => "true",
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

fn build_placeholder_record(
    fields: &[FieldDef],
    is_top_level: bool,
    own_lists_count: usize,
) -> crate::types::Value {
    use crate::types::Value;

    let mut rec_fields = Vec::new();
    for f in fields {
        match &f.field_type {
            FieldType::Scalar(s) => {
                let v = match s {
                    ScalarType::Str => Value::String("text".to_string()),
                    ScalarType::Int => Value::Int(7),
                    ScalarType::Float => Value::Float(1.5),
                    ScalarType::Bool => Value::Bool(true),
                };
                rec_fields.push((f.name.clone(), v));
            }
            FieldType::Enum(labels) => {
                let v = Value::Enum(labels.first().cloned().unwrap_or_default());
                rec_fields.push((f.name.clone(), v));
            }
            FieldType::Record(r) => {
                let sub_val = build_placeholder_record(&r.fields, false, own_lists_count);
                rec_fields.push((f.name.clone(), sub_val));
            }
            FieldType::List(item_type) => {
                if own_lists_count == 0 {
                    rec_fields.push((f.name.clone(), Value::List(Vec::new())));
                } else {
                    let items = match &**item_type {
                        ListItemType::Scalar(s) => {
                            let item = match s {
                                ScalarType::Str => Value::String("text".to_string()),
                                ScalarType::Int => Value::Int(7),
                                ScalarType::Float => Value::Float(1.5),
                                ScalarType::Bool => Value::Bool(true),
                            };
                            vec![item]
                        }
                        ListItemType::Enum(labels) => {
                            let item = Value::Enum(labels.first().cloned().unwrap_or_default());
                            vec![item]
                        }
                        ListItemType::Record(r) => {
                            let item1 = build_placeholder_record(&r.fields, false, 1);
                            let item2 = build_placeholder_record(&r.fields, false, 0);
                            vec![item1, item2]
                        }
                    };
                    rec_fields.push((f.name.clone(), Value::List(items)));
                }
            }
        }
    }

    if is_top_level {
        set_last_scalar_to_null(&mut rec_fields);
    }

    Value::Record(rec_fields)
}

fn set_last_scalar_to_null(fields: &mut [(String, crate::types::Value)]) -> bool {
    use crate::types::Value;
    for (_, val) in fields.iter_mut().rev() {
        match val {
            Value::String(_) | Value::Int(_) | Value::Float(_) | Value::Bool(_) => {
                *val = Value::Null;
                return true;
            }
            Value::Record(sub_fields) => {
                if set_last_scalar_to_null(sub_fields) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
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
    out.push_str("null means null. bool is true or false. enum values must be one of the listed labels exactly.\n");
    out.push_str("Inside a value, write \\| for | and \\\\ for \\.\n");

    // 3. When any class has a record{...} field:
    if any_class_has_record(&classes) {
        out.push_str("A record{...} field adds its fields as extra columns, in order.\n");
    }

    // 4. When any class has a list:
    if any_class_has_list(&classes) {
        out.push_str("List items go on the lines below their parent, starting with one > per level: > for its items, >> for theirs.\n");
        out.push_str("A > line ends with |; instead of | when it is the last of its group (the next line is shallower, or the data ends). A line with lists but no items also ends with |;.\n");
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

    // When any class has a list field, after the rules:
    if any_class_has_list(&classes) {
        out.push_str("List fields take no column on their parent's line; their items go on the child lines below it.\n");
    }

    // 8. Always last: one example line
    let first_class = &classes[0];
    let mut cols = Vec::new();
    collect_example_cols(&first_class.fields, &mut cols);

    let last_scalar_idx = cols.iter().rposition(|c| matches!(c, ExampleCol::Scalar(_)));
    if let Some(idx) = last_scalar_idx {
        cols[idx] = ExampleCol::Scalar("null".to_string());
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
    if first_class.has_list() {
        out.push_str("|;\n");
    } else {
        out.push_str("|\n");
    }

    // 9. When any class has a list field: worked example with lists
    if let Some(first_list_class) = classes.iter().find(|c| c.has_list()) {
        let placeholder_rec = build_placeholder_record(&first_list_class.fields, true, 1);
        let mut doc = crate::types::DecodedFile::new();
        doc.schemas.insert(first_list_class.name.clone(), first_list_class.clone());
        doc.blocks.push(crate::types::Block {
            class_name: first_list_class.name.clone(),
            records: vec![placeholder_rec],
        });
        let encoded = crate::encode::encode_with(schemas, &doc)?;
        out.push_str("Example with lists:\n");
        out.push_str(&encoded);
    }

    Ok(out)
}
