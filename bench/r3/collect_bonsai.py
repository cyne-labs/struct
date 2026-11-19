#!/usr/bin/env python3
"""
Collection script: Struct real-model benchmark on Ternary Bonsai 2 27B (run3 setup, comparable).

Same as collect_run3.py except:
- The server is ALREADY RUNNING at 127.0.0.1:8080 (prism-llama-cpp). This script never starts,
  stops or restarts any server.
- Two modes, each sending PrismML's recommended sampling in every request:
    bonsai_off: enable_thinking false, temp 0.7, top_p 0.80, top_k 20, min_p 0.0, presence_penalty 1.5, repeat_penalty 1.0
    bonsai_on : enable_thinking true,  temp 1.0, top_p 0.95, top_k 20, min_p 0.05, presence_penalty 0.0, repeat_penalty 1.0
- Thinking-off runs first. No max_tokens / n_predict. 20-minute safety timeout per run -> "timeout".
- Output: bench/r3/real/run3_bonsai/ (raw/, backup/, SHA256SUMS (LF), metadata.json, ground_truth.json).
- metadata.json is rewritten after every run; the script resumes and skips completed runs.
"""

import os
import sys
import time
import json
import shutil
import hashlib
import urllib.request
import urllib.error

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)

from tasks import TASKS, get_task_prompt

STRUCT_ROOT = r"C:\Users\Ashman Das\Documents\Struct"
BASE = "http://127.0.0.1:8080"

OUT_DIR = os.path.join(STRUCT_ROOT, "bench", "r3", "real", "run3_bonsai")
RAW_DIR = os.path.join(OUT_DIR, "raw")
BACKUP_DIR = os.path.join(OUT_DIR, "backup")
SHA256_FILE = os.path.join(OUT_DIR, "SHA256SUMS")
METADATA_FILE = os.path.join(OUT_DIR, "metadata.json")
GROUND_TRUTH_FILE = os.path.join(OUT_DIR, "ground_truth.json")

CONFIGS = [
    {
        "key": "bonsai_off", "name": "Bonsai-27B (thinking off)", "thinking": False,
        "template_kwargs": {"enable_thinking": False},
        "sampling": {"temperature": 0.7, "top_p": 0.80, "top_k": 20, "min_p": 0.0,
                     "presence_penalty": 1.5, "repeat_penalty": 1.0},
    },
    {
        "key": "bonsai_on", "name": "Bonsai-27B (thinking on)", "thinking": True,
        "template_kwargs": {"enable_thinking": True},
        "sampling": {"temperature": 1.0, "top_p": 0.95, "top_k": 20, "min_p": 0.05,
                     "presence_penalty": 0.0, "repeat_penalty": 1.0},
    },
]

SCHEMAS = ["s1_flat", "s2_nested_fixed", "s3_nested_variable", "s4_two_class_code"]
SEEDS = [42, 43, 44, 45, 46]
TIMEOUT_SEC = 1200


def get_json(path, timeout=10):
    with urllib.request.urlopen(BASE + path, timeout=timeout) as res:
        return json.loads(res.read())


def tokenize_text(text):
    if not text:
        return 0
    req = urllib.request.Request(
        BASE + "/tokenize",
        data=json.dumps({"content": text}).encode("utf-8"),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=60) as res:
        return len(json.loads(res.read()).get("tokens", []))


def run_chat_completion(payload):
    req = urllib.request.Request(
        BASE + "/v1/chat/completions",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=TIMEOUT_SEC) as res:
        return json.loads(res.read())


def save_metadata(metadata):
    tmp = METADATA_FILE + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(metadata, f, indent=2)
    os.replace(tmp, METADATA_FILE)


def main():
    health = get_json("/health")
    if health.get("status") != "ok":
        print(f"Server not healthy: {health}")
        sys.exit(1)
    print(f"Server health: {health}")

    os.makedirs(RAW_DIR, exist_ok=True)
    os.makedirs(BACKUP_DIR, exist_ok=True)

    with open(GROUND_TRUTH_FILE, "w", encoding="utf-8") as f:
        json.dump({k: v["ground_truth"] for k, v in TASKS.items()}, f, indent=2)

    task_prompts = {s: get_task_prompt(s) for s in SCHEMAS}

    metadata = []
    done = set()
    if os.path.exists(METADATA_FILE):
        with open(METADATA_FILE, "r", encoding="utf-8") as f:
            metadata = json.load(f)
        done = {(m["model"], m["schema"], m["run"]) for m in metadata}
        print(f"Resuming: {len(done)} runs already collected")

    for cfg in CONFIGS:
        key = cfg["key"]
        print(f"\n=== {cfg['name']} ({key}) ===")
        for schema_key in SCHEMAS:
            print(f"--- {schema_key} ---")
            for run_idx, seed in enumerate(SEEDS, start=1):
                if (key, schema_key, run_idx) in done:
                    continue
                payload = {
                    "messages": [{"role": "user", "content": task_prompts[schema_key]}],
                    "seed": seed,
                    "stream": False,
                    "chat_template_kwargs": cfg["template_kwargs"],
                }
                payload.update(cfg["sampling"])

                t0 = time.time()
                resp = None
                finish_reason = "unknown"
                content = ""
                reasoning = ""
                try:
                    resp = run_chat_completion(payload)
                    dt = time.time() - t0
                    choice = resp.get("choices", [{}])[0]
                    finish_reason = choice.get("finish_reason", "unknown")
                    msg = choice.get("message", {})
                    content = msg.get("content", "") or ""
                    reasoning = msg.get("reasoning_content", "") or ""
                except Exception as e:
                    dt = time.time() - t0
                    s = str(e).lower()
                    if "timed out" in s or isinstance(e, TimeoutError):
                        finish_reason = "timeout"
                        print(f"  Run {run_idx} (seed {seed}) TIMED OUT after {dt:.1f}s")
                        resp = {"error": str(e), "timeout": True}
                    else:
                        finish_reason = f"error: {e}"
                        print(f"  Run {run_idx} (seed {seed}) ERROR: {e}")
                        resp = {"error": str(e), "timeout": False}

                thinking_tokens = tokenize_text(reasoning) if reasoning else 0
                struct_tokens = tokenize_text(content) if content else 0
                usage = resp.get("usage", {}) if resp else {}
                total_completion_tokens = usage.get("completion_tokens")

                filename = f"{key}_{schema_key}_run{run_idx}.txt"
                with open(os.path.join(RAW_DIR, filename), "wb") as f:
                    f.write(content.encode("utf-8"))

                thinking_filename = None
                if reasoning.strip():
                    thinking_filename = f"{key}_{schema_key}_run{run_idx}_thinking.txt"
                    with open(os.path.join(RAW_DIR, thinking_filename), "wb") as f:
                        f.write(reasoning.encode("utf-8"))

                metadata.append({
                    "model": key,
                    "model_name": cfg["name"],
                    "schema": schema_key,
                    "run": run_idx,
                    "seed": seed,
                    "filename": filename,
                    "thinking_filename": thinking_filename,
                    "finish_reason": finish_reason,
                    "thinking_tokens": thinking_tokens,
                    "struct_tokens": struct_tokens,
                    "total_tokens": total_completion_tokens,
                    "latency_sec": round(dt, 2),
                    "request": payload,
                    "response": resp,
                })
                save_metadata(metadata)
                print(f"  Run {run_idx} (seed {seed}): {finish_reason}, {struct_tokens} out tok, "
                      f"{thinking_tokens} think tok, usage={total_completion_tokens}, {dt:.1f}s", flush=True)

    print("\n>>> Backing up raw outputs...")
    for fname in os.listdir(RAW_DIR):
        shutil.copy2(os.path.join(RAW_DIR, fname), os.path.join(BACKUP_DIR, fname))

    print(">>> Writing SHA256SUMS (LF)...")
    lines = []
    for fname in sorted(os.listdir(RAW_DIR)):
        with open(os.path.join(RAW_DIR, fname), "rb") as f:
            lines.append(f"{hashlib.sha256(f.read()).hexdigest()}  {fname}")
    with open(SHA256_FILE, "wb") as f:
        f.write("\n".join(lines).encode("utf-8") + b"\n")

    save_metadata(metadata)
    print(f"\nDone: {len(metadata)} runs in {OUT_DIR}")


if __name__ == "__main__":
    main()
