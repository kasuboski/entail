"""Focused unit tests for the black-box acceptance assertions."""
from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scripts import acceptance


class AcceptanceHelpersTest(unittest.TestCase):
    def test_nested_typed_obligation_kind_is_recognized(self) -> None:
        obligation = {"obligation": {"kind": "constraint_compliance"}, "status": "finding",
                      "finding_category": "violation", "rule_id": "R", "constraint_id": "C"}
        result = {"result": {"obligations": [obligation]}}
        self.assertEqual(acceptance.statuses_for(result, "constraint_compliance"), [obligation])
        acceptance.expect_check_label(result, "constraint_compliance", "violation", "nested")

    def test_labels_in_provenance_are_not_results(self) -> None:
        with self.assertRaises(acceptance.AcceptanceFailure):
            acceptance.expect_check_label({"finding": {"label": "coverage", "provenance": "uncovered"}},
                                          "coverage", "uncovered", "must not match text")

    def test_status_and_category_are_exact_values(self) -> None:
        good = {"obligation": {"kind": "coverage"}, "status": "finding", "finding_category": "uncovered"}
        wrong = {"obligation": {"kind": "coverage"}, "status": "satisfied", "finding_category": "covered"}
        acceptance.expect_check_label([good], "coverage", "uncovered", "exact")
        with self.assertRaises(acceptance.AcceptanceFailure):
            acceptance.expect_check_label([wrong], "coverage", "uncovered", "wrong")

    def test_compliance_requires_exact_rule_and_constraint_ids(self) -> None:
        result = {"obligations": [
            {"obligation": {"kind": "constraint_compliance", "rule_id": "R",
                             "constraint_id": "C-extra"}, "status": "satisfied", "source_ids": ["C"]}
        ]}
        with self.assertRaises(acceptance.AcceptanceFailure):
            acceptance._check_compliance(result, "unsafe-grant", {"ExpiryWithinContract": "violation"})

    def test_subprocess_timeout_is_bounded(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "hang"
            binary.write_text("#!/usr/bin/env python3\\nimport time; time.sleep(5)\\n", encoding="utf-8")
            binary.chmod(0o755)
            old_timeout = acceptance.TIMEOUT_SECONDS
            acceptance.TIMEOUT_SECONDS = 0.05
            try:
                with self.assertRaises(acceptance.AcceptanceFailure):
                    acceptance.run_json(binary, [])
            finally:
                acceptance.TIMEOUT_SECONDS = old_timeout


if __name__ == "__main__":
    unittest.main()
