# R4 build review 2 (independent re-check)

Scope: uncommitted `bench/r4/`. Read-only except this file. No server, no model. Scratch scripts in the session scratchpad (fake HTTP server, mutation and scoring probes, a TOON quote checker, an independent TOON row decoder).

## Verdicts

| # | Item | Result |
|---|------|--------|
| 1a | Blank or missing answer wrong on null question | PASS |
| 1b | Failed requests not done, rerun on resume | PASS (history of the failure is erased once the rerun works) |
| 1c | Budget exhaustion from thinking tokens | PASS (approximate, see below) |
| 1d | metadata.json in SHA256SUMS | PASS |
| 1e | Launch line uses `--port 8090` | PASS (but it is a constant, not captured from the server) |
| 1f | Report: per-kind table | PASS |
| 1g | Report: full prompt text | FAIL (hardcoded, invented sample) |
| 1h | questions.json never rewritten by tests | PASS |
| 2 | TOON per spec | PARTIAL: nested header valid, quoting valid, empty list NOT per spec |
| 3 | Scoring forms and bool equivalence | PASS with two small leaks |
| 4 | XML empty tag, no-loss still strict | PASS |
| 5 | Test counts | PASS (92, not 87) |
| 6 | Ground truths | PASS (48/48) |

## 1. Decisions 9-13

- **Blank answer (D9).** Own probe: for truth `null` and truth `none`, answers `""`, `"   "`, `None` all score wrong. `null`, `None`, `~`, `N/A`, `**null**`, `null.`, `_null_` score right. `unknown`, `0`, `false` score wrong.
- **Failed requests (D13).** Fake HTTP server, one seed, 24 requests. Forced: HTTP 500, empty content with reasoning, a timeout (TIMEOUT_SEC=1, server sleeps 3 s), whitespace-only content. All four recorded `success: false`. Resume reran exactly those 4 (4 requests), a third run made 0. Still-failed runs are listed in the report. Gap (low): when a rerun succeeds, the failure record is deleted from metadata.json, so "every failure is recorded" holds only for failures that stay failed.
- **Budget.** Fake reasoning of 8192 tokens counted as exhausted (xml: 1), 10-token runs not. Count uses re-tokenizing the reasoning text via `/tokenize`, not server usage, so a real run that hit 8192 may re-tokenize to a few under and be missed. Low. `finish_reason == "length"` also counts.
- **SHA256SUMS.** Has `metadata.json`, hash verified to equal sha256 of the file. 25 lines for 24 runs.
- **Launch line.** `--port 8090` in collect.py and evaluate.py (and in the report). The flags match the maintainer's R3 flags plus `--reasoning-budget 8192`. The collector never starts the server and does not record `/props`, so "the exact line used" is only what the builder typed. Low.
- **questions.json.** sha256 before and after `python bench/r4/test_offline.py` identical (98299b24...). Data files and bench/r1/fixtures also unchanged. Tests: 27, OK. Dry run: 120 prompts.
- **Report, full prompt (FAIL, medium).** `generate_report` prints a hand-typed sample, not a built prompt. It has `...` in place of rows, quoted titles and spaced pipes that the real Struct text does not have (real rows look like `2405.05254|Rope scaling hurts long ctx|refuted|2024|0|`), and question 11 differs from the real one (real: paper 2203.15556). Decision 13 asks for the full prompt text. It should be generated with `build_prompt` from the real data and questions.
- **Report TOON note is wrong (medium).** It says R1's fixtures had "`subnotes: []`" as if that were a flaw, and says nothing about the nested header. Under the spec (below), `subnotes: []` was the correct form and the R1 nested header is valid. The note must be fixed once the encoder is.

## 2. TOON

Spec fetched from https://github.com/toon-format/spec (SPEC.md, Version 4.4, 2026-10-08, Working Draft) and its CHANGELOG.

- **Nested header: the builder is right, the first review was wrong.** Spec 9.3: "a nested-uniform column is emitted as a nested field group `fieldname{…}`, its subfields in the first object's sub-object encounter order". Header form: `key[N<delim?>]{f1<delim>f2<delim>…}:`; "Root tabular arrays omit the key: `[N<delim?>]{…}:`". Nested groups arrived in 4.0 (2026-07-22). `[10]{text,verdict,source{id,year},verified}:` is valid, rows have 5 cells. `source` is always a non-empty object with the same keys, so nested-uniform holds. My own csv-style decode of all three tabular TOON texts equals the source with strict types.
- **Numeric-looking strings: PASS.** Spec 7.2 rule: `/^[+-]?[0-9]+(?:\.[0-9]+)?(?:e[+-]?[0-9]+)?$/i`. I coded all of 7.2 and compared against the encoder on all 298 distinct strings in the 4 datasets: 0 mismatches. IDs are quoted (`"2405.05254"`). The encoder is not general (it does not quote leading/trailing space, empty string, brackets, backslash, `+1`, `1e5`), but no such strings exist in this data.
- **Empty list: FAIL (medium).** Spec 9.1: "Empty arrays (object field position): encoders MUST emit `key: []`. The legacy header form `key[0<delim?>]:` MUST NOT be emitted." Decoders must accept the legacy form. Since 3.1 `key: []` was allowed; since 4.1 (2026-07-26) it is required. The encoder writes `subnotes[0]:`, the pre-3.1 form. So it uses 4.x nested headers and a 3.0 empty list: inconsistent. The R1 fixture's `subnotes: []` was right. Only one empty list exists in the data (LoRA claim, `checkpoint size under 50MB`). The maintainer's rule "empty lists written the way the spec says" points to `subnotes: []`. The module docstring in toon.py and the report both state the wrong form.
- Printed TOON for all four datasets was reviewed (flat_tabular, flat_100, nested_fixed, nested_variable). nested_variable uses list form with `notes[2]:` and inline `subnotes[2]: a,b` and one `subnotes[0]:`. All of it is valid except that one line.

## 3. Scoring

- **`[0]` accepted for empty list.** It is TOON's legacy spelling, so the only encoding that shows `[0]` is TOON. Plainly: yes, accepting it leans toward TOON, but only by a tiny amount: `[]`, `empty`, `none` and the null words were already accepted for all formats, and it affects one question per nested_variable run (nv_q11, truth `none`). Once the encoder emits `subnotes: []` there is no reason to keep it.
- **Bool equivalence.** `true/yes/1` and `false/no/0` match each other on non-count questions only. Probes: truth `1` on count_filter: `true`, `yes`, `one` wrong, `1`, `**1**`, `1.` right. truth `0`: `false`, `no` wrong. Count questions with truth 1 exist only as count_filter. Not leaking. A correct answer can still lose on spelling: `verified` / `not verified` for a status question is wrong. Low.
- **Leak 1 (low).** For null questions with truth `null` the "empty list" words also pass (`[]`, `empty`, `[0]`, `no notes`), because of `kind == "null_missing" and norm_exp in EMPTY_LIST_TOKENS` (which includes `null`). D9 says only explicit null words count there. 4 questions per format-seed.
- **Leak 2 (low).** `**a, b**` for a list answer scores wrong because bold is stripped per item, not around the whole answer (D10 says "wrapping the answer"). `a, b.` passes.
- D10: `**1.** a`, `**3**. c`, `*4.* d`, `__6.__ f`, `5) e` all parse. `2310.06825.` and `**2310.06825**` match. An indented `   2. nested` still overwrites answer 2 (low).

## 4. XML and the no-loss check

- Output: `<subnotes/>` exactly once (nested_variable), no `type=` attribute and no attribute of any kind in any dataset. `<year/>` for nulls.
- The relaxed compare (null equal to empty list) is inside `if format_name == "xml"` only. Mutation, scratch script, post-decode mutations on every format and dataset: year+1, int to str, drop a record, were all detected 24 of 24. Empty list to null: detected for json_min, toon, yaml, struct on nested_variable; undetected only for XML (the allowed limit). Other probes (bool to int, null to empty string) had no target in the datasets that have them, so they say nothing; the strict type check itself is `type(a) != type(b)` and is intact.
- Gap (low): the real data never trips the XML relaxation (the decoder maps the `subnotes` tag name to a list), so every XML row reads "Lossless match (100% exact)". The limit appears only in the report's note, not in the no-loss output.

## 5. Tests

- `python bench/r4/test_offline.py`: Ran 27 tests, OK (the first review saw 17).
- `cargo test --offline`, per binary: lib 0, doctor_tool 0, gbnf_tool 0, doctor_tests 16, float_tests 1, gbnf_tests 2, negative_tests 43, prompt_tests 14, roundtrip 1, spec_examples 15, doc-tests 0. Total 92, 0 failed.
- The builder's 87 is not what the tree produces. I get 92, equal to the previous known total, with no Rust file modified (git shows only bench/r4, research and milestones/R4.md). The builder most likely mis-summed. Nothing was lost.

## 6. Ground truth

All 48 recomputed from `bench/r4/data/*` and `bench/r1/fixtures/nested_variable.min.json` by a fresh script that parses each question text: 0 mismatches. IDs, claim texts used for lookup are unique. Data was regenerated earlier to the same bytes (hashes stable across my runs).

## 7. Other things that could make results wrong

- Failed requests that never succeed still count as 12 wrong answers each in the totals. They are listed, but the accuracy table does not exclude them. Report needs to read with that.
- Mean input tokens are averaged across datasets of very different size (100 rows vs 4), so they are not comparable per format by dataset.
- `real/backup/` is created and never used.
- Struct decoding was not re-run with `doctor_tool` here (it needs a schema file); relied on the python Struct decoder tests and the earlier review.

## Defects, ranked

1. TOON empty list uses the legacy `subnotes[0]:`; spec 4.4 requires `subnotes: []` (affects 1 line, nested_variable TOON only). Also the toon.py docstring, the report note and the `[0]` scoring token follow from it.
2. Report prints a fake, hand-typed "full prompt" with `...` and wrong lines instead of a real built prompt.
3. Report note about R1's TOON fixtures is wrong (blames `subnotes: []` and omits that the nested header is valid).
4. Null-truth questions accept empty-list words (D9 violation, 4 questions).
5. Bold around a whole list answer not stripped.
6. Failure history erased after successful rerun; budget count is a re-tokenization estimate.
7. XML limit never shown in the no-loss output.

## Bottom line

Close. The collector now does what it should: blank answers are wrong, failed requests rerun, budget is counted from thinking, and the checksums and port are right. The answers all check out and the tests pass (92, not 87).

Fix two things before running. Write the TOON empty list as `subnotes: []`, and have the report print the real prompt instead of a made-up one. Fix the wrong TOON note too. The rest is small. Not safe to run until those are done, but it is a short fix, and the results would only be slightly off if you ran now.
