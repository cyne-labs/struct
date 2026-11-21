import sys, json, os, random
sys.path.insert(0, r"C:\Users\Ashman Das\Documents\Struct\bench")
from r4 import evaluate as ev
from r4 import collect as c

R4 = r"C:\Users\Ashman Das\Documents\Struct\bench\r4"
OUT = os.path.join(R4, "grades", "work")
os.makedirs(OUT, exist_ok=True)
questions = c.load_questions()

def build(model, real_dir):
    meta = json.load(open(os.path.join(real_dir, "metadata.json"), encoding="utf-8"))
    raw_dir = os.path.join(real_dir, "raw")
    res = ev.evaluate_runs(meta, questions, raw_dir=raw_dir)
    wrong = {(w["format"], w["dataset"], w["seed"], w["question"]) for w in res["wrong_answers"]}
    # latest successful entry per key (same rule as evaluate)
    runs = {}
    for m in meta:
        if m.get("success") and not m.get("error"):
            runs[(m["format"], m["dataset"], m["seed"])] = m
    by_fmt = {}
    for (fmt, ds, seed), m in sorted(runs.items()):
        raw_path = os.path.join(raw_dir, m["file"])
        content = open(raw_path, encoding="utf-8").read()
        parsed = ev.parse_model_answers(content, expected_count=len(questions[ds]))
        for i, q in enumerate(questions[ds], start=1):
            strict_wrong = (fmt, ds, seed, q["question"]) in wrong
            by_fmt.setdefault(fmt, []).append({
                "raw_file": raw_path, "format": fmt, "dataset": ds, "seed": seed,
                "question_number": i, "question": q["question"], "kind": q.get("kind"),
                "expected": q["ground_truth"],
                "strict_grade": "wrong" if strict_wrong else "correct",
                "strict_parsed_answer": parsed.get(i, ""),
            })
    totals = {}
    for fmt, items in by_fmt.items():
        rng = random.Random(f"{model}-{fmt}-spotcheck")
        correct = [x for x in items if x["strict_grade"] == "correct"]
        spot = rng.sample(correct, max(1, round(len(correct) * 0.05))) if correct else []
        for x in spot:
            x["spot_check"] = True
        work = [x for x in items if x["strict_grade"] == "wrong"] + spot
        json.dump({"model": model, "format": fmt, "total_questions": len(items),
                   "strict_correct": len(correct), "strict_wrong": len(items) - len(correct),
                   "items": work}, open(os.path.join(OUT, f"{model}_{fmt}.json"), "w", encoding="utf-8"), indent=1)
        totals[fmt] = (len(items), len(items) - len(correct), len(spot))
    print(model, totals)

build("gemma", os.path.join(R4, "real"))
build("haiku", os.path.join(R4, "real_haiku"))
