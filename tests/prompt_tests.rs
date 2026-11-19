//! Tests for the prompt builder (SPEC.md §10, R3c Part A.2).
//!
//! Standard library only.

use structl::prompt;

// =========================================================================
// 1. One test per conditional section, both included and left out (10 tests)
// =========================================================================

#[test]
fn test_conditional_section_record_field_included() {
    let schema = "schema:claim{text, source:record{id, year:int}, verified:bool}";
    let p = prompt(schema).expect("prompt generation failed");
    assert!(
        p.contains("A record{...} field adds its fields as extra columns, in order.\n"),
        "Prompt should contain record rule when schema has record field"
    );
}

#[test]
fn test_conditional_section_record_field_left_out() {
    let schema = "schema:claim{id, title, year:int, verified:bool}";
    let p = prompt(schema).expect("prompt generation failed");
    assert!(
        !p.contains("record{...}"),
        "Prompt should omit record rule when schema has no record field"
    );
}

#[test]
fn test_conditional_section_list_included() {
    let schema = "schema:claim{text, notes:list[str]}";
    let p = prompt(schema).expect("prompt generation failed");
    assert!(
        p.contains("List items go on the lines below their parent, starting with one > per level: > for its items, >> for theirs.\n"),
        "Prompt should contain list rule when schema has a list"
    );
}

#[test]
fn test_conditional_section_list_left_out() {
    let schema = "schema:claim{text, count:int}";
    let p = prompt(schema).expect("prompt generation failed");
    assert!(
        !p.contains("List items go on the lines below"),
        "Prompt should omit list rule when schema has no list"
    );
}

#[test]
fn test_conditional_section_parent_two_or_more_lists_included() {
    let schema = "schema:claim{text, notes:list[str], tags:list[str]}";
    let p = prompt(schema).expect("prompt generation failed");
    assert!(
        p.contains("When a parent has several lists, start each item with the list's field number and |, e.g. >2|item|. Fields count from 0 in schema order.\n"),
        "Prompt should contain multiple lists rule when parent has 2+ lists"
    );
}

#[test]
fn test_conditional_section_parent_two_or_more_lists_left_out() {
    // 1 list on parent: should NOT include 2+ lists rule
    let schema = "schema:claim{text, notes:list[str]}";
    let p = prompt(schema).expect("prompt generation failed");
    assert!(
        !p.contains("When a parent has several lists"),
        "Prompt should omit multiple lists rule when parent has only 1 list"
    );
}

#[test]
fn test_conditional_section_text_field_included() {
    let schema = "schema:entry{title, count:int}";
    let p = prompt(schema).expect("prompt generation failed");
    assert!(
        p.contains("For text with line breaks, or with spaces or tabs at its start or end, start the value with \\r, then write it exactly as it is, keeping every space, tab, indent and line break, until the closing |.\n"),
        "Prompt should contain text rule when schema has a str field"
    );
}

#[test]
fn test_conditional_section_text_field_left_out() {
    // No str field
    let schema = "schema:entry{count:int, ratio:float, flag:bool}";
    let p = prompt(schema).expect("prompt generation failed");
    assert!(
        !p.contains("For text with line breaks"),
        "Prompt should omit text rule when schema has no str field"
    );
}

#[test]
fn test_conditional_section_two_or_more_classes_included() {
    let schema = "schema:file{name, path}\nschema:patch{file_name, code, line:int}";
    let p = prompt(schema).expect("prompt generation failed");
    assert!(
        p.contains("Start each block with schema:name on its own line.\n"),
        "Prompt should contain multi-class rule when schema has 2+ classes"
    );
}

#[test]
fn test_conditional_section_two_or_more_classes_left_out() {
    let schema = "schema:file{name, path}";
    let p = prompt(schema).expect("prompt generation failed");
    assert!(
        !p.contains("Start each block with schema:name"),
        "Prompt should omit multi-class rule when schema has only 1 class"
    );
}

// =========================================================================
// 2. Byte-for-byte tests of full prompt for the four schemas in bench/r2/schemas/
// =========================================================================

#[test]
fn test_byte_for_byte_prompt_s1_flat() {
    let schema_file = include_str!("../bench/r2/schemas/s1_flat.stc");
    let generated = prompt(schema_file).expect("prompt failed for s1_flat");

    let expected = "\
schema:claim{id, title, verdict:enum(supported,refuted,unverified), year:int, verified:bool}\n\
Reply with data only in Struct format: no prose, no code fences.\n\
Never write a header row or column names. The first line is already data.\n\
One record per line: values in schema order, separated by |, and every line ends with |.\n\
~ means null. bool is 1 or 0. enum values must be one of the listed labels exactly.\n\
Inside a value, write \\| for | and \\ for \\.\n\
For text with line breaks, or with spaces or tabs at its start or end, start the value with \\r, then write it exactly as it is, keeping every space, tab, indent and line break, until the closing |.\n\
Example: text|text|supported|7|~|\n";

    assert_eq!(generated, expected);
}

#[test]
fn test_byte_for_byte_prompt_s2_nested_fixed() {
    let schema_file = include_str!("../bench/r2/schemas/s2_nested_fixed.stc");
    let generated = prompt(schema_file).expect("prompt failed for s2_nested_fixed");

    let expected = "\
schema:claim{text, verdict:enum(supported,refuted,unverified), source:record{id, year:int}, verified:bool}\n\
Reply with data only in Struct format: no prose, no code fences.\n\
Never write a header row or column names. The first line is already data.\n\
One record per line: values in schema order, separated by |, and every line ends with |.\n\
~ means null. bool is 1 or 0. enum values must be one of the listed labels exactly.\n\
Inside a value, write \\| for | and \\ for \\.\n\
A record{...} field adds its fields as extra columns, in order.\n\
For text with line breaks, or with spaces or tabs at its start or end, start the value with \\r, then write it exactly as it is, keeping every space, tab, indent and line break, until the closing |.\n\
Example: text|supported|text|7|~|\n";

    assert_eq!(generated, expected);
}

#[test]
fn test_byte_for_byte_prompt_s3_nested_variable() {
    let schema_file = include_str!("../bench/r2/schemas/s3_nested_variable.stc");
    let generated = prompt(schema_file).expect("prompt failed for s3_nested_variable");

    let expected = "\
schema:claim{text, verdict:enum(supported,refuted,unknown), source:record{id, year:int}, notes:list[record{text, subnotes:list[str]}]}\n\
Reply with data only in Struct format: no prose, no code fences.\n\
Never write a header row or column names. The first line is already data.\n\
One record per line: values in schema order, separated by |, and every line ends with |.\n\
~ means null. bool is 1 or 0. enum values must be one of the listed labels exactly.\n\
Inside a value, write \\| for | and \\ for \\.\n\
A record{...} field adds its fields as extra columns, in order.\n\
List items go on the lines below their parent, starting with one > per level: > for its items, >> for theirs.\n\
For text with line breaks, or with spaces or tabs at its start or end, start the value with \\r, then write it exactly as it is, keeping every space, tab, indent and line break, until the closing |.\n\
Example: text|supported|text|~|\n";

    assert_eq!(generated, expected);
}

#[test]
fn test_byte_for_byte_prompt_s4_two_class_code() {
    let schema_file = include_str!("../bench/r2/schemas/s4_two_class_code.stc");
    let generated = prompt(schema_file).expect("prompt failed for s4_two_class_code");

    let expected = "\
schema:file{name, path}\n\
schema:patch{file_name, code, line:int}\n\
Reply with data only in Struct format: no prose, no code fences.\n\
Never write a header row or column names. The first line is already data.\n\
One record per line: values in schema order, separated by |, and every line ends with |.\n\
~ means null. bool is 1 or 0. enum values must be one of the listed labels exactly.\n\
Inside a value, write \\| for | and \\ for \\.\n\
For text with line breaks, or with spaces or tabs at its start or end, start the value with \\r, then write it exactly as it is, keeping every space, tab, indent and line break, until the closing |.\n\
Start each block with schema:name on its own line.\n\
Example: text|~|\n";

    assert_eq!(generated, expected);
}
