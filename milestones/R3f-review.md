# R3f review: check the simple doctor

You are an **independent reviewer** in a fresh session. You didn't write this code. Don't trust its author's claims: check every one of them yourself.

**Read-only.** Don't edit `src/`, `tests/`, `SPEC.md`, `bench/` or `research/`. Write only your review, to `research/r3f-review.md`. You may run `cargo test --offline` and `cargo build`. If you want to prove a test can fail, change one file temporarily; then restore it, and show with `git diff` that it's back to its original state. Don't run any model or `llama-server`.

Read `milestones/RULES.md`, `SPEC.md` section 9, and `milestones/R3f.md` (the brief the builder was given). The builder's changes are uncommitted: see `git diff` (the baseline is the last commit).

## Check, with evidence (file:line and command output)

1. **Tests:** `cargo test --offline` passes. Give the counts per test binary, and paste the damage-suite table.
2. **The doctor matches SPEC section 9 exactly:**
   - it has exactly 4 fixes;
   - fix 4 adds `|` to any data line that doesn't end with one, child lines included, and looks at nothing else: no parent, list or index lookup;
   - chatter has exactly the one exception in the spec;
   - nothing is ever deleted for being incomplete.

   Run at least 10 small inputs of your own through `doctor_tool`, including child lines, raw values open at the end, `Hope this helps!` under `{text}` and under `{year:int}`, and a multi-line raw record followed by a bad line. Show input → output.
3. **Removed code:** fix 5, the ambiguity rule, the parent scan and `get_entry_schema_at` are really gone, with no leftover dead paths.
4. **Simplicity:** the brief asked for the cleanup that follows (merge the 5 child-drop loops and the `expected N columns` branches, delete the dead branches, split `doctor()`). Is it done? `doctor.rs` went from 1,826 lines to 1,639. List the largest remaining parts, and anything still duplicated or more complex than section 9 requires.
5. **Splice fix:** the new test uses exactly the input the brief named, and the fix is real.
6. **Damage suite honesty:**
   - Expected results come from SPEC and the original data, never from the doctor.
   - There are no skips and no branch that accepts either of two results.
   - The `truncated-but-valid` column is computed, not hard-coded.
   - The false-repair check is real. Prove it: make the doctor change one value, confirm the suite fails, then restore it.
   - Explain why the table shows 6,200 cases and 6,053 restorations, and what the other columns mean.
7. **Scope:** list any change outside `src/` and `tests/`, or beyond the brief.

## Final message
PASS or FAIL for each item, with evidence; defects ranked by severity; and a plain-English bottom line of a few sentences. Use simple words, because the maintainer reads it.
