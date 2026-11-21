# R4 grading, Haiku: did the model actually get it right?

Read `milestones/RULES.md`, `milestones/R4.md`, `milestones/R4-haiku.md`, `milestones/R4-grade.md`, `bench/r4/questions.json` and `research/r4-retrieval-haiku.md`.

Do exactly what `R4-grade.md` says, with the same grading rules, but for Haiku's answers instead of Gemma's. Another agent is grading Gemma in parallel: don't touch its files (`bench/r4/grades/gemma_semantic.json`, `research/r4-semantic-grade.md`).

What changes from `R4-grade.md`:

- **Input:** the 96 Haiku raw answers, `bench/r4/real_haiku/raw/haiku_r4_{format}_{dataset}_seed{n}.txt`, seeds 1 to 4. The strict grades come from `research/r4-retrieval-haiku.md`.
- **Output:** `bench/r4/grades/haiku_semantic.json` and `research/r4-semantic-grade-haiku.md`, laid out the same way as the Gemma ones.
- **Totals check:** 192 questions per format, 96 for TSV and `key: value`.
- Haiku often adds a note in brackets after its answer, for example `2404.7570, 2411.9702 (verdict is "~")` or `No verdict given (the verdict field is ~)`. Grade the answer, not the note, unless the note changes or contradicts the answer. If the model corrects itself mid-answer ("wait, correction: ..."), grade the final answer it settles on, and say so in the reason.

**Don't touch** `bench/r4/real/`, `bench/r4/real_haiku/`, any `.py` file under `bench/r4/`, or any report other than your own. Don't call any model or start any server.

## Final message

The per-format table (strict vs by meaning), the number of changed and unclear grades, and a plain-English summary (RULES rule 10). Don't commit or push.
