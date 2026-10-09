use std::process::ExitCode;

use author_analysis::{AnalysisBudget, AnalysisOptions, Backend, Check, analyze};
use author_model::{InputValues, ModelLimits, OutcomeValue, compile_json, evaluate};
use author_worker::{WorkerConfig, run_worker};
use author_z3::Z3Backend;
use serde_json::{Value, json};

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments
        .first()
        .is_some_and(|argument| argument == "worker")
    {
        let worker_arguments = arguments.get(1..).unwrap_or_default();
        return match worker_config(worker_arguments).and_then(|config| {
            run_worker(&config).map_err(|error| (error.to_string(), ExitCode::from(3)))
        }) {
            Ok(()) => ExitCode::SUCCESS,
            Err((message, code)) => {
                eprintln!("{message}");
                code
            }
        };
    }
    match execute(&arguments) {
        Ok((output, code)) | Err((output, code)) => {
            println!("{output}");
            code
        }
    }
}

type CliResult = Result<(String, ExitCode), (String, ExitCode)>;

fn execute(arguments: &[String]) -> CliResult {
    if arguments.is_empty()
        || arguments
            .first()
            .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        return Ok((json!({"usage": [
            "entail validate MODEL",
            "entail analyze MODEL --checks comma-list --assumptions comma-list --budget-ms N",
            "entail evaluate MODEL SCENARIO",
            "entail worker [--max-frame-bytes N] [--max-handles N] [--max-budget-ms N] [--max-obligations N] [--max-inputs N] [--max-rules N]"
        ], "worker_environment": ["ENTAIL_MAX_FRAME_BYTES", "ENTAIL_MAX_HANDLES", "ENTAIL_MAX_BUDGET_MS", "ENTAIL_MAX_OBLIGATIONS", "ENTAIL_MAX_INPUTS", "ENTAIL_MAX_RULES", "ENTAIL_MAX_CONSTRAINTS", "ENTAIL_MAX_EXPRESSION_NODES", "ENTAIL_MAX_EXPRESSION_DEPTH"]}).to_string(), ExitCode::SUCCESS));
    }
    match arguments.first().map(String::as_str) {
        Some("validate") => validate_command(arguments),
        Some("analyze") => analyze_command(arguments),
        Some("evaluate") => evaluate_command(arguments),
        Some(command) => Err((json!({"error":{"code":"invalid_input","message":format!("unknown command {command}")}}).to_string(), ExitCode::from(2))),
        None => Err((json!({"error":{"code":"invalid_input","message":"missing command"}}).to_string(), ExitCode::from(2))),
    }
}

fn load_json(path: &str) -> Result<Value, (String, ExitCode)> {
    let bytes = std::fs::read(path).map_err(|error| (json!({"error":{"code":"invalid_input","message":format!("cannot read {path}: {error}")}}).to_string(), ExitCode::from(2)))?;
    serde_json::from_slice(&bytes).map_err(|error| (json!({"error":{"code":"invalid_input","message":format!("invalid JSON in {path}: {error}")}}).to_string(), ExitCode::from(2)))
}

fn compile_file(path: &str) -> Result<author_model::CompiledModel, (String, ExitCode)> {
    let value = load_json(path)?;
    compile_json(&value, &ModelLimits::default()).map_err(|diagnostics| {
        (
            json!({"valid":false,"diagnostics":diagnostics}).to_string(),
            ExitCode::from(2),
        )
    })
}

fn validate_command(arguments: &[String]) -> CliResult {
    if arguments.len() != 2 {
        return Err((
            json!({"error":{"code":"invalid_input","message":"usage: entail validate MODEL"}})
                .to_string(),
            ExitCode::from(2),
        ));
    }
    let path = arguments.get(1).ok_or_else(|| {
        (
            json!({"error":{"code":"invalid_input","message":"missing model path"}}).to_string(),
            ExitCode::from(2),
        )
    })?;
    let model = compile_file(path)?;
    let readiness = if model.raw().outcomes.is_empty()
        || (model.raw().inputs.is_empty() && model.raw().rules.is_empty())
    {
        "incomplete_model"
    } else {
        "ready_for_analysis"
    };
    Ok((json!({"valid":true,"model_id":model.raw().model_id,"content_hash":model.content_hash(),"semantic_hash":model.semantic_hash(),"language_version":"author-model/0.1","protocol_version":"author-worker/0.1","engine_version":author_analysis::ENGINE_VERSION,"solver_version":Z3Backend::new().solver_version(),"readiness":readiness,"diagnostics":[]}).to_string(), ExitCode::SUCCESS))
}

fn parse_list(value: &str) -> Vec<String> {
    let mut values: Vec<_> = value
        .split(',')
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect();
    values.sort();
    values.dedup();
    values
}

fn parse_cli_checks(value: &str) -> Result<Vec<Check>, String> {
    let mut checks = Vec::new();
    for name in parse_list(value) {
        let check = match name.as_str() {
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
    checks.sort_by_key(|check| match check {
        Check::DomainFeasibility => 0,
        Check::RuleReachability => 1,
        Check::Coverage => 2,
        Check::OutcomeConflict => 3,
        Check::ConstraintCompliance => 4,
    });
    Ok(checks)
}

fn analyze_command(arguments: &[String]) -> CliResult {
    let Some(path) = arguments.get(1) else {
        return Err((json!({"error":{"code":"invalid_input","message":"usage: entail analyze MODEL [--checks LIST] [--assumptions LIST] [--budget-ms N]"}}).to_string(), ExitCode::from(2)));
    };
    let mut checks = Vec::new();
    let mut assumptions = Vec::new();
    let mut budget_ms = None;
    let mut index = 2;
    while index < arguments.len() {
        let flag = arguments.get(index).map_or("", String::as_str);
        let value_index = index.checked_add(1).ok_or_else(|| {
            (
                json!({"error":{"code":"invalid_input","message":"argument index overflow"}})
                    .to_string(),
                ExitCode::from(2),
            )
        })?;
        let value = arguments.get(value_index).ok_or_else(|| (json!({"error":{"code":"invalid_input","message":format!("missing value for {flag}")}}).to_string(), ExitCode::from(2)))?;
        match flag {
            "--checks" => checks = parse_cli_checks(value).map_err(|message| (json!({"error":{"code":"invalid_input","message":message}}).to_string(), ExitCode::from(2)))?,
            "--assumptions" => assumptions = parse_list(value),
            "--budget-ms" => budget_ms = Some(value.parse::<u64>().map_err(|error| (json!({"error":{"code":"invalid_input","message":error.to_string()}}).to_string(), ExitCode::from(2)))?),
            _ => return Err((json!({"error":{"code":"invalid_input","message":format!("unknown argument {flag}")}}).to_string(), ExitCode::from(2))),
        }
        index = index.saturating_add(2);
    }
    if budget_ms == Some(0) {
        return Err((
            json!({"error":{"code":"invalid_input","message":"budget must be nonzero"}})
                .to_string(),
            ExitCode::from(2),
        ));
    }
    let model = compile_file(path)?;
    let options = AnalysisOptions {
        checks,
        assumption_ids: assumptions,
        budget: AnalysisBudget {
            total_timeout_ms: budget_ms,
            ..AnalysisBudget::default()
        },
    };
    let report = analyze(&model, &options, &mut Z3Backend::new()).map_err(|diagnostic| {
        let exit_code = if matches!(
            diagnostic.code.as_str(),
            "internal_error" | "serialization_error"
        ) {
            ExitCode::from(3)
        } else {
            ExitCode::from(2)
        };
        (
            json!({"error":{"code":diagnostic.code,"message":diagnostic.message}}).to_string(),
            exit_code,
        )
    })?;
    let backend_error = report
        .results
        .iter()
        .any(|result| result.status == author_analysis::ResultStatus::Error);
    let output = json!(report).to_string();
    if backend_error {
        Err((output, ExitCode::from(3)))
    } else {
        Ok((output, ExitCode::SUCCESS))
    }
}

fn evaluate_command(arguments: &[String]) -> CliResult {
    if arguments.len() != 3 {
        return Err((json!({"error":{"code":"invalid_input","message":"usage: entail evaluate MODEL SCENARIO"}}).to_string(), ExitCode::from(2)));
    }
    let model_path = arguments.get(1).ok_or_else(|| {
        (
            json!({"error":{"code":"invalid_input","message":"missing model path"}}).to_string(),
            ExitCode::from(2),
        )
    })?;
    let scenario_path = arguments.get(2).ok_or_else(|| {
        (
            json!({"error":{"code":"invalid_input","message":"missing scenario path"}}).to_string(),
            ExitCode::from(2),
        )
    })?;
    let model = compile_file(model_path)?;
    let scenario = load_json(scenario_path)?;
    let Some(scenario_object) = scenario.as_object() else {
        return Err((
            json!({"error":{"code":"invalid_input","message":"scenario must be a JSON object"}})
                .to_string(),
            ExitCode::from(2),
        ));
    };
    if scenario_object
        .keys()
        .any(|key| !["inputs", "assumption_ids", "expected_outcome"].contains(&key.as_str()))
    {
        return Err((json!({"error":{"code":"invalid_input","message":"scenario contains an unknown field"}}).to_string(), ExitCode::from(2)));
    }
    let inputs: InputValues = scenario
        .get("inputs")
        .cloned()
        .ok_or_else(|| {
            (
                json!({"error":{"code":"invalid_input","message":"scenario requires inputs"}})
                    .to_string(),
                ExitCode::from(2),
            )
        })
        .and_then(|value| {
            serde_json::from_value(value).map_err(|error| {
                (
                    json!({"error":{"code":"invalid_input","message":error.to_string()}})
                        .to_string(),
                    ExitCode::from(2),
                )
            })
        })?;
    let assumptions = match scenario.get("assumption_ids") {
        None => Vec::new(),
        Some(value) => {
            let Some(items) = value.as_array() else {
                return Err((json!({"error":{"code":"invalid_input","message":"assumption_ids must be an array"}}).to_string(), ExitCode::from(2)));
            };
            let mut ids = Vec::with_capacity(items.len());
            for item in items {
                let Some(id) = item.as_str() else {
                    return Err((json!({"error":{"code":"invalid_input","message":"assumption_ids must contain strings"}}).to_string(), ExitCode::from(2)));
                };
                ids.push(id.to_owned());
            }
            ids.sort();
            ids.dedup();
            ids
        }
    };
    let expected: Option<OutcomeValue> = scenario
        .get("expected_outcome")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| {
            (
                json!({"error":{"code":"invalid_input","message":error.to_string()}}).to_string(),
                ExitCode::from(2),
            )
        })?;
    let result =
        evaluate(&model, &inputs, &assumptions, expected.as_ref()).map_err(|diagnostic| {
            let code = if diagnostic.code == "internal_error" {
                ExitCode::from(3)
            } else {
                ExitCode::from(2)
            };
            (
                json!({"error":{"code":diagnostic.code,"message":diagnostic.message}}).to_string(),
                code,
            )
        })?;
    Ok((json!(result).to_string(), ExitCode::SUCCESS))
}

fn worker_config(arguments: &[String]) -> Result<WorkerConfig, (String, ExitCode)> {
    let mut config = WorkerConfig::default();
    config.max_frame_bytes = env_usize("ENTAIL_MAX_FRAME_BYTES", config.max_frame_bytes)?;
    config.max_handles = env_usize("ENTAIL_MAX_HANDLES", config.max_handles)?;
    config.max_budget_ms = env_u64("ENTAIL_MAX_BUDGET_MS", config.max_budget_ms)?;
    config.max_obligations = env_usize("ENTAIL_MAX_OBLIGATIONS", config.max_obligations)?;
    config.model_limits.inputs = env_usize("ENTAIL_MAX_INPUTS", config.model_limits.inputs)?;
    config.model_limits.rules = env_usize("ENTAIL_MAX_RULES", config.model_limits.rules)?;
    config.model_limits.constraints =
        env_usize("ENTAIL_MAX_CONSTRAINTS", config.model_limits.constraints)?;
    config.model_limits.expression_nodes = env_usize(
        "ENTAIL_MAX_EXPRESSION_NODES",
        config.model_limits.expression_nodes,
    )?;
    config.model_limits.expression_depth = env_usize(
        "ENTAIL_MAX_EXPRESSION_DEPTH",
        config.model_limits.expression_depth,
    )?;
    let mut index = 0;
    while index < arguments.len() {
        let flag = arguments.get(index).map_or("", String::as_str);
        let value_index = index.checked_add(1).ok_or_else(|| {
            (
                json!({"error":{"code":"invalid_input","message":"argument index overflow"}})
                    .to_string(),
                ExitCode::from(2),
            )
        })?;
        let value = arguments.get(value_index).ok_or_else(|| (json!({"error":{"code":"invalid_input","message":format!("missing value for {flag}")}}).to_string(), ExitCode::from(2)))?;
        let parsed = value.parse::<usize>().map_err(|error| {
            (
                json!({"error":{"code":"invalid_input","message":error.to_string()}}).to_string(),
                ExitCode::from(2),
            )
        })?;
        match flag {
            "--max-frame-bytes" => config.max_frame_bytes = parsed,
            "--max-handles" => config.max_handles = parsed,
            "--max-budget-ms" => config.max_budget_ms = u64::try_from(parsed).unwrap_or(u64::MAX),
            "--max-obligations" => config.max_obligations = parsed,
            "--max-inputs" => config.model_limits.inputs = parsed,
            "--max-rules" => config.model_limits.rules = parsed,
            "--max-constraints" => config.model_limits.constraints = parsed,
            "--max-expression-nodes" => config.model_limits.expression_nodes = parsed,
            "--max-expression-depth" => config.model_limits.expression_depth = parsed,
            _ => return Err((json!({"error":{"code":"invalid_input","message":format!("unknown worker option {flag}")}}).to_string(), ExitCode::from(2))),
        }
        index = index.saturating_add(2);
    }
    if config.max_frame_bytes == 0
        || config.max_handles == 0
        || config.max_budget_ms == 0
        || config.max_obligations == 0
    {
        return Err((
            json!({"error":{"code":"invalid_input","message":"worker limits must be nonzero"}})
                .to_string(),
            ExitCode::from(2),
        ));
    }
    Ok(config)
}

fn env_usize(name: &str, default: usize) -> Result<usize, (String, ExitCode)> {
    std::env::var(name).map_or(Ok(default), |value| {
        value.parse().map_err(|error| {
            (
                json!({"error":{"code":"invalid_input","message":format!("{name}: {error}")}})
                    .to_string(),
                ExitCode::from(2),
            )
        })
    })
}

fn env_u64(name: &str, default: u64) -> Result<u64, (String, ExitCode)> {
    std::env::var(name).map_or(Ok(default), |value| {
        value.parse().map_err(|error| {
            (
                json!({"error":{"code":"invalid_input","message":format!("{name}: {error}")}})
                    .to_string(),
                ExitCode::from(2),
            )
        })
    })
}
