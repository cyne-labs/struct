# Fix: bugs in the R4 Gemma harness

Read `milestones/RULES.md`, `milestones/R4.md` (all decisions, including 14), `bench/r4/collect.py`, `bench/r4/evaluate.py`, `bench/r4/launch_server.py`, `bench/r4/test_offline.py` and `research/r4-retrieval.md`.

The Gemma run is finished: 120/120 requests in `bench/r4/real/`. **No server is running.** Don't start one, and don't call any model.

**Don't touch** anything in `bench/r4/real/`, `bench/r4/real_haiku/`, `bench/r4/grades/`, `questions.json`, `data/`, `formats/`, or any `*_haiku*` file. A Haiku run is using `collect.py`'s functions right now, so don't change what `build_prompt`, `ENCODERS` or `load_questions` return. Don't change the scoring rules: they are maintainer decisions. A separate grader is working in parallel; don't touch its files.

## Known bugs (mechanical fixes)

1. **Broken prompt block in the report.** Section 2 wraps the example prompt in a ```` ```text ```` fence, but the prompt itself contains a ```` ```struct ```` fence, so the markdown breaks. Use a longer outer fence (four backticks or more), so the prompt shows exactly as sent, byte for byte.
2. **GPU check silently passes.** On this laptop `nvidia-smi --query-compute-apps=...,used_memory` prints `[N/A]` for memory. `int("[N/A]")` throws, the `except` prints a note and returns True, so the RULES rule 7 check never runs. Fix:
   - in `launch_server.py`, before starting, read the GPU's total used memory (`nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits`) and stop if more than 1 GB is in use; if even that can't be read, stop with a clear message rather than pass silently;
   - in `collect.py`, the check runs while our own server is already up, so it can't use the same rule. Make it check only that the process holding the GPU is `llama-server`, and print what it found. Never pass silently on a parse error.
3. **Find the rest.** Read the harness and the report for other bugs: wrong numbers, mislabelled tables, anything the report says that the data doesn't show. Fix the ones that are plainly mechanical. List everything else as a question with options (RULES rule 5); don't decide it.

## After fixing

- Regenerate `research/r4-retrieval.md` from the existing data with `python bench/r4/evaluate.py` (it needs no server). Check that every number left in it is unchanged except where a fix explains why.
- Add tests for each fix to `test_offline.py`. All existing tests must still pass, plus `test_haiku_offline.py`.

## Final message

Each bug, what you changed, the test counts, the questions you left for the maintainer, and a plain-English summary (RULES rule 10). Don't commit or push.
