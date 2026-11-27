//! Query engine for Struct (Milestone Q1).
//!
//! Standard library only.

use crate::decode::decode_with;
use crate::encode::encode_with;
use crate::schema::{ClassDef, FieldDef, FieldType, ScalarType};
use crate::types::{Block, DecodedFile, StructError, Value, float_eq};

/// Output format for `structl query`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Struct,
    Json,
}

/// Comparison operators for `--where` filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    Contains,
}

impl Operator {
    pub fn as_str(&self) -> &'static str {
        match self {
            Operator::Eq => "==",
            Operator::Neq => "!=",
            Operator::Lt => "<",
            Operator::Lte => "<=",
            Operator::Gt => ">",
            Operator::Gte => ">=",
            Operator::Contains => "contains",
        }
    }
}

/// A parsed `--where FIELD OP VALUE` clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhereClause {
    pub field: String,
    pub op: Operator,
    pub raw_value: String,
}

/// A compiled filter target.
#[derive(Debug, Clone)]
pub enum FilterTarget {
    Null,
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Enum(String),
    RecordNull,
}

/// A validated and compiled filter.
#[derive(Debug, Clone)]
pub struct CompiledWhere {
    pub field_path: String,
    pub op: Operator,
    pub target: FilterTarget,
}

/// Query execution options.
#[derive(Debug, Clone)]
pub struct QueryOptions {
    pub class_name: Option<String>,
    pub outside_schemas: String,
    pub where_clauses: Vec<String>,
    pub fields: Option<Vec<String>>,
    pub count: bool,
    pub format: OutputFormat,
}

/// Query error.
#[derive(Debug, Clone)]
pub enum QueryError {
    /// Usage errors (bad flag, unknown field, bad value): exit code 1.
    Usage(String),
    /// File failed to decode: exit code 2.
    Decode(StructError),
    /// I/O error: exit code 1.
    Io(String),
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryError::Usage(s) => write!(f, "{}", s),
            QueryError::Decode(e) => write!(f, "{}", e),
            QueryError::Io(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for QueryError {}

/// Escapes a string for JSON output.
pub fn escape_json(s: &str) -> String {
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

/// Converts a Struct Value into a JSON string representation.
pub fn value_to_json(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
        Value::Int(i) => i.to_string(),
        Value::Float(f) => {
            if f.is_nan() {
                "\"NaN\"".to_string()
            } else if f.is_infinite() {
                if f.is_sign_positive() { "\"inf\"".to_string() } else { "\"-inf\"".to_string() }
            } else if f.fract() == 0.0 {
                format!("{:.1}", f)
            } else {
                format!("{}", f)
            }
        }
        Value::String(s) => format!("\"{}\"", escape_json(s)),
        Value::Enum(s) => format!("\"{}\"", escape_json(s)),
        Value::Record(fields) => {
            let mut parts = Vec::new();
            for (k, val) in fields {
                parts.push(format!("\"{}\": {}", escape_json(k), value_to_json(val)));
            }
            format!("{{{}}}", parts.join(", "))
        }
        Value::List(items) => {
            let parts: Vec<String> = items.iter().map(value_to_json).collect();
            format!("[{}]", parts.join(", "))
        }
    }
}

/// Checks if a value is null (or a record whose fields are all null).
pub fn is_all_null(val: &Value) -> bool {
    match val {
        Value::Null => true,
        Value::Record(fields) => fields.iter().all(|(_, v)| is_all_null(v)),
        _ => false,
    }
}

/// Formats a text string according to Struct canonical rules.
fn format_text_canonical(s: &str) -> String {
    if s.is_empty() {
        return "null".to_string();
    }
    let needs_raw = s == "null"
        || s.contains('\n')
        || s.starts_with(' ')
        || s.starts_with('\t')
        || s.ends_with(' ')
        || s.ends_with('\t');

    if needs_raw {
        let mut out = String::from(r"\r");
        for c in s.chars() {
            match c {
                '\\' => out.push_str(r"\\"),
                '|' => out.push_str(r"\|"),
                '\r' => {}
                _ => out.push(c),
            }
        }
        out
    } else {
        let mut out = String::new();
        for (i, c) in s.chars().enumerate() {
            if i == 0 && c == '>' {
                out.push_str(r"\>");
                continue;
            }
            match c {
                '\\' => out.push_str(r"\\"),
                '|' => out.push_str(r"\|"),
                _ => out.push(c),
            }
        }
        out
    }
}

/// Formats a single field value for `--fields` output according to its FieldType.
pub fn format_field_value_struct(val: &Value, field_type: &FieldType) -> String {
    match field_type {
        FieldType::Record(rec_def) => {
            let mut parts = Vec::new();
            match val {
                Value::Record(fields) => {
                    for f in &rec_def.fields {
                        if let FieldType::List(_) = f.field_type {
                            continue;
                        }
                        let sub_val = fields
                            .iter()
                            .find(|(k, _)| k == &f.name)
                            .map(|(_, v)| v)
                            .unwrap_or(&Value::Null);
                        parts.push(format_field_value_struct(sub_val, &f.field_type));
                    }
                }
                _ => {
                    for f in &rec_def.fields {
                        if let FieldType::List(_) = f.field_type {
                            continue;
                        }
                        parts.push(format_field_value_struct(&Value::Null, &f.field_type));
                    }
                }
            }
            parts.join("|")
        }
        FieldType::List(_) => "null".to_string(),
        _ => match val {
            Value::Null => "null".to_string(),
            Value::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
            Value::Int(i) => i.to_string(),
            Value::Float(f) => {
                if f.is_nan() {
                    "NaN".to_string()
                } else if f.is_infinite() {
                    if f.is_sign_positive() { "inf".to_string() } else { "-inf".to_string() }
                } else if *f == 0.0 {
                    if f.is_sign_negative() { "-0".to_string() } else { "0".to_string() }
                } else {
                    let abs = f.abs();
                    if abs >= 0.0001 && abs < 10000.0 {
                        f.to_string()
                    } else {
                        format!("{:e}", f)
                    }
                }
            }
            Value::String(s) => format_text_canonical(s),
            Value::Enum(s) => format_text_canonical(s),
            Value::Record(_) => "null".to_string(),
            Value::List(_) => "null".to_string(),
        },
    }
}

/// Recursively collects all filterable and list field paths.
fn collect_paths_from_fields(
    fields: &[FieldDef],
    prefix: &str,
    paths: &mut Vec<String>,
    list_paths: &mut Vec<String>,
) {
    for f in fields {
        let full_name = if prefix.is_empty() {
            f.name.clone()
        } else {
            format!("{}.{}", prefix, f.name)
        };
        match &f.field_type {
            FieldType::Scalar(_) | FieldType::Enum(_) => {
                paths.push(full_name);
            }
            FieldType::Record(rec) => {
                paths.push(full_name.clone());
                collect_paths_from_fields(&rec.fields, &full_name, paths, list_paths);
            }
            FieldType::List(_) => {
                list_paths.push(full_name);
            }
        }
    }
}

/// Returns all valid filterable paths and list paths for a class.
pub fn get_all_paths(class_def: &ClassDef) -> (Vec<String>, Vec<String>) {
    let mut paths = Vec::new();
    let mut list_paths = Vec::new();
    collect_paths_from_fields(&class_def.fields, "", &mut paths, &mut list_paths);
    (paths, list_paths)
}

/// Field path lookup error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldPathError {
    IsList(String),
    Unknown(String, Vec<String>),
}

/// Resolves the FieldType of a dotted path within a class definition.
pub fn resolve_field_type<'a>(
    class_def: &'a ClassDef,
    path: &str,
) -> Result<&'a FieldType, FieldPathError> {
    let mut parts = path.split('.');
    let first = parts.next().unwrap();
    let current_field = class_def.fields.iter().find(|f| f.name == first);
    if current_field.is_none() {
        let (valid, _) = get_all_paths(class_def);
        return Err(FieldPathError::Unknown(path.to_string(), valid));
    }
    let mut current_type = &current_field.unwrap().field_type;
    for part in parts {
        match current_type {
            FieldType::Record(rec) => match rec.fields.iter().find(|f| f.name == part) {
                Some(f) => current_type = &f.field_type,
                None => {
                    let (valid, _) = get_all_paths(class_def);
                    return Err(FieldPathError::Unknown(path.to_string(), valid));
                }
            },
            FieldType::List(_) => {
                return Err(FieldPathError::IsList(path.to_string()));
            }
            _ => {
                let (valid, _) = get_all_paths(class_def);
                return Err(FieldPathError::Unknown(path.to_string(), valid));
            }
        }
    }
    if let FieldType::List(_) = current_type {
        return Err(FieldPathError::IsList(path.to_string()));
    }
    Ok(current_type)
}

/// Retrieves a value by dotted path from a record.
pub fn get_field_value<'a>(record: &'a Value, path: &str) -> &'a Value {
    let mut curr = record;
    for part in path.split('.') {
        match curr {
            Value::Record(fields) => {
                if let Some((_, val)) = fields.iter().find(|(k, _)| k == part) {
                    curr = val;
                } else {
                    return &Value::Null;
                }
            }
            Value::Null => return &Value::Null,
            _ => return &Value::Null,
        }
    }
    curr
}

/// Parses a raw `--where` expression string into a `WhereClause`.
pub fn parse_where_clause(raw: &str) -> Result<WhereClause, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("empty where expression".to_string());
    }

    // 1. Find field name: letters, digits, _, .
    let mut field_end = 0;
    for (i, c) in s.char_indices() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
            field_end = i + c.len_utf8();
        } else {
            break;
        }
    }

    if field_end == 0 {
        return Err(format!("invalid field name in where expression '{}'", s));
    }

    let field = s[0..field_end].to_string();
    let rest = s[field_end..].trim_start();

    // 2. Identify operator
    let (op, rest_val) = if rest.starts_with("==") {
        (Operator::Eq, &rest[2..])
    } else if rest.starts_with("!=") {
        (Operator::Neq, &rest[2..])
    } else if rest.starts_with("<=") {
        (Operator::Lte, &rest[2..])
    } else if rest.starts_with(">=") {
        (Operator::Gte, &rest[2..])
    } else if rest.starts_with('<') {
        (Operator::Lt, &rest[1..])
    } else if rest.starts_with('>') {
        (Operator::Gt, &rest[1..])
    } else if rest.starts_with("contains") {
        let after = &rest["contains".len()..];
        if after.is_empty() || after.starts_with(|c: char| c.is_whitespace()) {
            (Operator::Contains, after)
        } else {
            return Err(format!("invalid operator in where expression '{}'", s));
        }
    } else {
        return Err(format!("invalid or missing operator in where expression '{}'", s));
    };

    let mut val_str = rest_val.trim();
    if val_str.is_empty() {
        return Err(format!("missing value in where expression '{}'", s));
    }

    // Strip surrounding quotes
    if (val_str.starts_with('"') && val_str.ends_with('"') && val_str.len() >= 2)
        || (val_str.starts_with('\'') && val_str.ends_with('\'') && val_str.len() >= 2)
    {
        val_str = &val_str[1..val_str.len() - 1];
    }

    Ok(WhereClause {
        field,
        op,
        raw_value: val_str.to_string(),
    })
}

/// Compiles and type-checks a `WhereClause` against a `ClassDef`.
pub fn compile_where_clause(
    clause: &WhereClause,
    class_def: &ClassDef,
) -> Result<CompiledWhere, String> {
    let field_type = match resolve_field_type(class_def, &clause.field) {
        Ok(t) => t,
        Err(FieldPathError::IsList(name)) => {
            return Err(format!("list field '{}' cannot be filtered in Q1", name));
        }
        Err(FieldPathError::Unknown(name, valid)) => {
            return Err(format!("unknown field '{}'. Valid fields: {}", name, valid.join(", ")));
        }
    };

    // Check null target
    if clause.raw_value == "null" {
        if clause.op != Operator::Eq && clause.op != Operator::Neq {
            return Err(format!(
                "operator '{}' not allowed with null; only == and != are allowed",
                clause.op.as_str()
            ));
        }
        return Ok(CompiledWhere {
            field_path: clause.field.clone(),
            op: clause.op,
            target: FilterTarget::Null,
        });
    }

    match field_type {
        FieldType::Scalar(ScalarType::Int) => {
            if clause.op == Operator::Contains {
                return Err("operator 'contains' is only valid for str fields".to_string());
            }
            let n = clause.raw_value.parse::<i64>().map_err(|_| {
                format!("invalid int value '{}' for field '{}'", clause.raw_value, clause.field)
            })?;
            Ok(CompiledWhere {
                field_path: clause.field.clone(),
                op: clause.op,
                target: FilterTarget::Int(n),
            })
        }
        FieldType::Scalar(ScalarType::Float) => {
            if clause.op == Operator::Contains {
                return Err("operator 'contains' is only valid for str fields".to_string());
            }
            let f = clause.raw_value.parse::<f64>().map_err(|_| {
                format!("invalid float value '{}' for field '{}'", clause.raw_value, clause.field)
            })?;
            Ok(CompiledWhere {
                field_path: clause.field.clone(),
                op: clause.op,
                target: FilterTarget::Float(f),
            })
        }
        FieldType::Scalar(ScalarType::Bool) => {
            if clause.op == Operator::Contains {
                return Err("operator 'contains' is only valid for str fields".to_string());
            }
            let lower = clause.raw_value.to_lowercase();
            let b = if lower == "true" {
                true
            } else if lower == "false" {
                false
            } else {
                return Err(format!(
                    "invalid bool value '{}' for field '{}', must be true or false",
                    clause.raw_value, clause.field
                ));
            };
            Ok(CompiledWhere {
                field_path: clause.field.clone(),
                op: clause.op,
                target: FilterTarget::Bool(b),
            })
        }
        FieldType::Scalar(ScalarType::Str) => Ok(CompiledWhere {
            field_path: clause.field.clone(),
            op: clause.op,
            target: FilterTarget::Str(clause.raw_value.clone()),
        }),
        FieldType::Enum(labels) => {
            if clause.op == Operator::Contains {
                return Err("operator 'contains' is only valid for str fields".to_string());
            }
            if !labels.contains(&clause.raw_value) {
                return Err(format!(
                    "invalid enum label '{}' for field '{}'. Valid labels: {}",
                    clause.raw_value,
                    clause.field,
                    labels.join(", ")
                ));
            }
            Ok(CompiledWhere {
                field_path: clause.field.clone(),
                op: clause.op,
                target: FilterTarget::Enum(clause.raw_value.clone()),
            })
        }
        FieldType::Record(_) => {
            Err(format!(
                "record field '{}' can only be compared with null using == or !=",
                clause.field
            ))
        }
        FieldType::List(_) => {
            Err(format!("list field '{}' cannot be filtered in Q1", clause.field))
        }
    }
}

/// Evaluates a compiled where clause against a record.
pub fn eval_where(record: &Value, filter: &CompiledWhere) -> bool {
    let val = get_field_value(record, &filter.field_path);

    match &filter.target {
        FilterTarget::Null | FilterTarget::RecordNull => {
            let is_null = is_all_null(val);
            match filter.op {
                Operator::Eq => is_null,
                Operator::Neq => !is_null,
                _ => false,
            }
        }
        FilterTarget::Int(target_n) => {
            if is_all_null(val) {
                return filter.op == Operator::Neq;
            }
            match val {
                Value::Int(n) => match filter.op {
                    Operator::Eq => n == target_n,
                    Operator::Neq => n != target_n,
                    Operator::Lt => n < target_n,
                    Operator::Lte => n <= target_n,
                    Operator::Gt => n > target_n,
                    Operator::Gte => n >= target_n,
                    Operator::Contains => false,
                },
                _ => false,
            }
        }
        FilterTarget::Float(target_f) => {
            if is_all_null(val) {
                return filter.op == Operator::Neq;
            }
            match val {
                Value::Float(f) => match filter.op {
                    Operator::Eq => float_eq(*f, *target_f),
                    Operator::Neq => !float_eq(*f, *target_f),
                    Operator::Lt => *f < *target_f,
                    Operator::Lte => *f <= *target_f,
                    Operator::Gt => *f > *target_f,
                    Operator::Gte => *f >= *target_f,
                    Operator::Contains => false,
                },
                _ => false,
            }
        }
        FilterTarget::Bool(target_b) => {
            if is_all_null(val) {
                return filter.op == Operator::Neq;
            }
            match val {
                Value::Bool(b) => match filter.op {
                    Operator::Eq => b == target_b,
                    Operator::Neq => b != target_b,
                    Operator::Lt => !b && *target_b,
                    Operator::Lte => !b || *target_b,
                    Operator::Gt => *b && !target_b,
                    Operator::Gte => *b || !target_b,
                    Operator::Contains => false,
                },
                _ => false,
            }
        }
        FilterTarget::Str(target_s) => {
            if is_all_null(val) {
                return filter.op == Operator::Neq;
            }
            match val {
                Value::String(s) => match filter.op {
                    Operator::Eq => s == target_s,
                    Operator::Neq => s != target_s,
                    Operator::Lt => s < target_s,
                    Operator::Lte => s <= target_s,
                    Operator::Gt => s > target_s,
                    Operator::Gte => s >= target_s,
                    Operator::Contains => s.contains(target_s),
                },
                _ => false,
            }
        }
        FilterTarget::Enum(target_e) => {
            if is_all_null(val) {
                return filter.op == Operator::Neq;
            }
            match val {
                Value::Enum(s) | Value::String(s) => match filter.op {
                    Operator::Eq => s == target_e,
                    Operator::Neq => s != target_e,
                    Operator::Lt => s < target_e,
                    Operator::Lte => s <= target_e,
                    Operator::Gt => s > target_e,
                    Operator::Gte => s >= target_e,
                    Operator::Contains => false,
                },
                _ => false,
            }
        }
    }
}

/// Validates that all requested fields exist in the class definition.
pub fn validate_fields(fields: &[String], class_def: &ClassDef) -> Result<(), String> {
    for f in fields {
        match resolve_field_type(class_def, f) {
            Ok(_) => {}
            Err(FieldPathError::IsList(name)) => {
                return Err(format!("list field '{}' cannot be selected in Q1", name));
            }
            Err(FieldPathError::Unknown(name, valid)) => {
                return Err(format!(
                    "unknown field '{}' in --fields. Valid fields: {}",
                    name,
                    valid.join(", ")
                ));
            }
        }
    }
    Ok(())
}

/// Executes a query on Struct document content.
pub fn run_query(input: &str, options: &QueryOptions) -> Result<String, QueryError> {
    // 1. Decode file (returns exit code 2 on decode error)
    let decoded = decode_with(&options.outside_schemas, input).map_err(QueryError::Decode)?;

    // 2. Select class
    let class_def = if let Some(ref name) = options.class_name {
        match decoded.schemas.get(name) {
            Some(def) => def.clone(),
            None => {
                let available: Vec<String> = decoded.schemas.keys().cloned().collect();
                return Err(QueryError::Usage(format!(
                    "unknown class '{}'; available classes: {}",
                    name,
                    available.join(", ")
                )));
            }
        }
    } else {
        if decoded.schemas.is_empty() {
            return Err(QueryError::Usage("no classes defined in schema".to_string()));
        } else if decoded.schemas.len() == 1 {
            decoded.schemas.values().next().unwrap().clone()
        } else {
            let available: Vec<String> = decoded.schemas.keys().cloned().collect();
            return Err(QueryError::Usage(format!(
                "multiple classes found ({}); please specify one with --class",
                available.join(", ")
            )));
        }
    };

    // 3. Validate and compile --where filters
    let mut compiled_filters = Vec::new();
    for raw in &options.where_clauses {
        let clause = parse_where_clause(raw).map_err(QueryError::Usage)?;
        let compiled = compile_where_clause(&clause, &class_def).map_err(QueryError::Usage)?;
        compiled_filters.push(compiled);
    }

    // 4. Validate --fields if provided
    if let Some(ref fields) = options.fields {
        validate_fields(fields, &class_def).map_err(QueryError::Usage)?;
    }

    // 5. Filter records
    let all_records = decoded.lookup(&class_def.name);
    let mut matching = Vec::new();
    for rec in all_records {
        let mut matches = true;
        for filter in &compiled_filters {
            if !eval_where(rec, filter) {
                matches = false;
                break;
            }
        }
        if matches {
            matching.push(rec);
        }
    }

    // 6. Format output
    if options.count {
        return Ok(format!("{}\n", matching.len()));
    }

    match options.format {
        OutputFormat::Json => {
            let mut objects = Vec::new();
            for rec in matching {
                if let Some(ref fields) = options.fields {
                    let mut parts = Vec::new();
                    for f in fields {
                        let val = get_field_value(rec, f);
                        parts.push(format!("\"{}\": {}", escape_json(f), value_to_json(val)));
                    }
                    objects.push(format!("{{{}}}", parts.join(", ")));
                } else {
                    if let Value::Record(rec_fields) = rec {
                        let mut parts = Vec::new();
                        for (k, val) in rec_fields {
                            parts.push(format!("\"{}\": {}", escape_json(k), value_to_json(val)));
                        }
                        objects.push(format!("{{{}}}", parts.join(", ")));
                    } else {
                        objects.push(value_to_json(rec));
                    }
                }
            }
            Ok(format!("[{}]\n", objects.join(", ")))
        }
        OutputFormat::Struct => {
            if let Some(ref fields) = options.fields {
                let mut lines = Vec::new();
                for rec in matching {
                    let parts: Vec<String> = fields
                        .iter()
                        .map(|f| {
                            let val = get_field_value(rec, f);
                            let field_type = resolve_field_type(&class_def, f).unwrap();
                            format_field_value_struct(val, field_type)
                        })
                        .collect();
                    lines.push(format!("{}|", parts.join("|")));
                }
                if lines.is_empty() {
                    Ok(String::new())
                } else {
                    Ok(format!("{}\n", lines.join("\n")))
                }
            } else {
                if matching.is_empty() {
                    return Ok(String::new());
                }
                let mut single_file = DecodedFile::new();
                single_file.schemas.insert(class_def.name.clone(), class_def.clone());
                single_file.blocks.push(Block {
                    class_name: class_def.name.clone(),
                    records: matching.iter().map(|&r| r.clone()).collect(),
                });
                let outside = class_def.format_schema_line();
                encode_with(&outside, &single_file).map_err(|e| QueryError::Usage(e.to_string()))
            }
        }
    }
}
