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

use common::generator::{
    canonicalize_record, generate_random_class, generate_random_record, EdgeCaseStats, GenConfig,
};
use common::{Grammar, SimpleRng};
use structl::{
    decode, decode_with, encode, encode_with, gbnf, Block, ClassDef, DecodedFile,
};

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
                let c = generate_random_class(&name, &mut rng, 1, &GenConfig::ROUNDTRIP);
                defined_classes.push(c.clone());
                doc.schemas.insert(name, c.clone());
                c
            };

            // Generate records for this block (1 to 2 records)
            let num_records = rng.gen_range(1, 2);
            let mut records = Vec::new();
            for _ in 0..num_records {
                let rec = generate_random_record(&class_def.fields, &mut rng, 1, &mut stats, &GenConfig::ROUNDTRIP);
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

