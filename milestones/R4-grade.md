# R4 grading: did the model actually get it right?

Read `milestones/RULES.md`, `milestones/R4.md`, `bench/r4/questions.json` and `research/r4-retrieval.md`.

R4's scorer is strict: it compares normalized text. That misses answers that are right but worded differently, for example "the verdict for paper 2411.2824 is unverified" when the answer is `unverified`. Your job is to grade every Gemma answer **by meaning**, so the maintainer can see the true accuracy next to the strict one.

**Don't touch** `bench/r4/real/`, `bench/r4/real_haiku/`, any `.py` file under `bench/r4/` or `research/r4-retrieval.md`. Another agent is fixing harness bugs in parallel. Don't call any model or start any server. Write only the two output files below.

## Input

- The 120 Gemma raw answers: `bench/r4/real/raw/gemma4e2b_r4_{format}_{dataset}_seed{n}.txt`.
- The questions and true answers: `bench/r4/questions.json` (each question has a kind and the expected answer, computed from the source data).
- Read the raw text yourself. Don't trust the harness's parsing: if the model numbered its answers oddly, merged lines, or skipped one, work out which answer belongs to which question from the text, and say so in the reason.

## Grading rules

An answer is **correct** only if it gives exactly the right value, whatever the wording:
- A sentence that states the right value is correct. One that also states a wrong value, or hedges between values, is wrong.
- **Counts:** the number must be exact. A right number with a wrong supporting list is still correct for the count, but note it.
- **Lists:** exactly the right set of items, any order, nothing missing, nothing extra.
- **No value:** when the true answer is no value, any clear statement that it is missing is correct ("null", "none", "not provided", "the year field is empty", "no year"). Saying a value exists, or giving one, is wrong. A blank answer is wrong.
- **Yes/no fields:** true/false, yes/no, 1/0, verified/not verified all count when the meaning is clear.
- **Titles:** must match the title text; casing and punctuation don't matter, but missing or changed words do.
- When an answer is missing entirely, it is wrong.
- When you can't tell, mark it **unclear** and say why. Don't guess.

You may write a throwaway script to load the files and lay out the answers, and to mark answers that pass the strict scorer's rules as correct. Every other answer you judge yourself, by reading it. Don't use fuzzy matching or keyword rules to decide a grade.

## Output

1. `bench/r4/grades/gemma_semantic.json`: one entry per (format, dataset, seed, question number), with the question, the expected answer, the model's answer quoted exactly from the raw file, your grade (correct / wrong / unclear), the strict grade from the report, and a one-line reason whenever the two differ.
2. `research/r4-semantic-grade.md`:
   - accuracy per format by meaning, next to the strict accuracy, with counts;
   - the same per dataset and per question kind;
   - how many answers changed from wrong to correct, and from correct to wrong (if any), per format;
   - every changed grade listed: format, dataset, seed, question, expected, the model's exact answer, your reason;
   - the unclear ones listed separately;
   - plain description only: what happened, not why.

Check your totals: every format's correct + wrong + unclear must equal its question count in the strict report (240 for most formats, 120 for TSV and `key: value`).

## Final message

The per-format table (strict vs by meaning), the number of changed and unclear grades, and a plain-English summary (RULES rule 10). Don't commit or push.
