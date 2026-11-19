# Struct real-model run: Ternary Bonsai 2 27B (run3 setup)

40 runs: 2 modes x 4 schemas x 5 seeds (42 to 46). Collected by `bench/r3/collect_bonsai.py`, scored by `bench/r3/evaluate_bonsai.py`, data in `bench/r3/real/run3_bonsai/`. Observations only.

## 1. Setup

- Endpoint: `POST /v1/chat/completions` on the already-running server at `http://127.0.0.1:8080` (the collector never started or stopped a server). One user message per request, the same prompt text as run3, no prefills, no `max_tokens`, no `n_predict`, `stream: false`, seeds 42 to 46, 5 runs per schema, four `bench/r2/schemas`, no grammar. 20-minute safety timeout per run (recorded as `timeout`).
- Prompt check: the user message of every request equals, byte for byte, the run3 request prompt for the same schema: `s1_flat` yes, `s2_nested_fixed` yes, `s3_nested_variable` yes, `s4_two_class_code` yes.
- Scoring: `target/release/doctor_tool.exe` in salvage mode; field accuracy is positional only (decoded record N against ground-truth record N, no matching by id), pooled over decoded records, `n/a` if none decoded; s4 expects 6 records.
- Token counts: thinking text and answer text counted with the server's `/tokenize`; checked against `usage.completion_tokens` (section 7).

### Sampling per mode (sent in every request)

| Mode | `chat_template_kwargs` | temperature | top_p | top_k | min_p | presence_penalty | repeat_penalty |
|---|---|---|---|---|---|---|---|
| thinking on | `{"enable_thinking": true}` | 1.0 | 0.95 | 20 | 0.05 | 0.0 | 1.0 |
| thinking off | `{"enable_thinking": false}` | 0.7 | 0.80 | 20 | 0.0 | 1.5 | 1.0 |

The run3 models (Llama, Gemma, LFM, Mellum) used temperature 0.7 with every other sampling setting left at the server defaults. The Bonsai sampling differs on purpose: it is PrismML's recommended sampling for each mode, so the comparison is between each model in its documented setup, not between identical sampling settings.

## 2. Server settings (`/props`, read after the run)

- Build: `b12128-d7b289683` (prism-llama-cpp fork); model: `C:\Users\Ashman Das\Documents\prism-llama-cpp\models\Ternary-Bonsai-2-27B-PTQ1_0.gguf`; ftype: `PTQ1_0 - 1.75 bpw ternary (group 128)`
- Exact launch flags (given by the maintainer; `/props` does not show them), on the prism-llama-cpp fork: `-ngl 999 --load-mode mlock --cache-type-k q4_0 --cache-type-v q4_0 -fa on -c 65536`.
- KV cache: Bonsai ran with a 4-bit (q4_0) KV cache. Every run3 model used turbo4 (K) and turbo3 (V) KV. This is a recorded difference in setup only; no effect on any result is claimed.
- `n_ctx` 65536, `total_slots` 4.
- Server-default generation settings (overridden per request): temperature 1.0, top_p 0.949999988079071, top_k 20, min_p 0.05000000074505806, presence_penalty 0.0, repeat_penalty 1.0, `reasoning_format` `none`, `max_tokens` -1.
- Other server clients: the maintainer could use the server at the same time; the server has 4 slots.

```json
{
  "total_slots": 4,
  "model_alias": "C:\\Users\\Ashman Das\\Documents\\prism-llama-cpp\\models\\Ternary-Bonsai-2-27B-PTQ1_0.gguf",
  "model_ftype": "PTQ1_0 - 1.75 bpw ternary (group 128)",
  "model_path": "C:\\Users\\Ashman Das\\Documents\\prism-llama-cpp\\models\\Ternary-Bonsai-2-27B-PTQ1_0.gguf",
  "modalities": {
    "vision": false,
    "video": false,
    "audio": false
  },
  "media_marker": "<__media_hCFEczybWOZfvwNlURlzixrZhLDXCW85__>",
  "endpoint_slots": true,
  "endpoint_props": false,
  "endpoint_metrics": false,
  "ui": true,
  "ui_settings": {},
  "chat_template_caps": {
    "supports_object_arguments": true,
    "supports_parallel_tool_calls": true,
    "supports_preserve_reasoning": true,
    "supports_reasoning_effort": true,
    "supports_string_content": true,
    "supports_system_role": true,
    "supports_tool_calls": true,
    "supports_tools": true,
    "supports_typed_content": true
  },
  "bos_token": "<|endoftext|>",
  "eos_token": "<|im_end|>",
  "build_info": "b12128-d7b289683",
  "is_sleeping": false,
  "cors_proxy_enabled": false
}
```

## 3. Thinking switch (from the embedded chat template in `/props`)

The variable is `enable_thinking`. Template text, quoted:

```jinja
{%- if enable_thinking is undefined or enable_thinking is true %}
    {%- set resolved_reasoning_effort = reasoning_effort|default('xhigh') %}
    ...
{%- if add_generation_prompt %}
    {{- '<|im_start|>assistant\n' }}
    {%- if enable_thinking is defined and enable_thinking is false %}
        {{- '<think>\n\n</think>\n\n' }}
    {%- else %}
        {{- '<think>\n' }}
    {%- endif %}
{%- endif %}
```

With `enable_thinking: true` the template also adds a system line starting `Reasoning effort is set to xhigh.` (it is the default), so the thinking-on prompt is longer than the thinking-off prompt (467 against 427 prompt tokens on s1 in a probe). Thinking-off runs with non-empty `reasoning_content`: 0 of 20. The server returns thinking separately in `choices[0].message.reasoning_content`, so no tag cutting was needed.

## 4. Summary matrix

Same columns as run3.

| Model | Schema | Runs | Clean | Repaired | Salvaged | Partial | Unusable | Record Recall | Field Accuracy | Distinct | finish_reason | Mean Think Tok | Max Think Tok | Mean Out Tok | Max Out Tok |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Bonsai-27B (thinking off) | `s1_flat` | 5 | 5 | 0 | 0 | 0 | 0 | 100.0% (25/25) | 81.6% | 3/5 | stop: 5 | 0 | 0 | 103 | 137 |
| Bonsai-27B (thinking off) | `s2_nested_fixed` | 5 | 3 | 0 | 0 | 0 | 2 | 60.0% (15/25) | 78.7% | 5/5 | stop: 5 | 0 | 0 | 134 | 154 |
| Bonsai-27B (thinking off) | `s3_nested_variable` | 5 | 0 | 0 | 0 | 0 | 5 | 0.0% (0/25) | n/a | 5/5 | stop: 5 | 0 | 0 | 240 | 324 |
| Bonsai-27B (thinking off) | `s4_two_class_code` | 5 | 0 | 0 | 3 | 0 | 2 | 30.0% (9/30) | 100.0% | 4/5 | stop: 5 | 0 | 0 | 123 | 131 |
| Bonsai-27B (thinking on) | `s1_flat` | 5 | 5 | 0 | 0 | 0 | 0 | 100.0% (25/25) | 99.2% | 2/5 | stop: 5 | 672 | 1303 | 137 | 137 |
| Bonsai-27B (thinking on) | `s2_nested_fixed` | 5 | 5 | 0 | 0 | 0 | 0 | 100.0% (25/25) | 99.2% | 2/5 | stop: 5 | 2147 | 5096 | 139 | 139 |
| Bonsai-27B (thinking on) | `s3_nested_variable` | 5 | 5 | 0 | 0 | 0 | 0 | 100.0% (25/25) | 97.6% | 3/5 | stop: 5 | 6576 | 7453 | 317 | 320 |
| Bonsai-27B (thinking on) | `s4_two_class_code` | 5 | 4 | 0 | 1 | 0 | 0 | 90.0% (27/30) | 84.8% | 4/5 | stop: 5 | 8160 | 20275 | 121 | 133 |

Definitions are those of `research/r3-real-run3.md` section 5: recall is decoded over expected records; field accuracy is exact-match, positional, pooled over decoded records; outcomes are clean, repaired, salvaged (records recovered next to reported bad lines), partial (clean or repaired but fewer records than expected) and unusable (nothing decoded).

## 5. finish_reason, thinking and output tokens by mode

| Mode | Runs | finish_reason | Think tok mean | Think tok max | Out tok mean | Out tok max | Timeouts |
|---|---|---|---|---|---|---|---|
| Bonsai-27B (thinking off) | 20 | stop: 20 | 0 | 0 | 150 | 324 | 0 |
| Bonsai-27B (thinking on) | 20 | stop: 20 | 4389 | 20275 | 179 | 320 | 0 |

## 6. Comparison with every run3 model

Run3 cells are copied from `research/r3-real-run3.md` section 5 (recall, then field accuracy). Where run3 split Gemma thinking-on rows into reasoned and unreasoned runs, both cells are shown. Bonsai cells are this run. Sampling differs on purpose (section 1).

| Model | s1_flat recall / field acc | s2_nested_fixed recall / field acc | s3_nested_variable recall / field acc | s4_two_class_code recall / field acc |
|---|---|---|---|---|
| Llama-3.2-3B (thinking off) (run3) | 80.0% (20/25) / 74.0% | 40.0% (10/25) / 32.0% | 8.0% (2/25) / 0.0% | 0.0% (0/30) / n/a |
| Gemma-4-E2B (thinking off) (run3) | 100.0% (25/25) / 100.0% | 16.0% (4/25) / 40.0% | 0.0% (0/25) / n/a | 0.0% (0/30) / n/a |
| Gemma-4-E2B (thinking on) (run3) | reasoned: 100.0% (20/20) / 97.0% ; unreasoned: 100.0% (5/5) / 76.0% | 80.0% (20/25) / 99.0% | 20.0% (5/25) / 47.6% | reasoned: 0.0% (0/6) / n/a ; unreasoned: 0.0% (0/24) / n/a |
| LFM2.5-8B (thinking on) (run3) | 80.0% (20/25) / 80.0% | 0.0% (0/25) / n/a | 0.0% (0/25) / n/a | 0.0% (0/30) / n/a |
| Mellum2.1-12B (thinking on) (run3) | 100.0% (25/25) / 96.0% | 100.0% (25/25) / 96.8% | 60.0% (15/25) / 81.0% | 20.0% (6/30) / 100.0% |
| **Bonsai-27B (thinking off)** | 100.0% (25/25) / 81.6% | 60.0% (15/25) / 78.7% | 0.0% (0/25) / n/a | 30.0% (9/30) / 100.0% |
| **Bonsai-27B (thinking on)** | 100.0% (25/25) / 99.2% | 100.0% (25/25) / 99.2% | 100.0% (25/25) / 97.6% | 90.0% (27/30) / 84.8% |

## 7. Token counts against `usage.completion_tokens`

`usage.completion_tokens` minus (`/tokenize` of thinking + `/tokenize` of answer), over 40 runs: min 1, max 12. Distribution (difference: runs): 1: 20, 3: 16, 4: 2, 5: 1, 12: 1.

## 8. Doctor repairs by kind

| Mode | Fix | Occurrences |
|---|---|---|
| bonsai_off | `MissingFinalPipe` | 9 |
| bonsai_on | `MissingFinalPipe` | 3 |

## 9. Doctor bad lines by problem

| Mode | Problem | Occurrences |
|---|---|---|
| bonsai_off | `dropped as child of line 7` | 12 |
| bonsai_off | `with several outside classes, data before the first schema line is an error` | 12 |
| bonsai_off | `dropped as child of line 1` | 10 |
| bonsai_off | `dropped as child of line 18` | 10 |
| bonsai_off | `expected 4 columns, found 6` | 7 |
| bonsai_off | `dropped as child of line 14` | 6 |
| bonsai_off | `dropped as child of line 24` | 6 |
| bonsai_off | `invalid escape sequence '\n'` | 6 |
| bonsai_off | `expected 5 columns, found 6` | 5 |
| bonsai_off | `expected 5 columns, found 4` | 5 |
| bonsai_off | `expected 4 columns, found 5` | 5 |
| bonsai_off | `invalid escape sequence '\r'` | 3 |
| bonsai_off | `expected 4 columns, found 10` | 2 |
| bonsai_off | `expected 4 columns, found 11` | 2 |
| bonsai_off | `expected 4 columns, found 7` | 2 |
| bonsai_off | `expected 4 columns, found 9` | 1 |
| bonsai_off | `expected 4 columns, found 8` | 1 |
| bonsai_off | `expected 4 columns, found 14` | 1 |
| bonsai_on | `expected 3 columns, found 2` | 6 |
| bonsai_on | `expected 3 columns, found 1` | 3 |

### Auto-picked quotes per problem (first two each)

**bonsai_off** `dropped as child of line 1`

- `s3_nested_variable` run 4, line 2:

  ```text
  > notes are from abstract only|
  ```

- `s3_nested_variable` run 4, line 3:

  ```text
  >> abstract mentions 32k limit|
  ```

**bonsai_off** `dropped as child of line 14`

- `s3_nested_variable` run 4, line 15:

  ```text
  > outlier channels preserved|
  ```

- `s3_nested_variable` run 4, line 16:

  ```text
  >> per-head scales help|
  ```

**bonsai_off** `dropped as child of line 18`

- `s3_nested_variable` run 4, line 19:

  ```text
  > optimal ratio is 20 to 1|
  ```

- `s3_nested_variable` run 4, line 20:

  ```text
  >> tested across 400 models|
  ```

**bonsai_off** `dropped as child of line 24`

- `s3_nested_variable` run 4, line 25:

  ```text
  > scaling curve is smooth|
  ```

- `s3_nested_variable` run 4, line 26:

  ```text
  >> tested up to 175B|
  ```

**bonsai_off** `dropped as child of line 7`

- `s3_nested_variable` run 4, line 8:

  ```text
  > tested on GLUE and Wikisql|
  ```

- `s3_nested_variable` run 4, line 9:

  ```text
  >> rank 4 sufficient|
  ```

**bonsai_off** `expected 4 columns, found 10`

- `s3_nested_variable` run 1, line 1:

  ```text
  2405.05254|refuted|~|Rope scaling hurts long ctx||notes are from abstract only|abstract mentions 32k limit|appendix B clarifies|table 3 contradicts|llama 7b tested only|
  ```

- `s3_nested_variable` run 2, line 2:

  ```text
  supported|2106.09685|2021|tested on GLUE and Wikisql|rank 4 sufficient|higher ranks show diminishing returns|memory footprint reduced 3x|optimizer state minimal|checkpoint size under 50MB||
  ```

**bonsai_off** `expected 4 columns, found 11`

- `s3_nested_variable` run 1, line 2:

  ```text
  2106.09685|supported|~|LoRA matches full finetune|tested on GLUE and Wikisql|rank 4 sufficient|higher ranks show diminishing returns|memory footprint reduced 3x|optimizer state minimal|checkpoint size under 50MB||
  ```

- `s3_nested_variable` run 2, line 1:

  ```text
  text|refuted|2405.05254|2024|\rnotes are from abstract only|abstract mentions 32k limit||appendix B clarifies||table 3 contradicts|llama 7b tested only|
  ```

**bonsai_off** `expected 4 columns, found 14`

- `s3_nested_variable` run 3, line 1:

  ```text
  text|refuted|2405.05254|2024|~|notes are from abstract only|abstract mentions 32k limit||appendix B clarifies||table 3 contradicts||llama 7b tested only||
  ```

**bonsai_off** `expected 4 columns, found 5`

- `s3_nested_variable` run 4, line 1:

  ```text
  Rope scaling hurts long ctx|refuted|2405.05254|2024|~|
  ```

- `s3_nested_variable` run 4, line 7:

  ```text
  LoRA matches full finetune|supported|2106.09685|2021|~|
  ```

**bonsai_off** `expected 4 columns, found 6`

- `s3_nested_variable` run 2, line 3:

  ```text
  refuted|2406.12345|2024|outlier channels preserved|per-head scales help|perplexity intact|
  ```

- `s3_nested_variable` run 2, line 5:

  ```text
  supported|2005.14165|2020|scaling curve is smooth|tested up to 175B|in-context learning observed|
  ```

**bonsai_off** `expected 4 columns, found 7`

- `s3_nested_variable` run 1, line 3:

  ```text
  2406.12345|refuted|~|KV cache quant loses 2x acc|outlier channels preserved|per-head scales help|perplexity intact|
  ```

- `s3_nested_variable` run 1, line 5:

  ```text
  2005.14165|supported|~|Language models few-shot|scaling curve is smooth|tested up to 175B|in-context learning observed|
  ```

**bonsai_off** `expected 4 columns, found 8`

- `s3_nested_variable` run 2, line 4:

  ```text
  supported|2203.15556|2022|optimal ratio is 20 to 1|tested across 400 models|cosine schedule used|kaplan tokens were suboptimal|fixed learning rate flawed|
  ```

**bonsai_off** `expected 4 columns, found 9`

- `s3_nested_variable` run 1, line 4:

  ```text
  2203.15556|supported|~|Chinchilla tokens beat params|optimal ratio is 20 to 1|tested across 400 models|cosine schedule used|kaplan tokens were suboptimal|fixed learning rate flawed|
  ```

**bonsai_off** `expected 5 columns, found 4`

- `s2_nested_fixed` run 5, line 1:

  ```text
  Rope scaling hurts long ctx|refuted|text{2405.05254, 2024}|1|
  ```

- `s2_nested_fixed` run 5, line 2:

  ```text
  Attention is all you need|supported|text{1706.03762, 2017}|1|
  ```

**bonsai_off** `expected 5 columns, found 6`

- `s2_nested_fixed` run 2, line 1:

  ```text
  Rope scaling hurts long ctx|refuted|text|2405.05254|2024|1|
  ```

- `s2_nested_fixed` run 2, line 2:

  ```text
  Attention is all you need|supported|text|1706.03762|2017|1|
  ```

**bonsai_off** `invalid escape sequence '\n'`

- `s4_two_class_code` run 2, line 6:

  ```text
  lib.rs|\rpub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}|10|
  ```

- `s4_two_class_code` run 2, line 7:

  ```text
  util.rs|\rpub fn greet(name: &str) -> String {\n    format!("Hello, {}!", name)\n}|5|
  ```

**bonsai_off** `invalid escape sequence '\r'`

- `s4_two_class_code` run 3, line 6:

  ```text
  lib.rs|pub fn add(a: i32, b: i32) -> i32 {\r    a + b\r}|10|
  ```

- `s4_two_class_code` run 3, line 7:

  ```text
  util.rs|pub fn greet(name: &str) -> String {\r    format!("Hello, {}!", name)\r}|5|
  ```

**bonsai_off** `with several outside classes, data before the first schema line is an error`

- `s4_two_class_code` run 1, line 1:

  ```text
  schema:file|lib.rs|src/lib.rs|
  ```

- `s4_two_class_code` run 1, line 2:

  ```text
  schema:patch|lib.rs|pub fn add(a: i32, b: i32) -> i32 {\r    a + b\r}|10|
  ```

**bonsai_on** `expected 3 columns, found 1`

- `s4_two_class_code` run 1, line 7:

  ```text
      a + b|
  ```

- `s4_two_class_code` run 1, line 10:

  ```text
      format!("Hello, {}!", name)|
  ```

**bonsai_on** `expected 3 columns, found 2`

- `s4_two_class_code` run 1, line 6:

  ```text
  lib.rs|\rpub fn add(a: i32, b: i32) -> i32 {|
  ```

- `s4_two_class_code` run 1, line 8:

  ```text
  }|10|
  ```

## 10. New kinds of mistake

Quotes are copied from `bench/r3/real/run3_bonsai/raw/`. Observations only.

1. Thinking off, s2: the placeholder word from the prompt's example row is written as a value, and other numbers appear where the id and year belong. `bonsai_off_s2_nested_fixed_run1` (all five lines look like this):
   ```text
   Rope scaling hurts long ctx|refuted|text|7|1|
   Attention is all you need|supported|text|3|1|
   ```
   Run 2 writes `text` as a value on every line too: `Rope scaling hurts long ctx|refuted|text|2405.05254|2024|1|`. The prompt example is `text|supported|text|7|~|`.
2. Thinking off, s2: the `text` column is dropped. `bonsai_off_s2_nested_fixed_run3` line 1: `Rope scaling hurts long ctx|refuted|2405.05254|2024|0|` (the schema has 5 columns; the doctor reports `expected 5 columns, found 4`).
3. Thinking off, s3: a whole record on one line, list items inlined without `>` markers, and the nested source written as `~`. `bonsai_off_s3_nested_variable_run1` line 1: `2405.05254|refuted|~|Rope scaling hurts long ctx||notes are from abstract only|abstract mentions 32k limit|appendix B clarifies|table 3 contradicts|llama 7b tested only|`
4. Thinking off, s3: the field order is changed (text, verdict, id, year) and `>` lines are written, but some child lines lack the closing pipe. `bonsai_off_s3_nested_variable_run4`: `Rope scaling hurts long ctx|refuted|2405.05254|2024|~|` then `>> abstract mentions 32k limit` (no final `|`).
5. Thinking off, s4: the code is written with `\r` between lines, or with the two-character sequence `\n`, where the prompt asks for a leading `\r` and then real line breaks. `bonsai_off_s4_two_class_code_run1`: `schema:patch|lib.rs|pub fn add(a: i32, b: i32) -> i32 {\r    a + b\r}|10|`. `bonsai_off_s4_two_class_code_run2`: `lib.rs|\rpub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}|10|` (doctor: `invalid escape sequence '\n'`). Run 5 also writes `\"Hello, {}!\"` with backslash-escaped quotes.
6. Thinking off, s4: runs 1 and 4 put the schema name and data on one line (`schema:file|lib.rs|src/lib.rs|`); the prompt says `schema:name` on its own line. Doctor: `with several outside classes, data before the first schema line is an error`.
7. Thinking on, s4 run 1: a pipe closes the first line of the code, and the rest follows on lines of its own:
   ```text
   lib.rs|\rpub fn add(a: i32, b: i32) -> i32 {|
       a + b
   }|10|
   ```
   Doctor: `expected 3 columns, found 2` and `expected 3 columns, found 1`. Run 5 writes `\r` and then a real line break (`lib.rs|\r` newline `pub fn add(...`), so the decoded code starts with a newline.
8. Thinking on, s4 run 3: `schema:file` and `schema:patch` blocks alternate per file instead of being grouped (all other runs group them).
9. Thinking on, s1: one wrong field in 125 over the five runs (run 5, field accuracy 96.0%). Thinking on gives s1, s2 and s3 clean in 15 of 15 runs.
10. No raw output in either mode contains a code fence. All 40 runs ended with `finish_reason` `stop` and none of the 20 thinking-on runs ended inside the thinking text (every thinking-on run has a non-empty answer).

## 11. Per-run results

| Mode | Schema | Run | Seed | Outcome | Records | Repairs | Bad lines | finish_reason | Think tok | Out tok | Field acc | Latency s |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| bonsai_off | `s1_flat` | 1 | 42 | clean | 5/5 | 0 | 0 | stop | 0 | 137 | 96.0% | 4.5 |
| bonsai_off | `s1_flat` | 2 | 43 | clean | 5/5 | 0 | 0 | stop | 0 | 94 | 76.0% | 2.83 |
| bonsai_off | `s1_flat` | 3 | 44 | clean | 5/5 | 0 | 0 | stop | 0 | 94 | 76.0% | 2.84 |
| bonsai_off | `s1_flat` | 4 | 45 | clean | 5/5 | 0 | 0 | stop | 0 | 94 | 80.0% | 2.83 |
| bonsai_off | `s1_flat` | 5 | 46 | clean | 5/5 | 0 | 0 | stop | 0 | 94 | 80.0% | 2.82 |
| bonsai_off | `s2_nested_fixed` | 1 | 42 | clean | 5/5 | 0 | 0 | stop | 0 | 79 | 56.0% | 3.11 |
| bonsai_off | `s2_nested_fixed` | 2 | 43 | unusable | 0/5 | 0 | 5 | stop | 0 | 149 | n/a | 4.42 |
| bonsai_off | `s2_nested_fixed` | 3 | 44 | clean | 5/5 | 0 | 0 | stop | 0 | 139 | 100.0% | 4.13 |
| bonsai_off | `s2_nested_fixed` | 4 | 45 | clean | 5/5 | 0 | 0 | stop | 0 | 149 | 80.0% | 4.41 |
| bonsai_off | `s2_nested_fixed` | 5 | 46 | unusable | 0/5 | 0 | 5 | stop | 0 | 154 | n/a | 4.56 |
| bonsai_off | `s3_nested_variable` | 1 | 42 | unusable | 0/5 | 0 | 5 | stop | 0 | 261 | n/a | 8.77 |
| bonsai_off | `s3_nested_variable` | 2 | 43 | unusable | 0/5 | 0 | 5 | stop | 0 | 243 | n/a | 7.13 |
| bonsai_off | `s3_nested_variable` | 3 | 44 | unusable | 0/5 | 0 | 1 | stop | 0 | 57 | n/a | 1.77 |
| bonsai_off | `s3_nested_variable` | 4 | 45 | unusable | 0/5 | 9 | 27 | stop | 0 | 315 | n/a | 9.19 |
| bonsai_off | `s3_nested_variable` | 5 | 46 | unusable | 0/5 | 0 | 27 | stop | 0 | 324 | n/a | 9.46 |
| bonsai_off | `s4_two_class_code` | 1 | 42 | unusable | 0/6 | 0 | 6 | stop | 0 | 131 | n/a | 4.76 |
| bonsai_off | `s4_two_class_code` | 2 | 43 | salvaged | 3/6 | 0 | 3 | stop | 0 | 118 | 100.0% | 3.9 |
| bonsai_off | `s4_two_class_code` | 3 | 44 | salvaged | 3/6 | 0 | 3 | stop | 0 | 115 | 100.0% | 3.81 |
| bonsai_off | `s4_two_class_code` | 4 | 45 | unusable | 0/6 | 0 | 6 | stop | 0 | 131 | n/a | 4.31 |
| bonsai_off | `s4_two_class_code` | 5 | 46 | salvaged | 3/6 | 0 | 3 | stop | 0 | 122 | 100.0% | 3.72 |
| bonsai_on | `s1_flat` | 1 | 42 | clean | 5/5 | 0 | 0 | stop | 398 | 137 | 100.0% | 15.1 |
| bonsai_on | `s1_flat` | 2 | 43 | clean | 5/5 | 0 | 0 | stop | 448 | 137 | 100.0% | 16.53 |
| bonsai_on | `s1_flat` | 3 | 44 | clean | 5/5 | 0 | 0 | stop | 453 | 137 | 100.0% | 16.71 |
| bonsai_on | `s1_flat` | 4 | 45 | clean | 5/5 | 0 | 0 | stop | 759 | 137 | 100.0% | 25.21 |
| bonsai_on | `s1_flat` | 5 | 46 | clean | 5/5 | 0 | 0 | stop | 1303 | 137 | 96.0% | 40.55 |
| bonsai_on | `s2_nested_fixed` | 1 | 42 | clean | 5/5 | 0 | 0 | stop | 930 | 139 | 100.0% | 31.78 |
| bonsai_on | `s2_nested_fixed` | 2 | 43 | clean | 5/5 | 0 | 0 | stop | 3223 | 139 | 100.0% | 96.36 |
| bonsai_on | `s2_nested_fixed` | 3 | 44 | clean | 5/5 | 0 | 0 | stop | 5096 | 139 | 100.0% | 149.08 |
| bonsai_on | `s2_nested_fixed` | 4 | 45 | clean | 5/5 | 0 | 0 | stop | 890 | 139 | 96.0% | 30.79 |
| bonsai_on | `s2_nested_fixed` | 5 | 46 | clean | 5/5 | 0 | 0 | stop | 598 | 139 | 100.0% | 21.89 |
| bonsai_on | `s3_nested_variable` | 1 | 42 | clean | 5/5 | 0 | 0 | stop | 7453 | 319 | 88.1% | 225.79 |
| bonsai_on | `s3_nested_variable` | 2 | 43 | clean | 5/5 | 0 | 0 | stop | 5098 | 314 | 100.0% | 154.29 |
| bonsai_on | `s3_nested_variable` | 3 | 44 | clean | 5/5 | 0 | 0 | stop | 7311 | 314 | 100.0% | 217.58 |
| bonsai_on | `s3_nested_variable` | 4 | 45 | clean | 5/5 | 0 | 0 | stop | 6934 | 320 | 100.0% | 207.72 |
| bonsai_on | `s3_nested_variable` | 5 | 46 | clean | 5/5 | 0 | 0 | stop | 6084 | 320 | 100.0% | 182.24 |
| bonsai_on | `s4_two_class_code` | 1 | 42 | salvaged | 3/6 | 3 | 9 | stop | 4142 | 117 | 100.0% | 121.47 |
| bonsai_on | `s4_two_class_code` | 2 | 43 | clean | 6/6 | 0 | 0 | stop | 5359 | 117 | 100.0% | 157.67 |
| bonsai_on | `s4_two_class_code` | 3 | 44 | clean | 6/6 | 0 | 0 | stop | 7492 | 133 | 33.3% | 220.97 |
| bonsai_on | `s4_two_class_code` | 4 | 45 | clean | 6/6 | 0 | 0 | stop | 3532 | 117 | 100.0% | 105.65 |
| bonsai_on | `s4_two_class_code` | 5 | 46 | clean | 6/6 | 0 | 0 | stop | 20275 | 120 | 100.0% | 600.78 |
