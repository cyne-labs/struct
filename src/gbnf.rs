//! GBNF grammar generation for Struct (v0.4).
//!
//! Produces GBNF grammars enforcing canonical Struct output for local models.
//! Standard library only.

use crate::schema::{parse_outside_schemas, FieldDef, FieldType, ListItemType, ListTarget, ScalarType};
use crate::types::StructError;

/// Generates a full GBNF grammar for the given outside schemas.
///
/// SPEC.md §8: "The grammar is built from outside schemas: the model writes data only,
/// with `schema:name` lines to switch class when there are several. It allows only the
/// canonical form."
pub fn gbnf(schemas: &str) -> Result<String, StructError> {
    generate_gbnf_internal(schemas, None)
}

/// Generates a triggered GBNF grammar for the given outside schemas.
///
/// SPEC.md §8: "triggered: the model writes freely, for example while reasoning,
/// until a trigger text appears (</think> by default, and configurable).
/// From there on, the output must follow the grammar."
pub fn gbnf_triggered(schemas: &str, trigger: &str) -> Result<String, StructError> {
    if trigger.is_empty() {
        return Err(StructError::new("trigger cannot be empty", 0, 0));
    }
    generate_gbnf_internal(schemas, Some(trigger))
}

fn generate_gbnf_internal(schemas: &str, trigger: Option<&str>) -> Result<String, StructError> {
    let classes = parse_outside_schemas(schemas)?;
    if classes.is_empty() {
        return Err(StructError::new(
            "error in outside schemas: outside schemas must define at least one class",
            1,
            1,
        ));
    }

    let mut out = String::new();
    let num_outside = classes.len();

    // 1. Root rule
    if let Some(trig) = trigger {
        let escaped_trig = escape_gbnf_string(trig);
        if num_outside == 1 {
            let class_name = classes.keys().next().unwrap();
            out.push_str(&format!(
                "root ::= \"{}\" (\"\\n\" | \"\\r\\n\")? block-body-{}\n\n",
                escaped_trig, class_name
            ));
        } else {
            out.push_str(&format!(
                "root ::= \"{}\" (\"\\n\" | \"\\r\\n\")? doc-multi\n\n",
                escaped_trig
            ));
        }
    } else {
        if num_outside == 1 {
            let class_name = classes.keys().next().unwrap();
            out.push_str(&format!("root ::= block-body-{}\n\n", class_name));
        } else {
            out.push_str("root ::= doc-multi\n\n");
        }
    }

    if num_outside > 1 {
        let block_names: Vec<String> = classes
            .keys()
            .map(|name| format!("block-{}", name))
            .collect();
        out.push_str(&format!("doc-multi ::= ({})+\n\n", block_names.join(" | ")));
    }

    // 2. Class blocks and records
    for (class_name, class_def) in &classes {
        if num_outside > 1 {
            out.push_str(&format!(
                "block-{} ::= \"schema:{}\\n\" record-{}+\n",
                class_name, class_name, class_name
            ));
        }

        // Single outside class can have multiple blocks with schema:name\n for subsequent blocks
        out.push_str(&format!(
            "block-body-{} ::= record-{}+ (\"schema:{}\\n\" record-{}+)*\n",
            class_name, class_name, class_name, class_name
        ));

        // Record rule: depth-0 line + child lines
        let list_targets = class_def.find_lists();
        if list_targets.is_empty() {
            out.push_str(&format!(
                "record-{} ::= line-0-{}\n",
                class_name, class_name
            ));
            out.push_str(&format!("line-0-{} ::= ", class_name));
            let flat_cols = flatten_fields(&class_def.fields);
            for (i, col_type) in flat_cols.iter().enumerate() {
                let is_first = i == 0;
                let col_rule = column_type_rule_name(class_name, col_type, is_first, i);
                out.push_str(&format!("{} \"|\" ", col_rule));
            }
            out.push_str("\"\\n\"\n\n");
        } else {
            out.push_str(&format!(
                "record-{} ::= line-0-empty-{} | line-0-has-children-{} children-block-{}\n",
                class_name, class_name, class_name, class_name
            ));

            out.push_str(&format!("line-0-empty-{} ::= ", class_name));
            let flat_cols = flatten_fields(&class_def.fields);
            for (i, col_type) in flat_cols.iter().enumerate() {
                let is_first = i == 0;
                let col_rule = column_type_rule_name(class_name, col_type, is_first, i);
                out.push_str(&format!("{} \"|\" ", col_rule));
            }
            out.push_str("\";\\n\"\n");

            out.push_str(&format!("line-0-has-children-{} ::= ", class_name));
            for (i, col_type) in flat_cols.iter().enumerate() {
                let is_first = i == 0;
                let col_rule = column_type_rule_name(class_name, col_type, is_first, i);
                out.push_str(&format!("{} \"|\" ", col_rule));
            }
            out.push_str("\"\\n\"\n\n");

            generate_children_rules(class_name, &list_targets, 1, &mut out);
        }

        // Specific enum rules for this class
        generate_record_enum_rules(&format!("enum-{}", class_name), &class_def.fields, &mut out);
    }

    // 3. Shared canonical primitive rules
    out.push_str(SHARED_PRIMITIVES);

    Ok(out)
}

fn generate_children_rules(
    class_name: &str,
    list_targets: &[ListTarget],
    depth: usize,
    out: &mut String,
) {
    let mut open_options = Vec::new();
    let mut last_options = Vec::new();

    for (t_idx, target) in list_targets.iter().enumerate() {
        let rule_open = format!("child-{}-d{}-t{}-open", class_name, depth, t_idx);
        let rule_last = format!("child-{}-d{}-t{}-last", class_name, depth, t_idx);
        open_options.push(rule_open.clone());
        last_options.push(rule_last.clone());

        // Child rule: '>' x depth, optional canonical index, columns, newline, nested children
        let depth_str = ">".repeat(depth);
        let mut prefix = format!("\"{}\" ", depth_str);
        if !target.canonical_index.is_empty() {
            prefix.push_str(&format!("\"{}|\" ", target.canonical_index));
        }

        match &target.item_type {
            ListItemType::Scalar(st) => {
                let col_rule = match st {
                    ScalarType::Str => {
                        if target.canonical_index.is_empty() {
                            "col-first-str"
                        } else {
                            "col-str"
                        }
                    }
                    ScalarType::Int => "val-int",
                    ScalarType::Float => "val-float",
                    ScalarType::Bool => "val-bool",
                };
                out.push_str(&format!("{} ::= {}{} \"|\\n\"\n", rule_open, prefix, col_rule));
                out.push_str(&format!("{} ::= {}{} \"|;\\n\"\n", rule_last, prefix, col_rule));
            }
            ListItemType::Enum(labels) => {
                let enum_rule = format!("enum-child-{}-d{}-t{}", class_name, depth, t_idx);
                let label_alts: Vec<String> = labels.iter().map(|l| format!("\"{}\"", l)).collect();
                out.push_str(&format!("{} ::= \"null\" | {}\n", enum_rule, label_alts.join(" | ")));

                out.push_str(&format!("{} ::= {}{} \"|\\n\"\n", rule_open, prefix, enum_rule));
                out.push_str(&format!("{} ::= {}{} \"|;\\n\"\n", rule_last, prefix, enum_rule));
            }
            ListItemType::Record(rec) => {
                let flat_cols = flatten_fields(&rec.fields);
                let child_enum_prefix = format!("enum-{}-d{}-t{}", class_name, depth, t_idx);
                generate_record_enum_rules(&child_enum_prefix, &rec.fields, out);

                let mut cols_str = String::new();
                for (ci, ct) in flat_cols.iter().enumerate() {
                    let is_first = ci == 0 && target.canonical_index.is_empty();
                    let col_rule = match ct {
                        FlatColType::Scalar(st) => {
                            if *st == ScalarType::Str {
                                if is_first {
                                    "col-first-str".to_string()
                                } else {
                                    "col-str".to_string()
                                }
                            } else {
                                column_type_rule_name(class_name, ct, is_first, 0)
                            }
                        }
                        FlatColType::Enum(id) => format!("{}-{}", child_enum_prefix, id),
                    };
                    cols_str.push_str(&format!("{} \"|\" ", col_rule));
                }

                let sub_lists = rec.find_lists();
                if sub_lists.is_empty() {
                    out.push_str(&format!("{} ::= {}{}\"\\n\"\n", rule_open, prefix, cols_str));
                    out.push_str(&format!("{} ::= {}{}\";\\n\"\n", rule_last, prefix, cols_str));
                } else {
                    let nested_block = format!("children-block-{}-d{}", class_name, depth + 1);
                    out.push_str(&format!(
                        "{} ::= {}{}\";\\n\" | {}{}\"\\n\" {}\n",
                        rule_open, prefix, cols_str, prefix, cols_str, nested_block
                    ));
                    out.push_str(&format!(
                        "{} ::= {}{}\";\\n\" | {}{}\"\\n\" {}\n",
                        rule_last, prefix, cols_str, prefix, cols_str, nested_block
                    ));
                    generate_children_rules(class_name, &sub_lists, depth + 1, out);
                }
            }
        }
    }

    let block_name = if depth == 1 {
        format!("children-block-{}", class_name)
    } else {
        format!("children-block-{}-d{}", class_name, depth)
    };

    out.push_str(&format!(
        "{} ::= ({})* ({})\n",
        block_name,
        open_options.join(" | "),
        last_options.join(" | ")
    ));
}

fn generate_record_enum_rules(prefix: &str, fields: &[FieldDef], out: &mut String) {
    let mut enum_counter = 0;
    generate_record_enum_rules_internal(prefix, fields, &mut enum_counter, out);
}

fn generate_record_enum_rules_internal(
    prefix: &str,
    fields: &[FieldDef],
    counter: &mut usize,
    out: &mut String,
) {
    for f in fields {
        match &f.field_type {
            FieldType::Enum(labels) => {
                let id = *counter;
                *counter += 1;
                let alts: Vec<String> = labels.iter().map(|l| format!("\"{}\"", l)).collect();
                out.push_str(&format!(
                    "{}-{} ::= \"null\" | {}\n",
                    prefix, id, alts.join(" | ")
                ));
            }
            FieldType::Record(r) => {
                generate_record_enum_rules_internal(prefix, &r.fields, counter, out);
            }
            _ => {}
        }
    }
}

enum FlatColType {
    Scalar(ScalarType),
    Enum(usize),
}

fn flatten_fields(fields: &[FieldDef]) -> Vec<FlatColType> {
    let mut out = Vec::new();
    flatten_internal(fields, &mut out, &mut 0);
    out
}

fn flatten_internal(
    fields: &[FieldDef],
    out: &mut Vec<FlatColType>,
    enum_counter: &mut usize,
) {
    for f in fields {
        match &f.field_type {
            FieldType::Scalar(st) => out.push(FlatColType::Scalar(*st)),
            FieldType::Enum(_) => {
                let id = *enum_counter;
                *enum_counter += 1;
                out.push(FlatColType::Enum(id));
            }
            FieldType::Record(r) => flatten_internal(&r.fields, out, enum_counter),
            FieldType::List(_) => {}
        }
    }
}

fn column_type_rule_name(
    class_name: &str,
    col: &FlatColType,
    is_first: bool,
    _enum_id: usize,
) -> String {
    match col {
        FlatColType::Scalar(ScalarType::Str) => {
            if is_first {
                "col-first-str".to_string()
            } else {
                "col-str".to_string()
            }
        }
        FlatColType::Scalar(ScalarType::Int) => "val-int".to_string(),
        FlatColType::Scalar(ScalarType::Float) => "val-float".to_string(),
        FlatColType::Scalar(ScalarType::Bool) => "val-bool".to_string(),
        FlatColType::Enum(id) => format!("enum-{}-{}", class_name, id),
    }
}

fn escape_gbnf_string(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str(r#"\""#),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            '\t' => out.push_str(r"\t"),
            _ => out.push(c),
        }
    }
    out
}

const SHARED_PRIMITIVES: &str = r#"
# --- Canonical Struct v0.4 Primitives ---

val-bool ::= "true" | "false" | "null"

val-int ::= "null" | "0" | "-"? [1-9] [0-9]? [0-9]? [0-9]? [0-9]? [0-9]? [0-9]? [0-9]? [0-9]? [0-9]? [0-9]? [0-9]? [0-9]? [0-9]? [0-9]? [0-9]? [0-9]? [0-9]? [0-9]?

val-float ::= "null" | "NaN" | "inf" | "-inf" | "-0" | "0" | float-plain | float-exp

float-plain ::= "-"? ( [1-9] [0-9]? [0-9]? [0-9]? ( "." [0-9]* [1-9] )? | "0." ( "000" [1-9] [0-9]* | "00" [1-9] [0-9]* | "0" [1-9] [0-9]* | [1-9] [0-9]* ) )

float-exp ::= "-"? [1-9] ( "." [0-9]* [1-9] )? ( "e" ( [4-9] | [1-9] [0-9]+ ) | "e-" ( [5-9] | [1-9] [0-9]+ ) )

col-first-str ::= "null" | "\\rnull" | str-raw | "\\>" ( str-mid* str-last )? | str-first-char ( str-mid* str-last )?

col-str ::= "null" | "\\rnull" | str-raw | str-col-first-char ( str-mid* str-last )?

str-raw ::= "\\r" ( raw-char* "\n" raw-char* | (" " | "\t") raw-char* | raw-char* (" " | "\t") )

raw-char ::= [^\\|\r] | "\\|" | "\\\\"

str-first-char ::= [^ \t\r\n\\|>] | "\\|" | "\\\\"

str-col-first-char ::= [^ \t\r\n\\|] | "\\|" | "\\\\"

str-mid ::= [^\r\n\\|] | "\\|" | "\\\\"

str-last ::= [^ \t\r\n\\|] | "\\|" | "\\\\"
"#;
