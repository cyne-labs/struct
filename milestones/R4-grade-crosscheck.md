# R4 grading cross-check: grade a blind sample

Read `milestones/RULES.md`, the **Grading rules** section of `milestones/R4-grade.md`, the two extra rules in `milestones/R4-grade-haiku.md`, and the **Be clinical** section of `milestones/R4-grade-shard.md`.

Fourteen graders each graded one format for one model. Because each format had its own grader, any difference in how strict they were would look like a difference between formats. Your job is to grade a sample of their items again, independently, so the maintainer can see whether they agree.

The sample is `bench/r4/grades/crosscheck/sample.json`: items from all 14 shards, shuffled. It doesn't show the earlier grades, on purpose. **Don't open any other file under `bench/r4/grades/`**, and don't look for the earlier grades anywhere. Grade only from the raw file and the rules.

For each item: open the raw file, find the model's answer to that question number yourself, and grade it **correct**, **wrong** or **unclear** under the rules, clinically, the same way for every format. Judge each by reading it. No fuzzy matching or keyword rules. A script may only load and lay out the items.

Write only `bench/r4/grades/crosscheck/regrade.json`:

```json
{"items": [{"id": "<from the sample>", "model_answer": "<quoted exactly from the raw file>", "grade": "correct|wrong|unclear", "reason": "<one short factual line>"}]}
```

Every sample item must appear exactly once. Don't call any model or start any server. Don't commit or push.

## Final message

How many items you graded, and your counts of correct, wrong and unclear. Nothing else: the maintainer's session compares your grades with the shards'.
