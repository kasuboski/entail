#!/usr/bin/env python3
"""End-user acceptance checks for the entail CLI (not an engine mock).

Usage: python3 scripts/acceptance.py /path/to/entail
The binary must implement the CLI contract in author-engine-v0-spec.md.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Iterable

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / "author-engine-v0"
CHECKS = ["domain_feasibility", "rule_reachability", "coverage", "outcome_conflict", "constraint_compliance"]


class AcceptanceFailure(AssertionError):
    pass


def load(path: Path) -> Any:
    with path.open(encoding="utf-8") as stream:
        return json.load(stream)


def run_json(binary: Path, args: list[str], expected_exit: int = 0) -> Any:
    completed = subprocess.run([str(binary), *args], cwd=ROOT, text=True, capture_output=True, check=False)
    if completed.returncode != expected_exit:
        raise AcceptanceFailure(
            f"{binary.name} {' '.join(args)} exited {completed.returncode}, expected {expected_exit}\n"
            f"stdout: {completed.stdout[-4000:]}\nstderr: {completed.stderr[-4000:]}"
        )
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise AcceptanceFailure(f"command did not print JSON: {args}\n{completed.stdout[-4000:]}\n{completed.stderr[-2000:]}") from exc


def objects(value: Any) -> Iterable[dict[str, Any]]:
    """Yield JSON objects recursively; result schemas may add envelope layers."""
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from objects(child)
    elif isinstance(value, list):
        for child in value:
            yield from objects(child)


def text_values(value: Any) -> set[str]:
    return {str(item).lower() for obj in objects(value) for item in obj.values() if isinstance(item, (str, int, float))}


def statuses_for(result: Any, check: str) -> list[dict[str, Any]]:
    aliases = {check.lower(), check.lower().replace("_", "")}
    found = []
    for obj in objects(result):
        labels = [obj.get("check_kind"), obj.get("check"), obj.get("obligation_kind")]
        if any(str(label).lower() in aliases or str(label).lower().replace("_", "") in aliases for label in labels if label is not None):
            found.append(obj)
    return found


def expect_check_label(result: Any, check: str, label: str, context: str) -> None:
    matches = statuses_for(result, check)
    observed = set()
    for item in matches:
        for key in ("status", "finding_category", "category", "result"):
            value = item.get(key)
            if isinstance(value, str):
                observed.add(value.lower())
    if not matches or label.lower() not in observed:
        raise AcceptanceFailure(f"{context}: expected {check} status/category {label!r}; observed {sorted(observed)} in {matches!r}")


def all_strings(result: Any) -> set[str]:
    return text_values(result)


def expect_label(result: Any, label: str, context: str) -> None:
    if label.lower() not in all_strings(result):
        raise AcceptanceFailure(f"{context}: expected stable status/category {label!r}; observed strings: {sorted(all_strings(result))}")


def expect_diagnostic(result: Any, code: str, context: str) -> None:
    codes = {str(obj.get("code", obj.get("diagnostic_code", ""))).lower() for obj in objects(result)}
    if code.lower() not in codes:
        raise AcceptanceFailure(f"{context}: expected diagnostic code {code!r}; observed codes: {sorted(codes)}")


def analyze(binary: Path, fixture: str, checks: list[str] | None = None, assumptions: list[str] | None = None) -> Any:
    model = BUNDLE / "fixtures" / "valid" / f"{fixture}.json"
    args = ["analyze", str(model), "--checks", ",".join(checks or CHECKS)]
    if assumptions:
        args += ["--assumptions", ",".join(assumptions)]
    return run_json(binary, args)


def check_fixture(binary: Path, fixture: str, expected: dict[str, Any]) -> None:
    for invalid_name, diagnostic in load(BUNDLE / "fixtures" / "expectations.json")["invalid"].items():
        if invalid_name == fixture:
            result = run_json(binary, ["validate", str(BUNDLE / "fixtures" / "invalid" / f"{fixture}.json")], expected_exit=2)
            expect_diagnostic(result, diagnostic["expected_diagnostic"], fixture)
            return
    result = analyze(binary, fixture)
    # Compare the semantics contract, locating check result objects by their stable identifiers.
    for check, expected_value in expected.items():
        if check in {"business_policy_confirmed", "assumption_ids", "conditional", "note", "large_scenario_output", "conflict_constructor"}:
            continue
        if check == "compliance":
            for declaration, outcome in expected_value.items():
                matches = [o for o in objects(result) if declaration in str(o.get("declaration_ids", o.get("declaration_id", o.get("id", ""))))]
                if not matches:
                    raise AcceptanceFailure(f"{fixture}: no result references expected constraint {declaration}")
                if not any(str(m.get("status", "")).lower() == outcome or outcome in text_values(m) for m in matches):
                    raise AcceptanceFailure(f"{fixture}: {declaration} expected {outcome}, got {matches}")
        elif check == "dependent_checks":
            # Each dependent check must be skipped; the result may represent this as an overall status.
            skipped = [o for o in objects(result) if str(o.get("status", "")).lower() == "skipped"]
            if not skipped and str(expected_value).lower() not in all_strings(result):
                raise AcceptanceFailure(f"{fixture}: expected dependent checks skipped; observed {result!r}")
        else:
            expect_check_label(result, check, str(expected_value), f"{fixture}/{check}")
    if expected.get("conditional"):
        selected = {str(v) for o in objects(result) for key in ("selected_assumption_ids", "assumption_ids") for v in (o.get(key, []) if isinstance(o.get(key, []), list) else [])}
        if not set(expected["assumption_ids"]).issubset(selected):
            raise AcceptanceFailure(f"{fixture}: selected assumptions not exposed in result: {selected}")
        if not any(bool(o.get("conditional")) for o in objects(result)) and "conditional" not in all_strings(result):
            raise AcceptanceFailure(f"{fixture}: selected assumption context not marked conditional")
    if "conflict_constructor" in expected:
        if expected["conflict_constructor"].lower() not in all_strings(result):
            raise AcceptanceFailure(f"{fixture}: conflict constructor not reported")


def run_acceptance(binary: Path) -> int:
    if not binary.exists():
        raise AcceptanceFailure(f"binary does not exist: {binary}")
    expectations = load(BUNDLE / "fixtures" / "expectations.json")
    completed = 0
    for fixture, expected in expectations["valid"].items():
        if fixture == "big-integer":
            check_fixture(binary, fixture, expected)
            scenario = BUNDLE / "scenarios" / "large-integer.json"
            evaluated = run_json(binary, ["evaluate", str(BUNDLE / "fixtures/valid/big-integer.json"), str(scenario)])
            if expected["large_scenario_output"] not in all_strings(evaluated):
                raise AcceptanceFailure("big-integer scenario output did not preserve the exact arbitrary-precision value")
        elif fixture == "conditional-assumption":
            # Check both contexts: assumptions are never silently selected.
            no_assumption = analyze(binary, fixture)
            if "conditional" in all_strings(no_assumption) or any(o.get("selected_assumption_ids") for o in objects(no_assumption)):
                raise AcceptanceFailure("assumptions were selected implicitly")
            check_fixture(binary, fixture, expected)
        else:
            check_fixture(binary, fixture, expected)
        completed += 1

    for fixture, expectation in expectations["invalid"].items():
        result = run_json(binary, ["validate", str(BUNDLE / "fixtures" / "invalid" / f"{fixture}.json")], expected_exit=2)
        expect_diagnostic(result, expectation["expected_diagnostic"], fixture)
        completed += 1

    # Concrete evaluator acceptance: outside/unknown inputs are not silently interpreted as approval.
    for model_fixture, scenario in (("unsafe-grant", "unsafe-expiry"), ("unsafe-grant", "unknown-approval")):
        evaluation = run_json(binary, ["evaluate", str(BUNDLE / "fixtures" / "valid" / f"{model_fixture}.json"), str(BUNDLE / "scenarios" / f"{scenario}.json")])
        if scenario == "unsafe-expiry":
            expect_label(evaluation, "ExpiryWithinContract", "unsafe-expiry evaluator")
        else:
            expect_label(evaluation, "unspecified", "unknown approval must not grant")
        completed += 1
    print(f"PASS: {completed} fixture/scenario acceptance groups against {binary}")
    return completed


def self_tests() -> None:
    # Small unit checks for dynamic result traversal; no fake entail binary or pass claim.
    sample = {"result": {"obligations": [{"check_kind": "coverage", "status": "finding", "finding_category": "uncovered"}]}}
    expect_label(sample, "uncovered", "self-test")
    assert statuses_for(sample, "coverage")
    print("PASS: acceptance helper self-tests (not application acceptance)")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", nargs="?", type=Path, help="path to the built entail executable")
    parser.add_argument("--self-test", action="store_true", help="test acceptance helper logic only")
    args = parser.parse_args()
    try:
        if args.self_test:
            self_tests()
            return 0
        if args.binary is None:
            parser.error("provide the entail binary path")
        run_acceptance(args.binary.resolve())
        return 0
    except (AcceptanceFailure, OSError, ValueError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
