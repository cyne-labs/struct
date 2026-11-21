#!/usr/bin/env python3
"""
Evaluator and report generator for the Bonsai run (bench/r3/real/run3_bonsai).

Scoring is the same as research/r3-real-run3.md:
- target/release/doctor_tool.exe in salvage mode;
- record recall = decoded / expected (s4 expects 6);
- field accuracy pooled over decoded records only, strictly positional
  (decoded record N against ground-truth record N, no matching by id), n/a if none decoded;
- token counts from /tokenize checked against usage.completion_tokens.

Imports compute_field_accuracy (s1 to s3) from evaluate_run3.py without modifying it.
For s4 it computes both the whole-list positional figure and the run3-method figure
(records split by class, then compared in order within each class).
Pass --s4-only to print s4 figures without calling the server.

Writes research/r3-real-bonsai.md (machine part only; "New kinds of mistake" gets real quotes
picked automatically).
"""

import os
import re
import sys
import json
import hashlib
import subprocess
import urllib.request
from collections import Counter, defaultdict

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)

import evaluate_run3 as e3
from tasks import TASKS, get_task_prompt

ROOT = r"C:\Users\Ashman Das\Documents\Struct"
DOCTOR = os.path.join(ROOT, "target", "release", "doctor_tool.exe")
OUT = os.path.join(ROOT, "bench", "r3", "real", "run3_bonsai")
RAW = os.path.join(OUT, "raw")
META = os.path.join(OUT, "metadata.json")
REPORT = os.path.join(ROOT, "research", "r3-real-bonsai.md")
RUN3_REPORT = os.path.join(ROOT, "research", "r3-real-run3.md")
RUN3_META = os.path.join(ROOT, "bench", "r3", "real", "run3", "metadata.json")

SCHEMA_PATHS = e3.SCHEMAS
SCHEMAS = ["s1_flat", "s2_nested_fixed", "s3_nested_variable", "s4_two_class_code"]
EXPECTED = e3.EXPECTED_RECORD_COUNTS
MODES = [("bonsai_off", "Bonsai-27B (thinking off)"), ("bonsai_on", "Bonsai-27B (thinking on)")]


def run_doctor(schema, path):
    out = subprocess.check_output([DOCTOR, SCHEMA_PATHS[schema], path, "salvage", "true"], text=True, encoding="utf-8")
    return json.loads(out.strip())


def s4_positional(decoded):
    gt = TASKS["s4_two_class_code"]["ground_truth"]
    correct = evaluated = 0
    for i, dec in enumerate(decoded):
        if i >= len(gt):
            break
        exp = gt[i]
        dec = dec if isinstance(dec, dict) else {}
        for k, v in exp.items():
            if k == "class":
                continue
            evaluated += 1
            if str(dec.get(k, "")).strip() == str(v).strip():
                correct += 1
    return correct, evaluated


def s4_run3_method(decoded):
    """Compute s4 field accuracy exactly as evaluate_run3.py does (records split by class)."""
    if not decoded:
        return 0, 0
    c, ev, _ = e3.compute_field_accuracy("s4_two_class_code", decoded)
    return c, ev


def field_acc(schema, decoded):
    if not decoded:
        return 0, 0
    if schema == "s4_two_class_code":
        return s4_positional(decoded)
    c, ev, _ = e3.compute_field_accuracy(schema, decoded)
    return c, ev


def print_s4_only(meta):
    """Print only the s4 figures without calling the server."""
    print("=== s4_two_class_code Field Accuracy ===")
    for m, name in MODES:
        print(f"\n{name} ({m}):")
        pos_cor = pos_ev = 0
        r3_cor = r3_ev = 0
        for it in meta:
            if it["model"] != m or it["schema"] != "s4_two_class_code":
                continue
            r = it["run"]
            seed = it["seed"]
            fpath = os.path.join(RAW, it["filename"])
            d = run_doctor("s4_two_class_code", fpath)
            recs = d.get("records", [])
            c_pos, ev_pos = s4_positional(recs)
            c_r3, ev_r3 = s4_run3_method(recs)
            pos_cor += c_pos
            pos_ev += ev_pos
            r3_cor += c_r3
            r3_ev += ev_r3
            p_pos = pct(c_pos, ev_pos) if ev_pos else "n/a"
            p_r3 = pct(c_r3, ev_r3) if ev_r3 else "n/a"
            print(f"  Run {r} (seed {seed}): decoded {len(recs)}/6 records | "
                  f"whole-list: {c_pos}/{ev_pos} ({p_pos}) | "
                  f"run3-method: {c_r3}/{ev_r3} ({p_r3})")
        tot_pos_pct = pct(pos_cor, pos_ev) if pos_ev else "n/a"
        tot_r3_pct = pct(r3_cor, r3_ev) if r3_ev else "n/a"
        print(f"  Pooled total:")
        print(f"    whole-list positional:    {pos_cor}/{pos_ev} ({tot_pos_pct})")
        print(f"    run3-method (class-split): {r3_cor}/{r3_ev} ({tot_r3_pct})")


def sha(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def pct(n, d):
    return f"{n / d * 100:.1f}%" if d else "n/a"


def parse_run3_matrix():
    """Return {(model_label, schema): list of (recall_str, field_acc_str, label_suffix)} copied from run3 report."""
    rows = defaultdict(list)
    with open(RUN3_REPORT, "r", encoding="utf-8") as f:
        text = f.read()
    sec = text.split("## 5. Summary Matrix")[1].split("## 6.")[0]
    for line in sec.splitlines():
        if not line.startswith("| ") or line.startswith("| Model") or line.startswith("|---"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        model, schema = cells[0], cells[1].strip("`")
        rows[(model, schema)].append((cells[8], cells[9]))
    return rows


def fetch_props():
    try:
        with urllib.request.urlopen("http://127.0.0.1:8080/props", timeout=10) as r:
            return json.loads(r.read())
    except Exception as e:
        return {"error": str(e)}


def main():
    meta = json.load(open(META, encoding="utf-8"))
    if "--s4-only" in sys.argv:
        print_s4_only(meta)
        return

    pair = defaultdict(lambda: {
        "dec": 0, "exp": 0, "cor": 0, "ev": 0,
        "clean": 0, "repaired": 0, "salvaged": 0, "partial": 0, "unusable": 0,
        "hashes": set(), "fr": Counter(), "think": [], "out": [], "runs": 0,
        "r3_cor": 0, "r3_ev": 0,
    })
    repairs_by_kind = Counter()
    bad_by_problem = Counter()
    per_run = []
    tok_diffs = []
    off_reasoning_nonempty = 0
    problems_samples = defaultdict(list)

    for it in meta:
        m, s, r = it["model"], it["schema"], it["run"]
        fpath = os.path.join(RAW, it["filename"])
        exp_n = EXPECTED[s]
        d = run_doctor(s, fpath)
        has = d.get("has_data", False)
        n = d.get("record_count", 0)
        reps = d.get("repairs", [])
        bads = d.get("bad_lines", [])
        recs = d.get("records", [])
        if has:
            if not reps and not bads:
                cat = "clean" if n >= exp_n else "partial"
            elif bads:
                cat = "salvaged"
            elif n < exp_n:
                cat = "partial"
            else:
                cat = "repaired"
        else:
            cat = "unusable"
        c, ev = field_acc(s, recs)
        p = pair[(m, s)]
        p["dec"] += n
        p["exp"] += exp_n
        p["cor"] += c
        p["ev"] += ev
        if s == "s4_two_class_code":
            c_r3, ev_r3 = s4_run3_method(recs)
            p["r3_cor"] += c_r3
            p["r3_ev"] += ev_r3
            acc_str = f"{pct(c, ev)} whole-list / {pct(c_r3, ev_r3)} run3-method" if n else "n/a"
        else:
            acc_str = pct(c, ev) if n else "n/a"
        p[cat] += 1
        p["hashes"].add(sha(fpath))
        p["fr"][it["finish_reason"]] += 1
        p["think"].append(it["thinking_tokens"])
        p["out"].append(it["struct_tokens"])
        p["runs"] += 1
        for x in reps:
            repairs_by_kind[(m, x["fix"])] += 1
        for b in bads:
            bad_by_problem[(m, b["problem"])] += 1
            if len(problems_samples[(m, b["problem"])]) < 2 and b.get("original", "").strip():
                problems_samples[(m, b["problem"])].append((s, r, b.get("line"), b["original"]))
        usage = it["response"].get("usage", {}).get("completion_tokens") if isinstance(it["response"], dict) else None
        if usage is not None:
            tok_diffs.append(usage - (it["thinking_tokens"] + it["struct_tokens"]))
        if m == "bonsai_off":
            msg = (it["response"].get("choices") or [{}])[0].get("message", {}) if isinstance(it["response"], dict) else {}
            if (msg.get("reasoning_content") or "").strip():
                off_reasoning_nonempty += 1
        per_run.append((m, s, r, it["seed"], cat, n, exp_n, len(reps), len(bads), it["finish_reason"],
                        it["thinking_tokens"], it["struct_tokens"], acc_str, it.get("latency_sec")))

    props = fetch_props()
    tmpl = props.get("chat_template", "")
    i = tmpl.rfind("enable_thinking is defined and enable_thinking is false")
    tmpl_quote = tmpl[i - 40:i + 230] if i >= 0 else "(not found)"
    tmpl_quote = "    {%- if add_generation_prompt %}" if False else tmpl[tmpl.rfind("{%- if add_generation_prompt %}"):]
    props_small = {k: v for k, v in props.items() if k not in ("chat_template", "default_generation_settings")}
    dgs = props.get("default_generation_settings", {})

    # prompt equality with run3
    run3_meta = json.load(open(RUN3_META, encoding="utf-8"))
    prompt_equal = {}
    for sch in SCHEMAS:
        r3p = [x for x in run3_meta if x["schema"] == sch][0]["request"]["messages"][0]["content"]
        mine = [x for x in meta if x["schema"] == sch][0]["request"]["messages"][0]["content"]
        prompt_equal[sch] = (r3p == mine)

    run3 = parse_run3_matrix()
    run3_models = ["Llama-3.2-3B (thinking off)", "Gemma-4-E2B (thinking off)", "Gemma-4-E2B (thinking on)",
                   "LFM2.5-8B (thinking on)", "Mellum2.1-12B (thinking on)"]

    L = []
    A = L.append
    A("# Struct real-model run: Ternary Bonsai 2 27B (run3 setup)")
    A("")
    A(f"{len(meta)} runs: 2 modes x 4 schemas x 5 seeds (42 to 46). Collected by `bench/r3/collect_bonsai.py`, scored by `bench/r3/evaluate_bonsai.py`, data in `bench/r3/real/run3_bonsai/`. Observations only.")
    A("")
    A("## 1. Setup")
    A("")
    A("- Endpoint: `POST /v1/chat/completions` on the already-running server at `http://127.0.0.1:8080` (the collector never started or stopped a server). One user message per request, the same prompt text as run3, no prefills, no `max_tokens`, no `n_predict`, `stream: false`, seeds 42 to 46, 5 runs per schema, four `bench/r2/schemas`, no grammar. 20-minute safety timeout per run (recorded as `timeout`). The task text is byte-identical, but with thinking on the chat template adds a system line (\"Reasoning effort is set to xhigh\"), so the thinking-on input is 40 tokens longer (no causal claim).")
    A("- Prompt check: the user message of every request equals, byte for byte, the run3 request prompt for the same schema: " + ", ".join(f"`{k}` {'yes' if v else 'NO'}" for k, v in prompt_equal.items()) + ".")
    A("- Scoring: `target/release/doctor_tool.exe` in salvage mode; field accuracy is positional only (decoded record N against ground-truth record N, no matching by id), pooled over decoded records, `n/a` if none decoded; s4 expects 6 records. On s4, both the whole-list positional figure and the run3-method figure (records split by class) are reported.")
    A("- Token counts: thinking text and answer text counted with the server's `/tokenize`; checked against `usage.completion_tokens` (section 7).")
    A("")
    A("### Sampling per mode (sent in every request)")
    A("")
    A("| Mode | `chat_template_kwargs` | temperature | top_p | top_k | min_p | presence_penalty | repeat_penalty |")
    A("|---|---|---|---|---|---|---|---|")
    A('| thinking on | `{"enable_thinking": true}` | 1.0 | 0.95 | 20 | 0.05 | 0.0 | 1.0 |')
    A('| thinking off | `{"enable_thinking": false}` | 0.7 | 0.80 | 20 | 0.0 | 1.5 | 1.0 |')
    A("")
    A("The run3 models (Llama, Gemma, LFM, Mellum) used temperature 0.7 with every other sampling setting left at the server defaults. The Bonsai sampling differs on purpose: it is PrismML's recommended sampling for each mode, so the comparison is between each model in its documented setup, not between identical sampling settings.")
    A("")
    A("## 2. Server settings (`/props`, read after the run)")
    A("")
    A(f"- Build: `{props_small.get('build_info')}` (prism-llama-cpp fork); model: `{props_small.get('model_alias')}`; ftype: `{props_small.get('model_ftype')}`")
    A("- Exact launch flags (given by the maintainer; `/props` does not show them), on the prism-llama-cpp fork: `-ngl 999 --load-mode mlock --cache-type-k q4_0 --cache-type-v q4_0 -fa on -c 65536`.")
    A("- KV cache: Bonsai ran with a 4-bit (q4_0) KV cache. Every run3 model used turbo4 (K) and turbo3 (V) KV. This is a recorded difference in setup only; no effect on any result is claimed.")
    A(f"- `n_ctx` {dgs.get('n_ctx')}, `total_slots` {props_small.get('total_slots')}.")
    A(f"- Server-default generation settings (overridden per request): temperature {dgs.get('params', {}).get('temperature')}, top_p {dgs.get('params', {}).get('top_p')}, top_k {dgs.get('params', {}).get('top_k')}, min_p {dgs.get('params', {}).get('min_p')}, presence_penalty {dgs.get('params', {}).get('presence_penalty')}, repeat_penalty {dgs.get('params', {}).get('repeat_penalty')}, `reasoning_format` `{dgs.get('params', {}).get('reasoning_format')}`, `max_tokens` {dgs.get('params', {}).get('max_tokens')}.")
    A("- Other server clients: the maintainer could use the server at the same time; the server has 4 slots.")
    A("")
    A("```json")
    A(json.dumps(props_small, indent=2))
    A("```")
    A("")
    A("## 3. Thinking switch (from the embedded chat template in `/props`)")
    A("")
    A("The variable is `enable_thinking`. Template text, quoted:")
    A("")
    A("```jinja")
    A("{%- if enable_thinking is undefined or enable_thinking is true %}")
    A("    {%- set resolved_reasoning_effort = reasoning_effort|default('xhigh') %}")
    A("    ...")
    A(tmpl_quote.rstrip())
    A("```")
    A("")
    A(f"With `enable_thinking: true` the template also adds a system line starting `Reasoning effort is set to xhigh.` (it is the default), so the thinking-on prompt is longer than the thinking-off prompt (467 against 427 prompt tokens on s1 in a probe). Thinking-off runs with non-empty `reasoning_content`: {off_reasoning_nonempty} of {sum(1 for x in meta if x['model']=='bonsai_off')}. The server returns thinking separately in `choices[0].message.reasoning_content`, so no tag cutting was needed.")
    A("")
    A("## 4. Summary matrix")
    A("")
    A("Same columns as run3.")
    A("")
    A("| Model | Schema | Runs | Clean | Repaired | Salvaged | Partial | Unusable | Record Recall | Field Accuracy | Distinct | finish_reason | Mean Think Tok | Max Think Tok | Mean Out Tok | Max Out Tok |")
    A("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for m, name in MODES:
        for s in SCHEMAS:
            p = pair[(m, s)]
            if not p["runs"]:
                continue
            fr = ", ".join(f"{k}: {v}" for k, v in sorted(p["fr"].items()))
            if s == "s4_two_class_code":
                acc_display = f"{pct(p['cor'], p['ev'])} whole-list / {pct(p.get('r3_cor', 0), p.get('r3_ev', 0))} run3-method"
            else:
                acc_display = pct(p['cor'], p['ev'])
            A(f"| {name} | `{s}` | {p['runs']} | {p['clean']} | {p['repaired']} | {p['salvaged']} | {p['partial']} | {p['unusable']} | "
              f"{pct(p['dec'], p['exp'])} ({p['dec']}/{p['exp']}) | {acc_display} | {len(p['hashes'])}/{p['runs']} | {fr} | "
              f"{sum(p['think'])/len(p['think']):.0f} | {max(p['think'])} | {sum(p['out'])/len(p['out']):.0f} | {max(p['out'])} |")
    A("")
    A("Definitions are those of `research/r3-real-run3.md` section 5: recall is decoded over expected records; field accuracy is exact-match, positional, pooled over decoded records; outcomes are clean, repaired, salvaged (records recovered next to reported bad lines), partial (clean or repaired but fewer records than expected) and unusable (nothing decoded). On `s4_two_class_code`, both the whole-list positional figure and the run3-method figure (records split by class) are shown. Bonsai wrote files and patches alternating, which SPEC allows, and the whole-list method compares them against a list with all files first.")
    A("")
    A("## 5. finish_reason, thinking and output tokens by mode")
    A("")
    A("| Mode | Runs | finish_reason | Think tok mean | Think tok max | Out tok mean | Out tok max | Timeouts |")
    A("|---|---|---|---|---|---|---|---|")
    for m, name in MODES:
        fr = Counter()
        th, ot = [], []
        for s in SCHEMAS:
            fr.update(pair[(m, s)]["fr"])
            th += pair[(m, s)]["think"]
            ot += pair[(m, s)]["out"]
        if not th:
            continue
        A(f"| {name} | {len(th)} | {', '.join(f'{k}: {v}' for k, v in sorted(fr.items()))} | {sum(th)/len(th):.0f} | {max(th)} | {sum(ot)/len(ot):.0f} | {max(ot)} | {fr.get('timeout', 0)} |")
    A("")
    A("## 6. Comparison with every run3 model")
    A("")
    A("Run3 cells are copied from `research/r3-real-run3.md` section 5 (recall, then field accuracy). Where run3 split Gemma thinking-on rows into reasoned and unreasoned runs, both cells are shown. Bonsai cells are this run. Sampling differs on purpose (section 1). For `s4_two_class_code`, both the whole-list figure and the run3-method figure are shown; only the run3-method figure is comparable with the run3 models' s4 numbers. Bonsai wrote files and patches alternating, which SPEC allows, and the whole-list method compares them against a list with all files first.")
    A("")
    A("| Model | s1_flat recall / field acc | s2_nested_fixed recall / field acc | s3_nested_variable recall / field acc | s4_two_class_code recall / field acc |")
    A("|---|---|---|---|---|")
    for mdl in run3_models:
        cells = []
        for s in SCHEMAS:
            rr = run3.get((mdl, s), [])
            if not rr:
                # split Gemma rows
                rr = run3.get((mdl.replace(")", ", reasoned)"), s), []) + run3.get((mdl.replace(")", ", unreasoned)"), s), [])
                tag = ["reasoned", "unreasoned"]
                cells.append(" ; ".join(f"{tag[i]}: {a} / {b}" for i, (a, b) in enumerate(rr)) if rr else "?")
            else:
                cells.append(" ; ".join(f"{a} / {b}" for a, b in rr))
        A(f"| {mdl} (run3) | " + " | ".join(cells) + " |")
    for m, name in MODES:
        cells = []
        for s in SCHEMAS:
            p = pair[(m, s)]
            if not p["runs"]:
                cells.append("not run")
            elif s == "s4_two_class_code":
                cells.append(f"{pct(p['dec'], p['exp'])} ({p['dec']}/{p['exp']}) / {pct(p['cor'], p['ev'])} whole-list, {pct(p.get('r3_cor', 0), p.get('r3_ev', 0))} run3-method")
            else:
                cells.append(f"{pct(p['dec'], p['exp'])} ({p['dec']}/{p['exp']}) / {pct(p['cor'], p['ev'])}")
        A(f"| **{name}** | " + " | ".join(cells) + " |")
    A("")
    A("## 7. Token counts against `usage.completion_tokens`")
    A("")
    if tok_diffs:
        dc = Counter(tok_diffs)
        A(f"`usage.completion_tokens` minus (`/tokenize` of thinking + `/tokenize` of answer), over {len(tok_diffs)} runs: min {min(tok_diffs)}, max {max(tok_diffs)}. Distribution (difference: runs): " + ", ".join(f"{k}: {v}" for k, v in sorted(dc.items())) + ".")
    A("")
    A("## 8. Doctor repairs by kind")
    A("")
    A("| Mode | Fix | Occurrences |")
    A("|---|---|---|")
    if repairs_by_kind:
        for (m, k), v in sorted(repairs_by_kind.items(), key=lambda x: (x[0][0], -x[1])):
            A(f"| {m} | `{k}` | {v} |")
    else:
        A("| - | none | 0 |")
    A("")
    A("## 9. Doctor bad lines by problem")
    A("")
    A("| Mode | Problem | Occurrences |")
    A("|---|---|---|")
    if bad_by_problem:
        for (m, k), v in sorted(bad_by_problem.items(), key=lambda x: (x[0][0], -x[1])):
            A(f"| {m} | `{k}` | {v} |")
    else:
        A("| - | none | 0 |")
    A("")
    A("### Auto-picked quotes per problem (first two each)")
    A("")
    for (m, prob), samples in sorted(problems_samples.items()):
        A(f"**{m}** `{prob}`")
        for s, r, ln, text in samples:
            A("")
            A(f"- `{s}` run {r}, line {ln}:")
            A("")
            A("  ```text")
            for tl in text.splitlines() or [text]:
                A("  " + tl)
            A("  ```")
        A("")
    A("## 10. New kinds of mistake")
    A("")
    A("<!--NEW_MISTAKES-->")
    A("")
    A("## 11. Per-run results")
    A("")
    A("| Mode | Schema | Run | Seed | Outcome | Records | Repairs | Bad lines | finish_reason | Think tok | Out tok | Field acc | Latency s |")
    A("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for x in per_run:
        A(f"| {x[0]} | `{x[1]}` | {x[2]} | {x[3]} | {x[4]} | {x[5]}/{x[6]} | {x[7]} | {x[8]} | {x[9]} | {x[10]} | {x[11]} | {x[12]} | {x[13]} |")
    A("")

    # preserve hand-written section if report already has one
    text = "\n".join(L)
    if os.path.exists(REPORT):
        old = open(REPORT, encoding="utf-8").read()
        mt = re.search(r"## 10\. New kinds of mistake\n\n(.*?)\n\n## 11\.", old, re.S)
        if mt and "<!--NEW_MISTAKES-->" not in mt.group(1):
            text = text.replace("<!--NEW_MISTAKES-->", mt.group(1))
    with open(REPORT, "w", encoding="utf-8") as f:
        f.write(text)
    print("report written", REPORT, len(L), "lines")


if __name__ == "__main__":
    main()
