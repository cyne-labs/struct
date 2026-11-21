# R4 grading cross-check B: grade a blind sample

Read `milestones/RULES.md`, the **Grading rules** section of `milestones/R4-grade.md`, the two extra rules in `milestones/R4-grade-haiku.md` (they apply to any model), and the **Be clinical** section of `milestones/R4-grade-shard.md`.

Answers from one model, across seven data formats, were graded by meaning. Your job is to grade a sample of them again, independently, so the maintainer can see whether the grades hold.

The sample is `bench/r4/grades/crosscheck_b/sample.json`. Each item points to a raw answer file in `bench/r4/grades/crosscheck_b/raw/`, with the question number, the question and the expected answer (`null` means the true answer is no value). The earlier grades aren't shown, on purpose. **Read only the files in `bench/r4/grades/crosscheck_b/`**, plus the rule files above. Don't look for the earlier grades or the original files anywhere else.

For each item: open the raw file, find the answer to that question number yourself, and grade it **correct**, **wrong** or **unclear** under the rules, clinically, the same way for every item. Judge each by reading it. No fuzzy matching or keyword rules. A script may only load and lay out the items.

Write only `bench/r4/grades/crosscheck_b/regrade.json`:

```json
{"items": [{"id": "<from the sample>", "model_answer": "<quoted exactly from the raw file>", "grade": "correct|wrong|unclear", "reason": "<one short factual line>"}]}
```

Every sample item must appear exactly once. Don't call any model or start any server. Don't commit or push.

## Final message

How many items you graded, and your counts of correct, wrong and unclear. Nothing else.
