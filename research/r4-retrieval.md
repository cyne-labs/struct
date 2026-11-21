# R4: Retrieval Accuracy Across Formats

Benchmark measuring model comprehension across 7 data serialization formats on Gemma 4 E2B.

## 1. Setup

- **Model:** Gemma 4 E2B (`gemma-4-E2B-it-Q4_K_M.gguf`), thinking mode on (R4 decision 14).
- **Server:** `turboquant-llama-cpp` at `127.0.0.1:8090`, KV cache turbo4 K / turbo3 V.
- **Launch Command Line:**
  ```
  "C:\Users\Ashman Das\Documents\turboquant-llama-cpp\llama-server.exe" -m "C:\Users\Ashman Das\Documents\turboquant-llama-cpp\models\lmstudio-community\gemma-4-E2B-it-GGUF\gemma-4-E2B-it-Q4_K_M.gguf" -ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 65536 -np 4 --kv-unified --reasoning-budget 8192 --temp 1.0 --top-p 0.95 --top-k 64 --min-p 0.0 --presence-penalty 0.0 --repeat-penalty 1.0 --port 8090 --host 127.0.0.1
  ```
- **Reasoning budget:** 8192 tokens.
- **Sampling:** temperature 1.0, top_p 0.95, top_k 64, min_p 0.0, presence_penalty 0.0, repeat_penalty 1.0.
- **Seeds:** 42, 43, 44, 45, 46 (5 seeds).
- **Design:** TSV and Plain `key: value` run only on the two flat datasets (per maintainer Decision 1; marked as 'can\'t hold this shape' for nested datasets).

## 2. Standardized Prompt

The prompt is identical for every format except the data block and one line naming the format.
Here is the exact full prompt text as sent to the model (example: Struct on flat_tabular):

````text
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
````

## 3. Results Summary Across Formats

| Format | Accuracy (Counts) | Accuracy (%) | Mean Input Toks | Mean Thinking Toks | Max Thinking Toks | Budget Exhausted | Failed Runs |
|---|---|---|---|---|---|---|---|
| Minified JSON | 173/240 | 72.1% | 1687 | 1872 | 4295 | 0 | 0 |
| TOON | 148/240 | 61.7% | 1417 | 2032 | 7571 | 0 | 0 |
| YAML | 163/240 | 67.9% | 1956 | 1524 | 2948 | 0 | 0 |
| TSV | 54/120 | 45.0% | 2090 | 2446 | 4440 | 0 | 0 |
| XML | 165/240 | 68.8% | 2966 | 1640 | 3043 | 0 | 0 |
| Plain key: value | 71/120 | 59.2% | 2976 | 2056 | 3535 | 0 | 0 |
| Struct | 131/240 | 54.6% | 1371 | 2148 | 6887 | 0 | 0 |

## 4. Accuracy by Dataset

| Format | flat_tabular (20r) | flat_100 (100r) | nested_fixed (10r) | nested_variable (4r) |
|---|---|---|---|---|
| Minified JSON | 48/60 (80.0%) | 25/60 (41.7%) | 51/60 (85.0%) | 49/60 (81.7%) |
| TOON | 38/60 (63.3%) | 17/60 (28.3%) | 53/60 (88.3%) | 40/60 (66.7%) |
| YAML | 45/60 (75.0%) | 25/60 (41.7%) | 47/60 (78.3%) | 46/60 (76.7%) |
| TSV | 36/60 (60.0%) | 18/60 (30.0%) | can't hold this shape | can't hold this shape |
| XML | 43/60 (71.7%) | 18/60 (30.0%) | 49/60 (81.7%) | 55/60 (91.7%) |
| Plain key: value | 45/60 (75.0%) | 26/60 (43.3%) | can't hold this shape | can't hold this shape |
| Struct | 34/60 (56.7%) | 16/60 (26.7%) | 51/60 (85.0%) | 30/60 (50.0%) |

## 5. Accuracy by Question Kind

| Question Kind | Accuracy (Counts) | Accuracy (%) |
|---|---|---|
| single_lookup | 373/430 | 86.7% |
| nested_lookup | 129/150 | 86.0% |
| count_filter | 183/430 | 42.6% |
| null_missing | 220/430 | 51.2% |

## 6. Accuracy per Format by Question Kind

| Format | Single Lookup | Nested Lookup | Count / Filter | Null / Missing |
|---|---|---|---|---|
| Minified JSON | 63/70 (90.0%) | 27/30 (90.0%) | 36/70 (51.4%) | 47/70 (67.1%) |
| TOON | 56/70 (80.0%) | 26/30 (86.7%) | 28/70 (40.0%) | 38/70 (54.3%) |
| YAML | 60/70 (85.7%) | 24/30 (80.0%) | 35/70 (50.0%) | 44/70 (62.9%) |
| TSV | 38/40 (95.0%) | can't hold this shape | 12/40 (30.0%) | 4/40 (10.0%) |
| XML | 65/70 (92.9%) | 30/30 (100.0%) | 35/70 (50.0%) | 35/70 (50.0%) |
| Plain key: value | 37/40 (92.5%) | can't hold this shape | 10/40 (25.0%) | 24/40 (60.0%) |
| Struct | 54/70 (77.1%) | 22/30 (73.3%) | 27/70 (38.6%) | 28/70 (40.0%) |

## 7. Format Notes and Review Decisions

- **XML Empty Lists (Decision 12):** Plain XML cannot distinguish an empty list (`<subnotes/>`) from no value without schema or type attributes. This is recorded as an inherent limit of plain XML.
- **TOON Spec Adherence (Decision 13):** Tabular headers with uniform nested objects use valid Nested Field Groups (`source{id,year}` per TOON spec 9.3), numeric-looking strings like arXiv IDs are quoted (`"2405.05254"` per spec 7.2), and empty lists use `key: []` (`subnotes: []` per TOON spec 9.1).
- **Failed Requests (Decision 13):** Any request experiencing an HTTP error, timeout, or empty text is recorded as failed and not marked done; resume reruns them.

## 8. Seed Spread per Format

| Format | Seed 42 | Seed 43 | Seed 44 | Seed 45 | Seed 46 | Mean | Spread (Max-Min) |
|---|---|---|---|---|---|---|---|
| Minified JSON | 66.7% | 75.0% | 70.8% | 75.0% | 72.9% | 72.1% | 8.3% |
| TOON | 66.7% | 56.2% | 58.3% | 62.5% | 64.6% | 61.7% | 10.4% |
| YAML | 39.6% | 83.3% | 58.3% | 81.2% | 77.1% | 67.9% | 43.8% |
| TSV | 41.7% | 45.8% | 37.5% | 45.8% | 54.2% | 45.0% | 16.7% |
| XML | 68.8% | 68.8% | 70.8% | 66.7% | 68.8% | 68.8% | 4.2% |
| Plain key: value | 62.5% | 54.2% | 62.5% | 62.5% | 54.2% | 59.2% | 8.3% |
| Struct | 56.2% | 47.9% | 60.4% | 50.0% | 58.3% | 54.6% | 12.5% |

## 9. Wrong Answers Log

Total wrong answers: 535

| Format | Dataset | Seed | Question | Expected | Model Answer |
|---|---|---|---|---|---|
| Minified JSON | flat_tabular | 42 | How many records have verdict supported? | `16` | `12` |
| Minified JSON | flat_tabular | 42 | How many records were published in 2024? | `7` | `6` |
| TOON | flat_tabular | 42 | How many records have verdict supported? | `16` | `11` |
| TOON | flat_tabular | 42 | How many records were published in 2024? | `7` | `4` |
| TOON | flat_tabular | 42 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2403.09611, 2404.14219, 2408.03314` |
| YAML | flat_tabular | 42 | How many records have verdict supported? | `16` | `12` |
| YAML | flat_tabular | 42 | How many records were published in 2024? | `7` | `5` |
| TSV | flat_tabular | 42 | How many records have verdict supported? | `16` | `14` |
| TSV | flat_tabular | 42 | How many records were published in 2024? | `7` | `6` |
| TSV | flat_tabular | 42 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2404.14219, 2408.03314` |
| TSV | flat_tabular | 42 | Which paper ID has no verification status? | `2403.09611` | `None of the papers listed have no verdict. (All papers have a verdict status.)` |
| TSV | flat_tabular | 42 | What is the publication year of paper 2203.15556? | `null` | `` |
| TSV | flat_tabular | 42 | Which paper ID has no verdict? | `2408.03314` | `None of the papers listed have no verdict.` |
| XML | flat_tabular | 42 | How many records have verdict supported? | `16` | `13` |
| XML | flat_tabular | 42 | How many records were published in 2024? | `7` | `6` |
| XML | flat_tabular | 42 | What is the publication year of paper 2203.15556? | `null` | `(The year tag is empty)` |
| Plain key: value | flat_tabular | 42 | How many records have verdict refuted? | `1` | `13` |
| Plain key: value | flat_tabular | 42 | How many records have verdict supported? | `16` | `13` |
| Plain key: value | flat_tabular | 42 | How many records were published in 2024? | `7` | `5` |
| Struct | flat_tabular | 42 | What is the verification status of paper 2309.06180? | `true` | `supported` |
| Struct | flat_tabular | 42 | How many records have verdict supported? | `16` | `14` |
| Struct | flat_tabular | 42 | Which paper ID has no verification status? | `2403.09611` | `None` |
| Struct | flat_tabular | 42 | Which paper ID has no verdict? | `2408.03314` | `None` |
| Minified JSON | flat_100 | 42 | How many records have verdict refuted? | `26` | `9` |
| Minified JSON | flat_100 | 42 | How many records were published in 2024? | `11` | `4` |
| Minified JSON | flat_100 | 42 | How many records are marked as verified (true)? | `49` | `12` |
| Minified JSON | flat_100 | 42 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2411.9702` |
| Minified JSON | flat_100 | 42 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2410.2343, 2408.9479, 2407.1634, 2411.7691, 2407.4006` |
| Minified JSON | flat_100 | 42 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2412.6138, 2402.3504` |
| TOON | flat_100 | 42 | How many records have verdict refuted? | `26` | `11` |
| TOON | flat_100 | 42 | How many records were published in 2024? | `11` | `5` |
| TOON | flat_100 | 42 | How many records are marked as verified (true)? | `49` | `19` |
| TOON | flat_100 | 42 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2410.7912, 2408.6977, 2411.6313, 2402.3504, 2408.7547, 2407.1634, 2411.7691, 2411.9702` |
| TOON | flat_100 | 42 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2408.6977, 2411.6313, 2411.7691, 2411.9702, 2402.3504, 2408.7547, 2407.1634` |
| TOON | flat_100 | 42 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2402.5808, 2404.7570, 2411.3851` |
| YAML | flat_100 | 42 | What is the verdict for paper 2411.2824? | `unverified` | `the verdict for paper 2411.2824 is unverified` |
| YAML | flat_100 | 42 | What is the publication year of paper 2404.3286? | `2023` | `the publication year of paper 2404.3286 is 2023` |
| YAML | flat_100 | 42 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `the title of paper 2404.9279 is KV cache quantization losing accuracy on outliers v4` |
| YAML | flat_100 | 42 | What is the verification status of paper 2412.9928? | `false` | `the verification status of paper 2412.9928 is false` |
| YAML | flat_100 | 42 | How many records have verdict refuted? | `26` | `10 records have verdict refuted` |
| YAML | flat_100 | 42 | How many records were published in 2024? | `11` | `2 records were published in 2024 (2409.5803, 2403.9348, 2402.3504)` |
| YAML | flat_100 | 42 | How many records are marked as verified (true)? | `49` | `7 records are marked as verified (true)` |
| YAML | flat_100 | 42 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `The paper IDs with no verdict (null) are: 2410.7912, 2403.4527, 2408.9479, 2411.5374, 2411.7691, 2409.9004, 2404.7570, 2408.7547.` |
| YAML | flat_100 | 42 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `The paper IDs with no publication year (null) are: 2403.4527, 2411.5905, 2408.9479, 2407.1634, 2407.2964, 2402.5808, 2411.7691.` |
| YAML | flat_100 | 42 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `The paper IDs with no verification status (null) are: 2410.2638, 2411.7691, 2407.1634.` |
| YAML | flat_100 | 42 | What is the publication year of paper 2403.4527? | `null` | `the publication year of paper 2403.4527 is null` |
| YAML | flat_100 | 42 | What is the verdict for paper 2404.7570? | `null` | `the verdict for paper 2404.7570 is null` |
| TSV | flat_100 | 42 | How many records have verdict refuted? | `26` | `10` |
| TSV | flat_100 | 42 | How many records were published in 2024? | `11` | `6` |
| TSV | flat_100 | 42 | How many records are marked as verified (true)? | `49` | `10` |
| TSV | flat_100 | 42 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2410.7912, 2401.3615, 2403.4527, 2402.3504, 2411.5905, 2411.8062, 2410.2343, 2409.9004, 2407.1634 (or if strictly blank, 9)` |
| TSV | flat_100 | 42 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `4` |
| TSV | flat_100 | 42 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `9` |
| TSV | flat_100 | 42 | What is the publication year of paper 2403.4527? | `null` | `2020` |
| TSV | flat_100 | 42 | What is the verdict for paper 2404.7570? | `null` | `true` |
| XML | flat_100 | 42 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v32` |
| XML | flat_100 | 42 | How many records have verdict refuted? | `26` | `11` |
| XML | flat_100 | 42 | How many records were published in 2024? | `11` | `4` |
| XML | flat_100 | 42 | How many records are marked as verified (true)? | `49` | `15` |
| XML | flat_100 | 42 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2407.1634` |
| XML | flat_100 | 42 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977` |
| XML | flat_100 | 42 | What is the publication year of paper 2403.4527? | `null` | `` |
| XML | flat_100 | 42 | What is the verdict for paper 2404.7570? | `null` | `` |
| Plain key: value | flat_100 | 42 | How many records have verdict refuted? | `26` | `9` |
| Plain key: value | flat_100 | 42 | How many records were published in 2024? | `11` | `5` |
| Plain key: value | flat_100 | 42 | How many records are marked as verified (true)? | `49` | `10` |
| Plain key: value | flat_100 | 42 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2410.7912, 2408.6977, 2404.7570, 2411.9702` |
| Plain key: value | flat_100 | 42 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2408.9479, 2411.6313, 2407.1634, 2409.9004` |
| Plain key: value | flat_100 | 42 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2412.1887, 2402.3504, 2406.4441, 2404.7570` |
| Struct | flat_100 | 42 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `ReAct synergizing reasoning and acting in language` |
| Struct | flat_100 | 42 | What is the verification status of paper 2412.9928? | `false` | `supported` |
| Struct | flat_100 | 42 | How many records have verdict refuted? | `26` | `11` |
| Struct | flat_100 | 42 | How many records were published in 2024? | `11` | `3` |
| Struct | flat_100 | 42 | How many records are marked as verified (true)? | `49` | `13` |
| Struct | flat_100 | 42 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2411.2824, 2409.5803, 2411.5374, 2411.5889, 2411.5291, 2411.3041, 2403.8179, 2403.8787, 2408.7790, 2408.6180, 2410.2638, 2403.6718, 2403.8179, 2408.9479, 2404.9835, 2412.6107, 2406.9346, 2406.6000, 2409.3085, 2411.3851, 2411.3851, 2409.1006, 2409.9004, 2411.5905, 2405.4346, 2402.2753, 2408.7547, 2411.5889, 2408.6977, 2405.1727, 2407.1634, 2406.8066` |
| Struct | flat_100 | 42 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `11` |
| Struct | flat_100 | 42 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `0` |
| Struct | flat_100 | 42 | What is the publication year of paper 2403.4527? | `null` | `2020` |
| Minified JSON | nested_fixed | 42 | How many claims have verdict supported? | `7` | `6` |
| Minified JSON | nested_fixed | 42 | Which claim has no verification status? | `StarCoder 2 training code` | `StarCoder 2 training code (The verification status is null)` |
| TOON | nested_fixed | 42 | How many claims have verdict supported? | `7` | `5` |
| YAML | nested_fixed | 42 | What is the verdict for 'Attention is all you need'? | `supported` | `verdict for 'Attention is all you need': supported` |
| YAML | nested_fixed | 42 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `verification status for 'Mistral fine tuning rules': false` |
| YAML | nested_fixed | 42 | What is the verdict for 'Rope scaling hurts long ctx'? | `refuted` | `verdict for 'Rope scaling hurts long ctx': refuted` |
| YAML | nested_fixed | 42 | What is the source paper ID for 'Attention is all you need'? | `1706.03762` | `source paper ID for 'Attention is all you need': 1706.03762` |
| YAML | nested_fixed | 42 | What is the publication year of the source for 'LoRA low rank adaptation'? | `2021` | `publication year of the source for 'LoRA low rank adaptation': 2021` |
| YAML | nested_fixed | 42 | What is the source paper ID for 'Chinchilla scaling compute'? | `2203.15556` | `source paper ID for 'Chinchilla scaling compute': 2203.15556` |
| YAML | nested_fixed | 42 | How many claims have verdict supported? | `7` | `number of claims with verdict supported: 7` |
| YAML | nested_fixed | 42 | How many claims have a source published in 2024? | `2` | `number of claims with a source published in 2024: 2` |
| YAML | nested_fixed | 42 | Which source paper IDs belong to refuted claims? | `2405.05254` | `source paper IDs for refuted claims: 2405.05254` |
| YAML | nested_fixed | 42 | Which claim has no source year? | `DeepSeekMoE mixture experts` | `claim with no source year: DeepSeekMoE mixture experts` |
| YAML | nested_fixed | 42 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `publication year of the source for 'DeepSeekMoE mixture experts': null` |
| YAML | nested_fixed | 42 | Which claim has no verification status? | `StarCoder 2 training code` | `claim with no verification status: StarCoder 2 training code` |
| XML | nested_fixed | 42 | How many claims have verdict supported? | `7` | `6` |
| XML | nested_fixed | 42 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `(No year specified / Empty)` |
| Struct | nested_fixed | 42 | What is the verdict for 'Attention is all you need'? | `supported` | `refuted` |
| Struct | nested_fixed | 42 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| Minified JSON | nested_variable | 42 | What is the verdict for 'LoRA matches full finetune'? | `supported` | `refuted` |
| Minified JSON | nested_variable | 42 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `["abstract mentions 32k limit", "appendix B clarifies"]` |
| Minified JSON | nested_variable | 42 | What subnotes are listed under the note 'table 3 contradicts'? | `llama 7b tested only` | `["llama 7b tested only"]` |
| Minified JSON | nested_variable | 42 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `["per-head scales help", "perplexity intact"]` |
| Minified JSON | nested_variable | 42 | How many subnotes are under 'tested on GLUE and Wikisql'? | `2` | `0` |
| Minified JSON | nested_variable | 42 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `Rope scaling hurts long ctx` |
| TOON | nested_variable | 42 | What subnotes are listed under the note 'table 3 contradicts'? | `llama 7b tested only` | `(None)` |
| TOON | nested_variable | 42 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `1` |
| TOON | nested_variable | 42 | How many subnotes are under 'tested on GLUE and Wikisql'? | `2` | `1` |
| TOON | nested_variable | 42 | Which note under 'LoRA matches full finetune' has no subnotes? | `checkpoint size under 50MB` | `table 3 contradicts` |
| TOON | nested_variable | 42 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `(None)` |
| TOON | nested_variable | 42 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `table 3 contradicts` |
| YAML | nested_variable | 42 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `1` |
| YAML | nested_variable | 42 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `` |
| YAML | nested_variable | 42 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `checkpoint size under 50MB` |
| XML | nested_variable | 42 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `2` |
| XML | nested_variable | 42 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (or empty)` |
| Struct | nested_variable | 42 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `abstract mentions 32k limit` |
| Struct | nested_variable | 42 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `2` |
| Struct | nested_variable | 42 | How many total claims have verdict refuted? | `2` | `1` |
| Struct | nested_variable | 42 | Which note under 'LoRA matches full finetune' has no subnotes? | `checkpoint size under 50MB` | `None (Based on the provided data, all notes listed have subnotes.)` |
| Struct | nested_variable | 42 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `optimizer state minimal` |
| Struct | nested_variable | 42 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `None (Based on the provided data, all notes listed have subnotes.)` |
| Minified JSON | flat_tabular | 43 | How many records have verdict supported? | `16` | `17` |
| Minified JSON | flat_tabular | 43 | How many records were published in 2024? | `7` | `4` |
| TOON | flat_tabular | 43 | What is the verification status of paper 2309.06180? | `true` | `false` |
| TOON | flat_tabular | 43 | How many records have verdict supported? | `16` | `6` |
| TOON | flat_tabular | 43 | How many records were published in 2024? | `7` | `5` |
| TOON | flat_tabular | 43 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556,2408.03314` |
| TOON | flat_tabular | 43 | Which paper ID has no verification status? | `2403.09611` | `2403.09611,2404.14219` |
| TOON | flat_tabular | 43 | Which paper ID has no verdict? | `2408.03314` | `None` |
| YAML | flat_tabular | 43 | How many records have verdict supported? | `16` | `12` |
| YAML | flat_tabular | 43 | How many records were published in 2024? | `7` | `4` |
| TSV | flat_tabular | 43 | How many records have verdict supported? | `16` | `12` |
| TSV | flat_tabular | 43 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2404.14219, 2408.03314` |
| TSV | flat_tabular | 43 | Which paper ID has no verification status? | `2403.09611` | `2403.09611, 2404.14219` |
| TSV | flat_tabular | 43 | What is the publication year of paper 2203.15556? | `null` | `` |
| TSV | flat_tabular | 43 | Which paper ID has no verdict? | `2408.03314` | `None` |
| XML | flat_tabular | 43 | How many records have verdict supported? | `16` | `10` |
| XML | flat_tabular | 43 | How many records were published in 2024? | `7` | `6` |
| XML | flat_tabular | 43 | Which paper ID has no verification status? | `2403.09611` | `2408.03314` |
| XML | flat_tabular | 43 | What is the publication year of paper 2203.15556? | `null` | `2024` |
| Plain key: value | flat_tabular | 43 | What is the verification status of paper 2309.06180? | `true` | `false` |
| Plain key: value | flat_tabular | 43 | How many records have verdict supported? | `16` | `14` |
| Plain key: value | flat_tabular | 43 | Which paper ID has no verification status? | `2403.09611` | `2203.15556, 2403.09611` |
| Plain key: value | flat_tabular | 43 | Which paper ID has no verdict? | `2408.03314` | `2408.03314, 2403.09611` |
| Struct | flat_tabular | 43 | What is the verification status of paper 2309.06180? | `true` | `supported` |
| Struct | flat_tabular | 43 | How many records have verdict supported? | `16` | `11` |
| Struct | flat_tabular | 43 | How many records were published in 2024? | `7` | `5` |
| Struct | flat_tabular | 43 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556` |
| Struct | flat_tabular | 43 | Which paper ID has no verification status? | `2403.09611` | `2408.03314` |
| Struct | flat_tabular | 43 | Which paper ID has no verdict? | `2408.03314` | `` |
| Minified JSON | flat_100 | 43 | What is the verdict for paper 2411.2824? | `unverified` | `supported` |
| Minified JSON | flat_100 | 43 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `ReAct synergizing reasoning and acting in language` |
| Minified JSON | flat_100 | 43 | How many records have verdict refuted? | `26` | `12` |
| Minified JSON | flat_100 | 43 | How many records were published in 2024? | `11` | `3` |
| Minified JSON | flat_100 | 43 | How many records are marked as verified (true)? | `49` | `38` |
| Minified JSON | flat_100 | 43 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2410.7912, 2408.6977, 2411.5374, 2403.8179, 2402.3504, 2408.9479, 2408.7547, 2411.7691, 2407.1634, 2409.5119, 2402.2753, 2410.2638, 2411.3851, 2408.7790, 2405.4346, 2405.7484, 2411.5905, 2409.9004, 2411.3041, 2411.5291, 2407.2964, 2406.8066, 2408.2604, 2406.9346, 2412.6107, 2409.3085, 2409.1006, 2409.5803, 2402.8962, 2403.3900, 2411.3851, 2401.2889, 2411.9702` |
| Minified JSON | flat_100 | 43 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `11` |
| Minified JSON | flat_100 | 43 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `17` |
| Minified JSON | flat_100 | 43 | What is the verdict for paper 2404.7570? | `null` | `unverified` |
| TOON | flat_100 | 43 | What is the verdict for paper 2411.2824? | `unverified` | `true` |
| TOON | flat_100 | 43 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v40` |
| TOON | flat_100 | 43 | What is the verification status of paper 2412.9928? | `false` | `true` |
| TOON | flat_100 | 43 | How many records have verdict refuted? | `26` | `13` |
| TOON | flat_100 | 43 | How many records were published in 2024? | `11` | `4` |
| TOON | flat_100 | 43 | How many records are marked as verified (true)? | `49` | `25` |
| TOON | flat_100 | 43 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `15 (2408.6977, 2411.6313, 2402.5808, 2411.7691, 2411.9702, 2407.1634, 2408.9479, 2407.4006, 2405.7484, 2407.1634, 2406.9346, 2401.2889, 2401.2776, 2407.9379, 2408.6180, 2411.7691, 2409.1006, 2411.9702)` |
| TOON | flat_100 | 43 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `18 (2411.6313, 2402.5808, 2403.8179, 2402.3504, 2402.4848, 2405.1727, 2411.3851, 2411.9702)` |
| TOON | flat_100 | 43 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `11 (2408.6977, 2411.6313, 2402.5808, 2411.7691, 2411.3851, 2411.3851, 2411.3041)` |
| TOON | flat_100 | 43 | What is the publication year of paper 2403.4527? | `null` | `2020` |
| TOON | flat_100 | 43 | What is the verdict for paper 2404.7570? | `null` | `supported` |
| YAML | flat_100 | 43 | How many records have verdict refuted? | `26` | `11` |
| YAML | flat_100 | 43 | How many records were published in 2024? | `11` | `4` |
| YAML | flat_100 | 43 | How many records are marked as verified (true)? | `49` | `13` |
| YAML | flat_100 | 43 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2408.9479, 2410.2343, 2407.1634, 2409.9004, 2407.4006` |
| YAML | flat_100 | 43 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2403.8179` |
| TSV | flat_100 | 43 | How many records have verdict refuted? | `26` | `6` |
| TSV | flat_100 | 43 | How many records were published in 2024? | `11` | `4` |
| TSV | flat_100 | 43 | How many records are marked as verified (true)? | `49` | `12` |
| TSV | flat_100 | 43 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2408.6977, 2408.6180, 2408.9479, 2402.3504, 2402.8962, 2408.2604, 2411.6313, 2407.1634, 2406.9346` |
| TSV | flat_100 | 43 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2408.6977, 2411.7691` |
| TSV | flat_100 | 43 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2408.6977, 2411.7691` |
| TSV | flat_100 | 43 | What is the publication year of paper 2403.4527? | `null` | `2020` |
| TSV | flat_100 | 43 | What is the verdict for paper 2404.7570? | `null` | `true` |
| XML | flat_100 | 43 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v40` |
| XML | flat_100 | 43 | How many records have verdict refuted? | `26` | `12` |
| XML | flat_100 | 43 | How many records were published in 2024? | `11` | `5` |
| XML | flat_100 | 43 | How many records are marked as verified (true)? | `49` | `13` |
| XML | flat_100 | 43 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `5` |
| XML | flat_100 | 43 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `4` |
| XML | flat_100 | 43 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `4` |
| XML | flat_100 | 43 | What is the publication year of paper 2403.4527? | `null` | `` |
| XML | flat_100 | 43 | What is the verdict for paper 2404.7570? | `null` | `` |
| Plain key: value | flat_100 | 43 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v3` |
| Plain key: value | flat_100 | 43 | How many records have verdict refuted? | `26` | `16` |
| Plain key: value | flat_100 | 43 | How many records were published in 2024? | `11` | `6` |
| Plain key: value | flat_100 | 43 | How many records are marked as verified (true)? | `49` | `20` |
| Plain key: value | flat_100 | 43 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `12` |
| Plain key: value | flat_100 | 43 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `6` |
| Plain key: value | flat_100 | 43 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `10` |
| Struct | flat_100 | 43 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v4` |
| Struct | flat_100 | 43 | What is the verification status of paper 2412.9928? | `false` | `supported` |
| Struct | flat_100 | 43 | How many records have verdict refuted? | `26` | `12` |
| Struct | flat_100 | 43 | How many records were published in 2024? | `11` | `5` |
| Struct | flat_100 | 43 | How many records are marked as verified (true)? | `49` | `12` |
| Struct | flat_100 | 43 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `18` |
| Struct | flat_100 | 43 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `13` |
| Struct | flat_100 | 43 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `18` |
| Struct | flat_100 | 43 | What is the publication year of paper 2403.4527? | `null` | `2020` |
| TOON | nested_fixed | 43 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| TOON | nested_fixed | 43 | Which claim has no verification status? | `StarCoder 2 training code` | `Mistral fine tuning rules` |
| XML | nested_fixed | 43 | How many claims have verdict supported? | `7` | `6` |
| XML | nested_fixed | 43 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `(No publication year provided/empty)` |
| Struct | nested_fixed | 43 | How many claims have verdict supported? | `7` | `5` |
| Struct | nested_fixed | 43 | Which claim has no verification status? | `StarCoder 2 training code` | `Mistral fine tuning rules` |
| Minified JSON | nested_variable | 43 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `` |
| TOON | nested_variable | 43 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `1` |
| TOON | nested_variable | 43 | How many subnotes are under 'tested on GLUE and Wikisql'? | `2` | `1` |
| YAML | nested_variable | 43 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `` |
| Struct | nested_variable | 43 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `>>abstract mentions 32k limit` |
| Struct | nested_variable | 43 | What subnotes are listed under the note 'table 3 contradicts'? | `llama 7b tested only` | `>>llama 7b tested only` |
| Struct | nested_variable | 43 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `>>per-head scales help, >>perplexity intact` |
| Struct | nested_variable | 43 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `1` |
| Struct | nested_variable | 43 | How many total claims have verdict refuted? | `2` | `1` |
| Struct | nested_variable | 43 | Which note under 'LoRA matches full finetune' has no subnotes? | `checkpoint size under 50MB` | `>tested on GLUE and Wikisql` |
| Struct | nested_variable | 43 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `>>optimizer state minimal` |
| Struct | nested_variable | 43 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `None of the listed notes have no subnotes.` |
| Minified JSON | flat_tabular | 44 | How many records have verdict supported? | `16` | `14` |
| Minified JSON | flat_tabular | 44 | How many records were published in 2024? | `7` | `2` |
| Minified JSON | flat_tabular | 44 | Which paper IDs have verdict unverified? | `2310.06825, 2406.11794` | `2310.06825, 2408.03314, 2406.11794` |
| Minified JSON | flat_tabular | 44 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556` |
| Minified JSON | flat_tabular | 44 | Which paper ID has no verification status? | `2403.09611` | `2408.03314` |
| TOON | flat_tabular | 44 | What is the verification status of paper 2309.06180? | `true` | `false` |
| TOON | flat_tabular | 44 | How many records have verdict supported? | `16` | `13` |
| TOON | flat_tabular | 44 | How many records were published in 2024? | `7` | `6` |
| TOON | flat_tabular | 44 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2408.03314` |
| YAML | flat_tabular | 44 | How many records have verdict supported? | `16` | `12` |
| YAML | flat_tabular | 44 | How many records were published in 2024? | `7` | `6` |
| YAML | flat_tabular | 44 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2404.14219, 2403.09611` |
| YAML | flat_tabular | 44 | Which paper ID has no verification status? | `2403.09611` | `2308.12950, 2403.09611` |
| TSV | flat_tabular | 44 | How many records have verdict supported? | `16` | `12` |
| TSV | flat_tabular | 44 | How many records were published in 2024? | `7` | `5` |
| TSV | flat_tabular | 44 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2404.14219` |
| TSV | flat_tabular | 44 | Which paper ID has no verification status? | `2403.09611` | `2404.14219` |
| TSV | flat_tabular | 44 | What is the publication year of paper 2203.15556? | `null` | `` |
| TSV | flat_tabular | 44 | Which paper ID has no verdict? | `2408.03314` | `2404.14219` |
| XML | flat_tabular | 44 | How many records have verdict supported? | `16` | `15` |
| XML | flat_tabular | 44 | How many records were published in 2024? | `7` | `6` |
| XML | flat_tabular | 44 | What is the publication year of paper 2203.15556? | `null` | `(The year tag is empty)` |
| Plain key: value | flat_tabular | 44 | How many records have verdict supported? | `16` | `15` |
| Plain key: value | flat_tabular | 44 | How many records were published in 2024? | `7` | `6` |
| Plain key: value | flat_tabular | 44 | Which paper ID has no verification status? | `2403.09611` | `2403.09611, 2404.14219` |
| Struct | flat_tabular | 44 | What is the verification status of paper 2309.06180? | `true` | `supported` |
| Struct | flat_tabular | 44 | How many records have verdict supported? | `16` | `12` |
| Struct | flat_tabular | 44 | How many records were published in 2024? | `7` | `6` |
| Struct | flat_tabular | 44 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2404.14219, 2408.03314` |
| Struct | flat_tabular | 44 | Which paper ID has no verification status? | `2403.09611` | `None` |
| Struct | flat_tabular | 44 | Which paper ID has no verdict? | `2408.03314` | `` |
| Minified JSON | flat_100 | 44 | How many records have verdict refuted? | `26` | `14` |
| Minified JSON | flat_100 | 44 | How many records were published in 2024? | `11` | `4` |
| Minified JSON | flat_100 | 44 | How many records are marked as verified (true)? | `49` | `12` |
| Minified JSON | flat_100 | 44 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2403.4527, 2408.9479` |
| Minified JSON | flat_100 | 44 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2408.9479` |
| Minified JSON | flat_100 | 44 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2402.3504, 2408.6977, 2411.9702, 2408.7216, 2411.5905 (and others with null/false values)` |
| TOON | flat_100 | 44 | What is the verdict for paper 2411.2824? | `unverified` | `The verdict for paper 2411.2824 is true` |
| TOON | flat_100 | 44 | What is the publication year of paper 2404.3286? | `2023` | `The publication year of paper 2404.3286 is 2023` |
| TOON | flat_100 | 44 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `The title of paper 2404.9279 is KV cache quantization losing accuracy on outliers` |
| TOON | flat_100 | 44 | What is the verification status of paper 2412.9928? | `false` | `The verification status of paper 2412.9928 is false` |
| TOON | flat_100 | 44 | How many records have verdict refuted? | `26` | `13` |
| TOON | flat_100 | 44 | How many records were published in 2024? | `11` | `7` |
| TOON | flat_100 | 44 | How many records are marked as verified (true)? | `49` | `21` |
| TOON | flat_100 | 44 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2403.4527, 2408.6977, 2411.7691, 2411.9702` |
| TOON | flat_100 | 44 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2408.7547, 2402.3504, 2407.1634, 2411.5905, 2411.3851, 2411.3851, 2411.9702` |
| TOON | flat_100 | 44 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2401.3615, 2403.4527, 2408.6977, 2411.5374, 2411.5291, 2411.9797, 2407.2964, 2402.8962, 2402.2753, 2405.4346, 2402.8962, 2410.2343, 2405.4450, 2411.5905, 2407.4006, 2408.2604, 2411.3041, 2411.7691, 2409.3085, 2411.3851, 2409.1006, 2411.3851, 2409.1590, 2411.9702` |
| TOON | flat_100 | 44 | What is the publication year of paper 2403.4527? | `null` | `` |
| TOON | flat_100 | 44 | What is the verdict for paper 2404.7570? | `null` | `` |
| YAML | flat_100 | 44 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v50` |
| YAML | flat_100 | 44 | How many records have verdict refuted? | `26` | `12` |
| YAML | flat_100 | 44 | How many records were published in 2024? | `11` | `4` |
| YAML | flat_100 | 44 | How many records are marked as verified (true)? | `49` | `12` |
| YAML | flat_100 | 44 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2410.7912, 2411.6313, 2411.5905, 2407.1634, 2409.9004, 2411.9702 (These have `verdict: null`)` |
| YAML | flat_100 | 44 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2408.9479, 2411.7691 (These have `year: null`)` |
| YAML | flat_100 | 44 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `13 (The papers with `verified` status set to `false` or `null` are: 2412.9928, 2410.2343, 2407.2584, 2409.5803, 2405.2307, 2401.2889, 2406.7572, 2403.8787, 2407.2964, 2411.7691, 2409.1006, 2402.4848, 2404.7570)` |
| TSV | flat_100 | 44 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v50` |
| TSV | flat_100 | 44 | How many records have verdict refuted? | `26` | `10` |
| TSV | flat_100 | 44 | How many records were published in 2024? | `11` | `7` |
| TSV | flat_100 | 44 | How many records are marked as verified (true)? | `49` | `13` |
| TSV | flat_100 | 44 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `None` |
| TSV | flat_100 | 44 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2401.2889, 2411.7691, 2402.8962, 2402.2753, 2410.2343, 2408.9479, 2412.6138, 2409.9004` |
| TSV | flat_100 | 44 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `11` |
| TSV | flat_100 | 44 | What is the publication year of paper 2403.4527? | `null` | `` |
| TSV | flat_100 | 44 | What is the verdict for paper 2404.7570? | `null` | `` |
| XML | flat_100 | 44 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v40` |
| XML | flat_100 | 44 | How many records have verdict refuted? | `26` | `10` |
| XML | flat_100 | 44 | How many records were published in 2024? | `11` | `5` |
| XML | flat_100 | 44 | How many records are marked as verified (true)? | `49` | `14` |
| XML | flat_100 | 44 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2407.1634, 2407.4006, 2409.9004` |
| XML | flat_100 | 44 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2412.6138, 2411.8062` |
| XML | flat_100 | 44 | What is the publication year of paper 2403.4527? | `null` | `` |
| XML | flat_100 | 44 | What is the verdict for paper 2404.7570? | `null` | `` |
| Plain key: value | flat_100 | 44 | How many records have verdict refuted? | `26` | `9` |
| Plain key: value | flat_100 | 44 | How many records were published in 2024? | `11` | `8` |
| Plain key: value | flat_100 | 44 | How many records are marked as verified (true)? | `49` | `11` |
| Plain key: value | flat_100 | 44 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `13` |
| Plain key: value | flat_100 | 44 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `9` |
| Plain key: value | flat_100 | 44 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `10` |
| Struct | flat_100 | 44 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `KV cache quantization losing accuracy on outliers` |
| Struct | flat_100 | 44 | What is the verification status of paper 2412.9928? | `false` | `supported` |
| Struct | flat_100 | 44 | How many records have verdict refuted? | `26` | `25` |
| Struct | flat_100 | 44 | How many records were published in 2024? | `11` | `6` |
| Struct | flat_100 | 44 | How many records are marked as verified (true)? | `49` | `47` |
| Struct | flat_100 | 44 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `7, 13, 33, 35, 36, 41, 43, 44, 47, 50, 57, 58, 63, 67, 71, 73, 77, 80, 83, 87, 89, 93` |
| Struct | flat_100 | 44 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `None` |
| Struct | flat_100 | 44 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `None` |
| Minified JSON | nested_fixed | 44 | What is the verdict for 'Attention is all you need'? | `supported` | `refuted` |
| Minified JSON | nested_fixed | 44 | Which claim has no verification status? | `StarCoder 2 training code` | `Direct preference optim` |
| TOON | nested_fixed | 44 | How many claims have verdict supported? | `7` | `5` |
| YAML | nested_fixed | 44 | How many claims have verdict supported? | `7` | `6` |
| XML | nested_fixed | 44 | How many claims have verdict supported? | `7` | `6` |
| XML | nested_fixed | 44 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `(No publication year is provided)` |
| Struct | nested_fixed | 44 | How many claims have verdict supported? | `7` | `5` |
| Minified JSON | nested_variable | 44 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `` |
| TOON | nested_variable | 44 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `1` |
| TOON | nested_variable | 44 | How many subnotes are under 'tested on GLUE and Wikisql'? | `2` | `1` |
| TOON | nested_variable | 44 | Which note under 'LoRA matches full finetune' has no subnotes? | `checkpoint size under 50MB` | `LoRA matches full finetune` |
| YAML | nested_variable | 44 | What is the verdict for 'LoRA matches full finetune'? | `supported` | `LoRA matches full finetune` |
| YAML | nested_variable | 44 | What is the source paper ID for 'Chinchilla tokens beat params'? | `2203.15556` | `2106.09685` |
| YAML | nested_variable | 44 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `` |
| YAML | nested_variable | 44 | What subnotes are listed under the note 'table 3 contradicts'? | `llama 7b tested only` | `` |
| YAML | nested_variable | 44 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `` |
| YAML | nested_variable | 44 | How many subnotes are under 'tested on GLUE and Wikisql'? | `2` | `1` |
| YAML | nested_variable | 44 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `` |
| YAML | nested_variable | 44 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `checkpoint size under 50MB` |
| XML | nested_variable | 44 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `checkpoint size under 50MB` |
| Struct | nested_variable | 44 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `2` |
| Struct | nested_variable | 44 | How many total claims have verdict refuted? | `2` | `1` |
| Struct | nested_variable | 44 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `(None are listed)` |
| Struct | nested_variable | 44 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `checkpoint size under 50MB` |
| Minified JSON | flat_tabular | 45 | How many records were published in 2024? | `7` | `5` |
| TOON | flat_tabular | 45 | What is the verification status of paper 2309.06180? | `true` | `supported` |
| TOON | flat_tabular | 45 | How many records have verdict supported? | `16` | `18` |
| TOON | flat_tabular | 45 | How many records were published in 2024? | `7` | `5` |
| TOON | flat_tabular | 45 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2403.09611, 2404.14219` |
| TOON | flat_tabular | 45 | Which paper ID has no verification status? | `2403.09611` | `2203.15556 (or 2408.03314, depending on interpretation. Since 2203.15556 has 'null' for verified, we list it.)` |
| YAML | flat_tabular | 45 | How many records have verdict supported? | `16` | `9` |
| YAML | flat_tabular | 45 | How many records were published in 2024? | `7` | `2` |
| YAML | flat_tabular | 45 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2403.09611, 2404.14219` |
| TSV | flat_tabular | 45 | How many records have verdict supported? | `16` | `17` |
| TSV | flat_tabular | 45 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2404.14219, 2408.03314` |
| TSV | flat_tabular | 45 | What is the publication year of paper 2203.15556? | `null` | `(No publication year is listed for this paper)` |
| TSV | flat_tabular | 45 | Which paper ID has no verdict? | `2408.03314` | `(None)` |
| XML | flat_tabular | 45 | How many records have verdict refuted? | `1` | `15` |
| XML | flat_tabular | 45 | How many records have verdict supported? | `16` | `15` |
| XML | flat_tabular | 45 | How many records were published in 2024? | `7` | `5` |
| XML | flat_tabular | 45 | What is the publication year of paper 2203.15556? | `null` | `(The publication year tag is empty/not specified)` |
| Plain key: value | flat_tabular | 45 | How many records have verdict supported? | `16` | `13` |
| Plain key: value | flat_tabular | 45 | How many records were published in 2024? | `7` | `6` |
| Plain key: value | flat_tabular | 45 | Which paper ID has no verification status? | `2403.09611` | `2408.03314` |
| Struct | flat_tabular | 45 | How many records have verdict supported? | `16` | `14` |
| Struct | flat_tabular | 45 | How many records were published in 2024? | `7` | `6` |
| Struct | flat_tabular | 45 | Which paper ID has no verification status? | `2403.09611` | `None` |
| Struct | flat_tabular | 45 | What is the publication year of paper 2203.15556? | `null` | `` |
| Struct | flat_tabular | 45 | Which paper ID has no verdict? | `2408.03314` | `` |
| Minified JSON | flat_100 | 45 | How many records have verdict refuted? | `26` | `7` |
| Minified JSON | flat_100 | 45 | How many records were published in 2024? | `11` | `5` |
| Minified JSON | flat_100 | 45 | How many records are marked as verified (true)? | `49` | `16` |
| Minified JSON | flat_100 | 45 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2410.7912, 2411.6313, 2411.7691, 2407.1634, 2408.9479, 2410.2343, 2402.5808` |
| Minified JSON | flat_100 | 45 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2410.7912, 2411.6313, 2411.7691, 2407.1634, 2408.9479, 2410.2343, 2402.5808` |
| Minified JSON | flat_100 | 45 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2411.5291, 2408.7790, 2411.3041, 2402.2753, 2405.4450, 2405.7484, 2411.9702` |
| Minified JSON | flat_100 | 45 | What is the verdict for paper 2404.7570? | `null` | `` |
| TOON | flat_100 | 45 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v50` |
| TOON | flat_100 | 45 | How many records have verdict refuted? | `26` | `14` |
| TOON | flat_100 | 45 | How many records were published in 2024? | `11` | `4` |
| TOON | flat_100 | 45 | How many records are marked as verified (true)? | `49` | `13` |
| TOON | flat_100 | 45 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `5` |
| TOON | flat_100 | 45 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `4` |
| TOON | flat_100 | 45 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `6` |
| YAML | flat_100 | 45 | How many records have verdict refuted? | `26` | `10` |
| YAML | flat_100 | 45 | How many records were published in 2024? | `11` | `4` |
| YAML | flat_100 | 45 | How many records are marked as verified (true)? | `49` | `12` |
| YAML | flat_100 | 45 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `17` |
| YAML | flat_100 | 45 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `7` |
| YAML | flat_100 | 45 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `4` |
| TSV | flat_100 | 45 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v50` |
| TSV | flat_100 | 45 | How many records have verdict refuted? | `26` | `12` |
| TSV | flat_100 | 45 | How many records were published in 2024? | `11` | `2` |
| TSV | flat_100 | 45 | How many records are marked as verified (true)? | `49` | `15` |
| TSV | flat_100 | 45 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2403.4527, 2410.7912, 2408.7790, 2402.3504, 2408.7547, 2407.4006, 2410.2343, 2409.3085, 2405.9445` |
| TSV | flat_100 | 45 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2402.5808, 2402.2753, 2412.6138, 2411.7691, 2410.6279` |
| TSV | flat_100 | 45 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2403.4527, 2410.7912, 2408.7790, 2402.3504, 2405.9445` |
| TSV | flat_100 | 45 | What is the publication year of paper 2403.4527? | `null` | `` |
| TSV | flat_100 | 45 | What is the verdict for paper 2404.7570? | `null` | `` |
| XML | flat_100 | 45 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v32` |
| XML | flat_100 | 45 | How many records have verdict refuted? | `26` | `11` |
| XML | flat_100 | 45 | How many records were published in 2024? | `11` | `5` |
| XML | flat_100 | 45 | How many records are marked as verified (true)? | `49` | `13` |
| XML | flat_100 | 45 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2404.7570` |
| XML | flat_100 | 45 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2410.2343, 2407.4006` |
| XML | flat_100 | 45 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2408.6977, 2412.6138, 2402.3504` |
| XML | flat_100 | 45 | What is the publication year of paper 2403.4527? | `null` | `(No year / Empty year)` |
| XML | flat_100 | 45 | What is the verdict for paper 2404.7570? | `null` | `(No verdict / Empty verdict)` |
| Plain key: value | flat_100 | 45 | How many records have verdict refuted? | `26` | `11` |
| Plain key: value | flat_100 | 45 | How many records were published in 2024? | `11` | `6` |
| Plain key: value | flat_100 | 45 | How many records are marked as verified (true)? | `49` | `22` |
| Plain key: value | flat_100 | 45 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2411.2824, 2411.5374, 2411.5291, 2409.5803, 2404.9179, 2405.3287, 2408.7216, 2411.3851, 2402.8962, 2408.2604, 2406.7572, 2406.9346, 2405.8953, 2412.6138` |
| Plain key: value | flat_100 | 45 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2408.6977, 2411.6313, 2407.4006, 2407.1634, 2409.9004` |
| Plain key: value | flat_100 | 45 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2404.7570, 2403.8179` |
| Struct | flat_100 | 45 | What is the verdict for paper 2411.2824? | `unverified` | `supported` |
| Struct | flat_100 | 45 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v40` |
| Struct | flat_100 | 45 | How many records have verdict refuted? | `26` | `15` |
| Struct | flat_100 | 45 | How many records were published in 2024? | `11` | `3` |
| Struct | flat_100 | 45 | How many records are marked as verified (true)? | `49` | `13` |
| Struct | flat_100 | 45 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `0` |
| Struct | flat_100 | 45 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `1` |
| Struct | flat_100 | 45 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `0` |
| Struct | flat_100 | 45 | What is the publication year of paper 2403.4527? | `null` | `2021` |
| Struct | flat_100 | 45 | What is the verdict for paper 2404.7570? | `null` | `unverified` |
| Minified JSON | nested_fixed | 45 | What is the verdict for 'Attention is all you need'? | `supported` | `refuted` |
| Minified JSON | nested_fixed | 45 | How many claims have verdict supported? | `7` | `5` |
| XML | nested_fixed | 45 | How many claims have verdict supported? | `7` | `6` |
| XML | nested_fixed | 45 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `(The publication year is missing/not provided)` |
| Struct | nested_fixed | 45 | What is the verdict for 'Attention is all you need'? | `supported` | `refuted` |
| Struct | nested_fixed | 45 | How many claims have verdict supported? | `7` | `5` |
| Struct | nested_fixed | 45 | Which claim has no verification status? | `StarCoder 2 training code` | `Rope scaling hurts long ctx` |
| Minified JSON | nested_variable | 45 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `` |
| Minified JSON | nested_variable | 45 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `checkpoint size under 50MB` |
| TOON | nested_variable | 45 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `subnotes[2]: abstract mentions 32k limit,appendix B clarifies` |
| TOON | nested_variable | 45 | What subnotes are listed under the note 'table 3 contradicts'? | `llama 7b tested only` | `subnotes[1]: llama 7b tested only` |
| TOON | nested_variable | 45 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `subnotes[2]: per-head scales help,perplexity intact` |
| TOON | nested_variable | 45 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `1` |
| TOON | nested_variable | 45 | How many subnotes are under 'tested on GLUE and Wikisql'? | `2` | `1` |
| TOON | nested_variable | 45 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `checkpoint size under 50MB` |
| XML | nested_variable | 45 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `none (or empty)` |
| Struct | nested_variable | 45 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `>>abstract mentions 32k limit` |
| Struct | nested_variable | 45 | What subnotes are listed under the note 'table 3 contradicts'? | `llama 7b tested only` | `>>llama 7b tested only` |
| Struct | nested_variable | 45 | What subnotes are listed under the note 'outlier channels preserved'? | `per-head scales help, perplexity intact` | `>>per-head scales help, >>perplexity intact` |
| Struct | nested_variable | 45 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `0` |
| Struct | nested_variable | 45 | Which note under 'LoRA matches full finetune' has no subnotes? | `checkpoint size under 50MB` | `LoRA matches full finetune` |
| Struct | nested_variable | 45 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `>>optimizer state minimal` |
| Minified JSON | flat_tabular | 46 | How many records have verdict supported? | `16` | `13` |
| Minified JSON | flat_tabular | 46 | How many records were published in 2024? | `7` | `6` |
| TOON | flat_tabular | 46 | How many records were published in 2024? | `7` | `6` |
| TOON | flat_tabular | 46 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2403.09611, 2404.14219, 2408.03314` |
| TOON | flat_tabular | 46 | Which paper ID has no verification status? | `2403.09611` | `2203.15556, 2403.09611, 2404.14219, 2408.03314` |
| TOON | flat_tabular | 46 | Which paper ID has no verdict? | `2408.03314` | `None` |
| YAML | flat_tabular | 46 | How many records have verdict supported? | `16` | `11` |
| YAML | flat_tabular | 46 | How many records were published in 2024? | `7` | `4` |
| YAML | flat_tabular | 46 | Which paper IDs have no publication year? | `2203.15556, 2404.14219` | `2203.15556, 2404.14219, 2403.09611` |
| YAML | flat_tabular | 46 | Which paper ID has no verdict? | `2408.03314` | `2408.03314, 2406.11794` |
| TSV | flat_tabular | 46 | How many records have verdict supported? | `16` | `18` |
| TSV | flat_tabular | 46 | How many records were published in 2024? | `7` | `6` |
| TSV | flat_tabular | 46 | What is the publication year of paper 2203.15556? | `null` | `(No publication year is provided/empty)` |
| XML | flat_tabular | 46 | How many records have verdict supported? | `16` | `8` |
| XML | flat_tabular | 46 | How many records were published in 2024? | `7` | `4` |
| XML | flat_tabular | 46 | What is the publication year of paper 2203.15556? | `null` | `(empty)` |
| Plain key: value | flat_tabular | 46 | How many records have verdict supported? | `16` | `15` |
| Plain key: value | flat_tabular | 46 | How many records were published in 2024? | `7` | `5` |
| Struct | flat_tabular | 46 | How many records have verdict supported? | `16` | `14` |
| Struct | flat_tabular | 46 | How many records were published in 2024? | `7` | `4` |
| Struct | flat_tabular | 46 | Which paper ID has no verification status? | `2403.09611` | `None` |
| Struct | flat_tabular | 46 | What is the publication year of paper 2203.15556? | `null` | `` |
| Struct | flat_tabular | 46 | Which paper ID has no verdict? | `2408.03314` | `` |
| Minified JSON | flat_100 | 46 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v50` |
| Minified JSON | flat_100 | 46 | How many records have verdict refuted? | `26` | `10` |
| Minified JSON | flat_100 | 46 | How many records were published in 2024? | `11` | `5` |
| Minified JSON | flat_100 | 46 | How many records are marked as verified (true)? | `49` | `13` |
| Minified JSON | flat_100 | 46 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2410.7912, 2408.6977, 2408.7547, 2411.7691, 2402.2753, 2407.1634, 2404.7570, 2412.6138, 2407.2343, 2405.4346, 2410.2638, 2403.8179, 2411.5905, 2411.9702` |
| Minified JSON | flat_100 | 46 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2407.4006, 2408.9479, 2411.3041, 2411.9702` |
| Minified JSON | flat_100 | 46 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2411.9702` |
| TOON | flat_100 | 46 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v50` |
| TOON | flat_100 | 46 | How many records have verdict refuted? | `26` | `12` |
| TOON | flat_100 | 46 | How many records were published in 2024? | `11` | `5` |
| TOON | flat_100 | 46 | How many records are marked as verified (true)? | `49` | `11` |
| TOON | flat_100 | 46 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2404.7570, 2402.8962, 2408.9479, 2411.9702` |
| TOON | flat_100 | 46 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2410.2343, 2408.6977, 2411.6313, 2402.5808, 2408.9479, 2411.9702` |
| TOON | flat_100 | 46 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2411.9702` |
| YAML | flat_100 | 46 | How many records have verdict refuted? | `26` | `12` |
| YAML | flat_100 | 46 | How many records were published in 2024? | `11` | `4` |
| YAML | flat_100 | 46 | How many records are marked as verified (true)? | `49` | `14` |
| YAML | flat_100 | 46 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2408.9479, 2411.7691, 2407.1634` |
| YAML | flat_100 | 46 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2410.7912, 2408.6977, 2411.3851` |
| TSV | flat_100 | 46 | How many records have verdict refuted? | `26` | `11` |
| TSV | flat_100 | 46 | How many records were published in 2024? | `11` | `5` |
| TSV | flat_100 | 46 | How many records are marked as verified (true)? | `49` | `16` |
| TSV | flat_100 | 46 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2411.9702` |
| TSV | flat_100 | 46 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `4` |
| TSV | flat_100 | 46 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2` |
| TSV | flat_100 | 46 | What is the publication year of paper 2403.4527? | `null` | `` |
| TSV | flat_100 | 46 | What is the verdict for paper 2404.7570? | `null` | `` |
| XML | flat_100 | 46 | How many records have verdict refuted? | `26` | `11` |
| XML | flat_100 | 46 | How many records were published in 2024? | `11` | `4` |
| XML | flat_100 | 46 | How many records are marked as verified (true)? | `49` | `12` |
| XML | flat_100 | 46 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2411.9702` |
| XML | flat_100 | 46 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2411.6313, 2407.1634, 2411.7691` |
| XML | flat_100 | 46 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2408.6977, 2412.6138, 2403.8179` |
| XML | flat_100 | 46 | What is the publication year of paper 2403.4527? | `null` | `` |
| XML | flat_100 | 46 | What is the verdict for paper 2404.7570? | `null` | `` |
| Plain key: value | flat_100 | 46 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v50` |
| Plain key: value | flat_100 | 46 | How many records have verdict refuted? | `26` | `13` |
| Plain key: value | flat_100 | 46 | How many records were published in 2024? | `11` | `5` |
| Plain key: value | flat_100 | 46 | How many records are marked as verified (true)? | `49` | `12` |
| Plain key: value | flat_100 | 46 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `2411.2824, 2408.6977, 2411.7691, 2404.9835, 2412.6107, 2408.9479, 2402.6539, 2407.1634` |
| Plain key: value | flat_100 | 46 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `2403.4527, 2408.9479, 2407.1634` |
| Plain key: value | flat_100 | 46 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `2411.2824, 2410.3803, 2408.7216, 2411.3851, 2411.3041, 2411.7691, 2411.3851, 2411.5374, 2403.8179, 2409.9701, 2405.4346, 2408.7790, 2407.2584, 2402.2753, 2409.1006, 2410.2343, 2405.9445` |
| Plain key: value | flat_100 | 46 | What is the publication year of paper 2403.4527? | `null` | `2022` |
| Plain key: value | flat_100 | 46 | What is the verdict for paper 2404.7570? | `null` | `supported` |
| Struct | flat_100 | 46 | What is the title of paper 2404.9279? | `KV cache quantization losing accuracy on outliers v4` | `LoRA low rank adaptation of large language models v50` |
| Struct | flat_100 | 46 | What is the verification status of paper 2412.9928? | `false` | `false (or 0)` |
| Struct | flat_100 | 46 | How many records have verdict refuted? | `26` | `8` |
| Struct | flat_100 | 46 | How many records were published in 2024? | `11` | `3` |
| Struct | flat_100 | 46 | How many records are marked as verified (true)? | `49` | `22` |
| Struct | flat_100 | 46 | Which paper IDs have no verdict? | `2404.7570, 2411.9702` | `6` |
| Struct | flat_100 | 46 | Which paper IDs have no publication year? | `2403.4527, 2411.6313, 2402.5808, 2408.9479, 2407.4006, 2410.2343, 2409.9004, 2407.1634, 2411.7691, 2408.7547` | `3` |
| Struct | flat_100 | 46 | Which paper IDs have no verification status? | `2410.7912, 2408.6977, 2402.3504, 2402.8962, 2411.8062, 2412.6138, 2403.8179, 2406.4441, 2407.9379, 2405.9445` | `11` |
| Minified JSON | nested_fixed | 46 | What is the verdict for 'Attention is all you need'? | `supported` | `refuted` |
| Minified JSON | nested_fixed | 46 | How many claims have verdict supported? | `7` | `5` |
| Minified JSON | nested_fixed | 46 | Which claim has no source year? | `DeepSeekMoE mixture experts` | `Direct preference optim` |
| TOON | nested_fixed | 46 | What is the verification status for 'Mistral fine tuning rules'? | `false` | `unverified` |
| TOON | nested_fixed | 46 | How many claims have verdict supported? | `7` | `8` |
| TOON | nested_fixed | 46 | Which claim has no source year? | `DeepSeekMoE mixture experts` | `Direct preference optim` |
| XML | nested_fixed | 46 | What is the verdict for 'Attention is all you need'? | `supported` | `refuted` |
| XML | nested_fixed | 46 | How many claims have verdict supported? | `7` | `6` |
| XML | nested_fixed | 46 | What is the publication year of the source for 'DeepSeekMoE mixture experts'? | `null` | `(No publication year is listed for this claim)` |
| Struct | nested_fixed | 46 | How many claims have verdict supported? | `7` | `5` |
| Minified JSON | nested_variable | 46 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `(None)` |
| TOON | nested_variable | 46 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `1` |
| TOON | nested_variable | 46 | How many subnotes are under 'tested on GLUE and Wikisql'? | `2` | `1` |
| TOON | nested_variable | 46 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `checkpoint size under 50MB` |
| YAML | nested_variable | 46 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `1` |
| YAML | nested_variable | 46 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `checkpoint size under 50MB` |
| XML | nested_variable | 46 | What subnotes are listed under the note 'checkpoint size under 50MB'? | `none` | `None (or empty)` |
| Struct | nested_variable | 46 | What subnotes are listed under the note 'notes are from abstract only'? | `abstract mentions 32k limit, appendix B clarifies` | `abstract mentions 32k limit` |
| Struct | nested_variable | 46 | How many notes are attached to 'LoRA matches full finetune'? | `3` | `0` |
| Struct | nested_variable | 46 | How many total claims have verdict refuted? | `2` | `1` |
| Struct | nested_variable | 46 | How many subnotes are under 'tested on GLUE and Wikisql'? | `2` | `0` |
| Struct | nested_variable | 46 | Which note under 'LoRA matches full finetune' has no subnotes? | `checkpoint size under 50MB` | `LoRA matches full finetune` |
| Struct | nested_variable | 46 | Which claim text contains a note with no subnotes? | `LoRA matches full finetune` | `LoRA matches full finetune (or checkpoint size under 50MB, or tested on GLUE and Wikisql)` |
