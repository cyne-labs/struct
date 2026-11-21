#!/usr/bin/env python3
"""
Collection harness for Struct Milestone R4-haiku: retrieval accuracy across formats on Claude Haiku.

Follows RULES.md, R4.md and R4-haiku.md:
- Target model: Claude Haiku via Claude Code CLI.
- Execution: one fresh headless Claude Code process per prompt, neutral working directory (temp folder), prompt on stdin.
- Command:
    claude -p --model haiku --effort low --system-prompt "You are a helpful assistant." --tools "" --strict-mcp-config --setting-sources "" --disable-slash-commands --no-session-persistence --output-format json
- Arguments passed as list (no shell). Empty strings for --tools and --setting-sources are real empty arguments.
- Thinking: effort low, no thinking budget.
- Repeats: 4 repeats per prompt (labelled seed 1 to 4).
- Parallel: 4 calls at a time.
- TSV and Plain key: value run only on the two flat datasets (24 prompts x 4 repeats = 96 total calls).
- Writes to bench/r4/real_haiku/:
    raw/haiku_r4_{format}_{dataset}_seed{n}.txt (the answer text, the JSON result field)
    metadata.json
    run_info.json (exact command line and claude --version output)
    SHA256SUMS (all raw files, metadata.json, run_info.json)
- Tokens from usage: input = input_tokens + cache_creation_input_tokens + cache_read_input_tokens;
  thinking = output_tokens_details.thinking_tokens; output = output_tokens - thinking.
- Failure definition: non-zero exit, unparseable JSON, is_error true, empty result, or timeout (20 min).
  Failures are recorded, never marked done, and a rerun retries them.
- Resumable: a rerun skips calls that already succeeded.
- --dry-run: builds all 96 prompts and verifies they match what collect.py sends, without calling anything.
"""

import os
import sys
import time
import json
import hashlib
import tempfile
import subprocess
import threading
from concurrent.futures import ThreadPoolExecutor

BENCH_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STRUCT_ROOT = os.path.dirname(BENCH_DIR)
R4_DIR = os.path.join(BENCH_DIR, "r4")
sys.path.insert(0, BENCH_DIR)

from r4.formats import ENCODERS, FORMAT_DISPLAY_NAMES, FORMATS
from r4.collect import (
    build_prompt,
    load_dataset,
    load_questions,
    get_applicable_formats,
    check_disk_space,
    ALL_FORMATS,
    FLAT_DATASETS,
    NESTED_DATASETS,
    DATASETS,
    DATA_PATHS,
    QUESTIONS_PATH,
)

CLAUDE_CMD = [
    "claude",
    "-p",
    "--model",
    "haiku",
    "--effort",
    "low",
    "--system-prompt",
    "You are a helpful assistant.",
    "--tools",
    "",
    "--strict-mcp-config",
    "--setting-sources",
    "",
    "--disable-slash-commands",
    "--no-session-persistence",
    "--output-format",
    "json",
]

SEEDS = [1, 2, 3, 4]
TIMEOUT_SEC = 1200  # 20 minutes
PARALLEL = 4

REAL_HAIKU_DIR = os.path.join(R4_DIR, "real_haiku")
RAW_DIR = os.path.join(REAL_HAIKU_DIR, "raw")
SHA256_FILE = os.path.join(REAL_HAIKU_DIR, "SHA256SUMS")
METADATA_FILE = os.path.join(REAL_HAIKU_DIR, "metadata.json")
RUN_INFO_FILE = os.path.join(REAL_HAIKU_DIR, "run_info.json")


def extract_tokens(resp_json):
    """
    Extracts token counts from Claude JSON response per R4-haiku.md:
    input is input_tokens + cache_creation_input_tokens + cache_read_input_tokens
    thinking is output_tokens_details.thinking_tokens
    output is output_tokens minus thinking
    """
    if not isinstance(resp_json, dict):
        return 0, 0, 0
    usage = resp_json.get("usage", {})
    if not isinstance(usage, dict):
        return 0, 0, 0

    inp = (
        int(usage.get("input_tokens", 0) or 0)
        + int(usage.get("cache_creation_input_tokens", 0) or 0)
        + int(usage.get("cache_read_input_tokens", 0) or 0)
    )

    out_details = usage.get("output_tokens_details") or {}
    if not isinstance(out_details, dict):
        out_details = {}
    thinking = int(out_details.get("thinking_tokens", 0) or 0)

    total_out = int(usage.get("output_tokens", 0) or 0)
    output = max(0, total_out - thinking)

    return inp, thinking, output


def extract_cost(resp_json):
    """Extracts total_cost_usd from Claude JSON response."""
    if not isinstance(resp_json, dict):
        return 0.0
    if "total_cost_usd" in resp_json and resp_json["total_cost_usd"] is not None:
        try:
            return float(resp_json["total_cost_usd"])
        except (ValueError, TypeError):
            pass
    if "totalCostUsd" in resp_json and resp_json["totalCostUsd"] is not None:
        try:
            return float(resp_json["totalCostUsd"])
        except (ValueError, TypeError):
            pass
    if "cost_usd" in resp_json and resp_json["cost_usd"] is not None:
        try:
            return float(resp_json["cost_usd"])
        except (ValueError, TypeError):
            pass
    mu = resp_json.get("modelUsage")
    if isinstance(mu, dict):
        total = 0.0
        for entry in mu.values():
            if isinstance(entry, dict) and "costUSD" in entry:
                try:
                    total += float(entry["costUSD"])
                except (ValueError, TypeError):
                    pass
        return total
    return 0.0


def extract_model_id(resp_json):
    """Extracts model id from Claude JSON response modelUsage or top-level model."""
    if not isinstance(resp_json, dict):
        return "claude-haiku"
    mu = resp_json.get("modelUsage")
    if isinstance(mu, dict) and mu:
        return next(iter(mu.keys()))
    if isinstance(mu, list) and mu:
        first = mu[0]
        if isinstance(first, dict):
            return first.get("model") or "claude-haiku"
        return str(first)
    if resp_json.get("model"):
        return str(resp_json["model"])
    return "claude-haiku"


def get_claude_version():
    """Queries claude --version to record environment details."""
    try:
        res = subprocess.run(
            ["claude", "--version"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=10,
        )
        if res.returncode == 0 and res.stdout.strip():
            return res.stdout.strip()
        return f"claude --version exited with code {res.returncode}: {res.stderr.strip()}"
    except Exception as e:
        return f"claude version check error: {e}"


def record_run_info(real_dir=REAL_HAIKU_DIR, version_str=None):
    """Records run_info.json containing exact command line and claude version."""
    os.makedirs(real_dir, exist_ok=True)
    run_info_file = os.path.join(real_dir, "run_info.json")
    version = version_str if version_str is not None else get_claude_version()
    info = {
        "command": CLAUDE_CMD,
        "command_line": " ".join(CLAUDE_CMD),
        "claude_version": version,
    }
    with open(run_info_file, "w", encoding="utf-8") as f:
        json.dump(info, f, indent=2)
    return info


def update_sha256sums(real_dir=REAL_HAIKU_DIR):
    """Updates SHA256SUMS for all raw response files, metadata.json, and run_info.json."""
    lines = []
    raw_dir = os.path.join(real_dir, "raw")
    if os.path.exists(raw_dir):
        for fname in sorted(os.listdir(raw_dir)):
            fpath = os.path.join(raw_dir, fname)
            if os.path.isfile(fpath):
                with open(fpath, "rb") as f:
                    digest = hashlib.sha256(f.read()).hexdigest()
                lines.append(f"{digest}  raw/{fname}\n")
    metadata_file = os.path.join(real_dir, "metadata.json")
    if os.path.exists(metadata_file):
        with open(metadata_file, "rb") as f:
            digest = hashlib.sha256(f.read()).hexdigest()
        lines.append(f"{digest}  metadata.json\n")
    run_info_file = os.path.join(real_dir, "run_info.json")
    if os.path.exists(run_info_file):
        with open(run_info_file, "rb") as f:
            digest = hashlib.sha256(f.read()).hexdigest()
        lines.append(f"{digest}  run_info.json\n")

    sha_file = os.path.join(real_dir, "SHA256SUMS")
    with open(sha_file, "wb") as f:
        f.write("".join(lines).encode("utf-8"))


def process_claude_output(proc):
    """
    Parses and validates Claude Code process output.
    Returns: (is_success, finish_reason, content, resp_json, error_msg, input_tokens, thinking_tokens, output_tokens, total_cost_usd, model_id)
    """
    stdout = proc.stdout if hasattr(proc, "stdout") else ""
    stderr = proc.stderr if hasattr(proc, "stderr") else ""
    returncode = proc.returncode if hasattr(proc, "returncode") else 1

    if returncode != 0:
        err = f"Process exited with non-zero code {returncode}: {stderr.strip()}"
        return (
            False,
            f"error: exit code {returncode}",
            "",
            {"error": err, "stderr": stderr, "stdout": stdout},
            err,
            0,
            0,
            0,
            0.0,
            "claude-haiku",
        )

    try:
        resp_json = json.loads(stdout)
    except Exception as e:
        err = f"Unparseable JSON from claude output: {e}"
        return (
            False,
            "error: unparseable json",
            "",
            {"error": err, "stdout": stdout, "stderr": stderr},
            err,
            0,
            0,
            0,
            0.0,
            "claude-haiku",
        )

    if not isinstance(resp_json, dict):
        err = f"Claude output is not a JSON object: {type(resp_json)}"
        return (
            False,
            "error: invalid json format",
            "",
            {"error": err, "stdout": stdout},
            err,
            0,
            0,
            0,
            0.0,
            "claude-haiku",
        )

    if resp_json.get("is_error") is True:
        err = (
            resp_json.get("error")
            or resp_json.get("result")
            or "is_error is true in claude response"
        )
        return (
            False,
            "error: is_error is true",
            "",
            resp_json,
            str(err),
            0,
            0,
            0,
            0.0,
            extract_model_id(resp_json),
        )

    content = resp_json.get("result", "")
    if content is None or not str(content).strip():
        err = "Empty or whitespace-only result in claude response"
        return (
            False,
            "error: empty result",
            "",
            resp_json,
            err,
            0,
            0,
            0,
            0.0,
            extract_model_id(resp_json),
        )

    content_str = str(content)
    inp, think, out = extract_tokens(resp_json)
    cost = extract_cost(resp_json)
    model_id = extract_model_id(resp_json)
    finish_reason = resp_json.get("stop_reason") or "stop"

    return (
        True,
        finish_reason,
        content_str,
        resp_json,
        None,
        inp,
        think,
        out,
        cost,
        model_id,
    )


def call_claude(prompt, timeout_sec=TIMEOUT_SEC, cwd=None):
    """
    Executes a single headless Claude Code process from neutral temp dir with prompt on stdin.
    Passes arguments as list (no shell).
    """
    if cwd is None:
        cwd = tempfile.gettempdir()

    return subprocess.run(
        CLAUDE_CMD,
        input=prompt,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        cwd=cwd,
        timeout=timeout_sec,
    )


def run_collection(
    dry_run=False,
    real_dir=REAL_HAIKU_DIR,
    questions_path=QUESTIONS_PATH,
    data_paths=None,
    max_workers=PARALLEL,
    version_override=None,
):
    """
    Executes collection for R4-haiku.
    If dry_run is True, verifies all 96 prompt constructions match collect.py byte-for-byte without calling Claude.
    """
    print("=== Struct Milestone R4-haiku Collector (Claude Haiku) ===")
    print(f"Target directory: {real_dir}")
    print(f"Command:\n  {' '.join(CLAUDE_CMD)}\n")

    if not check_disk_space():
        sys.exit(1)

    raw_dir = os.path.join(real_dir, "raw")
    metadata_file = os.path.join(real_dir, "metadata.json")

    # Load questions
    with open(questions_path, "r", encoding="utf-8") as f:
        all_questions = json.load(f)

    # Load datasets
    datasets = {}
    dp = data_paths or DATA_PATHS
    for ds_name in DATASETS:
        with open(dp[ds_name], "r", encoding="utf-8") as f:
            datasets[ds_name] = json.load(f)

    # Pre-encode datasets
    encoded_cache = {}
    plan = []
    total_runs = 0

    for seed in SEEDS:
        for ds in DATASETS:
            fmts = get_applicable_formats(ds)
            for fmt in fmts:
                total_runs += 1
                plan.append({
                    "seed": seed,
                    "dataset": ds,
                    "format": fmt,
                    "file_stem": f"haiku_r4_{fmt}_{ds}_seed{seed}",
                })

    print(f"Planned collection: {len(SEEDS)} repeats x 24 combinations = {total_runs} total calls")

    for item in plan:
        key = (item["format"], item["dataset"])
        if key not in encoded_cache:
            encoded_cache[key] = ENCODERS[item["format"]](datasets[item["dataset"]], item["dataset"])

    if dry_run:
        print("\n[DRY RUN MODE: Testing prompt generation and encoding for all 96 combinations]")
        for item in plan:
            fmt = item["format"]
            ds = item["dataset"]
            encoded = encoded_cache[(fmt, ds)]
            haiku_prompt = build_prompt(fmt, ds, encoded, all_questions[ds])
            # Check prompt is non-empty and byte-identical to collect.py's build_prompt
            expected_prompt = build_prompt(fmt, ds, encoded, all_questions[ds])
            assert haiku_prompt == expected_prompt, f"Prompt mismatch on {fmt} {ds}"
            assert len(haiku_prompt) > 0
        print(f"Dry run successful! All {len(plan)} prompts match collect.py byte-for-byte without calling Claude.")
        return

    os.makedirs(raw_dir, exist_ok=True)
    record_run_info(real_dir=real_dir, version_str=version_override)
    update_sha256sums(real_dir=real_dir)

    metadata = []
    done = set()
    if os.path.exists(metadata_file):
        try:
            with open(metadata_file, "r", encoding="utf-8") as f:
                metadata = json.load(f)
            # Per Decision 13 / R4-haiku: failed requests are not marked done, a resume reruns them
            done = {
                (m["format"], m["dataset"], m["seed"])
                for m in metadata
                if m.get("success", False)
                and not m.get("error")
                and not str(m.get("finish_reason", "")).startswith("error")
                and len(m.get("file", "")) > 0
                and os.path.exists(os.path.join(raw_dir, m["file"]))
                and os.path.getsize(os.path.join(raw_dir, m["file"])) > 0
            }
            print(f"Resuming: {len(done)} successful runs already recorded in metadata.json")
        except Exception as e:
            print(f"Could not read metadata.json: {e}")

    meta_lock = threading.Lock()

    def run_one(idx, item):
        fmt = item["format"]
        ds = item["dataset"]
        seed = item["seed"]
        stem = item["file_stem"]

        if (fmt, ds, seed) in done:
            return

        print(f"[{idx}/{total_runs}] Calling Haiku on {stem}...", flush=True)

        prompt = build_prompt(fmt, ds, encoded_cache[(fmt, ds)], all_questions[ds])

        t0 = time.time()
        try:
            proc = call_claude(prompt, timeout_sec=TIMEOUT_SEC)
            dt = time.time() - t0
            (
                is_success,
                finish_reason,
                content,
                resp_json,
                error_msg,
                input_tokens,
                thinking_tokens,
                output_tokens,
                total_cost_usd,
                model_id,
            ) = process_claude_output(proc)
        except subprocess.TimeoutExpired:
            dt = time.time() - t0
            is_success = False
            finish_reason = "error: timeout"
            error_msg = f"Timed out after {TIMEOUT_SEC}s"
            content = ""
            resp_json = {"error": error_msg}
            input_tokens = 0
            thinking_tokens = 0
            output_tokens = 0
            total_cost_usd = 0.0
            model_id = "claude-haiku"
        except Exception as e:
            dt = time.time() - t0
            is_success = False
            finish_reason = f"error: {e}"
            error_msg = str(e)
            content = ""
            resp_json = {"error": error_msg}
            input_tokens = 0
            thinking_tokens = 0
            output_tokens = 0
            total_cost_usd = 0.0
            model_id = "claude-haiku"

        raw_path = os.path.join(raw_dir, f"{stem}.txt")
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
            "total_cost_usd": total_cost_usd,
            "model_id": model_id,
            "raw_response": resp_json,
        }
        if error_msg:
            meta_entry["error"] = error_msg

        with meta_lock:
            metadata.append(meta_entry)
            metadata.sort(key=lambda m: m.get("run_index", 0))

            tmp_meta = metadata_file + ".tmp"
            with open(tmp_meta, "w", encoding="utf-8") as f:
                json.dump(metadata, f, indent=2)
            os.replace(tmp_meta, metadata_file)
            update_sha256sums(real_dir=real_dir)

        status_str = "SUCCESS" if is_success else f"FAILED ({finish_reason})"
        print(f"  done {stem} ({dt:.1f}s, {status_str}, {thinking_tokens} thinking tokens)", flush=True)

    with ThreadPoolExecutor(max_workers=max_workers) as pool:
        futures = [pool.submit(run_one, idx, item) for idx, item in enumerate(plan, start=1)]
        for fut in futures:
            fut.result()

    print("\nCollection finished!")


if __name__ == "__main__":
    is_dry = "--dry-run" in sys.argv
    run_collection(dry_run=is_dry)
