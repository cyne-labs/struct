#!/usr/bin/env python3
"""
Evaluator and report generator for Struct Milestone R4: retrieval accuracy across formats.

Follows RULES.md and R4.md:
- Normalized match: lowercase, trim, strip surrounding quotes, collapse whitespace.
- No value: when true answer is no value, any of null, none, ~, n/a, or empty counts as correct.
- List answers: comma-separated, compared as a set (order ignored). Every item must match, nothing extra.
- No partial credit: missing or unparseable answer counts as wrong.
- Thinking budget exhausted: scored as given, counted and reported per format.
- Generates research/r4-retrieval.md with all required metrics and wrong answers table.
"""

import os
import sys
import re
import json
from collections import defaultdict

BENCH_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STRUCT_ROOT = os.path.dirname(BENCH_DIR)
R4_DIR = os.path.join(BENCH_DIR, "r4")
sys.path.insert(0, BENCH_DIR)

from r4.formats import FORMAT_DISPLAY_NAMES, FORMATS

QUESTIONS_PATH = os.path.join(R4_DIR, "questions.json")
REAL_DIR = os.path.join(R4_DIR, "real")
RAW_DIR = os.path.join(REAL_DIR, "raw")
METADATA_FILE = os.path.join(REAL_DIR, "metadata.json")
REPORT_FILE = os.path.join(STRUCT_ROOT, "research", "r4-retrieval.md")

SERVER_PROPS_FILE = os.path.join(REAL_DIR, "server_props.json")
LAUNCH_LINE_FILE = os.path.join(REAL_DIR, "launch_line.txt")

LAUNCH_LINE = (
    r'"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\llama-server.exe" -m '
    r'"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\models\lmstudio-community\gemma-4-E2B-it-GGUF\gemma-4-E2B-it-Q4_K_M.gguf" '
    r'-ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 65536 -np 4 --kv-unified '
    r'--reasoning-budget 8192 --temp 1.0 --top-p 0.95 --top-k 64 --min-p 0.0 '
    r'--presence-penalty 0.0 --repeat-penalty 1.0 --port 8090 --host 127.0.0.1'
)

def get_recorded_launch_line(metadata_list=None, launch_line_file=None):
    ll_file = launch_line_file or LAUNCH_LINE_FILE
    if os.path.exists(ll_file):
        try:
            with open(ll_file, "r", encoding="utf-8") as f:
                content = f.read().strip()
                if content:
                    return content
        except Exception:
            pass
    if metadata_list:
        for m in metadata_list:
            if m.get("server_launch_flags"):
                return m["server_launch_flags"]
    if os.path.exists(SERVER_PROPS_FILE):
        try:
            with open(SERVER_PROPS_FILE, "r", encoding="utf-8") as f:
                sp = json.load(f)
                if sp.get("launch_flags"):
                    return sp["launch_flags"]
        except Exception:
            pass
    return LAUNCH_LINE

NO_VALUE_TOKENS = {"null", "none", "~", "n/a", "na", "not applicable"}
EMPTY_LIST_TOKENS = {"none", "empty", "[]", "null", "~", "n/a", "na", "not applicable", "no subnotes", "no notes"}
NULL_TRUTH_TOKENS = {"null", "~", "n/a", "na", "not applicable"}
EMPTY_LIST_TRUTH_TOKENS = {"none", "empty", "[]", "no subnotes", "no notes"}
BOOL_TRUTHY = {"true", "yes", "1"}
BOOL_FALSEY = {"false", "no", "0"}

def normalize_text(text):
    if text is None:
        return ""
    text = str(text).strip()
    prev = None
    # Strip trailing periods, markdown bold/italics and surrounding quotes iteratively
    while text != prev:
        prev = text
        text = text.strip()
        # Strip one trailing period if at the end (preserve internal dots like in arXiv IDs)
        if text.endswith(".") and not re.match(r"^-?\d+\.\d+$", text):
            text = text[:-1].strip()
        # Strip markdown bold (**)
        if text.startswith("**") and text.endswith("**") and len(text) >= 4:
            inner = text[2:-2]
            if "**" not in inner:
                text = inner.strip()
        # Strip markdown bold (__)
        elif text.startswith("__") and text.endswith("__") and len(text) >= 4:
            inner = text[2:-2]
            if "__" not in inner:
                text = inner.strip()
        # Strip markdown italic (*)
        elif text.startswith("*") and text.endswith("*") and len(text) >= 2:
            inner = text[1:-1]
            if "*" not in inner:
                text = inner.strip()
        # Strip markdown italic (_)
        elif text.startswith("_") and text.endswith("_") and len(text) >= 2:
            inner = text[1:-1]
            if "_" not in inner:
                text = inner.strip()
        # Strip surrounding quotes and backticks
        text = re.sub(r'^["\'“”‘’`]+|["\'“”‘’`]+$', '', text).strip()

    # Lowercase and collapse multiple whitespace
    text = re.sub(r'\s+', ' ', text).lower().strip()
    return text

def is_no_value(text):
    norm = normalize_text(text)
    return norm in NO_VALUE_TOKENS

def parse_model_answers(raw_text, expected_count=12):
    """
    Parses a numbered list from model response:
    1. answer
    2. answer
    Handles markdown bold/italics in the number prefix: **1.** or *1.*
    Returns a dict {question_index: answer_str} (1-based index).
    """
    answers = {}
    if not raw_text:
        return answers

    lines = raw_text.splitlines()

    # Pattern for numbered list: "1. answer", "**1.** answer", "**1**. answer", "*1.* answer", "1) answer", etc.
    # Uses backreference to ensure markdown formatting wrapping the number is matched only when opened before it
    num_pattern = re.compile(r'^\s*(?P<pre>[\*_]{1,2})?\s*(?P<num>\d+)(?P=pre)?\s*[\.\)\:\-](?P=pre)?\s*(?P<ans>.*)$')

    for line in lines:
        line_clean = line.strip()
        if not line_clean:
            continue
        m = num_pattern.match(line_clean)
        if m:
            idx = int(m.group('num'))
            ans = m.group('ans').strip()
            if 1 <= idx <= expected_count:
                answers[idx] = ans

    # If regex missed some or model wrote without numbers, fallback to positional if exact line count
    if len(answers) < expected_count:
        non_empty = [l.strip() for l in lines if l.strip() and not l.lower().startswith(("here are", "answers:", "below"))]
        if len(non_empty) == expected_count and not answers:
            for i, line in enumerate(non_empty, start=1):
                m = num_pattern.match(line)
                answers[i] = m.group('ans').strip() if m else line

    return answers

def score_answer(expected, actual, kind=None):
    """
    Scores an answer according to R4.md rules and Review Decisions 9-11:
    - Normalized match (stripping markdown bold/italics and one trailing period)
    - Blank or missing answer is ALWAYS wrong, even when expected is no value (Decision 9)
    - No value rules: only explicit null words count (Decision 9)
    - Empty list rules: accepts none, empty, [], [0], and null words (Decision 11)
    - Several defensible forms: yes/no, true/false, 1/0 for booleans (Decision 11)
    - Comma-separated list set match
    - No partial credit
    Returns (is_correct: bool, norm_expected: str, norm_actual: str)
    """
    norm_exp = normalize_text(expected)
    norm_act = normalize_text(actual)

    # Blank or missing actual answer is ALWAYS wrong (Decision 9)
    if not norm_act:
        return False, norm_exp, norm_act

    # 1. Null truths (Decision 9, 11: accept ONLY explicit null words: null, none, ~, n/a)
    # Does NOT accept empty list words like [], empty, [0]
    if norm_exp in NULL_TRUTH_TOKENS:
        correct = (norm_act in NO_VALUE_TOKENS)
        return correct, norm_exp, norm_act

    # 2. Empty list check (Decision 11: accept none, empty, [], and null words; no [0])
    if norm_exp in EMPTY_LIST_TRUTH_TOKENS or (kind == "null_missing" and norm_exp in EMPTY_LIST_TOKENS):
        correct = (norm_act in EMPTY_LIST_TOKENS)
        return correct, norm_exp, norm_act

    # 3. List answer check (if expected contains comma)
    # Decision 10: strip bold/italics wrapping a whole list answer too
    if "," in str(expected):
        exp_items = {normalize_text(item) for item in str(expected).split(",") if normalize_text(item)}
        act_items = {normalize_text(item) for item in norm_act.split(",") if normalize_text(item)}
        correct = (exp_items == act_items)
        return correct, norm_exp, norm_act

    # 4. Scalar boolean handling (Decision 11: accept yes/no, true/false, 1/0)
    # Exclude count_filter questions where 1 or 0 is an integer count
    if kind != "count_filter":
        if norm_exp in BOOL_TRUTHY:
            return (norm_act in BOOL_TRUTHY), norm_exp, norm_act
        if norm_exp in BOOL_FALSEY:
            return (norm_act in BOOL_FALSEY), norm_exp, norm_act

    # 5. Standard scalar match
    correct = (norm_exp == norm_act)
    return correct, norm_exp, norm_act

def evaluate_runs(metadata_list, questions_dict, raw_dir=RAW_DIR):
    """
    Evaluates all completed runs in metadata_list.
    Returns detailed metrics dict, list of wrong answers, and list of failed runs.
    Per R4.md and Defect 1: A request that is still failed when the report is generated
    counts all its questions as wrong in every accuracy total (never dropped from the denominator),
    and failed runs are tracked per format.
    """
    results = {
        "by_format": defaultdict(lambda: {"correct": 0, "total": 0, "runs": 0, "input_tokens": [], "thinking_tokens": [], "budget_exhausted": 0, "failed_runs": 0}),
        "by_dataset": defaultdict(lambda: {"correct": 0, "total": 0}),
        "by_kind": defaultdict(lambda: {"correct": 0, "total": 0}),
        "by_format_dataset": defaultdict(lambda: {"correct": 0, "total": 0}),
        "by_format_kind": defaultdict(lambda: {"correct": 0, "total": 0}),
        "by_format_seed": defaultdict(lambda: defaultdict(lambda: {"correct": 0, "total": 0})),
        "wrong_answers": [],
        "failed_runs": [],
    }

    # Group all metadata attempts by (format, dataset, seed)
    # Record every failed attempt in results["failed_runs"] for failure history
    all_runs_by_key = defaultdict(list)
    for meta in metadata_list:
        fmt = meta["format"]
        ds = meta["dataset"]
        seed = meta["seed"]
        finish_reason = meta.get("finish_reason", "unknown")
        is_failed = (
            (meta.get("success") is False)
            or str(finish_reason).startswith("error")
            or bool(meta.get("error"))
        )
        if is_failed:
            results["failed_runs"].append(meta)
        all_runs_by_key[(fmt, ds, seed)].append(meta)

    successful_runs = {}
    still_failed_runs = {}
    for key, metas in all_runs_by_key.items():
        success_meta = None
        for m in reversed(metas):
            f_reason = m.get("finish_reason", "unknown")
            failed = (
                (m.get("success") is False)
                or str(f_reason).startswith("error")
                or bool(m.get("error"))
            )
            if not failed:
                success_meta = m
                break
        if success_meta:
            successful_runs[key] = success_meta
        else:
            still_failed_runs[key] = metas[-1]

    # Process successful runs
    for (fmt, ds, seed), meta in successful_runs.items():
        fname = meta["file"]
        finish_reason = meta.get("finish_reason", "unknown")
        thinking_toks = meta.get("thinking_tokens", 0)

        # Check thinking budget exhausted: per Decision 13, counted from thinking tokens reaching 8192 budget
        is_budget_hit = (thinking_toks >= 8192) or (finish_reason == "length")
        if is_budget_hit:
            results["by_format"][fmt]["budget_exhausted"] += 1

        results["by_format"][fmt]["runs"] += 1
        if "input_tokens" in meta and meta["input_tokens"] is not None:
            results["by_format"][fmt]["input_tokens"].append(meta["input_tokens"])
        if "thinking_tokens" in meta and meta["thinking_tokens"] is not None:
            results["by_format"][fmt]["thinking_tokens"].append(meta["thinking_tokens"])

        qs = questions_dict.get(ds, [])
        if not qs:
            continue

        raw_path = os.path.join(raw_dir, fname)
        content = ""
        if os.path.exists(raw_path):
            with open(raw_path, "r", encoding="utf-8") as f:
                content = f.read()

        parsed_answers = parse_model_answers(content, expected_count=len(qs))

        for q_idx, q in enumerate(qs, start=1):
            expected = q["ground_truth"]
            actual = parsed_answers.get(q_idx, "")
            kind = q.get("kind", "unknown")

            correct, norm_exp, norm_act = score_answer(expected, actual, kind=kind)

            # Update counters
            inc = 1 if correct else 0
            results["by_format"][fmt]["correct"] += inc
            results["by_format"][fmt]["total"] += 1

            results["by_dataset"][ds]["correct"] += inc
            results["by_dataset"][ds]["total"] += 1

            results["by_kind"][kind]["correct"] += inc
            results["by_kind"][kind]["total"] += 1

            results["by_format_dataset"][(fmt, ds)]["correct"] += inc
            results["by_format_dataset"][(fmt, ds)]["total"] += 1

            results["by_format_kind"][(fmt, kind)]["correct"] += inc
            results["by_format_kind"][(fmt, kind)]["total"] += 1

            results["by_format_seed"][fmt][seed]["correct"] += inc
            results["by_format_seed"][fmt][seed]["total"] += 1

            if not correct:
                results["wrong_answers"].append({
                    "format": fmt,
                    "dataset": ds,
                    "seed": seed,
                    "q_idx": q_idx,
                    "kind": kind,
                    "question": q["question"],
                    "expected": expected,
                    "model_answer": actual,
                })

    # Defect 1: A request that is still failed when the report is generated must count all its questions
    # as wrong in every accuracy total (never dropped from the denominator), and the main accuracy table
    # must show a 'failed runs' count per format.
    for (fmt, ds, seed), meta in still_failed_runs.items():
        fname = meta.get("file", "")
        finish_reason = meta.get("finish_reason", "unknown")
        thinking_toks = meta.get("thinking_tokens", 0)

        is_budget_hit = (thinking_toks >= 8192) or (finish_reason == "length")
        if is_budget_hit:
            results["by_format"][fmt]["budget_exhausted"] += 1

        results["by_format"][fmt]["runs"] += 1
        results["by_format"][fmt]["failed_runs"] += 1
        if "input_tokens" in meta and meta["input_tokens"] is not None:
            results["by_format"][fmt]["input_tokens"].append(meta["input_tokens"])
        if "thinking_tokens" in meta and meta["thinking_tokens"] is not None:
            results["by_format"][fmt]["thinking_tokens"].append(meta["thinking_tokens"])

        qs = questions_dict.get(ds, [])
        if not qs:
            continue

        raw_path = os.path.join(raw_dir, fname) if fname else ""
        content = ""
        if raw_path and os.path.exists(raw_path):
            with open(raw_path, "r", encoding="utf-8") as f:
                content = f.read()

        parsed_answers = parse_model_answers(content, expected_count=len(qs)) if content else {}

        for q_idx, q in enumerate(qs, start=1):
            expected = q["ground_truth"]
            actual = parsed_answers.get(q_idx, "")
            kind = q.get("kind", "unknown")

            # Still-failed run: answer is missing / wrong, denominator incremented across all totals
            inc = 0
            results["by_format"][fmt]["total"] += 1
            results["by_dataset"][ds]["total"] += 1
            results["by_kind"][kind]["total"] += 1
            results["by_format_dataset"][(fmt, ds)]["total"] += 1
            results["by_format_kind"][(fmt, kind)]["total"] += 1
            results["by_format_seed"][fmt][seed]["total"] += 1

            results["wrong_answers"].append({
                "format": fmt,
                "dataset": ds,
                "seed": seed,
                "q_idx": q_idx,
                "kind": kind,
                "question": q["question"],
                "expected": expected,
                "model_answer": actual,
            })

    return results

def generate_report(results, report_file=REPORT_FILE, metadata_list=None, launch_line_file=None):
    """Generates the Markdown report per R4.md specification."""
    lines = []
    lines.append("# R4: Retrieval Accuracy Across Formats\n\n")
    lines.append("Benchmark measuring model comprehension across 7 data serialization formats on Gemma 4 E2B.\n\n")

    lines.append("## 1. Setup\n\n")
    lines.append("- **Model:** Gemma 4 E2B (`gemma-4-E2B-it-Q4_K_M.gguf`), thinking mode on (R4 decision 14).\n")
    lines.append("- **Server:** `turboquant-llama-cpp` at `127.0.0.1:8090`, KV cache turbo4 K / turbo3 V.\n")
    launch_cmd = get_recorded_launch_line(metadata_list, launch_line_file=launch_line_file)
    lines.append(f"- **Launch Command Line:**\n  ```\n  {launch_cmd}\n  ```\n")
    lines.append("- **Reasoning budget:** 8192 tokens.\n")
    lines.append("- **Sampling:** temperature 1.0, top_p 0.95, top_k 64, min_p 0.0, presence_penalty 0.0, repeat_penalty 1.0.\n")
    lines.append("- **Seeds:** 42, 43, 44, 45, 46 (5 seeds).\n")
    lines.append("- **Design:** TSV and Plain `key: value` run only on the two flat datasets (per maintainer Decision 1; marked as 'can\\'t hold this shape' for nested datasets).\n\n")

    lines.append("## 2. Standardized Prompt\n\n")
    lines.append("The prompt is identical for every format except the data block and one line naming the format.\n")
    lines.append("Here is the exact full prompt text as sent to the model (example: Struct on flat_tabular):\n\n")
    lines.append("````text\n")
    # Render actual full prompt text using exact collector code
    try:
        from r4.collect import build_prompt, load_dataset, load_questions
        from r4.formats import ENCODERS
        ft_data = load_dataset("flat_tabular")
        ft_questions = load_questions().get("flat_tabular", [])
        encoded_data = ENCODERS["struct"](ft_data, "flat_tabular")
        full_prompt = build_prompt("struct", "flat_tabular", encoded_data, ft_questions)
        lines.append(full_prompt)
    except Exception as e:
        lines.append(f"[Error generating prompt: {e}]\n")
    lines.append("````\n\n")

    lines.append("## 3. Results Summary Across Formats\n\n")
    lines.append("| Format | Accuracy (Counts) | Accuracy (%) | Mean Input Toks | Mean Thinking Toks | Max Thinking Toks | Budget Exhausted | Failed Runs |\n")
    lines.append("|---|---|---|---|---|---|---|---|\n")

    for fmt in FORMATS:
        f_data = results["by_format"][fmt]
        tot = f_data["total"]
        cor = f_data["correct"]
        pct = (cor / tot * 100) if tot > 0 else 0.0
        in_toks = sum(f_data["input_tokens"]) / len(f_data["input_tokens"]) if f_data["input_tokens"] else 0
        th_toks = sum(f_data["thinking_tokens"]) / len(f_data["thinking_tokens"]) if f_data["thinking_tokens"] else 0
        max_th = max(f_data["thinking_tokens"]) if f_data["thinking_tokens"] else 0
        exhaust = f_data["budget_exhausted"]
        failed = f_data.get("failed_runs", 0)
        disp = FORMAT_DISPLAY_NAMES.get(fmt, fmt)
        lines.append(f"| {disp} | {cor}/{tot} | {pct:.1f}% | {in_toks:.0f} | {th_toks:.0f} | {max_th} | {exhaust} | {failed} |\n")

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
    lines.append("- **Failed Requests (Decision 13):** Any request experiencing an HTTP error, timeout, or empty text is recorded as failed and not marked done; resume reruns them.\n")

    failed_runs = results.get("failed_runs", [])
    if failed_runs:
        lines.append(f"\n### Recorded Failed Requests ({len(failed_runs)})\n\n")
        lines.append("| Run Index | Format | Dataset | Seed | Error |\n")
        lines.append("|---|---|---|---|---|\n")
        for fr in failed_runs:
            lines.append(f"| {fr.get('run_index', 'n/a')} | {fr.get('format')} | {fr.get('dataset')} | {fr.get('seed')} | `{fr.get('finish_reason', 'error')}` |\n")

    lines.append("\n## 8. Seed Spread per Format\n\n")
    lines.append("| Format | Seed 42 | Seed 43 | Seed 44 | Seed 45 | Seed 46 | Mean | Spread (Max-Min) |\n")
    lines.append("|---|---|---|---|---|---|---|---|\n")
    for fmt in FORMATS:
        disp = FORMAT_DISPLAY_NAMES.get(fmt, fmt)
        seed_scores = []
        seed_cells = []
        for s in [42, 43, 44, 45, 46]:
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

    results = evaluate_runs(metadata, questions, raw_dir=RAW_DIR)
    generate_report(results, report_file=REPORT_FILE, metadata_list=metadata)

if __name__ == "__main__":
    main()
