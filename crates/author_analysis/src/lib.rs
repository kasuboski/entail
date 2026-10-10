//! Solver-independent planning and result mapping for decision-model analysis.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use author_model::{
    CompiledModel, Diagnostic, InputValues, OutcomeValue, Value, evaluate, select_assumptions,
};
use serde::{Deserialize, Serialize};

pub const ENGINE_VERSION: &str = "author-analysis/0.1";
const CACHE_ENCODING_VERSION: &str = "author-analysis-obligations/2";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    #[default]
    DomainFeasibility,
    RuleReachability,
    Coverage,
    OutcomeConflict,
    ConstraintCompliance,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisBudget {
    pub total_timeout_ms: Option<u64>,
    pub per_obligation_timeout_ms: Option<u64>,
    pub max_obligations: Option<usize>,
    pub solver_resource_limit: Option<u32>,
}

impl AnalysisBudget {
    fn normalized(&self) -> EffectiveBudget {
        EffectiveBudget {
            total_timeout_ms: self.total_timeout_ms.unwrap_or(5_000),
            per_obligation_timeout_ms: self.per_obligation_timeout_ms.unwrap_or(1_000),
            max_obligations: self.max_obligations.unwrap_or(10_000),
            solver_resource_limit: self.solver_resource_limit,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisOptions {
    #[serde(default)]
    pub checks: Vec<Check>,
    #[serde(default)]
    pub assumption_ids: Vec<String>,
    #[serde(default)]
    pub budget: AnalysisBudget,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectiveBudget {
    pub total_timeout_ms: u64,
    pub per_obligation_timeout_ms: u64,
    pub max_obligations: usize,
    pub solver_resource_limit: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolveBudget {
    pub timeout_ms: u64,
    pub solver_resource_limit: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Obligation {
    DomainFeasibility,
    RuleReachability {
        rule_id: String,
    },
    Coverage,
    OutcomeConflict {
        left_rule_id: String,
        right_rule_id: String,
    },
    ConstraintCompliance {
        rule_id: String,
        constraint_id: String,
    },
}

impl Obligation {
    #[must_use]
    pub fn id(&self) -> String {
        match self {
            Self::DomainFeasibility => "domain_feasibility".to_owned(),
            Self::RuleReachability { rule_id } => format!("rule_reachability:{rule_id}"),
            Self::Coverage => "coverage".to_owned(),
            Self::OutcomeConflict {
                left_rule_id,
                right_rule_id,
            } => {
                format!("outcome_conflict:{left_rule_id}:{right_rule_id}")
            }
            Self::ConstraintCompliance {
                rule_id,
                constraint_id,
            } => {
                format!("constraint_compliance:{rule_id}:{constraint_id}")
            }
        }
    }

    fn declaration_ids(&self) -> Vec<String> {
        match self {
            Self::DomainFeasibility | Self::Coverage => Vec::new(),
            Self::RuleReachability { rule_id } => vec![rule_id.clone()],
            Self::OutcomeConflict {
                left_rule_id,
                right_rule_id,
            } => {
                vec![left_rule_id.clone(), right_rule_id.clone()]
            }
            Self::ConstraintCompliance {
                rule_id,
                constraint_id,
            } => {
                vec![rule_id.clone(), constraint_id.clone()]
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "snake_case", deny_unknown_fields)]
pub enum BackendAnswer {
    Sat { inputs: InputValues },
    Unsat,
    Unknown { reason: String },
    Timeout,
    Cancelled,
    Error { code: String, message: String },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolverStatistics {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, String>,
}

pub trait Backend {
    fn solve(
        &mut self,
        model: &CompiledModel,
        obligation: &Obligation,
        assumption_ids: &[String],
        budget: &SolveBudget,
    ) -> BackendAnswer;

    fn solver_version(&self) -> String;

    fn solver_configuration(&self) -> String {
        String::new()
    }

    /// Stable identity for this backend's answer semantics and configuration.
    ///
    /// Returning `None` disables the process-global completed-result cache. A backend
    /// opting in must ensure this namespace changes whenever any solver behavior that
    /// can affect answers changes; `solver_version` and `solver_configuration` are
    /// included separately in the cache key as well.
    fn cache_namespace(&self) -> Option<String> {
        None
    }

    fn statistics(&self) -> Option<SolverStatistics> {
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultStatus {
    Satisfied,
    Finding,
    Unknown,
    Timeout,
    Cancelled,
    Skipped,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCategory {
    Feasible,
    EmptyDomain,
    Reachable,
    Unreachable,
    Uncovered,
    Covered,
    BehaviorUnspecified,
    Conflict,
    Compatible,
    Violation,
    Compliant,
    NoRequirementsForOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObligationResult {
    pub obligation_id: String,
    pub obligation: Obligation,
    pub declaration_ids: Vec<String>,
    pub status: ResultStatus,
    pub finding_category: Option<FindingCategory>,
    pub witness: Option<InputValues>,
    pub source_ids: Vec<String>,
    pub conditional: bool,
    pub labels: Vec<String>,
    pub rule_reachability_status: Option<ResultStatus>,
    pub elapsed_ms: Option<u64>,
    pub statistics: Option<SolverStatistics>,
    pub skip_reason: Option<String>,
    pub diagnostic_code: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisReport {
    pub language_version: String,
    pub engine_version: String,
    pub solver_version: String,
    pub model_id: String,
    pub content_hash: String,
    pub semantic_hash: String,
    pub selected_assumption_ids: Vec<String>,
    pub domain_predicate_ids: Vec<String>,
    pub checks: Vec<Check>,
    pub budget: EffectiveBudget,
    pub participating_proposed_declaration_ids: Vec<String>,
    pub open_items: Vec<author_model::OpenItem>,
    pub results: Vec<ObligationResult>,
    pub complete: bool,
    #[serde(default)]
    pub cache_hits: usize,
    #[serde(default)]
    pub solver_query_count: usize,
    #[serde(default)]
    pub planned_obligation_count: usize,
    #[serde(default)]
    pub executed_obligation_count: usize,
    #[serde(default)]
    pub elapsed_ms: u64,
    #[serde(default)]
    pub remaining_work: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    semantic_hash: String,
    obligation: Obligation,
    assumptions: Vec<String>,
    engine_encoding: String,
    backend_type: &'static str,
    cache_namespace: String,
    solver_version: String,
    solver_configuration: String,
    budget: SolveBudget,
}

static COMPLETED_CACHE: OnceLock<Mutex<HashMap<CacheKey, BackendAnswer>>> = OnceLock::new();
const MAX_CACHE_ENTRIES: usize = 4_096;

fn cached_solve<B: Backend>(
    backend: &mut B,
    model: &CompiledModel,
    obligation: &Obligation,
    assumptions: &[String],
    budget: &SolveBudget,
) -> (BackendAnswer, bool) {
    let Some(cache_namespace) = backend.cache_namespace() else {
        return (backend.solve(model, obligation, assumptions, budget), false);
    };
    let key = CacheKey {
        semantic_hash: model.semantic_hash().to_owned(),
        obligation: obligation.clone(),
        assumptions: assumptions.to_vec(),
        engine_encoding: CACHE_ENCODING_VERSION.to_owned(),
        backend_type: std::any::type_name::<B>(),
        cache_namespace,
        solver_version: backend.solver_version(),
        solver_configuration: backend.solver_configuration(),
        budget: budget.clone(),
    };
    let cache = COMPLETED_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(answer) = guard.get(&key) {
            let validated = match answer {
                BackendAnswer::Sat { inputs } => {
                    replay_witness(model, obligation, inputs, assumptions).unwrap_or(false)
                }
                BackendAnswer::Unsat => true,
                _ => false,
            };
            if validated {
                return (answer.clone(), true);
            }
        }
    }
    let answer = backend.solve(model, obligation, assumptions, budget);
    let validated_sat = match &answer {
        BackendAnswer::Sat { inputs } => {
            replay_witness(model, obligation, inputs, assumptions).unwrap_or(false)
        }
        _ => false,
    };
    if validated_sat || matches!(answer, BackendAnswer::Unsat) {
        if let Ok(mut guard) = cache.lock() {
            if guard.len() >= MAX_CACHE_ENTRIES {
                guard.clear();
            }
            guard.insert(key, answer.clone());
        }
    }
    (answer, false)
}

#[derive(Clone)]
struct Planned {
    obligation: Obligation,
    synthetic: Option<FindingCategory>,
}

fn check_selected(options: &AnalysisOptions) -> Vec<Check> {
    let mut checks = Vec::new();
    // Feasibility is unconditional and occupies the first slot in every report.
    checks.push(Check::DomainFeasibility);
    let mut requested = options.checks.clone();
    requested.sort_by_key(|check| match check {
        Check::DomainFeasibility => 0,
        Check::RuleReachability => 1,
        Check::Coverage => 2,
        Check::OutcomeConflict => 3,
        Check::ConstraintCompliance => 4,
    });
    for check in &requested {
        if *check == Check::ConstraintCompliance && !checks.contains(&Check::RuleReachability) {
            checks.push(Check::RuleReachability);
        }
        if *check != Check::DomainFeasibility && !checks.contains(check) {
            checks.push(check.clone());
        }
    }
    checks
}

#[allow(clippy::too_many_lines)] // The cursor state machine keeps lazy obligation ordering explicit.
fn next_plan(
    rules: &[&author_model::Rule],
    constraints: &[&author_model::Constraint],
    output_signatures: &[String],
    checks: &[Check],
    cursors: &mut PlanCursors,
) -> Option<Planned> {
    loop {
        let check = checks.get(cursors.check)?;
        match check {
            Check::DomainFeasibility => {
                cursors.check = cursors.check.saturating_add(1);
                return Some(Planned {
                    obligation: Obligation::DomainFeasibility,
                    synthetic: None,
                });
            }
            Check::RuleReachability => {
                if let Some(rule) = rules.get(cursors.rule).copied() {
                    cursors.rule = cursors.rule.saturating_add(1);
                    return Some(Planned {
                        obligation: Obligation::RuleReachability {
                            rule_id: rule.id.clone(),
                        },
                        synthetic: None,
                    });
                }
            }
            Check::Coverage => {
                cursors.check = cursors.check.saturating_add(1);
                return Some(Planned {
                    obligation: Obligation::Coverage,
                    synthetic: rules
                        .is_empty()
                        .then_some(FindingCategory::BehaviorUnspecified),
                });
            }
            Check::OutcomeConflict => {
                let n = rules.len();
                if cursors.left < n {
                    if cursors.right >= n {
                        cursors.left = cursors.left.saturating_add(1);
                        cursors.right = cursors.left.saturating_add(1);
                        continue;
                    }
                    let left = rules.get(cursors.left).copied()?;
                    let right = rules.get(cursors.right).copied()?;
                    let outputs_are_identical =
                        output_signatures.get(cursors.left) == output_signatures.get(cursors.right);
                    cursors.right = cursors.right.saturating_add(1);
                    if outputs_are_identical
                        || guard_is_false(&left.when)
                        || guard_is_false(&right.when)
                    {
                        continue;
                    }
                    return Some(Planned {
                        obligation: Obligation::OutcomeConflict {
                            left_rule_id: left.id.clone(),
                            right_rule_id: right.id.clone(),
                        },
                        synthetic: None,
                    });
                }
            }
            Check::ConstraintCompliance => {
                while let Some(rule) = rules.get(cursors.compliance_rule).copied() {
                    let mut matching_constraints = constraints
                        .iter()
                        .copied()
                        .filter(|constraint| constraint.for_outcome == rule.then.outcome);
                    if let Some(constraint) = matching_constraints.nth(cursors.constraint) {
                        cursors.constraint = cursors.constraint.saturating_add(1);
                        return Some(Planned {
                            obligation: Obligation::ConstraintCompliance {
                                rule_id: rule.id.clone(),
                                constraint_id: constraint.id.clone(),
                            },
                            synthetic: None,
                        });
                    }
                    if cursors.constraint == 0
                        && !constraints
                            .iter()
                            .any(|constraint| constraint.for_outcome == rule.then.outcome)
                    {
                        cursors.constraint = 1;
                        return Some(Planned {
                            obligation: Obligation::ConstraintCompliance {
                                rule_id: rule.id.clone(),
                                constraint_id: String::new(),
                            },
                            synthetic: Some(FindingCategory::NoRequirementsForOutcome),
                        });
                    }
                    cursors.compliance_rule = cursors.compliance_rule.saturating_add(1);
                    cursors.constraint = 0;
                }
            }
        }
        cursors.check = cursors.check.saturating_add(1);
        cursors.rule = 0;
        cursors.left = 0;
        cursors.right = 1;
        cursors.compliance_rule = 0;
        cursors.constraint = 0;
    }
}

fn guard_is_false(expression: &author_model::Expr) -> bool {
    match expression {
        author_model::Expr::Bool { value: false } => true,
        author_model::Expr::And { args } => args.iter().any(guard_is_false),
        _ => false,
    }
}

#[derive(Default)]
struct PlanCursors {
    check: usize,
    rule: usize,
    left: usize,
    right: usize,
    compliance_rule: usize,
    constraint: usize,
}

/// Analyze a compiled model, replaying all SAT witnesses with the independent model evaluator.
///
/// # Errors
/// Returns a model diagnostic when selected assumptions are invalid, or a resource-limit
/// diagnostic when a configured budget is invalid.
#[allow(clippy::too_many_lines)] // Keeps ordered budget and prerequisite orchestration visible together.
pub fn analyze<B: Backend>(
    model: &CompiledModel,
    options: &AnalysisOptions,
    backend: &mut B,
) -> Result<AnalysisReport, Diagnostic> {
    if options.budget.total_timeout_ms == Some(0)
        || options.budget.per_obligation_timeout_ms == Some(0)
        || options.budget.max_obligations == Some(0)
        || options.budget.solver_resource_limit == Some(0)
    {
        return Err(diagnostic(
            "resource_limit",
            "analysis budget values must be nonzero",
        ));
    }
    let _ = select_assumptions(model, &options.assumption_ids)?;
    let mut assumptions = options.assumption_ids.clone();
    assumptions.sort();
    let checks = check_selected(options);
    let mut rules: Vec<_> = model.raw().rules.iter().collect();
    rules.sort_by(|left, right| left.id.cmp(&right.id));
    let mut constraints: Vec<_> = model.raw().constraints.iter().collect();
    constraints.sort_by(|left, right| left.id.cmp(&right.id));
    let output_signatures = rules
        .iter()
        .map(|rule| {
            serde_json::to_string(&rule.then)
                .map_err(|error| diagnostic("serialization_error", &error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let budget = options.budget.normalized();
    let started = Instant::now();
    let mut results = Vec::new();
    let mut cache_hits = 0_usize;
    let mut solver_query_count = 0_usize;
    let mut executed_obligation_count = 0_usize;
    let mut remaining_work = false;
    let mut cursors = PlanCursors {
        right: 1,
        ..PlanCursors::default()
    };
    let mut feasibility_empty = false;
    let mut feasibility_confirmed = false;
    let mut complete = true;

    while let Some(planned) = next_plan(
        &rules,
        &constraints,
        &output_signatures,
        &checks,
        &mut cursors,
    ) {
        if results.len() >= budget.max_obligations {
            complete = false;
            results.push(skipped_result(
                model,
                &planned.obligation,
                &assumptions,
                "max_obligations",
            ));
            remaining_work = true;
            break;
        }
        let elapsed = started.elapsed();
        if elapsed >= Duration::from_millis(budget.total_timeout_ms) {
            complete = false;
            results.push(skipped_result(
                model,
                &planned.obligation,
                &assumptions,
                "total_timeout",
            ));
            remaining_work = true;
            break;
        }
        let mut result = base_result(model, &planned.obligation, &assumptions);
        if planned.obligation != Obligation::DomainFeasibility
            && (model.raw().outcomes.is_empty()
                || (model.raw().inputs.is_empty() && model.raw().rules.is_empty()))
        {
            result.status = ResultStatus::Skipped;
            result.skip_reason = Some("incomplete_model".to_owned());
            results.push(result);
            continue;
        }
        if feasibility_empty && planned.obligation != Obligation::DomainFeasibility {
            result.status = ResultStatus::Skipped;
            result.skip_reason = Some("empty_domain".to_owned());
            results.push(result);
            continue;
        }
        if let Some(category) = planned.synthetic {
            result.status = ResultStatus::Skipped;
            result.finding_category = Some(category);
            result.skip_reason = Some(
                if matches!(
                    result.finding_category,
                    Some(FindingCategory::BehaviorUnspecified)
                ) {
                    "no_rules"
                } else {
                    "no_requirements"
                }
                .to_owned(),
            );
            results.push(result);
            continue;
        }
        let remaining_ms = budget
            .total_timeout_ms
            .saturating_sub(u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX));
        let solve_budget = SolveBudget {
            timeout_ms: budget.per_obligation_timeout_ms.min(remaining_ms),
            solver_resource_limit: budget.solver_resource_limit,
        };
        if solve_budget.timeout_ms == 0 {
            complete = false;
            result.status = ResultStatus::Skipped;
            result.skip_reason = Some("total_timeout".to_owned());
            results.push(result);
            continue;
        }
        let solve_started = Instant::now();
        let (answer, cache_hit) = cached_solve(
            backend,
            model,
            &planned.obligation,
            &assumptions,
            &solve_budget,
        );
        executed_obligation_count = executed_obligation_count.saturating_add(1);
        if cache_hit {
            cache_hits = cache_hits.saturating_add(1);
        } else {
            solver_query_count = solver_query_count.saturating_add(1);
            result.statistics = backend.statistics();
        }
        result.elapsed_ms =
            Some(u64::try_from(solve_started.elapsed().as_millis()).unwrap_or(u64::MAX));
        map_answer(
            model,
            &planned.obligation,
            answer,
            &mut result,
            &assumptions,
        );
        if planned.obligation == Obligation::DomainFeasibility {
            feasibility_empty = result.finding_category == Some(FindingCategory::EmptyDomain);
            feasibility_confirmed = matches!(
                result.finding_category,
                Some(FindingCategory::Feasible | FindingCategory::EmptyDomain)
            );
        } else if !feasibility_confirmed
            && matches!(
                result.finding_category,
                Some(
                    FindingCategory::Unreachable
                        | FindingCategory::Covered
                        | FindingCategory::Compatible
                        | FindingCategory::Compliant
                ),
            )
        {
            result.labels.push("feasibility_unconfirmed".to_owned());
        }
        if matches!(
            result.status,
            ResultStatus::Timeout | ResultStatus::Cancelled
        ) {
            complete = false;
        }
        results.push(result);
    }

    annotate_compliance_reachability(&mut results);
    let planned_obligation_count = results.len();
    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let participating_proposed_declaration_ids =
        proposed_declarations(model, &checks, &options.assumption_ids);
    Ok(AnalysisReport {
        language_version: model.raw().language_version.clone(),
        engine_version: ENGINE_VERSION.to_owned(),
        solver_version: backend.solver_version(),
        model_id: model.raw().model_id.clone(),
        content_hash: model.content_hash().to_owned(),
        semantic_hash: model.semantic_hash().to_owned(),
        selected_assumption_ids: assumptions,
        domain_predicate_ids: model.raw().domain.iter().map(|p| p.id.clone()).collect(),
        checks,
        budget,
        participating_proposed_declaration_ids,
        open_items: model.raw().open_items.clone(),
        results,
        complete,
        cache_hits,
        solver_query_count,
        planned_obligation_count,
        executed_obligation_count,
        elapsed_ms,
        remaining_work,
    })
}

fn map_answer(
    model: &CompiledModel,
    obligation: &Obligation,
    answer: BackendAnswer,
    result: &mut ObligationResult,
    assumptions: &[String],
) {
    match answer {
        BackendAnswer::Sat { inputs } => {
            let replayed = replay_witness(model, obligation, &inputs, assumptions).unwrap_or(false);
            if !replayed {
                result.status = ResultStatus::Error;
                result.diagnostic_code = Some("witness_mismatch".to_owned());
                result.message = Some(
                    "backend SAT witness does not satisfy the requested obligation".to_owned(),
                );
                return;
            }
            result.witness = Some(inputs);
            match obligation {
                Obligation::DomainFeasibility => {
                    result.status = ResultStatus::Satisfied;
                    result.finding_category = Some(FindingCategory::Feasible);
                }
                Obligation::RuleReachability { .. } => {
                    result.status = ResultStatus::Satisfied;
                    result.finding_category = Some(FindingCategory::Reachable);
                }
                Obligation::Coverage => {
                    result.status = ResultStatus::Finding;
                    result.finding_category = Some(FindingCategory::Uncovered);
                }
                Obligation::OutcomeConflict { .. } => {
                    result.status = ResultStatus::Finding;
                    result.finding_category = Some(FindingCategory::Conflict);
                }
                Obligation::ConstraintCompliance { .. } => {
                    result.status = ResultStatus::Finding;
                    result.finding_category = Some(FindingCategory::Violation);
                }
            }
        }
        BackendAnswer::Unsat => match obligation {
            Obligation::DomainFeasibility => {
                result.status = ResultStatus::Finding;
                result.finding_category = Some(FindingCategory::EmptyDomain);
            }
            Obligation::RuleReachability { .. } => {
                result.status = ResultStatus::Finding;
                result.finding_category = Some(FindingCategory::Unreachable);
            }
            Obligation::Coverage => {
                result.status = ResultStatus::Satisfied;
                result.finding_category = Some(FindingCategory::Covered);
            }
            Obligation::OutcomeConflict { .. } => {
                result.status = ResultStatus::Satisfied;
                result.finding_category = Some(FindingCategory::Compatible);
            }
            Obligation::ConstraintCompliance { .. } => {
                result.status = ResultStatus::Satisfied;
                result.finding_category = Some(FindingCategory::Compliant);
            }
        },
        BackendAnswer::Unknown { reason } => {
            result.status = ResultStatus::Unknown;
            result.message = Some(reason);
        }
        BackendAnswer::Timeout => {
            result.status = ResultStatus::Timeout;
            result.message = Some("solver query timed out".to_owned());
        }
        BackendAnswer::Cancelled => {
            result.status = ResultStatus::Cancelled;
            result.message = Some("solver query cancelled".to_owned());
        }
        BackendAnswer::Error { code, message } => {
            result.status = ResultStatus::Error;
            result.diagnostic_code = Some(code);
            result.message = Some(message);
        }
    }
}

fn replay_witness(
    model: &CompiledModel,
    obligation: &Obligation,
    inputs: &InputValues,
    assumptions: &[String],
) -> Result<bool, Diagnostic> {
    let evaluation = evaluate(model, inputs, assumptions, None)?;
    if evaluation.classification == "outside_domain" {
        return Ok(false);
    }
    let matched: BTreeSet<_> = evaluation
        .matched_rule_ids
        .iter()
        .map(String::as_str)
        .collect();
    Ok(match obligation {
        Obligation::DomainFeasibility => true,
        Obligation::RuleReachability { rule_id } => matched.contains(rule_id.as_str()),
        Obligation::Coverage => matched.is_empty(),
        Obligation::OutcomeConflict {
            left_rule_id,
            right_rule_id,
        } => {
            if !matched.contains(left_rule_id.as_str()) || !matched.contains(right_rule_id.as_str())
            {
                return Ok(false);
            }
            let outcomes = rule_outcome_values(
                model,
                inputs,
                &evaluation.derived,
                [left_rule_id, right_rule_id],
            )?;
            outcomes.first() != outcomes.get(1)
        }
        Obligation::ConstraintCompliance {
            rule_id,
            constraint_id,
        } => {
            if !matched.contains(rule_id.as_str()) {
                return Ok(false);
            }
            let Some(rule) = model.raw().rules.iter().find(|rule| rule.id == *rule_id) else {
                return Ok(false);
            };
            let Some(constraint) = model
                .raw()
                .constraints
                .iter()
                .find(|constraint| constraint.id == *constraint_id)
            else {
                return Ok(false);
            };
            if constraint.for_outcome != rule.then.outcome {
                return Ok(false);
            }
            let fields =
                rule_outcome_values(model, inputs, &evaluation.derived, [rule_id, rule_id])?;
            let outcome_fields = fields.first().map(|value| &value.fields).ok_or_else(|| {
                diagnostic("witness_mismatch", "rule outcome could not be replayed")
            })?;
            author_model::eval_expr(
                &constraint.must,
                inputs,
                &evaluation.derived,
                outcome_fields,
            )? != Value::Bool { value: true }
        }
    })
}

fn rule_outcome_values(
    model: &CompiledModel,
    inputs: &InputValues,
    derived: &BTreeMap<String, Value>,
    ids: [&str; 2],
) -> Result<Vec<OutcomeValue>, Diagnostic> {
    ids.iter()
        .map(|id| {
            let rule = model
                .raw()
                .rules
                .iter()
                .find(|rule| rule.id == *id)
                .ok_or_else(|| diagnostic("unknown_reference", "replay rule not found"))?;
            let mut fields = BTreeMap::new();
            for (name, expr) in &rule.then.fields {
                fields.insert(
                    name.clone(),
                    author_model::eval_expr(expr, inputs, derived, &BTreeMap::new())?,
                );
            }
            Ok(OutcomeValue {
                outcome: rule.then.outcome.clone(),
                fields,
            })
        })
        .collect()
}

fn base_result(
    model: &CompiledModel,
    obligation: &Obligation,
    assumptions: &[String],
) -> ObligationResult {
    let mut ids = obligation.declaration_ids();
    if obligation == &Obligation::Coverage {
        ids.extend(model.raw().rules.iter().map(|rule| rule.id.clone()));
        ids.sort();
        ids.dedup();
    }
    let conditional = !assumptions.is_empty();
    let mut source_ids: BTreeSet<_> = ids
        .iter()
        .filter_map(|id| declaration_origin(model, id))
        .flat_map(|origin| origin.source_ids.iter().cloned())
        .collect();
    for predicate in &model.raw().domain {
        source_ids.extend(predicate.origin.source_ids.iter().cloned());
    }
    for origin in model
        .raw()
        .inputs
        .iter()
        .map(|item| &item.origin)
        .chain(model.raw().derived.iter().map(|item| &item.origin))
        .chain(model.raw().enums.iter().map(|item| &item.origin))
        .chain(model.raw().outcomes.iter().map(|item| &item.origin))
    {
        source_ids.extend(origin.source_ids.iter().cloned());
    }
    for assumption in assumptions {
        if let Some(predicate) = model
            .raw()
            .assumptions
            .iter()
            .find(|item| item.id == *assumption)
        {
            source_ids.extend(predicate.origin.source_ids.iter().cloned());
        }
    }
    let mut labels = Vec::new();
    if conditional {
        labels.push("conditional_on_assumptions".to_owned());
    }
    ObligationResult {
        obligation_id: obligation.id(),
        obligation: obligation.clone(),
        declaration_ids: ids,
        status: ResultStatus::Skipped,
        finding_category: None,
        witness: None,
        source_ids: source_ids.into_iter().collect(),
        conditional,
        labels,
        rule_reachability_status: None,
        elapsed_ms: None,
        statistics: None,
        skip_reason: None,
        diagnostic_code: None,
        message: None,
    }
}

fn declaration_origin<'a>(model: &'a CompiledModel, id: &str) -> Option<&'a author_model::Origin> {
    model
        .raw()
        .inputs
        .iter()
        .find(|x| x.id == id)
        .map(|x| &x.origin)
        .or_else(|| {
            model
                .raw()
                .derived
                .iter()
                .find(|x| x.id == id)
                .map(|x| &x.origin)
        })
        .or_else(|| {
            model
                .raw()
                .enums
                .iter()
                .find(|x| x.id == id)
                .map(|x| &x.origin)
        })
        .or_else(|| {
            model
                .raw()
                .outcomes
                .iter()
                .find(|x| x.id == id)
                .map(|x| &x.origin)
        })
        .or_else(|| {
            model
                .raw()
                .domain
                .iter()
                .find(|x| x.id == id)
                .map(|x| &x.origin)
        })
        .or_else(|| {
            model
                .raw()
                .assumptions
                .iter()
                .find(|x| x.id == id)
                .map(|x| &x.origin)
        })
        .or_else(|| {
            model
                .raw()
                .rules
                .iter()
                .find(|x| x.id == id)
                .map(|x| &x.origin)
        })
        .or_else(|| {
            model
                .raw()
                .constraints
                .iter()
                .find(|x| x.id == id)
                .map(|x| &x.origin)
        })
}

fn annotate_compliance_reachability(results: &mut [ObligationResult]) {
    let reachability: HashMap<_, _> = results
        .iter()
        .filter_map(|result| {
            if let Obligation::RuleReachability { rule_id } = &result.obligation {
                Some((
                    rule_id.clone(),
                    (result.finding_category.clone(), result.status.clone()),
                ))
            } else {
                None
            }
        })
        .collect();
    for result in results {
        if let Obligation::ConstraintCompliance { rule_id, .. } = &result.obligation {
            if let Some((category, status)) = reachability.get(rule_id) {
                result.rule_reachability_status = Some(status.clone());
                if *category == Some(FindingCategory::Unreachable)
                    && result.finding_category == Some(FindingCategory::Compliant)
                {
                    result.labels.push("vacuous_unreachable_rule".to_owned());
                }
            }
        }
    }
}

fn proposed_declarations(
    model: &CompiledModel,
    checks: &[Check],
    assumption_ids: &[String],
) -> Vec<String> {
    let mut ids = BTreeSet::new();
    let include_rules = checks.iter().any(|c| {
        matches!(
            c,
            Check::RuleReachability
                | Check::Coverage
                | Check::OutcomeConflict
                | Check::ConstraintCompliance
        )
    });
    let include_constraints = checks.contains(&Check::ConstraintCompliance);
    ids.extend(
        model
            .raw()
            .inputs
            .iter()
            .filter(|x| matches!(&x.origin.review, author_model::Review::Proposed))
            .map(|x| x.id.clone()),
    );
    ids.extend(
        model
            .raw()
            .derived
            .iter()
            .filter(|x| matches!(&x.origin.review, author_model::Review::Proposed))
            .map(|x| x.id.clone()),
    );
    ids.extend(
        model
            .raw()
            .enums
            .iter()
            .filter(|x| matches!(&x.origin.review, author_model::Review::Proposed))
            .map(|x| x.id.clone()),
    );
    ids.extend(
        model
            .raw()
            .outcomes
            .iter()
            .filter(|x| matches!(&x.origin.review, author_model::Review::Proposed))
            .map(|x| x.id.clone()),
    );
    if include_rules {
        ids.extend(
            model
                .raw()
                .rules
                .iter()
                .filter(|x| matches!(&x.origin.review, author_model::Review::Proposed))
                .map(|x| x.id.clone()),
        );
    }
    if include_constraints {
        ids.extend(
            model
                .raw()
                .constraints
                .iter()
                .filter(|x| matches!(&x.origin.review, author_model::Review::Proposed))
                .map(|x| x.id.clone()),
        );
    }
    ids.extend(
        model
            .raw()
            .domain
            .iter()
            .filter(|x| matches!(&x.origin.review, author_model::Review::Proposed))
            .map(|x| x.id.clone()),
    );
    ids.extend(
        assumption_ids
            .iter()
            .filter_map(|id| model.raw().assumptions.iter().find(|item| item.id == *id))
            .filter(|x| matches!(&x.origin.review, author_model::Review::Proposed))
            .map(|x| x.id.clone()),
    );
    ids.into_iter().collect()
}

fn skipped_result(
    model: &CompiledModel,
    obligation: &Obligation,
    assumptions: &[String],
    reason: &str,
) -> ObligationResult {
    let mut result = base_result(model, obligation, assumptions);
    result.status = ResultStatus::Skipped;
    result.skip_reason = Some(reason.to_owned());
    result
}

fn diagnostic(code: &str, message: &str) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        message: message.to_owned(),
        pointer: None,
        declaration_id: None,
        source_ids: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockBackend {
        answers: Vec<BackendAnswer>,
        calls: Vec<Obligation>,
        configuration: String,
    }

    impl Backend for MockBackend {
        fn solve(
            &mut self,
            _model: &CompiledModel,
            obligation: &Obligation,
            _assumptions: &[String],
            _budget: &SolveBudget,
        ) -> BackendAnswer {
            self.calls.push(obligation.clone());
            if self.answers.is_empty() {
                return BackendAnswer::Error {
                    code: "unexpected_query".to_owned(),
                    message: "no mock answer remains".to_owned(),
                };
            }
            self.answers.remove(0)
        }

        fn solver_version(&self) -> String {
            "mock/1".to_owned()
        }

        fn solver_configuration(&self) -> String {
            self.configuration.clone()
        }

        fn cache_namespace(&self) -> Option<String> {
            Some("same-namespace-on-purpose".to_owned())
        }
    }

    struct OtherMockBackend {
        answer: BackendAnswer,
        calls: usize,
    }

    impl Backend for OtherMockBackend {
        fn solve(
            &mut self,
            _model: &CompiledModel,
            _obligation: &Obligation,
            _assumptions: &[String],
            _budget: &SolveBudget,
        ) -> BackendAnswer {
            self.calls = self.calls.saturating_add(1);
            self.answer.clone()
        }

        fn solver_version(&self) -> String {
            "mock/1".to_owned()
        }

        fn solver_configuration(&self) -> String {
            "same-settings".to_owned()
        }

        fn cache_namespace(&self) -> Option<String> {
            Some("same-namespace-on-purpose".to_owned())
        }
    }

    fn unsafe_model() -> CompiledModel {
        let input = include_str!("../../../docs/examples/models/unsafe-grant.json");
        let value: serde_json::Value = serde_json::from_str(input)
            .unwrap_or_else(|error| panic!("fixture JSON must parse: {error}"));
        author_model::compile_json(&value, &author_model::ModelLimits::default())
            .unwrap_or_else(|errors| panic!("fixture model must compile: {errors:?}"))
    }

    fn compile_fixture(value: &serde_json::Value) -> CompiledModel {
        author_model::compile_json(value, &author_model::ModelLimits::default())
            .unwrap_or_else(|errors| panic!("fixture model must compile: {errors:?}"))
    }

    fn witness() -> InputValues {
        BTreeMap::from([
            (
                "sponsor_approval".to_owned(),
                Value::Enum {
                    enum_name: "ApprovalStatus".to_owned(),
                    variant: "Approved".to_owned(),
                },
            ),
            (
                "contract_end".to_owned(),
                Value::Date {
                    value: "2026-12-31".to_owned(),
                },
            ),
            (
                "requested_expiry".to_owned(),
                Value::Date {
                    value: "2027-01-01".to_owned(),
                },
            ),
        ])
    }

    #[test]
    fn mandatory_feasibility_runs_first_and_sat_is_replayed() {
        let model = unsafe_model();
        let backend_answer = BackendAnswer::Sat { inputs: witness() };
        let mut backend = MockBackend {
            answers: vec![backend_answer],
            calls: Vec::new(),
            configuration: "mandatory-feasibility".to_owned(),
        };
        let report =
            analyze(&model, &AnalysisOptions::default(), &mut backend).expect("analysis succeeds");

        assert_eq!(backend.calls, vec![Obligation::DomainFeasibility]);
        assert_eq!(
            report.results[0].finding_category,
            Some(FindingCategory::Feasible)
        );
        assert_eq!(report.results[0].status, ResultStatus::Satisfied);
        assert!(report.results[0].witness.is_some());
    }

    #[test]
    fn reachability_coverage_and_compliance_answers_are_replayed_and_mapped() {
        let model = unsafe_model();
        let checks = AnalysisOptions {
            checks: vec![Check::RuleReachability, Check::Coverage],
            ..AnalysisOptions::default()
        };
        let mut backend = MockBackend {
            answers: vec![
                BackendAnswer::Sat { inputs: witness() },
                BackendAnswer::Sat { inputs: witness() },
                BackendAnswer::Sat {
                    inputs: BTreeMap::from([
                        (
                            "sponsor_approval".to_owned(),
                            Value::Enum {
                                enum_name: "ApprovalStatus".to_owned(),
                                variant: "NotApproved".to_owned(),
                            },
                        ),
                        (
                            "contract_end".to_owned(),
                            Value::Date {
                                value: "2026-12-31".to_owned(),
                            },
                        ),
                        (
                            "requested_expiry".to_owned(),
                            Value::Date {
                                value: "2027-01-01".to_owned(),
                            },
                        ),
                    ]),
                },
            ],
            calls: Vec::new(),
            configuration: "reachability-coverage".to_owned(),
        };
        let report = analyze(&model, &checks, &mut backend).expect("analysis succeeds");
        assert_eq!(
            report
                .results
                .get(1)
                .and_then(|result| result.finding_category.clone()),
            Some(FindingCategory::Reachable)
        );
        assert_eq!(
            report
                .results
                .get(2)
                .and_then(|result| result.finding_category.clone()),
            Some(FindingCategory::Uncovered)
        );

        let compliance_options = AnalysisOptions {
            checks: vec![Check::ConstraintCompliance],
            ..AnalysisOptions::default()
        };
        let mut compliance_backend = MockBackend {
            answers: vec![
                BackendAnswer::Sat { inputs: witness() },
                BackendAnswer::Sat { inputs: witness() },
                BackendAnswer::Unsat,
                BackendAnswer::Sat { inputs: witness() },
            ],
            calls: Vec::new(),
            configuration: "compliance-mapping".to_owned(),
        };
        let compliance_report = analyze(&model, &compliance_options, &mut compliance_backend)
            .expect("analysis succeeds");
        assert_eq!(
            compliance_report.checks,
            vec![
                Check::DomainFeasibility,
                Check::RuleReachability,
                Check::ConstraintCompliance
            ]
        );
        assert_eq!(
            compliance_report
                .results
                .get(2)
                .and_then(|result| result.finding_category.clone()),
            Some(FindingCategory::Compliant)
        );
        assert_eq!(
            compliance_report
                .results
                .get(3)
                .and_then(|result| result.finding_category.clone()),
            Some(FindingCategory::Violation)
        );
        assert_eq!(
            compliance_report
                .results
                .get(3)
                .and_then(|result| result.rule_reachability_status.clone()),
            Some(ResultStatus::Satisfied)
        );
    }

    #[test]
    fn compliance_only_tracks_unknown_reachability_within_the_obligation_budget() {
        let options = AnalysisOptions {
            checks: vec![Check::ConstraintCompliance],
            budget: AnalysisBudget {
                max_obligations: Some(2),
                ..AnalysisBudget::default()
            },
            ..AnalysisOptions::default()
        };
        let mut backend = MockBackend {
            answers: vec![
                BackendAnswer::Sat { inputs: witness() },
                BackendAnswer::Unknown {
                    reason: "reachability undecided".to_owned(),
                },
            ],
            calls: Vec::new(),
            configuration: "compliance-unknown-reachability-budget".to_owned(),
        };
        let report = analyze(&unsafe_model(), &options, &mut backend).expect("analysis succeeds");
        assert_eq!(backend.calls.len(), 2);
        assert_eq!(report.solver_query_count, 2);
        assert!(report.remaining_work);
        assert!(!report.complete);
        let compliance = report
            .results
            .last()
            .expect("budgeted compliance result exists");
        assert_eq!(compliance.status, ResultStatus::Skipped);
        assert_eq!(
            compliance.rule_reachability_status,
            Some(ResultStatus::Unknown)
        );
        assert!(
            !compliance
                .labels
                .iter()
                .any(|label| label == "vacuous_unreachable_rule")
        );
    }

    #[test]
    fn unreachable_rule_compliance_is_explicitly_vacuous() {
        let options = AnalysisOptions {
            checks: vec![Check::ConstraintCompliance],
            ..AnalysisOptions::default()
        };
        let mut backend = MockBackend {
            answers: vec![
                BackendAnswer::Sat { inputs: witness() },
                BackendAnswer::Unsat,
                BackendAnswer::Unsat,
                BackendAnswer::Unsat,
            ],
            calls: Vec::new(),
            configuration: "vacuous-compliance".to_owned(),
        };
        let report = analyze(&unsafe_model(), &options, &mut backend).expect("analysis succeeds");
        let compliance = report
            .results
            .get(2)
            .expect("first compliance result exists");

        assert_eq!(
            compliance.finding_category,
            Some(FindingCategory::Compliant)
        );
        assert_eq!(
            compliance.rule_reachability_status,
            Some(ResultStatus::Finding)
        );
        assert!(
            compliance
                .labels
                .iter()
                .any(|label| label == "vacuous_unreachable_rule")
        );
    }

    #[test]
    fn conflict_witness_requires_two_applicable_rules_with_distinct_outputs() {
        let input = include_str!("../../../docs/examples/models/conflict-tag.json");
        let value: serde_json::Value = serde_json::from_str(input)
            .unwrap_or_else(|error| panic!("fixture JSON must parse: {error}"));
        let model = compile_fixture(&value);
        let conflict_witness = BTreeMap::from([
            (
                "sponsor_approval".to_owned(),
                Value::Enum {
                    enum_name: "ApprovalStatus".to_owned(),
                    variant: "Approved".to_owned(),
                },
            ),
            (
                "contract_end".to_owned(),
                Value::Date {
                    value: "2026-12-31".to_owned(),
                },
            ),
            (
                "requested_expiry".to_owned(),
                Value::Date {
                    value: "2026-12-30".to_owned(),
                },
            ),
        ]);
        let options = AnalysisOptions {
            checks: vec![Check::OutcomeConflict],
            ..AnalysisOptions::default()
        };
        let mut rules: Vec<_> = model.raw().rules.iter().collect();
        rules.sort_by(|left, right| left.id.cmp(&right.id));
        let mut constraints: Vec<_> = model.raw().constraints.iter().collect();
        constraints.sort_by(|left, right| left.id.cmp(&right.id));
        let signatures = rules
            .iter()
            .map(|rule| serde_json::to_string(&rule.then).expect("rule output serializes"))
            .collect::<Vec<_>>();
        let checks = check_selected(&options);
        let mut cursors = PlanCursors {
            right: 1,
            ..PlanCursors::default()
        };
        let mut answers = Vec::new();
        while let Some(planned) =
            next_plan(&rules, &constraints, &signatures, &checks, &mut cursors)
        {
            answers.push(
                if replay_witness(&model, &planned.obligation, &conflict_witness, &[])
                    .unwrap_or(false)
                {
                    BackendAnswer::Sat {
                        inputs: conflict_witness.clone(),
                    }
                } else {
                    BackendAnswer::Unsat
                },
            );
        }
        let mut backend = MockBackend {
            answers,
            calls: Vec::new(),
            configuration: "conflict-replay-canonical".to_owned(),
        };
        let report = analyze(&model, &options, &mut backend).expect("analysis succeeds");

        assert_eq!(
            backend.calls.len(),
            3,
            "identical-output pair is pruned without solver invocation"
        );
        assert!(
            report
                .results
                .iter()
                .any(|result| { result.finding_category == Some(FindingCategory::Compatible) })
        );
        assert!(report.results.iter().any(|result| {
            result.finding_category == Some(FindingCategory::Conflict) && result.witness.is_some()
        }));
    }

    #[test]
    fn backend_inconclusive_and_error_answers_are_preserved() {
        let outcomes = [
            (
                BackendAnswer::Unknown {
                    reason: "injected unknown".to_owned(),
                },
                ResultStatus::Unknown,
            ),
            (BackendAnswer::Timeout, ResultStatus::Timeout),
            (BackendAnswer::Cancelled, ResultStatus::Cancelled),
            (
                BackendAnswer::Error {
                    code: "injected".to_owned(),
                    message: "injected error".to_owned(),
                },
                ResultStatus::Error,
            ),
        ];
        for (index, (answer, expected)) in outcomes.into_iter().enumerate() {
            let mut backend = MockBackend {
                answers: vec![answer],
                calls: Vec::new(),
                configuration: format!("inconclusive-{index}"),
            };
            let report = analyze(&unsafe_model(), &AnalysisOptions::default(), &mut backend)
                .expect("analysis succeeds");
            let result = report
                .results
                .first()
                .expect("mandatory feasibility result exists");
            assert_eq!(result.status, expected);
            assert!(result.witness.is_none());
            assert_eq!(
                report.complete,
                expected != ResultStatus::Timeout && expected != ResultStatus::Cancelled
            );
        }
    }

    #[test]
    fn unfinished_obligations_are_bounded_and_mark_report_incomplete() {
        let options = AnalysisOptions {
            checks: vec![Check::RuleReachability, Check::Coverage],
            assumption_ids: Vec::new(),
            budget: AnalysisBudget {
                max_obligations: Some(2),
                ..AnalysisBudget::default()
            },
        };
        let mut backend = MockBackend {
            answers: vec![
                BackendAnswer::Sat { inputs: witness() },
                BackendAnswer::Sat { inputs: witness() },
            ],
            calls: Vec::new(),
            configuration: "bounded-planning".to_owned(),
        };
        let report = analyze(&unsafe_model(), &options, &mut backend).expect("analysis succeeds");

        assert_eq!(
            backend.calls,
            vec![
                Obligation::DomainFeasibility,
                Obligation::RuleReachability {
                    rule_id: "GrantApprovedRequest".to_owned()
                }
            ]
        );
        assert!(!report.complete);
        assert_eq!(report.results.len(), 3);
        assert_eq!(
            report
                .results
                .last()
                .and_then(|result| result.skip_reason.as_deref()),
            Some("max_obligations")
        );
    }

    #[test]
    fn zero_input_constant_decision_is_analyzed_not_treated_as_a_draft() {
        let input = include_str!("../../../docs/examples/models/unsafe-grant.json");
        let mut value: serde_json::Value = serde_json::from_str(input)
            .unwrap_or_else(|error| panic!("fixture JSON must parse: {error}"));
        let constant_rule = serde_json::json!({
            "id": "AlwaysGrant",
            "when": {"kind": "bool", "value": true},
            "then": {"outcome": "Grant", "fields": {"expiry": {"kind": "date", "value": "2026-12-31"}}},
            "origin": {"kind": "proposal", "source_ids": [], "review": "proposed"}
        });
        if let Some(object) = value.as_object_mut() {
            object.insert("inputs".to_owned(), serde_json::Value::Array(Vec::new()));
            object.insert(
                "rules".to_owned(),
                serde_json::Value::Array(vec![constant_rule]),
            );
            object.insert("domain".to_owned(), serde_json::Value::Array(Vec::new()));
            object.insert(
                "constraints".to_owned(),
                serde_json::Value::Array(Vec::new()),
            );
            object.insert(
                "assumptions".to_owned(),
                serde_json::Value::Array(Vec::new()),
            );
        }
        let model = compile_fixture(&value);
        let mut backend = MockBackend {
            answers: vec![
                BackendAnswer::Sat {
                    inputs: InputValues::new(),
                },
                BackendAnswer::Sat {
                    inputs: InputValues::new(),
                },
            ],
            calls: Vec::new(),
            configuration: "constant-decision".to_owned(),
        };
        let options = AnalysisOptions {
            checks: vec![Check::RuleReachability],
            ..AnalysisOptions::default()
        };
        let report = analyze(&model, &options, &mut backend).expect("analysis succeeds");

        assert_eq!(
            report.results.first().map(|result| result.status.clone()),
            Some(ResultStatus::Satisfied)
        );
        assert_eq!(
            report
                .results
                .get(1)
                .and_then(|result| result.finding_category.clone()),
            Some(FindingCategory::Reachable)
        );
        assert_eq!(
            report
                .results
                .get(1)
                .and_then(|result| result.witness.as_ref().map(BTreeMap::len)),
            Some(0)
        );
    }

    #[test]
    fn empty_domain_skips_dependents_and_ruleless_coverage_is_unspecified() {
        let empty_input = include_str!("../../../docs/examples/models/empty-domain.json");
        let mut empty_value: serde_json::Value = serde_json::from_str(empty_input)
            .unwrap_or_else(|error| panic!("fixture JSON must parse: {error}"));
        if let Some(rules) = empty_value["rules"].as_array_mut() {
            for rule in rules {
                rule["origin"]["source_ids"] = serde_json::json!(["S1"]);
            }
        }
        let empty_model = compile_fixture(&empty_value);
        let options = AnalysisOptions {
            checks: vec![Check::Coverage],
            ..AnalysisOptions::default()
        };
        let mut empty_backend = MockBackend {
            answers: vec![BackendAnswer::Unsat],
            calls: Vec::new(),
            configuration: "empty-domain-prerequisite".to_owned(),
        };
        let empty_report =
            analyze(&empty_model, &options, &mut empty_backend).expect("analysis succeeds");
        assert_eq!(
            empty_report
                .results
                .first()
                .and_then(|result| result.finding_category.clone()),
            Some(FindingCategory::EmptyDomain)
        );
        assert_eq!(
            empty_report
                .results
                .get(1)
                .and_then(|result| result.skip_reason.as_deref()),
            Some("empty_domain")
        );
        assert_eq!(empty_backend.calls, vec![Obligation::DomainFeasibility]);
        let skipped_coverage = empty_report.results.get(1).expect("coverage skip exists");
        let mut expected_rule_ids: Vec<_> = empty_model
            .raw()
            .rules
            .iter()
            .map(|rule| rule.id.clone())
            .collect();
        expected_rule_ids.sort();
        assert_eq!(skipped_coverage.declaration_ids, expected_rule_ids);
        if !empty_model.raw().rules.is_empty() {
            assert!(
                skipped_coverage
                    .source_ids
                    .iter()
                    .any(|source| source == "S1")
            );
        }

        let mut draft_value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/examples/models/unsafe-grant.json"
        ))
        .unwrap_or_else(|error| panic!("fixture JSON must parse: {error}"));
        if let Some(object) = draft_value.as_object_mut() {
            object.insert("rules".to_owned(), serde_json::Value::Array(Vec::new()));
        }
        let ruleless_model = compile_fixture(&draft_value);
        let mut draft_backend = MockBackend {
            answers: vec![BackendAnswer::Sat { inputs: witness() }],
            calls: Vec::new(),
            configuration: "ruleless-coverage".to_owned(),
        };
        let draft_report =
            analyze(&ruleless_model, &options, &mut draft_backend).expect("analysis succeeds");
        assert_eq!(
            draft_report
                .results
                .get(1)
                .and_then(|result| result.finding_category.clone()),
            Some(FindingCategory::BehaviorUnspecified)
        );
        assert_eq!(draft_backend.calls, vec![Obligation::DomainFeasibility]);
    }

    #[test]
    fn coverage_provenance_tracks_all_rules_and_refreshes_on_cached_and_skipped_results() {
        let mut original_json: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/examples/models/unsafe-grant.json"
        ))
        .unwrap_or_else(|error| panic!("fixture JSON must parse: {error}"));
        if let Some(sources) = original_json["sources"].as_array_mut() {
            sources.push(serde_json::json!({"id":"RULE_SOURCE_OLD", "text":"Old rule provenance"}));
            sources.push(serde_json::json!({"id":"RULE_SOURCE_NEW", "text":"New rule provenance"}));
        }
        if let Some(rules) = original_json["rules"].as_array_mut() {
            for rule in rules {
                rule["origin"]["source_ids"] = serde_json::json!(["RULE_SOURCE_OLD"]);
            }
        }
        let original = compile_fixture(&original_json);
        let options = AnalysisOptions {
            checks: vec![Check::Coverage],
            ..AnalysisOptions::default()
        };
        let mut first_backend = MockBackend {
            answers: vec![
                BackendAnswer::Sat { inputs: witness() },
                BackendAnswer::Unsat,
            ],
            calls: Vec::new(),
            configuration: "coverage-provenance-cache-refresh".to_owned(),
        };
        let first = analyze(&original, &options, &mut first_backend).expect("first run succeeds");
        let first_coverage = first
            .results
            .iter()
            .find(|result| result.obligation == Obligation::Coverage)
            .expect("coverage result exists");
        let mut expected_rule_ids: Vec<_> = original
            .raw()
            .rules
            .iter()
            .map(|rule| rule.id.clone())
            .collect();
        expected_rule_ids.sort();
        assert_eq!(first_coverage.declaration_ids, expected_rule_ids);
        assert!(
            first_coverage
                .source_ids
                .iter()
                .any(|source| source == "RULE_SOURCE_OLD")
        );

        if let Some(rules) = original_json["rules"].as_array_mut() {
            for rule in rules {
                rule["origin"]["source_ids"] = serde_json::json!(["RULE_SOURCE_NEW"]);
            }
        }
        let updated = compile_fixture(&original_json);
        assert_eq!(original.semantic_hash(), updated.semantic_hash());
        let mut second_backend = MockBackend {
            answers: Vec::new(),
            calls: Vec::new(),
            configuration: "coverage-provenance-cache-refresh".to_owned(),
        };
        let second = analyze(&updated, &options, &mut second_backend).expect("cached run succeeds");
        let cached_coverage = second
            .results
            .iter()
            .find(|result| result.obligation == Obligation::Coverage)
            .expect("cached coverage result exists");
        assert_eq!(cached_coverage.declaration_ids, expected_rule_ids);
        assert!(
            cached_coverage
                .source_ids
                .iter()
                .any(|source| source == "RULE_SOURCE_NEW")
        );
        assert!(
            !cached_coverage
                .source_ids
                .iter()
                .any(|source| source == "RULE_SOURCE_OLD")
        );
        assert_eq!(second.cache_hits, 2);

        let skipped_options = AnalysisOptions {
            checks: vec![Check::Coverage],
            budget: AnalysisBudget {
                max_obligations: Some(1),
                ..AnalysisBudget::default()
            },
            ..AnalysisOptions::default()
        };
        let skipped = analyze(&updated, &skipped_options, &mut second_backend)
            .expect("budgeted run succeeds");
        let skipped_coverage = skipped
            .results
            .iter()
            .find(|result| result.obligation == Obligation::Coverage)
            .expect("budgeted coverage marker exists");
        assert_eq!(skipped_coverage.declaration_ids, expected_rule_ids);
        assert!(
            skipped_coverage
                .source_ids
                .iter()
                .any(|source| source == "RULE_SOURCE_NEW")
        );
    }

    #[test]
    fn cache_reuses_semantics_but_rebuilds_current_metadata() {
        let input = include_str!("../../../docs/examples/models/unsafe-grant.json");
        let mut original: serde_json::Value = serde_json::from_str(input)
            .unwrap_or_else(|error| panic!("fixture JSON must parse: {error}"));
        let initial_model = compile_fixture(&original);
        let first_answer = BackendAnswer::Unsat;
        let mut first_backend = MockBackend {
            answers: vec![first_answer],
            calls: Vec::new(),
            configuration: "metadata-cache".to_owned(),
        };
        let first_report = analyze(
            &initial_model,
            &AnalysisOptions::default(),
            &mut first_backend,
        )
        .expect("first analysis succeeds");

        let open_item = original
            .get_mut("open_items")
            .and_then(serde_json::Value::as_array_mut)
            .and_then(|items| items.first_mut())
            .and_then(serde_json::Value::as_object_mut)
            .and_then(|item| item.get_mut("text"));
        if let Some(text) = open_item {
            *text = serde_json::Value::String("Updated unresolved question".to_owned());
        }
        let current_model = compile_fixture(&original);
        assert_eq!(initial_model.semantic_hash(), current_model.semantic_hash());
        assert_ne!(first_report.content_hash, current_model.content_hash());
        let mut second_backend = MockBackend {
            answers: Vec::new(),
            calls: Vec::new(),
            configuration: "metadata-cache".to_owned(),
        };
        let current_report = analyze(
            &current_model,
            &AnalysisOptions::default(),
            &mut second_backend,
        )
        .expect("cached analysis succeeds");

        assert!(second_backend.calls.is_empty());
        assert_eq!(current_report.cache_hits, 1);
        assert_eq!(current_report.solver_query_count, 0);
        assert_eq!(current_report.executed_obligation_count, 1);
        assert_eq!(current_report.planned_obligation_count, 1);
        assert_eq!(current_report.content_hash, current_model.content_hash());
        assert_eq!(
            current_report
                .open_items
                .first()
                .map(|item| item.text.as_str()),
            Some("Updated unresolved question")
        );
    }

    #[test]
    fn recompiling_a_policy_edit_invalidates_cached_answers() {
        let input = include_str!("../../../docs/examples/models/empty-domain.json");
        let mut value: serde_json::Value = serde_json::from_str(input)
            .unwrap_or_else(|error| panic!("fixture JSON must parse: {error}"));
        let empty_domain = compile_fixture(&value);
        let mut first = MockBackend {
            answers: vec![BackendAnswer::Unsat],
            calls: Vec::new(),
            configuration: "recompile-policy-edit".to_owned(),
        };
        let options = AnalysisOptions {
            checks: Vec::new(),
            ..AnalysisOptions::default()
        };
        analyze(&empty_domain, &options, &mut first).expect("initial analysis succeeds");

        value["domain"][0]["expr"] = serde_json::json!({"kind":"bool", "value":true});
        let revised_policy = compile_fixture(&value);
        assert_ne!(empty_domain.semantic_hash(), revised_policy.semantic_hash());
        let mut second = MockBackend {
            answers: vec![BackendAnswer::Error {
                code: "recompiled_model_queried".to_owned(),
                message: "recompiled policy must not reuse the old UNSAT".to_owned(),
            }],
            calls: Vec::new(),
            configuration: "recompile-policy-edit".to_owned(),
        };
        let report = analyze(&revised_policy, &options, &mut second)
            .expect("revised policy analysis succeeds");

        assert_eq!(first.calls, vec![Obligation::DomainFeasibility]);
        assert_eq!(second.calls, vec![Obligation::DomainFeasibility]);
        assert_eq!(report.results[0].status, ResultStatus::Error);
    }

    #[test]
    fn different_backend_types_cannot_share_cached_answers() {
        let model = unsafe_model();
        let mut first = MockBackend {
            answers: vec![BackendAnswer::Unsat],
            calls: Vec::new(),
            configuration: "same-settings".to_owned(),
        };
        let options = AnalysisOptions {
            checks: Vec::new(),
            ..AnalysisOptions::default()
        };
        analyze(&model, &options, &mut first).expect("first analysis succeeds");

        let mut second = OtherMockBackend {
            answer: BackendAnswer::Error {
                code: "second_backend_called".to_owned(),
                message: "must not reuse the first backend's UNSAT".to_owned(),
            },
            calls: 0,
        };
        let report = analyze(&model, &options, &mut second).expect("second analysis succeeds");

        assert_eq!(first.calls, vec![Obligation::DomainFeasibility]);
        assert_eq!(second.calls, 1);
        assert_eq!(report.results[0].status, ResultStatus::Error);
    }

    #[test]
    fn malformed_sat_witness_is_not_cached_before_a_valid_retry() {
        let model = unsafe_model();
        let options = AnalysisOptions::default();
        let mut bad = MockBackend {
            answers: vec![BackendAnswer::Sat {
                inputs: InputValues::new(),
            }],
            calls: Vec::new(),
            configuration: "bad-then-good-witness-cache-regression".to_owned(),
        };
        let first = analyze(&model, &options, &mut bad).expect("first analysis succeeds");
        assert_eq!(first.results[0].status, ResultStatus::Error);

        let mut recovered = MockBackend {
            answers: vec![BackendAnswer::Sat { inputs: witness() }],
            calls: Vec::new(),
            configuration: "bad-then-good-witness-cache-regression".to_owned(),
        };
        let second = analyze(&model, &options, &mut recovered).expect("retry succeeds");
        assert_eq!(recovered.calls, vec![Obligation::DomainFeasibility]);
        assert_eq!(
            second.results[0].finding_category,
            Some(FindingCategory::Feasible)
        );
    }

    #[test]
    fn malformed_sat_witness_is_an_engine_error_not_a_finding() {
        let model = unsafe_model();
        let answer = BackendAnswer::Sat {
            inputs: InputValues::new(),
        };
        let mut backend = MockBackend {
            answers: vec![answer],
            calls: Vec::new(),
            configuration: "bad-witness".to_owned(),
        };
        let report =
            analyze(&model, &AnalysisOptions::default(), &mut backend).expect("analysis succeeds");

        assert_eq!(report.results[0].status, ResultStatus::Error);
        assert_eq!(
            report.results[0].diagnostic_code.as_deref(),
            Some("witness_mismatch")
        );
        assert!(report.results[0].witness.is_none());
    }
}
