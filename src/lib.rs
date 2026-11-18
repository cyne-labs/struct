//! # structl: Core Library for the Struct Structured-Output Format (v0.2)
//!
//! Struct is a token-minimal structured-output format designed for language models.
//! A file is composed of schema-declared classes, where data lines carry only values
//! separated by `|` delimiters with no repeated field names or quotation marks.
//!
//! This library provides the core standard-library implementation of Struct v0.2:
//! - **Schema reading:** schema declaration, class definition, reuse, and field typing.
//! - **Decoding:** text to structured in-memory values with precise line/column errors.
//! - **Encoding:** data to canonical Struct text.
//! - **Lookup:** querying records by class name across blocks.

pub mod types;
pub mod schema;
pub mod decode;
pub mod encode;

// Re-export primary types and functions for a clean public interface
pub use types::{Block, DecodedFile, StructError, Value, Warning, float_eq};
pub use schema::{ClassDef, FieldDef, FieldType, ListItemType, RecordDef, ScalarType};
pub use decode::decode;
pub use encode::encode;

/// Looks up and returns all records of class `name` from all blocks in `file`, in file order.
pub fn lookup<'a>(file: &'a DecodedFile, name: &str) -> Vec<&'a Value> {
    file.lookup(name)
}
