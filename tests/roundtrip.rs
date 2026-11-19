//! Random round-trip test suite for Struct v0.3.
//!
//! Generates random schemas up to 4 levels deep and random data covering all edge cases:
//! - ~ nulls in every scalar type
//! - records whose fields are all ~ (both Value::Null and Value::Record)
//! - all-~ records that contain lists (Value::Record)
//! - empty lists
//! - values containing |, \, ~, >, line breaks, leading/trailing spaces, and unicode
//! - values that need \r: line breaks, edge space, edge tab
//! - raw values whose inner lines start with >, schema: or spaces
//! - text values starting with schema:
//! - floats spread across the whole f64 range (specials, whole, decimal, subnormal, limits)
//! - files with one and several outside classes
//!
//! Runs every case twice:
//! 1. encode / decode (in-document schemas)
//! 2. encode_with / decode_with (outside schemas, 1 and several classes)
//! And verifies GBNF completeness: grammar.matches(&encoded_with) on every case.

mod common;

use common::{Grammar, SimpleRng};
use structl::{
    decode, decode_with, encode, encode_with, gbnf, Block, ClassDef, DecodedFile, FieldDef,
    FieldType, ListItemType, RecordDef, ScalarType, Value,
};

/// Statistics tracking coverage of all required edge cases.
#[derive(Default, Debug)]
pub struct EdgeCaseStats {
    pub total_cases: usize,
    pub null_str_count: usize,
    pub null_int_count: usize,
    pub null_float_count: usize,
    pub null_bool_count: usize,
    pub null_enum_count: usize,
    pub all_null_record_count: usize,
    pub all_null_fields_record_count: usize,
    pub all_null_list_holding_record_count: usize,
    pub empty_list_count: usize,
    pub pipe_val_count: usize,
    pub backslash_val_count: usize,
    pub tilde_val_count: usize,
    pub gt_val_count: usize,
    pub newline_val_count: usize,
    pub boundary_spaces_count: usize,
    pub unicode_val_count: usize,
    pub multi_block_count: usize,
    pub reused_class_count: usize,

    // v0.3 additions:
    pub raw_val_count: usize,
    pub raw_line_break_count: usize,
    pub raw_edge_space_count: usize,
    pub raw_edge_tab_count: usize,
    pub raw_inner_gt_count: usize,
    pub raw_inner_schema_count: usize,
    pub raw_inner_space_count: usize,
    pub text_starts_with_schema_count: usize,
    pub float_full_range_count: usize,
    pub single_outside_class_count: usize,
    pub multiple_outside_class_count: usize,
    pub completeness_verified_count: usize,
}

#[test]
fn test_5000_random_roundtrips() {
    let mut rng = SimpleRng::new(0xDEAD_BEEF_CAFE_1234);
    let mut stats = EdgeCaseStats::default();

    const NUM_CASES: usize = 5000;

    for case_idx in 0..NUM_CASES {
        stats.total_cases += 1;

        // Decide how many blocks in this document (1 to 3)
        let num_blocks = rng.gen_range(1, 3);
        if num_blocks > 1 {
            stats.multi_block_count += 1;
        }

        let mut doc = DecodedFile::new();
        let mut defined_classes: Vec<ClassDef> = Vec::new();

        for block_idx in 0..num_blocks {
            // Can we reuse a previously defined class?
            let class_def = if !defined_classes.is_empty() && rng.gen_bool(0.35) {
                stats.reused_class_count += 1;
                let pick = rng.gen_range(0, defined_classes.len() - 1);
                defined_classes[pick].clone()
            } else {
                let name = format!("cls_{}_{}", case_idx, block_idx);
                let c = generate_random_class(&name, &mut rng, 1);
                defined_classes.push(c.clone());
                doc.schemas.insert(name, c.clone());
                c
            };

            // Generate records for this block (1 to 2 records)
            let num_records = rng.gen_range(1, 2);
            let mut records = Vec::new();
            for _ in 0..num_records {
                let rec = generate_random_record(&class_def.fields, &mut rng, 1, &mut stats);
                records.push(canonicalize_record(rec, &class_def.fields));
            }

            doc.blocks.push(Block {
                class_name: class_def.name.clone(),
                records,
            });
        }

        // --- Run 1: In-document schemas (encode / decode) ---
        let encoded = encode(&doc).expect("Encoding failed during in-doc round-trip");
        let decoded = decode(&encoded).expect("Decoding failed during in-doc round-trip");
        assert_eq!(
            decoded.blocks, doc.blocks,
            "Decoded blocks mismatch on in-doc case {}\nEncoded text:\n{}",
            case_idx, encoded
        );
        let re_encoded = encode(&decoded).expect("Re-encoding failed");
        assert_eq!(
            encoded, re_encoded,
            "Re-encoded text mismatch on in-doc case {}",
            case_idx
        );

        // --- Run 2: Outside schemas (encode_with / decode_with) ---
        let mut schemas_str = String::new();
        for (_, c_def) in &doc.schemas {
            schemas_str.push_str(&c_def.format_schema_line());
            schemas_str.push('\n');
        }

        if doc.schemas.len() == 1 {
            stats.single_outside_class_count += 1;
        } else {
            stats.multiple_outside_class_count += 1;
        }

        let encoded_with = encode_with(&schemas_str, &doc)
            .expect("encode_with failed during outside schemas round-trip");
        let decoded_with = decode_with(&schemas_str, &encoded_with)
            .expect("decode_with failed during outside schemas round-trip");
        assert_eq!(
            decoded_with.blocks, doc.blocks,
            "Decoded blocks mismatch on outside-schemas case {}\nEncoded text:\n{}",
            case_idx, encoded_with
        );
        let re_encoded_with = encode_with(&schemas_str, &decoded_with).expect("Re-encode_with failed");
        assert_eq!(
            encoded_with, re_encoded_with,
            "Re-encoded text mismatch on outside-schemas case {}",
            case_idx
        );

        // --- Run 3: GBNF Completeness Check ---
        let gbnf_text = gbnf(&schemas_str).expect("GBNF generation failed");
        let grammar = Grammar::parse(&gbnf_text).expect("GBNF grammar parse failed");
        assert!(
            grammar.matches(&encoded_with),
            "GBNF completeness failure on case {}\nSchema:\n{}\nEncoded text:\n{}",
            case_idx, schemas_str, encoded_with
        );
        stats.completeness_verified_count += 1;
    }

    println!("\n========================================================");
    println!("RANDOM ROUND-TRIP TEST RESULTS (5,000 CASES)");
    println!("========================================================");
    println!("Total cases completed (x2 runs):             {}", stats.total_cases);
    println!("Single outside class runs:                   {}", stats.single_outside_class_count);
    println!("Multiple outside classes runs:               {}", stats.multiple_outside_class_count);
    println!("GBNF completeness checks verified:           {}", stats.completeness_verified_count);
    println!("Multi-block documents:                       {}", stats.multi_block_count);
    println!("Reused class blocks:                         {}", stats.reused_class_count);
    println!("~ null str fields:                           {}", stats.null_str_count);
    println!("~ null int fields:                           {}", stats.null_int_count);
    println!("~ null float fields:                         {}", stats.null_float_count);
    println!("~ null bool fields:                          {}", stats.null_bool_count);
    println!("~ null enum fields:                          {}", stats.null_enum_count);
    println!("Records with all ~ fields (Value::Null):     {}", stats.all_null_record_count);
    println!("Records with all ~ fields (Value::Record):   {}", stats.all_null_fields_record_count);
    println!("All-~ records containing lists:              {}", stats.all_null_list_holding_record_count);
    println!("Empty lists:                                 {}", stats.empty_list_count);
    println!("Values containing '|':                       {}", stats.pipe_val_count);
    println!("Values containing '\\':                      {}", stats.backslash_val_count);
    println!("Values containing '~':                       {}", stats.tilde_val_count);
    println!("Values containing '>':                       {}", stats.gt_val_count);
    println!("Values containing newline:                   {}", stats.newline_val_count);
    println!("Values with leading/trailing sp:             {}", stats.boundary_spaces_count);
    println!("Values containing Unicode:                   {}", stats.unicode_val_count);
    println!("Raw values (\\r total):                       {}", stats.raw_val_count);
    println!("Raw values with line breaks:                 {}", stats.raw_line_break_count);
    println!("Raw values with edge space:                  {}", stats.raw_edge_space_count);
    println!("Raw values with edge tab:                    {}", stats.raw_edge_tab_count);
    println!("Raw inner line starting with '>':            {}", stats.raw_inner_gt_count);
    println!("Raw inner line starting with 'schema:':      {}", stats.raw_inner_schema_count);
    println!("Raw inner line starting with space:          {}", stats.raw_inner_space_count);
    println!("Text values starting with 'schema:':         {}", stats.text_starts_with_schema_count);
    println!("Floats across full f64 range:                {}", stats.float_full_range_count);
    println!("========================================================\n");

    // Assert that every required edge case occurred
    assert!(stats.null_str_count > 0, "Must include null str");
    assert!(stats.null_int_count > 0, "Must include null int");
    assert!(stats.null_float_count > 0, "Must include null float");
    assert!(stats.null_bool_count > 0, "Must include null bool");
    assert!(stats.null_enum_count > 0, "Must include null enum");
    assert!(stats.all_null_record_count > 0, "Must include records whose fields are all ~ as Value::Null");
    assert!(stats.all_null_fields_record_count > 0, "Must include records whose fields are all ~ as Value::Record");
    assert!(stats.all_null_list_holding_record_count > 0, "Must include all-~ records containing lists");
    assert!(stats.empty_list_count > 0, "Must include empty lists");
    assert!(stats.pipe_val_count > 0, "Must include values containing |");
    assert!(stats.backslash_val_count > 0, "Must include values containing \\");
    assert!(stats.tilde_val_count > 0, "Must include values containing ~");
    assert!(stats.gt_val_count > 0, "Must include values containing >");
    assert!(stats.newline_val_count > 0, "Must include values containing line breaks");
    assert!(stats.boundary_spaces_count > 0, "Must include values containing leading/trailing spaces");
    assert!(stats.unicode_val_count > 0, "Must include values containing Unicode");
    assert!(stats.multi_block_count > 0, "Must include multi-block documents");
    assert!(stats.reused_class_count > 0, "Must include reused classes");
    assert!(stats.raw_val_count > 0, "Must include raw values");
    assert!(stats.raw_line_break_count > 0, "Must include raw values with line breaks");
    assert!(stats.raw_edge_space_count > 0, "Must include raw values with edge space");
    assert!(stats.raw_edge_tab_count > 0, "Must include raw values with edge tab");
    assert!(stats.raw_inner_gt_count > 0, "Must include raw inner lines starting with >");
    assert!(stats.raw_inner_schema_count > 0, "Must include raw inner lines starting with schema:");
    assert!(stats.raw_inner_space_count > 0, "Must include raw inner lines starting with space");
    assert!(stats.text_starts_with_schema_count > 0, "Must include text values starting with schema:");
    assert!(stats.float_full_range_count > 0, "Must include floats across full f64 range");
    assert!(stats.single_outside_class_count > 0, "Must include single outside class runs");
    assert!(stats.multiple_outside_class_count > 0, "Must include multiple outside classes runs");
    assert_eq!(stats.completeness_verified_count, NUM_CASES, "All cases must pass GBNF completeness");
}

fn canonicalize_record(val: Value, fields: &[FieldDef]) -> Value {
    match val {
        Value::Null => Value::Null,
        Value::Record(mut rec_fields) => {
            for f in fields {
                if let Some((_, v)) = rec_fields.iter_mut().find(|(k, _)| k == &f.name) {
                    match &f.field_type {
                        FieldType::Record(sub_r) => {
                            *v = canonicalize_record(v.clone(), &sub_r.fields);
                        }
                        FieldType::List(item_type) => {
                            if let Value::List(items) = v {
                                match &**item_type {
                                    ListItemType::Record(sub_r) => {
                                        *items = items
                                            .drain(..)
                                            .map(|it| canonicalize_record(it, &sub_r.fields))
                                            .collect();
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            let has_list = fields.iter().any(|f| f.has_list());
            if !has_list && rec_fields.iter().all(|(_, v)| v.is_null()) {
                Value::Null
            } else {
                Value::Record(rec_fields)
            }
        }
        other => other,
    }
}

fn generate_random_class(name: &str, rng: &mut SimpleRng, current_depth: usize) -> ClassDef {
    let fields = generate_random_fields(rng, current_depth);
    ClassDef {
        name: name.to_string(),
        fields,
    }
}

fn generate_random_fields(rng: &mut SimpleRng, current_depth: usize) -> Vec<FieldDef> {
    let num_fields = rng.gen_range(1, 3);
    let mut fields = Vec::new();

    for i in 0..num_fields {
        let field_name = format!("f{}", i);
        let field_type = if i == 0 || current_depth >= 3 {
            if current_depth >= 3 || rng.gen_bool(0.7) {
                generate_random_scalar_or_enum(rng)
            } else {
                let sub_fields = generate_random_fields(rng, current_depth + 1);
                FieldType::Record(RecordDef { fields: sub_fields })
            }
        } else {
            match rng.gen_range(0, 6) {
                0 => FieldType::Scalar(ScalarType::Str),
                1 => FieldType::Scalar(ScalarType::Int),
                2 => FieldType::Scalar(ScalarType::Float),
                3 => FieldType::Scalar(ScalarType::Bool),
                4 => FieldType::Enum(vec!["optA".into(), "optB".into(), "optC".into()]),
                5 => {
                    let sub_fields = generate_random_fields(rng, current_depth + 1);
                    FieldType::Record(RecordDef { fields: sub_fields })
                }
                _ => {
                    let item_type = generate_random_list_item_type(rng, current_depth + 1);
                    FieldType::List(Box::new(item_type))
                }
            }
        };

        fields.push(FieldDef {
            name: field_name,
            field_type,
        });
    }

    fields
}

fn generate_random_scalar_or_enum(rng: &mut SimpleRng) -> FieldType {
    match rng.gen_range(0, 4) {
        0 => FieldType::Scalar(ScalarType::Str),
        1 => FieldType::Scalar(ScalarType::Int),
        2 => FieldType::Scalar(ScalarType::Float),
        3 => FieldType::Scalar(ScalarType::Bool),
        _ => FieldType::Enum(vec!["optA".into(), "optB".into(), "optC".into()]),
    }
}

fn generate_random_list_item_type(rng: &mut SimpleRng, current_depth: usize) -> ListItemType {
    if current_depth >= 3 {
        match rng.gen_range(0, 4) {
            0 => ListItemType::Scalar(ScalarType::Str),
            1 => ListItemType::Scalar(ScalarType::Int),
            2 => ListItemType::Scalar(ScalarType::Float),
            3 => ListItemType::Scalar(ScalarType::Bool),
            _ => ListItemType::Enum(vec!["optA".into(), "optB".into(), "optC".into()]),
        }
    } else {
        match rng.gen_range(0, 5) {
            0 => ListItemType::Scalar(ScalarType::Str),
            1 => ListItemType::Scalar(ScalarType::Int),
            2 => ListItemType::Scalar(ScalarType::Float),
            3 => ListItemType::Scalar(ScalarType::Bool),
            4 => ListItemType::Enum(vec!["optA".into(), "optB".into(), "optC".into()]),
            _ => {
                let sub_fields = generate_random_fields(rng, current_depth + 1);
                ListItemType::Record(RecordDef { fields: sub_fields })
            }
        }
    }
}

fn generate_all_null_record_vals(fields: &[FieldDef]) -> Vec<(String, Value)> {
    let mut vals = Vec::new();
    for f in fields {
        let v = match &f.field_type {
            FieldType::Record(sub) => Value::Record(generate_all_null_record_vals(&sub.fields)),
            FieldType::List(_) => Value::List(Vec::new()),
            _ => Value::Null,
        };
        vals.push((f.name.clone(), v));
    }
    vals
}

fn generate_random_record(
    fields: &[FieldDef],
    rng: &mut SimpleRng,
    current_depth: usize,
    stats: &mut EdgeCaseStats,
) -> Value {
    let has_list = fields.iter().any(|f| f.has_list());

    if !has_list && !fields.is_empty() {
        if rng.gen_bool(0.04) {
            stats.all_null_record_count += 1;
            return Value::Null;
        } else if rng.gen_bool(0.04) {
            stats.all_null_fields_record_count += 1;
            return Value::Record(generate_all_null_record_vals(fields));
        }
    } else if has_list && !fields.is_empty() {
        if rng.gen_bool(0.05) {
            stats.all_null_list_holding_record_count += 1;
            return Value::Record(generate_all_null_record_vals(fields));
        }
    }

    let mut record_vals = Vec::new();

    for field in fields {
        let val = match &field.field_type {
            FieldType::Scalar(st) => {
                if rng.gen_bool(0.10) {
                    match st {
                        ScalarType::Str => stats.null_str_count += 1,
                        ScalarType::Int => stats.null_int_count += 1,
                        ScalarType::Float => stats.null_float_count += 1,
                        ScalarType::Bool => stats.null_bool_count += 1,
                    }
                    Value::Null
                } else {
                    generate_scalar_value(*st, rng, stats)
                }
            }
            FieldType::Enum(labels) => {
                if rng.gen_bool(0.10) {
                    stats.null_enum_count += 1;
                    Value::Null
                } else {
                    let pick = rng.gen_range(0, labels.len() - 1);
                    Value::Enum(labels[pick].clone())
                }
            }
            FieldType::Record(sub_rec) => {
                generate_random_record(&sub_rec.fields, rng, current_depth + 1, stats)
            }
            FieldType::List(item_type) => {
                if rng.gen_bool(0.35) {
                    stats.empty_list_count += 1;
                    Value::List(Vec::new())
                } else {
                    let count = 1;
                    let mut items = Vec::new();
                    for _ in 0..count {
                        let it = generate_list_item(item_type, rng, current_depth + 1, stats);
                        items.push(it);
                    }
                    Value::List(items)
                }
            }
        };
        record_vals.push((field.name.clone(), val));
    }

    Value::Record(record_vals)
}

fn generate_list_item(
    item_type: &ListItemType,
    rng: &mut SimpleRng,
    current_depth: usize,
    stats: &mut EdgeCaseStats,
) -> Value {
    match item_type {
        ListItemType::Scalar(st) => {
            if rng.gen_bool(0.10) {
                match st {
                    ScalarType::Str => stats.null_str_count += 1,
                    ScalarType::Int => stats.null_int_count += 1,
                    ScalarType::Float => stats.null_float_count += 1,
                    ScalarType::Bool => stats.null_bool_count += 1,
                }
                Value::Null
            } else {
                generate_scalar_value(*st, rng, stats)
            }
        }
        ListItemType::Enum(labels) => {
            if rng.gen_bool(0.10) {
                stats.null_enum_count += 1;
                Value::Null
            } else {
                let pick = rng.gen_range(0, labels.len() - 1);
                Value::Enum(labels[pick].clone())
            }
        }
        ListItemType::Record(rec_def) => {
            generate_random_record(&rec_def.fields, rng, current_depth + 1, stats)
        }
    }
}

fn generate_scalar_value(st: ScalarType, rng: &mut SimpleRng, stats: &mut EdgeCaseStats) -> Value {
    match st {
        ScalarType::Str => {
            let s = match rng.gen_range(0, 18) {
                0 => {
                    stats.tilde_val_count += 1;
                    "~".to_string()
                }
                1 => {
                    stats.pipe_val_count += 1;
                    "pipe|inside".to_string()
                }
                2 => {
                    stats.backslash_val_count += 1;
                    r"path\to\file".to_string()
                }
                3 => {
                    stats.gt_val_count += 1;
                    ">leading_gt".to_string()
                }
                4 => {
                    stats.newline_val_count += 1;
                    stats.raw_val_count += 1;
                    stats.raw_line_break_count += 1;
                    "line1\nline2".to_string()
                }
                5 => {
                    stats.boundary_spaces_count += 1;
                    stats.raw_val_count += 1;
                    stats.raw_edge_space_count += 1;
                    "  spaced text  ".to_string()
                }
                6 => {
                    stats.boundary_spaces_count += 1;
                    stats.raw_val_count += 1;
                    stats.raw_edge_space_count += 1;
                    " leading_only".to_string()
                }
                7 => {
                    stats.boundary_spaces_count += 1;
                    stats.raw_val_count += 1;
                    stats.raw_edge_space_count += 1;
                    "trailing_only ".to_string()
                }
                8 => {
                    stats.raw_val_count += 1;
                    stats.raw_edge_tab_count += 1;
                    "\tleading_tab".to_string()
                }
                9 => {
                    stats.raw_val_count += 1;
                    stats.raw_edge_tab_count += 1;
                    "trailing_tab\t".to_string()
                }
                10 => {
                    stats.raw_val_count += 1;
                    stats.raw_line_break_count += 1;
                    stats.raw_inner_gt_count += 1;
                    "start line\n>child like line\nend line".to_string()
                }
                11 => {
                    stats.raw_val_count += 1;
                    stats.raw_line_break_count += 1;
                    stats.raw_inner_schema_count += 1;
                    "start line\nschema:fake{x}\nend line".to_string()
                }
                12 => {
                    stats.raw_val_count += 1;
                    stats.raw_line_break_count += 1;
                    stats.raw_inner_space_count += 1;
                    "first line\n   indented line\nthird line".to_string()
                }
                13 => {
                    stats.text_starts_with_schema_count += 1;
                    "schema:test_label".to_string()
                }
                14 => {
                    stats.unicode_val_count += 1;
                    "Hello 🚀 世界".to_string()
                }
                15 => {
                    stats.tilde_val_count += 1;
                    "~prefix".to_string()
                }
                16 => {
                    stats.pipe_val_count += 1;
                    stats.backslash_val_count += 1;
                    r"mixed\|pipe\\back".to_string()
                }
                17 => {
                    "normal_word".to_string()
                }
                _ => "alphanumeric123".to_string(),
            };
            Value::String(s)
        }
        ScalarType::Int => {
            let n = match rng.gen_range(0, 6) {
                0 => 0,
                1 => -17,
                2 => 2024,
                3 => i64::MAX,
                4 => i64::MIN,
                _ => (rng.next_u64() % 10000) as i64 - 5000,
            };
            Value::Int(n)
        }
        ScalarType::Float => {
            stats.float_full_range_count += 1;
            let f = match rng.gen_range(0, 16) {
                0 => 0.0,
                1 => -0.0,
                2 => f64::INFINITY,
                3 => f64::NEG_INFINITY,
                4 => f64::NAN,
                5 => 3.14159,
                6 => -0.5,
                7 => 1.0,
                8 => 100.0,
                9 => 2024.0,
                10 => 0.0001,
                11 => 0.0000999,
                12 => 10000.0,
                13 => 12345.0,
                14 => f64::from_bits(1), // 5e-324 subnormal
                15 => f64::MAX,
                _ => f64::MIN_POSITIVE,
            };
            Value::Float(f)
        }
        ScalarType::Bool => {
            Value::Bool(rng.gen_bool(0.5))
        }
    }
}
