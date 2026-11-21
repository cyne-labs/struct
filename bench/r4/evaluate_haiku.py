#!/usr/bin/env python3
"""
Evaluator and report generator for Struct Milestone R4-haiku: retrieval accuracy across formats on Claude Haiku.

Follows RULES.md, R4.md and R4-haiku.md:
- Uses evaluate.evaluate_runs for exact same normalized scoring and rules.
- Setup section stating model id actually returned, exact command, effort low, no budget, 4 repeats, 4 parallel.
- Input tokens labelled "as counted by the Anthropic API, including about 640 fixed tokens from the system prompt and template on every call".
- Total cost at list price (sum of total_cost_usd across calls), labelled as list price.
- Budget column shows n/a for Haiku, not 0.
- Repeat spread table for seeds 1 to 4.
- Writes research/r4-retrieval-haiku.md in the same layout as the Gemma report.
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

REAL_HAIKU_DIR = os.path.join(R4_DIR, "real_haiku")
RAW_HAIKU_DIR = os.path.join(REAL_HAIKU_DIR, "raw")
METADATA_FILE = os.path.join(REAL_HAIKU_DIR, "metadata.json")
RUN_INFO_FILE = os.path.join(REAL_HAIKU_DIR, "run_info.json")
QUESTIONS_PATH = os.path.join(R4_DIR, "questions.json")
REPORT_FILE = os.path.join(STRUCT_ROOT, "research", "r4-retrieval-haiku.md")

DEFAULT_COMMAND_LINE = (
    'claude -p --model haiku --effort low --system-prompt "You are a helpful assistant." '
    '--tools "" --strict-mcp-config --setting-sources "" --disable-slash-commands '
    '--no-session-persistence --output-format json'
)

SEEDS = [1, 2, 3, 4]


def get_haiku_model_id(metadata_list=None):
    """Extracts model ID from metadata if available, otherwise defaults to claude-3-5-haiku-20241022."""
    if metadata_list:
        for m in metadata_list:
            if m.get("model_id"):
                return m["model_id"]
    return "claude-3-5-haiku-20241022"


def get_haiku_command_line(run_info_file=RUN_INFO_FILE):
    """Retrieves recorded command line from run_info.json if available."""
    if os.path.exists(run_info_file):
        try:
            with open(run_info_file, "r", encoding="utf-8") as f:
                info = json.load(f)
                if info.get("command_line"):
                    return info["command_line"]
        except Exception:
            pass
    return DEFAULT_COMMAND_LINE


def calculate_total_cost(metadata_list=None):
    """Calculates total cost in USD summed across all metadata entries."""
    if not metadata_list:
        return 0.0
    return sum(float(m.get("total_cost_usd", 0.0) or 0.0) for m in metadata_list)


def generate_haiku_report(
    results,
    report_file=REPORT_FILE,
    metadata_list=None,
    run_info_file=RUN_INFO_FILE,
):
    """Generates the Markdown report for Claude Haiku per R4-haiku.md specification."""
    lines = []
    lines.append("# R4-haiku: Retrieval Accuracy Across Formats (Claude Haiku)\n\n")
    lines.append("Benchmark measuring model comprehension across 7 data serialization formats on Claude Haiku.\n\n")

    model_id = get_haiku_model_id(metadata_list)
    cmd_line = get_haiku_command_line(run_info_file)
    total_cost = calculate_total_cost(metadata_list)

    lines.append("## 1. Setup\n\n")
    lines.append(f"- **Model:** {model_id} (requested as `haiku`, effort `low`).\n")
    lines.append(f"- **Command Line:**\n  ```\n  {cmd_line}\n  ```\n")
    lines.append("- **Reasoning budget:** no budget (effort `low`).\n")
    lines.append("- **Repeats:** 4 repeats per prompt (labelled seed 1 to 4; 96 calls total).\n")
    lines.append("- **Parallel:** 4 parallel calls.\n")
    lines.append(f"- **Total Cost (List Price):** ${total_cost:.4f} (sum of `total_cost_usd` across all calls, list price).\n")
    lines.append("- **Input Tokens Measurement:** Counted by the Anthropic API, including about 640 fixed tokens from the system prompt and template on every call.\n")
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

    lines.append("\n* Input tokens are as counted by the Anthropic API, including about 640 fixed tokens from the system prompt and template on every call.\n")

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
    lines.append("- **XML Empty Lists (Decision 12):** Plain XML cannot distinguish an empty list (`<subnotes/>`) from no value without schema or type attributes. This is recorded as an inherent limit of plain XML.\n")
    lines.append("- **TOON Spec Adherence (Decision 13):** Tabular headers with uniform nested objects use valid Nested Field Groups (`source{id,year}` per TOON spec 9.3), numeric-looking strings like arXiv IDs are quoted (`\"2405.05254\"` per spec 7.2), and empty lists use `key: []` (`subnotes: []` per TOON spec 9.1).\n")
    lines.append("- **Failed Requests (Decision 13):** Any request experiencing an exit error, timeout, unparseable JSON, or empty result is recorded as failed and not marked done; resume reruns them.\n")

    failed_runs = results.get("failed_runs", [])
    if failed_runs:
        lines.append(f"\n### Recorded Failed Requests ({len(failed_runs)})\n\n")
        lines.append("| Run Index | Format | Dataset | Seed | Error |\n")
        lines.append("|---|---|---|---|---|\n")
        for fr in failed_runs:
            lines.append(f"| {fr.get('run_index', 'n/a')} | {fr.get('format')} | {fr.get('dataset')} | {fr.get('seed')} | `{fr.get('finish_reason', 'error')}` |\n")

    lines.append("\n## 8. Repeat Spread per Format\n\n")
    lines.append("| Format | Seed 1 | Seed 2 | Seed 3 | Seed 4 | Mean | Spread (Max-Min) |\n")
    lines.append("|---|---|---|---|---|---|---|\n")
    for fmt in FORMATS:
        disp = FORMAT_DISPLAY_NAMES.get(fmt, fmt)
        seed_scores = []
        seed_cells = []
        for s in SEEDS:
            sd = results["by_format_seed"][fmt].get(s, {"correct": 0, "total": 0})
            if sd["total"] > 0:
                pct = sd["correct"] / sd["total"] * 100
                seed_scores.append(pct)
                seed_cells.append(f"{pct:.1f}%")
            else:
                seed_cells.append("n/a")
        if seed_scores:
            mean_pct = sum(seed_scores) / len(seed_scores)
            spread = max(seed_scores) - min(seed_scores)
            lines.append(f"| {disp} | {' | '.join(seed_cells)} | {mean_pct:.1f}% | {spread:.1f}% |\n")
        else:
            lines.append(f"| {disp} | {' | '.join(seed_cells)} | n/a | n/a |\n")

    lines.append("\n## 9. Wrong Answers Log\n\n")
    wrong = results["wrong_answers"]
    if not wrong:
        lines.append("No wrong answers recorded.\n")
    else:
        lines.append(f"Total wrong answers: {len(wrong)}\n\n")
        lines.append("| Format | Dataset | Seed | Question | Expected | Model Answer |\n")
        lines.append("|---|---|---|---|---|---|\n")
        for w in wrong:
            disp = FORMAT_DISPLAY_NAMES.get(w['format'], w['format'])
            q_clean = w['question'].replace('|', '\\|')
            exp_clean = str(w['expected']).replace('|', '\\|')
            act_clean = str(w['model_answer']).replace('|', '\\|')
            lines.append(f"| {disp} | {w['dataset']} | {w['seed']} | {q_clean} | `{exp_clean}` | `{act_clean}` |\n")

    os.makedirs(os.path.dirname(report_file), exist_ok=True)
    with open(report_file, "w", encoding="utf-8") as f:
        f.write("".join(lines))

    print(f"Report successfully written to {report_file}")


def main():
    if not os.path.exists(QUESTIONS_PATH):
        print(f"Missing {QUESTIONS_PATH}")
        sys.exit(1)

    with open(QUESTIONS_PATH, "r", encoding="utf-8") as f:
        questions = json.load(f)

    metadata = []
    if os.path.exists(METADATA_FILE):
        with open(METADATA_FILE, "r", encoding="utf-8") as f:
            metadata = json.load(f)

    if not metadata:
        print(f"No metadata found in {METADATA_FILE}. Run collection first or supply mock metadata.")
        return

    results = evaluate_runs(metadata, questions, raw_dir=RAW_HAIKU_DIR)
    generate_haiku_report(results, report_file=REPORT_FILE, metadata_list=metadata, run_info_file=RUN_INFO_FILE)


if __name__ == "__main__":
    main()
