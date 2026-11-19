#!/usr/bin/env python3
"""
Evaluator for Struct v0.3 real model outputs (R3c Part C.5).

Runs doctor_tool in salvage mode on all 100 outputs in bench/r3/real/run2/raw/.
Computes:
- Outcome: Clean as written, Repaired, Salvaged (N bad lines), Unusable
- Repairs by kind
- Bad lines by problem
- Thinking and output tokens
- Token cap hits
- Field accuracy against ground truth: share of fields exactly right among records that decoded
- New mistake kinds with real quotes
Generates research/r3-real-run2.md and prints summary tables.
"""

import os
import sys
import json
import hashlib
import subprocess
from collections import Counter, defaultdict

from tasks import TASKS, get_task_prompt

STRUCT_ROOT = r"C:\Users\Ashman Das\Documents\Struct"
DOCTOR_TOOL = os.path.join(STRUCT_ROOT, "target", "debug", "doctor_tool.exe")

RUN2_DIR = os.path.join(STRUCT_ROOT, "bench", "r3", "real", "run2")
RAW_DIR = os.path.join(RUN2_DIR, "raw")
SHA256_FILE = os.path.join(RUN2_DIR, "SHA256SUMS")
METADATA_FILE = os.path.join(RUN2_DIR, "metadata.json")
REPORT_FILE = os.path.join(STRUCT_ROOT, "research", "r3-real-run2.md")

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

MODEL_NAMES = {
    "llama_3b": "Llama-3.2-3B (thinking off)",
    "gemma_off": "Gemma-4-E2B (thinking off)",
    "gemma_on": "Gemma-4-E2B (thinking on)",
    "lfm": "LFM2.5-8B (thinking on)",
    "mellum": "Mellum2.1-12B (thinking on)",
}

def verify_hashes():
    if not os.path.exists(SHA256_FILE):
        return False
    with open(SHA256_FILE, "r", encoding="utf-8") as f:
        lines = [l.strip() for l in f if l.strip()]
    for line in lines:
        parts = line.split(maxsplit=1)
        if len(parts) != 2:
            continue
        expected_hash, fname = parts[0], parts[1].strip()
        fpath = os.path.join(RAW_DIR, fname)
        if not os.path.exists(fpath):
            return False
        with open(fpath, "rb") as bf:
            actual_hash = hashlib.sha256(bf.read()).hexdigest()
        if actual_hash != expected_hash:
            return False
    return True

def run_doctor(schema_path, file_path):
    cmd = [DOCTOR_TOOL, schema_path, file_path, "salvage", "true"]
    res = subprocess.check_output(cmd, text=True)
    return json.loads(res.strip())

def compute_field_accuracy(schema_key, decoded_records):
    """
    Computes field accuracy against ground truth.
    Returns (correct_fields, expected_fields_in_decoded, total_ground_truth_fields)
    """
    gt = TASKS[schema_key]["ground_truth"]
    if not decoded_records:
        # 0 decoded records
        total_gt = count_total_ground_truth_fields(schema_key, gt)
        return 0, 0, total_gt

    correct = 0
    evaluated_expected = 0

    if schema_key == "s1_flat":
        for i, dec in enumerate(decoded_records):
            if i >= len(gt):
                break
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
            exp = gt[i]
            # text, verdict, verified
            evaluated_expected += 5
            if str(dec.get("text", "")).strip() == str(exp.get("text", "")).strip():
                correct += 1
            if str(dec.get("verdict", "")).strip() == str(exp.get("verdict", "")).strip():
                correct += 1
            if dec.get("verified") == exp.get("verified"):
                correct += 1
            # source
            dec_src = dec.get("source", {})
            exp_src = exp.get("source", {})
            if str(dec_src.get("id", "")).strip() == str(exp_src.get("id", "")).strip():
                correct += 1
            if str(dec_src.get("year", "")).strip() == str(exp_src.get("year", "")).strip():
                correct += 1

    elif schema_key == "s3_nested_variable":
        for i, dec in enumerate(decoded_records):
            if i >= len(gt):
                break
            exp = gt[i]
            evaluated_expected += 4 # text, verdict, source.id, source.year
            if str(dec.get("text", "")).strip() == str(exp.get("text", "")).strip():
                correct += 1
            if str(dec.get("verdict", "")).strip() == str(exp.get("verdict", "")).strip():
                correct += 1
            dec_src = dec.get("source", {})
            exp_src = exp.get("source", {})
            if str(dec_src.get("id", "")).strip() == str(exp_src.get("id", "")).strip():
                correct += 1
            if str(dec_src.get("year", "")).strip() == str(exp_src.get("year", "")).strip():
                correct += 1

            # notes
            dec_notes = dec.get("notes", [])
            exp_notes = exp.get("notes", [])
            for j, exp_n in enumerate(exp_notes):
                evaluated_expected += 1 # note text
                evaluated_expected += len(exp_n.get("subnotes", [])) # subnotes
                if j < len(dec_notes):
                    dec_n = dec_notes[j]
                    if str(dec_n.get("text", "")).strip() == str(exp_n.get("text", "")).strip():
                        correct += 1
                    dec_sub = dec_n.get("subnotes", [])
                    exp_sub = exp_n.get("subnotes", [])
                    for k, s_val in enumerate(exp_sub):
                        if k < len(dec_sub) and str(dec_sub[k]).strip() == str(s_val).strip():
                            correct += 1

    elif schema_key == "s4_two_class_code":
        # Separate files and patches
        dec_files = [r for r in decoded_records if "path" in r]
        dec_patches = [r for r in decoded_records if "code" in r or "line" in r]
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

def main():
    print(">>> Verifying SHA256SUMS in run2...")
    if not verify_hashes():
        print("ERROR: SHA256SUMS verification failed in run2!")
        sys.exit(1)
    print("SHA256SUMS verified OK.")

    with open(METADATA_FILE, "r", encoding="utf-8") as f:
        metadata = json.load(f)

    results = []
    outcomes_by_model = defaultdict(lambda: Counter())
    outcomes_by_pair = defaultdict(lambda: Counter())
    records_by_pair = defaultdict(list)
    accuracy_by_pair = defaultdict(list)
    distinct_by_pair = defaultdict(set)
    cap_by_pair = defaultdict(int)
    correct_by_pair = defaultdict(int)
    eval_by_pair = defaultdict(int)
    repairs_by_fix = Counter()
    bad_lines_by_problem = Counter()
    repairs_by_model = defaultdict(lambda: Counter())

    for item in metadata:
        model = item["model"]
        schema = item["schema"]
        run_idx = item["run"]
        fname = item["filename"]
        fpath = os.path.join(RAW_DIR, fname)

        schema_path = SCHEMAS[schema]
        doc_res = run_doctor(schema_path, fpath)

        has_data = doc_res.get("has_data", False)
        rec_count = doc_res.get("record_count", 0)
        repairs = doc_res.get("repairs", [])
        bad_lines = doc_res.get("bad_lines", [])
        decoded_records = doc_res.get("records", [])

        # Categorize outcome
        exp_rec_n = EXPECTED_RECORD_COUNTS[schema]
        if has_data:
            if not repairs and not bad_lines:
                outcome = "Clean as written"
            elif bad_lines:
                outcome = f"Salvaged ({rec_count} records)"
            elif rec_count < exp_rec_n:
                # Repaired without bad lines but records are missing: never call this fully repaired.
                if item["stopped_limit"]:
                    outcome = f"Partial, cut by cap ({rec_count}/{exp_rec_n} records)"
                else:
                    outcome = f"Partial ({rec_count}/{exp_rec_n} records)"
            else:
                outcome = "Fully repaired"
        else:
            outcome = "Nothing usable"

        correct, eval_exp, total_gt = compute_field_accuracy(schema, decoded_records)
        field_acc = (correct / eval_exp) if eval_exp > 0 else 0.0
        out_hash = hashlib.sha256(open(fpath, "rb").read()).hexdigest()
        distinct_by_pair[(model, schema)].add(out_hash)
        cap_by_pair[(model, schema)] += 1 if item["stopped_limit"] else 0
        correct_by_pair[(model, schema)] += correct
        eval_by_pair[(model, schema)] += eval_exp

        outcomes_by_model[model][outcome] += 1
        outcomes_by_pair[(model, schema)][outcome] += 1
        records_by_pair[(model, schema)].append(rec_count)
        accuracy_by_pair[(model, schema)].append(field_acc)

        for r in repairs:
            repairs_by_fix[r["fix"]] += 1
            repairs_by_model[model][r["fix"]] += 1

        for b in bad_lines:
            prob = b["problem"]
            bad_lines_by_problem[prob] += 1

        results.append({
            "model": model,
            "model_name": MODEL_NAMES[model],
            "schema": schema,
            "run": run_idx,
            "seed": item["seed"],
            "stopped_limit": item["stopped_limit"],
            "truncated": item.get("truncated", False),
            "thinking_tokens": item["thinking_tokens"],
            "struct_tokens": item["struct_tokens"],
            "outcome": outcome,
            "record_count": rec_count,
            "repairs_count": len(repairs),
            "bad_lines_count": len(bad_lines),
            "field_accuracy": field_acc,
            "has_data": has_data,
            "correct_fields": correct,
            "evaluated_expected": eval_exp,
            "total_gt_fields": total_gt,
            "repairs": repairs,
            "bad_lines": bad_lines,
        })

    # Print summary table
    print("\n========================================================")
    print("RUN2 EVALUATION SUMMARY (100 REAL RUNS)")
    print("========================================================")
    for model_key in ["llama_3b", "gemma_off", "gemma_on", "lfm", "mellum"]:
        m_name = MODEL_NAMES[model_key]
        counts = outcomes_by_model[model_key]
        clean = counts.get("Clean as written", 0)
        repaired = counts.get("Fully repaired", 0)
        salvaged = sum(v for k, v in counts.items() if k.startswith("Salvaged"))
        partial = sum(v for k, v in counts.items() if k.startswith("Partial"))
        nothing = counts.get("Nothing usable", 0)
        print(f"{m_name:32} | Clean: {clean:2} | Repaired: {repaired:2} | Partial: {partial:2} | Salvaged: {salvaged:2} | Nothing: {nothing:2}")

    print("\nSafe fixes applied:")
    for fix, count in repairs_by_fix.most_common():
        print(f"  {fix:24}: {count}")

    print("\nBad lines by problem:")
    for prob, count in bad_lines_by_problem.most_common()[:15]:
        print(f"  {prob:48}: {count}")

    # Build report research/r3-real-run2.md
    lines_md = []
    lines_md.append("# Milestone R3c: Struct Doctor Real-Model Output Report (Run 2)\n")
    lines_md.append("> **This run is superseded by run3 (no token cap, no thinking prefills); kept as a record.** Most failures below are 1,500-token budget artifacts, not format failures. See section 8 for limits.\n")
    lines_md.append("This report documents the empirical evaluation of **Struct Doctor** (`structl::doctor`) across **100 real model outputs** (5 model configurations × 4 schemas × 5 runs). All runs were executed **without grammar constraints**, using canonical `prompt(schemas)` prompts with real source notes from R1 fixtures, evaluated under salvage mode against exact ground truth.\n")

    lines_md.append("## 1. Experimental Setup & Environment\n")
    lines_md.append("- **Hardware:** NVIDIA GeForce RTX 4060 Laptop GPU (8,188 MiB VRAM), Windows 11.")
    lines_md.append("- **Inference Engine:** `llama-server.exe` on `127.0.0.1:8090`.")
    lines_md.append("- **Parameters:** Temperature 0.7, token budget 1,500 (`n_predict`), fixed seeds 42–46 per schema.")
    lines_md.append("- **Token Cap Recording:** Hit cap recorded strictly from `stop_type == \"limit\"`. Prompt truncation kept separate.")
    lines_md.append("- **Prompts:** Canonical `prompt(schemas)` per SPEC §10, one blank line, real source notes ending with `Write these as Struct records.`")
    lines_md.append("- **Models Evaluated (5 configurations):**")
    lines_md.append("  1. `Llama-3.2-3B-Instruct` (thinking OFF)")
    lines_md.append("  2. `gemma-4-E2B-it` (thinking OFF)")
    lines_md.append("  3. `gemma-4-E2B-it` (thinking ON, end-of-thought `<channel|>`)")
    lines_md.append("  4. `LFM2.5-8B-A1B` (thinking ON, end-of-thought `</think>`)")
    lines_md.append("  5. `Mellum2.1-12B-A2.5B-Thinking` (thinking ON, end-of-thought `</think>`, `-ngl 16`)\n")

    lines_md.append("## 2. Prompts Used for Each Schema\n")
    for s_key in ["s1_flat", "s2_nested_fixed", "s3_nested_variable", "s4_two_class_code"]:
        lines_md.append(f"### Full Prompt for `{s_key}`\n")
        lines_md.append("```text")
        lines_md.append(get_task_prompt(s_key))
        lines_md.append("```\n")

    lines_md.append("## 3. 5×4 Summary Matrix (Outcomes, Recall, Field Accuracy, Cap Hits)\n")
    lines_md.append("Definitions: **Record Recall** = mean records decoded / records expected (zero-record runs count as 0). **Field Accuracy (Decoded)** = share of fields exactly right among the records that decoded, pooled over runs that decoded at least one record; `n/a` when no run decoded anything (zero-record runs are not averaged in as 0%). **Partial** = repaired with no bad lines but fewer records than expected. **Cap Hits** = runs with `stop_type == \"limit\"` at the 1,500-token cap. **Distinct Outputs** = distinct raw output files out of 5 seeds.\n")
    lines_md.append("| Model | Schema | Runs | Clean | Repaired | Partial | Salvaged | Nothing Usable | Avg Record Yield | Record Recall | Field Accuracy (Decoded) | Cap Hits | Distinct Outputs |")
    lines_md.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|")

    for model_key in ["llama_3b", "gemma_off", "gemma_on", "lfm", "mellum"]:
        m_name = MODEL_NAMES[model_key]
        for schema_key in ["s1_flat", "s2_nested_fixed", "s3_nested_variable", "s4_two_class_code"]:
            pair = (model_key, schema_key)
            counts = outcomes_by_pair[pair]
            clean = counts.get("Clean as written", 0)
            repaired = counts.get("Fully repaired", 0)
            partial = sum(v for k, v in counts.items() if k.startswith("Partial"))
            salvaged = sum(v for k, v in counts.items() if k.startswith("Salvaged"))
            nothing = counts.get("Nothing usable", 0)
            recs = records_by_pair[pair]
            avg_rec = sum(recs) / len(recs) if recs else 0.0
            exp_rec = EXPECTED_RECORD_COUNTS[schema_key]
            recall = avg_rec / exp_rec * 100
            if eval_by_pair[pair] > 0:
                acc_s = f"{correct_by_pair[pair] / eval_by_pair[pair] * 100:.1f}%"
            else:
                acc_s = "n/a"
            lines_md.append(f"| {m_name} | `{schema_key}` | 5 | {clean} | {repaired} | {partial} | {salvaged} | {nothing} | {avg_rec:.1f} / {exp_rec} | {recall:.0f}% | {acc_s} | {cap_by_pair[pair]}/5 | {len(distinct_by_pair[pair])}/5 |")

    lines_md.append("")
    cap_total = sum(cap_by_pair.values())
    lines_md.append(f"**Cap hits: {cap_total}/100 runs.** By model: " + "; ".join(f"{MODEL_NAMES[mk]} {sum(cap_by_pair[(mk, sk)] for sk in SCHEMAS)}/20" for mk in ["llama_3b", "gemma_off", "gemma_on", "lfm", "mellum"]) + ". Most thinking-model failures (LFM and Mellum) are budget artifacts: the 1,500-token cap ended generation inside or right after the reasoning block, so these rows measure the cap, not whether the model can write Struct.\n")
    lines_md.append("\n## 4. Repairs Applied by Kind\n")
    lines_md.append("| Safe Fix | Count Across All Runs |")
    lines_md.append("|---|---|")
    for fix, count in repairs_by_fix.most_common():
        lines_md.append(f"| `{fix}` | {count} |")

    lines_md.append("\n## 5. Bad Lines Encountered by Problem Diagnostic\n")
    lines_md.append("| Problem Diagnostic | Frequency |")
    lines_md.append("|---|---|")
    for prob, count in bad_lines_by_problem.most_common():
        lines_md.append(f"| {prob} | {count} |")

    lines_md.append("\n## 6. Analysis of Mistakes & Real Quotes\n")
    lines_md.append("Below are the primary failure patterns and real quotes observed in run 2:\n")
    lines_md.append("1. **Escaped Spaces (`\\ `):** Models (especially Gemma without thinking) frequently attempted to escape spaces inside string values with backslashes, e.g. `Claim\\ 2405.05254`. Struct v0.3 SPEC §3 permits only `\\|`, `\\\\`, and leading `\\r`, reporting any other backslash sequence as an error.")
    lines_md.append("2. **Data Inlined in Schema Definitions:** In multi-class code tasks (`s4_two_class_code`), models sometimes generated pseudo-schema lines embedding data values, such as `schema:file{name|src/lib.rs}`.")
    lines_md.append("3. **Missing Schema Headers with Multiple Classes:** When two or more classes are defined outside (`s4_two_class_code`), SPEC §6 mandates that data blocks begin with `schema:class_name`. Llama-3.2-3B emitted data rows immediately without a `schema:` header, triggering `with several outside classes, data before the first schema line is an error`.")
    lines_md.append("4. **Comma Delimiters with Terminal Pipe:** LFM on `s1_flat` occasionally generated comma-separated fields while keeping the terminal pipe: `2405.05254,Rope scaling hurts long ctx,refuted,2024,0|`.")
    lfm_s1 = [r for r in results if r["model"] == "lfm" and r["schema"] == "s1_flat"]
    lfm_acc_txt = ", ".join(f"run {r['run']}: {r['record_count']} records, {r['field_accuracy']*100:.0f}%" if r["has_data"] else f"run {r['run']}: nothing usable" for r in lfm_s1)
    mellum_r2 = [r for r in results if r["model"] == "mellum" and r["schema"] == "s1_flat" and r["run"] == 2][0]
    lines_md.append("5. **Missing Final Pipe (SafeFix):** LFM on `s1_flat` runs 3-5 produced valid fields but omitted the closing pipe on all records: `2405.05254|Rope scaling hurts long ctx|refuted|2024|0`. `MissingFinalPipe` repaired the pipes, but accuracy was not uniformly 100%. Real per-run results for LFM `s1_flat` (" + lfm_acc_txt + "). Run 5 wrote `Claim 2405.05254` in the id field (copied from the note text) in every record, so its id field is wrong in 5 of 5 records and accuracy is 80%; the doctor cannot fix a wrong value.")
    lines_md.append(f"6. **Cut by the cap, not repaired:** Mellum `s1_flat` run 2 decoded only {mellum_r2['record_count']} of 5 records, outcome `{mellum_r2['outcome']}`. It hit the 1,500-token cap (`stop_type == \"limit\"`); the output ends after record 3 with a lone trailing `2` (the start of record 4's id), which `Chatter` removed. The repair is real but the 2 missing records were never generated, so this is not a fully repaired run.")
    lines_md.append("7. **Thinking Budget Exhaustion:** On `s3_nested_variable` and `s4_two_class_code` every LFM and Mellum run hit the cap inside the reasoning block (`<think>...`) before any Struct output. Mellum also hit it on 5/5 `s1_flat` and 4/5 `s2_nested_fixed` runs, LFM on 1/5 `s2_nested_fixed`; all but 3 of those 10 cut-off runs (Mellum s1 run 2, 86 struct tokens; Mellum s2 run 5, 3; LFM s2 run 1, 127) emitted no Struct text at all. These are budget artifacts, not format failures.\n")
    lines_md.append("### Sample Diagnostic Quotes from Models:\n")

    # Collect sample bad lines
    seen_problems = set()
    for res in results:
        for b in res["bad_lines"]:
            p = b["problem"]
            if p not in seen_problems:
                seen_problems.add(p)
                lines_md.append(f"- **Problem:** `{p}` (from {res['model_name']} on `{res['schema']}` run {res['run']}):")
                lines_md.append(f"  ```text\n  {b['original']}\n  ```\n")

    lines_md.append("\n## 7. Complete Per-Run Results Table\n")
    lines_md.append("| Model | Schema | Run | Seed | Outcome | Records | Repairs | Bad Lines | Think Tok | Out Tok | Cap Hit | Field Acc |")
    lines_md.append("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for r in results:
        cap_str = "Yes" if r["stopped_limit"] else "No"
        lines_md.append(f"| {r['model_name']} | `{r['schema']}` | {r['run']} | {r['seed']} | {r['outcome']} | {r['record_count']} | {r['repairs_count']} | {r['bad_lines_count']} | {r['thinking_tokens']} | {r['struct_tokens']} | {cap_str} | {r['field_accuracy']*100:.1f}% |")

    lines_md.append("\n## 8. Limits of This Run\n")
    lines_md.append("- **Token cap dominates:** 31/100 runs hit the 1,500-token cap (Mellum 19/20, LFM 11/20, Llama 1/20, Gemma 0/40). Cap-hit runs are budget artifacts. See the Cap Hits column in section 3.")
    lines_md.append("- **Low output diversity:** Gemma (thinking off) produced identical raw output across most seeds (see Distinct Outputs; e.g. 1 distinct output in 5 seeds on `s2_nested_fixed`), so its 5 runs are not 5 independent samples.")
    lines_md.append("- **Templated task notes with quotes:** notes were written like `title 'Rope scaling hurts long ctx'`, and models copied the single quotes (and the `Claim` prefix) into values. Some value errors come from the note template, not from the format.")
    lines_md.append("- **Unspecified thinking prefills:** Gemma (thinking on) was run with `<|think|>` in the system turn plus a `<|channel>thought\\n` prefill, and Mellum with a `<think>\\n` prefill. The brief did not specify these. LFM had no prefill.")
    lines_md.append("- **Token counts are not server counts:** thinking and output token counts come from `/tokenize` on the split text, not the server's generation counts.")
    lines_md.append("- **Accuracy is positional:** field accuracy compares decoded records to ground truth by position; a skipped record shifts later ones.\n")

    with open(REPORT_FILE, "w", encoding="utf-8") as f:
        f.write("\n".join(lines_md) + "\n")

    print(f"\nFull report written to {REPORT_FILE}")

if __name__ == "__main__":
    main()
