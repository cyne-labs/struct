# R4-gemini-lite: the same retrieval test on Gemini 3.5 Flash Lite, through the API

Read `milestones/RULES.md`, `milestones/R4.md`, `milestones/R4-haiku.md`, `bench/r4/collect_haiku.py`, `bench/r4/evaluate_haiku.py` and `bench/r4/test_haiku_offline.py` first.

This adds a fourth model, Gemini 3.5 Flash Lite, called directly through the Gemini API. Prompts, questions, formats, datasets and scoring are exactly R4's. Mirror the Haiku harness as closely as you can.

**Don't touch** `bench/r4/real*/`, `grades/`, `questions.json`, `data/`, `formats/`, or any existing report. Don't change how the existing harness files behave; import from them. **Don't call the API or any model.** Test with mocked HTTP. The maintainer's session runs the real thing.

## The API key

- Read it only from the environment variable `GEMINI_API_KEY`. If it's missing, stop with a clear message.
- Send it only in the `x-goog-api-key` header, never in the URL.
- **Never print, log or save it**, including in error messages, metadata, `run_info.json` or raw replies. If an error reply or exception text could contain it, scrub it before recording. Test that the key never appears in any file the harness writes.

## Decisions (maintainer, 2026-11-21)

1. **Call:** `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`, standard library only (`urllib`). Body: the prompt as a single user turn, `generationConfig.thinkingConfig.thinkingLevel = "low"`, and `includeThoughts` left off. No system instruction, no tools, no other sampling settings (model defaults, like Haiku). The model ID is a `--model` argument (no default you invent); the maintainer fills it in after listing the models the key can reach.
2. **Repeats:** 4, seeds 1 to 4 (plain reruns). 24 prompts × 4 = 96 calls.
3. **Order: seed by seed.** Run all 24 prompts of seed 1 before any call of seed 2, then all of seed 2, and so on. Up to 4 calls at a time, but only within the current seed: never start a call of the next seed until every call of the current one has succeeded or failed for good. Reason: it's a free key and may run out; every finished seed must be a complete set across all 7 formats.
4. **Rate limits and quota:**
   - HTTP 429 or 503 with a per-minute limit: wait (use `Retry-After` or the retry delay in the error body if given, otherwise back off: 10 s, 20 s, 40 s, up to 5 tries) and retry the same call.
   - A daily or project quota exhausted (the error says so, or 429s keep coming after the backoff): **stop the whole run cleanly**, record which seed and call it stopped at, and exit. A later run resumes from there.
   - Any other failure (other HTTP errors, a blocked prompt, an empty answer, no candidates): recorded, never marked done, retried on resume, as in R4 decision 13.
5. **Partial runs:** the evaluator scores only complete seeds (all 24 calls succeeded). It lists any incomplete seed separately, with how many of its calls finished, and doesn't mix it into the accuracy tables.

## Recording

- `bench/r4/real_gemini_lite/`: `raw/gemini_lite_r4_{format}_{dataset}_seed{n}.txt` (the answer text: every text part of the first candidate, joined, with thought parts left out), `metadata.json`, `SHA256SUMS`, `run_info.json` (model ID, the request body without the prompt, thinking level, run order, start and stop times; never the key).
- Tokens from `usageMetadata`: input `promptTokenCount`; thinking `thoughtsTokenCount` (0 if absent); output `candidatesTokenCount`. Keep the whole `usageMetadata` and `modelVersion` in metadata as returned.
- Same metadata shape as the Haiku harness, so `evaluate.evaluate_runs` reads it, plus `finishReason`.

## Build

- `bench/r4/collect_gemini_lite.py` with `--model` and `--dry-run` (checks all 96 prompts are byte-identical to `collect.py`'s, and prints the planned order, without calling anything). Resumable. 20-minute timeout per call.
- `bench/r4/evaluate_gemini_lite.py`, writing `research/r4-retrieval-gemini-lite.md` in the Haiku report's layout: setup (model ID, the API, thinking low, 4 repeats, seed-by-seed order, how many seeds completed), budget column n/a, input and thinking tokens as reported by the API.
- `bench/r4/test_gemini_lite_offline.py`: mocked success; 429 with retry then success; daily quota stops the run and resume continues from the same seed; seed order (no call of seed 2 before seed 1 is done); other errors recorded and retried; key never written anywhere and never in the URL; missing key stops cleanly; partial seed left out of the accuracy tables. Run it with the other three test files: all must pass.

## Final message

What you built, test counts, anything you couldn't do, and a plain-English summary (RULES rule 10). Don't commit or push.
