//! Tests verifying all examples in SPEC.md (v0.2).
//!
//! Checks decoding, values structure, and byte-for-byte re-encoding.

use structl::{decode, encode, lookup, Value};

#[test]
fn test_spec_section_4_records_example() {
    let input = "schema:claim{text, source:record{id, year:int}, verdict}\n\
                 Rope scaling hurts long ctx|2405.05254|2024|refuted|\n";

    let doc = decode(input).expect("decoding failed");
    assert_eq!(doc.blocks.len(), 1);
    assert_eq!(doc.blocks[0].class_name, "claim");
    assert_eq!(doc.blocks[0].records.len(), 1);

    let rec = &doc.blocks[0].records[0];
    assert_eq!(rec.get_field("text"), Some(&Value::String("Rope scaling hurts long ctx".into())));
    assert_eq!(rec.get_field("verdict"), Some(&Value::String("refuted".into())));

    let source = rec.get_field("source").expect("source field missing");
    assert_eq!(source.get_field("id"), Some(&Value::String("2405.05254".into())));
    assert_eq!(source.get_field("year"), Some(&Value::Int(2024)));

    let encoded = encode(&doc).expect("encoding failed");
    assert_eq!(encoded, input, "Encoded output must match SPEC.md example byte-for-byte");
}

#[test]
fn test_spec_section_5_nested_notes_example() {
    let input = "schema:claim{text, notes:list[record{text, subnotes:list[str]}]}\n\
                 Rope scaling hurts long ctx|\n\
                 >notes are from the abstract only|\n\
                 >>abstract mentions a 32k limit|\n\
                 >>appendix B clarifies|\n\
                 >table 3 contradicts|\n\
                 >>llama 7b tested only|\n";

    let doc = decode(input).expect("decoding failed");
    assert_eq!(doc.blocks.len(), 1);
    let rec = &doc.blocks[0].records[0];
    assert_eq!(rec.get_field("text"), Some(&Value::String("Rope scaling hurts long ctx".into())));

    let notes = rec.get_field("notes").expect("notes field missing").as_list().unwrap();
    assert_eq!(notes.len(), 2);

    // Note 0
    let note0 = &notes[0];
    assert_eq!(note0.get_field("text"), Some(&Value::String("notes are from the abstract only".into())));
    let subnotes0 = note0.get_field("subnotes").unwrap().as_list().unwrap();
    assert_eq!(subnotes0.len(), 2);
    assert_eq!(subnotes0[0], Value::String("abstract mentions a 32k limit".into()));
    assert_eq!(subnotes0[1], Value::String("appendix B clarifies".into()));

    // Note 1
    let note1 = &notes[1];
    assert_eq!(note1.get_field("text"), Some(&Value::String("table 3 contradicts".into())));
    let subnotes1 = note1.get_field("subnotes").unwrap().as_list().unwrap();
    assert_eq!(subnotes1.len(), 1);
    assert_eq!(subnotes1[0], Value::String("llama 7b tested only".into()));

    let encoded = encode(&doc).expect("encoding failed");
    assert_eq!(encoded, input, "Encoded output must match SPEC.md example byte-for-byte");
}

#[test]
fn test_spec_section_5_dotted_indices_example() {
    let input = "schema:claim{text, verdict:record{label, evidence:list[str], counter:list[str]}, notes:list[str]}\n\
                 Rope scaling hurts long ctx|refuted|\n\
                 >1.1|table 3 contradicts the claim|\n\
                 >1.2|abstract says otherwise|\n\
                 >2|notes are from the abstract only|\n";

    let doc = decode(input).expect("decoding failed");
    assert_eq!(doc.blocks.len(), 1);
    let rec = &doc.blocks[0].records[0];
    assert_eq!(rec.get_field("text"), Some(&Value::String("Rope scaling hurts long ctx".into())));

    let verdict = rec.get_field("verdict").expect("verdict field missing");
    assert_eq!(verdict.get_field("label"), Some(&Value::String("refuted".into())));

    let evidence = verdict.get_field("evidence").unwrap().as_list().unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0], Value::String("table 3 contradicts the claim".into()));

    let counter = verdict.get_field("counter").unwrap().as_list().unwrap();
    assert_eq!(counter.len(), 1);
    assert_eq!(counter[0], Value::String("abstract says otherwise".into()));

    let notes = rec.get_field("notes").unwrap().as_list().unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0], Value::String("notes are from the abstract only".into()));

    let encoded = encode(&doc).expect("encoding failed");
    assert_eq!(encoded, input, "Encoded output must match SPEC.md example byte-for-byte");
}

#[test]
fn test_spec_section_2_class_reuse_and_lookup() {
    let input = "schema:item{name, qty:int}\n\
                 Apple|5|\n\
                 schema:item\n\
                 Banana|10|\n";

    let doc = decode(input).expect("decoding failed");
    assert_eq!(doc.blocks.len(), 2);
    assert_eq!(doc.blocks[0].class_name, "item");
    assert_eq!(doc.blocks[1].class_name, "item");

    let items = lookup(&doc, "item");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].get_field("name"), Some(&Value::String("Apple".into())));
    assert_eq!(items[0].get_field("qty"), Some(&Value::Int(5)));
    assert_eq!(items[1].get_field("name"), Some(&Value::String("Banana".into())));
    assert_eq!(items[1].get_field("qty"), Some(&Value::Int(10)));

    let encoded = encode(&doc).expect("encoding failed");
    assert_eq!(encoded, input, "Encoded output must match class reuse example byte-for-byte");
}

#[test]
fn test_spec_section_3_null_records() {
    let input = "schema:claim{text, source:record{id, year:int}, verdict}\n\
                 Rope scaling hurts long ctx|~|~|refuted|\n";

    let doc = decode(input).expect("decoding failed");
    let rec = &doc.blocks[0].records[0];
    assert_eq!(rec.get_field("text"), Some(&Value::String("Rope scaling hurts long ctx".into())));
    // SPEC.md §3: A record whose columns are all ~ reads back as a null record
    assert_eq!(rec.get_field("source"), Some(&Value::Null));
    assert_eq!(rec.get_field("verdict"), Some(&Value::String("refuted".into())));

    let encoded = encode(&doc).expect("encoding failed");
    assert_eq!(encoded, input, "Encoded null record must emit ~ for every column byte-for-byte");
}

#[test]
fn test_spec_section_3_escapes_roundtrip() {
    let input = "schema:escaped{gt, pipe, backslash, newline, tilde, space, hash}\n\
                 \\>title|\\||\\\\|\\n|\\~|\\shello\\s|#tag|\n";

    let doc = decode(input).expect("decoding failed");
    let rec = &doc.blocks[0].records[0];
    assert_eq!(rec.get_field("gt"), Some(&Value::String(">title".into())));
    assert_eq!(rec.get_field("pipe"), Some(&Value::String("|".into())));
    assert_eq!(rec.get_field("backslash"), Some(&Value::String("\\".into())));
    assert_eq!(rec.get_field("newline"), Some(&Value::String("\n".into())));
    assert_eq!(rec.get_field("tilde"), Some(&Value::String("~".into())));
    assert_eq!(rec.get_field("space"), Some(&Value::String(" hello ".into())));
    assert_eq!(rec.get_field("hash"), Some(&Value::String("#tag".into())));

    let encoded = encode(&doc).expect("encoding failed");
    assert_eq!(encoded, input, "Encoded escapes must match byte-for-byte");
}

#[test]
fn test_spec_section_5_leniency_warning() {
    let input = "schema:claim{text, notes:list[str]}\n\
                 Claim 1|\n\
                 >1|Note 1|\n";

    let doc = decode(input).expect("decoding failed");
    assert_eq!(doc.warnings.len(), 1);
    assert_eq!(doc.warnings[0].line, 3);
    assert!(doc.warnings[0].message.contains("unneeded list index"));

    // Canonical encoding removes the unneeded index
    let expected_canonical = "schema:claim{text, notes:list[str]}\n\
                              Claim 1|\n\
                              >Note 1|\n";
    let encoded = encode(&doc).expect("encoding failed");
    assert_eq!(encoded, expected_canonical);
}

#[test]
fn test_positive_all_tilde_list_holding_record_without_child_lines() {
    // 1.2: Decoding a line where all of its columns are ~ gives a record with null values
    // and its lists, which are empty when there are no child lines. It never gives null.
    let input = "schema:item{name, tags:list[str]}\n\
                 ~|\n";

    let doc = decode(input).expect("decoding failed");
    assert_eq!(doc.blocks.len(), 1);
    let rec = &doc.blocks[0].records[0];
    assert!(!rec.is_null(), "Record containing a list must never decode to null");
    assert_eq!(
        rec,
        &Value::Record(vec![
            ("name".to_string(), Value::Null),
            ("tags".to_string(), Value::List(vec![])),
        ])
    );

    let encoded = encode(&doc).expect("encoding failed");
    assert_eq!(encoded, input);
}

#[test]
fn test_positive_all_tilde_list_holding_record_with_child_lines() {
    // 1.2: With child lines, decodes to record with null scalar values and populated list.
    let input = "schema:item{name, tags:list[str]}\n\
                 ~|\n\
                 >rust|\n";

    let doc = decode(input).expect("decoding failed");
    assert_eq!(doc.blocks.len(), 1);
    let rec = &doc.blocks[0].records[0];
    assert!(!rec.is_null(), "Record containing a list must never decode to null");
    assert_eq!(
        rec,
        &Value::Record(vec![
            ("name".to_string(), Value::Null),
            ("tags".to_string(), Value::List(vec![Value::String("rust".to_string())])),
        ])
    );

    let encoded = encode(&doc).expect("encoding failed");
    assert_eq!(encoded, input);
}

#[test]
fn test_positive_all_tilde_record_with_no_list_decodes_to_null() {
    // 1.2 & 1.3: Records with no list inside still decode to null when all their columns are ~.
    let input = "schema:item{name, qty:int}\n\
                 ~|~|\n";

    let doc = decode(input).expect("decoding failed");
    assert_eq!(doc.blocks.len(), 1);
    let rec = &doc.blocks[0].records[0];
    assert_eq!(rec, &Value::Null, "Record with no list must decode to Value::Null when all columns are ~");

    let encoded = encode(&doc).expect("encoding failed");
    assert_eq!(encoded, input);
}

#[test]
fn test_positive_both_encoder_input_forms_of_null_record() {
    // 1.3: A record with no list inside it can arrive as Value::Null or as a record
    // whose fields are all null. The encoder accepts both and writes the same text,
    // and decode(encode(x)) returns Value::Null either way.
    use structl::{Block, ClassDef, DecodedFile, FieldDef, FieldType, ScalarType};

    let mut doc_null = DecodedFile::new();
    let class_def = ClassDef {
        name: "item".to_string(),
        fields: vec![
            FieldDef { name: "name".to_string(), field_type: FieldType::Scalar(ScalarType::Str) },
            FieldDef { name: "qty".to_string(), field_type: FieldType::Scalar(ScalarType::Int) },
        ],
    };
    doc_null.schemas.insert("item".to_string(), class_def.clone());
    doc_null.blocks.push(Block {
        class_name: "item".to_string(),
        records: vec![Value::Null],
    });

    let mut doc_fields = DecodedFile::new();
    doc_fields.schemas.insert("item".to_string(), class_def);
    doc_fields.blocks.push(Block {
        class_name: "item".to_string(),
        records: vec![Value::Record(vec![
            ("name".to_string(), Value::Null),
            ("qty".to_string(), Value::Null),
        ])],
    });

    let text_null = encode(&doc_null).expect("encoding Value::Null record failed");
    let text_fields = encode(&doc_fields).expect("encoding record with all null fields failed");

    // Both encoder forms must produce identical text
    assert_eq!(text_null, text_fields);
    assert_eq!(text_null, "schema:item{name, qty:int}\n~|~|\n");

    // decode(encode(x)) returns Value::Null either way
    let decoded_from_null = decode(&text_null).expect("decoding text_null failed");
    let decoded_from_fields = decode(&text_fields).expect("decoding text_fields failed");
    assert_eq!(decoded_from_null.blocks[0].records[0], Value::Null);
    assert_eq!(decoded_from_fields.blocks[0].records[0], Value::Null);
}
