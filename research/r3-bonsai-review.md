# Independent Review: Struct Doctor Real-Model Output Report (Bonsai Run 3)

**Reviewer:** Independent Verification Agent  
**Date:** 2026-11-21  
**Target Report:** [research/r3-real-bonsai.md](file:///C:/Users/Ashman%20Das/Documents/Struct/research/r3-real-bonsai.md)  
**Evaluator Script:** [bench/r3/evaluate_bonsai.py](file:///C:/Users/Ashman%20Das/Documents/Struct/bench/r3/evaluate_bonsai.py)  
**Collector Script:** [bench/r3/collect_bonsai.py](file:///C:/Users/Ashman%20Das/Documents/Struct/bench/r3/collect_bonsai.py)  
**Benchmark Artifacts:** [bench/r3/real/run3_bonsai/](file:///C:/Users/Ashman%20Das/Documents/Struct/bench/r3/real/run3_bonsai/)  
**Baseline Report:** [research/r3-real-run3.md](file:///C:/Users/Ashman%20Das/Documents/Struct/research/r3-real-run3.md)  
**Authority:** [SPEC.md](file:///C:/Users/Ashman%20Das/Documents/Struct/SPEC.md) (v0.3), [milestones/RULES.md](file:///C:/Users/Ashman%20Das/Documents/Struct/milestones/RULES.md), and [milestones/R3-bonsai-review.md](file:///C:/Users/Ashman%20Das/Documents/Struct/milestones/R3-bonsai-review.md)

---

## Executive Summary

| Item | Focus | Verdict | Key Finding |
|---|---|---|---|
| **1** | **Re-scoring** | **PASS** | Evaluated 7 runs across all modes and schemas by hand with `doctor_tool.exe`; all recall counts, repair counts, bad line counts, and field accuracies match the report exactly. |
| **2** | **Same Setup as Run 3** | **PASS** | User prompt strings sent in requests are 100% byte-identical to Run 3. Chat template adds 40 prompt tokens in thinking-on mode for reasoning effort system text; documented accurately. |
| **3** | **s4 Field Accuracy** | **FAIL** | `evaluate_bonsai.py` evaluated s4 positionally against a flat ground truth list, whereas `evaluate_run3.py` partitioned records by class. Alternating blocks in Run 3 penalized Bonsai unjustly. Under Run 3's method, Bonsai achieves **100.0%** (not 84.8%), rendering the direct comparison with Mellum invalid. |
| **4** | **Thinking-off s1 Analysis** | **PASS** | 81.6% accuracy verified (102/125 fields). 20 errors arose from the model writing `~` for the `id` field across runs 2–5; 3 errors arose from `verified` inverted to `1` in runs 1–3. |
| **5** | **Token Counts** | **PASS** | Differences between `usage.completion_tokens` and tokenized text range from 1 to 12 across all 40 runs. Max diff of 12 verified on thinking-on s4 run 5. |
| **6** | **Overclaims & Math** | **PASS** | All means, row totals, percentages, and matrix cells add up. q4_0 KV cache and sampling differences are stated without causal claims. Observations remain strictly non-causal. |
| **7** | **Integrity** | **PASS** | All SHA256 hashes for Run 1 (80 files), Run 2 (160 files), Run 3 (155 files), and Run 3 Bonsai (60 files) match. `git status` shows no modifications under `bench/r3/real/`. |

---

## 1. Re-scoring Verification

We rebuilt the doctor binary (`cargo build --release --bins`) and executed [target/release/doctor_tool.exe](file:///C:/Users/Ashman%20Das/Documents/Struct/target/release/doctor_tool.exe) in salvage mode (`salvage true`) across 7 selected runs covering both modes (`bonsai_off` and `bonsai_on`) and all four schemas (`s1_flat`, `s2_nested_fixed`, `s3_nested_variable`, `s4_two_class_code`), including the 20,275 thinking-token s4 run and thinking-off s3.

All decoded records from `doctor_tool.exe` were verified by hand against [bench/r3/real/run3_bonsai/ground_truth.json](file:///C:/Users/Ashman%20Das/Documents/Struct/bench/r3/real/run3_bonsai/ground_truth.json).

### Run 1: `bonsai_off_s1_flat_run1` (Seed 42)
- **Doctor Execution:** `doctor_tool.exe s1_flat.stc raw/bonsai_off_s1_flat_run1.txt salvage true`
- **Doctor Output:** `has_data: true`, `record_count: 5`, `repairs: 0`, `bad_lines: 0`. Category: **clean**.
- **Hand Working:**
  - Record 1: `id="2405.05254"`, `title="Rope scaling hurts long ctx"`, `verdict="refuted"`, `year=2024`, `verified=true`. Expected: `verified=false`. (4/5 fields correct)
  - Record 2: `id="1706.03762"`, `title="Attention is all you need"`, `verdict="supported"`, `year=2017`, `verified=true`. (5/5 fields correct)
  - Record 3: `id="2106.09685"`, `title="LoRA low rank adaptation"`, `verdict="supported"`, `year=2021`, `verified=true`. (5/5 fields correct)
  - Record 4: `id="2309.06180"`, `title="Mistral 7B architecture"`, `verdict="supported"`, `year=2023`, `verified=true`. (5/5 fields correct)
  - Record 5: `id="2401.04088"`, `title="DeepSeekMoE mixture experts"`, `verdict="supported"`, `year=2024`, `verified=true`. (5/5 fields correct)
- **Recall:** 5/5 = **100.0%**.
- **Field Accuracy:** 24/25 = **96.0%**.
- **Report Match:** Row 454 reports `clean`, `5/5`, `0 repairs`, `0 bad lines`, `96.0%`. **Matches.**

### Run 2: `bonsai_off_s2_nested_fixed_run3` (Seed 44)
- **Doctor Execution:** `doctor_tool.exe s2_nested_fixed.stc raw/bonsai_off_s2_nested_fixed_run3.txt salvage true`
- **Doctor Output:** `has_data: true`, `record_count: 5`, `repairs: 0`, `bad_lines: 0`. Category: **clean**.
- **Hand Working:**
  - 5 records with 5 fields each (`text`, `verdict`, `source.id`, `source.year`, `verified`).
  - Records 1–5 match ground truth exactly across all 5 fields.
- **Recall:** 5/5 = **100.0%**.
- **Field Accuracy:** 25/25 = **100.0%**.
- **Report Match:** Row 461 reports `clean`, `5/5`, `0 repairs`, `0 bad lines`, `100.0%`. **Matches.**

### Run 3: `bonsai_off_s3_nested_variable_run4` (Seed 45)
- **Doctor Execution:** `doctor_tool.exe s3_nested_variable.stc raw/bonsai_off_s3_nested_variable_run4.txt salvage true`
- **Doctor Output:** `has_data: false`, `record_count: 0`, `repairs: 9`, `bad_lines: 27`. Category: **unusable**.
- **Hand Working:**
  - The model wrote 5 top-level lines with an invalid 5th column `~` (e.g. `Rope scaling hurts long ctx|refuted|2405.05254|2024|~|`).
  - Lines 1, 7, 14, 18, 24 each produced `expected 4 columns, found 5` (5 bad parent lines).
  - 9 child lines lacked trailing pipes, producing 9 `MissingFinalPipe` repairs on lines 3, 4, 9, 10, 12, 16, 17, 20, 26.
  - Because each parent line failed, all 22 child lines were discarded in salvage mode (`dropped as child of line X`).
  - Total bad lines = 5 + 22 = 27 bad lines. Total salvaged records = 0.
- **Recall:** 0/5 = **0.0%**.
- **Field Accuracy:** **n/a** (0 records decoded).
- **Report Match:** Row 467 reports `unusable`, `0/5`, `9 repairs`, `27 bad lines`, `n/a`. **Matches.**

### Run 4: `bonsai_off_s4_two_class_code_run2` (Seed 43)
- **Doctor Execution:** `doctor_tool.exe s4_two_class_code.stc raw/bonsai_off_s4_two_class_code_run2.txt salvage true`
- **Doctor Output:** `has_data: true`, `record_count: 3`, `repairs: 0`, `bad_lines: 3`. Category: **salvaged**.
- **Hand Working:**
  - The model wrote a `schema:file` block with 3 file records, then a `schema:patch` block with 3 patch records.
  - Each patch record used literal `\n` characters (e.g. `lib.rs|\rpub fn add(...) {\n a + b\n}|10|`), violating the raw value escape rules.
  - The doctor rejected the 3 patch lines with `invalid escape sequence '\n'` (3 bad lines).
  - The doctor successfully salvaged the 3 file records: `lib.rs`, `util.rs`, `main.rs`.
  - Comparing decoded files 0, 1, 2 against ground truth files 0, 1, 2: `name` and `path` match on all 3 (6/6 fields).
- **Recall:** 3/6 = **50.0%** (3/6 decoded).
- **Field Accuracy:** 6/6 = **100.0%**.
- **Report Match:** Row 470 reports `salvaged`, `3/6`, `0 repairs`, `3 bad lines`, `100.0%`. **Matches.**

### Run 5: `bonsai_on_s1_flat_run5` (Seed 46)
- **Doctor Execution:** `doctor_tool.exe s1_flat.stc raw/bonsai_on_s1_flat_run5.txt salvage true`
- **Doctor Output:** `has_data: true`, `record_count: 5`, `repairs: 0`, `bad_lines: 0`. Category: **clean**.
- **Hand Working:**
  - Records 1–5 evaluated (25 fields). Record 1 has `verified=true` instead of `false`. All other 24 fields match ground truth.
- **Recall:** 5/5 = **100.0%**.
- **Field Accuracy:** 24/25 = **96.0%**.
- **Report Match:** Row 478 reports `clean`, `5/5`, `0 repairs`, `0 bad lines`, `96.0%`. **Matches.**

### Run 6: `bonsai_on_s3_nested_variable_run1` (Seed 42)
- **Doctor Execution:** `doctor_tool.exe s3_nested_variable.stc raw/bonsai_on_s3_nested_variable_run1.txt salvage true`
- **Doctor Output:** `has_data: true`, `record_count: 5`, `repairs: 0`, `bad_lines: 0`. Category: **clean**.
- **Hand Working:**
  - Total ground truth fields across all 5 records = 9 + 10 + 7 + 9 + 7 = 42 fields.
  - In all 5 records, the model wrapped the `text` field in double quotes: `"\"Rope scaling hurts long ctx\""`, etc.
  - Because quotes are preserved in string data lines, the `text` field failed exact-match on all 5 records.
  - All other 37 fields (verdicts, source IDs, years, notes, subnotes) matched ground truth exactly.
  - Correct fields = 42 - 5 = 37 out of 42.
- **Recall:** 5/5 = **100.0%**.
- **Field Accuracy:** 37/42 = **88.1%** (88.095%).
- **Report Match:** Row 484 reports `clean`, `5/5`, `0 repairs`, `0 bad lines`, `88.1%`. **Matches.**

### Run 7: `bonsai_on_s4_two_class_code_run5` (Seed 46, 20,275 Thinking Tokens)
- **Doctor Execution:** `doctor_tool.exe s4_two_class_code.stc raw/bonsai_on_s4_two_class_code_run5.txt salvage true`
- **Doctor Output:** `has_data: true`, `record_count: 6`, `repairs: 0`, `bad_lines: 0`. Category: **clean**.
- **Thinking Tokens:** 20,275 (highest in entire benchmark). Output tokens: 120.
- **Hand Working:**
  - The model generated `schema:file` with 3 files, then `schema:patch` with 3 patches.
  - Patch code values began with `\r` followed immediately by a newline (`\r\npub fn add...`). Under `.strip()`, leading whitespace trims, matching ground truth.
  - Records 0–2 (files): `name` and `path` match on all 3 (6 fields).
  - Records 3–5 (patches): `file_name`, `code`, `line` match on all 3 (9 fields).
  - Total evaluated fields = 6 + 9 = 15 fields. Correct = 15/15.
- **Recall:** 6/6 = **100.0%**.
- **Field Accuracy:** 15/15 = **100.0%**.
- **Report Match:** Row 493 reports `clean`, `6/6`, `0 repairs`, `0 bad lines`, `100.0%`. **Matches.**

---

## 2. Setup Comparison with Run 3 & Chat Template Analysis

### User Prompt Byte-Identity
We compared the `request.messages[0].content` strings from [bench/r3/real/run3_bonsai/metadata.json](file:///C:/Users/Ashman%20Das/Documents/Struct/bench/r3/real/run3_bonsai/metadata.json) against [bench/r3/real/run3/metadata.json](file:///C:/Users/Ashman%20Das/Documents/Struct/bench/r3/real/run3/metadata.json) across all 40 runs and all 4 schemas.
- `s1_flat`: Byte-identical (1,069 bytes).
- `s2_nested_fixed`: Byte-identical (1,154 bytes).
- `s3_nested_variable`: Byte-identical (1,847 bytes).
- `s4_two_class_code`: Byte-identical (936 bytes).

**Result:** The user message submitted to the API was 100% byte-identical to Run 3.

### Chat Template and Prompt Token Asymmetry
In `collect_bonsai.py`, `chat_template_kwargs` was passed as `{"enable_thinking": false}` for thinking-off, and `{"enable_thinking": true}` for thinking-on.

Inspection of the GGUF Jinja chat template reveals:
```jinja
{%- if enable_thinking is undefined or enable_thinking is true %}
    {%- set resolved_reasoning_effort = reasoning_effort|default('xhigh') %}
    ...
```
When `enable_thinking: true`, the template prepends a system instruction:
`Reasoning effort is set to xhigh.`

This adds **exactly 40 prompt tokens** to every thinking-on request across all schemas:
- `s1_flat`: 427 tokens (off) vs 467 tokens (on) (+40)
- `s2_nested_fixed`: 436 tokens (off) vs 476 tokens (on) (+40)
- `s3_nested_variable`: 680 tokens (off) vs 720 tokens (on) (+40)
- `s4_two_class_code`: 370 tokens (off) vs 410 tokens (on) (+40)

Furthermore, the assistant turn injection differs:
- Thinking off: `<|im_start|>assistant\n<think>\n\n</think>\n\n`
- Thinking on: `<|im_start|>assistant\n<think>\n`

**Conclusion on Input Equivalence:**
At the API interface level, both modes were given the exact same user input text. However, at the transformer input level (the token sequence presented to the model context), the two modes were **not given the same input**: thinking-on received additional system instructions injecting reasoning guidance. The report documents this mechanism accurately.

---

## 3. s4 Field Accuracy: Methodological Discrepancy & Comparability

### Root Cause Analysis of Scoring Divergence
The report claims in Section 4 and Section 6 that Bonsai (thinking on) achieved **84.8%** field accuracy on `s4_two_class_code` (compared against Mellum's **100.0%** in Run 3).

Inspection of [bench/r3/evaluate_bonsai.py](file:///C:/Users/Ashman%20Das/Documents/Struct/bench/r3/evaluate_bonsai.py) lines 56–70 shows how Bonsai was scored:
```python
def s4_positional(decoded):
    gt = TASKS["s4_two_class_code"]["ground_truth"]
    correct = evaluated = 0
    for i, dec in enumerate(decoded):
        if i >= len(gt):
            break
        exp = gt[i]
        dec = dec if isinstance(dec, dict) else {}
        for k, v in exp.items():
            if k == "class":
                continue
            evaluated += 1
            if str(dec.get(k, "")).strip() == str(v).strip():
                correct += 1
    return correct, evaluated
```
Ground truth `gt` consists of 3 files followed by 3 patches: `[file0, file1, file2, patch0, patch1, patch2]`.
`evaluate_bonsai.py` compares `decoded[i]` directly against `gt[i]`.

In contrast, [bench/r3/evaluate_run3.py](file:///C:/Users/Ashman%20Das/Documents/Struct/bench/r3/evaluate_run3.py) lines 167–194 scored Run 3 models by partitioning decoded records by class:
```python
dec_files = [r for r in decoded_records if isinstance(r, dict) and "path" in r]
dec_patches = [r for r in decoded_records if isinstance(r, dict) and ("code" in r or "line" in r)]
exp_files = [r for r in gt if r.get("class") == "file"]
exp_patches = [r for r in gt if r.get("class") == "patch"]
# Compares dec_files[i] with exp_files[i]
# Compares dec_patches[i] with exp_patches[i]
```

### The Impact on Bonsai Thinking-on Run 3
In Run 3 of `bonsai_on_s4_two_class_code`, Bonsai emitted records by alternating schema blocks per file:
1. `schema:file` -> `lib.rs|src/lib.rs|`
2. `schema:patch` -> patch for `lib.rs`
3. `schema:file` -> `util.rs|src/util.rs|`
4. `schema:patch` -> patch for `util.rs`
5. `schema:file` -> `main.rs|src/main.rs|`
6. `schema:patch` -> patch for `main.rs`

Under [SPEC.md](file:///C:/Users/Ashman%20Das/Documents/Struct/SPEC.md) Section 2:
> "`schema:name` with no braces reuses a class defined earlier (in the file, or supplied from outside it) and starts a new block of it."

Alternating blocks is 100% compliant with the Struct specification. Every record emitted was completely valid and contained zero data errors.

However, `evaluate_bonsai.py` evaluated the records sequentially:
- Index 0: File vs `gt[0]` (File) -> 2/2 correct.
- Index 1: Patch vs `gt[1]` (File) -> Patch lacks `name`/`path` -> 0/2 correct.
- Index 2: File (`util.rs`) vs `gt[2]` (File `main.rs`) -> 0/2 correct.
- Index 3: Patch (`util.rs`) vs `gt[3]` (Patch `lib.rs`) -> 0/3 correct.
- Index 4: File (`main.rs`) vs `gt[4]` (Patch `util.rs`) -> File lacks `file_name`/`code`/`line` -> 0/3 correct.
- Index 5: Patch (`main.rs`) vs `gt[5]` (Patch `main.rs`) -> 3/3 correct.

Run 3 was awarded only 5/15 (33.3%) field accuracy purely because it alternated blocks.

### True Score Under Run 3 Methodology
When evaluated using `evaluate_run3.py`'s class-partitioned method:
- Run 1 (salvaged): 3 files -> 6/6 (100.0%)
- Run 2 (clean): 3 files, 3 patches -> 15/15 (100.0%)
- Run 3 (clean): 3 files, 3 patches -> **15/15 (100.0%)** (was 5/15)
- Run 4 (clean): 3 files, 3 patches -> 15/15 (100.0%)
- Run 5 (clean): 3 files, 3 patches -> 15/15 (100.0%)

**Pooled Field Accuracy Comparison:**
- Under `evaluate_bonsai.py`: 56 / 66 = **84.8%**
- Under `evaluate_run3.py`: **66 / 66 = 100.0%**

**Verdict:** Bonsai's reported 84.8% field accuracy is **not comparable** with Mellum's 100.0%. Under Run 3's scoring method, Bonsai achieves **100.0% field accuracy**, matching Mellum's precision while providing 4.5× higher recall (90.0% vs 20.0%).

---

## 4. Thinking-off s1: Error Breakdown

In `bonsai_off` on `s1_flat`, record recall was 100.0% (25/25 records decoded across 5 runs), but field accuracy was **81.6%** (102/125 correct fields).

A total of 23 field mismatches occurred across the 125 evaluated fields. The exact errors:

### 1. `id` Field Omission (20 occurrences)
In Runs 2, 3, 4, and 5 (all 4 runs), the model wrote `~` (null) for the `id` field across every record instead of extracting the paper ID:
- Run 2, Line 1: `~|Rope scaling hurts long ctx|refuted|2024|1|` (expected `2405.05254`)
- Run 2, Line 2: `~|Attention is all you need|supported|2017|1|` (expected `1706.03762`)
- Run 2, Line 3: `~|LoRA low rank adaptation|supported|2021|1|` (expected `2106.09685`)
- Run 2, Line 4: `~|Mistral 7B architecture|supported|2023|1|` (expected `2309.06180`)
- Run 2, Line 5: `~|DeepSeekMoE mixture experts|supported|2024|1|` (expected `2401.04088`)
Runs 3, 4, and 5 exhibited the identical pattern for all 5 records (4 runs × 5 records = 20 errors).

### 2. `verified` Boolean Inversion on Record 1 (3 occurrences)
In Runs 1, 2, and 3, Record 1 (`Rope scaling hurts long ctx`) had the `verified` field emitted as `1` (true) instead of `0` (false):
- Run 1, Line 1: `2405.05254|Rope scaling hurts long ctx|refuted|2024|1|`
- Run 2, Line 1: `~|Rope scaling hurts long ctx|refuted|2024|1|`
- Run 3, Line 1: `~|Rope scaling hurts long ctx|refuted|2024|1|`
The prompt note stated: *"Paper 2405.05254 was published in 2024 with the title "Rope scaling hurts long ctx"; it turned out to be false, so the claim is refuted."* Ground truth expects `false` (`0`).

Total errors: 20 (`id`) + 3 (`verified`) = 23 errors.  
Total correct: 125 - 23 = 102 correct fields (102/125 = 81.6%).

---

## 5. Token Accounting & Completion Token Divergence

Across all 40 runs in `bench/r3/real/run3_bonsai/metadata.json`, we calculated:
$$\Delta = \text{usage.completion\_tokens} - (\text{thinking\_tokens} + \text{struct\_tokens})$$

- Minimum difference: **1 token**
- Maximum difference: **12 tokens**
- Difference distribution:
  - $\Delta = 1$: 20 runs
  - $\Delta = 3$: 16 runs
  - $\Delta = 4$: 2 runs
  - $\Delta = 5$: 1 run
  - $\Delta = 12$: 1 run

The largest difference occurred in `bonsai_on_s4_two_class_code_run5`:
- `usage.completion_tokens`: 20,407
- `thinking_tokens` (`/tokenize` on `reasoning_content`): 20,275
- `struct_tokens` (`/tokenize` on `content`): 120
- Sum: 20,395 tokens
- Discrepancy: $20,407 - 20,395 = \mathbf{12\text{ tokens}}$.

The stored data in `metadata.json` completely supports the statements in Section 7 of the report.

---

## 6. Overclaims, Causal Assertions, and Mathematical Verification

### Mathematical Accuracy
Every table and aggregation was programmatically recalculated against raw outputs and metadata:
- **Section 4 (Summary Matrix):** Row totals, clean/repaired/salvaged/partial/unusable counts, record recalls, positional field accuracies, distinct output counts, finish reason distributions, token means, and token maximums are mathematically exact.
- **Section 5 (Mode Aggregates):** Means (thinking: 0 off, 4389 on; output: 150 off, 179 on) and maximums match the underlying distributions.
- **Section 8 (Doctor Repairs):** 9 repairs for `bonsai_off`, 3 repairs for `bonsai_on`.
- **Section 9 (Bad Lines):** All 18 problem categories and their exact counts match.
- **Section 11 (Per-run Results):** All 40 rows match run metadata, doctor outputs, and latencies.
- **Section 6 (Run 3 Baseline Cells):** Every baseline cell copied from `research/r3-real-run3.md` matches.

### Causal Isolation and Setup Transparency
- **KV Cache:** Section 2 explicitly documents: *"KV cache: Bonsai ran with a 4-bit (q4_0) KV cache. Every run3 model used turbo4 (K) and turbo3 (V) KV. This is a recorded difference in setup only; no effect on any result is claimed."*
- **Sampling Differences:** Section 1 explicitly documents: *"The Bonsai sampling differs on purpose: it is PrismML's recommended sampling for each mode, so the comparison is between each model in its documented setup, not between identical sampling settings."*
- **Tone and Claims:** The report adheres strictly to observational phrasing (*"Observations only"*), quoting actual model outputs and reporting raw doctor findings without speculating on why models made specific mistakes.

---

## 7. Integrity & Reproducibility Check

- **Hash Verification:** We ran SHA256 integrity verification across all 4 benchmark suites:
  - `run1`: 80 files matched [bench/r3/real/run1/SHA256SUMS](file:///C:/Users/Ashman%20Das/Documents/Struct/bench/r3/real/run1/SHA256SUMS) (after stripping CRLF line endings).
  - `run2`: 160 files matched [bench/r3/real/run2/SHA256SUMS](file:///C:/Users/Ashman%20Das/Documents/Struct/bench/r3/real/run2/SHA256SUMS).
  - `run3`: 155 files matched [bench/r3/real/run3/SHA256SUMS](file:///C:/Users/Ashman%20Das/Documents/Struct/bench/r3/real/run3/SHA256SUMS).
  - `run3_bonsai`: 60 files matched [bench/r3/real/run3_bonsai/SHA256SUMS](file:///C:/Users/Ashman%20Das/Documents/Struct/bench/r3/real/run3_bonsai/SHA256SUMS).
- **Working Directory Status:** `git status --porcelain bench/r3/real/` produced zero output, confirming that no benchmark file, output, hash list, or metadata file was altered.

---

## Ranked Defects

### 1. High Severity: Methodological Inconsistency in s4 Field Accuracy
- **Location:** `bench/r3/evaluate_bonsai.py` lines 56–70, `research/r3-real-bonsai.md` Sections 4 and 6.
- **Defect:** Bonsai's s4 field accuracy was evaluated against a rigid flat ground truth array rather than class-partitioned lists as was done for Run 3 models in `evaluate_run3.py`. This unjustly penalized Bonsai for outputting alternating schema blocks (`schema:file` followed by `schema:patch`), which is fully permitted by `SPEC.md`.
- **Consequence:** Bonsai thinking-on was reported as 84.8% field accuracy on s4 and contrasted with Mellum's 100.0%. Under Run 3's scoring methodology, Bonsai achieves **100.0% field accuracy** on s4.

### 2. Medium Severity: Effective Prompt Token Asymmetry Not Highlighted in Summary
- **Location:** `research/r3-real-bonsai.md` Sections 1 and 3.
- **Defect:** Section 1 states *"the same prompt text as run3"*, which is true for the API request payload, but does not clarify in the executive summary that the thinking-on prompt received an extra 40 tokens of system instructions (`Reasoning effort is set to xhigh.`) generated by the model's chat template.
- **Consequence:** A casual reader could assume that both modes received identical token inputs.

### 3. Low Severity: Incomplete Failure Analysis for Thinking-off s1 in Observations
- **Location:** `research/r3-real-bonsai.md` Section 10.
- **Defect:** Observation 9 discusses thinking-on s1 having only 1 error in 125 fields, but Section 10 omits the failure breakdown for thinking-off s1 (where 20 paper IDs were emitted as `~` and 3 `verified` booleans were inverted), which explained the 81.6% score.

---

## Plain-English Bottom Line for the Maintainer

The Bonsai benchmark data and raw outputs are completely authentic, reproducible, and intact:
1. Every hash across all 4 benchmark runs matches perfectly, and the raw text outputs reflect genuine model generations.
2. The mathematics across the report (row sums, token averages, repair counts, bad line totals) are completely accurate.
3. However, **the reported 84.8% field accuracy for Bonsai on s4 is misleading because it was calculated differently than the other models in Run 3.**
   - In Run 3, the evaluator separated code patches from file definitions before scoring them.
   - For Bonsai, the evaluator checked records strictly in order against a fixed list (all files first, then all patches). In Run 3, Bonsai chose to write a file, then its patch, then another file, then its patch. Struct explicitly allows this. Because the evaluator did not expect alternating blocks, it compared patches against file definitions, marking perfectly valid data as wrong.
   - **If scored the same way as Run 3, Bonsai thinking-on achieves 100.0% field accuracy on s4**, matching Mellum's accuracy while delivering 90% record recall compared to Mellum's 20%.
4. In thinking-off mode, Bonsai struggled on s1 by emitting `~` (null) for every paper ID in 4 out of 5 runs (causing its 81.6% accuracy), and completely failed on s3 because it added an extra column to parent records.
5. In thinking-on mode, Bonsai decoded every single record cleanly on s1, s2, and s3 with near-perfect accuracy (97.6% to 99.2%), and decoded 27 of 30 records on s4 with 100% field accuracy under the standard evaluation method.
