# R4 grading, one shard: grade one model's answers for one format

Read `milestones/RULES.md` and the **Grading rules** section of `milestones/R4-grade.md`. Those rules are the only ones you use. For Haiku, also follow the two extra rules in `milestones/R4-grade-haiku.md`: grade the answer, not the bracketed note after it, unless the note changes or contradicts it; and if the model corrects itself mid-answer, grade the final answer it settles on and say so.

The prompt names your **model** and **format**. Your work list is `bench/r4/grades/work/{model}_{format}.json`. Thirteen other agents are grading the other shards in parallel. Write only your own output file. Don't touch anything else: no other file under `bench/r4/`, no report, no `.py` file. Don't call any model or start any server.

## The work list

Each item is one answer to re-grade:
- every answer the strict scorer marked **wrong**;
- plus a 5% sample of answers it marked **correct** (`"spot_check": true`), to check that strict "correct" really is correct.

Each item gives the raw file path, the question number, the question, its kind, the expected answer (`null` means the true answer is no value) and the strict scorer's parsed answer.

## Be clinical

Thirteen other graders apply these same rules to the other formats, and the results are compared format against format. Any leniency or strictness of your own shows up as a difference between formats, so:
- Apply the rules literally and the same way to every item. No benefit of the doubt, no extra strictness, no sympathy for a format.
- Don't infer what the model "meant". Grade only what it wrote. If the written answer doesn't plainly give the right value, it is wrong; if you can't tell, it is unclear.
- Same kind of answer, same grade, every time. Decide once how a pattern is graded (for example "No verdict given (the field is ~)" for a no-value answer), then grade every instance of it that way.
- Reasons are factual and short: what the answer says and which rule applies. No opinions about the format or the model.

A fresh session will re-grade a sample of every shard's changed grades and check that it agrees.

## For each item

1. Open the raw file and find the model's answer to that question number yourself. Don't trust `strict_parsed_answer`: the parser can line answers up wrongly when the model numbers oddly, merges lines or skips one. If the answer you find differs from the parsed one, say so in the reason.
2. Grade it by meaning: **correct**, **wrong** or **unclear**, under the grading rules. Judge each one by reading it. Don't use fuzzy matching or keyword rules to decide a grade. A script may only load and lay out the items.

## Output: `bench/r4/grades/{model}_{format}.json`

```json
{
  "model": "...", "format": "...",
  "total_questions": <from the work list>, "strict_correct": <from the work list>, "strict_wrong": <from the work list>,
  "items": [
    {"raw_file": "...", "dataset": "...", "seed": 0, "question_number": 0, "question": "...", "expected": "...",
     "strict_grade": "wrong|correct", "spot_check": false,
     "model_answer": "<quoted exactly from the raw file>",
     "grade": "correct|wrong|unclear",
     "reason": "<one line; required whenever grade differs from strict_grade, or the answer you found differs from strict_parsed_answer>"}
  ]
}
```

Every item in the work list must appear exactly once. Check the count before you finish.

## Final message

Your shard's counts: items graded; wrong changed to correct; correct changed to wrong; unclear. Plus one plain-English sentence on what you saw. Don't commit or push.
