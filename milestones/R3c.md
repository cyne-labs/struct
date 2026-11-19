# R3c: bring the doctor to the current spec, add the prompt, rerun real data

Read `RULES.md`, then `SPEC.md` sections 9 and 10. Both changed since R3b: the header-row fix was removed, and section 10 (the prompt) is new. Where SPEC.md doesn't answer something, stop and ask (rule 5). Make no changes beyond this brief. If you think something else needs changing, list it in your final message instead.

## Part A: code

1. **Remove the header-row fix.** The doctor now has exactly the 5 fixes in SPEC section 9. A line equal to the field names is read like any other line: kept if its values are valid, a bad line if not. Remove `SafeFix::HeaderRow`, its test and its damage kind, and add a test showing that `word|meaning|` under `{word, meaning}` is kept as a record.
2. **Add `prompt(schemas: &str) -> Result<String, StructError>`,** following SPEC section 10 exactly: the schema lines first, then the sections in the listed order, each included only under its stated condition, then the `Example: ` line. Add one test per conditional section, both included and left out, and byte-for-byte tests of the full prompt for each of the four schemas in `bench/r2/schemas/`.

## Part B: a real damage suite

Rebuild the damage suite in `tests/doctor_tests.rs` on top of the random file generator from the round-trip tests. Each case uses a **freshly generated random schema and multi-record file** (written with outside schemas), so the cases aren't variations of a fixed template.

- **Fixable damage:** each of the 5 fixes on its own, plus random combinations of 2 to 5 of them. A cut-off last record includes raw values cut off at the end of the file. Every case checks that the data equals the original, **and** that the reported repairs match the damage applied **exactly**: their kind, line numbers, and count.
- **Unfixable damage:** every kind in the R3b table, applied to a random line of a random file. Each case checks the exact bad line it expects (line number and problem), and that every value on every other line comes back byte-identical.
- **No damage:** zero repairs and zero bad lines.
- At least 5,000 cases in all. Keep the false-repair check from R3b. Print the per-kind table, and add a column counting the **distinct inputs**, to show the variety.

## Part C: harness and rerun

1. **`bench/r3/collect.py`:**
   - record hitting the token cap only from `stop_type == "limit"`. Keep `truncated` as its own separate field, since it means the prompt was cut;
   - don't write empty thinking files for models that don't think;
   - write to a **new run folder**, `bench/r3/real/run2/` (with `raw/`, `backup/` and `SHA256SUMS`). Move the old R3 data into `bench/r3/real/run1/` without changing a byte, and check its hashes before and after the move;
   - write `SHA256SUMS` with LF line endings.
2. **The prompt for each run** is exactly `prompt(schemas)`, then one blank line, then the task. The task gives the model **real source notes**: plain English sentences describing the records to write, taken from the R1 datasets in `bench/r1/fixtures/` (use 5 records per schema). End with `Write these as Struct records.` For the two-class code schema, write 3 short notes describing a file and a code patch.
   - Keep the exact data behind every note as the **ground truth**, saved next to the notes, so accuracy can be scored later.
   - Put every prompt text in the report.
3. **Models:**
   - **thinking off:** `Llama-3.2-3B-Instruct-Q4_K_M.gguf` and `gemma-4-E2B-it-Q4_K_M.gguf`;
   - **thinking on:** `gemma-4-E2B-it-Q4_K_M.gguf`, `LFM2.5-8B-A1B-Q4_K_M.gguf` and `Mellum2.1-12B-A2.5B-Thinking-Q4_K_M.gguf`.

   LFM runs **with thinking on only**: its template has no clean way to turn thinking off. That's a maintainer decision; don't prefill anything. Each model's end-of-thinking tag is as found in R2: Gemma uses `<channel|>`, and the others use `</think>`.
4. **Run settings:** no grammar; 5 runs per schema per model; temperature 0.7; seeds 42 to 46; a 1,500-token cap. Follow rule 7 for the GPU.
5. **Run the doctor** in salvage mode, and report per run:
   - whether it came out clean as written, repaired, salvaged (with how many bad lines), or unusable;
   - the repairs by kind, and the bad lines by problem;
   - thinking tokens and output tokens;
   - whether it hit the cap;
   - **field accuracy against the ground truth:** the share of fields exactly right among the records that decoded.

   List any new kinds of mistake, with real quotes. Write all of this to `research/r3-real-run2.md`.

## Done when
`cargo test --offline` passes; the damage suite shows real variety and 0 false repairs; run1's hashes still match after the move; run2 is collected, hashed and reported; every process you started is stopped.

## Final message
- Test counts and the per-kind table, exactly as printed;
- the run2 table, per model and per schema;
- any new kinds of mistake;
- any SPEC questions (rule 5);
- the plain-English overview (rule 10).
