//! Synthetic tests for Struct Doctor (R3 Part B).
//!
//! Standard library only.

mod common;

use structl::{
    doctor, splice, BadLine, Block, DecodedFile, DoctorMode, DoctorOptions,
    Repair, SafeFix, Value,
};

// =========================================================================
// 1. One test per safe fix (6 in all), at least 3 cases each
// =========================================================================

#[test]
fn test_safe_fix_1_code_fences() {
    let schema = "schema:claim{id, title, verdict:enum(supported,refuted), year:int, verified:bool}";
    let options = DoctorOptions::default();

    // Case 1: Simple backtick fences ``` wrapping data
    let input1 = "```\nc1|Attention|supported|2017|1|\n```\n";
    let report1 = doctor(schema, input1, &options);
    assert_eq!(report1.text, "c1|Attention|supported|2017|1|\n");
    assert_eq!(report1.repairs.len(), 2);
    assert_eq!(report1.repairs[0].fix, SafeFix::CodeFences);
    assert_eq!(report1.repairs[0].line, 1);
    assert_eq!(report1.repairs[0].original, "```");
    assert_eq!(report1.repairs[0].new_text, "");
    assert_eq!(report1.repairs[1].fix, SafeFix::CodeFences);
    assert_eq!(report1.repairs[1].line, 3);
    assert_eq!(report1.repairs[1].original, "```");
    assert_eq!(report1.repairs[1].new_text, "");
    assert!(report1.bad_lines.is_empty());

    // Case 2: Language tagged fences ```stc and ```struct
    let input2 = "```stc\nc2|FlashAttention|supported|2022|1|\n```\n";
    let report2 = doctor(schema, input2, &options);
    assert_eq!(report2.text, "c2|FlashAttention|supported|2022|1|\n");
    assert_eq!(report2.repairs.len(), 2);
    assert_eq!(report2.repairs[0].fix, SafeFix::CodeFences);
    assert_eq!(report2.repairs[0].line, 1);
    assert_eq!(report2.repairs[0].original, "```stc");
    assert_eq!(report2.repairs[1].fix, SafeFix::CodeFences);
    assert_eq!(report2.repairs[1].line, 3);
    assert!(report2.bad_lines.is_empty());

    // Case 3: Leading spaces before code fences
    let input3 = "   ```\nc3|Llama 405B|refuted|2024|0|\n  ```  \n";
    let report3 = doctor(schema, input3, &options);
    assert_eq!(report3.text, "c3|Llama 405B|refuted|2024|0|\n");
    assert_eq!(report3.repairs.len(), 2);
    assert_eq!(report3.repairs[0].fix, SafeFix::CodeFences);
    assert_eq!(report3.repairs[0].line, 1);
    assert_eq!(report3.repairs[1].fix, SafeFix::CodeFences);
    assert_eq!(report3.repairs[1].line, 3);
    assert!(report3.bad_lines.is_empty());
}

#[test]
fn test_safe_fix_2_echoed_schema() {
    let schema = "schema:claim{id, title, verdict:enum(supported,refuted), year:int, verified:bool}";
    let options = DoctorOptions::default();

    // Case 1: Exact echoed schema definition
    let input1 = "schema:claim{id, title, verdict:enum(supported,refuted), year:int, verified:bool}\nc1|Attention|supported|2017|1|\n";
    let report1 = doctor(schema, input1, &options);
    assert_eq!(report1.text, "c1|Attention|supported|2017|1|\n");
    assert_eq!(report1.repairs.len(), 1);
    assert_eq!(report1.repairs[0].fix, SafeFix::EchoedSchema);
    assert_eq!(report1.repairs[0].line, 1);
    assert_eq!(report1.repairs[0].new_text, "");
    assert!(report1.bad_lines.is_empty());

    // Case 2: Echoed schema with different spacing
    let input2 = "schema:claim { id, title, verdict:enum(supported, refuted), year:int, verified:bool }\nc2|FlashAttention|supported|2022|1|\n";
    let report2 = doctor(schema, input2, &options);
    assert_eq!(report2.text, "c2|FlashAttention|supported|2022|1|\n");
    assert_eq!(report2.repairs.len(), 1);
    assert_eq!(report2.repairs[0].fix, SafeFix::EchoedSchema);
    assert_eq!(report2.repairs[0].line, 1);
    assert!(report2.bad_lines.is_empty());

    // Case 3: Echoed schema with no spaces around commas
    let input3 = "schema:claim{id,title,verdict:enum(supported,refuted),year:int,verified:bool}\nc3|Llama 405B|refuted|2024|0|\n";
    let report3 = doctor(schema, input3, &options);
    assert_eq!(report3.text, "c3|Llama 405B|refuted|2024|0|\n");
    assert_eq!(report3.repairs.len(), 1);
    assert_eq!(report3.repairs[0].fix, SafeFix::EchoedSchema);
    assert_eq!(report3.repairs[0].line, 1);
    assert!(report3.bad_lines.is_empty());
}

#[test]
fn test_safe_fix_3_chatter() {
    let schema = "schema:claim{id, title, verdict:enum(supported,refuted), year:int, verified:bool}";
    let options = DoctorOptions::default();

    // Case 1: Leading chatter
    let input1 = "Here is the result you requested:\nc1|Attention|supported|2017|1|\n";
    let report1 = doctor(schema, input1, &options);
    assert_eq!(report1.text, "c1|Attention|supported|2017|1|\n");
    assert_eq!(report1.repairs.len(), 1);
    assert_eq!(report1.repairs[0].fix, SafeFix::Chatter);
    assert_eq!(report1.repairs[0].line, 1);
    assert_eq!(report1.repairs[0].original, "Here is the result you requested:");
    assert_eq!(report1.repairs[0].new_text, "");
    assert!(report1.bad_lines.is_empty());

    // Case 2: Trailing chatter
    let input2 = "c2|FlashAttention|supported|2022|1|\nI hope this helps! Let me know if you need anything else.\n";
    let report2 = doctor(schema, input2, &options);
    assert_eq!(report2.text, "c2|FlashAttention|supported|2022|1|\n");
    assert_eq!(report2.repairs.len(), 1);
    assert_eq!(report2.repairs[0].fix, SafeFix::Chatter);
    assert_eq!(report2.repairs[0].line, 2);
    assert_eq!(report2.repairs[0].original, "I hope this helps! Let me know if you need anything else.");
    assert!(report2.bad_lines.is_empty());

    // Case 3: Both leading and multiple trailing chatter lines
    let input3 = "Sure, here are the records:\nc3|Llama 405B|refuted|2024|0|\nNote that claim 3 is false.\nEnd of output.\n";
    let report3 = doctor(schema, input3, &options);
    assert_eq!(report3.text, "c3|Llama 405B|refuted|2024|0|\n");
    assert_eq!(report3.repairs.len(), 3);
    assert_eq!(report3.repairs[0].fix, SafeFix::Chatter);
    assert_eq!(report3.repairs[0].line, 1);
    assert_eq!(report3.repairs[1].fix, SafeFix::Chatter);
    assert_eq!(report3.repairs[1].line, 3);
    assert_eq!(report3.repairs[2].fix, SafeFix::Chatter);
    assert_eq!(report3.repairs[2].line, 4);
    assert!(report3.bad_lines.is_empty());
}

#[test]
fn test_safe_fix_4_header_row() {
    let schema = "schema:claim{id, title, verdict:enum(supported,refuted), year:int, verified:bool}";
    let options = DoctorOptions::default();

    // Case 1: Exact column names as header row
    let input1 = "id|title|verdict|year|verified|\nc1|Attention|supported|2017|1|\n";
    let report1 = doctor(schema, input1, &options);
    assert_eq!(report1.text, "c1|Attention|supported|2017|1|\n");
    assert_eq!(report1.repairs.len(), 1);
    assert_eq!(report1.repairs[0].fix, SafeFix::HeaderRow);
    assert_eq!(report1.repairs[0].line, 1);
    assert_eq!(report1.repairs[0].original, "id|title|verdict|year|verified|");
    assert_eq!(report1.repairs[0].new_text, "");
    assert!(report1.bad_lines.is_empty());

    // Case 2: Header row with spaces around pipes
    let input2 = "id | title | verdict | year | verified |\nc2|FlashAttention|supported|2022|1|\n";
    let report2 = doctor(schema, input2, &options);
    assert_eq!(report2.text, "c2|FlashAttention|supported|2022|1|\n");
    assert_eq!(report2.repairs.len(), 1);
    assert_eq!(report2.repairs[0].fix, SafeFix::HeaderRow);
    assert_eq!(report2.repairs[0].line, 1);
    assert!(report2.bad_lines.is_empty());

    // Case 3: Header row in a nested record schema
    let nested_schema = "schema:claim{text, source:record{id, year:int}, verdict:enum(supported,refuted)}";
    let input3 = "text|id|year|verdict|\nlinear attention|arxiv.123|2020|supported|\n";
    let report3 = doctor(nested_schema, input3, &options);
    assert_eq!(report3.text, "linear attention|arxiv.123|2020|supported|\n");
    assert_eq!(report3.repairs.len(), 1);
    assert_eq!(report3.repairs[0].fix, SafeFix::HeaderRow);
    assert_eq!(report3.repairs[0].line, 1);
    assert!(report3.bad_lines.is_empty());
}

#[test]
fn test_safe_fix_5_missing_final_pipe() {
    let schema = "schema:claim{id, title, verdict:enum(supported,refuted), year:int, verified:bool}";
    let options = DoctorOptions::default();

    // Case 1: Single line missing final pipe
    let input1 = "c1|Attention|supported|2017|1\n";
    let report1 = doctor(schema, input1, &options);
    assert_eq!(report1.text, "c1|Attention|supported|2017|1|\n");
    assert_eq!(report1.repairs.len(), 1);
    assert_eq!(report1.repairs[0].fix, SafeFix::MissingFinalPipe);
    assert_eq!(report1.repairs[0].line, 1);
    assert_eq!(report1.repairs[0].original, "c1|Attention|supported|2017|1");
    assert_eq!(report1.repairs[0].new_text, "c1|Attention|supported|2017|1|");
    assert!(report1.bad_lines.is_empty());

    // Case 2: Multiple lines, some missing final pipe, some with trailing spaces
    let input2 = "c1|Attention|supported|2017|1 \nc2|FlashAttention|supported|2022|1|\n";
    let report2 = doctor(schema, input2, &options);
    assert_eq!(report2.text, "c1|Attention|supported|2017|1|\nc2|FlashAttention|supported|2022|1|\n");
    assert_eq!(report2.repairs.len(), 1);
    assert_eq!(report2.repairs[0].fix, SafeFix::MissingFinalPipe);
    assert_eq!(report2.repairs[0].line, 1);
    assert!(report2.bad_lines.is_empty());

    // Case 3: Line with raw text containing escaped pipe, missing final pipe
    let input3 = "c3|Text with \\| pipe|refuted|2024|0\n";
    let report3 = doctor(schema, input3, &options);
    assert_eq!(report3.text, "c3|Text with \\| pipe|refuted|2024|0|\n");
    assert_eq!(report3.repairs.len(), 1);
    assert_eq!(report3.repairs[0].fix, SafeFix::MissingFinalPipe);
    assert_eq!(report3.repairs[0].line, 1);
    assert!(report3.bad_lines.is_empty());
}

#[test]
fn test_safe_fix_6_cut_off_last_record() {
    let schema = "schema:claim{id, title, verdict:enum(supported,refuted), year:int, verified:bool}";
    let options = DoctorOptions::default();

    // Case 1: Incomplete record with too few columns at EOF
    let input1 = "c1|Attention|supported|2017|1|\nc2|FlashAttention|\n";
    let report1 = doctor(schema, input1, &options);
    assert_eq!(report1.text, "c1|Attention|supported|2017|1|\n");
    assert_eq!(report1.repairs.len(), 1);
    assert_eq!(report1.repairs[0].fix, SafeFix::CutOffLastRecord);
    assert_eq!(report1.repairs[0].line, 2);
    assert_eq!(report1.repairs[0].original, "c2|FlashAttention|");
    assert_eq!(report1.repairs[0].new_text, "");
    assert!(report1.bad_lines.is_empty());

    // Case 2: Incomplete record without pipe at EOF
    let input2 = "c1|Attention|supported|2017|1|\nc2|FlashAtten\n";
    let report2 = doctor(schema, input2, &options);
    assert_eq!(report2.text, "c1|Attention|supported|2017|1|\n");
    assert_eq!(report2.repairs.len(), 1);
    assert_eq!(report2.repairs[0].fix, SafeFix::CutOffLastRecord);
    assert_eq!(report2.repairs[0].line, 2);
    assert!(report2.bad_lines.is_empty());

    // Case 3: Incomplete record with unclosed raw value at EOF
    let code_schema = "schema:patch{file, code}";
    let input3 = "main.rs|code1|\nlib.rs|\\rfn incomplete() {\n";
    let report3 = doctor(code_schema, input3, &options);
    assert_eq!(report3.text, "main.rs|code1|\n");
    assert_eq!(report3.repairs.len(), 1);
    assert_eq!(report3.repairs[0].fix, SafeFix::CutOffLastRecord);
    assert_eq!(report3.repairs[0].line, 2);
    assert!(report3.bad_lines.is_empty());
}

// =========================================================================
// 2. One test per kind of bad line, checking that nothing is edited
// =========================================================================

#[test]
fn test_bad_lines_coverage() {
    let schema = "schema:item{name:str, count:int, price:float, active:bool, rating:enum(poor,fair,good), notes:list[str]}";
    let options = DoctorOptions {
        mode: DoctorMode::Strict,
        retry_hint: true,
    };

    // 1. Wrong column count (too few) in middle of file
    let input = "item1|10|2.5|1|good|\nitem2|20|\nitem3|30|5.0|0|fair|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 2,
            column: 1,
            problem: "expected 5 columns, found 2".to_string(),
            original: "item2|20|".to_string(),
            dropped_as_child: false,
        }
    );

    // 2. Wrong column count (too many)
    let input = "item1|10|2.5|1|good|extra_field|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 1,
            problem: "expected 5 columns, found 6".to_string(),
            original: "item1|10|2.5|1|good|extra_field|".to_string(),
            dropped_as_child: false,
        }
    );

    // 3. Unescaped | inside text
    let input = "item|1|10|2.5|1|good|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 1,
            problem: "expected 5 columns, found 6".to_string(),
            original: "item|1|10|2.5|1|good|".to_string(),
            dropped_as_child: false,
        }
    );

    // 4. Enum typo
    let input = "item1|10|2.5|1|godd|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 16,
            problem: "'godd' is not one of: poor, fair, good".to_string(),
            original: "item1|10|2.5|1|godd|".to_string(),
            dropped_as_child: false,
        }
    );

    // 5. Depth jump
    let input = "item1|10|2.5|1|good|\n>>subnote|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 2,
            column: 1,
            problem: "depth increased by more than one level (from depth 0 to 2)".to_string(),
            original: ">>subnote|".to_string(),
            dropped_as_child: false,
        }
    );

    // 6. Missing index when parent has multiple lists
    let multi_list_schema = "schema:multi{id, l1:list[str], l2:list[str]}";
    let input = "id1|\n>child without index|\n";
    let report = doctor(multi_list_schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 2,
            column: 2,
            problem: "missing list index".to_string(),
            original: ">child without index|".to_string(),
            dropped_as_child: false,
        }
    );

    // 7. Unclosed raw value in the middle of the file (per R3b Bug 1.2)
    let input = "item1|10|2.5|1|good|\nitem2|\\runclosed\nitem3|30|5.0|0|fair|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 2,
            column: 1,
            problem: "expected 5 columns, found 6".to_string(),
            original: "item2|\\runclosed\nitem3|30|5.0|0|fair|".to_string(),
            dropped_as_child: false,
        }
    );

    // 8. N/A and "quoted" in a text field: MUST BE LEFT ALONE (valid!)
    let input = "N/A|10|2.5|1|good|\n\"quoted\"|20|3.5|0|fair|\n";
    let report = doctor(schema, input, &options);
    assert!(report.data.is_some());
    assert_eq!(report.bad_lines.len(), 0);
    assert_eq!(report.repairs.len(), 0);

    // 9. null, None, N/A, NaN in int, bool, enum fields: MUST BE REPORTED, never turned into ~
    let input = "item1|null|2.5|1|good|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 7,
            problem: "'null' is not a valid int (i64)".to_string(),
            original: "item1|null|2.5|1|good|".to_string(),
            dropped_as_child: false,
        }
    );

    let input = "item1|10|2.5|None|good|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 14,
            problem: "'None' is not a valid bool (expected 1 or 0)".to_string(),
            original: "item1|10|2.5|None|good|".to_string(),
            dropped_as_child: false,
        }
    );

    let input = "item1|10|2.5|1|N/A|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 16,
            problem: "'N/A' is not one of: poor, fair, good".to_string(),
            original: "item1|10|2.5|1|N/A|".to_string(),
            dropped_as_child: false,
        }
    );

    // 10. true, false, yes, no in a bool field: MUST BE REPORTED, never converted
    let input = "item1|10|2.5|true|good|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 14,
            problem: "'true' is not a valid bool (expected 1 or 0)".to_string(),
            original: "item1|10|2.5|true|good|".to_string(),
            dropped_as_child: false,
        }
    );

    let input = "item1|10|2.5|yes|good|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 14,
            problem: "'yes' is not a valid bool (expected 1 or 0)".to_string(),
            original: "item1|10|2.5|yes|good|".to_string(),
            dropped_as_child: false,
        }
    );

    // 11. Empty field, "2024" in an int, +5, 1,000, Supported, year: 2024
    let input = "item1||2.5|1|good|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 7,
            problem: "empty field is not allowed (use ~ for null)".to_string(),
            original: "item1||2.5|1|good|".to_string(),
            dropped_as_child: false,
        }
    );

    let input = "item1|\"2024\"|2.5|1|good|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 7,
            problem: "'\"2024\"' is not a valid int (i64)".to_string(),
            original: "item1|\"2024\"|2.5|1|good|".to_string(),
            dropped_as_child: false,
        }
    );

    let input = "item1|+5|2.5|1|good|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 7,
            problem: "leading '+' is not allowed in int '+5'".to_string(),
            original: "item1|+5|2.5|1|good|".to_string(),
            dropped_as_child: false,
        }
    );

    let input = "item1|1,000|2.5|1|good|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 7,
            problem: "'1,000' is not a valid int (i64)".to_string(),
            original: "item1|1,000|2.5|1|good|".to_string(),
            dropped_as_child: false,
        }
    );

    let input = "item1|10|2.5|1|Supported|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 16,
            problem: "'Supported' is not one of: poor, fair, good".to_string(),
            original: "item1|10|2.5|1|Supported|".to_string(),
            dropped_as_child: false,
        }
    );

    let input = "item1|year: 2024|2.5|1|good|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 7,
            problem: "'year: 2024' is not a valid int (i64)".to_string(),
            original: "item1|year: 2024|2.5|1|good|".to_string(),
            dropped_as_child: false,
        }
    );

    // 12. 3,14 in a float
    let input = "item1|10|3,14|1|good|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.data, None);
    assert_eq!(report.repairs.len(), 0);
    assert_eq!(report.bad_lines.len(), 1);
    assert_eq!(
        report.bad_lines[0],
        BadLine {
            line: 1,
            column: 10,
            problem: "'3,14' is not a valid float (f64)".to_string(),
            original: "item1|10|3,14|1|good|".to_string(),
            dropped_as_child: false,
        }
    );
}

// =========================================================================
// 3. Strict versus salvage on the same input
// =========================================================================

#[test]
fn test_strict_vs_salvage_cascading_drop() {
    let schema = "schema:claim{text, count:int, notes:list[str]}";

    let input = "claim 1|10|\n>note 1|\nclaim 2|bad_int|\n>dropped child 1|\n>>dropped grandchild 1|\nclaim 3|30|\n>note 3|\n";

    // Strict mode: returns None
    let strict_opt = DoctorOptions {
        mode: DoctorMode::Strict,
        retry_hint: true,
    };
    let strict_report = doctor(schema, input, &strict_opt);
    assert!(strict_report.data.is_none());
    assert_eq!(strict_report.text, "");
    assert_eq!(strict_report.bad_lines.len(), 3);
    assert_eq!(
        strict_report.bad_lines[0],
        BadLine {
            line: 3,
            column: 9,
            problem: "'bad_int' is not a valid int (i64)".to_string(),
            original: "claim 2|bad_int|".to_string(),
            dropped_as_child: false,
        }
    );
    assert_eq!(
        strict_report.bad_lines[1],
        BadLine {
            line: 4,
            column: 1,
            problem: "dropped as child of line 3".to_string(),
            original: ">dropped child 1|".to_string(),
            dropped_as_child: true,
        }
    );
    assert_eq!(
        strict_report.bad_lines[2],
        BadLine {
            line: 5,
            column: 1,
            problem: "dropped as child of line 3".to_string(),
            original: ">>dropped grandchild 1|".to_string(),
            dropped_as_child: true,
        }
    );

    // Salvage mode: keeps claim 1 and claim 3, drops claim 2 and its children
    let salvage_opt = DoctorOptions {
        mode: DoctorMode::Salvage,
        retry_hint: true,
    };
    let salvage_report = doctor(schema, input, &salvage_opt);
    assert!(salvage_report.data.is_some());
    assert_eq!(salvage_report.bad_lines.len(), 3);
    assert_eq!(salvage_report.bad_lines[0], strict_report.bad_lines[0]);
    assert_eq!(salvage_report.bad_lines[1], strict_report.bad_lines[1]);
    assert_eq!(salvage_report.bad_lines[2], strict_report.bad_lines[2]);
    assert_eq!(
        salvage_report.text,
        "claim 1|10|\n>note 1|\nclaim 3|30|\n>note 3|\n"
    );
}

// =========================================================================
// 4. Hint and splice
// =========================================================================

#[test]
fn test_hint_and_splice() {
    let schema = "schema:claim{id, title, verdict:enum(supported,refuted), year:int, verified:bool}";
    let options = DoctorOptions::default();

    let input = "c1|Attention|supported|2017|1|\nc2|FlashAttention|supproted|2022|1|\nc3|Llama 405B|refuted|+2024|0|\n";

    let report = doctor(schema, input, &options);
    assert_eq!(report.bad_lines.len(), 2);

    // Exact hint format check
    let hint = report.hint.as_ref().expect("Hint should be present");
    let expected_hint = "Some lines were invalid. Rewrite only these lines, one per line, in the same order:\n1. c2|FlashAttention|supproted|2022|1|   ('supproted' is not one of: supported, refuted)\n2. c3|Llama 405B|refuted|+2024|0|   (leading '+' is not allowed in int '+2024')";
    assert_eq!(hint, expected_hint);

    // Splice: correct reply produces clean data
    let clean_reply = "c2|FlashAttention|supported|2022|1|\nc3|Llama 405B|refuted|2024|0|";
    let spliced_report = splice(schema, input, &report, clean_reply, &options).expect("Splice succeeded");
    assert!(spliced_report.data.is_some());
    assert_eq!(spliced_report.bad_lines.len(), 0);
    assert_eq!(
        spliced_report.text,
        "c1|Attention|supported|2017|1|\nc2|FlashAttention|supported|2022|1|\nc3|Llama 405B|refuted|2024|0|\n"
    );

    // Splice: reply with wrong number of lines rejected
    let wrong_count_reply = "c2|FlashAttention|supported|2022|1|";
    let err = splice(schema, input, &report, wrong_count_reply, &options);
    assert!(err.is_err());
    assert!(err.unwrap_err().message.contains("reply has 1 lines, expected 2"));

    // Splice: reply that is itself still bad, reported again
    let still_bad_reply = "c2|FlashAttention|still_bad|2022|1|\nc3|Llama 405B|refuted|2024|0|";
    let still_bad_report = splice(schema, input, &report, still_bad_reply, &options).expect("Splice ran");
    assert_eq!(still_bad_report.bad_lines.len(), 1);
    assert_eq!(
        still_bad_report.bad_lines[0],
        BadLine {
            line: 2,
            column: 19,
            problem: "'still_bad' is not one of: supported, refuted".to_string(),
            original: "c2|FlashAttention|still_bad|2022|1|".to_string(),
            dropped_as_child: false,
        }
    );
}

// =========================================================================
// 5. R3b Dedicated Bug Regression Tests (1.1, 1.2, 1.3)
// =========================================================================

#[test]
fn test_r3b_bug_1_1_cut_off_raw_at_eof() {
    let schema = "schema:code{a, b}";
    let options = DoctorOptions {
        mode: DoctorMode::Strict,
        retry_hint: true,
    };
    // Input from R3b §1.1: x|y|, then w|\rfn main() {, then   let a = 1;, ending there
    let input = "x|y|\nw|\\rfn main() {\n  let a = 1;";
    let report = doctor(schema, input, &options);

    assert!(report.bad_lines.is_empty(), "Expected no bad line, found: {:?}", report.bad_lines);
    assert_eq!(report.repairs.len(), 1);
    assert_eq!(
        report.repairs[0],
        Repair {
            line: 2,
            column: 1,
            fix: SafeFix::CutOffLastRecord,
            original: "w|\\rfn main() {\n  let a = 1;".to_string(),
            new_text: String::new(),
        }
    );
    assert_eq!(report.text, "x|y|\n");
    assert!(report.data.is_some(), "Strict mode must succeed");
}

#[test]
fn test_r3b_bug_1_2_unclosed_raw_mid_file() {
    let options = DoctorOptions {
        mode: DoctorMode::Strict,
        retry_hint: true,
    };

    // Case 1: Input x|\rfoo, then y|z|, under schema {a, b}.
    // Raw value runs to next unescaped |, so foo\ny, then z: 3 values where 2 expected -> 1 bad line covering lines 1-2
    let schema1 = "schema:test{a, b}";
    let input1 = "x|\\rfoo\ny|z|\n";
    let report1 = doctor(schema1, input1, &options);
    assert_eq!(report1.data, None);
    assert_eq!(report1.repairs.len(), 0);
    assert_eq!(report1.bad_lines.len(), 1);
    assert_eq!(
        report1.bad_lines[0],
        BadLine {
            line: 1,
            column: 1,
            problem: "expected 2 columns, found 3".to_string(),
            original: "x|\\rfoo\ny|z|".to_string(),
            dropped_as_child: false,
        }
    );

    // Case 2: x|\rl1, >l2, schema:zz|, >n1| is a valid record with 1 child
    let schema2 = "schema:item{a, b, notes:list[str]}";
    let input2 = "x|\\rl1\n>l2\nschema:zz|\n>n1|\n";
    let report2 = doctor(schema2, input2, &options);
    assert_eq!(report2.bad_lines.len(), 0);
    assert_eq!(report2.repairs.len(), 0);
    assert!(report2.data.is_some());
    let file = report2.data.unwrap();
    assert_eq!(file.blocks.len(), 1);
    assert_eq!(file.blocks[0].records.len(), 1);
    if let Value::Record(fields) = &file.blocks[0].records[0] {
        assert_eq!(fields[0].0, "a");
        assert_eq!(fields[0].1, Value::String("x".to_string()));
        assert_eq!(fields[1].0, "b");
        assert_eq!(fields[1].1, Value::String("l1\n>l2\nschema:zz".to_string()));
        assert_eq!(fields[2].0, "notes");
        assert_eq!(fields[2].1, Value::List(vec![Value::String("n1".to_string())]));
    } else {
        panic!("Expected Value::Record, got: {:?}", file.blocks[0].records[0]);
    }
}

#[test]
fn test_r3b_bug_1_3_columns_reporting() {
    let options = DoctorOptions {
        mode: DoctorMode::Strict,
        retry_hint: true,
    };

    // Case 1: 1|zz| with zz invalid gives column 3
    let schema1 = "schema:test{id:int, val:int}";
    let input1 = "1|zz|\n";
    let report1 = doctor(schema1, input1, &options);
    assert_eq!(report1.bad_lines.len(), 1);
    assert_eq!(
        report1.bad_lines[0],
        BadLine {
            line: 1,
            column: 3,
            problem: "'zz' is not a valid int (i64)".to_string(),
            original: "1|zz|".to_string(),
            dropped_as_child: false,
        }
    );

    // Case 2:   \t bad| gives column where bad starts (column 5)
    let schema2 = "schema:test{name, count:int}";
    let input2 = "  \t bad|\nvalid|10|\n";
    let report2 = doctor(schema2, input2, &options);
    assert_eq!(report2.bad_lines.len(), 1);
    assert_eq!(
        report2.bad_lines[0],
        BadLine {
            line: 1,
            column: 5,
            problem: "expected 2 columns, found 1".to_string(),
            original: "  \t bad|".to_string(),
            dropped_as_child: false,
        }
    );
}

// =========================================================================
// 6. The damage suite (at least 5,000 cases)
// =========================================================================

#[derive(Default)]
struct DamageStats {
    cases_run: usize,
    exact_restorations: usize,
    bad_lines_reported: usize,
    false_repairs: usize,
}

fn make_doc_file(schema_name: &str, records: Vec<Value>) -> DecodedFile {
    let mut file = DecodedFile::new();
    file.blocks.push(Block {
        class_name: schema_name.to_string(),
        records,
    });
    file
}

fn encode_records(schema: &str, class_name: &str, records: &[Value]) -> String {
    let file = make_doc_file(class_name, records.to_vec());
    structl::encode_with(schema, &file).unwrap()
}

#[test]
fn test_damage_suite_5000_cases() {
    let doc_schema = "schema:doc{title:str, year:int, score:float, active:bool}";
    let review_schema = "schema:review{title:str, rating:enum(poor,fair,good,excellent), score:float, verified:bool}";
    let claim_schema = "schema:claim{text:str, notes:list[str]}";
    let multi_schema = "schema:multi{id:str, count:int, l1:list[str], l2:list[str]}";
    let fix_schema = "schema:fix{file:str, patch:str}";

    let strict_opt = DoctorOptions {
        mode: DoctorMode::Strict,
        retry_hint: true,
    };
    let salvage_opt = DoctorOptions {
        mode: DoctorMode::Salvage,
        retry_hint: true,
    };

    let kind_names = [
        "No damage",
        "Code fences",
        "Echoed schema",
        "Chatter",
        "Header row",
        "Missing final pipe",
        "Cut-off last record",
        "Fixable combinations",
        "Wrong column count (too few)",
        "Wrong column count (too many)",
        "Unescaped pipe inside text",
        "Enum typo",
        "Depth jump",
        "Missing list index",
        "Child line with no list parent",
        "Unclosed raw value mid-file",
        "Quoted int (\"2024\")",
        "Leading plus in int (+5)",
        "Comma in int (1,000)",
        "Enum case mismatch (Supported)",
        "Field-name prefix (year: 2024)",
        "Literal null",
        "Literal None",
        "Literal N/A",
        "Literal true in bool",
        "Literal yes in bool",
        "Empty field (||)",
        "Decimal comma in float (3,14)",
        "Invalid escape sequence",
    ];

    let mut stats: Vec<DamageStats> = (0..kind_names.len()).map(|_| DamageStats::default()).collect();
    const RUNS_PER_KIND: usize = 180;

    for kind_idx in 0..kind_names.len() {
        for run in 0..RUNS_PER_KIND {
            stats[kind_idx].cases_run += 1;

            match kind_idx {
                0 => {
                    // 1. No damage
                    let r1 = Value::Record(vec![
                        ("title".to_string(), Value::String(format!("alpha_{}", run))),
                        ("year".to_string(), Value::Int(2020 + (run as i64 % 5))),
                        ("score".to_string(), Value::Float(4.5)),
                        ("active".to_string(), Value::Bool(true)),
                    ]);
                    let r2 = Value::Record(vec![
                        ("title".to_string(), Value::String(format!("beta_{}", run))),
                        ("year".to_string(), Value::Int(2025)),
                        ("score".to_string(), Value::Float(3.0)),
                        ("active".to_string(), Value::Bool(false)),
                    ]);
                    let orig = vec![r1, r2];
                    let canonical = encode_records(doc_schema, "doc", &orig);

                    let report = doctor(doc_schema, &canonical, &salvage_opt);
                    assert!(report.bad_lines.is_empty());
                    assert!(report.repairs.is_empty());
                    assert_eq!(report.text, canonical);
                    let salvaged = report.data.unwrap().blocks[0].records.clone();
                    if salvaged == orig {
                        stats[kind_idx].exact_restorations += 1;
                    } else {
                        stats[kind_idx].false_repairs += 1;
                    }
                }
                1 => {
                    // 2. Code fences
                    let r1 = Value::Record(vec![
                        ("title".to_string(), Value::String(format!("alpha_{}", run))),
                        ("year".to_string(), Value::Int(2021)),
                        ("score".to_string(), Value::Float(5.0)),
                        ("active".to_string(), Value::Bool(true)),
                    ]);
                    let orig = vec![r1];
                    let canonical = encode_records(doc_schema, "doc", &orig);
                    let damaged = format!("```stc\n{}```\n", canonical);

                    let report = doctor(doc_schema, &damaged, &salvage_opt);
                    assert!(report.bad_lines.is_empty());
                    assert_eq!(report.repairs.len(), 2);
                    assert_eq!(report.repairs[0].fix, SafeFix::CodeFences);
                    assert_eq!(report.repairs[1].fix, SafeFix::CodeFences);
                    let salvaged = report.data.unwrap().blocks[0].records.clone();
                    if salvaged == orig {
                        stats[kind_idx].exact_restorations += 1;
                    } else {
                        stats[kind_idx].false_repairs += 1;
                    }
                }
                2 => {
                    // 3. Echoed schema
                    let r1 = Value::Record(vec![
                        ("title".to_string(), Value::String(format!("alpha_{}", run))),
                        ("year".to_string(), Value::Int(2022)),
                        ("score".to_string(), Value::Float(4.0)),
                        ("active".to_string(), Value::Bool(false)),
                    ]);
                    let orig = vec![r1];
                    let canonical = encode_records(doc_schema, "doc", &orig);
                    let damaged = format!("{}\n{}", doc_schema, canonical);

                    let report = doctor(doc_schema, &damaged, &salvage_opt);
                    assert!(report.bad_lines.is_empty());
                    assert_eq!(report.repairs.len(), 1);
                    assert_eq!(report.repairs[0].fix, SafeFix::EchoedSchema);
                    assert_eq!(report.repairs[0].line, 1);
                    let salvaged = report.data.unwrap().blocks[0].records.clone();
                    if salvaged == orig {
                        stats[kind_idx].exact_restorations += 1;
                    } else {
                        stats[kind_idx].false_repairs += 1;
                    }
                }
                3 => {
                    // 4. Chatter
                    let r1 = Value::Record(vec![
                        ("title".to_string(), Value::String(format!("alpha_{}", run))),
                        ("year".to_string(), Value::Int(2023)),
                        ("score".to_string(), Value::Float(3.5)),
                        ("active".to_string(), Value::Bool(true)),
                    ]);
                    let orig = vec![r1];
                    let canonical = encode_records(doc_schema, "doc", &orig);
                    let damaged = format!("Here is your data:\n{}Let me know if you need more.\n", canonical);

                    let report = doctor(doc_schema, &damaged, &salvage_opt);
                    assert!(report.bad_lines.is_empty());
                    assert_eq!(report.repairs.len(), 2);
                    assert_eq!(report.repairs[0].fix, SafeFix::Chatter);
                    assert_eq!(report.repairs[1].fix, SafeFix::Chatter);
                    let salvaged = report.data.unwrap().blocks[0].records.clone();
                    if salvaged == orig {
                        stats[kind_idx].exact_restorations += 1;
                    } else {
                        stats[kind_idx].false_repairs += 1;
                    }
                }
                4 => {
                    // 5. Header row
                    let r1 = Value::Record(vec![
                        ("title".to_string(), Value::String(format!("alpha_{}", run))),
                        ("year".to_string(), Value::Int(2024)),
                        ("score".to_string(), Value::Float(4.2)),
                        ("active".to_string(), Value::Bool(true)),
                    ]);
                    let orig = vec![r1];
                    let canonical = encode_records(doc_schema, "doc", &orig);
                    let damaged = format!("title|year|score|active|\n{}", canonical);

                    let report = doctor(doc_schema, &damaged, &salvage_opt);
                    assert!(report.bad_lines.is_empty());
                    assert_eq!(report.repairs.len(), 1);
                    assert_eq!(report.repairs[0].fix, SafeFix::HeaderRow);
                    assert_eq!(report.repairs[0].line, 1);
                    let salvaged = report.data.unwrap().blocks[0].records.clone();
                    if salvaged == orig {
                        stats[kind_idx].exact_restorations += 1;
                    } else {
                        stats[kind_idx].false_repairs += 1;
                    }
                }
                5 => {
                    // 6. Missing final pipe
                    let r1 = Value::Record(vec![
                        ("title".to_string(), Value::String(format!("alpha_{}", run))),
                        ("year".to_string(), Value::Int(2024)),
                        ("score".to_string(), Value::Float(4.2)),
                        ("active".to_string(), Value::Bool(true)),
                    ]);
                    let orig = vec![r1];
                    let canonical = encode_records(doc_schema, "doc", &orig);
                    let trimmed_line = canonical.trim_end();
                    assert!(trimmed_line.ends_with('|'));
                    let damaged = format!("{}\n", &trimmed_line[..trimmed_line.len() - 1]);

                    let report = doctor(doc_schema, &damaged, &salvage_opt);
                    assert!(report.bad_lines.is_empty());
                    assert_eq!(report.repairs.len(), 1);
                    assert_eq!(report.repairs[0].fix, SafeFix::MissingFinalPipe);
                    assert_eq!(report.repairs[0].line, 1);
                    let salvaged = report.data.unwrap().blocks[0].records.clone();
                    if salvaged == orig {
                        stats[kind_idx].exact_restorations += 1;
                    } else {
                        stats[kind_idx].false_repairs += 1;
                    }
                }
                6 => {
                    // 7. Cut-off last record
                    let r1 = Value::Record(vec![
                        ("title".to_string(), Value::String(format!("alpha_{}", run))),
                        ("year".to_string(), Value::Int(2024)),
                        ("score".to_string(), Value::Float(4.2)),
                        ("active".to_string(), Value::Bool(true)),
                    ]);
                    let orig = vec![r1];
                    let canonical = encode_records(doc_schema, "doc", &orig);
                    let damaged = format!("{}cutoff_title|42|\n", canonical);

                    let report = doctor(doc_schema, &damaged, &salvage_opt);
                    assert!(report.bad_lines.is_empty());
                    assert_eq!(report.repairs.len(), 1);
                    assert_eq!(report.repairs[0].fix, SafeFix::CutOffLastRecord);
                    assert_eq!(report.repairs[0].line, 2);
                    let salvaged = report.data.unwrap().blocks[0].records.clone();
                    if salvaged == orig {
                        stats[kind_idx].exact_restorations += 1;
                    } else {
                        stats[kind_idx].false_repairs += 1;
                    }
                }
                7 => {
                    // 8. Fixable combinations
                    let r1 = Value::Record(vec![
                        ("title".to_string(), Value::String(format!("alpha_{}", run))),
                        ("year".to_string(), Value::Int(2024)),
                        ("score".to_string(), Value::Float(4.2)),
                        ("active".to_string(), Value::Bool(true)),
                    ]);
                    let orig = vec![r1];
                    let canonical = encode_records(doc_schema, "doc", &orig);
                    // Combine Code fences + Header row + Cut-off last record
                    let damaged = format!(
                        "```\ntitle|year|score|active|\n{}cutoff_title|10|\n```\n",
                        canonical
                    );

                    let report = doctor(doc_schema, &damaged, &salvage_opt);
                    assert!(report.bad_lines.is_empty());
                    assert_eq!(report.repairs.len(), 4); // 2 fences + 1 header + 1 cut-off
                    let salvaged = report.data.unwrap().blocks[0].records.clone();
                    if salvaged == orig {
                        stats[kind_idx].exact_restorations += 1;
                    } else {
                        stats[kind_idx].false_repairs += 1;
                    }
                }
                _ => {
                    // Unfixable damage kinds (8..=28)
                    let (schema, orig, damaged) = match kind_idx {
                        8 => {
                            // Wrong column count (too few)
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}too_few_cols|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        9 => {
                            // Wrong column count (too many)
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}bad|2022|5.0|1|extra_field|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        10 => {
                            // Unescaped pipe inside text
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}unescaped|pipe|2022|5.0|1|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        11 => {
                            // Enum typo
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("rev_1_{}", run))),
                                ("rating".to_string(), Value::Enum("good".to_string())),
                                ("score".to_string(), Value::Float(4.0)),
                                ("verified".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("rev_2_{}", run))),
                                ("rating".to_string(), Value::Enum("poor".to_string())),
                                ("score".to_string(), Value::Float(2.0)),
                                ("verified".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(review_schema, "review", &[orig[0].clone()]);
                            let c2 = encode_records(review_schema, "review", &[orig[1].clone()]);
                            let damaged = format!("{}rev_bad|godd|3.0|1|\n{}", c1, c2);
                            (review_schema, orig, damaged)
                        }
                        12 => {
                            // Depth jump
                            let r1 = Value::Record(vec![
                                ("text".to_string(), Value::String(format!("claim_1_{}", run))),
                                ("notes".to_string(), Value::List(vec![])),
                            ]);
                            let r2 = Value::Record(vec![
                                ("text".to_string(), Value::String(format!("claim_2_{}", run))),
                                ("notes".to_string(), Value::List(vec![])),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(claim_schema, "claim", &[orig[0].clone()]);
                            let c2 = encode_records(claim_schema, "claim", &[orig[1].clone()]);
                            let damaged = format!("{}>>depth_jump_item|\n{}", c1, c2);
                            (claim_schema, orig, damaged)
                        }
                        13 => {
                            // Missing list index
                            let r1 = Value::Record(vec![
                                ("id".to_string(), Value::String(format!("id_1_{}", run))),
                                ("count".to_string(), Value::Int(10)),
                                ("l1".to_string(), Value::List(vec![])),
                                ("l2".to_string(), Value::List(vec![])),
                            ]);
                            let r2 = Value::Record(vec![
                                ("id".to_string(), Value::String(format!("id_2_{}", run))),
                                ("count".to_string(), Value::Int(20)),
                                ("l1".to_string(), Value::List(vec![])),
                                ("l2".to_string(), Value::List(vec![])),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(multi_schema, "multi", &[orig[0].clone()]);
                            let c2 = encode_records(multi_schema, "multi", &[orig[1].clone()]);
                            let damaged = format!("{}>missing_idx|\n{}", c1, c2);
                            (multi_schema, orig, damaged)
                        }
                        14 => {
                            // Child line with no list parent
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}>child_under_doc|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        15 => {
                            // Unclosed raw value mid-file
                            let r1 = Value::Record(vec![
                                ("file".to_string(), Value::String(format!("main_{}.rs", run))),
                                ("patch".to_string(), Value::String("let a = 1;".to_string())),
                            ]);
                            let r2 = Value::Record(vec![
                                ("file".to_string(), Value::String(format!("lib_{}.rs", run))),
                                ("patch".to_string(), Value::String("let b = 2;".to_string())),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(fix_schema, "fix", &[orig[0].clone()]);
                            let c2 = encode_records(fix_schema, "fix", &[orig[1].clone()]);
                            let damaged = format!("{}unclosed|\\rfn main() {{\nnext|patch|\n{}", c1, c2);
                            (fix_schema, orig, damaged)
                        }
                        16 => {
                            // Quoted int ("2024")
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}bad_doc|\"2024\"|5.0|1|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        17 => {
                            // Leading plus in int (+5)
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}bad_doc|+5|5.0|1|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        18 => {
                            // Comma in int (1,000)
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}bad_doc|1,000|5.0|1|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        19 => {
                            // Enum case mismatch (Supported)
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("rev_1_{}", run))),
                                ("rating".to_string(), Value::Enum("good".to_string())),
                                ("score".to_string(), Value::Float(4.0)),
                                ("verified".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("rev_2_{}", run))),
                                ("rating".to_string(), Value::Enum("poor".to_string())),
                                ("score".to_string(), Value::Float(2.0)),
                                ("verified".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(review_schema, "review", &[orig[0].clone()]);
                            let c2 = encode_records(review_schema, "review", &[orig[1].clone()]);
                            let damaged = format!("{}rev_case|Good|3.0|1|\n{}", c1, c2);
                            (review_schema, orig, damaged)
                        }
                        20 => {
                            // Field-name prefix (year: 2024)
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}bad_doc|year: 2024|5.0|1|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        21 => {
                            // Literal null
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}bad_doc|null|5.0|1|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        22 => {
                            // Literal None
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}bad_doc|2022|5.0|None|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        23 => {
                            // Literal N/A
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("rev_1_{}", run))),
                                ("rating".to_string(), Value::Enum("good".to_string())),
                                ("score".to_string(), Value::Float(4.0)),
                                ("verified".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("rev_2_{}", run))),
                                ("rating".to_string(), Value::Enum("poor".to_string())),
                                ("score".to_string(), Value::Float(2.0)),
                                ("verified".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(review_schema, "review", &[orig[0].clone()]);
                            let c2 = encode_records(review_schema, "review", &[orig[1].clone()]);
                            let damaged = format!("{}rev_bad|N/A|3.0|1|\n{}", c1, c2);
                            (review_schema, orig, damaged)
                        }
                        24 => {
                            // Literal true in bool
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}bad_doc|2022|5.0|true|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        25 => {
                            // Literal yes in bool
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}bad_doc|2022|5.0|yes|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        26 => {
                            // Empty field (||)
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}bad_doc||5.0|1|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        27 => {
                            // Decimal comma in float (3,14)
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}bad_doc|2022|3,14|1|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        28 => {
                            // Invalid escape sequence
                            let r1 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_1_{}", run))),
                                ("year".to_string(), Value::Int(2020)),
                                ("score".to_string(), Value::Float(4.0)),
                                ("active".to_string(), Value::Bool(true)),
                            ]);
                            let r2 = Value::Record(vec![
                                ("title".to_string(), Value::String(format!("doc_2_{}", run))),
                                ("year".to_string(), Value::Int(2021)),
                                ("score".to_string(), Value::Float(3.0)),
                                ("active".to_string(), Value::Bool(false)),
                            ]);
                            let orig = vec![r1, r2];
                            let c1 = encode_records(doc_schema, "doc", &[orig[0].clone()]);
                            let c2 = encode_records(doc_schema, "doc", &[orig[1].clone()]);
                            let damaged = format!("{}bad_\\q_doc|2022|5.0|1|\n{}", c1, c2);
                            (doc_schema, orig, damaged)
                        }
                        _ => unreachable!(),
                    };

                    // Strict mode must reject with bad lines and return None data
                    let strict_report = doctor(schema, &damaged, &strict_opt);
                    assert!(strict_report.data.is_none(), "Strict mode must reject unfixable damage for kind {}", kind_names[kind_idx]);
                    assert!(!strict_report.bad_lines.is_empty());

                    // Salvage mode must report bad line and return the undamaged records byte-identical
                    let salvage_report = doctor(schema, &damaged, &salvage_opt);
                    assert!(!salvage_report.bad_lines.is_empty());

                    if let Some(decoded_file) = salvage_report.data {
                        let salvaged = decoded_file.blocks[0].records.clone();
                        if salvaged == orig {
                            stats[kind_idx].bad_lines_reported += 1;
                        } else {
                            stats[kind_idx].false_repairs += 1;
                            panic!(
                                "False repair detected on kind {}: salvaged records differ from original.\nSalvaged: {:?}\nOriginal: {:?}",
                                kind_names[kind_idx], salvaged, orig
                            );
                        }
                    } else {
                        panic!("Salvage mode failed to return any data for kind {}", kind_names[kind_idx]);
                    }
                }
            }
        }
    }

    println!("\n=========================================================================================");
    println!("STRUCT DOCTOR SYNTHETIC DAMAGE SUITE RESULTS");
    println!("=========================================================================================\n");
    println!("| Damage Kind | Cases Run | Exact Restorations | Bad Lines Reported | False Repairs |");
    println!("|---|---|---|---|---|");
    for (i, name) in kind_names.iter().enumerate() {
        println!(
            "| {} | {} | {} | {} | {} |",
            name, stats[i].cases_run, stats[i].exact_restorations, stats[i].bad_lines_reported, stats[i].false_repairs
        );
    }
    println!("=========================================================================================\n");

    let total_cases: usize = stats.iter().map(|s| s.cases_run).sum();
    let total_false_repairs: usize = stats.iter().map(|s| s.false_repairs).sum();
    assert!(total_cases >= 5000, "Must run at least 5000 cases, ran {}", total_cases);
    assert_eq!(total_false_repairs, 0, "False repairs must be 0 under real check");
}
