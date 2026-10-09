#!/usr/bin/env python3
"""Black-box end-user acceptance checks for the real entail CLI."""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Iterable

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / "author-engine-v0"
CHECKS = ["domain_feasibility", "rule_reachability", "coverage", "outcome_conflict", "constraint_compliance"]
TIMEOUT_SECONDS = 20


class AcceptanceFailure(AssertionError):
    pass


def load(path: Path) -> Any:
    with path.open(encoding="utf-8") as stream:
        return json.load(stream)


def run_json(binary: Path, args: list[str], expected_exit: int = 0) -> Any:
    try:
        completed = subprocess.run([str(binary), *args], cwd=ROOT, text=True, capture_output=True,
                                   check=False, timeout=TIMEOUT_SECONDS)
    except subprocess.TimeoutExpired as exc:
        raise AcceptanceFailure(f"command timed out after {TIMEOUT_SECONDS}s: {binary} {' '.join(args)}") from exc
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
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from objects(child)
    elif isinstance(value, list):
        for child in value:
            yield from objects(child)


def obligation_kind(obj: dict[str, Any]) -> str | None:
    """Read the typed obligation descriptor, including result.obligation.kind."""
    for key in ("check_kind", "check", "obligation_kind", "kind"):
        value = obj.get(key)
        if isinstance(value, str):
            return value
    for key in ("obligation", "descriptor"):
        nested = obj.get(key)
        if isinstance(nested, dict):
            for field in ("check_kind", "kind", "check"):
                if isinstance(nested.get(field), str):
                    return nested[field]
    return None


def _norm(value: str) -> str:
    return value.lower().replace("_", "").replace("-", "")


def statuses_for(result: Any, check: str) -> list[dict[str, Any]]:
    aliases = {_norm(check)}
    return [obj for obj in objects(result)
            if (kind := obligation_kind(obj)) is not None and _norm(kind) in aliases
            and (_status(obj) is not None or _category(obj) is not None)]


def _status(obj: dict[str, Any]) -> str | None:
    value = obj.get("status")
    if isinstance(value, str):
        return value.lower()
    return None


def _category(obj: dict[str, Any]) -> str | None:
    for key in ("finding_category", "category", "result"):
        value = obj.get(key)
        if isinstance(value, str):
            return value.lower()
    return None


def expect_check_label(result: Any, check: str, label: str, context: str) -> list[dict[str, Any]]:
    matches = statuses_for(result, check)
    observed = [value for item in matches for value in (_status(item), _category(item)) if value is not None]
    wanted = label.lower()
    # A check can contain many typed obligations (for example several rule
    # pairs); an expected finding does not require every unrelated pair to find.
    if not matches or not any(wanted in (_status(item), _category(item)) for item in matches):
        raise AcceptanceFailure(f"{context}: expected typed {check} status/category {label!r}; observed {observed!r}")
    return matches


def exact_ids(obj: dict[str, Any], singular: str, plural: str) -> set[str]:
    values: set[str] = set()
    # Serde may put typed obligation identifiers in the descriptor rather than
    # repeat them at the result envelope. Search only typed ID-bearing fields.
    for candidate in (obj, obj.get("obligation"), obj.get("descriptor")):
        if not isinstance(candidate, dict):
            continue
        one = candidate.get(singular)
        if isinstance(one, str):
            values.add(one)
        many = candidate.get(plural)
        if isinstance(many, list):
            values.update(item for item in many if isinstance(item, str))
        if isinstance(candidate.get("declaration_ids"), list):
            values.update(item for item in candidate["declaration_ids"] if isinstance(item, str))
    return values


def analyze(binary: Path, fixture: str, checks: list[str] | None = None, assumptions: list[str] | None = None) -> Any:
    model = BUNDLE / "fixtures" / "valid" / f"{fixture}.json"
    args = ["analyze", str(model), "--checks", ",".join(checks or CHECKS)]
    if assumptions is not None:
        args += ["--assumptions", ",".join(assumptions)]
    return run_json(binary, args)


def _check_compliance(result: Any, fixture: str, expected: dict[str, str]) -> None:
    model = load(BUNDLE / "fixtures" / "valid" / f"{fixture}.json")
    constraints = {c["id"]: c["for_outcome"] for c in model.get("constraints", [])}
    rules = {r["id"]: r["then"]["outcome"] for r in model.get("rules", [])}
    entries = statuses_for(result, "constraint_compliance")
    for constraint_id, outcome in expected.items():
        if constraint_id not in constraints:
            raise AcceptanceFailure(f"{fixture}: expectation references absent constraint {constraint_id}")
        target_outcome = constraints[constraint_id]
        expected_rules = {rule_id for rule_id, rule_outcome in rules.items() if rule_outcome == target_outcome}
        if not expected_rules:
            raise AcceptanceFailure(f"{fixture}: no rules target expected constraint outcome {outcome}")
        for rule_id in sorted(expected_rules):
            matches = [o for o in entries
                       if constraint_id in exact_ids(o, "constraint_id", "constraint_ids")
                       and rule_id in exact_ids(o, "rule_id", "rule_ids")]
            if not matches:
                raise AcceptanceFailure(f"{fixture}: missing compliance result for exact rule={rule_id}, constraint={constraint_id}; got {entries!r}")
            observed = {_category(o) for o in matches}
            observed_statuses = {_status(o) for o in matches}
            wanted = expected[constraint_id]
            wanted_status = "finding" if wanted == "violation" else "satisfied"
            # Compliance is obligation-specific; category and status must both agree.
            if observed != {wanted} or observed_statuses != {wanted_status}:
                raise AcceptanceFailure(
                    f"{fixture}: {rule_id}/{constraint_id} expected {wanted_status}/{wanted}, "
                    f"got {observed_statuses}/{observed}"
                )


def _selected_ids(result: Any) -> set[str]:
    found: set[str] = set()
    for obj in objects(result):
        for key in ("selected_assumption_ids", "assumption_ids"):
            value = obj.get(key)
            if isinstance(value, list):
                found.update(item for item in value if isinstance(item, str))
    return found


def _validate_readiness(result: Any, fixture: str, expected: dict[str, Any]) -> None:
    if fixture == "complete-proposed":
        reports = [obj for obj in objects(result) if "complete" in obj and "results" in obj]
        if not reports or any(obj.get("complete") is not True for obj in reports):
            raise AcceptanceFailure("complete-proposed: expected complete=true on the analysis report")
        if any(obj.get("business_verified") is True for obj in objects(result)):
            raise AcceptanceFailure("complete-proposed: a solver result must not claim business_verified=true")
        if not any(isinstance(obj.get("open_items"), list) and obj["open_items"] for obj in objects(result)):
            raise AcceptanceFailure("complete-proposed: expected unresolved open items to remain visible")
        if not any(
            any(
                isinstance(obj.get(key), list) and obj[key]
                for key in ("proposed_declarations", "participating_proposed_declaration_ids")
            )
            for obj in objects(result)
        ):
            raise AcceptanceFailure("complete-proposed: expected proposed declaration IDs in context")
    report = next((obj for obj in objects(result) if "results" in obj and "selected_assumption_ids" in obj), None)
    if report is None:
        raise AcceptanceFailure(f"{fixture}: analysis report context is missing")
    model = load(BUNDLE / "fixtures/valid" / f"{fixture}.json")
    if report.get("model_id") != model["model_id"]:
        raise AcceptanceFailure(f"{fixture}: report model_id does not match submitted model")
    if not isinstance(report.get("content_hash"), str) or not report["content_hash"] or not isinstance(report.get("semantic_hash"), str) or not report["semantic_hash"]:
        raise AcceptanceFailure(f"{fixture}: report hashes are missing")
    if not isinstance(report.get("open_items"), list) or report["open_items"] != model.get("open_items", []):
        raise AcceptanceFailure(f"{fixture}: report open items do not match submitted model metadata")


def check_fixture(binary: Path, fixture: str, expected: dict[str, Any], assumptions: list[str] | None = None) -> Any:
    invalid = load(BUNDLE / "fixtures" / "expectations.json")["invalid"]
    if fixture in invalid:
        result = run_json(binary, ["validate", str(BUNDLE / "fixtures" / "invalid" / f"{fixture}.json")], expected_exit=2)
        expect_diagnostic(result, invalid[fixture]["expected_diagnostic"], fixture)
        return result
    result = analyze(binary, fixture, assumptions=assumptions)
    if expected.get("domain_feasibility") == "empty_domain":
        expect_check_label(result, "domain_feasibility", "empty_domain", fixture)
    else:
        feasible = expect_check_label(result, "domain_feasibility", "feasible", fixture)
        if any(_status(item) != "satisfied" or _category(item) != "feasible" for item in feasible):
            raise AcceptanceFailure(f"{fixture}: domain feasibility must be satisfied/feasible exactly")
    model = load(BUNDLE / "fixtures/valid" / f"{fixture}.json")
    reachability = statuses_for(result, "rule_reachability")
    for rule in model.get("rules", []):
        rule_results = [item for item in reachability if rule["id"] in exact_ids(item, "rule_id", "rule_ids")]
        if expected.get("domain_feasibility") == "empty_domain":
            if len(rule_results) != 1 or _status(rule_results[0]) != "skipped":
                raise AcceptanceFailure(f"{fixture}: empty domain must skip reachability for {rule['id']}: {rule_results!r}")
        elif (len(rule_results) != 1 or _status(rule_results[0]) != "satisfied"
              or _category(rule_results[0]) not in {"reachable", "unreachable"}):
            raise AcceptanceFailure(f"{fixture}: invalid reachability result for {rule['id']}: {rule_results!r}")
    for check in ("domain_feasibility", "coverage", "outcome_conflict"):
        if check in expected:
            expect_check_label(result, check, expected[check], f"{fixture}/{check}")
    if "compliance" in expected:
        _check_compliance(result, fixture, expected["compliance"])
    if expected.get("dependent_checks") == "skipped":
        domain = expect_check_label(result, "domain_feasibility", "empty_domain", fixture)
        del domain
        dependent = [o for check in ("coverage", "outcome_conflict", "constraint_compliance", "rule_reachability") for o in statuses_for(result, check)]
        if not dependent or any(_status(o) != "skipped" for o in dependent):
            raise AcceptanceFailure(f"{fixture}: every dependent obligation must be skipped, got {dependent!r}")
        if any(_category(o) in {"covered", "compatible", "compliant"} for o in dependent):
            raise AcceptanceFailure(f"{fixture}: impossible domain must not claim vacuous success")
    if expected.get("conditional"):
        required = set(expected["assumption_ids"])
        if not required.issubset(_selected_ids(result)):
            raise AcceptanceFailure(f"{fixture}: selected assumption IDs missing: {_selected_ids(result)}")
        if not any(obj.get("conditional") is True for obj in objects(result)):
            raise AcceptanceFailure(f"{fixture}: selected context not explicitly marked conditional")
        expect_check_label(result, "coverage", expected["coverage"], fixture)
    if "conflict_constructor" in expected:
        conflicts = statuses_for(result, "outcome_conflict")
        rules_by_id = {rule["id"]: rule["then"] for rule in model.get("rules", [])}
        expected_constructor = expected["conflict_constructor"]
        exact_pair = any(
            expected_constructor in exact_ids(o, "outcome_id", "outcome_ids")
            or expected_constructor in exact_ids(o, "constructor", "constructors")
            or (
                isinstance(o.get("obligation"), dict)
                and rules_by_id.get(o["obligation"].get("left_rule_id"), {}).get("outcome")
                == expected_constructor
                and rules_by_id.get(o["obligation"].get("right_rule_id"), {}).get("outcome")
                == expected_constructor
                and _status(o) == "finding"
            )
            for o in conflicts
        )
        if not exact_pair:
            raise AcceptanceFailure(f"{fixture}: conflict must name exact constructor {expected_constructor}")
    _validate_readiness(result, fixture, expected)
    return result


def expect_diagnostic(result: Any, code: str, context: str) -> None:
    codes = {str(obj.get("code", obj.get("diagnostic_code", ""))).lower() for obj in objects(result)}
    if code.lower() not in codes:
        raise AcceptanceFailure(f"{context}: expected diagnostic code {code!r}; observed {sorted(codes)}")


def run_acceptance(binary: Path) -> int:
    if not binary.is_file():
        raise AcceptanceFailure(f"binary does not exist or is not a file: {binary}")
    expectations = load(BUNDLE / "fixtures" / "expectations.json")
    completed = 0
    for fixture, expected in expectations["valid"].items():
        if fixture == "conditional-assumption":
            baseline = analyze(binary, fixture, assumptions=[])
            if _selected_ids(baseline) or any(obj.get("conditional") is True for obj in objects(baseline)):
                raise AcceptanceFailure("conditional-assumption: assumptions selected implicitly for empty assumption list")
            check_fixture(binary, fixture, expected, assumptions=expected["assumption_ids"])
        else:
            check_fixture(binary, fixture, expected)
            if fixture == "big-integer":
                evaluated = run_json(binary, ["evaluate", str(BUNDLE / "fixtures/valid/big-integer.json"),
                                              str(BUNDLE / "scenarios/large-integer.json")])
                _expect_exact_integer(evaluated, expected["large_scenario_output"])
        completed += 1
    for fixture, expected in expectations["invalid"].items():
        check_fixture(binary, fixture, expected)
        completed += 1

    for fixture, scenario in (("unsafe-grant", "unsafe-expiry"), ("unsafe-grant", "unknown-approval")):
        evaluation = run_json(binary, ["evaluate", str(BUNDLE / "fixtures/valid" / f"{fixture}.json"), str(BUNDLE / "scenarios" / f"{scenario}.json")])
        if scenario == "unknown-approval":
            _expect_evaluation_outcome(evaluation, "unspecified")
        else:
            _expect_evaluation_violation(evaluation, "ExpiryWithinContract")
        completed += 1
    _replay_unsafe_witness(binary)
    completed += 1
    _check_cli_errors(binary)
    completed += 1
    _check_constructor_payload_independently(binary)
    completed += 1
    print(f"PASS: {completed} fixture/scenario/CLI acceptance groups against {binary}")
    return completed


def _expect_exact_integer(result: Any, expected: str) -> None:
    typed_matches = [obj for obj in objects(result) if obj.get("kind") == "int" and obj.get("value") == expected]
    if not typed_matches:
        raise AcceptanceFailure(f"large-integer evaluation lacks exact typed integer value {expected}")


def _expect_evaluation_outcome(result: Any, expected: str) -> None:
    if not any(obj.get("classification") == expected or obj.get("status") == expected for obj in objects(result)):
        raise AcceptanceFailure(f"evaluation expected exact classification {expected}")


def _expect_evaluation_violation(result: Any, constraint: str) -> None:
    violations = set()
    for obj in objects(result):
        violations.update(exact_ids(obj, "violated_constraint_id", "violated_constraint_ids"))
    if constraint not in violations:
        raise AcceptanceFailure(f"unsafe evaluation did not identify exact violated constraint {constraint}: {result!r}")


def _replay_unsafe_witness(binary: Path) -> None:
    result = analyze(binary, "unsafe-grant")
    candidates = [o for o in objects(result) if obligation_kind(o) and _norm(obligation_kind(o) or "") == _norm("constraint_compliance")
                  and "ExpiryWithinContract" in exact_ids(o, "constraint_id", "constraint_ids")
                  and _status(o) == "finding"]
    witness = next((o.get("witness") for o in candidates if isinstance(o.get("witness"), dict)), None)
    if witness is None:
        raise AcceptanceFailure("unsafe-grant: expected typed SAT witness for ExpiryWithinContract violation")
    inputs = witness.get("inputs", witness)
    if not isinstance(inputs, dict) or not {"sponsor_approval", "contract_end", "requested_expiry"}.issubset(inputs):
        raise AcceptanceFailure(f"unsafe witness lacks typed complete inputs: {witness!r}")
    with tempfile.TemporaryDirectory(prefix="entail-acceptance-") as directory:
        scenario = Path(directory) / "witness.json"
        scenario.write_text(json.dumps({"inputs": inputs}, separators=(",", ":")), encoding="utf-8")
        evaluated = run_json(binary, ["evaluate", str(BUNDLE / "fixtures/valid/unsafe-grant.json"), str(scenario)])
    _expect_evaluation_violation(evaluated, "ExpiryWithinContract")


def _check_constructor_payload_independently(binary: Path) -> None:
    fixture = load(BUNDLE / "fixtures/valid/conflict-payload.json")
    outcomes = [rule["then"] for rule in fixture["rules"] if rule["then"]["outcome"] == "Grant"]
    distinct = {json.dumps(outcome.get("fields", {}), sort_keys=True) for outcome in outcomes}
    if len(distinct) < 2:
        raise AcceptanceFailure("conflict-payload fixture lacks independent distinct Grant field payloads")
    result = check_fixture(binary, "conflict-payload", load(BUNDLE / "fixtures/expectations.json")["valid"]["conflict-payload"])
    conflicts = statuses_for(result, "outcome_conflict")
    exact_payload_conflict = False
    rules = {rule["id"]: rule["then"] for rule in fixture.get("rules", [])}
    for obj in conflicts:
        left = obj.get("left_outcome", obj.get("outcome_a"))
        right = obj.get("right_outcome", obj.get("outcome_b"))
        if not (isinstance(left, dict) and isinstance(right, dict)):
            obligation = obj.get("obligation")
            if isinstance(obligation, dict):
                left = rules.get(obligation.get("left_rule_id"))
                right = rules.get(obligation.get("right_rule_id"))
        if (
            isinstance(left, dict)
            and isinstance(right, dict)
            and left.get("outcome", left.get("constructor"))
            == right.get("outcome", right.get("constructor"))
            == "Grant"
            and left.get("fields", left.get("payload"))
            != right.get("fields", right.get("payload"))
        ):
            exact_payload_conflict = True
    if not exact_payload_conflict and not any("Grant" in exact_ids(o, "outcome_id", "outcome_ids") and
                                                (o.get("payloads_differ") is True or o.get("constructor_payload_conflict") is True)
                                                for o in conflicts):
        raise AcceptanceFailure("conflict-payload: conflict result must demonstrate same Grant constructor with unequal field values")
    witness = next((o.get("witness") for o in conflicts if isinstance(o.get("witness"), dict)), None)
    if witness is None:
        raise AcceptanceFailure("conflict-payload: expected typed conflict witness to reconstruct independently")
    inputs = witness.get("inputs", witness)
    if not isinstance(inputs, dict):
        raise AcceptanceFailure(f"conflict-payload: malformed typed witness: {witness!r}")
    with tempfile.TemporaryDirectory(prefix="entail-payload-replay-") as directory:
        scenario = Path(directory) / "conflict-witness.json"
        scenario.write_text(json.dumps({"inputs": inputs}, separators=(",", ":")), encoding="utf-8")
        replay = run_json(binary, ["evaluate", str(BUNDLE / "fixtures/valid/conflict-payload.json"), str(scenario)])
    outcomes = [outcome for obj in objects(replay) for outcome in obj.get("outcomes", []) if isinstance(outcome, dict)]
    grants = [outcome for outcome in outcomes if outcome.get("constructor", outcome.get("outcome", outcome.get("id"))) == "Grant"]
    payloads = {json.dumps(outcome.get("fields", outcome.get("payload")), sort_keys=True) for outcome in grants}
    if len(payloads) < 2:
        raise AcceptanceFailure("conflict-payload: replay did not independently produce distinct Grant payloads")


def _check_cli_errors(binary: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="entail-cli-errors-") as directory:
        malformed = Path(directory) / "malformed.json"
        malformed.write_text("{not-json", encoding="utf-8")
        for path, description in ((malformed, "malformed JSON"), (Path(directory) / "missing.json", "missing file")):
            result = run_json(binary, ["validate", str(path)], expected_exit=2)
            if not any(obj.get("diagnostics") or obj.get("errors") or obj.get("code") for obj in objects(result)):
                raise AcceptanceFailure(f"{description}: exit 2 result must include structured error")


def self_tests() -> None:
    sample = {"obligations": [{"obligation": {"kind": "coverage"}, "status": "finding", "finding_category": "uncovered"}]}
    assert statuses_for(sample, "coverage") == [sample["obligations"][0]]
    expect_check_label(sample, "coverage", "uncovered", "nested kind")
    false_positive = {"finding": {"label": "coverage", "provenance": "uncovered"}}
    try:
        expect_check_label(false_positive, "coverage", "uncovered", "false positive")
    except AcceptanceFailure:
        pass
    else:
        raise AssertionError("free-text finding/provenance cannot satisfy a typed obligation")
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
