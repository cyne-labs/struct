"""
Deterministic dataset generator for Struct Milestone R4.
Generates:
1. flat_100.min.json: bigger flat dataset (~100 rows) with fixed seed (42).
2. flat_tabular.min.json: R4 copy of flat_tabular with a few nulls added (seed 42).
3. nested_fixed.min.json: R4 copy of nested_fixed with a few nulls added (seed 42).

Per Decision 6:
- Don't edit bench/r1/fixtures/
- R4 uses its own copies of flat tabular and nested fixed in bench/r4/data/ with a few nulls added (deterministic, fixed seed).
- Null questions on every dataset ask about real empty values in the data, never about records or fields that don't exist.
"""

import os
import json
import random

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
R1_FIXTURES = os.path.join(BASE_DIR, "r1", "fixtures")
DATA_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data")

FLAT_100_PATH = os.path.join(DATA_DIR, "flat_100.min.json")
FLAT_TABULAR_PATH = os.path.join(DATA_DIR, "flat_tabular.min.json")
NESTED_FIXED_PATH = os.path.join(DATA_DIR, "nested_fixed.min.json")

TITLES = [
    "Attention is all you need for sequence modelling",
    "LoRA low rank adaptation of large language models",
    "DeepSeekMoE mixture of experts architecture report",
    "Language models are few-shot learners at scale",
    "StarCoder 2 training code and data mixture",
    "Mistral fine tuning rules and safety guardrails",
    "Chinchilla scaling compute optimal training tokens",
    "Direct preference optimization without reinforcement",
    "Llama 3 herd of models technical overview",
    "Gemini models multimodal reasoning performance",
    "Gemma open models Google lightweight release",
    "Llama 2 foundation chat model benchmarks",
    "ReAct synergizing reasoning and acting in language",
    "Phi 3 technical report compact capability",
    "LLaMA efficient foundational language model",
    "Qwen2 technical report multilingual foundation",
    "Code Llama open model for specialized coding",
    "Nemotron 4 340B synthetic data training report",
    "Rope scaling hurts long context retrieval bounds",
    "KV cache quantization losing accuracy on outliers",
    "BitNet scaling 1-bit transformers for efficiency",
    "Mamba linear time sequence modeling with selective state",
    "FlashAttention fast and memory efficient exact attention",
    "Self-instruct aligning language model with self generated data",
    "Toolformer language models can teach themselves to use tools",
]

VERDICTS = ["supported", "refuted", "unverified"]

def generate_flat_100(seed=42):
    rng = random.Random(seed)
    records = []

    # Exactly 100 rows
    for i in range(1, 101):
        arxiv_month = rng.randint(1, 12)
        arxiv_num = rng.randint(1000, 9999)
        arxiv_id = f"24{arxiv_month:02d}.{arxiv_num:04d}"

        # Choose title
        base_title = rng.choice(TITLES)
        title = f"{base_title} v{i}"

        # Choose verdict
        verdict = rng.choice(VERDICTS)

        # Choose year (2018 - 2024)
        year = rng.randint(2018, 2024)

        # Choose verified
        verified = rng.choice([True, False])

        # Inject known nulls at specific deterministic intervals:
        # Rows ending in 7 (7, 17, 27, 37, 47, 57, 67, 77, 87, 97) have no year
        if i % 10 == 7:
            year = None

        # Rows ending in 3 (3, 13, 23, 33, 43, 53, 63, 73, 83, 93) have no verified status
        if i % 10 == 3:
            verified = None

        # Rows 50 and 100 have no verdict
        if i in (50, 100):
            verdict = None

        rec = {
            "id": arxiv_id,
            "title": title,
            "verdict": verdict,
            "year": year,
            "verified": verified,
        }
        records.append(rec)

    return records

def generate_flat_tabular_r4(seed=42):
    """
    Creates R4 copy of flat_tabular with a few deterministic nulls added.
    Preserves rows 0..3 for standard single lookup questions.
    """
    src_path = os.path.join(R1_FIXTURES, "flat_tabular.min.json")
    with open(src_path, "r", encoding="utf-8") as f:
        records = json.load(f)

    # Deterministic modifications based on seed 42:
    # 4 null values across 20 rows
    records[8]["year"] = None       # 2203.15556: year null
    records[12]["verified"] = None  # 2403.09611: verified null
    records[15]["year"] = None      # 2404.14219: year null
    records[17]["verdict"] = None   # 2408.03314: verdict null

    return records

def generate_nested_fixed_r4(seed=42):
    """
    Creates R4 copy of nested_fixed with a few deterministic nulls added.
    Preserves rows 0, 1, 2, 7, 8 for standard lookup questions.
    """
    src_path = os.path.join(R1_FIXTURES, "nested_fixed.min.json")
    with open(src_path, "r", encoding="utf-8") as f:
        records = json.load(f)

    # Deterministic modifications based on seed 42:
    # 3 null values across 10 rows
    records[4]["source"]["year"] = None  # DeepSeekMoE mixture experts: source year null
    records[6]["verified"] = None        # StarCoder 2 training code: verified null
    records[9]["verdict"] = None         # Direct preference optim: verdict null

    return records

def main():
    os.makedirs(DATA_DIR, exist_ok=True)

    # 1. flat_100
    f100 = generate_flat_100(seed=42)
    with open(FLAT_100_PATH, "w", encoding="utf-8") as f:
        json.dump(f100, f, separators=(',', ':'), ensure_ascii=False)
    print(f"Generated {len(f100)} records -> {FLAT_100_PATH}")

    # 2. flat_tabular (R4 copy with nulls)
    ft = generate_flat_tabular_r4(seed=42)
    with open(FLAT_TABULAR_PATH, "w", encoding="utf-8") as f:
        json.dump(ft, f, separators=(',', ':'), ensure_ascii=False)
    print(f"Generated {len(ft)} records -> {FLAT_TABULAR_PATH}")

    # 3. nested_fixed (R4 copy with nulls)
    nf = generate_nested_fixed_r4(seed=42)
    with open(NESTED_FIXED_PATH, "w", encoding="utf-8") as f:
        json.dump(nf, f, separators=(',', ':'), ensure_ascii=False)
    print(f"Generated {len(nf)} records -> {NESTED_FIXED_PATH}")

if __name__ == "__main__":
    main()
