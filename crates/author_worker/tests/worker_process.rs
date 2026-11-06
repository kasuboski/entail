#![allow(clippy::panic_in_result_fn)] // Process IO uses `?`; assertions remain the test oracle.

use std::error::Error;
use std::io::{self, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

const MAX_FRAME: usize = 8 * 1024 * 1024;

type TestResult<T> = Result<T, Box<dyn Error>>;

fn spawn(max_handles: usize) -> io::Result<(Child, ChildStdin, BufReader<ChildStdout>)> {
    spawn_with_budget(max_handles, 60_000)
}

fn spawn_with_budget(
    max_handles: usize,
    max_budget_ms: u64,
) -> io::Result<(Child, ChildStdin, BufReader<ChildStdout>)> {
    spawn_with_limits(max_handles, max_budget_ms, 10_000)
}

fn spawn_with_limits(
    max_handles: usize,
    max_budget_ms: u64,
    max_obligations: usize,
) -> io::Result<(Child, ChildStdin, BufReader<ChildStdout>)> {
    let max_handles = max_handles.to_string();
    let max_budget_ms = max_budget_ms.to_string();
    let max_obligations = max_obligations.to_string();
    let mut child = Command::new(env!("CARGO_BIN_EXE_entail"))
        .args([
            "worker",
            "--max-handles",
            max_handles.as_str(),
            "--max-budget-ms",
            max_budget_ms.as_str(),
            "--max-obligations",
            max_obligations.as_str(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let input = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("worker stdin missing"))?;
    let output = BufReader::new(
        child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("worker stdout missing"))?,
    );
    Ok((child, input, output))
}

fn write_request(input: &mut ChildStdin, request: &Value, fragment: bool) -> io::Result<()> {
    let payload = serde_json::to_vec(request).map_err(io::Error::other)?;
    let length = u32::try_from(payload.len()).map_err(io::Error::other)?;
    input.write_all(&length.to_be_bytes())?;
    if fragment {
        for byte in payload {
            input.write_all(&[byte])?;
        }
    } else {
        input.write_all(&payload)?;
    }
    input.flush()
}

fn read_response(output: &mut BufReader<ChildStdout>) -> TestResult<Value> {
    let mut header = [0_u8; 4];
    output.read_exact(&mut header)?;
    let length = usize::try_from(u32::from_be_bytes(header))?;
    assert!(length <= MAX_FRAME);
    let mut payload = vec![0; length];
    output.read_exact(&mut payload)?;
    Ok(serde_json::from_slice(&payload)?)
}

fn request(request_id: &str, op: &str, params: Value) -> Value {
    let mut envelope = serde_json::Map::new();
    envelope.insert("protocol_version".to_owned(), json!("author-worker/0.1"));
    envelope.insert("request_id".to_owned(), json!(request_id));
    envelope.insert("op".to_owned(), json!(op));
    envelope.insert("params".to_owned(), params);
    Value::Object(envelope)
}

fn model() -> TestResult<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/examples/models/partial.json");
    let bytes = std::fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn response_field<'a>(response: &'a Value, field: &str) -> TestResult<&'a Value> {
    response
        .get(field)
        .ok_or_else(|| io::Error::other(format!("response has no {field}")).into())
}

#[test]
fn text_compile_supports_analysis_evaluation_and_diagnostics_without_leaking_handles()
-> TestResult<()> {
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/examples/models/partial.author"),
    )?;
    let (mut child, mut input, mut output) = spawn(2)?;
    write_request(
        &mut input,
        &request("text", "compile", json!({"text":text})),
        false,
    )?;
    let compiled = read_response(&mut output)?;
    let result = response_field(&compiled, "result")?;
    assert_eq!(result.get("valid"), Some(&json!(true)));
    assert_eq!(result.get("readiness"), Some(&json!("ready_for_analysis")));
    let handle = result
        .get("handle")
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::other("missing handle"))?
        .to_owned();
    write_request(
        &mut input,
        &request("analysis", "analyze", json!({"handle":handle})),
        false,
    )?;
    let analysis_response = read_response(&mut output)?;
    let analysis = response_field(&analysis_response, "result")?;
    assert_eq!(analysis.get("complete"), Some(&json!(true)));
    assert_eq!(analysis.get("model_id"), result.get("model_id"));
    assert_eq!(analysis.get("semantic_hash"), result.get("semantic_hash"));
    assert!(
        analysis
            .get("results")
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty())
    );
    write_request(
        &mut input,
        &request(
            "evaluation",
            "evaluate",
            json!({"handle":handle,"inputs":{
                "sponsor_approval":{"kind":"enum","enum":"ApprovalStatus","variant":"Approved"},
                "contract_end":{"kind":"date","value":"2026-12-31"},
                "requested_expiry":{"kind":"date","value":"2026-11-30"}
            }}),
        ),
        false,
    )?;
    let evaluation_response = read_response(&mut output)?;
    let evaluation = response_field(&evaluation_response, "result")?;
    assert_eq!(evaluation.get("matched_rule_ids"), Some(&json!([])));
    assert_eq!(
        evaluation.get("classification"),
        Some(&json!("unspecified"))
    );
    assert_eq!(evaluation.get("outcomes"), Some(&json!([])));
    write_request(
        &mut input,
        &request("bad-text", "compile", json!({"text":"not valid"})),
        false,
    )?;
    let bad_response = read_response(&mut output)?;
    let bad = response_field(&bad_response, "result")?;
    assert_eq!(bad.get("valid"), Some(&json!(false)));
    assert_eq!(bad.get("handle"), Some(&Value::Null));
    assert!(
        bad.get("diagnostics")
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty())
    );
    // Failed compilation must not consume the remaining slot.
    write_request(
        &mut input,
        &request("json", "compile", json!({"model":model()?})),
        false,
    )?;
    let json_response = read_response(&mut output)?;
    let json_result = response_field(&json_response, "result")?;
    assert_eq!(json_result.get("valid"), Some(&json!(true)));
    for field in ["content_hash", "semantic_hash", "model_id", "readiness"] {
        assert_eq!(json_result.get(field), result.get(field));
    }
    assert_ne!(json_result.get("handle"), result.get("handle"));
    write_request(
        &mut input,
        &request("full", "compile", json!({"text":text})),
        false,
    )?;
    let full_response = read_response(&mut output)?;
    assert_eq!(
        response_field(response_field(&full_response, "error")?, "code")?,
        "capacity_exceeded"
    );
    drop(input);
    assert!(child.wait()?.success());
    Ok(())
}

#[test]
fn text_compile_rejects_invalid_parameter_combinations_without_consuming_capacity() -> TestResult<()>
{
    let (mut child, mut input, mut output) = spawn(1)?;
    for params in [
        json!({"model":model()?,"text":""}),
        json!({}),
        json!({"text":1}),
        json!({"text":null}),
        json!({"text":"", "unknown":true}),
        json!({"unknown":""}),
    ] {
        write_request(&mut input, &request("bad-params", "compile", params), false)?;
        let response = read_response(&mut output)?;
        assert_eq!(
            response_field(response_field(&response, "error")?, "code")?,
            "invalid_params"
        );
    }
    write_request(
        &mut input,
        &request("valid", "compile", json!({"model":model()?})),
        false,
    )?;
    let response = read_response(&mut output)?;
    assert_eq!(
        response_field(&response, "result")?.get("valid"),
        Some(&json!(true))
    );
    drop(input);
    assert!(child.wait()?.success());
    Ok(())
}

#[test]
fn omitted_budget_defaults_are_clamped_and_invalid_numeric_values_are_rejected() -> TestResult<()> {
    let (mut child, mut input, mut output) = spawn_with_limits(2, 500, 5)?;
    write_request(
        &mut input,
        &request("compile", "compile", json!({"model":model()?})),
        false,
    )?;
    let compiled = read_response(&mut output)?;
    let handle = response_field(response_field(&compiled, "result")?, "handle")?
        .as_str()
        .ok_or_else(|| io::Error::other("compile result handle is not a string"))?
        .to_owned();
    for (id, budget) in [("omitted", None), ("empty", Some(json!({})))] {
        let mut params = json!({"handle":handle});
        if let Some(budget) = budget {
            params["budget"] = budget;
        }
        write_request(&mut input, &request(id, "analyze", params), false)?;
        let response = read_response(&mut output)?;
        let result = response_field(&response, "result")?;
        let effective = response_field(result, "budget")?;
        assert_eq!(effective.get("max_obligations"), Some(&json!(5)));
        assert_eq!(effective.get("total_timeout_ms"), Some(&json!(500)));
        assert_eq!(
            effective.get("per_obligation_timeout_ms"),
            Some(&json!(500))
        );
    }
    for (id, budget) in [
        ("null-total", json!({"total_timeout_ms":null})),
        ("zero", json!({"total_timeout_ms":0})),
        ("fraction", json!({"total_timeout_ms":1.5})),
        (
            "overflow",
            json!({"total_timeout_ms":u128::from(u64::MAX) + 1}),
        ),
        ("zero-resource", json!({"solver_resource_limit":0})),
        ("unknown", json!({"unexpected":1})),
    ] {
        write_request(
            &mut input,
            &request(id, "analyze", json!({"handle":handle,"budget":budget})),
            false,
        )?;
        assert_eq!(
            response_field(
                response_field(&read_response(&mut output)?, "error")?,
                "code"
            )?,
            "invalid_params",
            "case {id}"
        );
    }
    drop(input);
    assert!(child.wait()?.success());
    Ok(())
}

// Keep the live-process lifecycle together so capacity and stale-handle behavior
// are observed against one process and its replacement.
#[allow(clippy::too_many_lines)]
#[test]
fn process_serves_fragmented_requests_and_enforces_capacity_release_and_restart() -> TestResult<()>
{
    let (mut child, mut input, mut output) = spawn(1)?;
    write_request(&mut input, &request("hello", "hello", json!({})), true)?;
    let hello = read_response(&mut output)?;
    assert!(hello.get("result").is_some());
    assert!(
        response_field(&hello, "result")?
            .get("capabilities")
            .is_some()
    );

    write_request(
        &mut input,
        &request("invalid-model", "compile", json!({"model":{}})),
        false,
    )?;
    let invalid = read_response(&mut output)?;
    assert!(
        !response_field(response_field(&invalid, "result")?, "valid")?
            .as_bool()
            .unwrap_or(true)
    );
    assert!(response_field(response_field(&invalid, "result")?, "handle")?.is_null());

    let model_value = model()?;
    write_request(
        &mut input,
        &request("compile", "compile", json!({"model":model_value})),
        true,
    )?;
    let compiled = read_response(&mut output)?;
    assert!(response_field(response_field(&compiled, "result")?, "compile_elapsed_ms")?.is_u64());
    let handle = response_field(response_field(&compiled, "result")?, "handle")?
        .as_str()
        .ok_or_else(|| io::Error::other("compile result handle is not a string"))?
        .to_owned();

    write_request(
        &mut input,
        &request("capacity", "compile", json!({"model":model()?})),
        false,
    )?;
    assert_eq!(
        response_field(
            response_field(&read_response(&mut output)?, "error")?,
            "code"
        )?,
        "capacity_exceeded"
    );

    write_request(
        &mut input,
        &request(
            "bad-params",
            "analyze",
            json!({"handle":handle,"unexpected":true}),
        ),
        false,
    )?;
    assert_eq!(
        response_field(
            response_field(&read_response(&mut output)?, "error")?,
            "code"
        )?,
        "invalid_params"
    );
    write_request(
        &mut input,
        &request(
            "analyze",
            "analyze",
            json!({
                "handle":handle,
                "checks":["coverage","coverage"],
                "assumption_ids":["SponsorsAlreadyApproved","SponsorsAlreadyApproved"]
            }),
        ),
        false,
    )?;
    let report_response = read_response(&mut output)?;
    let report = response_field(&report_response, "result")?;
    assert_eq!(
        report.get("checks").and_then(Value::as_array).map(Vec::len),
        Some(2)
    );
    assert_eq!(
        report
            .get("selected_assumption_ids")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );

    write_request(
        &mut input,
        &request(
            "evaluate",
            "evaluate",
            json!({
                "handle":handle,
                "inputs":{
                    "sponsor_approval":{"kind":"enum","enum":"ApprovalStatus","variant":"Unknown"},
                    "contract_end":{"kind":"date","value":"2026-12-31"},
                    "requested_expiry":{"kind":"date","value":"2026-12-01"}
                }
            }),
        ),
        false,
    )?;
    assert!(read_response(&mut output)?.get("result").is_some());

    write_request(
        &mut input,
        &request("release", "release", json!({"handle":handle})),
        false,
    )?;
    assert!(
        response_field(
            response_field(&read_response(&mut output)?, "result")?,
            "released"
        )?
        .as_bool()
        .unwrap_or(false)
    );
    write_request(
        &mut input,
        &request("compile-again", "compile", json!({"model":model()?})),
        false,
    )?;
    let restarted_model_handle = response_field(
        response_field(&read_response(&mut output)?, "result")?,
        "handle",
    )?
    .as_str()
    .ok_or_else(|| io::Error::other("compile result handle is not a string"))?
    .to_owned();
    child.kill()?;
    let status = child.wait()?;
    assert!(!status.success());
    drop(input);

    let (mut restarted, mut restarted_input, mut restarted_output) = spawn(1)?;
    write_request(
        &mut restarted_input,
        &request("recompile", "compile", json!({"model":model()?})),
        false,
    )?;
    let replacement_handle = response_field(
        response_field(&read_response(&mut restarted_output)?, "result")?,
        "handle",
    )?
    .as_str()
    .ok_or_else(|| io::Error::other("replacement handle is not a string"))?
    .to_owned();
    assert_ne!(replacement_handle, restarted_model_handle);
    write_request(
        &mut restarted_input,
        &request("stale", "analyze", json!({"handle":restarted_model_handle})),
        false,
    )?;
    assert_eq!(
        response_field(
            response_field(&read_response(&mut restarted_output)?, "error")?,
            "code"
        )?,
        "unknown_handle"
    );
    drop(restarted_input);
    assert!(restarted.wait()?.success());
    Ok(())
}

#[test]
fn killing_during_native_solver_analysis_reaps_process_and_invalidates_handles() -> TestResult<()> {
    let (mut child, mut input, mut output) = spawn(2)?;
    let model_value = model()?;
    write_request(
        &mut input,
        &request("compile", "compile", json!({"model":model_value})),
        false,
    )?;
    let compiled = read_response(&mut output)?;
    let old_handle = response_field(response_field(&compiled, "result")?, "handle")?
        .as_str()
        .ok_or_else(|| io::Error::other("compile result handle is not a string"))?
        .to_owned();

    let complete_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/examples/models/complete-proposed.json");
    let mut stress_model: Value = serde_json::from_slice(&std::fs::read(complete_path)?)?;
    let rules = stress_model
        .get_mut("rules")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| io::Error::other("fixture has no rules"))?;
    let template = rules
        .first()
        .cloned()
        .ok_or_else(|| io::Error::other("fixture has no rule"))?;
    for index in rules.len()..160 {
        let mut rule = template.clone();
        rule["id"] = json!(format!("StressRule{index}"));
        rules.push(rule);
    }
    write_request(
        &mut input,
        &request("stress-compile", "compile", json!({"model":stress_model})),
        false,
    )?;
    let stress_compiled = read_response(&mut output)?;
    let stress_handle = response_field(response_field(&stress_compiled, "result")?, "handle")?
        .as_str()
        .ok_or_else(|| io::Error::other("stress compile handle is not a string"))?
        .to_owned();
    write_request(
        &mut input,
        &request(
            "active-analysis",
            "analyze",
            json!({
                "handle":stress_handle,
                "checks":["outcome_conflict"],
                "budget":{"max_obligations":10000,"total_timeout_ms":60000,"per_obligation_timeout_ms":60000}
            }),
        ),
        false,
    )?;
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = read_response(&mut output);
        sender.send(()).ok();
    });
    assert!(matches!(
        receiver.recv_timeout(std::time::Duration::from_millis(100)),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ));
    assert!(
        child.try_wait()?.is_none(),
        "worker exited before cancellation"
    );
    child.kill()?;
    let status = child.wait()?;
    assert!(!status.success());
    drop(input);

    let (mut replacement, mut replacement_input, mut replacement_output) = spawn(2)?;
    write_request(
        &mut replacement_input,
        &request("recompile", "compile", json!({"model":model()?})),
        false,
    )?;
    let replacement_compiled = read_response(&mut replacement_output)?;
    let replacement_handle =
        response_field(response_field(&replacement_compiled, "result")?, "handle")?
            .as_str()
            .ok_or_else(|| io::Error::other("replacement handle is not a string"))?;
    assert_ne!(replacement_handle, old_handle);
    write_request(
        &mut replacement_input,
        &request("stale", "analyze", json!({"handle":old_handle})),
        false,
    )?;
    assert_eq!(
        response_field(
            response_field(&read_response(&mut replacement_output)?, "error")?,
            "code"
        )?,
        "unknown_handle"
    );
    drop(replacement_input);
    assert!(replacement.wait()?.success());
    Ok(())
}

#[test]
fn malformed_json_recovers_only_unambiguous_top_level_request_ids() -> TestResult<()> {
    let (mut child, mut input, mut output) = spawn(2)?;
    let cases: [(&[u8], &str); 7] = [
        (br#"{"params":{"request_id":"nested"},"protocol_version":"author-worker/0.1","request_id":"outer","op":"hello",BROKEN}"#, "outer"),
        (br#"{"note":"text \"request_id\":\"fake\"","request_id":"real","op":BROKEN}"#, "real"),
        (br#"{"items":[{"request_id":"nested"}],"request_id":"array-root","op":BROKEN}"#, "array-root"),
        (br#"{"request_id":"before-corruption","op":BROKEN}"#, "before-corruption"),
        (br#"{"\u0072equest_id":"escaped-key","op":BROKEN}"#, "escaped-key"),
        (br#"{"op":BROKEN,"request_id":"unreached"}"#, "null"),
        (br#"{"params":{"request_id":"nested"},"op":BROKEN}"#, "null"),
    ];
    for (payload, expected_id) in cases {
        let length = u32::try_from(payload.len())?;
        input.write_all(&length.to_be_bytes())?;
        input.write_all(payload)?;
        let response = read_response(&mut output)?;
        assert_eq!(
            response
                .get("request_id")
                .and_then(Value::as_str)
                .unwrap_or("null"),
            expected_id
        );
        assert_eq!(
            response_field(response_field(&response, "error")?, "code")?,
            "malformed_json"
        );
    }
    write_request(
        &mut input,
        &request("still-alive", "hello", json!({})),
        false,
    )?;
    assert_eq!(
        response_field(&read_response(&mut output)?, "request_id")?,
        "still-alive"
    );
    drop(input);
    assert!(child.wait()?.success());
    Ok(())
}

#[test]
fn malformed_json_recovers_request_id_and_worker_continues() -> TestResult<()> {
    let (mut child, mut input, mut output) = spawn(2)?;
    let payload = br#"{"protocol_version":"author-worker/0.1","request_id":"recover-me",}"#;
    let length = u32::try_from(payload.len())?;
    input.write_all(&length.to_be_bytes())?;
    input.write_all(payload)?;
    assert_eq!(
        response_field(&read_response(&mut output)?, "request_id")?,
        "recover-me"
    );
    write_request(
        &mut input,
        &request("after-malformed", "hello", json!({})),
        false,
    )?;
    assert!(read_response(&mut output)?.get("result").is_some());

    let mut wrong_version = request("wrong-version", "hello", json!({}));
    if let Some(object) = wrong_version.as_object_mut() {
        object.insert("protocol_version".to_owned(), json!("author-worker/9"));
    }
    write_request(&mut input, &wrong_version, false)?;
    assert_eq!(
        response_field(
            response_field(&read_response(&mut output)?, "error")?,
            "code"
        )?,
        "version_mismatch"
    );
    write_request(
        &mut input,
        &request("unknown-op", "unknown", json!({})),
        false,
    )?;
    assert_eq!(
        response_field(
            response_field(&read_response(&mut output)?, "error")?,
            "code"
        )?,
        "unknown_operation"
    );
    let mut extra_field = request("extra-field", "hello", json!({}));
    if let Some(object) = extra_field.as_object_mut() {
        object.insert("unexpected".to_owned(), json!(true));
    }
    write_request(&mut input, &extra_field, false)?;
    assert_eq!(
        response_field(
            response_field(&read_response(&mut output)?, "error")?,
            "code"
        )?,
        "invalid_envelope"
    );
    drop(input);
    assert!(child.wait()?.success());
    Ok(())
}

#[test]
fn truncated_header_and_payload_terminate_nonzero() -> TestResult<()> {
    for bytes in [&[0_u8, 0, 0][..], &[0, 0, 0, 5, b'{', b'}'][..]] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_entail"))
            .args(["worker"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let mut input = child.stdin.take().ok_or("worker stdin missing")?;
        input.write_all(bytes)?;
        drop(input);
        let status = child.wait()?;
        assert!(!status.success());
    }
    Ok(())
}

#[test]
fn invalid_frame_length_terminates_nonzero() -> TestResult<()> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_entail"))
        .args(["worker", "--max-frame-bytes", "64"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let mut input = child.stdin.take().ok_or("worker stdin missing")?;
    input.write_all(&65_u32.to_be_bytes())?;
    drop(input);
    let status = child.wait()?;
    assert!(!status.success());
    Ok(())
}
