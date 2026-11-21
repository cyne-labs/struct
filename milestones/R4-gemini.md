# R4-gemini: the same retrieval test on Gemini 3.8 Flash

Read `milestones/RULES.md`, `milestones/R4.md`, `milestones/R4-haiku.md`, `bench/r4/collect_haiku.py`, `bench/r4/evaluate_haiku.py` and `bench/r4/test_haiku_offline.py` first.

This adds a third model, Gemini 3.8 Flash, the same way R4-haiku added Haiku. Prompts, questions, formats, datasets and scoring are exactly R4's. Only the model and how it is called change. Mirror the Haiku harness as closely as you can.

**Don't touch** `bench/r4/real/`, `real_haiku/`, `real_run2/`, `grades/`, `questions.json`, `data/`, `formats/`, or any existing report. Don't change how the existing harness files behave; import from them. **Don't call any model**, including agy itself. Test with a mocked subprocess. The maintainer's session runs the real thing.

## Decisions (maintainer, 2026-11-21)

1. **How Gemini is called:** one fresh headless agy process per prompt, from a new empty temporary directory each time (so the agent has no files to look at), with the prompt attached to `-p`:
   ```
   agy -p=<prompt> --model gemini-3.8-flash-low --output-format json --sandbox --disable-slash-commands
   ```
   - Pass arguments as a list (no shell). The prompt must be one argument, `"-p=" + prompt`. agy doesn't read stdin, and a separate `-p <prompt>` breaks when the prompt starts with `-`.
   - The maintainer checked on 2026-11-21 that a prompt with newlines, tabs, quotes, backslashes, pipes and code fences arrives byte-identical this way. The largest R4 prompt is 20,743 characters, under the Windows 32,767-character limit. Check every prompt's length before calling, and fail that call (recorded, not silently skipped) if the command line would exceed the limit.
2. **Thinking:** the `low` model variant. No thinking budget, so the budget column shows n/a.
3. **Repeats:** 4 per prompt, seeds 1 to 4 (plain reruns, as for Haiku). 24 prompts × 4 = **96 calls**. 4 calls at a time.

## agy's JSON reply

A successful reply looks like this (real example):
```json
{"conversation_id":"...","status":"SUCCESS","response":"391\n","duration_seconds":3.45,"num_turns":1,
 "usage":{"input_tokens":13594,"output_tokens":47,"thinking_tokens":44,"cache_read_tokens":0,"total_tokens":13641}}
```
A failed one has `"status":"ERROR"`, an `"error"` field and `"finished_reason":"error"`.
- answer text: `response`;
- input tokens: `input_tokens + cache_read_tokens`;
- thinking tokens: `thinking_tokens`;
- output tokens: `output_tokens` minus `thinking_tokens`, never below 0. If the numbers suggest `output_tokens` already excludes thinking, don't guess: record both raw fields in metadata and say so in your final message;
- failure: non-zero exit, unparseable JSON, `status` other than `SUCCESS`, `num_turns` above 1 (the agent used tools), or an empty `response`. Failures are recorded, never marked done, and a rerun retries them.

## Build

- `bench/r4/collect_gemini.py`, writing to `bench/r4/real_gemini/` (`raw/gemini_r4_{format}_{dataset}_seed{n}.txt`, `metadata.json`, `SHA256SUMS`, `run_info.json` with the exact command and `agy --version` if it has one). Same shape and behaviour as `collect_haiku.py`: byte-identical prompts, a `--dry-run` that checks them against `collect.py`, resumable, 20-minute timeout per call.
- `bench/r4/evaluate_gemini.py`, writing `research/r4-retrieval-gemini.md` in the same layout as the Haiku report. Setup section: the model, the exact command, low variant, no budget, 4 repeats, 4 parallel, and this note: "agy is an agent: every call includes its own system prompt and tool definitions, about 13,600 input tokens, so input tokens here are not comparable with the other models' in absolute terms; compare formats within this model only." Budget column n/a.
- `bench/r4/test_gemini_offline.py`: mocked success, error status, empty response, `num_turns` above 1, timeout, a too-long command line; token arithmetic; resume; prompts byte-identical to Gemma's. Run it with `test_offline.py` and `test_haiku_offline.py`: all must pass.

## Final message

What you built, test counts, anything you couldn't do, and a plain-English summary (RULES rule 10). Don't commit or push.
