# R4-haiku: the same retrieval test on Claude Haiku

Read `milestones/RULES.md`, `milestones/R4.md` (all decisions, including 14), `bench/r4/collect.py`, `bench/r4/evaluate.py` and `bench/r4/test_offline.py` first.

R4 measures whether a model can read each format. This adds a second model, Claude Haiku. The prompts, questions, formats, datasets and scoring are exactly R4's. Only the model and how it is called change.

**Don't touch** `bench/r4/real/`, `research/r4-retrieval.md`, `questions.json`, `data/` or `formats/`. A Gemma run is writing to `bench/r4/real/` right now. Don't change how `collect.py` or `evaluate.py` behave for Gemma: import from them, don't edit them, unless a change is needed and is covered by the existing 40 tests still passing. **Don't call Claude or any model.** Test with a mocked subprocess. The maintainer's session runs the real thing.

## Decisions (maintainer, 2026-11-21)

1. **How Haiku is called:** one fresh headless Claude Code process per prompt, from a neutral working directory (the system temp folder), with the prompt on stdin:
   ```
   claude -p --model haiku --effort low --system-prompt "You are a helpful assistant." --tools "" --strict-mcp-config --setting-sources "" --disable-slash-commands --no-session-persistence --output-format json
   ```
   Pass arguments as a list (no shell). The empty strings for `--tools` and `--setting-sources` are real empty arguments.
2. **Thinking:** effort `low`, no thinking budget. The report says there is no budget, and the "budget hit" column shows n/a for Haiku, not 0.
3. **Repeats:** 4 per prompt, labelled seed 1 to 4 (Claude takes no seed; they are plain reruns). TSV and `key: value` still run only on the flat datasets, so 24 prompts × 4 = **96 calls**.
4. **Parallel:** 4 calls at a time.

## Build

- `bench/r4/collect_haiku.py`:
  - builds every prompt with `collect.build_prompt`, `ENCODERS` and `questions.json`, so each prompt is byte-identical to Gemma's;
  - writes to `bench/r4/real_haiku/`: `raw/haiku_r4_{format}_{dataset}_seed{n}.txt` (the answer text, the JSON `result` field), `metadata.json`, `SHA256SUMS` (all raw files plus metadata);
  - one metadata entry per call, in the same shape `evaluate.evaluate_runs` reads (`format`, `dataset`, `seed`, `file`, `success`, `finish_reason`, `elapsed_sec`, `input_tokens`, `thinking_tokens`, `output_tokens`, `error` when failed), plus the full JSON reply as `raw_response`, the model id from `modelUsage`, and `total_cost_usd`;
  - tokens come from the JSON `usage`: input is `input_tokens + cache_creation_input_tokens + cache_read_input_tokens`; thinking is `output_tokens_details.thinking_tokens`; output is `output_tokens` minus thinking;
  - a failure is a non-zero exit, unparseable JSON, `is_error` true, or an empty `result`. Failures are recorded, never marked done, and a rerun retries them (same rule as R4 decision 13);
  - a 20-minute timeout per call;
  - resumable: a rerun skips calls that already succeeded;
  - `--dry-run` builds all 96 prompts and checks they match what `collect.py` would send for the same format and dataset, without calling anything;
  - also records, once, the exact command line used and the `claude --version` output, into `real_haiku/run_info.json` (hashed in SHA256SUMS).
- `bench/r4/evaluate_haiku.py`: scores with `evaluate.evaluate_runs` (same scoring, unchanged) and writes `research/r4-retrieval-haiku.md` in the same layout as the Gemma report, with:
  - a setup section stating the model id actually returned, the exact command, effort low, no budget, 4 repeats, 4 parallel;
  - input tokens labelled "as counted by the Anthropic API, including about 640 fixed tokens from the system prompt and template on every call" (measure the fixed part from the data if you can: say how, or leave the label as is);
  - the total cost at list price (sum of `total_cost_usd`), labelled as list price;
  - budget column n/a.
  Reuse `evaluate.generate_report` if you can do it without changing its output for Gemma; otherwise write the Haiku report function separately.
- Offline tests in a new `bench/r4/test_haiku_offline.py`: mocked subprocess for success, failure, empty result and timeout; token arithmetic; resume skips done calls and retries failed ones; prompts byte-identical to Gemma's; report has the n/a budget column and the cost line. Run them and the existing `test_offline.py` (40 tests must still pass).

## Final message

What you built, the test results with counts, anything you could not do, and a plain-English summary (RULES rule 10). Don't commit or push.
