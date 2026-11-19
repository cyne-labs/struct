# Struct specification, v0.3

Struct is a structured-output format for language models. A file is made of **classes**: shapes declared once in `schema:` lines. Data lines carry only values, separated by `|`. There are no field names, no quotes and no braces in the data.

This spec records every decision made by the maintainer so far. Anything not yet decided is listed under **Open questions** at the end. Implementations must not guess at those.

---

## 1. Lines

- Windows line endings (CR LF) are treated exactly like plain LF, everywhere, including inside raw values (section 3).
- Blank lines, and lines containing only spaces or tabs, are ignored, except inside a raw value.
- Each non-blank line is one of:
  - a **schema line**: it starts with `schema:` and contains no `|` anywhere;
  - a **data line**: every other line. A data line ends with an unescaped `|`.
- A line that starts with `schema:` but contains a `|` is a data line. For example, `schema:foo|1.5|` is a data line whose first value is the text `schema:foo`. Schema lines can never contain `|`, so the two can't be confused.
- Leading and trailing spaces and tabs on every line are trimmed before reading, except inside a raw value.
- A data line that appears before any schema line is an error, unless the schema is supplied from outside the file (section 6).
- A **raw value** (section 3) may contain real line breaks. A data line then continues across physical lines until its final unescaped `|`. Physical lines inside a raw value are text: one starting with `>` or `schema:` is not a child line or a schema line.

## 2. Classes and blocks

- `schema:name{field, field, ...}` **defines** a class called `name` and **starts a block**. Every data line after it belongs to that class, until the next schema line.
- `schema:name` with no braces **reuses** a class defined earlier (in the file, or supplied from outside it) and starts a new block of it.
- Reusing a name that hasn't been defined yet is an error. Defining the same name twice is an error.
- Class names and field names are made of letters, digits and `_`, and don't start with a digit.

### Field types

A field is written `name` or `name:type`. With no type, it's text.

- `str`: text (the default).
- `int`: a whole number, optionally negative, such as `2024` or `-17`. Its range is Rust's `i64`. A leading `+` is an error.
- `float`: a decimal number, such as `3.14`, `-0.5`, `1e5`, `+5`, `inf` or `NaN`. It accepts exactly what Rust's `f64` parser accepts.
- `bool`: `1` for true, `0` for false.
- `enum(a,b,c)`: exactly one of the listed labels. Labels follow the same rule as names: letters, digits and `_`, no spaces, and they don't start with a digit.
- `record{...}`: a fixed group of fields inside this one (section 4).
- `list[T]`: any number of items of type `T`, where `T` is a scalar type, an enum, or `record{...}` (section 5).

A type name not in this list (such as `date`) is an error. Using the same field name twice inside one set of braces is an error.

Types are **enforced**. A value that doesn't match its type is an error. For example, `twenty twenty-four` in an `int` field is an error. Writing a value correctly is the job of whoever produces the data.

## 3. Values

- The fields on a data line are separated by `|`, and the line ends with `|`.
- A data line has exactly one value per column of its class or list item (records flattened, lists not counted). Too few or too many values is an error.
- Spaces and tabs next to each `|` are trimmed. Spaces and tabs inside a value are kept.
- **`~` means null** (no value) in a field of any scalar or enum type. A value that is exactly `~` is null; `~` anywhere else, as in `~abc`, is plain text.
- **A record can be null.** A record whose columns are all `~` reads back as a null record. There's no separate marker, so "null record" and "record with every field null" are the same thing.
- **A list is never null.** A list field always holds a list, which may be empty, and whose items may be `~`. Struct has no way to write a list that doesn't exist, by design. Think of a list as a box: once declared, the box always exists, even when it's empty.
- **A record that contains a list is never null.** If a record has a list anywhere inside it, the list must exist, so the record must too. When all its columns are `~`, it reads back as a record with null values and its lists (empty when it has no child lines). Handing an encoder a null in place of a list, or a null for a record that contains a list, is an error.
- **An empty field is always an error.** There's no empty string. Blank text is written `~`.

### Escapes

A backslash makes the next character literal.

| Escape | Means | Needed when |
|---|---|---|
| `\|` | a literal pipe | a value contains `|` |
| `\\` | a literal backslash | a value contains `\` |
| `\~` | a literal `~` | only when the whole value is exactly `~` and must not mean null |
| `\>` | a literal `>` | the first value on a line begins with `>` |
| `\#` | a literal `#` | reserved for edge cases; harmless anywhere |
| `\r` | start of a raw value | only as the first thing in a value; see below |

Any other backslash sequence is an error. There are no escapes for line breaks, tabs or spaces: whitespace that must be kept exactly is written in a raw value.

### Raw values

A value that starts with `\r` is **raw**. Everything after `\r`, up to the next unescaped `|`, is kept exactly as written:

- real line breaks, tabs, and leading and trailing spaces are all kept. Nothing inside is trimmed;
- `|` and `\` are still written `\|` and `\\`, so `|` always ends a value. Inside a raw value these are the only escapes; any other backslash sequence is an error;
- the spaces next to `|` before the `\r` are trimmed as usual. Whitespace after `\r` is part of the value;
- CR LF inside a raw value becomes LF, as everywhere else;
- a raw value with nothing after `\r` is an empty field, which is an error.

A raw value is always text: `\r` is allowed only in `str` fields and `list[str]` items. In any other type it's an error. `\r~` is the text `~`, not null.

```
schema:fix{file, patch}
main.rs|\rfn main() {
    println!("hi");
}|
```

The value of `patch` is the three lines of code, with the indentation kept.

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

A child line whose parent has no list is an error. When an index is needed (rules 2 and 3), leaving it out is an error, and an index that doesn't name one of the parent's lists is an error.

**Leniency:** an index written where none is needed (rule 1) is **accepted, with a warning**. It isn't an error.

**Empty lists** produce no child lines and read back as an empty list.

## 6. Decoding, and schemas supplied from outside the file

- Decoding always returns a **list of blocks, in file order**. Each block has its class name and its list of records. A file with one block still returns a list with one block in it.
- Alongside the data, decoding returns a list of **warnings**, such as an unneeded index. Errors stop decoding and name the line and column.
- `lookup(name)` returns every record of class `name` from every block, in file order.

### Outside schemas

The caller usually already knows the schema, because it's what they asked the model for. The schema can then go in the prompt only, and the model writes just data. This saves output tokens.

- A decoder can be given **one or more schema lines from outside the file**. They define classes exactly as if they appeared before the file's first line.
- **One outside class:** data lines at the start of the file, before any schema line, form a block of that class.
- **Several outside classes:** the file must start with a schema line (normally `schema:name` to pick a class). Data before it is an error.
- Inside the file, `schema:name` reuses an outside class. Defining an outside class's name again is an error, as for any class defined twice.
- An encoder given outside schemas writes no definitions for them. With one outside class, its first block has no schema line, and any later block starts with `schema:name`. With several, every block starts with `schema:name`.

## 7. Canonical form

Decoders are lenient: they accept spaces around `|`, blank lines and unneeded indices. Encoders write exactly one form, the **canonical form**, and a grammar (section 8) allows only that form:

- lines end with LF, and the file ends with a line break. There are no blank lines;
- no spaces next to `|`, and no spaces or tabs at the start or end of a line;
- schema lines are written `schema:name{field, field:type}`, with `, ` between fields;
- indices only where needed (section 5), never the lenient extra one;
- escapes only where needed:
  - `\|` and `\\` always;
  - `\~` only for a whole value of `~`;
  - `\>` only when the first value on a line begins with `>`;
  - `\#` never;
- `\r` only when a text value contains a line break, or starts or ends with a space or tab. Inside it, only `|` and `\` are escaped;
- `int` is plain digits with no leading zeros, and `-` for negatives;
- `float` uses the shortest digits that read back to exactly the same value:
  - **plain form** when the number is 0, or its size is at least `0.0001` and below `10000`: `3.14`, `2024.5`, `0.0012`, `-0.5`. A whole number has no `.0`;
  - **exponent form** otherwise: `1.2345e4` for 12345, `1e5` for 100000, `1e-5` for 0.00001;
  - the special values are written `NaN`, `inf` and `-inf`, and negative zero is `-0`.
- `bool` is `1` or `0`; null is `~`.

## 8. Grammars (GBNF)

For local models run through llama.cpp, an implementation can turn schemas into a **GBNF grammar**. llama.cpp then lets the model produce only text the grammar allows.

- The grammar is built from **outside schemas** (section 6): the model writes data only, with `schema:name` lines to switch class when there are several.
- It allows **only the canonical form** (section 7). Every allowed choice is one a small model can fumble and spend tokens on, so there are none to spare.
- **Completeness:** every canonical encoding of data for those schemas is accepted by the grammar.
- **Soundness:** every text the grammar accepts decodes without error against the same outside schemas. One exception: the grammar limits an `int` to 19 digits, so a 19-digit number above `i64`'s maximum passes the grammar and is caught by the decoder.
- The model can stop after any complete record, at depth 0 or as a child.
- **Two forms:**
  - **full:** the grammar constrains the whole output;
  - **triggered:** the model writes freely, for example while reasoning, until a trigger text appears (`</think>` by default, and configurable). From there on, the output must follow the grammar. This uses llama.cpp's lazy-grammar triggers.
- **GBNF only for now.** Other engines (vLLM, SGLang, Outlines, llguidance) come later, from the same internal description of the schemas.

---

## Open questions

None. Every question raised in review has been decided.

## Resolved in review

### 2026-11-19, v0.2

- Null lists don't exist. A list is always a list, possibly empty.
- A null record is allowed, and it's the same as a record whose fields are all `~`.
- `\~` is needed only when the whole value is exactly `~`.
- Number limits are Rust's: `i64` for `int`, and `f64` parsing rules for `float`.
- Enum labels follow name rules, with no spaces.
- Nesting depth is unlimited.
- **No class references.** A field can't refer to another class by name (`list[note]`). Nested shapes are always written inline with `record{...}`. The maintainer rejected references as too many ways to go wrong for small models.
- After R1 review:
  - wrong column counts, children under a parent with no list, missing or unknown indices, unknown type names and duplicate field names are all errors. Everyone assumed these; now they're written down;
  - a leading `+` on an int is an error. Floats keep Rust's rules, so `+5` is a valid float;
  - a record containing a list is never null: the list is a box that must exist. A null list, or a null list-holding record, given to the encoder is an error.

### 2026-11-19, v0.3

- **Raw values (`\r`)** replace `\n` and `\s`, and `\t` is never added. One escape covers all whitespace, so a model has one rule to remember, and code can be written with real line breaks. The maintainer accepted the cost: a raw value missing its closing `|` swallows the rest of the file. The doctor (R3) repairs that, and the grammar (R2) prevents it.
- **Line type is decided by `|`.** A line starting with `schema:` that contains a `|` is data. This fixes text values beginning with `schema:`, which v0.2 couldn't read back.
- **Floats use exponent form outside `0.0001` to `10000`.** That's past 4 digits, where tokenizer vocabularies stop merging digits. This fixes v0.2's encoder, which wrote `1e300` as 301 digits.
- **Schemas can be supplied from outside the file**, so models write only data.
- **Grammars are GBNF, canonical-only, in full and triggered forms.** Other engines come later.
