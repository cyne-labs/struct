# R3e: doctor fixes found by the random damage suite

Read `RULES.md`, then `SPEC.md` section 9. Fixes 3, 4 and 5 changed: read them again closely. Where SPEC.md doesn't answer something, stop and ask (rule 5). Make no changes beyond this brief.

**Another milestone (R3d) is running a GPU benchmark at the same time.** Don't start, stop or touch `llama-server`, and don't touch `bench/` or `research/`. Work only in `src/` and `tests/`.

## Background

`tests/doctor_tests.rs` now contains a random damage suite, `test_damage_suite_5000_cases`: 6,400 cases, each with its own random schema and file. To replay one case, set `DOCTOR_REPRO=<row>:<seed hex>`. It fails 327 cases. All of the failures come from fixes 4 and 5 on **child lines**: they pick the list from the class's first list, not from the child's own parent, and they ignore the index column.

## 1. Fix the three bugs in `src/doctor.rs`

Each bug below has its schema, the text (`\n` is a line break), the wrong behaviour, and the correct one.

1. **A valid last child line is dropped.** Schema `schema:c{n:int, a:list[record{p, q, r}], b:list[int]}`, text `1|\n>2|5|\n`. Today it gives a `CutOffLastRecord` on line 2. Correct: 0 repairs and 0 bad lines; `5` is an item of `b`.
2. **A child line missing its final `|` is never repaired under a parent with several lists.** Schema `schema:c{n:int, a:list[int], b:list[int]}`, text `1|\n>2|5`. Today it gives a bad line. Correct: fix 4, giving `>2|5|`.
3. **A depth-2 line under a parent with no lists is dropped as a cut-off.** Schema `schema:n{l, s:list[record{t, y}]}`, text `~|\n>a|b|\n>>c|\n`. Correct: a bad line (depth jump: `record{t, y}` has no list).

Fix the cause: fixes 4 and 5 must resolve a child line against **its own parent node**, using the index column when the parent has several lists. Don't special-case these three inputs.

## 2. Implement the SPEC changes

- **Fix 4** now also applies to a line with **no `|` at all**, when adding one gives exactly the right number of columns and every value is valid. Fix 4 is tried before fix 3, so a line it can complete is a record, not chatter. Under a one-column `{id:int}` schema, `1|\n1706` becomes two records, with one fix-4 repair on line 2. A trailing `Hope this helps!` under `{id:int}` is still chatter, since it isn't a valid int.
- On a record whose raw value spans several lines, fix 4 is reported on the record's **last** line.
- **Fix 5:**
  - a raw value still open at the end of the output is a cut-off;
  - a cut that leaves the last value broken (for example, a lone trailing `\`) is a cut-off;
  - a child line under a parent with several lists is a cut-off **only when it has one reading**. `>2|` as the last line, under `schema:c{n:int, a:list[int], b:list[int]}`, is a **bad line**: `2` could be the field number with its value cut off, or a value with its number left out. Report the problem as `ambiguous: missing list index or cut-off value`.

## 3. Tests

- Add a test for each of the three bugs, using the exact inputs above.
- Add a test for each SPEC change above, with its included and excluded cases. Each negative test checks the exact `BadLine`.
- Update the damage suite for the SPEC changes: the five questions its author couldn't decide are now answered (see section 9). Make it assert them instead of accepting either result or skipping them. Keep its random generation, its per-case seeds, and the real false-repair check.
- `cargo test --offline` passes in full, with the damage suite at **0 failures** and **0 false repairs**. Update the old `test_bad_lines_coverage` only where the SPEC changes require it, and say what you changed and why.

## Done when
All tests pass; the damage-suite table shows 0 failures, 0 false repairs and 0 skipped (or every skip is explained by a SPEC rule).

## Final message
- Test counts and the damage-suite table, exactly as printed;
- what caused the bugs, in two or three sentences;
- every test you changed, and why;
- any questions (rule 5);
- the plain-English overview (rule 10).
