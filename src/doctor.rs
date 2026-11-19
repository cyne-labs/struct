//! Doctor module for repairing and validating Struct documents (SPEC.md §9).
//!
//! Standard library only.

use crate::encode::encode_with;
use crate::schema::{
    parse_outside_schemas, ClassDef, FieldDef, FieldType, ListItemType, ListTarget,
    RecordDef, ScalarType,
};
use crate::types::{Block, DecodedFile, StructError, Value};

/// Doctor operating mode: Strict or Salvage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorMode {
    /// In strict mode, if there is any bad line, no data is returned.
    Strict,
    /// In salvage mode, every good record is returned and bad lines are dropped.
    Salvage,
}

/// Options configuring the doctor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorOptions {
    /// The repair mode (default Salvage).
    pub mode: DoctorMode,
    /// Whether to generate a retry hint for models (default true).
    pub retry_hint: bool,
}

impl Default for DoctorOptions {
    fn default() -> Self {
        Self {
            mode: DoctorMode::Salvage,
            retry_hint: true,
        }
    }
}

/// The 5 safe fixes defined in SPEC.md §9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SafeFix {
    /// 1. Markdown code fences (` ``` `) removed.
    CodeFences,
    /// 2. Line identical to an outside schema definition removed.
    EchoedSchema,
    /// 3. Leading and trailing chatter lines removed.
    Chatter,
    /// 4. Missing final pipe added when values and columns are valid.
    MissingFinalPipe,
    /// 5. Incomplete cut-off record at end of output dropped.
    CutOffLastRecord,
}

/// A safe repair made to the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repair {
    /// 1-based line number in original text.
    pub line: usize,
    /// 1-based column number in original text.
    pub column: usize,
    /// The kind of safe fix applied.
    pub fix: SafeFix,
    /// The original text that was found.
    pub original: String,
    /// The replacement text written instead (empty string if removed).
    pub new_text: String,
}

impl Repair {
    pub fn kind(&self) -> SafeFix {
        self.fix
    }
    pub fn original_text(&self) -> &str {
        &self.original
    }
    pub fn repaired_text(&self) -> &str {
        &self.new_text
    }
}

/// A line that could not be safely repaired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadLine {
    /// 1-based line number in original text.
    pub line: usize,
    /// 1-based column number in original text.
    pub column: usize,
    /// Description of the problem.
    pub problem: String,
    /// The original line text.
    pub original: String,
    /// Whether this line was dropped as a child of another bad line.
    pub dropped_as_child: bool,
}

impl BadLine {
    pub fn message(&self) -> &str {
        &self.problem
    }
    pub fn original_text(&self) -> &str {
        &self.original
    }
    pub fn is_child(&self) -> bool {
        self.dropped_as_child
    }
}

/// Report produced by running the doctor.
#[derive(Debug, Clone, PartialEq)]
pub struct DoctorReport {
    /// The decoded data. None in strict mode with bad lines or if nothing usable remains.
    pub data: Option<DecodedFile>,
    /// The canonical text corresponding to `data`. Empty if `data` is None.
    pub text: String,
    /// All safe repairs made to the input.
    pub repairs: Vec<Repair>,
    /// All bad lines detected and reported.
    pub bad_lines: Vec<BadLine>,
    /// Formatted retry hint for the model (if enabled and bad lines exist).
    pub hint: Option<String>,
}

#[derive(Debug, Clone)]
struct SourceLine {
    line_num: usize,
    text: String,
}

#[derive(Debug, Clone)]
enum ColType {
    Str,
    Int,
    Float,
    Bool,
    Enum(Vec<String>),
}

#[derive(Debug, Clone)]
struct ColInfo {
    col_type: ColType,
}

fn collect_columns(fields: &[FieldDef], out: &mut Vec<ColInfo>) {
    for f in fields {
        match &f.field_type {
            FieldType::Scalar(s) => {
                let ct = match s {
                    ScalarType::Str => ColType::Str,
                    ScalarType::Int => ColType::Int,
                    ScalarType::Float => ColType::Float,
                    ScalarType::Bool => ColType::Bool,
                };
                out.push(ColInfo {
                    col_type: ct,
                });
            }
            FieldType::Enum(labels) => {
                out.push(ColInfo {
                    col_type: ColType::Enum(labels.clone()),
                });
            }
            FieldType::Record(r) => {
                collect_columns(&r.fields, out);
            }
            FieldType::List(_) => {}
        }
    }
}

fn item_type_columns(item_type: &ListItemType) -> Vec<ColInfo> {
    match item_type {
        ListItemType::Scalar(s) => {
            let ct = match s {
                ScalarType::Str => ColType::Str,
                ScalarType::Int => ColType::Int,
                ScalarType::Float => ColType::Float,
                ScalarType::Bool => ColType::Bool,
            };
            vec![ColInfo {
                col_type: ct,
            }]
        }
        ListItemType::Enum(labels) => {
            vec![ColInfo {
                col_type: ColType::Enum(labels.clone()),
            }]
        }
        ListItemType::Record(r) => {
            let mut out = Vec::new();
            collect_columns(&r.fields, &mut out);
            out
        }
    }
}

#[derive(Debug, Clone)]
struct ParsedField {
    trimmed_val: String,
    is_raw: bool,
    is_null: bool,
    start_line: usize,
    start_col: usize,
    invalid_escape: Option<(char, usize)>,
}

#[derive(Debug, Clone)]
struct DoctorEntry {
    start_line_num: usize,
    #[allow(dead_code)]
    end_line_num: usize,
    text: String,
    depth: usize,
    marker_col: usize,
    content_start_col: usize,
    is_schema: bool,
    schema_name: Option<String>,
    fields: Vec<ParsedField>,
    ends_with_pipe: bool,
    has_unclosed_raw: bool,
}

fn parse_data_entry(phys_lines: &[SourceLine], start_idx: usize) -> (DoctorEntry, usize) {
    let first_line = &phys_lines[start_idx];
    let start_line_num = first_line.line_num;

    let ws_count = first_line.text.chars().take_while(|&c| c == ' ' || c == '\t').count();
    let depth = first_line.text.chars().skip(ws_count).take_while(|&c| c == '>').count();
    let post_depth_ws = first_line.text.chars().skip(ws_count + depth).take_while(|&c| c == ' ' || c == '\t').count();
    let marker_col = ws_count + 1;
    let content_start_col = ws_count + depth + post_depth_ws + 1;

    let mut curr_line_idx = start_idx;
    let mut curr_char_idx = ws_count + depth;
    let mut fields: Vec<ParsedField> = Vec::new();
    let mut ends_with_pipe = false;
    let mut has_unclosed_raw = false;

    loop {
        let cur_line = &phys_lines[curr_line_idx];
        let chars: Vec<char> = cur_line.text.chars().collect();

        // Skip spaces and tabs before field value
        while curr_char_idx < chars.len() && (chars[curr_char_idx] == ' ' || chars[curr_char_idx] == '\t') {
            curr_char_idx += 1;
        }

        if curr_char_idx >= chars.len() {
            // Physical line ended without more fields
            break;
        }

        let field_start_line = cur_line.line_num;
        let field_start_col = curr_char_idx + 1;

        let is_raw = curr_char_idx + 1 < chars.len() && chars[curr_char_idx] == '\\' && chars[curr_char_idx + 1] == 'r';

        if is_raw {
            curr_char_idx += 2; // skip \ and r
            let mut raw_val = String::new();
            let mut invalid_escape = None;

            'raw_loop: loop {
                let cur_chars: Vec<char> = phys_lines[curr_line_idx].text.chars().collect();
                while curr_char_idx < cur_chars.len() {
                    let c = cur_chars[curr_char_idx];
                    if c == '\\' {
                        if curr_char_idx + 1 < cur_chars.len() {
                            let nc = cur_chars[curr_char_idx + 1];
                            if nc == '|' {
                                raw_val.push('|');
                                curr_char_idx += 2;
                            } else if nc == '\\' {
                                raw_val.push('\\');
                                curr_char_idx += 2;
                            } else {
                                if invalid_escape.is_none() {
                                    invalid_escape = Some((nc, curr_char_idx + 1));
                                }
                                raw_val.push('\\');
                                raw_val.push(nc);
                                curr_char_idx += 2;
                            }
                        } else {
                            if invalid_escape.is_none() {
                                invalid_escape = Some((' ', curr_char_idx + 1));
                            }
                            raw_val.push('\\');
                            curr_char_idx += 1;
                        }
                    } else if c == '|' {
                        curr_char_idx += 1;
                        ends_with_pipe = true;
                        break 'raw_loop;
                    } else {
                        raw_val.push(c);
                        curr_char_idx += 1;
                    }
                }

                // End of this physical line inside raw value
                if curr_line_idx + 1 < phys_lines.len() {
                    curr_line_idx += 1;
                    curr_char_idx = 0;
                    raw_val.push('\n');
                } else {
                    // Reached EOF!
                    has_unclosed_raw = true;
                    ends_with_pipe = false;
                    break 'raw_loop;
                }
            }

            fields.push(ParsedField {
                trimmed_val: raw_val,
                is_raw: true,
                is_null: false,
                start_line: field_start_line,
                start_col: field_start_col,
                invalid_escape,
            });

            if has_unclosed_raw {
                break;
            }

            // Check if there are more characters on this physical line
            let cur_chars: Vec<char> = phys_lines[curr_line_idx].text.chars().collect();
            while curr_char_idx < cur_chars.len() && (cur_chars[curr_char_idx] == ' ' || cur_chars[curr_char_idx] == '\t') {
                curr_char_idx += 1;
            }
            if curr_char_idx >= cur_chars.len() {
                break;
            }
        } else {
            // Non-raw field on current physical line
            let field_raw_start = curr_char_idx;
            let mut field_chars = Vec::new();
            let mut closed = false;
            let mut in_escape = false;
            let mut invalid_escape = None;

            while curr_char_idx < chars.len() {
                let c = chars[curr_char_idx];
                if in_escape {
                    in_escape = false;
                    match c {
                        '|' | '\\' | '~' | '>' | '#' => field_chars.push(c),
                        other => {
                            if invalid_escape.is_none() {
                                invalid_escape = Some((other, curr_char_idx));
                            }
                            field_chars.push('\\');
                            field_chars.push(other);
                        }
                    }
                    curr_char_idx += 1;
                    continue;
                }

                if c == '\\' {
                    in_escape = true;
                    curr_char_idx += 1;
                    continue;
                }

                if c == '|' {
                    curr_char_idx += 1;
                    closed = true;
                    ends_with_pipe = true;
                    break;
                }

                field_chars.push(c);
                curr_char_idx += 1;
            }

            if in_escape && invalid_escape.is_none() {
                invalid_escape = Some((' ', curr_char_idx));
            }

            let field_slice_end = if closed { curr_char_idx - 1 } else { curr_char_idx };
            let raw_slice: String = chars[field_raw_start..field_slice_end].iter().collect();
            let is_null = raw_slice.trim() == "~";

            let raw_str: String = field_chars.into_iter().collect();
            let trimmed = raw_str.trim_end_matches(|c| c == ' ' || c == '\t').to_string();

            fields.push(ParsedField {
                trimmed_val: trimmed,
                is_raw: false,
                is_null,
                start_line: field_start_line,
                start_col: field_start_col,
                invalid_escape,
            });

            if !closed {
                ends_with_pipe = false;
                break;
            }

            // Skip spaces and check if more on this line
            while curr_char_idx < chars.len() && (chars[curr_char_idx] == ' ' || chars[curr_char_idx] == '\t') {
                curr_char_idx += 1;
            }
            if curr_char_idx >= chars.len() {
                break;
            }
        }
    }

    let original_text = phys_lines[start_idx..=curr_line_idx]
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let end_line_num = phys_lines[curr_line_idx].line_num;

    (
        DoctorEntry {
            start_line_num,
            end_line_num,
            text: original_text,
            depth,
            marker_col,
            content_start_col,
            is_schema: false,
            schema_name: None,
            fields,
            ends_with_pipe,
            has_unclosed_raw,
        },
        curr_line_idx + 1,
    )
}

/// Validates whether a value string matches the expected ColType.
/// Returns Ok(()) or Err(problem_message).
fn validate_field_value(
    val_str: &str,
    col_type: &ColType,
    is_raw: bool,
    is_null: bool,
) -> Result<(), String> {
    if is_null {
        return Ok(());
    }

    if is_raw {
        match col_type {
            ColType::Str => {
                if val_str.is_empty() {
                    return Err("empty raw value is an error".to_string());
                }
                return Ok(());
            }
            _ => {
                return Err("raw value '\\r' is not allowed in non-str field".to_string());
            }
        }
    }

    let val_trimmed = val_str.trim();

    // Empty field check: SPEC §3: "An empty field is always an error. There's no empty string. Blank text is written ~."
    if val_trimmed.is_empty() {
        return Err("empty field is not allowed (use ~ for null)".to_string());
    }

    match col_type {
        ColType::Str => Ok(()),
        ColType::Int => {
            if val_trimmed.starts_with('+') {
                return Err(format!("leading '+' is not allowed in int '{}'", val_trimmed));
            }
            match val_trimmed.parse::<i64>() {
                Ok(_) => Ok(()),
                Err(_) => Err(format!("'{}' is not a valid int (i64)", val_trimmed)),
            }
        }
        ColType::Float => {
            if val_trimmed == "NaN"
                || val_trimmed == "inf"
                || val_trimmed == "-inf"
                || val_trimmed == "-0"
                || val_trimmed == "0"
                || val_trimmed.parse::<f64>().is_ok()
            {
                Ok(())
            } else {
                Err(format!("'{}' is not a valid float (f64)", val_trimmed))
            }
        }
        ColType::Bool => {
            if val_trimmed == "1" || val_trimmed == "0" {
                Ok(())
            } else {
                Err(format!(
                    "'{}' is not a valid bool (expected 1 or 0)",
                    val_trimmed
                ))
            }
        }
        ColType::Enum(labels) => {
            if labels.contains(&val_trimmed.to_string()) {
                Ok(())
            } else {
                Err(format!("'{}' is not one of: {}", val_trimmed, labels.join(", ")))
            }
        }
    }
}

/// Converts a field value string into a structl `Value`.
fn parse_field_to_value(
    val_str: &str,
    col_type: &ColType,
    is_raw: bool,
    is_null: bool,
) -> Result<Value, String> {
    if is_null {
        return Ok(Value::Null);
    }

    if is_raw {
        return Ok(Value::String(val_str.to_string()));
    }

    let val_trimmed = val_str.trim();
    match col_type {
        ColType::Str => Ok(Value::String(val_trimmed.to_string())),
        ColType::Int => {
            let n = val_trimmed
                .parse::<i64>()
                .map_err(|_| format!("'{}' is not a valid int (i64)", val_trimmed))?;
            Ok(Value::Int(n))
        }
        ColType::Float => {
            let f = if val_trimmed == "NaN" {
                f64::NAN
            } else if val_trimmed == "inf" {
                f64::INFINITY
            } else if val_trimmed == "-inf" {
                f64::NEG_INFINITY
            } else if val_trimmed == "-0" {
                -0.0
            } else if val_trimmed == "0" {
                0.0
            } else {
                val_trimmed.parse::<f64>()
                    .map_err(|_| format!("'{}' is not a valid float (f64)", val_trimmed))?
            };
            Ok(Value::Float(f))
        }
        ColType::Bool => {
            if val_trimmed == "1" {
                Ok(Value::Bool(true))
            } else if val_trimmed == "0" {
                Ok(Value::Bool(false))
            } else {
                Err(format!("'{}' is not a valid bool (expected 1 or 0)", val_trimmed))
            }
        }
        ColType::Enum(_) => Ok(Value::Enum(val_trimmed.to_string())),
    }
}

/// Recursively builds a `Value::Record` from flat column values.
fn build_record_value(
    fields: &[FieldDef],
    flat_values: &[Value],
    cursor: &mut usize,
) -> Value {
    let mut record_fields = Vec::new();
    let mut all_null = true;
    let mut has_columns = false;
    let contains_list = fields.iter().any(|f| f.has_list());

    for f in fields {
        match &f.field_type {
            FieldType::Scalar(_) | FieldType::Enum(_) => {
                has_columns = true;
                let val = if *cursor < flat_values.len() {
                    let v = flat_values[*cursor].clone();
                    *cursor += 1;
                    v
                } else {
                    Value::Null
                };
                if !val.is_null() {
                    all_null = false;
                }
                record_fields.push((f.name.clone(), val));
            }
            FieldType::Record(sub) => {
                let sub_val = build_record_value(&sub.fields, flat_values, cursor);
                if sub.num_columns() > 0 {
                    has_columns = true;
                    if !sub_val.is_null() {
                        all_null = false;
                    }
                }
                record_fields.push((f.name.clone(), sub_val));
            }
            FieldType::List(_) => {
                record_fields.push((f.name.clone(), Value::List(Vec::new())));
            }
        }
    }

    if !contains_list && has_columns && all_null {
        Value::Null
    } else {
        Value::Record(record_fields)
    }
}

#[derive(Debug, Clone)]
enum NodeSchema {
    Class(ClassDef),
    Record(RecordDef),
    Scalar,
    Enum,
}

impl NodeSchema {
    fn find_lists(&self) -> Vec<ListTarget> {
        match self {
            NodeSchema::Class(c) => c.find_lists(),
            NodeSchema::Record(r) => r.find_lists(),
            NodeSchema::Scalar | NodeSchema::Enum => Vec::new(),
        }
    }
}

fn get_entry_schema_at(
    idx: usize,
    entries: &[DoctorEntry],
    outside_classes: &std::collections::BTreeMap<String, ClassDef>,
    single_outside_class: Option<&ClassDef>,
) -> Option<NodeSchema> {
    let mut current_class = single_outside_class.cloned();
    let mut stack: Vec<NodeSchema> = Vec::new();

    for i in 0..=idx {
        let e = &entries[i];
        if e.is_schema {
            stack.clear();
            if let Some(ref name) = e.schema_name {
                current_class = outside_classes.get(name).cloned();
            }
            if i == idx {
                return None;
            }
            continue;
        }

        let d = e.depth;
        stack.truncate(d);
        let parent_schema = if d > 0 && d - 1 < stack.len() {
            Some(&stack[d - 1])
        } else {
            None
        };

        let node_schema = if d == 0 {
            current_class.as_ref().map(|cd| NodeSchema::Class(cd.clone()))
        } else if let Some(ps) = parent_schema {
            let lists = ps.find_lists();
            if lists.is_empty() {
                None
            } else if lists.len() == 1 {
                let target = &lists[0];
                match &target.item_type {
                    ListItemType::Record(r) => Some(NodeSchema::Record(r.clone())),
                    ListItemType::Scalar(_) => Some(NodeSchema::Scalar),
                    ListItemType::Enum(_) => Some(NodeSchema::Enum),
                }
            } else {
                if e.fields.is_empty() {
                    None
                } else {
                    let index_str = e.fields[0].trimmed_val.trim();
                    let matched = lists.iter().find(|t| {
                        t.canonical_index == index_str
                            || t.accepted_indices.iter().any(|a| a == index_str)
                    });
                    matched.map(|target| match &target.item_type {
                        ListItemType::Record(r) => NodeSchema::Record(r.clone()),
                        ListItemType::Scalar(_) => NodeSchema::Scalar,
                        ListItemType::Enum(_) => NodeSchema::Enum,
                    })
                }
            }
        } else {
            None
        };

        if let Some(ns) = node_schema {
            if i == idx {
                return Some(ns);
            }
            stack.push(ns);
        } else if i == idx {
            return None;
        }
    }

    None
}

fn try_fix4(
    entry: &DoctorEntry,
    active_class: Option<&ClassDef>,
    parent_schema: Option<&NodeSchema>,
) -> Option<(DoctorEntry, usize)> {
    if entry.is_schema || entry.has_unclosed_raw || entry.ends_with_pipe {
        return None;
    }

    let cand_content = format!("{}|", entry.text.trim_end());
    let temp_lines = vec![SourceLine {
        line_num: entry.start_line_num,
        text: cand_content.clone(),
    }];
    let (cand_entry, _) = parse_data_entry(&temp_lines, 0);

    if !cand_entry.ends_with_pipe || cand_entry.has_unclosed_raw {
        return None;
    }

    let last_phys_line = entry.text.split('\n').last().unwrap_or(&entry.text);
    let col = last_phys_line.trim_end().chars().count() + 1;

    if entry.depth == 0 {
        let cd = active_class?;
        let mut cols = Vec::new();
        collect_columns(&cd.fields, &mut cols);

        if cand_entry.fields.len() == cols.len() {
            for (c_idx, val) in cand_entry.fields.iter().enumerate() {
                if val.invalid_escape.is_some()
                    || validate_field_value(
                        &val.trimmed_val,
                        &cols[c_idx].col_type,
                        val.is_raw,
                        val.is_null,
                    )
                    .is_err()
                {
                    return None;
                }
            }
            return Some((cand_entry, col));
        }
    } else {
        let ps = parent_schema?;
        let lists = ps.find_lists();
        if lists.is_empty() {
            return None;
        }

        if lists.len() == 1 {
            let target = &lists[0];
            let item_cols = item_type_columns(&target.item_type);

            let mut used_index = false;
            if cand_entry.fields.len() == item_cols.len() + 1 {
                let candidate_index = cand_entry.fields[0].trimmed_val.trim();
                if target
                    .accepted_indices
                    .iter()
                    .any(|idx| idx == candidate_index)
                {
                    used_index = true;
                }
            }

            let data_fields = if used_index {
                &cand_entry.fields[1..]
            } else {
                &cand_entry.fields[..]
            };

            if data_fields.len() == item_cols.len() {
                for (c_idx, val) in data_fields.iter().enumerate() {
                    if val.invalid_escape.is_some()
                        || validate_field_value(
                            &val.trimmed_val,
                            &item_cols[c_idx].col_type,
                            val.is_raw,
                            val.is_null,
                        )
                        .is_err()
                    {
                        return None;
                    }
                }
                return Some((cand_entry, col));
            }
        } else {
            // Parent has 2 or more lists -> index is required
            if cand_entry.fields.is_empty() {
                return None;
            }
            let index_str = cand_entry.fields[0].trimmed_val.trim();
            let matched = lists.iter().find(|t| {
                t.canonical_index == index_str
                    || t.accepted_indices.iter().any(|a| a == index_str)
            });

            if let Some(target) = matched {
                let item_cols = item_type_columns(&target.item_type);
                let data_fields = &cand_entry.fields[1..];
                if data_fields.len() == item_cols.len() {
                    for (c_idx, val) in data_fields.iter().enumerate() {
                        if val.invalid_escape.is_some()
                            || validate_field_value(
                                &val.trimmed_val,
                                &item_cols[c_idx].col_type,
                                val.is_raw,
                                val.is_null,
                            )
                            .is_err()
                        {
                            return None;
                        }
                    }
                    return Some((cand_entry, col));
                }
            }
        }
    }

    None
}

/// Runs the doctor on `text` with `schemas` and `options`.
pub fn doctor(schemas: &str, text: &str, options: &DoctorOptions) -> DoctorReport {
    let outside_classes = match parse_outside_schemas(schemas) {
        Ok(m) => m,
        Err(_) => {
            return DoctorReport {
                data: None,
                text: String::new(),
                repairs: Vec::new(),
                bad_lines: Vec::new(),
                hint: None,
            };
        }
    };

    let mut outside_schema_defs = Vec::new();
    for line in schemas.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("schema:") && trimmed.contains('{') && !trimmed.contains('|') {
            let non_ws: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
            outside_schema_defs.push(non_ws);
        }
    }

    // 1. Collect physical lines
    let mut raw_lines: Vec<&str> = text.split('\n').collect();
    if raw_lines.last() == Some(&"") {
        raw_lines.pop();
    }

    let mut lines: Vec<SourceLine> = Vec::new();
    for (idx, line_str) in raw_lines.into_iter().enumerate() {
        let clean = line_str.strip_suffix('\r').unwrap_or(line_str);
        lines.push(SourceLine {
            line_num: idx + 1,
            text: clean.to_string(),
        });
    }

    let mut repairs: Vec<Repair> = Vec::new();

    let single_outside_class = if outside_classes.len() == 1 {
        outside_classes.values().next().cloned()
    } else {
        None
    };

    // Parse entries from physical lines, stripping code fences and echoed schemas outside raw values.
    let mut raw_entries: Vec<DoctorEntry> = Vec::new();
    let mut phys_idx = 0;

    while phys_idx < lines.len() {
        let cur_line = &lines[phys_idx];
        let trimmed = cur_line.text.trim();

        if trimmed.is_empty() {
            phys_idx += 1;
            continue;
        }

        if trimmed.starts_with("```") {
            repairs.push(Repair {
                line: cur_line.line_num,
                column: 1,
                fix: SafeFix::CodeFences,
                original: cur_line.text.clone(),
                new_text: String::new(),
            });
            phys_idx += 1;
            continue;
        }

        if trimmed.starts_with("schema:") && trimmed.contains('{') && !trimmed.contains('|') {
            let non_ws: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
            if outside_schema_defs.contains(&non_ws) {
                repairs.push(Repair {
                    line: cur_line.line_num,
                    column: 1,
                    fix: SafeFix::EchoedSchema,
                    original: cur_line.text.clone(),
                    new_text: String::new(),
                });
                phys_idx += 1;
                continue;
            }
        }

        if trimmed.starts_with("schema:") && !trimmed.contains('|') {
            let name = trimmed["schema:".len()..].trim().to_string();
            let ws_count = cur_line.text.chars().take_while(|&c| c == ' ' || c == '\t').count();
            raw_entries.push(DoctorEntry {
                start_line_num: cur_line.line_num,
                end_line_num: cur_line.line_num,
                text: cur_line.text.clone(),
                depth: 0,
                marker_col: ws_count + 1,
                content_start_col: ws_count + "schema:".len() + 1,
                is_schema: true,
                schema_name: Some(name),
                fields: Vec::new(),
                ends_with_pipe: false,
                has_unclosed_raw: false,
            });
            phys_idx += 1;
            continue;
        }

        let (entry, next_idx) = parse_data_entry(&lines, phys_idx);
        raw_entries.push(entry);
        phys_idx = next_idx;
    }

    // Leading chatter removal: Fix 4 is tried first before removing as chatter
    let mut first_non_chatter = 0;
    let initial_class = single_outside_class.clone();

    while first_non_chatter < raw_entries.len() {
        let e = &raw_entries[first_non_chatter];
        if e.is_schema {
            break;
        }
        if e.has_unclosed_raw || e.text.contains('|') {
            break;
        }
        // Entry contains no pipe and is not a schema line. Try Fix 4:
        if try_fix4(e, initial_class.as_ref(), None).is_some() {
            // Fix 4 can turn it into a valid record!
            break;
        }
        // Otherwise, it's leading chatter
        repairs.push(Repair {
            line: e.start_line_num,
            column: 1,
            fix: SafeFix::Chatter,
            original: e.text.clone(),
            new_text: String::new(),
        });
        first_non_chatter += 1;
    }

    if first_non_chatter > 0 {
        raw_entries.drain(0..first_non_chatter);
    }

    if raw_entries.is_empty() {
        repairs.sort_by_key(|r| (r.line, r.column));
        return DoctorReport {
            data: None,
            text: String::new(),
            repairs,
            bad_lines: Vec::new(),
            hint: None,
        };
    }

    // Find the last entry in raw_entries that contains '|', is schema, or has unclosed raw
    let last_data_line_idx = raw_entries
        .iter()
        .rposition(|e| e.ends_with_pipe || e.text.contains('|') || e.is_schema || e.has_unclosed_raw);

    // Process remaining entries: apply Fix 4 where applicable
    let mut entries: Vec<DoctorEntry> = Vec::new();
    let mut active_class = single_outside_class.clone();
    let mut schema_stack: Vec<NodeSchema> = Vec::new();

    let mut raw_iter = raw_entries.into_iter().enumerate();
    while let Some((idx, mut entry)) = raw_iter.next() {
        if entry.is_schema {
            schema_stack.clear();
            if let Some(ref name) = entry.schema_name {
                active_class = outside_classes.get(name).cloned();
            }
            entries.push(entry);
            continue;
        }

        let d = entry.depth;
        schema_stack.truncate(d);
        let parent_schema = if d > 0 && d - 1 < schema_stack.len() {
            Some(&schema_stack[d - 1])
        } else {
            None
        };

        let is_trailing_candidate = match last_data_line_idx {
            Some(last_idx) => idx > last_idx,
            None => true,
        };

        if !entry.ends_with_pipe && !entry.has_unclosed_raw {
            if let Some((repaired_entry, col)) = try_fix4(&entry, active_class.as_ref(), parent_schema) {
                repairs.push(Repair {
                    line: entry.end_line_num,
                    column: col,
                    fix: SafeFix::MissingFinalPipe,
                    original: entry.text.clone(),
                    new_text: repaired_entry.text.clone(),
                });
                entry = repaired_entry;
            } else if is_trailing_candidate {
                // Fix 4 cannot turn this trailing line into a valid record.
                // Therefore this line and all subsequent lines are trailing chatter.
                repairs.push(Repair {
                    line: entry.start_line_num,
                    column: 1,
                    fix: SafeFix::Chatter,
                    original: entry.text,
                    new_text: String::new(),
                });
                while let Some((_, rem)) = raw_iter.next() {
                    repairs.push(Repair {
                        line: rem.start_line_num,
                        column: 1,
                        fix: SafeFix::Chatter,
                        original: rem.text,
                        new_text: String::new(),
                    });
                }
                break;
            }
        }

        let node_schema = if d == 0 {
            active_class.as_ref().map(|cd| NodeSchema::Class(cd.clone()))
        } else if let Some(ps) = parent_schema {
            let lists = ps.find_lists();
            if lists.len() == 1 {
                let target = &lists[0];
                match &target.item_type {
                    ListItemType::Record(r) => Some(NodeSchema::Record(r.clone())),
                    ListItemType::Scalar(_) => Some(NodeSchema::Scalar),
                    ListItemType::Enum(_) => Some(NodeSchema::Enum),
                }
            } else if lists.len() > 1 && !entry.fields.is_empty() {
                let index_str = entry.fields[0].trimmed_val.trim();
                let matched = lists.iter().find(|t| {
                    t.canonical_index == index_str || t.accepted_indices.iter().any(|a| a == index_str)
                });
                matched.map(|target| match &target.item_type {
                    ListItemType::Record(r) => NodeSchema::Record(r.clone()),
                    ListItemType::Scalar(_) => NodeSchema::Scalar,
                    ListItemType::Enum(_) => NodeSchema::Enum,
                })
            } else {
                None
            }
        } else {
            None
        };

        if let Some(ns) = node_schema {
            schema_stack.push(ns);
        }

        entries.push(entry);
    }

    // Fix 5: Cut-off last record
    let mut ambiguous_last_line = false;

    if let Some(last_entry) = entries.last() {
        if !last_entry.is_schema {
            let last_idx = entries.len() - 1;
            let depth = last_entry.depth;
            let mut is_incomplete = false;

            let trailing_bs = last_entry.text.trim_end().chars().rev().take_while(|&c| c == '\\').count();
            let broken_escape = trailing_bs % 2 == 1;

            if last_entry.has_unclosed_raw {
                is_incomplete = true;
            } else if broken_escape {
                is_incomplete = true;
            } else if depth == 0 {
                if let Some(cd) = get_entry_schema_at(last_idx, &entries, &outside_classes, single_outside_class.as_ref()) {
                    if let NodeSchema::Class(c) = cd {
                        let mut cols = Vec::new();
                        collect_columns(&c.fields, &mut cols);
                        if last_entry.fields.len() < cols.len() || !last_entry.ends_with_pipe {
                            is_incomplete = true;
                        }
                    }
                }
            } else {
                let mut parent_entry = None;
                for prev_idx in (0..last_idx).rev() {
                    if !entries[prev_idx].is_schema && entries[prev_idx].depth == depth - 1 {
                        parent_entry = Some(prev_idx);
                        break;
                    }
                }
                if let Some(p_idx) = parent_entry {
                    if let Some(ps) = get_entry_schema_at(p_idx, &entries, &outside_classes, single_outside_class.as_ref()) {
                        let p_lists = ps.find_lists();
                        if p_lists.len() == 1 {
                            let target = &p_lists[0];
                            let item_cols = item_type_columns(&target.item_type).len();
                            let mut used_index = false;
                            if last_entry.fields.len() == item_cols + 1 {
                                let cand_idx = last_entry.fields[0].trimmed_val.trim();
                                if target.accepted_indices.iter().any(|idx| idx == cand_idx) {
                                    used_index = true;
                                }
                            }
                            let val_cols = if used_index {
                                last_entry.fields.len().saturating_sub(1)
                            } else {
                                last_entry.fields.len()
                            };
                            if val_cols < item_cols || !last_entry.ends_with_pipe {
                                is_incomplete = true;
                            }
                        } else if p_lists.len() > 1 {
                            if !last_entry.fields.is_empty() {
                                let v0 = &last_entry.fields[0];
                                let v0_str = v0.trimmed_val.trim();
                                let matches_index = p_lists.iter().find(|t| {
                                    t.canonical_index == v0_str
                                        || t.accepted_indices.iter().any(|a| a == v0_str)
                                });
                                let is_val_for_any = p_lists.iter().any(|t| {
                                    let cols = item_type_columns(&t.item_type);
                                    !cols.is_empty()
                                        && validate_field_value(
                                            &v0.trimmed_val,
                                            &cols[0].col_type,
                                            v0.is_raw,
                                            v0.is_null,
                                        )
                                        .is_ok()
                                });

                                if let Some(target) = matches_index {
                                    let item_cols = item_type_columns(&target.item_type).len();
                                    let val_cols = last_entry.fields.len().saturating_sub(1);
                                    if val_cols < item_cols || !last_entry.ends_with_pipe {
                                        if val_cols == 0 && is_val_for_any {
                                            ambiguous_last_line = true;
                                        } else {
                                            is_incomplete = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if is_incomplete {
                let dropped = entries.pop().unwrap();
                repairs.push(Repair {
                    line: dropped.start_line_num,
                    column: 1,
                    fix: SafeFix::CutOffLastRecord,
                    original: dropped.text,
                    new_text: String::new(),
                });
            }
        }
    }

    // Now validate remaining entries and construct decoded file
    let mut current_block: Option<Block> = None;
    let mut current_class_def: Option<ClassDef> = single_outside_class.clone();

    if let Some(ref cd) = current_class_def {
        current_block = Some(Block {
            class_name: cd.name.clone(),
            records: Vec::new(),
        });
    }

    let mut decoded = DecodedFile::new();
    for (k, v) in &outside_classes {
        decoded.schemas.insert(k.clone(), v.clone());
    }

    let mut bad_lines: Vec<BadLine> = Vec::new();

    struct StackNode {
        #[allow(dead_code)]
        depth: usize,
        line_num: usize,
        is_bad: bool,
        value: Value,
        schema: NodeSchema,
        target_list_path: Vec<usize>,
    }

    let mut stack: Vec<StackNode> = Vec::new();

    let flush_stack = |stack: &mut Vec<StackNode>,
                       target_len: usize,
                       current_block: &mut Option<Block>| {
        while stack.len() > target_len {
            let node = stack.pop().unwrap();
            if !node.is_bad {
                if stack.is_empty() {
                    if let Some(b) = current_block.as_mut() {
                        b.records.push(node.value);
                    }
                } else {
                    let parent_idx = stack.len() - 1;
                    let parent_node = &mut stack[parent_idx];
                    if !parent_node.is_bad {
                        add_item_to_value_list(
                            &mut parent_node.value,
                            &node.target_list_path,
                            node.value,
                        );
                    }
                }
            }
        }
    };

    let mut i = 0;
    while i < entries.len() {
        let entry = &entries[i];

        if entry.is_schema {
            flush_stack(&mut stack, 0, &mut current_block);
            if let Some(b) = current_block.take() {
                if !b.records.is_empty() {
                    decoded.blocks.push(b);
                }
            }

            let name = entry.schema_name.as_ref().unwrap();
            if let Some(cd) = outside_classes.get(name) {
                current_class_def = Some(cd.clone());
                current_block = Some(Block {
                    class_name: name.clone(),
                    records: Vec::new(),
                });
            } else {
                bad_lines.push(BadLine {
                    line: entry.start_line_num,
                    column: 8,
                    problem: format!("reusing class '{}' that hasn't been defined yet", name),
                    original: entry.text.clone(),
                    dropped_as_child: false,
                });
            }
            i += 1;
            continue;
        }

        // Data line
        let class_def = match current_class_def.as_ref() {
            Some(cd) => cd.clone(),
            None => {
                bad_lines.push(BadLine {
                    line: entry.start_line_num,
                    column: entry.content_start_col,
                    problem: "with several outside classes, data before the first schema line is an error".to_string(),
                    original: entry.text.clone(),
                    dropped_as_child: false,
                });
                i += 1;
                continue;
            }
        };

        if current_block.is_none() {
            current_block = Some(Block {
                class_name: class_def.name.clone(),
                records: Vec::new(),
            });
        }

        let depth = entry.depth;
        let line_num = entry.start_line_num;

        if entry.has_unclosed_raw {
            bad_lines.push(BadLine {
                line: line_num,
                column: entry.content_start_col,
                problem: "unclosed raw value".to_string(),
                original: entry.text.clone(),
                dropped_as_child: false,
            });
            let parent_depth = depth;
            i += 1;
            while i < entries.len() {
                let next_depth = entries[i].depth;
                if next_depth > parent_depth {
                    bad_lines.push(BadLine {
                        line: entries[i].start_line_num,
                        column: 1,
                        problem: format!("dropped as child of line {}", line_num),
                        original: entries[i].text.clone(),
                        dropped_as_child: true,
                    });
                    i += 1;
                } else {
                    break;
                }
            }
            continue;
        }

        // Depth check
        if depth > stack.len() {
            let prev_depth = stack.len().saturating_sub(1);
            bad_lines.push(BadLine {
                line: line_num,
                column: entry.marker_col,
                problem: format!(
                    "depth increased by more than one level (from depth {} to {})",
                    prev_depth, depth
                ),
                original: entry.text.clone(),
                dropped_as_child: false,
            });
            let parent_depth = depth;
            i += 1;
            while i < entries.len() {
                let next_depth = entries[i].depth;
                if next_depth > parent_depth {
                    bad_lines.push(BadLine {
                        line: entries[i].start_line_num,
                        column: 1,
                        problem: format!("dropped as child of line {}", line_num),
                        original: entries[i].text.clone(),
                        dropped_as_child: true,
                    });
                    i += 1;
                } else {
                    break;
                }
            }
            continue;
        }

        // Parent check for child lines
        if depth > 0 {
            let parent = &stack[depth - 1];
            if parent.is_bad {
                bad_lines.push(BadLine {
                    line: line_num,
                    column: 1,
                    problem: format!("dropped as child of line {}", parent.line_num),
                    original: entry.text.clone(),
                    dropped_as_child: true,
                });
                i += 1;
                continue;
            }

            let lists = parent.schema.find_lists();
            if lists.is_empty() {
                bad_lines.push(BadLine {
                    line: line_num,
                    column: entry.marker_col,
                    problem: "parent record has no list fields to attach child line".to_string(),
                    original: entry.text.clone(),
                    dropped_as_child: false,
                });
                let parent_depth = depth;
                i += 1;
                while i < entries.len() {
                    let next_depth = entries[i].depth;
                    if next_depth > parent_depth {
                        bad_lines.push(BadLine {
                            line: entries[i].start_line_num,
                            column: 1,
                            problem: format!("dropped as child of line {}", line_num),
                            original: entries[i].text.clone(),
                            dropped_as_child: true,
                        });
                        i += 1;
                    } else {
                        break;
                    }
                }
                continue;
            }
        }

        if ambiguous_last_line && i == entries.len() - 1 {
            bad_lines.push(BadLine {
                line: line_num,
                column: if !entry.fields.is_empty() {
                    entry.fields[0].start_col
                } else {
                    entry.content_start_col
                },
                problem: "ambiguous: missing list index or cut-off value".to_string(),
                original: entry.text.clone(),
                dropped_as_child: false,
            });
            i += 1;
            continue;
        }

        if !entry.ends_with_pipe {
            bad_lines.push(BadLine {
                line: line_num,
                column: entry.text.trim_end().chars().count() + 1,
                problem: "data line must always end with an unescaped '|'".to_string(),
                original: entry.text.clone(),
                dropped_as_child: false,
            });
            let parent_depth = depth;
            i += 1;
            while i < entries.len() {
                let next_depth = entries[i].depth;
                if next_depth > parent_depth {
                    bad_lines.push(BadLine {
                        line: entries[i].start_line_num,
                        column: 1,
                        problem: format!("dropped as child of line {}", line_num),
                        original: entries[i].text.clone(),
                        dropped_as_child: true,
                    });
                    i += 1;
                } else {
                    break;
                }
            }
            continue;
        }

        // Unwind stack to current depth
        flush_stack(&mut stack, depth, &mut current_block);

        let mut line_error: Option<(String, usize, usize)> = None; // (problem, err_line, err_col)

        if depth == 0 {
            let mut cols = Vec::new();
            collect_columns(&class_def.fields, &mut cols);

            if entry.fields.len() < cols.len() {
                line_error = Some((
                    format!("expected {} columns, found {}", cols.len(), entry.fields.len()),
                    line_num,
                    entry.content_start_col,
                ));
            } else if entry.fields.len() > cols.len() {
                line_error = Some((
                    format!("expected {} columns, found {}", cols.len(), entry.fields.len()),
                    line_num,
                    entry.content_start_col,
                ));
            } else {
                let mut parsed_vals = Vec::new();
                for (c_idx, f) in entry.fields.iter().enumerate() {
                    if let Some((nc, esc_col)) = f.invalid_escape {
                        line_error = Some((
                            format!("invalid escape sequence '\\{}'", nc),
                            f.start_line,
                            esc_col,
                        ));
                        break;
                    }

                    match validate_field_value(&f.trimmed_val, &cols[c_idx].col_type, f.is_raw, f.is_null) {
                        Ok(()) => match parse_field_to_value(&f.trimmed_val, &cols[c_idx].col_type, f.is_raw, f.is_null) {
                            Ok(pv) => parsed_vals.push(pv),
                            Err(e) => {
                                line_error = Some((e, f.start_line, f.start_col));
                                break;
                            }
                        },
                        Err(e) => {
                            line_error = Some((e, f.start_line, f.start_col));
                            break;
                        }
                    }
                }

                if line_error.is_none() {
                    let mut cursor = 0;
                    let record_val = build_record_value(&class_def.fields, &parsed_vals, &mut cursor);
                    stack.push(StackNode {
                        depth: 0,
                        line_num,
                        is_bad: false,
                        value: record_val,
                        schema: NodeSchema::Class(class_def.clone()),
                        target_list_path: Vec::new(),
                    });
                }
            }
        } else {
            // Child line
            let parent = &stack[depth - 1];
            let list_targets = parent.schema.find_lists();

            let (target_list, data_fields_start) = if list_targets.len() == 1 {
                let single_target = &list_targets[0];
                let item_cols = item_type_columns(&single_target.item_type).len();
                let mut used_index = false;

                if entry.fields.len() == item_cols + 1 {
                    let candidate_index = entry.fields[0].trimmed_val.trim();
                    if single_target
                        .accepted_indices
                        .iter()
                        .any(|idx| idx == candidate_index)
                    {
                        used_index = true;
                    }
                }

                if used_index {
                    (Some(single_target.clone()), 1)
                } else if entry.fields.len() < item_cols {
                    line_error = Some((
                        format!("expected {} columns, found {}", item_cols, entry.fields.len()),
                        line_num,
                        entry.content_start_col,
                    ));
                    (None, 0)
                } else if entry.fields.len() > item_cols {
                    line_error = Some((
                        format!("expected {} columns, found {}", item_cols, entry.fields.len()),
                        line_num,
                        entry.content_start_col,
                    ));
                    (None, 0)
                } else {
                    (Some(single_target.clone()), 0)
                }
            } else {
                // 2 or more lists -> index is required
                if entry.fields.is_empty() {
                    line_error = Some(("empty child line".to_string(), line_num, entry.content_start_col));
                    (None, 0)
                } else {
                    let index_str = entry.fields[0].trimmed_val.trim();
                    let matched = list_targets.iter().find(|t| {
                        t.canonical_index == index_str
                            || t.accepted_indices.iter().any(|a| a == index_str)
                    });

                    match matched {
                        Some(t) => {
                            let item_cols = item_type_columns(&t.item_type).len();
                            let actual_cols = entry.fields.len() - 1;
                            if actual_cols < item_cols {
                                line_error = Some((
                                    format!("expected {} columns, found {}", item_cols, actual_cols),
                                    line_num,
                                    entry.content_start_col,
                                ));
                                (None, 1)
                            } else if actual_cols > item_cols {
                                line_error = Some((
                                    format!("expected {} columns, found {}", item_cols, actual_cols),
                                    line_num,
                                    entry.content_start_col,
                                ));
                                (None, 1)
                            } else {
                                (Some(t.clone()), 1)
                            }
                        }
                        None => {
                            line_error = Some((
                                "missing list index".to_string(),
                                line_num,
                                entry.fields[0].start_col,
                            ));
                            (None, 0)
                        }
                    }
                }
            };

            if line_error.is_none() {
                let target = target_list.unwrap();
                let item_cols = item_type_columns(&target.item_type);
                let item_fields = &entry.fields[data_fields_start..];

                let mut parsed_vals = Vec::new();
                for (c_idx, f) in item_fields.iter().enumerate() {
                    if let Some((nc, esc_col)) = f.invalid_escape {
                        line_error = Some((
                            format!("invalid escape sequence '\\{}'", nc),
                            f.start_line,
                            esc_col,
                        ));
                        break;
                    }

                    match validate_field_value(&f.trimmed_val, &item_cols[c_idx].col_type, f.is_raw, f.is_null) {
                        Ok(()) => match parse_field_to_value(&f.trimmed_val, &item_cols[c_idx].col_type, f.is_raw, f.is_null) {
                            Ok(pv) => parsed_vals.push(pv),
                            Err(e) => {
                                line_error = Some((e, f.start_line, f.start_col));
                                break;
                            }
                        },
                        Err(e) => {
                            line_error = Some((e, f.start_line, f.start_col));
                            break;
                        }
                    }
                }

                if line_error.is_none() {
                    let (parsed_item, node_schema) = match &target.item_type {
                        ListItemType::Scalar(_) => (parsed_vals[0].clone(), NodeSchema::Scalar),
                        ListItemType::Enum(_) => (parsed_vals[0].clone(), NodeSchema::Enum),
                        ListItemType::Record(r) => {
                            let mut cursor = 0;
                            let v = build_record_value(&r.fields, &parsed_vals, &mut cursor);
                            (v, NodeSchema::Record(r.clone()))
                        }
                    };

                    stack.push(StackNode {
                        depth,
                        line_num,
                        is_bad: false,
                        value: parsed_item,
                        schema: node_schema,
                        target_list_path: target.field_path,
                    });
                }
            }
        }

        if let Some((err_msg, err_line, err_col)) = line_error {
            bad_lines.push(BadLine {
                line: err_line,
                column: err_col,
                problem: err_msg,
                original: entry.text.clone(),
                dropped_as_child: false,
            });

            // Drop any children hanging under this bad line
            let parent_depth = depth;
            i += 1;
            while i < entries.len() {
                let next_depth = entries[i].depth;
                if next_depth > parent_depth {
                    bad_lines.push(BadLine {
                        line: entries[i].start_line_num,
                        column: 1,
                        problem: format!("dropped as child of line {}", line_num),
                        original: entries[i].text.clone(),
                        dropped_as_child: true,
                    });
                    i += 1;
                } else {
                    break;
                }
            }
            continue;
        }

        i += 1;
    }

    flush_stack(&mut stack, 0, &mut current_block);
    if let Some(b) = current_block {
        if !b.records.is_empty() {
            decoded.blocks.push(b);
        }
    }

    let has_usable_records = decoded.blocks.iter().any(|b| !b.records.is_empty());

    let (final_data, final_text) = match options.mode {
        DoctorMode::Strict => {
            if !bad_lines.is_empty() || !has_usable_records {
                (None, String::new())
            } else {
                let canonical = encode_with(schemas, &decoded).unwrap_or_default();
                (Some(decoded), canonical)
            }
        }
        DoctorMode::Salvage => {
            if !has_usable_records {
                (None, String::new())
            } else {
                let canonical = encode_with(schemas, &decoded).unwrap_or_default();
                (Some(decoded), canonical)
            }
        }
    };

    let hint = if options.retry_hint && !bad_lines.is_empty() {
        let hint_items: Vec<String> = bad_lines
            .iter()
            .filter(|b| !b.dropped_as_child)
            .enumerate()
            .map(|(i, b)| format!("{}. {}   ({})", i + 1, b.original, b.problem))
            .collect();
        if hint_items.is_empty() {
            None
        } else {
            Some(format!(
                "Some lines were invalid. Rewrite only these lines, one per line, in the same order:\n{}",
                hint_items.join("\n")
            ))
        }
    } else {
        None
    };

    repairs.sort_by_key(|r| (r.line, r.column));

    DoctorReport {
        data: final_data,
        text: final_text,
        repairs,
        bad_lines,
        hint,
    }
}

fn add_item_to_value_list(parent: &mut Value, path: &[usize], item: Value) {
    if let Value::Record(fields) = parent {
        if path.is_empty() {
            return;
        }
        let field_idx = path[0];
        if field_idx < fields.len() {
            let (_, field_val) = &mut fields[field_idx];
            if path.len() == 1 {
                if let Value::List(items) = field_val {
                    items.push(item);
                }
            } else {
                add_item_to_value_list(field_val, &path[1..], item);
            }
        }
    }
}

/// Slices out bad lines, replaces them with reply lines, and runs the doctor again.
pub fn splice(
    schemas: &str,
    original: &str,
    report: &DoctorReport,
    reply: &str,
    options: &DoctorOptions,
) -> Result<DoctorReport, StructError> {
    let target_bad_lines: Vec<&BadLine> = report
        .bad_lines
        .iter()
        .filter(|b| !b.dropped_as_child)
        .collect();

    let mut reply_lines: Vec<&str> = reply.lines().collect();
    if reply_lines.last() == Some(&"") {
        reply_lines.pop();
    }

    if reply_lines.len() != target_bad_lines.len() {
        return Err(StructError::new(
            format!(
                "reply has {} lines, expected {}",
                reply_lines.len(),
                target_bad_lines.len()
            ),
            1,
            1,
        ));
    }

    let mut orig_lines: Vec<String> = original.split('\n').map(|s| s.to_string()).collect();
    if orig_lines.last() == Some(&"".to_string()) {
        orig_lines.pop();
    }

    for (i, bad_line) in target_bad_lines.iter().enumerate() {
        let line_idx = bad_line.line.saturating_sub(1);
        let count = bad_line.original.lines().count().max(1);

        if line_idx < orig_lines.len() {
            orig_lines[line_idx] = reply_lines[i].to_string();
            for _ in 1..count {
                if line_idx + 1 < orig_lines.len() {
                    orig_lines.remove(line_idx + 1);
                }
            }
        }
    }

    let spliced_text = orig_lines.join("\n") + "\n";
    Ok(doctor(schemas, &spliced_text, options))
}
