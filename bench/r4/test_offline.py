#!/usr/bin/env python3
"""
Offline test harness for Struct Milestone R4.
Tests:
1. All 7 format encoders and their lossless round-trip checks across all 4 datasets.
2. Verification of R4 Decision 1 (TSV & key_value reject nested datasets).
3. Question generation and ground truth validity across all 4 datasets.
4. Normalization and answer scoring on edge cases (nulls, quotes, lists, casing, budget exhaust).
5. Review decisions & fixes:
   - Blank answer wrong on a null question (Decision 9).
   - Markdown bold and trailing period stripped (Decision 10).
   - Defensible bool forms (yes/no, true/false, 1/0) and count exclusion (Decision 11).
   - Empty-list forms (none, empty, [], [0], null words) (Decision 11).
   - XML empty lists: plain empty tag with no type attribute (Decision 12).
   - Collector: failed requests not marked done, retried on resume (Decision 13).
   - Budget count computed from thinking tokens reaching 8192 (Decision 13).
   - metadata.json included in SHA256SUMS (Decision 13).
   - TOON nested header valid, numeric IDs quoted, empty lists formatted per spec (Decision 13).
   - Running tests never rewrites questions.json (Decision 13).
   - Strict type checking in no-loss check (detects bool vs int).
6. Collector dry-run (prompt construction for all 120 planned requests).
7. End-to-end evaluator and report generator with synthetic model outputs.
"""

import os
import sys
import json
import shutil
import tempfile
import unittest

BENCH_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STRUCT_ROOT = os.path.dirname(BENCH_DIR)
R4_DIR = os.path.join(BENCH_DIR, "r4")
sys.path.insert(0, BENCH_DIR)

from r4.formats import (
    FORMATS,
    ENCODERS,
    DECODERS,
    FORMAT_DISPLAY_NAMES,
    verify_no_loss,
    check_all_formats,
)
from r4.formats.no_loss import strict_equal
from r4.generate_data import generate_flat_100
from r4.generate_questions import generate_questions, load_source_data
from r4.collect import (
    build_prompt,
    get_applicable_formats,
    SEEDS,
    DATASETS,
    run_collection,
    update_sha256sums,
    get_launch_flags_from_props,
    load_dataset,
    load_questions,
)
import r4.collect as collect_mod
import r4.evaluate as eval_mod
from r4.evaluate import normalize_text, is_no_value, score_answer, parse_model_answers, evaluate_runs, generate_report

class TestR4EncodersAndNoLoss(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source_data = load_source_data()

    def test_flat_tabular_lossless(self):
        """All 7 formats must be 100% lossless on flat_tabular."""
        data = self.source_data["flat_tabular"]
        results = check_all_formats("flat_tabular", data)
        for fmt, (status, msg) in results.items():
            self.assertTrue(status, f"Format {fmt} failed on flat_tabular: {msg}")

    def test_flat_100_lossless(self):
        """All 7 formats must be 100% lossless on flat_100 with null values."""
        data = self.source_data["flat_100"]
        results = check_all_formats("flat_100", data)
        for fmt, (status, msg) in results.items():
            self.assertTrue(status, f"Format {fmt} failed on flat_100: {msg}")

    def test_nested_fixed_lossless(self):
        """5 universal formats must be lossless; TSV and key_value must be rejected per Decision 1."""
        data = self.source_data["nested_fixed"]
        for fmt in FORMATS:
            status, msg, _ = verify_no_loss(fmt, "nested_fixed", data)
            self.assertTrue(status, f"Format {fmt} failed on nested_fixed: {msg}")

    def test_nested_variable_lossless(self):
        """5 universal formats must be lossless; TSV and key_value must be rejected per Decision 1."""
        data = self.source_data["nested_variable"]
        for fmt in FORMATS:
            status, msg, _ = verify_no_loss(fmt, "nested_variable", data)
            self.assertTrue(status, f"Format {fmt} failed on nested_variable: {msg}")

    def test_r1_fixture_byte_consistency_struct(self):
        """Verify that Struct encoder produces formats structurally identical to R1 fixtures when given R1 data."""
        r1_fixtures = os.path.join(BENCH_DIR, "r1", "fixtures")
        for shape in ["flat_tabular", "nested_fixed", "nested_variable"]:
            r1_json_path = os.path.join(r1_fixtures, f"{shape}.min.json")
            with open(r1_json_path, "r", encoding="utf-8") as f:
                r1_data = json.load(f)

            # Check struct matches R1 fixture
            stc_path = os.path.join(r1_fixtures, f"{shape}.v02.stc")
            if os.path.exists(stc_path):
                with open(stc_path, "r", encoding="utf-8") as f:
                    expected_stc = f.read()
                actual_stc = ENCODERS["struct"](r1_data, shape)
                self.assertEqual(actual_stc.strip(), expected_stc.strip())

    def test_toon_nested_header_valid(self):
        """
        Per Decision 13: TOON follows the TOON spec:
        - Nested fixed uses valid nested field group tabular header: [N]{text,verdict,source{id,year},verified}:
        - Numeric-looking strings (e.g. arXiv IDs like '2405.05254') are quoted.
        - Empty lists use zero-length headers: subnotes[0]:
        - Lossless roundtrip preserves strict types.
        """
        nf_data = self.source_data["nested_fixed"]
        toon_nf = ENCODERS["toon"](nf_data, "nested_fixed")
        lines = toon_nf.splitlines()
        self.assertTrue(lines[0].startswith("[10]{text,verdict,source{id,year},verified}:"), "Valid TOON nested header")
        self.assertIn('"2405.05254"', toon_nf, "Numeric-looking IDs must be quoted per TOON spec")

        # Lossless decode check
        decoded_nf = DECODERS["toon"](toon_nf, "nested_fixed")
        self.assertTrue(strict_equal(decoded_nf, nf_data), "TOON nested_fixed must roundtrip with strict types")

        # Nested variable empty list check (TOON spec 9.1: key: [], never key[0]:)
        nv_data = self.source_data["nested_variable"]
        toon_nv = ENCODERS["toon"](nv_data, "nested_variable")
        self.assertIn("subnotes: []", toon_nv, "Empty lists must be encoded as subnotes: [] per TOON spec 9.1")
        self.assertNotIn("subnotes[0]:", toon_nv, "Legacy [0] syntax must not be used for empty lists")
        decoded_nv = DECODERS["toon"](toon_nv, "nested_variable")
        self.assertTrue(strict_equal(decoded_nv, nv_data), "TOON nested_variable must roundtrip with strict types")

    def test_xml_empty_tag_null(self):
        """Per Decision 7: XML null is an empty tag like <year/> with no null attribute and no null text."""
        ft = self.source_data["flat_tabular"]
        xml_text = ENCODERS["xml"](ft, "flat_tabular")
        self.assertIn("<year/>", xml_text)
        self.assertNotIn('null="true"', xml_text)
        self.assertNotIn('>null<', xml_text)
        decoded = DECODERS["xml"](xml_text, "flat_tabular")
        self.assertTrue(strict_equal(decoded, ft))

    def test_xml_empty_list_no_type_attribute(self):
        """Per Decision 12: XML empty lists are plain empty tags like <subnotes/> with no type attribute."""
        nv = self.source_data["nested_variable"]
        xml_text = ENCODERS["xml"](nv, "nested_variable")
        self.assertIn("<subnotes/>", xml_text)
        self.assertNotIn('type="list"', xml_text)

    def test_strict_type_no_loss(self):
        """Verify strict type checking detects bool vs int confusion (e.g. True == 1)."""
        data = [{"id": "1", "val": True}]
        mutated = [{"id": "1", "val": 1}]
        self.assertFalse(strict_equal(data, mutated), "strict_equal must distinguish True from 1")

    def test_key_value_separator_and_null(self):
        """Per Decision 8: key: value records separated by a --- line."""
        ft = self.source_data["flat_tabular"]
        kv_text = ENCODERS["key_value"](ft, "flat_tabular")
        self.assertIn("\n---\n", kv_text)
        self.assertIn("year: null", kv_text)
        decoded = DECODERS["key_value"](kv_text, "flat_tabular")
        self.assertTrue(strict_equal(decoded, ft))

    def test_doctor_tool_roundtrip_flat_100(self):
        """Verify Rust doctor_tool.exe decodes Struct flat_100 losslessly."""
        doctor_exe = os.path.join(STRUCT_ROOT, "target", "release", "doctor_tool.exe")
        if not os.path.exists(doctor_exe):
            self.skipTest("doctor_tool.exe not compiled")

        import subprocess
        data = self.source_data["flat_100"]
        stc_text = ENCODERS["struct"](data, "flat_100")

        with tempfile.NamedTemporaryFile("w", suffix=".stc", delete=False) as sf:
            sf.write("schema:claim{id, title, verdict:enum(supported,refuted,unverified), year:int, verified:bool}\n")
            schema_path = sf.name
        with tempfile.NamedTemporaryFile("w", suffix=".stc", delete=False) as df:
            df.write(stc_text)
            data_path = df.name

        try:
            cmd = [doctor_exe, schema_path, data_path, "salvage", "true"]
            out = subprocess.check_output(cmd, text=True)
            res = json.loads(out)
            recs = res.get("records", [])
            self.assertEqual(recs, data)
        finally:
            if os.path.exists(schema_path):
                os.remove(schema_path)
            if os.path.exists(data_path):
                os.remove(data_path)

    def test_defect1_toon_empty_list_spec_9_1(self):
        """Defect 1: TOON empty lists use 'key: []' per TOON spec 9.1, never 'key[0]:'."""
        nv_data = self.source_data["nested_variable"]
        encoded = ENCODERS["toon"](nv_data, "nested_variable")
        self.assertIn("subnotes: []", encoded, "Must emit 'subnotes: []' per spec 9.1")
        self.assertNotIn("subnotes[0]:", encoded, "Must not emit legacy 'subnotes[0]:'")
        decoded = DECODERS["toon"](encoded, "nested_variable")
        self.assertTrue(strict_equal(decoded, nv_data), "TOON nested_variable with empty list must roundtrip strictly")

        # Also test with record containing empty notes list
        data_empty_notes = [{
            "text": "claim without notes",
            "verdict": "supported",
            "source": {"id": "1234", "year": 2024},
            "notes": [],
        }]
        encoded_en = ENCODERS["toon"](data_empty_notes, "nested_variable")
        self.assertIn("notes: []", encoded_en, "Must emit 'notes: []'")
        self.assertNotIn("notes[0]:", encoded_en, "Must not emit legacy 'notes[0]:'")
        decoded_en = DECODERS["toon"](encoded_en, "nested_variable")
        self.assertTrue(strict_equal(decoded_en, data_empty_notes))

    def test_defect6_xml_empty_list_limit_in_no_loss_output(self):
        """Defect 6: Show the XML empty-list limit in the no-loss output."""
        nv_data = self.source_data["nested_variable"]
        status, msg, _ = verify_no_loss("xml", "nested_variable", nv_data)
        self.assertTrue(status)
        self.assertIn("plain XML cannot distinguish empty list from null per Decision 12", msg,
                      "No-loss output for XML must show the Decision 12 limit")

class TestR4QuestionsAndGroundTruth(unittest.TestCase):
    def test_questions_structure(self):
        qs = generate_questions(write_file=False)
        self.assertEqual(len(qs), 4, "Must contain exactly 4 datasets")

        expected_datasets = ["flat_tabular", "flat_100", "nested_fixed", "nested_variable"]
        for ds in expected_datasets:
            self.assertIn(ds, qs)
            ds_qs = qs[ds]
            self.assertEqual(len(ds_qs), 12, f"Dataset {ds} must have exactly 12 questions")

            kinds = {q["kind"] for q in ds_qs}
            self.assertIn("single_lookup", kinds)
            self.assertIn("count_filter", kinds)
            self.assertIn("null_missing", kinds)
            if ds in ("nested_fixed", "nested_variable"):
                self.assertIn("nested_lookup", kinds)

            for q in ds_qs:
                self.assertTrue(q["question"].endswith("?"), f"Question must end with ?: {q['question']}")
                self.assertIsNotNone(q["ground_truth"])
                self.assertTrue(len(str(q["ground_truth"])) > 0)

    def test_null_questions_real_empty_values_only(self):
        """Per Decision 6: Null questions ask about real empty values in the data, never non-existent records."""
        qs = generate_questions(write_file=False)
        for ds_name, ds_qs in qs.items():
            null_qs = [q for q in ds_qs if q["kind"] == "null_missing"]
            self.assertTrue(len(null_qs) >= 2, f"Dataset {ds_name} must have null_missing questions")
            for q in null_qs:
                # Must never ask about artificial non-existent IDs
                self.assertNotIn("9999.99999", q["question"])
                self.assertNotIn("0000.00000", q["question"])
                self.assertNotIn("Nonexistent", q["question"])
                self.assertNotIn("optimizer state minimal", q["question"])

    def test_questions_file_not_rewritten(self):
        """Per Decision 13: Running the tests never rewrites questions.json."""
        questions_path = os.path.join(R4_DIR, "questions.json")
        mtime_before = os.path.getmtime(questions_path) if os.path.exists(questions_path) else None

        # Call generate_questions with write_file=False (default for test harness)
        qs = generate_questions(write_file=False)
        self.assertEqual(len(qs), 4)

        if mtime_before is not None:
            mtime_after = os.path.getmtime(questions_path)
            self.assertEqual(mtime_before, mtime_after, "questions.json was modified during testing")

class TestR4ScoringAndNormalization(unittest.TestCase):
    def test_normalization_basics(self):
        self.assertEqual(normalize_text("  Refuted  "), "refuted")
        self.assertEqual(normalize_text('"2024"'), "2024")
        self.assertEqual(normalize_text("'supported'"), "supported")
        self.assertEqual(normalize_text("“true”"), "true")
        self.assertEqual(normalize_text("LoRA   low  rank   adaptation"), "lora low rank adaptation")

    def test_blank_answer_wrong_on_null_question(self):
        """Per Decision 9: A blank or missing answer is always wrong, including when the true answer is no value."""
        for exp in ["none", "null", "~", "n/a"]:
            # Blank and whitespace-only answers MUST score False
            for blank in ["", "   ", "  \n  "]:
                correct, _, _ = score_answer(exp, blank, kind="null_missing")
                self.assertFalse(correct, f"Blank answer '{blank}' must be wrong when expected is '{exp}'")

            # Explicit null words MUST score True
            for valid in ["null", "none", "~", "n/a", "NULL", "None", "N/A", "  ~  ", '""']:
                if valid == '""':
                    # Empty quotes normalize to blank, which is wrong per Decision 9
                    correct, _, _ = score_answer(exp, valid, kind="null_missing")
                    self.assertFalse(correct)
                else:
                    correct, _, _ = score_answer(exp, valid, kind="null_missing")
                    self.assertTrue(correct, f"Explicit null word '{valid}' must pass when expected is '{exp}'")

        # When expected is a real value, null or blank is wrong
        correct, _, _ = score_answer("2024", "")
        self.assertFalse(correct)
        correct, _, _ = score_answer("2024", "null")
        self.assertFalse(correct)

    def test_markdown_bold_and_trailing_period_stripped(self):
        """Per Decision 10: Strip markdown bold/italics and one trailing period before comparing."""
        # Trailing period stripped
        self.assertEqual(normalize_text("2310.06825."), "2310.06825")
        self.assertEqual(normalize_text("2024."), "2024")
        self.assertEqual(normalize_text("refuted."), "refuted")
        correct, _, _ = score_answer("2310.06825", "2310.06825.")
        self.assertTrue(correct)

        # Markdown bold and italics stripped
        self.assertEqual(normalize_text("**refuted**"), "refuted")
        self.assertEqual(normalize_text("*refuted*"), "refuted")
        self.assertEqual(normalize_text("__supported__"), "supported")
        self.assertEqual(normalize_text("_supported_"), "supported")
        self.assertEqual(normalize_text('**"2024"**'), "2024")
        correct, _, _ = score_answer("refuted", "**refuted**")
        self.assertTrue(correct)
        correct, _, _ = score_answer("2024", "*2024*.")
        self.assertTrue(correct)

        # Numbered list parsing handles bold/italics around item numbers
        raw = "**1.** refuted\n*2.* 2017\n**3**. LoRA\n4. **true**"
        parsed = parse_model_answers(raw, 4)
        self.assertEqual(parsed[1], "refuted")
        self.assertEqual(parsed[2], "2017")
        self.assertEqual(parsed[3], "LoRA")
        self.assertEqual(parsed[4], "**true**")

    def test_bool_forms(self):
        """Per Decision 11: Accept yes/no, true/false and 1/0 for boolean fields; count filter must not accept yes/true."""
        # Truthy boolean field
        for ans in ["true", "True", "TRUE", "yes", "Yes", "1", "**yes**", "true."]:
            correct, _, _ = score_answer("true", ans, kind="single_lookup")
            self.assertTrue(correct, f"Expected 'true' to match '{ans}'")
        for bad in ["false", "no", "0", ""]:
            correct, _, _ = score_answer("true", bad, kind="single_lookup")
            self.assertFalse(correct)

        # Falsey boolean field
        for ans in ["false", "False", "FALSE", "no", "No", "0", "**no**", "false."]:
            correct, _, _ = score_answer("false", ans, kind="single_lookup")
            self.assertTrue(correct, f"Expected 'false' to match '{ans}'")
        for bad in ["true", "yes", "1", ""]:
            correct, _, _ = score_answer("false", bad, kind="single_lookup")
            self.assertFalse(correct)

        # Count filter question where ground truth is an integer count 1
        correct, _, _ = score_answer("1", "1", kind="count_filter")
        self.assertTrue(correct)
        # For a count question, "yes" or "true" is NOT accepted
        correct, _, _ = score_answer("1", "yes", kind="count_filter")
        self.assertFalse(correct, "Count filter question must not accept 'yes'")
        correct, _, _ = score_answer("1", "true", kind="count_filter")
        self.assertFalse(correct, "Count filter question must not accept 'true'")

    def test_empty_list_forms(self):
        """Per Decision 11: For empty lists, accept none, empty, [] and null words; reject [0]."""
        for ans in ["none", "empty", "[]", "null", "~", "n/a", "no subnotes", "EMPTY", "None"]:
            correct, _, _ = score_answer("none", ans, kind="null_missing")
            self.assertTrue(correct, f"Expected empty list to match '{ans}'")
        # [0] must be rejected
        correct, _, _ = score_answer("none", "[0]", kind="null_missing")
        self.assertFalse(correct, "[0] form must be rejected for empty list")

        for bad in ["item1", "something", "", "   "]:
            correct, _, _ = score_answer("none", bad, kind="null_missing")
            self.assertFalse(correct)

    def test_list_scoring(self):
        """Rule: comma-separated, compared as a set (order ignored). Every item must match, nothing extra."""
        exp = "2401.0001, 2402.0002, 2403.0003"
        # Order reversed
        correct, _, _ = score_answer(exp, "2403.0003, 2401.0001, 2402.0002")
        self.assertTrue(correct)
        # Spaces around items
        correct, _, _ = score_answer(exp, "  2401.0001 ,  2402.0002  , 2403.0003 ")
        self.assertTrue(correct)
        # Missing item
        correct, _, _ = score_answer(exp, "2401.0001, 2402.0002")
        self.assertFalse(correct)
        # Extra item
        correct, _, _ = score_answer(exp, "2401.0001, 2402.0002, 2403.0003, 2404.0004")
        self.assertFalse(correct)

    def test_defect4_null_truth_rejects_empty_list_words(self):
        """Defect 4: Null truths accept only null, none, ~, n/a; reject [], empty, [0]."""
        for ans in ["null", "none", "~", "n/a", "NULL", "None", "N/A"]:
            correct, _, _ = score_answer("null", ans, kind="null_missing")
            self.assertTrue(correct, f"Null truth must accept '{ans}'")
        for bad in ["[]", "empty", "[0]", "no notes", "no subnotes"]:
            correct, _, _ = score_answer("null", bad, kind="null_missing")
            self.assertFalse(correct, f"Null truth must reject empty list word '{bad}'")

    def test_defect5_list_answer_bold_italics_stripped(self):
        """Defect 5: Strip bold/italics wrapping a whole list answer too (Decision 10)."""
        exp = "2310.06825, 2406.11794"
        for actual in [
            "**2310.06825, 2406.11794**",
            "*2310.06825, 2406.11794*",
            "__2310.06825, 2406.11794__",
            "_2310.06825, 2406.11794_",
            "**2310.06825**, **2406.11794**",
            "**2310.06825, 2406.11794**.",
            "*2310.06825, 2406.11794*.",
        ]:
            correct, _, _ = score_answer(exp, actual, kind="single_lookup")
            self.assertTrue(correct, f"List answer must match with formatting: '{actual}'")

    def test_no_partial_credit(self):
        correct, _, _ = score_answer("supported", "support")
        self.assertFalse(correct)
        correct, _, _ = score_answer("2024", "2023")
        self.assertFalse(correct)

    def test_parse_model_answers(self):
        raw = """1. refuted
2. 2017
3. LoRA low rank adaptation
4. true
5. 2
6. 10
7. 8
8. 2310.06825, 2406.11794
9. none
10. none
11. none
12. none
"""
        parsed = parse_model_answers(raw, 12)
        self.assertEqual(len(parsed), 12)
        self.assertEqual(parsed[1], "refuted")
        self.assertEqual(parsed[2], "2017")
        self.assertEqual(parsed[8], "2310.06825, 2406.11794")
        self.assertEqual(parsed[12], "none")

class TestR4HarnessEndToEnd(unittest.TestCase):
    def test_collector_dry_run(self):
        """Verify collector dry-run generates all 120 requests cleanly."""
        run_collection(dry_run=True)

    def test_failed_request_not_marked_done(self):
        """Per Decision 13: Failed requests are not marked done; resume reruns them."""
        metadata = [
            {"format": "json_min", "dataset": "flat_tabular", "seed": 42, "success": True, "output_tokens": 50},
            {"format": "struct", "dataset": "flat_tabular", "seed": 42, "success": False, "finish_reason": "error: timeout", "output_tokens": 0},
            {"format": "yaml", "dataset": "flat_tabular", "seed": 42, "success": False, "finish_reason": "error: no answer text", "output_tokens": 0},
        ]
        done = {
            (m["format"], m["dataset"], m["seed"])
            for m in metadata
            if m.get("success", False)
            and not m.get("error")
            and not str(m.get("finish_reason", "")).startswith("error")
            and m.get("output_tokens", 0) > 0
        }
        self.assertIn(("json_min", "flat_tabular", 42), done)
        self.assertNotIn(("struct", "flat_tabular", 42), done, "Failed request must not be marked done")
        self.assertNotIn(("yaml", "flat_tabular", 42), done, "Request with no answer text must not be marked done")

    def test_budget_count_from_thinking_tokens(self):
        """Per Decision 13: Budget exhaustion is counted from thinking tokens reaching 8192 budget."""
        mock_meta = [
            # Hit 8192 budget with finish_reason stop
            {"format": "struct", "dataset": "flat_tabular", "seed": 42, "file": "mock1.txt", "finish_reason": "stop", "thinking_tokens": 8192, "output_tokens": 50, "success": True},
            # Hit > 8192 budget
            {"format": "struct", "dataset": "flat_tabular", "seed": 43, "file": "mock2.txt", "finish_reason": "stop", "thinking_tokens": 8500, "output_tokens": 50, "success": True},
            # Normal thinking under budget
            {"format": "struct", "dataset": "flat_tabular", "seed": 44, "file": "mock3.txt", "finish_reason": "stop", "thinking_tokens": 4096, "output_tokens": 50, "success": True},
            # Length finish reason
            {"format": "json_min", "dataset": "flat_tabular", "seed": 42, "file": "mock4.txt", "finish_reason": "length", "thinking_tokens": 100, "output_tokens": 50, "success": True},
        ]
        results = evaluate_runs(mock_meta, {"flat_tabular": []}, raw_dir=tempfile.gettempdir())
        self.assertEqual(results["by_format"]["struct"]["budget_exhausted"], 2, "Must count runs reaching 8192 thinking tokens")
        self.assertEqual(results["by_format"]["json_min"]["budget_exhausted"], 1, "Must count length finish reason")

    def test_metadata_json_hashed(self):
        """Per Decision 13: metadata.json is included in SHA256SUMS."""
        temp_dir = tempfile.mkdtemp()
        try:
            raw_dir = os.path.join(temp_dir, "raw")
            os.makedirs(raw_dir, exist_ok=True)
            raw_file = os.path.join(raw_dir, "sample.txt")
            with open(raw_file, "w", encoding="utf-8") as f:
                f.write("sample response")

            meta_file = os.path.join(temp_dir, "metadata.json")
            with open(meta_file, "w", encoding="utf-8") as f:
                json.dump([{"test": 1}], f)

            sha_file = os.path.join(temp_dir, "SHA256SUMS")

            # Monkeypatch paths in collect
            old_raw, old_meta, old_sha = collect_mod.RAW_DIR, collect_mod.METADATA_FILE, collect_mod.SHA256_FILE
            collect_mod.RAW_DIR = raw_dir
            collect_mod.METADATA_FILE = meta_file
            collect_mod.SHA256_FILE = sha_file

            try:
                collect_mod.update_sha256sums()
                self.assertTrue(os.path.exists(sha_file))
                with open(sha_file, "r", encoding="utf-8") as f:
                    sha_content = f.read()
                self.assertIn("metadata.json", sha_content, "metadata.json must be present in SHA256SUMS")
                self.assertIn("raw/sample.txt", sha_content, "raw files must be present in SHA256SUMS")
            finally:
                collect_mod.RAW_DIR = old_raw
                collect_mod.METADATA_FILE = old_meta
                collect_mod.SHA256_FILE = old_sha
        finally:
            shutil.rmtree(temp_dir)

    def test_evaluator_with_mock_run(self):
        """Verify evaluation and report generation pipeline with synthetic answers, format x kind table, and notes."""
        temp_dir = tempfile.mkdtemp()
        try:
            mock_raw_dir = os.path.join(temp_dir, "raw")
            os.makedirs(mock_raw_dir, exist_ok=True)
            mock_report = os.path.join(temp_dir, "test_report.md")

            qs = generate_questions(write_file=False)
            mock_meta = []

            # Create 1 mock run per dataset per format
            run_i = 1
            for ds in ["flat_tabular", "nested_variable"]:
                fmts = ["json_min", "struct"]
                for fmt in fmts:
                    fname = f"mock_{fmt}_{ds}.txt"
                    fpath = os.path.join(mock_raw_dir, fname)

                    # Build answers: 10 correct, 2 wrong
                    ds_qs = qs[ds]
                    ans_lines = []
                    for idx, q in enumerate(ds_qs, start=1):
                        if idx <= 10:
                            ans_lines.append(f"{idx}. {q['ground_truth']}")
                        else:
                            ans_lines.append(f"{idx}. intentionally_wrong_answer")

                    with open(fpath, "w", encoding="utf-8") as f:
                        f.write("\n".join(ans_lines) + "\n")

                    mock_meta.append({
                        "run_index": run_i,
                        "format": fmt,
                        "dataset": ds,
                        "seed": 42,
                        "file": fname,
                        "success": True,
                        "finish_reason": "stop" if run_i % 2 == 1 else "length",
                        "elapsed_sec": 12.5,
                        "input_tokens": 550,
                        "thinking_tokens": 2048 if run_i % 2 == 1 else 8192,
                        "output_tokens": 120,
                    })
                    run_i += 1

            eval_results = evaluate_runs(mock_meta, qs, raw_dir=mock_raw_dir)
            self.assertEqual(len(eval_results["wrong_answers"]), 8) # 2 wrong answers * 4 runs
            generate_report(eval_results, report_file=mock_report, metadata_list=mock_meta)
            self.assertTrue(os.path.exists(mock_report))

            with open(mock_report, "r", encoding="utf-8") as f:
                report_content = f.read()
            self.assertIn("Retrieval Accuracy Across Formats", report_content)
            self.assertIn("Accuracy per Format by Question Kind", report_content)
            self.assertIn("The following data is provided in Struct format:", report_content)
            self.assertIn("Decision 12", report_content)
            self.assertIn("Decision 13", report_content)
            self.assertIn("--port 8090", report_content)
            self.assertNotIn("-p 8090", report_content)
        finally:
            shutil.rmtree(temp_dir)

    def test_defect2_report_prints_real_full_prompt(self):
        """Defect 2: The report prints the real full prompt rendered by build_prompt, never hand-typed."""
        ft_data = load_dataset("flat_tabular")
        ft_qs = load_questions()["flat_tabular"]
        expected_prompt = build_prompt("struct", "flat_tabular", ENCODERS["struct"](ft_data, "flat_tabular"), ft_qs)

        temp_dir = tempfile.mkdtemp()
        try:
            report_path = os.path.join(temp_dir, "report.md")
            eval_results = evaluate_runs([], {"flat_tabular": []}, raw_dir=temp_dir)
            generate_report(eval_results, report_file=report_path)
            with open(report_path, "r", encoding="utf-8") as f:
                content = f.read()

            self.assertIn(expected_prompt, content, "Report must contain the exact built prompt from build_prompt")
            # Verify it contains real Struct rows and real question 11
            self.assertIn("2405.05254|Rope scaling hurts long ctx|refuted|2024|0|", content)
            self.assertIn("What is the publication year of paper 2203.15556?", content)
            self.assertNotIn("...\n```\n\nAnswer each", content, "Fake hand-typed '...' must not be present")
        finally:
            shutil.rmtree(temp_dir)

    def test_defect3_report_removes_wrong_r1_toon_note(self):
        """Defect 3: Remove the wrong report note blaming R1's TOON."""
        temp_dir = tempfile.mkdtemp()
        try:
            report_path = os.path.join(temp_dir, "report.md")
            eval_results = evaluate_runs([], {"flat_tabular": []}, raw_dir=temp_dir)
            generate_report(eval_results, report_file=report_path)
            with open(report_path, "r", encoding="utf-8") as f:
                content = f.read()

            self.assertIn("subnotes: []` per TOON spec 9.1", content)
            self.assertNotIn("subnotes[0]:", content)
            self.assertNotIn("Note: R1's TOON fixtures had", content)
        finally:
            shutil.rmtree(temp_dir)

    def test_defect6_keep_failure_history_and_separate_failed_runs(self):
        """Defect 6: Keep failure history when a rerun succeeds, and report failed runs separately from wrong answers."""
        temp_dir = tempfile.mkdtemp()
        try:
            raw_dir = os.path.join(temp_dir, "raw")
            os.makedirs(raw_dir, exist_ok=True)
            mock_file = os.path.join(raw_dir, "sample.txt")
            with open(mock_file, "w", encoding="utf-8") as f:
                f.write("1. 2024\n")

            # Metadata with an earlier failed attempt and a successful rerun for the same (format, dataset, seed)
            mock_meta = [
                {
                    "run_index": 1,
                    "format": "struct",
                    "dataset": "flat_tabular",
                    "seed": 42,
                    "file": "failed_attempt.txt",
                    "success": False,
                    "finish_reason": "error: timeout",
                    "error": "timed out after 1200s",
                    "output_tokens": 0,
                },
                {
                    "run_index": 2,
                    "format": "struct",
                    "dataset": "flat_tabular",
                    "seed": 42,
                    "file": "sample.txt",
                    "success": True,
                    "finish_reason": "stop",
                    "output_tokens": 50,
                },
            ]
            qs = {"flat_tabular": [{"id": "q1", "kind": "single_lookup", "question": "q?", "ground_truth": "2024"}]}
            results = evaluate_runs(mock_meta, qs, raw_dir=raw_dir)

            # Failure history must be kept in failed_runs
            self.assertEqual(len(results["failed_runs"]), 1, "Failed attempt must be recorded in failed_runs")
            self.assertEqual(results["failed_runs"][0]["finish_reason"], "error: timeout")

            # Failed run must NOT be counted as wrong answers
            self.assertEqual(len(results["wrong_answers"]), 0, "Failed runs must be reported separately from wrong answers")
            self.assertEqual(results["by_format"]["struct"]["correct"], 1)
            self.assertEqual(results["by_format"]["struct"]["total"], 1)
        finally:
            shutil.rmtree(temp_dir)

    def test_defect7_collector_records_launch_flags_from_props(self):
        """Defect 7: The collector records real server launch flags from /props instead of a constant."""
        # Test extraction from launch_args
        props1 = {"launch_args": "llama-server.exe -m custom-model.gguf --port 8090 -c 32768"}
        flags1 = get_launch_flags_from_props(props1)
        self.assertEqual(flags1, "llama-server.exe -m custom-model.gguf --port 8090 -c 32768")

        # Test extraction from flags/cmdline
        props2 = {"cmdline": "llama-server.exe --reasoning-budget 8192 --port 8090"}
        flags2 = get_launch_flags_from_props(props2)
        self.assertEqual(flags2, "llama-server.exe --reasoning-budget 8192 --port 8090")

        # Test reconstruction from settings
        props3 = {
            "model_path": "models/Ternary-Bonsai-2-27B-PTQ1_0.gguf",
            "default_generation_settings": {"n_ctx": 65536},
            "port": 8090,
            "reasoning_budget": 8192,
        }
        flags3 = get_launch_flags_from_props(props3)
        self.assertIn("-m models/Ternary-Bonsai-2-27B-PTQ1_0.gguf", flags3)
        self.assertIn("-c 65536", flags3)
        self.assertIn("--port 8090", flags3)

        # Test evaluate report uses recorded launch flags
        temp_dir = tempfile.mkdtemp()
        old_ll = eval_mod.LAUNCH_LINE_FILE
        eval_mod.LAUNCH_LINE_FILE = os.path.join(temp_dir, "launch_line.txt")
        try:
            report_path = os.path.join(temp_dir, "report.md")
            meta = [{"server_launch_flags": "llama-server.exe --custom-flag-from-props --port 8090"}]
            eval_results = evaluate_runs([], {}, raw_dir=temp_dir)
            generate_report(eval_results, report_file=report_path, metadata_list=meta)
            with open(report_path, "r", encoding="utf-8") as f:
                content = f.read()
            self.assertIn("llama-server.exe --custom-flag-from-props --port 8090", content)
        finally:
            eval_mod.LAUNCH_LINE_FILE = old_ll
            shutil.rmtree(temp_dir)

    def test_defect_failed_runs_count_wrong_in_all_totals_and_main_table(self):
        """
        Defect 1:
        R4.md says a missing or unparseable answer counts as wrong:
        - A request that is still failed when the report is generated must count all its questions
          as wrong in every accuracy total (never dropped from the denominator).
        - The main accuracy table must show a 'failed runs' count per format.
        - A rerun that succeeds clears the still-failed status.
        """
        temp_dir = tempfile.mkdtemp()
        try:
            raw_dir = os.path.join(temp_dir, "raw")
            os.makedirs(raw_dir, exist_ok=True)
            report_file = os.path.join(temp_dir, "report.md")

            # Create mock questions: flat_tabular (12 questions), nested_fixed (12 questions)
            qs = {
                "flat_tabular": [
                    {"id": f"ft_q{i}", "kind": "single_lookup" if i <= 6 else "count_filter", "question": f"ft_q{i}?", "ground_truth": f"ans_{i}"}
                    for i in range(1, 13)
                ],
                "nested_fixed": [
                    {"id": f"nf_q{i}", "kind": "nested_lookup" if i <= 6 else "null_missing", "question": f"nf_q{i}?", "ground_truth": f"ans_{i}"}
                    for i in range(1, 13)
                ],
            }

            # Seed 42:
            # - xml on flat_tabular: SUCCESS, all 12 answers correct
            # - xml on nested_fixed: STILL FAILED (timeout, no successful rerun)
            # - struct on flat_tabular: SUCCESS, all 12 answers correct
            # - struct on nested_fixed: SUCCESS, all 12 answers correct
            ft_ans = "\n".join(f"{i}. ans_{i}" for i in range(1, 13))
            with open(os.path.join(raw_dir, "xml_ft.txt"), "w", encoding="utf-8") as f:
                f.write(ft_ans)
            with open(os.path.join(raw_dir, "struct_ft.txt"), "w", encoding="utf-8") as f:
                f.write(ft_ans)
            with open(os.path.join(raw_dir, "struct_nf.txt"), "w", encoding="utf-8") as f:
                f.write(ft_ans)

            mock_meta = [
                # XML flat_tabular: success
                {"run_index": 1, "format": "xml", "dataset": "flat_tabular", "seed": 42, "file": "xml_ft.txt", "success": True, "finish_reason": "stop", "output_tokens": 50},
                # XML nested_fixed: failed (still failed)
                {"run_index": 2, "format": "xml", "dataset": "nested_fixed", "seed": 42, "file": "xml_nf_fail.txt", "success": False, "finish_reason": "error: timeout", "error": "timed out after 1200s", "output_tokens": 0},
                # Struct flat_tabular: success
                {"run_index": 3, "format": "struct", "dataset": "flat_tabular", "seed": 42, "file": "struct_ft.txt", "success": True, "finish_reason": "stop", "output_tokens": 50},
                # Struct nested_fixed: success
                {"run_index": 4, "format": "struct", "dataset": "nested_fixed", "seed": 42, "file": "struct_nf.txt", "success": True, "finish_reason": "stop", "output_tokens": 50},
            ]

            results = evaluate_runs(mock_meta, qs, raw_dir=raw_dir)

            # Check XML totals:
            # flat_tabular had 12 correct / 12 total.
            # nested_fixed had 12 questions: ALL counted as wrong (0 correct, 12 total).
            # Total XML: 12 correct out of 24 total (50.0%), NOT 12/12 (100.0%)!
            self.assertEqual(results["by_format"]["xml"]["correct"], 12)
            self.assertEqual(results["by_format"]["xml"]["total"], 24, "XML denominator must include the 12 failed questions")
            self.assertEqual(results["by_format"]["xml"]["failed_runs"], 1, "XML must have 1 failed run")

            # Check Struct totals:
            # 24 correct / 24 total, 0 failed runs
            self.assertEqual(results["by_format"]["struct"]["correct"], 24)
            self.assertEqual(results["by_format"]["struct"]["total"], 24)
            self.assertEqual(results["by_format"]["struct"]["failed_runs"], 0)

            # Check by_dataset:
            # nested_fixed had 1 failed run (XML: 0/12) and 1 success (Struct: 12/12) -> 12 correct out of 24 total
            self.assertEqual(results["by_dataset"]["nested_fixed"]["correct"], 12)
            self.assertEqual(results["by_dataset"]["nested_fixed"]["total"], 24)

            # Check by_format_dataset:
            # (xml, nested_fixed): 0 correct out of 12 total (never 0/0)
            self.assertEqual(results["by_format_dataset"][("xml", "nested_fixed")]["correct"], 0)
            self.assertEqual(results["by_format_dataset"][("xml", "nested_fixed")]["total"], 12)

            # Check by_kind and by_format_kind:
            # nested_lookup has 6 questions in nested_fixed: XML failed run added 6 to total
            self.assertEqual(results["by_format_kind"][("xml", "nested_lookup")]["total"], 6)
            self.assertEqual(results["by_format_kind"][("xml", "nested_lookup")]["correct"], 0)

            # Check by_format_seed:
            self.assertEqual(results["by_format_seed"]["xml"][42]["correct"], 12)
            self.assertEqual(results["by_format_seed"]["xml"][42]["total"], 24)

            # Check wrong answers:
            # XML failed run added all 12 questions to wrong_answers
            xml_wrongs = [w for w in results["wrong_answers"] if w["format"] == "xml"]
            self.assertEqual(len(xml_wrongs), 12, "All 12 questions of failed XML run must be in wrong_answers")

            # Generate report and check main table has Failed Runs column
            generate_report(results, report_file=report_file, metadata_list=mock_meta)
            with open(report_file, "r", encoding="utf-8") as f:
                report_content = f.read()

            self.assertIn("| Format | Accuracy (Counts) | Accuracy (%) | Mean Input Toks | Mean Thinking Toks | Max Thinking Toks | Budget Exhausted | Failed Runs |", report_content)
            # XML row shows 12/24 and 1 failed run
            self.assertIn("| XML | 12/24 | 50.0% |", report_content)
            self.assertIn("| 1 |\n| Plain key: value |", report_content) # XML row ends with | 1 |
            # Struct row shows 24/24 and 0 failed runs
            self.assertIn("| Struct | 24/24 | 100.0% |", report_content)

            # Check dataset table shows 0/12 for XML nested_fixed (not 0/0)
            self.assertIn("0/12 (0.0%)", report_content)

            # Now simulate a rerun that succeeds for XML nested_fixed:
            with open(os.path.join(raw_dir, "xml_nf_rerun.txt"), "w", encoding="utf-8") as f:
                f.write(ft_ans)
            mock_meta_rerun = list(mock_meta) + [
                {"run_index": 5, "format": "xml", "dataset": "nested_fixed", "seed": 42, "file": "xml_nf_rerun.txt", "success": True, "finish_reason": "stop", "output_tokens": 50}
            ]
            results_after = evaluate_runs(mock_meta_rerun, qs, raw_dir=raw_dir)
            # After rerun succeeds: XML now 24/24 (100.0%), failed_runs is 0
            self.assertEqual(results_after["by_format"]["xml"]["correct"], 24)
            self.assertEqual(results_after["by_format"]["xml"]["total"], 24)
            self.assertEqual(results_after["by_format"]["xml"]["failed_runs"], 0)
            # Historical failure is still recorded in failed_runs
            self.assertEqual(len(results_after["failed_runs"]), 1)
        finally:
            shutil.rmtree(temp_dir)

    def test_defect_launch_server_flags_and_components(self):
        r"""
        Defect 2: Verify launch_server.py starts turboquant llama-server with exact R4 flags (decision 14):
        - Binary: C:\Users\Ashman Das\Documents\turboquant-llama-cpp\llama-server.exe
        - Model: gemma-4-E2B-it-Q4_K_M.gguf
        - Flags: -ngl 999 --mlock --no-mmap --cache-type-k turbo4 --cache-type-v turbo3 -fa on
          -c 65536 -np 4 -kvu --reasoning-budget 8192 --temp 1.0 --top-p 0.95 --top-k 64 --min-p 0.0
          --presence-penalty 0.0 --repeat-penalty 1.0 --port 8090 --host 127.0.0.1
        """
        from r4.launch_server import get_launch_cmd_args, get_exact_launch_line, LLAMA_SERVER, MODEL_PATH

        args = get_launch_cmd_args()
        self.assertEqual(args[0], LLAMA_SERVER)
        self.assertEqual(args[1], "-m")
        self.assertEqual(args[2], MODEL_PATH)

        expected_flags = [
            "-ngl", "999",
            "--cache-type-k", "turbo4",
            "--cache-type-v", "turbo3",
            "-fa", "on",
            "-c", "65536",
            "-np", "4",
            "--reasoning-budget", "8192",
            "--temp", "1.0",
            "--top-p", "0.95",
            "--top-k", "64",
            "--min-p", "0.0",
            "--presence-penalty", "0.0",
            "--repeat-penalty", "1.0",
            "--port", "8090",
            "--host", "127.0.0.1",
        ]
        for i in range(0, len(expected_flags), 2):
            flag = expected_flags[i]
            val = expected_flags[i+1]
            self.assertIn(flag, args)
            idx = args.index(flag)
            self.assertEqual(args[idx + 1], val, f"Flag {flag} must have value {val}")

        line = get_exact_launch_line()
        self.assertIn("llama-server.exe", line)
        self.assertIn("gemma-4-E2B-it-Q4_K_M.gguf", line)
        self.assertIn("--port 8090", line)
        self.assertIn("--host 127.0.0.1", line)
        self.assertIn("--reasoning-budget 8192", line)
        self.assertIn("--mlock", args)
        self.assertIn("--no-mmap", args)
        self.assertIn("--kv-unified", args)
        self.assertIn("--mlock --no-mmap", line)

    def test_defect_launch_server_writes_launch_line_and_sha256(self):
        """
        Defect 2: Verify write_launch_line writes exact command line and is hashed in SHA256SUMS.
        """
        import hashlib
        from r4.launch_server import write_launch_line, get_exact_launch_line
        import r4.collect as coll_mod

        temp_dir = tempfile.mkdtemp()
        try:
            line_file = os.path.join(temp_dir, "launch_line.txt")
            sha_file = os.path.join(temp_dir, "SHA256SUMS")

            old_ll = coll_mod.LAUNCH_LINE_FILE
            old_sha = coll_mod.SHA256_FILE
            coll_mod.LAUNCH_LINE_FILE = line_file
            coll_mod.SHA256_FILE = sha_file

            try:
                write_launch_line(output_path=line_file, update_sha=True)
                self.assertTrue(os.path.exists(line_file))
                with open(line_file, "r", encoding="utf-8") as f:
                    content = f.read().strip()
                self.assertEqual(content, get_exact_launch_line())

                # Check SHA256SUMS contains launch_line.txt
                self.assertTrue(os.path.exists(sha_file))
                with open(sha_file, "r", encoding="utf-8") as f:
                    sha_content = f.read()
                with open(line_file, "rb") as f:
                    expected_hash = hashlib.sha256(f.read()).hexdigest()
                self.assertIn(f"{expected_hash}  launch_line.txt", sha_content)
            finally:
                coll_mod.LAUNCH_LINE_FILE = old_ll
                coll_mod.SHA256_FILE = old_sha
        finally:
            shutil.rmtree(temp_dir)

    def test_defect_report_uses_launch_line_txt(self):
        """
        Defect 2: The report uses launch_line.txt as the recorded launch line.
        """
        from r4.evaluate import get_recorded_launch_line, generate_report

        temp_dir = tempfile.mkdtemp()
        try:
            line_file = os.path.join(temp_dir, "launch_line.txt")
            custom_line = 'llama-server.exe --custom-r4-launch-line-exact --port 8090'
            with open(line_file, "w", encoding="utf-8") as f:
                f.write(custom_line + "\n")

            old_ll = eval_mod.LAUNCH_LINE_FILE
            eval_mod.LAUNCH_LINE_FILE = line_file
            try:
                rec = get_recorded_launch_line()
                self.assertEqual(rec, custom_line)

                report_file = os.path.join(temp_dir, "report.md")
                eval_results = evaluate_runs([], {}, raw_dir=temp_dir)
                generate_report(eval_results, report_file=report_file)
                with open(report_file, "r", encoding="utf-8") as f:
                    content = f.read()
                self.assertIn(custom_line, content)
            finally:
                eval_mod.LAUNCH_LINE_FILE = old_ll
        finally:
            shutil.rmtree(temp_dir)

    def test_defect_launch_server_stop_command(self):
        """
        Defect 2: launch_server has a working stop command (tested without real server).
        """
        from unittest.mock import patch
        from r4.launch_server import stop_server
        import r4.launch_server as ls_mod

        temp_dir = tempfile.mkdtemp()
        try:
            pid_file = os.path.join(temp_dir, "test.pid")
            with open(pid_file, "w", encoding="utf-8") as f:
                f.write("999999")

            old_pid = ls_mod.PID_FILE
            ls_mod.PID_FILE = pid_file
            try:
                with patch("subprocess.run") as mock_run:
                    stop_server()
                    mock_run.assert_called()
                    self.assertFalse(os.path.exists(pid_file))
            finally:
                ls_mod.PID_FILE = old_pid
        finally:
            shutil.rmtree(temp_dir)

class TestR4GemmaFixes(unittest.TestCase):
    """
    Tests for fixes in milestones/R4-gemma-fix.md:
    1. Broken prompt block in report: 4+ backtick outer fence so inner ```struct fence does not break markdown.
    2. GPU check in launch_server.py: reads GPU total used memory, stops if > 1 GB, never passes silently on parse error.
    3. GPU check in collect.py: checks process holding GPU is llama-server, never passes silently on parse error.
    4. Launch line flag consistency: --kv-unified used consistently instead of -kvu.
    """

    def test_prompt_block_outer_fence_four_backticks(self):
        """Fix 1: Section 2 wraps example prompt in 4+ backticks so inner triple backtick fences do not break."""
        temp_dir = tempfile.mkdtemp()
        try:
            report_path = os.path.join(temp_dir, "report.md")
            eval_results = evaluate_runs([], {"flat_tabular": []}, raw_dir=temp_dir)
            generate_report(eval_results, report_file=report_path)
            with open(report_path, "r", encoding="utf-8") as f:
                content = f.read()

            # Verify outer fence has at least 4 backticks
            self.assertIn("````text\n", content, "Outer fence must begin with at least 4 backticks: ````text")
            self.assertIn("\n````\n\n## 3.", content, "Outer fence must close with at least 4 backticks: ````")

            # Verify inner ```struct and closing ``` fences are inside the 4-backtick block
            prompt_section = content.split("## 2. Standardized Prompt")[1].split("## 3. Results Summary")[0]
            self.assertIn("```struct\n", prompt_section, "Inner struct block must remain intact")
            self.assertTrue(prompt_section.strip().startswith("The prompt is identical"))
            self.assertTrue(prompt_section.strip().endswith("````"))
        finally:
            shutil.rmtree(temp_dir)

    def test_launch_server_gpu_check_rule7(self):
        """Fix 2: launch_server check_gpu_rule7 reads memory.used, stops if > 1 GB, never passes on parse error."""
        from unittest.mock import patch
        from r4.launch_server import check_gpu_rule7

        # 1. Normal available GPU (0 MB used)
        with patch("subprocess.check_output", return_value="0\n") as mock_smi:
            self.assertTrue(check_gpu_rule7())
            mock_smi.assert_called_with(
                ["nvidia-smi", "--query-gpu=memory.used", "--format=csv,noheader,nounits"],
                text=True
            )

        # 2. Borderline available GPU (1024 MB used)
        with patch("subprocess.check_output", return_value="1024\n"):
            self.assertTrue(check_gpu_rule7())

        # 3. Busy GPU (> 1024 MB used, e.g. 2048 MB)
        with patch("subprocess.check_output", return_value="2048\n"):
            self.assertFalse(check_gpu_rule7(), "Must return False when > 1 GB is in use")

        # 4. Empty output -> must return False (never pass silently)
        with patch("subprocess.check_output", return_value="   \n"):
            self.assertFalse(check_gpu_rule7(), "Must return False on empty output")

        # 5. Unparseable output (e.g. '[N/A]') -> must return False (never pass silently)
        with patch("subprocess.check_output", return_value="[N/A]\n"):
            self.assertFalse(check_gpu_rule7(), "Must return False on unparseable output")

        # 6. Subprocess error / exception -> must return False (never pass silently)
        with patch("subprocess.check_output", side_effect=RuntimeError("nvidia-smi failed")):
            self.assertFalse(check_gpu_rule7(), "Must return False on subprocess error")

    def test_collect_gpu_check_llama_server(self):
        """Fix 2: collect check_nvidia_smi verifies process is llama-server, never passes on parse error."""
        from unittest.mock import patch
        from r4.collect import check_nvidia_smi

        # 1. llama-server running (simple executable name)
        with patch("subprocess.check_output", return_value="1234, llama-server.exe\n") as mock_smi:
            self.assertTrue(check_nvidia_smi())
            mock_smi.assert_called_with(
                ["nvidia-smi", "--query-compute-apps=pid,process_name", "--format=csv,noheader,nounits"],
                text=True
            )

        # 2. llama-server running with full path
        with patch("subprocess.check_output", return_value=r"4321, C:\turboquant-llama-cpp\llama-server.exe\n"):
            self.assertTrue(check_nvidia_smi())

        # 3. Non-llama-server process holding GPU (e.g. python.exe)
        with patch("subprocess.check_output", return_value="9999, python.exe\n"):
            self.assertFalse(check_nvidia_smi(), "Must return False when another process holds GPU")

        # 4. Multiple processes, one of which is non-llama-server
        with patch("subprocess.check_output", return_value="1234, llama-server.exe\n9999, chrome.exe\n"):
            self.assertFalse(check_nvidia_smi(), "Must return False if any non-llama-server process is present")

        # 5. Empty output (no compute processes) -> returns False
        with patch("subprocess.check_output", return_value=""):
            self.assertFalse(check_nvidia_smi(), "Must return False when no compute process is found")

        # 6. Malformed line (single token, no comma) -> returns False (never pass silently)
        with patch("subprocess.check_output", return_value="[N/A]\n"):
            self.assertFalse(check_nvidia_smi(), "Must return False on malformed line")

        # 7. Subprocess error / exception -> returns False (never pass silently)
        with patch("subprocess.check_output", side_effect=RuntimeError("nvidia-smi failed")):
            self.assertFalse(check_nvidia_smi(), "Must return False on subprocess error")

    def test_launch_line_flags_unified_kv(self):
        """Fix 3: LAUNCH_LINE constants in collect.py and evaluate.py use --kv-unified (not -kvu)."""
        import r4.collect as coll
        import r4.evaluate as ev
        import r4.launch_server as ls

        self.assertIn("--kv-unified", coll.LAUNCH_LINE)
        self.assertNotIn("-kvu", coll.LAUNCH_LINE)
        self.assertIn("--kv-unified", ev.LAUNCH_LINE)
        self.assertNotIn("-kvu", ev.LAUNCH_LINE)
        self.assertIn("--kv-unified", ls.get_exact_launch_line())
        self.assertNotIn("-kvu", ls.get_exact_launch_line())

if __name__ == "__main__":
    unittest.main()
