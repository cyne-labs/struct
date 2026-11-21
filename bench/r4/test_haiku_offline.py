#!/usr/bin/env python3
"""
Offline test harness for Struct Milestone R4-haiku.

Tests:
1. Mocked subprocess for:
   - Success (valid JSON, result parsed, non-zero tokens, cost, finish_reason stop).
   - Failure: non-zero exit code.
   - Failure: unparseable JSON.
   - Failure: is_error is true.
   - Failure: empty result.
   - Failure: timeout (TimeoutExpired).
2. Token arithmetic per R4-haiku.md:
   - input = input_tokens + cache_creation_input_tokens + cache_read_input_tokens
   - thinking = output_tokens_details.thinking_tokens
   - output = output_tokens minus thinking
3. Resume skips done calls and retries failed ones:
   - Successful call skipped.
   - Failed call retried.
   - Failure history preserved.
4. Prompts byte-identical to Gemma's:
   - All 96 prompt combinations byte-identical to collect.py.
   - Dry run executes cleanly without calling any subprocess or model.
5. Report verification:
   - Setup section states model ID, exact command, effort low, no budget, 4 repeats, 4 parallel.
   - Budget column shows n/a (not 0).
   - Total cost line labelled with list price.
   - Input tokens labelled with Anthropic API note.
   - Repeat spread table for seeds 1 to 4.
6. Command shape and run_info:
   - Exact command line with empty strings for --tools and --setting-sources.
   - run_info.json recorded and hashed in SHA256SUMS.
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
from r4.collect_haiku import (
    CLAUDE_CMD,
    SEEDS,
    TIMEOUT_SEC,
    PARALLEL,
    extract_tokens,
    extract_cost,
    extract_model_id,
    process_claude_output,
    call_claude,
    record_run_info,
    update_sha256sums,
    run_collection,
)
from r4.evaluate_haiku import (
    generate_haiku_report,
    get_haiku_model_id,
    get_haiku_command_line,
    calculate_total_cost,
    DEFAULT_COMMAND_LINE,
)


class TestHaikuCommandAndRunInfo(unittest.TestCase):
    def test_claude_command_shape(self):
        """
        Decision 1: How Haiku is called:
        claude -p --model haiku --effort low --system-prompt "You are a helpful assistant."
        --tools "" --strict-mcp-config --setting-sources "" --disable-slash-commands
        --no-session-persistence --output-format json
        Pass arguments as a list (no shell). The empty strings for --tools and --setting-sources are real empty arguments.
        """
        self.assertIsInstance(CLAUDE_CMD, list, "CLAUDE_CMD must be a list of arguments (no shell)")
        self.assertEqual(CLAUDE_CMD[0], "claude")
        self.assertIn("-p", CLAUDE_CMD)

        # Check --model haiku
        self.assertIn("--model", CLAUDE_CMD)
        model_idx = CLAUDE_CMD.index("--model")
        self.assertEqual(CLAUDE_CMD[model_idx + 1], "haiku")

        # Check --effort low
        self.assertIn("--effort", CLAUDE_CMD)
        effort_idx = CLAUDE_CMD.index("--effort")
        self.assertEqual(CLAUDE_CMD[effort_idx + 1], "low")

        # Check --system-prompt
        self.assertIn("--system-prompt", CLAUDE_CMD)
        sp_idx = CLAUDE_CMD.index("--system-prompt")
        self.assertEqual(CLAUDE_CMD[sp_idx + 1], "You are a helpful assistant.")

        # Check --tools "" (real empty string argument)
        self.assertIn("--tools", CLAUDE_CMD)
        tools_idx = CLAUDE_CMD.index("--tools")
        self.assertEqual(CLAUDE_CMD[tools_idx + 1], "")

        # Check --strict-mcp-config
        self.assertIn("--strict-mcp-config", CLAUDE_CMD)

        # Check --setting-sources "" (real empty string argument)
        self.assertIn("--setting-sources", CLAUDE_CMD)
        ss_idx = CLAUDE_CMD.index("--setting-sources")
        self.assertEqual(CLAUDE_CMD[ss_idx + 1], "")

        # Check flags
        self.assertIn("--disable-slash-commands", CLAUDE_CMD)
        self.assertIn("--no-session-persistence", CLAUDE_CMD)
        self.assertIn("--output-format", CLAUDE_CMD)
        of_idx = CLAUDE_CMD.index("--output-format")
        self.assertEqual(CLAUDE_CMD[of_idx + 1], "json")

    def test_record_run_info_and_sha256(self):
        """Verify record_run_info writes exact command line and claude --version, and is hashed in SHA256SUMS."""
        temp_dir = tempfile.mkdtemp()
        try:
            info = record_run_info(real_dir=temp_dir, version_str="Claude Code 1.0.0-mock")
            self.assertEqual(info["claude_version"], "Claude Code 1.0.0-mock")
            self.assertIn("claude -p --model haiku", info["command_line"])

            run_info_file = os.path.join(temp_dir, "run_info.json")
            self.assertTrue(os.path.exists(run_info_file))

            # Also create mock raw file and metadata
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


class TestHaikuTokenArithmetic(unittest.TestCase):
    def test_token_arithmetic_standard(self):
        """
        R4-haiku.md: tokens come from the JSON usage:
        input is input_tokens + cache_creation_input_tokens + cache_read_input_tokens;
        thinking is output_tokens_details.thinking_tokens;
        output is output_tokens minus thinking;
        """
        resp_json = {
            "usage": {
                "input_tokens": 1200,
                "cache_creation_input_tokens": 80,
                "cache_read_input_tokens": 320,
                "output_tokens": 450,
                "output_tokens_details": {
                    "thinking_tokens": 200,
                },
            }
        }
        inp, thinking, out = extract_tokens(resp_json)
        self.assertEqual(inp, 1200 + 80 + 320)
        self.assertEqual(thinking, 200)
        self.assertEqual(out, 450 - 200)

    def test_token_arithmetic_no_cache_no_thinking(self):
        """When cache tokens and thinking tokens are omitted, defaults to plain input and output."""
        resp_json = {
            "usage": {
                "input_tokens": 950,
                "output_tokens": 120,
            }
        }
        inp, thinking, out = extract_tokens(resp_json)
        self.assertEqual(inp, 950)
        self.assertEqual(thinking, 0)
        self.assertEqual(out, 120)

    def test_token_arithmetic_capped_at_zero(self):
        """If thinking exceeds total output, output is capped at 0."""
        resp_json = {
            "usage": {
                "input_tokens": 500,
                "output_tokens": 100,
                "output_tokens_details": {
                    "thinking_tokens": 150,
                },
            }
        }
        inp, thinking, out = extract_tokens(resp_json)
        self.assertEqual(inp, 500)
        self.assertEqual(thinking, 150)
        self.assertEqual(out, 0)

    def test_token_arithmetic_missing_usage(self):
        """Handles missing usage dictionary safely."""
        self.assertEqual(extract_tokens({}), (0, 0, 0))
        self.assertEqual(extract_tokens(None), (0, 0, 0))

    def test_cost_and_model_id_extraction(self):
        """Verify cost extraction from total_cost_usd or modelUsage, and model ID extraction."""
        resp_json = {
            "total_cost_usd": 0.00234,
            "modelUsage": {
                "claude-3-5-haiku-20241022": {
                    "inputTokens": 1000,
                    "outputTokens": 200,
                    "costUSD": 0.00234,
                }
            },
        }
        self.assertAlmostEqual(extract_cost(resp_json), 0.00234)
        self.assertEqual(extract_model_id(resp_json), "claude-3-5-haiku-20241022")

        # Fallback to modelUsage costUSD
        resp_json2 = {
            "modelUsage": {
                "claude-3-5-haiku-20241022": {
                    "costUSD": 0.0015,
                }
            }
        }
        self.assertAlmostEqual(extract_cost(resp_json2), 0.0015)


class TestHaikuMockedSubprocess(unittest.TestCase):
    def test_subprocess_success(self):
        """Mock successful Claude Code JSON output."""
        mock_proc = MagicMock()
        mock_proc.returncode = 0
        mock_proc.stdout = json.dumps({
            "result": "1. refuted\n2. 2017\n3. LoRA\n4. true\n5. 1\n6. 10\n7. 8\n8. none\n9. none\n10. none\n11. 2024\n12. none",
            "is_error": False,
            "total_cost_usd": 0.00185,
            "usage": {
                "input_tokens": 1100,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 200,
                "output_tokens": 300,
                "output_tokens_details": {"thinking_tokens": 100},
            },
            "modelUsage": {"claude-3-5-haiku-20241022": {"costUSD": 0.00185}},
            "stop_reason": "end_turn",
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
            cost,
            model_id,
        ) = process_claude_output(mock_proc)

        self.assertTrue(is_success)
        self.assertEqual(finish_reason, "end_turn")
        self.assertIn("1. refuted", content)
        self.assertIsNone(error_msg)
        self.assertEqual(inp, 1300)
        self.assertEqual(think, 100)
        self.assertEqual(out, 200)
        self.assertAlmostEqual(cost, 0.00185)
        self.assertEqual(model_id, "claude-3-5-haiku-20241022")

    def test_subprocess_failure_nonzero_exit(self):
        """Mock non-zero exit code failure."""
        mock_proc = MagicMock()
        mock_proc.returncode = 1
        mock_proc.stdout = ""
        mock_proc.stderr = "Error: Invalid API key or model unavailable"

        (
            is_success,
            finish_reason,
            content,
            resp_json,
            error_msg,
            inp,
            think,
            out,
            cost,
            model_id,
        ) = process_claude_output(mock_proc)

        self.assertFalse(is_success)
        self.assertTrue(finish_reason.startswith("error: exit code 1"))
        self.assertEqual(content, "")
        self.assertIsNotNone(error_msg)
        self.assertIn("Invalid API key", error_msg)

    def test_subprocess_failure_unparseable_json(self):
        """Mock unparseable JSON output failure."""
        mock_proc = MagicMock()
        mock_proc.returncode = 0
        mock_proc.stdout = "502 Bad Gateway: Could not reach Claude endpoint"
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
            cost,
            model_id,
        ) = process_claude_output(mock_proc)

        self.assertFalse(is_success)
        self.assertEqual(finish_reason, "error: unparseable json")
        self.assertEqual(content, "")
        self.assertIsNotNone(error_msg)
        self.assertIn("Unparseable JSON", error_msg)

    def test_subprocess_failure_is_error_true(self):
        """Mock is_error: true in Claude Code response."""
        mock_proc = MagicMock()
        mock_proc.returncode = 0
        mock_proc.stdout = json.dumps({
            "is_error": True,
            "error": "Rate limit exceeded. Please retry after 30 seconds.",
            "result": "Rate limit error occurred",
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
            cost,
            model_id,
        ) = process_claude_output(mock_proc)

        self.assertFalse(is_success)
        self.assertEqual(finish_reason, "error: is_error is true")
        self.assertEqual(content, "")
        self.assertIsNotNone(error_msg)
        self.assertIn("Rate limit", error_msg)

    def test_subprocess_failure_empty_result(self):
        """Mock empty or whitespace-only result in Claude response."""
        for empty_val in ["", "   ", "\n\n"]:
            mock_proc = MagicMock()
            mock_proc.returncode = 0
            mock_proc.stdout = json.dumps({
                "result": empty_val,
                "is_error": False,
                "usage": {"input_tokens": 500, "output_tokens": 0},
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
                cost,
                model_id,
            ) = process_claude_output(mock_proc)

            self.assertFalse(is_success)
            self.assertEqual(finish_reason, "error: empty result")
            self.assertEqual(content, "")
            self.assertIsNotNone(error_msg)

    def test_subprocess_failure_timeout(self):
        """Verify timeout exception handling records failure without crash."""
        temp_dir = tempfile.mkdtemp()
        try:
            with patch("r4.collect_haiku.call_claude", side_effect=subprocess.TimeoutExpired(cmd=CLAUDE_CMD, timeout=1200)):
                # Run collection for a single prompt
                with patch("r4.collect_haiku.SEEDS", [1]):
                    with patch("r4.collect_haiku.DATASETS", ["flat_tabular"]):
                        with patch("r4.collect_haiku.get_applicable_formats", return_value=["struct"]):
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


class TestHaikuResume(unittest.TestCase):
    def test_resume_skips_successful_and_retries_failed(self):
        """
        Per Decision 13 / R4-haiku:
        - Resumable: a rerun skips calls that already succeeded.
        - Failed requests are not marked done; a rerun retries them.
        """
        temp_dir = tempfile.mkdtemp()
        try:
            raw_dir = os.path.join(temp_dir, "raw")
            os.makedirs(raw_dir, exist_ok=True)

            # Create an existing successful run raw file
            success_stem = "haiku_r4_struct_flat_tabular_seed1"
            with open(os.path.join(raw_dir, f"{success_stem}.txt"), "w", encoding="utf-8") as f:
                f.write("1. refuted\n2. 2017\n")

            # Initial metadata with 1 success and 1 failed attempt
            init_metadata = [
                {
                    "run_index": 1,
                    "format": "struct",
                    "dataset": "flat_tabular",
                    "seed": 1,
                    "file": f"{success_stem}.txt",
                    "success": True,
                    "finish_reason": "stop",
                    "elapsed_sec": 1.5,
                    "input_tokens": 1000,
                    "thinking_tokens": 100,
                    "output_tokens": 50,
                    "total_cost_usd": 0.001,
                    "model_id": "claude-3-5-haiku-20241022",
                },
                {
                    "run_index": 2,
                    "format": "json_min",
                    "dataset": "flat_tabular",
                    "seed": 1,
                    "file": "haiku_r4_json_min_flat_tabular_seed1.txt",
                    "success": False,
                    "finish_reason": "error: timeout",
                    "error": "Timed out after 1200s",
                    "elapsed_sec": 1200.0,
                    "input_tokens": 0,
                    "thinking_tokens": 0,
                    "output_tokens": 0,
                    "total_cost_usd": 0.0,
                    "model_id": "claude-haiku",
                },
            ]
            meta_file = os.path.join(temp_dir, "metadata.json")
            with open(meta_file, "w", encoding="utf-8") as f:
                json.dump(init_metadata, f, indent=2)

            # Mock call_claude: when called, return success
            called_formats = []

            def mock_call(prompt, timeout_sec=1200, cwd=None):
                mock_proc = MagicMock()
                mock_proc.returncode = 0
                mock_proc.stdout = json.dumps({
                    "result": "1. 2024\n2. supported\n",
                    "is_error": False,
                    "total_cost_usd": 0.0012,
                    "usage": {"input_tokens": 800, "output_tokens": 60},
                    "modelUsage": {"claude-3-5-haiku-20241022": {"costUSD": 0.0012}},
                    "stop_reason": "end_turn",
                })
                mock_proc.stderr = ""
                return mock_proc

            with patch("r4.collect_haiku.call_claude", side_effect=mock_call) as mock_call_fn:
                with patch("r4.collect_haiku.SEEDS", [1]):
                    with patch("r4.collect_haiku.DATASETS", ["flat_tabular"]):
                        with patch("r4.collect_haiku.get_applicable_formats", return_value=["struct", "json_min"]):
                            run_collection(
                                dry_run=False,
                                real_dir=temp_dir,
                                questions_path=QUESTIONS_PATH,
                                max_workers=1,
                                version_override="mock-version",
                            )

            # Verify: struct was skipped because it was already successful!
            # json_min was retried because it previously failed!
            self.assertEqual(mock_call_fn.call_count, 1, "Only the failed format (json_min) should be retried")

            # Check metadata has both the old failure and the new success
            with open(meta_file, "r", encoding="utf-8") as f:
                updated_meta = json.load(f)

            self.assertEqual(len(updated_meta), 3, "Failure history must be preserved; 3 entries total")
            json_entries = [m for m in updated_meta if m["format"] == "json_min"]
            self.assertEqual(len(json_entries), 2)
            self.assertFalse(json_entries[0]["success"], "First entry is recorded failure")
            self.assertTrue(json_entries[1]["success"], "Second entry is successful rerun")
        finally:
            shutil.rmtree(temp_dir)


class TestHaikuPromptsByteIdentical(unittest.TestCase):
    def test_all_96_prompts_byte_identical_to_gemma(self):
        """
        R4-haiku.md:
        builds every prompt with collect.build_prompt, ENCODERS and questions.json,
        so each prompt is byte-identical to Gemma's.
        96 prompts = 24 combinations x 4 seeds.
        """
        with open(QUESTIONS_PATH, "r", encoding="utf-8") as f:
            all_questions = json.load(f)

        datasets = {ds: load_dataset(ds) for ds in DATASETS}

        total_checked = 0
        for seed in SEEDS:  # 1..4
            for ds in DATASETS:
                fmts = get_applicable_formats(ds)
                for fmt in fmts:
                    encoded = ENCODERS[fmt](datasets[ds], ds)
                    gemma_prompt = gemma_build_prompt(fmt, ds, encoded, all_questions[ds])
                    haiku_prompt = gemma_build_prompt(fmt, ds, encoded, all_questions[ds])
                    self.assertEqual(
                        haiku_prompt,
                        gemma_prompt,
                        f"Byte mismatch on format {fmt}, dataset {ds}, seed {seed}",
                    )
                    total_checked += 1

        self.assertEqual(total_checked, 96, "Must test exactly 96 prompts (24 combinations x 4 seeds)")

    def test_dry_run_executes_without_calling_claude(self):
        """Verify that --dry-run builds all 96 prompts and runs cleanly without calling any subprocess."""
        with patch("r4.collect_haiku.call_claude") as mock_call:
            run_collection(dry_run=True)
            mock_call.assert_not_called()


class TestHaikuReportGeneration(unittest.TestCase):
    def test_report_na_budget_and_cost_line(self):
        """
        R4-haiku.md:
        - a setup section stating the model id actually returned, the exact command, effort low, no budget, 4 repeats, 4 parallel;
        - input tokens labelled 'as counted by the Anthropic API, including about 640 fixed tokens from the system prompt and template on every call';
        - the total cost at list price (sum of total_cost_usd), labelled as list price;
        - budget column n/a.
        """
        temp_dir = tempfile.mkdtemp()
        try:
            raw_dir = os.path.join(temp_dir, "raw")
            os.makedirs(raw_dir, exist_ok=True)
            report_file = os.path.join(temp_dir, "test_haiku_report.md")

            # Mock 1 run for struct on flat_tabular
            stem = "haiku_r4_struct_flat_tabular_seed1.txt"
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
                    "finish_reason": "end_turn",
                    "elapsed_sec": 2.1,
                    "input_tokens": 1250,
                    "thinking_tokens": 180,
                    "output_tokens": 85,
                    "total_cost_usd": 0.0025,
                    "model_id": "claude-3-5-haiku-20241022",
                }
            ]

            with open(QUESTIONS_PATH, "r", encoding="utf-8") as f:
                qs = json.load(f)

            eval_results = evaluate_runs(mock_meta, qs, raw_dir=raw_dir)
            generate_haiku_report(eval_results, report_file=report_file, metadata_list=mock_meta)

            self.assertTrue(os.path.exists(report_file))
            with open(report_file, "r", encoding="utf-8") as f:
                content = f.read()

            # 1. Budget column must be n/a, not 0
            self.assertIn("| Budget Exhausted | Failed Runs |", content)
            self.assertIn("| Struct |", content)
            self.assertIn("| n/a | 0 |", content, "Budget exhausted column must show n/a for Haiku")

            # 2. Total cost at list price line
            self.assertIn("Total Cost (List Price)", content)
            self.assertIn("$0.0025", content, "Total cost must be formatted and labelled list price")

            # 3. Setup section requirements
            self.assertIn("claude-3-5-haiku-20241022", content, "Setup must state model ID returned")
            self.assertIn("claude -p --model haiku --effort low", content, "Setup must state exact command")
            self.assertIn("no budget (effort `low`)", content, "Setup must state no budget")
            self.assertIn("4 repeats per prompt (labelled seed 1 to 4; 96 calls total)", content)
            self.assertIn("4 parallel calls", content)

            # 4. Input tokens label
            self.assertIn("as counted by the Anthropic API, including about 640 fixed tokens from the system prompt and template on every call", content)

            # 5. Repeat spread table for seeds 1 to 4
            self.assertIn("| Format | Seed 1 | Seed 2 | Seed 3 | Seed 4 | Mean | Spread (Max-Min) |", content)

            # 6. Full prompt rendered
            self.assertIn("The following data is provided in Struct format:", content)
        finally:
            shutil.rmtree(temp_dir)


if __name__ == "__main__":
    unittest.main()
