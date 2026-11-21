# R4-gemini-lite: Retrieval Accuracy Across Formats (Gemini 3.5 Flash Lite)

Benchmark measuring model comprehension across 7 data serialization formats on Gemini 3.5 Flash Lite via Google Gemini API.

## 1. Setup

- **Model:** gemini-3.5-flash-lite (called through Gemini API, thinking level `low`).
- **API Endpoint:** `POST https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash-lite:generateContent`
- **Reasoning budget:** no budget (thinking level `low`, budget column shows `n/a`).
- **Repeats & Order:** 4 repeats (labelled seed 1 to 4; 96 calls total planned), run strictly **seed-by-seed** (all 24 prompts of seed 1 before seed 2, etc.).
- **Parallel:** up to 4 parallel calls within the current seed.
- **Completed Seeds Scored:** 4 complete seed(s) (Seed 1, Seed 2, Seed 3, Seed 4; 96 calls total scored).
- **Input & Thinking Tokens:** As reported by the Gemini API `usageMetadata` (`promptTokenCount` and `thoughtsTokenCount`).
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
| Minified JSON | 168/192 | 87.5% | 1688 | 0 | 0 | n/a | 0 |
| TOON | 159/192 | 82.8% | 1418 | 191 | 3060 | n/a | 0 |
| YAML | 170/192 | 88.5% | 1957 | 0 | 0 | n/a | 0 |
| TSV | 62/96 | 64.6% | 2092 | 0 | 0 | n/a | 0 |
| XML | 151/192 | 78.6% | 2967 | 0 | 0 | n/a | 0 |
| Plain key: value | 75/96 | 78.1% | 2978 | 0 | 0 | n/a | 0 |
| Struct | 140/192 | 72.9% | 1372 | 0 | 0 | n/a | 0 |

* Input and thinking tokens are as reported by the Gemini API `usageMetadata` (`promptTokenCount` and `thoughtsTokenCount`).

## 4. Accuracy by Dataset

| Format | flat_tabular (20r) | flat_100 (100r) | nested_fixed (10r) | nested_variable (4r) |
|---|---|---|---|---|
| Minified JSON | 46/48 (95.8%) | 30/48 (62.5%) | 44/48 (91.7%) | 48/48 (100.0%) |
| TOON | 44/48 (91.7%) | 27/48 (56.2%) | 40/48 (83.3%) | 48/48 (100.0%) |
| YAML | 48/48 (100.0%) | 31/48 (64.6%) | 43/48 (89.6%) | 48/48 (100.0%) |
| TSV | 40/48 (83.3%) | 22/48 (45.8%) | can't hold this shape | can't hold this shape |
| XML | 40/48 (83.3%) | 25/48 (52.1%) | 38/48 (79.2%) | 48/48 (100.0%) |
| Plain key: value | 45/48 (93.8%) | 30/48 (62.5%) | can't hold this shape | can't hold this shape |
| Struct | 40/48 (83.3%) | 24/48 (50.0%) | 36/48 (75.0%) | 40/48 (83.3%) |

## 5. Accuracy by Question Kind

| Question Kind | Accuracy (Counts) | Accuracy (%) |
|---|---|---|
| single_lookup | 336/344 | 97.7% |
| nested_lookup | 120/120 | 100.0% |
| count_filter | 213/344 | 61.9% |
| null_missing | 256/344 | 74.4% |

## 6. Accuracy per Format by Question Kind

| Format | Single Lookup | Nested Lookup | Count / Filter | Null / Missing |
|---|---|---|---|---|
| Minified JSON | 56/56 (100.0%) | 24/24 (100.0%) | 38/56 (67.9%) | 50/56 (89.3%) |
| TOON | 52/56 (92.9%) | 24/24 (100.0%) | 34/56 (60.7%) | 49/56 (87.5%) |
| YAML | 56/56 (100.0%) | 24/24 (100.0%) | 39/56 (69.6%) | 51/56 (91.1%) |
| TSV | 32/32 (100.0%) | can't hold this shape | 18/32 (56.2%) | 12/32 (37.5%) |
| XML | 56/56 (100.0%) | 24/24 (100.0%) | 35/56 (62.5%) | 36/56 (64.3%) |
| Plain key: value | 32/32 (100.0%) | can't hold this shape | 17/32 (53.1%) | 26/32 (81.2%) |
| Struct | 52/56 (92.9%) | 24/24 (100.0%) | 32/56 (57.1%) | 32/56 (57.1%) |

## 7. Format Notes and Review Decisions

- **Partial Runs (Decision 5):** The evaluator scores only complete seeds (all 24 calls succeeded). Incomplete seeds are listed separately and not mixed into accuracy tables.
- **XML Empty Lists (Decision 12):** Plain XML cannot distinguish an empty list (`<subnotes/>`) from no value without schema or type attributes. This is recorded as an inherent limit of plain XML.
- **TOON Spec Adherence (Decision 13):** Tabular headers with uniform nested objects use valid Nested Field Groups (`source{id,year}` per TOON spec 9.3), numeric-looking strings like arXiv IDs are quoted (`"2405.05254"` per spec 7.2), and empty lists use `key: []` (`subnotes: []` per TOON spec 9.1).
- **Failed Requests (Decision 13):** Any request experiencing an HTTP error, quota failure, timeout, unparseable JSON, or empty response is recorded as failed and not marked done; resume reruns them.

## 8. Repeat Spread per Format

| Format | Seed 1 | Seed 2 | Seed 3 | Seed 4 | Mean* | Spread (Max-Min)* |
|---|---|---|---|---|---|---|
| Minified JSON | 85.4% | 89.6% | 85.4% | 89.6% | 87.5% | 4.2% |
| TOON | 81.2% | 85.4% | 81.2% | 83.3% | 82.8% | 4.2% |
| YAML | 87.5% | 89.6% | 87.5% | 89.6% | 88.5% | 2.1% |
| TSV | 62.5% | 62.5% | 66.7% | 66.7% | 64.6% | 4.2% |
| XML | 79.2% | 79.2% | 77.1% | 79.2% | 78.6% | 2.1% |
| Plain key: value | 87.5% | 75.0% | 75.0% | 75.0% | 78.1% | 12.5% |
| Struct | 72.9% | 72.9% | 72.9% | 72.9% | 72.9% | 0.0% |

* Mean and spread are calculated across completed seeds only.

## 9. Wrong Answers Log

Total wrong answers across completed seeds: 227

| Format | Dataset | Seed | Question | Expected | Model Answer |
|---|---|---|---|---|---|
| Minified JSON | flat_tabular | 1 | How many records have verdict supported? | `16` | `15` |
| TOON | flat_tabular | 1 | How many records were published in 2024? | `7` | `9` |
| TSV | flat_tabular | 1 | How many records were published in 2024? | `7` | `8` |
| TSV | flat_tabular | 1 | What is the publication year of paper 2203.15556? | `null` | `[empty] (not provided in the data)` |
| XML | flat_tabular | 1 | How many records were published in 2024? | `7` | `8` |
| XML | flat_tabular | 1 | What is the publication year of paper 2203.15556? | `null` | `[unknown]` |
| Struct | flat_tabular | 1 | How many records were published in 2024? | `7` | `8` |
| Struct | flat_tabular | 1 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `None` |
| Minified JSON | flat_100 | 1 | How many records have verdict refuted? | `26` | `24` |
| Minified JSON | flat_100 | 1 | How many records were published in 2024? | `11` | `13` |
| Minified JSON | flat_100 | 1 | How many records are marked as verified (true)? | `49` | `43` |
| Minified JSON | flat_100 | 1 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2402.5808, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` |
| Minified JSON | flat_100 | 1 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2402.8962, 2411.8062, 2406.4441, 2403.8179, 2405.9445` |
| TOON | flat_100 | 1 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `KV cache quantization losing accuracy on outliers` |
| TOON | flat_100 | 1 | How many records have verdict refuted? | `26` | `24` |
| TOON | flat_100 | 1 | How many records were published in 2024? | `11` | `19` |
| TOON | flat_100 | 1 | How many records are marked as verified (true)? | `49` | `37` |
| TOON | flat_100 | 1 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2410.7912, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2411.6313, 2409.9004, 2407.1634, 2408.7547` |
| TOON | flat_100 | 1 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2403.4527, 2408.6977, 2411.8062, 2406.4441, 2403.8179, 2407.9379, 2405.9445` |
| YAML | flat_100 | 1 | How many records have verdict refuted? | `26` | `21` |
| YAML | flat_100 | 1 | How many records were published in 2024? | `11` | `12` |
| YAML | flat_100 | 1 | How many records are marked as verified (true)? | `49` | `43` |
| YAML | flat_100 | 1 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2407.4006, 2408.9479, 2410.2343, 2409.9004, 2407.1634, 2402.5808, 2407.547, 2410.7912` |
| TSV | flat_100 | 1 | How many records have verdict refuted? | `26` | `24` |
| TSV | flat_100 | 1 | How many records were published in 2024? | `11` | `13` |
| TSV | flat_100 | 1 | How many records are marked as verified (true)? | `49` | `43` |
| TSV | flat_100 | 1 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2410.7912, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2409.9004, 2407.1634, 2407.9379, 2411.7691, 2408.7547, 2410.2343` |
| TSV | flat_100 | 1 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2403.4527, 2411.8062, 2402.3504, 2403.8179, 2406.7373, 2402.8962, 2405.9445` |
| TSV | flat_100 | 1 | What is the publication year of paper 2403.4527? | `null` | `unknown` |
| TSV | flat_100 | 1 | What is the verdict for paper 2404.7570? | `null` | `unknown` |
| XML | flat_100 | 1 | How many records have verdict refuted? | `26` | `16` |
| XML | flat_100 | 1 | How many records were published in 2024? | `11` | `12` |
| XML | flat_100 | 1 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2402.3504, 2412.6138, 2403.8179, 2405.9445, 2407.9379` |
| XML | flat_100 | 1 | What is the publication year of paper 2403.4527? | `null` | `` |
| XML | flat_100 | 1 | What is the verdict for paper 2404.7570? | `null` | `` |
| Plain key: value | flat_100 | 1 | How many records have verdict refuted? | `26` | `20` |
| Plain key: value | flat_100 | 1 | How many records were published in 2024? | `11` | `12` |
| Plain key: value | flat_100 | 1 | How many records are marked as verified (true)? | `49` | `43` |
| Struct | flat_100 | 1 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `KV cache quantization losing accuracy on outliers` |
| Struct | flat_100 | 1 | How many records have verdict refuted? | `26` | `24` |
| Struct | flat_100 | 1 | How many records were published in 2024? | `11` | `13` |
| Struct | flat_100 | 1 | How many records are marked as verified (true)? | `49` | `46` |
| Struct | flat_100 | 1 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2410.7912, 2403.4527, 2411.6313, 2402.5808, 2408.9479, 2411.7691, 2407.4006, 2407.1634, 2410.2343, 2408.7547, 2409.9004` |
| Struct | flat_100 | 1 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2403.4527, 2408.6977, 2402.8962, 2411.8062, 2403.8179, 2407.3088, 2405.9445, 2411.7691, 2407.4006, 2407.9379, 2402.3504, 2412.6138` |
| Minified JSON | nested_fixed | 1 | How many claims have a source published in 2024? | `2` | `3` |
| TOON | nested_fixed | 1 | How many claims have verdict supported? | `7` | `6` |
| TOON | nested_fixed | 1 | How many claims have a source published in 2024? | `2` | `3` |
| YAML | nested_fixed | 1 | How many claims have verdict supported? | `7` | `6` |
| YAML | nested_fixed | 1 | How many claims have a source published in 2024? | `2` | `3` |
| XML | nested_fixed | 1 | How many claims have verdict supported? | `7` | `6` |
| XML | nested_fixed | 1 | How many claims have a source published in 2024? | `2` | `3` |
| XML | nested_fixed | 1 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `[None/Missing]` |
| Struct | nested_fixed | 1 | How many claims have verdict supported? | `7` | `6` |
| Struct | nested_fixed | 1 | How many claims have a source published in 2024? | `2` | `3` |
| Struct | nested_fixed | 1 | Which claim has no verification status? | `StarCoder 2 training code` | `Direct preference optim` |
| Struct | nested_variable | 1 | Which note under 'LoRA matches full finetune' has no subnotes? | `checkpoint size under 50MB` | `memory footprint reduced 3x` |
| Struct | nested_variable | 1 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `optimizer state minimal` |
| TOON | flat_tabular | 2 | How many records were published in 2024? | `7` | `8` |
| TSV | flat_tabular | 2 | How many records were published in 2024? | `7` | `8` |
| TSV | flat_tabular | 2 | What is the publication year of paper 2203.15556? | `null` | `[empty] (or missing)` |
| XML | flat_tabular | 2 | How many records were published in 2024? | `7` | `8` |
| XML | flat_tabular | 2 | What is the publication year of paper 2203.15556? | `null` | `(not provided)` |
| Plain key: value | flat_tabular | 2 | How many records were published in 2024? | `7` | `6` |
| Struct | flat_tabular | 2 | How many records were published in 2024? | `7` | `8` |
| Struct | flat_tabular | 2 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `None` |
| Minified JSON | flat_100 | 2 | How many records have verdict refuted? | `26` | `23` |
| Minified JSON | flat_100 | 2 | How many records were published in 2024? | `11` | `13` |
| Minified JSON | flat_100 | 2 | How many records are marked as verified (true)? | `49` | `43` |
| Minified JSON | flat_100 | 2 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2411.8062, 2403.8179, 2402.8962, 2407.3088, 2406.4441, 2407.9379, 2405.9445, 2412.6138` |
| TOON | flat_100 | 2 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `KV cache quantization losing accuracy on outliers` |
| TOON | flat_100 | 2 | How many records have verdict refuted? | `26` | `25` |
| TOON | flat_100 | 2 | How many records are marked as verified (true)? | `49` | `44` |
| TOON | flat_100 | 2 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2410.7912, 2411.6313, 2402.5808, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` |
| YAML | flat_100 | 2 | How many records have verdict refuted? | `26` | `16` |
| YAML | flat_100 | 2 | How many records were published in 2024? | `11` | `12` |
| YAML | flat_100 | 2 | How many records are marked as verified (true)? | `49` | `43` |
| YAML | flat_100 | 2 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2402.5808, 2408.9479, 2408.7547` |
| TSV | flat_100 | 2 | How many records have verdict refuted? | `26` | `23` |
| TSV | flat_100 | 2 | How many records were published in 2024? | `11` | `17` |
| TSV | flat_100 | 2 | How many records are marked as verified (true)? | `49` | `43` |
| TSV | flat_100 | 2 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2410.7912, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2407.9379, 2411.7691, 2408.7547` |
| TSV | flat_100 | 2 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2403.4527, 2408.6977, 2402.3504, 2411.8062, 2405.9445, 2402.8962, 2403.8179, 2407.3088, 2412.6138` |
| TSV | flat_100 | 2 | What is the publication year of paper 2403.4527? | `null` | `` |
| TSV | flat_100 | 2 | What is the verdict for paper 2404.7570? | `null` | `` |
| XML | flat_100 | 2 | How many records have verdict refuted? | `26` | `17` |
| XML | flat_100 | 2 | How many records were published in 2024? | `11` | `12` |
| XML | flat_100 | 2 | How many records are marked as verified (true)? | `49` | `43` |
| XML | flat_100 | 2 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2402.3504, 2412.6138, 2403.8179, 2411.8062, 2407.9379, 2405.9445` |
| XML | flat_100 | 2 | What is the publication year of paper 2403.4527? | `null` | `unknown (or not provided)` |
| XML | flat_100 | 2 | What is the verdict for paper 2404.7570? | `null` | `unknown (or not provided)` |
| Plain key: value | flat_100 | 2 | How many records have verdict refuted? | `26` | `23` |
| Plain key: value | flat_100 | 2 | How many records were published in 2024? | `11` | `13` |
| Plain key: value | flat_100 | 2 | How many records are marked as verified (true)? | `49` | `43` |
| Plain key: value | flat_100 | 2 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2407.1634, 2411.7691, 2409.8752, 2410.2343` |
| Plain key: value | flat_100 | 2 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2402.3504, 2411.8062, 2403.8179, 2406.4441, 2407.9379, 2405.9445, 2412.6138` |
| Struct | flat_100 | 2 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `KV cache quantization losing accuracy on outliers` |
| Struct | flat_100 | 2 | How many records have verdict refuted? | `26` | `23` |
| Struct | flat_100 | 2 | How many records were published in 2024? | `11` | `13` |
| Struct | flat_100 | 2 | How many records are marked as verified (true)? | `49` | `43` |
| Struct | flat_100 | 2 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2410.7912, 2403.4527, 2411.6313, 2402.5808, 2408.9479, 2411.7691, 2408.7547, 2410.2343, 2407.4006, 2407.1634, 2409.9004` |
| Struct | flat_100 | 2 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2403.4527, 2408.6977, 2402.8962, 2411.8062, 2403.8179, 2407.3088, 2407.3088 is not it wait: 2402.3504, 2412.6138, 2405.9445, 2403.837 (let me just scan: 2410.7912, 2403.4527, 2408.6977, 2402.8962, 2411.8062, 2407.4006, 2403.8179, 2409.9004, 2407.1634, 2407.9379, 2411.7691, 2405.9445, 2402.3504, 2412.6138)` |
| Minified JSON | nested_fixed | 2 | How many claims have a source published in 2024? | `2` | `3` |
| TOON | nested_fixed | 2 | How many claims have verdict supported? | `7` | `6` |
| TOON | nested_fixed | 2 | How many claims have a source published in 2024? | `2` | `3` |
| YAML | nested_fixed | 2 | How many claims have a source published in 2024? | `2` | `3` |
| XML | nested_fixed | 2 | How many claims have verdict supported? | `7` | `6` |
| XML | nested_fixed | 2 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `[missing]` |
| Struct | nested_fixed | 2 | How many claims have verdict supported? | `7` | `6` |
| Struct | nested_fixed | 2 | How many claims have a source published in 2024? | `2` | `3` |
| Struct | nested_fixed | 2 | Which claim has no verification status? | `StarCoder 2 training code` | `Direct preference optim` |
| Struct | nested_variable | 2 | Which note under 'LoRA matches full finetune' has no subnotes? | `checkpoint size under 50MB` | `memory footprint reduced 3x` |
| Struct | nested_variable | 2 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `optimizer state minimal` |
| Minified JSON | flat_tabular | 3 | How many records have verdict supported? | `16` | `15` |
| TOON | flat_tabular | 3 | How many records were published in 2024? | `7` | `8` |
| TSV | flat_tabular | 3 | How many records were published in 2024? | `7` | `8` |
| TSV | flat_tabular | 3 | What is the publication year of paper 2203.15556? | `null` | `(missing in data)` |
| XML | flat_tabular | 3 | How many records were published in 2024? | `7` | `8` |
| XML | flat_tabular | 3 | What is the publication year of paper 2203.15556? | `null` | `(missing)` |
| Plain key: value | flat_tabular | 3 | How many records were published in 2024? | `7` | `6` |
| Struct | flat_tabular | 3 | How many records were published in 2024? | `7` | `8` |
| Struct | flat_tabular | 3 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `None` |
| Minified JSON | flat_100 | 3 | How many records have verdict refuted? | `26` | `23` |
| Minified JSON | flat_100 | 3 | How many records were published in 2024? | `11` | `13` |
| Minified JSON | flat_100 | 3 | How many records are marked as verified (true)? | `49` | `39` |
| Minified JSON | flat_100 | 3 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2402.5808, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` |
| Minified JSON | flat_100 | 3 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2402.8962, 2411.8062, 2403.8179, 2406.4441, 2407.9379, 2405.9445, 2412.6138` |
| TOON | flat_100 | 3 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `KV cache quantization losing accuracy on outliers` |
| TOON | flat_100 | 3 | How many records have verdict refuted? | `26` | `24` |
| TOON | flat_100 | 3 | How many records were published in 2024? | `11` | `17` |
| TOON | flat_100 | 3 | How many records are marked as verified (true)? | `49` | `37` |
| TOON | flat_100 | 3 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `"2410.7912", "2411.6313", "2402.5808", "2408.9479", "2407.4006", "2410.2343", "2409.9004", "2407.1634", "2411.7691", "2408.7547"` |
| TOON | flat_100 | 3 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `"2410.7912", "2403.4527", "2408.6977", "2402.3504", "2405.3287", "2406.4441", "2407.9379", "2405.9445"` |
| YAML | flat_100 | 3 | How many records have verdict refuted? | `26` | `20` |
| YAML | flat_100 | 3 | How many records were published in 2024? | `11` | `13` |
| YAML | flat_100 | 3 | How many records are marked as verified (true)? | `49` | `43` |
| YAML | flat_100 | 3 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2407.4006, 2408.9479, 2410.2343, 2407.1634, 2411.7691, 2407.547, 2402.5808 (Wait, let me double check null years: 2403.4527, 2411.6313, 2407.4006, 2408.9479, 2410.2343, 2407.1634, 2411.7691, 2407.547? Let me scan properly. Let's list the IDs with null year: 2403.4527, 2411.6313, 2407.4006, 2408.9479, 2410.2343, 2407.1634, 2411.7691, 2407.547 (Wait, 2407.547 is 2408.7547? Let's check 2408.7547 has year: null). So: 2403.4527, 2411.6313, 2407.4006, 2408.9479, 2410.2343, 2407.1634, 2411.7691, 2408.7547, 2402.5808, 2409.9004.)` |
| YAML | flat_100 | 3 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2402.3504, 2411.8062, 2405.4346 (wait, 2405.4346 is verified: true), let's check verified: null -> 2410.7912, 2408.6977, 2402.3504, 2411.8062, 2403.8179, 2406.4441, 2407.9379, 2405.9445, 2412.6138.` |
| TSV | flat_100 | 3 | How many records were published in 2024? | `11` | `17` |
| TSV | flat_100 | 3 | How many records are marked as verified (true)? | `49` | `43` |
| TSV | flat_100 | 3 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2410.7912, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2407.9379, 2411.7691, 2408.7547` |
| TSV | flat_100 | 3 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2403.4527, 2408.6977, 2402.3504, 2411.8062, 2403.8179, 2407.3088, 2402.8962, 2406.4441, 2405.9445` |
| TSV | flat_100 | 3 | What is the publication year of paper 2403.4527? | `null` | `` |
| TSV | flat_100 | 3 | What is the verdict for paper 2404.7570? | `null` | `` |
| XML | flat_100 | 3 | How many records have verdict refuted? | `26` | `16` |
| XML | flat_100 | 3 | How many records were published in 2024? | `11` | `12` |
| XML | flat_100 | 3 | How many records are marked as verified (true)? | `49` | `43` |
| XML | flat_100 | 3 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2402.3504, 2411.8062, 2407.9379, 2403.8179, 2405.9445, 2412.6138` |
| XML | flat_100 | 3 | What is the publication year of paper 2403.4527? | `null` | `[None/Not provided]` |
| XML | flat_100 | 3 | What is the verdict for paper 2404.7570? | `null` | `[None/Not provided]` |
| Plain key: value | flat_100 | 3 | How many records have verdict refuted? | `26` | `23` |
| Plain key: value | flat_100 | 3 | How many records were published in 2024? | `11` | `13` |
| Plain key: value | flat_100 | 3 | How many records are marked as verified (true)? | `49` | `43` |
| Plain key: value | flat_100 | 3 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2407.1634, 2411.7691, 2409.8752, 2410.2343` |
| Plain key: value | flat_100 | 3 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2402.3504, 2411.8062, 2403.8179, 2406.4441, 2407.9379, 2405.9445, 2412.6138` |
| Struct | flat_100 | 3 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `KV cache quantization losing accuracy on outliers` |
| Struct | flat_100 | 3 | How many records have verdict refuted? | `26` | `23` |
| Struct | flat_100 | 3 | How many records were published in 2024? | `11` | `13` |
| Struct | flat_100 | 3 | How many records are marked as verified (true)? | `49` | `43` |
| Struct | flat_100 | 3 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2410.7912, 2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` |
| Struct | flat_100 | 3 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2403.4527, 2408.6977, 2402.3504, 2411.8062, 2407.4006, 2412.6138, 2403.8179, 2409.9004, 2406.73, 2407.9379, 2405.9445` |
| Minified JSON | nested_fixed | 3 | How many claims have a source published in 2024? | `2` | `3` |
| TOON | nested_fixed | 3 | How many claims have verdict supported? | `7` | `6` |
| TOON | nested_fixed | 3 | How many claims have a source published in 2024? | `2` | `3` |
| YAML | nested_fixed | 3 | How many claims have a source published in 2024? | `2` | `3` |
| XML | nested_fixed | 3 | How many claims have verdict supported? | `7` | `6` |
| XML | nested_fixed | 3 | How many claims have a source published in 2024? | `2` | `3` |
| XML | nested_fixed | 3 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `` |
| Struct | nested_fixed | 3 | How many claims have verdict supported? | `7` | `6` |
| Struct | nested_fixed | 3 | How many claims have a source published in 2024? | `2` | `3` |
| Struct | nested_fixed | 3 | Which claim has no verification status? | `StarCoder 2 training code` | `Direct preference optim` |
| Struct | nested_variable | 3 | Which note under 'LoRA matches full finetune' has no subnotes? | `checkpoint size under 50MB` | `memory footprint reduced 3x` |
| Struct | nested_variable | 3 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `optimizer state minimal` |
| TOON | flat_tabular | 4 | How many records were published in 2024? | `7` | `8` |
| TSV | flat_tabular | 4 | How many records were published in 2024? | `7` | `8` |
| TSV | flat_tabular | 4 | What is the publication year of paper 2203.15556? | `null` | `[empty] (not provided in the data)` |
| XML | flat_tabular | 4 | How many records were published in 2024? | `7` | `8` |
| XML | flat_tabular | 4 | What is the publication year of paper 2203.15556? | `null` | `[Unknown / empty]` |
| Plain key: value | flat_tabular | 4 | How many records were published in 2024? | `7` | `6` |
| Struct | flat_tabular | 4 | How many records were published in 2024? | `7` | `8` |
| Struct | flat_tabular | 4 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `None` |
| Minified JSON | flat_100 | 4 | How many records have verdict refuted? | `26` | `23` |
| Minified JSON | flat_100 | 4 | How many records were published in 2024? | `11` | `13` |
| Minified JSON | flat_100 | 4 | How many records are marked as verified (true)? | `49` | `37` |
| Minified JSON | flat_100 | 4 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2411.8062, 2402.8962, 2403.8179, 2406.4441, 2407.9379, 2405.9445, 2412.6138` |
| TOON | flat_100 | 4 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `KV cache quantization losing accuracy on outliers` |
| TOON | flat_100 | 4 | How many records were published in 2024? | `11` | `15` |
| TOON | flat_100 | 4 | How many records are marked as verified (true)? | `49` | `37` |
| TOON | flat_100 | 4 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `"2410.7912", "2411.6313", "2402.5808", "2408.9479", "2407.4006", "2410.2343", "2407.1634", "2409.9004", "2409.3085", "2408.7547"` |
| TOON | flat_100 | 4 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `"2410.7912", "2403.4527", "2408.6977", "2406.4441", "2407.9379", "2405.9445"` |
| YAML | flat_100 | 4 | How many records have verdict refuted? | `26` | `20` |
| YAML | flat_100 | 4 | How many records were published in 2024? | `11` | `12` |
| YAML | flat_100 | 4 | How many records are marked as verified (true)? | `49` | `42` |
| YAML | flat_100 | 4 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2407.4006, 2408.9479, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2402.5808, 2407.547` |
| TSV | flat_100 | 4 | How many records were published in 2024? | `11` | `17` |
| TSV | flat_100 | 4 | How many records are marked as verified (true)? | `49` | `43` |
| TSV | flat_100 | 4 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2410.7912, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2407.9379, 2411.7691, 2408.7547` |
| TSV | flat_100 | 4 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2403.4527, 2408.6977, 2402.3504, 2411.8062, 2403.8179, 2407.3088, 2402.8962, 2406.4441, 2405.9445` |
| TSV | flat_100 | 4 | What is the publication year of paper 2403.4527? | `null` | `[missing]` |
| TSV | flat_100 | 4 | What is the verdict for paper 2404.7570? | `null` | `[missing]` |
| XML | flat_100 | 4 | How many records have verdict refuted? | `26` | `18` |
| XML | flat_100 | 4 | How many records were published in 2024? | `11` | `13` |
| XML | flat_100 | 4 | How many records are marked as verified (true)? | `49` | `46` |
| XML | flat_100 | 4 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2402.3504, 2411.8062, 2403.8179, 2407.9379, 2405.9445, 2413.6138 (Wait, 2412.6138)` |
| XML | flat_100 | 4 | What is the publication year of paper 2403.4527? | `null` | `unknown (or blank/not provided)` |
| XML | flat_100 | 4 | What is the verdict for paper 2404.7570? | `null` | `unknown (or blank/not provided)` |
| Plain key: value | flat_100 | 4 | How many records have verdict refuted? | `26` | `21` |
| Plain key: value | flat_100 | 4 | How many records were published in 2024? | `11` | `12` |
| Plain key: value | flat_100 | 4 | How many records are marked as verified (true)? | `49` | `43` |
| Plain key: value | flat_100 | 4 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2402.5808, 2407.4006, 2407.1634, 2409.9004, 2411.7691, 2408.7547, 2410.2343` |
| Plain key: value | flat_100 | 4 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2402.3504, 2411.8062, 2406.4441, 2403.8179, 2409.379, 2405.9445, 2412.6138` |
| Struct | flat_100 | 4 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `KV cache quantization losing accuracy on outliers` |
| Struct | flat_100 | 4 | How many records have verdict refuted? | `26` | `24` |
| Struct | flat_100 | 4 | How many records were published in 2024? | `11` | `14` |
| Struct | flat_100 | 4 | How many records are marked as verified (true)? | `49` | `43` |
| Struct | flat_100 | 4 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2410.7912, 2403.4527, 2411.6313, 2402.5808, 2408.9479, 2411.7691, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2408.7547` |
| Struct | flat_100 | 4 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2403.4527, 2408.6977, 2402.8962, 2411.8062, 2403.8179, 2407.3088, 2406.7373, 2407.9379, 2412.6138, 2405.9445` |
| Minified JSON | nested_fixed | 4 | How many claims have a source published in 2024? | `2` | `3` |
| TOON | nested_fixed | 4 | How many claims have verdict supported? | `7` | `6` |
| TOON | nested_fixed | 4 | How many claims have a source published in 2024? | `2` | `3` |
| YAML | nested_fixed | 4 | How many claims have a source published in 2024? | `2` | `3` |
| XML | nested_fixed | 4 | How many claims have verdict supported? | `7` | `6` |
| XML | nested_fixed | 4 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `[missing]` |
| Struct | nested_fixed | 4 | How many claims have verdict supported? | `7` | `6` |
| Struct | nested_fixed | 4 | How many claims have a source published in 2024? | `2` | `3` |
| Struct | nested_fixed | 4 | Which claim has no verification status? | `StarCoder 2 training code` | `Direct preference optim` |
| Struct | nested_variable | 4 | Which note under 'LoRA matches full finetune' has no subnotes? | `checkpoint size under 50MB` | `memory footprint reduced 3x` |
| Struct | nested_variable | 4 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `optimizer state minimal` |
