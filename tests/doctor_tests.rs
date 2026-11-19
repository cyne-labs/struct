//! Synthetic tests for Struct Doctor (R3 Part B).
//!
//! Standard library only.

mod common;

use common::SimpleRng;
use structl::{
    doctor, splice, BadLine, Block, DecodedFile, DoctorMode, DoctorOptions,
    Repair, SafeFix, Value,
};

// =========================================================================
// 1. One test per safe fix (4 in all), at least 3 cases each
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
fn test_header_row_is_data_record() {
    let schema = "schema:entry{word, meaning}";
    let options = DoctorOptions::default();

    let input = "word|meaning|\napple|a round fruit|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.text, "word|meaning|\napple|a round fruit|\n");
    assert!(report.repairs.is_empty(), "Header-like line with valid types must not be repaired");
    assert!(report.bad_lines.is_empty(), "Valid record must not be reported as bad line");

    let data = report.data.expect("Should decode successfully");
    assert_eq!(data.blocks.len(), 1);
    assert_eq!(data.blocks[0].records.len(), 2);
    assert_eq!(
        data.blocks[0].records[0],
        Value::Record(vec![
            ("word".to_string(), Value::String("word".to_string())),
            ("meaning".to_string(), Value::String("meaning".to_string())),
        ])
    );
    assert_eq!(
        data.blocks[0].records[1],
        Value::Record(vec![
            ("word".to_string(), Value::String("apple".to_string())),
            ("meaning".to_string(), Value::String("a round fruit".to_string())),
        ])
    );
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

// =========================================================================
// 2. Bad lines coverage, checking that nothing is edited
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

    // 7. Unclosed raw value in the middle of the file
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

    // 8. N/A and "quoted" in a text field: valid
    let input = "N/A|10|2.5|1|good|\n\"quoted\"|20|3.5|0|fair|\n";
    let report = doctor(schema, input, &options);
    assert!(report.data.is_some());
    assert_eq!(report.bad_lines.len(), 0);
    assert_eq!(report.repairs.len(), 0);

    // 9. null, None, N/A in int, bool, enum fields: reported
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

    // 10. true, false, yes in a bool field: reported
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

    let hint = report.hint.as_ref().expect("Hint should be present");
    let expected_hint = "Some lines were invalid. Rewrite only these lines, one per line, in the same order:\n1. c2|FlashAttention|supproted|2022|1|   ('supproted' is not one of: supported, refuted)\n2. c3|Llama 405B|refuted|+2024|0|   (leading '+' is not allowed in int '+2024')";
    assert_eq!(hint, expected_hint);

    let clean_reply = "c2|FlashAttention|supported|2022|1|\nc3|Llama 405B|refuted|2024|0|";
    let spliced_report = splice(schema, input, &report, clean_reply, &options).expect("Splice succeeded");
    assert!(spliced_report.data.is_some());
    assert_eq!(spliced_report.bad_lines.len(), 0);
    assert_eq!(
        spliced_report.text,
        "c1|Attention|supported|2017|1|\nc2|FlashAttention|supported|2022|1|\nc3|Llama 405B|refuted|2024|0|\n"
    );

    let wrong_count_reply = "c2|FlashAttention|supported|2022|1|";
    let err = splice(schema, input, &report, wrong_count_reply, &options);
    assert!(err.is_err());
    assert!(err.unwrap_err().message.contains("reply has 1 lines, expected 2"));

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
    // Under R3f, raw value open at EOF gets closed by Fix 4 MissingFinalPipe
    let input = "x|y|\nw|\\rfn main() {\n  let a = 1;";
    let report = doctor(schema, input, &options);

    assert!(report.bad_lines.is_empty(), "Expected no bad line, found: {:?}", report.bad_lines);
    assert_eq!(report.repairs.len(), 1);
    assert_eq!(
        report.repairs[0],
        Repair {
            line: 3,
            column: 13,
            fix: SafeFix::MissingFinalPipe,
            original: "w|\\rfn main() {\n  let a = 1;".to_string(),
            new_text: "w|\\rfn main() {\n  let a = 1;|".to_string(),
        }
    );
    assert_eq!(report.text, "x|y|\nw|\\rfn main() {\n  let a = 1;|\n");
    assert!(report.data.is_some(), "Strict mode must succeed");
}

#[test]
fn test_r3b_bug_1_2_unclosed_raw_mid_file() {
    let options = DoctorOptions {
        mode: DoctorMode::Strict,
        retry_hint: true,
    };

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
// 5b. R3e Dedicated Tests: Three Bugs and SPEC Changes
// =========================================================================

#[test]
fn test_r3e_three_bugs() {
    let options = DoctorOptions::default();

    // Bug 1: A valid last child line is kept
    let schema1 = "schema:c{n:int, a:list[record{p, q, r}], b:list[int]}";
    let input1 = "1|\n>2|5|\n";
    let report1 = doctor(schema1, input1, &options);
    assert_eq!(report1.repairs.len(), 0, "Bug 1: must have 0 repairs");
    assert_eq!(report1.bad_lines.len(), 0, "Bug 1: must have 0 bad lines");
    let data1 = report1.data.expect("Bug 1: data should decode");
    assert_eq!(data1.blocks.len(), 1);
    assert_eq!(data1.blocks[0].records.len(), 1);
    if let Value::Record(fields) = &data1.blocks[0].records[0] {
        assert_eq!(fields[0].0, "n");
        assert_eq!(fields[0].1, Value::Int(1));
        assert_eq!(fields[1].0, "a");
        assert_eq!(fields[1].1, Value::List(vec![]));
        assert_eq!(fields[2].0, "b");
        assert_eq!(fields[2].1, Value::List(vec![Value::Int(5)]));
    } else {
        panic!("Expected record");
    }

    // Bug 2: A child line missing its final | is repaired under a parent with several lists
    let schema2 = "schema:c{n:int, a:list[int], b:list[int]}";
    let input2 = "1|\n>2|5";
    let report2 = doctor(schema2, input2, &options);
    assert_eq!(report2.bad_lines.len(), 0, "Bug 2: must have 0 bad lines");
    assert_eq!(report2.repairs.len(), 1, "Bug 2: must have 1 repair");
    assert_eq!(report2.repairs[0].fix, SafeFix::MissingFinalPipe);
    assert_eq!(report2.repairs[0].line, 2);
    assert_eq!(report2.repairs[0].original, ">2|5");
    assert_eq!(report2.repairs[0].new_text, ">2|5|");
    assert_eq!(report2.text, "1|\n>2|5|\n");

    // Bug 3: A depth-2 line under a parent with no lists is reported as a bad line
    let schema3 = "schema:n{l, s:list[record{t, y}]}";
    let input3 = "~|\n>a|b|\n>>c|\n";
    let report3 = doctor(schema3, input3, &options);
    assert_eq!(report3.repairs.len(), 0, "Bug 3: must have 0 repairs");
    assert_eq!(report3.bad_lines.len(), 1, "Bug 3: must report bad line");
    assert_eq!(report3.bad_lines[0].line, 3);
    assert_eq!(report3.bad_lines[0].column, 1);
    assert_eq!(report3.bad_lines[0].problem, "parent record has no list fields to attach child line");
    assert_eq!(report3.bad_lines[0].original, ">>c|");
}

#[test]
fn test_r3e_spec_changes() {
    let options = DoctorOptions {
        mode: DoctorMode::Strict,
        retry_hint: true,
    };

    // 1. Fix 4 on line with no pipe at all
    // Included case:
    let schema_id = "schema:test{id:int}";
    let input_no_pipe = "1|\n1706";
    let report_no_pipe = doctor(schema_id, input_no_pipe, &options);
    assert_eq!(report_no_pipe.bad_lines.len(), 0);
    assert_eq!(report_no_pipe.repairs.len(), 1);
    assert_eq!(
        report_no_pipe.repairs[0],
        Repair {
            line: 2,
            column: 5,
            fix: SafeFix::MissingFinalPipe,
            original: "1706".to_string(),
            new_text: "1706|".to_string(),
        }
    );
    assert_eq!(report_no_pipe.text, "1|\n1706|\n");

    // Excluded case (chatter, not a valid int):
    let input_chatter = "1|\nHope this helps!\n";
    let report_chatter = doctor(schema_id, input_chatter, &options);
    assert_eq!(report_chatter.bad_lines.len(), 0);
    assert_eq!(report_chatter.repairs.len(), 1);
    assert_eq!(
        report_chatter.repairs[0],
        Repair {
            line: 2,
            column: 1,
            fix: SafeFix::Chatter,
            original: "Hope this helps!".to_string(),
            new_text: String::new(),
        }
    );
    assert_eq!(report_chatter.text, "1|\n");

    // Under R3f, mid-file line gets | added by Fix 4, and is then a bad line:
    let schema_two = "schema:test{id:int, name:str}";
    let input_mid_bad = "1|a|\n1706\n2|b|\n";
    let report_mid_bad = doctor(schema_two, input_mid_bad, &options);
    assert_eq!(report_mid_bad.repairs.len(), 1);
    assert_eq!(report_mid_bad.repairs[0].fix, SafeFix::MissingFinalPipe);
    assert_eq!(report_mid_bad.repairs[0].line, 2);
    assert_eq!(report_mid_bad.repairs[0].original, "1706");
    assert_eq!(report_mid_bad.repairs[0].new_text, "1706|");
    assert_eq!(report_mid_bad.bad_lines.len(), 1);
    assert_eq!(
        report_mid_bad.bad_lines[0],
        BadLine {
            line: 2,
            column: 1,
            problem: "expected 2 columns, found 1".to_string(),
            original: "1706|".to_string(),
            dropped_as_child: false,
        }
    );

    // 2. Multi-line raw record missing final pipe reported on record's last line
    let schema_raw = "schema:fix{patch, file}";
    let input_raw_no_pipe = "\\rfn main() {\n    println!(\"hi\");\n}|main.rs\n";
    let report_raw_no_pipe = doctor(schema_raw, input_raw_no_pipe, &options);
    assert_eq!(report_raw_no_pipe.bad_lines.len(), 0);
    assert_eq!(report_raw_no_pipe.repairs.len(), 1);
    assert_eq!(
        report_raw_no_pipe.repairs[0],
        Repair {
            line: 3,
            column: 10,
            fix: SafeFix::MissingFinalPipe,
            original: "\\rfn main() {\n    println!(\"hi\");\n}|main.rs".to_string(),
            new_text: "\\rfn main() {\n    println!(\"hi\");\n}|main.rs|".to_string(),
        }
    );
    assert_eq!(report_raw_no_pipe.repairs[0].line, 3);
}

// =========================================================================
// 5c. R3f Dedicated Tests: New Rules and Splice Bug
// =========================================================================

#[test]
fn test_r3f_new_rules() {
    let options = DoctorOptions::default();

    // 1. 2024|Rope becomes 2024|Rope|, valid
    let schema1 = "schema:claim{year:int, title:str}";
    let input1 = "2024|Rope";
    let report1 = doctor(schema1, input1, &options);
    assert_eq!(report1.repairs.len(), 1);
    assert_eq!(
        report1.repairs[0],
        Repair {
            line: 1,
            column: 10,
            fix: SafeFix::MissingFinalPipe,
            original: "2024|Rope".to_string(),
            new_text: "2024|Rope|".to_string(),
        }
    );
    assert!(report1.bad_lines.is_empty());
    assert_eq!(report1.text, "2024|Rope|\n");

    // 2. 2024 under {year:int, title} gets | added, and is then a bad line with expected 2 columns, found 1
    let schema2 = "schema:claim{year:int, title:str}";
    let input2 = "1|first|\n2024\n2|second|\n";
    let report2 = doctor(schema2, input2, &options);
    assert_eq!(report2.repairs.len(), 1);
    assert_eq!(
        report2.repairs[0],
        Repair {
            line: 2,
            column: 5,
            fix: SafeFix::MissingFinalPipe,
            original: "2024".to_string(),
            new_text: "2024|".to_string(),
        }
    );
    assert_eq!(report2.bad_lines.len(), 1);
    assert_eq!(
        report2.bad_lines[0],
        BadLine {
            line: 2,
            column: 1,
            problem: "expected 2 columns, found 1".to_string(),
            original: "2024|".to_string(),
            dropped_as_child: false,
        }
    );
    assert_eq!(report2.text, "1|first|\n2|second|\n");

    // 3. A raw value open at the end of the output is closed
    let schema3 = "schema:code{file, patch}";
    let input3 = "main.rs|\\rfn main() {\n  println!(\"hi\");";
    let report3 = doctor(schema3, input3, &options);
    assert_eq!(report3.repairs.len(), 1);
    assert_eq!(
        report3.repairs[0],
        Repair {
            line: 2,
            column: 18,
            fix: SafeFix::MissingFinalPipe,
            original: "main.rs|\\rfn main() {\n  println!(\"hi\");".to_string(),
            new_text: "main.rs|\\rfn main() {\n  println!(\"hi\");|".to_string(),
        }
    );
    assert!(report3.bad_lines.is_empty());
    assert_eq!(report3.text, "main.rs|\\rfn main() {\n  println!(\"hi\");|\n");

    // 4. >rope (child, no final |) becomes >rope|, with a fix-4 repair, and is then checked like any other line
    let schema4 = "schema:claim{title, notes:list[str]}";
    let input4 = "Attention|\n>rope";
    let report4 = doctor(schema4, input4, &options);
    assert_eq!(report4.repairs.len(), 1);
    assert_eq!(
        report4.repairs[0],
        Repair {
            line: 2,
            column: 6,
            fix: SafeFix::MissingFinalPipe,
            original: ">rope".to_string(),
            new_text: ">rope|".to_string(),
        }
    );
    assert!(report4.bad_lines.is_empty());
    assert_eq!(report4.text, "Attention|\n>rope|\n");

    // 5. Hope this helps! at the end is a record under {text}, and chatter under {year:int}
    let schema_text = "schema:doc{text}";
    let input_text = "first record|\nHope this helps!\n";
    let report_text = doctor(schema_text, input_text, &options);
    assert_eq!(report_text.repairs.len(), 1);
    assert_eq!(
        report_text.repairs[0],
        Repair {
            line: 2,
            column: 17,
            fix: SafeFix::MissingFinalPipe,
            original: "Hope this helps!".to_string(),
            new_text: "Hope this helps!|".to_string(),
        }
    );
    assert!(report_text.bad_lines.is_empty());
    assert_eq!(report_text.text, "first record|\nHope this helps!|\n");

    let schema_year = "schema:claim{year:int}";
    let input_year = "2024|\nHope this helps!\n";
    let report_year = doctor(schema_year, input_year, &options);
    assert_eq!(report_year.repairs.len(), 1);
    assert_eq!(
        report_year.repairs[0],
        Repair {
            line: 2,
            column: 1,
            fix: SafeFix::Chatter,
            original: "Hope this helps!".to_string(),
            new_text: String::new(),
        }
    );
    assert!(report_year.bad_lines.is_empty());
    assert_eq!(report_year.text, "2024|\n");

    // 6. A cut-off last line 2024|Rope scal becomes a valid record, with the fix-4 repair reported
    let schema6 = "schema:claim{year:int, title}";
    let input6 = "2024|Rope scal";
    let report6 = doctor(schema6, input6, &options);
    assert_eq!(report6.repairs.len(), 1);
    assert_eq!(
        report6.repairs[0],
        Repair {
            line: 1,
            column: 15,
            fix: SafeFix::MissingFinalPipe,
            original: "2024|Rope scal".to_string(),
            new_text: "2024|Rope scal|".to_string(),
        }
    );
    assert!(report6.bad_lines.is_empty());
    assert_eq!(report6.text, "2024|Rope scal|\n");
}

#[test]
fn test_splice_after_multiline_raw_record() {
    let schema = "schema:entry{id:int, code:str, flag:int}";
    let options = DoctorOptions::default();

    // Bad line 1 is on lines 2-4 (multi-line raw record with invalid flag '+99').
    // Bad line 2 is on line 5 (invalid flag 'bad_flag').
    let input = "1|first|10|\n2|\\rline 1\nline 2\nline 3|+99|\n3|third|bad_flag|\n";
    let report = doctor(schema, input, &options);
    assert_eq!(report.bad_lines.len(), 2);
    assert_eq!(report.bad_lines[0].line, 4);
    assert_eq!(report.bad_lines[1].line, 5);

    let reply = "2|fixed|20|\n3|third|30|";
    let spliced = splice(schema, input, &report, reply, &options).expect("Splice succeeded");
    assert!(spliced.bad_lines.is_empty());
    assert_eq!(
        spliced.text,
        "1|first|10|\n2|fixed|20|\n3|third|30|\n"
    );
}

// =========================================================================
// 6. The damage suite (at least 5,000 cases)
// =========================================================================

use common::generator::{generate_random_file, GenConfig, GeneratedFile};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::panic::{catch_unwind, AssertUnwindSafe};
use structl::{decode_with, encode_with, FieldDef, FieldType, ListItemType, ScalarType};

// ---------- layout of the canonical text ----------

#[derive(Clone, Debug, PartialEq)]
enum Col {
    Str,
    Int,
    Float,
    Bool,
    Enum(Vec<String>),
}

#[derive(Clone, Debug)]
struct Fld {
    start: usize,
    end: usize,
    raw: bool,
}

#[derive(Clone, Debug)]
struct Ent {
    text: String,
    schema: bool,
    depth: usize,
    fields: Vec<Fld>,
    cols: Vec<Col>,
    prefix: usize,
    own_lists: usize,
    parent_lists: usize,
    last_desc: usize,
    line: usize,
}

struct RawEnt {
    text: String,
    schema: bool,
    depth: usize,
    fields: Vec<Fld>,
}

fn tokenize(text: &str) -> Vec<RawEnt> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < b.len() {
        let line_end = text[pos..].find('\n').map(|i| pos + i).unwrap_or(b.len());
        let first_line = &text[pos..line_end];
        if first_line.starts_with("schema:") && !first_line.contains('|') {
            out.push(RawEnt {
                text: first_line.to_string(),
                schema: true,
                depth: 0,
                fields: Vec::new(),
            });
            pos = line_end + 1;
            continue;
        }
        let mut depth = 0;
        let mut p = pos;
        while p < b.len() && b[p] == b'>' {
            depth += 1;
            p += 1;
        }
        let mut fields = Vec::new();
        loop {
            let fs = p;
            let raw = p + 1 < b.len() && b[p] == b'\\' && b[p + 1] == b'r';
            if raw {
                p += 2;
            }
            while p < b.len() {
                match b[p] {
                    b'\\' => p += 2,
                    b'|' => break,
                    _ => p += 1,
                }
            }
            assert!(p < b.len(), "layout: unterminated value in canonical text");
            fields.push(Fld {
                start: fs - pos,
                end: p - pos,
                raw,
            });
            p += 1;
            if p >= b.len() || b[p] == b'\n' {
                break;
            }
        }
        out.push(RawEnt {
            text: text[pos..p].to_string(),
            schema: false,
            depth,
            fields,
        });
        pos = p + 1;
    }
    out
}

struct Node {
    depth: usize,
    cols: Vec<Col>,
    own_lists: usize,
}

fn scalar_col(s: &ScalarType) -> Col {
    match s {
        ScalarType::Str => Col::Str,
        ScalarType::Int => Col::Int,
        ScalarType::Float => Col::Float,
        ScalarType::Bool => Col::Bool,
    }
}

fn flatten_cols(fields: &[FieldDef], out: &mut Vec<Col>) {
    for f in fields {
        match &f.field_type {
            FieldType::Scalar(s) => out.push(scalar_col(s)),
            FieldType::Enum(l) => out.push(Col::Enum(l.clone())),
            FieldType::Record(r) => flatten_cols(&r.fields, out),
            FieldType::List(_) => {}
        }
    }
}

fn count_lists(fields: &[FieldDef]) -> usize {
    fields
        .iter()
        .map(|f| match &f.field_type {
            FieldType::List(_) => 1,
            FieldType::Record(r) => count_lists(&r.fields),
            _ => 0,
        })
        .sum()
}

fn collect_lists<'a>(
    fields: &'a [FieldDef],
    val: &'a Value,
    out: &mut Vec<(&'a ListItemType, &'a [Value])>,
) {
    for f in fields {
        match &f.field_type {
            FieldType::List(it) => {
                let items = val
                    .get_field(&f.name)
                    .and_then(|v| v.as_list())
                    .unwrap_or(&[]);
                out.push((&**it, items));
            }
            FieldType::Record(sub) => {
                if count_lists(&sub.fields) > 0 {
                    if let Some(sv) = val.get_field(&f.name) {
                        collect_lists(&sub.fields, sv, out);
                    }
                }
            }
            _ => {}
        }
    }
}

fn walk_node(fields: &[FieldDef], val: &Value, depth: usize, out: &mut Vec<Node>) {
    let mut cols = Vec::new();
    flatten_cols(fields, &mut cols);
    out.push(Node {
        depth,
        cols,
        own_lists: count_lists(fields),
    });
    let mut lists = Vec::new();
    collect_lists(fields, val, &mut lists);
    for (it, items) in lists {
        for item in items {
            match it {
                ListItemType::Scalar(s) => out.push(Node {
                    depth: depth + 1,
                    cols: vec![scalar_col(s)],
                    own_lists: 0,
                }),
                ListItemType::Enum(l) => out.push(Node {
                    depth: depth + 1,
                    cols: vec![Col::Enum(l.clone())],
                    own_lists: 0,
                }),
                ListItemType::Record(r) => walk_node(&r.fields, item, depth + 1, out),
            }
        }
    }
}

fn build_layout(gf: &GeneratedFile) -> Vec<Ent> {
    let raw = tokenize(&gf.text);
    let mut nodes = Vec::new();
    for block in &gf.doc.blocks {
        let class = &gf.doc.schemas[&block.class_name];
        for rec in &block.records {
            walk_node(&class.fields, rec, 0, &mut nodes);
        }
    }
    let mut nodes = nodes.into_iter();
    let mut ents: Vec<Ent> = Vec::new();
    let mut line = 1;
    for r in raw {
        let nlines = r.text.matches('\n').count() + 1;
        let idx = ents.len();
        if r.schema {
            ents.push(Ent {
                text: r.text,
                schema: true,
                depth: 0,
                fields: Vec::new(),
                cols: Vec::new(),
                prefix: 0,
                own_lists: 0,
                parent_lists: 0,
                last_desc: idx,
                line,
            });
        } else {
            let node = nodes.next().expect("layout: more lines than data nodes");
            assert_eq!(node.depth, r.depth, "layout: depth mismatch at line {}", line);
            ents.push(Ent {
                text: r.text,
                schema: false,
                depth: r.depth,
                fields: r.fields,
                cols: node.cols,
                prefix: 0,
                own_lists: node.own_lists,
                parent_lists: 0,
                last_desc: idx,
                line,
            });
        }
        line += nlines;
    }
    assert!(nodes.next().is_none(), "layout: fewer lines than data nodes");

    let mut stack: Vec<usize> = Vec::new();
    for i in 0..ents.len() {
        if ents[i].schema {
            stack.clear();
            continue;
        }
        let d = ents[i].depth;
        assert!(d <= stack.len(), "layout: depth jump in canonical text");
        stack.truncate(d);
        if d > 0 {
            ents[i].parent_lists = ents[stack[d - 1]].own_lists;
        }
        stack.push(i);
        ents[i].prefix = if d >= 1 && ents[i].parent_lists >= 2 { 1 } else { 0 };
        assert_eq!(
            ents[i].fields.len(),
            ents[i].prefix + ents[i].cols.len(),
            "layout: column count mismatch at line {}",
            ents[i].line
        );
    }
    let n = ents.len();
    for i in 0..n {
        let mut j = i;
        while j + 1 < n && !ents[j + 1].schema && ents[j + 1].depth > ents[i].depth && !ents[i].schema {
            j += 1;
        }
        ents[i].last_desc = j;
    }
    ents
}

// ---------- shared helpers ----------

const CASES_PER_ROW: usize = 200;

fn hash_of<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut h = DefaultHasher::new();
    t.hash(&mut h);
    h.finish()
}

fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

fn pick_idx(rng: &mut SimpleRng, len: usize) -> usize {
    rng.gen_range(0, len - 1)
}

fn gt_count(s: &str) -> usize {
    s.chars().take_while(|&c| c == '>').count()
}

fn index_like(field_text: &str) -> bool {
    let t = field_text.trim_start_matches("\\r").trim();
    !t.is_empty() && t.chars().all(|c| c.is_ascii_digit() || c == '.')
}

fn lines_of(texts: &[String]) -> Vec<usize> {
    let mut v = Vec::new();
    let mut l = 1;
    for t in texts {
        v.push(l);
        l += t.matches('\n').count() + 1;
    }
    v
}

fn locate(text: &str, off: usize) -> (usize, usize) {
    let pre = &text[..off];
    let extra = pre.matches('\n').count();
    let ls = pre.rfind('\n').map(|i| i + 1).unwrap_or(0);
    (extra, pre[ls..].chars().count() + 1)
}

fn decode_blocks(schemas: &str, text: &str) -> Result<Vec<Block>, String> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    match decode_with(schemas, text) {
        Ok(d) => Ok(d.blocks.into_iter().filter(|b| !b.records.is_empty()).collect()),
        Err(e) => Err(format!("{:?}", e)),
    }
}

fn type_error(col: &Col, v: &str) -> String {
    match col {
        Col::Str => unreachable!("text has no type error"),
        Col::Int => {
            if v.starts_with('+') {
                format!("leading '+' is not allowed in int '{}'", v)
            } else {
                format!("'{}' is not a valid int (i64)", v)
            }
        }
        Col::Float => format!("'{}' is not a valid float (f64)", v),
        Col::Bool => format!("'{}' is not a valid bool (expected 1 or 0)", v),
        Col::Enum(l) => format!("'{}' is not one of: {}", v, l.join(", ")),
    }
}

const EMPTY_FIELD: &str = "empty field is not allowed (use ~ for null)";

// ---------- fixable damage ----------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Fix {
    Fences,
    Echo,
    Chatter,
    Pipe,
}

const ALL_FIXES: [Fix; 4] = [Fix::Fences, Fix::Echo, Fix::Chatter, Fix::Pipe];

fn fix_name(f: Fix) -> &'static str {
    match f {
        Fix::Fences => "code fences",
        Fix::Echo => "echoed schema",
        Fix::Chatter => "chatter",
        Fix::Pipe => "missing final pipe",
    }
}

#[derive(Clone, Debug)]
enum Role {
    Entry,
    Chatter,
    Fence,
    Echo,
    PipeDamaged(String),
}

struct ExpRepair {
    line: usize,
    kinds: Vec<SafeFix>,
    original: String,
    new_text: String,
}

struct Damaged {
    text: String,
    repairs: Vec<ExpRepair>,
    expected: Vec<Block>,
}

enum Built {
    NotApplicable,
    Case(Damaged),
}


const CHATTER: [&str; 12] = [
    "Here is the data you asked for:",
    "Sure, here are the records.",
    "Hope this helps!",
    "Let me know if you need more.",
    "Output:",
    "Done.",
    "Below are the records in Struct format:",
    "Note: nothing was invented.",
    "> a quoted remark from the model",
    "- a bullet remark",
    "  Here you go  ",
    "I found these entries.",
];

const FENCES: [&str; 6] = ["```", "```stc", "```struct", "```text", "  ```", "\t```"];

fn echo_variant(rng: &mut SimpleRng, gf: &GeneratedFile) -> String {
    let lines: Vec<&str> = gf.schemas.lines().collect();
    let l = lines[pick_idx(rng, lines.len())];
    match rng.gen_range(0, 3) {
        0 => l.to_string(),
        1 => l.replace(", ", ","),
        2 => format!("  {}  ", l),
        _ => l.replace(", ", ",  "),
    }
}

fn shuffle<T>(rng: &mut SimpleRng, v: &mut Vec<T>) {
    for i in (1..v.len()).rev() {
        let j = rng.gen_range(0, i);
        v.swap(i, j);
    }
}

fn eligible_pipe(e: &Ent) -> bool {
    !e.schema
        && !e.text.contains('\n')
        && e.text.ends_with('|')
        && e.text[..e.text.len() - 1].contains('|')
        && !e.fields.last().map(|f| f.raw).unwrap_or(true)
}

fn compose_fixable(rng: &mut SimpleRng, gf: &GeneratedFile, ents: &[Ent], fixes: &[Fix]) -> Built {
    let has = |f: Fix| fixes.contains(&f);
    let n = ents.len();

    let mut pipe_set: Vec<usize> = Vec::new();
    if has(Fix::Pipe) {
        let mut elig: Vec<usize> = (0..n).filter(|&i| eligible_pipe(&ents[i])).collect();
        if elig.is_empty() {
            return Built::NotApplicable;
        }
        let k = rng.gen_range(1, 3.min(elig.len()));
        for _ in 0..k {
            let j = pick_idx(rng, elig.len());
            pipe_set.push(elig.remove(j));
        }
    }

    let mut body: Vec<(String, Role)> = Vec::new();
    for i in 0..n {
        let t = ents[i].text.clone();
        if pipe_set.contains(&i) {
            let damaged = t[..t.len() - 1].to_string();
            body.push((damaged, Role::PipeDamaged(t)));
        } else {
            body.push((t, Role::Entry));
        }
    }
    let m = body.len();

    let mut lead: Vec<(String, Role)> = Vec::new();
    let mut trail: Vec<(String, Role)> = Vec::new();
    let mut mids: Vec<Vec<(String, Role)>> = (0..=m).map(|_| Vec::new()).collect();

    if has(Fix::Chatter) {
        let first_class = gf.doc.blocks.first().map(|b| b.class_name.as_str()).unwrap_or("");
        let last_class = gf.doc.blocks.last().map(|b| b.class_name.as_str()).unwrap_or("");
        let check_cand = |ch: &str, cls: &str| -> bool {
            let cand = format!("schema:{}\n{}|\n", cls, ch);
            decode_blocks(&gf.schemas, &cand).is_ok()
                || decode_blocks(&gf.schemas, &format!("{}{}|\n", gf.text, ch)).is_ok()
        };

        let (mut a, mut b) = (rng.gen_range(0, 3), rng.gen_range(0, 3));
        if a + b == 0 {
            if rng.gen_bool(0.5) {
                a = 1;
            } else {
                b = 1;
            }
        }
        for _ in 0..a {
            let ch = CHATTER[pick_idx(rng, CHATTER.len())].to_string();
            if check_cand(&ch, first_class) {
                return Built::NotApplicable;
            }
            lead.push((ch, Role::Chatter));
        }
        for _ in 0..b {
            let ch = CHATTER[pick_idx(rng, CHATTER.len())].to_string();
            if check_cand(&ch, last_class) {
                return Built::NotApplicable;
            }
            trail.push((ch, Role::Chatter));
        }
    }

    if has(Fix::Fences) {
        let cnt = rng.gen_range(1, 3);
        for _ in 0..cnt {
            let line = FENCES[pick_idx(rng, FENCES.len())].to_string();
            let place = rng.gen_range(0, 2);
            match place {
                0 => lead.push((line, Role::Fence)),
                1 => {
                    let b = rng.gen_range(0, m);
                    mids[b].push((line, Role::Fence));
                }
                _ => trail.push((line, Role::Fence)),
            }
        }
    }

    if has(Fix::Echo) {
        let cnt = rng.gen_range(1, 2);
        for _ in 0..cnt {
            let line = echo_variant(rng, gf);
            let b = rng.gen_range(0, m);
            mids[b].push((line, Role::Echo));
        }
    }

    shuffle(rng, &mut lead);
    shuffle(rng, &mut trail);
    for v in mids.iter_mut() {
        shuffle(rng, v);
    }

    let mut items: Vec<(String, Role)> = lead;
    let mut body_it = body.into_iter();
    for b in 0..=m {
        items.append(&mut mids[b]);
        if b < m {
            items.push(body_it.next().unwrap());
        }
    }
    items.extend(trail);

    let mut repairs = Vec::new();
    let mut line = 1;
    let mut out = String::new();
    for (i, (t, role)) in items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(t);
        let (kinds, orig, newt): (Vec<SafeFix>, String, String) = match role {
            Role::Entry => (vec![], String::new(), String::new()),
            Role::Chatter => (vec![SafeFix::Chatter], t.clone(), String::new()),
            Role::Fence => (vec![SafeFix::CodeFences], t.clone(), String::new()),
            Role::Echo => (vec![SafeFix::EchoedSchema], t.clone(), String::new()),
            Role::PipeDamaged(orig_line) => (
                vec![SafeFix::MissingFinalPipe],
                t.clone(),
                orig_line.clone(),
            ),
        };
        if !kinds.is_empty() {
            repairs.push(ExpRepair {
                line,
                kinds,
                original: orig,
                new_text: newt,
            });
        }
        line += t.matches('\n').count() + 1;
    }
    out.push('\n');

    Built::Case(Damaged {
        text: out,
        repairs,
        expected: gf.doc.blocks.clone(),
    })
}

// ---------- Cut-off damage (R3f) ----------

struct CutField {
    start: usize,
    val: String,
    raw: bool,
    is_null: bool,
    invalid_escape: Option<(char, usize)>,
}

fn parse_cut_line_fields(line_text: &str) -> (usize, Vec<CutField>) {
    let bytes = line_text.as_bytes();
    let mut p = 0;
    let mut depth = 0;
    while p < bytes.len() && bytes[p] == b'>' {
        depth += 1;
        p += 1;
    }
    let mut fields = Vec::new();
    while p < bytes.len() {
        while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
            p += 1;
        }
        if p >= bytes.len() {
            break;
        }
        let f_start = p;
        let is_raw = p + 1 < bytes.len() && bytes[p] == b'\\' && bytes[p + 1] == b'r';
        if is_raw {
            p += 2;
            let mut raw_val = String::new();
            let mut invalid_escape = None;
            while p < bytes.len() {
                if bytes[p] == b'\\' {
                    if p + 1 < bytes.len() {
                        let nc = bytes[p + 1] as char;
                        if nc == '|' || nc == '\\' {
                            raw_val.push(nc);
                            p += 2;
                        } else {
                            if invalid_escape.is_none() {
                                invalid_escape = Some((nc, p + 1));
                            }
                            raw_val.push('\\');
                            raw_val.push(nc);
                            p += 2;
                        }
                    } else {
                        raw_val.push('\\');
                        p += 1;
                    }
                } else if bytes[p] == b'|' {
                    break;
                } else {
                    raw_val.push(bytes[p] as char);
                    p += 1;
                }
            }
            if p < bytes.len() && bytes[p] == b'|' {
                p += 1;
            }
            fields.push(CutField {
                start: f_start,
                val: raw_val,
                raw: true,
                is_null: false,
                invalid_escape,
            });
        } else {
            let mut val_chars = Vec::new();
            let mut in_escape = false;
            let mut invalid_escape = None;
            let mut closed = false;
            while p < bytes.len() {
                let c = bytes[p] as char;
                if in_escape {
                    in_escape = false;
                    match c {
                        '|' | '\\' | '~' | '>' | '#' => val_chars.push(c),
                        other => {
                            if invalid_escape.is_none() {
                                invalid_escape = Some((other, p));
                            }
                            val_chars.push('\\');
                            val_chars.push(other);
                        }
                    }
                    p += 1;
                    continue;
                }
                if c == '\\' {
                    in_escape = true;
                    p += 1;
                    continue;
                }
                if c == '|' {
                    closed = true;
                    p += 1;
                    break;
                }
                val_chars.push(c);
                p += 1;
            }
            let f_end = if closed { p - 1 } else { p };
            let raw_slice = &line_text[f_start..f_end];
            let is_null = raw_slice.trim() == "~";
            let val_str: String = val_chars.into_iter().collect();
            let trimmed = val_str.trim_end_matches(|c| c == ' ' || c == '\t').to_string();
            fields.push(CutField {
                start: f_start,
                val: trimmed,
                raw: false,
                is_null,
                invalid_escape,
            });
        }
    }
    (depth, fields)
}

fn validate_col_type(col: &Col, val: &str, is_raw: bool, is_null: bool) -> Result<(), String> {
    if is_null {
        return Ok(());
    }
    if is_raw {
        match col {
            Col::Str => {
                if val.is_empty() {
                    return Err(EMPTY_FIELD.to_string());
                }
                return Ok(());
            }
            _ => return Err("raw value '\\r' is not allowed in non-str field".to_string()),
        }
    }
    let t = val.trim();
    if t.is_empty() {
        return Err(EMPTY_FIELD.to_string());
    }
    match col {
        Col::Str => Ok(()),
        Col::Int => {
            if t.starts_with('+') {
                Err(format!("leading '+' is not allowed in int '{}'", t))
            } else {
                t.parse::<i64>()
                    .map(|_| ())
                    .map_err(|_| format!("'{}' is not a valid int (i64)", t))
            }
        }
        Col::Float => {
            if t == "NaN" || t == "inf" || t == "-inf" || t.parse::<f64>().is_ok() {
                Ok(())
            } else {
                Err(format!("'{}' is not a valid float (f64)", t))
            }
        }
        Col::Bool => {
            if t == "1" || t == "0" {
                Ok(())
            } else {
                Err(format!("'{}' is not a valid bool (expected 1 or 0)", t))
            }
        }
        Col::Enum(labels) => {
            if labels.contains(&t.to_string()) {
                Ok(())
            } else {
                Err(format!("'{}' is not one of: {}", t, labels.join(", ")))
            }
        }
    }
}

fn spec_classify_cut_bad_line(
    ents: &[Ent],
    c: usize,
    cut_text: &str,
    new_text: &str,
) -> BadLine {
    let line = ents[c].line;

    let trailing_bs = cut_text
        .trim_end()
        .chars()
        .rev()
        .take_while(|&ch| ch == '\\')
        .count();
    if trailing_bs % 2 == 1 {
        let last_line = new_text.split('\n').last().unwrap_or(new_text);
        return BadLine {
            line: ents[c].line + cut_text.matches('\n').count(),
            column: last_line.trim_end().chars().count() + 1,
            problem: "data line must always end with an unescaped '|'".to_string(),
            original: new_text.to_string(),
            dropped_as_child: false,
        };
    }

    let (depth, fields) = parse_cut_line_fields(new_text);

    if depth == 0 {
        let cols = &ents[c].cols;
        if fields.len() != cols.len() {
            return BadLine {
                line,
                column: 1,
                problem: format!("expected {} columns, found {}", cols.len(), fields.len()),
                original: new_text.to_string(),
                dropped_as_child: false,
            };
        }
        for (ci, f) in fields.iter().enumerate() {
            if let Some((nc, esc_col)) = f.invalid_escape {
                return BadLine {
                    line,
                    column: esc_col + 1,
                    problem: format!("invalid escape sequence '\\{}'", nc),
                    original: new_text.to_string(),
                    dropped_as_child: false,
                };
            }
            if let Err(e) = validate_col_type(&cols[ci], &f.val, f.raw, f.is_null) {
                return BadLine {
                    line,
                    column: f.start + 1,
                    problem: e,
                    original: new_text.to_string(),
                    dropped_as_child: false,
                };
            }
        }
    } else {
        let parent_lists = ents[c].parent_lists;
        if parent_lists == 1 {
            let cols = &ents[c].cols;
            let mut used_index = false;
            if fields.len() == cols.len() + 1 && index_like(&fields[0].val) {
                used_index = true;
            }
            let actual_cols = if used_index {
                fields.len().saturating_sub(1)
            } else {
                fields.len()
            };
            let data_fields = if used_index { &fields[1..] } else { &fields[..] };
            if actual_cols != cols.len() {
                return BadLine {
                    line,
                    column: depth + 1,
                    problem: format!("expected {} columns, found {}", cols.len(), actual_cols),
                    original: new_text.to_string(),
                    dropped_as_child: false,
                };
            }
            for (ci, f) in data_fields.iter().enumerate() {
                if let Some((nc, esc_col)) = f.invalid_escape {
                    return BadLine {
                        line,
                        column: esc_col + 1,
                        problem: format!("invalid escape sequence '\\{}'", nc),
                        original: new_text.to_string(),
                        dropped_as_child: false,
                    };
                }
                if let Err(e) = validate_col_type(&cols[ci], &f.val, f.raw, f.is_null) {
                    return BadLine {
                        line,
                        column: f.start + 1,
                        problem: e,
                        original: new_text.to_string(),
                        dropped_as_child: false,
                    };
                }
            }
        } else {
            if fields.is_empty() {
                return BadLine {
                    line,
                    column: depth + 1,
                    problem: "missing list index".to_string(),
                    original: new_text.to_string(),
                    dropped_as_child: false,
                };
            }
            let idx_str = fields[0].val.trim();
            let orig_idx = &ents[c].text[ents[c].fields[0].start..ents[c].fields[0].end];
            if idx_str == orig_idx {
                let cols = &ents[c].cols;
                let actual_cols = fields.len().saturating_sub(1);
                let data_fields = &fields[1..];
                if actual_cols != cols.len() {
                    return BadLine {
                        line,
                        column: depth + 1,
                        problem: format!("expected {} columns, found {}", cols.len(), actual_cols),
                        original: new_text.to_string(),
                        dropped_as_child: false,
                    };
                }
                for (ci, f) in data_fields.iter().enumerate() {
                    if let Some((nc, esc_col)) = f.invalid_escape {
                        return BadLine {
                            line,
                            column: esc_col + 1,
                            problem: format!("invalid escape sequence '\\{}'", nc),
                            original: new_text.to_string(),
                            dropped_as_child: false,
                        };
                    }
                    if let Err(e) = validate_col_type(&cols[ci], &f.val, f.raw, f.is_null) {
                        return BadLine {
                            line,
                            column: f.start + 1,
                            problem: e,
                            original: new_text.to_string(),
                            dropped_as_child: false,
                        };
                    }
                }
            } else {
                return BadLine {
                    line,
                    column: fields[0].start + 1,
                    problem: "missing list index".to_string(),
                    original: new_text.to_string(),
                    dropped_as_child: false,
                };
            }
        }
    }

    panic!("spec_classify_cut_bad_line: no error found on line {:?}", new_text);
}

struct CutCase {
    text: String,
    repair: Repair,
    bad_line: Option<BadLine>,
    salvage_blocks: Vec<Block>,
    strict_blocks: Option<Vec<Block>>,
    truncated_but_valid: bool,
}

enum BuiltCut {
    NotApplicable,
    Case(CutCase),
}

fn has_open_raw_at_end(text: &str) -> bool {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() && b[i] == b'>' {
        i += 1;
    }
    let mut in_raw = false;
    while i < b.len() {
        while i < b.len() && (b[i] == b' ' || b[i] == b'\t' || b[i] == b'\r' || b[i] == b'\n') {
            i += 1;
        }
        if i >= b.len() {
            break;
        }
        if i + 1 < b.len() && b[i] == b'\\' && b[i + 1] == b'r' {
            in_raw = true;
            i += 2;
            while i < b.len() {
                if b[i] == b'\\' {
                    i += 2;
                } else if b[i] == b'|' {
                    in_raw = false;
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
        } else {
            in_raw = false;
            while i < b.len() {
                if b[i] == b'\\' {
                    i += 2;
                } else if b[i] == b'|' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
        }
    }
    in_raw
}

fn plan_cut_r3f(rng: &mut SimpleRng, gf: &GeneratedFile, ents: &[Ent]) -> BuiltCut {
    let data: Vec<usize> = (0..ents.len())
        .filter(|&i| !ents[i].schema && ents[i].text.chars().count() >= 2)
        .collect();
    if data.is_empty() {
        return BuiltCut::NotApplicable;
    }
    let raws: Vec<usize> = data
        .iter()
        .copied()
        .filter(|&i| ents[i].fields.iter().any(|f| f.raw))
        .collect();
    let want_raw = !raws.is_empty() && rng.gen_bool(0.5);
    let c = if want_raw {
        raws[pick_idx(rng, raws.len())]
    } else {
        data[pick_idx(rng, data.len())]
    };
    let e = &ents[c];
    let total = e.text.chars().count();
    let mut p = if want_raw {
        let rf: Vec<&Fld> = e.fields.iter().filter(|f| f.raw).collect();
        let f = rf[pick_idx(rng, rf.len())];
        let cs = e.text[..f.start].chars().count();
        let ce = e.text[..f.end].chars().count();
        rng.gen_range(cs + 1, ce.min(total - 1)).max(1)
    } else {
        rng.gen_range(1, total - 1)
    };
    let chars: Vec<char> = e.text.chars().collect();
    while p > 1 && (chars[p - 1] == '|' || chars[p - 1] == '\n' || chars[p - 1] == '\r') {
        p -= 1;
    }
    let mut cut_preview: String = chars[..p].iter().collect();
    while p > 1 && (cut_preview.trim_end().ends_with('|') || cut_preview.ends_with('\n') || cut_preview.ends_with('\r')) {
        p -= 1;
        cut_preview = chars[..p].iter().collect();
    }
    let byte = e.text.char_indices().nth(p).map(|(i, _)| i).unwrap_or(e.text.len());
    let cut_text = e.text[..byte].to_string();

    let mut prefix_text = String::new();
    for x in &ents[..c] {
        prefix_text.push_str(&x.text);
        prefix_text.push('\n');
    }
    let damaged_text = format!("{}{}", prefix_text, cut_text);

    let is_raw = has_open_raw_at_end(&cut_text);
    let has_pipe = cut_text.contains('|');

    if !has_pipe && !is_raw {
        let cand = format!("{}{}|\n", prefix_text, cut_text);
        if let Ok(blocks) = decode_blocks(&gf.schemas, &cand) {
            let repair = Repair {
                line: ents[c].line,
                column: cut_text.trim_end().chars().count() + 1,
                fix: SafeFix::MissingFinalPipe,
                original: cut_text.clone(),
                new_text: format!("{}|", cut_text.trim_end()),
            };
            BuiltCut::Case(CutCase {
                text: damaged_text,
                repair,
                bad_line: None,
                salvage_blocks: blocks.clone(),
                strict_blocks: Some(blocks),
                truncated_but_valid: true,
            })
        } else {
            let repair = Repair {
                line: ents[c].line,
                column: 1,
                fix: SafeFix::Chatter,
                original: cut_text.clone(),
                new_text: String::new(),
            };
            let salvage_blocks = decode_blocks(&gf.schemas, &prefix_text).unwrap_or_default();
            BuiltCut::Case(CutCase {
                text: damaged_text,
                repair,
                bad_line: None,
                salvage_blocks: salvage_blocks.clone(),
                strict_blocks: Some(salvage_blocks),
                truncated_but_valid: false,
            })
        }
    } else {
        let last_line = cut_text.split('\n').last().unwrap_or(&cut_text);
        let (new_text, col) = if is_raw {
            (format!("{}|", cut_text), last_line.chars().count() + 1)
        } else {
            (format!("{}|", cut_text.trim_end()), last_line.trim_end().chars().count() + 1)
        };
        let rep_line = ents[c].line + cut_text.matches('\n').count();
        let repair = Repair {
            line: rep_line,
            column: col,
            fix: SafeFix::MissingFinalPipe,
            original: cut_text.clone(),
            new_text: new_text.clone(),
        };
        let cand = format!("{}{}\n", prefix_text, new_text);
        if let Ok(blocks) = decode_blocks(&gf.schemas, &cand) {
            BuiltCut::Case(CutCase {
                text: damaged_text,
                repair,
                bad_line: None,
                salvage_blocks: blocks.clone(),
                strict_blocks: Some(blocks),
                truncated_but_valid: true,
            })
        } else {
            let bad_line = spec_classify_cut_bad_line(ents, c, &cut_text, &new_text);
            let salvage_blocks = decode_blocks(&gf.schemas, &prefix_text).unwrap_or_default();
            BuiltCut::Case(CutCase {
                text: damaged_text,
                repair,
                bad_line: Some(bad_line),
                salvage_blocks,
                strict_blocks: None,
                truncated_but_valid: false,
            })
        }
    }
}

fn check_cut_case(schemas: &str, c: &CutCase) -> Result<Outcome, CaseErr> {
    let plain = |m: String| CaseErr { false_repair: false, msg: m };

    // Salvage check
    let salvage = doctor(schemas, &c.text, &salvage_opts());
    if salvage.repairs.len() != 1 {
        return Err(plain(format!("salvage: expected 1 repair, got {}", salvage.repairs.len())));
    }
    let sr = &salvage.repairs[0];
    if sr.line != c.repair.line
        || sr.fix != c.repair.fix
        || sr.original != c.repair.original
        || sr.new_text != c.repair.new_text
    {
        return Err(plain(format!(
            "salvage: repair mismatch\n  expected: line {} fix {:?} orig {:?} new {:?}\n  got:      line {} fix {:?} orig {:?} new {:?}",
            c.repair.line, c.repair.fix, c.repair.original, c.repair.new_text, sr.line, sr.fix, sr.original, sr.new_text
        )));
    }
    match &c.bad_line {
        None => {
            if !salvage.bad_lines.is_empty() {
                return Err(plain(format!("salvage: unexpected bad lines: {:?}", salvage.bad_lines)));
            }
        }
        Some(expected_bad) => {
            if salvage.bad_lines.len() != 1 || salvage.bad_lines[0] != *expected_bad {
                return Err(plain(format!(
                    "salvage: bad line mismatch\n  expected: {:?}\n  got:      {:?}",
                    expected_bad, salvage.bad_lines
                )));
            }
            if let Some(h) = &salvage.hint {
                let want = format!("{}   ({})", expected_bad.original, expected_bad.problem);
                if !h.contains(&want) {
                    return Err(plain(format!("hint missing {:?} in {:?}", want, h)));
                }
            } else {
                return Err(plain("salvage: missing retry hint".to_string()));
            }
        }
    }
    check_data(schemas, &salvage, &c.salvage_blocks, "salvage")?;

    // Strict check
    let strict = doctor(schemas, &c.text, &strict_opts());
    if strict.repairs.len() != 1 {
        return Err(plain(format!("strict: expected 1 repair, got {}", strict.repairs.len())));
    }
    let str_rep = &strict.repairs[0];
    if str_rep.line != c.repair.line
        || str_rep.fix != c.repair.fix
        || str_rep.original != c.repair.original
        || str_rep.new_text != c.repair.new_text
    {
        return Err(plain(format!(
            "strict: repair mismatch\n  expected: {:?}\n  got: {:?}",
            c.repair, str_rep
        )));
    }
    match &c.strict_blocks {
        Some(expected_strict) => {
            if !strict.bad_lines.is_empty() {
                return Err(plain(format!("strict: unexpected bad lines: {:?}", strict.bad_lines)));
            }
            check_data(schemas, &strict, expected_strict, "strict")?;
        }
        None => {
            if strict.data.is_some() || !strict.text.is_empty() {
                return Err(plain("strict: expected no data with bad lines".to_string()));
            }
        }
    }

    Ok(Outcome {
        exact: !c.truncated_but_valid && c.bad_line.is_none(),
        truncated_but_valid: c.truncated_but_valid,
        bad: if c.bad_line.is_some() { 1 } else { 0 },
        repairs: 1,
    })
}

// ---------- unfixable damage ----------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum UK {
    Few,
    Many,
    Pipe,
    EnumTypo,
    Depth,
    MissIdx,
    NoList,
    UnclosedRaw,
    Esc,
    Quoted,
    Plus,
    Comma,
    Caps,
    Prefix,
    Null,
    NoneWord,
    NA,
    NaN,
    True,
    Yes,
    Empty,
    DecComma,
}

const ALL_UK: [(UK, &str); 22] = [
    (UK::Few, "Wrong column count (too few)"),
    (UK::Many, "Wrong column count (too many)"),
    (UK::Pipe, "Unescaped pipe inside text"),
    (UK::EnumTypo, "Enum typo"),
    (UK::Depth, "Depth jump"),
    (UK::MissIdx, "Missing list index"),
    (UK::NoList, "Child line, parent has no list"),
    (UK::UnclosedRaw, "Unclosed raw value mid-file"),
    (UK::Esc, "Invalid escape (\\q) in text"),
    (UK::Quoted, "Quoted int (\"2024\")"),
    (UK::Plus, "Leading plus in int (+5)"),
    (UK::Comma, "Comma in int (1,000)"),
    (UK::Caps, "Enum wrong case (Supported)"),
    (UK::Prefix, "Field-name prefix (year: 2024)"),
    (UK::Null, "Literal null in non-text"),
    (UK::NoneWord, "Literal None in non-text"),
    (UK::NA, "Literal N/A in non-text"),
    (UK::NaN, "NaN in int/bool/enum"),
    (UK::True, "true in bool"),
    (UK::Yes, "yes in bool"),
    (UK::Empty, "Empty field (two pipes in a row)"),
    (UK::DecComma, "Decimal comma in float (3,14)"),
];

struct Unf {
    texts: Vec<String>,
    bad: Vec<BadLine>,
    removed: Vec<usize>,
}

fn finish(
    ents: &[Ent],
    e: usize,
    new_text: String,
    err: (usize, usize),
    problem: String,
    merged_next: Option<usize>,
) -> Unf {
    let mut texts: Vec<String> = ents.iter().map(|x| x.text.clone()).collect();
    texts[e] = new_text.clone();
    let lines = lines_of(&texts);
    let mut original = new_text;
    if let Some(nx) = merged_next {
        original.push('\n');
        original.push_str(&ents[nx].text);
    }
    let mut bad = vec![BadLine {
        line: lines[e] + err.0,
        column: err.1,
        problem,
        original,
        dropped_as_child: false,
    }];
    for d in e + 1..=ents[e].last_desc {
        if Some(d) == merged_next {
            continue;
        }
        bad.push(BadLine {
            line: lines[d],
            column: 1,
            problem: format!("dropped as child of line {}", lines[e]),
            original: ents[d].text.clone(),
            dropped_as_child: true,
        });
    }
    let removed = (e..=ents[e].last_desc).collect();
    Unf { texts, bad, removed }
}

fn value_candidates(ents: &[Ent], pred: impl Fn(&Col) -> bool) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    for (i, e) in ents.iter().enumerate() {
        if e.schema {
            continue;
        }
        for (ci, c) in e.cols.iter().enumerate() {
            if pred(c) {
                v.push((i, ci));
            }
        }
    }
    v
}

fn build_unfixable(rng: &mut SimpleRng, ents: &[Ent], kind: UK) -> Option<Unf> {
    let last = ents.len() - 1;
    let data: Vec<usize> = (0..ents.len()).filter(|&i| !ents[i].schema).collect();
    match kind {
        UK::Few => {
            let c: Vec<usize> = data
                .iter()
                .copied()
                .filter(|&i| i != last && ents[i].cols.len() >= 2)
                .collect();
            if c.is_empty() {
                return None;
            }
            let e = c[pick_idx(rng, c.len())];
            let en = &ents[e];
            let ci = pick_idx(rng, en.cols.len());
            let f = &en.fields[en.prefix + ci];
            let new = format!("{}{}", &en.text[..f.start], &en.text[f.end + 1..]);
            if gt_count(&new) != en.depth {
                return None;
            }
            let problem = format!("expected {} columns, found {}", en.cols.len(), en.cols.len() - 1);
            Some(finish(ents, e, new, (0, en.depth + 1), problem, None))
        }
        UK::Many => {
            let e = data[pick_idx(rng, data.len())];
            let en = &ents[e];
            let mut j = rng.gen_range(0, en.cols.len());
            let single_child = en.depth >= 1 && en.parent_lists == 1;
            if single_child && j > 0 && index_like(&en.text[en.fields[0].start..en.fields[0].end]) {
                j = 0;
            }
            let pos = if j < en.cols.len() { en.fields[en.prefix + j].start } else { en.text.len() };
            let new = format!("{}zzz|{}", &en.text[..pos], &en.text[pos..]);
            if gt_count(&new) != en.depth {
                return None;
            }
            let problem = format!("expected {} columns, found {}", en.cols.len(), en.cols.len() + 1);
            Some(finish(ents, e, new, (0, en.depth + 1), problem, None))
        }
        UK::Pipe => {
            let mut c = Vec::new();
            for &(e, ci) in &value_candidates(ents, |c| *c == Col::Str) {
                let en = &ents[e];
                let f = &en.fields[en.prefix + ci];
                let vs = if f.raw { f.start + 2 } else { f.start };
                let v = &en.text[vs..f.end];
                if v.chars().count() >= 2 && !v.contains('\\') && !v.contains('\n') {
                    c.push((e, ci, vs));
                }
            }
            if c.is_empty() {
                return None;
            }
            let (e, ci, vs) = c[pick_idx(rng, c.len())];
            let en = &ents[e];
            let f = &en.fields[en.prefix + ci];
            let v = &en.text[vs..f.end];
            let at = rng.gen_range(1, v.chars().count() - 1);
            let byte = vs + v.char_indices().nth(at).map(|(i, _)| i).unwrap();
            let new = format!("{}|{}", &en.text[..byte], &en.text[byte..]);
            if en.depth >= 1 && en.parent_lists == 1 {
                let first_end = new.find('|').unwrap();
                if index_like(&new[en.depth..first_end]) {
                    return None;
                }
            }
            let problem = format!("expected {} columns, found {}", en.cols.len(), en.cols.len() + 1);
            Some(finish(ents, e, new, (0, en.depth + 1), problem, None))
        }
        UK::EnumTypo | UK::Caps => {
            let c = value_candidates(ents, |c| matches!(c, Col::Enum(_)));
            if c.is_empty() {
                return None;
            }
            let (e, ci) = c[pick_idx(rng, c.len())];
            let en = &ents[e];
            let f = &en.fields[en.prefix + ci];
            let labels = match &en.cols[ci] {
                Col::Enum(l) => l.clone(),
                _ => unreachable!(),
            };
            let cur = &en.text[f.start..f.end];
            let base = if labels.iter().any(|l| l == cur) {
                cur.to_string()
            } else {
                labels[pick_idx(rng, labels.len())].clone()
            };
            let bad_text = if kind == UK::Caps {
                let mut cs = base.chars();
                let first = cs.next().unwrap().to_uppercase().collect::<String>();
                format!("{}{}", first, cs.as_str())
            } else {
                match rng.gen_range(0, 2) {
                    0 => format!("{}x", base),
                    1 => format!("z{}", &base[1..]),
                    _ => format!("{}z", &base[..base.len() - 1]),
                }
            };
            if labels.iter().any(|l| *l == bad_text) || bad_text == base && kind == UK::Caps {
                return None;
            }
            let new = format!("{}{}{}", &en.text[..f.start], bad_text, &en.text[f.end..]);
            let (x, col) = locate(&new, f.start);
            let problem = type_error(&en.cols[ci], &bad_text);
            Some(finish(ents, e, new, (x, col), problem, None))
        }
        UK::Depth => {
            let c: Vec<usize> = data
                .iter()
                .copied()
                .filter(|&i| i >= 1 && !ents[i - 1].schema && ents[i].last_desc == i)
                .collect();
            if c.is_empty() {
                return None;
            }
            let e = c[pick_idx(rng, c.len())];
            let p = &ents[e - 1];
            let stacklen = p.depth + 1;
            let nd = stacklen + 1 + rng.gen_range(0, 1);
            let en = &ents[e];
            let new = format!("{}{}", ">".repeat(nd), &en.text[en.depth..]);
            if gt_count(&new) != nd {
                return None;
            }
            let problem = format!(
                "depth increased by more than one level (from depth {} to {})",
                p.depth, nd
            );
            Some(finish(ents, e, new, (0, 1), problem, None))
        }
        UK::NoList => {
            let c: Vec<usize> = data
                .iter()
                .copied()
                .filter(|&i| {
                    i >= 1 && !ents[i - 1].schema && ents[i - 1].own_lists == 0 && ents[i].last_desc == i
                })
                .collect();
            if c.is_empty() {
                return None;
            }
            let e = c[pick_idx(rng, c.len())];
            let nd = ents[e - 1].depth + 1;
            let en = &ents[e];
            let new = format!("{}{}", ">".repeat(nd), &en.text[en.depth..]);
            if gt_count(&new) != nd {
                return None;
            }
            Some(finish(
                ents,
                e,
                new,
                (0, 1),
                "parent record has no list fields to attach child line".to_string(),
                None,
            ))
        }
        UK::MissIdx => {
            let c: Vec<usize> = data
                .iter()
                .copied()
                .filter(|&i| i != last && ents[i].prefix == 1)
                .collect();
            if c.is_empty() {
                return None;
            }
            let e = c[pick_idx(rng, c.len())];
            let en = &ents[e];
            let new = format!("{}{}", &en.text[..en.fields[0].start], &en.text[en.fields[0].end + 1..]);
            let f1 = &en.fields[1];
            if gt_count(&new) != en.depth || index_like(&en.text[f1.start..f1.end]) {
                return None;
            }
            Some(finish(ents, e, new, (0, en.depth + 1), "missing list index".to_string(), None))
        }
        UK::UnclosedRaw => {
            let c: Vec<usize> = data
                .iter()
                .copied()
                .filter(|&i| {
                    let en = &ents[i];
                    en.last_desc > i
                        && !en.fields.last().unwrap().raw
                        && ents[i + 1].fields.len() >= 2
                        && !(en.depth >= 1
                            && en.parent_lists == 1
                            && index_like(&en.text[en.fields[0].start..en.fields[0].end]))
                })
                .collect();
            if c.is_empty() {
                return None;
            }
            let e = c[pick_idx(rng, c.len())];
            let en = &ents[e];
            let lf = en.fields.last().unwrap();
            let new = format!("{}\\r{}", &en.text[..lf.start], &en.text[lf.start..lf.end]);
            let merged = en.fields.len() + ents[e + 1].fields.len() - 1;
            let problem = format!("expected {} columns, found {}", en.cols.len(), merged - en.prefix);
            Some(finish(ents, e, new, (0, en.depth + 1), problem, Some(e + 1)))
        }
        UK::Esc => {
            let mut c = Vec::new();
            for &(e, ci) in &value_candidates(ents, |c| *c == Col::Str) {
                let en = &ents[e];
                if !en.fields[en.prefix + ci].raw {
                    c.push((e, ci));
                }
            }
            if c.is_empty() {
                return None;
            }
            let (e, ci) = c[pick_idx(rng, c.len())];
            let en = &ents[e];
            let f = &en.fields[en.prefix + ci];
            let v = &en.text[f.start..f.end];
            let mut positions = Vec::new();
            let mut prev: Option<char> = None;
            for (i, ch) in v.char_indices().chain(std::iter::once((v.len(), ' '))) {
                if prev != Some('\\') {
                    positions.push(i);
                }
                prev = Some(ch);
            }
            let at = f.start + positions[pick_idx(rng, positions.len())];
            let new = format!("{}\\q{}", &en.text[..at], &en.text[at..]);
            let (x, col) = locate(&new, at);
            Some(finish(ents, e, new, (x, col), "invalid escape sequence '\\q'".to_string(), None))
        }
        UK::Quoted | UK::Plus | UK::Comma | UK::Prefix | UK::Null | UK::NoneWord | UK::NA
        | UK::NaN | UK::True | UK::Yes | UK::Empty | UK::DecComma => {
            let pred = |c: &Col| match kind {
                UK::Quoted | UK::Plus | UK::Comma => *c == Col::Int,
                UK::Prefix | UK::Null | UK::NoneWord | UK::NA => *c != Col::Str,
                UK::NaN => matches!(c, Col::Int | Col::Bool | Col::Enum(_)),
                UK::True | UK::Yes => *c == Col::Bool,
                UK::DecComma => *c == Col::Float,
                _ => true,
            };
            let c = value_candidates(ents, pred);
            if c.is_empty() {
                return None;
            }
            let (e, ci) = c[pick_idx(rng, c.len())];
            let en = &ents[e];
            let f = &en.fields[en.prefix + ci];
            let col = &en.cols[ci];
            let bad_text = match kind {
                UK::Quoted => format!("\"{}\"", if rng.gen_bool(0.5) { 2024 } else { rng.gen_range(1, 9999) }),
                UK::Plus => format!("+{}", rng.gen_range(1, 9999)),
                UK::Comma => format!("{},{:03}", rng.gen_range(1, 99), rng.gen_range(0, 999)),
                UK::Prefix => {
                    let name = ["year", "score", "flag", "kind", "count"][rng.gen_range(0, 4)];
                    let v = match col {
                        Col::Int => "2024".to_string(),
                        Col::Float => "3.5".to_string(),
                        Col::Bool => "1".to_string(),
                        Col::Enum(l) => l[0].clone(),
                        Col::Str => unreachable!(),
                    };
                    format!("{}: {}", name, v)
                }
                UK::Null => "null".to_string(),
                UK::NoneWord => "None".to_string(),
                UK::NA => "N/A".to_string(),
                UK::NaN => "NaN".to_string(),
                UK::True => "true".to_string(),
                UK::Yes => "yes".to_string(),
                UK::Empty => String::new(),
                UK::DecComma => format!("{},{}", rng.gen_range(1, 99), rng.gen_range(1, 99)),
                _ => unreachable!(),
            };
            let new = format!("{}{}{}", &en.text[..f.start], bad_text, &en.text[f.end..]);
            let (x, colno) = locate(&new, f.start);
            let problem = if bad_text.is_empty() {
                EMPTY_FIELD.to_string()
            } else {
                type_error(col, &bad_text)
            };
            Some(finish(ents, e, new, (x, colno), problem, None))
        }
    }
}

// ---------- checking ----------

struct CaseErr {
    false_repair: bool,
    msg: String,
}

struct Outcome {
    exact: bool,
    truncated_but_valid: bool,
    bad: usize,
    repairs: usize,
}

fn strict_opts() -> DoctorOptions {
    DoctorOptions {
        mode: DoctorMode::Strict,
        retry_hint: true,
    }
}

fn salvage_opts() -> DoctorOptions {
    DoctorOptions {
        mode: DoctorMode::Salvage,
        retry_hint: true,
    }
}

fn describe_blocks_diff(got: &[Block], want: &[Block]) -> String {
    if got.len() != want.len() {
        return format!("block count {} != expected {}", got.len(), want.len());
    }
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        if g.class_name != w.class_name {
            return format!("block {} class {:?} != {:?}", i, g.class_name, w.class_name);
        }
        if g.records.len() != w.records.len() {
            return format!(
                "block {} has {} records, expected {}",
                i,
                g.records.len(),
                w.records.len()
            );
        }
        for (j, (a, b)) in g.records.iter().zip(&w.records).enumerate() {
            if a != b {
                return format!("block {} record {} differs\n  got:      {:?}\n  expected: {:?}", i, j, a, b);
            }
        }
    }
    "no difference found".to_string()
}

fn data_of(report: &structl::DoctorReport) -> Vec<Block> {
    report.data.as_ref().map(|d| d.blocks.clone()).unwrap_or_default()
}

fn check_data(
    schemas: &str,
    report: &structl::DoctorReport,
    expected: &[Block],
    label: &str,
) -> Result<(), CaseErr> {
    let got = data_of(report);
    if got != expected {
        return Err(CaseErr {
            false_repair: true,
            msg: format!("{}: data differs from expected: {}", label, describe_blocks_diff(&got, expected)),
        });
    }
    if expected.is_empty() {
        if report.data.is_some() {
            return Err(CaseErr {
                false_repair: false,
                msg: format!("{}: expected no data, got Some", label),
            });
        }
    } else {
        let mut file = DecodedFile::new();
        file.blocks = expected.to_vec();
        let want_text = encode_with(schemas, &file).unwrap();
        if report.text != want_text {
            return Err(CaseErr {
                false_repair: true,
                msg: format!(
                    "{}: canonical text differs from the expected data's canonical text\n  got:      {:?}\n  expected: {:?}",
                    label, report.text, want_text
                ),
            });
        }
    }
    Ok(())
}

fn check_fixable(schemas: &str, d: &Damaged) -> Result<Outcome, CaseErr> {
    let plain = |m: String| CaseErr { false_repair: false, msg: m };
    for (label, opts) in [("salvage", salvage_opts()), ("strict", strict_opts())] {
        let report = doctor(schemas, &d.text, &opts);
        if !report.bad_lines.is_empty() {
            return Err(plain(format!("{}: unexpected bad lines {:?}", label, report.bad_lines)));
        }
        if report.hint.is_some() {
            return Err(plain(format!("{}: hint present without bad lines", label)));
        }
        let mut got: Vec<&Repair> = report.repairs.iter().collect();
        got.sort_by_key(|r| r.line);
        if got.len() != d.repairs.len() {
            return Err(plain(format!(
                "{}: expected {} repairs, got {}\n  expected: {:?}\n  got: {:?}",
                label,
                d.repairs.len(),
                got.len(),
                d.repairs
                    .iter()
                    .map(|r| (r.line, r.kinds.clone()))
                    .collect::<Vec<_>>(),
                got.iter().map(|r| (r.line, r.fix)).collect::<Vec<_>>()
            )));
        }
        for (g, w) in got.iter().zip(&d.repairs) {
            if g.line != w.line
                || !w.kinds.contains(&g.fix)
                || g.original != w.original
                || g.new_text != w.new_text
            {
                return Err(plain(format!(
                    "{}: repair mismatch\n  expected: line {} kinds {:?} original {:?} new {:?}\n  got:      line {} kind {:?} original {:?} new {:?}",
                    label, w.line, w.kinds, w.original, w.new_text, g.line, g.fix, g.original, g.new_text
                )));
            }
        }
        check_data(schemas, &report, &d.expected, label)?;
    }
    Ok(Outcome {
        exact: true,
        truncated_but_valid: false,
        bad: 0,
        repairs: d.repairs.len(),
    })
}

fn check_unfixable(
    schemas: &str,
    damaged_text: &str,
    unf: &Unf,
    expected: &[Block],
) -> Result<Outcome, CaseErr> {
    let plain = |m: String| CaseErr { false_repair: false, msg: m };
    let salvage = doctor(schemas, damaged_text, &salvage_opts());
    let strict = doctor(schemas, damaged_text, &strict_opts());
    for (label, report) in [("salvage", &salvage), ("strict", &strict)] {
        if !report.repairs.is_empty() {
            return Err(plain(format!("{}: unexpected repairs {:?}", label, report.repairs)));
        }
        if report.bad_lines != unf.bad {
            return Err(plain(format!(
                "{}: bad lines differ\n  expected: {:#?}\n  got:      {:#?}",
                label, unf.bad, report.bad_lines
            )));
        }
    }
    if strict.data.is_some() || !strict.text.is_empty() {
        return Err(plain("strict: data returned although there are bad lines".to_string()));
    }
    check_data(schemas, &salvage, expected, "salvage")?;
    match &salvage.hint {
        None => return Err(plain("salvage: no retry hint although there are bad lines".to_string())),
        Some(h) => {
            for b in unf.bad.iter().filter(|b| !b.dropped_as_child) {
                let want = format!("{}   ({})", b.original, b.problem);
                if !h.contains(&want) {
                    return Err(plain(format!("hint does not quote the bad line: missing {:?} in {:?}", want, h)));
                }
            }
        }
    }
    Ok(Outcome {
        exact: true,
        truncated_but_valid: false,
        bad: unf.bad.len(),
        repairs: 0,
    })
}

// ---------- the driver ----------

#[derive(Clone, Copy)]
enum Row {
    NoDamage,
    Single(Fix),
    Combo(usize),
    Cut,
    Unfixable(UK),
}

enum CaseResult {
    NotApplicable,
    Done(Result<Outcome, CaseErr>, String, String),
}

fn run_case(row: Row, seed: u64) -> CaseResult {
    let mut rng = SimpleRng::new(seed);
    let cfg = GenConfig::RICH;
    for _ in 0..40 {
        let gf = generate_random_file(&mut rng, &cfg);
        let ents = build_layout(&gf);
        match row {
            Row::NoDamage => {
                let report = doctor(&gf.schemas, &gf.text, &salvage_opts());
                let res = (|| {
                    if !report.repairs.is_empty() || !report.bad_lines.is_empty() {
                        return Err(CaseErr {
                            false_repair: false,
                            msg: format!(
                                "clean file: repairs {:?}, bad lines {:?}",
                                report.repairs, report.bad_lines
                            ),
                        });
                    }
                    if report.text != gf.text {
                        return Err(CaseErr {
                            false_repair: true,
                            msg: "clean file: canonical text changed".to_string(),
                        });
                    }
                    check_data(&gf.schemas, &report, &gf.doc.blocks, "salvage")?;
                    Ok(Outcome {
                        exact: true,
                        truncated_but_valid: false,
                        bad: 0,
                        repairs: 0,
                    })
                })();
                return CaseResult::Done(res, gf.schemas.clone(), gf.text.clone());
            }
            Row::Single(_) | Row::Combo(_) => {
                let fixes: Vec<Fix> = match row {
                    Row::Single(f) => vec![f],
                    Row::Combo(k) => {
                        let mut all = ALL_FIXES.to_vec();
                        shuffle(&mut rng, &mut all);
                        all.truncate(k);
                        all
                    }
                    _ => unreachable!(),
                };
                match compose_fixable(&mut rng, &gf, &ents, &fixes) {
                    Built::NotApplicable => continue,
                    Built::Case(d) => {
                        let res = check_fixable(&gf.schemas, &d);
                        return CaseResult::Done(res, gf.schemas.clone(), d.text);
                    }
                }
            }
            Row::Cut => {
                match plan_cut_r3f(&mut rng, &gf, &ents) {
                    BuiltCut::NotApplicable => continue,
                    BuiltCut::Case(c) => {
                        let res = check_cut_case(&gf.schemas, &c);
                        return CaseResult::Done(res, gf.schemas.clone(), c.text);
                    }
                }
            }
            Row::Unfixable(k) => {
                let unf = match build_unfixable(&mut rng, &ents, k) {
                    Some(u) => u,
                    None => continue,
                };
                let mut text = unf.texts.join("\n");
                text.push('\n');
                let mut kept = String::new();
                for (i, t) in ents.iter().enumerate() {
                    if !unf.removed.contains(&i) {
                        kept.push_str(&t.text);
                        kept.push('\n');
                    }
                }
                let expected = match decode_blocks(&gf.schemas, &kept) {
                    Ok(b) => b,
                    Err(e) => panic!("oracle could not read the kept lines: {}", e),
                };
                let res = check_unfixable(&gf.schemas, &text, &unf, &expected);
                return CaseResult::Done(res, gf.schemas.clone(), text);
            }
        }
    }
    CaseResult::NotApplicable
}

#[derive(Default)]
struct RowStats {
    cases: usize,
    exact: usize,
    truncated_but_valid: usize,
    bad: usize,
    repairs: usize,
    false_repairs: usize,
    failures: usize,
    inputs: HashSet<u64>,
    schemas: HashSet<u64>,
}

struct Failure {
    row: usize,
    name: String,
    seed: u64,
    msg: String,
    schemas: String,
    text: String,
}

fn row_list() -> Vec<(Row, String)> {
    let mut rows = vec![(Row::NoDamage, "No damage".to_string())];
    for f in ALL_FIXES {
        let mut name = format!("Fix alone: {}", fix_name(f));
        if f == Fix::Pipe {
            name.push_str(" (1-3 lines)");
        }
        rows.push((Row::Single(f), name));
    }
    for k in 2..=4 {
        rows.push((Row::Combo(k), format!("Random combination of {} fixes", k)));
    }
    rows.push((Row::Cut, "Cut-off line at EOF".to_string()));
    for (k, name) in ALL_UK {
        rows.push((Row::Unfixable(k), name.to_string()));
    }
    rows
}

fn case_seed(row_idx: usize, attempt: u64) -> u64 {
    splitmix(0xD0C7_0125_2026_1119 ^ ((row_idx as u64) << 40) ^ attempt)
}

fn replay(rows: &[(Row, String)], spec: &str) {
    let (r, s) = spec.split_once(':').expect("DOCTOR_REPRO is <row>:<seed hex>");
    let row_idx: usize = r.parse().unwrap();
    let seed = u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap();
    match run_case(rows[row_idx].0, seed) {
        CaseResult::Done(Ok(_), schemas, text) => {
            println!("replay passed.\nschemas:\n{}\ndamaged text:\n{:?}", schemas, text)
        }
        CaseResult::Done(Err(e), schemas, text) => panic!(
            "replay of '{}' seed 0x{:016x} failed: {}\nschemas:\n{}\ndamaged text: {:?}",
            rows[row_idx].1, seed, e.msg, schemas, text
        ),
        CaseResult::NotApplicable => println!("replay: not applicable"),
    }
}

#[test]
fn test_damage_suite_5000_cases() {
    let rows = row_list();
    if let Ok(spec) = std::env::var("DOCTOR_REPRO") {
        replay(&rows, &spec);
        return;
    }

    let mut stats: Vec<RowStats> = rows.iter().map(|_| RowStats::default()).collect();
    let mut failures: Vec<Failure> = Vec::new();

    for (ri, (row, name)) in rows.iter().enumerate() {
        let st = &mut stats[ri];
        let mut attempt = 0u64;
        while st.cases < CASES_PER_ROW {
            attempt += 1;
            assert!(
                attempt < 100 * CASES_PER_ROW as u64,
                "row '{}': could not build {} applicable cases",
                name,
                CASES_PER_ROW
            );
            let seed = case_seed(ri, attempt);
            let r = catch_unwind(AssertUnwindSafe(|| run_case(*row, seed)));
            match r {
                Err(p) => {
                    st.cases += 1;
                    st.failures += 1;
                    let msg = p
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "panic".to_string());
                    failures.push(Failure {
                        row: ri,
                        name: name.clone(),
                        seed,
                        msg: format!("PANIC: {}", msg),
                        schemas: String::new(),
                        text: String::new(),
                    });
                }
                Ok(CaseResult::NotApplicable) => {}
                Ok(CaseResult::Done(res, schemas, text)) => {
                    st.cases += 1;
                    st.inputs.insert(hash_of(&text));
                    st.schemas.insert(hash_of(&schemas));
                    match res {
                        Ok(o) => {
                            if o.exact {
                                st.exact += 1;
                            }
                            if o.truncated_but_valid {
                                st.truncated_but_valid += 1;
                            }
                            st.bad += o.bad;
                            st.repairs += o.repairs;
                        }
                        Err(e) => {
                            st.failures += 1;
                            if e.false_repair {
                                st.false_repairs += 1;
                            }
                            failures.push(Failure {
                                row: ri,
                                name: name.clone(),
                                seed,
                                msg: e.msg,
                                schemas,
                                text,
                            });
                        }
                    }
                }
            }
        }
    }

    println!("\n=============================================================================================================================");
    println!("STRUCT DOCTOR DAMAGE SUITE (random schemas, random multi-record files, seeded)");
    println!("=============================================================================================================================");
    println!("| # | Damage kind | Cases | Exact restorations | Truncated-but-valid | Bad lines reported | Repairs reported | False repairs | Failures | Distinct inputs | Distinct schemas |");
    println!("|---|---|---|---|---|---|---|---|---|---|---|");
    let mut tot = RowStats::default();
    let mut all_inputs: HashSet<u64> = HashSet::new();
    let mut all_schemas: HashSet<u64> = HashSet::new();
    for (i, (_, name)) in rows.iter().enumerate() {
        let s = &stats[i];
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            i, name, s.cases, s.exact, s.truncated_but_valid, s.bad, s.repairs, s.false_repairs, s.failures,
            s.inputs.len(), s.schemas.len()
        );
        tot.cases += s.cases;
        tot.exact += s.exact;
        tot.truncated_but_valid += s.truncated_but_valid;
        tot.bad += s.bad;
        tot.repairs += s.repairs;
        tot.false_repairs += s.false_repairs;
        tot.failures += s.failures;
        all_inputs.extend(s.inputs.iter());
        all_schemas.extend(s.schemas.iter());
    }
    println!(
        "| | TOTAL | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
        tot.cases, tot.exact, tot.truncated_but_valid, tot.bad, tot.repairs, tot.false_repairs, tot.failures,
        all_inputs.len(), all_schemas.len()
    );
    println!("=============================================================================================================================\n");

    if !failures.is_empty() {
        let mut groups: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        for f in &failures {
            let first: String = f.msg.lines().next().unwrap_or("").chars().map(|c| if c.is_ascii_digit() { '#' } else { c }).take(75).collect();
            *groups.entry(format!("{} | {}", f.name, first)).or_insert(0) += 1;
        }
        println!("Failure groups (row | first line of message): count");
        for (k, c) in &groups {
            println!("  {:4}  {}", c, k);
        }
        let mut shown = 0;
        for f in &failures {
            shown += 1;
            if shown > 40 {
                break;
            }
            println!(
                "FAILURE row {} '{}' seed 0x{:016x} (replay: DOCTOR_REPRO={}:{:016x})\n{}\nschemas:\n{}damaged text: {:?}\n",
                f.row, f.name, f.seed, f.row, f.seed, f.msg, f.schemas, f.text
            );
        }
        println!("{} failures in total", failures.len());
    }
    assert!(tot.cases >= 5000, "must run at least 5000 cases, ran {}", tot.cases);
    assert_eq!(tot.false_repairs, 0, "false repairs must be 0");
    assert!(failures.is_empty(), "{} damage-suite cases failed (see above)", failures.len());
}
