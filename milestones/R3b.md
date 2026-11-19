# R3b: fixes after the R3 review

Read `RULES.md`, then `SPEC.md` section 9. Fix exactly what's listed here, and nothing else. **Don't** change how header rows are handled, and **don't** collect new model outputs. Both wait for maintainer decisions. Where SPEC.md doesn't answer something, stop and ask (rule 5).

An independent review found the problems below. Each one names the input that shows it.

## 1. Doctor bugs (`src/doctor.rs`)

1. **A raw value cut off at the end of the file.** Input: `x|y|`, then `w|\rfn main() {`, then `  let a = 1;`, with the file ending there (the `\r` is a literal backslash and `r`). Today this gives a bad line for line 2 and a cut-off for line 3. It must be **one** cut-off-last-record repair covering lines 2–3, with no bad line, so strict mode succeeds.
2. **An unclosed raw value in the middle of the file.** Input `x|\rfoo`, then `y|z|`, under schema `{a, b}`. Per SPEC section 3, the raw value runs to the next unescaped `|`. So the raw value is `foo` + line break + `y`, followed by `z`: that's 3 values where 2 are expected, so it's **one bad line covering lines 1–2**. Today `y|z|` is kept as a separate record, which turns part of a raw value into a fake record.
   Also, `x|\rl1`, `>l2`, `schema:zz|`, `>n1|` is a valid record with one child, because inner lines of a raw value are text (SPEC section 1). Today it's reported as unclosed.
3. **Columns.** A bad line's column must point at the start of the value or position that is wrong. For example, `1|zz|` with `zz` invalid gives column 3, and `  \t bad|` gives the column where `bad` starts. Apply the same rule to every message. Every column refers to the model's original text.

## 2. Tests (`tests/doctor_tests.rs`)

1. **Every negative check asserts the exact `BadLine`:** its line, column, full message and dropped-child flag. Substring matching is not allowed.
2. **Rewrite the damage suite** to at least 5,000 cases, covering:
   - **fixable damage:** each of the 6 fixes on its own *and* in random combinations, **including the cut-off last record**. Each case checks that the data equals the original **and** that the reported repairs are exactly the damage applied: their kind, line and count;
   - **unfixable damage:** every kind of bad line SPEC section 9 lists, plus each forbidden value fix (`"2024"`, `+5`, `1,000`, `Supported`, `year: 2024`, `null`, `None`, `N/A`, `true`, `yes`, an empty field), and an unclosed raw value mid-file. Each case checks that the damaged lines are reported, and that **every value on every other line comes back byte-identical to the original**, in salvage mode;
   - **a real false-repair check.** For each case, compare every value in the doctor's output with the original. Any value that differs and isn't a reported structural repair is a false repair, and must make the test fail;
   - **no damage:** zero repairs and zero bad lines.

   Print a table with one row per damage kind: the cases run, exact restorations, bad lines reported, and false repairs.
3. **Add tests for 1.1, 1.2 and 1.3,** using the exact inputs above.

## 3. Harness (`bench/r3/collect.py`)

Fix it, but **don't run collection**:
- write raw outputs in binary mode, so the model's exact bytes are kept (no CR LF conversion), and don't strip any characters;
- save each run's thinking text as its own file, next to the output;
- **turn LFM's thinking off properly.** Find out from its chat template how it's done (for example, prefilling an empty `<think></think>`). Document the method. If the template offers no clean way, stop and say so;
- record whether the run stopped at the token cap from llama-server's own response field, and say which field.

## 4. Report corrections

In `research/r3-doctor.md`:
- correct the repair total to the true count (the table sums to 499);
- replace the damage table with the new suite's real output;
- add a short section, "Known limits of the R3 real-data run". It covers: the prompt gave no notes and no value rules, so 0 of 80 runs were usable; LFM's thinking was not off; the saved files have CR LF line endings. That run will be redone with an official prompt.

## Done when
`cargo test --offline` passes with 0 false repairs under the real check; the per-kind table is printed; `collect.py` is fixed but not run.

## Final message
- The test counts;
- the per-kind damage table, exactly as printed;
- how LFM's thinking is now turned off;
- any SPEC questions (rule 5);
- the plain-English overview (rule 10), covering only what changed.
