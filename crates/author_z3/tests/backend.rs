use author_analysis::{
    AnalysisOptions, Backend, BackendAnswer, Check, FindingCategory, Obligation, ResultStatus,
    SolveBudget, analyze,
};
use author_model::{CompiledModel, InputValues, ModelLimits, Value, compile_json, evaluate};
use author_z3::Z3Backend;
use serde_json::{Value as JsonValue, json};

fn fixture(name: &str) -> JsonValue {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/examples/models")
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
fn apply_rate_lowers_derived_conditional_fixed_rates_as_linear_branches() {
    let mut value = fixture("big-integer.json");
    value["derived"] = json!([{
        "id": "rate",
        "type": {"kind": "int", "unit": "credits/count"},
        "expr": {
            "kind": "if",
            "condition": {"kind": "ge", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": "5", "unit": "count"}},
            "then": {"kind": "int", "value": "7", "unit": "credits/count"},
            "else": {"kind": "int", "value": "2", "unit": "credits/count"}
        },
        "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
    }]);
    value["domain"] = json!([{
        "id": "RateProductIsExact",
        "expr": {
            "kind": "eq",
            "left": {"kind": "apply_rate", "rate": {"kind": "ref", "path": "derived.rate"}, "quantity": {"kind": "ref", "path": "input.amount"}},
            "right": {"kind": "int", "value": "35", "unit": "credits"}
        },
        "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
    }]);
    let Some(model) = compile(&value) else {
        return;
    };
    let answer = solve(&model, &Obligation::DomainFeasibility, &[]);
    assert!(
        matches!(&answer, BackendAnswer::Sat { .. }),
        "expected SAT, got {answer:?}"
    );
    let BackendAnswer::Sat { inputs } = answer else {
        return;
    };
    assert_eq!(
        inputs.get("amount"),
        Some(&Value::Int {
            value: "5".to_owned(),
            unit: "count".to_owned()
        })
    );
    let evaluation = evaluate(&model, &inputs, &[], None).expect("solver witness must evaluate");
    assert_eq!(
        evaluation.derived.get("rate"),
        Some(&Value::Int {
            value: "7".to_owned(),
            unit: "credits/count".to_owned()
        })
    );
}

#[test]
fn apply_rate_rejects_an_unrestricted_input_rate_with_a_specific_diagnostic() {
    let mut value = fixture("big-integer.json");
    value["inputs"]
        .as_array_mut()
        .expect("fixture inputs")
        .push(json!({
            "id": "rate",
            "type": {"kind": "int", "unit": "credits/count"},
            "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
        }));
    value["outcomes"][0]["fields"][0]["type"] = json!({"kind": "int", "unit": "credits"});
    value["rules"][0]["then"]["fields"]["amount"] = json!({
        "kind": "apply_rate",
        "rate": {"kind": "ref", "path": "input.rate"},
        "quantity": {"kind": "ref", "path": "input.amount"}
    });
    value["constraints"] = json!([]);
    let diagnostics = compile_json(&value, &ModelLimits::default())
        .expect_err("unrestricted variable rate must be rejected");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "unsupported_nonlinear_operation"),
        "expected the nonlinear-rate diagnostic, got {diagnostics:?}"
    );
}

#[test]
fn shared_conditional_rate_references_are_bounded_and_remain_exact() {
    let mut value = fixture("big-integer.json");
    value["inputs"]
        .as_array_mut()
        .expect("fixture inputs")
        .push(json!({
            "id": "flag",
            "type": {"kind": "bool"},
            "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
        }));
    let mut derived = vec![json!({
        "id": "rate0",
        "type": {"kind": "int", "unit": "credits/count"},
        "expr": {"kind": "int", "value": "7", "unit": "credits/count"},
        "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
    })];
    for index in 1..=28 {
        let previous = format!("derived.rate{}", index - 1);
        derived.push(json!({
            "id": format!("rate{index}"),
            "type": {"kind": "int", "unit": "credits/count"},
            "expr": {
                "kind": "if",
                "condition": {"kind": "ref", "path": "input.flag"},
                "then": {"kind": "ref", "path": previous},
                "else": {"kind": "ref", "path": previous}
            },
            "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
        }));
    }
    value["derived"] = JsonValue::Array(derived);
    value["domain"] = json!([{
        "id": "SharedRateProductIsExact",
        "expr": {
            "kind": "eq",
            "left": {"kind": "apply_rate", "rate": {"kind": "ref", "path": "derived.rate28"}, "quantity": {"kind": "ref", "path": "input.amount"}},
            "right": {"kind": "int", "value": "14", "unit": "credits"}
        },
        "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
    }]);
    let Some(model) = compile(&value) else {
        return;
    };
    let answer = solve(&model, &Obligation::DomainFeasibility, &[]);
    assert!(
        matches!(&answer, BackendAnswer::Sat { .. }),
        "expected SAT, got {answer:?}"
    );
    let BackendAnswer::Sat { inputs } = answer else {
        return;
    };
    assert_eq!(
        inputs.get("amount"),
        Some(&Value::Int {
            value: "2".to_owned(),
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

const DATE_SAMPLES: [&str; 9] = [
    "0001-01-01",
    "0001-01-02",
    "1900-02-28",
    "1900-03-01",
    "2000-02-28",
    "2000-02-29",
    "2000-03-01",
    "9999-12-30",
    "9999-12-31",
];

fn finite_date_membership(input_id: &str) -> JsonValue {
    json!({"kind": "or", "args": DATE_SAMPLES.iter().map(|date| json!({
        "kind": "eq",
        "left": {"kind": "ref", "path": format!("input.{input_id}")},
        "right": {"kind": "date", "value": date}
    })).collect::<Vec<_>>()})
}

#[allow(clippy::expect_used, clippy::indexing_slicing)] // Fixture mutations assert the fixture schema.
fn finite_multitype_model() -> Option<CompiledModel> {
    let mut value = fixture("unsafe-grant.json");
    value["inputs"].as_array_mut().expect("inputs array").extend([
        json!({"id": "enabled", "type": {"kind": "bool"}, "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}}),
        json!({"id": "amount", "type": {"kind": "int", "unit": "count"}, "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}}),
    ]);
    value["derived"] = json!([
        {"id": "total_amount", "type": {"kind": "int", "unit": "count"}, "expr": {"kind": "add", "left": {"kind": "ref", "path": "derived.double_amount"}, "right": {"kind": "ref", "path": "input.amount"}}, "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}},
        {"id": "double_amount", "type": {"kind": "int", "unit": "count"}, "expr": {"kind": "scale", "factor": "2", "arg": {"kind": "ref", "path": "input.amount"}}, "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}}
    ]);
    value["domain"].as_array_mut().expect("domain array").extend([
        json!({
            "id": "BoundedAmount",
            "expr": {"kind": "and", "args": [
                {"kind": "ge", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": "-2", "unit": "count"}},
                {"kind": "le", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": "2", "unit": "count"}}
            ]},
            "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
        }),
        json!({"id": "FiniteContractDates", "expr": finite_date_membership("contract_end"), "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}}),
        json!({"id": "FiniteRequestedDates", "expr": finite_date_membership("requested_expiry"), "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}})
    ]);
    value["outcomes"]
        .as_array_mut()
        .expect("outcomes array")
        .iter_mut()
        .find(|outcome| outcome["id"] == "Grant")
        .expect("Grant outcome")["fields"] = json!([
        {"id": "expiry", "type": {"kind": "date"}},
        {"id": "enabled", "type": {"kind": "bool"}},
        {"id": "amount", "type": {"kind": "int", "unit": "count"}},
        {"id": "status", "type": {"kind": "enum", "name": "ApprovalStatus"}}
    ]);
    let first_rule = value["rules"]
        .as_array_mut()
        .expect("rules array")
        .iter_mut()
        .find(|rule| rule["id"] == "GrantApprovedRequest")
        .expect("approval rule");
    first_rule["when"] = json!({"kind": "and", "args": [
        first_rule["when"].clone(),
        {"kind": "ref", "path": "input.enabled"}
    ]});
    first_rule["then"]["fields"]["enabled"] = json!({"kind": "ref", "path": "input.enabled"});
    first_rule["then"]["fields"]["amount"] = json!({"kind": "ref", "path": "derived.total_amount"});
    first_rule["then"]["fields"]["status"] =
        json!({"kind": "ref", "path": "input.sponsor_approval"});
    value["rules"]
        .as_array_mut()
        .expect("rules array")
        .push(json!({
            "id": "AlwaysDifferentExpiry",
            "when": {"kind": "bool", "value": true},
            "then": {"outcome": "Grant", "fields": {
                "expiry": {"kind": "date", "value": "9999-12-31"},
                "enabled": {"kind": "bool", "value": false},
                "amount": {"kind": "int", "value": "0", "unit": "count"},
                "status": {"kind": "enum", "enum": "ApprovalStatus", "variant": "Unknown"}
            }},
            "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
        }));
    compile(&value)
}

#[test]
fn all_five_obligations_match_finite_enum_and_date_evaluation_with_assumptions() {
    let Some(model) = finite_multitype_model() else {
        return;
    };
    let approvals = ["Approved", "NotApproved", "Unknown"];
    for assumptions in [Vec::new(), vec!["SponsorsAlreadyApproved".to_owned()]] {
        let mut expected = [false; 5];
        for approval in approvals {
            for contract_end in DATE_SAMPLES {
                for requested_expiry in DATE_SAMPLES {
                    for enabled in [false, true] {
                        for amount in -2..=2 {
                            let inputs = InputValues::from([
                                (
                                    "sponsor_approval".to_owned(),
                                    Value::Enum {
                                        enum_name: "ApprovalStatus".to_owned(),
                                        variant: approval.to_owned(),
                                    },
                                ),
                                (
                                    "contract_end".to_owned(),
                                    Value::Date {
                                        value: contract_end.to_owned(),
                                    },
                                ),
                                (
                                    "requested_expiry".to_owned(),
                                    Value::Date {
                                        value: requested_expiry.to_owned(),
                                    },
                                ),
                                ("enabled".to_owned(), Value::Bool { value: enabled }),
                                (
                                    "amount".to_owned(),
                                    Value::Int {
                                        value: amount.to_string(),
                                        unit: "count".to_owned(),
                                    },
                                ),
                            ]);
                            let result = evaluate(&model, &inputs, &assumptions, None);
                            assert!(result.is_ok(), "oracle failed: {result:?}");
                            if let Ok(result) = result {
                                expected[0] |= result.classification != "outside_domain";
                                expected[1] |= result
                                    .matched_rule_ids
                                    .iter()
                                    .any(|id| id == "GrantApprovedRequest");
                                expected[2] |= result.classification != "outside_domain"
                                    && result.matched_rule_ids.is_empty();
                                expected[3] |= result.classification == "conflicting";
                                expected[4] |= result
                                    .matched_rule_ids
                                    .iter()
                                    .any(|id| id == "GrantApprovedRequest")
                                    && result
                                        .violated_constraint_ids
                                        .iter()
                                        .any(|id| id == "ExpiryWithinContract");
                            }
                        }
                    }
                }
            }
        }
        let obligations = [
            Obligation::DomainFeasibility,
            Obligation::RuleReachability {
                rule_id: "GrantApprovedRequest".to_owned(),
            },
            Obligation::Coverage,
            Obligation::OutcomeConflict {
                left_rule_id: "GrantApprovedRequest".to_owned(),
                right_rule_id: "AlwaysDifferentExpiry".to_owned(),
            },
            Obligation::ConstraintCompliance {
                rule_id: "GrantApprovedRequest".to_owned(),
                constraint_id: "ExpiryWithinContract".to_owned(),
            },
        ];
        for (obligation, oracle) in obligations.iter().zip(expected) {
            let answer = solve(&model, obligation, &assumptions);
            assert_eq!(
                matches!(answer, BackendAnswer::Sat { .. }),
                oracle,
                "{obligation:?}, assumptions={assumptions:?}, answer={answer:?}"
            );
        }
    }
}

#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::too_many_arguments,
    clippy::needless_pass_by_value
)] // Test builder assembles a fixed-schema typed model from owned AST values.
fn check_expression_case(
    name: &str,
    input_type: JsonValue,
    input_value: Value,
    input_literal: JsonValue,
    result_type: JsonValue,
    expression: JsonValue,
    expected_literal: JsonValue,
    expected_value: Value,
) {
    let value = json!({
        "language_version": "author-model/0.1",
        "model_id": name,
        "sources": [],
        "enums": [{"id": "ProbeEnum", "variants": ["First", "Second", "Third"], "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}}],
        "inputs": [{"id": "x", "type": input_type, "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}}],
        "derived": [],
        "outcomes": [{"id": "Probe", "fields": [{"id": "value", "type": result_type}], "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}}],
        "domain": [{"id": "FixX", "expr": {"kind": "eq", "left": {"kind": "ref", "path": "input.x"}, "right": input_literal}, "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}}],
        "assumptions": [],
        "rules": [
            {"id": "Capture", "when": {"kind": "bool", "value": true}, "then": {"outcome": "Probe", "fields": {"value": expression}}, "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}},
            {"id": "MatchesExpected", "when": {"kind": "eq", "left": expression, "right": expected_literal}, "then": {"outcome": "Probe", "fields": {"value": expression}}, "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}},
            {"id": "DoesNotMatch", "when": {"kind": "ne", "left": expression, "right": expected_literal}, "then": {"outcome": "Probe", "fields": {"value": expression}}, "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}}
        ],
        "constraints": [],
        "open_items": []
    });
    let Some(model) = compile(&value) else {
        return;
    };
    let inputs = InputValues::from([("x".to_owned(), input_value)]);
    let evaluation = evaluate(&model, &inputs, &[], None);
    assert!(
        evaluation.is_ok(),
        "concrete evaluation failed for {name}: {evaluation:?}"
    );
    let actual = match evaluation {
        Ok(evaluation) => evaluation
            .outcomes
            .iter()
            .find(|outcome| outcome.outcome == "Probe")
            .and_then(|outcome| outcome.fields.get("value"))
            .cloned(),
        Err(_) => None,
    };
    assert_eq!(
        actual.as_ref(),
        Some(&expected_value),
        "concrete expression value mismatch for {name}"
    );

    let answer = solve(
        &model,
        &Obligation::RuleReachability {
            rule_id: "MatchesExpected".to_owned(),
        },
        &[],
    );
    assert_eq!(
        matches!(answer, BackendAnswer::Sat { .. }),
        actual.as_ref() == Some(&expected_value),
        "Z3 disagrees with concrete equality for {name}: {answer:?}"
    );
    let not_equal_answer = solve(
        &model,
        &Obligation::RuleReachability {
            rule_id: "DoesNotMatch".to_owned(),
        },
        &[],
    );
    assert_eq!(
        matches!(not_equal_answer, BackendAnswer::Sat { .. }),
        actual.as_ref() != Some(&expected_value),
        "Z3 disagrees with concrete disequality for {name}: {not_equal_answer:?}"
    );
}

#[test]
#[allow(clippy::too_many_lines)] // Explicit typed operator cases are the deterministic generator corpus.
fn deterministic_typed_expression_corpus_matches_evaluation_under_fixed_assignments() {
    let bool_type = json!({"kind": "bool"});
    let int_type = json!({"kind": "int", "unit": "count"});
    let date_type = json!({"kind": "date"});
    let enum_type = json!({"kind": "enum", "name": "ProbeEnum"});
    let bool_input = Value::Bool { value: true };
    let bool_literal = json!({"kind": "bool", "value": true});
    let huge = "-999999999999999999999999999999999999999999";
    let int_input = Value::Int {
        value: huge.to_owned(),
        unit: "count".to_owned(),
    };
    let int_literal = json!({"kind": "int", "value": huge, "unit": "count"});
    let date_input = Value::Date {
        value: "2000-02-29".to_owned(),
    };
    let date_literal = json!({"kind": "date", "value": "2000-02-29"});
    let enum_input = Value::Enum {
        enum_name: "ProbeEnum".to_owned(),
        variant: "Second".to_owned(),
    };
    let enum_literal = json!({"kind": "enum", "enum": "ProbeEnum", "variant": "Second"});

    macro_rules! check_bool {
        ($name:literal, $expression:expr, $expected:expr) => {
            check_expression_case($name, bool_type.clone(), bool_input.clone(), bool_literal.clone(), bool_type.clone(), $expression, json!({"kind": "bool", "value": $expected}), Value::Bool { value: $expected });
        };
    }
    check_bool!(
        "not",
        json!({"kind": "not", "arg": {"kind": "ref", "path": "input.x"}}),
        false
    );
    check_bool!(
        "and",
        json!({"kind": "and", "args": [{"kind": "ref", "path": "input.x"}, {"kind": "bool", "value": true}]}),
        true
    );
    check_bool!(
        "or",
        json!({"kind": "or", "args": [{"kind": "ref", "path": "input.x"}, {"kind": "bool", "value": false}]}),
        true
    );
    check_bool!(
        "bool_eq",
        json!({"kind": "eq", "left": {"kind": "ref", "path": "input.x"}, "right": {"kind": "bool", "value": true}}),
        true
    );
    check_bool!(
        "bool_ne",
        json!({"kind": "ne", "left": {"kind": "ref", "path": "input.x"}, "right": {"kind": "bool", "value": false}}),
        true
    );
    check_bool!(
        "if_bool",
        json!({"kind": "if", "condition": {"kind": "ref", "path": "input.x"}, "then": {"kind": "bool", "value": false}, "else": {"kind": "bool", "value": true}}),
        false
    );

    macro_rules! check_int {
        ($name:literal, $expression:expr, $expected:expr) => {
            check_expression_case($name, int_type.clone(), int_input.clone(), int_literal.clone(), int_type.clone(), $expression, json!({"kind": "int", "value": $expected, "unit": "count"}), Value::Int { value: $expected.to_owned(), unit: "count".to_owned() });
        };
    }
    check_int!(
        "add_huge_negative",
        json!({"kind": "add", "left": {"kind": "ref", "path": "input.x"}, "right": {"kind": "int", "value": "1", "unit": "count"}}),
        "-999999999999999999999999999999999999999998"
    );
    check_int!(
        "sub_huge_negative",
        json!({"kind": "sub", "left": {"kind": "ref", "path": "input.x"}, "right": {"kind": "int", "value": "1", "unit": "count"}}),
        "-1000000000000000000000000000000000000000000"
    );
    check_int!(
        "scale_negative",
        json!({"kind": "scale", "factor": "-2", "arg": {"kind": "ref", "path": "input.x"}}),
        "1999999999999999999999999999999999999999998"
    );
    check_int!(
        "if_int",
        json!({"kind": "if", "condition": {"kind": "bool", "value": true}, "then": {"kind": "ref", "path": "input.x"}, "else": {"kind": "int", "value": "0", "unit": "count"}}),
        huge
    );
    let positive_huge = huge.strip_prefix('-').expect("negative integer literal");
    check_expression_case(
        "positive_huge_scale_negative",
        int_type.clone(),
        Value::Int {
            value: positive_huge.to_owned(),
            unit: "count".to_owned(),
        },
        json!({"kind": "int", "value": positive_huge, "unit": "count"}),
        int_type.clone(),
        json!({"kind": "scale", "factor": "-1", "arg": {"kind": "ref", "path": "input.x"}}),
        json!({"kind": "int", "value": huge, "unit": "count"}),
        Value::Int {
            value: huge.to_owned(),
            unit: "count".to_owned(),
        },
    );

    let compares = [
        (
            "lt",
            "lt",
            "-999999999999999999999999999999999999999998",
            true,
        ),
        ("le", "le", huge, true),
        (
            "gt",
            "gt",
            "-1000000000000000000000000000000000000000000",
            true,
        ),
        ("ge", "ge", huge, true),
        ("int_eq", "eq", huge, true),
        (
            "int_ne",
            "ne",
            "-999999999999999999999999999999999999999998",
            true,
        ),
    ];
    for (name, operator, right, result) in compares {
        check_expression_case(
            name,
            int_type.clone(),
            int_input.clone(),
            int_literal.clone(),
            bool_type.clone(),
            json!({"kind": operator, "left": {"kind": "ref", "path": "input.x"}, "right": {"kind": "int", "value": right, "unit": "count"}}),
            json!({"kind": "bool", "value": result}),
            Value::Bool { value: result },
        );
    }

    let enum_cases = [
        ("enum_eq", "eq", "Second", true),
        ("enum_ne", "ne", "First", true),
    ];
    for (name, operator, variant, result) in enum_cases {
        check_expression_case(
            name,
            enum_type.clone(),
            enum_input.clone(),
            enum_literal.clone(),
            bool_type.clone(),
            json!({"kind": operator, "left": {"kind": "ref", "path": "input.x"}, "right": {"kind": "enum", "enum": "ProbeEnum", "variant": variant}}),
            json!({"kind": "bool", "value": result}),
            Value::Bool { value: result },
        );
    }
    check_expression_case(
        "if_enum",
        enum_type.clone(),
        enum_input,
        enum_literal,
        enum_type,
        json!({"kind": "if", "condition": {"kind": "bool", "value": true}, "then": {"kind": "ref", "path": "input.x"}, "else": {"kind": "enum", "enum": "ProbeEnum", "variant": "First"}}),
        json!({"kind": "enum", "enum": "ProbeEnum", "variant": "Second"}),
        Value::Enum {
            enum_name: "ProbeEnum".to_owned(),
            variant: "Second".to_owned(),
        },
    );

    let date_cases = [
        ("date_eq_leap", "eq", "2000-02-29", true),
        ("date_ne_century", "ne", "1900-03-01", true),
        ("date_lt_adjacent", "lt", "2000-03-01", true),
        ("date_le_same", "le", "2000-02-29", true),
        ("date_gt_century", "gt", "1900-03-01", true),
        ("date_ge_same", "ge", "2000-02-29", true),
    ];
    for (name, operator, right, result) in date_cases {
        check_expression_case(
            name,
            date_type.clone(),
            date_input.clone(),
            date_literal.clone(),
            bool_type.clone(),
            json!({"kind": operator, "left": {"kind": "ref", "path": "input.x"}, "right": {"kind": "date", "value": right}}),
            json!({"kind": "bool", "value": result}),
            Value::Bool { value: result },
        );
    }
    check_expression_case(
        "if_date",
        date_type.clone(),
        date_input,
        date_literal,
        date_type.clone(),
        json!({"kind": "if", "condition": {"kind": "bool", "value": false}, "then": {"kind": "date", "value": "1900-03-01"}, "else": {"kind": "ref", "path": "input.x"}}),
        json!({"kind": "date", "value": "2000-02-29"}),
        Value::Date {
            value: "2000-02-29".to_owned(),
        },
    );
    check_expression_case(
        "date_first_day_boundary",
        date_type.clone(),
        Value::Date {
            value: "0001-01-01".to_owned(),
        },
        json!({"kind": "date", "value": "0001-01-01"}),
        bool_type.clone(),
        json!({"kind": "lt", "left": {"kind": "ref", "path": "input.x"}, "right": {"kind": "date", "value": "0001-01-02"}}),
        json!({"kind": "bool", "value": true}),
        Value::Bool { value: true },
    );
    check_expression_case(
        "date_last_day_boundary",
        date_type,
        Value::Date {
            value: "9999-12-31".to_owned(),
        },
        json!({"kind": "date", "value": "9999-12-31"}),
        bool_type,
        json!({"kind": "gt", "left": {"kind": "ref", "path": "input.x"}, "right": {"kind": "date", "value": "9999-12-30"}}),
        json!({"kind": "bool", "value": true}),
        Value::Bool { value: true },
    );
}

#[test]
fn coverage_tag_conflicts_identical_outputs_and_empty_domains_are_exact() {
    let Some(no_rules) = compile(&fixture("partial.json")) else {
        return;
    };
    assert!(matches!(
        solve(&no_rules, &Obligation::Coverage, &[]),
        BackendAnswer::Sat { .. }
    ));

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
fn analysis_replays_coverage_and_conflict_counterexamples() {
    let Some(model) = compile(&fixture("conflict-payload.json")) else {
        return;
    };
    let mut backend = Z3Backend::new();
    let report = analyze(
        &model,
        &AnalysisOptions {
            checks: vec![Check::Coverage, Check::OutcomeConflict],
            assumption_ids: Vec::new(),
            budget: author_analysis::AnalysisBudget::default(),
        },
        &mut backend,
    );
    assert!(report.is_ok(), "analysis/replay failed: {report:?}");
    let Ok(report) = report else {
        return;
    };
    assert!(report.results.iter().any(|result| {
        result.finding_category == Some(FindingCategory::Conflict) && result.witness.is_some()
    }));

    let Some(partial_model) = compile(&fixture("unsafe-grant.json")) else {
        return;
    };
    let report = analyze(
        &partial_model,
        &AnalysisOptions {
            checks: vec![Check::Coverage],
            assumption_ids: Vec::new(),
            budget: author_analysis::AnalysisBudget::default(),
        },
        &mut backend,
    );
    assert!(
        report.is_ok(),
        "coverage analysis/replay failed: {report:?}"
    );
    let Ok(report) = report else {
        return;
    };
    assert!(report.results.iter().any(|result| {
        result.finding_category == Some(FindingCategory::Uncovered) && result.witness.is_some()
    }));
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
    let violation = report.results.iter().find(|result| {
        result.status == ResultStatus::Finding
            && result.finding_category == Some(FindingCategory::Violation)
            && result.witness.is_some()
    });
    assert!(violation.is_some());
    let statistics = violation.and_then(|result| result.statistics.as_ref());
    assert!(
        statistics.is_some(),
        "native solver statistics should be attached"
    );
    let assertion_count = statistics.and_then(|stats| stats.values.get("assertion_count"));
    assert!(
        assertion_count
            .and_then(|count| count.parse::<usize>().ok())
            .is_some_and(|count| count > 0),
        "expected actual assertion count, got {statistics:?}"
    );
    let cached_report = analyze(
        &model,
        &AnalysisOptions {
            checks: vec![Check::ConstraintCompliance],
            assumption_ids: Vec::new(),
            budget: author_analysis::AnalysisBudget::default(),
        },
        &mut backend,
    );
    assert!(
        cached_report.is_ok(),
        "cached analysis failed: {cached_report:?}"
    );
    let Ok(cached_report) = cached_report else {
        return;
    };
    assert!(cached_report.cache_hits > 0);
    assert!(
        cached_report
            .results
            .iter()
            .all(|result| result.statistics.is_none())
    );
}

#[test]
fn low_resource_budget_returns_inconclusive_without_claiming_success() {
    let mut value = fixture("big-integer.json");
    value["domain"] = json!([{
        "id": "ResourceProbe",
        "expr": {"kind": "and", "args": [
            {"kind": "ge", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": "-1000000000000000000000000000000000000000000", "unit": "count"}},
            {"kind": "le", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": "1000000000000000000000000000000000000000000", "unit": "count"}},
            {"kind": "ne", "left": {"kind": "ref", "path": "input.amount"}, "right": {"kind": "int", "value": "0", "unit": "count"}}
        ]},
        "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
    }]);
    let Some(model) = compile(&value) else {
        return;
    };
    let mut backend = Z3Backend::new();
    let answer = backend.solve(
        &model,
        &Obligation::DomainFeasibility,
        &[],
        &SolveBudget {
            timeout_ms: 2_000,
            solver_resource_limit: Some(1),
        },
    );
    assert!(
        matches!(
            answer,
            BackendAnswer::Unknown { .. } | BackendAnswer::Timeout | BackendAnswer::Cancelled
        ),
        "a one-unit Z3 resource limit must not be treated as a completed answer: {answer:?}"
    );
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
