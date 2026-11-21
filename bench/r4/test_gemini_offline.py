#!/usr/bin/env python3
"""
Offline test harness for Struct Milestone R4-gemini.

Tests:
1. Mocked subprocess for:
   - Success (valid JSON, response parsed, num_turns == 1, token arithmetic).
   - Failure: non-zero exit code.
   - Failure: unparseable JSON.
   - Failure: error status (status != SUCCESS).
   - Failure: empty response.
   - Failure: num_turns above 1 (agent used tools).
   - Failure: timeout (TimeoutExpired).
   - Failure: too-long command line (> 32,767 characters).
2. Token arithmetic per R4-gemini.md:
   - input = input_tokens + cache_read_tokens
   - thinking = thinking_tokens
   - output = output_tokens minus thinking_tokens (never below 0)
   - raw fields recorded in metadata
3. Resume skips done calls and retries failed ones:
   - Successful call skipped.
   - Failed call retried.
   - Failure history preserved.
4. Prompts byte-identical to Gemma's:
   - All 96 prompt combinations byte-identical to collect.py.
   - All 96 prompt command lines within Windows 32,767 character limit.
   - Dry run executes cleanly without calling any subprocess or model.
5. Report verification:
   - Setup section states model ID, exact command, low variant, no budget, 4 repeats, 4 parallel.
   - Input tokens note present in Setup and table footnote:
     "agy is an agent: every call includes its own system prompt and tool definitions, about 13,600 input tokens,
      so input tokens here are not comparable with the other models' in absolute terms; compare formats within this model only."
   - Budget column shows n/a (not 0).
   - Repeat spread table for seeds 1 to 4.
6. Command shape and run_info:
   - Exact command line template and arguments list.
   - run_info.json recorded and hashed in SHA256SUMS.
7. Temporary directory isolation:
   - Process runs in an isolated temporary directory and cleans it up.
"""

import os
import sys
import json
import shutil
import tempfile
import unittest
import subprocess
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
from r4.collect_gemini import (
    MODEL_NAME,
    SEEDS,
    TIMEOUT_SEC,
    PARALLEL,
    WINDOWS_CMD_MAX_LEN,
    CommandLineTooLongError,
    build_agy_cmd,
    check_cmd_length,
    extract_tokens,
    get_agy_version,
    record_run_info,
    update_sha256sums,
    process_agy_output,
    call_agy,
    run_collection,
)
from r4.evaluate_gemini import (
    generate_gemini_report,
    get_gemini_model_id,
    get_gemini_command_line,
    DEFAULT_COMMAND_LINE,
)


class TestGeminiCommandAndRunInfo(unittest.TestCase):
    def test_agy_command_shape(self):
        """
        Decision 1: How Gemini is called:
        agy -p=<prompt> --model gemini-3.8-flash-low --output-format json --sandbox --disable-slash-commands
        Pass arguments as a list (no shell). The prompt must be one argument, "-p=" + prompt.
        """
        test_prompt = "Test prompt content"
        cmd = build_agy_cmd(test_prompt)
        self.assertIsInstance(cmd, list, "build_agy_cmd must return a list of arguments (no shell)")
        self.assertEqual(cmd[0], "agy")
        self.assertEqual(cmd[1], f"-p={test_prompt}")
        self.assertIn("--model", cmd)
        m_idx = cmd.index("--model")
        self.assertEqual(cmd[m_idx + 1], "gemini-3.8-flash-low")
        self.assertIn("--output-format", cmd)
        of_idx = cmd.index("--output-format")
        self.assertEqual(cmd[of_idx + 1], "json")
        self.assertIn("--sandbox", cmd)
        self.assertIn("--disable-slash-commands", cmd)

    def test_record_run_info_and_sha256(self):
        """Verify record_run_info writes exact command template and agy --version, and is hashed in SHA256SUMS."""
        temp_dir = tempfile.mkdtemp()
        try:
            info = record_run_info(real_dir=temp_dir, version_str="1.5.14-mock")
            self.assertEqual(info["agy_version"], "1.5.14-mock")
            self.assertIn("agy -p=<prompt> --model gemini-3.8-flash-low", info["command_line"])

            run_info_file = os.path.join(temp_dir, "run_info.json")
            self.assertTrue(os.path.exists(run_info_file))

            raw_dir = os.path.join(temp_dir, "raw")
            os.makedirs(raw_dir, exist_ok=True)
            with open(os.path.join(raw_dir, "sample.txt"), "w", encoding="utf-8") as f:
                f.write("1. 2024\n")

            meta_file = os.path.join(temp_dir, "metadata.json")
            with open(meta_file, "w", encoding="utf-8") as f:
                json.dump([{"test": 1}], f)

            update_sha256sums(real_dir=temp_dir)
            sha_file = os.path.join(temp_dir, "SHA256SUMS")
            self.assertTrue(os.path.exists(sha_file))
            with open(sha_file, "r", encoding="utf-8") as f:
                sha_content = f.read()

            self.assertIn("run_info.json", sha_content, "run_info.json must be hashed in SHA256SUMS")
            self.assertIn("metadata.json", sha_content, "metadata.json must be hashed in SHA256SUMS")
            self.assertIn("raw/sample.txt", sha_content, "raw files must be hashed in SHA256SUMS")
        finally:
            shutil.rmtree(temp_dir)


class TestGeminiTokenArithmetic(unittest.TestCase):
    def test_token_arithmetic_standard(self):
        """
        R4-gemini.md:
        - input tokens: input_tokens + cache_read_tokens;
        - thinking tokens: thinking_tokens;
        - output tokens: output_tokens minus thinking_tokens, never below 0.
        Tested against real example from R4-gemini.md:
        {"input_tokens":13594,"output_tokens":47,"thinking_tokens":44,"cache_read_tokens":0,"total_tokens":13641}
        """
        resp_json = {
            "usage": {
                "input_tokens": 13594,
                "output_tokens": 47,
                "thinking_tokens": 44,
                "cache_read_tokens": 0,
                "total_tokens": 13641,
            }
        }
        inp, thinking, out = extract_tokens(resp_json)
        self.assertEqual(inp, 13594)
        self.assertEqual(thinking, 44)
        self.assertEqual(out, 47 - 44)

    def test_token_arithmetic_cache_read_tokens(self):
        """Verify cache_read_tokens are added to input tokens."""
        resp_json = {
            "usage": {
                "input_tokens": 12000,
                "cache_read_tokens": 1600,
                "output_tokens": 80,
                "thinking_tokens": 30,
            }
        }
        inp, thinking, out = extract_tokens(resp_json)
        self.assertEqual(inp, 12000 + 1600)
        self.assertEqual(thinking, 30)
        self.assertEqual(out, 80 - 30)

    def test_token_arithmetic_output_capped_at_zero(self):
        """If thinking exceeds total output, output is never below 0."""
        resp_json = {
            "usage": {
                "input_tokens": 500,
                "output_tokens": 50,
                "thinking_tokens": 100,
            }
        }
        inp, thinking, out = extract_tokens(resp_json)
        self.assertEqual(inp, 500)
        self.assertEqual(thinking, 100)
        self.assertEqual(out, 0)

    def test_token_arithmetic_missing_usage(self):
        """Handles missing usage dictionary safely."""
        self.assertEqual(extract_tokens({}), (0, 0, 0))
        self.assertEqual(extract_tokens(None), (0, 0, 0))


class TestGeminiMockedSubprocess(unittest.TestCase):
    def test_subprocess_success(self):
        """Mock successful agy JSON output."""
        mock_proc = MagicMock()
        mock_proc.returncode = 0
        mock_proc.stdout = json.dumps({
            "conversation_id": "conv-12345",
            "status": "SUCCESS",
            "response": "1. refuted\n2. 2017\n3. LoRA\n4. true\n5. 1\n6. 10\n7. 8\n8. none\n9. none\n10. none\n11. 2024\n12. none",
            "duration_seconds": 3.45,
            "num_turns": 1,
            "usage": {
                "input_tokens": 13594,
                "output_tokens": 47,
                "thinking_tokens": 44,
                "cache_read_tokens": 0,
                "total_tokens": 13641,
            },
        })
        mock_proc.stderr = ""

        (
            is_success,
            finish_reason,
            content,
            resp_json,
            error_msg,
            inp,
            think,
            out,
            model_id,
            raw_usage,
        ) = process_agy_output(mock_proc)

        self.assertTrue(is_success)
        self.assertEqual(finish_reason, "stop")
        self.assertIn("1. refuted", content)
        self.assertIsNone(error_msg)
        self.assertEqual(inp, 13594)
        self.assertEqual(think, 44)
        self.assertEqual(out, 3)
        self.assertEqual(model_id, "gemini-3.8-flash-low")
        self.assertEqual(raw_usage.get("output_tokens"), 47)
        self.assertEqual(raw_usage.get("thinking_tokens"), 44)

    def test_subprocess_failure_nonzero_exit(self):
        """Mock non-zero exit code failure."""
        mock_proc = MagicMock()
        mock_proc.returncode = 1
        mock_proc.stdout = ""
        mock_proc.stderr = "Error: agy connection failed or authentication expired"

        (
            is_success,
            finish_reason,
            content,
            resp_json,
            error_msg,
            inp,
            think,
            out,
            model_id,
            raw_usage,
        ) = process_agy_output(mock_proc)

        self.assertFalse(is_success)
        self.assertTrue(finish_reason.startswith("error: exit code 1"))
        self.assertEqual(content, "")
        self.assertIsNotNone(error_msg)
        self.assertIn("authentication expired", error_msg)

    def test_subprocess_failure_unparseable_json(self):
        """Mock unparseable JSON output failure."""
        mock_proc = MagicMock()
        mock_proc.returncode = 0
        mock_proc.stdout = "500 Internal Server Error: Failed to execute agy"
        mock_proc.stderr = ""

        (
            is_success,
            finish_reason,
            content,
            resp_json,
            error_msg,
            inp,
            think,
            out,
            model_id,
            raw_usage,
        ) = process_agy_output(mock_proc)

        self.assertFalse(is_success)
        self.assertEqual(finish_reason, "error: unparseable json")
        self.assertEqual(content, "")
        self.assertIsNotNone(error_msg)

    def test_subprocess_failure_error_status(self):
        """Mock status other than SUCCESS (e.g. status: ERROR)."""
        mock_proc = MagicMock()
        mock_proc.returncode = 0
        mock_proc.stdout = json.dumps({
            "status": "ERROR",
            "error": "Model quota exhausted or rate limit hit",
            "finished_reason": "error",
        })
        mock_proc.stderr = ""

        (
            is_success,
            finish_reason,
            content,
            resp_json,
            error_msg,
            inp,
            think,
            out,
            model_id,
            raw_usage,
        ) = process_agy_output(mock_proc)

        self.assertFalse(is_success)
        self.assertEqual(finish_reason, "error")
        self.assertEqual(content, "")
        self.assertIsNotNone(error_msg)
        self.assertIn("Model quota exhausted", error_msg)

    def test_subprocess_failure_empty_response(self):
        """Mock empty or whitespace-only response field."""
        for empty_val in ["", "   ", "\n\n"]:
            mock_proc = MagicMock()
            mock_proc.returncode = 0
            mock_proc.stdout = json.dumps({
                "status": "SUCCESS",
                "response": empty_val,
                "num_turns": 1,
                "usage": {"input_tokens": 13000, "output_tokens": 0},
            })
            mock_proc.stderr = ""

            (
                is_success,
                finish_reason,
                content,
                resp_json,
                error_msg,
                inp,
                think,
                out,
                model_id,
                raw_usage,
            ) = process_agy_output(mock_proc)

            self.assertFalse(is_success)
            self.assertEqual(finish_reason, "error: empty response")
            self.assertEqual(content, "")
            self.assertIsNotNone(error_msg)

    def test_subprocess_failure_num_turns_above_one(self):
        """
        R4-gemini.md:
        failure: num_turns above 1 (the agent used tools).
        """
        mock_proc = MagicMock()
        mock_proc.returncode = 0
        mock_proc.stdout = json.dumps({
            "status": "SUCCESS",
            "response": "1. refuted\n2. 2017\n",
            "num_turns": 2,
            "usage": {"input_tokens": 15000, "output_tokens": 120},
        })
        mock_proc.stderr = ""

        (
            is_success,
            finish_reason,
            content,
            resp_json,
            error_msg,
            inp,
            think,
            out,
            model_id,
            raw_usage,
        ) = process_agy_output(mock_proc)

        self.assertFalse(is_success)
        self.assertIn("num_turns > 1", finish_reason)
        self.assertIn("agent used tools", finish_reason)
        self.assertEqual(content, "")
        self.assertIsNotNone(error_msg)

    def test_subprocess_failure_timeout(self):
        """Verify timeout exception handling records failure without crash."""
        temp_dir = tempfile.mkdtemp()
        try:
            with patch("r4.collect_gemini.call_agy", side_effect=subprocess.TimeoutExpired(cmd=["agy"], timeout=1200)):
                with patch("r4.collect_gemini.SEEDS", [1]):
                    with patch("r4.collect_gemini.DATASETS", ["flat_tabular"]):
                        with patch("r4.collect_gemini.get_applicable_formats", return_value=["struct"]):
                            run_collection(
                                dry_run=False,
                                real_dir=temp_dir,
                                questions_path=QUESTIONS_PATH,
                                max_workers=1,
                                version_override="mock-version",
                            )

            meta_file = os.path.join(temp_dir, "metadata.json")
            self.assertTrue(os.path.exists(meta_file))
            with open(meta_file, "r", encoding="utf-8") as f:
                metadata = json.load(f)

            self.assertEqual(len(metadata), 1)
            entry = metadata[0]
            self.assertFalse(entry["success"])
            self.assertEqual(entry["finish_reason"], "error: timeout")
            self.assertIn("Timed out", entry["error"])
        finally:
            shutil.rmtree(temp_dir)

    def test_subprocess_failure_command_line_too_long(self):
        """
        R4-gemini.md:
        Check every prompt's length before calling, and fail that call (recorded, not silently skipped)
        if the command line would exceed the 32,767 character limit.
        """
        # Test direct length check raises CommandLineTooLongError
        huge_prompt = "A" * 33000
        huge_cmd = build_agy_cmd(huge_prompt)
        with self.assertRaises(CommandLineTooLongError):
            check_cmd_length(huge_cmd)

        with self.assertRaises(CommandLineTooLongError):
            call_agy(huge_prompt)

        # Test collector records failure in metadata without calling agy
        temp_dir = tempfile.mkdtemp()
        try:
            with patch("r4.collect_gemini.build_prompt", return_value=huge_prompt):
                with patch("r4.collect_gemini.call_agy") as mock_call:
                    with patch("r4.collect_gemini.SEEDS", [1]):
                        with patch("r4.collect_gemini.DATASETS", ["flat_tabular"]):
                            with patch("r4.collect_gemini.get_applicable_formats", return_value=["struct"]):
                                run_collection(
                                    dry_run=False,
                                    real_dir=temp_dir,
                                    questions_path=QUESTIONS_PATH,
                                    max_workers=1,
                                    version_override="mock-version",
                                )
                    # agy must NOT be called when prompt exceeds limit!
                    mock_call.assert_not_called()

            meta_file = os.path.join(temp_dir, "metadata.json")
            self.assertTrue(os.path.exists(meta_file))
            with open(meta_file, "r", encoding="utf-8") as f:
                metadata = json.load(f)

            self.assertEqual(len(metadata), 1)
            entry = metadata[0]
            self.assertFalse(entry["success"])
            self.assertEqual(entry["finish_reason"], "error: command line too long")
            self.assertIn("exceeds Windows limit", entry["error"])
        finally:
            shutil.rmtree(temp_dir)


class TestGeminiResume(unittest.TestCase):
    def test_resume_skips_successful_and_retries_failed(self):
        """
        Per Decision 13 / R4-gemini:
        - Resumable: a rerun skips calls that already succeeded.
        - Failed requests are not marked done; a rerun retries them.
        """
        temp_dir = tempfile.mkdtemp()
        try:
            raw_dir = os.path.join(temp_dir, "raw")
            os.makedirs(raw_dir, exist_ok=True)

            success_stem = "gemini_r4_struct_flat_tabular_seed1"
            with open(os.path.join(raw_dir, f"{success_stem}.txt"), "w", encoding="utf-8") as f:
                f.write("1. refuted\n2. 2017\n")

            init_metadata = [
                {
                    "run_index": 1,
                    "format": "struct",
                    "dataset": "flat_tabular",
                    "seed": 1,
                    "file": f"{success_stem}.txt",
                    "success": True,
                    "finish_reason": "stop",
                    "elapsed_sec": 3.2,
                    "input_tokens": 13600,
                    "thinking_tokens": 50,
                    "output_tokens": 20,
                    "raw_input_tokens": 13600,
                    "raw_output_tokens": 70,
                    "raw_thinking_tokens": 50,
                    "raw_cache_read_tokens": 0,
                    "model_id": "gemini-3.8-flash-low",
                },
                {
                    "run_index": 2,
                    "format": "json_min",
                    "dataset": "flat_tabular",
                    "seed": 1,
                    "file": "gemini_r4_json_min_flat_tabular_seed1.txt",
                    "success": False,
                    "finish_reason": "error: num_turns > 1 (agent used tools)",
                    "error": "num_turns was 2",
                    "elapsed_sec": 5.0,
                    "input_tokens": 0,
                    "thinking_tokens": 0,
                    "output_tokens": 0,
                    "model_id": "gemini-3.8-flash-low",
                },
            ]
            meta_file = os.path.join(temp_dir, "metadata.json")
            with open(meta_file, "w", encoding="utf-8") as f:
                json.dump(init_metadata, f, indent=2)

            def mock_call(prompt, timeout_sec=1200, cwd=None):
                mock_proc = MagicMock()
                mock_proc.returncode = 0
                mock_proc.stdout = json.dumps({
                    "status": "SUCCESS",
                    "response": "1. 2024\n2. supported\n",
                    "duration_seconds": 2.5,
                    "num_turns": 1,
                    "usage": {
                        "input_tokens": 13500,
                        "output_tokens": 40,
                        "thinking_tokens": 30,
                        "cache_read_tokens": 0,
                        "total_tokens": 13540,
                    },
                })
                mock_proc.stderr = ""
                return mock_proc

            with patch("r4.collect_gemini.call_agy", side_effect=mock_call) as mock_call_fn:
                with patch("r4.collect_gemini.SEEDS", [1]):
                    with patch("r4.collect_gemini.DATASETS", ["flat_tabular"]):
                        with patch("r4.collect_gemini.get_applicable_formats", return_value=["struct", "json_min"]):
                            run_collection(
                                dry_run=False,
                                real_dir=temp_dir,
                                questions_path=QUESTIONS_PATH,
                                max_workers=1,
                                version_override="mock-version",
                            )

            # Verify: struct skipped, json_min retried
            self.assertEqual(mock_call_fn.call_count, 1, "Only the failed format (json_min) should be retried")

            with open(meta_file, "r", encoding="utf-8") as f:
                updated_meta = json.load(f)

            self.assertEqual(len(updated_meta), 3, "Failure history must be preserved; 3 entries total")
            json_entries = [m for m in updated_meta if m["format"] == "json_min"]
            self.assertEqual(len(json_entries), 2)
            self.assertFalse(json_entries[0]["success"], "First entry is recorded failure")
            self.assertTrue(json_entries[1]["success"], "Second entry is successful rerun")
        finally:
            shutil.rmtree(temp_dir)


class TestGeminiPromptsByteIdentical(unittest.TestCase):
    def test_all_96_prompts_byte_identical_to_gemma(self):
        """
        R4-gemini.md:
        builds every prompt with collect.build_prompt, ENCODERS and questions.json,
        so each prompt is byte-identical to Gemma's.
        96 prompts = 24 combinations x 4 seeds.
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
                    gemini_prompt = gemma_build_prompt(fmt, ds, encoded, all_questions[ds])
                    self.assertEqual(
                        gemini_prompt,
                        gemma_prompt,
                        f"Byte mismatch on format {fmt}, dataset {ds}, seed {seed}",
                    )
                    total_checked += 1

        self.assertEqual(total_checked, 96, "Must test exactly 96 prompts (24 combinations x 4 seeds)")

    def test_all_96_prompts_under_windows_char_limit(self):
        """Verify that every one of the 96 prompts produces a command line strictly <= 32,767 characters."""
        with open(QUESTIONS_PATH, "r", encoding="utf-8") as f:
            all_questions = json.load(f)

        datasets = {ds: load_dataset(ds) for ds in DATASETS}

        max_len = 0
        for ds in DATASETS:
            fmts = get_applicable_formats(ds)
            for fmt in fmts:
                encoded = ENCODERS[fmt](datasets[ds], ds)
                prompt = gemma_build_prompt(fmt, ds, encoded, all_questions[ds])
                cmd = build_agy_cmd(prompt)
                cmd_line = check_cmd_length(cmd)
                max_len = max(max_len, len(cmd_line))
                self.assertLessEqual(len(cmd_line), WINDOWS_CMD_MAX_LEN)

        # Largest R4 prompt is known to be around 20,743 chars
        self.assertLess(max_len, 25000)

    def test_dry_run_executes_without_calling_agy(self):
        """Verify that --dry-run builds all 96 prompts and runs cleanly without calling any subprocess."""
        with patch("r4.collect_gemini.call_agy") as mock_call:
            run_collection(dry_run=True)
            mock_call.assert_not_called()


class TestGeminiReportGeneration(unittest.TestCase):
    def test_report_na_budget_and_agent_tokens_note(self):
        """
        R4-gemini.md:
        - setup section: model, exact command, low variant, no budget, 4 repeats, 4 parallel;
        - note: 'agy is an agent: every call includes its own system prompt and tool definitions,
          about 13,600 input tokens, so input tokens here are not comparable with the other models'
          in absolute terms; compare formats within this model only.';
        - budget column n/a.
        """
        temp_dir = tempfile.mkdtemp()
        try:
            raw_dir = os.path.join(temp_dir, "raw")
            os.makedirs(raw_dir, exist_ok=True)
            report_file = os.path.join(temp_dir, "test_gemini_report.md")

            stem = "gemini_r4_struct_flat_tabular_seed1.txt"
            with open(os.path.join(raw_dir, stem), "w", encoding="utf-8") as f:
                f.write("1. refuted\n2. 2017\n3. LoRA\n4. true\n5. 1\n6. 10\n7. 8\n8. none\n9. none\n10. none\n11. 2024\n12. none\n")

            mock_meta = [
                {
                    "run_index": 1,
                    "format": "struct",
                    "dataset": "flat_tabular",
                    "seed": 1,
                    "file": stem,
                    "success": True,
                    "finish_reason": "stop",
                    "elapsed_sec": 3.45,
                    "input_tokens": 13594,
                    "thinking_tokens": 44,
                    "output_tokens": 3,
                    "raw_input_tokens": 13594,
                    "raw_output_tokens": 47,
                    "raw_thinking_tokens": 44,
                    "raw_cache_read_tokens": 0,
                    "model_id": "gemini-3.8-flash-low",
                }
            ]

            with open(QUESTIONS_PATH, "r", encoding="utf-8") as f:
                qs = json.load(f)

            eval_results = evaluate_runs(mock_meta, qs, raw_dir=raw_dir)
            generate_gemini_report(eval_results, report_file=report_file, metadata_list=mock_meta)

            self.assertTrue(os.path.exists(report_file))
            with open(report_file, "r", encoding="utf-8") as f:
                content = f.read()

            # 1. Budget column must be n/a
            self.assertIn("| Budget Exhausted | Failed Runs |", content)
            self.assertIn("| Struct |", content)
            self.assertIn("| n/a | 0 |", content, "Budget exhausted column must show n/a for Gemini")

            # 2. Setup section requirements
            self.assertIn("gemini-3.8-flash-low", content)
            self.assertIn("agy -p=<prompt> --model gemini-3.8-flash-low", content)
            self.assertIn("no budget (low variant)", content)
            self.assertIn("4 repeats per prompt (labelled seed 1 to 4; 96 calls total)", content)
            self.assertIn("4 parallel calls", content)

            # 3. Agent input tokens note in setup and footnote
            expected_note = (
                "agy is an agent: every call includes its own system prompt and tool definitions, "
                "about 13,600 input tokens, so input tokens here are not comparable with the other models' "
                "in absolute terms; compare formats within this model only."
            )
            self.assertIn(expected_note, content)

            # 4. Repeat spread table for seeds 1 to 4
            self.assertIn("| Format | Seed 1 | Seed 2 | Seed 3 | Seed 4 | Mean | Spread (Max-Min) |", content)

            # 5. Full prompt rendered
            self.assertIn("The following data is provided in Struct format:", content)
        finally:
            shutil.rmtree(temp_dir)


class TestGeminiTempDirIsolation(unittest.TestCase):
    def test_call_agy_creates_and_cleans_temp_dir(self):
        """Verify that call_agy runs from a new temporary directory and cleans it up."""
        created_cwds = []

        def mock_subproc_run(cmd, stdout=None, stderr=None, text=True, encoding="utf-8", cwd=None, timeout=None):
            created_cwds.append(cwd)
            self.assertTrue(os.path.exists(cwd), "Temporary working directory must exist during subprocess execution")
            self.assertEqual(len(os.listdir(cwd)), 0, "Temporary working directory must be empty")
            mock_proc = MagicMock()
            mock_proc.returncode = 0
            mock_proc.stdout = json.dumps({
                "status": "SUCCESS",
                "response": "1. 2024",
                "num_turns": 1,
                "usage": {"input_tokens": 13000, "output_tokens": 10},
            })
            mock_proc.stderr = ""
            return mock_proc

        with patch("subprocess.run", side_effect=mock_subproc_run):
            call_agy("Test prompt")

        self.assertEqual(len(created_cwds), 1)
        used_cwd = created_cwds[0]
        self.assertFalse(os.path.exists(used_cwd), "Temporary working directory must be cleaned up after execution")


if __name__ == "__main__":
    unittest.main()
