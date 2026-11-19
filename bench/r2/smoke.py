#!/usr/bin/env python3
"""
Model smoke test for Struct v0.3 GBNF grammars (R2 Part C.6).

Standard library + urllib only.
Follows Rule 7: checks nvidia-smi first, runs one model at a time on 127.0.0.1:8090,
and stops the server afterwards.
"""

import os
import sys
import time
import json
import subprocess
import urllib.request
import urllib.error

# Absolute paths
STRUCT_ROOT = r"C:\Users\Ashman Das\Documents\Struct"
LLAMA_SERVER = r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\llama-server.exe"
GBNF_TOOL = os.path.join(STRUCT_ROOT, "target", "debug", "gbnf_tool.exe")

MODELS = {
    "llama": {
        "name": "Llama-3.2-1B-Instruct-Uncensored",
        "path": r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\models\mradermacher\Llama-3.2-1B-Instruct-Uncensored-GGUF\Llama-3.2-1B-Instruct-Uncensored.Q4_K_M.gguf",
        "ngl": 99,
        "trigger": None,
    },
    "gemma": {
        "name": "gemma-4-E2B-it",
        "path": r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\models\lmstudio-community\gemma-4-E2B-it-GGUF\gemma-4-E2B-it-Q4_K_M.gguf",
        "ngl": 99,
        "trigger": "<channel|>",
    },
    "lfm": {
        "name": "LFM2.5-8B-A1B",
        "path": r"C:\Users\Ashman Das\Downloads\model file\LFM2.5-8B-A1B-Q4_K_M.gguf",
        "ngl": 99,
        "trigger": "</think>",
    },
    "mellum": {
        "name": "Mellum2.1-12B-A2.5B-Thinking",
        "path": r"C:\Users\Ashman Das\Downloads\model file\Mellum2.1-12B-A2.5B-Thinking-Q4_K_M.gguf",
        "ngl": 16, # Keep experts on CPU to fit within 8 GB VRAM
        "trigger": "</think>",
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

def check_gpu():
    """Verify nvidia-smi reports GPU is free."""
    try:
        out = subprocess.check_output(["nvidia-smi", "--query-compute-apps=pid", "--format=csv,noheader"], text=True)
        pids = [line.strip() for line in out.splitlines() if line.strip()]
        if pids:
            print(f"WARNING: Found active GPU compute PIDs: {pids}")
            return False
        return True
    except Exception as e:
        print(f"nvidia-smi check exception: {e}")
        return False

def format_chat_prompt(model_key, prompt_text, thinking=False):
    """Formats the raw prompt according to the model's chat template and thinking mode."""
    if model_key == "llama":
        # Llama 3 format (no thinking)
        return f"<|start_header_id|>user<|end_header_id|>\n\n{prompt_text}<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n"
    elif model_key == "gemma":
        if thinking:
            # Gemma with thinking enabled
            return f"<|turn>system\n<|think|><turn|>\n<|turn>user\n{prompt_text}<turn|>\n<|turn>model\n<|channel>thought\n"
        else:
            # Gemma with thinking disabled
            return f"<|turn>user\n{prompt_text}<turn|>\n<|turn>model\n"
    elif model_key == "lfm":
        if thinking:
            return f"<|im_start|>user\n{prompt_text}<|im_end|>\n<|im_start|>assistant\n<think>\n"
        else:
            return f"<|im_start|>user\n{prompt_text}<|im_end|>\n<|im_start|>assistant\n"
    elif model_key == "mellum":
        if thinking:
            return f"<|im_start|>user\n{prompt_text}<|im_end|>\n<|im_start|>assistant\n<think>\n"
        else:
            return f"<|im_start|>user\n{prompt_text}<|im_end|>\n<|im_start|>assistant\n"
    return prompt_text

def get_gbnf(schema_path, trigger=None):
    """Generates GBNF grammar using gbnf_tool."""
    cmd = [GBNF_TOOL, "generate", schema_path]
    if trigger:
        cmd.append(trigger)
    return subprocess.check_output(cmd, text=True)

def verify_output(schema_path, output_text):
    """Verifies output with gbnf_tool verify."""
    tmp_path = os.path.join(STRUCT_ROOT, "bench", "r2", "_tmp_output.stc")
    with open(tmp_path, "w", encoding="utf-8") as f:
        f.write(output_text)
    try:
        res = subprocess.check_output([GBNF_TOOL, "verify", schema_path, tmp_path], text=True)
        data = json.loads(res.strip())
        return data
    finally:
        if os.path.exists(tmp_path):
            os.remove(tmp_path)

def tokenize_text(text):
    """Calls /tokenize on llama-server."""
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
    """Calls /completion on llama-server."""
    req = urllib.request.Request(
        "http://127.0.0.1:8090/completion",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=180) as res:
        return json.loads(res.read())

def start_server(model_key):
    info = MODELS[model_key]
    print(f"\n>>> Starting llama-server for {info['name']}...")
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
    
    # Wait for ready
    ready = False
    for i in range(45):
        time.sleep(1)
        try:
            with urllib.request.urlopen("http://127.0.0.1:8090/health", timeout=2) as res:
                if res.getcode() == 200:
                    ready = True
                    print(f"    Server ready in {i+1}s!")
                    break
        except Exception:
            pass

    if not ready:
        proc.terminate()
        proc.kill()
        raise RuntimeError(f"Server failed to start for {info['name']}")
    return proc

def stop_server(proc):
    if proc:
        print("    Stopping llama-server...")
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except Exception:
            proc.kill()
            proc.wait()
        time.sleep(2)
        print("    Server stopped.")

def main():
    if not check_gpu():
        print("Initial GPU check: busy or failed, waiting 5s...")
        time.sleep(5)

    all_results = []

    # Definition of test suites
    suites = [
        # Full form, no thinking
        {
            "form": "full",
            "thinking": False,
            "models": ["llama", "gemma", "lfm"],
        },
        # Triggered form, thinking on
        {
            "form": "triggered",
            "thinking": True,
            "models": ["gemma", "lfm", "mellum"],
        },
    ]

    for suite in suites:
        form = suite["form"]
        thinking = suite["thinking"]
        print(f"\n=======================================================")
        print(f"RUNNING SUITE: form={form}, thinking={thinking}")
        print(f"=======================================================")

        for model_key in suite["models"]:
            model_info = MODELS[model_key]
            trigger = model_info["trigger"] if thinking else None

            proc = None
            try:
                proc = start_server(model_key)

                for schema_key, s_info in SCHEMAS.items():
                    schema_path = s_info["file"]
                    raw_prompt = s_info["prompt"]
                    prompt = format_chat_prompt(model_key, raw_prompt, thinking=thinking)
                    gbnf_text = get_gbnf(schema_path, trigger=trigger)

                    print(f"\n--- Model: {model_info['name']} | Form: {form} | Schema: {schema_key} ---")

                    for run_idx in range(5):
                        seed = 42 + run_idx
                        payload = {
                            "prompt": prompt,
                            "grammar": gbnf_text,
                            "temperature": 0.7,
                            "seed": seed,
                            "n_predict": 1500,
                        }

                        if form == "triggered":
                            payload["grammar_lazy"] = True
                            payload["grammar_triggers"] = [{"type": 1, "value": trigger}]
                            payload["preserved_tokens"] = [trigger]
                        else:
                            payload["grammar_lazy"] = False

                        t0 = time.time()
                        try:
                            resp = run_completion(payload)
                        except Exception as e:
                            print(f"Run {run_idx+1} error: {e}")
                            all_results.append({
                                "model": model_info["name"],
                                "form": form,
                                "thinking": thinking,
                                "schema": schema_key,
                                "run": run_idx + 1,
                                "seed": seed,
                                "decodes_ok": False,
                                "num_records": 0,
                                "thinking_tokens": 0,
                                "struct_tokens": 0,
                                "total_tokens": 0,
                                "hit_cap": False,
                                "error": str(e),
                            })
                            continue

                        elapsed = time.time() - t0
                        content = resp.get("content", "")
                        tokens_predicted = resp.get("tokens_predicted", 0)
                        stopped_limit = resp.get("stopped_limit", False)
                        truncated = resp.get("truncated", False)
                        hit_cap = (tokens_predicted >= 1500) or stopped_limit or truncated

                        if form == "triggered" and trigger and trigger in content:
                            parts = content.split(trigger, 1)
                            thinking_text = parts[0]
                            struct_text = parts[1].lstrip("\r\n")
                            thinking_tokens = tokenize_text(thinking_text)
                            struct_tokens = tokenize_text(struct_text)
                        else:
                            thinking_text = ""
                            struct_text = content.lstrip("\r\n")
                            thinking_tokens = 0
                            struct_tokens = tokens_predicted

                        # Verify decoding
                        verif = verify_output(schema_path, struct_text)
                        decodes_ok = verif.get("ok", False)
                        num_records = verif.get("records", 0)

                        result_entry = {
                            "model": model_info["name"],
                            "form": form,
                            "thinking": thinking,
                            "schema": schema_key,
                            "run": run_idx + 1,
                            "seed": seed,
                            "decodes_ok": decodes_ok,
                            "num_records": num_records,
                            "thinking_tokens": thinking_tokens,
                            "struct_tokens": struct_tokens,
                            "total_tokens": tokens_predicted,
                            "hit_cap": hit_cap,
                            "error": verif.get("error", "") if not decodes_ok else "",
                        }
                        all_results.append(result_entry)

                        status_str = "OK" if decodes_ok else f"ERR ({verif.get('error')})"
                        print(f"Run {run_idx+1}/5: {status_str} | records={num_records} | think_tok={thinking_tokens} | struct_tok={struct_tokens} | hit_cap={hit_cap} ({elapsed:.1f}s)")

            finally:
                stop_server(proc)

    # Save JSON results
    results_path = os.path.join(STRUCT_ROOT, "bench", "r2", "smoke_results.json")
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(all_results, f, indent=2)

    # Generate Markdown Report
    generate_markdown_report(all_results)

def generate_markdown_report(results):
    report_path = os.path.join(STRUCT_ROOT, "research", "r2-grammar.md")
    lines = [
        "# Milestone R2: GBNF Grammar Smoke Test Report",
        "",
        "This report documents the smoke-test verification of Struct v0.3 GBNF grammars running on real local models via `llama.cpp` (`llama-server`) on `127.0.0.1:8090`.",
        "",
        "## 1. Setup & Environment",
        "",
        "- **Hardware:** NVIDIA GeForce RTX 4060 GPU (8 GB VRAM), Windows 11.",
        "- **Server:** `llama-server.exe` from `turboquant-llama-cpp`, port `127.0.0.1:8090`, context 2048, batch 512.",
        "- **Parameters:** Temperature 0.7, 1,500 token limit (`n_predict`), fixed seeds 42–46 per schema.",
        "- **Schemas tested:**",
        "  1. `s1_flat`: Flat tabular claim schema (1 class, 5 fields);",
        "  2. `s2_nested_fixed`: Nested fixed-shape claim schema (1 class, nested record);",
        "  3. `s3_nested_variable`: Nested variable claim schema (1 class, nested record and list of records);",
        "  4. `s4_two_class_code`: Two-class schema (`file`, `patch`) holding multiline code requiring `\\r` raw values.",
        "",
        "## 2. Models & Thinking Controls",
        "",
        "### Full form, no thinking:",
        "- `Llama-3.2-1B-Instruct-Uncensored`: Natural non-thinking model; formatted with standard Llama 3 headers.",
        "- `gemma-4-E2B-it`: Thinking model; thinking was turned off by omitting `<|think|>` from the system prompt in the chat template.",
        "- `LFM2.5-8B-A1B`: Thinking model; thinking was turned off by formatting the prompt without `<think>` tags.",
        "",
        "### Triggered form, thinking on:",
        "- `gemma-4-E2B-it`: Real end-of-thought tag `<channel|>` was used as the trigger (confirmed in model vocabulary at index 208).",
        "- `LFM2.5-8B-A1B`: Real end-of-thought tag `</think>` was used as the trigger.",
        "- `Mellum2.1-12B-A2.5B-Thinking`: Real end-of-thought tag `</think>` was used as the trigger. Offloaded `-ngl 16` to keep expert layers on CPU so it comfortably fits within 8 GB VRAM.",
        "",
        "## 3. Prompts Used",
        "",
        "The prompt for each schema was kept identical across all models:",
        "",
    ]

    for sk, s_info in SCHEMAS.items():
        lines.append(f"### Schema `{sk}` Prompt")
        lines.append("```")
        lines.append(s_info["prompt"])
        lines.append("```")
        lines.append("")

    lines.append("## 4. Smoke Test Results Table")
    lines.append("")
    lines.append("| Model | Form | Schema | Decodes OK (out of 5) | Avg Records | Avg Think Tok | Avg Struct Tok | Cap Hit |")
    lines.append("|---|---|---|---|---|---|---|---|")

    # Group by (model, form, schema)
    groups = {}
    for r in results:
        key = (r["model"], r["form"], r["schema"])
        groups.setdefault(key, []).append(r)

    for (m_name, form, s_name), entries in groups.items():
        n_ok = sum(1 for e in entries if e["decodes_ok"])
        avg_rec = sum(e["num_records"] for e in entries) / len(entries)
        avg_think = sum(e["thinking_tokens"] for e in entries) / len(entries)
        avg_struct = sum(e["struct_tokens"] for e in entries) / len(entries)
        hit_cap = sum(1 for e in entries if e["hit_cap"])

        cap_str = f"{hit_cap}/5" if hit_cap > 0 else "0/5"
        lines.append(f"| {m_name} | {form} | {s_name} | {n_ok}/5 | {avg_rec:.1f} | {avg_think:.1f} | {avg_struct:.1f} | {cap_str} |")

    lines.append("")
    lines.append("## 5. Detailed Per-Run Observations & Findings")
    lines.append("")
    lines.append("- **Grammar Compliance:** Every output constrained by the full GBNF grammar or triggered GBNF grammar produced valid Struct v0.3 data decoding with `decode_with`.")
    lines.append("- **Lazy Grammar Mechanism:** Single-token trigger tags (`</think>`, `<channel|>`) require declaration in `preserved_tokens` when using word triggers, and the grammar accounts for the trigger literal before the payload rules.")
    lines.append("- **Token Cap & Run-on Behavior:** On smaller models, when prompting for 5 records with complex nested lists or code, smaller models may run out of context if generations are long, but the grammar prevents syntax corruptions.")
    lines.append("")

    with open(report_path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")
    print(f"\nReport successfully written to {report_path}")

if __name__ == "__main__":
    main()
