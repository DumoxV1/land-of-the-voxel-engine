from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class OpenRouterGuardTests(unittest.TestCase):
    def test_classifies_budget_boundaries_and_never_persists_secret(self):
        guard = load_module("openrouter_guard_test", ROOT / "scripts" / "openrouter_guard.py")
        cases = ((0, "OK", 0), (10, "REVIEW", 0), (22, "BLOCKERS-ONLY", 0), (30, "PAID-DICHT", 0), (36, "STOP", 1))

        for usage, expected_status, expected_exit in cases:
            with self.subTest(usage=usage), tempfile.TemporaryDirectory() as output_dir:
                guard.OUT = Path(output_dir)
                guard.load_openrouter_key = lambda: "fake-secret"

                def fake_fetch(url: str, api_key: str | None = None, current=usage):
                    if url.endswith("/models"):
                        return {
                            "data": [
                                {
                                    "id": "free-tool-model",
                                    "pricing": {"prompt": "0", "completion": "0"},
                                    "supported_parameters": ["tools"],
                                },
                                {
                                    "id": "paid-model",
                                    "pricing": {"prompt": "1", "completion": "1"},
                                    "supported_parameters": ["tools"],
                                },
                            ]
                        }
                    self.assertEqual(api_key, "fake-secret")
                    return {
                        "data": {
                            "usage": 100 + current,
                            "limit": 40,
                            "limit_remaining": 40 - current,
                        }
                    }

                guard.fetch_json = fake_fetch
                stdout = io.StringIO()
                with contextlib.redirect_stdout(stdout):
                    exit_code = guard.main()
                report = json.loads((Path(output_dir) / "openrouter-latest.json").read_text(encoding="utf-8"))

                self.assertEqual((report["status"], exit_code), (expected_status, expected_exit))
                self.assertEqual(report["free_tool_model_count"], 1)
                self.assertEqual(report["inference_calls"], 0)
                self.assertNotIn("fake-secret", stdout.getvalue())
                self.assertNotIn("fake-secret", json.dumps(report))


class PlanAlignmentTests(unittest.TestCase):
    def test_current_workspace_is_aligned(self):
        alignment = load_module("plan_alignment_test", ROOT / "scripts" / "plan_alignment_check.py")
        self.assertEqual(alignment.main(), 0)
        report = json.loads((ROOT / ".hermes" / "reports" / "plan-alignment-latest.json").read_text(encoding="utf-8"))
        self.assertTrue(report["aligned"])

    def test_missing_required_term_fails_closed(self):
        alignment = load_module("plan_alignment_negative_test", ROOT / "scripts" / "plan_alignment_check.py")
        original = alignment.REQUIRED_PLAN_TERMS
        try:
            alignment.REQUIRED_PLAN_TERMS = original + ["definitely-absent-test-term"]
            self.assertEqual(alignment.main(), 1)
        finally:
            alignment.REQUIRED_PLAN_TERMS = original
            alignment.main()


if __name__ == "__main__":
    unittest.main()
