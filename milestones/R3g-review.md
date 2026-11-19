# R3g review: check the regenerated run3 report

You are an **independent reviewer** in a fresh session. You didn't write `research/r3-real-run3.md`. Don't trust its claims: check them yourself.

**Read-only.** The only file you write is your review, `research/r3g-review.md`. Don't edit the report itself, `src/`, `tests/` or anything under `bench/r3/real/`. Don't run any model or `llama-server`. You may run `cargo build --release --bins` and `target/release/doctor_tool`. Put any scratch scripts outside the repository.

Read `milestones/RULES.md`, `SPEC.md` section 9, `milestones/R3d.md`, and `milestones/R3g.md` (the brief the report's author was given). The previous version of the report is in git (`git show HEAD:research/r3-real-run3.md`). The new one is uncommitted.

## Check, with evidence

1. **Re-scoring:** pick at least 6 runs, covering every model and at least 3 schemas, including one Gemma run with no reasoning and one Mellum s4 run. Run the current doctor on each raw output yourself, then work out recall and positional field accuracy against `ground_truth.json` by hand. Confirm that the report's numbers match, and show your working.
2. **The author's explanation:** the report says field accuracy rose because the old evaluator *"compared boolean values as strings against raw ground truth integers"*. An earlier independent review found a different cause: the old doctor build truncated values (for example, "Mistral 7B architecture" came out as "Mistral 7B architectur"). Find out which is true. Compare the old and new evaluation code, and run the old commit's doctor on `llama_3b_s1_flat_run1` if you need to. Say which explanation is correct, with evidence.
3. **The brief's checklist (R3g.md):**
   - the current doctor was used;
   - scoring is positional only;
   - every causal claim nobody isolated has been removed (quote anything left);
   - Gemma's runs with no reasoning are listed separately, and the averages exclude them;
   - tokens are compared with `usage.completion_tokens` for every run, with the largest difference stated (check that difference yourself);
   - the s4 decoded classes are stated;
   - the two rejected questions are gone;
   - the setup section and the prompt texts are still there.
4. **The old → new list:** check at least 5 of the entries against the two versions of the report.
5. **Overclaims and errors:** anything stated as fact that the data doesn't support, and any number that doesn't add up (row totals, percentages).
6. **Integrity:** the hashes for run1, run2 and run3 all match, and nothing under `bench/r3/real/` changed (`git status`).

## Final message
PASS or FAIL for each item, with evidence; defects ranked by severity; and a plain-English bottom line in simple words.
