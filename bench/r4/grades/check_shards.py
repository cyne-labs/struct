import json, os
G = r"C:\Users\Ashman Das\Documents\Struct\bench\r4\grades"
FMTS = ["json_min", "toon", "yaml", "tsv", "xml", "key_value", "struct"]
for model in ["gemma", "haiku"]:
    print(f"== {model}")
    print(f"{'format':10} {'strict':>9} {'meaning':>9} {'w->c':>5} {'c->w':>5} {'unclr':>5}  problems")
    for fmt in FMTS:
        work = json.load(open(os.path.join(G, "work", f"{model}_{fmt}.json"), encoding="utf-8"))
        out_p = os.path.join(G, f"{model}_{fmt}.json")
        if not os.path.exists(out_p):
            print(f"{fmt:10} MISSING OUTPUT")
            continue
        try:
            out = json.load(open(out_p, encoding="utf-8"))
        except Exception as e:
            print(f"{fmt:10} BAD JSON: {e}")
            continue
        probs = []
        wk = {(x["dataset"], x["seed"], x["question_number"]) for x in work["items"]}
        ok_items = out.get("items", [])
        ok = {(x.get("dataset"), x.get("seed"), x.get("question_number")) for x in ok_items}
        if len(ok_items) != len(work["items"]):
            probs.append(f"items {len(ok_items)} vs work {len(work['items'])}")
        if wk != ok:
            probs.append(f"key mismatch missing={len(wk-ok)} extra={len(ok-wk)}")
        bad = [x for x in ok_items if x.get("grade") not in ("correct", "wrong", "unclear")]
        if bad:
            probs.append(f"{len(bad)} invalid grades")
        w2c = sum(1 for x in ok_items if x.get("strict_grade") == "wrong" and x.get("grade") == "correct")
        c2w = sum(1 for x in ok_items if x.get("strict_grade") == "correct" and x.get("grade") == "wrong")
        unc = sum(1 for x in ok_items if x.get("grade") == "unclear")
        unc_c = sum(1 for x in ok_items if x.get("grade") == "unclear" and x.get("strict_grade") == "correct")
        total = work["total_questions"]
        sc = work["strict_correct"]
        meaning = sc + w2c - c2w - unc_c
        print(f"{fmt:10} {sc:>4}/{total:<4} {meaning:>4}/{total:<4} {w2c:>5} {c2w:>5} {unc:>5}  {'; '.join(probs)}")
