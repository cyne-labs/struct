#!/usr/bin/env python3
"""
Collection script for Struct v0.3 real model outputs (R3 Part C.1).

Runs 4 models without grammar across 4 schemas, 5 runs each (80 runs total).
Follows Rule 7: checks nvidia-smi first, runs one model at a time on 127.0.0.1:8090,
and stops the server afterwards.
Saves raw outputs to bench/r3/real/raw/, backs up to bench/r3/real/backup/,
and generates bench/r3/real/SHA256SUMS.
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

STRUCT_ROOT = r"C:\Users\Ashman Das\Documents\Struct"
LLAMA_SERVER = r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\llama-server.exe"

RAW_DIR = os.path.join(STRUCT_ROOT, "bench", "r3", "real", "raw")
BACKUP_DIR = os.path.join(STRUCT_ROOT, "bench", "r3", "real", "backup")
SHA256_FILE = os.path.join(STRUCT_ROOT, "bench", "r3", "real", "SHA256SUMS")
METADATA_FILE = os.path.join(STRUCT_ROOT, "bench", "r3", "real", "metadata.json")

MODELS = {
    "llama": {
        "name": "Llama-3.2-1B-Instruct-Uncensored",
        "path": r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\models\mradermacher\Llama-3.2-1B-Instruct-Uncensored-GGUF\Llama-3.2-1B-Instruct-Uncensored.Q4_K_M.gguf",
        "ngl": 99,
        "thinking": False,
    },
    "gemma": {
        "name": "gemma-4-E2B-it",
        "path": r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\models\lmstudio-community\gemma-4-E2B-it-GGUF\gemma-4-E2B-it-Q4_K_M.gguf",
        "ngl": 99,
        "thinking": False,
    },
    "lfm": {
        "name": "LFM2.5-8B-A1B",
        "path": r"C:\Users\Ashman Das\Downloads\model file\LFM2.5-8B-A1B-Q4_K_M.gguf",
        "ngl": 99,
        "thinking": False,
    },
    "mellum": {
        "name": "Mellum2.1-12B-A2.5B-Thinking",
        "path": r"C:\Users\Ashman Das\Downloads\model file\Mellum2.1-12B-A2.5B-Thinking-Q4_K_M.gguf",
        "ngl": 16, # Keep experts on CPU to fit within 8 GB VRAM
        "thinking": True,
    },
}

SCHEMAS = {
    "s1_flat": {
        "file": os.path.join(STRUCT_ROOT, "bench", "r2", "schemas", "s1_flat.stc"),
        "description": "Flat tabular claim schema (1 class, 5 fields)",
        "prompt": """schema:claim{id, title, verdict:enum(supported,refuted,unverified), year:int, verified:bool}

Format: Output each record as '|' separated values matching the schema columns, ending each line with '|'.
Task: Write 5 records from the given notes.
Notes:
1. Claim c1: Attention is all you need, supported, published 2017, verified true.
2. Claim c2: FlashAttention memory reduction on A100, supported, published 2022, verified true.
3. Claim c3: Llama 3 405B runs on single 8GB GPU, refuted, published 2024, verified false.
4. Claim c4: Speculative decoding achieves 3x speedup, unverified, published 2023, verified false.
5. Claim c5: Quantization below 2 bits preserves full FP16 perplexity, refuted, published 2024, verified false.""",
    },
    "s2_nested_fixed": {
        "file": os.path.join(STRUCT_ROOT, "bench", "r2", "schemas", "s2_nested_fixed.stc"),
        "description": "Nested fixed-shape claim schema (1 class, nested record)",
        "prompt": """schema:claim{text, verdict:enum(supported,refuted,unverified), source:record{id, year:int}, verified:bool}

Format: Output each record as '|' separated values with record columns flattened in order, ending each line with '|'.
Task: Write 5 records from the given notes.
Notes:
1. Linear attention scales linearly with sequence length, supported, source arxiv.1234 published 2020, verified true.
2. Mixture of experts routes tokens dynamically, supported, source arxiv.5678 published 2021, verified true.
3. LoRA fine-tuning beats full fine-tuning on all benchmarks, refuted, source arxiv.9999 published 2023, verified false.
4. 1-bit LLMs match full precision on reasoning tasks, unverified, source arxiv.4444 published 2024, verified false.
5. Direct preference optimization replaces RLHF entirely, unverified, source arxiv.7777 published 2023, verified true.""",
    },
    "s3_nested_variable": {
        "file": os.path.join(STRUCT_ROOT, "bench", "r2", "schemas", "s3_nested_variable.stc"),
        "description": "Nested variable claim schema (1 class, nested record and list of records)",
        "prompt": """schema:claim{text, verdict:enum(supported,refuted,unknown), source:record{id, year:int}, notes:list[record{text, subnotes:list[str]}]}

Format: Output each record as '|' separated values, with list items on subsequent lines starting with '>' and sublists with '>>', ending lines with '|'.
Task: Write 5 records from the given notes.
Notes:
1. Scaling laws hold across modalities, supported, source s1 in 2024. Notes: experimental verification (subnotes: vision tested, audio tested).
2. KV cache compression loses zero accuracy, refuted, source s2 in 2023. Notes: 4-bit KV causes drift.
3. Speculative decoding reduces latency without output change, supported, source s3 in 2023. Notes: lossless verification (subnotes: greedy matching confirmed).
4. Pruning eliminates 70% parameters without fine-tuning, unknown, source s4 in 2024. Notes: needs deeper benchmark.
5. Context length extension to 1M tokens requires zero retargeting, refuted, source s5 in 2024. Notes: RoPE scaling needed (subnotes: theta adjustment required).""",
    },
    "s4_two_class_code": {
        "file": os.path.join(STRUCT_ROOT, "bench", "r2", "schemas", "s4_two_class_code.stc"),
        "description": "Two-class schema with code field requiring \\r raw values",
        "prompt": """schema:file{name, path}
schema:patch{file_name, code, line:int}

Format: Output lines for each class block beginning with 'schema:name', with multiline code written as a raw value starting with '\\r' and ending with '|'.
Task: Write 5 records from the given notes.
Notes:
1. File 'lib.rs' at path 'src/lib.rs'.
2. Patch for 'lib.rs' at line 10 with code:
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
3. Patch for 'lib.rs' at line 20 with code:
pub fn sub(a: i32, b: i32) -> i32 {
    a - b
}
4. File 'main.rs' at path 'src/main.rs'.
5. Patch for 'main.rs' at line 1 with code:
fn main() {
    println!(\"hello\");
}""",
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

def format_prompt(model_key, prompt_text, thinking=False):
    if model_key == "llama":
        return f"<|start_header_id|>user<|end_header_id|>\n\n{prompt_text}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n"
    elif model_key == "gemma":
        return f"<|turn>user\n{prompt_text}<turn|>\n<|turn>model\n"
    elif model_key == "lfm":
        # In LFM's GGUF chat template (tokenizer.chat_template), thinking blocks are rendered when
        # message.thinking is present, and assistant generation unconditionally starts with
        # '<|im_start|>assistant\n'. There is no system prompt toggle or Jinja variable to turn thinking off.
        # Prefilling the assistant turn with '<think>\n</think>\n' pre-closes the think block so generation
        # immediately emits response tokens without thinking.
        return f"<|im_start|>user\n{prompt_text}<|im_end|>\n<|im_start|>assistant\n<think>\n</think>\n"
    elif model_key == "mellum":
        return f"<|im_start|>user\n{prompt_text}<|im_end|>\n<|im_start|>assistant\n<think>\n"
    return prompt_text

def tokenize_text(text):
    if not text:
        return 0
    req = urllib.request.Request(
        "http://127.0.0.1:8090/tokenize",
        data=json.dumps({"content": text}).encode("utf-8"),
        headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=10) as res:
        data = json.loads(res.read())
        return len(data.get("tokens", []))

def run_completion(payload):
    req = urllib.request.Request(
        "http://127.0.0.1:8090/completion",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=180) as res:
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
        "-c", "2048",
        "-b", "512",
        "-ub", "128",
    ]
    proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    ready = False
    for i in range(60):
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
        raise RuntimeError(f"Server failed to start within 60s for {info['name']}")
    return proc

def stop_server(proc):
    print("Stopping llama-server...")
    proc.terminate()
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)
    time.sleep(2)
    print("Server stopped.")

def main():
    if not check_gpu():
        print("Aborting: GPU memory check failed.")
        sys.exit(1)

    os.makedirs(RAW_DIR, exist_ok=True)
    os.makedirs(BACKUP_DIR, exist_ok=True)

    metadata = []

    model_order = ["llama", "gemma", "lfm", "mellum"]

    for model_key in model_order:
        info = MODELS[model_key]
        proc = start_server(model_key)

        try:
            for schema_key, s_data in SCHEMAS.items():
                print(f"\n>>> Running {info['name']} on schema {schema_key} (5 runs)...")
                raw_prompt = s_data["prompt"]
                formatted_prompt = format_prompt(model_key, raw_prompt, thinking=info["thinking"])

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
                    # llama-server records token limit exhaustion in "truncated": bool and "stop_type": "limit"
                    stopped_limit = bool(resp.get("truncated", False) or (resp.get("stop_type") == "limit"))

                    # Handle thinking tags
                    if info["thinking"]:
                        end_tag = "</think>"
                        if end_tag in full_content:
                            parts = full_content.split(end_tag, 1)
                            thinking_content = parts[0] + end_tag
                            # Keep exact output bytes without stripping any characters
                            struct_output = parts[1]
                            thinking_tokens = tokenize_text(thinking_content)
                            struct_tokens = tokenize_text(struct_output)
                        else:
                            # Tag never appeared: all tokens are thinking
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

                    # Write raw outputs in binary mode so exact bytes are kept (no CR LF conversion)
                    with open(raw_file_path, "wb") as f:
                        f.write(struct_output.encode("utf-8"))

                    # Save thinking text as its own file next to the output in binary mode
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
                        "thinking_tokens": thinking_tokens,
                        "struct_tokens": struct_tokens,
                        "total_tokens": thinking_tokens + struct_tokens,
                        "latency_sec": round(dt, 2),
                    }
                    metadata.append(run_meta)

                    print(f"  Run {run_idx} (seed {seed}): {struct_tokens} struct tok, {thinking_tokens} think tok, {dt:.1f}s, cap_hit={stopped_limit}")

        finally:
            stop_server(proc)

    # Backup files
    print("\n>>> Copying raw outputs to backup directory...")
    for fname in os.listdir(RAW_DIR):
        src = os.path.join(RAW_DIR, fname)
        dst = os.path.join(BACKUP_DIR, fname)
        shutil.copy2(src, dst)

    # Compute SHA256SUMS
    print(">>> Generating SHA256SUMS...")
    sha_lines = []
    for fname in sorted(os.listdir(RAW_DIR)):
        fpath = os.path.join(RAW_DIR, fname)
        with open(fpath, "rb") as f:
            digest = hashlib.sha256(f.read()).hexdigest()
        sha_lines.append(f"{digest}  {fname}")

    with open(SHA256_FILE, "w", encoding="utf-8") as f:
        f.write("\n".join(sha_lines) + "\n")

    with open(METADATA_FILE, "w", encoding="utf-8") as f:
        json.dump(metadata, f, indent=2)

    print(f"\nCollection complete: {len(metadata)} outputs saved to {RAW_DIR}")
    print(f"Backed up to {BACKUP_DIR}")
    print(f"Hashes written to {SHA256_FILE}")

if __name__ == "__main__":
    main()
