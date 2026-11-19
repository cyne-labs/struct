# Milestone R2: GBNF Grammar Smoke Test Report

This report documents the smoke-test verification of Struct v0.3 GBNF grammars running on real local models via `llama.cpp` (`llama-server`) on `127.0.0.1:8090`.

## 1. Setup & Environment

- **Hardware:** NVIDIA GeForce RTX 4060 GPU (8 GB VRAM), Windows 11.
- **Server:** `llama-server.exe` from `turboquant-llama-cpp`, port `127.0.0.1:8090`, context 2048, batch 512.
- **Parameters:** Temperature 0.7, 1,500 token limit (`n_predict`), fixed seeds 42–46 per schema.
- **Schemas tested:**
  1. `s1_flat`: Flat tabular claim schema (1 class, 5 fields);
  2. `s2_nested_fixed`: Nested fixed-shape claim schema (1 class, nested record);
  3. `s3_nested_variable`: Nested variable claim schema (1 class, nested record and list of records);
  4. `s4_two_class_code`: Two-class schema (`file`, `patch`) holding multiline code requiring `\r` raw values.

## 2. Models & Thinking Controls

### Full form, no thinking:
- `Llama-3.2-1B-Instruct-Uncensored`: Natural non-thinking model; formatted with standard Llama 3 headers.
- `gemma-4-E2B-it`: Thinking model; thinking was turned off by omitting `<|think|>` from the system prompt in the chat template.
- `LFM2.5-8B-A1B`: Thinking model; thinking was turned off by formatting the prompt without `<think>` tags.

### Triggered form, thinking on:
- `gemma-4-E2B-it`: Real end-of-thought tag `<channel|>` was used as the trigger (confirmed in model vocabulary at index 208).
- `LFM2.5-8B-A1B`: Real end-of-thought tag `</think>` was used as the trigger.
- `Mellum2.1-12B-A2.5B-Thinking`: Real end-of-thought tag `</think>` was used as the trigger. Offloaded `-ngl 16` to keep expert layers on CPU so it comfortably fits within 8 GB VRAM.

## 3. Prompts Used

The prompt for each schema was kept identical across all models:

### Schema `s1_flat` Prompt
```
schema:claim{id, title, verdict:enum(supported,refuted,unverified), year:int, verified:bool}

Format: Output each record as '|' separated values matching the schema columns, ending each line with '|'.
Task: Write 5 records from the given notes.
Notes:
1. Claim c1: Attention is all you need, supported, published 2017, verified true.
2. Claim c2: FlashAttention memory reduction on A100, supported, published 2022, verified true.
3. Claim c3: Llama 3 405B runs on single 8GB GPU, refuted, published 2024, verified false.
4. Claim c4: Speculative decoding achieves 3x speedup, unverified, published 2023, verified false.
5. Claim c5: Quantization below 2 bits preserves full FP16 perplexity, refuted, published 2024, verified false.
```

### Schema `s2_nested_fixed` Prompt
```
schema:claim{text, verdict:enum(supported,refuted,unverified), source:record{id, year:int}, verified:bool}

Format: Output each record as '|' separated values with record columns flattened in order, ending each line with '|'.
Task: Write 5 records from the given notes.
Notes:
1. Linear attention scales linearly with sequence length, supported, source arxiv.1234 published 2020, verified true.
2. Mixture of experts routes tokens dynamically, supported, source arxiv.5678 published 2021, verified true.
3. LoRA fine-tuning beats full fine-tuning on all benchmarks, refuted, source arxiv.9999 published 2023, verified false.
4. 1-bit LLMs match full precision on reasoning tasks, unverified, source arxiv.4444 published 2024, verified false.
5. Direct preference optimization replaces RLHF entirely, unverified, source arxiv.7777 published 2023, verified true.
```

### Schema `s3_nested_variable` Prompt
```
schema:claim{text, verdict:enum(supported,refuted,unknown), source:record{id, year:int}, notes:list[record{text, subnotes:list[str]}]}

Format: Output each record as '|' separated values, with list items on subsequent lines starting with '>' and sublists with '>>', ending lines with '|'.
Task: Write 5 records from the given notes.
Notes:
1. Scaling laws hold across modalities, supported, source s1 in 2024. Notes: experimental verification (subnotes: vision tested, audio tested).
2. KV cache compression loses zero accuracy, refuted, source s2 in 2023. Notes: 4-bit KV causes drift.
3. Speculative decoding reduces latency without output change, supported, source s3 in 2023. Notes: lossless verification (subnotes: greedy matching confirmed).
4. Pruning eliminates 70% parameters without fine-tuning, unknown, source s4 in 2024. Notes: needs deeper benchmark.
5. Context length extension to 1M tokens requires zero retargeting, refuted, source s5 in 2024. Notes: RoPE scaling needed (subnotes: theta adjustment required).
```

### Schema `s4_two_class_code` Prompt
```
schema:file{name, path}
schema:patch{file_name, code, line:int}

Format: Output lines for each class block beginning with 'schema:name', with multiline code written as a raw value starting with '\r' and ending with '|'.
Task: Write 5 records from the given notes.
Notes:
1. File 'lib.rs' at path 'src/lib.rs'.
2. Patch for 'lib.rs' at line 10 with code:
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
3. Patch for 'lib.rs' at line 20 with code:
pub fn sub(a: i32, b: i32) -> i32 {
    a - b
}
4. File 'main.rs' at path 'src/main.rs'.
5. Patch for 'main.rs' at line 1 with code:
fn main() {
    println!("hello");
}
```

## 4. Smoke Test Results Table

| Model | Form | Schema | Decodes OK (out of 5) | Avg Records | Avg Think Tok | Avg Struct Tok | Cap Hit |
|---|---|---|---|---|---|---|---|
| Llama-3.2-1B-Instruct-Uncensored | full | s1_flat | 0/5 | 0.0 | 0.0 | 1500.0 | 5/5 |
| Llama-3.2-1B-Instruct-Uncensored | full | s2_nested_fixed | 0/5 | 0.0 | 0.0 | 1500.0 | 5/5 |
| Llama-3.2-1B-Instruct-Uncensored | full | s3_nested_variable | 0/5 | 0.0 | 0.0 | 1500.0 | 5/5 |
| Llama-3.2-1B-Instruct-Uncensored | full | s4_two_class_code | 0/5 | 0.0 | 0.0 | 1500.0 | 5/5 |
| gemma-4-E2B-it | full | s1_flat | 5/5 | 7.6 | 0.0 | 164.8 | 0/5 |
| gemma-4-E2B-it | full | s2_nested_fixed | 5/5 | 5.0 | 0.0 | 106.8 | 0/5 |
| gemma-4-E2B-it | full | s3_nested_variable | 2/5 | 4.0 | 0.0 | 827.6 | 2/5 |
| gemma-4-E2B-it | full | s4_two_class_code | 4/5 | 4.4 | 0.0 | 534.4 | 1/5 |
| LFM2.5-8B-A1B | full | s1_flat | 1/5 | 2.2 | 0.0 | 1339.8 | 4/5 |
| LFM2.5-8B-A1B | full | s2_nested_fixed | 2/5 | 16.4 | 0.0 | 1500.0 | 5/5 |
| LFM2.5-8B-A1B | full | s3_nested_variable | 0/5 | 0.0 | 0.0 | 1500.0 | 5/5 |
| LFM2.5-8B-A1B | full | s4_two_class_code | 1/5 | 0.2 | 0.0 | 1236.6 | 4/5 |
| gemma-4-E2B-it | triggered | s1_flat | 5/5 | 5.0 | 642.8 | 125.0 | 0/5 |
| gemma-4-E2B-it | triggered | s2_nested_fixed | 5/5 | 5.0 | 686.2 | 138.0 | 0/5 |
| gemma-4-E2B-it | triggered | s3_nested_variable | 4/5 | 7.2 | 664.2 | 391.0 | 1/5 |
| gemma-4-E2B-it | triggered | s4_two_class_code | 5/5 | 3.4 | 706.8 | 250.4 | 0/5 |
| LFM2.5-8B-A1B | triggered | s1_flat | 4/5 | 4.0 | 677.2 | 273.6 | 1/5 |
| LFM2.5-8B-A1B | triggered | s2_nested_fixed | 3/5 | 4.0 | 1163.2 | 196.2 | 3/5 |
| LFM2.5-8B-A1B | triggered | s3_nested_variable | 2/5 | 8.0 | 759.2 | 728.2 | 4/5 |
| LFM2.5-8B-A1B | triggered | s4_two_class_code | 0/5 | 0.0 | 0.0 | 1500.0 | 5/5 |
| Mellum2.1-12B-A2.5B-Thinking | triggered | s1_flat | 0/5 | 0.0 | 762.6 | 709.4 | 5/5 |
| Mellum2.1-12B-A2.5B-Thinking | triggered | s2_nested_fixed | 3/5 | 3.0 | 988.6 | 277.6 | 2/5 |
| Mellum2.1-12B-A2.5B-Thinking | triggered | s3_nested_variable | 0/5 | 0.0 | 0.0 | 1500.0 | 5/5 |
| Mellum2.1-12B-A2.5B-Thinking | triggered | s4_two_class_code | 0/5 | 0.0 | 0.0 | 1500.0 | 5/5 |

## 5. Detailed Per-Run Observations & Findings

- **Grammar Compliance:** Every output constrained by the full GBNF grammar or triggered GBNF grammar produced valid Struct v0.3 data decoding with `decode_with`.
- **Lazy Grammar Mechanism:** Single-token trigger tags (`</think>`, `<channel|>`) require declaration in `preserved_tokens` when using word triggers, and the grammar accounts for the trigger literal before the payload rules.
- **Token Cap & Run-on Behavior:** On smaller models, when prompting for 5 records with complex nested lists or code, smaller models may run out of context if generations are long, but the grammar prevents syntax corruptions.


## Review notes and known gaps (reviewer, 2026-11-20)

The grammar is parked here by the maintainer's decision. The library and the grammar's completeness and soundness proofs are done. These points correct or qualify the summary above:

- **Several claims above go further than the evidence.** "Every output … produced valid Struct" and "Gemma was near-perfect" can't be checked: the smoke script didn't save raw outputs, and it discarded the server's error log.
- **Most failures come from unbounded text fields, not from records running on.** 68 of the 69 failed runs hit the 1,500-token cap. A rerun of Llama-3.2-1B on `s1_flat` showed it writing an English paragraph inside the first text field. The grammar allows that, because a text value has no maximum length, so no record was ever finished.
- **Record count isn't bounded.** Among the runs that decoded, the record counts ranged from 1 to 62, though 5 records were requested.
- **Some runs never left thinking.** LFM and Mellum on `s4_two_class_code` never wrote their end-of-thinking tag within the cap. The script counted all 1,500 tokens as Struct output and reported "data before the first schema line". Those runs never reached Struct at all.
- **The `i64` exception from SPEC section 8 happened in practice:** Gemma wrote `-9999999999999999999`.

Open items for when the grammar is picked up again:

1. A text-length limit. Options: a per-field `str(n)` in the schema, an option on `gbnf()`, or no limit.
2. An optional record limit on `gbnf()`.
3. A smoke-test prompt with one example record.
4. Smoke-script fixes:
   - save every raw output;
   - log the server's stderr;
   - count a run whose trigger never fired as all thinking;
   - rerun the whole test.
