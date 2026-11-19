#!/usr/bin/env python3
"""
Collection script for Struct v0.3 real model outputs (R3c Part C).

Runs 5 model configurations across 4 schemas, 5 runs each (100 runs total).
Follows Rule 7: checks nvidia-smi first, runs one model at a time on 127.0.0.1:8090,
and stops the server afterwards.
Saves raw outputs to bench/r3/real/run2/raw/, backs up to bench/r3/real/run2/backup/,
and generates bench/r3/real/run2/SHA256SUMS with LF endings.
"""

import os
import sys
import time
import json
import shutil
import hashlib
import subprocess
import urllib.request
import urllib.error

from tasks import TASKS, get_task_prompt

STRUCT_ROOT = r"C:\Users\Ashman Das\Documents\Struct"
LLAMA_SERVER = r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\llama-server.exe"

RUN2_DIR = os.path.join(STRUCT_ROOT, "bench", "r3", "real", "run2")
RAW_DIR = os.path.join(RUN2_DIR, "raw")
BACKUP_DIR = os.path.join(RUN2_DIR, "backup")
SHA256_FILE = os.path.join(RUN2_DIR, "SHA256SUMS")
METADATA_FILE = os.path.join(RUN2_DIR, "metadata.json")
GROUND_TRUTH_FILE = os.path.join(RUN2_DIR, "ground_truth.json")

MODELS = {
    "llama_3b": {
        "name": "Llama-3.2-3B-Instruct",
        "path": r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\models\lmstudio-community\Llama-3.2-3B-Instruct-GGUF\Llama-3.2-3B-Instruct-Q4_K_M.gguf",
        "ngl": 99,
        "thinking": False,
        "end_tag": None,
    },
    "gemma_off": {
        "name": "gemma-4-E2B-it (thinking off)",
        "path": r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\models\lmstudio-community\gemma-4-E2B-it-GGUF\gemma-4-E2B-it-Q4_K_M.gguf",
        "ngl": 99,
        "thinking": False,
        "end_tag": None,
    },
    "gemma_on": {
        "name": "gemma-4-E2B-it (thinking on)",
        "path": r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\models\lmstudio-community\gemma-4-E2B-it-GGUF\gemma-4-E2B-it-Q4_K_M.gguf",
        "ngl": 99,
        "thinking": True,
        "end_tag": "<channel|>",
    },
    "lfm": {
        "name": "LFM2.5-8B-A1B (thinking on)",
        "path": r"C:\Users\Ashman Das\Downloads\model file\LFM2.5-8B-A1B-Q4_K_M.gguf",
        "ngl": 99,
        "thinking": True,
        "end_tag": "</think>",
    },
    "mellum": {
        "name": "Mellum2.1-12B-A2.5B-Thinking (thinking on)",
        "path": r"C:\Users\Ashman Das\Downloads\model file\Mellum2.1-12B-A2.5B-Thinking-Q4_K_M.gguf",
        "ngl": 16, # Keep expert layers on CPU to comfortably fit within 8 GB VRAM
        "thinking": True,
        "end_tag": "</think>",
    },
}

SEEDS = [42, 43, 44, 45, 46]

def check_gpu():
    try:
        out = subprocess.check_output(
            ["nvidia-smi", "--query-compute-apps=pid,used_memory", "--format=csv,noheader"],
            text=True
        )
        lines = [l.strip() for l in out.splitlines() if l.strip()]
        for l in lines:
            parts = l.split(",")
            if len(parts) >= 2:
                mem_str = parts[1].strip().replace("MiB", "").strip()
                try:
                    mem_val = int(mem_str)
                    if mem_val > 1000:
                        print(f"ERROR: Active GPU compute process holds > 1 GB: {l}")
                        return False
                except ValueError:
                    pass
        return True
    except Exception as e:
        print(f"nvidia-smi check exception: {e}")
        return True

def format_prompt(model_key, prompt_text):
    if model_key == "llama_3b":
        return f"<|start_header_id|>user<|end_header_id|>\n\n{prompt_text}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n"
    elif model_key == "gemma_off":
        return f"<|turn>user\n{prompt_text}<turn|>\n<|turn>model\n"
    elif model_key == "gemma_on":
        return f"<|turn>system\n<|think|><turn|>\n<|turn>user\n{prompt_text}<turn|>\n<|turn>model\n<|channel>thought\n"
    elif model_key == "lfm":
        # Per R3c: LFM runs with thinking on only, don't prefill anything.
        return f"<|im_start|>user\n{prompt_text}<|im_end|>\n<|im_start|>assistant\n"
    elif model_key == "mellum":
        return f"<|im_start|>user\n{prompt_text}<|im_end|>\n<|im_start|>assistant\n<think>\n"
    return prompt_text

def tokenize_text(text):
    if not text:
        return 0
    try:
        req = urllib.request.Request(
            "http://127.0.0.1:8090/tokenize",
            data=json.dumps({"content": text}).encode("utf-8"),
            headers={"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(req, timeout=10) as res:
            data = json.loads(res.read())
            return len(data.get("tokens", []))
    except Exception:
        # Fallback estimation if server tokenize fails
        return len(text.split())

def run_completion(payload):
    req = urllib.request.Request(
        "http://127.0.0.1:8090/completion",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=300) as res:
        return json.loads(res.read())

def start_server(model_key):
    info = MODELS[model_key]
    print(f"\n========================================================")
    print(f"Starting llama-server for {info['name']}...")
    print(f"Path: {info['path']}")
    print(f"ngl: {info['ngl']}")
    print(f"========================================================")

    cmd = [
        LLAMA_SERVER,
        "-m", info["path"],
        "--host", "127.0.0.1",
        "--port", "8090",
        "-ngl", str(info["ngl"]),
        "-c", "4096",
        "-b", "512",
        "-ub", "128",
    ]
    proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    ready = False
    for i in range(90):
        time.sleep(1)
        try:
            with urllib.request.urlopen("http://127.0.0.1:8090/health", timeout=2) as res:
                if res.getcode() == 200:
                    ready = True
                    print(f"Server ready after {i+1}s.")
                    break
        except Exception:
            pass

    if not ready:
        proc.kill()
        raise RuntimeError(f"Server failed to start within 90s for {info['name']}")
    return proc

def stop_server(proc):
    print("Stopping llama-server...")
    proc.terminate()
    try:
        proc.wait(timeout=15)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)
    time.sleep(2)
    # Ensure no lingering llama-server processes
    subprocess.run(["taskkill", "/F", "/IM", "llama-server.exe"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1)
    print("Server stopped.")

def main():
    if not check_gpu():
        print("Aborting: GPU memory check failed.")
        sys.exit(1)

    os.makedirs(RAW_DIR, exist_ok=True)
    os.makedirs(BACKUP_DIR, exist_ok=True)

    # Save ground truth in run2 folder
    ground_truth_export = {k: v["ground_truth"] for k, v in TASKS.items()}
    with open(GROUND_TRUTH_FILE, "w", encoding="utf-8") as f:
        json.dump(ground_truth_export, f, indent=2)
    print(f"Ground truth written to {GROUND_TRUTH_FILE}")

    # Pre-generate task prompts
    task_prompts = {}
    for schema_key in TASKS:
        task_prompts[schema_key] = get_task_prompt(schema_key)
        print(f"Task prompt for {schema_key}: {len(task_prompts[schema_key])} chars")

    metadata = []
    model_order = ["llama_3b", "gemma_off", "gemma_on", "lfm", "mellum"]

    for model_key in model_order:
        info = MODELS[model_key]
        proc = start_server(model_key)

        try:
            for schema_key in ["s1_flat", "s2_nested_fixed", "s3_nested_variable", "s4_two_class_code"]:
                print(f"\n>>> Running {info['name']} on schema {schema_key} (5 runs)...")
                raw_prompt = task_prompts[schema_key]
                formatted_prompt = format_prompt(model_key, raw_prompt)

                for run_idx, seed in enumerate(SEEDS, start=1):
                    payload = {
                        "prompt": formatted_prompt,
                        "temperature": 0.7,
                        "n_predict": 1500,
                        "seed": seed,
                        "stream": False,
                    }

                    t0 = time.time()
                    resp = run_completion(payload)
                    dt = time.time() - t0

                    full_content = resp.get("content", "")
                    # R3c: record hitting token cap ONLY from stop_type == "limit". Keep truncated separate.
                    stopped_limit = (resp.get("stop_type") == "limit")
                    truncated = bool(resp.get("truncated", False))

                    # Handle thinking tags
                    if info["thinking"] and info["end_tag"]:
                        end_tag = info["end_tag"]
                        if end_tag in full_content:
                            parts = full_content.split(end_tag, 1)
                            thinking_content = parts[0] + end_tag
                            struct_output = parts[1]
                            thinking_tokens = tokenize_text(thinking_content)
                            struct_tokens = tokenize_text(struct_output)
                        else:
                            # Tag never appeared: all tokens treated as thinking
                            thinking_content = full_content
                            struct_output = ""
                            thinking_tokens = tokenize_text(thinking_content)
                            struct_tokens = 0
                    else:
                        thinking_content = ""
                        struct_output = full_content
                        thinking_tokens = 0
                        struct_tokens = tokenize_text(struct_output)

                    filename = f"{model_key}_{schema_key}_run{run_idx}.txt"
                    raw_file_path = os.path.join(RAW_DIR, filename)

                    # Write raw outputs in binary mode (exact bytes, no CR LF mangling)
                    with open(raw_file_path, "wb") as f:
                        f.write(struct_output.encode("utf-8"))

                    # R3c: don't write empty thinking files for models that don't think
                    thinking_filename = None
                    if info["thinking"] and thinking_content.strip():
                        thinking_filename = f"{model_key}_{schema_key}_run{run_idx}_thinking.txt"
                        thinking_file_path = os.path.join(RAW_DIR, thinking_filename)
                        with open(thinking_file_path, "wb") as f:
                            f.write(thinking_content.encode("utf-8"))

                    run_meta = {
                        "model": model_key,
                        "model_name": info["name"],
                        "schema": schema_key,
                        "run": run_idx,
                        "seed": seed,
                        "filename": filename,
                        "thinking_filename": thinking_filename,
                        "stopped_limit": stopped_limit,
                        "truncated": truncated,
                        "thinking_tokens": thinking_tokens,
                        "struct_tokens": struct_tokens,
                        "total_tokens": thinking_tokens + struct_tokens,
                        "latency_sec": round(dt, 2),
                    }
                    metadata.append(run_meta)

                    print(f"  Run {run_idx} (seed {seed}): {struct_tokens} struct tok, {thinking_tokens} think tok, {dt:.1f}s, cap_hit={stopped_limit}, trunc={truncated}")

        finally:
            stop_server(proc)

    # Backup files
    print("\n>>> Copying raw outputs to backup directory...")
    for fname in os.listdir(RAW_DIR):
        src = os.path.join(RAW_DIR, fname)
        dst = os.path.join(BACKUP_DIR, fname)
        shutil.copy2(src, dst)

    # Compute SHA256SUMS with LF endings
    print(">>> Generating SHA256SUMS with LF endings...")
    sha_lines = []
    for fname in sorted(os.listdir(RAW_DIR)):
        fpath = os.path.join(RAW_DIR, fname)
        with open(fpath, "rb") as f:
            digest = hashlib.sha256(f.read()).hexdigest()
        sha_lines.append(f"{digest}  {fname}")

    with open(SHA256_FILE, "wb") as f:
        f.write("\n".join(sha_lines).encode("utf-8") + b"\n")

    with open(METADATA_FILE, "w", encoding="utf-8") as f:
        json.dump(metadata, f, indent=2)

    print(f"\nCollection complete: {len(metadata)} outputs saved to {RAW_DIR}")
    print(f"Backed up to {BACKUP_DIR}")
    print(f"Hashes written to {SHA256_FILE} with LF line endings")

if __name__ == "__main__":
    main()
