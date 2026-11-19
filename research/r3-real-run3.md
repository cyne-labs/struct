# Milestone R3d: Struct Doctor Real-Model Output Report (Run 3)

Empirical evaluation of **Struct Doctor** (`structl::doctor`) across **100 real model outputs** (5 model configurations × 4 schemas × 5 seeds: 42 to 46). Run 3 evaluates models under authentic deployment conditions: standard `POST /v1/chat/completions` endpoint, native Jinja chat templates without manual prompt crafting or prefills, unconstrained generation budget (128k/131k context window with turboquant k4/v3 KV cache, no `max_tokens` or `n_predict` cap), natural sentence task descriptions, and explicit extraction of thinking tokens via server `reasoning_content`.

## 1. Experimental Setup & Environment

- **Hardware:** NVIDIA GeForce RTX 4060 Laptop GPU (8,188 MiB VRAM, 80W TGP), Windows 11.
- **Inference Server:** `llama-server.exe` from `C:\Users\Ashman Das\Documents\turboquant-llama-cpp`, bound to `127.0.0.1:8090`.
- **Endpoint:** `POST /v1/chat/completions` with a single user message containing the full prompt (`prompt(schemas)`, blank line, natural notes, and `Write these as Struct records.`). No assistant prefills, no manual turn headers.
- **Context Size & KV Cache:** 131,072 tokens for Llama-3.2-3B and Gemma-4-E2B; 128,000 tokens for LFM2.5-8B and Mellum2.1-12B. Quantized KV cache with `--cache-type-k turbo4 --cache-type-v turbo3`, `--mlock`, `--no-mmap`, and Flash Attention (`-fa on`).
- **Sampling Parameters:** Temperature 0.7, seeds 42 to 46 per schema, no output token cap (`max_tokens` and `n_predict` omitted). Safety net timeout of 1,200 seconds (20 minutes).
- **Thinking & Token Accounting:** Thinking traces extracted from the server's separate `reasoning_content` field in `choices[0].message`. Since server `usage` does not split thinking from content completion tokens, exact token counts for thinking and Struct output are measured via `/tokenize`.
- **Doctor Mode:** Salvage mode (`salvage true`) via `target/debug/doctor_tool.exe`.
- **Models Evaluated (5 configurations):**
  1. `Llama-3.2-3B-Instruct` (thinking OFF)
  2. `gemma-4-E2B-it` (thinking OFF, `chat_template_kwargs: {"enable_thinking": false}`)
  3. `gemma-4-E2B-it` (thinking ON, `chat_template_kwargs: {"enable_thinking": true}`)
  4. `LFM2.5-8B-A1B` (thinking ON, template default)
  5. `Mellum2.1-12B-A2.5B-Thinking` (thinking ON, template default, `--n-cpu-moe 4`)

## 2. Server Launch Commands & Flag Verification

All models were launched sequentially following Rule 7 (one heavy job at a time, checking `nvidia-smi` first, confirming server termination). The exact commands used were:

```bash
# Llama-3.2-3B
llama-server.exe --host 127.0.0.1 --port 8090 -m models\lmstudio-community\Llama-3.2-3B-Instruct-GGUF\Llama-3.2-3B-Instruct-Q4_K_M.gguf -ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 131072

# Gemma-4-E2B (runs both thinking off and thinking on)
llama-server.exe --host 127.0.0.1 --port 8090 -m models\lmstudio-community\gemma-4-E2B-it-GGUF\gemma-4-E2B-it-Q4_K_M.gguf -ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 131072

# LFM2.5-8B-A1B
llama-server.exe --host 127.0.0.1 --port 8090 -m "C:\Users\Ashman Das\Downloads\model file\LFM2.5-8B-A1B-Q4_K_M.gguf" -ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 128000

# Mellum2.1-12B-A2.5B-Thinking
llama-server.exe --host 127.0.0.1 --port 8090 -m "C:\Users\Ashman Das\Downloads\model file\Mellum2.1-12B-A2.5B-Thinking-Q4_K_M.gguf" -ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 128000 --n-cpu-moe 4
```

> **Jinja Flag Verification:** Inspection of `llama-server.exe --help` confirms that `--jinja` is enabled by default in this build (`--jinja, --no-jinja whether to use jinja template engine for chat (default: enabled)`). No explicit `--jinja` flag was necessary.

## 3. Gemma Thinking Switch & Template Verification

In Gemma-4-E2B's embedded Jinja chat template (`tokenizer.chat_template` in the GGUF metadata), the variable controlling reasoning is `enable_thinking`. The exact lines from the template are:

```jinja
{%- if (enable_thinking is defined and enable_thinking) or tools or messages[0]['role'] in ['system', 'developer'] -%}
    {{- '<|turn>system\n' -}}

    {#- Inject Thinking token at the very top of the FIRST system turn -#}
    {%- if enable_thinking is defined and enable_thinking -%}
        {{- '<|think|>\n' -}}
        {%- set ns.prev_message_type = 'think' -%}
    {%- endif -%}
```

In `collect_run3.py`, this switch is passed in the request payload as `chat_template_kwargs: {"enable_thinking": False}` for `gemma_off`, and `chat_template_kwargs: {"enable_thinking": True}` for `gemma_on`.

## 4. Prompts Used for Each Schema

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

1. Paper 2405.05254 was published in 2024 with the title "Rope scaling hurts long ctx"; it turned out to be false, so the claim is refuted.
2. In 2017, paper 1706.03762 was published with the title "Attention is all you need"; verification was true, and the claim is supported.
3. Paper 2106.09685, titled "LoRA low rank adaptation", appeared in 2021; it was verified as true, so the claim is supported.
4. Titled "Mistral 7B architecture", paper 2309.06180 was published in 2023; verification was true, and the claim is supported.
5. Paper 2401.04088 was published in 2024 under the title "DeepSeekMoE mixture experts"; it was verified as true, and the claim is supported.
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

1. Paper 2405.05254 claimed "Rope scaling hurts long ctx" in 2024; it turned out to be false, so the claim is refuted.
2. In 2017, paper 1706.03762 claimed "Attention is all you need"; verification was true, so the claim is supported.
3. Paper 2106.09685 made the claim "LoRA low rank adaptation" in 2021; it was verified as true, and the claim is supported.
4. In 2023, paper 2309.06180 claimed "Mistral 7B architecture"; verification was true, and the claim is supported.
5. Paper 2401.04088 made the claim "DeepSeekMoE mixture experts" in 2024; it was verified as true, and the claim is supported.
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

1. Paper 2405.05254 claimed "Rope scaling hurts long ctx" in 2024, but the claim was refuted. A note says "notes are from abstract only", with two subnotes: "abstract mentions 32k limit" and "appendix B clarifies". A second note says "table 3 contradicts", with the subnote "llama 7b tested only".
2. In 2021, paper 2106.09685 made the claim "LoRA matches full finetune", and the verdict is supported. The first note is "tested on GLUE and Wikisql", with subnotes "rank 4 sufficient" and "higher ranks show diminishing returns". The second note is "memory footprint reduced 3x", with the subnote "optimizer state minimal". The third note is "checkpoint size under 50MB", without any subnotes.
3. Paper 2406.12345 made the claim "KV cache quant loses 2x acc" in 2024, but the verdict is refuted. It has a note "outlier channels preserved", with subnotes "per-head scales help" and "perplexity intact".
4. Published in 2022, paper 2203.15556 claimed "Chinchilla tokens beat params", which is supported. One note states "optimal ratio is 20 to 1", with subnotes "tested across 400 models" and "cosine schedule used". Another note states "kaplan tokens were suboptimal", with the subnote "fixed learning rate flawed".
5. Paper 2005.14165 from 2020 claimed "Language models few-shot", and the claim is supported. There is a note "scaling curve is smooth", with subnotes "tested up to 175B" and "in-context learning observed".
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

1. The file "lib.rs" is at path "src/lib.rs". At line 10, a patch for "lib.rs" has the code:
"pub fn add(a: i32, b: i32) -> i32 {
    a + b
}"
2. The file "util.rs" is at path "src/util.rs". At line 5, a patch for "util.rs" has the code:
"pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}"
3. The file "main.rs" is at path "src/main.rs". At line 1, a patch for "main.rs" has the code:
"fn main() {
    println!("Hello, world!");
}"
Write these as Struct records.
```

## 5. 5×4 Summary Matrix

Definitions:
- **Record Recall:** Total decoded records / total expected records across 5 runs.
- **Field Accuracy (Decoded):** Exact-match field accuracy pooled strictly over decoded records; `n/a` if no records decoded.
- **Outcomes:** Clean (valid without repairs), Repaired (fully valid after safe repairs), Salvaged (valid records recovered alongside reported bad lines), Partial (clean/repaired structure but incomplete record count), Unusable (0 records decoded).
- **Distinct Outputs:** Count of distinct raw outputs across 5 seeds (based on SHA256 digest).
- **finish_reason:** Server completion termination status (e.g. `stop`, `length`, `timeout`).

| Model | Schema | Runs | Clean | Repaired | Salvaged | Partial | Unusable | Record Recall | Field Accuracy | Distinct | finish_reason | Mean Think Tok | Max Think Tok | Mean Out Tok | Max Out Tok |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Llama-3.2-3B (thinking off) | `s1_flat` | 5 | 4 | 0 | 0 | 0 | 1 | 80.0% (20/25) | 70.0% | 3/5 | stop: 5 | 0 | 0 | 78 | 80 |
| Llama-3.2-3B (thinking off) | `s2_nested_fixed` | 5 | 0 | 0 | 2 | 0 | 3 | 40.0% (10/25) | 32.0% | 5/5 | stop: 5 | 0 | 0 | 116 | 196 |
| Llama-3.2-3B (thinking off) | `s3_nested_variable` | 5 | 0 | 0 | 1 | 0 | 4 | 8.0% (2/25) | 0.0% | 5/5 | stop: 5 | 0 | 0 | 647 | 1133 |
| Llama-3.2-3B (thinking off) | `s4_two_class_code` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0% (0/30) | n/a | 5/5 | stop: 5 | 0 | 0 | 135 | 280 |
| Gemma-4-E2B (thinking off) | `s1_flat` | 5 | 0 | 0 | 5 | 0 | 0 | 100.0% (25/25) | 96.0% | 2/5 | stop: 5 | 0 | 0 | 149 | 150 |
| Gemma-4-E2B (thinking off) | `s2_nested_fixed` | 5 | 0 | 0 | 4 | 0 | 1 | 16.0% (4/25) | 40.0% | 3/5 | stop: 5 | 0 | 0 | 140 | 145 |
| Gemma-4-E2B (thinking off) | `s3_nested_variable` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0% (0/25) | n/a | 5/5 | stop: 5 | 0 | 0 | 413 | 455 |
| Gemma-4-E2B (thinking off) | `s4_two_class_code` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0% (0/30) | n/a | 5/5 | stop: 5 | 0 | 0 | 111 | 143 |
| Gemma-4-E2B (thinking on) | `s1_flat` | 5 | 4 | 1 | 0 | 0 | 0 | 100.0% (25/25) | 88.8% | 4/5 | stop: 5 | 881 | 1217 | 128 | 138 |
| Gemma-4-E2B (thinking on) | `s2_nested_fixed` | 5 | 1 | 3 | 0 | 0 | 1 | 80.0% (20/25) | 97.0% | 5/5 | stop: 5 | 1116 | 1267 | 151 | 188 |
| Gemma-4-E2B (thinking on) | `s3_nested_variable` | 5 | 0 | 0 | 1 | 0 | 4 | 20.0% (5/25) | 45.2% | 5/5 | stop: 5 | 1232 | 1632 | 417 | 503 |
| Gemma-4-E2B (thinking on) | `s4_two_class_code` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0% (0/30) | n/a | 2/5 | stop: 5 | 161 | 805 | 95 | 201 |
| LFM2.5-8B (thinking on) | `s1_flat` | 5 | 2 | 2 | 0 | 0 | 1 | 80.0% (20/25) | 76.0% | 5/5 | stop: 5 | 1051 | 1461 | 108 | 118 |
| LFM2.5-8B (thinking on) | `s2_nested_fixed` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0% (0/25) | n/a | 5/5 | stop: 5 | 2121 | 2807 | 243 | 356 |
| LFM2.5-8B (thinking on) | `s3_nested_variable` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0% (0/25) | n/a | 5/5 | stop: 5 | 3214 | 3822 | 419 | 509 |
| LFM2.5-8B (thinking on) | `s4_two_class_code` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0% (0/30) | n/a | 5/5 | stop: 5 | 2942 | 4238 | 175 | 283 |
| Mellum2.1-12B (thinking on) | `s1_flat` | 5 | 5 | 0 | 0 | 0 | 0 | 100.0% (25/25) | 92.0% | 1/5 | stop: 5 | 1777 | 2280 | 139 | 139 |
| Mellum2.1-12B (thinking on) | `s2_nested_fixed` | 5 | 5 | 0 | 0 | 0 | 0 | 100.0% (25/25) | 92.8% | 2/5 | stop: 5 | 3035 | 3915 | 139 | 139 |
| Mellum2.1-12B (thinking on) | `s3_nested_variable` | 5 | 2 | 0 | 1 | 0 | 2 | 60.0% (15/25) | 78.6% | 5/5 | stop: 5 | 21624 | 26689 | 339 | 381 |
| Mellum2.1-12B (thinking on) | `s4_two_class_code` | 5 | 0 | 0 | 2 | 0 | 3 | 20.0% (6/30) | 100.0% | 5/5 | stop: 5 | 11430 | 16584 | 147 | 154 |

## 6. Finish Reasons & Timeouts

- **Total runs:** 100
- **`stop`:** 100 runs
- **Timeouts (> 1,200s):** 0 runs

## 7. Doctor Repairs Analysis

| Fix Type | Occurrences | Percentage of Repairs |
|---|---|---|
| `Chatter` | 32 | 37.6% |
| `MissingFinalPipe` | 28 | 32.9% |
| `CutOffLastRecord` | 21 | 24.7% |
| `EchoedSchema` | 4 | 4.7% |
| **Total Repairs** | **85** | **100.0%** |

## 8. Doctor Bad Lines Analysis

| Problem Reported | Occurrences | Percentage |
|---|---|---|
| `with several outside classes, data before the first schema line is an error` | 144 | 28.9% |
| `data line must always end with an unescaped '|'` | 60 | 12.0% |
| `expected 4 columns, found 5` | 23 | 4.6% |
| `dropped as child of line 6` | 21 | 4.2% |
| `expected 5 columns, found 6` | 20 | 4.0% |
| `invalid escape sequence '\r'` | 20 | 4.0% |
| `expected 5 columns, found 4` | 17 | 3.4% |
| `dropped as child of line 18` | 12 | 2.4% |
| `dropped as child of line 7` | 12 | 2.4% |
| `expected 4 columns, found 8` | 10 | 2.0% |
| `dropped as child of line 1` | 10 | 2.0% |
| `expected 4 columns, found 13` | 9 | 1.8% |
| `expected 1 columns, found 2` | 9 | 1.8% |
| `empty field is not allowed (use ~ for null)` | 7 | 1.4% |
| `expected 4 columns, found 10` | 7 | 1.4% |
| `expected 4 columns, found 9` | 7 | 1.4% |
| `expected 4 columns, found 7` | 6 | 1.2% |
| `reusing class 'name' that hasn't been defined yet` | 6 | 1.2% |
| `dropped as child of line 14` | 6 | 1.2% |
| `dropped as child of line 24` | 6 | 1.2% |
| `invalid escape sequence '\n'` | 6 | 1.2% |
| `expected 4 columns, found 6` | 5 | 1.0% |
| `expected 5 columns, found 2` | 4 | 0.8% |
| `expected 4 columns, found 11` | 4 | 0.8% |
| `'verdict' is not one of: supported, refuted, unverified` | 4 | 0.8% |
| `expected 5 columns, found 1` | 4 | 0.8% |
| `'>' is not one of: supported, refuted, unknown` | 3 | 0.6% |
| `expected 4 columns, found 17` | 3 | 0.6% |
| `expected 4 columns, found 12` | 2 | 0.4% |
| `expected 4 columns, found 37` | 2 | 0.4% |
| `expected 4 columns, found 31` | 2 | 0.4% |
| `'text' is not one of: supported, refuted, unknown` | 2 | 0.4% |
| `dropped as child of line 3` | 2 | 0.4% |
| `dropped as child of line 10` | 2 | 0.4% |
| `dropped as child of line 23` | 2 | 0.4% |
| `dropped as child of line 30` | 2 | 0.4% |
| `dropped as child of line 2` | 2 | 0.4% |
| `dropped as child of line 8` | 2 | 0.4% |
| `dropped as child of line 15` | 2 | 0.4% |
| `dropped as child of line 19` | 2 | 0.4% |
| `dropped as child of line 25` | 2 | 0.4% |
| `expected 5 columns, found 3` | 1 | 0.2% |
| `'year' is not a valid int (i64)` | 1 | 0.2% |
| `expected 4 columns, found 3` | 1 | 0.2% |
| `expected 4 columns, found 47` | 1 | 0.2% |
| `'1706.03762' is not a valid int (i64)` | 1 | 0.2% |
| `'2106.09685' is not a valid int (i64)` | 1 | 0.2% |
| `'2309.06180' is not a valid int (i64)` | 1 | 0.2% |
| `'2401.04088' is not a valid int (i64)` | 1 | 0.2% |
| `dropped as child of line 13` | 1 | 0.2% |
| `dropped as child of line 26` | 1 | 0.2% |
| `expected 4 columns, found 21` | 1 | 0.2% |
| `'2106.09685,"LoRA low rank adaptation",supported,2021,1' is not one of: supported, refuted, unverified` | 1 | 0.2% |
| `reusing class 'name lib.rs' that hasn't been defined yet` | 1 | 0.2% |
| `reusing class 'file name="src/lib.rs" path="lib.rs"' that hasn't been defined yet` | 1 | 0.2% |
| `reusing class 'code 'pub fn add(a: i32, b: i32) -> i32\n    a + b'' that hasn't been defined yet` | 1 | 0.2% |
| `reusing class 'line 10' that hasn't been defined yet` | 1 | 0.2% |
| `reusing class 'name util.rs' that hasn't been defined yet` | 1 | 0.2% |
| `reusing class 'file name="src/util.rs" path="util.rs"' that hasn't been defined yet` | 1 | 0.2% |
| `reusing class 'code 'pub fn greet(name: &str) -> String {\n    format!("Hello, {}!", name)'' that hasn't been defined yet` | 1 | 0.2% |
| `reusing class 'line 5' that hasn't been defined yet` | 1 | 0.2% |
| `reusing class 'name main.rs' that hasn't been defined yet` | 1 | 0.2% |
| `reusing class 'file name="src/main.rs" path="main.rs"' that hasn't been defined yet` | 1 | 0.2% |
| `reusing class 'code 'fn main() {\n    println!("Hello, world!");}'' that hasn't been defined yet` | 1 | 0.2% |
| `reusing class 'line 1' that hasn't been defined yet` | 1 | 0.2% |
| `dropped as child of line 5` | 1 | 0.2% |
| `dropped as child of line 11` | 1 | 0.2% |
| `dropped as child of line 22` | 1 | 0.2% |
| **Total Bad Lines** | **498** | **100.0%** |

## 9. New Kinds of Mistakes (with Real Quotes)

### Problem: `expected 5 columns, found 4`
- **Llama-3.2-3B (thinking off)** on `s1_flat` (run 4, line 1):
  ```text
  ~|Rope scaling hurts long ctx|supported|2024|
  ```
- **Llama-3.2-3B (thinking off)** on `s1_flat` (run 4, line 2):
  ```text
  ~|Attention is all you need|supported|2017|
  ```
- **Llama-3.2-3B (thinking off)** on `s1_flat` (run 4, line 4):
  ```text
  ~|Mistral 7B architecture|supported|2023|
  ```

### Problem: `expected 5 columns, found 3`
- **Llama-3.2-3B (thinking off)** on `s1_flat` (run 4, line 3):
  ```text
  \r\r\r\rLoRA low rank adaptation\r|supported|2021|
  ```

### Problem: `data line must always end with an unescaped '|'`
- **Llama-3.2-3B (thinking off)** on `s2_nested_fixed` (run 1, line 1):
  ```text
  \rPaper 2405.05254\r|supported\r|\r2024\r|~\r
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 1, line 2):
  ```text
  ~|~|refuted|~|~|~|~|> table 3 contradicts|> >> llama 7b tested only|>
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 1, line 18):
  ```text
  ~|~|refuted|~|~|~|~|> Rope scaling hurts long ctx|> >> table 3 contradicts|> >> llama 7b tested only|>
  ```

### Problem: `'year' is not a valid int (i64)`
- **Llama-3.2-3B (thinking off)** on `s2_nested_fixed` (run 2, line 1):
  ```text
  text|supported|text|year|verified|
  ```

### Problem: `expected 5 columns, found 6`
- **Llama-3.2-3B (thinking off)** on `s2_nested_fixed` (run 3, line 1):
  ```text
  ~|text|Rope scaling hurts long ctx|2024|~|refuted|
  ```
- **Llama-3.2-3B (thinking off)** on `s2_nested_fixed` (run 3, line 2):
  ```text
  supported|text|Attention is all you need|2017|~|supported|
  ```
- **Llama-3.2-3B (thinking off)** on `s2_nested_fixed` (run 3, line 3):
  ```text
  supported|text|LoRA low rank adaptation|2021|~|supported|
  ```

### Problem: `expected 5 columns, found 2`
- **Llama-3.2-3B (thinking off)** on `s2_nested_fixed` (run 4, line 2):
  ```text
  \rAttention is all you need\r|~|
  ```
- **Llama-3.2-3B (thinking off)** on `s2_nested_fixed` (run 4, line 4):
  ```text
  \rLoRA low rank adaptation\r|~|
  ```
- **Llama-3.2-3B (thinking off)** on `s2_nested_fixed` (run 4, line 6):
  ```text
  \rMistral 7B architecture\r|~|
  ```

### Problem: `empty field is not allowed (use ~ for null)`
- **Llama-3.2-3B (thinking off)** on `s2_nested_fixed` (run 5, line 4):
  ```text
  |7|0|2024|1|
  ```
- **Llama-3.2-3B (thinking off)** on `s2_nested_fixed` (run 5, line 8):
  ```text
  |Attention is all you need|0|2017|1|
  ```
- **Llama-3.2-3B (thinking off)** on `s2_nested_fixed` (run 5, line 12):
  ```text
  |LoRA low rank adaptation|1|2021|1|
  ```

### Problem: `expected 4 columns, found 11`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 1, line 1):
  ```text
  ~|~|supported|~|~|~|~|> notes are from abstract only|> > abstract mentions 32k limit|> > appendix B clarifies|> > notes are from abstract only|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 1, line 10):
  ```text
  ~|~|refuted|~|~|~|~|> KV cache quant loses 2x acc|> >> outlier channels preserved|> >> per-head scales help|> >> perplexity intact|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 1, line 14):
  ```text
  ~|~|supported|~|~|~|~|> Language models few-shot|> >> scaling curve is smooth|> >> tested up to 175B|> >> in-context learning observed|
  ```

### Problem: `expected 4 columns, found 12`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 1, line 3):
  ```text
  ~|~|supported|~|2021|~|~|tested on GLUE and Wikisql|> > rank 4 sufficient|> > higher ranks show diminishing returns|> > memory footprint reduced 3x|> > optimizer state minimal|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 1, line 6):
  ```text
  ~|~|supported|~|~|~|~|> optimal ratio is 20 to 1|> > tested across 400 models|> > cosine schedule used|> > kaplan tokens were suboptimal|> > fixed learning rate flawed|
  ```

### Problem: `expected 4 columns, found 13`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 1, line 4):
  ```text
  ~|~|supported|~|2021|~|~|tested on GLUE and Wikisql|> > rank 4 sufficient|> > higher ranks show diminishing returns|> > memory footprint reduced 3x|> > optimizer state minimal|> > checkpoint size under 50MB|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 1, line 8):
  ```text
  ~|~|supported|~|~|~|~|> LoRA matches full finetune|> > tested on GLUE and Wikisql|> > rank 4 sufficient|> > higher ranks show diminishing returns|> > memory footprint reduced 3x|> > optimizer state minimal|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 1, line 9):
  ```text
  ~|~|supported|~|2106.09685|~|~|tested on GLUE and Wikisql|> > rank 4 sufficient|> > higher ranks show diminishing returns|> > memory footprint reduced 3x|> > optimizer state minimal|> > checkpoint size under 50MB|
  ```

### Problem: `expected 4 columns, found 10`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 1, line 5):
  ```text
  ~|~|refuted|~|~|~|~|> outlier channels preserved|> >> per-head scales help|> >> perplexity intact|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 1, line 7):
  ```text
  ~|~|supported|~|~|~|~|> scaling curve is smooth|> > tested up to 175B|> > in-context learning observed|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 1, line 19):
  ```text
  ~|~|refuted|~|~|~|~|> Rope scaling hurts long ctx|> >> table 3 contradicts|> >> llama 7b tested only|
  ```

### Problem: `expected 4 columns, found 8`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 2, line 1):
  ```text
  ~|~|~|~|~|~|~|~|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 2, line 3):
  ```text
  text|Rope scaling hurts long ctx|~|~|~|~|~|~|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 2, line 4):
  ```text
  verdict|refuted|~|~|~|~|~|~|
  ```

### Problem: `expected 4 columns, found 7`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 2, line 2):
  ```text
  ~|supported|~|~|~|~|~|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 2, line 5):
  ```text
  source|Paper 2405.05254|year|2024|~|~|~|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 2, line 10):
  ```text
  source|Paper 2106.09685|year|2021|~|~|~|
  ```

### Problem: `expected 4 columns, found 5`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 2, line 6):
  ```text
  notes|> notes are from abstract only|~|> abstract mentions 32k limit|>> llama 7b tested only|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 4, line 7):
  ```text
  \r~|~|supported|~|~|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 4, line 21):
  ```text
  \r~|~|supported|~|~|
  ```

### Problem: `expected 4 columns, found 9`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 2, line 7):
  ```text
  |>|~|~|~|~|~|~|~|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 2, line 9):
  ```text
  verdict|~|~|~|~|~|~|~|~|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 2, line 17):
  ```text
  verdict|~|~|~|~|~|~|~|~|
  ```

### Problem: `'>' is not one of: supported, refuted, unknown`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 2, line 11):
  ```text
  notes|>|tested on GLUE and Wikisql|> rank 4 sufficient>> higher ranks show diminishing returns|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 2, line 24):
  ```text
  notes|>|optimal ratio is 20 to 1|> tested across 400 models>> cosine schedule used|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 2, line 30):
  ```text
  notes|>|scaling curve is smooth|> tested up to 175B>> in-context learning observed|
  ```

### Problem: `expected 4 columns, found 3`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 2, line 19):
  ```text
  notes|> outlier channels preserved|> per-head scales help>> perplexity intact|
  ```

### Problem: `expected 4 columns, found 47`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 3, line 1):
  ```text
  ~|~|supported|~|text:Rope scaling hurts long ctx|~|~|refuted|~|source:record{id:2405.05254, year:2024}|~|~|notes:|~|>subnotes:|~|>text:abstract mentions 32k limit|~|>text:appendix B clarifies|~|>subnotes:|~|>text:llama 7b tested only|~|~|notes:|~|>text:table 3 contradicts|~|>subnotes:|~|>text:memory footprint reduced 3x|~|>text:optimizer state minimal|~|>text:checkpoint size under 50MB|~|~|claims:|~|>text:Rope scaling hurts long ctx|~|~|verdict:refuted|~|~|~|
  ```

### Problem: `expected 4 columns, found 37`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 3, line 2):
  ```text
  ~|~|supported|~|text:LoRA matches full finetune|~|~|verdict:supported|~|source:record{id:2106.09685, year:2021}|~|~|notes:|~|>subnotes:|~|>text:rank 4 sufficient|~|>text:higher ranks show diminishing returns|~|>subnotes:|~|>text:memory footprint reduced 3x|~|>text:optimizer state minimal|~|>text:checkpoint size under 50MB|~|~|claims:|~|>text:LoRA matches full finetune|~|~|verdict:supported|~|~|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 3, line 4):
  ```text
  ~|~|supported|~|text:Chinchilla tokens beat params|~|~|verdict:supported|~|source:record{id:2203.15556, year:2022}|~|~|notes:|~|>subnotes:|~|>text:optimal ratio is 20 to 1|~|>text:tested across 400 models|~|>text:cosine schedule used|~|>subnotes:|~|>text:kaplan tokens were suboptimal|~|>text:fixed learning rate flawed|~|~|claims:|~|>text:Chinchilla tokens beat params|~|~|verdict:supported|~|~|
  ```

### Problem: `expected 4 columns, found 31`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 3, line 3):
  ```text
  ~|~|refuted|~|text:KV cache quant loses 2x acc|~|~|verdict:refuted|~|source:record{id:2406.12345, year:2024}|~|~|notes:|~|>subnotes:|~|>text:outlier channels preserved|~|>text:per-head scales help|~|>text:perplexity intact|~|~|claims:|~|>text:KV cache quant loses 2x acc|~|~|verdict:refuted|~|~|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 3, line 5):
  ```text
  ~|~|supported|~|text:Language models few-shot|~|~|verdict:supported|~|source:record{id:2005.14165, year:2020}|~|~|notes:|~|>subnotes:|~|>text:scaling curve is smooth|~|>text:tested up to 175B|~|>text:in-context learning observed|~|~|claims:|~|>text:Language models few-shot|~|~|verdict:supported|~|~|
  ```

### Problem: `expected 4 columns, found 6`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 4, line 1):
  ```text
  ~|~|refuted|2024|~|~|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 4, line 3):
  ```text
  \rnotes >abstract mentions 32k limit \|abstract\|
\rnotes >>appendix B clarifies \|subnotes\|
\rnotes >>llama 7b tested only \|subnotes\|
\r~|~|~|2021|~|~|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 4, line 8):
  ```text
  \rLoRA matches full finetune \|LoRA matches full finetune\|
\rnotes >tested on GLUE and Wikisql \|subnotes\|
\rnotes >>rank 4 sufficient \|subnotes\|
\rnotes >>higher ranks show diminishing returns \|subnotes\|
\rnotes >memory footprint reduced 3x \|subnotes\|
\rnotes >>optimizer state minimal \|subnotes\|
\rnotes >checkpoint size under 50MB \|subnotes\|
\r~|~|refuted|2024|~|~|
  ```

### Problem: `'text' is not one of: supported, refuted, unknown`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 5, line 1):
  ```text
  ~|text| supported |~|
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 5, line 2):
  ```text
  ~|text| Rope scaling hurts long ctx |~|
  ```

### Problem: `dropped as child of line 6`
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 5, line 7):
  ```text
  > notes | text | notes are from abstract only |>
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 5, line 8):
  ```text
  >> notes | text | abstract mentions 32k limit |>
  ```
- **Llama-3.2-3B (thinking off)** on `s3_nested_variable` (run 5, line 9):
  ```text
  >> notes | text | appendix B clarifies |>
  ```

### Problem: `with several outside classes, data before the first schema line is an error`
- **Llama-3.2-3B (thinking off)** on `s4_two_class_code` (run 1, line 1):
  ```text
  ~|~|src/lib.rs|~|
  ```
- **Llama-3.2-3B (thinking off)** on `s4_two_class_code` (run 1, line 2):
  ```text
  ~|~|pub fn add(a: i32, b: i32) -> i32 { |
  ```
- **Llama-3.2-3B (thinking off)** on `s4_two_class_code` (run 1, line 3):
  ```text
  ~|~|src/util.rs|~|
  ```

### Problem: `'verdict' is not one of: supported, refuted, unverified`
- **Gemma-4-E2B (thinking off)** on `s1_flat` (run 2, line 1):
  ```text
  id|title|verdict|year|verified|
  ```
- **Gemma-4-E2B (thinking off)** on `s1_flat` (run 3, line 1):
  ```text
  id|title|verdict|year|verified|
  ```
- **Gemma-4-E2B (thinking off)** on `s1_flat` (run 4, line 1):
  ```text
  id|title|verdict|year|verified|
  ```

### Problem: `'1706.03762' is not a valid int (i64)`
- **Gemma-4-E2B (thinking off)** on `s2_nested_fixed` (run 5, line 2):
  ```text
  Attention is all you need|supported|record|1706.03762|2017|
  ```

### Problem: `'2106.09685' is not a valid int (i64)`
- **Gemma-4-E2B (thinking off)** on `s2_nested_fixed` (run 5, line 3):
  ```text
  LoRA low rank adaptation|supported|record|2106.09685|2021|
  ```

### Problem: `'2309.06180' is not a valid int (i64)`
- **Gemma-4-E2B (thinking off)** on `s2_nested_fixed` (run 5, line 4):
  ```text
  Mistral 7B architecture|supported|record|2309.06180|2023|
  ```

### Problem: `'2401.04088' is not a valid int (i64)`
- **Gemma-4-E2B (thinking off)** on `s2_nested_fixed` (run 5, line 5):
  ```text
  DeepSeekMoE mixture experts|supported|record|2401.04088|2024|
  ```

### Problem: `invalid escape sequence '\r'`
- **Gemma-4-E2B (thinking off)** on `s3_nested_variable` (run 3, line 1):
  ```text
  text|refuted|source{id:2405.05254,year:2024}|notes:[{text:\r\nabstract mentions 32k limit\r\nappendix B clarifies,subnotes:[\r\nabstract mentions 32k limit\r\nappendix B clarifies\r\n]},{text:\r\ntable 3 contradicts,subnotes:[\r\nllama 7b tested only\r\n]}]|
  ```
- **Gemma-4-E2B (thinking off)** on `s3_nested_variable` (run 3, line 2):
  ```text
  text|supported|source{id:2106.09685,year:2021}|notes:[{text:\r\ntested on GLUE and Wikisql,subnotes:[\r\nrank 4 sufficient\r\nhigher ranks show diminishing returns\r\n]},{text:\r\nmemory footprint reduced 3x,subnotes:[\r\noptimizer state minimal\r\n]},{text:\r\ncheckpoint size under 50MB}]|
  ```
- **Gemma-4-E2B (thinking off)** on `s3_nested_variable` (run 3, line 3):
  ```text
  text|refuted|source{id:2406.12345,year:2024}|notes:[{text:\r\noutlier channels preserved,subnotes:[\r\nper-head scales help\r\nperplexity intact\r\n]}]|
  ```

### Problem: `dropped as child of line 3`
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 2, line 4):
  ```text
  >>>abstract mentions 32k limit
  ```
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 2, line 5):
  ```text
  >>>appendix B clarifies
  ```

### Problem: `dropped as child of line 10`
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 2, line 11):
  ```text
  >>>rank 4 sufficient
  ```
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 2, line 12):
  ```text
  >>>higher ranks show diminishing returns
  ```

### Problem: `dropped as child of line 13`
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 2, line 14):
  ```text
  >>>optimizer state minimal
  ```

### Problem: `dropped as child of line 18`
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 2, line 19):
  ```text
  >>>per-head scales help
  ```
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 2, line 20):
  ```text
  >>>perplexity intact|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 1, line 19):
  ```text
  > optimal ratio is 20 to 1|
  ```

### Problem: `dropped as child of line 23`
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 2, line 24):
  ```text
  >>>tested across 400 models
  ```
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 2, line 25):
  ```text
  >>>cosine schedule used
  ```

### Problem: `dropped as child of line 26`
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 2, line 27):
  ```text
  >>>fixed learning rate flawed|
  ```

### Problem: `dropped as child of line 30`
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 2, line 31):
  ```text
  >>>tested up to 175B
  ```
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 2, line 32):
  ```text
  >>>in-context learning observed|
  ```

### Problem: `expected 4 columns, found 17`
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 3, line 1):
  ```text
  "Rope scaling hurts long ctx"|refuted|record{id:2405.05254,year:2024}|[record{text:"notes are from abstract only",notes:[record{text:"abstract mentions 32k limit",subnotes:[<|"|>abstract mentions 32k limit<|"|>,<|"|>appendix B clarifies<|"|>]},record{text:"table 3 contradicts",subnotes:[<|"|>llama 7b tested only<|"|>]}]}}||
  ```
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 3, line 2):
  ```text
  "LoRA matches full finetune"|supported|record{id:2106.09685,year:2021}|[record{text:"tested on GLUE and Wikisql",notes:[record{text:"rank 4 sufficient",subnotes:[<|"|>rank 4 sufficient<|"|>,<|"|>higher ranks show diminishing returns<|"|>]},record{text:"memory footprint reduced 3x",subnotes:[<|"|>optimizer state minimal<|"|>]},record{text:"checkpoint size under 50MB"}]}}||
  ```
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 3, line 4):
  ```text
  "Chinchilla tokens beat params"|supported|record{id:2203.15556,year:2022}|[record{text:"optimal ratio is 20 to 1",notes:[record{text:"tested across 400 models",subnotes:[<|"|>tested across 400 models<|"|>,<|"|>cosine schedule used<|"|>]},record{text:"kaplan tokens were suboptimal",subnotes:[<|"|>fixed learning rate flawed<|"|>]}]}]||
  ```

### Problem: `expected 4 columns, found 21`
- **Gemma-4-E2B (thinking on)** on `s3_nested_variable` (run 3, line 3):
  ```text
  "KV cache quant loses 2x acc"|refuted|record{id:2406.12345,year:2024}|[record{text:"outlier channels preserved",notes:[record{text:"per-head scales help",subnotes:[<|"|>per-head scales help<|"|>,<|"|>perplexity intact<|"|>]},record{text:"perplexity intact",subnotes:[<|"|>per-head scales help<|"|>,<|"|>perplexity intact<|"|>]}]}]||
  ```

### Problem: `'2106.09685,"LoRA low rank adaptation",supported,2021,1' is not one of: supported, refuted, unverified`
- **LFM2.5-8B (thinking on)** on `s1_flat` (run 2, line 2):
  ```text
  2405.05254,"Rope scaling hurts long ctx",refuted,2024,0|1706.03762,"Attention is all you need",supported,2017,1|2106.09685,"LoRA low rank adaptation",supported,2021,1|2309.06180,"Mistral 7B architecture",supported,2023,1|2401.04088,"DeepSeekMoE mixture experts",supported,2024,1|
  ```

### Problem: `expected 5 columns, found 1`
- **LFM2.5-8B (thinking on)** on `s2_nested_fixed` (run 4, line 2):
  ```text
  2405.05254 claimed "Rope scaling hurts long ctx" in 2024; it turned out to be false, so the claim is refuted, "refuted", {"id":"2405.05254","year":2024}, 0|
  ```
- **LFM2.5-8B (thinking on)** on `s2_nested_fixed` (run 4, line 3):
  ```text
  In 2017, paper 1706.03762 claimed "Attention is all you need" in 2017; verification was true, so the claim is supported", "supported", {"id":"1706.03762","year":2017}, 1|
  ```
- **LFM2.5-8B (thinking on)** on `s2_nested_fixed` (run 4, line 4):
  ```text
  Paper 2106.09685 made the claim "LoRA low rank adaptation" in 2021; it was verified as true, and the claim is supported", "supported", {"id":"2106.09685","year":2021}, 1|
  ```

### Problem: `reusing class 'name' that hasn't been defined yet`
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 2, line 2):
  ```text
  schema:name
  ```
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 2, line 10):
  ```text
  schema:name
  ```
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 2, line 18):
  ```text
  schema:name
  ```

### Problem: `reusing class 'name lib.rs' that hasn't been defined yet`
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 5, line 2):
  ```text
  schema:name lib.rs
  ```

### Problem: `reusing class 'file name="src/lib.rs" path="lib.rs"' that hasn't been defined yet`
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 5, line 3):
  ```text
  schema:file name="src/lib.rs" path="lib.rs"
  ```

### Problem: `reusing class 'code 'pub fn add(a: i32, b: i32) -> i32\n    a + b'' that hasn't been defined yet`
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 5, line 4):
  ```text
  schema:code 'pub fn add(a: i32, b: i32) -> i32\n    a + b'
  ```

### Problem: `reusing class 'line 10' that hasn't been defined yet`
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 5, line 5):
  ```text
  schema:line 10
  ```

### Problem: `reusing class 'name util.rs' that hasn't been defined yet`
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 5, line 6):
  ```text
  schema:name util.rs
  ```

### Problem: `reusing class 'file name="src/util.rs" path="util.rs"' that hasn't been defined yet`
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 5, line 7):
  ```text
  schema:file name="src/util.rs" path="util.rs"
  ```

### Problem: `reusing class 'code 'pub fn greet(name: &str) -> String {\n    format!("Hello, {}!", name)'' that hasn't been defined yet`
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 5, line 8):
  ```text
  schema:code 'pub fn greet(name: &str) -> String {\n    format!("Hello, {}!", name)'
  ```

### Problem: `reusing class 'line 5' that hasn't been defined yet`
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 5, line 9):
  ```text
  schema:line 5
  ```

### Problem: `reusing class 'name main.rs' that hasn't been defined yet`
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 5, line 10):
  ```text
  schema:name main.rs
  ```

### Problem: `reusing class 'file name="src/main.rs" path="main.rs"' that hasn't been defined yet`
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 5, line 11):
  ```text
  schema:file name="src/main.rs" path="main.rs"
  ```

### Problem: `reusing class 'code 'fn main() {\n    println!("Hello, world!");}'' that hasn't been defined yet`
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 5, line 12):
  ```text
  schema:code 'fn main() {\n    println!("Hello, world!");}'
  ```

### Problem: `reusing class 'line 1' that hasn't been defined yet`
- **LFM2.5-8B (thinking on)** on `s4_two_class_code` (run 5, line 13):
  ```text
  schema:line 1
  ```

### Problem: `dropped as child of line 1`
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 1, line 2):
  ```text
  > notes are from abstract only|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 1, line 3):
  ```text
  >> abstract mentions 32k limit|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 1, line 4):
  ```text
  >> appendix B clarifies|
  ```

### Problem: `dropped as child of line 7`
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 1, line 8):
  ```text
  > tested on GLUE and Wikisql|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 1, line 9):
  ```text
  >> rank 4 sufficient|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 1, line 10):
  ```text
  >> higher ranks show diminishing returns|
  ```

### Problem: `dropped as child of line 14`
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 1, line 15):
  ```text
  > outlier channels preserved|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 1, line 16):
  ```text
  >> per-head scales help|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 1, line 17):
  ```text
  >> perplexity intact|
  ```

### Problem: `dropped as child of line 24`
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 1, line 25):
  ```text
  > scaling curve is smooth|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 1, line 26):
  ```text
  >> tested up to 175B|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 1, line 27):
  ```text
  >> in-context learning observed|
  ```

### Problem: `expected 1 columns, found 2`
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 2):
  ```text
  > notes are from abstract only|~|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 5):
  ```text
  > table 3 contradicts|~|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 8):
  ```text
  > tested on GLUE and Wikisql|~|
  ```

### Problem: `dropped as child of line 2`
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 3):
  ```text
  >> abstract mentions 32k limit|~|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 4):
  ```text
  >> appendix B clarifies|~|
  ```

### Problem: `dropped as child of line 5`
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 6):
  ```text
  >> llama 7b tested only|~|
  ```

### Problem: `dropped as child of line 8`
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 9):
  ```text
  >> rank 4 sufficient|~|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 10):
  ```text
  >> higher ranks show diminishing returns|~|
  ```

### Problem: `dropped as child of line 11`
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 12):
  ```text
  >> optimizer state minimal|~|
  ```

### Problem: `dropped as child of line 15`
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 16):
  ```text
  >> per-head scales help|~|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 17):
  ```text
  >> perplexity intact|~|
  ```

### Problem: `dropped as child of line 19`
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 20):
  ```text
  >> tested across 400 models|~|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 21):
  ```text
  >> cosine schedule used|~|
  ```

### Problem: `dropped as child of line 22`
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 23):
  ```text
  >> fixed learning rate flawed|~|
  ```

### Problem: `dropped as child of line 25`
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 26):
  ```text
  >> tested up to 175B|~|
  ```
- **Mellum2.1-12B (thinking on)** on `s3_nested_variable` (run 5, line 27):
  ```text
  >> in-context learning observed|~|
  ```

### Problem: `invalid escape sequence '\n'`
- **Mellum2.1-12B (thinking on)** on `s4_two_class_code` (run 2, line 4):
  ```text
  lib.rs|\rpub fn add(a: i32, b: i32) -> i32 {
    a + b
}\n|10|
  ```
- **Mellum2.1-12B (thinking on)** on `s4_two_class_code` (run 2, line 10):
  ```text
  util.rs|\rpub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}\n|5|
  ```
- **Mellum2.1-12B (thinking on)** on `s4_two_class_code` (run 2, line 16):
  ```text
  main.rs|\rfn main() {
    println!("Hello, world!");
}\n|1|
  ```

## 10. Comparison with Run 2

### Setup Differences
| Aspect | Run 2 | Run 3 |
|---|---|---|
| **API Endpoint** | `POST /completion` (raw token generation) | `POST /v1/chat/completions` (OpenAI-compatible chat) |
| **Chat Formatting** | Hand-crafted role headers & prompt prefills | Model-native Jinja chat template applied by server |
| **Thinking Control** | Prefills (`<|turn>system\n<|think|>`, `<think>\n`) | `chat_template_kwargs: {"enable_thinking": bool}` (Gemma); template default (LFM, Mellum) |
| **Output Budget** | Hard 1,500 token limit (`n_predict: 1500`) | Uncapped (full 128k/131k context window) |
| **Task Notes** | Templated notes with single quotes (`title '...'`) | Natural English sentences with double quotes (`title "..."`) |
| **Reasoning Extraction** | Inline tag chopping (`<channel|>`, `</think>`) | Dedicated `reasoning_content` field from server |

### Empirical Observations
- **Elimination of budget truncation artifacts:** In Run 2, 31 out of 100 runs were cut off by the 1,500 token cap (Mellum 19/20, LFM 11/20, Llama 1/20), leaving unfinished lines and unclosed structures. In Run 3, zero runs hit a token cap or timed out.
- **Impact of natural sentences:** Rewriting single-quoted templated phrases to natural sentences prevented models from copying single quotation marks into string fields.
- **Chat template adherence:** Without artificial system turn prefills, thinking models followed their natural reasoning distribution before emitting Struct records.

## 11. Complete Per-Run Results Table

| Model | Schema | Run | Seed | Outcome | Records | Repairs | Bad Lines | finish_reason | Think Tok | Out Tok | Field Acc |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Llama-3.2-3B (thinking off) | `s1_flat` | 1 | 42 | Clean as written | 5/5 | 0 | 0 | stop | 0 | 79 | 68.0% |
| Llama-3.2-3B (thinking off) | `s1_flat` | 2 | 43 | Clean as written | 5/5 | 0 | 0 | stop | 0 | 79 | 68.0% |
| Llama-3.2-3B (thinking off) | `s1_flat` | 3 | 44 | Clean as written | 5/5 | 0 | 0 | stop | 0 | 79 | 68.0% |
| Llama-3.2-3B (thinking off) | `s1_flat` | 4 | 45 | Nothing usable | 0/5 | 1 | 4 | stop | 0 | 72 | n/a |
| Llama-3.2-3B (thinking off) | `s1_flat` | 5 | 46 | Clean as written | 5/5 | 0 | 0 | stop | 0 | 80 | 76.0% |
| Llama-3.2-3B (thinking off) | `s2_nested_fixed` | 1 | 42 | Nothing usable | 0/5 | 1 | 1 | stop | 0 | 105 | n/a |
| Llama-3.2-3B (thinking off) | `s2_nested_fixed` | 2 | 43 | Salvaged (5 records) | 5/5 | 0 | 1 | stop | 0 | 84 | 60.0% |
| Llama-3.2-3B (thinking off) | `s2_nested_fixed` | 3 | 44 | Nothing usable | 0/5 | 0 | 5 | stop | 0 | 88 | n/a |
| Llama-3.2-3B (thinking off) | `s2_nested_fixed` | 4 | 45 | Salvaged (5 records) | 5/5 | 1 | 4 | stop | 0 | 109 | 4.0% |
| Llama-3.2-3B (thinking off) | `s2_nested_fixed` | 5 | 46 | Nothing usable | 0/5 | 0 | 16 | stop | 0 | 196 | n/a |
| Llama-3.2-3B (thinking off) | `s3_nested_variable` | 1 | 42 | Nothing usable | 0/5 | 0 | 23 | stop | 0 | 1133 | n/a |
| Llama-3.2-3B (thinking off) | `s3_nested_variable` | 2 | 43 | Nothing usable | 0/5 | 0 | 30 | stop | 0 | 577 | n/a |
| Llama-3.2-3B (thinking off) | `s3_nested_variable` | 3 | 44 | Nothing usable | 0/5 | 0 | 5 | stop | 0 | 726 | n/a |
| Llama-3.2-3B (thinking off) | `s3_nested_variable` | 4 | 45 | Salvaged (2 records) | 2/5 | 1 | 8 | stop | 0 | 455 | 0.0% |
| Llama-3.2-3B (thinking off) | `s3_nested_variable` | 5 | 46 | Nothing usable | 0/5 | 1 | 26 | stop | 0 | 342 | n/a |
| Llama-3.2-3B (thinking off) | `s4_two_class_code` | 1 | 42 | Nothing usable | 0/6 | 0 | 7 | stop | 0 | 90 | n/a |
| Llama-3.2-3B (thinking off) | `s4_two_class_code` | 2 | 43 | Nothing usable | 0/6 | 0 | 3 | stop | 0 | 107 | n/a |
| Llama-3.2-3B (thinking off) | `s4_two_class_code` | 3 | 44 | Nothing usable | 0/6 | 0 | 12 | stop | 0 | 124 | n/a |
| Llama-3.2-3B (thinking off) | `s4_two_class_code` | 4 | 45 | Nothing usable | 0/6 | 0 | 1 | stop | 0 | 76 | n/a |
| Llama-3.2-3B (thinking off) | `s4_two_class_code` | 5 | 46 | Nothing usable | 0/6 | 0 | 11 | stop | 0 | 280 | n/a |
| Gemma-4-E2B (thinking off) | `s1_flat` | 1 | 42 | Salvaged (5 records) | 5/5 | 5 | 1 | stop | 0 | 144 | 96.0% |
| Gemma-4-E2B (thinking off) | `s1_flat` | 2 | 43 | Salvaged (5 records) | 5/5 | 0 | 1 | stop | 0 | 150 | 96.0% |
| Gemma-4-E2B (thinking off) | `s1_flat` | 3 | 44 | Salvaged (5 records) | 5/5 | 0 | 1 | stop | 0 | 150 | 96.0% |
| Gemma-4-E2B (thinking off) | `s1_flat` | 4 | 45 | Salvaged (5 records) | 5/5 | 0 | 1 | stop | 0 | 150 | 96.0% |
| Gemma-4-E2B (thinking off) | `s1_flat` | 5 | 46 | Salvaged (5 records) | 5/5 | 0 | 1 | stop | 0 | 150 | 96.0% |
| Gemma-4-E2B (thinking off) | `s2_nested_fixed` | 1 | 42 | Nothing usable | 0/5 | 0 | 5 | stop | 0 | 145 | n/a |
| Gemma-4-E2B (thinking off) | `s2_nested_fixed` | 2 | 43 | Salvaged (1 records) | 1/5 | 1 | 3 | stop | 0 | 145 | 40.0% |
| Gemma-4-E2B (thinking off) | `s2_nested_fixed` | 3 | 44 | Salvaged (1 records) | 1/5 | 1 | 3 | stop | 0 | 145 | 40.0% |
| Gemma-4-E2B (thinking off) | `s2_nested_fixed` | 4 | 45 | Salvaged (1 records) | 1/5 | 1 | 3 | stop | 0 | 145 | 40.0% |
| Gemma-4-E2B (thinking off) | `s2_nested_fixed` | 5 | 46 | Salvaged (1 records) | 1/5 | 0 | 4 | stop | 0 | 121 | 40.0% |
| Gemma-4-E2B (thinking off) | `s3_nested_variable` | 1 | 42 | Nothing usable | 0/5 | 1 | 4 | stop | 0 | 405 | n/a |
| Gemma-4-E2B (thinking off) | `s3_nested_variable` | 2 | 43 | Nothing usable | 0/5 | 1 | 4 | stop | 0 | 410 | n/a |
| Gemma-4-E2B (thinking off) | `s3_nested_variable` | 3 | 44 | Nothing usable | 0/5 | 0 | 5 | stop | 0 | 455 | n/a |
| Gemma-4-E2B (thinking off) | `s3_nested_variable` | 4 | 45 | Nothing usable | 0/5 | 1 | 4 | stop | 0 | 414 | n/a |
| Gemma-4-E2B (thinking off) | `s3_nested_variable` | 5 | 46 | Nothing usable | 0/5 | 1 | 4 | stop | 0 | 382 | n/a |
| Gemma-4-E2B (thinking off) | `s4_two_class_code` | 1 | 42 | Nothing usable | 0/6 | 0 | 6 | stop | 0 | 143 | n/a |
| Gemma-4-E2B (thinking off) | `s4_two_class_code` | 2 | 43 | Nothing usable | 0/6 | 0 | 6 | stop | 0 | 132 | n/a |
| Gemma-4-E2B (thinking off) | `s4_two_class_code` | 3 | 44 | Nothing usable | 0/6 | 2 | 10 | stop | 0 | 138 | n/a |
| Gemma-4-E2B (thinking off) | `s4_two_class_code` | 4 | 45 | Nothing usable | 0/6 | 0 | 6 | stop | 0 | 74 | n/a |
| Gemma-4-E2B (thinking off) | `s4_two_class_code` | 5 | 46 | Nothing usable | 0/6 | 0 | 6 | stop | 0 | 68 | n/a |
| Gemma-4-E2B (thinking on) | `s1_flat` | 1 | 42 | Fully repaired | 5/5 | 5 | 0 | stop | 0 | 88 | 72.0% |
| Gemma-4-E2B (thinking on) | `s1_flat` | 2 | 43 | Clean as written | 5/5 | 0 | 0 | stop | 1066 | 138 | 96.0% |
| Gemma-4-E2B (thinking on) | `s1_flat` | 3 | 44 | Clean as written | 5/5 | 0 | 0 | stop | 1124 | 138 | 92.0% |
| Gemma-4-E2B (thinking on) | `s1_flat` | 4 | 45 | Clean as written | 5/5 | 0 | 0 | stop | 1217 | 138 | 92.0% |
| Gemma-4-E2B (thinking on) | `s1_flat` | 5 | 46 | Clean as written | 5/5 | 0 | 0 | stop | 999 | 138 | 92.0% |
| Gemma-4-E2B (thinking on) | `s2_nested_fixed` | 1 | 42 | Fully repaired | 5/5 | 1 | 0 | stop | 1267 | 137 | 96.0% |
| Gemma-4-E2B (thinking on) | `s2_nested_fixed` | 2 | 43 | Nothing usable | 0/5 | 1 | 4 | stop | 1228 | 188 | n/a |
| Gemma-4-E2B (thinking on) | `s2_nested_fixed` | 3 | 44 | Clean as written | 5/5 | 0 | 0 | stop | 918 | 138 | 96.0% |
| Gemma-4-E2B (thinking on) | `s2_nested_fixed` | 4 | 45 | Fully repaired | 5/5 | 1 | 0 | stop | 1255 | 147 | 96.0% |
| Gemma-4-E2B (thinking on) | `s2_nested_fixed` | 5 | 46 | Fully repaired | 5/5 | 1 | 0 | stop | 912 | 147 | 100.0% |
| Gemma-4-E2B (thinking on) | `s3_nested_variable` | 1 | 42 | Nothing usable | 0/5 | 0 | 5 | stop | 1173 | 396 | n/a |
| Gemma-4-E2B (thinking on) | `s3_nested_variable` | 2 | 43 | Salvaged (5 records) | 5/5 | 5 | 22 | stop | 1632 | 391 | 45.2% |
| Gemma-4-E2B (thinking on) | `s3_nested_variable` | 3 | 44 | Nothing usable | 0/5 | 0 | 5 | stop | 1243 | 503 | n/a |
| Gemma-4-E2B (thinking on) | `s3_nested_variable` | 4 | 45 | Nothing usable | 0/5 | 0 | 5 | stop | 931 | 354 | n/a |
| Gemma-4-E2B (thinking on) | `s3_nested_variable` | 5 | 46 | Nothing usable | 0/5 | 1 | 4 | stop | 1182 | 443 | n/a |
| Gemma-4-E2B (thinking on) | `s4_two_class_code` | 1 | 42 | Nothing usable | 0/6 | 0 | 3 | stop | 805 | 201 | n/a |
| Gemma-4-E2B (thinking on) | `s4_two_class_code` | 2 | 43 | Nothing usable | 0/6 | 0 | 3 | stop | 0 | 68 | n/a |
| Gemma-4-E2B (thinking on) | `s4_two_class_code` | 3 | 44 | Nothing usable | 0/6 | 0 | 3 | stop | 0 | 68 | n/a |
| Gemma-4-E2B (thinking on) | `s4_two_class_code` | 4 | 45 | Nothing usable | 0/6 | 0 | 3 | stop | 0 | 68 | n/a |
| Gemma-4-E2B (thinking on) | `s4_two_class_code` | 5 | 46 | Nothing usable | 0/6 | 0 | 3 | stop | 0 | 68 | n/a |
| LFM2.5-8B (thinking on) | `s1_flat` | 1 | 42 | Fully repaired | 5/5 | 5 | 0 | stop | 979 | 107 | 96.0% |
| LFM2.5-8B (thinking on) | `s1_flat` | 2 | 43 | Nothing usable | 0/5 | 0 | 1 | stop | 1024 | 108 | n/a |
| LFM2.5-8B (thinking on) | `s1_flat` | 3 | 44 | Clean as written | 5/5 | 0 | 0 | stop | 887 | 91 | 56.0% |
| LFM2.5-8B (thinking on) | `s1_flat` | 4 | 45 | Clean as written | 5/5 | 0 | 0 | stop | 1461 | 118 | 76.0% |
| LFM2.5-8B (thinking on) | `s1_flat` | 5 | 46 | Fully repaired | 5/5 | 5 | 0 | stop | 904 | 117 | 76.0% |
| LFM2.5-8B (thinking on) | `s2_nested_fixed` | 1 | 42 | Nothing usable | 0/5 | 1 | 4 | stop | 1886 | 239 | n/a |
| LFM2.5-8B (thinking on) | `s2_nested_fixed` | 2 | 43 | Nothing usable | 0/5 | 1 | 4 | stop | 1885 | 356 | n/a |
| LFM2.5-8B (thinking on) | `s2_nested_fixed` | 3 | 44 | Nothing usable | 0/5 | 1 | 4 | stop | 1631 | 216 | n/a |
| LFM2.5-8B (thinking on) | `s2_nested_fixed` | 4 | 45 | Nothing usable | 0/5 | 1 | 4 | stop | 2395 | 286 | n/a |
| LFM2.5-8B (thinking on) | `s2_nested_fixed` | 5 | 46 | Nothing usable | 0/5 | 1 | 4 | stop | 2807 | 117 | n/a |
| LFM2.5-8B (thinking on) | `s3_nested_variable` | 1 | 42 | Nothing usable | 0/5 | 6 | 0 | stop | 2894 | 475 | n/a |
| LFM2.5-8B (thinking on) | `s3_nested_variable` | 2 | 43 | Nothing usable | 0/5 | 0 | 1 | stop | 3168 | 509 | n/a |
| LFM2.5-8B (thinking on) | `s3_nested_variable` | 3 | 44 | Nothing usable | 0/5 | 1 | 4 | stop | 2825 | 297 | n/a |
| LFM2.5-8B (thinking on) | `s3_nested_variable` | 4 | 45 | Nothing usable | 0/5 | 1 | 4 | stop | 3822 | 437 | n/a |
| LFM2.5-8B (thinking on) | `s3_nested_variable` | 5 | 46 | Nothing usable | 0/5 | 5 | 0 | stop | 3363 | 379 | n/a |
| LFM2.5-8B (thinking on) | `s4_two_class_code` | 1 | 42 | Nothing usable | 0/6 | 0 | 3 | stop | 1813 | 111 | n/a |
| LFM2.5-8B (thinking on) | `s4_two_class_code` | 2 | 43 | Nothing usable | 0/6 | 4 | 17 | stop | 3187 | 160 | n/a |
| LFM2.5-8B (thinking on) | `s4_two_class_code` | 3 | 44 | Nothing usable | 0/6 | 0 | 9 | stop | 3134 | 177 | n/a |
| LFM2.5-8B (thinking on) | `s4_two_class_code` | 4 | 45 | Nothing usable | 0/6 | 14 | 27 | stop | 4238 | 283 | n/a |
| LFM2.5-8B (thinking on) | `s4_two_class_code` | 5 | 46 | Nothing usable | 0/6 | 0 | 12 | stop | 2340 | 142 | n/a |
| Mellum2.1-12B (thinking on) | `s1_flat` | 1 | 42 | Clean as written | 5/5 | 0 | 0 | stop | 2280 | 139 | 92.0% |
| Mellum2.1-12B (thinking on) | `s1_flat` | 2 | 43 | Clean as written | 5/5 | 0 | 0 | stop | 1574 | 139 | 92.0% |
| Mellum2.1-12B (thinking on) | `s1_flat` | 3 | 44 | Clean as written | 5/5 | 0 | 0 | stop | 1797 | 139 | 92.0% |
| Mellum2.1-12B (thinking on) | `s1_flat` | 4 | 45 | Clean as written | 5/5 | 0 | 0 | stop | 1926 | 139 | 92.0% |
| Mellum2.1-12B (thinking on) | `s1_flat` | 5 | 46 | Clean as written | 5/5 | 0 | 0 | stop | 1309 | 139 | 92.0% |
| Mellum2.1-12B (thinking on) | `s2_nested_fixed` | 1 | 42 | Clean as written | 5/5 | 0 | 0 | stop | 2602 | 139 | 92.0% |
| Mellum2.1-12B (thinking on) | `s2_nested_fixed` | 2 | 43 | Clean as written | 5/5 | 0 | 0 | stop | 2756 | 139 | 96.0% |
| Mellum2.1-12B (thinking on) | `s2_nested_fixed` | 3 | 44 | Clean as written | 5/5 | 0 | 0 | stop | 3915 | 139 | 92.0% |
| Mellum2.1-12B (thinking on) | `s2_nested_fixed` | 4 | 45 | Clean as written | 5/5 | 0 | 0 | stop | 2803 | 139 | 92.0% |
| Mellum2.1-12B (thinking on) | `s2_nested_fixed` | 5 | 46 | Clean as written | 5/5 | 0 | 0 | stop | 3098 | 139 | 92.0% |
| Mellum2.1-12B (thinking on) | `s3_nested_variable` | 1 | 42 | Nothing usable | 0/5 | 0 | 27 | stop | 16161 | 324 | n/a |
| Mellum2.1-12B (thinking on) | `s3_nested_variable` | 2 | 43 | Nothing usable | 0/5 | 0 | 27 | stop | 20549 | 381 | n/a |
| Mellum2.1-12B (thinking on) | `s3_nested_variable` | 3 | 44 | Clean as written | 5/5 | 0 | 0 | stop | 21913 | 314 | 95.2% |
| Mellum2.1-12B (thinking on) | `s3_nested_variable` | 4 | 45 | Clean as written | 5/5 | 0 | 0 | stop | 22807 | 317 | 97.6% |
| Mellum2.1-12B (thinking on) | `s3_nested_variable` | 5 | 46 | Salvaged (5 records) | 5/5 | 0 | 22 | stop | 26689 | 358 | 42.9% |
| Mellum2.1-12B (thinking on) | `s4_two_class_code` | 1 | 42 | Nothing usable | 0/6 | 1 | 11 | stop | 12518 | 136 | n/a |
| Mellum2.1-12B (thinking on) | `s4_two_class_code` | 2 | 43 | Salvaged (3 records) | 3/6 | 0 | 3 | stop | 14824 | 154 | 100.0% |
| Mellum2.1-12B (thinking on) | `s4_two_class_code` | 3 | 44 | Nothing usable | 0/6 | 2 | 6 | stop | 5634 | 147 | n/a |
| Mellum2.1-12B (thinking on) | `s4_two_class_code` | 4 | 45 | Salvaged (3 records) | 3/6 | 0 | 3 | stop | 16584 | 150 | 100.0% |
| Mellum2.1-12B (thinking on) | `s4_two_class_code` | 5 | 46 | Nothing usable | 0/6 | 2 | 6 | stop | 7592 | 148 | n/a |
