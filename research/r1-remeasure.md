# Milestone R1: Struct v0.2 Token Re-measurement & Delimiter Analysis

## 1. Executive Summary

Milestone R1 re-evaluates token efficiency following format updates in Struct v0.2 (`SPEC.md`). The v0.2 specification introduces three syntactic refinements compared to the initial M0 draft:
1. **Mandatory trailing pipe (`|`)**: Every data line now terminates with an unescaped `|`.
2. **Explicit Null marker (`~`)**: Null values are represented by `~` (instead of empty `||`).
3. **Explicit Schema blocks (`schema:name{...}`)**: Schema definitions now start with the `schema:` keyword, and files are structured as schema-governed blocks. In nested lists, child record types are written inline (`record{...}`) rather than referencing separate type definitions.

### Key Results at a Glance

| Dataset Shape | Metric | Minified JSON | TOON | Struct v0 (M0 with schema) | Struct v0.2 (+ typed schema) | vs TOON | vs Min JSON |
|---|---|---|---|---|---|---|---|
| **Flat Tabular** (20 rows × 5 cols) | Mean Tokens | 698.2 | 495.8 | 492.2 | **507.8** | +2.4% | -27.3% |
| **Nested Fixed** (10 rows × 2 cols sub) | Mean Tokens | 367.5 | 248.5 | 259.0 | **269.5** | +8.5% | -26.7% |
| **Nested Variable** (4 rows × 2-level lists) | Mean Tokens | 317.0 | 387.8 | 267.8 | **282.8** | **-27.1%** | **-10.8%** |

### Core Insights
1. **Trailing Pipe Cost**:
   - In **Llama** and **LFM**, `|\n` merges into a single combined token (`['|\n']`), meaning trailing pipes add **zero tokens** to line endings.
   - In **Qwen** and **Gemma**, `|` and `\n` remain separate tokens (`['|', '\n']`), adding exactly 1 token per data line (+20 tokens across 20 rows).
2. **Schema Keyword Overhead**:
   - The `schema:` prefix adds 2 tokens across all tokenizers. Inlining `record{...}` in `nested_variable` adds 4–7 tokens depending on the vocabulary.
3. **The `~` Null Marker**:
   - An isolated empty field `||` was a single token in M0. The explicit null `|~|` decomposes into `['|', '~', '|']` (3 tokens), adding 2 tokens per null field over `||`.
4. **Hierarchical Lists Dominate TOON**:
   - On variable nested data, Struct v0.2 uses **282.8 tokens** vs TOON's **387.8 tokens**, outperforming TOON by **-27.1%** and minified JSON by **-10.8%**, because TOON is forced into repeating dictionary keys (`- text:`, `subnotes:`) on every hierarchical record.

---

## 2. Methodology

### Test Environment & Tokenizers
Tokenization was performed in-process on CPU via `llama.cpp`'s C API (`llama.dll`) with `vocab_only = True` and zero GPU memory allocation (`-ngl 0`). Exact token counts, token IDs, and string piece decompositions were collected across all four target model families:

1. **Qwen**: `Qwen3.5-4B-Q4_K_M.gguf` (248,320 vocabulary, BPE)
2. **Llama**: `Llama-3.2-3B-Instruct-Q4_K_M.gguf` (128,256 vocabulary, BPE)
3. **Gemma**: `gemma-3-4b-it-Q4_K_M.gguf` (262,144 vocabulary, SentencePiece BPE)
4. **LFM**: `LFM2.5-8B-A1B-Q4_K_M.gguf` (128,000 vocabulary, BPE)

### Benchmark Datasets
The datasets represent the three canonical shapes evaluated in M0:
1. **Flat Tabular**: 20 rows of academic claims, 5 columns (`id`, `title`, `verdict`, `year`, `verified`).
2. **Nested Fixed**: 10 rows of claims with a 2-field nested source record (`source: {id, year}`).
3. **Nested Variable**: 4 claim records with 2 levels of variable-length child lists (`notes` and `subnotes`).

---

## 3. Dataset Token Counts & Comparisons

### 3.1 Flat Tabular (20 records × 5 fields)

| Format | Qwen | Llama | Gemma | LFM | Mean | vs TOON | vs Min JSON | vs Struct v0 (+schema) |
|---|---|---|---|---|---|---|---|---|
| Minified JSON | 759 | 617 | 756 | 661 | 698.2 | +40.8% | 0.0% | +41.8% |
| TOON | 564 | 421 | 570 | 428 | 495.8 | 0.0% | -29.0% | +0.7% |
| Struct v0 (values only, M0) | 534 | 392 | 534 | 420 | 470.0 | -5.2% | -32.7% | -4.5% |
| Struct v0 (+ schema, M0) | 556 | 413 | 557 | 443 | 492.2 | -0.7% | -29.5% | 0.0% |
| Struct v0.2 (values only) | 554 | 392 | 554 | 420 | 480.0 | -3.2% | -31.3% | -2.5% |
| **Struct v0.2 (+ typed schema)** | **581** | **418** | **583** | **449** | **507.8** | **+2.4%** | **-27.3%** | **+3.1%** |
| Struct v0.2 (+ untyped schema) | 578 | 415 | 579 | 445 | 504.2 | +1.7% | -27.8% | +2.4% |

**Observations**:
- In Llama and LFM, values-only tokens are completely identical between v0 and v0.2 (392 and 420 tokens) because the trailing pipe merges with newline into `|\n`.
- In Qwen and Gemma, the 20 trailing pipes add exactly 20 tokens (534 -> 554).
- With schema included, Struct v0.2 averages 507.8 tokens, beating minified JSON by 27.3% and sitting within 2.4% of TOON.

### 3.2 Nested Fixed (10 records with 2-field nested object)

| Format | Qwen | Llama | Gemma | LFM | Mean | vs TOON | vs Min JSON | vs Struct v0 (+schema) |
|---|---|---|---|---|---|---|---|---|
| Minified JSON | 397 | 327 | 396 | 350 | 367.5 | +47.9% | 0.0% | +41.9% |
| TOON | 279 | 208 | 292 | 215 | 248.5 | 0.0% | -32.4% | -4.1% |
| Struct v0 (values only, M0) | 265 | 195 | 264 | 209 | 233.2 | -6.1% | -36.5% | -9.9% |
| Struct v0 (+ schema, M0) | 290 | 219 | 291 | 236 | 259.0 | +4.2% | -29.5% | 0.0% |
| Struct v0.2 (values only) | 275 | 195 | 274 | 209 | 238.2 | -4.1% | -35.2% | -8.0% |
| **Struct v0.2 (+ typed schema)** | **305** | **224** | **307** | **242** | **269.5** | **+8.5%** | **-26.7%** | **+4.1%** |
| Struct v0.2 (+ untyped schema) | 302 | 221 | 303 | 238 | 266.0 | +7.0% | -27.6% | +2.7% |

**Observations**:
- Values-only Struct beats TOON across all models (238.2 vs 248.5, -4.1%).
- TOON's compact header notation `source{id,year}` costs only 17.0 tokens vs Struct's explicit `schema:claim{text, verdict:..., source:record{id, year:int}, verified:bool}` (30–33 tokens). On a short 10-row dataset, this gives TOON an 8.5% edge when the schema is transmitted in-band. Against minified JSON, Struct v0.2 saves 26.7% of tokens.

### 3.3 Nested Variable (4 records with 2-level variable child lists)

| Format | Qwen | Llama | Gemma | LFM | Mean | vs TOON | vs Min JSON | vs Struct v0 (+schema) |
|---|---|---|---|---|---|---|---|---|
| Minified JSON | 337 | 304 | 323 | 304 | 317.0 | -18.2% | 0.0% | +18.4% |
| TOON | 410 | 367 | 404 | 370 | 387.8 | 0.0% | +22.3% | +44.8% |
| Struct v0 (values only, M0) | 249 | 216 | 239 | 221 | 231.2 | -40.4% | -27.1% | -13.6% |
| Struct v0 (+ schema, M0) | 283 | 249 | 279 | 260 | 267.8 | -30.9% | -15.5% | 0.0% |
| Struct v0.2 (values only) | 272 | 216 | 262 | 221 | 242.8 | -37.4% | -23.4% | -9.3% |
| **Struct v0.2 (+ typed schema)** | **309** | **252** | **306** | **264** | **282.8** | **-27.1%** | **-10.8%** | **+5.6%** |
| Struct v0.2 (+ untyped schema) | 308 | 251 | 304 | 262 | 281.2 | -27.5% | -11.3% | +5.0% |

**Observations**:
- Struct v0.2 decisively outperforms TOON by **-27.1%** (282.8 vs 387.8 tokens) and beats minified JSON by **-10.8%**.
- TOON cannot maintain tabular form when child list lengths vary; it is forced to emit key-value maps (`- text:`, `subnotes:`), which inflates token volume. Struct's positional child lines (`>`) preserve token efficiency even on deeply nested structures.

---

## 4. Micro-Benchmarks: Line Prefixes, Endings, and Special Tokens

### 4.1 Line Ending: `|` + newline (`|\n`)

| Scenario | Qwen | Llama | Gemma | LFM | Mean Tokens | Token Decomposition (Qwen / Llama / Gemma / LFM) |
|---|---|---|---|---|---|---|
| `|\n` (isolated) | 2 | 1 | 2 | 1 | 1.50 | `['|', '\n']` / `['|\n']` / `['|', '\n']` / `['|\n']` |
| `\n` (isolated) | 1 | 1 | 1 | 1 | 1.00 | `['\n']` / `['\n']` / `['\n']` / `['\n']` |
| Full line with `|\n` | 28 | 20 | 29 | 22 | 24.75 | `...|`, `\n` / `...|\n` / `...|`, `\n` / `...|\n` |
| Full line with plain `\n` | 27 | 20 | 28 | 22 | 24.25 | `...\n` / `...\n` / `...\n` / `...\n` |
| Word + `|\n` (`refuted|\n`) | 4 | 3 | 4 | 3 | 3.50 | `['ref', 'uted', '|', '\n']` / `['ref', 'uted', '|\n']` |
| Word + `\n` (`refuted\n`) | 3 | 3 | 3 | 3 | 3.00 | `['ref', 'uted', '\n']` / `['ref', 'uted', '\n']` |
| Number + `|\n` (`2024|\n`) | 6 | 3 | 6 | 3 | 4.50 | `['2', '0', '2', '4', '|', '\n']` / `['202', '4', '|\n']` |
| Number + `\n` (`2024\n`) | 5 | 3 | 5 | 3 | 4.00 | `['2', '0', '2', '4', '\n']` / `['202', '4', '\n']` |

**Takeaway**:
In Llama and LFM, terminating a line with `|` has **zero token penalty** compared to an unterminated line because the tokenizer vocabulary contains a dedicated `|\n` token. In Qwen and Gemma, the trailing pipe adds exactly 1 token per line.

### 4.2 Child Line Markers: `>` Repeated 1–8 Times

Tested alone, followed by text (`>notes are from abstract only|`), and followed by list index (`>3|notes...` and `>1.2|notes...`).

| Depth | Marker | Marker Tokens (Q/L/G/LFM) | Marker Decomposition | Followed by Text (Mean Tok) | Followed by `3|` (Mean Tok) | Followed by `1.2|` (Mean Tok) |
|---|---|---|---|---|---|---|
| 1 | `>` | 1 / 1 / 1 / 1 | `['>']` | 7.00 | 9.00 | 11.00 |
| 2 | `>>` | 1 / 1 / 1 / 1 | `['>>']` | 7.00 | 9.00 | 11.00 |
| 3 | `>>>` | 1 / 1 / 1 / 1 | `['>>>']` | 7.00 | 9.00 | 11.00 |
| 4 | `>>>>` | 1 / 1 / 1 / 1 | `['>>>>']` | 7.00 | 9.00 | 11.00 |
| 5 | `>>>>>` | 2 / 2 / 2 / 2 | `['>>>>', '>']` or `['>>', '>>>']` | 8.00 | 10.00 | 12.00 |
| 6 | `>>>>>>` | 2 / 2 / 2 / 2 | `['>>>>', '>>']` | 8.00 | 10.00 | 12.00 |
| 7 | `>>>>>>>` | 1 / 1 / 2 / 2 | `['>>>>>>>']` (Q/L) | 7.50 | 9.50 | 11.50 |
| 8 | `>>>>>>>>` | 1 / 1 / 1 / 2 | `['>>>>>>>>']` (Q/L/G) | 7.25 | 9.25 | 11.25 |

**Takeaway**:
- Depths 1 through 4 cost exactly **1 token** across all tokenizers.
- Depths 5 and 6 cost 2 tokens.
- Depths 7 and 8 form 1 token in Qwen and Llama, and 1–2 tokens in Gemma and LFM.
- Appending an explicit field index like `3|` adds exactly 2 tokens (`'3'`, `'|'`). A dotted index like `1.2|` adds 4 tokens (`'1'`, `'.' `, `'2'`, `'|'`). In no case does the marker merge with the index digits or text.

### 4.3 Null Field: `~` Between Pipes (`|~|`)

| Scenario | Qwen | Llama | Gemma | LFM | Mean Tokens | Pieces |
|---|---|---|---|---|---|---|
| `~` (isolated) | 1 | 1 | 1 | 1 | 1.00 | `['~']` |
| `|~|` (isolated) | 3 | 3 | 3 | 3 | 3.00 | `['|', '~', '|']` |
| `||` (empty field, M0) | 1 | 1 | 1 | 1 | 1.00 | `['||']` |
| `refuted|~|2024|` | 10 | 8 | 10 | 8 | 9.00 | `...|~|...` (3 tokens for `|~|`) |
| `refuted||2024|` | 8 | 6 | 8 | 6 | 7.00 | `...||...` (1 token for `||`) |
| `|~|~|~|` (3 null fields) | 7 | 7 | 7 | 7 | 7.00 | `['|', '~', '|', '~', '|', '~', '|']` |
| `||||` (3 empty fields) | 1 | 1 | 2 | 1 | 1.25 | `['||||']` |

**Takeaway**:
Representing null with `~` costs **2 more tokens** per field than empty `||`. However, this cost buys significant semantic clarity:
1. It eliminates ambiguities between trailing empty strings and trailing delimiters.
2. It makes null visually explicit for human reviewers and small language models.
3. In SPEC.md, `~` is null, while `\~` is literal `~`.

### 4.4 Escape Sequence: `\|` Inside Text

| Scenario | Qwen | Llama | Gemma | LFM | Mean Tokens | Pieces |
|---|---|---|---|---|---|---|
| `\|` (isolated) | 2 | 2 | 1 | 1 | 1.50 | `['\\', '|']` / `['\\', '|']` / `['\\|']` / `['\\|']` |
| `|` (isolated) | 1 | 1 | 1 | 1 | 1.00 | `['|']` |
| `\` (isolated) | 1 | 1 | 1 | 1 | 1.00 | `['\\']` |
| `Rope \| scaling` | 4 | 4 | 4 | 4 | 4.00 | `['R', 'ope', ' \\|', ' scaling']` |
| `Rope   scaling` (2 spaces) | 4 | 4 | 4 | 4 | 4.00 | `['R', 'ope', '  ', ' scaling']` |
| `Rope | scaling` (pipe + space) | 4 | 4 | 4 | 4 | 4.00 | `['R', 'ope', ' |', ' scaling']` |
| `if a \|\| b:` | 7 | 7 | 6 | 7 | 6.75 | `['if', ' a', ' \\', '|\\', '|', ' b', ':']` |
| `pipe\|inside` | 4 | 4 | 3 | 3 | 3.50 | `['pipe', '\\', '|', 'inside']` (Q/L) / `['pipe', '\\|', 'inside']` (G/LFM) |

**Takeaway**:
In Gemma and LFM, `\|` is recognized as a single token `['\\|']`. In Qwen and Llama, it splits cleanly into `['\\']` and `['|']`. When preceded by a space (the most common pattern in prose), all four tokenizers merge the space with the escape sequence (`[' \\|']`), incurring zero overhead relative to an unescaped pipe.
