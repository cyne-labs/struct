//! CLI helper for Struct Doctor (R3 Part C benchmark).
//!
//! Standard library only.

use std::env;
use std::fs;
use std::process;
use structl::{doctor, DoctorMode, DoctorOptions, SafeFix};

fn escape_json(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

fn fix_name(fix: SafeFix) -> &'static str {
    match fix {
        SafeFix::CodeFences => "CodeFences",
        SafeFix::EchoedSchema => "EchoedSchema",
        SafeFix::Chatter => "Chatter",
        SafeFix::HeaderRow => "HeaderRow",
        SafeFix::MissingFinalPipe => "MissingFinalPipe",
        SafeFix::CutOffLastRecord => "CutOffLastRecord",
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: doctor_tool <schema_file> <input_file> [mode] [retry_hint]");
        process::exit(1);
    }

    let schema_path = &args[1];
    let input_path = &args[2];

    let schemas = fs::read_to_string(schema_path).expect("Failed to read schema file");
    let input = fs::read_to_string(input_path).expect("Failed to read input file");

    let mode = if args.len() >= 4 && args[3].eq_ignore_ascii_case("strict") {
        DoctorMode::Strict
    } else {
        DoctorMode::Salvage
    };

    let retry_hint = if args.len() >= 5 && args[4].eq_ignore_ascii_case("false") {
        false
    } else {
        true
    };

    let options = DoctorOptions { mode, retry_hint };
    let report = doctor(&schemas, &input, &options);

    let mut record_count = 0;
    if let Some(ref doc) = report.data {
        for b in &doc.blocks {
            record_count += b.records.len();
        }
    }

    let mut repairs_json = Vec::new();
    for r in &report.repairs {
        repairs_json.push(format!(
            "{{\"line\": {}, \"col\": {}, \"fix\": \"{}\", \"original\": \"{}\", \"new_text\": \"{}\"}}",
            r.line,
            r.column,
            fix_name(r.fix),
            escape_json(&r.original),
            escape_json(&r.new_text)
        ));
    }

    let mut bad_lines_json = Vec::new();
    for b in &report.bad_lines {
        bad_lines_json.push(format!(
            "{{\"line\": {}, \"col\": {}, \"problem\": \"{}\", \"original\": \"{}\", \"dropped_as_child\": {}}}",
            b.line,
            b.column,
            escape_json(&b.problem),
            escape_json(&b.original),
            b.dropped_as_child
        ));
    }

    let hint_str = match report.hint {
        Some(ref h) => format!("\"{}\"", escape_json(h)),
        None => "null".to_string(),
    };

    println!(
        "{{\"has_data\": {}, \"record_count\": {}, \"text\": \"{}\", \"repairs\": [{}], \"bad_lines\": [{}], \"hint\": {}}}",
        report.data.is_some(),
        record_count,
        escape_json(&report.text),
        repairs_json.join(", "),
        bad_lines_json.join(", "),
        hint_str
    );
}
