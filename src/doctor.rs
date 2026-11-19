//! Doctor module for repairing and validating Struct documents (SPEC.md §9).
//!
//! Standard library only.

use crate::encode::encode_with;
use crate::schema::{
    parse_outside_schemas, ClassDef, FieldDef, FieldType, ListItemType, ListTarget,
    RecordDef, ScalarType,
};
use crate::types::{Block, DecodedFile, StructError, Value};
use std::collections::BTreeMap;

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

/// The 4 safe fixes defined in SPEC.md §9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SafeFix {
    /// 1. Markdown code fences (` ``` `) removed.
    CodeFences,
    /// 2. Line identical to an outside schema definition removed.
    EchoedSchema,
    /// 3. Leading and trailing chatter lines removed.
    Chatter,
    /// 4. Missing final pipe added.
    MissingFinalPipe,
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
                out.push(ColInfo { col_type: ct });
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
            vec![ColInfo { col_type: ct }]
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
    let post_depth_ws = first_line
        .text
        .chars()
        .skip(ws_count + depth)
        .take_while(|&c| c == ' ' || c == '\t')
        .count();
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

        while curr_char_idx < chars.len() && (chars[curr_char_idx] == ' ' || chars[curr_char_idx] == '\t') {
            curr_char_idx += 1;
        }

        if curr_char_idx >= chars.len() {
            break;
        }

        let field_start_line = cur_line.line_num;
        let field_start_col = curr_char_idx + 1;

        let is_raw = curr_char_idx + 1 < chars.len()
            && chars[curr_char_idx] == '\\'
            && chars[curr_char_idx + 1] == 'r';

        if is_raw {
            curr_char_idx += 2;
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

                if curr_line_idx + 1 < phys_lines.len() {
                    curr_line_idx += 1;
                    curr_char_idx = 0;
                    raw_val.push('\n');
                } else {
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

            let cur_chars: Vec<char> = phys_lines[curr_line_idx].text.chars().collect();
            while curr_char_idx < cur_chars.len() && (cur_chars[curr_char_idx] == ' ' || cur_chars[curr_char_idx] == '\t') {
                curr_char_idx += 1;
            }
            if curr_char_idx >= cur_chars.len() {
                break;
            }
        } else {
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
            } else {
                val_trimmed
                    .parse::<f64>()
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

// =========================================================================
// Step 1 & 2: Line Preparation, Fences, and Echoed Schemas
// =========================================================================

fn prepare_lines(text: &str) -> Vec<SourceLine> {
    let mut raw_lines: Vec<&str> = text.split('\n').collect();
    if raw_lines.last() == Some(&"") {
        raw_lines.pop();
    }
    raw_lines
        .into_iter()
        .enumerate()
        .map(|(idx, line_str)| SourceLine {
            line_num: idx + 1,
            text: line_str.strip_suffix('\r').unwrap_or(line_str).to_string(),
        })
        .collect()
}

fn extract_outside_schema_defs(schemas: &str) -> Vec<String> {
    let mut defs = Vec::new();
    for line in schemas.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("schema:") && trimmed.contains('{') && !trimmed.contains('|') {
            let non_ws: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
            defs.push(non_ws);
        }
    }
    defs
}

fn remove_fences_and_echoed_schemas(
    lines: &[SourceLine],
    outside_schema_defs: &[String],
) -> (Vec<DoctorEntry>, Vec<Repair>) {
    let mut entries = Vec::new();
    let mut repairs = Vec::new();
    let mut phys_idx = 0;

    while phys_idx < lines.len() {
        let cur_line = &lines[phys_idx];
        let trimmed = cur_line.text.trim();

        if trimmed.is_empty() {
            phys_idx += 1;
            continue;
        }

        // 1. Code fences
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

        // 2. Echoed schema
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

        // Schema block header (e.g. `schema:name` reuse)
        if trimmed.starts_with("schema:") && !trimmed.contains('|') {
            let name = trimmed["schema:".len()..].trim().to_string();
            let ws_count = cur_line.text.chars().take_while(|&c| c == ' ' || c == '\t').count();
            entries.push(DoctorEntry {
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

        let (entry, next_idx) = parse_data_entry(lines, phys_idx);
        entries.push(entry);
        phys_idx = next_idx;
    }

    (entries, repairs)
}

// =========================================================================
// Step 3: Chatter Removal (Fix 3)
// =========================================================================

/// Tests whether adding `|` to a candidate entry turns it into a valid record under active schemas.
fn is_valid_record_cand(
    entry: &DoctorEntry,
    active_class: Option<&ClassDef>,
    parent_schema: Option<&NodeSchema>,
) -> bool {
    let cand_content = format!("{}|", entry.text.trim_end());
    let temp_lines = vec![SourceLine {
        line_num: entry.start_line_num,
        text: cand_content,
    }];
    let (cand_entry, _) = parse_data_entry(&temp_lines, 0);

    if !cand_entry.ends_with_pipe || cand_entry.has_unclosed_raw {
        return false;
    }

    if entry.depth == 0 {
        let cd = match active_class {
            Some(c) => c,
            None => return false,
        };
        let mut cols = Vec::new();
        collect_columns(&cd.fields, &mut cols);

        if cand_entry.fields.len() != cols.len() {
            return false;
        }

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
                return false;
            }
        }
        true
    } else {
        let ps = match parent_schema {
            Some(p) => p,
            None => return false,
        };
        let lists = ps.find_lists();
        if lists.is_empty() {
            return false;
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

            if data_fields.len() != item_cols.len() {
                return false;
            }

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
                    return false;
                }
            }
            true
        } else {
            if cand_entry.fields.is_empty() {
                return false;
            }
            let index_str = cand_entry.fields[0].trimmed_val.trim();
            let matched = lists.iter().find(|t| {
                t.canonical_index == index_str
                    || t.accepted_indices.iter().any(|a| a == index_str)
            });

            if let Some(target) = matched {
                let item_cols = item_type_columns(&target.item_type);
                let data_fields = &cand_entry.fields[1..];
                if data_fields.len() != item_cols.len() {
                    return false;
                }
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
                        return false;
                    }
                }
                true
            } else {
                false
            }
        }
    }
}

fn node_schema_for_entry(
    entry: &DoctorEntry,
    active_class: Option<&ClassDef>,
    parent_schema: Option<&NodeSchema>,
) -> Option<NodeSchema> {
    if entry.depth == 0 {
        active_class.map(|cd| NodeSchema::Class(cd.clone()))
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
                t.canonical_index == index_str
                    || t.accepted_indices.iter().any(|a| a == index_str)
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
    }
}

fn remove_chatter(
    raw_entries: &mut Vec<DoctorEntry>,
    outside_classes: &BTreeMap<String, ClassDef>,
    single_outside_class: Option<&ClassDef>,
    repairs: &mut Vec<Repair>,
) {
    // Leading chatter: Lines before the first line that contains '|' or is a schema line
    let mut first_non_chatter = 0;
    while first_non_chatter < raw_entries.len() {
        let e = &raw_entries[first_non_chatter];
        if e.is_schema || e.text.contains('|') || e.has_unclosed_raw {
            break;
        }
        if is_valid_record_cand(e, single_outside_class, None) {
            break;
        }
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
        return;
    }

    // Trailing chatter: Lines after the last line that contains '|', apart from raw values
    let last_data_line_idx = raw_entries
        .iter()
        .rposition(|e| e.ends_with_pipe || e.text.contains('|') || e.is_schema || e.has_unclosed_raw);

    let mut active_class = single_outside_class.cloned();
    let mut schema_stack: Vec<NodeSchema> = Vec::new();

    let mut boundary = match last_data_line_idx {
        Some(idx) => idx,
        None => 0,
    };

    for i in 0..=boundary {
        let e = &raw_entries[i];
        if e.is_schema {
            schema_stack.clear();
            if let Some(ref name) = e.schema_name {
                active_class = outside_classes.get(name).cloned();
            }
        } else {
            let d = e.depth;
            schema_stack.truncate(d);
            let parent_schema = if d > 0 && d - 1 < schema_stack.len() {
                Some(&schema_stack[d - 1])
            } else {
                None
            };
            if let Some(ns) = node_schema_for_entry(e, active_class.as_ref(), parent_schema) {
                schema_stack.push(ns);
            }
        }
    }

    while boundary + 1 < raw_entries.len() {
        let e = &raw_entries[boundary + 1];
        let d = e.depth;
        schema_stack.truncate(d);
        let parent_schema = if d > 0 && d - 1 < schema_stack.len() {
            Some(&schema_stack[d - 1])
        } else {
            None
        };

        if is_valid_record_cand(e, active_class.as_ref(), parent_schema) {
            if let Some(ns) = node_schema_for_entry(e, active_class.as_ref(), parent_schema) {
                schema_stack.push(ns);
            }
            boundary += 1;
        } else {
            for rem in raw_entries.drain(boundary + 1..) {
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
}

// =========================================================================
// Step 4: Missing Final Pipe (Fix 4)
// =========================================================================

fn apply_missing_final_pipe(entries: &mut [DoctorEntry], repairs: &mut Vec<Repair>) {
    for entry in entries.iter_mut() {
        if entry.is_schema || (entry.ends_with_pipe && !entry.has_unclosed_raw) {
            continue;
        }

        let last_line = entry.text.split('\n').last().unwrap_or(&entry.text);
        let (new_text, col) = if entry.has_unclosed_raw {
            (
                format!("{}|", entry.text),
                last_line.chars().count() + 1,
            )
        } else {
            (
                format!("{}|", entry.text.trim_end()),
                last_line.trim_end().chars().count() + 1,
            )
        };

        let temp_lines: Vec<SourceLine> = new_text
            .split('\n')
            .enumerate()
            .map(|(li, s)| SourceLine {
                line_num: entry.start_line_num + li,
                text: s.to_string(),
            })
            .collect();
        let (repaired_entry, _) = parse_data_entry(&temp_lines, 0);

        repairs.push(Repair {
            line: entry.end_line_num,
            column: col,
            fix: SafeFix::MissingFinalPipe,
            original: entry.text.clone(),
            new_text: repaired_entry.text.clone(),
        });

        *entry = repaired_entry;
    }
}

// =========================================================================
// Step 5: Validation and Decoding
// =========================================================================

fn drop_child_lines(
    entries: &[DoctorEntry],
    idx: &mut usize,
    parent_line: usize,
    parent_depth: usize,
    bad_lines: &mut Vec<BadLine>,
) {
    while *idx < entries.len() {
        let next_depth = entries[*idx].depth;
        if next_depth > parent_depth {
            bad_lines.push(BadLine {
                line: entries[*idx].start_line_num,
                column: 1,
                problem: format!("dropped as child of line {}", parent_line),
                original: entries[*idx].text.clone(),
                dropped_as_child: true,
            });
            *idx += 1;
        } else {
            break;
        }
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

struct StackNode {
    value: Value,
    schema: NodeSchema,
    target_list_path: Vec<usize>,
}

fn flush_stack(
    stack: &mut Vec<StackNode>,
    target_len: usize,
    current_block: &mut Option<Block>,
) {
    while stack.len() > target_len {
        let node = stack.pop().unwrap();
        if stack.is_empty() {
            if let Some(b) = current_block.as_mut() {
                b.records.push(node.value);
            }
        } else {
            let parent_idx = stack.len() - 1;
            let parent_node = &mut stack[parent_idx];
            add_item_to_value_list(
                &mut parent_node.value,
                &node.target_list_path,
                node.value,
            );
        }
    }
}

fn validate_and_decode(
    entries: &[DoctorEntry],
    outside_classes: &BTreeMap<String, ClassDef>,
    single_outside_class: Option<&ClassDef>,
) -> (DecodedFile, Vec<BadLine>) {
    let mut current_block: Option<Block> = None;
    let mut current_class_def: Option<ClassDef> = single_outside_class.cloned();

    if let Some(ref cd) = current_class_def {
        current_block = Some(Block {
            class_name: cd.name.clone(),
            records: Vec::new(),
        });
    }

    let mut decoded = DecodedFile::new();
    for (k, v) in outside_classes {
        decoded.schemas.insert(k.clone(), v.clone());
    }

    let mut bad_lines: Vec<BadLine> = Vec::new();
    let mut stack: Vec<StackNode> = Vec::new();

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
            i += 1;
            drop_child_lines(entries, &mut i, line_num, depth, &mut bad_lines);
            continue;
        }

        // Parent check for child lines
        if depth > 0 {
            let parent = &stack[depth - 1];
            let lists = parent.schema.find_lists();
            if lists.is_empty() {
                bad_lines.push(BadLine {
                    line: line_num,
                    column: entry.marker_col,
                    problem: "parent record has no list fields to attach child line".to_string(),
                    original: entry.text.clone(),
                    dropped_as_child: false,
                });
                i += 1;
                drop_child_lines(entries, &mut i, line_num, depth, &mut bad_lines);
                continue;
            }
        }

        if !entry.ends_with_pipe {
            bad_lines.push(BadLine {
                line: line_num,
                column: entry.text.trim_end().chars().count() + 1,
                problem: "data line must always end with an unescaped '|'".to_string(),
                original: entry.text.clone(),
                dropped_as_child: false,
            });
            i += 1;
            drop_child_lines(entries, &mut i, line_num, depth, &mut bad_lines);
            continue;
        }

        flush_stack(&mut stack, depth, &mut current_block);

        let mut line_error: Option<(String, usize, usize)> = None;

        if depth == 0 {
            let mut cols = Vec::new();
            collect_columns(&class_def.fields, &mut cols);

            if entry.fields.len() != cols.len() {
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
                        value: record_val,
                        schema: NodeSchema::Class(class_def.clone()),
                        target_list_path: Vec::new(),
                    });
                }
            }
        } else {
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

                let actual_cols = if used_index {
                    entry.fields.len().saturating_sub(1)
                } else {
                    entry.fields.len()
                };

                if actual_cols != item_cols {
                    line_error = Some((
                        format!("expected {} columns, found {}", item_cols, actual_cols),
                        line_num,
                        entry.content_start_col,
                    ));
                    (None, 0)
                } else {
                    (Some(single_target.clone()), if used_index { 1 } else { 0 })
                }
            } else {
                let index_str = if entry.fields.is_empty() {
                    ""
                } else {
                    entry.fields[0].trimmed_val.trim()
                };
                let matched = list_targets.iter().find(|t| {
                    t.canonical_index == index_str
                        || t.accepted_indices.iter().any(|a| a == index_str)
                });

                match matched {
                    Some(t) => {
                        let item_cols = item_type_columns(&t.item_type).len();
                        let actual_cols = entry.fields.len().saturating_sub(1);
                        if actual_cols != item_cols {
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
                            if entry.fields.is_empty() {
                                entry.content_start_col
                            } else {
                                entry.fields[0].start_col
                            },
                        ));
                        (None, 0)
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
            i += 1;
            drop_child_lines(entries, &mut i, line_num, depth, &mut bad_lines);
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

    (decoded, bad_lines)
}

// =========================================================================
// Step 6: Final Report Assembly
// =========================================================================

fn build_report(
    schemas: &str,
    decoded: DecodedFile,
    mut repairs: Vec<Repair>,
    bad_lines: Vec<BadLine>,
    options: &DoctorOptions,
) -> DoctorReport {
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

// =========================================================================
// Main Doctor Function
// =========================================================================

/// Runs the doctor on `text` with `schemas` and `options`.
pub fn doctor(schemas: &str, text: &str, options: &DoctorOptions) -> DoctorReport {
    // Step 1: Parse outside schemas
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

    let outside_schema_defs = extract_outside_schema_defs(schemas);
    let single_outside_class = if outside_classes.len() == 1 {
        outside_classes.values().next().cloned()
    } else {
        None
    };

    // Step 2: Prepare lines, remove code fences and echoed schemas (Fixes 1 & 2)
    let phys_lines = prepare_lines(text);
    let (mut entries, mut repairs) =
        remove_fences_and_echoed_schemas(&phys_lines, &outside_schema_defs);

    // Step 3: Remove leading and trailing chatter (Fix 3)
    remove_chatter(
        &mut entries,
        &outside_classes,
        single_outside_class.as_ref(),
        &mut repairs,
    );

    if entries.is_empty() {
        repairs.sort_by_key(|r| (r.line, r.column));
        return DoctorReport {
            data: None,
            text: String::new(),
            repairs,
            bad_lines: Vec::new(),
            hint: None,
        };
    }

    // Step 4: Apply missing final pipe unconditionally (Fix 4)
    apply_missing_final_pipe(&mut entries, &mut repairs);

    // Step 5: Validate entries against schemas and decode blocks
    let (decoded, bad_lines) =
        validate_and_decode(&entries, &outside_classes, single_outside_class.as_ref());

    // Step 6: Construct final report according to options
    build_report(schemas, decoded, repairs, bad_lines, options)
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

    let mut orig_lines: Vec<String> = original
        .split('\n')
        .map(|s| s.strip_suffix('\r').unwrap_or(s).to_string())
        .collect();
    if orig_lines.last() == Some(&"".to_string()) {
        orig_lines.pop();
    }

    // Iterate in reverse order so line removals do not shift lower line indices
    for (i, bad_line) in target_bad_lines.iter().enumerate().rev() {
        let count = bad_line.original.split('\n').count().max(1);
        let first_orig_line = bad_line.original.lines().next().unwrap_or(&bad_line.original);

        let mut start_idx = bad_line.line.saturating_sub(1);
        if count > 1 {
            let min_idx = start_idx.saturating_sub(count - 1);
            for test_idx in (min_idx..=start_idx).rev() {
                if test_idx < orig_lines.len() && orig_lines[test_idx] == first_orig_line {
                    start_idx = test_idx;
                    break;
                }
            }
        }

        if start_idx < orig_lines.len() {
            orig_lines[start_idx] = reply_lines[i].to_string();
            for _ in 1..count {
                if start_idx + 1 < orig_lines.len() {
                    orig_lines.remove(start_idx + 1);
                }
            }
        }
    }

    let spliced_text = orig_lines.join("\n") + "\n";
    Ok(doctor(schemas, &spliced_text, options))
}
