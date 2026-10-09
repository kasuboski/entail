use author_analysis::{
    AnalysisOptions, Backend, BackendAnswer, Check, FindingCategory, Obligation, ResultStatus,
    SolveBudget, analyze,
};
use author_model::{CompiledModel, InputValues, ModelLimits, Value, compile_json, evaluate};
use author_z3::Z3Backend;
use serde_json::{Value as JsonValue, json};

fn fixture(name: &str) -> JsonValue {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../author-engine-v0/fixtures/valid")
        .join(name);
    let bytes = std::fs::read(path).unwrap_or_default();
    serde_json::from_slice(&bytes).unwrap_or(JsonValue::Null)
}

fn compile(value: &JsonValue) -> Option<CompiledModel> {
    let result = compile_json(value, &ModelLimits::default());
    assert!(
        result.is_ok(),
        "fixture model failed to compile: {result:?}"
    );
    result.ok()
}

fn solve(model: &CompiledModel, obligation: &Obligation, assumptions: &[String]) -> BackendAnswer {
    let mut backend = Z3Backend::new();
    backend.solve(
        model,
        obligation,
        assumptions,
        &SolveBudget {
            timeout_ms: 2_000,
            solver_resource_limit: None,
        },
    )
}

fn int_eq(path: &str, literal: &str, unit: &str) -> JsonValue {
    json!({
        "kind": "eq",
        "left": {"kind": "ref", "path": path},
        "right": {"kind": "int", "value": literal, "unit": unit}
    })
}

#[test]
fn lowers_every_expression_operator_with_unbounded_integer_witnesses() {
    let mut value = fixture("big-integer.json");
    let huge = "-999999999999999999999999999999999999999999";
    let next = "-999999999999999999999999999999999999999998";
    let previous = "-1000000000000000000000000000000000000000000";
    let doubled = "-1999999999999999999999999999999999999999998";
    value["derived"] = json!([{
        "id": "twice",
        "type": {"kind": "int", "unit": "count"},
        "expr": {"kind": "scale", "factor": "2", "arg": {"kind": "ref", "path": "input.amount"}},
        "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
    }]);
    value["domain"] = json!([{
        "id": "ExactProbe",
        "expr": {
            "kind": "and",
            "args": [
                int_eq("input.amount", huge, "count"),
                {"kind": "ne", "left": {"kind": "add", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": "1", "unit": "count"}}, "right": {"kind": "ref", "path": "input.amount"}},
                {"kind": "lt", "left": {"kind": "sub", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": "1", "unit": "count"}}, "right": {"kind": "ref", "path": "input.amount"}},
                {"kind": "le", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": huge, "unit": "count"}},
                {"kind": "gt", "left": {"kind": "add", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": "1", "unit": "count"}}, "right": {"kind": "ref", "path": "input.amount"}},
                {"kind": "ge", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": huge, "unit": "count"}},
                {"kind": "gt", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": previous, "unit": "count"}},
                {"kind": "eq", "left": {"kind": "scale", "factor": "2", "arg": {"kind": "ref", "path": "input.amount"}}, "right": {"kind": "int", "value": doubled, "unit": "count"}},
                {"kind": "eq", "left": {"kind": "ref", "path": "derived.twice"}, "right": {"kind": "int", "value": doubled, "unit": "count"}},
                {"kind": "eq", "left": {"kind": "if", "condition": {"kind": "bool", "value": true}, "then": {"kind": "ref", "path": "input.amount"}, "else": {"kind": "int", "value": next, "unit": "count"}}, "right": {"kind": "int", "value": huge, "unit": "count"}},
                {"kind": "or", "args": [{"kind": "bool", "value": false}, {"kind": "bool", "value": true}]},
                {"kind": "not", "arg": {"kind": "bool", "value": false}}
            ]
        },
        "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
    }]);
    let Some(model) = compile(&value) else {
        return;
    };
    let answer = solve(&model, &Obligation::DomainFeasibility, &[]);
    assert!(
        matches!(&answer, BackendAnswer::Sat { .. }),
        "expected SAT witness, received {answer:?}"
    );
    let BackendAnswer::Sat { inputs } = answer else {
        return;
    };
    assert_eq!(
        inputs.get("amount"),
        Some(&Value::Int {
            value: huge.to_owned(),
            unit: "count".to_owned()
        })
    );
}

#[test]
fn decodes_enum_unknown_and_gregorian_date_bounds_exactly() {
    let mut enum_model = fixture("partial.json");
    enum_model["domain"] = json!([{
        "id": "UnknownIsAnOrdinaryValue",
        "expr": {
            "kind": "eq",
            "left": {"kind": "ref", "path": "input.sponsor_approval"},
            "right": {"kind": "enum", "enum": "ApprovalStatus", "variant": "Unknown"}
        },
        "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
    }]);
    let Some(enum_model) = compile(&enum_model) else {
        return;
    };
    let answer = solve(&enum_model, &Obligation::DomainFeasibility, &[]);
    assert!(
        matches!(&answer, BackendAnswer::Sat { .. }),
        "expected SAT witness, received {answer:?}"
    );
    let BackendAnswer::Sat { inputs } = answer else {
        return;
    };
    assert_eq!(
        inputs.get("sponsor_approval"),
        Some(&Value::Enum {
            enum_name: "ApprovalStatus".to_owned(),
            variant: "Unknown".to_owned()
        })
    );

    let mut conditional = fixture("partial.json");
    conditional["domain"] = json!([{
        "id": "NotApproved",
        "expr": {
            "kind": "eq",
            "left": {"kind": "ref", "path": "input.sponsor_approval"},
            "right": {"kind": "enum", "enum": "ApprovalStatus", "variant": "NotApproved"}
        },
        "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
    }]);
    let Some(conditional_model) = compile(&conditional) else {
        return;
    };
    assert!(matches!(
        solve(&conditional_model, &Obligation::DomainFeasibility, &[]),
        BackendAnswer::Sat { .. }
    ));
    assert_eq!(
        solve(
            &conditional_model,
            &Obligation::DomainFeasibility,
            &["SponsorsAlreadyApproved".to_owned()]
        ),
        BackendAnswer::Unsat
    );

    let mut dates = fixture("partial.json");
    dates["domain"] = json!([
        {
            "id": "FirstDate",
            "expr": {"kind": "eq", "left": {"kind": "ref", "path": "input.contract_end"}, "right": {"kind": "date", "value": "0001-01-01"}},
            "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
        },
        {
            "id": "LastDate",
            "expr": {"kind": "eq", "left": {"kind": "ref", "path": "input.requested_expiry"}, "right": {"kind": "date", "value": "9999-12-31"}},
            "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
        }
    ]);
    let Some(date_model) = compile(&dates) else {
        return;
    };
    let answer = solve(&date_model, &Obligation::DomainFeasibility, &[]);
    assert!(
        matches!(&answer, BackendAnswer::Sat { .. }),
        "expected SAT witness, received {answer:?}"
    );
    let BackendAnswer::Sat { inputs } = answer else {
        return;
    };
    assert_eq!(
        inputs.get("contract_end"),
        Some(&Value::Date {
            value: "0001-01-01".to_owned()
        })
    );
    assert_eq!(
        inputs.get("requested_expiry"),
        Some(&Value::Date {
            value: "9999-12-31".to_owned()
        })
    );
}

#[test]
fn finds_payload_conflicts_and_independent_constraint_violations() {
    let Some(conflict) = compile(&fixture("conflict-payload.json")) else {
        return;
    };
    let answer = solve(
        &conflict,
        &Obligation::OutcomeConflict {
            left_rule_id: "GrantApprovedRequest".to_owned(),
            right_rule_id: "GrantUntilContractEnd".to_owned(),
        },
        &[],
    );
    assert!(matches!(answer, BackendAnswer::Sat { .. }), "{answer:?}");

    let Some(unsafe_model) = compile(&fixture("unsafe-grant.json")) else {
        return;
    };
    let answer = solve(
        &unsafe_model,
        &Obligation::ConstraintCompliance {
            rule_id: "GrantApprovedRequest".to_owned(),
            constraint_id: "ExpiryWithinContract".to_owned(),
        },
        &[],
    );
    assert!(matches!(answer, BackendAnswer::Sat { .. }), "{answer:?}");
}

#[test]
fn finite_integer_solver_answers_match_exhaustive_model_evaluation() {
    let mut value = fixture("big-integer.json");
    value["domain"] = json!([{
        "id": "BoundedProbe",
        "expr": {"kind": "and", "args": [
            {"kind": "ge", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": "-2", "unit": "count"}},
            {"kind": "le", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": "2", "unit": "count"}}
        ]},
        "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
    }]);
    let Some(rules) = value.get_mut("rules").and_then(JsonValue::as_array_mut) else {
        return;
    };
    let Some(first_rule) = rules.first_mut().and_then(JsonValue::as_object_mut) else {
        return;
    };
    first_rule.insert("when".to_owned(), json!({
        "kind": "if",
        "condition": {"kind": "lt", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": "0", "unit": "count"}},
        "then": {"kind": "ge", "left": {"kind": "scale", "factor": "-2", "arg": {"kind": "ref", "path": "input.amount"}}, "right": {"kind": "int", "value": "2", "unit": "count"}},
        "else": {"kind": "eq", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": "2", "unit": "count"}}
    }));
    rules.push(json!({
        "id": "Impossible",
        "when": {"kind": "gt", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": "2", "unit": "count"}},
        "then": {"outcome": "Value", "fields": {"amount": {"kind": "ref", "path": "input.amount"}}},
        "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
    }));
    let Some(model) = compile(&value) else {
        return;
    };
    let mut found_guard = false;
    let mut found_never = false;
    for amount in ["-2", "-1", "0", "1", "2"] {
        let inputs = InputValues::from([(
            "amount".to_owned(),
            Value::Int {
                value: amount.to_owned(),
                unit: "count".to_owned(),
            },
        )]);
        let result = evaluate(&model, &inputs, &[], None);
        assert!(result.is_ok(), "concrete evaluation failed: {result:?}");
        if let Ok(result) = result {
            found_guard |= result.matched_rule_ids.iter().any(|id| id == "Double");
            found_never |= result.matched_rule_ids.iter().any(|id| id == "Impossible");
        }
    }
    let guard_answer = solve(
        &model,
        &Obligation::RuleReachability {
            rule_id: "Double".to_owned(),
        },
        &[],
    );
    assert_eq!(
        matches!(guard_answer, BackendAnswer::Sat { .. }),
        found_guard
    );
    let impossible_answer = solve(
        &model,
        &Obligation::RuleReachability {
            rule_id: "Impossible".to_owned(),
        },
        &[],
    );
    assert_eq!(
        matches!(impossible_answer, BackendAnswer::Sat { .. }),
        found_never
    );
}

#[test]
fn coverage_tag_conflicts_identical_outputs_and_empty_domains_are_exact() {
    let Some(complete) = compile(&fixture("complete-proposed.json")) else {
        return;
    };
    assert_eq!(
        solve(&complete, &Obligation::Coverage, &[]),
        BackendAnswer::Unsat
    );

    let Some(conflict) = compile(&fixture("conflict-tag.json")) else {
        return;
    };
    assert!(matches!(
        solve(
            &conflict,
            &Obligation::OutcomeConflict {
                left_rule_id: "GrantApprovedRequest".to_owned(),
                right_rule_id: "AlsoDenyApproved".to_owned()
            },
            &[]
        ),
        BackendAnswer::Sat { .. }
    ));

    let Some(duplicate) = compile(&fixture("duplicate-output.json")) else {
        return;
    };
    assert_eq!(
        solve(
            &duplicate,
            &Obligation::OutcomeConflict {
                left_rule_id: "GrantApprovedRequest".to_owned(),
                right_rule_id: "DuplicateGrant".to_owned()
            },
            &[]
        ),
        BackendAnswer::Unsat
    );

    let Some(empty) = compile(&fixture("empty-domain.json")) else {
        return;
    };
    assert_eq!(
        solve(&empty, &Obligation::DomainFeasibility, &[]),
        BackendAnswer::Unsat
    );
}

#[test]
fn analysis_replays_z3_compliance_counterexamples() {
    let Some(model) = compile(&fixture("unsafe-grant.json")) else {
        return;
    };
    let mut backend = Z3Backend::new();
    let report = analyze(
        &model,
        &AnalysisOptions {
            checks: vec![Check::ConstraintCompliance],
            assumption_ids: Vec::new(),
            budget: author_analysis::AnalysisBudget::default(),
        },
        &mut backend,
    );
    assert!(report.is_ok(), "analysis failed: {report:?}");
    let Ok(report) = report else {
        return;
    };
    assert!(report.results.iter().any(|result| {
        result.status == ResultStatus::Finding
            && result.finding_category == Some(FindingCategory::Violation)
            && result.witness.is_some()
    }));
}

#[test]
fn empty_input_assignment_is_a_complete_empty_tuple() {
    let value = json!({
        "language_version": "author-model/0.1",
        "model_id": "Constant",
        "sources": [], "enums": [], "inputs": [], "derived": [],
        "outcomes": [{"id": "Proceed", "fields": [], "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}}],
        "domain": [], "assumptions": [],
        "rules": [{"id": "Always", "when": {"kind": "bool", "value": true}, "then": {"outcome": "Proceed", "fields": {}}, "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}}],
        "constraints": [], "open_items": []
    });
    let Some(model) = compile(&value) else {
        return;
    };
    let answer = solve(
        &model,
        &Obligation::RuleReachability {
            rule_id: "Always".to_owned(),
        },
        &[],
    );
    assert_eq!(
        answer,
        BackendAnswer::Sat {
            inputs: std::collections::BTreeMap::default()
        }
    );
}
