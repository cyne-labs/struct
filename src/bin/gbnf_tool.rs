//! CLI helper for R2 smoke test benchmark.
//!
//! Standard library only.

use std::env;
use std::fs;
use std::process;
use structl::{decode_with, gbnf, gbnf_triggered};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: gbnf_tool <generate|verify> ...");
        process::exit(1);
    }

    match args[1].as_str() {
        "generate" => {
            if args.len() < 3 {
                eprintln!("Usage: gbnf_tool generate <schema_file> [trigger]");
                process::exit(1);
            }
            let schema_path = &args[2];
            let schemas = fs::read_to_string(schema_path).expect("Failed to read schema file");
            let trigger = if args.len() >= 4 && !args[3].is_empty() {
                Some(args[3].as_str())
            } else {
                None
            };

            let grammar = match trigger {
                Some(trig) => gbnf_triggered(&schemas, trig).expect("gbnf_triggered failed"),
                None => gbnf(&schemas).expect("gbnf failed"),
            };

            print!("{}", grammar);
        }
        "verify" => {
            if args.len() < 4 {
                eprintln!("Usage: gbnf_tool verify <schema_file> <output_file>");
                process::exit(1);
            }
            let schema_path = &args[2];
            let output_path = &args[3];
            let schemas = fs::read_to_string(schema_path).expect("Failed to read schema file");
            let output = fs::read_to_string(output_path).expect("Failed to read output file");

            match decode_with(&schemas, &output) {
                Ok(doc) => {
                    let mut record_count = 0;
                    for b in doc.blocks {
                        record_count += b.records.len();
                    }
                    println!("{{\"ok\": true, \"records\": {}}}", record_count);
                }
                Err(e) => {
                    let escaped_msg = e.message.replace('"', "\\\"");
                    println!("{{\"ok\": false, \"error\": \"{}\", \"line\": {}, \"col\": {}}}", escaped_msg, e.line, e.column);
                }
            }
        }
        other => {
            eprintln!("Unknown command: {}", other);
            process::exit(1);
        }
    }
}
