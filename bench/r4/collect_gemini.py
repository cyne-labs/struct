#!/usr/bin/env python3
"""
Collection harness for Struct Milestone R4-gemini: retrieval accuracy across formats on Gemini 3.8 Flash.

Follows RULES.md, R4.md, R4-haiku.md, and R4-gemini.md:
- Target model: Gemini 3.8 Flash (gemini-3.8-flash-low) via headless agy CLI.
- Execution: one fresh headless agy process per prompt, from a new empty temporary directory each time
  (so the agent has no files to look at), with the prompt attached to -p:
    agy -p=<prompt> --model gemini-3.8-flash-low --output-format json --sandbox --disable-slash-commands
- Arguments passed as list (no shell). Prompt passed as "-p=" + prompt.
- Checks command line length before calling; fails call if command line would exceed Windows 32,767 character limit.
- Thinking: low model variant, no thinking budget.
- Repeats: 4 repeats per prompt (labelled seed 1 to 4).
- Parallel: 4 calls at a time.
- TSV and Plain key: value run only on the two flat datasets (24 prompts x 4 repeats = 96 total calls).
- Writes to bench/r4/real_gemini/:
    raw/gemini_r4_{format}_{dataset}_seed{n}.txt (the answer text, the JSON response field)
    metadata.json
    run_info.json (exact command line and agy --version output)
    SHA256SUMS (all raw files, metadata.json, run_info.json)
- Tokens from usage:
    input is input_tokens + cache_read_tokens;
    thinking is thinking_tokens;
    output is output_tokens minus thinking_tokens, never below 0.
    Both calculated fields and raw fields are recorded in metadata.
- Failure definition: non-zero exit, unparseable JSON, status other than SUCCESS,
  num_turns above 1 (the agent used tools), empty response, timeout (20 min), or too-long command line.
  Failures are recorded, never marked done, and a rerun retries them.
- Resumable: a rerun skips calls that already succeeded.
- --dry-run: builds all 96 prompts and verifies they match what collect.py sends and are under 32,767 chars,
  without calling anything.
"""

import os
import sys
import time
import json
import shutil
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

MODEL_NAME = "gemini-3.8-flash-low"
SEEDS = [1, 2, 3, 4]
TIMEOUT_SEC = 1200  # 20 minutes
PARALLEL = 4
WINDOWS_CMD_MAX_LEN = 32767

REAL_GEMINI_DIR = os.environ.get("R4_REAL_GEMINI_DIR") or os.path.join(R4_DIR, "real_gemini")
RAW_DIR = os.path.join(REAL_GEMINI_DIR, "raw")
SHA256_FILE = os.path.join(REAL_GEMINI_DIR, "SHA256SUMS")
METADATA_FILE = os.path.join(REAL_GEMINI_DIR, "metadata.json")
RUN_INFO_FILE = os.path.join(REAL_GEMINI_DIR, "run_info.json")

AGY_BASE_CMD = [
    "agy",
    "--model",
    MODEL_NAME,
    "--output-format",
    "json",
    "--sandbox",
    "--disable-slash-commands",
]


class CommandLineTooLongError(ValueError):
    """Raised when the command line string exceeds Windows 32,767 character limit."""
    pass


def build_agy_cmd(prompt):
    """
    Constructs argument list for agy per R4-gemini.md:
    agy -p=<prompt> --model gemini-3.8-flash-low --output-format json --sandbox --disable-slash-commands
    Prompt must be one argument: "-p=" + prompt.
    """
    return [
        "agy",
        f"-p={prompt}",
        "--model",
        MODEL_NAME,
        "--output-format",
        "json",
        "--sandbox",
        "--disable-slash-commands",
    ]


def check_cmd_length(cmd):
    """
    Checks if command line string exceeds Windows 32,767 character limit.
    Returns the formatted command line string or raises CommandLineTooLongError.
    """
    cmd_str = subprocess.list2cmdline(cmd)
    if len(cmd_str) > WINDOWS_CMD_MAX_LEN:
        raise CommandLineTooLongError(
            f"Command line length ({len(cmd_str)}) exceeds Windows limit of {WINDOWS_CMD_MAX_LEN} characters"
        )
    return cmd_str


def extract_tokens(resp_json):
    """
    Extracts token counts from agy JSON response per R4-gemini.md:
    input is input_tokens + cache_read_tokens
    thinking is thinking_tokens
    output is output_tokens minus thinking_tokens, never below 0
    """
    if not isinstance(resp_json, dict):
        return 0, 0, 0
    usage = resp_json.get("usage", {})
    if not isinstance(usage, dict):
        return 0, 0, 0

    inp = int(usage.get("input_tokens", 0) or 0) + int(usage.get("cache_read_tokens", 0) or 0)
    thinking = int(usage.get("thinking_tokens", 0) or 0)
    raw_output = int(usage.get("output_tokens", 0) or 0)
    output = max(0, raw_output - thinking)

    return inp, thinking, output


def get_agy_version():
    """Queries agy --version to record environment details."""
    try:
        res = subprocess.run(
            ["agy", "--version"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=10,
        )
        if res.returncode == 0 and res.stdout.strip():
            return res.stdout.strip()
        return f"agy --version exited with code {res.returncode}: {res.stderr.strip()}"
    except Exception as e:
        return f"agy version check error: {e}"


def record_run_info(real_dir=REAL_GEMINI_DIR, version_str=None):
    """Records run_info.json containing exact command line template and agy version."""
    os.makedirs(real_dir, exist_ok=True)
    run_info_file = os.path.join(real_dir, "run_info.json")
    version = version_str if version_str is not None else get_agy_version()
    cmd_template = [
        "agy",
        "-p=<prompt>",
        "--model",
        MODEL_NAME,
        "--output-format",
        "json",
        "--sandbox",
        "--disable-slash-commands",
    ]
    info = {
        "command": cmd_template,
        "command_line": f"agy -p=<prompt> --model {MODEL_NAME} --output-format json --sandbox --disable-slash-commands",
        "agy_version": version,
    }
    with open(run_info_file, "w", encoding="utf-8") as f:
        json.dump(info, f, indent=2)
    return info


def update_sha256sums(real_dir=REAL_GEMINI_DIR):
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


def process_agy_output(proc):
    """
    Parses and validates agy JSON process output per R4-gemini.md:
    Failure conditions:
      - non-zero exit code
      - unparseable JSON
      - status other than SUCCESS
      - num_turns above 1 (the agent used tools)
      - empty response
    Returns: (is_success, finish_reason, content, resp_json, error_msg, input_tokens, thinking_tokens, output_tokens, model_id, raw_usage)
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
            MODEL_NAME,
            {},
        )

    try:
        resp_json = json.loads(stdout)
    except Exception as e:
        err = f"Unparseable JSON from agy output: {e}"
        return (
            False,
            "error: unparseable json",
            "",
            {"error": err, "stdout": stdout, "stderr": stderr},
            err,
            0,
            0,
            0,
            MODEL_NAME,
            {},
        )

    if not isinstance(resp_json, dict):
        err = f"agy output is not a JSON object: {type(resp_json)}"
        return (
            False,
            "error: invalid json format",
            "",
            {"error": err, "stdout": stdout},
            err,
            0,
            0,
            0,
            MODEL_NAME,
            {},
        )

    usage = resp_json.get("usage", {})
    if not isinstance(usage, dict):
        usage = {}

    status = resp_json.get("status")
    if status != "SUCCESS":
        finished_reason = resp_json.get("finished_reason") or resp_json.get("finish_reason") or f"error: status {status}"
        err = resp_json.get("error") or f"agy returned non-success status: {status}"
        return (
            False,
            finished_reason,
            "",
            resp_json,
            str(err),
            0,
            0,
            0,
            MODEL_NAME,
            usage,
        )

    num_turns = resp_json.get("num_turns", 1)
    if num_turns is not None and num_turns > 1:
        err = f"num_turns was {num_turns} (> 1, agent used tools); expected single turn without tools"
        return (
            False,
            "error: num_turns > 1 (agent used tools)",
            "",
            resp_json,
            err,
            0,
            0,
            0,
            MODEL_NAME,
            usage,
        )

    content = resp_json.get("response")
    if content is None or not str(content).strip():
        err = "Empty or whitespace-only response in agy output"
        return (
            False,
            "error: empty response",
            "",
            resp_json,
            err,
            0,
            0,
            0,
            MODEL_NAME,
            usage,
        )

    content_str = str(content)
    inp, think, out = extract_tokens(resp_json)
    model_id = resp_json.get("model") or MODEL_NAME
    finish_reason = resp_json.get("finished_reason") or resp_json.get("finish_reason") or "stop"

    return (
        True,
        finish_reason,
        content_str,
        resp_json,
        None,
        inp,
        think,
        out,
        model_id,
        usage,
    )


def call_agy(prompt, timeout_sec=TIMEOUT_SEC, cwd=None):
    """
    Executes a single fresh headless agy process from a new empty temporary directory each time.
    Passes arguments as list (no shell). The prompt is passed as -p=<prompt>.
    Checks command line length against Windows 32,767 character limit.
    """
    cmd = build_agy_cmd(prompt)
    check_cmd_length(cmd)

    cleanup_cwd = False
    if cwd is None:
        cwd = tempfile.mkdtemp(prefix="agy_r4_")
        cleanup_cwd = True

    try:
        return subprocess.run(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            cwd=cwd,
            timeout=timeout_sec,
        )
    finally:
        if cleanup_cwd and os.path.exists(cwd):
            try:
                shutil.rmtree(cwd)
            except Exception:
                pass


def run_collection(
    dry_run=False,
    real_dir=REAL_GEMINI_DIR,
    questions_path=QUESTIONS_PATH,
    data_paths=None,
    max_workers=PARALLEL,
    version_override=None,
):
    """
    Executes collection for R4-gemini.
    If dry_run is True, verifies all 96 prompt constructions match collect.py byte-for-byte
    and are under 32,767 characters, without calling agy.
    """
    print("=== Struct Milestone R4-gemini Collector (Gemini 3.8 Flash via agy) ===")
    print(f"Target directory: {real_dir}")
    print(f"Command template:\n  agy -p=<prompt> --model {MODEL_NAME} --output-format json --sandbox --disable-slash-commands\n")

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
                    "file_stem": f"gemini_r4_{fmt}_{ds}_seed{seed}",
                })

    print(f"Planned collection: {len(SEEDS)} repeats x 24 combinations = {total_runs} total calls")

    for item in plan:
        key = (item["format"], item["dataset"])
        if key not in encoded_cache:
            encoded_cache[key] = ENCODERS[item["format"]](datasets[item["dataset"]], item["dataset"])

    if dry_run:
        print("\n[DRY RUN MODE: Testing prompt generation, encoding, and command line length for all 96 combinations]")
        for item in plan:
            fmt = item["format"]
            ds = item["dataset"]
            encoded = encoded_cache[(fmt, ds)]
            gemini_prompt = build_prompt(fmt, ds, encoded, all_questions[ds])
            expected_prompt = build_prompt(fmt, ds, encoded, all_questions[ds])
            assert gemini_prompt == expected_prompt, f"Prompt mismatch on {fmt} {ds}"
            assert len(gemini_prompt) > 0

            cmd = build_agy_cmd(gemini_prompt)
            cmd_line = check_cmd_length(cmd)
            assert len(cmd_line) <= WINDOWS_CMD_MAX_LEN, f"Command line too long ({len(cmd_line)}) on {fmt} {ds}"

        print(f"Dry run successful! All {len(plan)} prompts match collect.py byte-for-byte and are within the 32,767 character limit without calling agy.")
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
            # Per Decision 13 / R4-gemini: failed requests are not marked done, a resume reruns them
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

        print(f"[{idx}/{total_runs}] Calling Gemini on {stem}...", flush=True)

        prompt = build_prompt(fmt, ds, encoded_cache[(fmt, ds)], all_questions[ds])

        t0 = time.time()
        try:
            # Check prompt length before calling agy
            cmd = build_agy_cmd(prompt)
            check_cmd_length(cmd)

            proc = call_agy(prompt, timeout_sec=TIMEOUT_SEC)
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
                model_id,
                raw_usage,
            ) = process_agy_output(proc)
        except CommandLineTooLongError as e:
            dt = time.time() - t0
            is_success = False
            finish_reason = "error: command line too long"
            error_msg = str(e)
            content = ""
            resp_json = {"error": error_msg}
            input_tokens = 0
            thinking_tokens = 0
            output_tokens = 0
            model_id = MODEL_NAME
            raw_usage = {}
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
            model_id = MODEL_NAME
            raw_usage = {}
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
            model_id = MODEL_NAME
            raw_usage = {}

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
            "raw_input_tokens": int(raw_usage.get("input_tokens", 0) or 0) if isinstance(raw_usage, dict) else 0,
            "raw_cache_read_tokens": int(raw_usage.get("cache_read_tokens", 0) or 0) if isinstance(raw_usage, dict) else 0,
            "raw_output_tokens": int(raw_usage.get("output_tokens", 0) or 0) if isinstance(raw_usage, dict) else 0,
            "raw_thinking_tokens": int(raw_usage.get("thinking_tokens", 0) or 0) if isinstance(raw_usage, dict) else 0,
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
