//! Float formatting tests for Struct v0.3.
//!
//! SPEC.md §3:
//! - plain form when the number is 0, or its size is at least 0.0001 and below 10000:
//!   3.14, 2024.5, 0.0012, -0.5. A whole number has no .0;
//! - exponent form otherwise: 1.2345e4 for 12345, 1e5 for 100000, 1e-5 for 0.00001;
//! - the special values are written NaN, inf and -inf, and negative zero is -0.
//!
//! Table test of at least 40 values covering both sides of 0.0001 and 10000,
//! whole numbers, negatives, 0, -0, NaN, inf, -inf, largest and smallest f64, and subnormals.

use structl::{decode_with, encode_with, Block, ClassDef, DecodedFile, FieldDef, FieldType, ScalarType, Value};

fn encode_single_float(f: f64) -> String {
    let mut file = DecodedFile::new();
    let class_def = ClassDef {
        name: "test".to_string(),
        fields: vec![FieldDef {
            name: "val".to_string(),
            field_type: FieldType::Scalar(ScalarType::Float),
        }],
    };
    file.schemas.insert("test".to_string(), class_def);
    file.blocks.push(Block {
        class_name: "test".to_string(),
        records: vec![Value::Record(vec![("val".to_string(), Value::Float(f))])],
    });
    let encoded = encode_with("schema:test{val:float}", &file).expect("encode failed");
    // Format is "val|\n"
    encoded.trim_end_matches('\n').trim_end_matches('|').to_string()
}

fn float_eq(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        true
    } else if a == 0.0 && b == 0.0 {
        a.is_sign_negative() == b.is_sign_negative()
    } else {
        a.to_bits() == b.to_bits()
    }
}

#[test]
fn test_float_canonical_formatting_table() {
    let test_cases: Vec<(f64, &str)> = vec![
        // 1. Special values
        (0.0, "0"),
        (-0.0, "-0"),
        (f64::NAN, "NaN"),
        (f64::INFINITY, "inf"),
        (f64::NEG_INFINITY, "-inf"),

        // 2. Whole numbers in [0.0001, 10000.0) - no .0
        (1.0, "1"),
        (-1.0, "-1"),
        (2.0, "2"),
        (-2.0, "-2"),
        (10.0, "10"),
        (-10.0, "-10"),
        (100.0, "100"),
        (-100.0, "-100"),
        (2024.0, "2024"),
        (-2024.0, "-2024"),
        (9999.0, "9999"),
        (-9999.0, "-9999"),

        // 3. Decimal numbers in [0.0001, 10000.0)
        (3.14, "3.14"),
        (-3.14, "-3.14"),
        (2024.5, "2024.5"),
        (-2024.5, "-2024.5"),
        (0.0012, "0.0012"),
        (-0.0012, "-0.0012"),
        (0.5, "0.5"),
        (-0.5, "-0.5"),
        (0.1, "0.1"),
        (-0.1, "-0.1"),
        (0.0001, "0.0001"),
        (-0.0001, "-0.0001"),
        (0.00015, "0.00015"),
        (-0.00015, "-0.00015"),
        (9999.9, "9999.9"),
        (-9999.9, "-9999.9"),

        // 4. Around 10000 boundary (>= 10000 is exponent form)
        (10000.0, "1e4"),
        (-10000.0, "-1e4"),
        (12345.0, "1.2345e4"),
        (-12345.0, "-1.2345e4"),
        (100000.0, "1e5"),
        (-100000.0, "-1e5"),

        // 5. Around 0.0001 boundary (< 0.0001 is exponent form)
        (0.00001, "1e-5"),
        (-0.00001, "-1e-5"),
        (0.0000999, "9.99e-5"),
        (-0.0000999, "-9.99e-5"),
        (0.000001, "1e-6"),
        (-0.000001, "-1e-6"),

        // 6. Largest and smallest normal f64
        (f64::MAX, "1.7976931348623157e308"),
        (f64::MIN, "-1.7976931348623157e308"),
        (f64::MIN_POSITIVE, "2.2250738585072014e-308"),
        (-f64::MIN_POSITIVE, "-2.2250738585072014e-308"),

        // 7. Subnormals (denormals)
        (f64::from_bits(1), "5e-324"),
        (-f64::from_bits(1), "-5e-324"),
        (f64::from_bits(2), "1e-323"),
        (-f64::from_bits(2), "-1e-323"),
        (f64::from_bits(42), "2.08e-322"),
        (f64::from_bits((1 << 52) - 1), "2.225073858507201e-308"),
    ];

    assert!(test_cases.len() >= 40, "Must have at least 40 test cases, found {}", test_cases.len());

    for (f, expected_str) in &test_cases {
        let actual_str = encode_single_float(*f);
        assert_eq!(
            &actual_str,
            expected_str,
            "Float {:?} ({:?}) encoded as '{}', expected '{}'",
            f,
            f.to_bits(),
            actual_str,
            expected_str
        );

        // Round-trip test: decode the string back to float and compare
        let doc_text = format!("{}|\n", actual_str);
        let decoded = decode_with("schema:test{val:float}", &doc_text)
            .unwrap_or_else(|e| panic!("Failed to decode float string '{}': {:?}", actual_str, e));
        let read_val = match &decoded.blocks[0].records[0] {
            Value::Record(fields) => match fields[0].1 {
                Value::Float(rf) => rf,
                ref other => panic!("Expected Float, found {:?}", other),
            },
            ref other => panic!("Expected Record, found {:?}", other),
        };

        assert!(
            float_eq(*f, read_val),
            "Round-trip mismatch for '{}': input {:?}, decoded {:?}",
            expected_str,
            f,
            read_val
        );
    }
}
