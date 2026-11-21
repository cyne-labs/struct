#!/usr/bin/env python3
"""
Collection harness for Struct Milestone R4: retrieval accuracy across formats.

Follows RULES.md and R4.md:
- Target model: Gemma 4 E2B only, thinking on (R4 decision 14).
- Server: turboquant build at C:\\Users\\Ashman Das\\Documents\\turboquant-llama-cpp, port 8090 (per RULES rule 7).
- KV cache turbo4 K / turbo3 V, reasoning budget 8192.
- Sampling: temp 1.0, top_p 0.95, top_k 64, min_p 0.0, presence_penalty 0.0, repeat_penalty 1.0.
- POST /v1/chat/completions with enable_thinking: true, 20-min safety timeout per request.
- 5 seeds: 42 to 46.
- 4 datasets: flat_tabular, flat_100, nested_fixed, nested_variable.
- 7 formats for flat datasets (json_min, toon, yaml, tsv, xml, key_value, struct).
- 5 formats for nested datasets (json_min, toon, yaml, xml, struct).
  (TSV and key: value skipped on nested datasets per Decision 1: 24 requests/seed * 5 = 120 total requests).
- Saves raw outputs to bench/r4/real/raw/, backups to bench/r4/real/backup/,
  metadata to bench/r4/real/metadata.json, and SHA256SUMS.
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
import threading
from concurrent.futures import ThreadPoolExecutor

# Add parent directory to path
BENCH_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STRUCT_ROOT = os.path.dirname(BENCH_DIR)
R4_DIR = os.path.join(BENCH_DIR, "r4")
sys.path.insert(0, BENCH_DIR)

from r4.formats import ENCODERS, FORMAT_DISPLAY_NAMES

SERVER_DIR = r"C:\Users\Ashman Das\Documents\turboquant-llama-cpp"
LLAMA_SERVER = os.path.join(SERVER_DIR, "llama-server.exe")
# R4_PROFILE picks the model. Default "gemma4e2b" (R4 decision 14); "qwen35_4b" is Qwen3.5-4B with MTP.
R4_PROFILE = os.environ.get("R4_PROFILE", "gemma4e2b")
PROFILES = {
    "gemma4e2b": {
        "model": os.path.join(SERVER_DIR, "models", "lmstudio-community", "gemma-4-E2B-it-GGUF", "gemma-4-E2B-it-Q4_K_M.gguf"),
        "stem": "gemma4e2b",
        "sampling": {"temperature": 1.0, "top_p": 0.95, "top_k": 64, "min_p": 0.0, "presence_penalty": 0.0, "repeat_penalty": 1.0},
        "extra_flags": [],
    },
    "qwen35_4b": {
        "model": os.path.join(SERVER_DIR, "models", "unsloth", "Qwen3.5-4B-GGUF", "Qwen3.5-4B-Q4_K_M.gguf"),
        "stem": "qwen35_4b",
        # Qwen3.5 model card, thinking mode, general tasks
        "sampling": {"temperature": 1.0, "top_p": 0.95, "top_k": 20, "min_p": 0.0, "presence_penalty": 1.5, "repeat_penalty": 1.0},
        "extra_flags": ["--spec-type", "draft-mtp,ngram-mod", "--spec-draft-n-max", "3", "-ctkd", "turbo3", "-ctvd", "turbo2",
                        "--spec-ngram-mod-n-max", "3", "--spec-ngram-mod-n-min", "2", "--spec-ngram-mod-n-match", "8"],
    },
}
PROFILE = PROFILES[R4_PROFILE]
MODEL_PATH = PROFILE["model"]
SERVER_BASE = "http://127.0.0.1:8090"

LAUNCH_LINE = (
    r'"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\llama-server.exe" -m '
    r'"C:\Users\Ashman Das\Documents\turboquant-llama-cpp\models\lmstudio-community\gemma-4-E2B-it-GGUF\gemma-4-E2B-it-Q4_K_M.gguf" '
    r'-ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on -c 65536 -np 4 --kv-unified '
    r'--reasoning-budget 8192 --temp 1.0 --top-p 0.95 --top-k 64 --min-p 0.0 '
    r'--presence-penalty 0.0 --repeat-penalty 1.0 --port 8090 --host 127.0.0.1'
)

DATA_PATHS = {
    "flat_tabular": os.path.join(R4_DIR, "data", "flat_tabular.min.json"),
    "flat_100": os.path.join(R4_DIR, "data", "flat_100.min.json"),
    "nested_fixed": os.path.join(R4_DIR, "data", "nested_fixed.min.json"),
    "nested_variable": os.path.join(BENCH_DIR, "r1", "fixtures", "nested_variable.min.json"),
}

QUESTIONS_PATH = os.path.join(R4_DIR, "questions.json")
# R4_REAL_DIR (optional) sends a repeat run to its own folder, e.g. real_run2, so it never overwrites real/.
REAL_DIR = os.environ.get("R4_REAL_DIR") or os.path.join(R4_DIR, "real")
RAW_DIR = os.path.join(REAL_DIR, "raw")
BACKUP_DIR = os.path.join(REAL_DIR, "backup")
SHA256_FILE = os.path.join(REAL_DIR, "SHA256SUMS")
METADATA_FILE = os.path.join(REAL_DIR, "metadata.json")
SERVER_PROPS_FILE = os.path.join(REAL_DIR, "server_props.json")
LAUNCH_LINE_FILE = os.path.join(REAL_DIR, "launch_line.txt")

def fetch_server_props(server_base=SERVER_BASE):
    """Fetches server properties from /props."""
    req = urllib.request.Request(server_base + "/props")
    with urllib.request.urlopen(req, timeout=10) as res:
        return json.loads(res.read())

def get_launch_flags_from_props(props):
    """
    Extracts or reconstructs the real server launch flags from the server's /props response.
    Reads 'launch_args', 'flags', or 'cmdline' if present, or derives them from props settings
    (model_path, n_ctx, total_slots, etc.).
    """
    if not isinstance(props, dict):
        return ""
    if "launch_args" in props and props["launch_args"]:
        return str(props["launch_args"]).strip()
    if "flags" in props and props["flags"]:
        return str(props["flags"]).strip()
    if "cmdline" in props and props["cmdline"]:
        return str(props["cmdline"]).strip()

    parts = []
    model = props.get("model_path") or props.get("model_alias")
    if model:
        parts.append(f"-m {model}")
    dgs = props.get("default_generation_settings", {})
    n_ctx = dgs.get("n_ctx")
    if n_ctx:
        parts.append(f"-c {n_ctx}")
    if "reasoning_budget" in props:
        parts.append(f"--reasoning-budget {props['reasoning_budget']}")
    if "port" in props:
        parts.append(f"--port {props['port']}")
    if "host" in props:
        parts.append(f"--host {props['host']}")

    if parts:
        return "llama-server.exe " + " ".join(parts)
    return ""

DATASETS = ["flat_tabular", "flat_100", "nested_fixed", "nested_variable"]
FLAT_DATASETS = ["flat_tabular", "flat_100"]
NESTED_DATASETS = ["nested_fixed", "nested_variable"]

ALL_FORMATS = ["json_min", "toon", "yaml", "tsv", "xml", "key_value", "struct"]
SEEDS = [42, 43, 44, 45, 46]
TIMEOUT_SEC = 1200 # 20 minutes
PARALLEL = 4  # concurrent requests, matches the server's -np 4 (R4 decision 14)

SAMPLING = dict(PROFILE["sampling"])

FENCE_TAGS = {
    "json_min": "json",
    "toon": "toon",
    "yaml": "yaml",
    "tsv": "tsv",
    "xml": "xml",
    "key_value": "text",
    "struct": "struct",
}

def load_dataset(dataset_name):
    path = DATA_PATHS[dataset_name]
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)

def load_questions():
    with open(QUESTIONS_PATH, "r", encoding="utf-8") as f:
        return json.load(f)

def build_prompt(format_name, dataset_name, encoded_data, questions):
    """
    Builds the standardized R4 retrieval prompt.
    The prompt is identical for every format except the data block and one line naming the format.
    """
    disp_name = FORMAT_DISPLAY_NAMES[format_name]
    fence = FENCE_TAGS.get(format_name, "text")

    lines = [
        f"The following data is provided in {disp_name} format:\n",
        f"```{fence}\n",
        encoded_data,
        "\n```\n\n",
        "Answer each of the following questions based only on the data above.\n",
        'Provide your answers as a numbered list, one short answer per line (e.g. "1. answer"):\n\n',
    ]

    for idx, q in enumerate(questions, start=1):
        lines.append(f"{idx}. {q['question']}\n")

    return "".join(lines)

def check_nvidia_smi():
    """
    RULES Rule 7: check nvidia-smi during collection while llama-server is running.
    Checks that the process holding the GPU is llama-server, and prints what it found.
    Never passes silently on a parse error.
    """
    try:
        res = subprocess.check_output(
            ["nvidia-smi", "--query-compute-apps=pid,process_name", "--format=csv,noheader,nounits"],
            text=True
        )
        lines = [l.strip() for l in res.strip().splitlines() if l.strip()]
        if not lines:
            print("ERROR: No compute process found holding the GPU (expected llama-server).")
            return False

        for line in lines:
            parts = [p.strip() for p in line.split(",")]
            if len(parts) < 2:
                print(f"ERROR: Cannot parse nvidia-smi compute-apps line: '{line}'")
                return False
            pid, proc_name = parts[0], parts[1]
            print(f"Found GPU compute process: {proc_name} (PID {pid})")
            if "llama-server" not in proc_name.lower():
                print(f"ERROR: Process '{proc_name}' (PID {pid}) is holding GPU, expected llama-server. RULES Rule 7 violated.")
                return False
        return True
    except Exception as e:
        print(f"ERROR: Failed to query or parse nvidia-smi compute-apps: {e}")
        return False

def check_disk_space():
    """Rule 6: check C: free space. Must be > 25 GB."""
    try:
        import ctypes
        free_bytes = ctypes.c_ulonglong(0)
        ctypes.windll.kernel32.GetDiskFreeSpaceExW(
            ctypes.c_wchar_p("C:\\"),
            ctypes.byref(free_bytes),
            None,
            None
        )
        free_gb = free_bytes.value / (1024 ** 3)
        if free_gb < 25.0:
            print(f"ERROR: C: drive free space is {free_gb:.2f} GB (< 25 GB limit)")
            return False
        return True
    except Exception as e:
        print(f"Disk check error: {e}")
        return True

def tokenize_text(text):
    if not text:
        return 0
    try:
        req = urllib.request.Request(
            SERVER_BASE + "/tokenize",
            data=json.dumps({"content": text}).encode("utf-8"),
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=60) as res:
            return len(json.loads(res.read()).get("tokens", []))
    except Exception:
        return 0

def post_chat_completion(payload):
    req = urllib.request.Request(
        SERVER_BASE + "/v1/chat/completions",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=TIMEOUT_SEC) as res:
        return json.loads(res.read())

def get_applicable_formats(dataset_name):
    if dataset_name in FLAT_DATASETS:
        return ALL_FORMATS
    else:
        # TSV and key_value skipped per Decision 1
        return [f for f in ALL_FORMATS if f not in ("tsv", "key_value")]

def update_sha256sums():
    lines = []
    if os.path.exists(RAW_DIR):
        for fname in sorted(os.listdir(RAW_DIR)):
            fpath = os.path.join(RAW_DIR, fname)
            if os.path.isfile(fpath):
                with open(fpath, "rb") as f:
                    digest = hashlib.sha256(f.read()).hexdigest()
                lines.append(f"{digest}  raw/{fname}\n")
    if os.path.exists(METADATA_FILE):
        with open(METADATA_FILE, "rb") as f:
            digest = hashlib.sha256(f.read()).hexdigest()
        lines.append(f"{digest}  metadata.json\n")
    if os.path.exists(SERVER_PROPS_FILE):
        with open(SERVER_PROPS_FILE, "rb") as f:
            digest = hashlib.sha256(f.read()).hexdigest()
        lines.append(f"{digest}  server_props.json\n")
    if os.path.exists(LAUNCH_LINE_FILE):
        with open(LAUNCH_LINE_FILE, "rb") as f:
            digest = hashlib.sha256(f.read()).hexdigest()
        lines.append(f"{digest}  launch_line.txt\n")
    with open(SHA256_FILE, "wb") as f:
        f.write("".join(lines).encode("utf-8"))

def run_collection(dry_run=False):
    """
    Executes the collection workflow.
    If dry_run is True, verifies all prompt constructions and planning without sending HTTP requests.
    """
    print("=== Struct Milestone R4 Collector ===")
    print(f"Server directory: {SERVER_DIR}")
    print(f"Target launch line:\n  {LAUNCH_LINE}\n")

    if not check_disk_space():
        sys.exit(1)

    os.makedirs(RAW_DIR, exist_ok=True)
    os.makedirs(BACKUP_DIR, exist_ok=True)

    all_questions = load_questions()
    datasets = {d: load_dataset(d) for d in DATASETS}

    # Pre-encode all dataset-format pairs
    encoded_cache = {}
    total_runs = 0
    plan = []

    for seed in SEEDS:
        for ds in DATASETS:
            fmts = get_applicable_formats(ds)
            for fmt in fmts:
                total_runs += 1
                plan.append({
                    "seed": seed,
                    "dataset": ds,
                    "format": fmt,
                    "file_stem": f"{PROFILE['stem']}_r4_{fmt}_{ds}_seed{seed}",
                })

    print(f"Planned collection: {len(SEEDS)} seeds x 24 combinations = {total_runs} total requests")

    metadata = []
    done = set()
    if os.path.exists(METADATA_FILE):
        try:
            with open(METADATA_FILE, "r", encoding="utf-8") as f:
                metadata = json.load(f)
            # Per Decision 13: failed requests are not marked done, a resume reruns them
            done = {
                (m["format"], m["dataset"], m["seed"])
                for m in metadata
                if m.get("success", False)
                and not m.get("error")
                and not str(m.get("finish_reason", "")).startswith("error")
                and m.get("output_tokens", 0) > 0
            }
            print(f"Resuming: {len(done)} successful runs already recorded in metadata.json")
        except Exception as e:
            print(f"Could not read metadata.json: {e}")

    if dry_run:
        print("\n[DRY RUN MODE: Testing prompt generation and encoding for all combinations]")
        for item in plan:
            fmt = item["format"]
            ds = item["dataset"]
            if (fmt, ds) not in encoded_cache:
                data = datasets[ds]
                encoded_cache[(fmt, ds)] = ENCODERS[fmt](data, ds)
            prompt = build_prompt(fmt, ds, encoded_cache[(fmt, ds)], all_questions[ds])
            assert len(prompt) > 0
        print(f"Dry run successful! All {len(plan)} prompts generated losslessly without calling model.")
        return

    # Real collection execution
    if not check_nvidia_smi():
        print("Aborting: GPU check failed.")
        sys.exit(1)

    # Server health check and props capture
    server_props = {}
    recorded_launch_line = LAUNCH_LINE
    if os.path.exists(LAUNCH_LINE_FILE):
        try:
            with open(LAUNCH_LINE_FILE, "r", encoding="utf-8") as f:
                ll = f.read().strip()
                if ll:
                    recorded_launch_line = ll
        except Exception:
            pass
    try:
        with urllib.request.urlopen(SERVER_BASE + "/health", timeout=10) as r:
            health = json.loads(r.read())
            if health.get("status") != "ok":
                print(f"Server not healthy: {health}")
                sys.exit(1)
            print(f"Server health: {health}")
        try:
            server_props = fetch_server_props(SERVER_BASE)
            extracted = get_launch_flags_from_props(server_props)
            if extracted and not os.path.exists(LAUNCH_LINE_FILE):
                recorded_launch_line = extracted
            with open(SERVER_PROPS_FILE, "w", encoding="utf-8") as f:
                json.dump({"launch_flags": recorded_launch_line, "props": server_props}, f, indent=2)
            print(f"Server launch flags recorded:\n  {recorded_launch_line}")
        except Exception as pe:
            print(f"Note: Could not query /props: {pe}")
    except Exception as e:
        print(f"ERROR: Could not connect to llama-server at {SERVER_BASE}: {e}")
        print("Per brief, start the turboquant llama-server before running real collection.")
        sys.exit(1)

    meta_lock = threading.Lock()
    for item in plan:
        key = (item["format"], item["dataset"])
        if key not in encoded_cache:
            encoded_cache[key] = ENCODERS[item["format"]](datasets[item["dataset"]], item["dataset"])

    def run_one(idx, item):
        fmt = item["format"]
        ds = item["dataset"]
        seed = item["seed"]
        stem = item["file_stem"]

        if (fmt, ds, seed) in done:
            return

        print(f"[{idx}/{total_runs}] Running {stem}...", flush=True)

        if (fmt, ds) not in encoded_cache:
            data = datasets[ds]
            encoded_cache[(fmt, ds)] = ENCODERS[fmt](data, ds)

        prompt = build_prompt(fmt, ds, encoded_cache[(fmt, ds)], all_questions[ds])
        input_tokens = tokenize_text(prompt)

        payload = {
            "messages": [{"role": "user", "content": prompt}],
            "seed": seed,
            "stream": False,
            "chat_template_kwargs": {"enable_thinking": True},
        }
        payload.update(SAMPLING)

        t0 = time.time()
        resp = None
        finish_reason = "unknown"
        content = ""
        reasoning = ""
        thinking_tokens = 0
        output_tokens = 0
        is_success = False
        error_msg = None

        try:
            resp = post_chat_completion(payload)
            dt = time.time() - t0
            choice = resp.get("choices", [{}])[0]
            finish_reason = choice.get("finish_reason", "unknown")
            msg = choice.get("message", {})
            content = msg.get("content", "") or ""
            reasoning = msg.get("reasoning_content", "") or ""
            thinking_tokens = tokenize_text(reasoning) if reasoning else 0
            output_tokens = tokenize_text(content) if content else 0
            if content.strip():
                is_success = True
            else:
                finish_reason = "error: no answer text"
                error_msg = "No answer text in model response"
        except Exception as e:
            dt = time.time() - t0
            finish_reason = f"error: {e}"
            error_msg = str(e)
            print(f"  Error on {stem}: {e}")
            resp = {"error": str(e)}

        raw_path = os.path.join(RAW_DIR, f"{stem}.txt")

        with open(raw_path, "w", encoding="utf-8") as f:
            f.write(content)

        meta_entry = {
            "run_index": idx,
            "format": fmt,
            "dataset": ds,
            "seed": seed,
            "file": f"{stem}.txt",
            "success": is_success,
            "finish_reason": finish_reason,
            "elapsed_sec": dt,
            "input_tokens": input_tokens,
            "thinking_tokens": thinking_tokens,
            "output_tokens": output_tokens,
            "server_launch_flags": recorded_launch_line,
            "raw_response": resp,
        }
        if error_msg:
            meta_entry["error"] = error_msg

        # Per Decision 13 / review: keep failure history when a rerun succeeds
        with meta_lock:
            metadata.append(meta_entry)
            metadata.sort(key=lambda m: m.get("run_index", 0))

            # Atomic write of metadata
            tmp_meta = METADATA_FILE + ".tmp"
            with open(tmp_meta, "w", encoding="utf-8") as f:
                json.dump(metadata, f, indent=2)
            os.replace(tmp_meta, METADATA_FILE)
            update_sha256sums()
        print(f"  done {stem} ({dt:.0f}s, {thinking_tokens} thinking tokens)", flush=True)

    with ThreadPoolExecutor(max_workers=PARALLEL) as pool:
        futures = [pool.submit(run_one, idx, item) for idx, item in enumerate(plan, start=1)]
        for fut in futures:
            fut.result()

    print("\nCollection finished!")

if __name__ == "__main__":
    is_dry = "--dry-run" in sys.argv
    run_collection(dry_run=is_dry)
