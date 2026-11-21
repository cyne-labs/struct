# Fix: Bonsai report, s4 scoring and two wording gaps

Read `milestones/RULES.md`, `research/r3-bonsai-review.md` (the review that found these problems), `research/r3-real-bonsai.md`, `bench/r3/evaluate_bonsai.py` and `bench/r3/evaluate_run3.py`.

**Don't touch** anything under `bench/r3/real/`, `src/`, `tests/` or `evaluate_run3.py`. Don't run any model or `llama-server`. **No server is running**, so a full rerun of `evaluate_bonsai.py` would lose the server-settings and template sections of the report. Don't regenerate the whole report; edit the affected parts of it by hand, using numbers your script computes.

## Fixes (the maintainer decided these)

1. **s4: show both numbers.**
   - Add to `evaluate_bonsai.py` a second s4 field-accuracy figure computed exactly the way `evaluate_run3.py` scores s4 (records split by class, then compared in order within each class). Keep the existing whole-list positional figure. Add a way to print only the s4 figures without calling the server (for example a `--s4-only` flag).
   - In the report, everywhere Bonsai's s4 field accuracy appears (summary matrix, comparison table, per-run table, observations, bottom line), show both: the whole-list figure and the run3-method figure, clearly labelled. Add one plain sentence explaining why they differ: Bonsai wrote files and patches alternating, which SPEC allows, and the whole-list method compares them against a list with all files first.
   - Only the run3-method figure is comparable with the run3 models' s4 numbers. Say so where the comparison table sits.
2. **Prompt wording.** Where the report says the prompt is the same as run3's, add that the task text is byte-identical, but with thinking on the chat template adds a system line ("Reasoning effort is set to xhigh"), so the thinking-on input is 40 tokens longer. No causal claim.
3. **Thinking-off s1.** Add the breakdown the review found: in runs 2 to 5 the model wrote `~` for every paper id (20 of the 23 wrong fields), and 3 `verified` values were wrong. Quote one real line from the raw output. Describe what happened, not why.

Check every number you write against your script's output and the raw files. Don't change any other number in the report.

## Final message
What changed in the script and the report, the new s4 figures per mode with their counts, and a plain-English summary (RULES rule 10).
