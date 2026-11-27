//! Unit, integration, and acceptance tests for `structl query` (Milestone Q1).
//!
//! Standard library only.

use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

fn run_structl(args: &[&str]) -> (i32, String, String) {
    let bin = env!("CARGO_BIN_EXE_structl");
    let output = Command::new(bin)
        .args(args)
        .output()
        .expect("Failed to execute structl");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

fn run_structl_stdin(args: &[&str], input: &str) -> (i32, String, String) {
    let bin = env!("CARGO_BIN_EXE_structl");
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn structl");

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

/// Normalizes and matches an actual answer to expected ground truth,
/// implementing the R4 benchmark rules from `bench/r4/evaluate.py`:
/// - lists are compared as sets (order ignored, comma-separated or newline-separated)
/// - null values: "null" in ground truth matches "~" or "null"
/// - booleans: "false" in ground truth matches "0" or "false"; "true" matches "1" or "true"
/// - exact trimmed equality for other scalar values.
fn assert_r4_answer_matches(actual: &str, expected: &str) {
    let act_trimmed = actual.trim();
    let exp_trimmed = expected.trim();

    // Strip trailing '|' if present from Struct data line
    let act_val = act_trimmed.strip_suffix('|').unwrap_or(act_trimmed).trim();

    if exp_trimmed == "null" {
        assert!(
            act_val == "~" || act_val == "null",
            "Expected null (~ or null), got: '{}'",
            actual
        );
        return;
    }

    if exp_trimmed == "false" {
        assert!(
            act_val == "0" || act_val == "false",
            "Expected false (0 or false), got: '{}'",
            actual
        );
        return;
    }

    if exp_trimmed == "true" {
        assert!(
            act_val == "1" || act_val == "true",
            "Expected true (1 or true), got: '{}'",
            actual
        );
        return;
    }

    if exp_trimmed.contains(',') {
        let exp_set: HashSet<String> = exp_trimmed
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let act_set: HashSet<String> = act_trimmed
            .lines()
            .map(|line| line.trim().trim_end_matches('|').trim())
            .flat_map(|line| line.split(','))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(
            act_set, exp_set,
            "List set mismatch! Actual: {:?}, Expected: {:?}",
            act_set, exp_set
        );
        return;
    }

    assert_eq!(act_val, exp_trimmed);
}

// ============================================================================
// Acceptance Test: 12 R4 Questions for flat_100
// ============================================================================

#[test]
fn test_acceptance_flat_100_all_12_questions() {
    let fixture_path = "tests/fixtures/flat_100.struct";
    assert!(
        std::path::Path::new(fixture_path).exists(),
        "Fixture flat_100.struct must exist"
    );

    // Q1: What is the verdict for paper 2411.2824?
    // Ground truth: "unverified"
    // Query: structl query tests/fixtures/flat_100.struct --where "id == 2411.2824" --fields verdict
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "id == 2411.2824",
        "--fields",
        "verdict",
    ]);
    assert_eq!(code, 0, "Q01 failed with stderr: {}", stderr);
    assert_r4_answer_matches(&stdout, "unverified");

    // Q2: What is the publication year of paper 2404.3286?
    // Ground truth: "2023"
    // Query: structl query tests/fixtures/flat_100.struct --where "id == 2404.3286" --fields year
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "id == 2404.3286",
        "--fields",
        "year",
    ]);
    assert_eq!(code, 0, "Q02 failed with stderr: {}", stderr);
    assert_r4_answer_matches(&stdout, "2023");

    // Q3: What is the title of paper 2404.9279?
    // Ground truth: "KV cache quantization losing accuracy on outliers v4"
    // Query: structl query tests/fixtures/flat_100.struct --where "id == 2404.9279" --fields title
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "id == 2404.9279",
        "--fields",
        "title",
    ]);
    assert_eq!(code, 0, "Q03 failed with stderr: {}", stderr);
    assert_r4_answer_matches(
        &stdout,
        "KV cache quantization losing accuracy on outliers v4",
    );

    // Q4: What is the verification status of paper 2412.9928?
    // Ground truth: "false"
    // Query: structl query tests/fixtures/flat_100.struct --where "id == 2412.9928" --fields verified
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "id == 2412.9928",
        "--fields",
        "verified",
    ]);
    assert_eq!(code, 0, "Q04 failed with stderr: {}", stderr);
    assert_r4_answer_matches(&stdout, "false");

    // Q5: How many records have verdict refuted?
    // Ground truth: "26"
    // Query: structl query tests/fixtures/flat_100.struct --where "verdict == refuted" --count
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "verdict == refuted",
        "--count",
    ]);
    assert_eq!(code, 0, "Q05 failed with stderr: {}", stderr);
    assert_r4_answer_matches(&stdout, "26");

    // Q6: How many records were published in 2024?
    // Ground truth: "11"
    // Query: structl query tests/fixtures/flat_100.struct --where "year == 2024" --count
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "year == 2024",
        "--count",
    ]);
    assert_eq!(code, 0, "Q06 failed with stderr: {}", stderr);
    assert_r4_answer_matches(&stdout, "11");

    // Q7: How many records are marked as verified (true)?
    // Ground truth: "49"
    // Query: structl query tests/fixtures/flat_100.struct --where "verified == true" --count
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "verified == true",
        "--count",
    ]);
    assert_eq!(code, 0, "Q07 failed with stderr: {}", stderr);
    assert_r4_answer_matches(&stdout, "49");

    // Q8: Which paper IDs have no verdict?
    // Ground truth: "2404.7570, 2411.9702"
    // Query: structl query tests/fixtures/flat_100.struct --where "verdict == null" --fields id
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "verdict == null",
        "--fields",
        "id",
    ]);
    assert_eq!(code, 0, "Q08 failed with stderr: {}", stderr);
    assert_r4_answer_matches(&stdout, "2404.7570, 2411.9702");

    // Q9: Which paper IDs have no publication year?
    // Ground truth: "2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547"
    // Query: structl query tests/fixtures/flat_100.struct --where "year == null" --fields id
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "year == null",
        "--fields",
        "id",
    ]);
    assert_eq!(code, 0, "Q09 failed with stderr: {}", stderr);
    assert_r4_answer_matches(
        &stdout,
        "2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547",
    );

    // Q10: Which paper IDs have no verification status?
    // Ground truth: "2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445"
    // Query: structl query tests/fixtures/flat_100.struct --where "verified == null" --fields id
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "verified == null",
        "--fields",
        "id",
    ]);
    assert_eq!(code, 0, "Q10 failed with stderr: {}", stderr);
    assert_r4_answer_matches(
        &stdout,
        "2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445",
    );

    // Q11: What is the publication year of paper 2403.4527?
    // Ground truth: "null"
    // Query: structl query tests/fixtures/flat_100.struct --where "id == 2403.4527" --fields year
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "id == 2403.4527",
        "--fields",
        "year",
    ]);
    assert_eq!(code, 0, "Q11 failed with stderr: {}", stderr);
    assert_r4_answer_matches(&stdout, "null");

    // Q12: What is the verdict for paper 2404.7570?
    // Ground truth: "null"
    // Query: structl query tests/fixtures/flat_100.struct --where "id == 2404.7570" --fields verdict
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "id == 2404.7570",
        "--fields",
        "verdict",
    ]);
    assert_eq!(code, 0, "Q12 failed with stderr: {}", stderr);
    assert_r4_answer_matches(&stdout, "null");
}

// ============================================================================
// Error Cases (Q1 Decision 5)
// ============================================================================

#[test]
fn test_error_unknown_field_in_where() {
    let fixture_path = "tests/fixtures/flat_100.struct";
    let (code, _stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "nonexistent == foo",
    ]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("unknown field 'nonexistent'"),
        "Expected unknown field message, got: {}",
        stderr
    );
    assert!(
        stderr.contains("Valid fields:"),
        "Expected valid fields list, got: {}",
        stderr
    );
}

#[test]
fn test_error_unknown_field_in_fields() {
    let fixture_path = "tests/fixtures/flat_100.struct";
    let (code, _stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--fields",
        "id,author",
    ]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("unknown field 'author' in --fields"),
        "Expected unknown field in fields message, got: {}",
        stderr
    );
    assert!(
        stderr.contains("Valid fields:"),
        "Expected valid fields list, got: {}",
        stderr
    );
}

#[test]
fn test_error_bad_enum_label() {
    let fixture_path = "tests/fixtures/flat_100.struct";
    let (code, _stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "verdict == invalid_label",
    ]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("invalid enum label 'invalid_label' for field 'verdict'"),
        "Expected bad enum label message, got: {}",
        stderr
    );
    assert!(
        stderr.contains("Valid labels: supported, refuted, unverified"),
        "Expected valid labels list, got: {}",
        stderr
    );
}

#[test]
fn test_error_bad_int() {
    let fixture_path = "tests/fixtures/flat_100.struct";
    let (code, _stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "year == twenty_twenty",
    ]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("invalid int value 'twenty_twenty' for field 'year'"),
        "Expected bad int message, got: {}",
        stderr
    );
}

#[test]
fn test_error_null_with_less_than() {
    let fixture_path = "tests/fixtures/flat_100.struct";
    let (code, _stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "year < null",
    ]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("operator '<' not allowed with null"),
        "Expected null operator restriction message, got: {}",
        stderr
    );
}

#[test]
fn test_error_several_classes_without_class_flag() {
    let doc = "\
schema:first{a}\n\
val1|\n\
schema:second{b}\n\
val2|\n";

    let (code, _stdout, stderr) = run_structl_stdin(&["query", "-"], doc);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("multiple classes found"),
        "Expected multiple classes message, got: {}",
        stderr
    );
    assert!(
        stderr.contains("first") && stderr.contains("second"),
        "Expected list of classes in message, got: {}",
        stderr
    );
}

#[test]
fn test_error_file_fails_to_decode() {
    // Missing closing pipe on data line produces fatal decode error
    let bad_doc = "\
schema:person{name, age:int}\n\
Alice|30\n";

    let (code, _stdout, stderr) = run_structl_stdin(&["query", "-"], bad_doc);
    assert_eq!(code, 2, "Decoder errors must exit with code 2");
    assert!(
        stderr.contains("error at line 2"),
        "Expected decoder error with line and column, got: {}",
        stderr
    );
}

#[test]
fn test_error_filter_on_list_field() {
    let doc = "\
schema:item{name, tags:list[str]}\n\
book|\n\
>fiction|;\n";

    let (code, _stdout, stderr) = run_structl_stdin(
        &["query", "-", "--where", "tags == fiction"],
        doc,
    );
    assert_eq!(code, 1);
    assert!(
        stderr.contains("list field 'tags' cannot be filtered in Q1"),
        "Expected list field error, got: {}",
        stderr
    );
}

#[test]
fn test_error_select_list_field() {
    let doc = "\
schema:item{name, tags:list[str]}\n\
book|\n\
>fiction|;\n";

    let (code, _stdout, stderr) = run_structl_stdin(
        &["query", "-", "--fields", "name,tags"],
        doc,
    );
    assert_eq!(code, 1);
    assert!(
        stderr.contains("list field 'tags' cannot be selected in Q1"),
        "Expected list field selection error, got: {}",
        stderr
    );
}

#[test]
fn test_error_invalid_format() {
    let fixture_path = "tests/fixtures/flat_100.struct";
    let (code, _stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--format",
        "yaml",
    ]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("invalid format 'yaml', must be struct or json"),
        "Expected format error, got: {}",
        stderr
    );
}

#[test]
fn test_error_both_schema_and_schema_file() {
    let fixture_path = "tests/fixtures/flat_100.struct";
    let (code, _stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--schema",
        "schema:test{a}",
        "--schema-file",
        "some/path",
    ]);
    assert_eq!(code, 1);
    assert!(
        stderr.contains("cannot specify multiple schema sources or both --schema and --schema-file"),
        "Expected schema conflict error, got: {}",
        stderr
    );
}

// ============================================================================
// Record Fields (Dotted Names)
// ============================================================================

#[test]
fn test_record_fields_dotted_names() {
    let doc = "\
schema:claim{text, source:record{id, year:int}, verdict:enum(supported,refuted,unverified)}\n\
Rope scaling hurts long ctx|2405.05254|2024|refuted|\n\
Attention is all you need|1706.03762|2017|supported|\n\
DeepSeekMoE mixture experts|2401.06080|null|supported|\n";

    // 1. Filter on dotted field: source.year == 2024
    let (code, stdout, stderr) = run_structl_stdin(
        &[
            "query",
            "-",
            "--where",
            "source.year == 2024",
            "--fields",
            "text,source.id",
        ],
        doc,
    );
    assert_eq!(code, 0, "Stderr: {}", stderr);
    assert_eq!(stdout.trim(), "Rope scaling hurts long ctx|2405.05254|");

    // 2. Filter on dotted field: source.id == 1706.03762
    let (code, stdout, stderr) = run_structl_stdin(
        &[
            "query",
            "-",
            "--where",
            "source.id == 1706.03762",
            "--fields",
            "text,source.year",
        ],
        doc,
    );
    assert_eq!(code, 0, "Stderr: {}", stderr);
    assert_eq!(stdout.trim(), "Attention is all you need|2017|");

    // 3. Filter on null inside record: source.year == null
    let (code, stdout, stderr) = run_structl_stdin(
        &[
            "query",
            "-",
            "--where",
            "source.year == null",
            "--fields",
            "text",
        ],
        doc,
    );
    assert_eq!(code, 0, "Stderr: {}", stderr);
    assert_eq!(stdout.trim(), "DeepSeekMoE mixture experts|");

    // 4. Ordering on dotted field: source.year > 2020 (null row must not match)
    let (code, stdout, stderr) = run_structl_stdin(
        &[
            "query",
            "-",
            "--where",
            "source.year > 2020",
            "--fields",
            "text,source.year",
        ],
        doc,
    );
    assert_eq!(code, 0, "Stderr: {}", stderr);
    assert_eq!(stdout.trim(), "Rope scaling hurts long ctx|2024|");

    // 5. JSON output with dotted fields
    let (code, stdout, stderr) = run_structl_stdin(
        &[
            "query",
            "-",
            "--where",
            "source.year == 2024",
            "--fields",
            "text,source.year",
            "--format",
            "json",
        ],
        doc,
    );
    assert_eq!(code, 0, "Stderr: {}", stderr);
    assert!(
        stdout.contains("\"source.year\": 2024") && stdout.contains("\"text\": \"Rope scaling hurts long ctx\""),
        "Unexpected JSON: {}",
        stdout
    );
}

// ============================================================================
// Stdin and Outside Schemas
// ============================================================================

#[test]
fn test_stdin_input_with_outside_schema() {
    let schema = "schema:claim{id, title, verdict:enum(supported,refuted,unverified), year:int, verified:bool}\n";
    let data = "2411.2824|Attention is all you need for sequence modelling v1|unverified|2020|true|\n";

    let (code, stdout, stderr) = run_structl_stdin(
        &[
            "query",
            "-",
            "--schema",
            schema,
            "--where",
            "id == 2411.2824",
            "--fields",
            "verdict",
        ],
        data,
    );
    assert_eq!(code, 0, "Stderr: {}", stderr);
    assert_eq!(stdout.trim(), "unverified|");
}

#[test]
fn test_schema_file_flag() {
    let temp_schema = "target/test_schema.struct";
    fs::write(
        temp_schema,
        "schema:claim{id, title, verdict:enum(supported,refuted,unverified), year:int, verified:bool}\n",
    )
    .unwrap();

    let data = "2411.2824|Attention is all you need for sequence modelling v1|unverified|2020|true|\n";
    let (code, stdout, stderr) = run_structl_stdin(
        &[
            "query",
            "-",
            "--schema-file",
            temp_schema,
            "--where",
            "id == 2411.2824",
            "--fields",
            "year",
        ],
        data,
    );
    assert_eq!(code, 0, "Stderr: {}", stderr);
    assert_eq!(stdout.trim(), "2020|");
}

// ============================================================================
// Canonical Default Output and JSON Output
// ============================================================================

#[test]
fn test_default_canonical_struct_output() {
    let fixture_path = "tests/fixtures/flat_100.struct";
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "id == 2411.2824",
    ]);
    assert_eq!(code, 0, "Stderr: {}", stderr);
    // Default output should be canonical data line with trailing pipe and NO schema line
    assert_eq!(
        stdout,
        "2411.2824|Attention is all you need for sequence modelling v1|unverified|2020|true|\n"
    );
}

#[test]
fn test_json_array_output_full_and_fields() {
    let fixture_path = "tests/fixtures/flat_100.struct";

    // 1. With --fields
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "id == 2411.2824",
        "--fields",
        "id,year,verified",
        "--format",
        "json",
    ]);
    assert_eq!(code, 0, "Stderr: {}", stderr);
    assert_eq!(
        stdout.trim(),
        "[{\"id\": \"2411.2824\", \"year\": 2020, \"verified\": true}]"
    );

    // 2. Without --fields (all fields)
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "id == 2411.2824",
        "--format",
        "json",
    ]);
    assert_eq!(code, 0, "Stderr: {}", stderr);
    assert!(stdout.starts_with("[{") && stdout.ends_with("}]\n"));
    assert!(stdout.contains("\"id\": \"2411.2824\""));
    assert!(stdout.contains("\"verdict\": \"unverified\""));
}

#[test]
fn test_contains_operator_and_quotes_stripping() {
    let fixture_path = "tests/fixtures/flat_100.struct";

    // String contains substring with surrounding quotes
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "title contains \"FlashAttention fast\"",
        "--count",
    ]);
    assert_eq!(code, 0, "Stderr: {}", stderr);
    assert_eq!(stdout.trim(), "5");
}

#[test]
fn test_zero_matches_returns_exit_code_zero() {
    let fixture_path = "tests/fixtures/flat_100.struct";

    // Count returns 0
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "id == 9999.9999",
        "--count",
    ]);
    assert_eq!(code, 0, "Stderr: {}", stderr);
    assert_eq!(stdout.trim(), "0");

    // Struct format returns empty string
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "id == 9999.9999",
    ]);
    assert_eq!(code, 0, "Stderr: {}", stderr);
    assert_eq!(stdout, "");

    // JSON format returns []
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--where",
        "id == 9999.9999",
        "--format",
        "json",
    ]);
    assert_eq!(code, 0, "Stderr: {}", stderr);
    assert_eq!(stdout.trim(), "[]");
}

#[test]
fn test_fields_output_ends_with_pipe() {
    let fixture_path = "tests/fixtures/flat_100.struct";
    let (code, stdout, stderr) = run_structl(&[
        "query",
        fixture_path,
        "--fields",
        "id,year",
    ]);
    assert_eq!(code, 0, "Stderr: {}", stderr);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 100);
    for (i, line) in lines.iter().enumerate() {
        assert!(
            line.ends_with('|'),
            "Line {} does not end with '|': {}",
            i,
            line
        );
    }
}

#[test]
fn test_fields_record_consistent_column_count() {
    let doc = "\
schema:item{id, meta:record{source, year:int, sub:record{active:bool, note}}, status}\n\
1|arxiv|2024|true|important|ok|\n\
2|null|null|null|null|pending|\n";

    // 1. Selecting the record field along with other fields
    let (code, stdout, stderr) = run_structl_stdin(
        &["query", "-", "--fields", "id,meta,status"],
        doc,
    );
    assert_eq!(code, 0, "Stderr: {}", stderr);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "1|arxiv|2024|true|important|ok|");
    assert_eq!(lines[1], "2|null|null|null|null|pending|");

    let cols0: Vec<&str> = lines[0].split('|').collect();
    let cols1: Vec<&str> = lines[1].split('|').collect();
    assert_eq!(
        cols0.len(),
        cols1.len(),
        "Both lines must have the same number of columns"
    );

    // 2. Selecting only the record field: both lines must have the same number of columns
    let (code2, stdout2, stderr2) = run_structl_stdin(
        &["query", "-", "--fields", "meta"],
        doc,
    );
    assert_eq!(code2, 0, "Stderr: {}", stderr2);
    let lines2: Vec<&str> = stdout2.lines().collect();
    assert_eq!(lines2.len(), 2);
    assert_eq!(lines2[0], "arxiv|2024|true|important|");
    assert_eq!(lines2[1], "null|null|null|null|");

    let cols2_0: Vec<&str> = lines2[0].split('|').collect();
    let cols2_1: Vec<&str> = lines2[1].split('|').collect();
    assert_eq!(
        cols2_0.len(),
        cols2_1.len(),
        "Record-only selection: both lines must have the same number of columns"
    );
}

#[test]
fn test_not_equal_with_nulls_adds_up_to_total() {
    let fixture_path = "tests/fixtures/flat_100.struct";

    // 1. Total records count in flat_100
    let (code, stdout, stderr) = run_structl(&["query", fixture_path, "--count"]);
    assert_eq!(code, 0, "Stderr: {}", stderr);
    let total: usize = stdout.trim().parse().expect("total count should be an integer");
    assert_eq!(total, 100);

    // Test on 'year' (int field with 10 nulls in flat_100)
    let (code_eq_year, stdout_eq_year, _) = run_structl(&[
        "query", fixture_path, "--where", "year == 2024", "--count",
    ]);
    assert_eq!(code_eq_year, 0);
    let count_eq_year: usize = stdout_eq_year.trim().parse().unwrap();

    let (code_neq_year, stdout_neq_year, _) = run_structl(&[
        "query", fixture_path, "--where", "year != 2024", "--count",
    ]);
    assert_eq!(code_neq_year, 0);
    let count_neq_year: usize = stdout_neq_year.trim().parse().unwrap();

    assert_eq!(
        count_eq_year + count_neq_year,
        total,
        "Counts of 'year == 2024' ({}) and 'year != 2024' ({}) must add up to total ({})",
        count_eq_year,
        count_neq_year,
        total
    );

    // Test on 'verdict' (enum field with 2 nulls in flat_100)
    let (code_eq_verdict, stdout_eq_verdict, _) = run_structl(&[
        "query", fixture_path, "--where", "verdict == supported", "--count",
    ]);
    assert_eq!(code_eq_verdict, 0);
    let count_eq_verdict: usize = stdout_eq_verdict.trim().parse().unwrap();

    let (code_neq_verdict, stdout_neq_verdict, _) = run_structl(&[
        "query", fixture_path, "--where", "verdict != supported", "--count",
    ]);
    assert_eq!(code_neq_verdict, 0);
    let count_neq_verdict: usize = stdout_neq_verdict.trim().parse().unwrap();

    assert_eq!(
        count_eq_verdict + count_neq_verdict,
        total,
        "Counts of 'verdict == supported' ({}) and 'verdict != supported' ({}) must add up to total ({})",
        count_eq_verdict,
        count_neq_verdict,
        total
    );

    // 2. Also test on an inline document with str, int, bool fields containing nulls
    let doc = "\
schema:person{name, age:int, active:bool}\n\
Alice|30|true|\n\
Bob|25|false|\n\
Charlie|null|true|\n\
null|40|false|\n\
Eve|null|null|\n";

    let (code, stdout, _) = run_structl_stdin(&["query", "-", "--count"], doc);
    assert_eq!(code, 0);
    let doc_total: usize = stdout.trim().parse().unwrap();
    assert_eq!(doc_total, 5);

    // String field with null
    let (_, out_eq_name, _) = run_structl_stdin(
        &["query", "-", "--where", "name == Alice", "--count"],
        doc,
    );
    let (_, out_neq_name, _) = run_structl_stdin(
        &["query", "-", "--where", "name != Alice", "--count"],
        doc,
    );
    let eq_name: usize = out_eq_name.trim().parse().unwrap();
    let neq_name: usize = out_neq_name.trim().parse().unwrap();
    assert_eq!(eq_name + neq_name, doc_total);
    // null name (row 4) is counted in != Alice (Bob, Charlie, null, Eve = 4)
    assert_eq!(eq_name, 1);
    assert_eq!(neq_name, 4);

    // Int field with nulls
    let (_, out_eq_age, _) = run_structl_stdin(
        &["query", "-", "--where", "age == 30", "--count"],
        doc,
    );
    let (_, out_neq_age, _) = run_structl_stdin(
        &["query", "-", "--where", "age != 30", "--count"],
        doc,
    );
    let eq_age: usize = out_eq_age.trim().parse().unwrap();
    let neq_age: usize = out_neq_age.trim().parse().unwrap();
    assert_eq!(eq_age + neq_age, doc_total);
    assert_eq!(eq_age, 1);
    assert_eq!(neq_age, 4);

    // Bool field with null
    let (_, out_eq_active, _) = run_structl_stdin(
        &["query", "-", "--where", "active == true", "--count"],
        doc,
    );
    let (_, out_neq_active, _) = run_structl_stdin(
        &["query", "-", "--where", "active != true", "--count"],
        doc,
    );
    let eq_active: usize = out_eq_active.trim().parse().unwrap();
    let neq_active: usize = out_neq_active.trim().parse().unwrap();
    assert_eq!(eq_active + neq_active, doc_total);
    assert_eq!(eq_active, 2);
    assert_eq!(neq_active, 3);
}
