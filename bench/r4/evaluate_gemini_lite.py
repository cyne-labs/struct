#!/usr/bin/env python3
"""
Evaluator and report generator for Struct Milestone R4-gemini-lite: retrieval accuracy across formats on Gemini 3.5 Flash Lite.

Follows RULES.md, R4.md, R4-haiku.md, and R4-gemini-lite.md:
- Uses evaluate.evaluate_runs for exact same normalized scoring and rules.
- Setup section stating model ID, the API, thinking low, 4 repeats, seed-by-seed order, and completed seeds count.
- Incomplete seeds (Decision 5): scores only complete seeds (all 24 calls succeeded).
  Lists any incomplete seed separately with how many of its calls finished, and doesn't mix it into the accuracy tables.
- Input and thinking tokens reported directly from Gemini API usageMetadata.
- Budget column shows n/a for Gemini (thinking low, no budget).
- Repeat spread table for seeds 1 to 4 (calculated over complete seeds).
- Writes research/r4-retrieval-gemini-lite.md in the same layout as the Haiku report.
"""

import os
import sys
import json
from collections import defaultdict

BENCH_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STRUCT_ROOT = os.path.dirname(BENCH_DIR)
R4_DIR = os.path.join(BENCH_DIR, "r4")
sys.path.insert(0, BENCH_DIR)

from r4.formats import FORMAT_DISPLAY_NAMES, FORMATS, ENCODERS
from r4.collect import build_prompt, load_dataset, load_questions
from r4.evaluate import evaluate_runs, normalize_text, score_answer, parse_model_answers

REAL_GEMINI_LITE_DIR = os.environ.get("R4_REAL_GEMINI_LITE_DIR") or os.path.join(R4_DIR, "real_gemini_lite")
RAW_GEMINI_LITE_DIR = os.path.join(REAL_GEMINI_LITE_DIR, "raw")
METADATA_FILE = os.path.join(REAL_GEMINI_LITE_DIR, "metadata.json")
RUN_INFO_FILE = os.path.join(REAL_GEMINI_LITE_DIR, "run_info.json")
QUESTIONS_PATH = os.path.join(R4_DIR, "questions.json")
REPORT_FILE = os.environ.get("R4_GEMINI_LITE_REPORT_FILE") or os.path.join(STRUCT_ROOT, "research", "r4-retrieval-gemini-lite.md")

DEFAULT_ENDPOINT = "POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"
SEEDS = [1, 2, 3, 4]
CALLS_PER_SEED = 24


def get_gemini_lite_model_id(metadata_list=None, run_info_file=RUN_INFO_FILE):
    """Extracts model ID from run_info or metadata if available, otherwise defaults to gemini-3.5-flash-lite."""
    if os.path.exists(run_info_file):
        try:
            with open(run_info_file, "r", encoding="utf-8") as f:
                info = json.load(f)
                if info.get("model_id"):
                    return info["model_id"]
        except Exception:
            pass
    if metadata_list:
        for m in metadata_list:
            if m.get("model_id"):
                return m["model_id"]
    return "gemini-3.5-flash-lite"


def identify_complete_and_incomplete_seeds(metadata_list, raw_dir=RAW_GEMINI_LITE_DIR):
    """
    Identifies complete seeds (all 24 calls succeeded with non-empty raw files)
    and incomplete seeds per Decision 5.
    Returns (complete_seeds_list, incomplete_seeds_dict)
    where incomplete_seeds_dict maps seed -> count of successful calls.
    """
    successful_calls_by_seed = defaultdict(set)
    for m in (metadata_list or []):
        if not isinstance(m, dict):
            continue
        if m.get("success", False) and not m.get("error"):
            finish = str(m.get("finish_reason", ""))
            if not finish.startswith("error"):
                fname = m.get("file", "")
                if fname:
                    fpath = os.path.join(raw_dir, fname)
                    if os.path.exists(fpath) and os.path.getsize(fpath) > 0:
                        key = (m.get("format"), m.get("dataset"))
                        successful_calls_by_seed[m.get("seed")].add(key)

    complete_seeds = []
    incomplete_seeds = {}

    for s in SEEDS:
        count = len(successful_calls_by_seed[s])
        if count == CALLS_PER_SEED:
            complete_seeds.append(s)
        else:
            incomplete_seeds[s] = count

    return sorted(complete_seeds), incomplete_seeds


def generate_gemini_lite_report(
    results,
    complete_seeds,
    incomplete_seeds,
    report_file=REPORT_FILE,
    metadata_list=None,
    run_info_file=RUN_INFO_FILE,
):
    """Generates the Markdown report for Gemini 3.5 Flash Lite per R4-gemini-lite.md specification."""
    lines = []
    lines.append("# R4-gemini-lite: Retrieval Accuracy Across Formats (Gemini 3.5 Flash Lite)\n\n")
    lines.append("Benchmark measuring model comprehension across 7 data serialization formats on Gemini 3.5 Flash Lite via Google Gemini API.\n\n")

    model_id = get_gemini_lite_model_id(metadata_list, run_info_file=run_info_file)
    endpoint = f"POST https://generativelanguage.googleapis.com/v1beta/models/{model_id}:generateContent"

    lines.append("## 1. Setup\n\n")
    lines.append(f"- **Model:** {model_id} (called through Gemini API, thinking level `low`).\n")
    lines.append(f"- **API Endpoint:** `{endpoint}`\n")
    lines.append("- **Reasoning budget:** no budget (thinking level `low`, budget column shows `n/a`).\n")
    lines.append(f"- **Repeats & Order:** 4 repeats (labelled seed 1 to 4; 96 calls total planned), run strictly **seed-by-seed** (all 24 prompts of seed 1 before seed 2, etc.).\n")
    lines.append("- **Parallel:** up to 4 parallel calls within the current seed.\n")

    if complete_seeds:
        seeds_str = ", ".join(f"Seed {s}" for s in complete_seeds)
        lines.append(f"- **Completed Seeds Scored:** {len(complete_seeds)} complete seed(s) ({seeds_str}; {len(complete_seeds) * CALLS_PER_SEED} calls total scored).\n")
    else:
        lines.append("- **Completed Seeds Scored:** 0 complete seeds. (Run was partial; no complete seeds available to score).\n")

    if incomplete_seeds:
        inc_parts = [f"Seed {s} ({cnt}/{CALLS_PER_SEED} calls finished)" for s, cnt in sorted(incomplete_seeds.items()) if cnt > 0]
        if not inc_parts:
            inc_parts = [f"Seed {s} (0/{CALLS_PER_SEED} calls finished)" for s in sorted(incomplete_seeds.keys())]
        lines.append(f"- **Incomplete Seeds Excluded (Decision 5):** {', '.join(inc_parts)} (excluded from accuracy tables below).\n")

    lines.append("- **Input & Thinking Tokens:** As reported by the Gemini API `usageMetadata` (`promptTokenCount` and `thoughtsTokenCount`).\n")
    lines.append("- **Design:** TSV and Plain `key: value` run only on the two flat datasets (per maintainer Decision 1; marked as 'can\\'t hold this shape' for nested datasets).\n\n")

    lines.append("## 2. Standardized Prompt\n\n")
    lines.append("The prompt is identical for every format except the data block and one line naming the format.\n")
    lines.append("Here is the exact full prompt text as sent to the model (example: Struct on flat_tabular):\n\n")
    lines.append("```text\n")
    try:
        ft_data = load_dataset("flat_tabular")
        ft_questions = load_questions().get("flat_tabular", [])
        encoded_data = ENCODERS["struct"](ft_data, "flat_tabular")
        full_prompt = build_prompt("struct", "flat_tabular", encoded_data, ft_questions)
        lines.append(full_prompt)
    except Exception as e:
        lines.append(f"[Error generating prompt: {e}]\n")
    lines.append("```\n\n")

    lines.append("## 3. Results Summary Across Formats\n\n")
    lines.append("| Format | Accuracy (Counts) | Accuracy (%) | Mean Input Toks* | Mean Thinking Toks | Max Thinking Toks | Budget Exhausted | Failed Runs |\n")
    lines.append("|---|---|---|---|---|---|---|---|\n")

    for fmt in FORMATS:
        f_data = results["by_format"][fmt]
        tot = f_data["total"]
        cor = f_data["correct"]
        pct = (cor / tot * 100) if tot > 0 else 0.0
        in_toks = sum(f_data["input_tokens"]) / len(f_data["input_tokens"]) if f_data["input_tokens"] else 0
        th_toks = sum(f_data["thinking_tokens"]) / len(f_data["thinking_tokens"]) if f_data["thinking_tokens"] else 0
        max_th = max(f_data["thinking_tokens"]) if f_data["thinking_tokens"] else 0
        failed = f_data.get("failed_runs", 0)
        disp = FORMAT_DISPLAY_NAMES.get(fmt, fmt)
        lines.append(f"| {disp} | {cor}/{tot} | {pct:.1f}% | {in_toks:.0f} | {th_toks:.0f} | {max_th} | n/a | {failed} |\n")

    lines.append("\n* Input and thinking tokens are as reported by the Gemini API `usageMetadata` (`promptTokenCount` and `thoughtsTokenCount`).\n")

    lines.append("\n## 4. Accuracy by Dataset\n\n")
    lines.append("| Format | flat_tabular (20r) | flat_100 (100r) | nested_fixed (10r) | nested_variable (4r) |\n")
    lines.append("|---|---|---|---|---|\n")

    datasets = ["flat_tabular", "flat_100", "nested_fixed", "nested_variable"]
    for fmt in FORMATS:
        disp = FORMAT_DISPLAY_NAMES.get(fmt, fmt)
        row = [disp]
        for ds in datasets:
            if fmt in ("tsv", "key_value") and ds in ("nested_fixed", "nested_variable"):
                row.append("can't hold this shape")
            else:
                fd = results["by_format_dataset"].get((fmt, ds), {"correct": 0, "total": 0})
                c, t = fd["correct"], fd["total"]
                pct = (c / t * 100) if t > 0 else 0.0
                row.append(f"{c}/{t} ({pct:.1f}%)")
        lines.append(f"| {' | '.join(row)} |\n")

    lines.append("\n## 5. Accuracy by Question Kind\n\n")
    lines.append("| Question Kind | Accuracy (Counts) | Accuracy (%) |\n")
    lines.append("|---|---|---|\n")
    for kind in ["single_lookup", "nested_lookup", "count_filter", "null_missing"]:
        kd = results["by_kind"].get(kind, {"correct": 0, "total": 0})
        c, t = kd["correct"], kd["total"]
        pct = (c / t * 100) if t > 0 else 0.0
        lines.append(f"| {kind} | {c}/{t} | {pct:.1f}% |\n")

    lines.append("\n## 6. Accuracy per Format by Question Kind\n\n")
    lines.append("| Format | Single Lookup | Nested Lookup | Count / Filter | Null / Missing |\n")
    lines.append("|---|---|---|---|---|\n")
    kinds = ["single_lookup", "nested_lookup", "count_filter", "null_missing"]
    for fmt in FORMATS:
        disp = FORMAT_DISPLAY_NAMES.get(fmt, fmt)
        row = [disp]
        for k in kinds:
            if fmt in ("tsv", "key_value") and k == "nested_lookup":
                row.append("can't hold this shape")
            else:
                kd = results["by_format_kind"].get((fmt, k), {"correct": 0, "total": 0})
                c, t = kd["correct"], kd["total"]
                pct = (c / t * 100) if t > 0 else 0.0
                row.append(f"{c}/{t} ({pct:.1f}%)")
        lines.append(f"| {' | '.join(row)} |\n")

    lines.append("\n## 7. Format Notes and Review Decisions\n\n")
    lines.append("- **Partial Runs (Decision 5):** The evaluator scores only complete seeds (all 24 calls succeeded). Incomplete seeds are listed separately and not mixed into accuracy tables.\n")
    lines.append("- **XML Empty Lists (Decision 12):** Plain XML cannot distinguish an empty list (`<subnotes/>`) from no value without schema or type attributes. This is recorded as an inherent limit of plain XML.\n")
    lines.append("- **TOON Spec Adherence (Decision 13):** Tabular headers with uniform nested objects use valid Nested Field Groups (`source{id,year}` per TOON spec 9.3), numeric-looking strings like arXiv IDs are quoted (`\"2405.05254\"` per spec 7.2), and empty lists use `key: []` (`subnotes: []` per TOON spec 9.1).\n")
    lines.append("- **Failed Requests (Decision 13):** Any request experiencing an HTTP error, quota failure, timeout, unparseable JSON, or empty response is recorded as failed and not marked done; resume reruns them.\n")

    failed_runs = results.get("failed_runs", [])
    if failed_runs:
        lines.append(f"\n### Recorded Failed Requests in Scored Seeds ({len(failed_runs)})\n\n")
        lines.append("| Run Index | Format | Dataset | Seed | Error |\n")
        lines.append("|---|---|---|---|---|\n")
        for fr in failed_runs:
            lines.append(f"| {fr.get('run_index', 'n/a')} | {fr.get('format')} | {fr.get('dataset')} | {fr.get('seed')} | `{fr.get('finish_reason', 'error')}` |\n")

    lines.append("\n## 8. Repeat Spread per Format\n\n")
    lines.append("| Format | Seed 1 | Seed 2 | Seed 3 | Seed 4 | Mean* | Spread (Max-Min)* |\n")
    lines.append("|---|---|---|---|---|---|---|\n")
    for fmt in FORMATS:
        disp = FORMAT_DISPLAY_NAMES.get(fmt, fmt)
        seed_scores = []
        seed_cells = []
        for s in SEEDS:
            if s in complete_seeds:
                sd = results["by_format_seed"][fmt].get(s, {"correct": 0, "total": 0})
                if sd["total"] > 0:
                    pct = sd["correct"] / sd["total"] * 100
                    seed_scores.append(pct)
                    seed_cells.append(f"{pct:.1f}%")
                else:
                    seed_cells.append("n/a")
            else:
                finished_cnt = incomplete_seeds.get(s, 0)
                seed_cells.append(f"incomplete ({finished_cnt}/24)")

        if seed_scores:
            mean_pct = sum(seed_scores) / len(seed_scores)
            spread = max(seed_scores) - min(seed_scores)
            lines.append(f"| {disp} | {' | '.join(seed_cells)} | {mean_pct:.1f}% | {spread:.1f}% |\n")
        else:
            lines.append(f"| {disp} | {' | '.join(seed_cells)} | n/a | n/a |\n")

    lines.append("\n* Mean and spread are calculated across completed seeds only.\n")

    if incomplete_seeds:
        lines.append("\n## 9. Incomplete Seeds Summary\n\n")
        lines.append("Per maintainer Decision 5, seeds with fewer than 24 successful calls are excluded from the primary accuracy tables:\n\n")
        lines.append("| Seed | Successful Calls | Status | Excluded From Accuracy |\n")
        lines.append("|---|---|---|---|\n")
        for s in sorted(incomplete_seeds.keys()):
            cnt = incomplete_seeds[s]
            status_desc = "Partial (stopped before completion)" if cnt > 0 else "Not started"
            lines.append(f"| Seed {s} | {cnt}/{CALLS_PER_SEED} | {status_desc} | Yes |\n")

    section_num = 10 if incomplete_seeds else 9
    lines.append(f"\n## {section_num}. Wrong Answers Log\n\n")
    wrong = results.get("wrong_answers", [])
    if not wrong:
        lines.append("No wrong answers recorded in complete seeds.\n")
    else:
        lines.append(f"Total wrong answers across completed seeds: {len(wrong)}\n\n")
        lines.append("| Format | Dataset | Seed | Question | Expected | Model Answer |\n")
        lines.append("|---|---|---|---|---|---|\n")
        for w in wrong:
            disp = FORMAT_DISPLAY_NAMES.get(w['format'], w['format'])
            q_clean = str(w['question']).replace('|', '\\|')
            exp_clean = str(w['expected']).replace('|', '\\|')
            act_clean = str(w['model_answer']).replace('|', '\\|')
            lines.append(f"| {disp} | {w['dataset']} | {w['seed']} | {q_clean} | `{exp_clean}` | `{act_clean}` |\n")

    os.makedirs(os.path.dirname(report_file), exist_ok=True)
    with open(report_file, "w", encoding="utf-8") as f:
        f.write("".join(lines))

    print(f"Report successfully written to {report_file}")


def run_evaluation(
    real_dir=REAL_GEMINI_LITE_DIR,
    report_file=REPORT_FILE,
    questions_path=QUESTIONS_PATH,
    run_info_file=None,
):
    """
    Evaluates recorded runs in real_dir and generates markdown report at report_file.
    Per Decision 5: scores only complete seeds (all 24 calls succeeded).
    Incomplete seeds are listed separately and not mixed into accuracy tables.
    """
    if not os.path.exists(questions_path):
        print(f"Missing {questions_path}")
        sys.exit(1)

    with open(questions_path, "r", encoding="utf-8") as f:
        questions = json.load(f)

    metadata_file = os.path.join(real_dir, "metadata.json")
    raw_dir = os.path.join(real_dir, "raw")
    info_file = run_info_file or os.path.join(real_dir, "run_info.json")

    metadata = []
    if os.path.exists(metadata_file):
        try:
            with open(metadata_file, "r", encoding="utf-8") as f:
                metadata = json.load(f)
        except Exception as e:
            print(f"Error reading {metadata_file}: {e}")

    if not metadata:
        print(f"No metadata found in {metadata_file}. Run collection first or supply mock metadata.")
        return None, [], {}

    complete_seeds, incomplete_seeds = identify_complete_and_incomplete_seeds(
        metadata, raw_dir=raw_dir
    )

    # Per Decision 5: score only complete seeds!
    filtered_metadata = [m for m in metadata if m.get("seed") in complete_seeds]

    results = evaluate_runs(filtered_metadata, questions, raw_dir=raw_dir)
    generate_gemini_lite_report(
        results,
        complete_seeds=complete_seeds,
        incomplete_seeds=incomplete_seeds,
        report_file=report_file,
        metadata_list=metadata,
        run_info_file=info_file,
    )
    return results, complete_seeds, incomplete_seeds


def main():
    run_evaluation()


if __name__ == "__main__":
    main()
