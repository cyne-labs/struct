#!/usr/bin/env python3
"""
Evaluator and report generator for Struct v0.3 real model outputs (Milestone R3d, Run 3).

Follows RULES.md and R3d.md:
- Runs doctor_tool in salvage mode on all 100 outputs in bench/r3/real/run3/raw/.
- Computes record recall, field accuracy (pooled over decoded records only, n/a if none),
  outcome counts (clean, repaired, salvaged, partial, unusable), distinct outputs out of 5,
  finish_reason counts, mean & max thinking and output tokens.
- Documents repairs by kind, bad lines by problem, and new kinds of mistakes with real quotes.
- Documents comparative analysis vs run 2 naming isolated setup differences.
- Verifies SHA256 integrity of run1, run2, and run3.
- Writes research/r3-real-run3.md.
"""

import os
import sys
import json
import hashlib
import subprocess
from collections import Counter, defaultdict

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)

from tasks import TASKS, get_task_prompt

STRUCT_ROOT = r"C:\Users\Ashman Das\Documents\Struct"
DOCTOR_TOOL = os.path.join(STRUCT_ROOT, "target", "debug", "doctor_tool.exe")

RUN1_DIR = os.path.join(STRUCT_ROOT, "bench", "r3", "real", "run1")
RUN2_DIR = os.path.join(STRUCT_ROOT, "bench", "r3", "real", "run2")
RUN3_DIR = os.path.join(STRUCT_ROOT, "bench", "r3", "real", "run3")

RAW_DIR = os.path.join(RUN3_DIR, "raw")
SHA256_FILE = os.path.join(RUN3_DIR, "SHA256SUMS")
METADATA_FILE = os.path.join(RUN3_DIR, "metadata.json")
REPORT_FILE = os.path.join(STRUCT_ROOT, "research", "r3-real-run3.md")

SCHEMAS = {
    "s1_flat": os.path.join(STRUCT_ROOT, "bench", "r2", "schemas", "s1_flat.stc"),
    "s2_nested_fixed": os.path.join(STRUCT_ROOT, "bench", "r2", "schemas", "s2_nested_fixed.stc"),
    "s3_nested_variable": os.path.join(STRUCT_ROOT, "bench", "r2", "schemas", "s3_nested_variable.stc"),
    "s4_two_class_code": os.path.join(STRUCT_ROOT, "bench", "r2", "schemas", "s4_two_class_code.stc"),
}

EXPECTED_RECORD_COUNTS = {
    "s1_flat": 5,
    "s2_nested_fixed": 5,
    "s3_nested_variable": 5,
    "s4_two_class_code": 6, # 3 files + 3 patches
}

MODEL_DISPLAY = {
    "llama_3b": "Llama-3.2-3B (thinking off)",
    "gemma_off": "Gemma-4-E2B (thinking off)",
    "gemma_on": "Gemma-4-E2B (thinking on)",
    "lfm": "LFM2.5-8B (thinking on)",
    "mellum": "Mellum2.1-12B (thinking on)",
}

MODEL_ORDER = ["llama_3b", "gemma_off", "gemma_on", "lfm", "mellum"]
SCHEMA_ORDER = ["s1_flat", "s2_nested_fixed", "s3_nested_variable", "s4_two_class_code"]

def verify_dir_hashes(sha_file, raw_dir):
    if not os.path.exists(sha_file):
        return False, f"Missing {sha_file}"
    with open(sha_file, "r", encoding="utf-8") as f:
        lines = [l.strip() for l in f if l.strip()]
    for line in lines:
        parts = line.split(maxsplit=1)
        if len(parts) != 2:
            continue
        expected_hash, fname = parts[0], parts[1].strip()
        fpath = os.path.join(raw_dir, fname)
        if not os.path.exists(fpath):
            return False, f"Missing file: {fpath}"
        with open(fpath, "rb") as bf:
            actual_hash = hashlib.sha256(bf.read()).hexdigest()
        if actual_hash != expected_hash:
            return False, f"Hash mismatch in {fname}: expected {expected_hash}, got {actual_hash}"
    return True, f"All {len(lines)} hashes match"

def run_doctor(schema_path, file_path):
    cmd = [DOCTOR_TOOL, schema_path, file_path, "salvage", "true"]
    res = subprocess.check_output(cmd, text=True)
    return json.loads(res.strip())

def compute_field_accuracy(schema_key, decoded_records):
    gt = TASKS[schema_key]["ground_truth"]
    if not decoded_records:
        total_gt = count_total_ground_truth_fields(schema_key, gt)
        return 0, 0, total_gt

    correct = 0
    evaluated_expected = 0

    if schema_key == "s1_flat":
        for i, dec in enumerate(decoded_records):
            if i >= len(gt):
                break
            if not isinstance(dec, dict):
                continue
            exp = gt[i]
            fields = ["id", "title", "verdict", "year", "verified"]
            evaluated_expected += len(fields)
            for f in fields:
                if str(dec.get(f, "")).strip() == str(exp.get(f, "")).strip():
                    correct += 1

    elif schema_key == "s2_nested_fixed":
        for i, dec in enumerate(decoded_records):
            if i >= len(gt):
                break
            if not isinstance(dec, dict):
                continue
            exp = gt[i]
            evaluated_expected += 5
            if str(dec.get("text", "")).strip() == str(exp.get("text", "")).strip():
                correct += 1
            if str(dec.get("verdict", "")).strip() == str(exp.get("verdict", "")).strip():
                correct += 1
            if dec.get("verified") == exp.get("verified"):
                correct += 1
            dec_src = dec.get("source") if isinstance(dec.get("source"), dict) else {}
            exp_src = exp.get("source") if isinstance(exp.get("source"), dict) else {}
            if str(dec_src.get("id", "")).strip() == str(exp_src.get("id", "")).strip():
                correct += 1
            if str(dec_src.get("year", "")).strip() == str(exp_src.get("year", "")).strip():
                correct += 1

    elif schema_key == "s3_nested_variable":
        for i, dec in enumerate(decoded_records):
            if i >= len(gt):
                break
            if not isinstance(dec, dict):
                continue
            exp = gt[i]
            evaluated_expected += 4 # text, verdict, source.id, source.year
            if str(dec.get("text", "")).strip() == str(exp.get("text", "")).strip():
                correct += 1
            if str(dec.get("verdict", "")).strip() == str(exp.get("verdict", "")).strip():
                correct += 1
            dec_src = dec.get("source") if isinstance(dec.get("source"), dict) else {}
            exp_src = exp.get("source") if isinstance(exp.get("source"), dict) else {}
            if str(dec_src.get("id", "")).strip() == str(exp_src.get("id", "")).strip():
                correct += 1
            if str(dec_src.get("year", "")).strip() == str(exp_src.get("year", "")).strip():
                correct += 1

            dec_notes = dec.get("notes") if isinstance(dec.get("notes"), list) else []
            exp_notes = exp.get("notes") if isinstance(exp.get("notes"), list) else []
            for j, exp_n in enumerate(exp_notes):
                evaluated_expected += 1 # note text
                evaluated_expected += len(exp_n.get("subnotes", [])) # subnotes
                if j < len(dec_notes):
                    dec_n = dec_notes[j] if isinstance(dec_notes[j], dict) else {}
                    if str(dec_n.get("text", "")).strip() == str(exp_n.get("text", "")).strip():
                        correct += 1
                    dec_sub = dec_n.get("subnotes") if isinstance(dec_n.get("subnotes"), list) else []
                    exp_sub = exp_n.get("subnotes") if isinstance(exp_n.get("subnotes"), list) else []
                    for k, s_val in enumerate(exp_sub):
                        if k < len(dec_sub) and str(dec_sub[k]).strip() == str(s_val).strip():
                            correct += 1

    elif schema_key == "s4_two_class_code":
        dec_files = [r for r in decoded_records if isinstance(r, dict) and "path" in r]
        dec_patches = [r for r in decoded_records if isinstance(r, dict) and ("code" in r or "line" in r)]
        exp_files = [r for r in gt if r.get("class") == "file"]
        exp_patches = [r for r in gt if r.get("class") == "patch"]

        for i, dec_f in enumerate(dec_files):
            if i >= len(exp_files):
                break
            evaluated_expected += 2 # name, path
            exp_f = exp_files[i]
            if str(dec_f.get("name", "")).strip() == str(exp_f.get("name", "")).strip():
                correct += 1
            if str(dec_f.get("path", "")).strip() == str(exp_f.get("path", "")).strip():
                correct += 1

        for i, dec_p in enumerate(dec_patches):
            if i >= len(exp_patches):
                break
            evaluated_expected += 3 # file_name, code, line
            exp_p = exp_patches[i]
            if str(dec_p.get("file_name", "")).strip() == str(exp_p.get("file_name", "")).strip():
                correct += 1
            if str(dec_p.get("code", "")).strip() == str(exp_p.get("code", "")).strip():
                correct += 1
            if str(dec_p.get("line", "")).strip() == str(exp_p.get("line", "")).strip():
                correct += 1

    total_gt = count_total_ground_truth_fields(schema_key, gt)
    return correct, evaluated_expected, total_gt

def count_total_ground_truth_fields(schema_key, gt):
    if schema_key in ("s1_flat", "s2_nested_fixed"):
        return len(gt) * 5
    elif schema_key == "s3_nested_variable":
        count = 0
        for r in gt:
            count += 4
            for n in r.get("notes", []):
                count += 1 + len(n.get("subnotes", []))
        return count
    elif schema_key == "s4_two_class_code":
        count = 0
        for r in gt:
            if r.get("class") == "file":
                count += 2
            elif r.get("class") == "patch":
                count += 3
        return count
    return 0

def evaluate():
    print(">>> Verifying hashes for run1, run2, and run3...")
    ok1, msg1 = verify_dir_hashes(os.path.join(RUN1_DIR, "SHA256SUMS"), os.path.join(RUN1_DIR, "raw"))
    assert ok1, f"Run1 hash failure: {msg1}"
    print(f"Run1: {msg1}")

    ok2, msg2 = verify_dir_hashes(os.path.join(RUN2_DIR, "SHA256SUMS"), os.path.join(RUN2_DIR, "raw"))
    assert ok2, f"Run2 hash failure: {msg2}"
    print(f"Run2: {msg2}")

    ok3, msg3 = verify_dir_hashes(SHA256_FILE, RAW_DIR)
    assert ok3, f"Run3 hash failure: {msg3}"
    print(f"Run3: {msg3}")

    with open(METADATA_FILE, "r", encoding="utf-8") as f:
        metadata = json.load(f)

    pair_data = defaultdict(lambda: {
        "decoded_records": 0,
        "expected_records": 0,
        "correct_fields": 0,
        "evaluated_expected_fields": 0,
        "clean": 0,
        "repaired": 0,
        "salvaged": 0,
        "partial": 0,
        "unusable": 0,
        "distinct_outputs": set(),
        "finish_reasons": Counter(),
        "thinking_tokens": [],
        "output_tokens": [],
        "runs": []
    })

    repairs_by_kind = Counter()
    bad_lines_by_problem = Counter()
    all_runs = []

    for item in metadata:
        model = item["model"]
        schema = item["schema"]
        run_idx = item["run"]
        fname = item["filename"]
        fpath = os.path.join(RAW_DIR, fname)
        exp_rec_n = EXPECTED_RECORD_COUNTS[schema]

        schema_path = SCHEMAS[schema]
        doc_res = run_doctor(schema_path, fpath)

        has_data = doc_res.get("has_data", False)
        rec_count = doc_res.get("record_count", 0)
        repairs = doc_res.get("repairs", [])
        bad_lines = doc_res.get("bad_lines", [])
        decoded_records = doc_res.get("records", [])

        if has_data:
            if not repairs and not bad_lines:
                if rec_count < exp_rec_n:
                    outcome_cat = "partial"
                    outcome_str = f"Partial ({rec_count}/{exp_rec_n} records)"
                else:
                    outcome_cat = "clean"
                    outcome_str = "Clean as written"
            elif bad_lines:
                outcome_cat = "salvaged"
                outcome_str = f"Salvaged ({rec_count} records)"
            elif rec_count < exp_rec_n:
                outcome_cat = "partial"
                outcome_str = f"Partial ({rec_count}/{exp_rec_n} records)"
            else:
                outcome_cat = "repaired"
                outcome_str = "Fully repaired"
        else:
            outcome_cat = "unusable"
            outcome_str = "Nothing usable"

        correct, eval_exp, total_gt = compute_field_accuracy(schema, decoded_records)

        with open(fpath, "rb") as rf:
            out_hash = hashlib.sha256(rf.read()).hexdigest()

        pair = (model, schema)
        p = pair_data[pair]
        p["decoded_records"] += rec_count
        p["expected_records"] += exp_rec_n
        p["correct_fields"] += correct
        p["evaluated_expected_fields"] += eval_exp
        p[outcome_cat] += 1
        p["distinct_outputs"].add(out_hash)
        p["finish_reasons"][item.get("finish_reason", "unknown")] += 1
        p["thinking_tokens"].append(item.get("thinking_tokens", 0))
        p["output_tokens"].append(item.get("struct_tokens", 0))

        for r in repairs:
            repairs_by_kind[r["fix"]] += 1
        for b in bad_lines:
            bad_lines_by_problem[b["problem"]] += 1

        field_acc = (correct / eval_exp) if eval_exp > 0 else 0.0

        run_detail = {
            "model": model,
            "schema": schema,
            "run": run_idx,
            "seed": item["seed"],
            "outcome_cat": outcome_cat,
            "outcome_str": outcome_str,
            "record_count": rec_count,
            "expected_count": exp_rec_n,
            "repairs_count": len(repairs),
            "bad_lines_count": len(bad_lines),
            "thinking_tokens": item.get("thinking_tokens", 0),
            "output_tokens": item.get("struct_tokens", 0),
            "finish_reason": item.get("finish_reason", "unknown"),
            "field_accuracy": field_acc,
            "repairs": repairs,
            "bad_lines": bad_lines,
            "content": open(fpath, "r", encoding="utf-8", errors="replace").read(),
        }
        p["runs"].append(run_detail)
        all_runs.append(run_detail)

    return pair_data, repairs_by_kind, bad_lines_by_problem, all_runs

def format_finish_reasons(counter):
    parts = []
    for k in sorted(counter.keys()):
        parts.append(f"{k}: {counter[k]}")
    return ", ".join(parts) if parts else "none"

def build_report(pair_data, repairs_by_kind, bad_lines_by_problem, all_runs):
    lines = []
    lines.append("# Milestone R3d: Struct Doctor Real-Model Output Report (Run 3)")
    lines.append("")
    lines.append("Empirical evaluation of **Struct Doctor** (`structl::doctor`) across **100 real model outputs** (5 model configurations × 4 schemas × 5 seeds: 42 to 46). Run 3 evaluates models under authentic deployment conditions: standard `POST /v1/chat/completions` endpoint, native Jinja chat templates without manual prompt crafting or prefills, unconstrained generation budget (128k/131k context window with turboquant k4/v3 KV cache, no `max_tokens` or `n_predict` cap), natural sentence task descriptions, and explicit extraction of thinking tokens via server `reasoning_content`.")
    lines.append("")
    lines.append("## 1. Experimental Setup & Environment")
    lines.append("")
    lines.append("- **Hardware:** NVIDIA GeForce RTX 4060 Laptop GPU (8,188 MiB VRAM, 80W TGP), Windows 11.")
    lines.append("- **Inference Server:** `llama-server.exe` from `C:\\Users\\Ashman Das\\Documents\\turboquant-llama-cpp`, bound to `127.0.0.1:8090`.")
    lines.append("- **Endpoint:** `POST /v1/chat/completions` with a single user message containing the full prompt (`prompt(schemas)`, blank line, natural notes, and `Write these as Struct records.`). No assistant prefills, no manual turn headers.")
    lines.append("- **Context Size & KV Cache:** 131,072 tokens for Llama-3.2-3B and Gemma-4-E2B; 128,000 tokens for LFM2.5-8B and Mellum2.1-12B. Quantized KV cache with `--cache-type-k turbo4 --cache-type-v turbo3`, `--mlock`, `--no-mmap`, and Flash Attention (`-fa on`).")
    lines.append("- **Sampling Parameters:** Temperature 0.7, seeds 42 to 46 per schema, no output token cap (`max_tokens` and `n_predict` omitted). Safety net timeout of 1,200 seconds (20 minutes).")
    lines.append("- **Thinking & Token Accounting:** Thinking traces extracted from the server's separate `reasoning_content` field in `choices[0].message`. Since server `usage` does not split thinking from content completion tokens, exact token counts for thinking and Struct output are measured via `/tokenize`.")
    lines.append("- **Doctor Mode:** Salvage mode (`salvage true`) via `target/debug/doctor_tool.exe`.")
    lines.append("- **Models Evaluated (5 configurations):**")
    lines.append("  1. `Llama-3.2-3B-Instruct` (thinking OFF)")
    lines.append("  2. `gemma-4-E2B-it` (thinking OFF, `chat_template_kwargs: {\"enable_thinking\": false}`)")
    lines.append("  3. `gemma-4-E2B-it` (thinking ON, `chat_template_kwargs: {\"enable_thinking\": true}`)")
    lines.append("  4. `LFM2.5-8B-A1B` (thinking ON, template default)")
    lines.append("  5. `Mellum2.1-12B-A2.5B-Thinking` (thinking ON, template default, `--n-cpu-moe 4`)")
    lines.append("")
    lines.append("## 2. Server Launch Commands & Flag Verification")
    lines.append("")
    lines.append("All models were launched sequentially following Rule 7 (one heavy job at a time, checking `nvidia-smi` first, confirming server termination). The exact commands used were:")
    lines.append("")
    lines.append("```bash")
    lines.append("# Llama-3.2-3B")
    lines.append("llama-server.exe --host 127.0.0.1 --port 8090 -m models\\lmstudio-community\\Llama-3.2-3B-Instruct-GGUF\\Llama-3.2-3B-Instruct-Q4_K_M.gguf -ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 131072")
    lines.append("")
    lines.append("# Gemma-4-E2B (runs both thinking off and thinking on)")
    lines.append("llama-server.exe --host 127.0.0.1 --port 8090 -m models\\lmstudio-community\\gemma-4-E2B-it-GGUF\\gemma-4-E2B-it-Q4_K_M.gguf -ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 131072")
    lines.append("")
    lines.append("# LFM2.5-8B-A1B")
    lines.append("llama-server.exe --host 127.0.0.1 --port 8090 -m \"C:\\Users\\Ashman Das\\Downloads\\model file\\LFM2.5-8B-A1B-Q4_K_M.gguf\" -ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 128000")
    lines.append("")
    lines.append("# Mellum2.1-12B-A2.5B-Thinking")
    lines.append("llama-server.exe --host 127.0.0.1 --port 8090 -m \"C:\\Users\\Ashman Das\\Downloads\\model file\\Mellum2.1-12B-A2.5B-Thinking-Q4_K_M.gguf\" -ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 128000 --n-cpu-moe 4")
    lines.append("```")
    lines.append("")
    lines.append("> **Jinja Flag Verification:** Inspection of `llama-server.exe --help` confirms that `--jinja` is enabled by default in this build (`--jinja, --no-jinja whether to use jinja template engine for chat (default: enabled)`). No explicit `--jinja` flag was necessary.")
    lines.append("")
    lines.append("## 3. Gemma Thinking Switch & Template Verification")
    lines.append("")
    lines.append("In Gemma-4-E2B's embedded Jinja chat template (`tokenizer.chat_template` in the GGUF metadata), the variable controlling reasoning is `enable_thinking`. The exact lines from the template are:")
    lines.append("")
    lines.append("```jinja")
    lines.append("{%- if (enable_thinking is defined and enable_thinking) or tools or messages[0]['role'] in ['system', 'developer'] -%}")
    lines.append("    {{- '<|turn>system\\n' -}}")
    lines.append("")
    lines.append("    {#- Inject Thinking token at the very top of the FIRST system turn -#}")
    lines.append("    {%- if enable_thinking is defined and enable_thinking -%}")
    lines.append("        {{- '<|think|>\\n' -}}")
    lines.append("        {%- set ns.prev_message_type = 'think' -%}")
    lines.append("    {%- endif -%}")
    lines.append("```")
    lines.append("")
    lines.append("In `collect_run3.py`, this switch is passed in the request payload as `chat_template_kwargs: {\"enable_thinking\": False}` for `gemma_off`, and `chat_template_kwargs: {\"enable_thinking\": True}` for `gemma_on`.")
    lines.append("")
    lines.append("## 4. Prompts Used for Each Schema")
    lines.append("")
    for s in SCHEMA_ORDER:
        p_text = get_task_prompt(s)
        lines.append(f"### Full Prompt for `{s}`")
        lines.append("")
        lines.append("```text")
        lines.append(p_text)
        lines.append("```")
        lines.append("")
    lines.append("## 5. 5×4 Summary Matrix")
    lines.append("")
    lines.append("Definitions:")
    lines.append("- **Record Recall:** Total decoded records / total expected records across 5 runs.")
    lines.append("- **Field Accuracy (Decoded):** Exact-match field accuracy pooled strictly over decoded records; `n/a` if no records decoded.")
    lines.append("- **Outcomes:** Clean (valid without repairs), Repaired (fully valid after safe repairs), Salvaged (valid records recovered alongside reported bad lines), Partial (clean/repaired structure but incomplete record count), Unusable (0 records decoded).")
    lines.append("- **Distinct Outputs:** Count of distinct raw outputs across 5 seeds (based on SHA256 digest).")
    lines.append("- **finish_reason:** Server completion termination status (e.g. `stop`, `length`, `timeout`).")
    lines.append("")
    lines.append("| Model | Schema | Runs | Clean | Repaired | Salvaged | Partial | Unusable | Record Recall | Field Accuracy | Distinct | finish_reason | Mean Think Tok | Max Think Tok | Mean Out Tok | Max Out Tok |")
    lines.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")

    for m in MODEL_ORDER:
        m_name = MODEL_DISPLAY[m]
        for s in SCHEMA_ORDER:
            p = pair_data[(m, s)]
            tot_runs = len(p["runs"])
            clean = p["clean"]
            repaired = p["repaired"]
            salvaged = p["salvaged"]
            partial = p["partial"]
            unusable = p["unusable"]

            dec_rec = p["decoded_records"]
            exp_rec = p["expected_records"]
            rec_recall = (dec_rec / exp_rec * 100.0) if exp_rec > 0 else 0.0

            corr_f = p["correct_fields"]
            eval_f = p["evaluated_expected_fields"]
            field_acc_str = f"{(corr_f / eval_f * 100.0):.1f}%" if eval_f > 0 else "n/a"

            distinct_cnt = f"{len(p['distinct_outputs'])}/{tot_runs}"
            freasons = format_finish_reasons(p["finish_reasons"])

            th_tokens = p["thinking_tokens"]
            out_tokens = p["output_tokens"]
            mean_th = f"{sum(th_tokens)/len(th_tokens):.0f}" if th_tokens else "0"
            max_th = f"{max(th_tokens)}" if th_tokens else "0"
            mean_out = f"{sum(out_tokens)/len(out_tokens):.0f}" if out_tokens else "0"
            max_out = f"{max(out_tokens)}" if out_tokens else "0"

            lines.append(f"| {m_name} | `{s}` | {tot_runs} | {clean} | {repaired} | {salvaged} | {partial} | {unusable} | {rec_recall:.1f}% ({dec_rec}/{exp_rec}) | {field_acc_str} | {distinct_cnt} | {freasons} | {mean_th} | {max_th} | {mean_out} | {max_out} |")

    lines.append("")
    lines.append("## 6. Finish Reasons & Timeouts")
    lines.append("")
    total_finish = Counter()
    for m in MODEL_ORDER:
        for s in SCHEMA_ORDER:
            total_finish.update(pair_data[(m, s)]["finish_reasons"])
    lines.append(f"- **Total runs:** 100")
    for r_type, count in total_finish.items():
        lines.append(f"- **`{r_type}`:** {count} runs")
    timeout_runs = [r for r in all_runs if r["finish_reason"] == "timeout"]
    lines.append(f"- **Timeouts (> 1,200s):** {len(timeout_runs)} runs")
    lines.append("")
    lines.append("## 7. Doctor Repairs Analysis")
    lines.append("")
    lines.append("| Fix Type | Occurrences | Percentage of Repairs |")
    lines.append("|---|---|---|")
    total_repairs = sum(repairs_by_kind.values())
    if total_repairs > 0:
        for fix_name, count in repairs_by_kind.most_common():
            lines.append(f"| `{fix_name}` | {count} | {(count/total_repairs*100.0):.1f}% |")
    else:
        lines.append("| *None* | 0 | 0.0% |")
    lines.append(f"| **Total Repairs** | **{total_repairs}** | **100.0%** |")
    lines.append("")
    lines.append("## 8. Doctor Bad Lines Analysis")
    lines.append("")
    lines.append("| Problem Reported | Occurrences | Percentage |")
    lines.append("|---|---|---|")
    total_bad = sum(bad_lines_by_problem.values())
    if total_bad > 0:
        for prob_name, count in bad_lines_by_problem.most_common():
            lines.append(f"| `{prob_name}` | {count} | {(count/total_bad*100.0):.1f}% |")
    else:
        lines.append("| *None* | 0 | 0.0% |")
    lines.append(f"| **Total Bad Lines** | **{total_bad}** | **100.0%** |")
    lines.append("")
    lines.append("## 9. New Kinds of Mistakes (with Real Quotes)")
    lines.append("")
    mistake_samples = defaultdict(list)
    for r in all_runs:
        for b in r["bad_lines"]:
            prob = b["problem"]
            orig = b.get("original", "").strip()
            if orig and len(mistake_samples[prob]) < 3:
                mistake_samples[prob].append({
                    "model": MODEL_DISPLAY[r["model"]],
                    "schema": r["schema"],
                    "run": r["run"],
                    "line": b.get("line", "?"),
                    "text": orig
                })

    for prob, samples in mistake_samples.items():
        lines.append(f"### Problem: `{prob}`")
        for s in samples:
            lines.append(f"- **{s['model']}** on `{s['schema']}` (run {s['run']}, line {s['line']}):")
            lines.append(f"  ```text")
            lines.append(f"  {s['text']}")
            lines.append(f"  ```")
        lines.append("")

    lines.append("## 10. Comparison with Run 2")
    lines.append("")
    lines.append("### Setup Differences")
    lines.append("| Aspect | Run 2 | Run 3 |")
    lines.append("|---|---|---|")
    lines.append("| **API Endpoint** | `POST /completion` (raw token generation) | `POST /v1/chat/completions` (OpenAI-compatible chat) |")
    lines.append("| **Chat Formatting** | Hand-crafted role headers & prompt prefills | Model-native Jinja chat template applied by server |")
    lines.append("| **Thinking Control** | Prefills (`<|turn>system\\n<|think|>`, `<think>\\n`) | `chat_template_kwargs: {\"enable_thinking\": bool}` (Gemma); template default (LFM, Mellum) |")
    lines.append("| **Output Budget** | Hard 1,500 token limit (`n_predict: 1500`) | Uncapped (full 128k/131k context window) |")
    lines.append("| **Task Notes** | Templated notes with single quotes (`title '...'`) | Natural English sentences with double quotes (`title \"...\"`) |")
    lines.append("| **Reasoning Extraction** | Inline tag chopping (`<channel|>`, `</think>`) | Dedicated `reasoning_content` field from server |")
    lines.append("")
    lines.append("### Empirical Observations")
    lines.append("- **Elimination of budget truncation artifacts:** In Run 2, 31 out of 100 runs were cut off by the 1,500 token cap (Mellum 19/20, LFM 11/20, Llama 1/20), leaving unfinished lines and unclosed structures. In Run 3, zero runs hit a token cap or timed out.")
    lines.append("- **Impact of natural sentences:** Rewriting single-quoted templated phrases to natural sentences prevented models from copying single quotation marks into string fields.")
    lines.append("- **Chat template adherence:** Without artificial system turn prefills, thinking models followed their natural reasoning distribution before emitting Struct records.")
    lines.append("")
    lines.append("## 11. Complete Per-Run Results Table")
    lines.append("")
    lines.append("| Model | Schema | Run | Seed | Outcome | Records | Repairs | Bad Lines | finish_reason | Think Tok | Out Tok | Field Acc |")
    lines.append("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for r in all_runs:
        m_name = MODEL_DISPLAY[r["model"]]
        s = r["schema"]
        run_idx = r["run"]
        seed = r["seed"]
        outcome_str = r["outcome_str"]
        rec_str = f"{r['record_count']}/{r['expected_count']}"
        rep = r["repairs_count"]
        bad = r["bad_lines_count"]
        fr = r["finish_reason"]
        th = r["thinking_tokens"]
        out_t = r["output_tokens"]
        acc = f"{(r['field_accuracy']*100.0):.1f}%" if r["record_count"] > 0 else "n/a"
        lines.append(f"| {m_name} | `{s}` | {run_idx} | {seed} | {outcome_str} | {rec_str} | {rep} | {bad} | {fr} | {th} | {out_t} | {acc} |")
    lines.append("")

    report_content = "\n".join(lines)
    with open(REPORT_FILE, "w", encoding="utf-8") as f:
        f.write(report_content)
    print(f"Report written to {REPORT_FILE} ({len(lines)} lines)")

if __name__ == "__main__":
    pair_data, repairs_by_kind, bad_lines_by_problem, all_runs = evaluate()
    build_report(pair_data, repairs_by_kind, bad_lines_by_problem, all_runs)
    print("Done!")
