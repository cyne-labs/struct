# R4-gemini: Retrieval Accuracy Across Formats (Gemini 3.8 Flash)

Benchmark measuring model comprehension across 7 data serialization formats on Gemini 3.8 Flash.

## 1. Setup

- **Model:** gemini-3.8-flash-low (requested as `gemini-3.8-flash-low`, low variant).
- **Command Line:**
  ```
  agy -p=<prompt> --model gemini-3.8-flash-low --output-format json --sandbox --disable-slash-commands
  ```
- **Reasoning budget:** no budget (low variant).
- **Repeats:** 4 repeats per prompt (labelled seed 1 to 4; 96 calls total).
- **Parallel:** 4 parallel calls.
- **Input Tokens Note:** agy is an agent: every call includes its own system prompt and tool definitions, about 13,600 input tokens, so input tokens here are not comparable with the other models' in absolute terms; compare formats within this model only.
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
| Minified JSON | 174/192 | 90.6% | 47180 | 0 | 0 | n/a | 0 |
| TOON | 181/192 | 94.3% | 33265 | 0 | 0 | n/a | 0 |
| YAML | 173/192 | 90.1% | 23698 | 0 | 0 | n/a | 0 |
| TSV | 83/96 | 86.5% | 32756 | 0 | 0 | n/a | 0 |
| XML | 170/192 | 88.5% | 35984 | 0 | 0 | n/a | 0 |
| Plain key: value | 93/96 | 96.9% | 32610 | 0 | 0 | n/a | 0 |
| Struct | 157/192 | 81.8% | 20381 | 0 | 0 | n/a | 0 |

* Input tokens note: agy is an agent: every call includes its own system prompt and tool definitions, about 13,600 input tokens, so input tokens here are not comparable with the other models' in absolute terms; compare formats within this model only.

## 4. Accuracy by Dataset

| Format | flat_tabular (20r) | flat_100 (100r) | nested_fixed (10r) | nested_variable (4r) |
|---|---|---|---|---|
| Minified JSON | 42/48 (87.5%) | 44/48 (91.7%) | 44/48 (91.7%) | 44/48 (91.7%) |
| TOON | 47/48 (97.9%) | 46/48 (95.8%) | 44/48 (91.7%) | 44/48 (91.7%) |
| YAML | 44/48 (91.7%) | 44/48 (91.7%) | 45/48 (93.8%) | 40/48 (83.3%) |
| TSV | 43/48 (89.6%) | 40/48 (83.3%) | can't hold this shape | can't hold this shape |
| XML | 40/48 (83.3%) | 42/48 (87.5%) | 44/48 (91.7%) | 44/48 (91.7%) |
| Plain key: value | 45/48 (93.8%) | 48/48 (100.0%) | can't hold this shape | can't hold this shape |
| Struct | 38/48 (79.2%) | 36/48 (75.0%) | 40/48 (83.3%) | 43/48 (89.6%) |

## 5. Accuracy by Question Kind

| Question Kind | Accuracy (Counts) | Accuracy (%) |
|---|---|---|
| single_lookup | 330/344 | 95.9% |
| nested_lookup | 114/120 | 95.0% |
| count_filter | 335/344 | 97.4% |
| null_missing | 252/344 | 73.3% |

## 6. Accuracy per Format by Question Kind

| Format | Single Lookup | Nested Lookup | Count / Filter | Null / Missing |
|---|---|---|---|---|
| Minified JSON | 54/56 (96.4%) | 24/24 (100.0%) | 56/56 (100.0%) | 40/56 (71.4%) |
| TOON | 56/56 (100.0%) | 24/24 (100.0%) | 56/56 (100.0%) | 45/56 (80.4%) |
| YAML | 56/56 (100.0%) | 20/24 (83.3%) | 55/56 (98.2%) | 42/56 (75.0%) |
| TSV | 32/32 (100.0%) | can't hold this shape | 31/32 (96.9%) | 20/32 (62.5%) |
| XML | 56/56 (100.0%) | 24/24 (100.0%) | 52/56 (92.9%) | 38/56 (67.9%) |
| Plain key: value | 32/32 (100.0%) | can't hold this shape | 31/32 (96.9%) | 30/32 (93.8%) |
| Struct | 44/56 (78.6%) | 22/24 (91.7%) | 54/56 (96.4%) | 37/56 (66.1%) |

## 7. Format Notes and Review Decisions

- **XML Empty Lists (Decision 12):** Plain XML cannot distinguish an empty list (`<subnotes/>`) from no value without schema or type attributes. This is recorded as an inherent limit of plain XML.
- **TOON Spec Adherence (Decision 13):** Tabular headers with uniform nested objects use valid Nested Field Groups (`source{id,year}` per TOON spec 9.3), numeric-looking strings like arXiv IDs are quoted (`"2405.05254"` per spec 7.2), and empty lists use `key: []` (`subnotes: []` per TOON spec 9.1).
- **Failed Requests (Decision 13):** Any request experiencing an exit error, timeout, unparseable JSON, non-success status, num_turns > 1, or empty response is recorded as failed and not marked done; resume reruns them.

## 8. Repeat Spread per Format

| Format | Seed 1 | Seed 2 | Seed 3 | Seed 4 | Mean | Spread (Max-Min) |
|---|---|---|---|---|---|---|
| Minified JSON | 89.6% | 93.8% | 91.7% | 87.5% | 90.6% | 6.2% |
| TOON | 95.8% | 95.8% | 91.7% | 93.8% | 94.3% | 4.2% |
| YAML | 91.7% | 85.4% | 91.7% | 91.7% | 90.1% | 6.2% |
| TSV | 87.5% | 83.3% | 87.5% | 87.5% | 86.5% | 4.2% |
| XML | 87.5% | 87.5% | 87.5% | 91.7% | 88.5% | 4.2% |
| Plain key: value | 91.7% | 100.0% | 95.8% | 100.0% | 96.9% | 8.3% |
| Struct | 81.2% | 81.2% | 81.2% | 83.3% | 81.8% | 2.1% |

## 9. Wrong Answers Log

Total wrong answers: 121

| Format | Dataset | Seed | Question | Expected | Model Answer |
|---|---|---|---|---|---|
| Minified JSON | flat_tabular | 1 | What is the publication year of paper 2203.15556? | `null` | `null (no publication year provided)` |
| YAML | flat_tabular | 1 | How many records were published in 2024? | `7` | `6` |
| YAML | flat_tabular | 1 | What is the publication year of paper 2203.15556? | `null` | `None (null)` |
| TSV | flat_tabular | 1 | What is the publication year of paper 2203.15556? | `null` | `None (missing/empty)` |
| XML | flat_tabular | 1 | How many records were published in 2024? | `7` | `6` |
| XML | flat_tabular | 1 | What is the publication year of paper 2203.15556? | `null` | `None (empty / not specified)` |
| Plain key: value | flat_tabular | 1 | How many records were published in 2024? | `7` | `6` |
| Plain key: value | flat_tabular | 1 | What is the publication year of paper 2203.15556? | `null` | `null (missing/not provided)` |
| Struct | flat_tabular | 1 | What is the verification status of paper 2309.06180? | `true` | `1 (or verified / true)` |
| Struct | flat_tabular | 1 | How many records were published in 2024? | `7` | `6` |
| Struct | flat_tabular | 1 | What is the publication year of paper 2203.15556? | `null` | `None (~ / missing)` |
| Minified JSON | flat_100 | 1 | What is the publication year of paper 2403.4527? | `null` | `None (null)` |
| Minified JSON | flat_100 | 1 | What is the verdict for paper 2404.7570? | `null` | `None (null)` |
| TSV | flat_100 | 1 | What is the publication year of paper 2403.4527? | `null` | `None (missing/empty)` |
| TSV | flat_100 | 1 | What is the verdict for paper 2404.7570? | `null` | `None (missing/empty)` |
| XML | flat_100 | 1 | What is the publication year of paper 2403.4527? | `null` | `None (empty / not specified)` |
| XML | flat_100 | 1 | What is the verdict for paper 2404.7570? | `null` | `None (empty / not specified)` |
| Struct | flat_100 | 1 | What is the verification status of paper 2412.9928? | `false` | `0 (false)` |
| Struct | flat_100 | 1 | What is the publication year of paper 2403.4527? | `null` | `None (~ / missing)` |
| Struct | flat_100 | 1 | What is the verdict for paper 2404.7570? | `null` | `None (~ / missing)` |
| Minified JSON | nested_fixed | 1 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `null (no year provided)` |
| TOON | nested_fixed | 1 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `null (none)` |
| YAML | nested_fixed | 1 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `null (None)` |
| XML | nested_fixed | 1 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `None (empty / not specified)` |
| Struct | nested_fixed | 1 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `0 (unverified / false)` |
| Struct | nested_fixed | 1 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `None / missing (~)` |
| Minified JSON | nested_variable | 1 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list)` |
| TOON | nested_variable | 1 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (or empty: `[]`)` |
| YAML | nested_variable | 1 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list)` |
| XML | nested_variable | 1 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (or empty)` |
| Struct | nested_variable | 1 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (or empty list / no subnotes)` |
| Minified JSON | flat_tabular | 2 | What is the publication year of paper 2203.15556? | `null` | `None (null)` |
| YAML | flat_tabular | 2 | What is the publication year of paper 2203.15556? | `null` | `null (no publication year provided)` |
| TSV | flat_tabular | 2 | How many records were published in 2024? | `7` | `6` |
| TSV | flat_tabular | 2 | What is the publication year of paper 2203.15556? | `null` | `None (missing/empty)` |
| XML | flat_tabular | 2 | How many records were published in 2024? | `7` | `6` |
| XML | flat_tabular | 2 | What is the publication year of paper 2203.15556? | `null` | `None (empty / missing)` |
| Struct | flat_tabular | 2 | What is the verification status of paper 2309.06180? | `true` | `1 (verified)` |
| Struct | flat_tabular | 2 | How many records were published in 2024? | `7` | `6` |
| Struct | flat_tabular | 2 | What is the publication year of paper 2203.15556? | `null` | `Missing / null (indicated by `~`)` |
| YAML | flat_100 | 2 | What is the publication year of paper 2403.4527? | `null` | `None (null)` |
| YAML | flat_100 | 2 | What is the verdict for paper 2404.7570? | `null` | `None (null)` |
| TSV | flat_100 | 2 | What is the publication year of paper 2403.4527? | `null` | `None (blank)` |
| TSV | flat_100 | 2 | What is the verdict for paper 2404.7570? | `null` | `None (blank)` |
| XML | flat_100 | 2 | What is the publication year of paper 2403.4527? | `null` | `None (missing / empty)` |
| XML | flat_100 | 2 | What is the verdict for paper 2404.7570? | `null` | `None (missing / empty)` |
| Struct | flat_100 | 2 | What is the verification status of paper 2412.9928? | `false` | `0 (False / unverified)` |
| Struct | flat_100 | 2 | What is the publication year of paper 2403.4527? | `null` | `None (missing / ~)` |
| Struct | flat_100 | 2 | What is the verdict for paper 2404.7570? | `null` | `None (missing / ~)` |
| Minified JSON | nested_fixed | 2 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `null (no publication year provided)` |
| TOON | nested_fixed | 2 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `null (missing/none)` |
| YAML | nested_fixed | 2 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `null (None)` |
| XML | nested_fixed | 2 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `None (empty)` |
| Struct | nested_fixed | 2 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified (or 0 / false)` |
| Struct | nested_fixed | 2 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `None (missing / ~)` |
| Minified JSON | nested_variable | 2 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list)` |
| TOON | nested_variable | 2 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list / [])` |
| YAML | nested_variable | 2 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `'abstract mentions 32k limit' and 'appendix B clarifies'` |
| YAML | nested_variable | 2 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `'per-head scales help' and 'perplexity intact'` |
| YAML | nested_variable | 2 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list)` |
| XML | nested_variable | 2 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty)` |
| Struct | nested_variable | 2 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (it has no subnotes)` |
| Minified JSON | flat_tabular | 3 | What is the verification status of paper 2309.06180? | `true` | `true (verified)` |
| Minified JSON | flat_tabular | 3 | What is the publication year of paper 2203.15556? | `null` | `null (no publication year)` |
| TSV | flat_tabular | 3 | What is the publication year of paper 2203.15556? | `null` | `None (missing / blank)` |
| XML | flat_tabular | 3 | How many records were published in 2024? | `7` | `6` |
| XML | flat_tabular | 3 | What is the publication year of paper 2203.15556? | `null` | `None (empty / missing)` |
| Plain key: value | flat_tabular | 3 | What is the publication year of paper 2203.15556? | `null` | `null (no publication year provided)` |
| Struct | flat_tabular | 3 | What is the verification status of paper 2309.06180? | `true` | `1 (verified)` |
| Struct | flat_tabular | 3 | What is the publication year of paper 2203.15556? | `null` | `Not specified (~)` |
| TOON | flat_100 | 3 | What is the publication year of paper 2403.4527? | `null` | `None (null)` |
| TOON | flat_100 | 3 | What is the verdict for paper 2404.7570? | `null` | `None (null)` |
| TSV | flat_100 | 3 | What is the publication year of paper 2403.4527? | `null` | `None (missing/empty)` |
| TSV | flat_100 | 3 | What is the verdict for paper 2404.7570? | `null` | `None (missing/empty)` |
| XML | flat_100 | 3 | What is the publication year of paper 2403.4527? | `null` | `None (empty/missing)` |
| XML | flat_100 | 3 | What is the verdict for paper 2404.7570? | `null` | `None (empty/missing)` |
| Struct | flat_100 | 3 | What is the verification status of paper 2412.9928? | `false` | `0 (false)` |
| Struct | flat_100 | 3 | What is the publication year of paper 2403.4527? | `null` | `None (~ / missing)` |
| Struct | flat_100 | 3 | What is the verdict for paper 2404.7570? | `null` | `None (~ / missing)` |
| Minified JSON | nested_fixed | 3 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `None (null)` |
| TOON | nested_fixed | 3 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `null (missing/none)` |
| YAML | nested_fixed | 3 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `None (null)` |
| XML | nested_fixed | 3 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `None (empty/missing)` |
| Struct | nested_fixed | 3 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `0 (unverified / false)` |
| Struct | nested_fixed | 3 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `None (~ / missing)` |
| Minified JSON | nested_variable | 3 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list)` |
| TOON | nested_variable | 3 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list)` |
| YAML | nested_variable | 3 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `abstract mentions 32k limit; appendix B clarifies` |
| YAML | nested_variable | 3 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `per-head scales help; perplexity intact` |
| YAML | nested_variable | 3 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list)` |
| XML | nested_variable | 3 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes listed)` |
| Struct | nested_variable | 3 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `abstract mentions 32k limit; appendix B clarifies` |
| Struct | nested_variable | 3 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `per-head scales help; perplexity intact` |
| Minified JSON | flat_tabular | 4 | What is the verification status of paper 2309.06180? | `true` | `true (verified)` |
| Minified JSON | flat_tabular | 4 | What is the publication year of paper 2203.15556? | `null` | `null (no publication year)` |
| TOON | flat_tabular | 4 | What is the publication year of paper 2203.15556? | `null` | `null (no publication year provided)` |
| YAML | flat_tabular | 4 | What is the publication year of paper 2203.15556? | `null` | `null (no publication year provided)` |
| TSV | flat_tabular | 4 | What is the publication year of paper 2203.15556? | `null` | `None (missing/empty)` |
| XML | flat_tabular | 4 | How many records were published in 2024? | `7` | `6` |
| XML | flat_tabular | 4 | What is the publication year of paper 2203.15556? | `null` | `None (or empty / not specified)` |
| Struct | flat_tabular | 4 | What is the verification status of paper 2309.06180? | `true` | `1 (verified / true)` |
| Struct | flat_tabular | 4 | What is the publication year of paper 2203.15556? | `null` | `None (missing / ~)` |
| Minified JSON | flat_100 | 4 | What is the publication year of paper 2403.4527? | `null` | `None (null)` |
| Minified JSON | flat_100 | 4 | What is the verdict for paper 2404.7570? | `null` | `None (null)` |
| YAML | flat_100 | 4 | What is the publication year of paper 2403.4527? | `null` | `None (null)` |
| YAML | flat_100 | 4 | What is the verdict for paper 2404.7570? | `null` | `None (null)` |
| TSV | flat_100 | 4 | What is the publication year of paper 2403.4527? | `null` | `None (missing/empty)` |
| TSV | flat_100 | 4 | What is the verdict for paper 2404.7570? | `null` | `None (missing/empty)` |
| Struct | flat_100 | 4 | What is the verification status of paper 2412.9928? | `false` | `0 (false)` |
| Struct | flat_100 | 4 | What is the publication year of paper 2403.4527? | `null` | `None (~ / missing)` |
| Struct | flat_100 | 4 | What is the verdict for paper 2404.7570? | `null` | `None (~ / missing)` |
| Minified JSON | nested_fixed | 4 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `None (null)` |
| TOON | nested_fixed | 4 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `null (no year specified)` |
| XML | nested_fixed | 4 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `None (empty)` |
| Struct | nested_fixed | 4 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `0 (unverified / false)` |
| Struct | nested_fixed | 4 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `None / missing (`~`)` |
| Minified JSON | nested_variable | 4 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list)` |
| TOON | nested_variable | 4 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list)` |
| YAML | nested_variable | 4 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty list)` |
| XML | nested_variable | 4 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (empty)` |
| Struct | nested_variable | 4 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (no subnotes listed)` |
