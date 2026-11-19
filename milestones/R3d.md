# R3d: real-data run 3, set up the way people actually run these models

Read `RULES.md`, then `SPEC.md` section 10, then `research/r3-real-run2.md` (section 8, Limits). Run2 was measured in a setup no real user has: a hand-built chat format with thinking prefills, a 1,500-token cap, a 4K context and default server flags. Run3 fixes that. Where something isn't answered here, stop and ask (rule 5). Make no changes beyond this brief.

**Don't touch `src/` or `tests/`**: another change is in progress there. Don't run `cargo test`. You may `cargo build --release --bins` to get `doctor_tool`. Leave `bench/r3/real/run1/` and `run2/` exactly as they are.

## 1. Server: the maintainer's own commands

Launch each model with `llama-server.exe` from `C:\Users\Ashman Das\Documents\turboquant-llama-cpp`, on `127.0.0.1:8090` (not 8080), one model at a time, following rule 7. Use exactly these flags, plus `--port 8090 --host 127.0.0.1`:

| Model | `-m` | Flags |
|---|---|---|
| Llama-3.2-3B | `models\lmstudio-community\Llama-3.2-3B-Instruct-GGUF\Llama-3.2-3B-Instruct-Q4_K_M.gguf` | `-ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 131072` |
| Gemma-4-E2B | `models\lmstudio-community\gemma-4-E2B-it-GGUF\gemma-4-E2B-it-Q4_K_M.gguf` | `-ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 131072` |
| LFM2.5-8B-A1B | `C:\Users\Ashman Das\Downloads\model file\LFM2.5-8B-A1B-Q4_K_M.gguf` | `-ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 128000` |
| Mellum2.1-12B-A2.5B-Thinking | `C:\Users\Ashman Das\Downloads\model file\Mellum2.1-12B-A2.5B-Thinking-Q4_K_M.gguf` | `-ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 128000 --n-cpu-moe 4` |

No other flags, except `--jinja` if this build doesn't turn it on by default (check `--help` and say which). If a model fails to load with these flags, stop and report the exact error; don't change the flags.

## 2. Requests: the way an app would send them

- Use `POST /v1/chat/completions` with a single user message, so each model's own chat template builds the prompt. **No prefills** and no hand-built prompt strings.
- **Thinking:** Gemma runs twice, with thinking off and with thinking on, set only through the template's own switch (`chat_template_kwargs`). Find the variable name in Gemma's embedded template, and quote the template lines in the report. LFM and Mellum run with their templates' defaults (thinking on), with nothing set.
- **No output cap:** don't send `max_tokens` or `n_predict`. The context size is the only limit. Record `finish_reason` for every run. As a safety net only, abort any single run after 20 minutes of wall-clock time and record it as `timeout`. That's not an output cap, and it should be rare.
- **Thinking text and token counts:** take the thinking text from the server's separate reasoning field (`reasoning_content`), not by cutting at tags. If the server returns thinking inline instead, say so and cut at the tag from R2 (Gemma `<channel|>`, others `</think>`). Take token counts from the response's `usage`. If `usage` doesn't split thinking from answer tokens, count the thinking text with `/tokenize` and say so.
- **Settings:** temperature 0.7, seeds 42 to 46, 5 runs per schema per model, the four `bench/r2/schemas`, no grammar. Everything else stays at the server's defaults.

## 3. The notes: natural sentences

Rewrite the task notes in `bench/r3/tasks.py` as plain English sentences. The most complex form allowed is this one: `Paper 2405.05254 claimed "Rope scaling hurts long ctx" in 2024; it turned out to be false, so the claim is refuted.`
- Every text value appears **inside double quotes, exactly as in the ground truth**.
- Numbers, enum labels and booleans are said in ordinary words, never as `field: value` labels. Each one must still be derivable without ambiguity: one reading only.
- Each fact is stated once. Don't list facts in schema order on purpose.
- Use the same records as run2 (5 per schema, from `bench/r1/fixtures/`). For the code schema, write 3 notes; the code goes in quotes, with real line breaks.
- The prompt is `prompt(schemas)`, then one blank line, then the notes, then `Write these as Struct records.`

Save the ground truth as before.

## 4. Collection: `bench/r3/real/run3/`

Use the same layout as run2: `raw/` (exact bytes; one thinking file per run, only for runs that think), `backup/`, `SHA256SUMS` (LF), `metadata.json` and `ground_truth.json`. Store the full request body and the full server response JSON for every run in `metadata.json`.

## 5. Report: `research/r3-real-run3.md`

Run the doctor in salvage mode. Per model × schema, report:
- record recall (records decoded / records expected);
- field accuracy, pooled over decoded records only, `n/a` if none decoded;
- outcome counts: clean, repaired, salvaged, partial, unusable;
- distinct outputs out of 5;
- `finish_reason` counts (incl. `length` and `timeout`);
- mean and max thinking tokens and output tokens.

Also include repairs by kind, bad lines by problem, and new kinds of mistake with real quotes. Add a short comparison with run2 that names the setup differences, and don't claim causes you didn't isolate. Put every prompt text, and the exact server command for each model, in the report.

## Done when
run3 is collected, hashed and reported; run1's and run2's hashes still match; every process you started is stopped.

## Final message
- The per-model × schema table;
- `finish_reason` counts and any timeouts;
- Gemma's thinking switch, with the template quote;
- new kinds of mistake;
- any questions (rule 5);
- the plain-English overview (rule 10).
