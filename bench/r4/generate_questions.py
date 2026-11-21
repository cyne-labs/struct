"""
Question and Ground Truth generator for Struct Milestone R4.
Generates bench/r4/questions.json from source data only.
Covers 4 datasets, 12 questions each (48 questions total):
- single lookup
- nested lookup (for nested datasets)
- count or filter
- null or missing (Per Decision 6: asks about real empty values in data only, never non-existent records/fields)
"""

import os
import json

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
R1_FIXTURES = os.path.join(BASE_DIR, "r1", "fixtures")
R4_DATA = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data")
OUTPUT_QUESTIONS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "questions.json")

def load_source_data():
    with open(os.path.join(R4_DATA, "flat_tabular.min.json"), "r", encoding="utf-8") as f:
        flat_tabular = json.load(f)
    with open(os.path.join(R4_DATA, "flat_100.min.json"), "r", encoding="utf-8") as f:
        flat_100 = json.load(f)
    with open(os.path.join(R4_DATA, "nested_fixed.min.json"), "r", encoding="utf-8") as f:
        nested_fixed = json.load(f)
    with open(os.path.join(R1_FIXTURES, "nested_variable.min.json"), "r", encoding="utf-8") as f:
        nested_variable = json.load(f)

    return {
        "flat_tabular": flat_tabular,
        "flat_100": flat_100,
        "nested_fixed": nested_fixed,
        "nested_variable": nested_variable,
    }

def generate_questions(write_file=False):
    data = load_source_data()
    all_questions = {}

    # 1. flat_tabular (20 rows)
    ft = data["flat_tabular"]
    ft_null_year_ids = [r["id"] for r in ft if r.get("year") is None]
    ft_null_ver_ids = [r["id"] for r in ft if r.get("verified") is None]
    ft_null_verd_ids = [r["id"] for r in ft if r.get("verdict") is None]

    ft_questions = [
        # single lookup (4)
        {
            "id": "ft_q01", "kind": "single_lookup",
            "question": f"What is the verdict for paper {ft[0]['id']}?",
            "ground_truth": ft[0]["verdict"],
        },
        {
            "id": "ft_q02", "kind": "single_lookup",
            "question": f"What is the publication year of paper {ft[1]['id']}?",
            "ground_truth": str(ft[1]["year"]),
        },
        {
            "id": "ft_q03", "kind": "single_lookup",
            "question": f"What is the title of paper {ft[2]['id']}?",
            "ground_truth": ft[2]["title"],
        },
        {
            "id": "ft_q04", "kind": "single_lookup",
            "question": f"What is the verification status of paper {ft[3]['id']}?",
            "ground_truth": "true" if ft[3]["verified"] else "false",
        },
        # count or filter (4)
        {
            "id": "ft_q05", "kind": "count_filter",
            "question": "How many records have verdict refuted?",
            "ground_truth": str(sum(1 for r in ft if r.get("verdict") == "refuted")),
        },
        {
            "id": "ft_q06", "kind": "count_filter",
            "question": "How many records have verdict supported?",
            "ground_truth": str(sum(1 for r in ft if r.get("verdict") == "supported")),
        },
        {
            "id": "ft_q07", "kind": "count_filter",
            "question": "How many records were published in 2024?",
            "ground_truth": str(sum(1 for r in ft if r.get("year") == 2024)),
        },
        {
            "id": "ft_q08", "kind": "count_filter",
            "question": "Which paper IDs have verdict unverified?",
            "ground_truth": ", ".join(r["id"] for r in ft if r.get("verdict") == "unverified"),
        },
        # null or missing (4) - all asking about real null values in existing records
        {
            "id": "ft_q09", "kind": "null_missing",
            "question": "Which paper IDs have no publication year?",
            "ground_truth": ", ".join(ft_null_year_ids),
        },
        {
            "id": "ft_q10", "kind": "null_missing",
            "question": "Which paper ID has no verification status?",
            "ground_truth": ", ".join(ft_null_ver_ids),
        },
        {
            "id": "ft_q11", "kind": "null_missing",
            "question": f"What is the publication year of paper {ft_null_year_ids[0]}?",
            "ground_truth": "null",
        },
        {
            "id": "ft_q12", "kind": "null_missing",
            "question": "Which paper ID has no verdict?",
            "ground_truth": ", ".join(ft_null_verd_ids),
        },
    ]
    all_questions["flat_tabular"] = ft_questions

    # 2. flat_100 (100 rows)
    f100 = data["flat_100"]
    null_year_ids = [r["id"] for r in f100 if r.get("year") is None]
    null_ver_ids = [r["id"] for r in f100 if r.get("verified") is None]
    null_verd_ids = [r["id"] for r in f100 if r.get("verdict") is None]

    f100_questions = [
        # single lookup (4)
        {
            "id": "f100_q01", "kind": "single_lookup",
            "question": f"What is the verdict for paper {f100[0]['id']}?",
            "ground_truth": f100[0]["verdict"],
        },
        {
            "id": "f100_q02", "kind": "single_lookup",
            "question": f"What is the publication year of paper {f100[1]['id']}?",
            "ground_truth": str(f100[1]["year"]),
        },
        {
            "id": "f100_q03", "kind": "single_lookup",
            "question": f"What is the title of paper {f100[3]['id']}?",
            "ground_truth": f100[3]["title"],
        },
        {
            "id": "f100_q04", "kind": "single_lookup",
            "question": f"What is the verification status of paper {f100[4]['id']}?",
            "ground_truth": "true" if f100[4]["verified"] else "false",
        },
        # count or filter (4)
        {
            "id": "f100_q05", "kind": "count_filter",
            "question": "How many records have verdict refuted?",
            "ground_truth": str(sum(1 for r in f100 if r.get("verdict") == "refuted")),
        },
        {
            "id": "f100_q06", "kind": "count_filter",
            "question": "How many records were published in 2024?",
            "ground_truth": str(sum(1 for r in f100 if r.get("year") == 2024)),
        },
        {
            "id": "f100_q07", "kind": "count_filter",
            "question": "How many records are marked as verified (true)?",
            "ground_truth": str(sum(1 for r in f100 if r.get("verified") is True)),
        },
        {
            "id": "f100_q08", "kind": "count_filter",
            "question": "Which paper IDs have no verdict?",
            "ground_truth": ", ".join(null_verd_ids),
        },
        # null or missing (4) - all asking about real null values in existing records
        {
            "id": "f100_q09", "kind": "null_missing",
            "question": "Which paper IDs have no publication year?",
            "ground_truth": ", ".join(null_year_ids),
        },
        {
            "id": "f100_q10", "kind": "null_missing",
            "question": "Which paper IDs have no verification status?",
            "ground_truth": ", ".join(null_ver_ids),
        },
        {
            "id": "f100_q11", "kind": "null_missing",
            "question": f"What is the publication year of paper {null_year_ids[0]}?",
            "ground_truth": "null",
        },
        {
            "id": "f100_q12", "kind": "null_missing",
            "question": f"What is the verdict for paper {null_verd_ids[0]}?",
            "ground_truth": "null",
        },
    ]
    all_questions["flat_100"] = f100_questions

    # 3. nested_fixed (10 rows)
    nf = data["nested_fixed"]
    nf_null_year_texts = [r["text"] for r in nf if r.get("source", {}).get("year") is None]
    nf_null_ver_texts = [r["text"] for r in nf if r.get("verified") is None]
    nf_null_verd_texts = [r["text"] for r in nf if r.get("verdict") is None]

    nf_questions = [
        # single lookup (3)
        {
            "id": "nf_q01", "kind": "single_lookup",
            "question": f"What is the verdict for '{nf[1]['text']}'?",
            "ground_truth": nf[1]["verdict"],
        },
        {
            "id": "nf_q02", "kind": "single_lookup",
            "question": f"What is the verification status for '{nf[7]['text']}'?",
            "ground_truth": "true" if nf[7]["verified"] else "false",
        },
        {
            "id": "nf_q03", "kind": "single_lookup",
            "question": f"What is the verdict for '{nf[0]['text']}'?",
            "ground_truth": nf[0]["verdict"],
        },
        # nested lookup (3)
        {
            "id": "nf_q04", "kind": "nested_lookup",
            "question": f"What is the source paper ID for '{nf[1]['text']}'?",
            "ground_truth": nf[1]["source"]["id"],
        },
        {
            "id": "nf_q05", "kind": "nested_lookup",
            "question": f"What is the publication year of the source for '{nf[2]['text']}'?",
            "ground_truth": str(nf[2]["source"]["year"]),
        },
        {
            "id": "nf_q06", "kind": "nested_lookup",
            "question": f"What is the source paper ID for '{nf[8]['text']}'?",
            "ground_truth": nf[8]["source"]["id"],
        },
        # count or filter (3)
        {
            "id": "nf_q07", "kind": "count_filter",
            "question": "How many claims have verdict supported?",
            "ground_truth": str(sum(1 for r in nf if r.get("verdict") == "supported")),
        },
        {
            "id": "nf_q08", "kind": "count_filter",
            "question": "How many claims have a source published in 2024?",
            "ground_truth": str(sum(1 for r in nf if r.get("source", {}).get("year") == 2024)),
        },
        {
            "id": "nf_q09", "kind": "count_filter",
            "question": "Which source paper IDs belong to refuted claims?",
            "ground_truth": ", ".join(r["source"]["id"] for r in nf if r.get("verdict") == "refuted"),
        },
        # null or missing (3) - all asking about real null values in existing records
        {
            "id": "nf_q10", "kind": "null_missing",
            "question": "Which claim has no source year?",
            "ground_truth": ", ".join(nf_null_year_texts),
        },
        {
            "id": "nf_q11", "kind": "null_missing",
            "question": f"What is the publication year of the source for '{nf_null_year_texts[0]}'?",
            "ground_truth": "null",
        },
        {
            "id": "nf_q12", "kind": "null_missing",
            "question": "Which claim has no verification status?",
            "ground_truth": ", ".join(nf_null_ver_texts),
        },
    ]
    all_questions["nested_fixed"] = nf_questions

    # 4. nested_variable (4 rows)
    nv = data["nested_variable"]
    nv_questions = [
        # single lookup (3)
        {
            "id": "nv_q01", "kind": "single_lookup",
            "question": f"What is the verdict for '{nv[1]['text']}'?",
            "ground_truth": nv[1]["verdict"],
        },
        {
            "id": "nv_q02", "kind": "single_lookup",
            "question": f"What is the verdict for '{nv[2]['text']}'?",
            "ground_truth": nv[2]["verdict"],
        },
        {
            "id": "nv_q03", "kind": "single_lookup",
            "question": f"What is the source paper ID for '{nv[3]['text']}'?",
            "ground_truth": nv[3]["source"]["id"],
        },
        # nested lookup (3)
        {
            "id": "nv_q04", "kind": "nested_lookup",
            "question": f"What subnotes are listed under the note '{nv[0]['notes'][0]['text']}'?",
            "ground_truth": ", ".join(nv[0]["notes"][0]["subnotes"]),
        },
        {
            "id": "nv_q05", "kind": "nested_lookup",
            "question": f"What subnotes are listed under the note '{nv[0]['notes'][1]['text']}'?",
            "ground_truth": ", ".join(nv[0]["notes"][1]["subnotes"]),
        },
        {
            "id": "nv_q06", "kind": "nested_lookup",
            "question": f"What subnotes are listed under the note '{nv[2]['notes'][0]['text']}'?",
            "ground_truth": ", ".join(nv[2]["notes"][0]["subnotes"]),
        },
        # count or filter (3)
        {
            "id": "nv_q07", "kind": "count_filter",
            "question": f"How many notes are attached to '{nv[1]['text']}'?",
            "ground_truth": str(len(nv[1]["notes"])),
        },
        {
            "id": "nv_q08", "kind": "count_filter",
            "question": "How many total claims have verdict refuted?",
            "ground_truth": str(sum(1 for r in nv if r.get("verdict") == "refuted")),
        },
        {
            "id": "nv_q09", "kind": "count_filter",
            "question": f"How many subnotes are under '{nv[1]['notes'][0]['text']}'?",
            "ground_truth": str(len(nv[1]["notes"][0]["subnotes"])),
        },
        # null or missing (3) - all asking about real empty subnotes in existing claim/note
        {
            "id": "nv_q10", "kind": "null_missing",
            "question": f"Which note under '{nv[1]['text']}' has no subnotes?",
            "ground_truth": "checkpoint size under 50MB",
        },
        {
            "id": "nv_q11", "kind": "null_missing",
            "question": "What subnotes are listed under the note 'checkpoint size under 50MB'?",
            "ground_truth": "none",
        },
        {
            "id": "nv_q12", "kind": "null_missing",
            "question": "Which claim text contains a note with no subnotes?",
            "ground_truth": nv[1]["text"],
        },
    ]
    all_questions["nested_variable"] = nv_questions

    if write_file:
        with open(OUTPUT_QUESTIONS, "w", encoding="utf-8") as f:
            json.dump(all_questions, f, indent=2, ensure_ascii=False)
        print(f"Generated {sum(len(qs) for qs in all_questions.values())} questions across {len(all_questions)} datasets saved to {OUTPUT_QUESTIONS}")
    return all_questions

if __name__ == "__main__":
    generate_questions(write_file=True)
