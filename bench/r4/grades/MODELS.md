# Which model each grade file is for

Some grade files use neutral names, so the graders couldn't tell which model they were grading.

| Files | Model | Run folder |
|---|---|---|
| `gemma_*.json` | Gemma 4 E2B, thinking on (run 1) | `real/` |
| `haiku_*.json` | Claude Haiku 5.5, effort low | `real_haiku/` |
| `gemini_*.json` | Gemini 3.8 Flash, low variant, through the Antigravity CLI | `real_gemini/` |
| `m4_*.json`, `m4_raw/` | Gemini 3.5 Flash Lite, thinking low | `real_gemini_lite/` |
| `m5_*.json`, `m5_raw/` | Gemini 3.5 Flash Lite, thinking high | `real_gemini_lite_high/` |

Cross-checks: `crosscheck/` (Gemma and Haiku), `crosscheck_b/` (Gemini 3.8 Flash), `crosscheck_jb007/` (m4), `crosscheck_m5/` (m5). Each has `sample.json` (what the checker saw), `regrade.json` (its grades) and `key.json` (the shard grades it was compared with, kept out of the repo while the check ran). `m4_mapping.json` and `m5_mapping.json` map the neutral answer files back to the original raw files.
