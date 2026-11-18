"""
Analysis script for Milestone R1 results.
Produces summary tables comparing M0 vs v0.2, and breaks down micro-benchmarks.
"""

import os
import csv
from collections import defaultdict

BENCH_DIR = os.path.dirname(__file__)

def analyze_fixtures():
    fixtures_csv = os.path.join(BENCH_DIR, "results_r1_fixtures.csv")
    with open(fixtures_csv, "r", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))

    # M0 baseline data from struct-py/bench/m0/results_baseline.csv
    m0_data = {
        ("flat_tabular", "min_json"): {"Qwen": 759, "Llama": 617, "Gemma": 756, "LFM": 661},
        ("flat_tabular", "toon"): {"Qwen": 564, "Llama": 421, "Gemma": 570, "LFM": 428},
        ("flat_tabular", "struct_v0_values"): {"Qwen": 534, "Llama": 392, "Gemma": 534, "LFM": 420},
        ("flat_tabular", "struct_v0_with_schema"): {"Qwen": 556, "Llama": 413, "Gemma": 557, "LFM": 443},

        ("nested_fixed", "min_json"): {"Qwen": 397, "Llama": 327, "Gemma": 396, "LFM": 350},
        ("nested_fixed", "toon"): {"Qwen": 279, "Llama": 208, "Gemma": 292, "LFM": 215},
        ("nested_fixed", "struct_v0_values"): {"Qwen": 265, "Llama": 195, "Gemma": 264, "LFM": 209},
        ("nested_fixed", "struct_v0_with_schema"): {"Qwen": 290, "Llama": 219, "Gemma": 291, "LFM": 236},

        ("nested_variable", "min_json"): {"Qwen": 337, "Llama": 304, "Gemma": 323, "LFM": 304},
        ("nested_variable", "toon"): {"Qwen": 410, "Llama": 367, "Gemma": 404, "LFM": 370},
        ("nested_variable", "struct_v0_values"): {"Qwen": 249, "Llama": 216, "Gemma": 239, "LFM": 221},
        ("nested_variable", "struct_v0_with_schema"): {"Qwen": 283, "Llama": 249, "Gemma": 279, "LFM": 260},
    }

    # Aggregate v0.2 data
    v02_data = defaultdict(dict)
    for r in rows:
        key = (r["shape"], r["format"])
        v02_data[key][r["family"]] = int(r["token_count"])

    families = ["Qwen", "Llama", "Gemma", "LFM"]

    print("## FIXTURE COMPARISON TABLES\n")
    for shape in ["flat_tabular", "nested_fixed", "nested_variable"]:
        print(f"### Shape: {shape}")
        print("| Format | Qwen | Llama | Gemma | LFM | Mean | vs TOON | vs Min JSON | vs Struct v0 (with schema) |")
        print("|---|---|---|---|---|---|---|---|---|")

        # TOON mean
        toon_toks = [v02_data[(shape, "toon")][f] for f in families]
        toon_mean = sum(toon_toks) / len(toon_toks)

        # Min JSON mean
        json_toks = [v02_data[(shape, "min_json")][f] for f in families]
        json_mean = sum(json_toks) / len(json_toks)

        # M0 v0 with schema mean
        v0_toks = [m0_data[(shape, "struct_v0_with_schema")][f] for f in families]
        v0_mean = sum(v0_toks) / len(v0_toks)

        formats_to_show = [
            ("Min JSON", v02_data[(shape, "min_json")]),
            ("TOON", v02_data[(shape, "toon")]),
            ("Struct v0 (values only, M0)", m0_data[(shape, "struct_v0_values")]),
            ("Struct v0 (+ schema, M0)", m0_data[(shape, "struct_v0_with_schema")]),
            ("Struct v0.2 (values only)", v02_data[(shape, "struct_v02_values_only")]),
            ("Struct v0.2 (+ typed schema)", v02_data[(shape, "struct_v02_typed")]),
            ("Struct v0.2 (+ untyped schema)", v02_data[(shape, "struct_v02_untyped")]),
        ]

        for label, model_dict in formats_to_show:
            toks = [model_dict[f] for f in families]
            mean_val = sum(toks) / len(toks)
            vs_toon = f"{(mean_val - toon_mean) / toon_mean * 100:+.1f}%"
            vs_json = f"{(mean_val - json_mean) / json_mean * 100:+.1f}%"
            vs_v0 = f"{(mean_val - v0_mean) / v0_mean * 100:+.1f}%"
            print(f"| {label} | {toks[0]} | {toks[1]} | {toks[2]} | {toks[3]} | {mean_val:.1f} | {vs_toon} | {vs_json} | {vs_v0} |")
        print()

def analyze_micro():
    micro_csv = os.path.join(BENCH_DIR, "results_r1_micro.csv")
    with open(micro_csv, "r", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))

    # Group by group and case
    data = defaultdict(lambda: defaultdict(dict))
    pieces_data = defaultdict(lambda: defaultdict(dict))
    for r in rows:
        data[r["group"]][r["case"]][r["family"]] = int(r["token_count"])
        pieces_data[r["group"]][r["case"]][r["family"]] = r["pieces"]

    print("## MICRO-BENCHMARK RESULTS\n")
    families = ["Qwen", "Llama", "Gemma", "LFM"]

    for group in ["pipe_newline", "child_markers_depth", "tilde_null_field", "escape_pipe"]:
        print(f"### Group: {group}")
        print("| Case | Qwen | Llama | Gemma | LFM | Mean | Pieces (Qwen / Llama / Gemma / LFM) |")
        print("|---|---|---|---|---|---|---|")
        for case, family_dict in data[group].items():
            toks = [family_dict[f] for f in families]
            mean_val = sum(toks) / len(toks)
            p_str = " / ".join(pieces_data[group][case][f] for f in families)
            print(f"| {case} | {toks[0]} | {toks[1]} | {toks[2]} | {toks[3]} | {mean_val:.2f} | `{p_str[:80]}...` |")
        print()

if __name__ == "__main__":
    analyze_fixtures()
    analyze_micro()
