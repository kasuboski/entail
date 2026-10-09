#![allow(clippy::panic_in_result_fn)] // Process IO uses `?`; assertions remain the test oracle.

use std::error::Error;
use std::process::{Command, Stdio};

use serde_json::Value;

type TestResult<T> = Result<T, Box<dyn Error>>;

fn fixture(relative_path: &str) -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../author-engine-v0")
        .join(relative_path)
        .to_string_lossy()
        .into_owned()
}

fn run(arguments: &[&str]) -> TestResult<(std::process::ExitStatus, Value, String)> {
    let output = Command::new(env!("CARGO_BIN_EXE_entail"))
        .args(arguments)
        .stdin(Stdio::null())
        .output()?;
    let stdout: Value = serde_json::from_slice(&output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    Ok((output.status, stdout, stderr))
}

#[test]
fn help_and_validate_emit_json_with_documented_exit_codes() -> TestResult<()> {
    let (help_status, help, help_stderr) = run(&["--help"])?;
    assert!(help_status.success());
    assert!(help.get("usage").is_some());
    assert!(help_stderr.is_empty());

    let valid_path = fixture("fixtures/valid/partial.json");
    let (valid_status, valid, valid_stderr) = run(&["validate", &valid_path])?;
    assert!(valid_status.success());
    assert_eq!(valid.get("valid"), Some(&Value::Bool(true)));
    assert_eq!(
        valid.get("language_version").and_then(Value::as_str),
        Some("author-model/0.1")
    );
    assert_eq!(
        valid.get("protocol_version").and_then(Value::as_str),
        Some("author-worker/0.1")
    );
    assert!(
        valid
            .get("engine_version")
            .and_then(Value::as_str)
            .is_some()
    );
    assert!(
        valid
            .get("solver_version")
            .and_then(Value::as_str)
            .is_some()
    );
    assert!(matches!(
        valid.get("readiness").and_then(Value::as_str),
        Some("ready_for_analysis" | "incomplete_model")
    ));
    assert!(valid_stderr.is_empty());

    let invalid_path = fixture("fixtures/invalid/unknown-reference.json");
    let (invalid_status, invalid, _) = run(&["validate", &invalid_path])?;
    assert_eq!(invalid_status.code(), Some(2));
    assert!(invalid.get("diagnostics").is_some());
    Ok(())
}

#[test]
fn evaluate_and_analyze_return_structured_findings_without_failure_exit() -> TestResult<()> {
    let model = fixture("fixtures/valid/unsafe-grant.json");
    let scenario = fixture("scenarios/unsafe-expiry.json");
    let (evaluate_status, evaluation, _) = run(&["evaluate", &model, &scenario])?;
    assert!(evaluate_status.success());
    assert_eq!(
        evaluation.get("classification").and_then(Value::as_str),
        Some("unique")
    );
    assert_eq!(
        evaluation
            .get("violated_constraint_ids")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );

    let (analyze_status, report, _) = run(&[
        "analyze",
        &model,
        "--checks",
        "coverage,constraint_compliance",
        "--assumptions",
        "",
        "--budget-ms",
        "5000",
    ])?;
    assert!(analyze_status.success());
    assert_eq!(report.get("complete").and_then(Value::as_bool), Some(true));
    assert!(
        report
            .get("results")
            .and_then(Value::as_array)
            .is_some_and(|results| {
                results.iter().any(|result| {
                    result.get("finding_category").and_then(Value::as_str) == Some("violation")
                })
            })
    );
    Ok(())
}
