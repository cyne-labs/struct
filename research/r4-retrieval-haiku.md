# R4-haiku: Retrieval Accuracy Across Formats (Claude Haiku)

Benchmark measuring model comprehension across 7 data serialization formats on Claude Haiku.

## 1. Setup

- **Model:** claude-haiku-5-5 (requested as `haiku`, effort `low`).
- **Command Line:**
  ```
  claude -p --model haiku --effort low --system-prompt You are a helpful assistant. --tools  --strict-mcp-config --setting-sources  --disable-slash-commands --no-session-persistence --output-format json
  ```
- **Reasoning budget:** no budget (effort `low`).
- **Repeats:** 4 repeats per prompt (labelled seed 1 to 4; 96 calls total).
- **Parallel:** 4 parallel calls.
- **Total Cost (List Price):** $0.1002 (sum of `total_cost_usd` across all calls, list price).
- **Input Tokens Measurement:** Counted by the Anthropic API, including about 640 fixed tokens from the system prompt and template on every call.
- **Design:** TSV and Plain `key: value` run only on the two flat datasets (per maintainer Decision 1; marked as 'can\'t hold this shape' for nested datasets).

## 2. Standardized Prompt

The prompt is identical for every format except the data block and one line naming the format.
Here is the exact full prompt text as sent to the model (example: Struct on flat_tabular):

```text
The following data is provided in Struct format:
```struct
schema:claim{id, title, verdict:enum(supported,refuted,unverified), year:int, verified:bool}
2405.05254|Rope scaling hurts long ctx|refuted|2024|0|
1706.03762|Attention is all you need|supported|2017|1|
2106.09685|LoRA low rank adaptation|supported|2021|1|
2309.06180|Mistral 7B architecture|supported|2023|1|
2401.04088|DeepSeekMoE mixture experts|supported|2024|1|
2005.14165|Language models few-shot|supported|2020|1|
2402.14905|StarCoder 2 training code|supported|2024|1|
2310.06825|Mistral fine tuning rules|unverified|2023|0|
2203.15556|Chinchilla scaling compute|supported|~|1|
2305.18290|Direct preference optim|supported|2023|1|
2407.21783|Llama 3 herd models tech|supported|2024|1|
2312.11805|Gemini models multimodal|supported|2023|1|
2403.09611|Gemma open models Google|supported|2024|~|
2307.09288|Llama 2 foundation chat|supported|2023|1|
2210.03629|ReAct syner reasoning act|supported|2022|1|
2404.14219|Phi 3 technical report|supported|~|1|
2302.13971|LLaMA efficient found LM|supported|2023|1|
2408.03314|Qwen2 technical report|~|2024|1|
2308.12950|Code Llama open model|supported|2023|1|
2406.11794|Nemotron 4 340B report|unverified|2024|0|

```

Answer each of the following questions based only on the data above.
Provide your answers as a numbered list, one short answer per line (e.g. "1. answer"):

1. What is the verdict for paper 2405.05254?
2. What is the publication year of paper 1706.03762?
3. What is the title of paper 2106.09685?
4. What is the verification status of paper 2309.06180?
5. How many records have verdict refuted?
6. How many records have verdict supported?
7. How many records were published in 2024?
8. Which paper IDs have verdict unverified?
9. Which paper IDs have no publication year?
10. Which paper ID has no verification status?
11. What is the publication year of paper 2203.15556?
12. Which paper ID has no verdict?
```

## 3. Results Summary Across Formats

| Format | Accuracy (Counts) | Accuracy (%) | Mean Input Toks* | Mean Thinking Toks | Max Thinking Toks | Budget Exhausted | Failed Runs |
|---|---|---|---|---|---|---|---|
| Minified JSON | 158/192 | 82.3% | 2717 | 2975 | 5897 | n/a | 0 |
| TOON | 163/192 | 84.9% | 2169 | 4597 | 5607 | n/a | 0 |
| YAML | 154/192 | 80.2% | 2776 | 4708 | 5156 | n/a | 0 |
| TSV | 75/96 | 78.1% | 2810 | 3902 | 7454 | n/a | 0 |
| XML | 130/192 | 67.7% | 3922 | 4046 | 5224 | n/a | 0 |
| Plain key: value | 76/96 | 79.2% | 3831 | 3828 | 4588 | n/a | 0 |
| Struct | 146/192 | 76.0% | 2123 | 2274 | 7646 | n/a | 0 |

* Input tokens are as counted by the Anthropic API, including about 640 fixed tokens from the system prompt and template on every call.

## 4. Accuracy by Dataset

| Format | flat_tabular (20r) | flat_100 (100r) | nested_fixed (10r) | nested_variable (4r) |
|---|---|---|---|---|
| Minified JSON | 35/48 (72.9%) | 48/48 (100.0%) | 34/48 (70.8%) | 41/48 (85.4%) |
| TOON | 36/48 (75.0%) | 48/48 (100.0%) | 36/48 (75.0%) | 43/48 (89.6%) |
| YAML | 35/48 (72.9%) | 47/48 (97.9%) | 40/48 (83.3%) | 32/48 (66.7%) |
| TSV | 36/48 (75.0%) | 39/48 (81.2%) | can't hold this shape | can't hold this shape |
| XML | 36/48 (75.0%) | 40/48 (83.3%) | 29/48 (60.4%) | 25/48 (52.1%) |
| Plain key: value | 29/48 (60.4%) | 47/48 (97.9%) | can't hold this shape | can't hold this shape |
| Struct | 39/48 (81.2%) | 31/48 (64.6%) | 39/48 (81.2%) | 37/48 (77.1%) |

## 5. Accuracy by Question Kind

| Question Kind | Accuracy (Counts) | Accuracy (%) |
|---|---|---|
| single_lookup | 287/344 | 83.4% |
| nested_lookup | 93/120 | 77.5% |
| count_filter | 290/344 | 84.3% |
| null_missing | 232/344 | 67.4% |

## 6. Accuracy per Format by Question Kind

| Format | Single Lookup | Nested Lookup | Count / Filter | Null / Missing |
|---|---|---|---|---|
| Minified JSON | 48/56 (85.7%) | 24/24 (100.0%) | 48/56 (85.7%) | 38/56 (67.9%) |
| TOON | 48/56 (85.7%) | 24/24 (100.0%) | 50/56 (89.3%) | 41/56 (73.2%) |
| YAML | 48/56 (85.7%) | 16/24 (66.7%) | 48/56 (85.7%) | 42/56 (75.0%) |
| TSV | 28/32 (87.5%) | can't hold this shape | 29/32 (90.6%) | 18/32 (56.2%) |
| XML | 47/56 (83.9%) | 13/24 (54.2%) | 38/56 (67.9%) | 32/56 (57.1%) |
| Plain key: value | 24/32 (75.0%) | can't hold this shape | 25/32 (78.1%) | 27/32 (84.4%) |
| Struct | 44/56 (78.6%) | 16/24 (66.7%) | 52/56 (92.9%) | 34/56 (60.7%) |

## 7. Format Notes and Review Decisions

- **XML Empty Lists (Decision 12):** Plain XML cannot distinguish an empty list (`<subnotes/>`) from no value without schema or type attributes. This is recorded as an inherent limit of plain XML.
- **TOON Spec Adherence (Decision 13):** Tabular headers with uniform nested objects use valid Nested Field Groups (`source{id,year}` per TOON spec 9.3), numeric-looking strings like arXiv IDs are quoted (`"2405.05254"` per spec 7.2), and empty lists use `key: []` (`subnotes: []` per TOON spec 9.1).
- **Failed Requests (Decision 13):** Any request experiencing an exit error, timeout, unparseable JSON, or empty result is recorded as failed and not marked done; resume reruns them.

## 8. Repeat Spread per Format

| Format | Seed 1 | Seed 2 | Seed 3 | Seed 4 | Mean | Spread (Max-Min) |
|---|---|---|---|---|---|---|
| Minified JSON | 83.3% | 81.2% | 83.3% | 81.2% | 82.3% | 2.1% |
| TOON | 79.2% | 85.4% | 89.6% | 85.4% | 84.9% | 10.4% |
| YAML | 79.2% | 79.2% | 81.2% | 81.2% | 80.2% | 2.1% |
| TSV | 66.7% | 83.3% | 79.2% | 83.3% | 78.1% | 16.7% |
| XML | 56.2% | 72.9% | 70.8% | 70.8% | 67.7% | 16.7% |
| Plain key: value | 66.7% | 79.2% | 87.5% | 83.3% | 79.2% | 20.8% |
| Struct | 70.8% | 77.1% | 77.1% | 79.2% | 76.0% | 8.3% |

## 9. Wrong Answers Log

Total wrong answers: 250

| Format | Dataset | Seed | Question | Expected | Model Answer |
|---|---|---|---|---|---|
| Minified JSON | flat_tabular | 1 | What is the verdict for paper 2405.05254? | `refuted` | `null` |
| Minified JSON | flat_tabular | 1 | How many records have verdict supported? | `16` | `15` |
| Minified JSON | flat_tabular | 1 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2404.14219, 2302.13971 — wait, correction: 2302.13971 has year 2023, so the correct list is 2203.15556 and 2404.14219` |
| Minified JSON | flat_tabular | 1 | Which paper ID has no verification status? | `2403.09611` | `2408.03314` |
| TOON | flat_tabular | 1 | What is the verification status of paper 2309.06180? | `true` | `Unverified... no, verified is false. Its status is "false" (not verified).` |
| TOON | flat_tabular | 1 | How many records have verdict supported? | `16` | `15` |
| TOON | flat_tabular | 1 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2404.14219... no, 2404.14219 has year 2024. Only 2203.15556 has a null year? Correction: 2203.15556 and 2404.14219 have year null? Checking: 2404.14219 shows year null, so both 2203.15556 and 2404.14219 lack a year... wait, 2203.15556 has year null and 2404.14219 has year null. Answer: 2203.15556, 2404.14219` |
| TOON | flat_tabular | 1 | Which paper ID has no verification status? | `2403.09611` | `2408.03314` |
| TOON | flat_tabular | 1 | What is the publication year of paper 2203.15556? | `null` | `null (no publication year is recorded)` |
| TOON | flat_tabular | 1 | Which paper ID has no verdict? | `2408.03314` | `None (every record has a verdict)` |
| YAML | flat_tabular | 1 | What is the verdict for paper 2405.05254? | `refuted` | `unverified... wait, let me correct that: the verdict for 2405.05254 is refuted.` |
| YAML | flat_tabular | 1 | How many records have verdict supported? | `16` | `14` |
| YAML | flat_tabular | 1 | How many records were published in 2024? | `7` | `9` |
| YAML | flat_tabular | 1 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556 has year null... no, correction: 2203.15556, 2404.14219` |
| TSV | flat_tabular | 1 | What is the verification status of paper 2309.06180? | `true` | `verified: true` |
| TSV | flat_tabular | 1 | How many records have verdict supported? | `16` | `14` |
| TSV | flat_tabular | 1 | How many records were published in 2024? | `7` | `8` |
| TSV | flat_tabular | 1 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556` |
| TSV | flat_tabular | 1 | What is the publication year of paper 2203.15556? | `null` | `(blank, no year recorded)` |
| XML | flat_tabular | 1 | What is the verification status of paper 2309.06180? | `true` | `true (verified)` |
| XML | flat_tabular | 1 | How many records were published in 2024? | `7` | `9` |
| XML | flat_tabular | 1 | What is the publication year of paper 2203.15556? | `null` | `(blank; no year listed)` |
| Plain key: value | flat_tabular | 1 | What is the verdict for paper 2405.05254? | `refuted` | `unverified... wait, let me correct that: 2405.05254 has verdict **refuted**.` |
| Plain key: value | flat_tabular | 1 | What is the title of paper 2106.09685? | `LoRA low rank adaptation` | `Language models few-shot... no, 2106.09685's title is **LoRA low rank adaptation**.` |
| Plain key: value | flat_tabular | 1 | What is the verification status of paper 2309.06180? | `true` | `Verified: **true**` |
| Plain key: value | flat_tabular | 1 | How many records have verdict supported? | `16` | `12` |
| Plain key: value | flat_tabular | 1 | How many records were published in 2024? | `7` | `6` |
| Plain key: value | flat_tabular | 1 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2404.14219... no, that one has year null too, so: 2203.15556, 2404.14219, 2408.03314 — wait, 2408.03314 has year 2024. Correct list: 2203.15556, 2404.14219` |
| Plain key: value | flat_tabular | 1 | What is the publication year of paper 2203.15556? | `null` | `null (no publication year recorded)` |
| Struct | flat_tabular | 1 | What is the verification status of paper 2309.06180? | `true` | `Verified (verified = 1)` |
| Struct | flat_tabular | 1 | How many records were published in 2024? | `7` | `8` |
| Struct | flat_tabular | 1 | What is the publication year of paper 2203.15556? | `null` | `No year (listed as ~)` |
| TSV | flat_100 | 1 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2408.7547` |
| TSV | flat_100 | 1 | What is the publication year of paper 2403.4527? | `null` | `Not given (blank)` |
| TSV | flat_100 | 1 | What is the verdict for paper 2404.7570? | `null` | `Not given (blank)` |
| XML | flat_100 | 1 | What is the publication year of paper 2403.4527? | `null` | `No year is recorded (empty)` |
| XML | flat_100 | 1 | What is the verdict for paper 2404.7570? | `null` | `No verdict is recorded (empty)` |
| Plain key: value | flat_100 | 1 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` |
| Struct | flat_100 | 1 | What is the verification status of paper 2412.9928? | `false` | `Not verified (verified = 0)` |
| Struct | flat_100 | 1 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2404.7570, 2411.9702 (verdict is "~")` |
| Struct | flat_100 | 1 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547 (year is "~")` |
| Struct | flat_100 | 1 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445 (verified is "~")` |
| Struct | flat_100 | 1 | What is the publication year of paper 2403.4527? | `null` | `Unknown (year is "~")` |
| Struct | flat_100 | 1 | What is the verdict for paper 2404.7570? | `null` | `Unknown (verdict is "~")` |
| Minified JSON | nested_fixed | 1 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| Minified JSON | nested_fixed | 1 | How many claims have verdict supported? | `7` | `6` |
| Minified JSON | nested_fixed | 1 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `Not available (year is null)` |
| TOON | nested_fixed | 1 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| TOON | nested_fixed | 1 | How many claims have a source published in 2024? | `2` | `3` |
| TOON | nested_fixed | 1 | Which claim has no verification status? | `StarCoder 2 training code` | `Direct preference optim` |
| YAML | nested_fixed | 1 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| YAML | nested_fixed | 1 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `null (no year recorded)` |
| XML | nested_fixed | 1 | What is the verdict for 'Attention is all you need'? | `supported` | `The verdict is supported.` |
| XML | nested_fixed | 1 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `Verified is false.` |
| XML | nested_fixed | 1 | What is the verdict for 'Rope scaling hurts long ctx'? | `refuted` | `The verdict is refuted.` |
| XML | nested_fixed | 1 | What is the source paper ID for 'Attention is all you need'? | `1706.03762` | `The source paper ID is 1706.03762.` |
| XML | nested_fixed | 1 | What is the publication year of the source for 'LoRA low rank adaptation'? | `2021` | `The publication year is 2021.` |
| XML | nested_fixed | 1 | What is the source paper ID for 'Chinchilla scaling compute'? | `2203.15556` | `The source paper ID is 2203.15556.` |
| XML | nested_fixed | 1 | How many claims have verdict supported? | `7` | `Six claims have verdict supported.` |
| XML | nested_fixed | 1 | How many claims have a source published in 2024? | `2` | `Two claims have a source published in 2024.` |
| XML | nested_fixed | 1 | Which source paper IDs belong to refuted claims? | `2405.05254` | `The refuted claim's source paper ID is 2405.05254.` |
| XML | nested_fixed | 1 | Which claim has no source year? | `DeepSeekMoE mixture experts` | `DeepSeekMoE mixture experts has no source year.` |
| XML | nested_fixed | 1 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `The year is blank (no year is given).` |
| XML | nested_fixed | 1 | Which claim has no verification status? | `StarCoder 2 training code` | `StarCoder 2 training code has no verification status.` |
| Struct | nested_fixed | 1 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified (verified: false)` |
| Struct | nested_fixed | 1 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `~ (no year given)` |
| Minified JSON | nested_variable | 1 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes)` |
| TOON | nested_variable | 1 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes are listed)` |
| YAML | nested_variable | 1 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `abstract mentions 32k limit; appendix B clarifies` |
| YAML | nested_variable | 1 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `per-head scales help; perplexity intact` |
| YAML | nested_variable | 1 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list)` |
| YAML | nested_variable | 1 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `checkpoint size under 50MB` |
| XML | nested_variable | 1 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `Abstract mentions 32k limit; appendix B clarifies` |
| XML | nested_variable | 1 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `Per-head scales help; perplexity intact` |
| XML | nested_variable | 1 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes are listed)` |
| XML | nested_variable | 1 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `Rope scaling hurts long ctx` |
| Struct | nested_variable | 1 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `abstract mentions 32k limit; appendix B clarifies` |
| Struct | nested_variable | 1 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `per-head scales help; perplexity intact` |
| Struct | nested_variable | 1 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes)` |
| Minified JSON | flat_tabular | 2 | What is the verification status of paper 2309.06180? | `true` | `false (not verified)` |
| Minified JSON | flat_tabular | 2 | How many records were published in 2024? | `7` | `8` |
| Minified JSON | flat_tabular | 2 | What is the publication year of paper 2203.15556? | `null` | `null (no year)` |
| TOON | flat_tabular | 2 | What is the verification status of paper 2309.06180? | `true` | `Unverified (verified is false)` |
| TOON | flat_tabular | 2 | How many records were published in 2024? | `7` | `6` |
| TOON | flat_tabular | 2 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556 (no, wait: 2203.15556 has year null) Correction: 2203.15556 and 2404.14219` |
| YAML | flat_tabular | 2 | What is the verdict for paper 2405.05254? | `refuted` | `unverified? No: refuted` |
| YAML | flat_tabular | 2 | What is the verification status of paper 2309.06180? | `true` | `verified: true` |
| YAML | flat_tabular | 2 | How many records have verdict supported? | `16` | `14` |
| YAML | flat_tabular | 2 | How many records were published in 2024? | `7` | `10` |
| TSV | flat_tabular | 2 | What is the verification status of paper 2309.06180? | `true` | `true (verified)` |
| TSV | flat_tabular | 2 | What is the publication year of paper 2203.15556? | `null` | `Blank (no year listed)` |
| XML | flat_tabular | 2 | How many records have verdict supported? | `16` | `17` |
| XML | flat_tabular | 2 | How many records were published in 2024? | `7` | `9` |
| XML | flat_tabular | 2 | What is the publication year of paper 2203.15556? | `null` | `(blank; no year listed)` |
| Plain key: value | flat_tabular | 2 | What is the verdict for paper 2405.05254? | `refuted` | `unverified... wait, correction: 2405.05254 has verdict refuted.` |
| Plain key: value | flat_tabular | 2 | What is the verification status of paper 2309.06180? | `true` | `verified: true` |
| Plain key: value | flat_tabular | 2 | How many records have verdict supported? | `16` | `14` |
| Plain key: value | flat_tabular | 2 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2404.14219... no, that one is supported with null year, so: 2404.14219, 2403.09611? No, 2403.09611 has year 2024. Correct list: 2404.14219, 2408.03314? No, 2408.03314 has year 2024. The records with null year are 2203.15556 and 2404.14219.` |
| Plain key: value | flat_tabular | 2 | What is the publication year of paper 2203.15556? | `null` | `null (no publication year is recorded)` |
| Struct | flat_tabular | 2 | What is the verification status of paper 2309.06180? | `true` | `1 (verified: true)` |
| Struct | flat_tabular | 2 | What is the publication year of paper 2203.15556? | `null` | `Unknown (listed as ~)` |
| TSV | flat_100 | 2 | What is the publication year of paper 2403.4527? | `null` | `(blank; no publication year is recorded)` |
| TSV | flat_100 | 2 | What is the verdict for paper 2404.7570? | `null` | `(blank; no verdict is recorded)` |
| XML | flat_100 | 2 | What is the publication year of paper 2403.4527? | `null` | `Empty (no publication year)` |
| XML | flat_100 | 2 | What is the verdict for paper 2404.7570? | `null` | `Empty (no verdict)` |
| Struct | flat_100 | 2 | What is the verification status of paper 2412.9928? | `false` | `Not verified (verified = false)` |
| Struct | flat_100 | 2 | How many records are marked as verified (true)? | `49` | `48` |
| Struct | flat_100 | 2 | What is the publication year of paper 2403.4527? | `null` | `No year given (the year field is ~)` |
| Struct | flat_100 | 2 | What is the verdict for paper 2404.7570? | `null` | `No verdict given (the verdict field is ~)` |
| Minified JSON | nested_fixed | 2 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| Minified JSON | nested_fixed | 2 | How many claims have verdict supported? | `7` | `6` |
| Minified JSON | nested_fixed | 2 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `Not specified (null)` |
| Minified JSON | nested_fixed | 2 | Which claim has no verification status? | `StarCoder 2 training code` | `Attention... no: StarCoder 2 training code` |
| TOON | nested_fixed | 2 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| TOON | nested_fixed | 2 | How many claims have a source published in 2024? | `2` | `3` |
| TOON | nested_fixed | 2 | Which claim has no verification status? | `StarCoder 2 training code` | `Direct preference optim` |
| YAML | nested_fixed | 2 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| YAML | nested_fixed | 2 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `null (no year listed)` |
| XML | nested_fixed | 2 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| XML | nested_fixed | 2 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `No year is given (the year field is empty)` |
| Struct | nested_fixed | 2 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `Not verified (the verified field is 0, and the verdict is unverified)` |
| Struct | nested_fixed | 2 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `Unknown (listed as "~")` |
| Struct | nested_fixed | 2 | Which claim has no verification status? | `StarCoder 2 training code` | `Direct preference optim` |
| Minified JSON | nested_variable | 2 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes)` |
| Minified JSON | nested_variable | 2 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `Rope scaling hurts long ctx` |
| TOON | nested_variable | 2 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes)` |
| YAML | nested_variable | 2 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `abstract mentions 32k limit; appendix B clarifies` |
| YAML | nested_variable | 2 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `per-head scales help; perplexity intact` |
| YAML | nested_variable | 2 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list)` |
| YAML | nested_variable | 2 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `checkpoint size under 50MB` |
| XML | nested_variable | 2 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `Abstract mentions 32k limit; appendix B clarifies` |
| XML | nested_variable | 2 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `Per-head scales help; perplexity intact` |
| XML | nested_variable | 2 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `Three` |
| XML | nested_variable | 2 | How many total claims have verdict refuted? | `2` | `Two` |
| XML | nested_variable | 2 | How many subnotes are under 'tested on GLUE and Wikisql'? | `2` | `Two` |
| XML | nested_variable | 2 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes listed)` |
| Struct | nested_variable | 2 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `abstract mentions 32k limit; appendix B clarifies` |
| Struct | nested_variable | 2 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `per-head scales help; perplexity intact` |
| Minified JSON | flat_tabular | 3 | What is the verification status of paper 2309.06180? | `true` | `false (not verified)` |
| Minified JSON | flat_tabular | 3 | How many records were published in 2024? | `7` | `8` |
| Minified JSON | flat_tabular | 3 | What is the publication year of paper 2203.15556? | `null` | `null (no year listed)` |
| TOON | flat_tabular | 3 | What is the verification status of paper 2309.06180? | `true` | `Unverified (verified is false)` |
| YAML | flat_tabular | 3 | How many records have verdict supported? | `16` | `15` |
| YAML | flat_tabular | 3 | How many records were published in 2024? | `7` | `10` |
| TSV | flat_tabular | 3 | What is the verification status of paper 2309.06180? | `true` | `false` |
| TSV | flat_tabular | 3 | How many records were published in 2024? | `7` | `8` |
| TSV | flat_tabular | 3 | What is the publication year of paper 2203.15556? | `null` | `No publication year is recorded for 2203.15556.` |
| XML | flat_tabular | 3 | How many records have verdict supported? | `16` | `17` |
| XML | flat_tabular | 3 | How many records were published in 2024? | `7` | `9` |
| XML | flat_tabular | 3 | What is the publication year of paper 2203.15556? | `null` | `(no year; the field is empty)` |
| Plain key: value | flat_tabular | 3 | What is the verdict for paper 2405.05254? | `refuted` | `unverified? No: refuted` |
| Plain key: value | flat_tabular | 3 | How many records have verdict supported? | `16` | `14` |
| Plain key: value | flat_tabular | 3 | How many records were published in 2024? | `7` | `9` |
| Struct | flat_tabular | 3 | What is the verification status of paper 2309.06180? | `true` | `verified (true)` |
| Struct | flat_tabular | 3 | What is the publication year of paper 2203.15556? | `null` | `Not given (year is "~" in the data)` |
| YAML | flat_100 | 3 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2408.7547` |
| TSV | flat_100 | 3 | What is the publication year of paper 2403.4527? | `null` | `(blank, no publication year)` |
| TSV | flat_100 | 3 | What is the verdict for paper 2404.7570? | `null` | `(blank, no verdict)` |
| XML | flat_100 | 3 | What is the publication year of paper 2403.4527? | `null` | `No publication year is recorded (empty).` |
| XML | flat_100 | 3 | What is the verdict for paper 2404.7570? | `null` | `No verdict is recorded (empty).` |
| Struct | flat_100 | 3 | What is the verification status of paper 2412.9928? | `false` | `Not verified (verified = 0)` |
| Struct | flat_100 | 3 | How many records are marked as verified (true)? | `49` | `49 (verified = 1)` |
| Struct | flat_100 | 3 | What is the publication year of paper 2403.4527? | `null` | `Unknown (missing, "~")` |
| Struct | flat_100 | 3 | What is the verdict for paper 2404.7570? | `null` | `Unknown (missing, "~")` |
| Minified JSON | nested_fixed | 3 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| Minified JSON | nested_fixed | 3 | How many claims have verdict supported? | `7` | `6` |
| Minified JSON | nested_fixed | 3 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `null (no year listed)` |
| TOON | nested_fixed | 3 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| TOON | nested_fixed | 3 | How many claims have a source published in 2024? | `2` | `3` |
| TOON | nested_fixed | 3 | Which claim has no verification status? | `StarCoder 2 training code` | `Direct preference optim` |
| YAML | nested_fixed | 3 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| YAML | nested_fixed | 3 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `null (no year is recorded)` |
| XML | nested_fixed | 3 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| XML | nested_fixed | 3 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `No year is given (the year field is empty).` |
| Struct | nested_fixed | 3 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| Struct | nested_fixed | 3 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `~ (no year given)` |
| Minified JSON | nested_variable | 3 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes)` |
| Minified JSON | nested_variable | 3 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `Rope scaling hurts long ctx` |
| TOON | nested_variable | 3 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes are listed)` |
| YAML | nested_variable | 3 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `abstract mentions 32k limit; appendix B clarifies` |
| YAML | nested_variable | 3 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `per-head scales help; perplexity intact` |
| YAML | nested_variable | 3 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list)` |
| YAML | nested_variable | 3 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `checkpoint size under 50MB` |
| XML | nested_variable | 3 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `Abstract mentions 32k limit; appendix B clarifies` |
| XML | nested_variable | 3 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `Per-head scales help; perplexity intact` |
| XML | nested_variable | 3 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `Three notes are attached to 'LoRA matches full finetune'. Wait, correcting that: two notes are attached.` |
| XML | nested_variable | 3 | How many total claims have verdict refuted? | `2` | `Two claims have verdict refuted` |
| XML | nested_variable | 3 | How many subnotes are under 'tested on GLUE and Wikisql'? | `2` | `Two subnotes` |
| XML | nested_variable | 3 | Which note under 'LoRA matches full finetune' has no subnotes? | `checkpoint size under 50MB` | `The note 'checkpoint size under 50MB'` |
| XML | nested_variable | 3 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (the subnotes element is empty)` |
| Struct | nested_variable | 3 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `abstract mentions 32k limit; appendix B clarifies` |
| Struct | nested_variable | 3 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `per-head scales help; perplexity intact` |
| Struct | nested_variable | 3 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes listed)` |
| Minified JSON | flat_tabular | 4 | What is the verification status of paper 2309.06180? | `true` | `Not verified (verified is false)` |
| Minified JSON | flat_tabular | 4 | How many records were published in 2024? | `7` | `8` |
| Minified JSON | flat_tabular | 4 | What is the publication year of paper 2203.15556? | `null` | `null (no publication year recorded)` |
| TOON | flat_tabular | 4 | What is the verification status of paper 2309.06180? | `true` | `Unverified (verified: false)` |
| TOON | flat_tabular | 4 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2407.21783 — no, correction: 2203.15556` |
| YAML | flat_tabular | 4 | What is the verdict for paper 2405.05254? | `refuted` | `unverified... no, let me restate: refuted` |
| YAML | flat_tabular | 4 | How many records have verdict supported? | `16` | `15` |
| YAML | flat_tabular | 4 | How many records were published in 2024? | `7` | `9` |
| TSV | flat_tabular | 4 | What is the verification status of paper 2309.06180? | `true` | `verified (true)` |
| TSV | flat_tabular | 4 | What is the publication year of paper 2203.15556? | `null` | `Blank (no year listed)` |
| XML | flat_tabular | 4 | What is the title of paper 2106.09685? | `LoRA low rank adaptation` | `Attention is all you need... no: 2106.09685 is "LoRA low rank adaptation"` |
| XML | flat_tabular | 4 | What is the verification status of paper 2309.06180? | `true` | `verified (true)` |
| XML | flat_tabular | 4 | What is the publication year of paper 2203.15556? | `null` | `None (the year field is empty)` |
| Plain key: value | flat_tabular | 4 | What is the verdict for paper 2405.05254? | `refuted` | `unverified... wait, let me be precise: refuted` |
| Plain key: value | flat_tabular | 4 | What is the verification status of paper 2309.06180? | `true` | `verified: true` |
| Plain key: value | flat_tabular | 4 | How many records have verdict supported? | `16` | `13` |
| Plain key: value | flat_tabular | 4 | How many records were published in 2024? | `7` | `8` |
| Struct | flat_tabular | 4 | What is the verification status of paper 2309.06180? | `true` | `verified (true)` |
| Struct | flat_tabular | 4 | What is the publication year of paper 2203.15556? | `null` | `Not given (value is ~)` |
| TSV | flat_100 | 4 | What is the publication year of paper 2403.4527? | `null` | `(blank)` |
| TSV | flat_100 | 4 | What is the verdict for paper 2404.7570? | `null` | `(blank)` |
| XML | flat_100 | 4 | What is the publication year of paper 2403.4527? | `null` | `(empty; no year recorded)` |
| XML | flat_100 | 4 | What is the verdict for paper 2404.7570? | `null` | `(empty; no verdict recorded)` |
| Struct | flat_100 | 4 | What is the verification status of paper 2412.9928? | `false` | `Not verified (verified = 0)` |
| Struct | flat_100 | 4 | What is the publication year of paper 2403.4527? | `null` | `Unknown (year is "~", i.e., missing)` |
| Struct | flat_100 | 4 | What is the verdict for paper 2404.7570? | `null` | `Unknown (verdict is "~", i.e., missing)` |
| Minified JSON | nested_fixed | 4 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| Minified JSON | nested_fixed | 4 | How many claims have verdict supported? | `7` | `6` |
| Minified JSON | nested_fixed | 4 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `Not specified (null)` |
| Minified JSON | nested_fixed | 4 | Which claim has no verification status? | `StarCoder 2 training code` | `Claim with no verification status: StarCoder 2 training code (verified is null)` |
| TOON | nested_fixed | 4 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| TOON | nested_fixed | 4 | How many claims have a source published in 2024? | `2` | `3` |
| TOON | nested_fixed | 4 | Which claim has no verification status? | `StarCoder 2 training code` | `Direct preference optim` |
| YAML | nested_fixed | 4 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| YAML | nested_fixed | 4 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `null (no year is recorded)` |
| XML | nested_fixed | 4 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `Unverified` |
| XML | nested_fixed | 4 | How many claims have a source published in 2024? | `2` | `3` |
| XML | nested_fixed | 4 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `No year is given (the year field is empty)` |
| Struct | nested_fixed | 4 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified (verified field is 0, i.e., false)` |
| Struct | nested_fixed | 4 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `Not given (the year field is "~")` |
| Minified JSON | nested_variable | 4 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes)` |
| Minified JSON | nested_variable | 4 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `Rope scaling hurts long ctx` |
| TOON | nested_variable | 4 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes are listed; the subnotes field is an empty list)` |
| TOON | nested_variable | 4 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `checkpoint size under 50MB` |
| YAML | nested_variable | 4 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `abstract mentions 32k limit; appendix B clarifies` |
| YAML | nested_variable | 4 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `per-head scales help; perplexity intact` |
| YAML | nested_variable | 4 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list)` |
| YAML | nested_variable | 4 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `checkpoint size under 50MB` |
| XML | nested_variable | 4 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `Abstract mentions 32k limit; appendix B clarifies` |
| XML | nested_variable | 4 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `Per-head scales help; perplexity intact` |
| XML | nested_variable | 4 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `Three` |
| XML | nested_variable | 4 | How many total claims have verdict refuted? | `2` | `Two` |
| XML | nested_variable | 4 | How many subnotes are under 'tested on GLUE and Wikisql'? | `2` | `Two` |
| XML | nested_variable | 4 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (the subnotes element is empty)` |
| Struct | nested_variable | 4 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `abstract mentions 32k limit; appendix B clarifies` |
| Struct | nested_variable | 4 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `per-head scales help; perplexity intact` |
| Struct | nested_variable | 4 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes)` |
