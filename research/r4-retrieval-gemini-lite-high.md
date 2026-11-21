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
| Minified JSON | 188/192 | 97.9% | 1688 | 5508 | 18808 | n/a | 0 |
| TOON | 188/192 | 97.9% | 1418 | 4394 | 11765 | n/a | 0 |
| YAML | 190/192 | 99.0% | 1957 | 5900 | 17435 | n/a | 0 |
| TSV | 88/96 | 91.7% | 2092 | 9252 | 19216 | n/a | 0 |
| XML | 186/192 | 96.9% | 2967 | 4684 | 14646 | n/a | 0 |
| Plain key: value | 94/96 | 97.9% | 2978 | 8279 | 14701 | n/a | 0 |
| Struct | 191/192 | 99.5% | 1372 | 5408 | 15483 | n/a | 0 |

* Input and thinking tokens are as reported by the Gemini API `usageMetadata` (`promptTokenCount` and `thoughtsTokenCount`).

## 4. Accuracy by Dataset

| Format | flat_tabular (20r) | flat_100 (100r) | nested_fixed (10r) | nested_variable (4r) |
|---|---|---|---|---|
| Minified JSON | 48/48 (100.0%) | 44/48 (91.7%) | 48/48 (100.0%) | 48/48 (100.0%) |
| TOON | 48/48 (100.0%) | 44/48 (91.7%) | 48/48 (100.0%) | 48/48 (100.0%) |
| YAML | 48/48 (100.0%) | 46/48 (95.8%) | 48/48 (100.0%) | 48/48 (100.0%) |
| TSV | 47/48 (97.9%) | 41/48 (85.4%) | can't hold this shape | can't hold this shape |
| XML | 47/48 (97.9%) | 43/48 (89.6%) | 48/48 (100.0%) | 48/48 (100.0%) |
| Plain key: value | 47/48 (97.9%) | 47/48 (97.9%) | can't hold this shape | can't hold this shape |
| Struct | 48/48 (100.0%) | 47/48 (97.9%) | 48/48 (100.0%) | 48/48 (100.0%) |

## 5. Accuracy by Question Kind

| Question Kind | Accuracy (Counts) | Accuracy (%) |
|---|---|---|
| single_lookup | 343/344 | 99.7% |
| nested_lookup | 120/120 | 100.0% |
| count_filter | 335/344 | 97.4% |
| null_missing | 327/344 | 95.1% |

## 6. Accuracy per Format by Question Kind

| Format | Single Lookup | Nested Lookup | Count / Filter | Null / Missing |
|---|---|---|---|---|
| Minified JSON | 56/56 (100.0%) | 24/24 (100.0%) | 54/56 (96.4%) | 54/56 (96.4%) |
| TOON | 55/56 (98.2%) | 24/24 (100.0%) | 54/56 (96.4%) | 55/56 (98.2%) |
| YAML | 56/56 (100.0%) | 24/24 (100.0%) | 55/56 (98.2%) | 55/56 (98.2%) |
| TSV | 32/32 (100.0%) | can't hold this shape | 32/32 (100.0%) | 24/32 (75.0%) |
| XML | 56/56 (100.0%) | 24/24 (100.0%) | 55/56 (98.2%) | 51/56 (91.1%) |
| Plain key: value | 32/32 (100.0%) | can't hold this shape | 30/32 (93.8%) | 32/32 (100.0%) |
| Struct | 56/56 (100.0%) | 24/24 (100.0%) | 55/56 (98.2%) | 56/56 (100.0%) |

## 7. Format Notes and Review Decisions

- **Partial Runs (Decision 5):** The evaluator scores only complete seeds (all 24 calls succeeded). Incomplete seeds are listed separately and not mixed into accuracy tables.
- **XML Empty Lists (Decision 12):** Plain XML cannot distinguish an empty list (`<subnotes/>`) from no value without schema or type attributes. This is recorded as an inherent limit of plain XML.
- **TOON Spec Adherence (Decision 13):** Tabular headers with uniform nested objects use valid Nested Field Groups (`source{id,year}` per TOON spec 9.3), numeric-looking strings like arXiv IDs are quoted (`"2405.05254"` per spec 7.2), and empty lists use `key: []` (`subnotes: []` per TOON spec 9.1).
- **Failed Requests (Decision 13):** Any request experiencing an HTTP error, quota failure, timeout, unparseable JSON, or empty response is recorded as failed and not marked done; resume reruns them.

## 8. Repeat Spread per Format

| Format | Seed 1 | Seed 2 | Seed 3 | Seed 4 | Mean* | Spread (Max-Min)* |
|---|---|---|---|---|---|---|
| Minified JSON | 97.9% | 95.8% | 100.0% | 97.9% | 97.9% | 4.2% |
| TOON | 100.0% | 100.0% | 95.8% | 95.8% | 97.9% | 4.2% |
| YAML | 100.0% | 100.0% | 97.9% | 97.9% | 99.0% | 2.1% |
| TSV | 87.5% | 100.0% | 87.5% | 91.7% | 91.7% | 12.5% |
| XML | 97.9% | 95.8% | 95.8% | 97.9% | 96.9% | 2.1% |
| Plain key: value | 95.8% | 100.0% | 100.0% | 95.8% | 97.9% | 4.2% |
| Struct | 100.0% | 100.0% | 100.0% | 97.9% | 99.5% | 2.1% |

* Mean and spread are calculated across completed seeds only.

## 9. Wrong Answers Log

Total wrong answers across completed seeds: 27

| Format | Dataset | Seed | Question | Expected | Model Answer |
|---|---|---|---|---|---|
| XML | flat_tabular | 1 | What is the publication year of paper 2203.15556? | `null` | `Not specified` |
| Minified JSON | flat_100 | 1 | How many records have verdict refuted? | `26` | `25` |
| TSV | flat_100 | 1 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2402.5808, 2402.8962, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` |
| TSV | flat_100 | 1 | What is the publication year of paper 2403.4527? | `null` | `missing` |
| TSV | flat_100 | 1 | What is the verdict for paper 2404.7570? | `null` | `missing` |
| Plain key: value | flat_100 | 1 | How many records are marked as verified (true)? | `49` | `48` |
| Minified JSON | flat_100 | 2 | How many records are marked as verified (true)? | `49` | `45` |
| Minified JSON | flat_100 | 2 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` |
| XML | flat_100 | 2 | What is the publication year of paper 2403.4527? | `null` | `not specified` |
| XML | flat_100 | 2 | What is the verdict for paper 2404.7570? | `null` | `not specified` |
| TOON | flat_100 | 3 | How many records are marked as verified (true)? | `49` | `48` |
| TOON | flat_100 | 3 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2402.5808, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` |
| YAML | flat_100 | 3 | How many records have verdict refuted? | `26` | `25` |
| TSV | flat_100 | 3 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2408.7547` |
| TSV | flat_100 | 3 | What is the publication year of paper 2403.4527? | `null` | `missing` |
| TSV | flat_100 | 3 | What is the verdict for paper 2404.7570? | `null` | `missing` |
| XML | flat_100 | 3 | What is the publication year of paper 2403.4527? | `null` | `Not provided` |
| XML | flat_100 | 3 | What is the verdict for paper 2404.7570? | `null` | `Not provided` |
| TSV | flat_tabular | 4 | What is the publication year of paper 2203.15556? | `null` | `Not specified` |
| Plain key: value | flat_tabular | 4 | How many records were published in 2024? | `7` | `8` |
| Minified JSON | flat_100 | 4 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2407.9379, 2411.7691, 2408.7547` |
| TOON | flat_100 | 4 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `KV cache quantization losing accuracy on outliers` |
| TOON | flat_100 | 4 | How many records are marked as verified (true)? | `49` | `48` |
| YAML | flat_100 | 4 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2403.8179, 2412.6138, 2406.4441, 2407.9379, 2405.9445` |
| TSV | flat_100 | 4 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2408.7547` |
| XML | flat_100 | 4 | How many records have verdict refuted? | `26` | `23` |
| Struct | flat_100 | 4 | How many records have verdict refuted? | `26` | `25` |
