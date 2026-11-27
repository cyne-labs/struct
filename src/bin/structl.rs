//! structl: CLI tool for Struct files (Milestone Q1).
//!
//! Standard library only.

use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;
use structl::{run_query, OutputFormat, QueryError, QueryOptions};

fn print_usage_and_exit() -> ! {
    eprintln!(
        "Usage: structl query FILE [--schema TEXT | --schema-file PATH] [--class NAME]\n\
         \x20                  [--where EXPR]... [--fields a,b,...] [--count] [--format struct|json]"
    );
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage_and_exit();
    }

    if args[1] != "query" {
        eprintln!("error: unknown subcommand '{}', did you mean 'query'?", args[1]);
        process::exit(1);
    }

    let mut file_arg: Option<String> = None;
    let mut schema_arg: Option<String> = None;
    let mut schema_file_arg: Option<String> = None;
    let mut class_arg: Option<String> = None;
    let mut where_args: Vec<String> = Vec::new();
    let mut fields_arg: Option<Vec<String>> = None;
    let mut count_arg = false;
    let mut format_arg = OutputFormat::Struct;

    let mut i = 2;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--schema" {
            i += 1;
            if i >= args.len() {
                eprintln!("error: --schema requires an argument");
                process::exit(1);
            }
            if schema_arg.is_some() || schema_file_arg.is_some() {
                eprintln!("error: cannot specify multiple schema sources or both --schema and --schema-file");
                process::exit(1);
            }
            schema_arg = Some(args[i].clone());
        } else if arg == "--schema-file" {
            i += 1;
            if i >= args.len() {
                eprintln!("error: --schema-file requires an argument");
                process::exit(1);
            }
            if schema_arg.is_some() || schema_file_arg.is_some() {
                eprintln!("error: cannot specify multiple schema sources or both --schema and --schema-file");
                process::exit(1);
            }
            schema_file_arg = Some(args[i].clone());
        } else if arg == "--class" {
            i += 1;
            if i >= args.len() {
                eprintln!("error: --class requires an argument");
                process::exit(1);
            }
            class_arg = Some(args[i].clone());
        } else if arg == "--where" {
            i += 1;
            if i >= args.len() {
                eprintln!("error: --where requires an argument");
                process::exit(1);
            }
            where_args.push(args[i].clone());
        } else if arg == "--fields" {
            i += 1;
            if i >= args.len() {
                eprintln!("error: --fields requires an argument");
                process::exit(1);
            }
            let fields: Vec<String> = args[i]
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            fields_arg = Some(fields);
        } else if arg == "--count" {
            count_arg = true;
        } else if arg == "--format" {
            i += 1;
            if i >= args.len() {
                eprintln!("error: --format requires an argument");
                process::exit(1);
            }
            let fmt_str = args[i].to_lowercase();
            if fmt_str == "struct" {
                format_arg = OutputFormat::Struct;
            } else if fmt_str == "json" {
                format_arg = OutputFormat::Json;
            } else {
                eprintln!("error: invalid format '{}', must be struct or json", args[i]);
                process::exit(1);
            }
        } else if arg.starts_with('-') && arg != "-" {
            eprintln!("error: unknown flag '{}'", arg);
            process::exit(1);
        } else {
            if file_arg.is_some() {
                eprintln!("error: unexpected positional argument '{}'", arg);
                process::exit(1);
            }
            file_arg = Some(arg.clone());
        }
        i += 1;
    }

    let file_path = match file_arg {
        Some(f) => f,
        None => {
            eprintln!("error: missing FILE argument");
            process::exit(1);
        }
    };

    let content = if file_path == "-" {
        let mut buf = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buf) {
            eprintln!("error: failed to read from stdin: {}", e);
            process::exit(1);
        }
        buf
    } else {
        match fs::read_to_string(&file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: failed to read file '{}': {}", file_path, e);
                process::exit(1);
            }
        }
    };

    let outside_schemas = if let Some(text) = schema_arg {
        text
    } else if let Some(path) = schema_file_arg {
        match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: failed to read schema file '{}': {}", path, e);
                process::exit(1);
            }
        }
    } else {
        String::new()
    };

    let options = QueryOptions {
        class_name: class_arg,
        outside_schemas,
        where_clauses: where_args,
        fields: fields_arg,
        count: count_arg,
        format: format_arg,
    };

    match run_query(&content, &options) {
        Ok(output) => {
            print!("{}", output);
            process::exit(0);
        }
        Err(QueryError::Usage(msg)) | Err(QueryError::Io(msg)) => {
            eprintln!("error: {}", msg);
            process::exit(1);
        }
        Err(QueryError::Decode(err)) => {
            eprintln!("{}", err);
            process::exit(2);
        }
    }
}
