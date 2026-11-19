# Struct

Struct is a structured-output format for language models: declare the shape once, then write only values.

The Rust crate is `structl`, files use the `.stc` extension, and everything is licensed under Apache 2.0.

## Example

The same two claims, as minified JSON and as Struct. The data comes from our `nested_variable` benchmark fixture, trimmed to two records.

Minified JSON:

```json
[{"text":"Rope scaling hurts long ctx","verdict":"refuted","source":{"id":"2405.05254","year":2024},"notes":[{"text":"notes are from abstract only","subnotes":["abstract mentions 32k limit","appendix B clarifies"]},{"text":"table 3 contradicts","subnotes":["llama 7b tested only"]}]},{"text":"KV cache quant loses 2x acc","verdict":"refuted","source":{"id":"2406.12345","year":2024},"notes":[{"text":"outlier channels preserved","subnotes":["per-head scales help","perplexity intact"]}]}]
```

Struct:

```
schema:claim{text, verdict:enum(supported,refuted,unknown), source:record{id, year:int}, notes:list[record{text, subnotes:list[str]}]}
Rope scaling hurts long ctx|refuted|2405.05254|2024|
>notes are from abstract only|
>>abstract mentions 32k limit|
>>appendix B clarifies|
>table 3 contradicts|
>>llama 7b tested only|
KV cache quant loses 2x acc|refuted|2406.12345|2024|
>outlier channels preserved|
>>per-head scales help|
>>perplexity intact|
```

What is different from JSON, and from other compact formats:

- The schema is declared once, in a `schema:` line. Data lines carry only values, and every line ends with `|`. There are no field names, quotes or braces in the data.
- Lists of any depth are child lines. `>` is depth 1, `>>` is depth 2, and so on.- `~` is null.
- A value that starts with `\r` is raw: real line breaks, tabs and indentation are kept as written, up to the next unescaped `|`. This is for multi-line text and code.
- One file can hold several classes. Each `schema:` line starts a new block, and `schema:name` with no braces reuses a class defined earlier.

Null, raw values and several classes together:

```
schema:fix{file, patch, note}
schema:claim{text, verdict:enum(supported,refuted,unknown)}
main.rs|\rfn main() {
    println!("hi");
}|~|
schema:claim
Rope scaling hurts long ctx|refuted|
```

## Token benchmark (R1)

We counted tokens for three data shapes, in minified JSON, TOON and Struct, with four tokenizers. Counts are exact, taken in-process through llama.cpp's C API with `vocab_only`. The model files were:

- Qwen: `Qwen3.5-4B-Q4_K_M.gguf`
- Llama: `Llama-3.2-3B-Instruct-Q4_K_M.gguf`
- Gemma: `gemma-3-4b-it-Q4_K_M.gguf`
- LFM: `LFM2.5-8B-A1B-Q4_K_M.gguf`

The Struct rows are Struct v0.2 with the typed schema line included, so every Struct number pays for its own schema. Lower is better. Cells are token counts. The last two columns compare Struct with TOON and with minified JSON.

Source: `research/r1-remeasure.md` and `bench/r1/results_r1_fixtures.csv`.

### Flat table, 20 rows x 5 columns

| Model | Minified JSON | TOON | Struct | Struct vs TOON | Struct vs JSON |
|---|---|---|---|---|---|
| Qwen3.5-4B | 759 | 564 | 581 | 3.0% worse than TOON | 23.5% better than JSON |
| Llama-3.2-3B | 617 | 421 | 418 | 0.7% better than TOON | 32.3% better than JSON |
| gemma-3-4b | 756 | 570 | 583 | 2.3% worse than TOON | 22.9% better than JSON |
| LFM2.5-8B-A1B | 661 | 428 | 449 | 4.9% worse than TOON | 32.1% better than JSON |
| **Mean** | 698.2 | 495.8 | 507.8 | 2.4% worse than TOON | 27.3% better than JSON |

### Nested, fixed shape, 10 records with a 2-field sub-record

| Model | Minified JSON | TOON | Struct | Struct vs TOON | Struct vs JSON |
|---|---|---|---|---|---|
| Qwen3.5-4B | 397 | 279 | 305 | 9.3% worse than TOON | 23.2% better than JSON |
| Llama-3.2-3B | 327 | 208 | 224 | 7.7% worse than TOON | 31.5% better than JSON |
| gemma-3-4b | 396 | 292 | 307 | 5.1% worse than TOON | 22.5% better than JSON |
| LFM2.5-8B-A1B | 350 | 215 | 242 | 12.6% worse than TOON | 30.9% better than JSON |
| **Mean** | 367.5 | 248.5 | 269.5 | 8.5% worse than TOON | 26.7% better than JSON |

### Nested, variable shape, 4 records with 2-level child lists

| Model | Minified JSON | TOON | Struct | Struct vs TOON | Struct vs JSON |
|---|---|---|---|---|---|
| Qwen3.5-4B | 337 | 410 | 309 | 24.6% better than TOON | 8.3% better than JSON |
| Llama-3.2-3B | 304 | 367 | 252 | 31.3% better than TOON | 17.1% better than JSON |
| gemma-3-4b | 323 | 404 | 306 | 24.3% better than TOON | 5.3% better than JSON |
| LFM2.5-8B-A1B | 304 | 370 | 264 | 28.6% better than TOON | 13.2% better than JSON |
| **Mean** | 317.0 | 387.8 | 282.8 | 27.1% better than TOON | 10.8% better than JSON |

### What the numbers say

We lose to TOON on the first two shapes: +2.4% on flat tables and +8.5% on fixed nesting. TOON's tabular header is very compact, and when every row has the same shape it is hard to beat. Struct's schema line costs more than TOON's header, and on short files that cost shows.

Struct wins where nesting varies. TOON can only stay tabular while every row has the same shape. Once child list lengths differ, it falls back to repeating keys (`- text:`, `subnotes[2]:`) on every record. Struct's positional child lines carry no keys at any depth, which is where the -27.1% against TOON comes from.

The benchmark ran on spec v0.2. The v0.3 changes are raw values and float formatting. We checked the fixtures: none of the three schemas has a `float` field, and no `.stc` fixture contains a backslash, so neither change touches these files. The v0.2 file format and the v0.3 one are identical for them. Struct values-only counts (no schema line) and untyped-schema counts are in the CSV.

## Small per-token facts

From the same report, measured on the same four tokenizers.

- Trailing `|` plus newline: Llama and LFM have a single `|\n` token, so the closing pipe costs nothing there. On Qwen and Gemma it is two tokens, so it costs one extra token per data line.
- Child markers: `>`, `>>`, `>>>` and `>>>>` are each one token on all four tokenizers. Depths 5 and 6 cost two tokens, depths 7 and 8 cost one or two depending on the tokenizer.
- `\|`: alone it is one token on Gemma and LFM and two on Qwen and Llama. After a space, which is the usual place in prose, all four tokenizers merge the space and the escape into one token.
- Null: `~` alone is one token, but `|~|` is three, two more than an empty `||`. We pay this for clarity.

## Syntax at a glance

`SPEC.md` is the authority. This is the short version of v0.3.

**Lines.** A line starting with `schema:` that contains no `|` is a schema line. Every other non-blank line is a data line and ends with an unescaped `|`. CR LF is the same as LF. Spaces and tabs next to `|` are trimmed.

**Schema.** `schema:name{field, field:type}` defines a class and starts a block. `schema:name` reuses one. Names are letters, digits and `_`, and don't start with a digit.

**Types.**

| Type | Meaning |
|---|---|
| `str` | text, the default when no type is given |
| `int` | whole number in the `i64` range; a leading `+` is an error |
| `float` | anything Rust's `f64` parser accepts, including `inf` and `NaN` |
| `bool` | `1` or `0` |
| `enum(a,b,c)` | exactly one of the labels |
| `record{...}` | a fixed group, flattened into extra columns |
| `list[T]` | any number of items, written as child lines |

Types are enforced. A value that doesn't match its type is an error. A type name not listed here is an error.

**Null.** A value that is exactly `~` is null, in any scalar or enum field. A record whose columns are all `~` is a null record. A list is never null and may be empty. A record that contains a list is never null. An empty field is always an error; blank text is `~`.

**Escapes.** A backslash makes the next character literal.

| Escape | Meaning |
|---|---|
| `\\|` | literal pipe |
| `\\` | literal backslash |
| `\~` | literal `~`, only when the whole value is `~` |
| `\>` | literal `>`, when the first value on a line begins with `>` |
| `\#` | literal `#`, harmless anywhere |
| `\r` | start of a raw value, only as the first thing in a value |

Any other backslash sequence is an error. There are no escapes for line breaks, tabs or spaces.

**Raw values.** A value starting with `\r` is raw, and only allowed in `str` fields and `list[str]` items. Everything up to the next unescaped `|` is kept exactly, including real line breaks, tabs and edge spaces. Only `\|` and `\\` are escapes inside it. A raw value that is missing its closing `|` swallows the rest of the file.

**Depth and indices.** A child line starts with one `>` per level. A line at depth d attaches to the nearest line above it at depth d-1, and depth can rise by only one level at a time. Fields are numbered from 0 in schema order.

- One list under the parent: no index, `>text|`.
- Two or more lists: the child line names its list, `>2|text|`.
- Lists nested in a record field: a dotted index, `>1.2|text|`.

**Outside schemas.** The caller usually knows the schema, because it is what they asked the model for. The schema can go in the prompt only, and the model writes just data. With one outside class, data lines at the top of the file form a block of that class. With several, every block starts with `schema:name`.

**Canonical form.** Decoders are lenient. Encoders write one form: LF endings, no blank lines, no spaces next to `|`, escapes only where needed, and the shortest float digits that read back exactly, in exponent form outside `0.0001` to `10000`.

## Rust

`structl` uses only the standard library.

```rust
use structl::{decode_with, encode_with, lookup};

fn main() -> Result<(), structl::StructError> {
    // The schema lives in the prompt; the model writes only data.
    let schemas = "schema:claim{text, verdict:enum(supported,refuted,unknown), year:int}";
    let model_output = "\
Rope scaling hurts long ctx|refuted|2024|
LoRA matches full finetune|supported|2021|
";

    // decode_with(schemas: &str, input: &str) -> Result<DecodedFile, StructError>
    let file = decode_with(schemas, model_output)?;
    let claims = lookup(&file, "claim"); // Vec<&Value>, in file order
    println!("{} claims", claims.len());

    // encode_with(schemas: &str, file: &DecodedFile) -> Result<String, StructError>
    let text = encode_with(schemas, &file)?;
    print!("{text}"); // canonical Struct, with no schema line
    Ok(())
}
```

`decode(input)` and `encode(&file)` are the same calls with the schemas defined inside the file. A decode returns a list of blocks in file order, plus warnings. Errors stop decoding and name the line and column.

**Experimental:** `gbnf(schemas)` returns a GBNF grammar for llama.cpp that allows only canonical Struct for the given schemas. `gbnf_triggered(schemas, trigger)` lets the model write freely, for example while reasoning, until the trigger text appears, then constrains the rest. Both return `Result<String, StructError>`.

## Status

Pre-release. The spec is v0.3. The crate is std-only Rust, and a 5,000-case round-trip test suite covers it.

Next:

- The doctor: structural repair of model output that never touches values. It fixes code fences, echoed schemas, chatter, header rows, a missing final `|` and a cut-off last record, and reports everything else as a bad line. It is specified in `SPEC.md` and not built yet.
- A total-cost benchmark: tokens per correct result, including reasoning and retries. The R1 numbers above count output tokens only.

## License

Apache 2.0.
