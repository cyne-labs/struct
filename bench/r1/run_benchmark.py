r"""
Milestone R1 Benchmark:
Part A: Token re-measurement for Struct v0.2 across Qwen, Llama, Gemma, LFM.
Step 3: Comparative token counting (JSON, TOON, Struct v0, Struct v0.2).
Step 4: Micro-benchmarks for line prefixes and endings:
  - '|' + newline at end of line
  - '>' repeated 1-8 times at start of line, followed by text and by index like '3|'
  - '~' as a whole field between pipes
  - r'\|' inside text
"""

import os
import csv
import json
from typing import List, Dict, Any
from tokenizer import VocabTokenizer, MODELS

BENCH_DIR = os.path.dirname(__file__)
FIXTURES_DIR = os.path.join(BENCH_DIR, "fixtures")
RESULTS_DIR = BENCH_DIR

# -------------------------------------------------------------
# Part A Step 3: Fixture sets
# -------------------------------------------------------------
SHAPES = ["flat_tabular", "nested_fixed", "nested_variable"]

RECORD_COUNTS = {
    "flat_tabular": 20,
    "nested_fixed": 10,
    "nested_variable": 4,
}

FIXTURE_FILES = [
    ("min_json", "{shape}.min.json"),
    ("toon", "{shape}.toon"),
    ("struct_v02_typed", "{shape}.v02.stc"),
    ("struct_v02_untyped", "{shape}.v02.untyped.stc"),
    ("struct_v02_values_only", "{shape}.v02.values_only.stc"),
    ("struct_v02_schema_only", "{shape}.v02.schema.stc"),
]

def run_fixture_benchmarks(tok_instances: Dict[str, VocabTokenizer]) -> List[Dict[str, Any]]:
    results = []
    print("\n========================================================")
    print("PART A - STEP 3: DATASET FIXTURES BENCHMARK")
    print("========================================================")

    for shape in SHAPES:
        for fmt_name, file_pattern in FIXTURE_FILES:
            filename = file_pattern.format(shape=shape)
            filepath = os.path.join(FIXTURES_DIR, filename)
            if not os.path.exists(filepath):
                print(f"Skipping missing fixture: {filepath}")
                continue
            with open(filepath, "r", encoding="utf-8") as f:
                content = f.read()

            for family, tok in tok_instances.items():
                n_tokens, token_ids, pieces = tok.tokenize(content)
                results.append({
                    "family": family,
                    "shape": shape,
                    "format": fmt_name,
                    "filename": filename,
                    "bytes": len(content.encode("utf-8")),
                    "chars": len(content),
                    "token_count": n_tokens,
                    "tokens_per_char": round(n_tokens / len(content), 4) if len(content) > 0 else 0,
                })
                print(f"[{family:5}] {shape:15} | {fmt_name:22} | {n_tokens:5} tokens | {len(content):5} chars")

    csv_path = os.path.join(RESULTS_DIR, "results_r1_fixtures.csv")
    with open(csv_path, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=["family", "shape", "format", "filename", "bytes", "chars", "token_count", "tokens_per_char"])
        writer.writeheader()
        writer.writerows(results)
    print(f"\nSaved fixture results to {csv_path}")
    return results


# -------------------------------------------------------------
# Part A Step 4: Line prefixes and endings micro-measurements
# -------------------------------------------------------------
def run_pattern_benchmarks(tok_instances: Dict[str, VocabTokenizer]) -> List[Dict[str, Any]]:
    print("\n========================================================")
    print("PART A - STEP 4: LINE PREFIXES & ENDINGS MICRO-BENCHMARK")
    print("========================================================")

    results = []

    # 1. '|' + newline at end of line
    # Measure in isolation and in realistic line contexts
    pipe_newline_cases = [
        ("pipe_newline_isolated", "|\n"),
        ("newline_isolated", "\n"),
        ("line_with_pipe_nl", "2405.05254|Rope scaling hurts long ctx|refuted|2024|0|\n"),
        ("line_without_pipe_nl", "2405.05254|Rope scaling hurts long ctx|refuted|2024|0\n"),
        ("short_word_with_pipe_nl", "refuted|\n"),
        ("short_word_with_nl", "refuted\n"),
        ("num_with_pipe_nl", "2024|\n"),
        ("num_with_nl", "2024\n"),
    ]

    # 2. '>' repeated 1-8 times at start of line
    # Followed by text, and followed by index like '3|'
    depth_cases = []
    sample_text = "notes are from abstract only"
    for d in range(1, 9):
        prefix = ">" * d
        # Pure prefix
        depth_cases.append((f"depth_{d}_marker_only", prefix))
        # Prefix followed by text
        depth_cases.append((f"depth_{d}_text", f"{prefix}{sample_text}|"))
        # Prefix followed by index like '3|' + text
        depth_cases.append((f"depth_{d}_index_3", f"{prefix}3|{sample_text}|"))
        # Prefix followed by dotted index like '1.2|' + text
        depth_cases.append((f"depth_{d}_index_dotted", f"{prefix}1.2|{sample_text}|"))

    # 3. '~' as a whole field between pipes
    tilde_cases = [
        ("tilde_isolated", "~"),
        ("pipe_tilde_pipe_isolated", "|~|"),
        ("pipe_empty_pipe_isolated", "||"),
        ("field_tilde_between_pipes", "refuted|~|2024|"),
        ("field_empty_between_pipes", "refuted||2024|"),
        ("field_value_between_pipes", "refuted|unverified|2024|"),
        ("three_tildes_fields", "|~|~|~|"),
        ("three_empty_fields", "||||"),
    ]

    # 4. r'\|' inside text
    escape_cases = [
        ("escape_pipe_isolated", r"\|"),
        ("pipe_isolated", "|"),
        ("backslash_isolated", "\\"),
        ("text_with_escape_pipe", r"Rope \| scaling"),
        ("text_with_plain_space", "Rope   scaling"),
        ("text_with_literal_pipe", "Rope | scaling"),
        ("code_with_escape_pipe", r"if a \|\| b:"),
        ("inside_word_escape_pipe", r"pipe\|inside"),
        ("inside_word_plain_pipe", "pipe|inside"),
    ]

    all_test_groups = [
        ("pipe_newline", pipe_newline_cases),
        ("child_markers_depth", depth_cases),
        ("tilde_null_field", tilde_cases),
        ("escape_pipe", escape_cases),
    ]

    for group_name, cases in all_test_groups:
        for case_name, test_str in cases:
            for family, tok in tok_instances.items():
                n_tokens, token_ids, pieces = tok.tokenize(test_str)
                results.append({
                    "family": family,
                    "group": group_name,
                    "case": case_name,
                    "test_string": test_str,
                    "token_count": n_tokens,
                    "token_ids": str(token_ids),
                    "pieces": str(pieces),
                })

    csv_path = os.path.join(RESULTS_DIR, "results_r1_micro.csv")
    with open(csv_path, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=["family", "group", "case", "test_string", "token_count", "token_ids", "pieces"])
        writer.writeheader()
        writer.writerows(results)
    print(f"\nSaved micro-benchmark results to {csv_path}")
    return results


def main():
    tok_instances = {}
    try:
        for family, path in MODELS.items():
            print(f"Loading vocabulary for {family} from {path}...")
            tok_instances[family] = VocabTokenizer(family, path)

        run_fixture_benchmarks(tok_instances)
        run_pattern_benchmarks(tok_instances)

    finally:
        for tok in tok_instances.values():
            tok.close()

if __name__ == "__main__":
    main()
