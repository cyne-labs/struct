# R3f: the simple doctor

Read `RULES.md`, then `SPEC.md` section 9 closely: it was rewritten. The maintainer's goal is **a doctor that is as simple as possible**. Where SPEC.md doesn't answer something, stop and ask (rule 5). Make no changes beyond this brief. Don't touch `bench/` or `research/`.

## What changed in SPEC section 9

- **Only 4 fixes remain:** code fences, echoed schema, chatter, and a missing final `|`.
- **Fix 4 is now unconditional.** Any data line that doesn't end with `|`, child lines (`>`) included, gets `|` added, and that's all. The doctor doesn't check whether the result is valid; normal validation decides that afterwards. A raw value still open at the end of the output gets its `|` at the very end.
- **Chatter has one exception.** A no-`|` line at the top or bottom is chatter *unless* it becomes a valid record once `|` is added. Under a one-text-column schema, `Hope this helps!` is therefore a record.
- **The cut-off fix is gone.** Nothing is ever deleted for being incomplete.
- **Fix 4 never looks at the line's contents.** It doesn't look up the parent, the list or the index. It only adds the final `|`.

## Part A: the code (`src/doctor.rs`)

1. Implement section 9 exactly as written above.
2. **Delete what's no longer needed:**
   - fix 5 (`CutOffLastRecord`) and everything that only served it: the parent scan, `get_entry_schema_at`, and the ambiguity rule and its message;
   - all of the parent, list and index logic in `try_fix4`. Fix 4 becomes "append `|`". The chatter check alone may still need to test whether a line is a valid record once `|` is added.

   `SafeFix` keeps only the 4 kinds.
3. **Clean up without changing behaviour:**
   - merge the 5 copy-pasted "drop the child lines of a bad line" loops into one;
   - merge the repeated `expected N columns` branches;
   - delete the dead branches: the `parent.is_bad` branch, the unreachable `unclosed raw value` and `empty child line` messages, and the redundant float `"0"` and `"-0"` cases;
   - split the ~915-line `doctor()` into a few clearly named steps that follow the section 9 order.

   Give the final line count of `doctor.rs`.
4. **Fix the splice bug.** `splice` replaces lines by their original line number while the list shrinks, so a bad line after a multi-line raw record lands in the wrong place. Add a test with exactly that input.

## Part B: tests

- Delete the tests for removed behaviour: fix 5 and the ambiguity rule; update the child-line fix-4 tests to the new rule. Say which ones you deleted.
- Add tests for the new rules:
  - `2024|Rope` becomes `2024|Rope|`, valid;
  - `2024` under `{year:int, title}` gets `|` added, and is then a bad line with `expected 2 columns, found 1`;
  - a raw value open at the end of the output is closed;
  - `>rope` (child, no final `|`) becomes `>rope|`, with a fix-4 repair, and is then checked like any other line;
  - `Hope this helps!` at the end is a record under `{text}`, and chatter under `{year:int}`;
  - a cut-off last line `2024|Rope scal` becomes a valid record, with the fix-4 repair reported.
- **Damage suite:** keep the random generator, the per-case seeds and the real false-repair check. Remove the cut-off-last-record kind as a separate fix. A cut-off is now just damage whose expected outcome follows from the new rules:
  - any cut line, top-level or child, gets `|` added; then the line is either valid or a bad line.

  Compute the expected outcome from SPEC; never take it from the doctor. Redefine "false repair" as any value change other than the reported structural fixes. A truncated value that a fix-4 `|` turned into a record is the expected result here, not a false repair. Count those cases in their own column, `truncated-but-valid`, so they're visible.
- `cargo test --offline` passes in full: the damage suite has 0 failures and 0 false repairs, and no case is skipped or allowed to match either of two results.

## Final message
- Test counts and the damage-suite table, exactly as printed;
- the line count of `doctor.rs`, before and after;
- the tests you deleted or changed, and why;
- any questions (rule 5);
- the plain-English overview (rule 10).
