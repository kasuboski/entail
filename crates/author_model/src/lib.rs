use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, NaiveDate};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawModel {
    pub language_version: String,
    pub model_id: String,
    pub sources: Vec<Source>,
    pub enums: Vec<EnumDecl>,
    pub inputs: Vec<InputDecl>,
    pub derived: Vec<DerivedDecl>,
    pub outcomes: Vec<OutcomeDecl>,
    pub domain: Vec<Predicate>,
    pub assumptions: Vec<Predicate>,
    pub rules: Vec<Rule>,
    pub constraints: Vec<Constraint>,
    pub open_items: Vec<OpenItem>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    pub text: String,
    pub locator: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub kind: OriginKind,
    pub source_ids: Vec<String>,
    pub review: Review,
    pub note: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OriginKind {
    Source,
    Proposal,
    Assumption,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Review {
    Proposed,
    Confirmed,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnumDecl {
    pub id: String,
    pub variants: Vec<String>,
    pub origin: Origin,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputDecl {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: Type,
    pub origin: Origin,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedDecl {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: Type,
    pub expr: Expr,
    pub origin: Origin,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldDecl {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: Type,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeDecl {
    pub id: String,
    pub fields: Vec<FieldDecl>,
    pub origin: Origin,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Predicate {
    pub id: String,
    pub expr: Expr,
    pub origin: Origin,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub when: Expr,
    pub then: RuleConclusion,
    pub origin: Origin,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleConclusion {
    pub outcome: String,
    pub fields: BTreeMap<String, Expr>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constraint {
    pub id: String,
    pub for_outcome: String,
    pub must: Expr,
    pub origin: Origin,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenItem {
    pub id: String,
    pub text: String,
    pub source_ids: Vec<String>,
    pub affects: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Type {
    #[serde(rename = "bool")]
    Bool,
    #[serde(rename = "enum")]
    Enum { name: String },
    #[serde(rename = "int")]
    Int { unit: String },
    #[serde(rename = "date")]
    Date,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Expr {
    #[serde(rename = "bool")]
    Bool { value: bool },
    #[serde(rename = "int")]
    Int { value: String, unit: String },
    #[serde(rename = "date")]
    Date { value: String },
    #[serde(rename = "enum")]
    Enum {
        #[serde(rename = "enum")]
        enum_name: String,
        variant: String,
    },
    #[serde(rename = "ref")]
    Ref { path: String },
    #[serde(rename = "not")]
    Not { arg: Box<Self> },
    #[serde(rename = "and")]
    And { args: Vec<Self> },
    #[serde(rename = "or")]
    Or { args: Vec<Self> },
    #[serde(rename = "eq")]
    Eq { left: Box<Self>, right: Box<Self> },
    #[serde(rename = "ne")]
    Ne { left: Box<Self>, right: Box<Self> },
    #[serde(rename = "lt")]
    Lt { left: Box<Self>, right: Box<Self> },
    #[serde(rename = "le")]
    Le { left: Box<Self>, right: Box<Self> },
    #[serde(rename = "gt")]
    Gt { left: Box<Self>, right: Box<Self> },
    #[serde(rename = "ge")]
    Ge { left: Box<Self>, right: Box<Self> },
    #[serde(rename = "add")]
    Add { left: Box<Self>, right: Box<Self> },
    #[serde(rename = "sub")]
    Sub { left: Box<Self>, right: Box<Self> },
    #[serde(rename = "scale")]
    Scale { factor: String, arg: Box<Self> },
    #[serde(rename = "if")]
    If {
        condition: Box<Self>,
        #[serde(rename = "then")]
        then_expr: Box<Self>,
        #[serde(rename = "else")]
        else_expr: Box<Self>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Value {
    #[serde(rename = "bool")]
    Bool { value: bool },
    #[serde(rename = "int")]
    Int { value: String, unit: String },
    #[serde(rename = "date")]
    Date { value: String },
    #[serde(rename = "enum")]
    Enum {
        #[serde(rename = "enum")]
        enum_name: String,
        variant: String,
    },
}
pub type InputValues = BTreeMap<String, Value>;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutcomeValue {
    pub outcome: String,
    pub fields: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub pointer: Option<String>,
    pub declaration_id: Option<String>,
    pub source_ids: Vec<String>,
}
impl Diagnostic {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            pointer: None,
            declaration_id: None,
            source_ids: vec![],
        }
    }
}
#[derive(Clone, Debug)]
pub struct ModelLimits {
    pub inputs: usize,
    pub rules: usize,
    pub constraints: usize,
    pub expression_nodes: usize,
    pub expression_depth: usize,
}
impl Default for ModelLimits {
    fn default() -> Self {
        Self {
            inputs: 256,
            rules: 2048,
            constraints: 2048,
            expression_nodes: 50_000,
            expression_depth: 128,
        }
    }
}
/// A validated, immutable model snapshot produced by [`compile_json`].
///
/// Its raw declarations and hashes cannot be changed independently, preserving the
/// compiler's semantic validation and cache identity guarantees.
///
/// ```compile_fail
/// # use author_model::{compile_json, CompiledModel, ModelLimits};
/// # let input = serde_json::json!({"model_id":"example"});
/// # let mut model: CompiledModel = compile_json(&input, &ModelLimits::default()).unwrap();
/// model.raw().domain[0].expr = serde_json::json!(true);
/// ```
#[derive(Clone, Debug)]
pub struct CompiledModel {
    raw: RawModel,
    derived_order: Vec<String>,
    content_hash: String,
    semantic_hash: String,
}

impl CompiledModel {
    /// The validated source model. This is an immutable reference, not an editing API.
    #[must_use]
    pub const fn raw(&self) -> &RawModel {
        &self.raw
    }

    /// Compiler-derived topological order of derived declarations.
    #[must_use]
    pub fn derived_order(&self) -> &[String] {
        &self.derived_order
    }

    /// Hash of the submitted JSON content, including non-semantic metadata.
    #[must_use]
    pub fn content_hash(&self) -> &str {
        &self.content_hash
    }

    /// Hash of the model's normalized semantics.
    #[must_use]
    pub fn semantic_hash(&self) -> &str {
        &self.semantic_hash
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evaluation {
    pub classification: String,
    pub matched_rule_ids: Vec<String>,
    pub outcomes: Vec<OutcomeValue>,
    pub violated_constraint_ids: Vec<String>,
    pub derived: BTreeMap<String, Value>,
    pub expectation: Option<String>,
}

fn error(code: &str, msg: impl Into<String>) -> Diagnostic {
    Diagnostic::new(code, msg)
}
fn valid_identifier(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
fn canonical(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(m) => {
            let mut b = BTreeMap::new();
            for (k, x) in m {
                b.insert(k.clone(), canonical(x));
            }
            serde_json::to_value(b).unwrap_or(serde_json::Value::Null)
        }
        serde_json::Value::Array(a) => serde_json::Value::Array(a.iter().map(canonical).collect()),
        x => x.clone(),
    }
}
fn hash(v: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(&canonical(v)).unwrap_or_default();
    format!("{:x}", Sha256::digest(bytes))
}
fn canonical_integer(value: &str) -> bool {
    if value == "0" {
        return true;
    }
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty()
        && !digits.starts_with('0')
        && digits.bytes().all(|byte| byte.is_ascii_digit())
}
fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .filter(|d| (1..=9999).contains(&d.year()))
        .filter(|d| d.format("%Y-%m-%d").to_string() == s)
}
fn expr_count(e: &Expr, depth: usize, count: &mut usize, maxdepth: usize) -> bool {
    *count = count.saturating_add(1);
    if depth > maxdepth {
        return false;
    }
    let child_depth = depth.saturating_add(1);
    let child = |x: &Expr, c: &mut usize| expr_count(x, child_depth, c, maxdepth);
    match e {
        Expr::Not { arg } | Expr::Scale { arg, .. } => child(arg, count),
        Expr::And { args } | Expr::Or { args } => args.iter().all(|x| child(x, count)),
        Expr::Eq { left, right }
        | Expr::Ne { left, right }
        | Expr::Lt { left, right }
        | Expr::Le { left, right }
        | Expr::Gt { left, right }
        | Expr::Ge { left, right }
        | Expr::Add { left, right }
        | Expr::Sub { left, right } => child(left, count) && child(right, count),
        Expr::If {
            condition,
            then_expr,
            else_expr,
        } => child(condition, count) && child(then_expr, count) && child(else_expr, count),
        _ => true,
    }
}
fn refs(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Ref { path } => out.push(path.clone()),
        Expr::Not { arg } | Expr::Scale { arg, .. } => refs(arg, out),
        Expr::And { args } | Expr::Or { args } => {
            for x in args {
                refs(x, out);
            }
        }
        Expr::Eq { left, right }
        | Expr::Ne { left, right }
        | Expr::Lt { left, right }
        | Expr::Le { left, right }
        | Expr::Gt { left, right }
        | Expr::Ge { left, right }
        | Expr::Add { left, right }
        | Expr::Sub { left, right } => {
            refs(left, out);
            refs(right, out);
        }
        Expr::If {
            condition,
            then_expr,
            else_expr,
        } => {
            refs(condition, out);
            refs(then_expr, out);
            refs(else_expr, out);
        }
        _ => {}
    }
}
fn check_refs(
    e: &Expr,
    raw: &RawModel,
    allow_outcome: bool,
    ds: &mut Vec<Diagnostic>,
    declaration_id: &str,
    pointer: &str,
    source_ids: &[String],
) {
    let mut paths = vec![];
    refs(e, &mut paths);
    for path in paths {
        let mut p = path.splitn(2, '.');
        let ns = p.next().unwrap_or("");
        let id = p.next().unwrap_or("");
        let valid = match ns {
            "input" => raw.inputs.iter().any(|x| x.id == id),
            "derived" => raw.derived.iter().any(|x| x.id == id),
            "outcome" => {
                allow_outcome
                    && raw
                        .outcomes
                        .iter()
                        .any(|o| o.fields.iter().any(|f| f.id == id))
            }
            _ => false,
        };
        if !valid {
            let mut diagnostic = error(
                "unknown_reference",
                format!("unknown or out-of-scope reference {path}"),
            );
            diagnostic.declaration_id = Some(declaration_id.to_string());
            diagnostic.pointer = Some(pointer.to_string());
            diagnostic.source_ids = source_ids.to_vec();
            ds.push(diagnostic);
        }
    }
}
// Type inference is intentionally one exhaustive dispatch over the public expression variants.
#[allow(clippy::too_many_lines)]
fn infer(
    expression: &Expr,
    raw: &RawModel,
    allow_outcome: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    let child = |node: &Expr, diagnostics: &mut Vec<Diagnostic>| {
        infer(node, raw, allow_outcome, diagnostics)
    };
    match expression {
        Expr::Bool { .. } => Some(Type::Bool),
        Expr::Int { value, unit } => {
            if !valid_identifier(unit) {
                diagnostics.push(error("schema_error", format!("invalid unit {unit}")));
            }
            if !canonical_integer(value) {
                diagnostics.push(error(
                    "type_mismatch",
                    "integer literal must be canonical decimal",
                ));
            }
            Some(Type::Int { unit: unit.clone() })
        }
        Expr::Date { value } => {
            if parse_date(value).is_none() {
                diagnostics.push(error(
                    "invalid_date",
                    format!("invalid date literal {value}"),
                ));
            }
            Some(Type::Date)
        }
        Expr::Enum { enum_name, variant } => {
            if !valid_identifier(enum_name) || !valid_identifier(variant) {
                diagnostics.push(error("schema_error", "invalid enum literal identifier"));
            }
            if !raw
                .enums
                .iter()
                .any(|x| x.id == *enum_name && x.variants.contains(variant))
            {
                diagnostics.push(error(
                    "unknown_variant",
                    format!("unknown variant {enum_name}.{variant}"),
                ));
            }
            Some(Type::Enum {
                name: enum_name.clone(),
            })
        }
        Expr::Ref { path } => {
            let mut p = path.splitn(2, '.');
            let ns = p.next().unwrap_or("");
            let id = p.next().unwrap_or("");
            match ns {
                "input" => raw
                    .inputs
                    .iter()
                    .find(|x| x.id == id)
                    .map(|x| x.r#type.clone()),
                "derived" => raw
                    .derived
                    .iter()
                    .find(|x| x.id == id)
                    .map(|x| x.r#type.clone()),
                "outcome" if allow_outcome => raw
                    .outcomes
                    .iter()
                    .flat_map(|x| x.fields.iter())
                    .find(|x| x.id == id)
                    .map(|x| x.r#type.clone()),
                _ => None,
            }
        }
        Expr::Not { arg } => {
            if child(arg, diagnostics) != Some(Type::Bool) {
                diagnostics.push(error("type_mismatch", "not requires bool"));
            }
            Some(Type::Bool)
        }
        Expr::And { args } | Expr::Or { args } => {
            if args.is_empty() {
                diagnostics.push(error(
                    "type_mismatch",
                    "and/or requires at least one argument",
                ));
            }
            for x in args {
                if child(x, diagnostics) != Some(Type::Bool) {
                    diagnostics.push(error("type_mismatch", "and/or arguments must be bool"));
                }
            }
            Some(Type::Bool)
        }
        Expr::Eq { left, right }
        | Expr::Ne { left, right }
        | Expr::Lt { left, right }
        | Expr::Le { left, right }
        | Expr::Gt { left, right }
        | Expr::Ge { left, right } => {
            let left_type = child(left, diagnostics);
            let right_type = child(right, diagnostics);
            if left_type != right_type {
                if matches!((&left_type,&right_type),(Some(Type::Int{unit:left_unit}),Some(Type::Int{unit:right_unit})) if left_unit!=right_unit)
                {
                    diagnostics.push(error("unit_mismatch", "integer units differ"));
                } else {
                    diagnostics.push(error("type_mismatch", "comparison operands differ in type"));
                }
            }
            if matches!(
                expression,
                Expr::Lt { .. } | Expr::Le { .. } | Expr::Gt { .. } | Expr::Ge { .. }
            ) && !matches!(left_type, Some(Type::Int { .. } | Type::Date))
            {
                diagnostics.push(error(
                    "type_mismatch",
                    "ordered comparison requires int or date",
                ));
            }
            Some(Type::Bool)
        }
        Expr::Add { left, right } | Expr::Sub { left, right } => {
            let left_type = child(left, diagnostics);
            let right_type = child(right, diagnostics);
            if left_type != right_type {
                if matches!((&left_type,&right_type),(Some(Type::Int{unit:left_unit}),Some(Type::Int{unit:right_unit})) if left_unit!=right_unit)
                {
                    diagnostics.push(error("unit_mismatch", "integer units differ"));
                } else {
                    diagnostics.push(error("type_mismatch", "arithmetic operands differ"));
                }
            }
            if !matches!(left_type, Some(Type::Int { .. })) {
                diagnostics.push(error("type_mismatch", "arithmetic requires integers"));
            }
            left_type
        }
        Expr::Scale { factor, arg } => {
            if !canonical_integer(factor) {
                diagnostics.push(error(
                    "type_mismatch",
                    "scale factor must be canonical integer",
                ));
            }
            let left_type = child(arg, diagnostics);
            if !matches!(left_type, Some(Type::Int { .. })) {
                diagnostics.push(error("type_mismatch", "scale requires integer"));
            }
            left_type
        }
        Expr::If {
            condition,
            then_expr,
            else_expr,
        } => {
            if child(condition, diagnostics) != Some(Type::Bool) {
                diagnostics.push(error("type_mismatch", "if condition must be bool"));
            }
            let left_type = child(then_expr, diagnostics);
            let right_type = child(else_expr, diagnostics);
            if left_type != right_type {
                diagnostics.push(error("type_mismatch", "if branches differ"));
            }
            left_type
        }
    }
}

fn infer_in_context(
    expression: &Expr,
    raw: &RawModel,
    allow_outcome: bool,
    diagnostics: &mut Vec<Diagnostic>,
    declaration_id: &str,
    pointer: &str,
    source_ids: &[String],
) -> Option<Type> {
    let start = diagnostics.len();
    let inferred = infer(expression, raw, allow_outcome, diagnostics);
    if let Some(new_diagnostics) = diagnostics.get_mut(start..) {
        for diagnostic in new_diagnostics {
            diagnostic.declaration_id = Some(declaration_id.to_string());
            diagnostic.pointer = Some(pointer.to_string());
            diagnostic.source_ids = source_ids.to_vec();
        }
    }
    inferred
}

fn check_declaration_limits(
    root: &serde_json::Map<String, serde_json::Value>,
    limits: &ModelLimits,
) -> Result<(), Diagnostic> {
    for (key, limit) in [
        ("inputs", limits.inputs),
        ("derived", limits.inputs),
        ("enums", limits.inputs),
        ("outcomes", limits.inputs),
        ("rules", limits.rules),
        ("domain", limits.rules),
        ("assumptions", limits.rules),
        ("open_items", limits.rules),
        ("constraints", limits.constraints),
    ] {
        if root
            .get(key)
            .and_then(serde_json::Value::as_array)
            .is_some_and(|items| items.len() > limit)
        {
            return Err(error(
                "resource_limit",
                format!("{key} declaration limit exceeded"),
            ));
        }
    }
    Ok(())
}

fn add_unique_name(id: &str, names: &mut BTreeSet<String>, diagnostics: &mut Vec<Diagnostic>) {
    if !names.insert(id.to_string()) {
        diagnostics.push(error(
            "duplicate_name",
            format!("duplicate declaration {id}"),
        ));
    }
}

fn strip_semantic_metadata(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            object.remove("origin");
            object.remove("note");
            for child in object.values_mut() {
                strip_semantic_metadata(child);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                strip_semantic_metadata(item);
            }
        }
        _ => {}
    }
}

fn preflight(value: &serde_json::Value, limits: &ModelLimits) -> Result<(), Vec<Diagnostic>> {
    let Some(root) = value.as_object() else {
        return Err(vec![error("schema_error", "model must be an object")]);
    };
    check_declaration_limits(root, limits).map_err(|diagnostic| vec![diagnostic])?;
    let mut roots: Vec<&serde_json::Value> = Vec::new();
    for key in ["derived", "domain", "assumptions", "rules", "constraints"] {
        if let Some(items) = root.get(key).and_then(serde_json::Value::as_array) {
            for item in items {
                match key {
                    "derived" | "domain" | "assumptions" => {
                        if let Some(expr) = item.get("expr") {
                            roots.push(expr);
                        }
                    }
                    "rules" => {
                        if let Some(expr) = item.get("when") {
                            roots.push(expr);
                        }
                        if let Some(fields) = item
                            .get("then")
                            .and_then(|then| then.get("fields"))
                            .and_then(serde_json::Value::as_object)
                        {
                            roots.extend(fields.values());
                        }
                    }
                    "constraints" => {
                        if let Some(expr) = item.get("must") {
                            roots.push(expr);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let mut object_stack = vec![value];
    while let Some(node) = object_stack.pop() {
        match node {
            serde_json::Value::Object(object) => {
                for (key, child) in object {
                    if matches!(key.as_str(), "locator" | "note") && child.is_null() {
                        return Err(vec![error(
                            "schema_error",
                            format!("{key} must be omitted or a string"),
                        )]);
                    }
                    object_stack.push(child);
                }
            }
            serde_json::Value::Array(items) => object_stack.extend(items),
            _ => {}
        }
    }
    // Keep recursive serde construction and type inference bounded even if a caller
    // supplies a looser limit than the crate's supported safety ceiling.
    let max_depth = limits
        .expression_depth
        .min(ModelLimits::default().expression_depth);
    let mut count = 0usize;
    let mut stack: Vec<(&serde_json::Value, usize)> =
        roots.into_iter().map(|expr| (expr, 1)).collect();
    while let Some((expr, depth)) = stack.pop() {
        count = count.saturating_add(1);
        if count > limits.expression_nodes {
            return Err(vec![error(
                "resource_limit",
                "expression node limit exceeded",
            )]);
        }
        if depth > max_depth {
            return Err(vec![error(
                "resource_limit",
                "expression depth limit exceeded",
            )]);
        }
        if let Some(object) = expr.as_object() {
            for (key, child) in object {
                if matches!(
                    key.as_str(),
                    "arg" | "left" | "right" | "condition" | "then" | "else"
                ) {
                    stack.push((child, depth.saturating_add(1)));
                } else if key == "args" {
                    if let Some(args) = child.as_array() {
                        stack.extend(args.iter().map(|arg| (arg, depth.saturating_add(1))));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Validate and compile a JSON model into its executable representation.
///
/// # Errors
/// Returns diagnostics for schema, resource-limit, reference, type, or semantic errors.
// This remains cohesive as the compiler's ordered semantic-validation pipeline.
#[allow(clippy::too_many_lines)]
pub fn compile_json(
    value: &serde_json::Value,
    limits: &ModelLimits,
) -> Result<CompiledModel, Vec<Diagnostic>> {
    preflight(value, limits)?;
    let raw: RawModel = serde_json::from_value(value.clone())
        .map_err(|e| vec![error("schema_error", e.to_string())])?;
    let mut ds = vec![];
    if raw.language_version != "author-model/0.1" {
        ds.push(error("version_mismatch", "expected author-model/0.1"));
    }
    if raw.inputs.len() > limits.inputs
        || raw.rules.len() > limits.rules
        || raw.constraints.len() > limits.constraints
    {
        ds.push(error("resource_limit", "model declaration limit exceeded"));
    }
    let mut names = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let valid_id = |id: &str| {
        let mut chars = id.chars();
        chars.next().is_some_and(|c| c.is_ascii_alphabetic())
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    if !valid_id(&raw.model_id) {
        ds.push(error(
            "schema_error",
            format!("invalid model_id {}", raw.model_id),
        ));
    }
    for source in &raw.sources {
        if !valid_id(&source.id) || source.text.is_empty() {
            ds.push(error(
                "schema_error",
                format!("invalid source {}", source.id),
            ));
        }
    }
    for decl_id in raw
        .enums
        .iter()
        .map(|x| &x.id)
        .chain(raw.inputs.iter().map(|x| &x.id))
        .chain(raw.derived.iter().map(|x| &x.id))
        .chain(raw.outcomes.iter().map(|x| &x.id))
        .chain(raw.domain.iter().map(|x| &x.id))
        .chain(raw.assumptions.iter().map(|x| &x.id))
        .chain(raw.rules.iter().map(|x| &x.id))
        .chain(raw.constraints.iter().map(|x| &x.id))
        .chain(raw.open_items.iter().map(|x| &x.id))
    {
        if !valid_id(decl_id) {
            ds.push(error(
                "schema_error",
                format!("invalid identifier {decl_id}"),
            ));
        }
    }
    let declared_types = raw
        .inputs
        .iter()
        .map(|x| &x.r#type)
        .chain(raw.derived.iter().map(|x| &x.r#type))
        .chain(
            raw.outcomes
                .iter()
                .flat_map(|x| x.fields.iter().map(|field| &field.r#type)),
        );
    for ty in declared_types {
        match ty {
            Type::Enum { name } if !raw.enums.iter().any(|en| en.id == *name) => ds.push(error(
                "unknown_reference",
                format!("unknown enum type {name}"),
            )),
            Type::Int { unit } if !valid_id(unit) => {
                ds.push(error("schema_error", format!("invalid unit {unit}")));
            }
            _ => {}
        }
    }
    for en in &raw.enums {
        let unique: BTreeSet<_> = en.variants.iter().collect();
        if en.variants.is_empty()
            || unique.len() != en.variants.len()
            || en.variants.iter().any(|variant| !valid_id(variant))
        {
            ds.push(error(
                "schema_error",
                format!("invalid variants for enum {}", en.id),
            ));
        }
    }
    for outcome in &raw.outcomes {
        let fields: BTreeSet<_> = outcome.fields.iter().map(|field| &field.id).collect();
        if fields.len() != outcome.fields.len()
            || outcome.fields.iter().any(|field| !valid_id(&field.id))
        {
            ds.push(error(
                "schema_error",
                format!("invalid fields for outcome {}", outcome.id),
            ));
        }
    }
    for s in &raw.sources {
        if !sources.insert(s.id.clone()) {
            ds.push(error(
                "duplicate_name",
                format!("duplicate source {}", s.id),
            ));
        }
    }
    for id in raw
        .enums
        .iter()
        .map(|item| &item.id)
        .chain(raw.inputs.iter().map(|item| &item.id))
        .chain(raw.derived.iter().map(|item| &item.id))
        .chain(raw.outcomes.iter().map(|item| &item.id))
        .chain(raw.domain.iter().map(|item| &item.id))
        .chain(raw.assumptions.iter().map(|item| &item.id))
        .chain(raw.rules.iter().map(|item| &item.id))
        .chain(raw.constraints.iter().map(|item| &item.id))
        .chain(raw.open_items.iter().map(|item| &item.id))
    {
        add_unique_name(id, &mut names, &mut ds);
    }
    for (id, origin) in raw
        .enums
        .iter()
        .map(|x| (&x.id, &x.origin))
        .chain(raw.inputs.iter().map(|x| (&x.id, &x.origin)))
        .chain(raw.derived.iter().map(|x| (&x.id, &x.origin)))
        .chain(raw.outcomes.iter().map(|x| (&x.id, &x.origin)))
        .chain(raw.domain.iter().map(|x| (&x.id, &x.origin)))
        .chain(raw.assumptions.iter().map(|x| (&x.id, &x.origin)))
        .chain(raw.rules.iter().map(|x| (&x.id, &x.origin)))
        .chain(raw.constraints.iter().map(|x| (&x.id, &x.origin)))
    {
        if origin.kind == OriginKind::Source && origin.source_ids.is_empty() {
            let mut d = error(
                "source_reference_error",
                format!("{id} source origin requires a source"),
            );
            d.declaration_id = Some(id.clone());
            ds.push(d);
        }
        if origin.kind == OriginKind::Assumption && !raw.assumptions.iter().any(|x| x.id == *id) {
            ds.push(error(
                "source_reference_error",
                format!("{id}: assumption origin only allowed on assumptions"),
            ));
        }
        for sid in &origin.source_ids {
            if !sources.contains(sid) {
                ds.push(error(
                    "source_reference_error",
                    format!("{id} references missing source {sid}"),
                ));
            }
        }
    }
    for item in &raw.open_items {
        if item.text.is_empty()
            || !matches!(
                item.affects.as_str(),
                "input_domain"
                    | "outcome_schema"
                    | "rule_policy"
                    | "constraint"
                    | "lifecycle"
                    | "interpretation"
            )
        {
            ds.push(error(
                "schema_error",
                format!("invalid open item {}", item.id),
            ));
        }
        for source_id in &item.source_ids {
            if !sources.contains(source_id) {
                ds.push(error(
                    "source_reference_error",
                    format!("{} references missing source {source_id}", item.id),
                ));
            }
        }
    }
    for a in &raw.assumptions {
        if a.origin.kind != OriginKind::Assumption {
            ds.push(error(
                "source_reference_error",
                format!("assumption {} must use assumption origin", a.id),
            ));
        }
    }
    let mut counts = 0usize;
    let mut all = vec![];
    for d in &raw.derived {
        all.push(&d.expr);
    }
    for p in &raw.domain {
        all.push(&p.expr);
    }
    for p in &raw.assumptions {
        all.push(&p.expr);
    }
    for r in &raw.rules {
        all.push(&r.when);
        all.extend(r.then.fields.values());
    }
    for c in &raw.constraints {
        all.push(&c.must);
    }
    for e in all {
        if !expr_count(e, 1, &mut counts, limits.expression_depth) {
            ds.push(error("resource_limit", "expression depth limit exceeded"));
            break;
        }
    }
    if counts > limits.expression_nodes {
        ds.push(error("resource_limit", "expression node limit exceeded"));
    }
    for (index, derived) in raw.derived.iter().enumerate() {
        check_refs(
            &derived.expr,
            &raw,
            false,
            &mut ds,
            &derived.id,
            &format!("/derived/{index}/expr"),
            &derived.origin.source_ids,
        );
    }
    for (index, predicate) in raw.domain.iter().enumerate() {
        check_refs(
            &predicate.expr,
            &raw,
            false,
            &mut ds,
            &predicate.id,
            &format!("/domain/{index}/expr"),
            &predicate.origin.source_ids,
        );
    }
    for (index, predicate) in raw.assumptions.iter().enumerate() {
        check_refs(
            &predicate.expr,
            &raw,
            false,
            &mut ds,
            &predicate.id,
            &format!("/assumptions/{index}/expr"),
            &predicate.origin.source_ids,
        );
    }
    for (index, rule) in raw.rules.iter().enumerate() {
        check_refs(
            &rule.when,
            &raw,
            false,
            &mut ds,
            &rule.id,
            &format!("/rules/{index}/when"),
            &rule.origin.source_ids,
        );
        for (field, expression) in &rule.then.fields {
            check_refs(
                expression,
                &raw,
                false,
                &mut ds,
                &rule.id,
                &format!("/rules/{index}/then/fields/{field}"),
                &rule.origin.source_ids,
            );
        }
    }
    for (index, constraint) in raw.constraints.iter().enumerate() {
        check_refs(
            &constraint.must,
            &raw,
            true,
            &mut ds,
            &constraint.id,
            &format!("/constraints/{index}/must"),
            &constraint.origin.source_ids,
        );
    }
    for d in &raw.derived {
        if infer_in_context(
            &d.expr,
            &raw,
            false,
            &mut ds,
            &d.id,
            &format!(
                "/derived/{}/expr",
                raw.derived
                    .iter()
                    .position(|item| item.id == d.id)
                    .unwrap_or(0)
            ),
            &d.origin.source_ids,
        )
        .as_ref()
            != Some(&d.r#type)
        {
            ds.push(error(
                "type_mismatch",
                format!("derived {} expression has wrong type", d.id),
            ));
        }
    }
    for (pointer, p) in raw
        .domain
        .iter()
        .enumerate()
        .map(|(index, p)| (format!("/domain/{index}/expr"), p))
        .chain(
            raw.assumptions
                .iter()
                .enumerate()
                .map(|(index, p)| (format!("/assumptions/{index}/expr"), p)),
        )
    {
        if infer_in_context(
            &p.expr,
            &raw,
            false,
            &mut ds,
            &p.id,
            &pointer,
            &p.origin.source_ids,
        ) != Some(Type::Bool)
        {
            ds.push(error(
                "type_mismatch",
                format!("predicate {} must be boolean", p.id),
            ));
        }
    }
    for r in &raw.rules {
        let rule_index = raw
            .rules
            .iter()
            .position(|item| item.id == r.id)
            .unwrap_or(0);
        if infer_in_context(
            &r.when,
            &raw,
            false,
            &mut ds,
            &r.id,
            &format!("/rules/{rule_index}/when"),
            &r.origin.source_ids,
        ) != Some(Type::Bool)
        {
            ds.push(error(
                "type_mismatch",
                format!("rule {} guard must be boolean", r.id),
            ));
        }
        let Some(o) = raw.outcomes.iter().find(|o| o.id == r.then.outcome) else {
            ds.push(error(
                "unknown_reference",
                format!("rule {} names unknown outcome", r.id),
            ));
            continue;
        };
        for f in &o.fields {
            match r.then.fields.get(&f.id) {
                Some(e)
                    if infer_in_context(
                        e,
                        &raw,
                        false,
                        &mut ds,
                        &r.id,
                        &format!("/rules/{rule_index}/then/fields/{}", f.id),
                        &r.origin.source_ids,
                    )
                    .as_ref()
                        == Some(&f.r#type) => {}
                Some(_) => ds.push(error(
                    "type_mismatch",
                    format!("rule {} field {} has wrong type", r.id, f.id),
                )),
                None => ds.push(error(
                    "missing_outcome_field",
                    format!("rule {} omits field {}", r.id, f.id),
                )),
            }
        }
        for field in r.then.fields.keys() {
            if !o.fields.iter().any(|f| f.id == *field) {
                ds.push(error(
                    "extra_outcome_field",
                    format!("rule {} has extra field {field}", r.id),
                ));
            }
        }
    }
    for c in &raw.constraints {
        let target = raw.outcomes.iter().find(|o| o.id == c.for_outcome);
        if target.is_none() {
            ds.push(error(
                "unknown_reference",
                format!("constraint {} names unknown outcome", c.id),
            ));
        }
        let mut paths = vec![];
        refs(&c.must, &mut paths);
        if let Some(target) = target {
            for path in paths
                .iter()
                .filter_map(|path| path.strip_prefix("outcome."))
            {
                match target.fields.iter().find(|field| field.id == path) {
                    Some(_field) => {}
                    None => ds.push(error(
                        "unknown_reference",
                        format!(
                            "constraint {} references missing field outcome.{path} on {}",
                            c.id, target.id
                        ),
                    )),
                }
            }
        }
        let mut scoped = raw.clone();
        scoped
            .outcomes
            .retain(|outcome| outcome.id == c.for_outcome);
        let constraint_index = raw
            .constraints
            .iter()
            .position(|item| item.id == c.id)
            .unwrap_or(0);
        if infer_in_context(
            &c.must,
            &scoped,
            true,
            &mut ds,
            &c.id,
            &format!("/constraints/{constraint_index}/must"),
            &c.origin.source_ids,
        ) != Some(Type::Bool)
        {
            ds.push(error(
                "type_mismatch",
                format!("constraint {} must be boolean", c.id),
            ));
        }
    }
    for d in &raw.derived {
        let mut rs = vec![];
        refs(&d.expr, &mut rs);
        for r in rs {
            if let Some(id) = r.strip_prefix("derived.") {
                if !raw.derived.iter().any(|x| x.id == id) {
                    ds.push(error(
                        "unknown_reference",
                        format!("derived.{} references unknown derived.{id}", d.id),
                    ));
                }
            }
        }
    }
    let mut order = vec![];
    let mut pending: BTreeSet<String> = raw.derived.iter().map(|x| x.id.clone()).collect();
    let mut done = BTreeSet::new();
    while !pending.is_empty() {
        let ready = pending
            .iter()
            .filter(|id| {
                raw.derived.iter().find(|x| x.id == ***id).is_some_and(|x| {
                    let mut r = vec![];
                    refs(&x.expr, &mut r);
                    r.iter()
                        .filter_map(|v| v.strip_prefix("derived."))
                        .all(|v| done.contains(v))
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        if ready.is_empty() {
            let cycle_id = pending.iter().next().map_or("unknown", String::as_str);
            ds.push(error(
                "derived_cycle",
                format!("derived dependency graph contains a cycle involving {cycle_id}"),
            ));
            break;
        }
        for id in ready {
            pending.remove(&id);
            done.insert(id.clone());
            order.push(id);
        }
    }
    enrich_diagnostics(&mut ds, &raw);
    if !ds.is_empty() {
        return Err(ds);
    }
    let content_hash = hash(value);
    let mut semantic = serde_json::to_value(&raw).unwrap_or(serde_json::Value::Null);
    // Semantic identity excludes provenance, descriptive source records and open-item prose.
    if let serde_json::Value::Object(m) = &mut semantic {
        m.remove("sources");
        m.remove("open_items");
    }
    strip_semantic_metadata(&mut semantic);
    Ok(CompiledModel {
        raw,
        derived_order: order,
        content_hash,
        semantic_hash: hash(&semantic),
    })
}

fn enrich_diagnostics(diagnostics: &mut [Diagnostic], raw: &RawModel) {
    let declarations: Vec<_> = raw
        .enums
        .iter()
        .map(|item| (&item.id, "/enums", &item.origin.source_ids))
        .chain(
            raw.inputs
                .iter()
                .map(|item| (&item.id, "/inputs", &item.origin.source_ids)),
        )
        .chain(
            raw.derived
                .iter()
                .map(|item| (&item.id, "/derived", &item.origin.source_ids)),
        )
        .chain(
            raw.outcomes
                .iter()
                .map(|item| (&item.id, "/outcomes", &item.origin.source_ids)),
        )
        .chain(
            raw.domain
                .iter()
                .map(|item| (&item.id, "/domain", &item.origin.source_ids)),
        )
        .chain(
            raw.assumptions
                .iter()
                .map(|item| (&item.id, "/assumptions", &item.origin.source_ids)),
        )
        .chain(
            raw.rules
                .iter()
                .map(|item| (&item.id, "/rules", &item.origin.source_ids)),
        )
        .chain(
            raw.constraints
                .iter()
                .map(|item| (&item.id, "/constraints", &item.origin.source_ids)),
        )
        .chain(
            raw.open_items
                .iter()
                .map(|item| (&item.id, "/open_items", &item.source_ids)),
        )
        .collect();
    for diagnostic in diagnostics {
        if diagnostic.pointer.is_some() && diagnostic.declaration_id.is_some() {
            continue;
        }
        if let Some((id, pointer, source_ids)) = declarations
            .iter()
            .find(|(id, _, _)| diagnostic.message.contains(id.as_str()))
        {
            diagnostic.declaration_id = Some((*id).clone());
            let index = match *pointer {
                "/enums" => raw.enums.iter().position(|item| item.id == **id),
                "/inputs" => raw.inputs.iter().position(|item| item.id == **id),
                "/derived" => raw.derived.iter().position(|item| item.id == **id),
                "/outcomes" => raw.outcomes.iter().position(|item| item.id == **id),
                "/domain" => raw.domain.iter().position(|item| item.id == **id),
                "/assumptions" => raw.assumptions.iter().position(|item| item.id == **id),
                "/rules" => raw.rules.iter().position(|item| item.id == **id),
                "/constraints" => raw.constraints.iter().position(|item| item.id == **id),
                "/open_items" => raw.open_items.iter().position(|item| item.id == **id),
                _ => None,
            };
            if diagnostic.pointer.is_none() {
                diagnostic.pointer = Some(index.map_or_else(
                    || (*pointer).to_string(),
                    |index| format!("{pointer}/{index}"),
                ));
            }
            if diagnostic.source_ids.is_empty() {
                diagnostic.source_ids.clone_from(source_ids);
            }
        } else if diagnostic.pointer.is_none() {
            diagnostic.pointer = Some("/".to_string());
        }
    }
}

fn value_type(v: &Value) -> Type {
    match v {
        Value::Bool { .. } => Type::Bool,
        Value::Int { unit, .. } => Type::Int { unit: unit.clone() },
        Value::Date { .. } => Type::Date,
        Value::Enum { enum_name, .. } => Type::Enum {
            name: enum_name.clone(),
        },
    }
}
/// Validate that a scenario supplies exactly the model's declared inputs.
///
/// # Errors
/// Returns a diagnostic for missing, extra, malformed, or incorrectly typed values.
pub fn validate_inputs(model: &CompiledModel, inputs: &InputValues) -> Result<(), Diagnostic> {
    let decl: BTreeMap<_, _> = model
        .raw()
        .inputs
        .iter()
        .map(|x| (x.id.as_str(), &x.r#type))
        .collect();
    for (id, t) in &decl {
        let Some(v) = inputs.get(*id) else {
            return Err(error("missing_input", format!("missing input {id}")));
        };
        if &value_type(v) != *t {
            return Err(error("type_mismatch", format!("input {id} has wrong type")));
        }
        match v {
            Value::Int { value, .. } if !canonical_integer(value) => {
                return Err(error(
                    "invalid_value",
                    format!("input {id} integer is not canonical"),
                ));
            }
            Value::Date { value } if parse_date(value).is_none() => {
                return Err(error(
                    "invalid_date",
                    format!("input {id} is not a valid date"),
                ));
            }
            Value::Enum { enum_name, variant }
                if !model
                    .raw()
                    .enums
                    .iter()
                    .any(|e| e.id == *enum_name && e.variants.contains(variant)) =>
            {
                return Err(error(
                    "unknown_variant",
                    format!("invalid variant {enum_name}.{variant}"),
                ));
            }
            _ => {}
        }
    }
    if inputs.len() != decl.len() {
        return Err(error("extra_input", "scenario includes undeclared inputs"));
    }
    Ok(())
}
fn getpath(
    path: &str,
    inputs: &InputValues,
    derived: &BTreeMap<String, Value>,
    outcome: &BTreeMap<String, Value>,
) -> Result<Value, Diagnostic> {
    let mut p = path.splitn(2, '.');
    let ns = p.next().unwrap_or("");
    let id = p.next().unwrap_or("");
    let v = match ns {
        "input" => inputs.get(id),
        "derived" => derived.get(id),
        "outcome" => outcome.get(id),
        _ => None,
    };
    v.cloned()
        .ok_or_else(|| error("unknown_reference", format!("unavailable reference {path}")))
}
// BigInt is arbitrary precision: these operations cannot overflow, so checked fixed-width
// arithmetic would add no safety and would incorrectly suggest a bounded numeric domain.
#[allow(clippy::arithmetic_side_effects)]
fn arbitrary_precision_add(left: &BigInt, right: &BigInt) -> BigInt {
    left + right
}

#[allow(clippy::arithmetic_side_effects)]
fn arbitrary_precision_subtract(left: &BigInt, right: &BigInt) -> BigInt {
    left - right
}

#[allow(clippy::arithmetic_side_effects)]
fn arbitrary_precision_multiply(left: &BigInt, right: &BigInt) -> BigInt {
    left * right
}

/// Evaluate an expression with the supplied input, derived, and outcome values.
///
/// # Errors
/// Returns a diagnostic for unavailable references, invalid values, or incompatible operands.
// Evaluation is kept as one exhaustive dispatch so each expression's runtime behavior is auditable.
#[allow(clippy::too_many_lines)]
pub fn eval_expr(
    expr: &Expr,
    inputs: &InputValues,
    derived: &BTreeMap<String, Value>,
    outcome_fields: &BTreeMap<String, Value>,
) -> Result<Value, Diagnostic> {
    let ev = |e: &Expr| eval_expr(e, inputs, derived, outcome_fields);
    let boolean = |right_unit: Value| {
        if let Value::Bool { value } = right_unit {
            Ok(value)
        } else {
            Err(error("type_mismatch", "expected boolean"))
        }
    };
    let int = |right_unit: Value| {
        if let Value::Int { value, unit } = right_unit {
            BigInt::parse_bytes(value.as_bytes(), 10)
                .map(|result| (result, unit))
                .ok_or_else(|| error("invalid_value", "invalid integer"))
        } else {
            Err(error("type_mismatch", "expected integer"))
        }
    };
    let bool_value = |boolean_value| {
        Ok(Value::Bool {
            value: boolean_value,
        })
    };
    match expr {
        Expr::Bool { value } => Ok(Value::Bool { value: *value }),
        Expr::Int { value, unit } => Ok(Value::Int {
            value: value.clone(),
            unit: unit.clone(),
        }),
        Expr::Date { value } => {
            if parse_date(value).is_none() {
                Err(error("invalid_date", "invalid date literal"))
            } else {
                Ok(Value::Date {
                    value: value.clone(),
                })
            }
        }
        Expr::Enum { enum_name, variant } => Ok(Value::Enum {
            enum_name: enum_name.clone(),
            variant: variant.clone(),
        }),
        Expr::Ref { path } => getpath(path, inputs, derived, outcome_fields),
        Expr::Not { arg } => bool_value(!boolean(ev(arg)?)?),
        Expr::And { args } => {
            for left_operand in args {
                if !boolean(ev(left_operand)?)? {
                    return bool_value(false);
                }
            }
            bool_value(true)
        }
        Expr::Or { args } => {
            for left_operand in args {
                if boolean(ev(left_operand)?)? {
                    return bool_value(true);
                }
            }
            bool_value(false)
        }
        Expr::Eq { left, right } => bool_value(ev(left)? == ev(right)?),
        Expr::Ne { left, right } => bool_value(ev(left)? != ev(right)?),
        Expr::Lt { left, right }
        | Expr::Le { left, right }
        | Expr::Gt { left, right }
        | Expr::Ge { left, right } => {
            let left_value = ev(left)?;
            let right_value = ev(right)?;
            let ord = match (&left_value, &right_value) {
                (
                    Value::Int {
                        value: left_operand,
                        unit: left_unit,
                    },
                    Value::Int {
                        value: right_operand,
                        unit: right_unit,
                    },
                ) if left_unit == right_unit => BigInt::parse_bytes(left_operand.as_bytes(), 10)
                    .and_then(|boolean_value| {
                        BigInt::parse_bytes(right_operand.as_bytes(), 10)
                            .map(|y| boolean_value.cmp(&y))
                    }),
                (
                    Value::Date {
                        value: left_operand,
                    },
                    Value::Date {
                        value: right_operand,
                    },
                ) => parse_date(left_operand)
                    .zip(parse_date(right_operand))
                    .map(|(boolean_value, y)| boolean_value.cmp(&y)),
                _ => None,
            }
            .ok_or_else(|| error("type_mismatch", "incompatible ordered values"))?;
            bool_value(match expr {
                Expr::Lt { .. } => ord.is_lt(),
                Expr::Le { .. } => !ord.is_gt(),
                Expr::Gt { .. } => ord.is_gt(),
                _ => !ord.is_lt(),
            })
        }
        Expr::Add { left, right } | Expr::Sub { left, right } => {
            let (left_operand, left_unit) = int(ev(left)?)?;
            let (right_operand, right_unit) = int(ev(right)?)?;
            if left_unit != right_unit {
                return Err(error("unit_mismatch", "integer units differ"));
            }
            let result = if matches!(expr, Expr::Add { .. }) {
                arbitrary_precision_add(&left_operand, &right_operand)
            } else {
                arbitrary_precision_subtract(&left_operand, &right_operand)
            };
            Ok(Value::Int {
                value: result.to_string(),
                unit: left_unit,
            })
        }
        Expr::Scale { factor, arg } => {
            let (result, left_unit) = int(ev(arg)?)?;
            if !canonical_integer(factor) {
                return Err(error("invalid_value", "invalid scale factor"));
            }
            let f = BigInt::parse_bytes(factor.as_bytes(), 10)
                .ok_or_else(|| error("invalid_value", "invalid scale factor"))?;
            Ok(Value::Int {
                value: arbitrary_precision_multiply(&result, &f).to_string(),
                unit: left_unit,
            })
        }
        Expr::If {
            condition,
            then_expr,
            else_expr,
        } => {
            if boolean(ev(condition)?)? {
                ev(then_expr)
            } else {
                ev(else_expr)
            }
        }
    }
}
/// Compute derived values in dependency order after validating the input scenario.
///
/// # Errors
/// Returns a diagnostic if inputs are invalid or expression evaluation fails.
pub fn compute_derived(
    model: &CompiledModel,
    inputs: &InputValues,
) -> Result<BTreeMap<String, Value>, Diagnostic> {
    validate_inputs(model, inputs)?;
    let mut vals = BTreeMap::new();
    for id in model.derived_order() {
        let d = model
            .raw()
            .derived
            .iter()
            .find(|x| x.id == *id)
            .ok_or_else(|| error("internal_error", "missing derived declaration"))?;
        let v = eval_expr(&d.expr, inputs, &vals, &BTreeMap::new())?;
        if value_type(&v) != d.r#type {
            return Err(error(
                "type_mismatch",
                format!("derived {id} type mismatch"),
            ));
        }
        vals.insert(id.clone(), v);
    }
    Ok(vals)
}
/// Select assumptions explicitly by declaration ID; no assumptions are selected by default.
///
/// # Errors
/// Returns a diagnostic for unknown IDs or duplicate selections.
pub fn select_assumptions(
    model: &CompiledModel,
    ids: &[String],
) -> Result<Vec<Predicate>, Diagnostic> {
    let mut out = vec![];
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(error(
                "duplicate_name",
                format!("assumption {id} selected twice"),
            ));
        }
        let p = model
            .raw()
            .assumptions
            .iter()
            .find(|x| x.id == *id)
            .ok_or_else(|| error("unknown_reference", format!("unknown assumption {id}")))?;
        out.push(p.clone());
    }
    Ok(out)
}
/// Check the model domain and only the explicitly selected assumptions.
///
/// # Errors
/// Returns a diagnostic if assumption selection or predicate evaluation fails.
pub fn in_domain(
    model: &CompiledModel,
    inputs: &InputValues,
    derived: &BTreeMap<String, Value>,
    assumptions: &[String],
) -> Result<bool, Diagnostic> {
    for p in &model.raw().domain {
        if eval_expr(&p.expr, inputs, derived, &BTreeMap::new())? != (Value::Bool { value: true }) {
            return Ok(false);
        }
    }
    for p in select_assumptions(model, assumptions)? {
        if eval_expr(&p.expr, inputs, derived, &BTreeMap::new())? != (Value::Bool { value: true }) {
            return Ok(false);
        }
    }
    Ok(true)
}
/// Evaluate matching rules, applicable constraints, and an optional expected outcome.
///
/// # Errors
/// Returns a diagnostic when input validation, assumption selection, or evaluation fails.
pub fn evaluate(
    model: &CompiledModel,
    inputs: &InputValues,
    assumption_ids: &[String],
    expected: Option<&OutcomeValue>,
) -> Result<Evaluation, Diagnostic> {
    let derived = compute_derived(model, inputs)?;
    if !in_domain(model, inputs, &derived, assumption_ids)? {
        return Ok(Evaluation {
            classification: "outside_domain".into(),
            matched_rule_ids: vec![],
            outcomes: vec![],
            violated_constraint_ids: vec![],
            derived,
            expectation: expected.map(|_| "unspecified".into()),
        });
    }
    let mut matched = vec![];
    let mut outs = BTreeSet::new();
    for r in &model.raw().rules {
        if eval_expr(&r.when, inputs, &derived, &BTreeMap::new())? == (Value::Bool { value: true })
        {
            let mut fields = BTreeMap::new();
            for (k, e) in &r.then.fields {
                fields.insert(k.clone(), eval_expr(e, inputs, &derived, &BTreeMap::new())?);
            }
            matched.push(r.id.clone());
            outs.insert(OutcomeValue {
                outcome: r.then.outcome.clone(),
                fields,
            });
        }
    }
    let outcomes: Vec<_> = outs.into_iter().collect();
    let classification = match outcomes.len() {
        0 => "unspecified",
        1 => "unique",
        _ => "conflicting",
    };
    let mut violated = BTreeSet::new();
    for o in &outcomes {
        let ov = BTreeMap::from_iter(o.fields.iter().map(|(k, v)| (k.clone(), v.clone())));
        for c in model
            .raw()
            .constraints
            .iter()
            .filter(|c| c.for_outcome == o.outcome)
        {
            if eval_expr(&c.must, inputs, &derived, &ov)? != (Value::Bool { value: true }) {
                violated.insert(c.id.clone());
            }
        }
    }
    let expectation = expected.map(|e| {
        if classification == "conflicting" {
            "disagrees"
        } else if classification == "unspecified" {
            "unspecified"
        } else if outcomes.first() == Some(e) {
            "agrees"
        } else {
            "disagrees"
        }
        .to_string()
    });
    Ok(Evaluation {
        classification: classification.into(),
        matched_rule_ids: matched,
        outcomes,
        violated_constraint_ids: violated.into_iter().collect(),
        derived,
        expectation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixtures_compile_or_fail_as_expected() {
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../author-engine-v0/fixtures");
        for name in [
            "valid/partial.json",
            "valid/repaired-partial.json",
            "valid/unsafe-grant.json",
            "valid/empty-domain.json",
            "valid/duplicate-output.json",
            "valid/conflict-tag.json",
            "valid/conflict-payload.json",
            "valid/conditional-assumption.json",
            "valid/complete-proposed.json",
            "valid/big-integer.json",
        ] {
            let v: serde_json::Value =
                serde_json::from_slice(&std::fs::read(base.join(name)).unwrap()).unwrap();
            assert!(compile_json(&v, &ModelLimits::default()).is_ok(), "{name}");
        }
        for name in [
            "invalid/unsupported-node.json",
            "invalid/unknown-reference.json",
            "invalid/unit-mismatch.json",
            "invalid/invalid-date.json",
            "invalid/derived-cycle.json",
        ] {
            let v: serde_json::Value =
                serde_json::from_slice(&std::fs::read(base.join(name)).unwrap()).unwrap();
            assert!(compile_json(&v, &ModelLimits::default()).is_err(), "{name}");
        }
    }
    #[test]
    fn exact_big_integer_expression() {
        let expr = Expr::Add {
            left: Box::new(Expr::Int {
                value: "999999999999999999999999".into(),
                unit: "count".into(),
            }),
            right: Box::new(Expr::Int {
                value: "1".into(),
                unit: "count".into(),
            }),
        };
        let v = eval_expr(&expr, &BTreeMap::new(), &BTreeMap::new(), &BTreeMap::new()).unwrap();
        assert_eq!(
            v,
            Value::Int {
                value: "1000000000000000000000000".into(),
                unit: "count".into()
            }
        );
    }
    #[test]
    fn unsafe_scenario_replays_constraint_violation() {
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../author-engine-v0");
        let model_value: serde_json::Value = serde_json::from_slice(
            &std::fs::read(base.join("fixtures/valid/unsafe-grant.json")).unwrap(),
        )
        .unwrap();
        let model = compile_json(&model_value, &ModelLimits::default()).unwrap();
        let scenario: serde_json::Value = serde_json::from_slice(
            &std::fs::read(base.join("scenarios/unsafe-expiry.json")).unwrap(),
        )
        .unwrap();
        let inputs: InputValues = serde_json::from_value(scenario["inputs"].clone()).unwrap();
        let result = evaluate(&model, &inputs, &[], None).unwrap();
        assert_eq!(result.classification, "unique");
        assert!(
            result
                .violated_constraint_ids
                .iter()
                .any(|x| x == "ExpiryWithinContract")
        );
    }
    #[test]
    fn metadata_changes_content_but_not_semantics() {
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../author-engine-v0/fixtures/valid/partial.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(base).unwrap()).unwrap();
        let first = compile_json(&value, &ModelLimits::default()).unwrap();
        value["sources"][0]["text"] = serde_json::Value::String("different provenance text".into());
        let second = compile_json(&value, &ModelLimits::default()).unwrap();
        assert_ne!(first.content_hash(), second.content_hash());
        assert_eq!(first.semantic_hash(), second.semantic_hash());
    }
    fn fixture() -> serde_json::Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../author-engine-v0/fixtures/valid/partial.json");
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }
    #[test]
    fn constraint_outcome_fields_are_scoped_to_target_constructor() {
        let mut value = fixture();
        value["outcomes"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "id": "Other", "fields": [{"id":"expiry", "type":{"kind":"int", "unit":"days"}}],
                "origin":{"kind":"proposal", "source_ids":[], "review":"proposed"}
            }));
        let constraint = value["constraints"][1].as_object_mut().unwrap();
        constraint.insert("for_outcome".into(), serde_json::json!("Other"));
        constraint.insert("must".into(), serde_json::json!({"kind":"eq", "left":{"kind":"ref", "path":"outcome.expiry"}, "right":{"kind":"int", "value":"1", "unit":"days"}}));
        assert!(compile_json(&value, &ModelLimits::default()).is_ok());
        value["constraints"][1]["must"]["right"]["unit"] = serde_json::json!("hours");
        assert!(compile_json(&value, &ModelLimits::default()).is_err());
    }
    #[test]
    fn year_zero_date_is_rejected() {
        let mut value = fixture();
        value["constraints"][1]["must"]["left"] =
            serde_json::json!({"kind":"date", "value":"0000-01-01"});
        assert!(compile_json(&value, &ModelLimits::default()).is_err());
    }
    #[test]
    fn preflight_rejects_depth_before_deserialization() {
        let mut value = fixture();
        let mut expr = serde_json::json!({"kind":"bool", "value":true});
        for _ in 0..20 {
            expr = serde_json::json!({"kind":"not", "arg":expr});
        }
        value["constraints"][0]["must"] = expr;
        let limits = ModelLimits {
            expression_depth: 8,
            ..ModelLimits::default()
        };
        let errors = compile_json(&value, &limits).unwrap_err();
        assert!(errors.iter().any(|diag| diag.code == "resource_limit"));
    }
    #[test]
    fn schema_patterns_and_optional_nulls_are_enforced() {
        let mut value = fixture();
        value["model_id"] = serde_json::json!("1bad");
        assert!(compile_json(&value, &ModelLimits::default()).is_err());
        value = fixture();
        value["sources"][0]["locator"] = serde_json::Value::Null;
        assert!(compile_json(&value, &ModelLimits::default()).is_err());
        value = fixture();
        value["open_items"] = serde_json::json!([{"id":"Question", "text":"x", "source_ids":[], "affects":"unknown"}]);
        assert!(compile_json(&value, &ModelLimits::default()).is_err());
    }
    #[test]
    fn if_expression_uses_schema_else_property() {
        let value = serde_json::json!({"kind":"if", "condition":{"kind":"bool", "value":true}, "then":{"kind":"bool", "value":true}, "else":{"kind":"bool", "value":false}});
        assert!(serde_json::from_value::<Expr>(value).is_ok());
    }

    #[test]
    fn selected_assumptions_are_explicit_and_unknown_is_rejected() {
        let model = compile_json(&fixture(), &ModelLimits::default()).unwrap();
        assert!(select_assumptions(&model, &[]).unwrap().is_empty());
        let missing = select_assumptions(&model, &["NoSuchAssumption".into()]).unwrap_err();
        assert_eq!(missing.code, "unknown_reference");
    }

    #[test]
    fn dates_cover_calendar_edges_and_year_9999() {
        assert!(parse_date("0001-01-01").is_some());
        assert!(parse_date("9999-12-31").is_some());
        assert!(parse_date("0000-12-31").is_none());
        assert!(parse_date("1900-02-29").is_none());
        assert!(parse_date("2000-02-29").is_some());
    }

    #[test]
    fn arbitrary_precision_arithmetic_scale_and_if_are_exact() {
        let huge = "184467440737095516160000000000000000000";
        let scaled = Expr::Scale {
            factor: "-3".into(),
            arg: Box::new(Expr::Int {
                value: huge.into(),
                unit: "credits".into(),
            }),
        };
        assert_eq!(
            eval_expr(
                &scaled,
                &BTreeMap::new(),
                &BTreeMap::new(),
                &BTreeMap::new()
            )
            .unwrap(),
            Value::Int {
                value: "-553402322211286548480000000000000000000".into(),
                unit: "credits".into()
            }
        );
        let conditional = Expr::If {
            condition: Box::new(Expr::Bool { value: false }),
            then_expr: Box::new(Expr::Int {
                value: "999999999999999999999999999".into(),
                unit: "credits".into(),
            }),
            else_expr: Box::new(Expr::Sub {
                left: Box::new(Expr::Int {
                    value: "0".into(),
                    unit: "credits".into(),
                }),
                right: Box::new(Expr::Int {
                    value: "999999999999999999999999999".into(),
                    unit: "credits".into(),
                }),
            }),
        };
        assert_eq!(
            eval_expr(
                &conditional,
                &BTreeMap::new(),
                &BTreeMap::new(),
                &BTreeMap::new()
            )
            .unwrap(),
            Value::Int {
                value: "-999999999999999999999999999".into(),
                unit: "credits".into()
            }
        );
    }

    #[test]
    fn semantic_failures_include_declaration_context() {
        let mut value = fixture();
        value["constraints"][0]["must"] = serde_json::json!({"kind":"ref", "path":"input.Missing"});
        let diagnostics = compile_json(&value, &ModelLimits::default()).unwrap_err();
        let diagnostic = diagnostics
            .iter()
            .find(|item| item.code == "unknown_reference")
            .unwrap();
        assert_eq!(
            diagnostic.declaration_id.as_deref(),
            Some(value["constraints"][0]["id"].as_str().unwrap())
        );
        assert!(
            diagnostic
                .pointer
                .as_deref()
                .is_some_and(|pointer| pointer.starts_with("/constraints/"))
        );
        assert!(!diagnostic.source_ids.is_empty());
    }

    #[test]
    fn derived_forward_dependencies_sort_and_cycles_identify_a_declaration() {
        let mut value = fixture();
        value["derived"] = serde_json::json!([
            {"id":"Later", "type":{"kind":"int", "unit":"days"}, "expr":{"kind":"ref", "path":"derived.Earlier"}, "origin":{"kind":"proposal", "source_ids":[], "review":"proposed"}},
            {"id":"Earlier", "type":{"kind":"int", "unit":"days"}, "expr":{"kind":"int", "value":"1", "unit":"days"}, "origin":{"kind":"proposal", "source_ids":[], "review":"proposed"}}
        ]);
        let model = compile_json(&value, &ModelLimits::default()).unwrap();
        assert_eq!(model.derived_order(), ["Earlier", "Later"]);
        value["derived"][1]["expr"] = serde_json::json!({"kind":"ref", "path":"derived.Later"});
        let diagnostics = compile_json(&value, &ModelLimits::default()).unwrap_err();
        let cycle = diagnostics
            .iter()
            .find(|item| item.code == "derived_cycle")
            .unwrap();
        assert!(cycle.declaration_id.is_some());
        assert!(cycle.pointer.is_some());
    }

    #[test]
    fn deduplicates_equal_outcomes_but_reports_conflicting_payloads() {
        let mut value = fixture();
        value["rules"] = serde_json::json!([
            {"id":"R1", "when":{"kind":"bool", "value":true}, "then":{"outcome":"Grant", "fields":{"expiry":{"kind":"date", "value":"2025-01-01"}}}, "origin":{"kind":"proposal", "source_ids":[], "review":"proposed"}},
            {"id":"R2", "when":{"kind":"bool", "value":true}, "then":{"outcome":"Grant", "fields":{"expiry":{"kind":"date", "value":"2025-01-01"}}}, "origin":{"kind":"proposal", "source_ids":[], "review":"proposed"}}
        ]);
        let model = compile_json(&value, &ModelLimits::default()).unwrap();
        let inputs: InputValues =
            serde_json::from_value(value_for_declared_inputs(&model)).unwrap();
        assert_eq!(
            evaluate(&model, &inputs, &[], None).unwrap().outcomes.len(),
            1
        );
        value["rules"][1]["then"]["fields"]["expiry"]["value"] = serde_json::json!("2026-01-01");
        let model = compile_json(&value, &ModelLimits::default()).unwrap();
        let inputs: InputValues =
            serde_json::from_value(value_for_declared_inputs(&model)).unwrap();
        assert_eq!(
            evaluate(&model, &inputs, &[], None).unwrap().classification,
            "conflicting"
        );
    }

    fn value_for_declared_inputs(model: &CompiledModel) -> serde_json::Value {
        let mut values = serde_json::Map::new();
        for input in &model.raw().inputs {
            let value = match &input.r#type {
                Type::Bool => serde_json::json!({"kind":"bool", "value":false}),
                Type::Int { unit } => serde_json::json!({"kind":"int", "value":"0", "unit":unit}),
                Type::Date => serde_json::json!({"kind":"date", "value":"2000-01-01"}),
                Type::Enum { name } => {
                    serde_json::json!({"kind":"enum", "enum":name, "variant":model.raw().enums.iter().find(|item| item.id == *name).unwrap().variants[0]})
                }
            };
            values.insert(input.id.clone(), value);
        }
        serde_json::Value::Object(values)
    }
}
