#!/usr/bin/env python3
"""
Collection script for Struct v0.3 real model outputs (Milestone R3d, Run 3).

Follows RULES.md:
- Uses llama-server.exe on 127.0.0.1:8090, one model at a time.
- Checks nvidia-smi first (Rule 7) and ensures no process holds > 1 GB.
- Checks disk free space on C: (Rule 6) and ensures > 25 GB free.
- Uses exact server flags specified in R3d.md Section 1.
- Uses POST /v1/chat/completions with a single user message (no prefills).
- No output cap (no max_tokens, no n_predict).
- Aborts any run after 20 minutes (1200s) as timeout safety net.
- Uses reasoning_content for thinking text and tokenize endpoint for token counts.
- Saves raw outputs to bench/r3/real/run3/raw/, backups to bench/r3/real/run3/backup/,
  generates bench/r3/real/run3/SHA256SUMS with LF endings, metadata.json and ground_truth.json.
- Stores full request body and full server response JSON in metadata.json.
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

# Ensure bench/r3 is on sys.path
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)

from tasks import TASKS, get_task_prompt

STRUCT_ROOT = r"C:\Users\Ashman Das\Documents\Struct"
TURBOQUANT_DIR = r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp"
LLAMA_SERVER = os.path.join(TURBOQUANT_DIR, "llama-server.exe")

RUN3_DIR = os.path.join(STRUCT_ROOT, "bench", "r3", "real", "run3")
RAW_DIR = os.path.join(RUN3_DIR, "raw")
BACKUP_DIR = os.path.join(RUN3_DIR, "backup")
SHA256_FILE = os.path.join(RUN3_DIR, "SHA256SUMS")
METADATA_FILE = os.path.join(RUN3_DIR, "metadata.json")
GROUND_TRUTH_FILE = os.path.join(RUN3_DIR, "ground_truth.json")

# Model server definitions per R3d.md Section 1
SERVER_DEFS = {
    "llama_3b": {
        "name": "Llama-3.2-3B-Instruct",
        "cmd_args": [
            "-m", r"models\lmstudio-community\Llama-3.2-3B-Instruct-GGUF\Llama-3.2-3B-Instruct-Q4_K_M.gguf",
            "-ngl", "999",
            "--mlock",
            "--no-mmap",
            "--cache-type-k", "turbo4",
            "--cache-type-v", "turbo3",
            "-fa", "on",
            "-c", "131072"
        ],
        "configs": [
            {"key": "llama_3b", "name": "Llama-3.2-3B (thinking off)", "thinking": False, "template_kwargs": None}
        ]
    },
    "gemma": {
        "name": "gemma-4-E2B-it",
        "cmd_args": [
            "-m", r"models\lmstudio-community\gemma-4-E2B-it-GGUF\gemma-4-E2B-it-Q4_K_M.gguf",
            "-ngl", "999",
            "--mlock",
            "--no-mmap",
            "--cache-type-k", "turbo4",
            "--cache-type-v", "turbo3",
            "-fa", "on",
            "-c", "131072"
        ],
        "configs": [
            {"key": "gemma_off", "name": "Gemma-4-E2B (thinking off)", "thinking": False, "template_kwargs": {"enable_thinking": False}},
            {"key": "gemma_on", "name": "Gemma-4-E2B (thinking on)", "thinking": True, "template_kwargs": {"enable_thinking": True}},
        ]
    },
    "lfm": {
        "name": "LFM2.5-8B-A1B",
        "cmd_args": [
            "-m", r"C:\Users\Ashman Das\Downloads\model file\LFM2.5-8B-A1B-Q4_K_M.gguf",
            "-ngl", "999",
            "--mlock",
            "--no-mmap",
            "--cache-type-k", "turbo4",
            "--cache-type-v", "turbo3",
            "-fa", "on",
            "-c", "128000"
        ],
        "configs": [
            {"key": "lfm", "name": "LFM2.5-8B (thinking on)", "thinking": True, "template_kwargs": None}
        ]
    },
    "mellum": {
        "name": "Mellum2.1-12B-A2.5B-Thinking",
        "cmd_args": [
            "-m", r"C:\Users\Ashman Das\Downloads\model file\Mellum2.1-12B-A2.5B-Thinking-Q4_K_M.gguf",
            "-ngl", "999",
            "--mlock",
            "--no-mmap",
            "--cache-type-k", "turbo4",
            "--cache-type-v", "turbo3",
            "-fa", "on",
            "-c", "128000",
            "--n-cpu-moe", "4"
        ],
        "configs": [
            {"key": "mellum", "name": "Mellum2.1-12B (thinking on)", "thinking": True, "template_kwargs": None}
        ]
    }
}

SCHEMAS = ["s1_flat", "s2_nested_fixed", "s3_nested_variable", "s4_two_class_code"]
SEEDS = [42, 43, 44, 45, 46]

def check_disk_space():
    total, used, free = shutil.disk_usage("C:")
    free_gb = free / (1024**3)
    print(f"C: free space: {free_gb:.2f} GB")
    if free_gb < 25.0:
        raise RuntimeError(f"Disk space below 25 GB limit: {free_gb:.2f} GB available")

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

def start_server(server_key):
    info = SERVER_DEFS[server_key]
    print(f"\n========================================================")
    print(f"Starting llama-server for {info['name']}...")
    print(f"========================================================")

    cmd = [
        LLAMA_SERVER,
        "--host", "127.0.0.1",
        "--port", "8090",
    ] + info["cmd_args"]

    proc = subprocess.Popen(cmd, cwd=TURBOQUANT_DIR, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    ready = False
    for i in range(120):
        time.sleep(1)
        if proc.poll() is not None:
            raise RuntimeError(f"Server exited prematurely with code {proc.returncode} for {info['name']}")
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
        raise RuntimeError(f"Server failed to start within 120s for {info['name']}")
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
    except Exception as e:
        print(f"Tokenize error: {e}")
        return len(text.split())

def run_chat_completion(payload, timeout_sec=1200):
    req = urllib.request.Request(
        "http://127.0.0.1:8090/v1/chat/completions",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=timeout_sec) as res:
        return json.loads(res.read())

def main():
    check_disk_space()
    if not check_gpu():
        print("Aborting: GPU memory check failed.")
        sys.exit(1)

    os.makedirs(RAW_DIR, exist_ok=True)
    os.makedirs(BACKUP_DIR, exist_ok=True)

    # Save ground truth in run3 folder
    ground_truth_export = {k: v["ground_truth"] for k, v in TASKS.items()}
    with open(GROUND_TRUTH_FILE, "w", encoding="utf-8") as f:
        json.dump(ground_truth_export, f, indent=2)
    print(f"Ground truth written to {GROUND_TRUTH_FILE}")

    # Pre-generate task prompts
    task_prompts = {}
    for schema_key in SCHEMAS:
        task_prompts[schema_key] = get_task_prompt(schema_key)
        print(f"Task prompt for {schema_key}: {len(task_prompts[schema_key])} chars")

    metadata = []
    server_order = ["llama_3b", "gemma", "lfm", "mellum"]

    for server_key in server_order:
        s_info = SERVER_DEFS[server_key]
        proc = start_server(server_key)

        try:
            for cfg in s_info["configs"]:
                model_key = cfg["key"]
                model_name = cfg["name"]
                is_thinking_model = cfg["thinking"]
                tmpl_kwargs = cfg["template_kwargs"]

                print(f"\n========================================================")
                print(f">>> Running model config: {model_name} ({model_key})")
                print(f"========================================================")

                for schema_key in SCHEMAS:
                    print(f"\n--- Schema: {schema_key} (5 seeds) ---")
                    prompt_text = task_prompts[schema_key]

                    for run_idx, seed in enumerate(SEEDS, start=1):
                        payload = {
                            "messages": [{"role": "user", "content": prompt_text}],
                            "temperature": 0.7,
                            "seed": seed,
                            "stream": False,
                        }
                        if tmpl_kwargs is not None:
                            payload["chat_template_kwargs"] = tmpl_kwargs

                        t0 = time.time()
                        timed_out = False
                        resp = None
                        finish_reason = "unknown"
                        content = ""
                        reasoning_content = ""

                        try:
                            resp = run_chat_completion(payload, timeout_sec=1200)
                            dt = time.time() - t0
                            choice = resp.get("choices", [{}])[0]
                            finish_reason = choice.get("finish_reason", "unknown")
                            msg = choice.get("message", {})
                            content = msg.get("content", "") or ""
                            reasoning_content = msg.get("reasoning_content", "") or ""
                        except urllib.error.URLError as e:
                            dt = time.time() - t0
                            if "timed out" in str(e).lower():
                                timed_out = True
                                finish_reason = "timeout"
                                print(f"  Run {run_idx} (seed {seed}) TIMED OUT after {dt:.1f}s")
                            else:
                                print(f"  Run {run_idx} (seed {seed}) HTTP ERROR: {e}")
                                finish_reason = f"error: {e}"
                            resp = {"error": str(e), "timeout": timed_out}

                        # Token counts: count via tokenize endpoint as usage does not split reasoning
                        thinking_tokens = 0
                        struct_tokens = 0
                        if reasoning_content:
                            thinking_tokens = tokenize_text(reasoning_content)
                        if content:
                            struct_tokens = tokenize_text(content)

                        # Check usage if available
                        usage = resp.get("usage", {}) if resp else {}
                        total_completion_tokens = usage.get("completion_tokens", thinking_tokens + struct_tokens)

                        filename = f"{model_key}_{schema_key}_run{run_idx}.txt"
                        raw_file_path = os.path.join(RAW_DIR, filename)

                        # Write raw output exact bytes
                        with open(raw_file_path, "wb") as f:
                            f.write(content.encode("utf-8"))

                        # Save thinking text only if non-empty
                        thinking_filename = None
                        if reasoning_content.strip():
                            thinking_filename = f"{model_key}_{schema_key}_run{run_idx}_thinking.txt"
                            thinking_file_path = os.path.join(RAW_DIR, thinking_filename)
                            with open(thinking_file_path, "wb") as f:
                                f.write(reasoning_content.encode("utf-8"))

                        run_meta = {
                            "model": model_key,
                            "model_name": model_name,
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
                        }
                        metadata.append(run_meta)

                        print(f"  Run {run_idx} (seed {seed}): finish_reason={finish_reason}, {struct_tokens} struct tok, {thinking_tokens} think tok, {dt:.1f}s")

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
