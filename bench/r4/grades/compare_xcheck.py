import json
from collections import defaultdict
S = r"C:\Users\ASHMAN~1\AppData\Local\Temp\claude\1bd3168601fc9879\27e7a27a-211e-4c5f-a14d-ae90cc1ebfca\scratchpad"
key = {k["id"]: k for k in json.load(open(r"C:\Users\Ashman Das\Documents\Struct\bench\r4\grades\crosscheck\key.json", encoding="utf-8"))}
re = json.load(open(r"C:\Users\Ashman Das\Documents\Struct\bench\r4\grades\crosscheck\regrade.json", encoding="utf-8"))["items"]
rg = {r["id"]: r for r in re}
print("sample", len(key), "regraded", len(rg), "missing", len(set(key) - set(rg)), "extra", len(set(rg) - set(key)))
stats = defaultdict(lambda: {"n": 0, "agree": 0, "changed_n": 0, "changed_agree": 0})
disagree = []
for i, k in key.items():
    if i not in rg:
        continue
    model, fmt = i.split("|")[:2]
    s = stats[(model, fmt)]
    s["n"] += 1
    ok = rg[i]["grade"] == k["shard_grade"]
    s["agree"] += ok
    if k["shard_grade"] != k["strict_grade"]:
        s["changed_n"] += 1
        s["changed_agree"] += ok
    if not ok:
        disagree.append((i, k["shard_grade"], rg[i]["grade"], k.get("shard_answer"), rg[i].get("reason"), k.get("shard_reason")))
print(f"{'shard':18} {'agree':>8} {'changed agree':>14}")
for (m, f), s in sorted(stats.items()):
    print(f"{m+'/'+f:18} {s['agree']:>3}/{s['n']:<4} {s['changed_agree']:>6}/{s['changed_n']:<4}")
tot = sum(s["agree"] for s in stats.values()); n = sum(s["n"] for s in stats.values())
print("overall", tot, "/", n)
print("=== disagreements")
for d in disagree:
    print(" | ".join(str(x)[:140] for x in d))
