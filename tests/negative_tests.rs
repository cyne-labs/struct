//! Negative tests for Struct (v0.2).
//!
//! One test per rule in SPEC.md, each asserting the exact line,
//! the exact column, and the full exact message (not a substring).

use structl::{decode, decode_with, encode, Block, ClassDef, DecodedFile, FieldDef, FieldType, ListItemType, RecordDef, ScalarType, Value};

#[test]
fn test_error_data_line_before_schema_line() {
    // SPEC.md §1: "A data line that appears before any schema line is an error."
    let input = "Rope scaling hurts long ctx|2405.05254|2024|refuted|\n";
    let err = decode(input).expect_err("Must fail: data line before any schema line");
    assert_eq!(err.line, 1);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "data line appears before any schema line");
}

#[test]
fn test_error_reusing_undefined_class() {
    // SPEC.md §2: "Reusing a name that hasn't been defined yet is an error."
    let input = "schema:unknown_class\n";
    let err = decode(input).expect_err("Must fail: reusing undefined class");
    assert_eq!(err.line, 1);
    assert_eq!(err.column, 8);
    assert_eq!(err.message, "reusing class 'unknown_class' that hasn't been defined yet");
}

#[test]
fn test_error_defining_same_class_twice() {
    // SPEC.md §2: "Defining the same name twice is an error."
    let input = "schema:claim{text}\n\
                 schema:claim{text, year:int}\n";
    let err = decode(input).expect_err("Must fail: duplicate class definition");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 8);
    assert_eq!(err.message, "class 'claim' defined twice");
}

#[test]
fn test_error_invalid_class_name() {
    // SPEC.md §2: "Class names and field names are made of letters, digits and _, and don't start with a digit."
    let input = "schema:123claim{text}\n";
    let err = decode(input).expect_err("Must fail: invalid class name starting with digit");
    assert_eq!(err.line, 1);
    assert_eq!(err.column, 8);
    assert_eq!(err.message, "invalid class name '123claim'");
}

#[test]
fn test_error_invalid_field_name() {
    // SPEC.md §2: "Class names and field names are made of letters, digits and _, and don't start with a digit."
    let input = "schema:claim{123text}\n";
    let err = decode(input).expect_err("Must fail: invalid field name starting with digit");
    assert_eq!(err.line, 1);
    assert_eq!(err.column, 14);
    assert_eq!(err.message, "invalid field name '123text'");
}

#[test]
fn test_error_invalid_enum_label() {
    // SPEC.md §2: "Labels follow the same rule as names: letters, digits and _, no spaces, and they don't start with a digit."
    let input = "schema:claim{verdict:enum(1bad,ok)}\n";
    let err = decode(input).expect_err("Must fail: invalid enum label starting with digit");
    assert_eq!(err.line, 1);
    assert_eq!(err.column, 27);
    assert_eq!(err.message, "invalid enum label '1bad'");
}

#[test]
fn test_error_duplicate_field_name() {
    // SPEC.md §2: "Using the same field name twice inside one set of braces is an error."
    let input = "schema:claim{text,text:int}\n";
    let err = decode(input).expect_err("Must fail: duplicate field name in schema");
    assert_eq!(err.line, 1);
    assert_eq!(err.column, 19);
    assert_eq!(err.message, "duplicate field name 'text' in schema");
}

#[test]
fn test_error_unknown_type_name() {
    // SPEC.md §2: "A type name not in this list (such as date) is an error."
    let input = "schema:claim{created:date}\n";
    let err = decode(input).expect_err("Must fail: unknown type name");
    assert_eq!(err.line, 1);
    assert_eq!(err.column, 22);
    assert_eq!(err.message, "unknown or invalid type 'date'");
}

#[test]
fn test_error_schema_line_containing_pipe_is_data_line() {
    // SPEC.md §1: "If a line starts with schema: and also contains |, it is a data line (the schema: prefix belongs to its first value, and the line is not a schema line)."
    // Since no schema has been defined yet, it is a data line before any schema line.
    let input = "schema:claim{text|verdict}\n";
    let err = decode(input).expect_err("Must fail: data line before any schema line");
    assert_eq!(err.line, 1);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "data line appears before any schema line");
}

#[test]
fn test_error_schema_line_containing_pipe_in_outside_schemas() {
    // SPEC.md §1: "a schema line: it starts with schema:, and never contains |;"
    let err = decode_with("schema:claim{text|verdict}", "").expect_err("Must fail: schema line containing '|'");
    assert_eq!(err.line, 1);
    assert_eq!(err.column, 18);
    assert_eq!(err.message, "error in outside schemas: schema line must never contain '|'");
}

#[test]
fn test_error_data_line_not_ending_in_unescaped_pipe() {
    // SPEC.md §1: "a data line: it always ends with an unescaped |."
    let input = "schema:claim{text}\n\
                 hello\n";
    let err = decode(input).expect_err("Must fail: data line not ending in '|'");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 6);
    assert_eq!(err.message, "data line must always end with an unescaped '|'");
}

#[test]
fn test_error_too_few_columns_top_level() {
    // SPEC.md §3: "A data line has exactly one value per column of its class or list item. Too few or too many values is an error."
    let input = "schema:claim{text, year:int}\n\
                 only_one|\n";
    let err = decode(input).expect_err("Must fail: too few columns on top-level line");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "too few columns: expected 2, found 1");
}

#[test]
fn test_error_too_many_columns_top_level() {
    // SPEC.md §3: "A data line has exactly one value per column of its class or list item. Too few or too many values is an error."
    let input = "schema:claim{text, year:int}\n\
                 one|2024|three|\n";
    let err = decode(input).expect_err("Must fail: too many columns on top-level line");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "too many columns: expected 2, found 3");
}

#[test]
fn test_error_too_few_columns_child_line() {
    // SPEC.md §3: "A data line has exactly one value per column of its class or list item. Too few or too many values is an error."
    let input = "schema:claim{text:str, notes:list[record{a, b}]}\n\
                 claim 1|\n\
                 >only_one|\n";
    let err = decode(input).expect_err("Must fail: too few columns on child line");
    assert_eq!(err.line, 3);
    assert_eq!(err.column, 2);
    assert_eq!(err.message, "too few columns for child item: expected 2, found 1");
}

#[test]
fn test_error_too_many_columns_child_line() {
    // SPEC.md §3: "A data line has exactly one value per column of its class or list item. Too few or too many values is an error."
    let input = "schema:claim{text:str, notes:list[record{a, b}]}\n\
                 claim 1|\n\
                 >one|two|three|\n";
    let err = decode(input).expect_err("Must fail: too many columns on child line");
    assert_eq!(err.line, 3);
    assert_eq!(err.column, 2);
    assert_eq!(err.message, "too many columns for child item: expected 2, found 3");
}

#[test]
fn test_error_type_mismatch_int() {
    // SPEC.md §2: "Types are enforced. A value that doesn't match its type is an error.
    // For example, twenty twenty-four in an int field is an error."
    let input = "schema:claim{year:int}\n\
                 twenty twenty-four|\n";
    let err = decode(input).expect_err("Must fail: invalid int");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "type mismatch: 'twenty twenty-four' is not a valid int (i64)");
}

#[test]
fn test_error_type_mismatch_float() {
    // SPEC.md §2: "Types are enforced. A value that doesn't match its type is an error."
    let input = "schema:metric{score:float}\n\
                 not_a_float|\n";
    let err = decode(input).expect_err("Must fail: invalid float");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "type mismatch: 'not_a_float' is not a valid float (f64)");
}

#[test]
fn test_error_type_mismatch_bool() {
    // SPEC.md §2: "bool: 1 for true, 0 for false. Types are enforced."
    let input = "schema:claim{verified:bool}\n\
                 true|\n";
    let err = decode(input).expect_err("Must fail: bool must be 1 or 0");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "type mismatch: 'true' is not a valid bool (must be 1 or 0)");
}

#[test]
fn test_error_type_mismatch_enum() {
    // SPEC.md §2: "enum(a,b,c): exactly one of the listed labels. Types are enforced."
    let input = "schema:claim{verdict:enum(supported,refuted)}\n\
                 maybe|\n";
    let err = decode(input).expect_err("Must fail: value not in enum");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "type mismatch: value 'maybe' is not in enum(supported,refuted)");
}

#[test]
fn test_error_leading_plus_in_int() {
    // SPEC.md §2: "A leading + is an error."
    let input = "schema:claim{year:int}\n\
                 +5|\n";
    let err = decode(input).expect_err("Must fail: leading '+' on int");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "leading '+' is not allowed in int '+5'");
}

#[test]
fn test_error_int_out_of_i64_range() {
    // SPEC.md §2: "Its range is Rust's i64."
    let input = "schema:claim{year:int}\n\
                 99999999999999999999999|\n";
    let err = decode(input).expect_err("Must fail: int out of i64 range");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "integer '99999999999999999999999' is out of i64 range");
}

#[test]
fn test_error_empty_field() {
    // SPEC.md §3: "An empty field is always an error. There's no empty string. Blank text is written ~."
    let input = "schema:claim{text, year:int}\n\
                 |2024|\n";
    let err = decode(input).expect_err("Must fail: empty field");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "empty field is an error; blank text must be written as '~'");
}

#[test]
fn test_error_unknown_backslash_sequence() {
    // SPEC.md §3: "Any other backslash sequence is an error."
    let input = "schema:claim{text}\n\
                 hello\\qworld|\n";
    let err = decode(input).expect_err("Must fail: invalid escape");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 6);
    assert_eq!(err.message, "invalid escape sequence '\\q'");
}

#[test]
fn test_error_depth_rise_by_more_than_one() {
    // SPEC.md §5: "Depth can rise by only one level at a time; a depth-0 line followed directly by a >> line is an error."
    let input = "schema:claim{text, notes:list[record{text, subnotes:list[str]}]}\n\
                 Rope scaling hurts long ctx|\n\
                 >>abstract mentions limit|\n";
    let err = decode(input).expect_err("Must fail: depth increased by more than 1");
    assert_eq!(err.line, 3);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "depth increased by more than one level (from depth 0 to 2)");
}

#[test]
fn test_error_depth_rise_straight_after_schema() {
    // SPEC.md §5: A '>' line straight after the schema line is an error (depth rise > 1 from 0 items).
    let input = "schema:claim{notes:list[str]}\n\
                 >a note|\n";
    let err = decode(input).expect_err("Must fail: child line straight after schema");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "depth increased by more than one level (from depth 0 to 1)");
}

#[test]
fn test_error_child_line_under_parent_with_no_list() {
    // SPEC.md §5: "A child line whose parent has no list is an error."
    let input = "schema:claim{text}\n\
                 hello|\n\
                 >child|\n";
    let err = decode(input).expect_err("Must fail: child line under parent with no list");
    assert_eq!(err.line, 3);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "parent record has no list fields to attach child line");
}

#[test]
fn test_error_missing_index_when_needed() {
    // SPEC.md §5: "When an index is needed (rules 2 and 3), leaving it out is an error"
    let input = "schema:claim{text:str, a:list[str], b:list[str]}\n\
                 claim 1|\n\
                 >item|\n";
    let err = decode(input).expect_err("Must fail: missing index when 2 lists exist");
    assert_eq!(err.line, 3);
    assert_eq!(err.column, 2);
    assert_eq!(err.message, "missing list index when parent has multiple lists");
}

#[test]
fn test_error_index_names_non_list_field() {
    // SPEC.md §5: "an index that doesn't name one of the parent's lists is an error."
    let input = "schema:claim{title:str, a:list[str], b:list[str]}\n\
                 test|\n\
                 >0|item|\n";
    let err = decode(input).expect_err("Must fail: index 0 names title (non-list)");
    assert_eq!(err.line, 3);
    assert_eq!(err.column, 2);
    assert_eq!(err.message, "index '0' names a non-list field");
}

#[test]
fn test_error_index_out_of_range() {
    // SPEC.md §5: "an index that doesn't name one of the parent's lists is an error."
    let input = "schema:claim{text:str, a:list[str], b:list[str]}\n\
                 claim 1|\n\
                 >99|item|\n";
    let err = decode(input).expect_err("Must fail: index 99 is out of range");
    assert_eq!(err.line, 3);
    assert_eq!(err.column, 2);
    assert_eq!(err.message, "list index '99' is out of range");
}

#[test]
fn test_error_encoder_null_where_list_belongs() {
    // R1b 1.4: "Handing encode a Value::Null where a list belongs, or for a record that contains a list, is an error."
    let mut doc = DecodedFile::new();
    let class_def = ClassDef {
        name: "claim".to_string(),
        fields: vec![
            FieldDef {
                name: "notes".to_string(),
                field_type: FieldType::List(Box::new(ListItemType::Scalar(ScalarType::Str))),
            },
        ],
    };
    doc.schemas.insert("claim".to_string(), class_def);
    doc.blocks.push(Block {
        class_name: "claim".to_string(),
        records: vec![Value::Record(vec![("notes".to_string(), Value::Null)])],
    });

    let err = encode(&doc).expect_err("Must fail: null where list belongs");
    assert_eq!(err.line, 0);
    assert_eq!(err.column, 0);
    assert_eq!(err.message, "cannot encode null where list belongs: class 'claim', field 'notes'");
}

#[test]
fn test_error_encoder_null_for_record_containing_list() {
    // R1b 1.4: "Handing encode a Value::Null where a list belongs, or for a record that contains a list, is an error."
    let mut doc = DecodedFile::new();
    let class_def = ClassDef {
        name: "claim".to_string(),
        fields: vec![
            FieldDef {
                name: "info".to_string(),
                field_type: FieldType::Record(RecordDef {
                    fields: vec![
                        FieldDef {
                            name: "notes".to_string(),
                            field_type: FieldType::List(Box::new(ListItemType::Scalar(ScalarType::Str))),
                        },
                    ],
                }),
            },
        ],
    };
    doc.schemas.insert("claim".to_string(), class_def);
    doc.blocks.push(Block {
        class_name: "claim".to_string(),
        records: vec![Value::Record(vec![("info".to_string(), Value::Null)])],
    });

    let err = encode(&doc).expect_err("Must fail: null for record containing a list");
    assert_eq!(err.line, 0);
    assert_eq!(err.column, 0);
    assert_eq!(err.message, "cannot encode null for record containing a list: class 'claim', field 'info'");
}

#[test]
fn test_error_escape_n() {
    // SPEC.md §3: \n is no longer an escape sequence in v0.3
    let input = "schema:claim{text}\nhello\\nworld|\n";
    let err = decode(input).expect_err("Must fail: \\n escape");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 6);
    assert_eq!(err.message, "invalid escape sequence '\\n'");
}

#[test]
fn test_error_escape_s() {
    // SPEC.md §3: \s is no longer an escape sequence in v0.3
    let input = "schema:claim{text}\nhello\\sworld|\n";
    let err = decode(input).expect_err("Must fail: \\s escape");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 6);
    assert_eq!(err.message, "invalid escape sequence '\\s'");
}

#[test]
fn test_error_raw_in_int() {
    // SPEC.md §3: raw values are only valid for str and list[str]
    let input = "schema:claim{val:int}\n\\r42|\n";
    let err = decode(input).expect_err("Must fail: \\r in int field");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "raw value '\\r' is not allowed in int field");
}

#[test]
fn test_error_raw_in_float() {
    // SPEC.md §3: raw values are only valid for str and list[str]
    let input = "schema:claim{val:float}\n\\r3.14|\n";
    let err = decode(input).expect_err("Must fail: \\r in float field");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "raw value '\\r' is not allowed in float field");
}

#[test]
fn test_error_raw_in_bool() {
    // SPEC.md §3: raw values are only valid for str and list[str]
    let input = "schema:claim{val:bool}\n\\r1|\n";
    let err = decode(input).expect_err("Must fail: \\r in bool field");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "raw value '\\r' is not allowed in bool field");
}

#[test]
fn test_error_raw_in_enum() {
    // SPEC.md §3: raw values are only valid for str and list[str]
    let input = "schema:claim{val:enum(a,b)}\n\\ra|\n";
    let err = decode(input).expect_err("Must fail: \\r in enum field");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "raw value '\\r' is not allowed in enum field");
}

#[test]
fn test_error_empty_raw_value() {
    // SPEC.md §3: "An empty raw value (\r|) is an error."
    let input = "schema:claim{text}\n\\r|\n";
    let err = decode(input).expect_err("Must fail: empty raw value");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "empty raw value is an error");
}

#[test]
fn test_error_unknown_escape_in_raw_value() {
    // SPEC.md §3: Inside raw value, only \| and \\ are allowed
    let input = "schema:claim{text}\n\\rhello\\qworld|\n";
    let err = decode(input).expect_err("Must fail: invalid escape inside raw value");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 8);
    assert_eq!(err.message, "invalid escape sequence '\\q' inside raw value");
}

#[test]
fn test_error_raw_value_no_closing_pipe_at_eof() {
    // SPEC.md §3: raw value with no closing | before EOF
    let input = "schema:claim{text}\n\\runclosed";
    let err = decode(input).expect_err("Must fail: unclosed raw value");
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "raw value has no closing '|' before the end of the file");
}

#[test]
fn test_error_several_outside_classes_data_before_schema_line() {
    // SPEC.md §6: with several outside classes, data before first schema line is an error
    let schemas = "schema:c1{x}\nschema:c2{y}";
    let input = "foo|\n";
    let err = decode_with(schemas, input).expect_err("Must fail: data before schema line with multiple outside classes");
    assert_eq!(err.line, 1);
    assert_eq!(err.column, 1);
    assert_eq!(err.message, "with several outside classes, data before the first schema line is an error");
}

#[test]
fn test_error_redefining_outside_class() {
    // SPEC.md §6: redefining an outside class is an error
    let schemas = "schema:claim{text}";
    let input = "schema:claim{text, year:int}\nclaim 1|2024|\n";
    let err = decode_with(schemas, input).expect_err("Must fail: redefining outside class");
    assert_eq!(err.line, 1);
    assert_eq!(err.column, 8);
    assert_eq!(err.message, "class 'claim' defined twice");
}

#[test]
fn test_error_inside_schemas_argument() {
    // R2.md Part A.3: errors inside schemas report line, column and state it was in outside schemas
    let schemas = "schema:claim{text,text:int}";
    let input = "claim 1|\n";
    let err = decode_with(schemas, input).expect_err("Must fail: error inside schemas argument");
    assert_eq!(err.line, 1);
    assert_eq!(err.column, 19);
    assert_eq!(err.message, "error in outside schemas: duplicate field name 'text' in schema");
}
