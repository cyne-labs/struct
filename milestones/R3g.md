# R3g: regenerate the run3 report

Read `RULES.md`, then `SPEC.md` section 9 (the simple doctor), then `milestones/R3d.md` and `research/r3-real-run3.md`. You are a fresh session: you didn't write that report. Treat it as someone else's work and check it.

Don't run any model or `llama-server`. Don't change anything under `bench/r3/real/`; all of it is hashed. Don't touch `src/` or `tests/`. Rule 5 applies.

## What to do

1. **Build the current doctor** (`cargo build --release --bins`). Re-score every run3 output with it, in salvage mode. Use the release build and say its file time.
2. **Field accuracy stays positional:** record 1 against ground truth 1, record 2 against ground truth 2, and so on. A missing, extra or out-of-order record counts as wrong. Don't match records by id or by any other key. The test must be honest, not generous.
3. **Rewrite `research/r3-real-run3.md`** with the new numbers:
   - **Report only what was observed.** Remove every causal claim that wasn't isolated, for example "natural sentences prevented quote copying", "the uncapped budget resolved truncation" and "thinking improves parsing". Run2 and run3 differ in many ways at once.
   - **Gemma "thinking on":** list the runs where it produced no reasoning at all (an empty or missing `reasoning_content`). Show them as their own row, so they don't change the thinking-on averages.
   - **Token counts:** say that they came from `/tokenize` on `reasoning_content` and `content`. Compare them with the server's `usage.completion_tokens` for every run, and give the largest difference.
   - **s4:** say plainly which classes decoded. If only `file` records decoded, the field accuracy covers only those.
   - **Questions:** remove the two maintainer questions from the old report, the class-marker regex and dropping the trailing `~`. The maintainer rejected both. Keep the new-mistake list with its real quotes.
   - Keep the setup section (the exact server commands and every prompt text).
4. **Check the old report against yours.** List every number or claim that changed, as old → new.

## Done when
The report is regenerated with the current doctor; the hashes for run1, run2 and run3 still match; nothing under `bench/r3/real/` changed.

## Final message
- The new per-model × schema table;
- the old → new list;
- any questions (rule 5);
- the plain-English overview (rule 10).
