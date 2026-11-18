//! Public data types for the Struct structured-output format (v0.2).
//!
//! Standard library only.

use std::collections::BTreeMap;
use std::fmt;

/// Compares two 64-bit floats by their exact IEEE-754 bit patterns,
/// treating NaN as equal to itself.
#[inline]
pub fn float_eq(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        true
    } else {
        a.to_bits() == b.to_bits()
    }
}

/// A decoded Struct value.
///
/// Covers null, 64-bit integers, 64-bit floats, booleans, strings,
/// enum labels, records, and lists.
#[derive(Debug, Clone)]
pub enum Value {
    /// Null (no value), written as `~`.
    Null,
    /// A signed 64-bit integer, such as `2024` or `-17`.
    Int(i64),
    /// A 64-bit floating point number, such as `3.14`, `-0.5`, `inf`, or `NaN`.
    Float(f64),
    /// A boolean value: `1` for true, `0` for false.
    Bool(bool),
    /// A UTF-8 text string.
    String(String),
    /// An enum label matching one of the declared options.
    Enum(String),
    /// A record of named fields in declaration order.
    Record(Vec<(String, Value)>),
    /// A list of items, decoded from child lines.
    List(Vec<Value>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => float_eq(*a, *b),
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Enum(a), Value::Enum(b)) => a == b,
            (Value::Record(a), Value::Record(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            _ => false,
        }
    }
}

impl Value {
    /// Returns true if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Returns the integer value if this is an `Int`.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns the float value if this is a `Float`.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Returns the boolean value if this is a `Bool`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the string slice if this is a `String`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Returns the enum label if this is an `Enum`.
    pub fn as_enum(&self) -> Option<&str> {
        match self {
            Value::Enum(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Returns the record fields if this is a `Record`.
    pub fn as_record(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Record(fields) => Some(fields.as_slice()),
            _ => None,
        }
    }

    /// Returns a specific field of this record by name.
    pub fn get_field(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Record(fields) => {
                for (k, v) in fields {
                    if k == name {
                        return Some(v);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Returns the items slice if this is a `List`.
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(items) => Some(items.as_slice()),
            _ => None,
        }
    }
}

/// A block of records belonging to a class, in file order.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The class name for this block.
    pub class_name: String,
    /// The records of this class, in file order.
    pub records: Vec<Value>,
}

/// A non-fatal warning generated during decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// Description of the warning.
    pub message: String,
    /// 1-based line number where the warning occurred.
    pub line: usize,
    /// 1-based column number where the warning occurred.
    pub column: usize,
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "warning at line {}, column {}: {}", self.line, self.column, self.message)
    }
}

/// A fatal error encountered during decoding or encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructError {
    /// Description of the error.
    pub message: String,
    /// 1-based line number where the error occurred.
    pub line: usize,
    /// 1-based column number where the error occurred.
    pub column: usize,
}

impl fmt::Display for StructError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error at line {}, column {}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for StructError {}

impl StructError {
    /// Constructs a new StructError with line and column information.
    pub fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            line,
            column,
        }
    }
}

/// The complete decoded representation of a Struct document.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedFile {
    /// The list of class blocks in file order.
    pub blocks: Vec<Block>,
    /// Any warnings generated during decoding.
    pub warnings: Vec<Warning>,
    /// The class definitions registered in this document.
    pub schemas: BTreeMap<String, crate::schema::ClassDef>,
}

impl DecodedFile {
    /// Creates an empty DecodedFile.
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            warnings: Vec::new(),
            schemas: BTreeMap::new(),
        }
    }

    /// Returns every record of class `name` from every block, in file order.
    pub fn lookup(&self, name: &str) -> Vec<&Value> {
        let mut out = Vec::new();
        for block in &self.blocks {
            if block.class_name == name {
                for rec in &block.records {
                    out.push(rec);
                }
            }
        }
        out
    }
}

impl Default for DecodedFile {
    fn default() -> Self {
        Self::new()
    }
}
