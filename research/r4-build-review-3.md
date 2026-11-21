# R4 build review 3 (final check)

Own inputs, fake HTTP server on 8090, no model. Scratch in session scratchpad (t1-t3.py).

| # | Item | Result |
|---|------|--------|
| 1 | TOON empty list `subnotes: []`, no `[0]` | PASS (printed nested_variable TOON; `[0]` only in comments/tests/legacy decode) |
| 2 | Report prompt = collector prompt | PASS (struct/flat_tabular: byte-identical, 1932 chars, no `...`). Report shows one example prompt only. |
| 3 | Wrong R1 TOON note gone | PASS (new note states nested groups, quoting, `key: []`) |
| 4 | Scoring | PASS (see below) |
| 5 | Failure history / counting | PASS for history, FAIL-ish for denominator (see below) |
| 6 | /props flags | PASS with caveat |
| 7 | Tests | PASS: test_offline 35 OK; questions.json sha 98299b24... unchanged; cargo 92 pass (16+1+2+43+14+1+15) |

## 4 Scoring
- Null truth `null`: accepted null/none/~/n/a/N/A/**null**/null.; wrong: empty, [], [0], "no notes", "", "  ", None, unknown.
- Empty-list truth `none`: accepts none/empty/[]/null/~/n/a; wrong: "", [0], unknown, 0.
- List `a, b`: `**a, b**`, `*a, b*`, `**a**, **b**`, reordered, `__a, b__.` right; extra item wrong.
- Quirk (low): nv_q11 truth `none` has kind null_missing, so empty/[]/no notes pass there. That is D11 (empty list), correct.

## 5 Failures
- Each run appends to metadata; failure entries kept after a successful rerun (run1 fail, run2 fail, run3 ok: 26 entries, 2 failures kept, both listed in the report's failed table).
- No in-run retries. A failed request is just a metadata entry; resume reruns only it.
- DENOMINATOR: failed runs are excluded from every accuracy total (not counted wrong). Simulated seed 42, all answers perfect, xml/nested_fixed fails: XML shows 36/36 (100%) over 3 runs instead of 48 questions; by_format_dataset xml/nested_fixed shows 0/0 (0.0%). After the rerun succeeds: 48/48. A format whose failures are the hard/long ones (timeouts) gets inflated accuracy, and the main table has no column for missing runs. Failed runs appear only in a separate table. Fix: show runs completed/expected per format, or count unfinished as wrong.

## 6 /props
- Fake /props with `launch_args`: stored in server_props.json (`launch_flags` + full props), in every metadata entry, and server_props.json hashed in SHA256SUMS (hash verified).
- Caveat (medium): real llama-server /props has no launch args. Without them the code derives a partial line (`-m -c --reasoning-budget --port --host`; missing -ngl, cache types, -fa, mlock) and uses it INSTEAD of the full LAUNCH_LINE constant, so the report's "exact line" would be incomplete. Full props are still saved.

## Other
- Budget count: re-tokenised reasoning, approximate (unchanged, low).
- `real/backup/` created, unused.

## Defects ranked
1. Failed runs dropped from accuracy denominator (inflation, silent in main table).
2. Derived partial launch line replaces the full constant.
3. Budget count approximate.

Verdict: safe to run, provided the operator checks the failed-runs table and reruns until none remain (then item 1 is moot); item 2 means record the real launch line by hand.
