# R4 build review (independent)

Scope: uncommitted `bench/r4/`. Scratch scripts in the session scratchpad. No server, no model, no repo edits except this file.

## Verdicts

| # | Item | Result |
|---|------|--------|
| 1 | Tests real | PASS (small gap) |
| 2 | Encodings correct | FAIL for TOON (nested_fixed, numeric ids, empty list); XML has an extra attribute |
| 3 | Same data everywhere | PASS |
| 4 | Questions and truths | PASS (some ambiguity) |
| 5 | Prompt | PASS (minor) |
| 6 | Scoring | FAIL (missing answer scores as correct on null questions) |
| 7 | Collector | FAIL (wrong launch line, failed requests never retried) |
| 8 | Scope | PASS with notes |

## 1. Tests
`python bench/r4/test_offline.py`: Ran 17 tests, OK. Dry run: 120 prompts built.
Mutation check (monkeypatched encoders in a scratch script): changing one year in the encoded text was caught for json_min, toon, yaml, xml and struct, on flat_tabular and nested_fixed (10 of 10 detected). So the no-loss check can fail.
Gap: it compares with `==`, so `True == 1`. A bool changed to int in the source went undetected (False for "detected"). Type loss is invisible. Low.
Side effect: running the tests rewrites `bench/r4/questions.json`.

## 2. Encodings
Independently parsed: JSON (json), YAML (PyYAML safe_load), XML (ElementTree), TSV (csv), key:value (own splitter on `---`), Struct (`target/release/doctor_tool.exe strict`). All decode equal to source, with strict type equality. doctor_tool: 20/100/10/4 records, no repairs, no bad lines.
Nulls: JSON/YAML/TOON/key:value `null`, TSV empty cell, XML `<year/>`, Struct `~`. All match the R4 table.
Defects:
- **TOON nested_fixed** uses `[10]{text,verdict,source{id,year},verified}:`. Nested fields inside a tabular header are not valid TOON. It is inherited from the R1 fixture, but R4 says "per its spec".
- **TOON ids** like `2405.05254` are unquoted. Per spec, numeric-looking strings must be quoted. A TOON reader would read them as numbers. Same for the YAML/TOON difference: YAML quotes them, TOON does not.
- **TOON empty list** is written `subnotes: []`. Spec form is `subnotes[0]:`.
- **XML nested_variable** writes empty lists as `<subnotes type="list"></subnotes>`. This is an extra attribute no other format gets, and it is a hint. The brief said no null attributes. This is a list marker, not a null marker, but it is still an extra.
- TOON and Struct encoders are hardcoded per shape, not general. Fine for 4 datasets.
- Struct inherently carries the enum value list and types in its schema line. Not a defect, but report it as a confound.

## 3. Same data
R1 fixtures: `git status` and `git diff --stat bench/r1` are empty (unchanged). R4 data regenerated in memory from `generate_data.py` twice: identical to each other and to the files on disk. All formats of a dataset equal the same source (section 2).
nested_variable is R1's file, so it has no nulls. Its "null" questions use empty subnote lists.

## 4. Questions
12 per dataset, 48 total. Kinds: flat 4/4/4 (single, count, null), nested 3/3/3/3.
All 48 ground truths recomputed independently from source JSON: 0 mismatches. IDs and titles used for lookup are unique. Null questions point at real nulls or real empty lists only.
Ambiguity risks:
- "verification status" has truth `true`/`false`. `yes`, `verified`, `no` score wrong. Struct shows `1`/`0`.
- nv_q11 truth is `none`. `empty`, `[]`, `no subnotes` score wrong.
- Several null questions are labelled `count_filter` (f100_q08) or are plain lookups. Kind tallies are slightly off.
- f100_q09 and q10 need 10-item lists, very hard for exact set match. Fine, but expect low scores.
- There are no real empty-string values anywhere (brief: "some empty values if allowed"). XML empty tag cannot tell empty from null anyway.

## 5. Prompt
Printed one full prompt (struct, flat_tabular). Same template for all formats: one naming line plus the data block. Differences: the code fence tag (`json`, `toon`, `tsv`, `xml`, `text`, `struct`, `yaml`). This is a second format name, small. No syntax explanations. Report prints only a template, not an actual full prompt.

## 6. Scoring
Works: case, quotes, whitespace; null words; list set match; extra or missing item wrong; skipped number handled (answers 4,5 kept); chatter ignored.
Defects:
- **HIGH:** empty or missing answer counts as correct when truth is no-value. A completely empty run scored 1/12 (the `null` scalar question). Error and timeout runs get free credit too. Brief: missing or unparseable = wrong.
- Bool and number: `true`/`1` and `false`/`0` are accepted both ways, good for Struct. Side effect: a count answer `1` equals `true`. Low. `yes` is wrong. `2,024`, `2024.0` wrong.
- Trailing period (`2310.06825.`), markdown bold (`**1**`), `**1.** a` numbering all score wrong or unparsed. Strict per brief, but this is a model-chatter risk to the results. Medium.
- An indented line like `   2. nested thing` overwrites answer 2. Low.
- Budget exhaustion is detected only by `finish_reason == "length"`. With no output cap and `--reasoning-budget`, that will rarely fire, so the count is probably always 0. Medium.
- Report lacks accuracy per format by question kind (only by kind overall). Input tokens are averaged across datasets of very different size.

## 7. Collector
Right: seeds 42-46, 24 per seed, 120 total; TSV and key:value flat only; thinking on via `chat_template_kwargs`; temp 1.0, top_p 0.95, top_k 20, min_p 0.05, presence 0.0, repeat 1.0; no prefill; no `max_tokens`; 1200 s timeout; raw `.txt` plus SHA256SUMS over `raw/`; resume by (format, dataset, seed) from `metadata.json`.
Server: **does not start or stop it.** It only checks `/health` on 127.0.0.1:8090 and exits if down. The user starts and stops it. (R3 collect did its own start/stop.)
Defects:
- **MEDIUM-HIGH:** `LAUNCH_LINE` (collect.py and evaluate.py, goes into the report) says `-p 8090`. In llama-server `-p` is the prompt, not the port; it should be `--port 8090`. It also omits `--mmproj ... --no-mmproj-offload` from the maintainer's line.
- A failed request is saved to `metadata.json` as done (`finish_reason: "error: ..."`), so a rerun never retries it. It then scores as empty (see section 6).
- `reasoning_content` and usage exist only inside `metadata.json["raw_response"]`. `metadata.json` is not in SHA256SUMS. Thinking tokens come from `/tokenize` of the text, not from usage.
- Tokenize failure silently returns 0.
- Extra: `real/backup/` copies of raw outputs (not asked for).

## 8. Scope
Beyond the brief: `real/backup/`, a GPU and disk check (from RULES, OK), `generate_*` scripts (needed). Missing: format x kind table; budget-hit detection that works; real prompt printed in the report.

## Defects ranked
1. Empty or missing answer scored correct on null questions (evaluate.py).
2. Wrong launch line (`-p`) written into the report; mmproj flags dropped.
3. Failed requests marked done, not retried.
4. TOON not per spec (nested header, unquoted numeric ids, `[]`).
5. Budget-exhaustion count unreliable.
6. XML `type="list"` extra attribute.
7. metadata.json (reasoning) unhashed; no format x kind table.
8. Strict chatter handling, `==` type blindness, ambiguity in bool and `none` answers.

## Bottom line
The data, the question answers and most encodings are right. I rechecked all 48 answers and the Struct, JSON, YAML, XML, TSV and key:value files myself, and they match.
Do not run it yet. Fix three things first. One: an empty answer must not count as right. Two: write the real launch line, with `--port`. Three: failed requests must be retried. TOON should also be fixed or the maintainer should accept the R1 form. The XML list marker should go.
