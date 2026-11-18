//! Random round-trip test suite for Struct (v0.2).
//!
//! Generates random schemas up to 4 levels deep and random data including:
//! - ~ nulls in every scalar type
//! - records whose fields are all ~ (both Value::Null and Value::Record)
//! - all-~ records that contain lists (Value::Record)
//! - empty lists
//! - values containing |, \, ~, >, line breaks, leading/trailing spaces, and unicode
//! - files with several blocks and reused classes
//!
//! Checks decode(encode(x)) == x on at least 5,000 cases.
//! Compares floats by exact bits, treating NaN as equal to itself.

use structl::{decode, encode, Block, ClassDef, DecodedFile, FieldDef, FieldType, ListItemType, RecordDef, ScalarType, Value};

/// Small seeded pseudo-random number generator (SplitMix64).
/// Completely standard library only - no external crates.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    fn gen_range(&mut self, low: usize, high: usize) -> usize {
        if low >= high {
            return low;
        }
        let range = (high - low + 1) as u64;
        low + (self.next_u64() % range) as usize
    }

    fn gen_bool(&mut self, p_true: f64) -> bool {
        (self.next_u64() % 1000) < (p_true * 1000.0) as u64
    }
}

/// Statistics tracking coverage of all required edge cases.
#[derive(Default, Debug)]
struct EdgeCaseStats {
    total_cases: usize,
    null_str_count: usize,
    null_int_count: usize,
    null_float_count: usize,
    null_bool_count: usize,
    null_enum_count: usize,
    all_null_record_count: usize,
    all_null_fields_record_count: usize,
    all_null_list_holding_record_count: usize,
    empty_list_count: usize,
    pipe_val_count: usize,
    backslash_val_count: usize,
    tilde_val_count: usize,
    gt_val_count: usize,
    newline_val_count: usize,
    boundary_spaces_count: usize,
    unicode_val_count: usize,
    multi_block_count: usize,
    reused_class_count: usize,
}

#[test]
fn test_5000_random_roundtrips() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);
    let mut stats = EdgeCaseStats::default();

    const NUM_CASES: usize = 5000;

    for case_idx in 0..NUM_CASES {
        stats.total_cases += 1;

        // Decide how many blocks in this document (1 to 4)
        let num_blocks = rng.gen_range(1, 4);
        if num_blocks > 1 {
            stats.multi_block_count += 1;
        }

        let mut doc = DecodedFile::new();
        let mut defined_classes: Vec<ClassDef> = Vec::new();

        for block_idx in 0..num_blocks {
            // Can we reuse a previously defined class?
            let class_def = if !defined_classes.is_empty() && rng.gen_bool(0.4) {
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

            // Generate records for this block (1 to 3 records)
            let num_records = rng.gen_range(1, 3);
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

        // Encode
        let encoded = encode(&doc).expect("Encoding failed during round-trip");

        // Decode
        let decoded = decode(&encoded).expect("Decoding failed during round-trip");

        // Verify equality
        assert_eq!(
            decoded.blocks, doc.blocks,
            "Decoded blocks mismatch on case {}\nEncoded text:\n{}",
            case_idx, encoded
        );

        // Verify idempotency
        let re_encoded = encode(&decoded).expect("Re-encoding failed");
        assert_eq!(
            encoded, re_encoded,
            "Re-encoded text mismatch on case {}",
            case_idx
        );
    }

    println!("\n========================================================");
    println!("RANDOM ROUND-TRIP TEST RESULTS (5,000 CASES)");
    println!("========================================================");
    println!("Total cases completed:                       {}", stats.total_cases);
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
}

/// SPEC.md §3:
/// "A record can be null. A record whose columns are all ~ reads back as a null record.
/// There's no separate marker, so 'null record' and 'record with every field null' are the same thing."
/// "A record that contains a list is never null. If a record has a list anywhere inside it,
/// the list must exist, so the record must too. When all its columns are ~, it reads back as a
/// record with null values and its lists (empty when it has no child lines)."
fn canonicalize_record(val: Value, fields: &[FieldDef]) -> Value {
    match val {
        Value::Null => Value::Null,
        Value::Record(mut rec_fields) => {
            // First canonicalize subfields
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

/// Generates a random class definition up to 4 levels deep.
fn generate_random_class(name: &str, rng: &mut Rng, current_depth: usize) -> ClassDef {
    let fields = generate_random_fields(rng, current_depth);
    ClassDef {
        name: name.to_string(),
        fields,
    }
}

/// Generates random fields for a class or record.
fn generate_random_fields(rng: &mut Rng, current_depth: usize) -> Vec<FieldDef> {
    let num_fields = rng.gen_range(1, 4);
    let mut fields = Vec::new();

    // Ensure at least one field has columns (not a pure list) so lines can exist
    for i in 0..num_fields {
        let field_name = format!("f{}", i);
        let field_type = if i == 0 || current_depth >= 4 {
            // First field is always scalar/enum/record to guarantee at least 1 column
            if current_depth >= 4 || rng.gen_bool(0.7) {
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

fn generate_random_scalar_or_enum(rng: &mut Rng) -> FieldType {
    match rng.gen_range(0, 4) {
        0 => FieldType::Scalar(ScalarType::Str),
        1 => FieldType::Scalar(ScalarType::Int),
        2 => FieldType::Scalar(ScalarType::Float),
        3 => FieldType::Scalar(ScalarType::Bool),
        _ => FieldType::Enum(vec!["optA".into(), "optB".into(), "optC".into()]),
    }
}

fn generate_random_list_item_type(rng: &mut Rng, current_depth: usize) -> ListItemType {
    if current_depth >= 4 {
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

/// Helper to generate record values where all scalar columns are null and lists are empty.
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

/// Generates a random record conforming to `fields`.
fn generate_random_record(
    fields: &[FieldDef],
    rng: &mut Rng,
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
                if rng.gen_bool(0.12) {
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
                if rng.gen_bool(0.12) {
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
                // List: 30% empty, otherwise 1 to 2 items
                if rng.gen_bool(0.3) {
                    stats.empty_list_count += 1;
                    Value::List(Vec::new())
                } else {
                    let count = rng.gen_range(1, 2);
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
    rng: &mut Rng,
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

fn generate_scalar_value(st: ScalarType, rng: &mut Rng, stats: &mut EdgeCaseStats) -> Value {
    match st {
        ScalarType::Str => {
            // Generate tricky strings with edge cases
            let s = match rng.gen_range(0, 11) {
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
                    "line1\nline2".to_string()
                }
                5 => {
                    stats.boundary_spaces_count += 1;
                    "  spaced text  ".to_string()
                }
                6 => {
                    stats.unicode_val_count += 1;
                    "Hello 🚀 世界".to_string()
                }
                7 => {
                    stats.tilde_val_count += 1;
                    "~prefix".to_string()
                }
                8 => {
                    stats.pipe_val_count += 1;
                    stats.backslash_val_count += 1;
                    r"mixed\|pipe\\back".to_string()
                }
                9 => {
                    stats.boundary_spaces_count += 1;
                    " single_space ".to_string()
                }
                _ => "regular_word".to_string(),
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
            let f = match rng.gen_range(0, 7) {
                0 => 0.0,
                1 => -0.0,
                2 => f64::INFINITY,
                3 => f64::NEG_INFINITY,
                4 => f64::NAN,
                5 => 3.14159,
                6 => -0.5,
                _ => (rng.gen_range(1, 1000) as f64) * 1.5,
            };
            Value::Float(f)
        }
        ScalarType::Bool => {
            Value::Bool(rng.gen_bool(0.5))
        }
    }
}
