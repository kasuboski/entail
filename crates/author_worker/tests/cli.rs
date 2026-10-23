#![allow(clippy::panic_in_result_fn)] // Process IO uses `?`; assertions remain the test oracle.

use std::error::Error;
use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

type TestResult<T> = Result<T, Box<dyn Error>>;

fn fixture(relative_path: &str) -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs")
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

fn run_text(
    arguments: &[&str],
    input: &str,
) -> TestResult<(std::process::ExitStatus, String, String)> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_entail"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("child stdin unavailable")?
        .write_all(input.as_bytes())?;
    let output = child.wait_with_output()?;
    Ok((
        output.status,
        String::from_utf8(output.stdout)?,
        String::from_utf8(output.stderr)?,
    ))
}

#[test]
fn help_and_validate_emit_json_with_documented_exit_codes() -> TestResult<()> {
    let (help_status, help, help_stderr) = run(&["--help"])?;
    assert!(help_status.success());
    assert!(help.get("usage").is_some());
    assert!(help_stderr.is_empty());

    let valid_path = fixture("examples/models/partial.json");
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
    let model = fixture("examples/models/unsafe-grant.json");
    let text_model = fixture("examples/models/unsafe-grant.author");
    let scenario = fixture("examples/scenarios/unsafe-expiry.json");
    let (json_validate_status, json_valid, _) = run(&["validate", &model])?;
    let (text_validate_status, text_valid, _) = run(&["validate", &text_model])?;
    assert!(json_validate_status.success());
    assert!(text_validate_status.success());
    assert_eq!(
        text_valid.get("content_hash"),
        json_valid.get("content_hash")
    );
    assert_eq!(
        text_valid.get("semantic_hash"),
        json_valid.get("semantic_hash")
    );
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

    let (text_evaluate_status, text_evaluation, _) = run(&["evaluate", &text_model, &scenario])?;
    assert!(text_evaluate_status.success());
    assert_eq!(
        text_evaluation
            .get("classification")
            .and_then(Value::as_str),
        Some("unique")
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

    let (text_analyze_status, text_report, _) = run(&[
        "analyze",
        &text_model,
        "--checks",
        "coverage,constraint_compliance",
        "--assumptions",
        "",
        "--budget-ms",
        "5000",
    ])?;
    assert!(text_analyze_status.success());
    assert!(
        text_report
            .get("results")
            .and_then(Value::as_array)
            .is_some_and(|results| results.iter().any(|result| {
                result.get("finding_category").and_then(Value::as_str) == Some("violation")
            }))
    );
    Ok(())
}

#[test]
fn text_models_and_fmt_support_stdin_file_and_check_modes() -> TestResult<()> {
    let input = "# comment\n\ndecision Test\n\nversion \"author-model/0.1\"\ntype Status = Open | Closed origin proposal review proposed sources []\ninput current: Status origin proposal review proposed sources []\n";
    let expected = "decision Test\nversion \"author-model/0.1\"\ntype Status = Open | Closed origin proposal review proposed sources []\ninput current: Status origin proposal review proposed sources []\n";

    let (stdin_status, stdin_stdout, stdin_stderr) = run_text(&["fmt"], input)?;
    assert!(stdin_status.success());
    assert_eq!(stdin_stdout, expected);
    assert!(stdin_stderr.is_empty());

    let path = std::env::temp_dir().join(format!("entail-cli-fmt-{}.author", std::process::id()));
    std::fs::write(&path, input)?;
    let path_string = path.to_string_lossy().into_owned();
    let (check_status, check_stdout, check_stderr) =
        run_text(&["fmt", "--check", &path_string], "")?;
    assert_eq!(check_status.code(), Some(1));
    assert!(check_stdout.is_empty());
    assert!(check_stderr.contains("needs formatting"));
    assert_eq!(std::fs::read_to_string(&path)?, input);

    let (format_status, format_stdout, format_stderr) = run_text(&["fmt", &path_string], "")?;
    assert!(format_status.success());
    assert!(format_stdout.is_empty());
    assert!(format_stderr.is_empty());
    assert_eq!(std::fs::read_to_string(&path)?, expected);

    let (valid_status, valid, _) = run(&["validate", &path_string])?;
    assert!(valid_status.success());
    assert_eq!(valid.get("valid"), Some(&Value::Bool(true)));
    std::fs::remove_file(path)?;
    Ok(())
}
