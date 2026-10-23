#![allow(clippy::panic_in_result_fn)] // Assertions and fixture context are the test oracle.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use author_model::{
    Constraint, DerivedDecl, EnumDecl, Expr, FieldDecl, InputDecl, ModelLimits, OpenItem, Origin,
    OriginKind, OutcomeDecl, Predicate, RawModel, Review, Rule, RuleConclusion, Source, Type,
    compile_json, compile_text, format_text, parse_text,
};
use serde_json::Value;

type TestResult<T> = Result<T, Box<dyn Error>>;

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/examples/models")
}

fn model_paths() -> TestResult<Vec<PathBuf>> {
    let mut paths = fs::read_dir(examples_dir())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn normalized_json<T: serde::Serialize>(value: &T) -> TestResult<Value> {
    Ok(serde_json::to_value(value)?)
}

fn replace_reference(value: &mut Value, old_path: &str, new_path: &str) {
    match value {
        Value::Object(object) => {
            if object.get("path").and_then(Value::as_str) == Some(old_path) {
                object.insert("path".to_owned(), Value::String(new_path.to_owned()));
            }
            for child in object.values_mut() {
                replace_reference(child, old_path, new_path);
            }
        }
        Value::Array(values) => {
            for child in values {
                replace_reference(child, old_path, new_path);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

const fn proposal() -> Origin {
    Origin {
        kind: OriginKind::Proposal,
        source_ids: vec![],
        review: Review::Proposed,
        note: None,
    }
}

fn reference(path: &str) -> Expr {
    Expr::Ref {
        path: path.to_owned(),
    }
}

fn equality(left: Expr, right: Expr) -> Expr {
    Expr::Eq {
        left: Box::new(left),
        right: Box::new(right),
    }
}

#[allow(clippy::too_many_lines)]
fn exact_expression_model() -> RawModel {
    let count = Type::Int {
        unit: "count".to_owned(),
    };
    let mut declarations = vec![
        ("bool_literal", Type::Bool, Expr::Bool { value: true }),
        (
            "int_literal",
            count.clone(),
            Expr::Int {
                value: "123456789012345678901234567890".to_owned(),
                unit: "count".to_owned(),
            },
        ),
        (
            "negative_int_literal",
            count.clone(),
            Expr::Int {
                value: "-98765432109876543210987654321".to_owned(),
                unit: "count".to_owned(),
            },
        ),
        (
            "date_literal",
            Type::Date,
            Expr::Date {
                value: "2024-02-29".to_owned(),
            },
        ),
        (
            "enum_literal",
            Type::Enum {
                name: "Status".to_owned(),
            },
            Expr::Enum {
                enum_name: "Status".to_owned(),
                variant: "Ready".to_owned(),
            },
        ),
        ("reference", count.clone(), reference("input.amount")),
        (
            "negation",
            Type::Bool,
            Expr::Not {
                arg: Box::new(reference("input.flag")),
            },
        ),
        (
            "conjunction",
            Type::Bool,
            Expr::And {
                args: vec![
                    Expr::And {
                        args: vec![reference("input.flag"), Expr::Bool { value: true }],
                    },
                    Expr::Bool { value: false },
                ],
            },
        ),
        (
            "three_way_conjunction",
            Type::Bool,
            Expr::And {
                args: vec![
                    reference("input.flag"),
                    Expr::Bool { value: true },
                    Expr::Bool { value: false },
                ],
            },
        ),
        (
            "disjunction",
            Type::Bool,
            Expr::Or {
                args: vec![reference("input.flag"), Expr::Bool { value: false }],
            },
        ),
        (
            "equality",
            Type::Bool,
            equality(
                reference("input.amount"),
                Expr::Int {
                    value: "0".to_owned(),
                    unit: "count".to_owned(),
                },
            ),
        ),
        (
            "inequality",
            Type::Bool,
            Expr::Ne {
                left: Box::new(reference("input.amount")),
                right: Box::new(Expr::Int {
                    value: "0".to_owned(),
                    unit: "count".to_owned(),
                }),
            },
        ),
        (
            "less_than",
            Type::Bool,
            Expr::Lt {
                left: Box::new(reference("input.amount")),
                right: Box::new(Expr::Int {
                    value: "1".to_owned(),
                    unit: "count".to_owned(),
                }),
            },
        ),
        (
            "less_or_equal",
            Type::Bool,
            Expr::Le {
                left: Box::new(reference("input.amount")),
                right: Box::new(Expr::Int {
                    value: "1".to_owned(),
                    unit: "count".to_owned(),
                }),
            },
        ),
        (
            "greater_than",
            Type::Bool,
            Expr::Gt {
                left: Box::new(reference("input.amount")),
                right: Box::new(Expr::Int {
                    value: "1".to_owned(),
                    unit: "count".to_owned(),
                }),
            },
        ),
        (
            "greater_or_equal",
            Type::Bool,
            Expr::Ge {
                left: Box::new(reference("input.amount")),
                right: Box::new(Expr::Int {
                    value: "1".to_owned(),
                    unit: "count".to_owned(),
                }),
            },
        ),
        (
            "addition",
            count.clone(),
            Expr::Add {
                left: Box::new(reference("input.amount")),
                right: Box::new(Expr::Int {
                    value: "1".to_owned(),
                    unit: "count".to_owned(),
                }),
            },
        ),
        (
            "addition_with_negative_literal",
            count.clone(),
            Expr::Add {
                left: Box::new(reference("input.amount")),
                right: Box::new(Expr::Int {
                    value: "-1".to_owned(),
                    unit: "count".to_owned(),
                }),
            },
        ),
        (
            "subtraction",
            count.clone(),
            Expr::Sub {
                left: Box::new(reference("input.amount")),
                right: Box::new(Expr::Int {
                    value: "1".to_owned(),
                    unit: "count".to_owned(),
                }),
            },
        ),
        (
            "scaling",
            count.clone(),
            Expr::Scale {
                factor: "-2".to_owned(),
                arg: Box::new(reference("input.amount")),
            },
        ),
        (
            "rate_application",
            Type::Int {
                unit: "cents".to_owned(),
            },
            Expr::ApplyRate {
                rate: Box::new(Expr::Int {
                    value: "40".to_owned(),
                    unit: "cents/sample".to_owned(),
                }),
                quantity: Box::new(Expr::Int {
                    value: "3".to_owned(),
                    unit: "sample".to_owned(),
                }),
            },
        ),
        (
            "conditional",
            count.clone(),
            Expr::If {
                condition: Box::new(reference("input.flag")),
                then_expr: Box::new(reference("input.amount")),
                else_expr: Box::new(Expr::Int {
                    value: "0".to_owned(),
                    unit: "count".to_owned(),
                }),
            },
        ),
        (
            "conditional_as_binary_operand",
            Type::Bool,
            equality(
                Expr::If {
                    condition: Box::new(reference("input.flag")),
                    then_expr: Box::new(reference("input.amount")),
                    else_expr: Box::new(Expr::Int {
                        value: "0".to_owned(),
                        unit: "count".to_owned(),
                    }),
                },
                Expr::Int {
                    value: "1".to_owned(),
                    unit: "count".to_owned(),
                },
            ),
        ),
    ];
    declarations.push((
        "outcome_reference",
        count.clone(),
        reference("outcome.value"),
    ));

    RawModel {
        language_version: "author-model/0.1".to_owned(),
        model_id: "TextRoundTrip".to_owned(),
        sources: vec![Source {
            id: "S1".to_owned(),
            text: "Source says \"yes\".\nExact text survives.".to_owned(),
            locator: Some("policy.section_1".to_owned()),
        }],
        enums: vec![EnumDecl {
            id: "Status".to_owned(),
            variants: vec!["Ready".to_owned(), "Waiting".to_owned()],
            origin: Origin {
                kind: OriginKind::Source,
                source_ids: vec!["S1".to_owned()],
                review: Review::Confirmed,
                note: Some(
                    "Vocabulary note with \"quoted\" text; origin review is explicit.".to_owned(),
                ),
            },
        }],
        inputs: vec![
            InputDecl {
                id: "flag".to_owned(),
                r#type: Type::Bool,
                origin: proposal(),
            },
            InputDecl {
                id: "amount".to_owned(),
                r#type: count.clone(),
                origin: proposal(),
            },
            InputDecl {
                id: "date".to_owned(),
                r#type: Type::Date,
                origin: proposal(),
            },
            InputDecl {
                id: "status".to_owned(),
                r#type: Type::Enum {
                    name: "Status".to_owned(),
                },
                origin: proposal(),
            },
        ],
        derived: declarations
            .into_iter()
            .map(|(id, r#type, expr)| DerivedDecl {
                id: id.to_owned(),
                r#type,
                expr,
                origin: proposal(),
            })
            .collect(),
        outcomes: vec![OutcomeDecl {
            id: "Value".to_owned(),
            fields: vec![FieldDecl {
                id: "value".to_owned(),
                r#type: count,
            }],
            origin: proposal(),
        }],
        domain: vec![Predicate {
            id: "EscapedDomain".to_owned(),
            expr: reference("input.flag"),
            origin: proposal(),
        }],
        assumptions: vec![],
        rules: vec![Rule {
            id: "ReturnAmount".to_owned(),
            when: reference("input.flag"),
            then: RuleConclusion {
                outcome: "Value".to_owned(),
                fields: BTreeMap::from([("value".to_owned(), reference("input.amount"))]),
            },
            origin: proposal(),
        }],
        constraints: vec![Constraint {
            id: "OutcomeValueIsNonnegative".to_owned(),
            for_outcome: "Value".to_owned(),
            must: Expr::Ge {
                left: Box::new(reference("outcome.value")),
                right: Box::new(Expr::Int {
                    value: "0".to_owned(),
                    unit: "count".to_owned(),
                }),
            },
            origin: proposal(),
        }],
        open_items: vec![OpenItem {
            id: "QuotedQuestion".to_owned(),
            text: "Does \"quoted\" text and a newline survive?\nYes.".to_owned(),
            source_ids: vec!["S1".to_owned()],
            affects: "interpretation".to_owned(),
        }],
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn every_json_example_has_a_faithful_text_frontend_and_canonical_round_trip() -> TestResult<()> {
    let limits = ModelLimits::default();
    let paths = model_paths()?;
    assert!(
        !paths.is_empty(),
        "the documentation model fixtures must exist"
    );

    for json_path in paths {
        let text_path = json_path.with_extension("author");
        assert!(
            text_path.is_file(),
            "missing text counterpart for {}",
            json_path.display()
        );

        let json_source = fs::read_to_string(&json_path)?;
        let text_source = fs::read_to_string(&text_path)?;
        let submitted_json: Value = serde_json::from_str(&json_source)?;
        let json_model: RawModel = serde_json::from_value(submitted_json.clone())?;

        let parsed_text = parse_text(&text_source)
            .unwrap_or_else(|diagnostics| panic!("{}: {diagnostics:#?}", text_path.display()));
        assert_eq!(
            normalized_json(&parsed_text)?,
            normalized_json(&json_model)?,
            "text model does not preserve the JSON declarations and metadata: {}",
            text_path.display()
        );

        let compiled_json = compile_json(&submitted_json, &limits)
            .unwrap_or_else(|diagnostics| panic!("{}: {diagnostics:#?}", json_path.display()));
        let compiled_text = compile_text(&text_source, &limits)
            .unwrap_or_else(|diagnostics| panic!("{}: {diagnostics:#?}", text_path.display()));
        assert_eq!(compiled_text.semantic_hash(), compiled_json.semantic_hash());

        let canonical = format_text(&parsed_text)
            .unwrap_or_else(|diagnostic| panic!("{}: {diagnostic:#?}", text_path.display()));
        assert_eq!(
            text_source,
            canonical,
            "text fixture is not in canonical format: {}",
            text_path.display()
        );
        let reparsed = parse_text(&canonical).unwrap_or_else(|diagnostics| {
            panic!("canonical {}: {diagnostics:#?}", text_path.display())
        });
        assert_eq!(normalized_json(&reparsed)?, normalized_json(&parsed_text)?);
        let canonical_again = format_text(&reparsed)
            .unwrap_or_else(|diagnostic| panic!("{}: {diagnostic:#?}", text_path.display()));
        assert_eq!(canonical_again, canonical);
    }

    Ok(())
}

#[test]
fn contractor_access_documentation_example_is_a_complete_supported_model() -> TestResult<()> {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let text_source = fs::read_to_string(docs.join("examples/contractor-access.author"))?;
    let compiled_text = compile_text(&text_source, &ModelLimits::default())
        .unwrap_or_else(|diagnostics| panic!("contractor-access.author: {diagnostics:#?}"));
    assert_eq!(compiled_text.raw().rules.len(), 1);
    assert_eq!(compiled_text.raw().constraints.len(), 2);
    assert_eq!(compiled_text.raw().open_items.len(), 1);
    Ok(())
}

#[test]
fn specification_statement_style_supports_bare_names_multiline_guards_and_positional_fields()
-> TestResult<()> {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let canonical = fs::read_to_string(docs.join("examples/contractor-access.author"))?;
    let statement_style = canonical
        .replace(
            "when and((input.sponsor_approval == ApprovalStatus::Approved), (input.requested_expiry <= input.contract_end))",
            "when sponsor_approval == Approved\n   and requested_expiry <= contract_end",
        )
        .replace(
            "conclude Grant(expiry = input.requested_expiry)",
            "conclude Grant(requested_expiry)",
        )
        .replace(
            "require (input.sponsor_approval == ApprovalStatus::Approved)",
            "require sponsor_approval == Approved",
        )
        .replace(
            "require (outcome.expiry <= input.contract_end)",
            "require expiry <= contract_end",
        );
    assert_ne!(statement_style, canonical);
    let expected = compile_text(&canonical, &ModelLimits::default())
        .unwrap_or_else(|diagnostics| panic!("canonical contractor model: {diagnostics:#?}"));
    let actual = compile_text(&statement_style, &ModelLimits::default())
        .unwrap_or_else(|diagnostics| panic!("SPEC-style contractor model: {diagnostics:#?}"));
    assert_eq!(actual.semantic_hash(), expected.semantic_hash());
    Ok(())
}

#[test]
fn formatter_preserves_reserved_enum_names_and_disambiguates_source_labels() -> TestResult<()> {
    let mut model = exact_expression_model();
    for (enum_name, variant) in [("Bool", "Enabled"), ("Date", "Today"), ("date", "Ready")] {
        model.enums.push(EnumDecl {
            id: enum_name.to_owned(),
            variants: vec![variant.to_owned()],
            origin: proposal(),
        });
        model.derived.push(DerivedDecl {
            id: format!("{enum_name}Literal"),
            r#type: Type::Enum {
                name: enum_name.to_owned(),
            },
            expr: Expr::Enum {
                enum_name: enum_name.to_owned(),
                variant: variant.to_owned(),
            },
            origin: proposal(),
        });
    }

    let canonical = format_text(&model)
        .unwrap_or_else(|diagnostic| panic!("reserved enum model: {diagnostic:#?}"));
    let parsed = parse_text(&canonical)
        .unwrap_or_else(|diagnostics| panic!("reserved enum model: {diagnostics:#?}"));
    assert_eq!(normalized_json(&parsed)?, normalized_json(&model)?);
    assert!(canonical.contains("Enum[Bool]"));
    assert!(canonical.contains("Enum[Date]"));
    assert!(canonical.contains("date::Ready"));

    let mut duplicate_locators = exact_expression_model();
    duplicate_locators.sources.push(Source {
        id: "S2".to_owned(),
        text: "Second source.".to_owned(),
        locator: Some("policy.section_1".to_owned()),
    });
    let constraint = duplicate_locators
        .constraints
        .first_mut()
        .ok_or_else(|| std::io::Error::other("synthetic model has no constraint"))?;
    constraint.origin = Origin {
        kind: OriginKind::Source,
        source_ids: vec!["S2".to_owned()],
        review: Review::Confirmed,
        note: None,
    };
    let canonical = format_text(&duplicate_locators)
        .unwrap_or_else(|diagnostic| panic!("duplicate source locator model: {diagnostic:#?}"));
    let parsed = parse_text(&canonical)
        .unwrap_or_else(|diagnostics| panic!("duplicate source locator model: {diagnostics:#?}"));
    assert_eq!(
        normalized_json(&parsed)?,
        normalized_json(&duplicate_locators)?
    );

    let mut colliding_locator = exact_expression_model();
    colliding_locator.sources.push(Source {
        id: "S2".to_owned(),
        text: "Second source.".to_owned(),
        locator: Some("policy.section_2".to_owned()),
    });
    let source = colliding_locator
        .sources
        .iter_mut()
        .find(|source| source.id == "S1")
        .ok_or_else(|| std::io::Error::other("synthetic model has no S1"))?;
    source.locator = Some("S2".to_owned());
    let constraint = colliding_locator
        .constraints
        .first_mut()
        .ok_or_else(|| std::io::Error::other("synthetic model has no constraint"))?;
    constraint.origin = Origin {
        kind: OriginKind::Source,
        source_ids: vec!["S1".to_owned()],
        review: Review::Confirmed,
        note: None,
    };
    let canonical = format_text(&colliding_locator)
        .unwrap_or_else(|diagnostic| panic!("colliding source label model: {diagnostic:#?}"));
    let parsed = parse_text(&canonical)
        .unwrap_or_else(|diagnostics| panic!("colliding source label model: {diagnostics:#?}"));
    assert_eq!(
        normalized_json(&parsed)?,
        normalized_json(&colliding_locator)?
    );
    Ok(())
}

#[test]
fn unqualified_constraint_fields_shadow_same_named_inputs() -> TestResult<()> {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let source = fs::read_to_string(docs.join("examples/contractor-access.author"))?;
    let mut model = parse_text(&source)
        .unwrap_or_else(|diagnostics| panic!("contractor-access.author: {diagnostics:#?}"));
    model.inputs.push(InputDecl {
        id: "expiry".to_owned(),
        r#type: Type::Date,
        origin: proposal(),
    });

    let canonical =
        format_text(&model).unwrap_or_else(|diagnostic| panic!("shadowing model: {diagnostic:#?}"));
    let spec_style = canonical.replace(
        "require (outcome.expiry <= input.contract_end)",
        "require expiry <= contract_end",
    );
    assert_ne!(spec_style, canonical);

    let compiled = compile_text(&spec_style, &ModelLimits::default())
        .unwrap_or_else(|diagnostics| panic!("shadowing model: {diagnostics:#?}"));
    let constraint = compiled
        .raw()
        .constraints
        .iter()
        .find(|constraint| constraint.id == "ExpiryWithinContract")
        .ok_or_else(|| std::io::Error::other("missing expiry constraint"))?;
    let expected = Expr::Le {
        left: Box::new(reference("outcome.expiry")),
        right: Box::new(reference("input.contract_end")),
    };
    assert_eq!(
        normalized_json(&constraint.must)?,
        normalized_json(&expected)?
    );
    Ok(())
}

#[test]
fn text_frontend_rejects_syntax_unknown_references_and_type_errors() -> TestResult<()> {
    let json_source = fs::read_to_string(examples_dir().join("complete-proposed.json"))?;
    let mut json: Value = serde_json::from_str(&json_source)?;
    let model: RawModel = serde_json::from_value(json.clone())?;
    let canonical = format_text(&model)
        .unwrap_or_else(|diagnostic| panic!("complete-proposed.json: {diagnostic:#?}"));

    let malformed = format!("{canonical}\nnot a declaration");
    assert!(
        parse_text(&malformed).is_err(),
        "trailing garbage must be rejected"
    );

    for malformed in [
        "",
        "decision",
        "decision M version \"author-model/0.1\"\nsource S1 text \"unterminated",
        "decision M version \"author-model/0.1\"\ninput x: Bool origin proposal review proposed sources []",
        "decision M version \"author-model/0.1\"\n@unknown syntax",
    ] {
        let parsed = std::panic::catch_unwind(|| parse_text(malformed));
        assert!(parsed.is_ok(), "parser panicked on {malformed:?}");
        assert!(
            parsed.is_ok_and(|result| result.is_err()),
            "accepted {malformed:?}"
        );
    }

    replace_reference(
        &mut json,
        "input.sponsor_approval",
        "input.missing_sponsor_approval",
    );
    let unknown_reference_model: RawModel = serde_json::from_value(json.clone())?;
    let unknown_reference = format_text(&unknown_reference_model)
        .unwrap_or_else(|diagnostic| panic!("unknown-reference mutation: {diagnostic:#?}"));
    assert!(
        compile_text(&unknown_reference, &ModelLimits::default()).is_err(),
        "an undeclared input reference must be rejected"
    );

    let mut json: Value = serde_json::from_str(&json_source)?;
    replace_reference(&mut json, "input.contract_end", "input.sponsor_approval");
    let type_mismatch_model: RawModel = serde_json::from_value(json)?;
    let type_mismatch = format_text(&type_mismatch_model)
        .unwrap_or_else(|diagnostic| panic!("type-mismatch mutation: {diagnostic:#?}"));
    assert!(
        compile_text(&type_mismatch, &ModelLimits::default()).is_err(),
        "comparing dates with an enum must be rejected"
    );

    Ok(())
}

#[test]
fn parser_rejects_duplicate_headers_and_malformed_blocks_and_sources() -> TestResult<()> {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
    let canonical = fs::read_to_string(docs.join("examples/contractor-access.author"))?;

    let duplicate_decision = format!("{canonical}\ndecision Duplicate");
    assert!(parse_text(&duplicate_decision).is_err());
    let duplicate_version = format!("{canonical}\nversion \"author-model/0.1\"");
    assert!(parse_text(&duplicate_version).is_err());

    let source_trailing_garbage = canonical.replace(
        "locator \"policy.section_2\"",
        "locator \"policy.section_2\" ignored",
    );
    assert_ne!(source_trailing_garbage, canonical);
    assert!(parse_text(&source_trailing_garbage).is_err());

    let unknown_rule_line = canonical.replace(
        "  status proposed\n  origin proposal",
        "  status proposed\n  unsupported clause\n  origin proposal",
    );
    assert_ne!(unknown_rule_line, canonical);
    assert!(parse_text(&unknown_rule_line).is_err());

    let commented_unknown_rule = format!("# preserved source line numbers\n{unknown_rule_line}");
    let diagnostics = parse_text(&commented_unknown_rule)
        .err()
        .ok_or_else(|| std::io::Error::other("accepted unknown rule clause"))?;
    let diagnostic = diagnostics
        .first()
        .ok_or_else(|| std::io::Error::other("missing syntax diagnostic"))?;
    let rule_line = commented_unknown_rule
        .lines()
        .position(|line| line.starts_with("rule GrantApprovedRequest"))
        .map(|index| index + 1)
        .ok_or_else(|| std::io::Error::other("missing rule line"))?;
    assert_eq!(
        diagnostic.declaration_id.as_deref(),
        Some("GrantApprovedRequest")
    );
    assert_eq!(
        diagnostic.pointer.as_deref(),
        Some(format!("text:line:{rule_line}").as_str())
    );

    let duplicate_conclusion = canonical.replace(
        "  conclude Grant(expiry = input.requested_expiry)",
        "  conclude Grant(expiry = input.requested_expiry)\n  conclude Grant(expiry = input.requested_expiry)",
    );
    assert_ne!(duplicate_conclusion, canonical);
    assert!(parse_text(&duplicate_conclusion).is_err());

    let duplicate_outcome_field = canonical.replace(
        "  conclude Grant(expiry = input.requested_expiry)",
        "  conclude Grant(expiry = input.requested_expiry, expiry = input.requested_expiry)",
    );
    assert_ne!(duplicate_outcome_field, canonical);
    assert!(parse_text(&duplicate_outcome_field).is_err());
    Ok(())
}

#[test]
fn canonical_text_preserves_every_expression_kind_and_metadata() -> TestResult<()> {
    let model = exact_expression_model();
    let canonical = format_text(&model)
        .unwrap_or_else(|diagnostic| panic!("synthetic expression model: {diagnostic:#?}"));
    let decorated = format!("# ignored comment\n\n{canonical}");
    let parsed = parse_text(&decorated)
        .unwrap_or_else(|diagnostics| panic!("synthetic expression model: {diagnostics:#?}"));
    assert_eq!(normalized_json(&parsed)?, normalized_json(&model)?);

    let canonical_again = format_text(&parsed)
        .unwrap_or_else(|diagnostic| panic!("reformat synthetic model: {diagnostic:#?}"));
    assert_eq!(canonical_again, canonical);

    let missing_date_parentheses = canonical.replace("date(\"2024-02-29\")", "date \"2024-02-29\"");
    assert_ne!(missing_date_parentheses, canonical);
    assert!(parse_text(&missing_date_parentheses).is_err());

    let missing_scale_parenthesis =
        canonical.replace("scale(-2, input.amount)", "scale(-2, input.amount");
    assert_ne!(missing_scale_parenthesis, canonical);
    assert!(parse_text(&missing_scale_parenthesis).is_err());

    let conflicting_status = canonical.replacen("status proposed", "status confirmed", 1);
    assert_ne!(conflicting_status, canonical);
    assert!(parse_text(&conflicting_status).is_err());

    let without_integer_unit = canonical.replace(
        "123456789012345678901234567890[count]",
        "123456789012345678901234567890",
    );
    assert_ne!(without_integer_unit, canonical);
    assert!(parse_text(&without_integer_unit).is_err());

    let open_parens = "(".repeat(140);
    let close_parens = ")".repeat(140);
    let deep_expression = canonical.replace(
        "domain EscapedDomain require input.flag",
        &format!("domain EscapedDomain require {open_parens}input.flag{close_parens}"),
    );
    assert_ne!(deep_expression, canonical);
    assert!(parse_text(&deep_expression).is_err());
    Ok(())
}
