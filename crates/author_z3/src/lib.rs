//! Native Z3 backend for Author's typed decision analysis.

use author_analysis::{Backend, BackendAnswer, Obligation, SolveBudget, SolverStatistics};
use author_model::{CompiledModel, Expr, InputValues, Type, Value};
use std::{collections::BTreeMap, str::FromStr};
use z3::{
    SatResult, Solver,
    ast::{Bool, Int},
};

const SOLVER_VERSION: &str = "Z3";
const DATE_MAX_ORDINAL: i64 = 3_652_058;

enum Term {
    Bool(Bool),
    Int(Int),
}

impl Clone for Term {
    fn clone(&self) -> Self {
        match self {
            Self::Bool(value) => Self::Bool(value.clone()),
            Self::Int(value) => Self::Int(value.clone()),
        }
    }
}

/// Each obligation gets an isolated Z3 solver; only detached statistics are retained.
#[derive(Clone, Debug, Default)]
pub struct Z3Backend {
    statistics: Option<SolverStatistics>,
}

impl Z3Backend {
    #[must_use]
    pub const fn new() -> Self {
        Self { statistics: None }
    }
}

impl Backend for Z3Backend {
    fn solve(
        &mut self,
        model: &CompiledModel,
        obligation: &Obligation,
        assumption_ids: &[String],
        budget: &SolveBudget,
    ) -> BackendAnswer {
        self.statistics = None;
        solve_query(
            model,
            obligation,
            assumption_ids,
            budget,
            &mut self.statistics,
        )
    }

    fn statistics(&self) -> Option<SolverStatistics> {
        self.statistics.clone()
    }

    fn solver_version(&self) -> String {
        format!("{SOLVER_VERSION} {}", z3::full_version())
    }

    fn solver_configuration(&self) -> String {
        "author-z3-encoding/1; qf-lia; enum-as-finite-integer; dates-as-bounded-ordinals; named-derived-dag".to_owned()
    }

    fn cache_namespace(&self) -> Option<String> {
        Some("author-z3-z3-encoding-v1".to_owned())
    }
}

#[allow(clippy::too_many_lines)] // Keep the exhaustive obligation dispatch easy to audit.
fn solve_query(
    model: &CompiledModel,
    obligation: &Obligation,
    assumption_ids: &[String],
    budget: &SolveBudget,
    statistics: &mut Option<SolverStatistics>,
) -> BackendAnswer {
    let solver = Solver::new();
    let mut params = z3::Params::new();
    let timeout_ms = u32::try_from(budget.timeout_ms).unwrap_or(u32::MAX);
    params.set_u32("timeout", timeout_ms);
    if let Some(limit) = budget.solver_resource_limit {
        params.set_u32("rlimit", limit);
    }
    solver.set_params(&params);

    let mut environment = BTreeMap::new();
    for input in &model.raw().inputs {
        let term = match &input.r#type {
            Type::Bool => Term::Bool(Bool::new_const(format!("input_{}", input.id))),
            Type::Int { .. } | Type::Date | Type::Enum { .. } => {
                Term::Int(Int::new_const(format!("input_{}", input.id)))
            }
        };
        environment.insert(format!("input.{}", input.id), term);
    }
    if let Err(message) = add_type_bounds(model, &environment, &solver) {
        return translation_error(message);
    }

    for derived_id in model.derived_order() {
        let Some(declaration) = model
            .raw()
            .derived
            .iter()
            .find(|item| item.id == *derived_id)
        else {
            return BackendAnswer::Error {
                code: "invalid_compiled_model".to_owned(),
                message: format!("derived declaration {derived_id} is missing"),
            };
        };
        let named = match &declaration.r#type {
            Type::Bool => Term::Bool(Bool::new_const(format!("derived_{}", declaration.id))),
            Type::Int { .. } | Type::Date | Type::Enum { .. } => {
                Term::Int(Int::new_const(format!("derived_{}", declaration.id)))
            }
        };
        let expression = match lower_expr(&declaration.expr, &environment, model) {
            Ok(value) => value,
            Err(message) => return translation_error(message),
        };
        if let Err(message) = assert_equal(&solver, &named, &expression) {
            return translation_error(message);
        }
        environment.insert(format!("derived.{}", declaration.id), named);
        if let Err(message) = add_term_type_bounds(
            model,
            &declaration.r#type,
            &environment,
            &solver,
            &format!("derived.{}", declaration.id),
        ) {
            return translation_error(message);
        }
    }

    for predicate in &model.raw().domain {
        match lower_bool(&predicate.expr, &environment, model) {
            Ok(term) => solver.assert(&term),
            Err(message) => return translation_error(message),
        }
    }
    for assumption_id in assumption_ids {
        let Some(assumption) = model
            .raw()
            .assumptions
            .iter()
            .find(|item| item.id == *assumption_id)
        else {
            return BackendAnswer::Error {
                code: "unknown_assumption".to_owned(),
                message: format!("selected assumption {assumption_id} does not exist"),
            };
        };
        match lower_bool(&assumption.expr, &environment, model) {
            Ok(term) => solver.assert(&term),
            Err(message) => return translation_error(message),
        }
    }

    let query = match obligation {
        Obligation::DomainFeasibility => Bool::from_bool(true),
        Obligation::RuleReachability { rule_id } => {
            let Some(rule) = model.raw().rules.iter().find(|item| item.id == *rule_id) else {
                return missing_declaration("rule", rule_id);
            };
            match lower_bool(&rule.when, &environment, model) {
                Ok(guard) => guard,
                Err(message) => return translation_error(message),
            }
        }
        Obligation::Coverage => {
            let guards = model
                .raw()
                .rules
                .iter()
                .map(|rule| lower_bool(&rule.when, &environment, model))
                .collect::<Result<Vec<_>, _>>();
            let guards = match guards {
                Ok(guards) => guards,
                Err(message) => return translation_error(message),
            };
            let terms = guards.iter().collect::<Vec<_>>();
            Bool::or(&terms).not()
        }
        Obligation::OutcomeConflict {
            left_rule_id,
            right_rule_id,
        } => {
            let Some(left) = model
                .raw()
                .rules
                .iter()
                .find(|rule| rule.id == *left_rule_id)
            else {
                return missing_declaration("rule", left_rule_id);
            };
            let Some(right) = model
                .raw()
                .rules
                .iter()
                .find(|rule| rule.id == *right_rule_id)
            else {
                return missing_declaration("rule", right_rule_id);
            };
            let left_guard = match lower_bool(&left.when, &environment, model) {
                Ok(guard) => guard,
                Err(message) => return translation_error(message),
            };
            let right_guard = match lower_bool(&right.when, &environment, model) {
                Ok(guard) => guard,
                Err(message) => return translation_error(message),
            };
            let different = match outcomes_differ(left, right, &environment, model) {
                Ok(value) => value,
                Err(message) => return translation_error(message),
            };
            Bool::and(&[&left_guard, &right_guard, &different])
        }
        Obligation::ConstraintCompliance {
            rule_id,
            constraint_id,
        } => {
            let Some(rule) = model.raw().rules.iter().find(|item| item.id == *rule_id) else {
                return missing_declaration("rule", rule_id);
            };
            let Some(constraint) = model
                .raw()
                .constraints
                .iter()
                .find(|item| item.id == *constraint_id)
            else {
                return missing_declaration("constraint", constraint_id);
            };
            let guard = match lower_bool(&rule.when, &environment, model) {
                Ok(guard) => guard,
                Err(message) => return translation_error(message),
            };
            if constraint.for_outcome != rule.then.outcome {
                return BackendAnswer::Error {
                    code: "invalid_obligation".to_owned(),
                    message: "constraint does not target the queried rule's outcome".to_owned(),
                };
            }
            let fields = match lower_outcome_fields(rule, &environment, model) {
                Ok(fields) => fields,
                Err(message) => return translation_error(message),
            };
            let mut constraint_environment = environment.clone();
            for (field, value) in fields {
                constraint_environment.insert(format!("outcome.{field}"), value);
            }
            match lower_bool(&constraint.must, &constraint_environment, model) {
                Ok(requirement) => Bool::and(&[&guard, &requirement.not()]),
                Err(message) => return translation_error(message),
            }
        }
    };
    solver.assert(&query);

    let answer = match solver.check() {
        SatResult::Sat => solver.get_model().map_or_else(
            || BackendAnswer::Error {
                code: "model_unavailable".to_owned(),
                message: "Z3 reported SAT but returned no model".to_owned(),
            },
            |witness_model| match decode_inputs(model, &environment, &witness_model) {
                Ok(inputs) => BackendAnswer::Sat { inputs },
                Err(message) => translation_error(message),
            },
        ),
        SatResult::Unsat => BackendAnswer::Unsat,
        SatResult::Unknown => unknown_answer(solver.get_reason_unknown()),
    };
    let mut values = BTreeMap::new();
    values.insert(
        "assertion_count".to_owned(),
        solver.get_assertions().len().to_string(),
    );
    for entry in solver.get_statistics().entries() {
        let value = match entry.value {
            z3::StatisticsValue::UInt(value) => value.to_string(),
            z3::StatisticsValue::Double(value) => value.to_string(),
        };
        values.insert(entry.key, value);
    }
    *statistics = Some(SolverStatistics { values });
    answer
}

fn unknown_answer(reason: Option<String>) -> BackendAnswer {
    let reason = reason.unwrap_or_else(|| "solver returned unknown without a reason".to_owned());
    let normalized = reason.to_ascii_lowercase();
    if normalized.contains("timeout") {
        BackendAnswer::Timeout
    } else if normalized.contains("canceled") || normalized.contains("cancelled") {
        BackendAnswer::Cancelled
    } else {
        BackendAnswer::Unknown { reason }
    }
}

fn add_type_bounds(
    model: &CompiledModel,
    environment: &BTreeMap<String, Term>,
    solver: &Solver,
) -> Result<(), String> {
    for input in &model.raw().inputs {
        let key = format!("input.{}", input.id);
        add_term_type_bounds(model, &input.r#type, environment, solver, &key)?;
    }
    Ok(())
}

fn add_term_type_bounds(
    model: &CompiledModel,
    ty: &Type,
    environment: &BTreeMap<String, Term>,
    solver: &Solver,
    key: &str,
) -> Result<(), String> {
    let Some(term) = environment.get(key) else {
        return Err(format!("missing symbolic term {key}"));
    };
    let int = match (ty, term) {
        (Type::Bool, Term::Bool(_)) => return Ok(()),
        (Type::Bool, Term::Int(_)) => return Err(format!("expected Boolean term for {key}")),
        (Type::Int { .. } | Type::Date | Type::Enum { .. }, Term::Int(int)) => int,
        (Type::Int { .. } | Type::Date | Type::Enum { .. }, Term::Bool(_)) => {
            return Err(format!("expected integer term for {key}"));
        }
    };
    match ty {
        Type::Bool | Type::Int { .. } => Ok(()),
        Type::Date => {
            solver.assert(int.ge(Int::from_i64(0)));
            solver.assert(int.le(Int::from_i64(DATE_MAX_ORDINAL)));
            Ok(())
        }
        Type::Enum { name } => {
            let Some(declaration) = model.raw().enums.iter().find(|item| item.id == *name) else {
                return Err(format!("unknown enum type {name}"));
            };
            let choices = declaration
                .variants
                .iter()
                .enumerate()
                .filter_map(|(index, _)| {
                    u64::try_from(index)
                        .ok()
                        .map(|ordinal| int.eq(Int::from_u64(ordinal)))
                })
                .collect::<Vec<_>>();
            let refs = choices.iter().collect::<Vec<_>>();
            solver.assert(Bool::or(&refs));
            Ok(())
        }
    }
}

#[allow(clippy::too_many_lines)] // Keep exhaustive AST lowering in one auditable match.
fn lower_expr(
    expr: &Expr,
    environment: &BTreeMap<String, Term>,
    model: &CompiledModel,
) -> Result<Term, String> {
    let lower = |child: &Expr| lower_expr(child, environment, model);
    let lower_integer = |child: &Expr| match lower(child)? {
        Term::Int(value) => Ok(value),
        Term::Bool(_) => Err("integer operator received a Boolean term".to_owned()),
    };
    let lower_boolean = |child: &Expr| match lower(child)? {
        Term::Bool(value) => Ok(value),
        Term::Int(_) => Err("Boolean operator received an integer term".to_owned()),
    };
    match expr {
        Expr::Bool { value } => Ok(Term::Bool(Bool::from_bool(*value))),
        Expr::Int { value, .. } => Ok(Term::Int(parse_integer(value)?)),
        Expr::Date { value } => Ok(Term::Int(Int::from_i64(date_ordinal(value)?))),
        Expr::Enum { enum_name, variant } => {
            let declaration = model
                .raw()
                .enums
                .iter()
                .find(|item| item.id == *enum_name)
                .ok_or_else(|| format!("unknown enum {enum_name}"))?;
            let index = declaration
                .variants
                .iter()
                .position(|item| item == variant)
                .ok_or_else(|| format!("unknown variant {enum_name}.{variant}"))?;
            Ok(Term::Int(Int::from_u64(
                u64::try_from(index).map_err(|_| "enum ordinal overflow".to_owned())?,
            )))
        }
        Expr::Ref { path } => environment
            .get(path)
            .cloned()
            .ok_or_else(|| format!("unbound expression reference {path}")),
        Expr::Not { arg } => Ok(Term::Bool(lower_boolean(arg)?.not())),
        Expr::And { args } => {
            let terms = args
                .iter()
                .map(&lower_boolean)
                .collect::<Result<Vec<_>, _>>()?;
            let refs = terms.iter().collect::<Vec<_>>();
            Ok(Term::Bool(Bool::and(&refs)))
        }
        Expr::Or { args } => {
            let terms = args
                .iter()
                .map(&lower_boolean)
                .collect::<Result<Vec<_>, _>>()?;
            let refs = terms.iter().collect::<Vec<_>>();
            Ok(Term::Bool(Bool::or(&refs)))
        }
        Expr::Eq { left, right } => {
            compare_terms(lower(left)?, lower(right)?, |a, b| a.eq(b), |a, b| a.eq(b))
        }
        Expr::Ne { left, right } => compare_terms(
            lower(left)?,
            lower(right)?,
            |a, b| a.eq(b).not(),
            |a, b| a.eq(b).not(),
        ),
        Expr::Lt { left, right } => Ok(Term::Bool(compare_integer(
            &lower_integer(left)?,
            &lower_integer(right)?,
            |a, b| a.lt(b),
        ))),
        Expr::Le { left, right } => Ok(Term::Bool(compare_integer(
            &lower_integer(left)?,
            &lower_integer(right)?,
            |a, b| a.le(b),
        ))),
        Expr::Gt { left, right } => Ok(Term::Bool(compare_integer(
            &lower_integer(left)?,
            &lower_integer(right)?,
            |a, b| a.gt(b),
        ))),
        Expr::Ge { left, right } => Ok(Term::Bool(compare_integer(
            &lower_integer(left)?,
            &lower_integer(right)?,
            |a, b| a.ge(b),
        ))),
        Expr::Add { left, right } => Ok(Term::Int(Int::add(&[
            &lower_integer(left)?,
            &lower_integer(right)?,
        ]))),
        Expr::Sub { left, right } => Ok(Term::Int(Int::sub(&[
            &lower_integer(left)?,
            &lower_integer(right)?,
        ]))),
        Expr::Scale { factor, arg } => Ok(Term::Int(Int::mul(&[
            &parse_integer(factor)?,
            &lower_integer(arg)?,
        ]))),
        Expr::If {
            condition,
            then_expr,
            else_expr,
        } => {
            let condition = lower_boolean(condition)?;
            let then_term = lower(then_expr)?;
            let else_term = lower(else_expr)?;
            match (then_term, else_term) {
                (Term::Bool(left), Term::Bool(right)) => {
                    Ok(Term::Bool(condition.ite(&left, &right)))
                }
                (Term::Int(left), Term::Int(right)) => Ok(Term::Int(condition.ite(&left, &right))),
                _ => Err("conditional branches have different symbolic sorts".to_owned()),
            }
        }
    }
}

fn compare_terms(
    left: Term,
    right: Term,
    bool_compare: impl FnOnce(&Bool, &Bool) -> Bool,
    int_compare: impl FnOnce(&Int, &Int) -> Bool,
) -> Result<Term, String> {
    match (left, right) {
        (Term::Bool(left), Term::Bool(right)) => Ok(Term::Bool(bool_compare(&left, &right))),
        (Term::Int(left), Term::Int(right)) => Ok(Term::Bool(int_compare(&left, &right))),
        _ => Err("equality operands have different symbolic sorts".to_owned()),
    }
}

fn compare_integer(left: &Int, right: &Int, compare: impl FnOnce(&Int, &Int) -> Bool) -> Bool {
    compare(left, right)
}

fn lower_bool(
    expr: &Expr,
    environment: &BTreeMap<String, Term>,
    model: &CompiledModel,
) -> Result<Bool, String> {
    match lower_expr(expr, environment, model)? {
        Term::Bool(value) => Ok(value),
        Term::Int(_) => Err("expected Boolean expression".to_owned()),
    }
}

fn lower_outcome_fields(
    rule: &author_model::Rule,
    environment: &BTreeMap<String, Term>,
    model: &CompiledModel,
) -> Result<BTreeMap<String, Term>, String> {
    rule.then
        .fields
        .iter()
        .map(|(field, expr)| Ok((field.clone(), lower_expr(expr, environment, model)?)))
        .collect()
}

fn outcomes_differ(
    left: &author_model::Rule,
    right: &author_model::Rule,
    environment: &BTreeMap<String, Term>,
    model: &CompiledModel,
) -> Result<Bool, String> {
    if left.then.outcome != right.then.outcome {
        return Ok(Bool::from_bool(true));
    }
    let left_fields = lower_outcome_fields(left, environment, model)?;
    let right_fields = lower_outcome_fields(right, environment, model)?;
    let mut unequal = vec![];
    for (field, left_value) in &left_fields {
        let Some(right_value) = right_fields.get(field) else {
            return Err(format!("outcome field {field} is missing from a rule"));
        };
        let same = match (left_value, right_value) {
            (Term::Bool(a), Term::Bool(b)) => a.eq(b),
            (Term::Int(a), Term::Int(b)) => a.eq(b),
            _ => return Err(format!("outcome field {field} has incompatible sorts")),
        };
        unequal.push(same.not());
    }
    let refs = unequal.iter().collect::<Vec<_>>();
    Ok(Bool::or(&refs))
}

fn assert_equal(solver: &Solver, left: &Term, right: &Term) -> Result<(), String> {
    match (left, right) {
        (Term::Bool(left), Term::Bool(right)) => solver.assert(left.eq(right)),
        (Term::Int(left), Term::Int(right)) => solver.assert(left.eq(right)),
        _ => return Err("derived expression has a different sort from its declaration".to_owned()),
    }
    Ok(())
}

fn decode_inputs(
    model: &CompiledModel,
    environment: &BTreeMap<String, Term>,
    witness: &z3::Model,
) -> Result<InputValues, String> {
    let mut values = InputValues::new();
    for input in &model.raw().inputs {
        let key = format!("input.{}", input.id);
        let Some(term) = environment.get(&key) else {
            return Err(format!("missing symbolic input {key}"));
        };
        let value = match (&input.r#type, term) {
            (Type::Bool, Term::Bool(term)) => {
                let evaluated = witness
                    .eval(term, true)
                    .ok_or_else(|| format!("could not complete input {key}"))?;
                Value::Bool {
                    value: evaluated
                        .as_bool()
                        .ok_or_else(|| format!("could not decode Boolean input {key}"))?,
                }
            }
            (Type::Int { unit }, Term::Int(term)) => {
                let evaluated = witness
                    .eval(term, true)
                    .ok_or_else(|| format!("could not complete input {key}"))?;
                Value::Int {
                    value: integer_string(&evaluated.to_string())?,
                    unit: unit.clone(),
                }
            }
            (Type::Date, Term::Int(term)) => {
                let evaluated = witness
                    .eval(term, true)
                    .ok_or_else(|| format!("could not complete input {key}"))?;
                let ordinal = integer_string(&evaluated.to_string())?
                    .parse::<i64>()
                    .map_err(|_| format!("invalid date ordinal for {key}"))?;
                Value::Date {
                    value: date_from_ordinal(ordinal)?,
                }
            }
            (Type::Enum { name }, Term::Int(term)) => {
                let evaluated = witness
                    .eval(term, true)
                    .ok_or_else(|| format!("could not complete input {key}"))?;
                let ordinal = integer_string(&evaluated.to_string())?
                    .parse::<usize>()
                    .map_err(|_| format!("invalid enum ordinal for {key}"))?;
                let declaration = model
                    .raw()
                    .enums
                    .iter()
                    .find(|item| item.id == *name)
                    .ok_or_else(|| format!("unknown enum type {name}"))?;
                let variant = declaration
                    .variants
                    .get(ordinal)
                    .ok_or_else(|| format!("enum ordinal out of range for {key}"))?;
                Value::Enum {
                    enum_name: name.clone(),
                    variant: variant.clone(),
                }
            }
            _ => return Err(format!("symbolic sort does not match input type for {key}")),
        };
        values.insert(input.id.clone(), value);
    }
    Ok(values)
}

fn integer_string(value: &str) -> Result<String, String> {
    if let Some(inner) = value
        .strip_prefix("(- ")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        return Ok(format!("-{inner}"));
    }
    if value.bytes().all(|byte| byte.is_ascii_digit())
        || value
            .strip_prefix('-')
            .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Ok(value.to_owned());
    }
    Err(format!("Z3 returned a non-numeral integer {value}"))
}

fn parse_integer(value: &str) -> Result<Int, String> {
    Int::from_str(value).map_err(|()| format!("invalid exact integer {value}"))
}

#[allow(clippy::arithmetic_side_effects)]
fn date_ordinal(value: &str) -> Result<i64, String> {
    let mut parts = value.split('-');
    let year = parts
        .next()
        .ok_or_else(|| "invalid canonical date".to_owned())?
        .parse::<i64>()
        .map_err(|_| "invalid date year".to_owned())?;
    let month = parts
        .next()
        .ok_or_else(|| "invalid canonical date".to_owned())?
        .parse::<usize>()
        .map_err(|_| "invalid date month".to_owned())?;
    let day = parts
        .next()
        .ok_or_else(|| "invalid canonical date".to_owned())?
        .parse::<i64>()
        .map_err(|_| "invalid date day".to_owned())?;
    if parts.next().is_some() || !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
        return Err(format!("invalid canonical Gregorian date {value}"));
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = [
        31_i64,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let max_day = month_days
        .get(month - 1)
        .copied()
        .ok_or_else(|| "invalid date month".to_owned())?;
    if !(1..=max_day).contains(&day) {
        return Err(format!("invalid canonical Gregorian date {value}"));
    }
    let prior_year = year - 1;
    let prior_days = prior_year * 365 + prior_year / 4 - prior_year / 100 + prior_year / 400;
    let prior_month_days = month_days.iter().take(month - 1).sum::<i64>();
    Ok(prior_days + prior_month_days + day - 1)
}

#[allow(clippy::arithmetic_side_effects)]
fn date_from_ordinal(ordinal: i64) -> Result<String, String> {
    if !(0..=DATE_MAX_ORDINAL).contains(&ordinal) {
        return Err(format!(
            "date ordinal {ordinal} is outside supported bounds"
        ));
    }
    let mut low = 1_i64;
    let mut high = 9999_i64;
    while low < high {
        let middle = low + (high - low + 1) / 2;
        let start = date_ordinal(&format!("{middle:04}-01-01"))?;
        if start <= ordinal {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let mut day_of_year = ordinal - date_ordinal(&format!("{low:04}-01-01"))?;
    let leap = low % 4 == 0 && (low % 100 != 0 || low % 400 == 0);
    let month_days = [
        31_i64,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    for (index, days) in month_days.iter().enumerate() {
        if day_of_year < *days {
            return Ok(format!("{low:04}-{:02}-{:02}", index + 1, day_of_year + 1));
        }
        day_of_year -= *days;
    }
    Err(format!("invalid day ordinal {ordinal}"))
}

fn translation_error(message: String) -> BackendAnswer {
    BackendAnswer::Error {
        code: "translation_error".to_owned(),
        message,
    }
}

fn missing_declaration(kind: &str, id: &str) -> BackendAnswer {
    BackendAnswer::Error {
        code: "invalid_obligation".to_owned(),
        message: format!("{kind} declaration {id} does not exist"),
    }
}

#[cfg(test)]
mod tests {
    use super::{DATE_MAX_ORDINAL, date_from_ordinal, date_ordinal, unknown_answer};
    use author_analysis::BackendAnswer;
    use z3::{SatResult, Solver, ast::Bool};

    #[test]
    fn native_solver_reports_sat_and_unsat() {
        let proposition = Bool::new_const("smoke_proposition");

        let sat_solver = Solver::new();
        sat_solver.assert(&proposition);
        assert_eq!(sat_solver.check(), SatResult::Sat);

        let unsat_solver = Solver::new();
        unsat_solver.assert(&proposition);
        unsat_solver.assert(proposition.not());
        assert_eq!(unsat_solver.check(), SatResult::Unsat);
    }

    #[test]
    fn z3_inconclusive_reasons_remain_distinct() {
        assert_eq!(
            unknown_answer(Some("timeout".to_owned())),
            BackendAnswer::Timeout
        );
        assert_eq!(
            unknown_answer(Some("canceled".to_owned())),
            BackendAnswer::Cancelled
        );
        assert_eq!(
            unknown_answer(Some("max. resource limit exceeded".to_owned())),
            BackendAnswer::Unknown {
                reason: "max. resource limit exceeded".to_owned()
            }
        );
    }

    #[test]
    fn gregorian_date_ordinals_round_trip_at_boundaries() {
        for value in [
            "0001-01-01",
            "0004-02-29",
            "1900-03-01",
            "2000-02-29",
            "9999-12-31",
        ] {
            assert_eq!(
                date_ordinal(value).map(|ordinal| date_from_ordinal(ordinal).ok()),
                Ok(Some(value.to_owned()))
            );
        }
        assert!(date_ordinal("1900-02-29").is_err());
        assert!(date_ordinal("0000-01-01").is_err());
        assert!(date_ordinal("10000-01-01").is_err());
        assert_eq!(date_ordinal("9999-12-31"), Ok(DATE_MAX_ORDINAL));
    }
}
