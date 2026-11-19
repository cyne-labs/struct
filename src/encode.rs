//! Encoding implementation for Struct (v0.3).
//!
//! Serializes in-memory Struct data into canonical Struct text format.

use std::collections::{BTreeMap, HashSet};
use crate::schema::{
    parse_outside_schemas, ClassDef, FieldDef, FieldType, ListItemType, ListTarget, ScalarType,
};
use crate::types::{DecodedFile, StructError, Value};

/// Encodes a `DecodedFile` into its canonical Struct text representation.
///
/// Calls `encode_with("", file)`.
pub fn encode(file: &DecodedFile) -> Result<String, StructError> {
    encode_with("", file)
}

/// Encodes a `DecodedFile` given schemas from outside the file.
///
/// An encoder given outside schemas writes no definitions for them.
/// With one outside class, its first block has no schema line, and any later block starts with `schema:name`.
/// With several outside classes, every block starts with `schema:name`.
pub fn encode_with(schemas: &str, file: &DecodedFile) -> Result<String, StructError> {
    let outside_classes = parse_outside_schemas(schemas)?;
    let num_outside = outside_classes.len();

    let mut all_schemas: BTreeMap<String, ClassDef> = BTreeMap::new();
    for (k, v) in &outside_classes {
        all_schemas.insert(k.clone(), v.clone());
    }
    for (k, v) in &file.schemas {
        all_schemas.insert(k.clone(), v.clone());
    }

    let mut out = String::new();
    let mut defined_in_doc: HashSet<String> = HashSet::new();

    let single_outside_name = if num_outside == 1 {
        outside_classes.keys().next().cloned()
    } else {
        None
    };

    for (block_idx, block) in file.blocks.iter().enumerate() {
        let class_def = match all_schemas.get(&block.class_name) {
            Some(def) => def,
            None => {
                return Err(StructError::new(
                    format!("schema definition for class '{}' not found", block.class_name),
                    0,
                    0,
                ));
            }
        };

        // Schema line emission rules
        if block_idx == 0 && num_outside == 1 && single_outside_name.as_deref() == Some(&block.class_name) {
            // First block of the single outside class: NO schema line!
        } else if outside_classes.contains_key(&block.class_name) {
            // Outside class (reused or one of several outside classes)
            out.push_str(&format!("schema:{}\n", block.class_name));
        } else if defined_in_doc.contains(&block.class_name) {
            // Reused in-doc class
            out.push_str(&format!("schema:{}\n", block.class_name));
        } else {
            // First definition of in-doc class
            out.push_str(&class_def.format_schema_line());
            defined_in_doc.insert(block.class_name.clone());
        }

        // Records
        for record in &block.records {
            encode_record_tree(record, class_def, &mut out)?;
        }
    }

    Ok(out)
}

/// Encodes a top-level record and all its hierarchical child lines.
fn encode_record_tree(
    record: &Value,
    class_def: &ClassDef,
    out: &mut String,
) -> Result<(), StructError> {
    if record.is_null() && class_def.has_list() {
        let field_name = class_def.first_list_field_name().unwrap_or_default();
        return Err(StructError::new(
            format!(
                "cannot encode null for record containing a list: class '{}', field '{}'",
                class_def.name, field_name
            ),
            0,
            0,
        ));
    }

    // 1. Encode depth-0 data line
    encode_record_columns(record, &class_def.fields, &class_def.name, true, out)?;
    out.push('\n');

    // 2. Encode child lines for any lists
    let rec_def = class_def.as_record_def();
    let list_targets = rec_def.find_lists();
    encode_child_lists_for_record(record, &list_targets, &class_def.name, 1, out)?;

    Ok(())
}

/// Recursively flattens columns of a record into `col1|col2|...|`.
fn encode_record_columns(
    record: &Value,
    fields: &[FieldDef],
    class_name: &str,
    is_start_of_line: bool,
    out: &mut String,
) -> Result<(), StructError> {
    match record {
        Value::Null => {
            for f in fields {
                if f.has_list() {
                    return Err(StructError::new(
                        format!(
                            "cannot encode null for record containing a list: class '{}', field '{}'",
                            class_name, f.name
                        ),
                        0,
                        0,
                    ));
                }
            }
            for f in fields {
                match &f.field_type {
                    FieldType::Scalar(_) | FieldType::Enum(_) => {
                        out.push_str("~|");
                    }
                    FieldType::Record(r) => {
                        for _ in 0..r.num_columns() {
                            out.push_str("~|");
                        }
                    }
                    FieldType::List(_) => {}
                }
            }
        }
        Value::Record(rec_fields) => {
            let mut first = is_start_of_line;
            for field_def in fields {
                let val_opt = rec_fields.iter().find(|(k, _)| k == &field_def.name).map(|(_, v)| v);

                match &field_def.field_type {
                    FieldType::List(_) => {
                        if let Some(val) = val_opt {
                            if val.is_null() {
                                return Err(StructError::new(
                                    format!(
                                        "cannot encode null where list belongs: class '{}', field '{}'",
                                        class_name, field_def.name
                                    ),
                                    0,
                                    0,
                                ));
                            }
                        } else {
                            return Err(StructError::new(
                                format!(
                                    "cannot encode null where list belongs: class '{}', field '{}'",
                                    class_name, field_def.name
                                ),
                                0,
                                0,
                            ));
                        }
                    }
                    FieldType::Record(sub_rec) => {
                        let sub_val = val_opt.unwrap_or(&Value::Null);
                        if sub_val.is_null() && sub_rec.has_list() {
                            return Err(StructError::new(
                                format!(
                                    "cannot encode null for record containing a list: class '{}', field '{}'",
                                    class_name, field_def.name
                                ),
                                0,
                                0,
                            ));
                        }
                        encode_record_columns(sub_val, &sub_rec.fields, class_name, first, out)?;
                        first = false;
                    }
                    FieldType::Scalar(st) => {
                        let val = val_opt.unwrap_or(&Value::Null);
                        encode_scalar_value(val, *st, first, out);
                        out.push('|');
                        first = false;
                    }
                    FieldType::Enum(labels) => {
                        let val = val_opt.unwrap_or(&Value::Null);
                        encode_enum_value(val, labels, first, out);
                        out.push('|');
                        first = false;
                    }
                }
            }
        }
        _ => {
            return Err(StructError::new(
                "expected Record or Null value for record row",
                0,
                0,
            ));
        }
    }
    Ok(())
}

/// Encodes child lines at a given depth for all lists on a record.
fn encode_child_lists_for_record(
    parent_val: &Value,
    list_targets: &[ListTarget],
    class_name: &str,
    depth: usize,
    out: &mut String,
) -> Result<(), StructError> {
    for target in list_targets {
        if let Some(items) = get_list_by_path(parent_val, &target.field_path) {
            for item in items {
                for _ in 0..depth {
                    out.push('>');
                }

                if !target.canonical_index.is_empty() {
                    out.push_str(&target.canonical_index);
                    out.push('|');
                }

                match &target.item_type {
                    ListItemType::Scalar(st) => {
                        encode_scalar_value(item, *st, target.canonical_index.is_empty(), out);
                        out.push_str("|\n");
                    }
                    ListItemType::Enum(labels) => {
                        encode_enum_value(item, labels, target.canonical_index.is_empty(), out);
                        out.push_str("|\n");
                    }
                    ListItemType::Record(rec_def) => {
                        if item.is_null() && rec_def.has_list() {
                            let field_name = rec_def.first_list_field_name().unwrap_or_default();
                            return Err(StructError::new(
                                format!(
                                    "cannot encode null for record containing a list: class '{}', field '{}'",
                                    class_name, field_name
                                ),
                                0,
                                0,
                            ));
                        }
                        encode_record_columns(
                            item,
                            &rec_def.fields,
                            class_name,
                            target.canonical_index.is_empty(),
                            out,
                        )?;
                        out.push('\n');

                        let sub_lists = rec_def.find_lists();
                        encode_child_lists_for_record(item, &sub_lists, class_name, depth + 1, out)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn get_list_by_path<'a>(val: &'a Value, path: &[usize]) -> Option<&'a [Value]> {
    match val {
        Value::Record(fields) => {
            let field_idx = path[0];
            if field_idx >= fields.len() {
                return None;
            }
            let (_name, child_val) = &fields[field_idx];
            if path.len() == 1 {
                if let Value::List(items) = child_val {
                    Some(items.as_slice())
                } else {
                    None
                }
            } else {
                get_list_by_path(child_val, &path[1..])
            }
        }
        _ => None,
    }
}

/// Encodes a scalar value.
fn encode_scalar_value(val: &Value, _st: ScalarType, is_first_col: bool, out: &mut String) {
    match val {
        Value::Null => out.push('~'),
        Value::Int(n) => out.push_str(&n.to_string()),
        Value::Float(f) => {
            if f.is_nan() {
                out.push_str("NaN");
            } else if f.is_infinite() {
                if f.is_sign_positive() {
                    out.push_str("inf");
                } else {
                    out.push_str("-inf");
                }
            } else if *f == 0.0 {
                if f.is_sign_negative() {
                    out.push_str("-0");
                } else {
                    out.push('0');
                }
            } else {
                let abs = f.abs();
                if abs >= 0.0001 && abs < 10000.0 {
                    out.push_str(&f.to_string());
                } else {
                    out.push_str(&format!("{:e}", f));
                }
            }
        }
        Value::Bool(b) => {
            if *b {
                out.push('1');
            } else {
                out.push('0');
            }
        }
        Value::String(s) => {
            encode_text(s, is_first_col, out);
        }
        Value::Enum(s) => {
            encode_text(s, is_first_col, out);
        }
        _ => out.push('~'),
    }
}

/// Encodes an enum value.
fn encode_enum_value(val: &Value, _labels: &[String], is_first_col: bool, out: &mut String) {
    match val {
        Value::Null => out.push('~'),
        Value::Enum(s) | Value::String(s) => encode_text(s, is_first_col, out),
        _ => out.push('~'),
    }
}

/// Encodes a text string according to Struct v0.3 canonical rules.
fn encode_text(s: &str, is_first_col: bool, out: &mut String) {
    if s.is_empty() {
        out.push('~');
        return;
    }

    if s == "~" {
        out.push_str(r"\~");
        return;
    }

    // SPEC.md §7: \r only when a text value contains a line break, or starts or ends with a space or tab.
    // Inside it, only | and \ are escaped.
    let needs_raw = s.contains('\n')
        || s.starts_with(' ')
        || s.starts_with('\t')
        || s.ends_with(' ')
        || s.ends_with('\t');

    if needs_raw {
        out.push_str(r"\r");
        for c in s.chars() {
            match c {
                '\\' => out.push_str(r"\\"),
                '|' => out.push_str(r"\|"),
                '\r' => {} // CRLF normalization: omit bare CR
                _ => out.push(c),
            }
        }
    } else {
        for (i, c) in s.chars().enumerate() {
            if i == 0 && is_first_col && c == '>' {
                out.push_str(r"\>");
                continue;
            }
            match c {
                '\\' => out.push_str(r"\\"),
                '|' => out.push_str(r"\|"),
                _ => out.push(c),
            }
        }
    }
}
