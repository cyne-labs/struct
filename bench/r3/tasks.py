#!/usr/bin/env python3
"""
Source notes and ground truth for Struct v0.3 real model benchmark (R3c Part C.2).
"""

import os
import subprocess

STRUCT_ROOT = r"C:\Users\Ashman Das\Documents\Struct"
DOCTOR_TOOL = os.path.join(STRUCT_ROOT, "target", "debug", "doctor_tool.exe")

S1_NOTES = """1. Paper 2405.05254 was published in 2024 with the title "Rope scaling hurts long ctx"; it turned out to be false, so the claim is refuted.
2. In 2017, paper 1706.03762 was published with the title "Attention is all you need"; verification was true, and the claim is supported.
3. Paper 2106.09685, titled "LoRA low rank adaptation", appeared in 2021; it was verified as true, so the claim is supported.
4. Titled "Mistral 7B architecture", paper 2309.06180 was published in 2023; verification was true, and the claim is supported.
5. Paper 2401.04088 was published in 2024 under the title "DeepSeekMoE mixture experts"; it was verified as true, and the claim is supported.
Write these as Struct records."""

S1_GROUND_TRUTH = [
    {"id": "2405.05254", "title": "Rope scaling hurts long ctx", "verdict": "refuted", "year": 2024, "verified": False},
    {"id": "1706.03762", "title": "Attention is all you need", "verdict": "supported", "year": 2017, "verified": True},
    {"id": "2106.09685", "title": "LoRA low rank adaptation", "verdict": "supported", "year": 2021, "verified": True},
    {"id": "2309.06180", "title": "Mistral 7B architecture", "verdict": "supported", "year": 2023, "verified": True},
    {"id": "2401.04088", "title": "DeepSeekMoE mixture experts", "verdict": "supported", "year": 2024, "verified": True},
]

S2_NOTES = """1. Paper 2405.05254 claimed "Rope scaling hurts long ctx" in 2024; it turned out to be false, so the claim is refuted.
2. In 2017, paper 1706.03762 claimed "Attention is all you need"; verification was true, so the claim is supported.
3. Paper 2106.09685 made the claim "LoRA low rank adaptation" in 2021; it was verified as true, and the claim is supported.
4. In 2023, paper 2309.06180 claimed "Mistral 7B architecture"; verification was true, and the claim is supported.
5. Paper 2401.04088 made the claim "DeepSeekMoE mixture experts" in 2024; it was verified as true, and the claim is supported.
Write these as Struct records."""

S2_GROUND_TRUTH = [
    {"text": "Rope scaling hurts long ctx", "verdict": "refuted", "source": {"id": "2405.05254", "year": 2024}, "verified": False},
    {"text": "Attention is all you need", "verdict": "supported", "source": {"id": "1706.03762", "year": 2017}, "verified": True},
    {"text": "LoRA low rank adaptation", "verdict": "supported", "source": {"id": "2106.09685", "year": 2021}, "verified": True},
    {"text": "Mistral 7B architecture", "verdict": "supported", "source": {"id": "2309.06180", "year": 2023}, "verified": True},
    {"text": "DeepSeekMoE mixture experts", "verdict": "supported", "source": {"id": "2401.04088", "year": 2024}, "verified": True},
]

S3_NOTES = """1. Paper 2405.05254 claimed "Rope scaling hurts long ctx" in 2024, but the claim was refuted. A note says "notes are from abstract only", with two subnotes: "abstract mentions 32k limit" and "appendix B clarifies". A second note says "table 3 contradicts", with the subnote "llama 7b tested only".
2. In 2021, paper 2106.09685 made the claim "LoRA matches full finetune", and the verdict is supported. The first note is "tested on GLUE and Wikisql", with subnotes "rank 4 sufficient" and "higher ranks show diminishing returns". The second note is "memory footprint reduced 3x", with the subnote "optimizer state minimal". The third note is "checkpoint size under 50MB", without any subnotes.
3. Paper 2406.12345 made the claim "KV cache quant loses 2x acc" in 2024, but the verdict is refuted. It has a note "outlier channels preserved", with subnotes "per-head scales help" and "perplexity intact".
4. Published in 2022, paper 2203.15556 claimed "Chinchilla tokens beat params", which is supported. One note states "optimal ratio is 20 to 1", with subnotes "tested across 400 models" and "cosine schedule used". Another note states "kaplan tokens were suboptimal", with the subnote "fixed learning rate flawed".
5. Paper 2005.14165 from 2020 claimed "Language models few-shot", and the claim is supported. There is a note "scaling curve is smooth", with subnotes "tested up to 175B" and "in-context learning observed".
Write these as Struct records."""

S3_GROUND_TRUTH = [
    {
        "text": "Rope scaling hurts long ctx",
        "verdict": "refuted",
        "source": {"id": "2405.05254", "year": 2024},
        "notes": [
            {"text": "notes are from abstract only", "subnotes": ["abstract mentions 32k limit", "appendix B clarifies"]},
            {"text": "table 3 contradicts", "subnotes": ["llama 7b tested only"]},
        ],
    },
    {
        "text": "LoRA matches full finetune",
        "verdict": "supported",
        "source": {"id": "2106.09685", "year": 2021},
        "notes": [
            {"text": "tested on GLUE and Wikisql", "subnotes": ["rank 4 sufficient", "higher ranks show diminishing returns"]},
            {"text": "memory footprint reduced 3x", "subnotes": ["optimizer state minimal"]},
            {"text": "checkpoint size under 50MB", "subnotes": []},
        ],
    },
    {
        "text": "KV cache quant loses 2x acc",
        "verdict": "refuted",
        "source": {"id": "2406.12345", "year": 2024},
        "notes": [
            {"text": "outlier channels preserved", "subnotes": ["per-head scales help", "perplexity intact"]},
        ],
    },
    {
        "text": "Chinchilla tokens beat params",
        "verdict": "supported",
        "source": {"id": "2203.15556", "year": 2022},
        "notes": [
            {"text": "optimal ratio is 20 to 1", "subnotes": ["tested across 400 models", "cosine schedule used"]},
            {"text": "kaplan tokens were suboptimal", "subnotes": ["fixed learning rate flawed"]},
        ],
    },
    {
        "text": "Language models few-shot",
        "verdict": "supported",
        "source": {"id": "2005.14165", "year": 2020},
        "notes": [
            {"text": "scaling curve is smooth", "subnotes": ["tested up to 175B", "in-context learning observed"]},
        ],
    },
]

S4_NOTES = """1. The file "lib.rs" is at path "src/lib.rs". At line 10, a patch for "lib.rs" has the code:
"pub fn add(a: i32, b: i32) -> i32 {
    a + b
}"
2. The file "util.rs" is at path "src/util.rs". At line 5, a patch for "util.rs" has the code:
"pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}"
3. The file "main.rs" is at path "src/main.rs". At line 1, a patch for "main.rs" has the code:
"fn main() {
    println!("Hello, world!");
}"
Write these as Struct records."""

S4_GROUND_TRUTH = [
    {"class": "file", "name": "lib.rs", "path": "src/lib.rs"},
    {"class": "file", "name": "util.rs", "path": "src/util.rs"},
    {"class": "file", "name": "main.rs", "path": "src/main.rs"},
    {"class": "patch", "file_name": "lib.rs", "code": "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}", "line": 10},
    {"class": "patch", "file_name": "util.rs", "code": "pub fn greet(name: &str) -> String {\n    format!(\"Hello, {}!\", name)\n}", "line": 5},
    {"class": "patch", "file_name": "main.rs", "code": "fn main() {\n    println!(\"Hello, world!\");\n}", "line": 1},
]

TASKS = {
    "s1_flat": {
        "schema_file": os.path.join(STRUCT_ROOT, "bench", "r2", "schemas", "s1_flat.stc"),
        "notes": S1_NOTES,
        "ground_truth": S1_GROUND_TRUTH,
    },
    "s2_nested_fixed": {
        "schema_file": os.path.join(STRUCT_ROOT, "bench", "r2", "schemas", "s2_nested_fixed.stc"),
        "notes": S2_NOTES,
        "ground_truth": S2_GROUND_TRUTH,
    },
    "s3_nested_variable": {
        "schema_file": os.path.join(STRUCT_ROOT, "bench", "r2", "schemas", "s3_nested_variable.stc"),
        "notes": S3_NOTES,
        "ground_truth": S3_GROUND_TRUTH,
    },
    "s4_two_class_code": {
        "schema_file": os.path.join(STRUCT_ROOT, "bench", "r2", "schemas", "s4_two_class_code.stc"),
        "notes": S4_NOTES,
        "ground_truth": S4_GROUND_TRUTH,
    },
}

def get_task_prompt(schema_key: str) -> str:
    task = TASKS[schema_key]
    # Build prompt(schemas) via doctor_tool
    base_prompt = subprocess.check_output([DOCTOR_TOOL, "prompt", task["schema_file"]], text=True)
    # SPEC: exactly prompt(schemas), then one blank line, then the task
    # base_prompt already ends with a newline, so + '\n' gives one blank line
    return base_prompt + "\n" + task["notes"]

if __name__ == "__main__":
    for k in TASKS:
        p = get_task_prompt(k)
        print(f"=== {k} full prompt ({len(p)} chars) ===")
        print(p)
        print("-" * 60)
