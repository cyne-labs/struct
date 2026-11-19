//! GBNF AST, parser, matcher and random walker for Struct test suite.
//!
//! Standard library only.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GbnfExpr {
    Literal(String),
    CharClass {
        ranges: Vec<(char, char)>,
        chars: HashSet<char>,
        negated: bool,
    },
    RuleRef(usize),
    Sequence(Vec<GbnfExpr>),
    Alt(Vec<GbnfExpr>),
    Optional(Box<GbnfExpr>),
    Star(Box<GbnfExpr>),
    Plus(Box<GbnfExpr>),
}

#[derive(Debug, Clone)]
pub struct Grammar {
    pub rule_names: HashMap<String, usize>,
    pub rules: Vec<GbnfExpr>,
    pub root_id: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RawExpr {
    Literal(String),
    CharClass {
        ranges: Vec<(char, char)>,
        chars: HashSet<char>,
        negated: bool,
    },
    RuleRef(String),
    Sequence(Vec<RawExpr>),
    Alt(Vec<RawExpr>),
    Optional(Box<RawExpr>),
    Star(Box<RawExpr>),
    Plus(Box<RawExpr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Literal(String),
    CharClass {
        ranges: Vec<(char, char)>,
        chars: HashSet<char>,
        negated: bool,
    },
    Ident(String),
    Pipe,
    Star,
    Plus,
    Question,
    LParen,
    RParen,
}

impl Grammar {
    /// Parses a GBNF grammar string.
    pub fn parse(input: &str) -> Result<Self, String> {
        let mut raw_rules: Vec<(String, RawExpr)> = Vec::new();
        let mut current_name = String::new();
        let mut current_expr_str = String::new();

        for line in input.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some(eq_idx) = trimmed.find("::=") {
                if !current_name.is_empty() {
                    let expr = parse_expression(&current_expr_str)?;
                    raw_rules.push((current_name, expr));
                    current_expr_str.clear();
                }
                current_name = trimmed[..eq_idx].trim().to_string();
                current_expr_str = trimmed[eq_idx + 3..].trim().to_string();
            } else {
                if !current_name.is_empty() {
                    current_expr_str.push(' ');
                    current_expr_str.push_str(trimmed);
                }
            }
        }

        if !current_name.is_empty() {
            let expr = parse_expression(&current_expr_str)?;
            raw_rules.push((current_name, expr));
        }

        let mut rule_names = HashMap::new();
        for (i, (name, _)) in raw_rules.iter().enumerate() {
            rule_names.insert(name.clone(), i);
        }

        let root_id = match rule_names.get("root") {
            Some(&id) => id,
            None => return Err("No root rule found in grammar".to_string()),
        };

        let mut rules = Vec::with_capacity(raw_rules.len());
        for (_, raw_expr) in raw_rules {
            rules.push(resolve_rule_refs(&raw_expr, &rule_names)?);
        }

        Ok(Self {
            rule_names,
            rules,
            root_id,
        })
    }

    /// Matches an input string against the `root` rule.
    pub fn matches(&self, input: &str) -> bool {
        let mut memo = HashMap::new();
        let ends = self.match_rule(self.root_id, input, 0, &mut memo);
        ends.contains(&input.len())
    }

    fn match_rule(
        &self,
        rule_id: usize,
        input: &str,
        pos: usize,
        memo: &mut HashMap<(usize, usize), Vec<usize>>,
    ) -> Vec<usize> {
        let key = (rule_id, pos);
        if let Some(cached) = memo.get(&key) {
            return cached.clone();
        }

        // Insert empty sentinel to avoid infinite recursion
        memo.insert(key, Vec::new());
        let results = self.match_expr(&self.rules[rule_id], input, pos, memo);
        memo.insert(key, results.clone());
        results
    }

    fn match_expr(
        &self,
        expr: &GbnfExpr,
        input: &str,
        pos: usize,
        memo: &mut HashMap<(usize, usize), Vec<usize>>,
    ) -> Vec<usize> {
        match expr {
            GbnfExpr::Literal(s) => {
                if input[pos..].starts_with(s) {
                    vec![pos + s.len()]
                } else {
                    Vec::new()
                }
            }
            GbnfExpr::CharClass { ranges, chars, negated } => {
                if pos < input.len() {
                    let c = input[pos..].chars().next().unwrap();
                    let in_set = chars.contains(&c) || ranges.iter().any(|(lo, hi)| c >= *lo && c <= *hi);
                    let matched = if *negated { !in_set } else { in_set };
                    if matched {
                        vec![pos + c.len_utf8()]
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            }
            GbnfExpr::RuleRef(rule_id) => self.match_rule(*rule_id, input, pos, memo),
            GbnfExpr::Alt(branches) => {
                let mut set = HashSet::new();
                for b in branches {
                    for p in self.match_expr(b, input, pos, memo) {
                        set.insert(p);
                    }
                }
                set.into_iter().collect()
            }
            GbnfExpr::Sequence(elems) => {
                let mut current = vec![pos];
                for elem in elems {
                    let mut next_set = HashSet::new();
                    for &p in &current {
                        for next_p in self.match_expr(elem, input, p, memo) {
                            next_set.insert(next_p);
                        }
                    }
                    if next_set.is_empty() {
                        return Vec::new();
                    }
                    current = next_set.into_iter().collect();
                }
                current
            }
            GbnfExpr::Optional(inner) => {
                let mut set = HashSet::new();
                set.insert(pos);
                for p in self.match_expr(inner, input, pos, memo) {
                    set.insert(p);
                }
                set.into_iter().collect()
            }
            GbnfExpr::Star(inner) => {
                let mut visited = HashSet::new();
                visited.insert(pos);
                let mut frontier = vec![pos];
                while let Some(p) = frontier.pop() {
                    for next_p in self.match_expr(inner, input, p, memo) {
                        if next_p > p && visited.insert(next_p) {
                            frontier.push(next_p);
                        }
                    }
                }
                visited.into_iter().collect()
            }
            GbnfExpr::Plus(inner) => {
                let mut visited = HashSet::new();
                let first = self.match_expr(inner, input, pos, memo);
                let mut frontier = Vec::new();
                for p in first {
                    if visited.insert(p) {
                        frontier.push(p);
                    }
                }
                while let Some(p) = frontier.pop() {
                    for next_p in self.match_expr(inner, input, p, memo) {
                        if next_p > p && visited.insert(next_p) {
                            frontier.push(next_p);
                        }
                    }
                }
                visited.into_iter().collect()
            }
        }
    }

    /// Random walker producing a sample conforming to the grammar.
    pub fn random_walk(&self, rng: &mut SimpleRng) -> String {
        let mut out = String::new();
        let mut state = WalkerState {
            record_count: 0,
            max_records: rng.gen_range(1, 3),
            depth: 0,
        };
        self.walk_rule(self.root_id, &mut out, rng, &mut state);
        out
    }

    fn walk_rule(&self, rule_id: usize, out: &mut String, rng: &mut SimpleRng, state: &mut WalkerState) {
        state.depth += 1;
        if state.depth < 100 {
            self.walk_expr(&self.rules[rule_id], out, rng, state);
        }
        state.depth -= 1;
    }

    fn walk_expr(&self, expr: &GbnfExpr, out: &mut String, rng: &mut SimpleRng, state: &mut WalkerState) {
        match expr {
            GbnfExpr::Literal(s) => {
                out.push_str(s);
            }
            GbnfExpr::CharClass { ranges, chars, negated } => {
                if *negated {
                    let safe_pool: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-"
                        .chars()
                        .filter(|c| !chars.contains(c) && !ranges.iter().any(|(lo, hi)| c >= lo && c <= hi))
                        .collect();
                    if !safe_pool.is_empty() {
                        let pick = rng.gen_range(0, safe_pool.len() - 1);
                        out.push(safe_pool[pick]);
                    } else {
                        out.push('x');
                    }
                } else {
                    let mut all_chars: Vec<char> = chars.iter().copied().collect();
                    for (lo, hi) in ranges {
                        for b in (*lo as u32)..=(*hi as u32) {
                            if let Some(c) = std::char::from_u32(b) {
                                all_chars.push(c);
                            }
                        }
                    }
                    if !all_chars.is_empty() {
                        let pick = rng.gen_range(0, all_chars.len() - 1);
                        out.push(all_chars[pick]);
                    } else {
                        out.push('0');
                    }
                }
            }
            GbnfExpr::RuleRef(rule_id) => {
                for (name, id) in &self.rule_names {
                    if *id == *rule_id && name.starts_with("record-") {
                        state.record_count += 1;
                        break;
                    }
                }
                self.walk_rule(*rule_id, out, rng, state);
            }
            GbnfExpr::Alt(branches) => {
                if !branches.is_empty() {
                    let pick = rng.gen_range(0, branches.len() - 1);
                    self.walk_expr(&branches[pick], out, rng, state);
                }
            }
            GbnfExpr::Sequence(elems) => {
                for elem in elems {
                    self.walk_expr(elem, out, rng, state);
                }
            }
            GbnfExpr::Optional(inner) => {
                if rng.gen_bool(0.5) {
                    self.walk_expr(inner, out, rng, state);
                }
            }
            GbnfExpr::Star(inner) => {
                let reps = if is_record_expr(inner, &self.rule_names) {
                    if state.record_count >= state.max_records { 0 } else { rng.gen_range(0, 1) }
                } else {
                    match rng.gen_range(0, 3) {
                        0 => 0,
                        1 => 1,
                        2 => 2,
                        _ => 0,
                    }
                };
                for _ in 0..reps {
                    self.walk_expr(inner, out, rng, state);
                }
            }
            GbnfExpr::Plus(inner) => {
                let reps = if is_record_expr(inner, &self.rule_names) {
                    if state.record_count >= state.max_records { 1 } else { rng.gen_range(1, 2) }
                } else {
                    match rng.gen_range(0, 2) {
                        0 => 1,
                        _ => 2,
                    }
                };
                for _ in 0..reps {
                    self.walk_expr(inner, out, rng, state);
                }
            }
        }
    }
}

fn resolve_rule_refs(raw: &RawExpr, names: &HashMap<String, usize>) -> Result<GbnfExpr, String> {
    match raw {
        RawExpr::Literal(s) => Ok(GbnfExpr::Literal(s.clone())),
        RawExpr::CharClass { ranges, chars, negated } => Ok(GbnfExpr::CharClass {
            ranges: ranges.clone(),
            chars: chars.clone(),
            negated: *negated,
        }),
        RawExpr::RuleRef(name) => match names.get(name) {
            Some(&id) => Ok(GbnfExpr::RuleRef(id)),
            None => Err(format!("Rule reference '{}' not found in grammar", name)),
        },
        RawExpr::Sequence(elems) => {
            let mut resolved = Vec::new();
            for e in elems {
                resolved.push(resolve_rule_refs(e, names)?);
            }
            Ok(GbnfExpr::Sequence(resolved))
        }
        RawExpr::Alt(branches) => {
            let mut resolved = Vec::new();
            for b in branches {
                resolved.push(resolve_rule_refs(b, names)?);
            }
            Ok(GbnfExpr::Alt(resolved))
        }
        RawExpr::Optional(inner) => Ok(GbnfExpr::Optional(Box::new(resolve_rule_refs(inner, names)?))),
        RawExpr::Star(inner) => Ok(GbnfExpr::Star(Box::new(resolve_rule_refs(inner, names)?))),
        RawExpr::Plus(inner) => Ok(GbnfExpr::Plus(Box::new(resolve_rule_refs(inner, names)?))),
    }
}

fn is_record_expr(expr: &GbnfExpr, rule_names: &HashMap<String, usize>) -> bool {
    match expr {
        GbnfExpr::RuleRef(id) => {
            for (name, rid) in rule_names {
                if rid == id && (name.starts_with("record-") || name.starts_with("block-") || name.starts_with("child-")) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

struct WalkerState {
    record_count: usize,
    max_records: usize,
    depth: usize,
}

pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    pub fn gen_range(&mut self, low: usize, high: usize) -> usize {
        if low >= high {
            return low;
        }
        let range = (high - low + 1) as u64;
        low + (self.next_u64() % range) as usize
    }

    pub fn gen_bool(&mut self, p_true: f64) -> bool {
        (self.next_u64() % 1000) < (p_true * 1000.0) as u64
    }
}

fn parse_expression(input: &str) -> Result<RawExpr, String> {
    let tokens = tokenize(input)?;
    let mut pos = 0;
    let expr = parse_alt(&tokens, &mut pos)?;
    if pos < tokens.len() {
        return Err(format!("Extra tokens after parsing: {:?}", &tokens[pos..]));
    }
    Ok(expr)
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
            i += 1;
            continue;
        }

        if b == b'|' {
            tokens.push(Token::Pipe);
            i += 1;
        } else if b == b'*' {
            tokens.push(Token::Star);
            i += 1;
        } else if b == b'+' {
            tokens.push(Token::Plus);
            i += 1;
        } else if b == b'?' {
            tokens.push(Token::Question);
            i += 1;
        } else if b == b'(' {
            tokens.push(Token::LParen);
            i += 1;
        } else if b == b')' {
            tokens.push(Token::RParen);
            i += 1;
        } else if b == b'"' {
            // String literal
            i += 1;
            let mut lit = String::new();
            while i < len {
                let cb = bytes[i];
                if cb == b'\\' {
                    if i + 1 < len {
                        let next_cb = bytes[i + 1];
                        match next_cb {
                            b'\\' => lit.push('\\'),
                            b'"' => lit.push('"'),
                            b'n' => lit.push('\n'),
                            b'r' => lit.push('\r'),
                            b't' => lit.push('\t'),
                            other => {
                                lit.push('\\');
                                lit.push(other as char);
                            }
                        }
                        i += 2;
                    } else {
                        return Err("Dangling escape in literal".to_string());
                    }
                } else if cb == b'"' {
                    i += 1;
                    break;
                } else {
                    lit.push(cb as char);
                    i += 1;
                }
            }
            tokens.push(Token::Literal(lit));
        } else if b == b'[' {
            // Char class
            i += 1;
            let mut negated = false;
            if i < len && bytes[i] == b'^' {
                negated = true;
                i += 1;
            }

            let mut ranges = Vec::new();
            let mut chars = HashSet::new();

            while i < len {
                if bytes[i] == b']' {
                    i += 1;
                    break;
                }

                let c1 = if bytes[i] == b'\\' {
                    i += 1;
                    if i >= len {
                        return Err("Dangling escape in char class".to_string());
                    }
                    let esc = match bytes[i] {
                        b'r' => '\r',
                        b'n' => '\n',
                        b't' => '\t',
                        b'\\' => '\\',
                        b']' => ']',
                        b'-' => '-',
                        other => other as char,
                    };
                    i += 1;
                    esc
                } else {
                    let ch = bytes[i] as char;
                    i += 1;
                    ch
                };

                // Check for range '-'
                if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] != b']' {
                    i += 1; // skip '-'
                    let c2 = if bytes[i] == b'\\' {
                        i += 1;
                        let esc = match bytes[i] {
                            b'r' => '\r',
                            b'n' => '\n',
                            b't' => '\t',
                            b'\\' => '\\',
                            b']' => ']',
                            b'-' => '-',
                            other => other as char,
                        };
                        i += 1;
                        esc
                    } else {
                        let ch = bytes[i] as char;
                        i += 1;
                        ch
                    };
                    ranges.push((c1, c2));
                } else {
                    chars.insert(c1);
                }
            }

            tokens.push(Token::CharClass { ranges, chars, negated });
        } else if (b >= b'a' && b <= b'z') || (b >= b'A' && b <= b'Z') || (b >= b'0' && b <= b'9') || b == b'_' || b == b'-' {
            let start = i;
            while i < len {
                let cb = bytes[i];
                if (cb >= b'a' && cb <= b'z') || (cb >= b'A' && cb <= b'Z') || (cb >= b'0' && cb <= b'9') || cb == b'_' || cb == b'-' {
                    i += 1;
                } else {
                    break;
                }
            }
            let ident = std::str::from_utf8(&bytes[start..i]).unwrap().to_string();
            tokens.push(Token::Ident(ident));
        } else {
            return Err(format!("Unexpected character '{}' in GBNF", b as char));
        }
    }

    Ok(tokens)
}

fn parse_alt(tokens: &[Token], pos: &mut usize) -> Result<RawExpr, String> {
    let mut alts = vec![parse_seq(tokens, pos)?];
    while *pos < tokens.len() && tokens[*pos] == Token::Pipe {
        *pos += 1;
        alts.push(parse_seq(tokens, pos)?);
    }
    if alts.len() == 1 {
        Ok(alts.pop().unwrap())
    } else {
        Ok(RawExpr::Alt(alts))
    }
}

fn parse_seq(tokens: &[Token], pos: &mut usize) -> Result<RawExpr, String> {
    let mut seq = Vec::new();
    while *pos < tokens.len() && tokens[*pos] != Token::Pipe && tokens[*pos] != Token::RParen {
        seq.push(parse_postfix(tokens, pos)?);
    }
    if seq.is_empty() {
        return Err("Empty sequence in GBNF".to_string());
    }
    if seq.len() == 1 {
        Ok(seq.pop().unwrap())
    } else {
        Ok(RawExpr::Sequence(seq))
    }
}

fn parse_postfix(tokens: &[Token], pos: &mut usize) -> Result<RawExpr, String> {
    let mut atom = parse_atom(tokens, pos)?;
    while *pos < tokens.len() {
        match tokens[*pos] {
            Token::Star => {
                *pos += 1;
                atom = RawExpr::Star(Box::new(atom));
            }
            Token::Plus => {
                *pos += 1;
                atom = RawExpr::Plus(Box::new(atom));
            }
            Token::Question => {
                *pos += 1;
                atom = RawExpr::Optional(Box::new(atom));
            }
            _ => break,
        }
    }
    Ok(atom)
}

fn parse_atom(tokens: &[Token], pos: &mut usize) -> Result<RawExpr, String> {
    if *pos >= tokens.len() {
        return Err("Unexpected end of tokens".to_string());
    }

    match &tokens[*pos] {
        Token::Literal(s) => {
            let lit = s.clone();
            *pos += 1;
            Ok(RawExpr::Literal(lit))
        }
        Token::CharClass { ranges, chars, negated } => {
            let expr = RawExpr::CharClass {
                ranges: ranges.clone(),
                chars: chars.clone(),
                negated: *negated,
            };
            *pos += 1;
            Ok(expr)
        }
        Token::Ident(name) => {
            let rule = name.clone();
            *pos += 1;
            Ok(RawExpr::RuleRef(rule))
        }
        Token::LParen => {
            *pos += 1;
            let expr = parse_alt(tokens, pos)?;
            if *pos >= tokens.len() || tokens[*pos] != Token::RParen {
                return Err("Missing ')' after group".to_string());
            }
            *pos += 1;
            Ok(expr)
        }
        other => Err(format!("Unexpected token in atom: {:?}", other)),
    }
}
