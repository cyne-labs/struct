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
        SafeFix::MissingFinalPipe => "MissingFinalPipe",
    }
}

fn value_to_json(v: &structl::Value) -> String {
    match v {
        structl::Value::Null => "null".to_string(),
        structl::Value::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
        structl::Value::Int(i) => i.to_string(),
        structl::Value::Float(f) => {
            if f.fract() == 0.0 {
                format!("{:.1}", f)
            } else {
                format!("{}", f)
            }
        }
        structl::Value::String(s) => format!("\"{}\"", escape_json(s)),
        structl::Value::Enum(s) => format!("\"{}\"", escape_json(s)),
        structl::Value::Record(fields) => {
            let mut parts = Vec::new();
            for (k, val) in fields {
                parts.push(format!("\"{}\": {}", escape_json(k), value_to_json(val)));
            }
            format!("{{{}}}", parts.join(", "))
        }
        structl::Value::List(items) => {
            let parts: Vec<String> = items.iter().map(value_to_json).collect();
            format!("[{}]", parts.join(", "))
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 3 && args[1] == "prompt" {
        let schema_path = &args[2];
        let schemas = fs::read_to_string(schema_path).expect("Failed to read schema file");
        let p = structl::prompt(&schemas).expect("Failed to build prompt");
        print!("{}", p);
        return;
    }

    if args.len() < 3 {
        eprintln!("Usage: doctor_tool <schema_file> <input_file> [mode] [retry_hint]");
        eprintln!("   or: doctor_tool prompt <schema_file>");
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

    let mut records_json = Vec::new();
    if let Some(ref doc) = report.data {
        for b in &doc.blocks {
            for r in &b.records {
                records_json.push(value_to_json(r));
            }
        }
    }

    let hint_str = match report.hint {
        Some(ref h) => format!("\"{}\"", escape_json(h)),
        None => "null".to_string(),
    };

    println!(
        "{{\"has_data\": {}, \"record_count\": {}, \"records\": [{}], \"text\": \"{}\", \"repairs\": [{}], \"bad_lines\": [{}], \"hint\": {}}}",
        report.data.is_some(),
        record_count,
        records_json.join(", "),
        escape_json(&report.text),
        repairs_json.join(", "),
        bad_lines_json.join(", "),
        hint_str
    );
}
