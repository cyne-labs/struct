# Rules for every milestone

These apply to any agent working a milestone in this repo. Read them together with `SPEC.md` and the milestone brief before starting.

1. **Stay inside this repo:** `C:\Users\Ashman Das\Documents\Struct`. Other folders may be read, but only where a brief names them:
   - the old Python reference at `C:\Users\Ashman Das\Documents\struct-py`;
   - the llama.cpp build at `C:\Users\Ashman Das\Documents\turboquant-llama-cpp`;
   - model files under `...\turboquant-llama-cpp\models\` and `C:\Users\Ashman Das\Downloads\model file\`.
   Never modify anything outside this repo.
2. **Languages.** Struct itself is **Rust, using only the standard library**: no crates, so the build works offline. Benchmark and measurement helpers are **Python**, standard library plus whatever is already installed.
3. **Download nothing:** no crates, pip packages, models, datasets or toolchains. Reading web pages is fine. If something essential is missing, stop and say exactly what.
4. **Never run `git commit` or `git push`.** The reviewer commits after the maintainer has seen the work.
5. **Make no design decisions.** `SPEC.md` is the only authority on how Struct behaves. If it's ambiguous, incomplete or contradictory for anything you need, **don't pick an interpretation**. Stop and list the open questions in your final message, with the options for each. Only the maintainer decides.
6. **Disk:** C: has only a few GB above the floor. Before any large write, check free space. If it would drop below 25 GB, stop and report. Never copy model files. Keep Rust build output inside this repo's `target/` folder.
7. **GPU:** one heavy job at a time on an 8 GB card. Check `nvidia-smi` first, and if something else holds more than 1 GB, stop and report. Bind any server to `127.0.0.1` on port 8090. Stop every process you start, and confirm before finishing.
8. **Production quality:** no stubs, no TODO placeholders, no `...` inside specs or schemas.
9. **Honesty:** report every number as measured, including losses. Never call something passed by picking the favourable subset; say exactly which cases pass and which fail.
10. **Plain-English transparency overview, required in the final message.** The maintainer wants to understand all of the work. Write it in plain English: technical detail is welcome, jargon isn't, and don't cite line numbers. Include:
    - every file you created or changed, with what it does;
    - what the test data looks like, with a small real sample;
    - how each test or measurement works, step by step;
    - what the results mean, and what's still unverified;
    - every place you were unsure, and how you handled it (rule 5).
