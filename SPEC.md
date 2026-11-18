# Struct specification, v0.2 (draft for review)

Struct is a structured-output format for language models. A file is made of **classes**: shapes declared once in `schema:` lines. Data lines carry only values, separated by `|`. There are no field names, no quotes and no braces in the data.

This draft records every decision made by the maintainer so far. Anything not yet decided is listed under **Open questions** at the end. Implementations must not guess at those.

---

## 1. Lines

- A file is read line by line. Windows line endings (CR LF) are treated exactly like plain LF.
- Blank lines, and lines containing only spaces or tabs, are ignored.
- Each non-blank line is exactly one of:
  - a **schema line**: it starts with `schema:`, and never contains `|`;
  - a **data line**: it always ends with an unescaped `|`.
- Leading and trailing spaces and tabs on every line are trimmed before reading.
- A data line that appears before any schema line is an error.

## 2. Classes and blocks

- `schema:name{field, field, ...}` **defines** a class called `name` and **starts a block**. Every data line after it belongs to that class, until the next schema line.
- `schema:name` with no braces **reuses** a class defined earlier in the file and starts a new block of it.
- Reusing a name that hasn't been defined yet is an error. Defining the same name twice is an error.
- Class names and field names are made of letters, digits and `_`, and don't start with a digit.

### Field types

A field is written `name` or `name:type`. With no type, it's text.

- `str`: text (the default).
- `int`: a whole number, optionally negative, such as `2024` or `-17`. Its range is Rust's `i64`.
- `float`: a decimal number, such as `3.14`, `-0.5`, `1e5`, `inf` or `NaN`. It accepts exactly what Rust's `f64` parser accepts.
- `bool`: `1` for true, `0` for false.
- `enum(a,b,c)`: exactly one of the listed labels. Labels follow the same rule as names: letters, digits and `_`, no spaces, and they don't start with a digit.
- `record{...}`: a fixed group of fields inside this one (section 4).
- `list[T]`: any number of items of type `T`, where `T` is a scalar type, an enum, or `record{...}` (section 5).

Types are **enforced**. A value that doesn't match its type is an error. For example, `twenty twenty-four` in an `int` field is an error. Writing a value correctly is the job of whoever produces the data.

## 3. Values

- The fields on a data line are separated by `|`, and the line ends with `|`.
- Spaces next to each `|` are trimmed. Spaces inside a value are kept.
- **`~` means null** (no value) in a field of any scalar or enum type. A value that is exactly `~` is null; `~` anywhere else, as in `~abc`, is plain text.
- **A record can be null.** A record whose columns are all `~` reads back as a null record. There's no separate marker, so "null record" and "record with every field null" are the same thing.
- **A list is never null.** A list field always holds a list, which may be empty. Struct has no way to write a list that doesn't exist, by design.
- **An empty field is always an error.** There's no empty string. Blank text is written `~`.

### Escapes

A backslash makes the next character literal.

| Escape | Means | Needed when |
|---|---|---|
| `\|` | a literal pipe | a value contains `|` |
| `\\` | a literal backslash | a value contains `\` |
| `\n` | a line break inside the value | a value spans lines (a real line break would end the line) |
| `\~` | a literal `~` | only when the whole value is exactly `~` and must not mean null |
| `\s` | a space that must not be trimmed | a value starts or ends with a space |
| `\>` | a literal `>` | a value at the start of a line begins with `>` |
| `\#` | a literal `#` | reserved for edge cases; harmless anywhere |

Any other backslash sequence is an error.

There's no raw-string mode. The maintainer considered it and rejected it for v0.2. Pipes in any field are written `\|`, at a cost of at most one extra token each.

## 4. Records: fixed groups, flattened into columns

A `record{...}` field adds its fields to the line as extra columns, in order, with no extra syntax.

```
schema:claim{text, source:record{id, year:int}, verdict}
Rope scaling hurts long ctx|2405.05254|2024|refuted|
```

A record can contain records, and these flatten the same way. A list inside a record doesn't take a column; its items become child lines (section 5).

## 5. Lists: child lines

### Depth

The items of a list are written as **child lines** below their parent. A child line starts with one `>` per level of depth: `>` is depth 1, `>>` is depth 2, and so on. Data lines with no `>` are depth 0, the records of the block.

Depth is unlimited.

**Attachment:** a line at depth *d* belongs to the most recent line above it at depth *d − 1*. Depth can rise by only one level at a time; a depth-0 line followed directly by a `>>` line is an error.

```
schema:claim{text, notes:list[record{text, subnotes:list[str]}]}
Rope scaling hurts long ctx|
>notes are from the abstract only|
>>abstract mentions a 32k limit|
>>appendix B clarifies|
>table 3 contradicts|
>>llama 7b tested only|
```

### Which list a child belongs to: indices

Fields are numbered from 0 in the order the schema lists them.

1. **One list:** if the parent contains exactly one list anywhere inside it, a child line has no index: `>text|`.
2. **Two or more lists:** a child line names its list by the field number, followed by `|`: `>2|text|`.
3. **Several lists inside one record field:** the index is dotted, giving the field number at each level, as in `>1.2|text|` for field 1, then field 2 inside it. If a record field contains only one list, its plain field number is enough (`>1|`).

```
schema:claim{text, verdict:record{label, evidence:list[str], counter:list[str]}, notes:list[str]}
Rope scaling hurts long ctx|refuted|
>1.1|table 3 contradicts the claim|
>1.2|abstract says otherwise|
>2|notes are from the abstract only|
```

The numbers count **schema fields**, not values on the line.

**Leniency:** an index written where none is needed (rule 1) is **accepted, with a warning**. It isn't an error.

**Empty lists** produce no child lines and read back as an empty list.

## 6. What decoding returns

- Decoding always returns a **list of blocks, in file order**. Each block has its class name and its list of records. A file with one block still returns a list with one block in it.
- Alongside the data, decoding returns a list of **warnings**, such as an unneeded index. Errors stop decoding and name the line and column.
- `lookup(name)` returns every record of class `name` from every block, in file order.

---

## Open questions

None. Every question raised in review has been decided.

## Resolved in review (2026-11-19)

- Null lists don't exist. A list is always a list, possibly empty.
- A null record is allowed, and it's the same as a record whose fields are all `~`.
- `\~` is needed only when the whole value is exactly `~`.
- Number limits are Rust's: `i64` for `int`, and `f64` parsing rules for `float`.
- Enum labels follow name rules, with no spaces.
- Nesting depth is unlimited.
- **No class references.** A field can't refer to another class by name (`list[note]`). Nested shapes are always written inline with `record{...}`. The maintainer rejected references as too many ways to go wrong for small models.
