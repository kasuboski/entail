//! Framed JSON transport for the persistent author worker.

use std::io::{self, Read, Write};

use serde_json::Value;

pub const DEFAULT_MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

/// Reads one length-prefixed JSON frame. Clean EOF is `Ok(None)`; a partial
/// header or payload is an I/O error rather than a silently truncated request.
///
/// # Errors
///
/// Returns an error for truncated frames, oversized frames, invalid JSON, or IO failures.
pub fn read_frame(reader: &mut impl Read, max_frame_bytes: usize) -> io::Result<Option<Value>> {
    let Some(payload) = read_payload(reader, max_frame_bytes)? else {
        return Ok(None);
    };
    let value = serde_json::from_slice(&payload)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(Some(value))
}

fn read_payload(reader: &mut impl Read, max_frame_bytes: usize) -> io::Result<Option<Vec<u8>>> {
    let mut header = [0_u8; 4];
    let mut filled = 0;
    while filled < header.len() {
        let remaining = header.get_mut(filled..).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid frame header offset")
        })?;
        match reader.read(remaining) {
            Ok(0) if filled == 0 => return Ok(None),
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "partial frame header",
                ));
            }
            Ok(count) => filled = filled.saturating_add(count),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }

    let length = usize::try_from(u32::from_be_bytes(header)).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "frame length cannot be represented",
        )
    })?;
    if length > max_frame_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame exceeds configured maximum",
        ));
    }
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload)?;
    Ok(Some(payload))
}

/// Writes one JSON value as a complete big-endian length-prefixed frame.
///
/// # Errors
///
/// Returns an error when JSON serialization fails, the payload exceeds a limit,
/// or the writer reports an IO failure.
pub fn write_frame(
    writer: &mut impl Write,
    value: &Value,
    max_frame_bytes: usize,
) -> io::Result<()> {
    let payload = serde_json::to_vec(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if payload.len() > max_frame_bytes || u32::try_from(payload.len()).is_err() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "frame exceeds configured maximum",
        ));
    }
    let length = u32::try_from(payload.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "frame length exceeds protocol limit",
        )
    })?;
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(&payload)?;
    writer.flush()
}

use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io::{BufReader, BufWriter};

use author_analysis::{AnalysisBudget, AnalysisOptions, Backend, Check, analyze};
use author_model::{CompiledModel, ModelLimits, OutcomeValue, compile_json, evaluate};
use author_z3::Z3Backend;
use serde_json::{Map, json};

const PROTOCOL_VERSION: &str = "author-worker/0.1";
const LANGUAGE_VERSION: &str = "author-model/0.1";

struct WorkerState {
    nonce: String,
    next_handle: u64,
    handles: HashMap<String, CompiledModel>,
}

/// Runtime resource limits for a worker process.
#[derive(Clone, Debug)]
pub struct WorkerConfig {
    pub max_frame_bytes: usize,
    pub max_handles: usize,
    pub max_budget_ms: u64,
    pub max_obligations: usize,
    pub model_limits: ModelLimits,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            max_frame_bytes: DEFAULT_MAX_FRAME_BYTES,
            max_handles: 64,
            max_budget_ms: 60_000,
            max_obligations: 10_000,
            model_limits: ModelLimits::default(),
        }
    }
}

/// Runs a serial worker until stdin reaches a clean frame boundary EOF.
///
/// # Errors
///
/// Returns an error for invalid/truncated frames, unavailable session entropy,
/// or stdin/stdout failures.
pub fn run_worker(config: &WorkerConfig) -> io::Result<()> {
    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce)
        .map_err(|error| io::Error::other(format!("session entropy unavailable: {error}")))?;
    let mut nonce_hex = String::with_capacity(32);
    for byte in nonce {
        write!(&mut nonce_hex, "{byte:02x}").map_err(io::Error::other)?;
    }
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = BufReader::new(stdin.lock());
    let mut output = BufWriter::new(stdout.lock());
    let mut state = WorkerState {
        nonce: nonce_hex,
        next_handle: 0,
        handles: HashMap::new(),
    };

    loop {
        let payload = match read_payload(&mut input, config.max_frame_bytes) {
            Ok(Some(payload)) => payload,
            Ok(None) => {
                drop(input);
                drop(output);
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        let request = match serde_json::from_slice::<Value>(&payload) {
            Ok(request) => request,
            Err(error) => {
                let response = failure(
                    recover_request_id(&payload),
                    "malformed_json",
                    error.to_string(),
                );
                write_frame(&mut output, &response, config.max_frame_bytes)?;
                continue;
            }
        };
        let (response, compiled) = match request_envelope(&request) {
            Ok((request_id, operation, params)) => dispatch(
                request_id,
                operation.as_str(),
                &params,
                config,
                &mut state,
                &mut Z3Backend::new(),
            ),
            Err((request_id, code, message)) => (failure(request_id, code, message), None),
        };
        if let Some((handle, model)) = compiled {
            state.handles.insert(handle, model);
        }
        write_frame(&mut output, &response, config.max_frame_bytes)?;
    }
}

fn request_envelope(
    request: &Value,
) -> Result<(Value, String, Value), (Value, &'static str, String)> {
    let Some(object) = request.as_object() else {
        return Err((
            Value::Null,
            "invalid_envelope",
            "request must be an object".to_owned(),
        ));
    };
    if object.len() != 4
        || !["protocol_version", "request_id", "op", "params"]
            .iter()
            .all(|key| object.contains_key(*key))
    {
        return Err((
            request_id(request),
            "invalid_envelope",
            "request must contain exactly protocol_version, request_id, op, and params".to_owned(),
        ));
    }
    if object.get("protocol_version").and_then(Value::as_str) != Some(PROTOCOL_VERSION) {
        return Err((
            request_id(request),
            "version_mismatch",
            "unsupported protocol version".to_owned(),
        ));
    }
    let Some(id) = object
        .get("request_id")
        .filter(|value| value.as_str().is_some_and(|value| !value.is_empty()))
    else {
        return Err((
            Value::Null,
            "invalid_envelope",
            "request_id must be a nonempty string".to_owned(),
        ));
    };
    let Some(operation) = object.get("op").and_then(Value::as_str) else {
        return Err((
            id.clone(),
            "invalid_envelope",
            "op must be a string".to_owned(),
        ));
    };
    let Some(params) = object.get("params").filter(|value| value.is_object()) else {
        return Err((
            id.clone(),
            "invalid_envelope",
            "params must be an object".to_owned(),
        ));
    };
    Ok((id.clone(), operation.to_owned(), params.clone()))
}

fn recover_request_id(payload: &[u8]) -> Value {
    let text = String::from_utf8_lossy(payload);
    let Some(key) = text.find("\"request_id\"") else {
        return Value::Null;
    };
    let Some(rest) = key.checked_add(12).and_then(|offset| text.get(offset..)) else {
        return Value::Null;
    };
    let Some(colon) = rest.find(':') else {
        return Value::Null;
    };
    let Some(value) = colon.checked_add(1).and_then(|offset| rest.get(offset..)) else {
        return Value::Null;
    };
    let trimmed = value.trim_start();
    if !trimmed.starts_with('"') {
        return Value::Null;
    }
    let mut escaped = false;
    for (offset, character) in trimmed.char_indices().skip(1) {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            let Some(end) = offset.checked_add(1) else {
                return Value::Null;
            };
            let Some(encoded) = trimmed.get(..end) else {
                return Value::Null;
            };
            return serde_json::from_str::<String>(encoded).map_or(Value::Null, Value::String);
        }
    }
    Value::Null
}

fn request_id(request: &Value) -> Value {
    request
        .get("request_id")
        .filter(|value| value.as_str().is_some())
        .cloned()
        .unwrap_or(Value::Null)
}

fn success(request_id: Value, result: Value) -> Value {
    let mut envelope = Map::new();
    envelope.insert("protocol_version".to_owned(), json!(PROTOCOL_VERSION));
    envelope.insert("request_id".to_owned(), request_id);
    envelope.insert("result".to_owned(), result);
    Value::Object(envelope)
}

fn failure(request_id: Value, code: &str, message: String) -> Value {
    let mut error = Map::new();
    error.insert("code".to_owned(), json!(code));
    error.insert("message".to_owned(), Value::String(message));
    let mut envelope = Map::new();
    envelope.insert("protocol_version".to_owned(), json!(PROTOCOL_VERSION));
    envelope.insert("request_id".to_owned(), request_id);
    envelope.insert("error".to_owned(), Value::Object(error));
    Value::Object(envelope)
}

fn dispatch(
    request_id: Value,
    operation: &str,
    params: &Value,
    config: &WorkerConfig,
    state: &mut WorkerState,
    backend: &mut impl Backend,
) -> (Value, Option<(String, CompiledModel)>) {
    if operation == "hello" {
        if params.as_object().is_some_and(Map::is_empty) {
            return (success(request_id, hello_result(config)), None);
        }
        return (
            failure(
                request_id,
                "invalid_params",
                "hello params must be empty".to_owned(),
            ),
            None,
        );
    }
    match operation {
        "compile" => compile_request(request_id, params, config, state),
        "analyze" => analyze_request(request_id, params, config, &state.handles, backend),
        "evaluate" => evaluate_request(request_id, params, &state.handles),
        "release" => release_request(request_id, params, &mut state.handles),
        _ => (
            failure(
                request_id,
                "unknown_operation",
                format!("unknown operation {operation}"),
            ),
            None,
        ),
    }
}

fn hello_result(config: &WorkerConfig) -> Value {
    json!({
        "protocol_version": PROTOCOL_VERSION,
        "language_version": LANGUAGE_VERSION,
        "engine_version": author_analysis::ENGINE_VERSION,
        "solver_version": Z3Backend::new().solver_version(),
        "operations": ["hello", "compile", "analyze", "evaluate", "release"],
        "checks": ["domain_feasibility", "rule_reachability", "coverage", "outcome_conflict", "constraint_compliance"],
        "capabilities": {"compile": true, "analyze": true, "evaluate": true, "release": true, "hard_cancellation": true},
        "limits": {
            "max_frame_bytes": config.max_frame_bytes,
            "max_handles": config.max_handles,
            "max_budget_ms": config.max_budget_ms,
            "max_obligations": config.max_obligations,
            "model": {
                "inputs": config.model_limits.inputs,
                "rules": config.model_limits.rules,
                "constraints": config.model_limits.constraints,
                "expression_nodes": config.model_limits.expression_nodes,
                "expression_depth": config.model_limits.expression_depth
            }
        }
    })
}

fn compile_request(
    request_id: Value,
    params: &Value,
    config: &WorkerConfig,
    state: &mut WorkerState,
) -> (Value, Option<(String, CompiledModel)>) {
    if params
        .as_object()
        .is_none_or(|object| object.len() != 1 || !object.contains_key("model"))
    {
        return (
            failure(
                request_id,
                "invalid_params",
                "compile requires only a model parameter".to_owned(),
            ),
            None,
        );
    }
    let Some(model_json) = params.get("model") else {
        return (
            failure(
                request_id,
                "invalid_params",
                "compile requires model".to_owned(),
            ),
            None,
        );
    };
    if state.handles.len() >= config.max_handles {
        return (
            failure(
                request_id,
                "capacity_exceeded",
                "worker handle capacity reached".to_owned(),
            ),
            None,
        );
    }
    let started = std::time::Instant::now();
    let model = match compile_json(model_json, &config.model_limits) {
        Ok(model) => model,
        Err(diagnostics) => {
            return (
                success(
                    request_id,
                    json!({"valid": false, "handle": null, "diagnostics": diagnostics}),
                ),
                None,
            );
        }
    };
    let compile_elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let Some(handle_id) = state.next_handle.checked_add(1) else {
        return (
            failure(
                request_id,
                "capacity_exceeded",
                "worker handle ID space exhausted".to_owned(),
            ),
            None,
        );
    };
    state.next_handle = handle_id;
    let handle = format!("{}-{handle_id:016x}", state.nonce);
    let result = json!({"valid": true, "handle": handle, "model_id": model.raw.model_id, "content_hash": model.content_hash, "semantic_hash": model.semantic_hash, "diagnostics": [], "compile_elapsed_ms": compile_elapsed_ms, "readiness": if model.raw.outcomes.is_empty() || (model.raw.inputs.is_empty() && model.raw.rules.is_empty()) { "incomplete_model" } else { "ready_for_analysis" }});
    (success(request_id, result), Some((handle, model)))
}

fn parse_assumptions(params: &Value) -> Result<Vec<String>, String> {
    let Some(values) = params.get("assumption_ids") else {
        return Ok(Vec::new());
    };
    let Some(values) = values.as_array() else {
        return Err("assumption_ids must be an array".to_owned());
    };
    let mut ids = Vec::with_capacity(values.len());
    for value in values {
        let Some(id) = value.as_str() else {
            return Err("assumption_ids must contain strings".to_owned());
        };
        ids.push(id.to_owned());
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

fn has_only_keys(params: &Value, allowed: &[&str]) -> bool {
    params
        .as_object()
        .is_some_and(|object| object.keys().all(|key| allowed.contains(&key.as_str())))
}

fn analyze_request(
    request_id: Value,
    params: &Value,
    config: &WorkerConfig,
    handles: &HashMap<String, CompiledModel>,
    backend: &mut impl Backend,
) -> (Value, Option<(String, CompiledModel)>) {
    if !has_only_keys(params, &["handle", "checks", "assumption_ids", "budget"]) {
        return (
            failure(
                request_id,
                "invalid_params",
                "analyze contains an unknown parameter".to_owned(),
            ),
            None,
        );
    }
    let Some(handle) = params.get("handle").and_then(Value::as_str) else {
        return (
            failure(
                request_id,
                "invalid_params",
                "analyze requires a string handle".to_owned(),
            ),
            None,
        );
    };
    let Some(model) = handles.get(handle) else {
        return (
            failure(
                request_id,
                "unknown_handle",
                "compiled model handle is unknown or expired".to_owned(),
            ),
            None,
        );
    };
    let assumptions = match parse_assumptions(params) {
        Ok(assumptions) => assumptions,
        Err(message) => return (failure(request_id, "invalid_params", message), None),
    };
    let checks = match parse_checks(params) {
        Ok(checks) => checks,
        Err(message) => return (failure(request_id, "invalid_params", message), None),
    };
    let budget = match parse_budget(params, config.max_budget_ms, config.max_obligations) {
        Ok(budget) => budget,
        Err(message) => return (failure(request_id, "invalid_params", message), None),
    };
    let options = AnalysisOptions {
        checks,
        assumption_ids: assumptions,
        budget,
    };
    match analyze(model, &options, backend) {
        Ok(report) => (success(request_id, json!(report)), None),
        Err(diagnostic) => {
            let code = if matches!(
                diagnostic.code.as_str(),
                "internal_error" | "serialization_error"
            ) {
                "internal_error"
            } else {
                "invalid_params"
            };
            (failure(request_id, code, diagnostic.message), None)
        }
    }
}

fn parse_checks(params: &Value) -> Result<Vec<Check>, String> {
    let Some(values) = params.get("checks") else {
        return Ok(Vec::new());
    };
    let Some(values) = values.as_array() else {
        return Err("checks must be an array".to_owned());
    };
    let mut checks = Vec::new();
    for value in values {
        let Some(name) = value.as_str() else {
            return Err("checks must contain strings".to_owned());
        };
        let check = match name {
            "domain_feasibility" => Check::DomainFeasibility,
            "rule_reachability" => Check::RuleReachability,
            "coverage" => Check::Coverage,
            "outcome_conflict" => Check::OutcomeConflict,
            "constraint_compliance" => Check::ConstraintCompliance,
            _ => return Err(format!("unsupported check {name}")),
        };
        if !checks.contains(&check) {
            checks.push(check);
        }
    }
    checks.sort_by_key(check_order);
    Ok(checks)
}

const fn check_order(check: &Check) -> u8 {
    match check {
        Check::DomainFeasibility => 0,
        Check::RuleReachability => 1,
        Check::Coverage => 2,
        Check::OutcomeConflict => 3,
        Check::ConstraintCompliance => 4,
    }
}

fn parse_budget(
    params: &Value,
    maximum: u64,
    max_obligations: usize,
) -> Result<AnalysisBudget, String> {
    let empty_budget = Value::Object(Map::new());
    let value = params.get("budget").unwrap_or(&empty_budget);
    let Some(object) = value.as_object() else {
        return Err("budget must be an object".to_owned());
    };
    if object.keys().any(|key| {
        ![
            "total_timeout_ms",
            "per_obligation_timeout_ms",
            "max_obligations",
            "solver_resource_limit",
        ]
        .contains(&key.as_str())
    }) {
        return Err("budget contains an unknown field".to_owned());
    }
    for key in [
        "total_timeout_ms",
        "per_obligation_timeout_ms",
        "max_obligations",
        "solver_resource_limit",
    ] {
        if value.get(key).is_some_and(Value::is_null) {
            return Err(format!("{key} must be a nonzero integer"));
        }
    }
    let mut budget: AnalysisBudget =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    if budget.total_timeout_ms == Some(0)
        || budget.per_obligation_timeout_ms == Some(0)
        || budget.max_obligations == Some(0)
        || budget.solver_resource_limit == Some(0)
    {
        return Err("budget values must be nonzero".to_owned());
    }
    budget.total_timeout_ms = Some(budget.total_timeout_ms.unwrap_or(5_000).min(maximum));
    budget.per_obligation_timeout_ms = Some(
        budget
            .per_obligation_timeout_ms
            .unwrap_or(1_000)
            .min(maximum),
    );
    budget.max_obligations = Some(
        budget
            .max_obligations
            .unwrap_or(max_obligations)
            .min(max_obligations),
    );
    Ok(budget)
}

fn evaluate_request(
    request_id: Value,
    params: &Value,
    handles: &HashMap<String, CompiledModel>,
) -> (Value, Option<(String, CompiledModel)>) {
    if !has_only_keys(
        params,
        &["handle", "inputs", "assumption_ids", "expected_outcome"],
    ) {
        return (
            failure(
                request_id,
                "invalid_params",
                "evaluate contains an unknown parameter".to_owned(),
            ),
            None,
        );
    }
    let Some(handle) = params.get("handle").and_then(Value::as_str) else {
        return (
            failure(
                request_id,
                "invalid_params",
                "evaluate requires a string handle".to_owned(),
            ),
            None,
        );
    };
    let Some(model) = handles.get(handle) else {
        return (
            failure(
                request_id,
                "unknown_handle",
                "compiled model handle is unknown or expired".to_owned(),
            ),
            None,
        );
    };
    let Some(inputs) = params.get("inputs") else {
        return (
            failure(
                request_id,
                "invalid_params",
                "evaluate requires inputs".to_owned(),
            ),
            None,
        );
    };
    let inputs = match serde_json::from_value(inputs.clone()) {
        Ok(inputs) => inputs,
        Err(error) => {
            return (
                failure(request_id, "invalid_params", error.to_string()),
                None,
            );
        }
    };
    let assumptions = match parse_assumptions(params) {
        Ok(assumptions) => assumptions,
        Err(message) => return (failure(request_id, "invalid_params", message), None),
    };
    let expected: Option<OutcomeValue> = match params.get("expected_outcome") {
        Some(value) => match serde_json::from_value(value.clone()) {
            Ok(value) => Some(value),
            Err(error) => {
                return (
                    failure(request_id, "invalid_params", error.to_string()),
                    None,
                );
            }
        },
        None => None,
    };
    match evaluate(model, &inputs, &assumptions, expected.as_ref()) {
        Ok(result) => (success(request_id, json!(result)), None),
        Err(diagnostic) => {
            let code = if diagnostic.code == "internal_error" {
                "internal_error"
            } else {
                "invalid_params"
            };
            (failure(request_id, code, diagnostic.message), None)
        }
    }
}

fn release_request(
    request_id: Value,
    params: &Value,
    handles: &mut HashMap<String, CompiledModel>,
) -> (Value, Option<(String, CompiledModel)>) {
    if params
        .as_object()
        .is_none_or(|object| object.len() != 1 || !object.contains_key("handle"))
    {
        return (
            failure(
                request_id,
                "invalid_params",
                "release requires only a handle parameter".to_owned(),
            ),
            None,
        );
    }
    let Some(handle) = params.get("handle").and_then(Value::as_str) else {
        return (
            failure(
                request_id,
                "invalid_params",
                "release handle must be a string".to_owned(),
            ),
            None,
        );
    };
    (
        success(
            request_id,
            json!({"released": handles.remove(handle).is_some()}),
        ),
        None,
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic_in_result_fn)] // IO propagation and assertion macros coexist in transport tests.

    use std::io::{self, Read};

    use serde_json::json;

    use super::{read_frame, write_frame};

    struct Fragmented<'a> {
        bytes: &'a [u8],
        chunk_size: usize,
    }

    #[derive(Default)]
    struct PartialWriter {
        bytes: Vec<u8>,
    }

    impl std::io::Write for PartialWriter {
        fn write(&mut self, input: &[u8]) -> io::Result<usize> {
            let Some(byte) = input.first() else {
                return Ok(0);
            };
            self.bytes.push(*byte);
            Ok(1)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Read for Fragmented<'_> {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if self.bytes.is_empty() {
                return Ok(0);
            }
            let count = output.len().min(self.chunk_size).min(self.bytes.len());
            let (written, remaining) = self.bytes.split_at(count);
            let Some(destination) = output.get_mut(..count) else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "invalid read buffer",
                ));
            };
            destination.copy_from_slice(written);
            self.bytes = remaining;
            Ok(count)
        }
    }

    #[test]
    fn fragmented_header_and_payload_are_read_completely() -> io::Result<()> {
        let value = json!({"request_id":"fragmented", "op":"hello"});
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &value, 1024)?;
        let mut input = Fragmented {
            bytes: &bytes,
            chunk_size: 1,
        };
        assert_eq!(read_frame(&mut input, 1024)?, Some(value));
        assert_eq!(read_frame(&mut input, 1024)?, None);
        Ok(())
    }

    #[test]
    fn partial_writes_produce_a_complete_frame() -> io::Result<()> {
        let value = json!({"request_id":"partial-write"});
        let mut output = PartialWriter::default();
        super::write_frame(&mut output, &value, 1024)?;
        let mut input = Fragmented {
            bytes: &output.bytes,
            chunk_size: 2,
        };
        assert_eq!(read_frame(&mut input, 1024)?, Some(value));
        Ok(())
    }

    #[test]
    fn oversized_length_is_rejected_before_allocating_payload() {
        let mut input = Fragmented {
            bytes: &[0, 0, 0, 9],
            chunk_size: 4,
        };
        assert_eq!(
            read_frame(&mut input, 8).err().map(|error| error.kind()),
            Some(io::ErrorKind::InvalidData)
        );
    }

    #[test]
    fn partial_header_is_not_clean_eof() {
        let mut input = Fragmented {
            bytes: &[0, 0],
            chunk_size: 1,
        };
        assert_eq!(
            read_frame(&mut input, 8).err().map(|error| error.kind()),
            Some(io::ErrorKind::UnexpectedEof)
        );
    }
}
