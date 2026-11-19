# Milestone R3c: Struct Doctor Real-Model Output Report (Run 2)

> **This run is superseded by run3 (no token cap, no thinking prefills); kept as a record.** Most failures below are 1,500-token budget artifacts, not format failures. See section 8 for limits.

This report documents the empirical evaluation of **Struct Doctor** (`structl::doctor`) across **100 real model outputs** (5 model configurations × 4 schemas × 5 runs). All runs were executed **without grammar constraints**, using canonical `prompt(schemas)` prompts with real source notes from R1 fixtures, evaluated under salvage mode against exact ground truth.

## 1. Experimental Setup & Environment

- **Hardware:** NVIDIA GeForce RTX 4060 Laptop GPU (8,188 MiB VRAM), Windows 11.
- **Inference Engine:** `llama-server.exe` on `127.0.0.1:8090`.
- **Parameters:** Temperature 0.7, token budget 1,500 (`n_predict`), fixed seeds 42–46 per schema.
- **Token Cap Recording:** Hit cap recorded strictly from `stop_type == "limit"`. Prompt truncation kept separate.
- **Prompts:** Canonical `prompt(schemas)` per SPEC §10, one blank line, real source notes ending with `Write these as Struct records.`
- **Models Evaluated (5 configurations):**
  1. `Llama-3.2-3B-Instruct` (thinking OFF)
  2. `gemma-4-E2B-it` (thinking OFF)
  3. `gemma-4-E2B-it` (thinking ON, end-of-thought `<channel|>`)
  4. `LFM2.5-8B-A1B` (thinking ON, end-of-thought `</think>`)
  5. `Mellum2.1-12B-A2.5B-Thinking` (thinking ON, end-of-thought `</think>`, `-ngl 16`)

## 2. Prompts Used for Each Schema

### Full Prompt for `s1_flat`

```text
schema:claim{id, title, verdict:enum(supported,refuted,unverified), year:int, verified:bool}
Reply with data only in Struct format: no prose, no code fences.
Never write a header row or column names. The first line is already data.
One record per line: values in schema order, separated by |, and every line ends with |.
~ means null. bool is 1 or 0. enum values must be one of the listed labels exactly.
Inside a value, write \| for | and \ for \.
For text with line breaks, or with spaces or tabs at its start or end, start the value with \r, then write it exactly as it is, keeping every space, tab, indent and line break, until the closing |.
Example: text|text|supported|7|~|

1. Claim 2405.05254: title 'Rope scaling hurts long ctx', published in 2024, verdict refuted, verified false.
2. Claim 1706.03762: title 'Attention is all you need', published in 2017, verdict supported, verified true.
3. Claim 2106.09685: title 'LoRA low rank adaptation', published in 2021, verdict supported, verified true.
4. Claim 2309.06180: title 'Mistral 7B architecture', published in 2023, verdict supported, verified true.
5. Claim 2401.04088: title 'DeepSeekMoE mixture experts', published in 2024, verdict supported, verified true.
Write these as Struct records.
```

### Full Prompt for `s2_nested_fixed`

```text
schema:claim{text, verdict:enum(supported,refuted,unverified), source:record{id, year:int}, verified:bool}
Reply with data only in Struct format: no prose, no code fences.
Never write a header row or column names. The first line is already data.
One record per line: values in schema order, separated by |, and every line ends with |.
~ means null. bool is 1 or 0. enum values must be one of the listed labels exactly.
Inside a value, write \| for | and \ for \.
A record{...} field adds its fields as extra columns, in order.
For text with line breaks, or with spaces or tabs at its start or end, start the value with \r, then write it exactly as it is, keeping every space, tab, indent and line break, until the closing |.
Example: text|supported|text|7|~|

1. Claim 'Rope scaling hurts long ctx', verdict refuted, source id 2405.05254 from year 2024, verified false.
2. Claim 'Attention is all you need', verdict supported, source id 1706.03762 from year 2017, verified true.
3. Claim 'LoRA low rank adaptation', verdict supported, source id 2106.09685 from year 2021, verified true.
4. Claim 'Mistral 7B architecture', verdict supported, source id 2309.06180 from year 2023, verified true.
5. Claim 'DeepSeekMoE mixture experts', verdict supported, source id 2401.04088 from year 2024, verified true.
Write these as Struct records.
```

### Full Prompt for `s3_nested_variable`

```text
schema:claim{text, verdict:enum(supported,refuted,unknown), source:record{id, year:int}, notes:list[record{text, subnotes:list[str]}]}
Reply with data only in Struct format: no prose, no code fences.
Never write a header row or column names. The first line is already data.
One record per line: values in schema order, separated by |, and every line ends with |.
~ means null. bool is 1 or 0. enum values must be one of the listed labels exactly.
Inside a value, write \| for | and \ for \.
A record{...} field adds its fields as extra columns, in order.
List items go on the lines below their parent, starting with one > per level: > for its items, >> for theirs.
For text with line breaks, or with spaces or tabs at its start or end, start the value with \r, then write it exactly as it is, keeping every space, tab, indent and line break, until the closing |.
Example: text|supported|text|~|

1. Claim 'Rope scaling hurts long ctx', verdict refuted, source id 2405.05254 in year 2024. Notes:
- Note text 'notes are from abstract only', with subnotes 'abstract mentions 32k limit' and 'appendix B clarifies'.
- Note text 'table 3 contradicts', with subnote 'llama 7b tested only'.
2. Claim 'LoRA matches full finetune', verdict supported, source id 2106.09685 in year 2021. Notes:
- Note text 'tested on GLUE and Wikisql', with subnotes 'rank 4 sufficient' and 'higher ranks show diminishing returns'.
- Note text 'memory footprint reduced 3x', with subnote 'optimizer state minimal'.
- Note text 'checkpoint size under 50MB', with no subnotes.
3. Claim 'KV cache quant loses 2x acc', verdict refuted, source id 2406.12345 in year 2024. Notes:
- Note text 'outlier channels preserved', with subnotes 'per-head scales help' and 'perplexity intact'.
4. Claim 'Chinchilla tokens beat params', verdict supported, source id 2203.15556 in year 2022. Notes:
- Note text 'optimal ratio is 20 to 1', with subnotes 'tested across 400 models' and 'cosine schedule used'.
- Note text 'kaplan tokens were suboptimal', with subnote 'fixed learning rate flawed'.
5. Claim 'Language models few-shot', verdict supported, source id 2005.14165 in year 2020. Notes:
- Note text 'scaling curve is smooth', with subnotes 'tested up to 175B' and 'in-context learning observed'.
Write these as Struct records.
```

### Full Prompt for `s4_two_class_code`

```text
schema:file{name, path}
schema:patch{file_name, code, line:int}
Reply with data only in Struct format: no prose, no code fences.
Never write a header row or column names. The first line is already data.
One record per line: values in schema order, separated by |, and every line ends with |.
~ means null. bool is 1 or 0. enum values must be one of the listed labels exactly.
Inside a value, write \| for | and \ for \.
For text with line breaks, or with spaces or tabs at its start or end, start the value with \r, then write it exactly as it is, keeping every space, tab, indent and line break, until the closing |.
Start each block with schema:name on its own line.
Example: text|~|

1. File 'lib.rs' at path 'src/lib.rs'. Patch for 'lib.rs' at line 10 with code:
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
2. File 'util.rs' at path 'src/util.rs'. Patch for 'util.rs' at line 5 with code:
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
3. File 'main.rs' at path 'src/main.rs'. Patch for 'main.rs' at line 1 with code:
fn main() {
    println!("Hello, world!");
}
Write these as Struct records.
```

## 3. 5×4 Summary Matrix (Outcomes, Recall, Field Accuracy, Cap Hits)

Definitions: **Record Recall** = mean records decoded / records expected (zero-record runs count as 0). **Field Accuracy (Decoded)** = share of fields exactly right among the records that decoded, pooled over runs that decoded at least one record; `n/a` when no run decoded anything (zero-record runs are not averaged in as 0%). **Partial** = repaired with no bad lines but fewer records than expected. **Cap Hits** = runs with `stop_type == "limit"` at the 1,500-token cap. **Distinct Outputs** = distinct raw output files out of 5 seeds.

| Model | Schema | Runs | Clean | Repaired | Partial | Salvaged | Nothing Usable | Avg Record Yield | Record Recall | Field Accuracy (Decoded) | Cap Hits | Distinct Outputs |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Llama-3.2-3B (thinking off) | `s1_flat` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0 / 5 | 0% | n/a | 0/5 | 5/5 |
| Llama-3.2-3B (thinking off) | `s2_nested_fixed` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0 / 5 | 0% | n/a | 1/5 | 5/5 |
| Llama-3.2-3B (thinking off) | `s3_nested_variable` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0 / 5 | 0% | n/a | 0/5 | 5/5 |
| Llama-3.2-3B (thinking off) | `s4_two_class_code` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0 / 6 | 0% | n/a | 0/5 | 5/5 |
| Gemma-4-E2B (thinking off) | `s1_flat` | 5 | 2 | 0 | 0 | 0 | 3 | 2.0 / 5 | 40% | 60.0% | 0/5 | 2/5 |
| Gemma-4-E2B (thinking off) | `s2_nested_fixed` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0 / 5 | 0% | n/a | 0/5 | 1/5 |
| Gemma-4-E2B (thinking off) | `s3_nested_variable` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0 / 5 | 0% | n/a | 0/5 | 2/5 |
| Gemma-4-E2B (thinking off) | `s4_two_class_code` | 5 | 0 | 0 | 0 | 2 | 3 | 0.4 / 6 | 7% | 50.0% | 0/5 | 4/5 |
| Gemma-4-E2B (thinking on) | `s1_flat` | 5 | 5 | 0 | 0 | 0 | 0 | 5.0 / 5 | 100% | 96.0% | 0/5 | 2/5 |
| Gemma-4-E2B (thinking on) | `s2_nested_fixed` | 5 | 3 | 0 | 0 | 0 | 2 | 3.0 / 5 | 60% | 100.0% | 0/5 | 3/5 |
| Gemma-4-E2B (thinking on) | `s3_nested_variable` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0 / 5 | 0% | n/a | 0/5 | 5/5 |
| Gemma-4-E2B (thinking on) | `s4_two_class_code` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0 / 6 | 0% | n/a | 0/5 | 4/5 |
| LFM2.5-8B (thinking on) | `s1_flat` | 5 | 0 | 3 | 0 | 0 | 2 | 3.0 / 5 | 60% | 93.3% | 0/5 | 4/5 |
| LFM2.5-8B (thinking on) | `s2_nested_fixed` | 5 | 0 | 1 | 0 | 0 | 4 | 1.0 / 5 | 20% | 100.0% | 1/5 | 4/5 |
| LFM2.5-8B (thinking on) | `s3_nested_variable` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0 / 5 | 0% | n/a | 5/5 | 1/5 |
| LFM2.5-8B (thinking on) | `s4_two_class_code` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0 / 6 | 0% | n/a | 5/5 | 1/5 |
| Mellum2.1-12B (thinking on) | `s1_flat` | 5 | 0 | 0 | 1 | 0 | 4 | 0.6 / 5 | 12% | 100.0% | 5/5 | 2/5 |
| Mellum2.1-12B (thinking on) | `s2_nested_fixed` | 5 | 1 | 0 | 0 | 0 | 4 | 1.0 / 5 | 20% | 100.0% | 4/5 | 3/5 |
| Mellum2.1-12B (thinking on) | `s3_nested_variable` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0 / 5 | 0% | n/a | 5/5 | 1/5 |
| Mellum2.1-12B (thinking on) | `s4_two_class_code` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0 / 6 | 0% | n/a | 5/5 | 1/5 |

**Cap hits: 31/100 runs.** By model: Llama-3.2-3B (thinking off) 1/20; Gemma-4-E2B (thinking off) 0/20; Gemma-4-E2B (thinking on) 0/20; LFM2.5-8B (thinking on) 11/20; Mellum2.1-12B (thinking on) 19/20. Most thinking-model failures (LFM and Mellum) are budget artifacts: the 1,500-token cap ended generation inside or right after the reasoning block, so these rows measure the cap, not whether the model can write Struct.


## 4. Repairs Applied by Kind

| Safe Fix | Count Across All Runs |
|---|---|
| `MissingFinalPipe` | 20 |
| `CutOffLastRecord` | 11 |
| `Chatter` | 6 |

## 5. Bad Lines Encountered by Problem Diagnostic

| Problem Diagnostic | Frequency |
|---|---|
| with several outside classes, data before the first schema line is an error | 85 |
| data line must always end with an unescaped '|' | 31 |
| expected 4 columns, found 6 | 26 |
| expected 5 columns, found 7 | 25 |
| expected 4 columns, found 5 | 20 |
| expected 3 columns, found 2 | 16 |
| invalid escape sequence '\ ' | 15 |
| expected 4 columns, found 8 | 13 |
| expected 5 columns, found 1 | 12 |
| expected 5 columns, found 4 | 9 |
| invalid escape sequence '\r' | 7 |
| expected 5 columns, found 6 | 6 |
| dropped as child of line 1 | 6 |
| raw value '\r' is not allowed in non-str field | 5 |
| dropped as child of line 7 | 5 |
| 'true' is not a valid bool (expected 1 or 0) | 4 |
| expected 4 columns, found 1 | 4 |
| dropped as child of line 18 | 4 |
| dropped as child of line 4 | 3 |
| '2024' is not one of: supported, refuted, unverified | 2 |
| expected 4 columns, found 7 | 2 |
| expected 4 columns, found 3 | 2 |
| 'year:2024' is not a valid int (i64) | 2 |
| dropped as child of line 14 | 2 |
| dropped as child of line 24 | 2 |
| dropped as child of line 10 | 2 |
| '|2021' is not a valid int (i64) | 1 |
| '2017' is not one of: supported, refuted, unverified | 1 |
| '2021' is not one of: supported, refuted, unverified | 1 |
| '2023' is not one of: supported, refuted, unverified | 1 |
| expected 5 columns, found 10 | 1 |
| expected 5 columns, found 26 | 1 |
| 'false' is not a valid bool (expected 1 or 0) | 1 |
| expected 4 columns, found 9 | 1 |
| expected 4 columns, found 2 | 1 |
| reusing class 'file{name:lib.rs, path:src/lib.rs}' that hasn't been defined yet | 1 |
| reusing class 'patch{file_name:lib.rs, code:pub fn add(a: i32, b: i32) -> i32 {a + b}, line:10}' that hasn't been defined yet | 1 |
| reusing class 'file{name:util.rs, path:src/util.rs}' that hasn't been defined yet | 1 |
| reusing class 'patch{file_name:util.rs, code:pub fn greet(name: &str) -> String {format!("Hello, {}!", name)}, line:5}' that hasn't been defined yet | 1 |
| reusing class 'file{name:main.rs, path:src/main.rs}' that hasn't been defined yet | 1 |
| reusing class 'patch{file_name:main.rs, code:fn main() {println!("Hello, world!");}, line:1}' that hasn't been defined yet | 1 |
| 'year:2017' is not a valid int (i64) | 1 |
| 'year:2021' is not a valid int (i64) | 1 |
| 'year:2023' is not a valid int (i64) | 1 |
| dropped as child of line 8 | 1 |
| dropped as child of line 13 | 1 |
| invalid escape sequence '\t' | 1 |

## 6. Analysis of Mistakes & Real Quotes

Below are the primary failure patterns and real quotes observed in run 2:

1. **Escaped Spaces (`\ `):** Models (especially Gemma without thinking) frequently attempted to escape spaces inside string values with backslashes, e.g. `Claim\ 2405.05254`. Struct v0.3 SPEC §3 permits only `\|`, `\\`, and leading `\r`, reporting any other backslash sequence as an error.
2. **Data Inlined in Schema Definitions:** In multi-class code tasks (`s4_two_class_code`), models sometimes generated pseudo-schema lines embedding data values, such as `schema:file{name|src/lib.rs}`.
3. **Missing Schema Headers with Multiple Classes:** When two or more classes are defined outside (`s4_two_class_code`), SPEC §6 mandates that data blocks begin with `schema:class_name`. Llama-3.2-3B emitted data rows immediately without a `schema:` header, triggering `with several outside classes, data before the first schema line is an error`.
4. **Comma Delimiters with Terminal Pipe:** LFM on `s1_flat` occasionally generated comma-separated fields while keeping the terminal pipe: `2405.05254,Rope scaling hurts long ctx,refuted,2024,0|`.
5. **Missing Final Pipe (SafeFix):** LFM on `s1_flat` runs 3-5 produced valid fields but omitted the closing pipe on all records: `2405.05254|Rope scaling hurts long ctx|refuted|2024|0`. `MissingFinalPipe` repaired the pipes, but accuracy was not uniformly 100%. Real per-run results for LFM `s1_flat` (run 1: nothing usable, run 2: nothing usable, run 3: 5 records, 100%, run 4: 5 records, 100%, run 5: 5 records, 80%). Run 5 wrote `Claim 2405.05254` in the id field (copied from the note text) in every record, so its id field is wrong in 5 of 5 records and accuracy is 80%; the doctor cannot fix a wrong value.
6. **Cut by the cap, not repaired:** Mellum `s1_flat` run 2 decoded only 3 of 5 records, outcome `Partial, cut by cap (3/5 records)`. It hit the 1,500-token cap (`stop_type == "limit"`); the output ends after record 3 with a lone trailing `2` (the start of record 4's id), which `Chatter` removed. The repair is real but the 2 missing records were never generated, so this is not a fully repaired run.
7. **Thinking Budget Exhaustion:** On `s3_nested_variable` and `s4_two_class_code` every LFM and Mellum run hit the cap inside the reasoning block (`<think>...`) before any Struct output. Mellum also hit it on 5/5 `s1_flat` and 4/5 `s2_nested_fixed` runs, LFM on 1/5 `s2_nested_fixed`; all but 3 of those 10 cut-off runs (Mellum s1 run 2, 86 struct tokens; Mellum s2 run 5, 3; LFM s2 run 1, 127) emitted no Struct text at all. These are budget artifacts, not format failures.

### Sample Diagnostic Quotes from Models:

- **Problem:** `expected 5 columns, found 6` (from Llama-3.2-3B (thinking off) on `s1_flat` run 1):
  ```text
  ~|Rope scaling hurts long ctx|supported|7|2024|~|
  ```

- **Problem:** `'|2021' is not a valid int (i64)` (from Llama-3.2-3B (thinking off) on `s1_flat` run 1):
  ```text
  ~|LoRA low rank adaptation|supported|\|2021|~|
  ```

- **Problem:** `'2024' is not one of: supported, refuted, unverified` (from Llama-3.2-3B (thinking off) on `s1_flat` run 2):
  ```text
  ~|~Rope scaling hurts long ctx|2024|refuted|~|
  ```

- **Problem:** `'2017' is not one of: supported, refuted, unverified` (from Llama-3.2-3B (thinking off) on `s1_flat` run 2):
  ```text
  ~|~Attention is all you need|2017|supported|1|
  ```

- **Problem:** `'2021' is not one of: supported, refuted, unverified` (from Llama-3.2-3B (thinking off) on `s1_flat` run 2):
  ```text
  ~|~LoRA low rank adaptation|2021|supported|1|
  ```

- **Problem:** `'2023' is not one of: supported, refuted, unverified` (from Llama-3.2-3B (thinking off) on `s1_flat` run 2):
  ```text
  ~|~Mistral 7B architecture|2023|supported|1|
  ```

- **Problem:** `raw value '\r' is not allowed in non-str field` (from Llama-3.2-3B (thinking off) on `s1_flat` run 3):
  ```text
  1. 2405.05254|Rope scaling hurts long ctx|\r published in 2024|refuted|false|
  ```

- **Problem:** `expected 5 columns, found 1` (from Llama-3.2-3B (thinking off) on `s1_flat` run 4):
  ```text
  1. 2405.05254 \| Rope scaling hurts long ctx \| supported \| 7 \| false |
  ```

- **Problem:** `expected 5 columns, found 4` (from Llama-3.2-3B (thinking off) on `s1_flat` run 5):
  ```text
  1. \rRope scaling hurts long ctx|2024|supported|1|
  ```

- **Problem:** `expected 5 columns, found 10` (from Llama-3.2-3B (thinking off) on `s2_nested_fixed` run 1):
  ```text
  \rRope scaling hurts long ctx|supported|\rAttention is all you need|supported|\rLoRA low rank adaptation|supported|\rMistral 7B architecture|supported|\rDeepSeekMoE mixture experts|supported|
  ```

- **Problem:** `expected 5 columns, found 26` (from Llama-3.2-3B (thinking off) on `s2_nested_fixed` run 3):
  ```text
  text|refuted|supported|7|~|text|supported|supported|2017|\rAttention is all you need.|text|supported|2021|\rLoRA low rank adaptation.|text|supported|2023|\rMistral 7B architecture.|text|supported|2024|\rDeepSeekMoE mixture experts.|~|refuted|2405.05254|2401.04088|
  ```

- **Problem:** `invalid escape sequence '\r'` (from Llama-3.2-3B (thinking off) on `s2_nested_fixed` run 4):
  ```text
  \rAttention is all you need|supported|\r\r|1706.03762|true|
  ```

- **Problem:** `'false' is not a valid bool (expected 1 or 0)` (from Llama-3.2-3B (thinking off) on `s2_nested_fixed` run 5):
  ```text
  ~|refuted|Rope scaling hurts long ctx|2024|false|
  ```

- **Problem:** `'true' is not a valid bool (expected 1 or 0)` (from Llama-3.2-3B (thinking off) on `s2_nested_fixed` run 5):
  ```text
  ~|supported|Attention is all you need|2017|true|
  ```

- **Problem:** `expected 4 columns, found 8` (from Llama-3.2-3B (thinking off) on `s3_nested_variable` run 1):
  ```text
  1. id:2405.05254|year:2024|supported|~|text|~|\rRope scaling hurts long ctx\r|~|
  ```

- **Problem:** `expected 4 columns, found 5` (from Llama-3.2-3B (thinking off) on `s3_nested_variable` run 1):
  ```text
    |notes|>  - Note text 'notes are from abstract only', with subnotes 'abstract mentions 32k limit' and 'appendix B clarifies'\r|>>  - Note text 'table 3 contradicts', with subnote 'llama 7b tested only'\r|~|
  ```

- **Problem:** `expected 4 columns, found 6` (from Llama-3.2-3B (thinking off) on `s3_nested_variable` run 1):
  ```text
    |notes|>  - Note text 'tested on GLUE and Wikisql', with subnotes 'rank 4 sufficient' and 'higher ranks show diminishing returns'\r|>>  - Note text 'memory footprint reduced 3x', with subnote 'optimizer state minimal'\r|>>  - Note text 'checkpoint size under 50MB', with no subnotes\r|~|
  ```

- **Problem:** `data line must always end with an unescaped '|'` (from Llama-3.2-3B (thinking off) on `s3_nested_variable` run 2):
  ```text
  5. text|supported|year:2020|id:2005.14165|\rLanguage models few-shot\r|notes:> - Note text 'scaling curve is smooth', with subnotes 'tested up to 175B' and 'in-context learning observed'.>
  ```

- **Problem:** `expected 4 columns, found 9` (from Llama-3.2-3B (thinking off) on `s3_nested_variable` run 3):
  ```text
  2. Claim|supported|2106.09685|2021|Notes:|>  - Note text 'tested on GLUE and Wikisql', with subnotes 'rank 4 sufficient' and 'higher ranks show diminishing returns'.|>  - Note text 'memory footprint reduced 3x', with subnote 'optimizer state minimal'.|>  - Note text 'checkpoint size under 50MB', with no subnotes.|~|
  ```

- **Problem:** `expected 4 columns, found 7` (from Llama-3.2-3B (thinking off) on `s3_nested_variable` run 3):
  ```text
  5. Claim|supported|2005.14165|2020|Notes:|>  - Note text 'scaling curve is smooth', with subnotes 'tested up to 175B' and 'in-context learning observed'.|~|
  ```

- **Problem:** `expected 4 columns, found 3` (from Llama-3.2-3B (thinking off) on `s3_nested_variable` run 5):
  ```text
  |> - Note text 'memory footprint reduced 3x', > |> subnotes 'optimizer state minimal' > |
  ```

- **Problem:** `expected 4 columns, found 2` (from Llama-3.2-3B (thinking off) on `s3_nested_variable` run 5):
  ```text
  |> - Note text 'checkpoint size under 50MB', > |
  ```

- **Problem:** `with several outside classes, data before the first schema line is an error` (from Llama-3.2-3B (thinking off) on `s4_two_class_code` run 1):
  ```text
  name|~|~|
  ```

- **Problem:** `invalid escape sequence '\ '` (from Gemma-4-E2B (thinking off) on `s1_flat` run 1):
  ```text
  id|Claim\ 2405.05254|refuted|2024|0|
  ```

- **Problem:** `expected 5 columns, found 7` (from Gemma-4-E2B (thinking off) on `s2_nested_fixed` run 1):
  ```text
  text|Rope scaling hurts long ctx|refuted|source|2405.05254|2024|false|
  ```

- **Problem:** `expected 3 columns, found 2` (from Gemma-4-E2B (thinking off) on `s4_two_class_code` run 2):
  ```text
  file_name|lib.rs|
  ```

- **Problem:** `reusing class 'file{name:lib.rs, path:src/lib.rs}' that hasn't been defined yet` (from Gemma-4-E2B (thinking off) on `s4_two_class_code` run 4):
  ```text
  schema:file{name:lib.rs, path:src/lib.rs}
  ```

- **Problem:** `reusing class 'patch{file_name:lib.rs, code:pub fn add(a: i32, b: i32) -> i32 {a + b}, line:10}' that hasn't been defined yet` (from Gemma-4-E2B (thinking off) on `s4_two_class_code` run 4):
  ```text
  schema:patch{file_name:lib.rs, code:pub fn add(a: i32, b: i32) -> i32 {a + b}, line:10}
  ```

- **Problem:** `reusing class 'file{name:util.rs, path:src/util.rs}' that hasn't been defined yet` (from Gemma-4-E2B (thinking off) on `s4_two_class_code` run 4):
  ```text
  schema:file{name:util.rs, path:src/util.rs}
  ```

- **Problem:** `reusing class 'patch{file_name:util.rs, code:pub fn greet(name: &str) -> String {format!("Hello, {}!", name)}, line:5}' that hasn't been defined yet` (from Gemma-4-E2B (thinking off) on `s4_two_class_code` run 4):
  ```text
  schema:patch{file_name:util.rs, code:pub fn greet(name: &str) -> String {format!("Hello, {}!", name)}, line:5}
  ```

- **Problem:** `reusing class 'file{name:main.rs, path:src/main.rs}' that hasn't been defined yet` (from Gemma-4-E2B (thinking off) on `s4_two_class_code` run 4):
  ```text
  schema:file{name:main.rs, path:src/main.rs}
  ```

- **Problem:** `reusing class 'patch{file_name:main.rs, code:fn main() {println!("Hello, world!");}, line:1}' that hasn't been defined yet` (from Gemma-4-E2B (thinking off) on `s4_two_class_code` run 4):
  ```text
  schema:patch{file_name:main.rs, code:fn main() {println!("Hello, world!");}, line:1}
  ```

- **Problem:** `'year:2024' is not a valid int (i64)` (from Gemma-4-E2B (thinking on) on `s2_nested_fixed` run 5):
  ```text
  Rope scaling hurts long ctx|refuted|id:2405.05254|year:2024|0|
  ```

- **Problem:** `'year:2017' is not a valid int (i64)` (from Gemma-4-E2B (thinking on) on `s2_nested_fixed` run 5):
  ```text
  Attention is all you need|supported|id:1706.03762|year:2017|1|
  ```

- **Problem:** `'year:2021' is not a valid int (i64)` (from Gemma-4-E2B (thinking on) on `s2_nested_fixed` run 5):
  ```text
  LoRA low rank adaptation|supported|id:2106.09685|year:2021|1|
  ```

- **Problem:** `'year:2023' is not a valid int (i64)` (from Gemma-4-E2B (thinking on) on `s2_nested_fixed` run 5):
  ```text
  Mistral 7B architecture|supported|id:2309.06180|year:2023|1|
  ```

- **Problem:** `dropped as child of line 1` (from Gemma-4-E2B (thinking on) on `s3_nested_variable` run 1):
  ```text
  >text|notes are from abstract only|
  ```

- **Problem:** `expected 4 columns, found 1` (from Gemma-4-E2B (thinking on) on `s3_nested_variable` run 1):
  ```text
  ]|
  ```

- **Problem:** `dropped as child of line 7` (from Gemma-4-E2B (thinking on) on `s3_nested_variable` run 1):
  ```text
  >text|tested on GLUE and Wikisql|
  ```

- **Problem:** `dropped as child of line 14` (from Gemma-4-E2B (thinking on) on `s3_nested_variable` run 1):
  ```text
  >text|outlier channels preserved|
  ```

- **Problem:** `dropped as child of line 18` (from Gemma-4-E2B (thinking on) on `s3_nested_variable` run 1):
  ```text
  >text|optimal ratio is 20 to 1|
  ```

- **Problem:** `dropped as child of line 24` (from Gemma-4-E2B (thinking on) on `s3_nested_variable` run 1):
  ```text
  >text|scaling curve is smooth|
  ```

- **Problem:** `dropped as child of line 4` (from Gemma-4-E2B (thinking on) on `s3_nested_variable` run 2):
  ```text
  > tested on GLUE and Wikisql|>> rank 4 sufficient|>> higher ranks show diminishing returns|
  ```

- **Problem:** `dropped as child of line 8` (from Gemma-4-E2B (thinking on) on `s3_nested_variable` run 2):
  ```text
  > outlier channels preserved|>> per-head scales help|>> perplexity intact|
  ```

- **Problem:** `dropped as child of line 10` (from Gemma-4-E2B (thinking on) on `s3_nested_variable` run 2):
  ```text
  > optimal ratio is 20 to 1|>> tested across 400 models|>> cosine schedule used|
  ```

- **Problem:** `dropped as child of line 13` (from Gemma-4-E2B (thinking on) on `s3_nested_variable` run 2):
  ```text
  > scaling curve is smooth|>> tested up to 175B|>> in-context learning observed|
  ```

- **Problem:** `invalid escape sequence '\t'` (from Gemma-4-E2B (thinking on) on `s3_nested_variable` run 4):
  ```text
  Language models few-shot|supported|record{id:2005.14165, year:2020}|notes:[record{text:\tscaling curve is smooth,subnotes:[tested up to 175B,in-context learning observed]}]|
  ```


## 7. Complete Per-Run Results Table

| Model | Schema | Run | Seed | Outcome | Records | Repairs | Bad Lines | Think Tok | Out Tok | Cap Hit | Field Acc |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Llama-3.2-3B (thinking off) | `s1_flat` | 1 | 42 | Nothing usable | 0 | 0 | 5 | 0 | 88 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s1_flat` | 2 | 43 | Nothing usable | 0 | 0 | 5 | 0 | 85 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s1_flat` | 3 | 44 | Nothing usable | 0 | 0 | 5 | 0 | 133 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s1_flat` | 4 | 45 | Nothing usable | 0 | 1 | 4 | 0 | 116 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s1_flat` | 5 | 46 | Nothing usable | 0 | 1 | 4 | 0 | 89 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s2_nested_fixed` | 1 | 42 | Nothing usable | 0 | 0 | 1 | 0 | 49 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s2_nested_fixed` | 2 | 43 | Nothing usable | 0 | 1 | 1 | 0 | 1500 | Yes | 0.0% |
| Llama-3.2-3B (thinking off) | `s2_nested_fixed` | 3 | 44 | Nothing usable | 0 | 0 | 1 | 0 | 89 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s2_nested_fixed` | 4 | 45 | Nothing usable | 0 | 0 | 6 | 0 | 112 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s2_nested_fixed` | 5 | 46 | Nothing usable | 0 | 0 | 5 | 0 | 78 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s3_nested_variable` | 1 | 42 | Nothing usable | 0 | 0 | 10 | 0 | 495 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s3_nested_variable` | 2 | 43 | Nothing usable | 0 | 0 | 5 | 0 | 381 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s3_nested_variable` | 3 | 44 | Nothing usable | 0 | 0 | 5 | 0 | 362 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s3_nested_variable` | 4 | 45 | Nothing usable | 0 | 0 | 5 | 0 | 227 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s3_nested_variable` | 5 | 46 | Nothing usable | 0 | 0 | 8 | 0 | 319 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s4_two_class_code` | 1 | 42 | Nothing usable | 0 | 0 | 10 | 0 | 122 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s4_two_class_code` | 2 | 43 | Nothing usable | 0 | 4 | 11 | 0 | 160 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s4_two_class_code` | 3 | 44 | Nothing usable | 0 | 0 | 7 | 0 | 129 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s4_two_class_code` | 4 | 45 | Nothing usable | 0 | 0 | 13 | 0 | 156 | No | 0.0% |
| Llama-3.2-3B (thinking off) | `s4_two_class_code` | 5 | 46 | Nothing usable | 0 | 0 | 5 | 0 | 108 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s1_flat` | 1 | 42 | Nothing usable | 0 | 0 | 5 | 0 | 130 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s1_flat` | 2 | 43 | Nothing usable | 0 | 0 | 5 | 0 | 130 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s1_flat` | 3 | 44 | Nothing usable | 0 | 0 | 5 | 0 | 130 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s1_flat` | 4 | 45 | Clean as written | 5 | 0 | 0 | 0 | 125 | No | 60.0% |
| Gemma-4-E2B (thinking off) | `s1_flat` | 5 | 46 | Clean as written | 5 | 0 | 0 | 0 | 125 | No | 60.0% |
| Gemma-4-E2B (thinking off) | `s2_nested_fixed` | 1 | 42 | Nothing usable | 0 | 0 | 5 | 0 | 158 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s2_nested_fixed` | 2 | 43 | Nothing usable | 0 | 0 | 5 | 0 | 158 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s2_nested_fixed` | 3 | 44 | Nothing usable | 0 | 0 | 5 | 0 | 158 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s2_nested_fixed` | 4 | 45 | Nothing usable | 0 | 0 | 5 | 0 | 158 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s2_nested_fixed` | 5 | 46 | Nothing usable | 0 | 0 | 5 | 0 | 158 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s3_nested_variable` | 1 | 42 | Nothing usable | 0 | 0 | 5 | 0 | 429 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s3_nested_variable` | 2 | 43 | Nothing usable | 0 | 0 | 5 | 0 | 429 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s3_nested_variable` | 3 | 44 | Nothing usable | 0 | 0 | 5 | 0 | 429 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s3_nested_variable` | 4 | 45 | Nothing usable | 0 | 0 | 5 | 0 | 429 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s3_nested_variable` | 5 | 46 | Nothing usable | 0 | 0 | 5 | 0 | 442 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s4_two_class_code` | 1 | 42 | Nothing usable | 0 | 0 | 6 | 0 | 158 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s4_two_class_code` | 2 | 43 | Salvaged (1 records) | 1 | 1 | 8 | 0 | 144 | No | 50.0% |
| Gemma-4-E2B (thinking off) | `s4_two_class_code` | 3 | 44 | Salvaged (1 records) | 1 | 1 | 8 | 0 | 144 | No | 50.0% |
| Gemma-4-E2B (thinking off) | `s4_two_class_code` | 4 | 45 | Nothing usable | 0 | 0 | 6 | 0 | 170 | No | 0.0% |
| Gemma-4-E2B (thinking off) | `s4_two_class_code` | 5 | 46 | Nothing usable | 0 | 0 | 6 | 0 | 179 | No | 0.0% |
| Gemma-4-E2B (thinking on) | `s1_flat` | 1 | 42 | Clean as written | 5 | 0 | 0 | 973 | 138 | No | 100.0% |
| Gemma-4-E2B (thinking on) | `s1_flat` | 2 | 43 | Clean as written | 5 | 0 | 0 | 981 | 148 | No | 80.0% |
| Gemma-4-E2B (thinking on) | `s1_flat` | 3 | 44 | Clean as written | 5 | 0 | 0 | 1063 | 138 | No | 100.0% |
| Gemma-4-E2B (thinking on) | `s1_flat` | 4 | 45 | Clean as written | 5 | 0 | 0 | 931 | 138 | No | 100.0% |
| Gemma-4-E2B (thinking on) | `s1_flat` | 5 | 46 | Clean as written | 5 | 0 | 0 | 944 | 138 | No | 100.0% |
| Gemma-4-E2B (thinking on) | `s2_nested_fixed` | 1 | 42 | Nothing usable | 0 | 1 | 4 | 853 | 168 | No | 0.0% |
| Gemma-4-E2B (thinking on) | `s2_nested_fixed` | 2 | 43 | Clean as written | 5 | 0 | 0 | 990 | 138 | No | 100.0% |
| Gemma-4-E2B (thinking on) | `s2_nested_fixed` | 3 | 44 | Clean as written | 5 | 0 | 0 | 1013 | 138 | No | 100.0% |
| Gemma-4-E2B (thinking on) | `s2_nested_fixed` | 4 | 45 | Clean as written | 5 | 0 | 0 | 1124 | 138 | No | 100.0% |
| Gemma-4-E2B (thinking on) | `s2_nested_fixed` | 5 | 46 | Nothing usable | 0 | 0 | 5 | 912 | 158 | No | 0.0% |
| Gemma-4-E2B (thinking on) | `s3_nested_variable` | 1 | 42 | Nothing usable | 0 | 1 | 26 | 787 | 353 | No | 0.0% |
| Gemma-4-E2B (thinking on) | `s3_nested_variable` | 2 | 43 | Nothing usable | 0 | 0 | 14 | 1152 | 309 | No | 0.0% |
| Gemma-4-E2B (thinking on) | `s3_nested_variable` | 3 | 44 | Nothing usable | 0 | 0 | 5 | 801 | 472 | No | 0.0% |
| Gemma-4-E2B (thinking on) | `s3_nested_variable` | 4 | 45 | Nothing usable | 0 | 0 | 5 | 868 | 417 | No | 0.0% |
| Gemma-4-E2B (thinking on) | `s3_nested_variable` | 5 | 46 | Nothing usable | 0 | 0 | 5 | 768 | 355 | No | 0.0% |
| Gemma-4-E2B (thinking on) | `s4_two_class_code` | 1 | 42 | Nothing usable | 0 | 0 | 6 | 729 | 152 | No | 0.0% |
| Gemma-4-E2B (thinking on) | `s4_two_class_code` | 2 | 43 | Nothing usable | 0 | 0 | 3 | 918 | 101 | No | 0.0% |
| Gemma-4-E2B (thinking on) | `s4_two_class_code` | 3 | 44 | Nothing usable | 0 | 0 | 6 | 802 | 188 | No | 0.0% |
| Gemma-4-E2B (thinking on) | `s4_two_class_code` | 4 | 45 | Nothing usable | 0 | 0 | 6 | 853 | 173 | No | 0.0% |
| Gemma-4-E2B (thinking on) | `s4_two_class_code` | 5 | 46 | Nothing usable | 0 | 0 | 6 | 804 | 152 | No | 0.0% |
| LFM2.5-8B (thinking on) | `s1_flat` | 1 | 42 | Nothing usable | 0 | 1 | 4 | 1187 | 102 | No | 0.0% |
| LFM2.5-8B (thinking on) | `s1_flat` | 2 | 43 | Nothing usable | 0 | 1 | 4 | 802 | 123 | No | 0.0% |
| LFM2.5-8B (thinking on) | `s1_flat` | 3 | 44 | Fully repaired | 5 | 5 | 0 | 1115 | 107 | No | 100.0% |
| LFM2.5-8B (thinking on) | `s1_flat` | 4 | 45 | Fully repaired | 5 | 5 | 0 | 706 | 107 | No | 100.0% |
| LFM2.5-8B (thinking on) | `s1_flat` | 5 | 46 | Fully repaired | 5 | 5 | 0 | 760 | 122 | No | 80.0% |
| LFM2.5-8B (thinking on) | `s2_nested_fixed` | 1 | 42 | Nothing usable | 0 | 1 | 3 | 1373 | 127 | Yes | 0.0% |
| LFM2.5-8B (thinking on) | `s2_nested_fixed` | 2 | 43 | Nothing usable | 0 | 0 | 5 | 1055 | 107 | No | 0.0% |
| LFM2.5-8B (thinking on) | `s2_nested_fixed` | 3 | 44 | Nothing usable | 0 | 0 | 5 | 904 | 107 | No | 0.0% |
| LFM2.5-8B (thinking on) | `s2_nested_fixed` | 4 | 45 | Nothing usable | 0 | 1 | 4 | 1230 | 132 | No | 0.0% |
| LFM2.5-8B (thinking on) | `s2_nested_fixed` | 5 | 46 | Fully repaired | 5 | 5 | 0 | 1069 | 107 | No | 100.0% |
| LFM2.5-8B (thinking on) | `s3_nested_variable` | 1 | 42 | Nothing usable | 0 | 0 | 0 | 1499 | 0 | Yes | 0.0% |
| LFM2.5-8B (thinking on) | `s3_nested_variable` | 2 | 43 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| LFM2.5-8B (thinking on) | `s3_nested_variable` | 3 | 44 | Nothing usable | 0 | 0 | 0 | 1501 | 0 | Yes | 0.0% |
| LFM2.5-8B (thinking on) | `s3_nested_variable` | 4 | 45 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| LFM2.5-8B (thinking on) | `s3_nested_variable` | 5 | 46 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| LFM2.5-8B (thinking on) | `s4_two_class_code` | 1 | 42 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| LFM2.5-8B (thinking on) | `s4_two_class_code` | 2 | 43 | Nothing usable | 0 | 0 | 0 | 1499 | 0 | Yes | 0.0% |
| LFM2.5-8B (thinking on) | `s4_two_class_code` | 3 | 44 | Nothing usable | 0 | 0 | 0 | 1499 | 0 | Yes | 0.0% |
| LFM2.5-8B (thinking on) | `s4_two_class_code` | 4 | 45 | Nothing usable | 0 | 0 | 0 | 1499 | 0 | Yes | 0.0% |
| LFM2.5-8B (thinking on) | `s4_two_class_code` | 5 | 46 | Nothing usable | 0 | 0 | 0 | 1497 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s1_flat` | 1 | 42 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s1_flat` | 2 | 43 | Partial, cut by cap (3/5 records) | 3 | 1 | 0 | 1414 | 86 | Yes | 100.0% |
| Mellum2.1-12B (thinking on) | `s1_flat` | 3 | 44 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s1_flat` | 4 | 45 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s1_flat` | 5 | 46 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s2_nested_fixed` | 1 | 42 | Clean as written | 5 | 0 | 0 | 1352 | 141 | No | 100.0% |
| Mellum2.1-12B (thinking on) | `s2_nested_fixed` | 2 | 43 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s2_nested_fixed` | 3 | 44 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s2_nested_fixed` | 4 | 45 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s2_nested_fixed` | 5 | 46 | Nothing usable | 0 | 1 | 0 | 1496 | 3 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s3_nested_variable` | 1 | 42 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s3_nested_variable` | 2 | 43 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s3_nested_variable` | 3 | 44 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s3_nested_variable` | 4 | 45 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s3_nested_variable` | 5 | 46 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s4_two_class_code` | 1 | 42 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s4_two_class_code` | 2 | 43 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s4_two_class_code` | 3 | 44 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s4_two_class_code` | 4 | 45 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |
| Mellum2.1-12B (thinking on) | `s4_two_class_code` | 5 | 46 | Nothing usable | 0 | 0 | 0 | 1500 | 0 | Yes | 0.0% |

## 8. Limits of This Run

- **Token cap dominates:** 31/100 runs hit the 1,500-token cap (Mellum 19/20, LFM 11/20, Llama 1/20, Gemma 0/40). Cap-hit runs are budget artifacts. See the Cap Hits column in section 3.
- **Low output diversity:** Gemma (thinking off) produced identical raw output across most seeds (see Distinct Outputs; e.g. 1 distinct output in 5 seeds on `s2_nested_fixed`), so its 5 runs are not 5 independent samples.
- **Templated task notes with quotes:** notes were written like `title 'Rope scaling hurts long ctx'`, and models copied the single quotes (and the `Claim` prefix) into values. Some value errors come from the note template, not from the format.
- **Unspecified thinking prefills:** Gemma (thinking on) was run with `<|think|>` in the system turn plus a `<|channel>thought\n` prefill, and Mellum with a `<think>\n` prefill. The brief did not specify these. LFM had no prefill.
- **Token counts are not server counts:** thinking and output token counts come from `/tokenize` on the split text, not the server's generation counts.
- **Accuracy is positional:** field accuracy compares decoded records to ground truth by position; a skipped record shifts later ones.

