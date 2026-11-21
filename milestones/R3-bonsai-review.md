# Review: check the Bonsai run3 report

You are an **independent reviewer** in a fresh session. You didn't write `research/r3-real-bonsai.md` or the scripts behind it. Don't trust their claims: check them yourself.

**Read-only.** The only file you write is your review, `research/r3-bonsai-review.md`. Don't edit the report, `src/`, `tests/`, any script under `bench/`, or anything under `bench/r3/real/`. Don't run any model or `llama-server`. You may run `cargo build --release --bins` and `target/release/doctor_tool`. Put any scratch scripts outside the repository.

Read `milestones/RULES.md`, `SPEC.md` section 9, and `research/r3-real-run3.md` (the run3 report Bonsai is compared against). The work under review is in the last commit (`git show --stat HEAD`): `bench/r3/collect_bonsai.py`, `bench/r3/evaluate_bonsai.py`, `bench/r3/real/run3_bonsai/` and `research/r3-real-bonsai.md`.

## Check, with evidence

1. **Re-scoring:** pick at least 6 runs, covering both modes and all 4 schemas, including the thinking-on s4 run 5 (20,275 thinking tokens) and one thinking-off s3 run. Run the current doctor on each raw output yourself, then work out record recall and positional field accuracy against `ground_truth.json` by hand. Confirm the report's numbers match, and show your working.
2. **Same setup as run3:** the user prompt sent to Bonsai is byte-identical to run3's for each schema. Confirm it. The report says the thinking-on prompt is 40 tokens longer because the chat template adds a "Reasoning effort is set to xhigh" system line. Check that against the template text saved in the report, and say whether it means the two modes were not given the same input.
3. **s4 field accuracy:** `evaluate_bonsai.py` scores s4 positionally over the whole record list. Compare that with how `evaluate_run3.py` scored s4 for the run3 models. Are Bonsai's s4 numbers comparable with Mellum's 20% / 100%? If not, say exactly what differs and what Bonsai's s4 number would be under run3's method.
4. **Thinking-off s1:** recall is 100% but field accuracy is 81.6%. Find which fields are wrong in those runs, and quote examples. Describe what happened; don't guess why.
5. **Token counts:** the report says `usage.completion_tokens` differs from `/tokenize` (thinking plus answer) by 1 to 12 tokens per run. You can't call `/tokenize`, so check that the stored numbers in `metadata.json` support this, and state the largest difference.
6. **Overclaims and errors:** anything stated as fact that the data doesn't support, any causal claim nobody isolated, and any number that doesn't add up (row totals, percentages, means). The report must state the q4_0 KV cache and the different sampling without claiming either caused anything.
7. **Integrity:** the hashes for run1, run2, run3 and run3_bonsai all match (run1's SHA256SUMS has CRLF endings; strip them before checking), and `git status` shows nothing under `bench/r3/real/` changed.

## Final message
PASS or FAIL for each item, with evidence; defects ranked by severity; and a plain-English bottom line in simple words, because the maintainer reads it.
