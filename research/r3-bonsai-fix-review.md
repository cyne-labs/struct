# Review of the R3 Bonsai fix

1. PASS. `s4_run3_method` calls `e3.compute_field_accuracy("s4_two_class_code", ...)`, the run3 function itself (class split, then positional within class). `--s4-only` runs without a server. Its output matches the report: off 18/18 and 18/18; on 56/66 whole-list and 66/66 run3-method; run 3 is 5/15 vs 15/15.
2. PASS. My own script (doctor + ground_truth.json, class split) gives 15/15 for thinking-on runs 3, 2, 4 and 5, and 6/6 for run 1. Run 3's raw file alternates file/patch blocks; every field is correct.
3. PASS. Both figures appear in the summary matrix, comparison table, per-run table, observation 8 and the bottom line. Only the run3-method figure is called comparable (sections 1, 4, 6 and the bottom line).
4. PASS. `bonsai_off_s1_flat_run2.txt` line 1 is verbatim `~|Rope scaling hurts long ctx|refuted|2024|1|`. I recomputed the thinking-off s1 errors: 23 wrong, 20 are `~` ids in runs 2 to 5, 3 are `verified` (record 1, runs 1 to 3). 102 of 125 is correct. The 40 tokens (467 against 427) is in section 3.
5. PASS with caveats. A diff of the non-s4 lines shows no other number changed. Observation 10 became 11, so the numbering shifted.
6. PASS. `git status --short bench/r3/real` is empty.

Defects, by severity:
- Low. "Bonsai wrote files and patches alternating" is written as a general statement, in sections 1, 4 and 6 and the bottom line. Only thinking-on s4 run 3 alternated; observation 8 says all other runs group. The whole-list gap comes entirely from that one run. It should say "in run 3".
- Low. "40 tokens longer" comes from one s1 probe. The report applies it to every schema in sections 1 and 12.
- Low. "Which SPEC allows" is inferred from the SPEC sections on outside classes and `lookup`. It is not stated outright.
- No causal overclaims found. The text has no "artifact" or "does not make data errors" wording. The s1 text describes what happened, not why.

Bottom line: The fix does what the brief asked. Bonsai's s4 score is now shown two ways. Under the way the other models were scored, it is 100% in both modes. The lower 84.8% comes from one run, where Bonsai interleaved files and patches. I recomputed every figure and they match. Nothing in the raw data changed. The one thing to tidy is wording that says Bonsai "alternated" in general, when it did so in one run.
