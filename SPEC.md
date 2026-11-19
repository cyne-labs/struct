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

## 9. The doctor

Models writing without a grammar make mistakes. The **doctor** takes their output plus the outside schemas (section 6), fixes only what has exactly one possible reading, and reports everything else. It never changes data silently, and it never guesses.

### Safe fixes (the only ones it makes)

Each fix is reported with its line, its column, what it found, and what it wrote instead.

1. **Code fences.** A line that, after trimming, starts with three backticks is removed.
2. **Echoed schema.** A line identical to one of the outside schema definitions, ignoring spaces, is removed.
3. **Leading and trailing chatter.** Lines before the first line that contains `|` or is a schema line are removed when they contain no `|`. So are lines after the last line that contains `|`, apart from the inner lines of a raw value. A line that fix 4 turns into a valid record is a record, not chatter: fix 4 is tried first.
4. **Missing final `|`.** A line that doesn't end with `|` gets `|` added, when the result has exactly the right number of columns and every value is valid. This includes a line with no `|` at all, such as `1706` under a one-column schema. On a record whose raw value spans several lines, the fix is reported on the record's last line, where the `|` was missing.
5. **Cut-off last record.** When the output's last line is an incomplete record (too few columns, or an unclosed raw value), that line is dropped, along with any child lines that would hang under it. Nothing is invented to complete it.
   - A raw value still open at the end of the output is a cut-off.
   - A cut that leaves the last value broken (for example, ending in a lone `\`) is a cut-off.
   - A child line under a parent with several lists is a cut-off only when it has one reading. If its first value could be either the list's field number or a value with the number left out (for example, `>2|` under a parent whose field 2 is a list), it's a bad line.

In the maintainer's words: *the box has a hole, the doctor patches it. Not my problem if the water in the box is dirty.*

**The doctor never touches values.** Fixing data is not its job: a value as written may be exactly what was meant, even in quotes. Only `~` is null, a `bool` is only `1` or `0`, and every value that fails its type check makes its line a bad line. That includes an empty field, `null`, `true`, `"2024"` in an int field, `+5`, `1,000`, an enum's wrong capitalisation (`Supported`), and a field-name prefix (`year: 2024`).

**Header rows are data.** The doctor never removes a line because it looks like column names: `word|meaning|` can be a real record. A header line is read like any other line: kept if its values are valid, reported as bad if not. Headers are prevented by the prompt (section 10) instead.

### Everything else is a bad line

Anything the fixes above can't resolve is a **bad line**: wrong column count, an unescaped `|` inside text, an enum typo, a depth jump, a missing index, an unclosed raw value in the middle of the file, and so on. The doctor reports each one with its line, its column, the problem, and the original line text. It never edits them.

### What the caller chooses

- **strict:** if there's any bad line, no data is returned, only the full report.
- **salvage:** every good record is returned. Bad lines are dropped and reported. A dropped line takes its child lines with it, and those are reported too.

### Retry hints

The doctor can also write a short **retry hint** for the model. It lists each bad line with its original text and the problem in a few words, and asks the model to rewrite only those lines, one per line, in the same order. The hint quotes each line's text instead of only giving a line number, because models count lines unreliably. Hints are on by default; the caller can turn them off, for example for small models, which may make changes nobody asked for.

The model's reply is spliced back: its lines replace the bad lines in order, and the result goes through the doctor again. A reply with a different number of lines than were asked for is rejected, with an error.

## 10. The prompt

Models are told the format by a prompt that the library builds from the schemas: `prompt(schemas)`. It contains only the rules the schemas need, so simple schemas pay fewer input tokens. The schema lines come first, followed by the parts below, in this order.

**Always:**
```
Reply with data only in Struct format: no prose, no code fences.
Never write a header row or column names. The first line is already data.
One record per line: values in schema order, separated by |, and every line ends with |.
~ means null. bool is 1 or 0. enum values must be one of the listed labels exactly.
Inside a value, write \| for | and \ for \.
```

**When any class has a `record{...}` field:**
```
A record{...} field adds its fields as extra columns, in order.
```

**When any class has a list:**
```
List items go on the lines below their parent, starting with one > per level: > for its items, >> for theirs.
```

**When any parent has two or more lists:**
```
When a parent has several lists, start each item with the list's field number and |, e.g. >2|item|. Fields count from 0 in schema order.
```

**When any class has a text field:**
```
For text with line breaks, or with spaces or tabs at its start or end, start the value with \r, then write it exactly as it is, keeping every space, tab, indent and line break, until the closing |.
```

**When there are two or more classes:**
```
Start each block with schema:name on its own line.
```

**Always last: one example line,** built from the first class, with neutral placeholder values. Text is `text`, int is `7`, float is `1.5`, bool is `1`, and an enum uses its first label. Exactly one scalar field, the last one, is written `~`, to show null. Record fields are flattened. Lists are left out of the example. It's written as `Example: ` followed by the canonical line.

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

### 2026-11-20, the doctor

- **Only `~` is null.** The doctor never reads `null`, `None`, `N/A` or similar words as null. In text they're text; elsewhere they're bad values. Likewise `true`/`false` in a bool field is a bad line, never converted to `1`/`0`.
- **The doctor fixes structure, never values.** Its only fixes are code fences, echoed schema, chatter, a missing final `|`, and a cut-off last record.
- **Header rows are never removed.** A line equal to the field names may be a real record. The prompt forbids headers instead. Every wrong value is reported. The user may have meant it, and fixing data isn't the doctor's job.
- **Safe fixes only.** Guessing was rejected: it burns tokens when a model has to work out why the doctor changed something it shouldn't have. Every case with more than one reading is reported as a bad line.
- **The caller chooses strict or salvage.**
- **Retry hints are on by default and can be turned off.** They might confuse small models, but API models handle them well.
- **The official prompt (section 10)** is built from the schema and contains only the rules that schema needs, plus one placeholder example line. Its exact wording was approved by the maintainer.
- **Tests come first from synthetic damage.** Real model outputs come after, kept apart and backed up, and never used as test fixtures.
