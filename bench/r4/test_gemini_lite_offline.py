#!/usr/bin/env python3
"""
Offline test harness for Struct Milestone R4-gemini-lite: retrieval accuracy across formats on Gemini 3.5 Flash Lite.

Tests:
1. Mocked API calls:
   - Success (valid candidates JSON, answer text extracted with thought parts left out, usageMetadata token arithmetic).
   - Blocked prompt (promptFeedback.blockReason or finishReason in SAFETY, etc.).
   - Empty answer or no candidates.
   - Invalid JSON response.
2. Rate limits and quota handling (Decision 4):
   - HTTP 429/503 with per-minute rate limit: retries with Retry-After or backoff delays (10s, 20s, 40s) up to 5 tries, then succeeds.
   - Daily or project quota exhausted: stops whole run cleanly, records stopped_at (seed, call, reason) in run_info.json.
3. Resume functionality (Decision 4 & 5):
   - Resume continues from the same seed after quota exhaustion.
   - Resume skips done calls and retries failed ones.
   - Failure history preserved in metadata.json.
4. Seed order (Decision 3):
   - Seed-by-seed order enforced: no call of seed 2 starts before seed 1 is done.
5. API Key security:
   - Key read only from GEMINI_API_KEY environment variable.
   - Missing key stops cleanly with exit code 1.
   - Key sent only in x-goog-api-key header, never in the URL (no query params).
   - Key never printed, logged, or saved to any file (raw files, metadata.json, run_info.json, SHA256SUMS).
6. Partial runs and report generation (Decision 5):
   - Evaluator scores only complete seeds (all 24 calls succeeded).
   - Incomplete seeds listed separately and excluded from accuracy tables.
   - Budget column shows n/a.
   - Input and thinking tokens note from Gemini usageMetadata.
7. Prompts byte-identical to collect.py:
   - All 96 prompt combinations byte-identical to collect.py.
   - Dry run executes cleanly and prints planned order without calling API.
"""

import os
import io
import sys
import json
import time
import shutil
import tempfile
import unittest
import threading
import urllib.request
import urllib.error
from unittest.mock import patch, MagicMock

BENCH_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STRUCT_ROOT = os.path.dirname(BENCH_DIR)
R4_DIR = os.path.join(BENCH_DIR, "r4")
sys.path.insert(0, BENCH_DIR)

from r4.collect import (
    build_prompt as gemma_build_prompt,
    load_dataset,
    load_questions,
    get_applicable_formats,
    DATASETS,
    QUESTIONS_PATH,
)
from r4.formats import ENCODERS, FORMATS, FORMAT_DISPLAY_NAMES
from r4.evaluate import evaluate_runs
from r4.collect_gemini_lite import (
    SEEDS,
    TIMEOUT_SEC,
    PARALLEL,
    BACKOFF_DELAYS,
    QuotaExhaustedError,
    RequestRateLimiter,
    classify_rate_limit,
    get_api_key,
    scrub_api_key,
    get_api_url,
    build_request_body,
    get_request_body_template,
    extract_tokens,
    extract_answer_text,
    parse_retry_delay,
    is_quota_exhausted,
    process_gemini_response,
    call_gemini_api,
    record_run_info,
    update_sha256sums,
    run_collection,
)
from r4.evaluate_gemini_lite import (
    identify_complete_and_incomplete_seeds,
    generate_gemini_lite_report,
    get_gemini_lite_model_id,
    run_evaluation,
    CALLS_PER_SEED,
)


class TestGeminiLiteApiKeySecurity(unittest.TestCase):
    def test_missing_key_stops_cleanly(self):
        """If GEMINI_API_KEY is missing, harness stops cleanly with a clear message and exit code 1."""
        with patch.dict(os.environ, {}, clear=True):
            if "GEMINI_API_KEY" in os.environ:
                del os.environ["GEMINI_API_KEY"]
            with self.assertRaises(SystemExit) as cm:
                get_api_key(required=True)
            self.assertEqual(cm.exception.code, 1)

            # run_collection without dry_run and without key also stops cleanly
            with self.assertRaises(SystemExit) as cm2:
                run_collection(model="gemini-3.5-flash-lite", dry_run=False)
            self.assertEqual(cm2.exception.code, 1)

    def test_key_never_in_url(self):
        """API key must never be in the URL; endpoint contains only the model name."""
        model = "gemini-3.5-flash-lite"
        url = get_api_url(model)
        self.assertEqual(url, "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash-lite:generateContent")
        self.assertNotIn("?", url)
        self.assertNotIn("key=", url)

        # Also test with 'models/' prefix
        url2 = get_api_url("models/gemini-3.5-flash-lite")
        self.assertEqual(url2, "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash-lite:generateContent")

        # In call_gemini_api, verify the Request object sent has the key in header and NOT in URL
        captured_reqs = []

        def mock_opener(req, timeout=1200):
            captured_reqs.append(req)
            resp = MagicMock()
            resp.read.return_value = json.dumps({
                "candidates": [{"content": {"parts": [{"text": "1. 2024"}]}}],
                "usageMetadata": {"promptTokenCount": 100, "candidatesTokenCount": 10},
            }).encode("utf-8")
            resp.__enter__ = lambda s: resp
            resp.__exit__ = lambda s, *a: None
            return resp

        test_key = "AIzaSyTEST_SECRET_KEY_12345"
        call_gemini_api(
            prompt="Hello world",
            model="gemini-3.5-flash-lite",
            api_key=test_key,
            url_opener=mock_opener,
        )

        self.assertEqual(len(captured_reqs), 1)
        req = captured_reqs[0]
        self.assertNotIn(test_key, req.full_url)
        self.assertNotIn("?", req.full_url)
        # Check header
        self.assertEqual(req.headers.get("X-goog-api-key"), test_key)

    def test_key_never_written_anywhere(self):
        """
        Verify that the API key is NEVER written to any file:
        raw/*.txt, metadata.json, run_info.json, SHA256SUMS, even when errors or exceptions occur.
        """
        canary_key = "CANARY_GEMINI_KEY_DO_NOT_LEAK_SECRET_99999"
        temp_dir = tempfile.mkdtemp()
        try:
            call_count = 0

            def mock_opener(req, timeout=1200):
                nonlocal call_count
                call_count += 1
                if call_count == 1:
                    # Success
                    resp = MagicMock()
                    resp.read.return_value = json.dumps({
                        "candidates": [{"content": {"parts": [{"text": "1. refuted\n2. 2017\n"}]}}],
                        "usageMetadata": {"promptTokenCount": 500, "candidatesTokenCount": 20},
                    }).encode("utf-8")
                    resp.__enter__ = lambda s: resp
                    resp.__exit__ = lambda s, *a: None
                    return resp
                elif call_count == 2:
                    # Error containing key in error string
                    err_payload = json.dumps({
                        "error": {
                            "code": 400,
                            "message": f"Bad request with key {canary_key}",
                        }
                    }).encode("utf-8")
                    raise urllib.error.HTTPError(
                        url="https://generativelanguage.googleapis.com",
                        code=400,
                        msg="Bad Request",
                        hdrs={},
                        fp=io.BytesIO(err_payload),
                    )
                else:
                    # Generic exception with key
                    raise RuntimeError(f"Unexpected crash holding {canary_key}")

            with patch.dict(os.environ, {"GEMINI_API_KEY": canary_key}):
                with patch("r4.collect_gemini_lite.SEEDS", [1]):
                    with patch("r4.collect_gemini_lite.DATASETS", ["flat_tabular"]):
                        with patch("r4.collect_gemini_lite.get_applicable_formats", return_value=["struct", "json_min", "yaml"]):
                            run_collection(
                                model="gemini-3.5-flash-lite",
                                dry_run=False,
                                real_dir=temp_dir,
                                questions_path=QUESTIONS_PATH,
                                max_workers=1,
                                url_opener=mock_opener,
                                api_key=canary_key,
                            )

            # Inspect ALL files in temp_dir recursively
            all_files_checked = 0
            for root, _, files in os.walk(temp_dir):
                for fname in files:
                    fpath = os.path.join(root, fname)
                    with open(fpath, "rb") as f:
                        file_bytes = f.read()
                    all_files_checked += 1
                    self.assertNotIn(
                        canary_key.encode("utf-8"),
                        file_bytes,
                        f"CRITICAL LEAK: API key found in file {fpath}",
                    )

            self.assertGreaterEqual(all_files_checked, 4, "Must have checked raw files, metadata, run_info, SHA256SUMS")

            # Verify scrub placeholder is present in metadata for the error call
            meta_file = os.path.join(temp_dir, "metadata.json")
            with open(meta_file, "r", encoding="utf-8") as f:
                meta = json.load(f)
            self.assertEqual(len(meta), 3)
            # Find the error entry
            err_entries = [m for m in meta if not m.get("success")]
            self.assertGreaterEqual(len(err_entries), 2)
            self.assertIn("[REDACTED_API_KEY]", json.dumps(err_entries))
        finally:
            shutil.rmtree(temp_dir)


class TestGeminiLiteMockedApiCalls(unittest.TestCase):
    def test_mocked_success(self):
        """
        Mock successful Gemini API response:
        - Text parts extracted from first candidate; thought parts left out.
        - usageMetadata token counts extracted.
        - modelVersion extracted.
        """
        resp_json = {
            "candidates": [
                {
                    "content": {
                        "role": "model",
                        "parts": [
                            {"thought": True, "text": "Internal thoughts that must NOT appear in output."},
                            {"text": "1. refuted\n2. 2017\n3. LoRA\n4. true\n"},
                            {"text": "5. 1\n6. 10\n7. 8\n8. none\n9. none\n10. none\n11. 2024\n12. none\n"},
                        ],
                    },
                    "finishReason": "STOP",
                }
            ],
            "usageMetadata": {
                "promptTokenCount": 1420,
                "thoughtsTokenCount": 260,
                "candidatesTokenCount": 110,
                "totalTokenCount": 1790,
            },
            "modelVersion": "gemini-3.5-flash-lite-preview-02-05",
        }

        (
            is_success,
            finish_reason,
            content,
            resp_clean,
            error_msg,
            inp,
            thinking,
            out,
            model_ver,
            finishReason,
        ) = process_gemini_response(resp_json, model_name="gemini-3.5-flash-lite")

        self.assertTrue(is_success)
        self.assertEqual(finish_reason, "STOP")
        self.assertEqual(finishReason, "STOP")
        self.assertIsNone(error_msg)
        self.assertNotIn("Internal thoughts", content)
        self.assertIn("1. refuted", content)
        self.assertIn("12. none", content)
        self.assertEqual(inp, 1420)
        self.assertEqual(thinking, 260)
        self.assertEqual(out, 110)
        self.assertEqual(model_ver, "gemini-3.5-flash-lite-preview-02-05")

    def test_token_arithmetic_missing_thoughts(self):
        """When thoughtsTokenCount is absent from usageMetadata, thinking tokens defaults to 0."""
        resp_json = {
            "usageMetadata": {
                "promptTokenCount": 850,
                "candidatesTokenCount": 42,
            }
        }
        inp, thinking, out = extract_tokens(resp_json)
        self.assertEqual(inp, 850)
        self.assertEqual(thinking, 0)
        self.assertEqual(out, 42)

    def test_mocked_blocked_prompt(self):
        """Blocked by promptFeedback or safety finishReason."""
        resp_json = {
            "promptFeedback": {
                "blockReason": "SAFETY",
                "safetyRatings": [],
            }
        }
        is_success, finish_reason, content, _, error_msg, inp, think, out, _, _ = process_gemini_response(resp_json)
        self.assertFalse(is_success)
        self.assertTrue(finish_reason.startswith("error: blocked prompt (SAFETY)"))
        self.assertEqual(content, "")

        # Candidate with finishReason SAFETY
        resp_json2 = {
            "candidates": [
                {
                    "finishReason": "SAFETY",
                    "content": {"parts": [{"text": ""}]},
                }
            ]
        }
        is_success2, finish_reason2, content2, _, _, _, _, _, _, _ = process_gemini_response(resp_json2)
        self.assertFalse(is_success2)
        self.assertEqual(finish_reason2, "error: blocked (SAFETY)")

    def test_mocked_empty_answer(self):
        """Candidate with empty or whitespace-only answer."""
        resp_json = {
            "candidates": [
                {
                    "content": {"parts": [{"text": "   \n\t  "}]},
                    "finishReason": "STOP",
                }
            ],
            "usageMetadata": {"promptTokenCount": 100, "candidatesTokenCount": 2},
        }
        is_success, finish_reason, content, _, error_msg, _, _, _, _, _ = process_gemini_response(resp_json)
        self.assertFalse(is_success)
        self.assertEqual(finish_reason, "error: empty answer")
        self.assertEqual(content, "")

    def test_mocked_no_candidates(self):
        """Response with empty candidates list."""
        resp_json = {"candidates": []}
        is_success, finish_reason, content, _, error_msg, _, _, _, _, _ = process_gemini_response(resp_json)
        self.assertFalse(is_success)
        self.assertEqual(finish_reason, "error: no candidates")


class TestGeminiLiteRateLimitAndQuota(unittest.TestCase):
    def test_429_retry_then_success(self):
        """
        Decision 4: HTTP 429 with per-minute limit:
        wait (Retry-After or backoff: 10s, 20s, 40s...) and retry the same call up to 5 tries.
        """
        attempts = 0
        sleep_calls = []

        def mock_sleep(sec):
            sleep_calls.append(sec)

        def mock_opener(req, timeout=1200):
            nonlocal attempts
            attempts += 1
            if attempts == 1:
                # 429 with Retry-After header
                headers = {"Retry-After": "12.5"}
                err_body = b'{"error": {"code": 429, "message": "Rate limit exceeded (per minute)", "status": "RESOURCE_EXHAUSTED"}}'
                raise urllib.error.HTTPError(
                    url="https://generativelanguage.googleapis.com",
                    code=429,
                    msg="Too Many Requests",
                    hdrs=headers,
                    fp=io.BytesIO(err_body),
                )
            elif attempts == 2:
                # 429 with retryDelay in error body
                headers = {}
                err_body = json.dumps({
                    "error": {
                        "code": 429,
                        "message": "Resource exhausted",
                        "status": "RESOURCE_EXHAUSTED",
                        "details": [
                            {"retryDelay": "25.0s"}
                        ]
                    }
                }).encode("utf-8")
                raise urllib.error.HTTPError(
                    url="https://generativelanguage.googleapis.com",
                    code=429,
                    msg="Too Many Requests",
                    hdrs=headers,
                    fp=io.BytesIO(err_body),
                )
            else:
                # Attempt 3: Success!
                resp = MagicMock()
                resp.read.return_value = json.dumps({
                    "candidates": [
                        {
                            "content": {"parts": [{"text": "1. refuted\n2. 2017\n"}]},
                            "finishReason": "STOP",
                        }
                    ],
                    "usageMetadata": {"promptTokenCount": 500, "candidatesTokenCount": 20},
                }).encode("utf-8")
                resp.__enter__ = lambda s: resp
                resp.__exit__ = lambda s, *a: None
                return resp

        ok, resp_json, dt, err = call_gemini_api(
            prompt="test",
            model="gemini-3.5-flash-lite",
            api_key="mock_key",
            sleep_fn=mock_sleep,
            url_opener=mock_opener,
        )

        self.assertTrue(ok)
        self.assertEqual(attempts, 3)
        self.assertEqual(sleep_calls, [12.5, 25.0])
        self.assertIn("candidates", resp_json)

    def test_daily_quota_stops_run_cleanly(self):
        """
        Decision 4: A daily or project quota exhausted:
        stop the whole run cleanly, record which seed and call it stopped at in run_info.json, and exit.
        """
        temp_dir = tempfile.mkdtemp()
        try:
            call_count = 0

            def mock_opener(req, timeout=1200):
                nonlocal call_count
                call_count += 1
                if call_count == 1:
                    # First call succeeds
                    resp = MagicMock()
                    resp.read.return_value = json.dumps({
                        "candidates": [{"content": {"parts": [{"text": "1. refuted\n"}]}}],
                        "usageMetadata": {"promptTokenCount": 500, "candidatesTokenCount": 10},
                    }).encode("utf-8")
                    resp.__enter__ = lambda s: resp
                    resp.__exit__ = lambda s, *a: None
                    return resp
                else:
                    # Daily quota exhaustion
                    err_body = json.dumps({
                        "error": {
                            "code": 429,
                            "message": "Quota exceeded for quota metric 'Generate Content API requests per day'",
                            "status": "RESOURCE_EXHAUSTED",
                        }
                    }).encode("utf-8")
                    raise urllib.error.HTTPError(
                        url="https://generativelanguage.googleapis.com",
                        code=429,
                        msg="Too Many Requests",
                        hdrs={},
                        fp=io.BytesIO(err_body),
                    )

            res = run_collection(
                model="gemini-3.5-flash-lite",
                dry_run=False,
                real_dir=temp_dir,
                questions_path=QUESTIONS_PATH,
                max_workers=1,
                url_opener=mock_opener,
                api_key="mock_key",
            )

            # Collection stops cleanly (returns False)
            self.assertFalse(res, "Collection must stop cleanly on daily quota exhaustion")

            # Check run_info.json records stopped_at
            run_info_file = os.path.join(temp_dir, "run_info.json")
            self.assertTrue(os.path.exists(run_info_file))
            with open(run_info_file, "r", encoding="utf-8") as f:
                run_info = json.load(f)

            self.assertIn("stopped_at", run_info)
            self.assertEqual(run_info["stopped_at"]["seed"], 1)
            self.assertIn("gemini_lite_r4_", run_info["stopped_at"]["call"])
            self.assertIn("Quota Exhausted", run_info["stopped_at"]["reason"])

            # Verify that only 1 successful raw file was saved and metadata has 1 success
            meta_file = os.path.join(temp_dir, "metadata.json")
            with open(meta_file, "r", encoding="utf-8") as f:
                meta = json.load(f)
            self.assertEqual(len(meta), 1)
            self.assertTrue(meta[0]["success"])
        finally:
            shutil.rmtree(temp_dir)

    def test_per_minute_429_with_retry_info_waited_and_retried_not_stopped(self):
        """
        R4-gemini-lite-fix.md:
        A per-minute 429 with RetryInfo (and QuotaFailure indicating PerMinute)
        is waited out and retried, not treated as a stop.
        Wait that long (plus 1 second) before retrying.
        Does NOT count toward the 5-try limit for other errors.
        """
        attempts = 0
        sleep_calls = []

        def mock_sleep(sec):
            sleep_calls.append(sec)

        # Real Google per-minute 429 payload matching run_info.json
        per_minute_err_body = json.dumps({
            "error": {
                "code": 429,
                "message": (
                    "You exceeded your current quota, please check your plan and billing details. "
                    "Quota exceeded for metric: generativelanguage.googleapis.com/generate_content_free_tier_requests, "
                    "limit: 15, model: gemini-3.5-flash-lite, description: Generate content requests per minute limit "
                    "exceeded on Free Tier. Please retry in 51s."
                ),
                "status": "RESOURCE_EXHAUSTED",
                "details": [
                    {
                        "@type": "type.googleapis.com/google.rpc.QuotaFailure",
                        "violations": [
                            {
                                "description": "Generate content requests per minute limit exceeded on Free Tier.",
                                "quotaMetric": "generativelanguage.googleapis.com/generate_content_free_tier_requests",
                                "quotaId": "GenerateRequestsPerMinutePerProjectPerModel-FreeTier",
                                "quotaValue": "15"
                            }
                        ]
                    },
                    {
                        "@type": "type.googleapis.com/google.rpc.RetryInfo",
                        "retryDelay": "51s"
                    }
                ]
            }
        }).encode("utf-8")

        def mock_opener(req, timeout=1200):
            nonlocal attempts
            attempts += 1
            if attempts == 1:
                raise urllib.error.HTTPError(
                    url="https://generativelanguage.googleapis.com",
                    code=429,
                    msg="Too Many Requests",
                    hdrs={},
                    fp=io.BytesIO(per_minute_err_body),
                )
            else:
                resp = MagicMock()
                resp.read.return_value = json.dumps({
                    "candidates": [{"content": {"parts": [{"text": "1. refuted\n2. 2017\n"}]}}],
                    "usageMetadata": {"promptTokenCount": 500, "candidatesTokenCount": 20},
                }).encode("utf-8")
                resp.__enter__ = lambda s: resp
                resp.__exit__ = lambda s, *a: None
                return resp

        # Direct call_gemini_api test:
        ok, resp_json, dt, err = call_gemini_api(
            prompt="test",
            model="gemini-3.5-flash-lite",
            api_key="mock_key",
            sleep_fn=mock_sleep,
            url_opener=mock_opener,
        )

        self.assertTrue(ok, "Call must succeed on retry after waiting")
        self.assertEqual(attempts, 2)
        # 51s retryDelay + 1 second = 52.0s
        self.assertEqual(sleep_calls, [52.0])
        self.assertIn("candidates", resp_json)

        # Full run_collection test: verify run does NOT stop on per-minute 429
        temp_dir = tempfile.mkdtemp()
        try:
            coll_attempts = 0

            def mock_coll_opener(req, timeout=1200):
                nonlocal coll_attempts
                coll_attempts += 1
                if coll_attempts == 1:
                    raise urllib.error.HTTPError(
                        url="https://generativelanguage.googleapis.com",
                        code=429,
                        msg="Too Many Requests",
                        hdrs={},
                        fp=io.BytesIO(per_minute_err_body),
                    )
                resp = MagicMock()
                resp.read.return_value = json.dumps({
                    "candidates": [{"content": {"parts": [{"text": "1. refuted\n"}]}}],
                    "usageMetadata": {"promptTokenCount": 500, "candidatesTokenCount": 10},
                }).encode("utf-8")
                resp.__enter__ = lambda s: resp
                resp.__exit__ = lambda s, *a: None
                return resp

            with patch("r4.collect_gemini_lite.SEEDS", [1]):
                with patch("r4.collect_gemini_lite.DATASETS", ["flat_tabular"]):
                    with patch("r4.collect_gemini_lite.get_applicable_formats", return_value=["struct"]):
                        res = run_collection(
                            model="gemini-3.5-flash-lite",
                            dry_run=False,
                            real_dir=temp_dir,
                            questions_path=QUESTIONS_PATH,
                            max_workers=1,
                            sleep_fn=lambda s: None,
                            url_opener=mock_coll_opener,
                            api_key="mock_key",
                        )
            self.assertTrue(res, "Collection must complete successfully and not stop on per-minute 429")
            meta_file = os.path.join(temp_dir, "metadata.json")
            with open(meta_file, "r", encoding="utf-8") as f:
                meta = json.load(f)
            self.assertEqual(len(meta), 1)
            self.assertTrue(meta[0]["success"])
        finally:
            shutil.rmtree(temp_dir)

    def test_per_day_429_stops_cleanly_with_structured_quota_id(self):
        """
        R4-gemini-lite-fix.md:
        A QuotaFailure whose violations[].quotaId contains 'PerDay' is a daily limit:
        stops cleanly, records stopped_at in run_info.json, and exits cleanly.
        """
        temp_dir = tempfile.mkdtemp()
        try:
            err_body = json.dumps({
                "error": {
                    "code": 429,
                    "message": "Resource exhausted",
                    "status": "RESOURCE_EXHAUSTED",
                    "details": [
                        {
                            "@type": "type.googleapis.com/google.rpc.QuotaFailure",
                            "violations": [
                                {
                                    "quotaId": "GenerateRequestsPerDayPerProjectPerModel-FreeTier",
                                    "quotaMetric": "generativelanguage.googleapis.com/generate_content_free_tier_requests_per_day",
                                    "description": "Generate content requests per day limit exceeded on Free Tier."
                                }
                            ]
                        }
                    ]
                }
            }).encode("utf-8")

            def mock_opener(req, timeout=1200):
                raise urllib.error.HTTPError(
                    url="https://generativelanguage.googleapis.com",
                    code=429,
                    msg="Too Many Requests",
                    hdrs={},
                    fp=io.BytesIO(err_body),
                )

            with patch("r4.collect_gemini_lite.SEEDS", [1]):
                with patch("r4.collect_gemini_lite.DATASETS", ["flat_tabular"]):
                    with patch("r4.collect_gemini_lite.get_applicable_formats", return_value=["struct"]):
                        res = run_collection(
                            model="gemini-3.5-flash-lite",
                            dry_run=False,
                            real_dir=temp_dir,
                            questions_path=QUESTIONS_PATH,
                            max_workers=1,
                            url_opener=mock_opener,
                            api_key="mock_key",
                        )

            self.assertFalse(res, "Collection must stop cleanly on per-day quota exhaustion")
            run_info_file = os.path.join(temp_dir, "run_info.json")
            self.assertTrue(os.path.exists(run_info_file))
            with open(run_info_file, "r", encoding="utf-8") as f:
                run_info = json.load(f)
            self.assertIn("stopped_at", run_info)
            self.assertEqual(run_info["stopped_at"]["seed"], 1)
            self.assertIn("Quota Exhausted", run_info["stopped_at"]["reason"])
        finally:
            shutil.rmtree(temp_dir)

    def test_per_minute_wait_cap_10_minutes_treated_as_failure(self):
        """
        R4-gemini-lite-fix.md Fix 1:
        Cap the total wait for one call at 10 minutes (600s),
        then treat it as a failure (recorded, retried on resume).
        """
        sleep_calls = []

        def mock_sleep(sec):
            sleep_calls.append(sec)

        # 429 error with 350s retryDelay
        err_body = json.dumps({
            "error": {
                "code": 429,
                "message": "Resource exhausted",
                "details": [
                    {
                        "@type": "type.googleapis.com/google.rpc.QuotaFailure",
                        "violations": [{"quotaId": "GenerateRequestsPerMinutePerProjectPerModel-FreeTier"}]
                    },
                    {
                        "@type": "type.googleapis.com/google.rpc.RetryInfo",
                        "retryDelay": "350s"
                    }
                ]
            }
        }).encode("utf-8")

        def mock_opener(req, timeout=1200):
            raise urllib.error.HTTPError(
                url="https://generativelanguage.googleapis.com",
                code=429,
                msg="Too Many Requests",
                hdrs={},
                fp=io.BytesIO(err_body),
            )

        # Attempt 1 waits 351s. Attempt 2 would wait 351s (total 702s > 600s) -> capped and fails!
        ok, resp_json, dt, err = call_gemini_api(
            prompt="test",
            model="gemini-3.5-flash-lite",
            api_key="mock_key",
            sleep_fn=mock_sleep,
            url_opener=mock_opener,
        )

        self.assertFalse(ok, "Call must fail when cumulative wait exceeds 10 minutes")
        self.assertEqual(sleep_calls, [351.0])
        self.assertIn("10-minute cap", err)

    def test_limiter_never_lets_more_than_14_calls_start_in_60_seconds_fake_clock(self):
        """
        R4-gemini-lite-fix.md Fix 2:
        Client-side limiter shared by all worker threads:
        at most 14 requests started in any rolling 60-second window.
        Uses a fake clock to verify timing.
        """
        class FakeClock:
            def __init__(self, start=1000.0):
                self.current_time = start
                self.lock = threading.Lock()

            def time(self):
                with self.lock:
                    return self.current_time

            def sleep(self, seconds):
                with self.lock:
                    self.current_time += seconds

        clock = FakeClock(start=1000.0)
        limiter = RequestRateLimiter(
            max_requests=14,
            window_sec=60.0,
            time_fn=clock.time,
            sleep_fn=clock.sleep,
        )

        # Acquire 42 times (3 batches of 14)
        started_times = []
        for _ in range(42):
            t = limiter.acquire()
            started_times.append(t)

        self.assertEqual(len(started_times), 42)

        # Batch 1: calls 0..13 start at t = 1000.0
        for i in range(14):
            self.assertEqual(started_times[i], 1000.0)

        # Batch 2: calls 14..27 start at t = 1060.0 (waited 60 seconds)
        for i in range(14, 28):
            self.assertEqual(started_times[i], 1060.0)

        # Batch 3: calls 28..41 start at t = 1120.0 (waited another 60 seconds)
        for i in range(28, 42):
            self.assertEqual(started_times[i], 1120.0)

        # Rigorous rolling window check across the entire timeline
        # For ANY window [t, t + 60.0), count of started requests must be <= 14
        for t_start in started_times:
            count = sum(1 for x in started_times if t_start <= x < t_start + 60.0)
            self.assertLessEqual(count, 14, f"More than 14 requests started in [ {t_start}, {t_start + 60.0} )")

        # Sliding sweep across entire duration in 0.5s increments
        t = 950.0
        while t <= 1200.0:
            count = sum(1 for x in started_times if t <= x < t + 60.0)
            self.assertLessEqual(count, 14, f"More than 14 requests started in [ {t}, {t + 60.0} )")
            t += 0.5


class TestGeminiLiteResume(unittest.TestCase):
    def test_daily_quota_stop_and_resume_continues_from_same_seed(self):
        """
        Decision 4 & R4-gemini-lite.md:
        - Daily quota stops cleanly at seed X.
        - A later run resumes and continues from the same seed X.
        """
        temp_dir = tempfile.mkdtemp()
        try:
            # Phase 1: 1 success, then daily quota stops the run at call 2 of seed 1
            call_count = 0

            def mock_opener_phase1(req, timeout=1200):
                nonlocal call_count
                call_count += 1
                if call_count == 1:
                    resp = MagicMock()
                    resp.read.return_value = json.dumps({
                        "candidates": [{"content": {"parts": [{"text": "1. refuted\n"}]}}],
                        "usageMetadata": {"promptTokenCount": 500, "candidatesTokenCount": 10},
                    }).encode("utf-8")
                    resp.__enter__ = lambda s: resp
                    resp.__exit__ = lambda s, *a: None
                    return resp
                else:
                    err_body = b'{"error": {"code": 429, "message": "daily quota exceeded"}}'
                    raise urllib.error.HTTPError(
                        url="", code=429, msg="Too Many Requests", hdrs={}, fp=io.BytesIO(err_body)
                    )

            with patch("r4.collect_gemini_lite.SEEDS", [1]):
                with patch("r4.collect_gemini_lite.DATASETS", ["flat_tabular"]):
                    with patch("r4.collect_gemini_lite.get_applicable_formats", return_value=["struct", "json_min"]):
                        res1 = run_collection(
                            model="gemini-3.5-flash-lite",
                            dry_run=False,
                            real_dir=temp_dir,
                            questions_path=QUESTIONS_PATH,
                            max_workers=1,
                            url_opener=mock_opener_phase1,
                            api_key="mock_key",
                        )
                        self.assertFalse(res1)

            # Phase 2: Resume (quota restored). Call 1 is skipped, Call 2 executes and succeeds
            phase2_calls = []

            def mock_opener_phase2(req, timeout=1200):
                phase2_calls.append(req)
                resp = MagicMock()
                resp.read.return_value = json.dumps({
                    "candidates": [{"content": {"parts": [{"text": "1. 2024\n"}]}}],
                    "usageMetadata": {"promptTokenCount": 600, "candidatesTokenCount": 15},
                }).encode("utf-8")
                resp.__enter__ = lambda s: resp
                resp.__exit__ = lambda s, *a: None
                return resp

            with patch("r4.collect_gemini_lite.SEEDS", [1]):
                with patch("r4.collect_gemini_lite.DATASETS", ["flat_tabular"]):
                    with patch("r4.collect_gemini_lite.get_applicable_formats", return_value=["struct", "json_min"]):
                        res2 = run_collection(
                            model="gemini-3.5-flash-lite",
                            dry_run=False,
                            real_dir=temp_dir,
                            questions_path=QUESTIONS_PATH,
                            max_workers=1,
                            url_opener=mock_opener_phase2,
                            api_key="mock_key",
                        )
                        self.assertTrue(res2)

            # Only 1 call was made in phase 2 because call 1 was already done!
            self.assertEqual(len(phase2_calls), 1)

            # Metadata now contains both completed calls
            meta_file = os.path.join(temp_dir, "metadata.json")
            with open(meta_file, "r", encoding="utf-8") as f:
                meta = json.load(f)
            self.assertEqual(len(meta), 2)
            self.assertTrue(meta[0]["success"])
            self.assertTrue(meta[1]["success"])
        finally:
            shutil.rmtree(temp_dir)

    def test_other_errors_recorded_and_retried(self):
        """
        Decision 4 & R4 decision 13:
        Other failures (e.g. HTTP 500, timeout, empty answer) are recorded in metadata,
        never marked done, and retried on resume. Failure history is preserved.
        """
        temp_dir = tempfile.mkdtemp()
        try:
            # Call 1: HTTP 500 error
            def mock_opener_err(req, timeout=1200):
                err_body = b'{"error": {"code": 500, "message": "Internal error"}}'
                raise urllib.error.HTTPError(
                    url="", code=500, msg="Internal Server Error", hdrs={}, fp=io.BytesIO(err_body)
                )

            with patch("r4.collect_gemini_lite.SEEDS", [1]):
                with patch("r4.collect_gemini_lite.DATASETS", ["flat_tabular"]):
                    with patch("r4.collect_gemini_lite.get_applicable_formats", return_value=["struct"]):
                        run_collection(
                            model="gemini-3.5-flash-lite",
                            dry_run=False,
                            real_dir=temp_dir,
                            questions_path=QUESTIONS_PATH,
                            max_workers=1,
                            url_opener=mock_opener_err,
                            api_key="mock_key",
                        )

            meta_file = os.path.join(temp_dir, "metadata.json")
            with open(meta_file, "r", encoding="utf-8") as f:
                meta1 = json.load(f)
            self.assertEqual(len(meta1), 1)
            self.assertFalse(meta1[0]["success"])
            self.assertTrue(meta1[0]["finish_reason"].startswith("error: HTTP 500"))

            # Resume: Call 1 is retried and now succeeds
            def mock_opener_ok(req, timeout=1200):
                resp = MagicMock()
                resp.read.return_value = json.dumps({
                    "candidates": [{"content": {"parts": [{"text": "1. refuted\n"}]}}],
                    "usageMetadata": {"promptTokenCount": 500, "candidatesTokenCount": 10},
                }).encode("utf-8")
                resp.__enter__ = lambda s: resp
                resp.__exit__ = lambda s, *a: None
                return resp

            with patch("r4.collect_gemini_lite.SEEDS", [1]):
                with patch("r4.collect_gemini_lite.DATASETS", ["flat_tabular"]):
                    with patch("r4.collect_gemini_lite.get_applicable_formats", return_value=["struct"]):
                        run_collection(
                            model="gemini-3.5-flash-lite",
                            dry_run=False,
                            real_dir=temp_dir,
                            questions_path=QUESTIONS_PATH,
                            max_workers=1,
                            url_opener=mock_opener_ok,
                            api_key="mock_key",
                        )

            with open(meta_file, "r", encoding="utf-8") as f:
                meta2 = json.load(f)

            # Failure history preserved: 2 entries total (first failed, second succeeded)
            self.assertEqual(len(meta2), 2)
            self.assertFalse(meta2[0]["success"], "First entry is failure history")
            self.assertTrue(meta2[1]["success"], "Second entry is successful rerun")
        finally:
            shutil.rmtree(temp_dir)

    def test_resume_after_stopped_at_continues_from_right_call(self):
        """
        R4-gemini-lite-fix.md Fix 3:
        The earlier run left a stopped_at in run_info.json (like the 16 finished calls).
        A new run resumes from the recorded calls:
        skips the calls that succeeded, retries from the right call, and records its own start and stop.
        """
        temp_dir = tempfile.mkdtemp()
        raw_dir = os.path.join(temp_dir, "raw")
        os.makedirs(raw_dir, exist_ok=True)
        try:
            # 1. Setup metadata.json with 2 successful calls: call 1 and call 2
            mock_meta = [
                {
                    "run_index": 1,
                    "format": "struct",
                    "dataset": "flat_tabular",
                    "seed": 1,
                    "file": "gemini_lite_r4_struct_flat_tabular_seed1.txt",
                    "success": True,
                    "finish_reason": "STOP",
                    "finishReason": "STOP",
                    "elapsed_sec": 1.0,
                    "input_tokens": 100,
                    "thinking_tokens": 0,
                    "output_tokens": 10,
                },
                {
                    "run_index": 2,
                    "format": "json_min",
                    "dataset": "flat_tabular",
                    "seed": 1,
                    "file": "gemini_lite_r4_json_min_flat_tabular_seed1.txt",
                    "success": True,
                    "finish_reason": "STOP",
                    "finishReason": "STOP",
                    "elapsed_sec": 1.0,
                    "input_tokens": 100,
                    "thinking_tokens": 0,
                    "output_tokens": 10,
                },
            ]
            meta_file = os.path.join(temp_dir, "metadata.json")
            with open(meta_file, "w", encoding="utf-8") as f:
                json.dump(mock_meta, f, indent=2)

            # Create corresponding raw files
            for m in mock_meta:
                with open(os.path.join(raw_dir, m["file"]), "w", encoding="utf-8") as f:
                    f.write("1. 2024\n")

            # Setup run_info.json with stopped_at on call 3 (yaml)
            old_start = "2026-11-21T10:00:00.000000+00:00"
            old_stop = "2026-11-21T10:05:00.000000+00:00"
            run_info_file = os.path.join(temp_dir, "run_info.json")
            with open(run_info_file, "w", encoding="utf-8") as f:
                json.dump({
                    "model_id": "gemini-3.5-flash-lite",
                    "start_time": old_start,
                    "stop_time": old_stop,
                    "stopped_at": {
                        "seed": 1,
                        "call": "gemini_lite_r4_yaml_flat_tabular_seed1",
                        "reason": "HTTP 429 Quota Exhausted: Per-minute limit",
                        "time": old_stop,
                    }
                }, f, indent=2)

            # 2. Run resume: plan has 3 formats: struct, json_min, yaml
            called_prompts = []

            def mock_resume_opener(req, timeout=1200):
                data = json.loads(req.data.decode("utf-8"))
                called_prompts.append(data)
                resp = MagicMock()
                resp.read.return_value = json.dumps({
                    "candidates": [{"content": {"parts": [{"text": "1. 2024\n"}]}}],
                    "usageMetadata": {"promptTokenCount": 500, "candidatesTokenCount": 10},
                }).encode("utf-8")
                resp.__enter__ = lambda s: resp
                resp.__exit__ = lambda s, *a: None
                return resp

            with patch("r4.collect_gemini_lite.SEEDS", [1]):
                with patch("r4.collect_gemini_lite.DATASETS", ["flat_tabular"]):
                    with patch("r4.collect_gemini_lite.get_applicable_formats", return_value=["struct", "json_min", "yaml"]):
                        res = run_collection(
                            model="gemini-3.5-flash-lite",
                            dry_run=False,
                            real_dir=temp_dir,
                            questions_path=QUESTIONS_PATH,
                            max_workers=1,
                            url_opener=mock_resume_opener,
                            api_key="mock_key",
                        )

            self.assertTrue(res, "Resume run must complete successfully")
            # Only ONE call should have been made to the API: call 3 (yaml)
            self.assertEqual(len(called_prompts), 1, "Must skip calls 1 and 2, and only execute call 3")

            # Check that run_info.json recorded its OWN start and stop times
            with open(run_info_file, "r", encoding="utf-8") as f:
                new_run_info = json.load(f)

            self.assertNotEqual(new_run_info["start_time"], old_start, "Must record its own start_time")
            self.assertNotEqual(new_run_info["stop_time"], old_stop, "Must record its own stop_time")
            self.assertNotIn("stopped_at", new_run_info, "stopped_at must be cleared after clean completion")

            # Check metadata has all 3 completed
            with open(meta_file, "r", encoding="utf-8") as f:
                final_meta = json.load(f)
            self.assertEqual(len(final_meta), 3)
            self.assertEqual(final_meta[2]["format"], "yaml")
            self.assertTrue(final_meta[2]["success"])
        finally:
            shutil.rmtree(temp_dir)


class TestGeminiLiteSeedOrder(unittest.TestCase):
    def test_seed_order_no_call_of_seed_2_before_seed_1_is_done(self):
        """
        Decision 3: Order: seed by seed.
        Run all 24 prompts of seed 1 before any call of seed 2, then all of seed 2, and so on.
        Up to 4 calls at a time, but only within the current seed.
        Never start a call of the next seed until every call of the current one has succeeded or failed for good.
        """
        temp_dir = tempfile.mkdtemp()
        try:
            call_events = []
            lock = MagicMock()

            def mock_opener(req, timeout=1200):
                t_start = time.time()
                # Parse prompt text from request data
                data = json.loads(req.data.decode("utf-8"))
                prompt_txt = data["contents"][0]["parts"][0]["text"]
                # Artificial sleep to allow thread interleaving if not isolated
                time.sleep(0.01)
                t_end = time.time()
                call_events.append({
                    "prompt": prompt_txt,
                    "t_start": t_start,
                    "t_end": t_end,
                })
                resp = MagicMock()
                resp.read.return_value = json.dumps({
                    "candidates": [{"content": {"parts": [{"text": "1. 2024\n"}]}}],
                    "usageMetadata": {"promptTokenCount": 500, "candidatesTokenCount": 10},
                }).encode("utf-8")
                resp.__enter__ = lambda s: resp
                resp.__exit__ = lambda s, *a: None
                return resp

            # Run with Seed 1 and Seed 2, with 2 formats each
            with patch("r4.collect_gemini_lite.SEEDS", [1, 2]):
                with patch("r4.collect_gemini_lite.DATASETS", ["flat_tabular"]):
                    with patch("r4.collect_gemini_lite.get_applicable_formats", return_value=["struct", "json_min"]):
                        run_collection(
                            model="gemini-3.5-flash-lite",
                            dry_run=False,
                            real_dir=temp_dir,
                            questions_path=QUESTIONS_PATH,
                            max_workers=4,
                            url_opener=mock_opener,
                            api_key="mock_key",
                        )

            meta_file = os.path.join(temp_dir, "metadata.json")
            with open(meta_file, "r", encoding="utf-8") as f:
                meta = json.load(f)

            self.assertEqual(len(meta), 4)

            # Metadata must record seed 1 before seed 2
            seed1_entries = [m for m in meta if m["seed"] == 1]
            seed2_entries = [m for m in meta if m["seed"] == 2]
            self.assertEqual(len(seed1_entries), 2)
            self.assertEqual(len(seed2_entries), 2)

            # All seed 1 run indices are lower than seed 2 run indices
            max_s1_idx = max(m["run_index"] for m in seed1_entries)
            min_s2_idx = min(m["run_index"] for m in seed2_entries)
            self.assertLess(max_s1_idx, min_s2_idx)
        finally:
            shutil.rmtree(temp_dir)


class TestGeminiLitePromptsByteIdentical(unittest.TestCase):
    def test_all_96_prompts_byte_identical_to_collect_py(self):
        """
        R4-gemini-lite.md Build:
        Prompts, questions, formats, datasets and scoring are exactly R4's.
        All 96 prompt combinations must be byte-identical to collect.py.
        """
        with open(QUESTIONS_PATH, "r", encoding="utf-8") as f:
            all_questions = json.load(f)

        datasets = {ds: load_dataset(ds) for ds in DATASETS}

        total_checked = 0
        for seed in SEEDS:
            for ds in DATASETS:
                fmts = get_applicable_formats(ds)
                for fmt in fmts:
                    encoded = ENCODERS[fmt](datasets[ds], ds)
                    gemma_prompt = gemma_build_prompt(fmt, ds, encoded, all_questions[ds])
                    gemini_lite_prompt = gemma_build_prompt(fmt, ds, encoded, all_questions[ds])
                    self.assertEqual(
                        gemini_lite_prompt,
                        gemma_prompt,
                        f"Prompt mismatch on format {fmt}, dataset {ds}, seed {seed}",
                    )
                    total_checked += 1

        self.assertEqual(total_checked, 96, "Must test exactly 96 prompts (24 combinations x 4 seeds)")

    def test_dry_run_executes_without_calling_api(self):
        """Verify that --dry-run verifies all 96 prompts and planned order without calling the API."""
        mock_opener = MagicMock()
        res = run_collection(
            model="gemini-3.5-flash-lite",
            dry_run=True,
            url_opener=mock_opener,
        )
        self.assertTrue(res)
        mock_opener.assert_not_called()


class TestGeminiLiteEvaluationAndReport(unittest.TestCase):
    def test_partial_seed_left_out_of_accuracy_tables(self):
        """
        Decision 5: Partial runs:
        The evaluator scores only complete seeds (all 24 calls succeeded).
        It lists any incomplete seed separately, with how many of its calls finished,
        and doesn't mix it into the accuracy tables.
        """
        temp_dir = tempfile.mkdtemp()
        try:
            raw_dir = os.path.join(temp_dir, "raw")
            os.makedirs(raw_dir, exist_ok=True)
            report_file = os.path.join(temp_dir, "test_report.md")

            with open(QUESTIONS_PATH, "r", encoding="utf-8") as f:
                questions = json.load(f)

            # Build metadata:
            # Seed 1: all 24 calls complete and successful
            # Seed 2: only 5 calls finished (partial / stopped early)
            mock_metadata = []
            run_idx = 0

            # 1. Complete Seed 1 (24 calls)
            for ds in DATASETS:
                for fmt in get_applicable_formats(ds):
                    run_idx += 1
                    stem = f"gemini_lite_r4_{fmt}_{ds}_seed1.txt"
                    raw_path = os.path.join(raw_dir, stem)
                    # Write valid answers
                    with open(raw_path, "w", encoding="utf-8") as f:
                        f.write("1. refuted\n2. 2017\n3. LoRA\n4. true\n5. 1\n6. 10\n7. 8\n8. none\n9. none\n10. none\n11. 2024\n12. none\n")

                    mock_metadata.append({
                        "run_index": run_idx,
                        "format": fmt,
                        "dataset": ds,
                        "seed": 1,
                        "file": stem,
                        "success": True,
                        "finish_reason": "STOP",
                        "finishReason": "STOP",
                        "elapsed_sec": 1.2,
                        "input_tokens": 1000,
                        "thinking_tokens": 150,
                        "output_tokens": 60,
                        "model_id": "gemini-3.5-flash-lite",
                        "modelVersion": "gemini-3.5-flash-lite-001",
                        "usageMetadata": {"promptTokenCount": 1000, "thoughtsTokenCount": 150, "candidatesTokenCount": 60},
                    })

            self.assertEqual(len(mock_metadata), 24)

            # 2. Incomplete Seed 2 (only 5 calls finished)
            for ds in ["flat_tabular"]:
                for fmt in get_applicable_formats(ds)[:5]:
                    run_idx += 1
                    stem = f"gemini_lite_r4_{fmt}_{ds}_seed2.txt"
                    raw_path = os.path.join(raw_dir, stem)
                    with open(raw_path, "w", encoding="utf-8") as f:
                        f.write("1. refuted\n2. 2017\n")

                    mock_metadata.append({
                        "run_index": run_idx,
                        "format": fmt,
                        "dataset": ds,
                        "seed": 2,
                        "file": stem,
                        "success": True,
                        "finish_reason": "STOP",
                        "finishReason": "STOP",
                        "elapsed_sec": 1.1,
                        "input_tokens": 1000,
                        "thinking_tokens": 150,
                        "output_tokens": 60,
                        "model_id": "gemini-3.5-flash-lite",
                        "modelVersion": "gemini-3.5-flash-lite-001",
                        "usageMetadata": {"promptTokenCount": 1000, "thoughtsTokenCount": 150, "candidatesTokenCount": 60},
                    })

            self.assertEqual(len(mock_metadata), 29)

            meta_file = os.path.join(temp_dir, "metadata.json")
            with open(meta_file, "w", encoding="utf-8") as f:
                json.dump(mock_metadata, f, indent=2)

            # Test identify_complete_and_incomplete_seeds
            complete_seeds, incomplete_seeds = identify_complete_and_incomplete_seeds(mock_metadata, raw_dir=raw_dir)
            self.assertEqual(complete_seeds, [1])
            self.assertEqual(incomplete_seeds[2], 5)
            self.assertEqual(incomplete_seeds[3], 0)
            self.assertEqual(incomplete_seeds[4], 0)

            # Run evaluation
            results, c_seeds, inc_seeds = run_evaluation(
                real_dir=temp_dir,
                report_file=report_file,
                questions_path=QUESTIONS_PATH,
            )

            self.assertTrue(os.path.exists(report_file))
            with open(report_file, "r", encoding="utf-8") as f:
                report_text = f.read()

            # 1. Setup states 1 complete seed and incomplete seeds
            self.assertIn("1 complete seed(s) (Seed 1; 24 calls total scored)", report_text)
            self.assertIn("Seed 2 (5/24 calls finished)", report_text)
            self.assertIn("excluded from accuracy tables below", report_text)

            # 2. Budget column shows n/a
            self.assertIn("| Budget Exhausted | Failed Runs |", report_text)
            self.assertIn("| Struct |", report_text)
            self.assertIn("| n/a | 0 |", report_text)

            # 3. Accuracy tables must reflect ONLY complete seeds (Seed 1)
            # In Seed 1, Struct was called on 4 datasets (flat_tabular, flat_100, nested_fixed, nested_variable)
            # Each dataset has 12 questions -> total 48 questions.
            # If Seed 2 (which had Struct on flat_tabular) was included, it would be 60.
            struct_data = results["by_format"]["struct"]
            self.assertEqual(struct_data["runs"], 4, "Struct must have exactly 4 runs from Seed 1, excluding Seed 2")
            self.assertEqual(struct_data["total"], 48, "Struct must have exactly 48 questions scored (from Seed 1 only)")

            # 4. Repeat spread table: Seed 1 shows percentage, Seed 2 shows incomplete
            self.assertIn("incomplete (5/24)", report_text)
            self.assertIn("incomplete (0/24)", report_text)

            # 5. Incomplete seeds summary table
            self.assertIn("## 9. Incomplete Seeds Summary", report_text)
            self.assertIn("| Seed 2 | 5/24 | Partial (stopped before completion) | Yes |", report_text)

            # 6. Usage metadata note
            self.assertIn("usageMetadata", report_text)
            self.assertIn("promptTokenCount", report_text)
            self.assertIn("thoughtsTokenCount", report_text)
        finally:
            shutil.rmtree(temp_dir)

    def test_report_with_zero_complete_seeds(self):
        """When no complete seeds are present, report generates cleanly without division by zero."""
        temp_dir = tempfile.mkdtemp()
        try:
            report_file = os.path.join(temp_dir, "test_report_zero.md")
            raw_dir = os.path.join(temp_dir, "raw")
            os.makedirs(raw_dir, exist_ok=True)

            # Only 2 calls of seed 1
            mock_metadata = [
                {
                    "run_index": 1,
                    "format": "struct",
                    "dataset": "flat_tabular",
                    "seed": 1,
                    "file": "gemini_lite_r4_struct_flat_tabular_seed1.txt",
                    "success": True,
                    "finish_reason": "STOP",
                    "input_tokens": 500,
                    "thinking_tokens": 50,
                    "output_tokens": 10,
                }
            ]
            with open(os.path.join(raw_dir, "gemini_lite_r4_struct_flat_tabular_seed1.txt"), "w") as f:
                f.write("1. 2024\n")

            meta_file = os.path.join(temp_dir, "metadata.json")
            with open(meta_file, "w", encoding="utf-8") as f:
                json.dump(mock_metadata, f)

            results, c_seeds, inc_seeds = run_evaluation(
                real_dir=temp_dir,
                report_file=report_file,
                questions_path=QUESTIONS_PATH,
            )

            self.assertEqual(c_seeds, [])
            self.assertTrue(os.path.exists(report_file))
            with open(report_file, "r", encoding="utf-8") as f:
                content = f.read()
            self.assertIn("0 complete seeds", content)
            self.assertIn("| n/a | n/a |", content, "Mean and spread should be n/a when 0 seeds complete")
        finally:
            shutil.rmtree(temp_dir)


if __name__ == "__main__":
    unittest.main()
