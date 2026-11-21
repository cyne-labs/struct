#!/usr/bin/env python3
"""
Collection harness for Struct Milestone R4-gemini-lite: retrieval accuracy across formats on Gemini 3.5 Flash Lite via Google Gemini API.

Follows RULES.md, R4.md, R4-haiku.md, and R4-gemini-lite.md:
- Target model: Gemini 3.5 Flash Lite via Gemini API.
- Call: POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent
  Standard library only (urllib).
  Body: single user turn, generationConfig.thinkingConfig.thinkingLevel = "low", includeThoughts left off.
  No system instruction, no tools, no other sampling settings.
- The model ID is a --model argument (no default invented).
- API key read only from GEMINI_API_KEY environment variable. If missing, stops with a clear message.
- Sent only in x-goog-api-key header, never in the URL.
- Never printed, logged or saved, including in error messages, metadata, run_info.json or raw replies.
  Scrubbed before recording.
- Repeats: 4 repeats, seeds 1 to 4 (plain reruns; 24 prompts x 4 = 96 total calls).
- Order: seed-by-seed. Run all 24 prompts of seed 1 before any call of seed 2, then seed 2, and so on.
  Up to 4 calls at a time, but only within the current seed.
- Rate limits and quota:
  HTTP 429 or 503 with per-minute limit: wait (Retry-After or retry delay in error body, or back off 10s, 20s, 40s, up to 5 tries).
  Daily or project quota exhausted (or 429s persist after backoff): stop the whole run cleanly, record which seed and call it stopped at, and exit. Resumable.
  Other failures: recorded, never marked done, retried on resume.
- Writes to bench/r4/real_gemini_lite/:
    raw/gemini_lite_r4_{format}_{dataset}_seed{n}.txt (answer text: text parts of first candidate, thought parts left out)
    metadata.json
    run_info.json (model ID, request body without prompt, thinking level, run order, start/stop times; never the key)
    SHA256SUMS (all raw files, metadata.json, run_info.json)
- Tokens from usageMetadata: input promptTokenCount, thinking thoughtsTokenCount (0 if absent), output candidatesTokenCount.
- Resumable: rerun skips calls that already succeeded.
- --dry-run: verifies all 96 prompts match collect.py byte-for-byte and prints planned order, without calling API.
"""

import os
import sys
import time
import json
import socket
import hashlib
import argparse
import threading
import urllib.request
import urllib.error
from datetime import datetime, timezone
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

SEEDS = [1, 2, 3, 4]
TIMEOUT_SEC = 1200  # 20 minutes
PARALLEL = 4
BACKOFF_DELAYS = (10.0, 20.0, 40.0, 80.0, 160.0)

REAL_GEMINI_LITE_DIR = os.environ.get("R4_REAL_GEMINI_LITE_DIR") or os.path.join(R4_DIR, "real_gemini_lite")
RAW_DIR = os.path.join(REAL_GEMINI_LITE_DIR, "raw")
SHA256_FILE = os.path.join(REAL_GEMINI_LITE_DIR, "SHA256SUMS")
METADATA_FILE = os.path.join(REAL_GEMINI_LITE_DIR, "metadata.json")
RUN_INFO_FILE = os.path.join(REAL_GEMINI_LITE_DIR, "run_info.json")


class QuotaExhaustedError(RuntimeError):
    """Raised when daily or project quota is exhausted, stopping the run cleanly."""
    pass


class RequestRateLimiter:
    """
    Client-side rate limiter shared across worker threads.
    Enforces at most `max_requests` started in any rolling `window_sec` seconds.
    Defaults to 14 requests started in any rolling 60 seconds (per R4-gemini-lite-fix.md).
    """
    def __init__(self, max_requests=14, window_sec=60.0, time_fn=time.time, sleep_fn=time.sleep):
        self.max_requests = max_requests
        self.window_sec = window_sec
        self.time_fn = time_fn
        self.sleep_fn = sleep_fn
        self.timestamps = []
        self.lock = threading.Lock()

    def acquire(self):
        """
        Blocks until a request can start without exceeding max_requests in any rolling window.
        Records and returns the timestamp when permission is granted.
        """
        while True:
            with self.lock:
                now = self.time_fn()
                # Prune timestamps outside rolling window
                cutoff = now - self.window_sec
                self.timestamps = [t for t in self.timestamps if t > cutoff]

                if len(self.timestamps) < self.max_requests:
                    self.timestamps.append(now)
                    return now

                # Oldest timestamp in window
                oldest = self.timestamps[0]
                wait_needed = (oldest + self.window_sec) - now

            if wait_needed > 0:
                self.sleep_fn(wait_needed)
            else:
                self.sleep_fn(0.001)


def get_api_key(required=True):
    """
    Reads API key exclusively from GEMINI_API_KEY environment variable.
    If missing and required, stops with a clear message.
    """
    key = os.environ.get("GEMINI_API_KEY", "").strip()
    if not key and required:
        print("ERROR: GEMINI_API_KEY environment variable is not set. Please set GEMINI_API_KEY before running collection.", file=sys.stderr)
        sys.exit(1)
    return key


def scrub_api_key(obj, api_key):
    """
    Recursively scrubs API key from strings, dicts, lists, and tuples.
    Guarantees that the key never appears in printed text, error logs, or saved JSON files.
    """
    if not api_key:
        return obj
    if isinstance(obj, str):
        return obj.replace(api_key, "[REDACTED_API_KEY]")
    elif isinstance(obj, bytes):
        return obj.replace(api_key.encode("utf-8"), b"[REDACTED_API_KEY]")
    elif isinstance(obj, Exception):
        return scrub_api_key(str(obj), api_key)
    elif isinstance(obj, dict):
        return {scrub_api_key(k, api_key): scrub_api_key(v, api_key) for k, v in obj.items()}
    elif isinstance(obj, list):
        return [scrub_api_key(item, api_key) for item in obj]
    elif isinstance(obj, tuple):
        return tuple(scrub_api_key(item, api_key) for item in obj)
    return obj


def get_api_url(model):
    """
    Builds Google Gemini API endpoint URL per Decision 1.
    URL contains only the model name, never any query parameters or API key.
    """
    model_clean = str(model).strip()
    if model_clean.startswith("models/"):
        model_clean = model_clean[len("models/"):]
    return f"https://generativelanguage.googleapis.com/v1beta/models/{model_clean}:generateContent"


# Thinking level sent to the API. Default "low" (R4-gemini-lite decision 2); a thinking-on rerun
# sets R4_THINKING_LEVEL (for example "high") and writes to its own --real-dir.
THINKING_LEVEL = os.environ.get("R4_THINKING_LEVEL", "low")


def build_request_body(prompt):
    """
    Builds the Gemini API request body per Decision 1:
    - prompt as a single user turn
    - generationConfig.thinkingConfig.thinkingLevel = "low"
    - includeThoughts left off
    - no system instruction, no tools, no other sampling settings
    """
    return {
        "contents": [
            {
                "role": "user",
                "parts": [
                    {"text": prompt}
                ]
            }
        ],
        "generationConfig": {
            "thinkingConfig": {
                "thinkingLevel": THINKING_LEVEL
            }
        }
    }


def get_request_body_template():
    """Returns the request body structure without prompt for recording in run_info.json."""
    return {
        "contents": [
            {
                "role": "user",
                "parts": [
                    {"text": "<prompt>"}
                ]
            }
        ],
        "generationConfig": {
            "thinkingConfig": {
                "thinkingLevel": THINKING_LEVEL
            }
        }
    }


def extract_tokens(resp_json):
    """
    Extracts token counts from Gemini usageMetadata per R4-gemini-lite.md:
    input is promptTokenCount;
    thinking is thoughtsTokenCount (0 if absent);
    output is candidatesTokenCount.
    """
    if not isinstance(resp_json, dict):
        return 0, 0, 0
    usage = resp_json.get("usageMetadata", {})
    if not isinstance(usage, dict):
        return 0, 0, 0
    inp = int(usage.get("promptTokenCount", 0) or 0)
    thinking = int(usage.get("thoughtsTokenCount", 0) or 0)
    out = int(usage.get("candidatesTokenCount", 0) or 0)
    return inp, thinking, out


def extract_answer_text(resp_json):
    """
    Extracts answer text from Gemini response per R4-gemini-lite.md:
    every text part of the first candidate, joined, with thought parts left out.
    """
    if not isinstance(resp_json, dict):
        return ""
    candidates = resp_json.get("candidates")
    if not isinstance(candidates, list) or not candidates:
        return ""
    first_candidate = candidates[0]
    if not isinstance(first_candidate, dict):
        return ""
    content = first_candidate.get("content")
    if not isinstance(content, dict):
        return ""
    parts = content.get("parts")
    if not isinstance(parts, list):
        return ""

    text_parts = []
    for part in parts:
        if isinstance(part, dict):
            if part.get("thought") is True:
                continue
            txt = part.get("text")
            if txt:
                text_parts.append(str(txt))
    return "".join(text_parts)


def parse_retry_delay(http_err, error_json=None):
    """
    Extracts retry delay in seconds from Gemini error details (RetryInfo) or HTTP headers (Retry-After).
    Per R4-gemini-lite-fix.md:
    - a RetryInfo with retryDelay, for example "51s". Wait that long (plus 1 second) before retrying.
    - Fall back to Retry-After or the 10 / 20 / 40 s backoff only if neither is present.
    """
    details = []
    if isinstance(error_json, dict):
        err_obj = error_json.get("error", {})
        if isinstance(err_obj, dict):
            details = err_obj.get("details", [])
        elif isinstance(error_json.get("details"), list):
            details = error_json.get("details")

    # 1. Look for RetryInfo with retryDelay in details
    if isinstance(details, list):
        for d in details:
            if isinstance(d, dict) and "retryDelay" in d:
                is_retry_info = "retryinfo" in str(d.get("@type", "")).lower()
                rd = str(d["retryDelay"]).strip()
                if rd.endswith("s"):
                    rd = rd[:-1]
                try:
                    val = float(rd)
                    if val > 0:
                        if is_retry_info:
                            return val + 1.0
                except ValueError:
                    pass

    # 2. Fall back to Retry-After header
    if hasattr(http_err, "headers") and http_err.headers:
        retry_after = http_err.headers.get("Retry-After")
        if retry_after:
            try:
                val = float(retry_after)
                if val > 0:
                    return val
            except ValueError:
                pass

    # 3. Fall back to any other retryDelay in details without explicit RetryInfo type
    if isinstance(details, list):
        for d in details:
            if isinstance(d, dict) and "retryDelay" in d:
                rd = str(d["retryDelay"]).strip()
                if rd.endswith("s"):
                    rd = rd[:-1]
                try:
                    val = float(rd)
                    if val > 0:
                        return val
                except ValueError:
                    pass

    return None


def classify_rate_limit(status_code, error_body_str="", error_json=None):
    """
    Classifies an HTTP 429 / 403 / 503 response per R4-gemini-lite-fix.md:
    1. Check structured details:
       - a QuotaFailure whose violations[].quotaId names the limit:
         - quotaId containing 'PerMinute': 'per_minute' (wait and retry)
         - quotaId containing 'PerDay': 'daily' (stop cleanly)
       - a RetryInfo with retryDelay:
         - if no daily indication: 'per_minute'
    2. When details say neither per-minute nor per-day:
       - Check message text for explicit daily signals -> 'daily'
       - Otherwise -> 'neither'
    """
    if status_code not in (429, 403, 503):
        return "neither"

    details = []
    if isinstance(error_json, dict):
        err = error_json.get("error", {})
        if isinstance(err, dict):
            details = err.get("details", [])
        elif isinstance(error_json.get("details"), list):
            details = error_json.get("details")
        if not isinstance(details, list):
            details = []

    has_per_minute = False
    has_per_day = False
    has_retry_info = False

    for d in details:
        if not isinstance(d, dict):
            continue
        if "retryinfo" in str(d.get("@type", "")).lower() or "retryDelay" in d:
            has_retry_info = True

        violations = d.get("violations", [])
        if isinstance(violations, list):
            for v in violations:
                if isinstance(v, dict):
                    qid = str(v.get("quotaId", ""))
                    qmetric = str(v.get("quotaMetric", ""))
                    qdesc = str(v.get("description", ""))
                    combined = f"{qid} {qmetric} {qdesc}".lower()
                    if "perminute" in combined or "per_minute" in combined:
                        has_per_minute = True
                    if "perday" in combined or "per_day" in combined:
                        has_per_day = True

    if has_per_minute:
        return "per_minute"
    if has_per_day:
        return "daily"
    if has_retry_info:
        return "per_minute"

    # Details say neither per-minute nor per-day. Check explicit daily text signals
    text = (error_body_str or "").lower()
    if isinstance(error_json, dict):
        err = error_json.get("error", {})
        if isinstance(err, dict):
            text += " " + str(err.get("message", "")).lower()

    daily_signals = [
        "per day",
        "per_day",
        "perday",
        "daily",
        "requests per day",
    ]
    if any(q in text for q in daily_signals):
        return "daily"

    return "neither"


def is_quota_exhausted(status_code, error_body_str="", error_json=None):
    """
    Detects whether an HTTP error signifies daily or project quota exhaustion per Decision 4.
    """
    return classify_rate_limit(status_code, error_body_str, error_json) == "daily"


def process_gemini_response(resp_json, model_name=""):
    """
    Parses and validates a Gemini JSON response.
    Returns:
    (is_success, finish_reason, answer_text, resp_json, error_msg, input_tokens, thinking_tokens, output_tokens, model_version, finishReason)
    """
    if not isinstance(resp_json, dict):
        err = f"Response is not a JSON object: {type(resp_json)}"
        return False, "error: invalid json", "", {"error": err}, err, 0, 0, 0, model_name, "INVALID"

    if "error" in resp_json:
        err_obj = resp_json["error"]
        msg = err_obj.get("message") if isinstance(err_obj, dict) else str(err_obj)
        return False, "error: API error", "", resp_json, str(msg), 0, 0, 0, model_name, "ERROR"

    candidates = resp_json.get("candidates")
    if not isinstance(candidates, list) or len(candidates) == 0:
        pf = resp_json.get("promptFeedback", {})
        block_reason = pf.get("blockReason") if isinstance(pf, dict) else None
        if block_reason:
            err = f"Prompt blocked: {block_reason}"
            return False, f"error: blocked prompt ({block_reason})", "", resp_json, err, 0, 0, 0, model_name, block_reason
        err = "No candidates in response"
        return False, "error: no candidates", "", resp_json, err, 0, 0, 0, model_name, "NO_CANDIDATES"

    first_cand = candidates[0]
    if not isinstance(first_cand, dict):
        err = "First candidate is not a dict"
        return False, "error: invalid candidate", "", resp_json, err, 0, 0, 0, model_name, "INVALID"

    finish_reason = first_cand.get("finishReason") or "UNKNOWN"
    if finish_reason in ("SAFETY", "RECITATION", "BLOCKLIST", "PROHIBITED_CONTENT", "SPII"):
        err = f"Blocked by safety/filter: {finish_reason}"
        return False, f"error: blocked ({finish_reason})", "", resp_json, err, 0, 0, 0, model_name, finish_reason

    answer_text = extract_answer_text(resp_json)
    if not answer_text.strip():
        err = "Empty or whitespace-only answer text from model"
        return False, "error: empty answer", "", resp_json, err, 0, 0, 0, model_name, finish_reason

    inp, thinking, out = extract_tokens(resp_json)
    model_version = resp_json.get("modelVersion") or model_name

    return True, finish_reason, answer_text, resp_json, None, inp, thinking, out, model_version, finish_reason


def call_gemini_api(
    prompt,
    model,
    api_key,
    timeout_sec=TIMEOUT_SEC,
    backoff_delays=BACKOFF_DELAYS,
    sleep_fn=time.sleep,
    url_opener=None,
    rate_limiter=None,
):
    """
    Executes a POST request to the Gemini API using standard library urllib.
    Handles per-minute rate limits with retries, and raises QuotaExhaustedError on daily/project quota exhaustion.
    Per-minute waits don't count toward the 5-try limit for other errors.
    Caps total per-minute wait for one call at 10 minutes (600s), then treats as failure.
    Paces requests with rate_limiter if provided (at most 14 requests started in rolling 60s).
    """
    url = get_api_url(model)
    body = build_request_body(prompt)
    data = json.dumps(body).encode("utf-8")
    headers = {
        "Content-Type": "application/json",
        "x-goog-api-key": api_key,
    }

    max_general_tries = 5  # Per Decision 4: up to 5 tries for general errors / backoff
    general_try = 0
    total_per_minute_wait = 0.0
    per_minute_count = 0

    while True:
        if rate_limiter is not None:
            rate_limiter.acquire()

        req = urllib.request.Request(url, data=data, headers=headers, method="POST")
        t0 = time.time()
        try:
            opener = url_opener if url_opener is not None else urllib.request.urlopen
            with opener(req, timeout=timeout_sec) as resp:
                elapsed = time.time() - t0
                raw_data = resp.read()
                resp_str = raw_data.decode("utf-8") if isinstance(raw_data, bytes) else str(raw_data)
                resp_json = json.loads(resp_str)
                resp_json = scrub_api_key(resp_json, api_key)
                return True, resp_json, elapsed, None
        except urllib.error.HTTPError as e:
            elapsed = time.time() - t0
            raw_err = e.read() if hasattr(e, "read") else b""
            try:
                err_str = raw_err.decode("utf-8") if isinstance(raw_err, bytes) else str(raw_err)
                err_json = json.loads(err_str)
            except Exception:
                err_str = str(raw_err)
                err_json = {"error": err_str}

            err_str = scrub_api_key(err_str, api_key)
            err_json = scrub_api_key(err_json, api_key)

            limit_type = classify_rate_limit(e.code, err_str, err_json)

            # Daily limit: stop cleanly as now
            if limit_type == "daily":
                raise QuotaExhaustedError(f"HTTP {e.code} Quota Exhausted: {err_str}")

            # Per-minute limit: wait and retry; does not count toward the 5-try limit
            if limit_type == "per_minute":
                wait_sec = parse_retry_delay(e, err_json)
                if wait_sec is None or wait_sec <= 0:
                    idx = min(per_minute_count, len(backoff_delays) - 1)
                    wait_sec = backoff_delays[idx]

                # Cap the total wait for one call at 10 minutes (600s), then treat as failure
                if total_per_minute_wait + wait_sec > 600.0:
                    print(
                        f"Per-minute rate limit wait exceeded 10-minute cap "
                        f"({total_per_minute_wait + wait_sec:.1f}s > 600s). Marking call as failed.",
                        flush=True,
                    )
                    return False, err_json, elapsed, f"Per-minute rate limit wait exceeded 10-minute cap: {err_str}"

                total_per_minute_wait += wait_sec
                per_minute_count += 1
                print(
                    f"Rate limited (HTTP {e.code}, per-minute limit). Waiting {wait_sec:.1f}s before retry "
                    f"(waited {total_per_minute_wait:.1f}s/600s)...",
                    flush=True,
                )
                sleep_fn(wait_sec)
                continue

            # When details say neither per-minute nor per-day, keep existing behavior:
            # back off, and stop cleanly only if 429s keep coming after the backoff
            if e.code in (429, 503):
                general_try += 1
                if general_try < max_general_tries:
                    wait_sec = parse_retry_delay(e, err_json)
                    if wait_sec is None or wait_sec <= 0:
                        idx = min(general_try - 1, len(backoff_delays) - 1)
                        wait_sec = backoff_delays[idx]
                    print(
                        f"Rate limited (HTTP {e.code}). Waiting {wait_sec:.1f}s before retry "
                        f"(attempt {general_try}/{max_general_tries})...",
                        flush=True,
                    )
                    sleep_fn(wait_sec)
                    continue
                else:
                    raise QuotaExhaustedError(
                        f"Rate limits persisted after {max_general_tries} retries (HTTP {e.code}): {err_str}"
                    )

            return False, err_json, elapsed, f"HTTP {e.code}: {err_str}"
        except urllib.error.URLError as e:
            elapsed = time.time() - t0
            reason_str = scrub_api_key(str(e.reason), api_key)
            if "timed out" in reason_str.lower():
                return False, {"error": "timeout"}, elapsed, f"Timed out after {timeout_sec}s"
            return False, {"error": reason_str}, elapsed, f"URLError: {reason_str}"
        except (TimeoutError, socket.timeout):
            elapsed = time.time() - t0
            return False, {"error": "timeout"}, elapsed, f"Timed out after {timeout_sec}s"
        except Exception as e:
            elapsed = time.time() - t0
            if isinstance(e, QuotaExhaustedError):
                raise
            e_str = scrub_api_key(str(e), api_key)
            return False, {"error": e_str}, elapsed, f"Exception: {e_str}"

    return False, {"error": "Exhausted retries"}, 0.0, "Exhausted retries"


def record_run_info(
    real_dir=REAL_GEMINI_LITE_DIR,
    model_id="gemini-3.5-flash-lite",
    start_time=None,
    stop_time=None,
    stopped_at=None,
    preserve_stopped_at=False,
    api_key=None,
):
    """
    Records run_info.json per R4-gemini-lite.md:
    model ID, request body without prompt, thinking level, run order, start and stop times; never the key.
    """
    os.makedirs(real_dir, exist_ok=True)
    run_info_file = os.path.join(real_dir, "run_info.json")

    # Load existing if available to preserve start_time
    existing = {}
    if os.path.exists(run_info_file):
        try:
            with open(run_info_file, "r", encoding="utf-8") as f:
                existing = json.load(f)
        except Exception:
            pass

    st_str = start_time or existing.get("start_time") or datetime.now(timezone.utc).isoformat()
    end_str = stop_time or datetime.now(timezone.utc).isoformat()

    info = {
        "model_id": model_id,
        "request_body_template": get_request_body_template(),
        "thinking_level": THINKING_LEVEL,
        "run_order": "seed-by-seed (seed 1 to 4)",
        "start_time": st_str,
        "stop_time": end_str,
    }
    if stopped_at:
        info["stopped_at"] = stopped_at
    elif preserve_stopped_at and "stopped_at" in existing:
        info["stopped_at"] = existing["stopped_at"]

    cur_key = api_key or os.environ.get("GEMINI_API_KEY", "")
    info = scrub_api_key(info, cur_key)

    with open(run_info_file, "w", encoding="utf-8") as f:
        json.dump(info, f, indent=2)
    return info
    return info


def update_sha256sums(real_dir=REAL_GEMINI_LITE_DIR):
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


def run_collection(
    model,
    dry_run=False,
    real_dir=REAL_GEMINI_LITE_DIR,
    questions_path=QUESTIONS_PATH,
    data_paths=None,
    max_workers=PARALLEL,
    backoff_delays=BACKOFF_DELAYS,
    sleep_fn=time.sleep,
    url_opener=None,
    api_key=None,
    rate_limiter=None,
    time_fn=time.time,
):
    """
    Executes collection for R4-gemini-lite.
    Enforces seed-by-seed order: run all calls of seed 1 before seed 2, etc.
    """
    print("=== Struct Milestone R4-gemini-lite Collector (Gemini API) ===")
    print(f"Target directory: {real_dir}")
    print(f"Model ID: {model}")
    print(f"API Endpoint: {get_api_url(model or '<model>')}\n")

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

    # Build plan grouped strictly by seed
    for seed in SEEDS:
        for ds in DATASETS:
            fmts = get_applicable_formats(ds)
            for fmt in fmts:
                total_runs += 1
                plan.append({
                    "seed": seed,
                    "dataset": ds,
                    "format": fmt,
                    "file_stem": f"gemini_lite_r4_{fmt}_{ds}_seed{seed}",
                })

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
            prompt = build_prompt(fmt, ds, encoded, all_questions[ds])
            expected_prompt = build_prompt(fmt, ds, encoded, all_questions[ds])
            assert prompt == expected_prompt, f"Prompt mismatch on {fmt} {ds}"
            assert len(prompt) > 0

        print(f"Planned order (seed-by-seed):")
        for s in SEEDS:
            s_count = sum(1 for it in plan if it["seed"] == s)
            print(f"  Seed {s}: {s_count} calls (7 flat_tabular, 7 flat_100, 5 nested_fixed, 5 nested_variable)")
        print(f"Total: {len(plan)} calls")
        print(f"Dry run successful! All {len(plan)} prompts match collect.py byte-for-byte without calling API.")
        return True

    if not api_key:
        api_key = get_api_key(required=True)

    if not dry_run and rate_limiter is None:
        rate_limiter = RequestRateLimiter(
            max_requests=14,
            window_sec=60.0,
            time_fn=time_fn,
            sleep_fn=sleep_fn,
        )

    os.makedirs(raw_dir, exist_ok=True)
    start_time_iso = datetime.now(timezone.utc).isoformat()
    record_run_info(
        real_dir=real_dir,
        model_id=model,
        start_time=start_time_iso,
        preserve_stopped_at=True,
        api_key=api_key,
    )
    update_sha256sums(real_dir=real_dir)

    metadata = []
    done = set()
    if os.path.exists(metadata_file):
        try:
            with open(metadata_file, "r", encoding="utf-8") as f:
                metadata = json.load(f)
            # Only successful runs count as done; failures are retried on resume
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
    quota_exhausted_event = threading.Event()
    stopped_at_info = {}

    def run_one(idx, item):
        if quota_exhausted_event.is_set():
            return

        fmt = item["format"]
        ds = item["dataset"]
        seed = item["seed"]
        stem = item["file_stem"]

        if (fmt, ds, seed) in done:
            return

        print(f"[{idx}/{total_runs}] Calling Gemini API on {stem}...", flush=True)

        prompt = build_prompt(fmt, ds, encoded_cache[(fmt, ds)], all_questions[ds])

        try:
            call_ok, resp_json, dt, call_err = call_gemini_api(
                prompt=prompt,
                model=model,
                api_key=api_key,
                timeout_sec=TIMEOUT_SEC,
                backoff_delays=backoff_delays,
                sleep_fn=sleep_fn,
                url_opener=url_opener,
                rate_limiter=rate_limiter,
            )
            if call_ok:
                (
                    is_success,
                    finish_reason,
                    content,
                    resp_json_clean,
                    error_msg,
                    input_tokens,
                    thinking_tokens,
                    output_tokens,
                    model_version,
                    finishReason,
                ) = process_gemini_response(resp_json, model_name=model)
            else:
                is_success = False
                error_msg = scrub_api_key(call_err, api_key)
                finish_reason = f"error: {error_msg}"
                content = ""
                resp_json_clean = scrub_api_key(resp_json, api_key)
                input_tokens = 0
                thinking_tokens = 0
                output_tokens = 0
                model_version = model
                finishReason = "ERROR"
        except QuotaExhaustedError as qe:
            with meta_lock:
                quota_exhausted_event.set()
                stopped_at_info["seed"] = seed
                stopped_at_info["call"] = stem
                stopped_at_info["reason"] = scrub_api_key(str(qe), api_key)
                stopped_at_info["time"] = datetime.now(timezone.utc).isoformat()
            print(f"\n[QUOTA EXHAUSTED] Seed {seed}, call {stem}: {scrub_api_key(str(qe), api_key)}", flush=True)
            return
        except Exception as e:
            dt = 0.0
            is_success = False
            error_msg = scrub_api_key(str(e), api_key)
            finish_reason = f"error: {error_msg}"
            content = ""
            resp_json_clean = {"error": error_msg}
            input_tokens = 0
            thinking_tokens = 0
            output_tokens = 0
            model_version = model
            finishReason = "ERROR"

        content = scrub_api_key(content, api_key)
        raw_path = os.path.join(raw_dir, f"{stem}.txt")
        with open(raw_path, "w", encoding="utf-8") as f:
            f.write(content)

        usage_meta = resp_json_clean.get("usageMetadata", {}) if isinstance(resp_json_clean, dict) else {}

        meta_entry = {
            "run_index": idx,
            "format": fmt,
            "dataset": ds,
            "seed": seed,
            "file": f"{stem}.txt",
            "success": is_success,
            "finish_reason": finish_reason,
            "finishReason": finishReason,
            "elapsed_sec": dt,
            "input_tokens": input_tokens,
            "thinking_tokens": thinking_tokens,
            "output_tokens": output_tokens,
            "model_id": model,
            "modelVersion": model_version,
            "usageMetadata": usage_meta,
            "raw_response": resp_json_clean,
        }
        if error_msg:
            meta_entry["error"] = error_msg

        meta_entry = scrub_api_key(meta_entry, api_key)

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

    # Process seed by seed sequentially per Decision 3
    for seed in SEEDS:
        if quota_exhausted_event.is_set():
            break

        seed_items = [(idx, item) for idx, item in enumerate(plan, start=1) if item["seed"] == seed]
        pending_seed_items = [
            (idx, item)
            for idx, item in seed_items
            if (item["format"], item["dataset"], item["seed"]) not in done
        ]

        if not pending_seed_items:
            print(f"Seed {seed} already fully complete ({len(seed_items)}/{len(seed_items)} calls done).")
            continue

        print(f"\n--- Starting Seed {seed}: {len(pending_seed_items)} calls remaining of {len(seed_items)} ---", flush=True)

        with ThreadPoolExecutor(max_workers=max_workers) as pool:
            futures = [pool.submit(run_one, idx, item) for idx, item in pending_seed_items]
            for fut in futures:
                fut.result()

        if quota_exhausted_event.is_set():
            print(f"Run stopped at Seed {seed} due to quota exhaustion.")
            break

    stop_time_iso = datetime.now(timezone.utc).isoformat()
    record_run_info(
        real_dir=real_dir,
        model_id=model,
        start_time=start_time_iso,
        stop_time=stop_time_iso,
        stopped_at=stopped_at_info if stopped_at_info else None,
        preserve_stopped_at=False,
        api_key=api_key,
    )
    update_sha256sums(real_dir=real_dir)

    if quota_exhausted_event.is_set():
        print(f"\n[RUN STOPPED CLEANLY] Quota exhausted at seed {stopped_at_info.get('seed')}, call {stopped_at_info.get('call')}.")
        print("A later run will resume from this point.")
        return False

    print("\nCollection finished successfully!")
    return True


def main():
    parser = argparse.ArgumentParser(description="Collector for R4-gemini-lite")
    parser.add_argument("--model", type=str, default=None, help="Gemini model ID to call (required unless --dry-run)")
    parser.add_argument("--dry-run", action="store_true", help="Verify prompts and plan without calling API")
    parser.add_argument("--parallel", type=int, default=PARALLEL, help="Concurrent workers within current seed")
    parser.add_argument("--real-dir", type=str, default=REAL_GEMINI_LITE_DIR, help="Target output directory")

    args = parser.parse_args()

    if not args.dry_run and not args.model:
        print("ERROR: --model argument is required for collection (e.g. --model gemini-2.5-flash-lite)", file=sys.stderr)
        sys.exit(1)

    model_arg = args.model if args.model else "gemini-3.5-flash-lite"
    run_collection(
        model=model_arg,
        dry_run=args.dry_run,
        real_dir=args.real_dir,
        max_workers=args.parallel,
    )


if __name__ == "__main__":
    main()
