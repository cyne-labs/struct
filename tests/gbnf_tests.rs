//! Tests for GBNF grammar generation, completeness, soundness, and canonical-only constraints.
//!
//! SPEC.md §8:
//! - Completeness: every canonical encoding of data for those schemas is accepted by the grammar.
//! - Soundness: every text the grammar accepts decodes without error against the same outside schemas.
//!   One exception: the grammar limits an int to 19 digits, so a 19-digit number above i64's maximum passes
//!   the grammar and is caught by the decoder.
//! - Canonical only: non-canonical variants are rejected by the grammar while accepted by decode_with.

mod common;

use common::{Grammar, SimpleRng};
use structl::{decode_with, gbnf};

#[test]
fn test_gbnf_canonical_only_at_least_10_variants() {
    // 1. Single class schema
    let schemas = "schema:claim{text:str, count:int, score:float}\nschema:item{name:str, notes:list[str]}";
    let grammar_text = gbnf(schemas).expect("gbnf generation failed");
    let grammar = Grammar::parse(&grammar_text).expect("Grammar parsing failed");

    // Canonical baseline: should match!
    let canonical = "schema:claim\nhello|42|3.14|\nschema:item\nmy_item|\n>a note|;\n";
    assert!(
        grammar.matches(canonical),
        "Canonical text must be accepted by grammar"
    );
    assert!(decode_with(schemas, canonical).is_ok(), "Canonical text must decode");

    // Test 1: space after '|'
    let non_canonical_1 = "schema:claim\nhello| 42|3.14|\n";
    assert!(decode_with(schemas, non_canonical_1).is_ok(), "Variant 1 must decode");
    assert!(!grammar.matches(non_canonical_1), "Variant 1 (space after |) must be rejected by grammar");

    // Test 2: unneeded index (item has only one list, so index 1 is unneeded)
    let non_canonical_2 = "schema:item\nmy_item|\n>1|a note|;\n";
    assert!(decode_with(schemas, non_canonical_2).is_ok(), "Variant 2 must decode");
    assert!(!grammar.matches(non_canonical_2), "Variant 2 (unneeded index) must be rejected by grammar");

    // Test 3: blank line between records
    let non_canonical_3 = "schema:claim\nhello|42|3.14|\n\nworld|10|2.5|\n";
    assert!(decode_with(schemas, non_canonical_3).is_ok(), "Variant 3 must decode");
    assert!(!grammar.matches(non_canonical_3), "Variant 3 (blank line) must be rejected by grammar");

    // Test 4: \r that isn't needed (no line breaks, no edge whitespace)
    let non_canonical_4 = "schema:claim\n\\rhello|42|3.14|\n";
    assert!(decode_with(schemas, non_canonical_4).is_ok(), "Variant 4 must decode");
    assert!(!grammar.matches(non_canonical_4), "Variant 4 (unneeded \\r) must be rejected by grammar");

    // Test 5: 1e2 for a float that should be 100
    let non_canonical_5 = "schema:claim\nhello|42|1e2|\n";
    assert!(decode_with(schemas, non_canonical_5).is_ok(), "Variant 5 must decode");
    assert!(!grammar.matches(non_canonical_5), "Variant 5 (1e2 for 100) must be rejected by grammar");

    // Test 6: +5 in a float
    let non_canonical_6 = "schema:claim\nhello|42|+5|\n";
    assert!(decode_with(schemas, non_canonical_6).is_ok(), "Variant 6 must decode");
    assert!(!grammar.matches(non_canonical_6), "Variant 6 (+5 in float) must be rejected by grammar");

    // Test 7: trailing space before a line break
    let non_canonical_7 = "schema:claim\nhello|42|3.14 |\n";
    assert!(decode_with(schemas, non_canonical_7).is_ok(), "Variant 7 must decode");
    assert!(!grammar.matches(non_canonical_7), "Variant 7 (trailing space) must be rejected by grammar");

    // Test 8: leading zero in int
    let non_canonical_8 = "schema:claim\nhello|07|3.14|\n";
    assert!(decode_with(schemas, non_canonical_8).is_ok(), "Variant 8 must decode");
    assert!(!grammar.matches(non_canonical_8), "Variant 8 (leading zero in int) must be rejected by grammar");

    // Test 9: trailing zero in float decimal part (3.140 instead of 3.14)
    let non_canonical_9 = "schema:claim\nhello|42|3.140|\n";
    assert!(decode_with(schemas, non_canonical_9).is_ok(), "Variant 9 must decode");
    assert!(!grammar.matches(non_canonical_9), "Variant 9 (trailing zero in float) must be rejected by grammar");

    // Test 10: .0 on whole number float (100.0 instead of 100)
    let non_canonical_10 = "schema:claim\nhello|42|100.0|\n";
    assert!(decode_with(schemas, non_canonical_10).is_ok(), "Variant 10 must decode");
    assert!(!grammar.matches(non_canonical_10), "Variant 10 (.0 on whole float) must be rejected by grammar");

    // Test 11: \r with space only in the middle (a b) where \r is not needed
    let non_canonical_11 = "schema:claim\n\\ra b|42|3.14|\n";
    assert!(decode_with(schemas, non_canonical_11).is_ok(), "Variant 11 must decode");
    assert!(!grammar.matches(non_canonical_11), "Variant 11 (\\r with middle space only) must be rejected by grammar");

    // Test 12: tab next to '|'
    let non_canonical_12 = "schema:claim\nhello|\t42|3.14|\n";
    assert!(decode_with(schemas, non_canonical_12).is_ok(), "Variant 12 must decode");
    assert!(!grammar.matches(non_canonical_12), "Variant 12 (tab next to |) must be rejected by grammar");
}

#[test]
fn test_gbnf_soundness_5000_random_walker_samples() {
    let mut rng = SimpleRng::new(0xCAFE_BABE_9876_5432);

    let schemas_pool = vec![
        "schema:doc{title:str, year:int, score:float, active:bool}",
        "schema:fix{file:str, patch:str}",
        "schema:claim{text:str, notes:list[str]}",
        "schema:review{title:str, rating:enum(poor,fair,good,excellent)}",
        "schema:c1{a:str, b:int}\nschema:c2{x:float, y:bool}",
    ];

    let mut grammars = Vec::new();
    for s in &schemas_pool {
        let g_text = gbnf(s).expect("gbnf failed");
        let g = Grammar::parse(&g_text).expect("grammar parse failed");
        grammars.push((*s, g));
    }

    const SAMPLES: usize = 5000;
    let mut int_overflow_count = 0;
    let mut success_count = 0;

    for i in 0..SAMPLES {
        let (schema_text, grammar) = &grammars[i % grammars.len()];
        let generated = grammar.random_walk(&mut rng);

        match decode_with(schema_text, &generated) {
            Ok(_) => {
                success_count += 1;
            }
            Err(e) => {
                // Section 8 allows exactly one exception:
                // "the grammar limits an int to 19 digits, so a 19-digit number above i64's maximum passes the grammar and is caught by the decoder."
                if e.message.contains("is out of i64 range") {
                    int_overflow_count += 1;
                } else {
                    panic!(
                        "Soundness violation on sample {}:\nSchema:\n{}\nGenerated:\n{}\nError:\n{:?}",
                        i, schema_text, generated, e
                    );
                }
            }
        }
    }

    println!("\n========================================================");
    println!("GBNF SOUNDNESS RANDOM WALKER RESULTS (5,000 SAMPLES)");
    println!("========================================================");
    println!("Total samples generated:                     {}", SAMPLES);
    println!("Successfully decoded samples:                {}", success_count);
    println!("Allowed 19-digit int overflow exceptions:    {}", int_overflow_count);
    println!("========================================================\n");

    assert_eq!(success_count + int_overflow_count, SAMPLES);
}
