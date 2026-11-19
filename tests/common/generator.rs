//! Shared random schema, record and file generator for the Struct test suites.
//!
//! Used by `roundtrip.rs` (the original generator, behaviour unchanged) and by
//! `doctor_tests.rs` (the "rich" configuration: several lists per parent, nested
//! lists, enums with varied labels, varied field names, multi-class files).
//!
//! Standard library only.

#![allow(dead_code)]

use super::SimpleRng;
use structl::{
    decode_with, encode_with, Block, ClassDef, DecodedFile, FieldDef, FieldType, ListItemType,
    RecordDef, ScalarType, Value,
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

/// Knobs for the generator.
#[derive(Debug, Clone, Copy)]
pub struct GenConfig {
    /// Fewest fields in any braces (inclusive).
    pub min_fields: usize,
    /// Most fields in any braces (inclusive).
    pub max_fields: usize,
    /// Most items in one list (inclusive). With 1, every non-empty list has exactly one item.
    pub max_list_items: usize,
    /// Rich mode: varied field names and enum labels, list-heavy field types,
    /// empty lists less common, extra random words in text values.
    pub rich: bool,
}

impl GenConfig {
    /// The configuration the round-trip suite has always used.
    pub const ROUNDTRIP: GenConfig = GenConfig {
        min_fields: 1,
        max_fields: 3,
        max_list_items: 1,
        rich: false,
    };

    /// The configuration the doctor damage suite uses.
    pub const RICH: GenConfig = GenConfig {
        min_fields: 2,
        max_fields: 4,
        max_list_items: 3,
        rich: true,
    };
}

const FIELD_NAME_POOL: [&str; 20] = [
    "title", "year", "score", "flag", "kind", "notes", "tags", "source", "id", "label", "status",
    "count", "ratio", "body", "items", "parts", "owner", "stage", "verdict", "evidence",
];

const LABEL_POOL: [&str; 12] = [
    "supported", "refuted", "mixed", "pending", "open", "closed", "draft", "final", "low", "mid",
    "high", "unknown",
];

const CLASS_NAME_POOL: [&str; 8] = [
    "claim", "item", "note", "entry", "task", "paper", "rule", "event",
];

const WORD_SYLLABLES: [&str; 12] = [
    "ka", "lo", "mi", "ren", "tur", "va", "shi", "dor", "pel", "nu", "gra", "xo",
];

pub fn canonicalize_record(val: Value, fields: &[FieldDef]) -> Value {
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

pub fn generate_random_class(
    name: &str,
    rng: &mut SimpleRng,
    current_depth: usize,
    cfg: &GenConfig,
) -> ClassDef {
    let fields = generate_random_fields(rng, current_depth, cfg);
    ClassDef {
        name: name.to_string(),
        fields,
    }
}

fn random_labels(rng: &mut SimpleRng, cfg: &GenConfig) -> Vec<String> {
    if !cfg.rich {
        return vec!["optA".into(), "optB".into(), "optC".into()];
    }
    let mut pool: Vec<&str> = LABEL_POOL.to_vec();
    let n = rng.gen_range(2, 4);
    let mut out = Vec::new();
    for _ in 0..n {
        let pick = rng.gen_range(0, pool.len() - 1);
        out.push(pool.remove(pick).to_string());
    }
    out
}

pub fn generate_random_fields(
    rng: &mut SimpleRng,
    current_depth: usize,
    cfg: &GenConfig,
) -> Vec<FieldDef> {
    let num_fields = rng.gen_range(cfg.min_fields, cfg.max_fields);
    let mut fields = Vec::new();
    let mut name_pool: Vec<&str> = FIELD_NAME_POOL.to_vec();

    for i in 0..num_fields {
        let field_name = if cfg.rich {
            let pick = rng.gen_range(0, name_pool.len() - 1);
            name_pool.remove(pick).to_string()
        } else {
            format!("f{}", i)
        };
        let field_type = if i == 0 || current_depth >= 3 {
            if current_depth >= 3 || rng.gen_bool(0.7) {
                generate_random_scalar_or_enum(rng, cfg)
            } else {
                let sub_fields = generate_random_fields(rng, current_depth + 1, cfg);
                FieldType::Record(RecordDef { fields: sub_fields })
            }
        } else if cfg.rich {
            match rng.gen_range(0, 8) {
                0 => FieldType::Scalar(ScalarType::Str),
                1 => FieldType::Scalar(ScalarType::Int),
                2 => FieldType::Scalar(ScalarType::Float),
                3 => FieldType::Scalar(ScalarType::Bool),
                4 => FieldType::Enum(random_labels(rng, cfg)),
                5 => {
                    let sub_fields = generate_random_fields(rng, current_depth + 1, cfg);
                    FieldType::Record(RecordDef { fields: sub_fields })
                }
                _ => {
                    let item_type = generate_random_list_item_type(rng, current_depth + 1, cfg);
                    FieldType::List(Box::new(item_type))
                }
            }
        } else {
            match rng.gen_range(0, 6) {
                0 => FieldType::Scalar(ScalarType::Str),
                1 => FieldType::Scalar(ScalarType::Int),
                2 => FieldType::Scalar(ScalarType::Float),
                3 => FieldType::Scalar(ScalarType::Bool),
                4 => FieldType::Enum(random_labels(rng, cfg)),
                5 => {
                    let sub_fields = generate_random_fields(rng, current_depth + 1, cfg);
                    FieldType::Record(RecordDef { fields: sub_fields })
                }
                _ => {
                    let item_type = generate_random_list_item_type(rng, current_depth + 1, cfg);
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

fn generate_random_scalar_or_enum(rng: &mut SimpleRng, cfg: &GenConfig) -> FieldType {
    match rng.gen_range(0, 4) {
        0 => FieldType::Scalar(ScalarType::Str),
        1 => FieldType::Scalar(ScalarType::Int),
        2 => FieldType::Scalar(ScalarType::Float),
        3 => FieldType::Scalar(ScalarType::Bool),
        _ => FieldType::Enum(random_labels(rng, cfg)),
    }
}

fn generate_random_list_item_type(
    rng: &mut SimpleRng,
    current_depth: usize,
    cfg: &GenConfig,
) -> ListItemType {
    if current_depth >= 3 {
        match rng.gen_range(0, 4) {
            0 => ListItemType::Scalar(ScalarType::Str),
            1 => ListItemType::Scalar(ScalarType::Int),
            2 => ListItemType::Scalar(ScalarType::Float),
            3 => ListItemType::Scalar(ScalarType::Bool),
            _ => ListItemType::Enum(random_labels(rng, cfg)),
        }
    } else {
        match rng.gen_range(0, 5) {
            0 => ListItemType::Scalar(ScalarType::Str),
            1 => ListItemType::Scalar(ScalarType::Int),
            2 => ListItemType::Scalar(ScalarType::Float),
            3 => ListItemType::Scalar(ScalarType::Bool),
            4 => ListItemType::Enum(random_labels(rng, cfg)),
            _ => {
                let sub_fields = generate_random_fields(rng, current_depth + 1, cfg);
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

pub fn generate_random_record(
    fields: &[FieldDef],
    rng: &mut SimpleRng,
    current_depth: usize,
    stats: &mut EdgeCaseStats,
    cfg: &GenConfig,
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
                    generate_scalar_value(*st, rng, stats, cfg)
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
                generate_random_record(&sub_rec.fields, rng, current_depth + 1, stats, cfg)
            }
            FieldType::List(item_type) => {
                let empty_p = if cfg.rich { 0.2 } else { 0.35 };
                if rng.gen_bool(empty_p) {
                    stats.empty_list_count += 1;
                    Value::List(Vec::new())
                } else {
                    let count = rng.gen_range(1, cfg.max_list_items);
                    let mut items = Vec::new();
                    for _ in 0..count {
                        let it = generate_list_item(item_type, rng, current_depth + 1, stats, cfg);
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
    cfg: &GenConfig,
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
                generate_scalar_value(*st, rng, stats, cfg)
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
            generate_random_record(&rec_def.fields, rng, current_depth + 1, stats, cfg)
        }
    }
}

/// A random lowercase word of 2 to 4 syllables.
fn random_word(rng: &mut SimpleRng) -> String {
    let n = rng.gen_range(2, 4);
    let mut w = String::new();
    for _ in 0..n {
        w.push_str(WORD_SYLLABLES[rng.gen_range(0, WORD_SYLLABLES.len() - 1)]);
    }
    w
}

pub fn generate_scalar_value(
    st: ScalarType,
    rng: &mut SimpleRng,
    stats: &mut EdgeCaseStats,
    cfg: &GenConfig,
) -> Value {
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
                    if cfg.rich {
                        random_word(rng)
                    } else {
                        "normal_word".to_string()
                    }
                }
                _ => {
                    if cfg.rich {
                        format!("{} {}", random_word(rng), random_word(rng))
                    } else {
                        "alphanumeric123".to_string()
                    }
                }
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
        ScalarType::Bool => Value::Bool(rng.gen_bool(0.5)),
    }
}

/// A generated multi-class, multi-record file written with outside schemas.
pub struct GeneratedFile {
    /// The outside schema lines, one per class, each ending in a line break.
    pub schemas: String,
    /// The data the file encodes (canonical form, already checked to round-trip).
    pub doc: DecodedFile,
    /// The canonical text, written with `encode_with(schemas, doc)`.
    pub text: String,
}

/// Generates a fresh random schema set (1 to 3 classes) and a random multi-record file
/// (several blocks, reused classes), writes it with outside schemas, and checks that
/// the writer and reader agree on it (a failure here is a bug in the generator or the
/// codec, not in the doctor).
pub fn generate_random_file(rng: &mut SimpleRng, cfg: &GenConfig) -> GeneratedFile {
    let mut stats = EdgeCaseStats::default();

    let num_classes = rng.gen_range(1, 3);
    let mut name_pool: Vec<&str> = CLASS_NAME_POOL.to_vec();
    let mut classes: Vec<ClassDef> = Vec::new();
    for _ in 0..num_classes {
        let pick = rng.gen_range(0, name_pool.len() - 1);
        let name = name_pool.remove(pick);
        classes.push(generate_random_class(name, rng, 1, cfg));
    }

    // Every class gets at least one block; extra blocks reuse classes at random.
    let num_blocks = rng.gen_range(1, 4).max(num_classes);
    let mut order: Vec<usize> = (0..num_classes).collect();
    // Fisher-Yates shuffle so the class order in the file varies.
    for i in (1..order.len()).rev() {
        let j = rng.gen_range(0, i);
        order.swap(i, j);
    }
    while order.len() < num_blocks {
        order.push(rng.gen_range(0, num_classes - 1));
    }

    let mut doc = DecodedFile::new();
    for c in &classes {
        doc.schemas.insert(c.name.clone(), c.clone());
    }
    for &ci in &order {
        let class_def = &classes[ci];
        let num_records = rng.gen_range(1, 4);
        let mut records = Vec::new();
        for _ in 0..num_records {
            let rec = generate_random_record(&class_def.fields, rng, 1, &mut stats, cfg);
            records.push(canonicalize_record(rec, &class_def.fields));
        }
        doc.blocks.push(Block {
            class_name: class_def.name.clone(),
            records,
        });
    }

    let mut schemas = String::new();
    for (_, c_def) in &doc.schemas {
        schemas.push_str(&c_def.format_schema_line());
    }

    let text = encode_with(&schemas, &doc).expect("encode_with failed in generator");
    let back = decode_with(&schemas, &text).expect("decode_with failed in generator");
    assert_eq!(
        back.blocks, doc.blocks,
        "generator sanity: outside-schema round trip differs\nSchemas:\n{}\nText:\n{}",
        schemas, text
    );

    GeneratedFile { schemas, doc, text }
}
