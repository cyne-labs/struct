import json, os, random
G = r"C:\Users\Ashman Das\Documents\Struct\bench\r4\grades"
OUT = os.path.join(G, "crosscheck")
os.makedirs(OUT, exist_ok=True)
FMTS = ["json_min", "toon", "yaml", "tsv", "xml", "key_value", "struct"]
blind, key = [], []
for model in ["gemma", "haiku"]:
    for fmt in FMTS:
        out = json.load(open(os.path.join(G, f"{model}_{fmt}.json"), encoding="utf-8"))
        rng = random.Random(f"xcheck-{model}-{fmt}")
        changed = [x for x in out["items"] if x.get("grade") != x.get("strict_grade")]
        kept_wrong = [x for x in out["items"] if x.get("strict_grade") == "wrong" and x.get("grade") == "wrong"]
        pick = rng.sample(changed, min(12, len(changed))) + rng.sample(kept_wrong, min(4, len(kept_wrong)))
        for x in pick:
            item_id = f"{model}|{fmt}|{x['dataset']}|{x['seed']}|{x['question_number']}"
            blind.append({"id": item_id, "model": model, "format": fmt, "raw_file": x["raw_file"],
                          "dataset": x["dataset"], "seed": x["seed"], "question_number": x["question_number"],
                          "question": x["question"], "expected": x["expected"]})
            key.append({"id": item_id, "shard_grade": x["grade"], "strict_grade": x["strict_grade"],
                        "shard_answer": x.get("model_answer"), "shard_reason": x.get("reason")})
rng = random.Random("xcheck-shuffle")
rng.shuffle(blind)  # mix formats so no format is graded as a block
json.dump({"items": blind}, open(os.path.join(OUT, "sample.json"), "w", encoding="utf-8"), indent=1)
# The answer key was kept outside the repo while the cross-checker ran, so it could not see it; it is now in crosscheck/key.json.
json.dump(key, open(r"C:\Users\ASHMAN~1\AppData\Local\Temp\claude\1bd3168601fc9879\27e7a27a-211e-4c5f-a14d-ae90cc1ebfca\scratchpad\xcheck_key.json", "w", encoding="utf-8"), indent=1)
print(len(blind), "items")
