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

## Retrieval accuracy (R4): Claude Haiku

Can a model read the data back? We gave Claude Haiku 5.5 the same data in seven formats and asked the same 48 questions about it: single lookups, nested lookups, counts and filters, and questions about missing values. Only the format changed. Haiku ran through `claude -p` at low effort, with a one-line system prompt and no tools, 4 times per prompt (96 calls).

Answers were graded by meaning: right if they give exactly the right value, whatever the wording. Strict text matching is shown next to it, because it marks right answers wrong when the model adds a note, as in `2404.7570, 2411.9702 (verdict is "~")`. A blind re-grade of 184 sampled answers agreed with the grades on 180, and on all 15 sampled Struct answers.

| Format | By meaning | Strict match | Mean input tokens |
|---|---|---|---|
| **Struct** | **189/192 (98.4%)** | 146/192 (76.0%) | 2,123 |
| XML-style tags | 183/192 (95.3%) | 130/192 (67.7%) | 3,922 |
| Minified JSON | 176/192 (91.7%) | 158/192 (82.3%) | 2,717 |
| YAML | 175/192 (91.1%) | 154/192 (80.2%) | 2,776 |
| TOON | 174/192 (90.6%) | 163/192 (84.9%) | 2,169 |
| TSV (flat data only) | 90/96 (93.8%) | 75/96 (78.1%) | 2,810 |
| `key: value` lines (flat data only) | 88/96 (91.7%) | 76/96 (79.2%) | 3,831 |

Input tokens are as counted by the Anthropic API, including about 640 fixed tokens per call. TSV and `key: value` can't hold nested data, so they ran on the two flat datasets only and their numbers aren't directly comparable with the rest.

On Haiku, Struct had both the highest accuracy and the fewest input tokens. This is one model. Results on small local models are not in this table.

By seed, by meaning. Each seed is one complete run of all 24 prompts, so every format is compared on the same runs:

| Format | Seed 1 | Seed 2 | Seed 3 | Seed 4 |
|---|---|---|---|---|
| **Struct** | **47/48** | **46/48** | **48/48** | **48/48** |
| XML-style tags | 45/48 | 46/48 | 45/48 | 47/48 |
| YAML | 44/48 | 44/48 | 43/48 | 44/48 |
| Minified JSON | 44/48 | 44/48 | 44/48 | 44/48 |
| TOON | 42/48 | 44/48 | 45/48 | 43/48 |
| `key: value` lines (flat only) | 21/24 | 23/24 | 22/24 | 22/24 |
| TSV (flat only) | 20/24 | 24/24 | 22/24 | 24/24 |

Source: `research/r4-retrieval-haiku.md`, `bench/r4/grades/haiku_*.json` and `bench/r4/grades/crosscheck/`.

## Retrieval accuracy (R4): Gemini 3.8 Flash

The same test on Gemini 3.8 Flash (low variant), run headless through the Antigravity CLI, 4 times per prompt (96 calls). Grading is the same: by meaning, 7 graders, then a blind re-grade of 82 sampled answers with the model's name and the formats removed. It agreed with all 82.

| Format | By meaning | Strict match | Mean tokens per answer |
|---|---|---|---|
| Minified JSON | 192/192 (100%) | 174/192 (90.6%) | 50,374 |
| TOON | 192/192 (100%) | 181/192 (94.3%) | 35,422 |
| YAML | 191/192 (99.5%) | 173/192 (90.1%) | 25,596 |
| **Struct** | **190/192 (99.0%)** | 157/192 (81.8%) | **21,550** |
| XML-style tags | 188/192 (97.9%) | 170/192 (88.5%) | 39,898 |
| TSV (flat data only) | 95/96 (99.0%) | 83/96 (86.5%) | 35,424 |
| `key: value` lines (flat data only) | 95/96 (99.0%) | 93/96 (96.9%) | 36,250 |

Tokens per answer are input plus output as the CLI reports them, the total cost of getting one set of answers. They include about 13,600 tokens of the CLI's own agent prompt on every call, and the model's thinking, which the CLI counts inside output. So compare them across formats here, not with the Haiku table. TSV and `key: value` ran only on the two flat datasets, which are the largest, so their means aren't directly comparable with the rest.

Gemini read every format almost perfectly. At that accuracy, Struct was the cheapest format to get answers from: about 43% of JSON's cost per answer. Most of the difference is thinking: on the 100-row table, Gemini wrote about 4,350 output tokens per answer on Struct and about 10,000 to 15,000 on JSON.

By seed, by meaning. Each seed is one complete run of all 24 prompts, so every format is compared on the same runs:

| Format | Seed 1 | Seed 2 | Seed 3 | Seed 4 |
|---|---|---|---|---|
| **Struct** | **47/48** | **47/48** | **48/48** | **48/48** |
| XML-style tags | 47/48 | 47/48 | 47/48 | 47/48 |
| YAML | 47/48 | 48/48 | 48/48 | 48/48 |
| Minified JSON | 48/48 | 48/48 | 48/48 | 48/48 |
| TOON | 48/48 | 48/48 | 48/48 | 48/48 |
| `key: value` lines (flat only) | 23/24 | 24/24 | 24/24 | 24/24 |
| TSV (flat only) | 24/24 | 23/24 | 24/24 | 24/24 |

Source: `research/r4-retrieval-gemini.md`, `bench/r4/grades/gemini_*.json` and `bench/r4/grades/crosscheck_b/`.

## Retrieval accuracy (R4): Gemini 3.5 Flash Lite, thinking off and on

A much weaker model, called directly through the Gemini API (no agent prompt, exact token counts), 4 times per prompt. We ran it twice: with thinking at `low`, where it barely thinks (0 thinking tokens on almost every call), and with thinking at `high`. Grading is by meaning, with graders that weren't told which model they were grading, then a blind re-grade: 73 of 76 sampled answers agreed for the low run, and all 55 for the high run.

| Format | Thinking low | Thinking high | Mean thinking tokens (high) | Mean input tokens |
|---|---|---|---|---|
| **Struct** | 140/192 (72.9%) | **191/192 (99.5%)** | 5,408 | **1,372** |
| XML-style tags | 164/192 (85.4%) | 191/192 (99.5%) | 4,684 | 2,967 |
| YAML | 170/192 (88.5%) | 190/192 (99.0%) | 5,900 | 1,957 |
| Minified JSON | 168/192 (87.5%) | 188/192 (97.9%) | 5,508 | 1,688 |
| TOON | 159/192 (82.8%) | 188/192 (97.9%) | 4,394 | 1,418 |
| `key: value` lines (flat data only) | 75/96 (78.1%) | 94/96 (97.9%) | 8,279 | 2,978 |
| TSV (flat data only) | 70/96 (72.9%) | 93/96 (96.9%) | 9,252 | 2,092 |

With thinking on, Struct tied XML for the highest accuracy, with the fewest input tokens of any format.

With thinking off, Struct came last among the five formats that ran on every dataset. Its misses were mostly questions about missing values (24 misses, against 7 for TOON and 6 for JSON): Struct writes no value as `~` and an empty list as no line at all, and without reasoning the model often missed both. Counting questions were hard in every format. So on a weak model, a single pass without thinking is where Struct is fragile.

By seed, by meaning. Each seed is one complete run of all 24 prompts, so every format is compared on the same runs:

Thinking low:

| Format | Seed 1 | Seed 2 | Seed 3 | Seed 4 |
|---|---|---|---|---|
| **Struct** | **35/48** | **35/48** | **35/48** | **35/48** |
| XML-style tags | 40/48 | 42/48 | 40/48 | 42/48 |
| YAML | 42/48 | 43/48 | 42/48 | 43/48 |
| Minified JSON | 41/48 | 43/48 | 41/48 | 43/48 |
| TOON | 39/48 | 41/48 | 39/48 | 40/48 |
| `key: value` lines (flat only) | 21/24 | 18/24 | 18/24 | 18/24 |
| TSV (flat only) | 18/24 | 16/24 | 17/24 | 19/24 |

Thinking high:

| Format | Seed 1 | Seed 2 | Seed 3 | Seed 4 |
|---|---|---|---|---|
| **Struct** | **48/48** | **48/48** | **48/48** | **47/48** |
| XML-style tags | 48/48 | 48/48 | 48/48 | 47/48 |
| YAML | 48/48 | 48/48 | 47/48 | 47/48 |
| Minified JSON | 47/48 | 46/48 | 48/48 | 47/48 |
| TOON | 48/48 | 48/48 | 46/48 | 46/48 |
| `key: value` lines (flat only) | 23/24 | 24/24 | 24/24 | 23/24 |
| TSV (flat only) | 23/24 | 24/24 | 23/24 | 23/24 |

Source: `research/r4-retrieval-gemini-lite.md`, `research/r4-retrieval-gemini-lite-high.md`, `bench/r4/grades/m4_*.json` (thinking low) and `m5_*.json` (thinking high), and `bench/r4/grades/crosscheck_jb007/` and `crosscheck_m5/`.

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
