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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub kind: OriginKind,
    pub source_ids: Vec<String>,
    pub review: Review,
    #[serde(skip_serializing_if = "Option::is_none")]
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

    fn with_pointer(mut self, pointer: String) -> Self {
        self.pointer = Some(pointer);
        self
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
/// # use author_model::{CompiledModel, Expr};
/// fn attempt_edit(model: &mut CompiledModel) {
///     model.raw().domain[0].expr = Expr::Bool { value: true };
/// }
/// ```
///
/// Construction is also private, even with otherwise correctly typed fields:
///
/// ```compile_fail
/// # use author_model::{CompiledModel, RawModel};
/// fn attempt_construct(raw: RawModel) {
///     let _model = CompiledModel {
///         raw,
///         derived_order: Vec::new(),
///         content_hash: String::new(),
///         semantic_hash: String::new(),
///     };
/// }
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

fn contextual_error(
    code: &str,
    message: impl Into<String>,
    declaration_id: Option<&str>,
    pointer: impl Into<String>,
    source_ids: &[String],
) -> Diagnostic {
    let mut diagnostic = error(code, message);
    diagnostic.declaration_id = declaration_id.map(str::to_owned);
    diagnostic.pointer = Some(pointer.into());
    diagnostic.source_ids = source_ids.to_vec();
    diagnostic
}

fn pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
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
fn collect_reference_paths(expression: &Expr, pointer: &str, output: &mut Vec<(String, String)>) {
    match expression {
        Expr::Ref { path } => output.push((path.clone(), format!("{pointer}/path"))),
        Expr::Not { arg } | Expr::Scale { arg, .. } => {
            collect_reference_paths(arg, &format!("{pointer}/arg"), output);
        }
        Expr::And { args } | Expr::Or { args } => {
            for (index, argument) in args.iter().enumerate() {
                collect_reference_paths(argument, &format!("{pointer}/args/{index}"), output);
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
            collect_reference_paths(left, &format!("{pointer}/left"), output);
            collect_reference_paths(right, &format!("{pointer}/right"), output);
        }
        Expr::If {
            condition,
            then_expr,
            else_expr,
        } => {
            collect_reference_paths(condition, &format!("{pointer}/condition"), output);
            collect_reference_paths(then_expr, &format!("{pointer}/then"), output);
            collect_reference_paths(else_expr, &format!("{pointer}/else"), output);
        }
        Expr::Bool { .. } | Expr::Int { .. } | Expr::Date { .. } | Expr::Enum { .. } => {}
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
    let mut references = vec![];
    collect_reference_paths(e, pointer, &mut references);
    for (path, reference_pointer) in references {
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
            diagnostic.pointer = Some(reference_pointer);
            diagnostic.source_ids = source_ids.to_vec();
            ds.push(diagnostic);
        }
    }
}
struct InferContext<'a> {
    declaration_id: &'a str,
    pointer: &'a str,
    source_ids: &'a [String],
}

// Type inference is intentionally one exhaustive dispatch over the public expression variants.
#[allow(clippy::too_many_lines)]
fn infer(
    expression: &Expr,
    raw: &RawModel,
    allow_outcome: bool,
    diagnostics: &mut Vec<Diagnostic>,
    context: &InferContext<'_>,
) -> Option<Type> {
    let start = diagnostics.len();
    let child = |node: &Expr, suffix: &str, diagnostics: &mut Vec<Diagnostic>| {
        let pointer = format!("{}{suffix}", context.pointer);
        infer(
            node,
            raw,
            allow_outcome,
            diagnostics,
            &InferContext {
                declaration_id: context.declaration_id,
                pointer: &pointer,
                source_ids: context.source_ids,
            },
        )
    };
    let inferred = match expression {
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
            if child(arg, "/arg", diagnostics) != Some(Type::Bool) {
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
            for (index, argument) in args.iter().enumerate() {
                if child(argument, &format!("/args/{index}"), diagnostics) != Some(Type::Bool) {
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
            let left_type = child(left, "/left", diagnostics);
            let right_type = child(right, "/right", diagnostics);
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
            let left_type = child(left, "/left", diagnostics);
            let right_type = child(right, "/right", diagnostics);
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
            let left_type = child(arg, "/arg", diagnostics);
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
            if child(condition, "/condition", diagnostics) != Some(Type::Bool) {
                diagnostics.push(error("type_mismatch", "if condition must be bool"));
            }
            let left_type = child(then_expr, "/then", diagnostics);
            let right_type = child(else_expr, "/else", diagnostics);
            if left_type != right_type {
                diagnostics.push(error("type_mismatch", "if branches differ"));
            }
            left_type
        }
    };
    if let Some(new_diagnostics) = diagnostics.get_mut(start..) {
        for diagnostic in new_diagnostics {
            if diagnostic.pointer.is_none() {
                diagnostic.declaration_id = Some(context.declaration_id.to_string());
                diagnostic.pointer = Some(context.pointer.to_string());
                diagnostic.source_ids = context.source_ids.to_vec();
            }
        }
    }
    inferred
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
    infer(
        expression,
        raw,
        allow_outcome,
        diagnostics,
        &InferContext {
            declaration_id,
            pointer,
            source_ids,
        },
    )
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
            return Err(contextual_error(
                "resource_limit",
                format!("{key} declaration limit exceeded"),
                None,
                format!("/{key}"),
                &[],
            ));
        }
    }
    Ok(())
}

fn derived_cycle_node(raw: &RawModel, pending: &BTreeSet<String>) -> Option<String> {
    fn visit(
        id: &str,
        raw: &RawModel,
        pending: &BTreeSet<String>,
        states: &mut BTreeMap<String, bool>,
        stack: &mut Vec<String>,
    ) -> Option<String> {
        if let Some(position) = stack.iter().position(|active| active == id) {
            return stack.get(position).cloned();
        }
        if states.contains_key(id) || !pending.contains(id) {
            return None;
        }
        states.insert(id.to_string(), true);
        stack.push(id.to_string());
        let declaration = raw.derived.iter().find(|item| item.id == id)?;
        let mut references = Vec::new();
        refs(&declaration.expr, &mut references);
        for dependency in references
            .iter()
            .filter_map(|path| path.strip_prefix("derived."))
        {
            if let Some(cycle_node) = visit(dependency, raw, pending, states, stack) {
                return Some(cycle_node);
            }
        }
        stack.pop();
        states.insert(id.to_string(), false);
        None
    }

    let mut states = BTreeMap::new();
    let mut stack = Vec::new();
    for id in pending {
        if let Some(cycle_node) = visit(id, raw, pending, &mut states, &mut stack) {
            return Some(cycle_node);
        }
    }
    None
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

fn expression_roots(
    root: &serde_json::Map<String, serde_json::Value>,
) -> Vec<(&serde_json::Value, String)> {
    let mut roots = Vec::new();
    for key in ["derived", "domain", "assumptions"] {
        if let Some(items) = root.get(key).and_then(serde_json::Value::as_array) {
            for (index, item) in items.iter().enumerate() {
                if let Some(expression) = item.get("expr") {
                    roots.push((expression, format!("/{key}/{index}/expr")));
                }
            }
        }
    }
    if let Some(items) = root.get("rules").and_then(serde_json::Value::as_array) {
        for (index, item) in items.iter().enumerate() {
            if let Some(expression) = item.get("when") {
                roots.push((expression, format!("/rules/{index}/when")));
            }
            if let Some(fields) = item
                .get("then")
                .and_then(|then| then.get("fields"))
                .and_then(serde_json::Value::as_object)
            {
                roots.extend(fields.iter().map(|(field, expression)| {
                    (
                        expression,
                        format!("/rules/{index}/then/fields/{}", pointer_segment(field)),
                    )
                }));
            }
        }
    }
    if let Some(items) = root
        .get("constraints")
        .and_then(serde_json::Value::as_array)
    {
        for (index, item) in items.iter().enumerate() {
            if let Some(expression) = item.get("must") {
                roots.push((expression, format!("/constraints/{index}/must")));
            }
        }
    }
    roots
}

// This is the exhaustive JSON-side dispatch for expression variants before serde construction.
#[allow(clippy::too_many_lines)]
fn expression_shape_error(expression: &serde_json::Value, pointer: &str) -> Option<Diagnostic> {
    let Some(object) = expression.as_object() else {
        return Some(
            error("schema_error", "expression must be an object").with_pointer(pointer.to_string()),
        );
    };
    let Some(kind) = object.get("kind").and_then(serde_json::Value::as_str) else {
        return Some(
            error("schema_error", "expression kind must be a string")
                .with_pointer(format!("{pointer}/kind")),
        );
    };
    let fields: &[&str] = match kind {
        "bool" | "date" => &["kind", "value"],
        "int" => &["kind", "value", "unit"],
        "enum" => &["kind", "enum", "variant"],
        "ref" => &["kind", "path"],
        "not" => &["kind", "arg"],
        "and" | "or" => &["kind", "args"],
        "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "add" | "sub" => &["kind", "left", "right"],
        "scale" => &["kind", "factor", "arg"],
        "if" => &["kind", "condition", "then", "else"],
        _ => {
            return Some(
                error(
                    "schema_error",
                    format!("unsupported expression kind {kind}"),
                )
                .with_pointer(format!("{pointer}/kind")),
            );
        }
    };
    if let Some(key) = object.keys().find(|key| !fields.contains(&key.as_str())) {
        return Some(
            error("schema_error", format!("unknown expression field {key}"))
                .with_pointer(format!("{pointer}/{}", pointer_segment(key))),
        );
    }
    if let Some(field) = fields.iter().find(|field| !object.contains_key(**field)) {
        return Some(
            error("schema_error", format!("missing expression field {field}"))
                .with_pointer(format!("{pointer}/{field}")),
        );
    }
    let string_fields: &[&str] = match kind {
        "int" => &["value", "unit"],
        "date" => &["value"],
        "enum" => &["enum", "variant"],
        "ref" => &["path"],
        "scale" => &["factor"],
        _ => &[],
    };
    if let Some(field) = string_fields.iter().find(|field| {
        !object
            .get(**field)
            .is_some_and(serde_json::Value::is_string)
    }) {
        return Some(
            error(
                "schema_error",
                format!("expression field {field} must be a string"),
            )
            .with_pointer(format!("{pointer}/{field}")),
        );
    }
    if kind == "bool"
        && !object
            .get("value")
            .is_some_and(serde_json::Value::is_boolean)
    {
        return Some(
            error("schema_error", "boolean expression value must be a boolean")
                .with_pointer(format!("{pointer}/value")),
        );
    }
    let object_fields: &[&str] = match kind {
        "not" | "scale" => &["arg"],
        "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "add" | "sub" => &["left", "right"],
        "if" => &["condition", "then", "else"],
        _ => &[],
    };
    if let Some(field) = object_fields.iter().find(|field| {
        !object
            .get(**field)
            .is_some_and(serde_json::Value::is_object)
    }) {
        return Some(
            error(
                "schema_error",
                format!("expression field {field} must be an object"),
            )
            .with_pointer(format!("{pointer}/{field}")),
        );
    }
    if (kind == "and" || kind == "or")
        && !object.get("args").is_some_and(serde_json::Value::is_array)
    {
        return Some(
            error("schema_error", "and/or args must be an array")
                .with_pointer(format!("{pointer}/args")),
        );
    }
    None
}

fn object_shape_error(
    value: &serde_json::Value,
    pointer: &str,
    allowed: &[&str],
    required: &[&str],
) -> Option<Diagnostic> {
    let Some(object) = value.as_object() else {
        return Some(error("schema_error", "expected an object").with_pointer(pointer.to_string()));
    };
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Some(
            error("schema_error", format!("unknown field {key}"))
                .with_pointer(format!("{pointer}/{}", pointer_segment(key))),
        );
    }
    required
        .iter()
        .find(|key| !object.contains_key(**key))
        .map(|key| {
            error("schema_error", format!("missing required field {key}"))
                .with_pointer(format!("{pointer}/{key}"))
        })
}

fn validate_object_shape(
    value: &serde_json::Value,
    pointer: &str,
    allowed: &[&str],
    required: &[&str],
) -> Result<(), Diagnostic> {
    object_shape_error(value, pointer, allowed, required).map_or(Ok(()), Err)
}

fn validate_origin_shape(origin: &serde_json::Value, pointer: &str) -> Result<(), Diagnostic> {
    validate_object_shape(
        origin,
        pointer,
        &["kind", "source_ids", "review", "note"],
        &["kind", "source_ids", "review"],
    )?;
    let origin_object = origin.as_object().ok_or_else(|| {
        error("schema_error", "origin must be an object").with_pointer(pointer.to_string())
    })?;
    for field in ["kind", "review"] {
        if !origin_object
            .get(field)
            .is_some_and(serde_json::Value::is_string)
        {
            return Err(
                error("schema_error", format!("origin {field} must be a string"))
                    .with_pointer(format!("{pointer}/{field}")),
            );
        }
    }
    if !origin_object
        .get("source_ids")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|ids| ids.iter().all(serde_json::Value::is_string))
    {
        return Err(error(
            "schema_error",
            "origin source_ids must be an array of strings",
        )
        .with_pointer(format!("{pointer}/source_ids")));
    }
    if !origin_object
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|kind| ["source", "proposal", "assumption"].contains(&kind))
    {
        return Err(
            error("schema_error", "unknown origin kind").with_pointer(format!("{pointer}/kind"))
        );
    }
    if !origin_object
        .get("review")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|review| ["proposed", "confirmed"].contains(&review))
    {
        return Err(error("schema_error", "unknown review status")
            .with_pointer(format!("{pointer}/review")));
    }
    if origin_object
        .get("note")
        .is_some_and(|note| !note.is_null() && !note.is_string())
    {
        return Err(error("schema_error", "note must be a string when present")
            .with_pointer(format!("{pointer}/note")));
    }
    Ok(())
}

fn validate_enum_variant_shapes(
    object: &serde_json::Map<String, serde_json::Value>,
    pointer: &str,
) -> Result<(), Diagnostic> {
    for (index, variant) in object
        .get("variants")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        if !variant.is_string() {
            return Err(error("schema_error", "enum variants must be strings")
                .with_pointer(format!("{pointer}/variants/{index}")));
        }
    }
    Ok(())
}

fn validate_outcome_field_shapes(
    object: &serde_json::Map<String, serde_json::Value>,
    pointer: &str,
) -> Result<(), Diagnostic> {
    let Some(fields) = object.get("fields").and_then(serde_json::Value::as_array) else {
        return Err(error("schema_error", "outcome fields must be an array")
            .with_pointer(format!("{pointer}/fields")));
    };
    for (index, field) in fields.iter().enumerate() {
        let field_pointer = format!("{pointer}/fields/{index}");
        validate_object_shape(field, &field_pointer, &["id", "type"], &["id", "type"])?;
        let field_object = field.as_object().ok_or_else(|| {
            error("schema_error", "outcome field must be an object")
                .with_pointer(field_pointer.clone())
        })?;
        if !field_object
            .get("id")
            .is_some_and(serde_json::Value::is_string)
        {
            return Err(error("schema_error", "field id must be a string")
                .with_pointer(format!("{field_pointer}/id")));
        }
        if let Some(ty) = field.get("type") {
            type_shape_error(ty, &format!("{field_pointer}/type"))?;
        }
    }
    Ok(())
}

fn validate_rule_conclusion_shape(
    object: &serde_json::Map<String, serde_json::Value>,
    pointer: &str,
) -> Result<(), Diagnostic> {
    let then = object.get("then").ok_or_else(|| {
        error("schema_error", "missing rule conclusion").with_pointer(format!("{pointer}/then"))
    })?;
    let then_pointer = format!("{pointer}/then");
    validate_object_shape(
        then,
        &then_pointer,
        &["outcome", "fields"],
        &["outcome", "fields"],
    )?;
    let then_object = then.as_object().ok_or_else(|| {
        error("schema_error", "then must be an object").with_pointer(then_pointer.clone())
    })?;
    if !then_object
        .get("outcome")
        .is_some_and(serde_json::Value::is_string)
    {
        return Err(error("schema_error", "outcome must be a string")
            .with_pointer(format!("{then_pointer}/outcome")));
    }
    if !then_object
        .get("fields")
        .is_some_and(serde_json::Value::is_object)
    {
        return Err(error("schema_error", "fields must be an object")
            .with_pointer(format!("{then_pointer}/fields")));
    }
    Ok(())
}

fn validate_collection_item_shape(
    collection: &str,
    item: &serde_json::Value,
    pointer: &str,
    fields: &[&str],
    required: &[&str],
) -> Result<(), Diagnostic> {
    validate_object_shape(item, pointer, fields, required)?;
    let object = item.as_object().ok_or_else(|| {
        error("schema_error", "expected an object").with_pointer(pointer.to_string())
    })?;
    let string_fields: &[&str] = match collection {
        "sources" | "open_items" => &["id", "text"],
        "constraints" => &["id", "for_outcome"],
        "enums" | "inputs" | "derived" | "outcomes" | "domain" | "assumptions" | "rules" => &["id"],
        _ => &[],
    };
    for field in string_fields {
        if !object.get(*field).is_some_and(serde_json::Value::is_string) {
            return Err(error("schema_error", format!("{field} must be a string"))
                .with_pointer(format!("{pointer}/{field}")));
        }
    }
    let array_fields: &[&str] = match collection {
        "enums" => &["variants"],
        "open_items" => &["source_ids"],
        _ => &[],
    };
    for field in array_fields {
        if !object.get(*field).is_some_and(serde_json::Value::is_array) {
            return Err(error("schema_error", format!("{field} must be an array"))
                .with_pointer(format!("{pointer}/{field}")));
        }
    }
    if collection == "enums" {
        validate_enum_variant_shapes(object, pointer)?;
    }
    if let Some(origin) = object.get("origin") {
        validate_origin_shape(origin, &format!("{pointer}/origin"))?;
    }
    if collection == "sources"
        && object
            .get("locator")
            .is_some_and(|locator| !locator.is_null() && !locator.is_string())
    {
        return Err(
            error("schema_error", "locator must be a string when present")
                .with_pointer(format!("{pointer}/locator")),
        );
    }
    if collection == "open_items" {
        if !object
            .get("affects")
            .is_some_and(serde_json::Value::is_string)
        {
            return Err(error("schema_error", "affects must be a string")
                .with_pointer(format!("{pointer}/affects")));
        }
        for (index, source_id) in object
            .get("source_ids")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            if !source_id.is_string() {
                return Err(error("schema_error", "source_ids entries must be strings")
                    .with_pointer(format!("{pointer}/source_ids/{index}")));
            }
        }
    }
    if let Some(ty) = object.get("type") {
        type_shape_error(ty, &format!("{pointer}/type"))?;
    }
    if collection == "outcomes" {
        validate_outcome_field_shapes(object, pointer)?;
    }
    if collection == "rules" {
        validate_rule_conclusion_shape(object, pointer)?;
    }
    Ok(())
}

fn validate_model_shapes(
    root: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), Diagnostic> {
    const ROOT_FIELDS: &[&str] = &[
        "language_version",
        "model_id",
        "sources",
        "enums",
        "inputs",
        "derived",
        "outcomes",
        "domain",
        "assumptions",
        "rules",
        "constraints",
        "open_items",
    ];
    if let Some(key) = root.keys().find(|key| !ROOT_FIELDS.contains(&key.as_str())) {
        return Err(error("schema_error", format!("unknown field {key}"))
            .with_pointer(format!("/{}", pointer_segment(key))));
    }
    if let Some(field) = ROOT_FIELDS.iter().find(|field| !root.contains_key(**field)) {
        return Err(
            error("schema_error", format!("missing required field {field}"))
                .with_pointer(format!("/{field}")),
        );
    }
    for (collection, fields, required) in [
        (
            "sources",
            &(["id", "text", "locator"][..]),
            &(["id", "text"][..]),
        ),
        (
            "enums",
            &(["id", "variants", "origin"][..]),
            &(["id", "variants", "origin"][..]),
        ),
        (
            "inputs",
            &(["id", "type", "origin"][..]),
            &(["id", "type", "origin"][..]),
        ),
        (
            "derived",
            &(["id", "type", "expr", "origin"][..]),
            &(["id", "type", "expr", "origin"][..]),
        ),
        (
            "outcomes",
            &(["id", "fields", "origin"][..]),
            &(["id", "fields", "origin"][..]),
        ),
        (
            "domain",
            &(["id", "expr", "origin"][..]),
            &(["id", "expr", "origin"][..]),
        ),
        (
            "assumptions",
            &(["id", "expr", "origin"][..]),
            &(["id", "expr", "origin"][..]),
        ),
        (
            "rules",
            &(["id", "when", "then", "origin"][..]),
            &(["id", "when", "then", "origin"][..]),
        ),
        (
            "constraints",
            &(["id", "for_outcome", "must", "origin"][..]),
            &(["id", "for_outcome", "must", "origin"][..]),
        ),
        (
            "open_items",
            &(["id", "text", "source_ids", "affects"][..]),
            &(["id", "text", "source_ids", "affects"][..]),
        ),
    ] {
        let Some(items) = root.get(collection).and_then(serde_json::Value::as_array) else {
            return Err(
                error("schema_error", format!("{collection} must be an array"))
                    .with_pointer(format!("/{collection}")),
            );
        };
        for (index, item) in items.iter().enumerate() {
            let pointer = format!("/{collection}/{index}");
            validate_collection_item_shape(collection, item, &pointer, fields, required)?;
        }
    }
    Ok(())
}

fn type_shape_error(value: &serde_json::Value, pointer: &str) -> Result<(), Diagnostic> {
    let Some(object) = value.as_object() else {
        return Err(
            error("schema_error", "type must be an object").with_pointer(pointer.to_string())
        );
    };
    let Some(kind) = object.get("kind").and_then(serde_json::Value::as_str) else {
        return Err(error("schema_error", "type kind must be a string")
            .with_pointer(format!("{pointer}/kind")));
    };
    let (allowed, required): (&[&str], &[&str]) = match kind {
        "bool" | "date" => (&["kind"], &["kind"]),
        "enum" => (&["kind", "name"], &["kind", "name"]),
        "int" => (&["kind", "unit"], &["kind", "unit"]),
        _ => {
            return Err(error("schema_error", format!("unknown type kind {kind}"))
                .with_pointer(format!("{pointer}/kind")));
        }
    };
    validate_object_shape(value, pointer, allowed, required)?;
    let string_field = match kind {
        "enum" => Some("name"),
        "int" => Some("unit"),
        _ => None,
    };
    if let Some(field) = string_field {
        if !object.get(field).is_some_and(serde_json::Value::is_string) {
            return Err(
                error("schema_error", format!("type {field} must be a string"))
                    .with_pointer(format!("{pointer}/{field}")),
            );
        }
    }
    Ok(())
}

fn attach_json_declaration_context(diagnostic: &mut Diagnostic, root: &serde_json::Value) {
    let Some(pointer) = diagnostic.pointer.as_deref() else {
        return;
    };
    let mut segments = pointer.split('/').skip(1);
    let Some(collection) = segments.next() else {
        return;
    };
    let Some(index) = segments
        .next()
        .and_then(|value| value.parse::<usize>().ok())
    else {
        return;
    };
    let Some(item) = root
        .get(collection)
        .and_then(serde_json::Value::as_array)
        .and_then(|items| items.get(index))
    else {
        return;
    };
    if let Some(id) = item.get("id").and_then(serde_json::Value::as_str) {
        diagnostic.declaration_id = Some(id.to_string());
        diagnostic.source_ids = item
            .get("origin")
            .and_then(|origin| origin.get("source_ids"))
            .or_else(|| item.get("source_ids"))
            .and_then(serde_json::Value::as_array)
            .map(|ids| {
                ids.iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        if collection == "sources" && diagnostic.source_ids.is_empty() {
            diagnostic.source_ids.push(id.to_string());
        }
    }
}

fn preflight(value: &serde_json::Value, limits: &ModelLimits) -> Result<(), Vec<Diagnostic>> {
    let Some(root) = value.as_object() else {
        return Err(vec![
            error("schema_error", "model must be an object").with_pointer(String::new()),
        ]);
    };
    check_declaration_limits(root, limits).map_err(|diagnostic| vec![diagnostic])?;
    validate_model_shapes(root).map_err(|diagnostic| vec![diagnostic])?;
    for field in ["language_version", "model_id"] {
        if !root.get(field).is_some_and(serde_json::Value::is_string) {
            return Err(vec![
                error("schema_error", format!("{field} must be a string"))
                    .with_pointer(format!("/{field}")),
            ]);
        }
    }
    let roots = expression_roots(root);
    let mut object_stack = vec![(value, String::new())];
    while let Some((node, pointer)) = object_stack.pop() {
        match node {
            serde_json::Value::Object(object) => {
                for (key, child) in object {
                    let child_pointer = format!("{pointer}/{}", pointer_segment(key));
                    if matches!(key.as_str(), "locator" | "note") && child.is_null() {
                        return Err(vec![
                            error("schema_error", format!("{key} must be omitted or a string"))
                                .with_pointer(child_pointer),
                        ]);
                    }
                    object_stack.push((child, child_pointer));
                }
            }
            serde_json::Value::Array(items) => object_stack.extend(
                items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| (item, format!("{pointer}/{index}"))),
            ),
            _ => {}
        }
    }
    // Keep recursive serde construction and type inference bounded even if a caller
    // supplies a looser limit than the crate's supported safety ceiling.
    let max_depth = limits
        .expression_depth
        .min(ModelLimits::default().expression_depth);
    let mut count = 0usize;
    let mut stack: Vec<(&serde_json::Value, String, usize)> = roots
        .into_iter()
        .map(|(expression, pointer)| (expression, pointer, 1))
        .collect();
    while let Some((expr, pointer, depth)) = stack.pop() {
        if let Some(diagnostic) = expression_shape_error(expr, &pointer) {
            return Err(vec![diagnostic]);
        }
        count = count.saturating_add(1);
        if count > limits.expression_nodes {
            return Err(vec![
                error("resource_limit", "expression node limit exceeded").with_pointer(pointer),
            ]);
        }
        if depth > max_depth {
            return Err(vec![
                error("resource_limit", "expression depth limit exceeded").with_pointer(pointer),
            ]);
        }
        if let Some(object) = expr.as_object() {
            for (key, child) in object {
                if matches!(
                    key.as_str(),
                    "arg" | "left" | "right" | "condition" | "then" | "else"
                ) {
                    stack.push((child, format!("{pointer}/{key}"), depth.saturating_add(1)));
                } else if key == "args" {
                    if let Some(args) = child.as_array() {
                        stack.extend(args.iter().enumerate().map(|(index, argument)| {
                            (
                                argument,
                                format!("{pointer}/args/{index}"),
                                depth.saturating_add(1),
                            )
                        }));
                    }
                }
            }
        }
    }
    Ok(())
}

struct DeclarationContext {
    id: String,
    pointer: String,
    source_ids: Vec<String>,
    origin_kind: Option<OriginKind>,
}

fn declaration_contexts(raw: &RawModel) -> Vec<DeclarationContext> {
    macro_rules! add_origin_contexts {
        ($contexts:ident, $field:ident, $name:literal) => {
            for (index, item) in raw.$field.iter().enumerate() {
                $contexts.push(DeclarationContext {
                    id: item.id.clone(),
                    pointer: format!(concat!("/", $name, "/{}"), index),
                    source_ids: item.origin.source_ids.clone(),
                    origin_kind: Some(item.origin.kind.clone()),
                });
            }
        };
    }
    let mut contexts = Vec::new();
    add_origin_contexts!(contexts, enums, "enums");
    add_origin_contexts!(contexts, inputs, "inputs");
    add_origin_contexts!(contexts, derived, "derived");
    add_origin_contexts!(contexts, outcomes, "outcomes");
    add_origin_contexts!(contexts, domain, "domain");
    add_origin_contexts!(contexts, assumptions, "assumptions");
    add_origin_contexts!(contexts, rules, "rules");
    add_origin_contexts!(contexts, constraints, "constraints");
    contexts.extend(
        raw.open_items
            .iter()
            .enumerate()
            .map(|(index, item)| DeclarationContext {
                id: item.id.clone(),
                pointer: format!("/open_items/{index}"),
                source_ids: item.source_ids.clone(),
                origin_kind: None,
            }),
    );
    contexts
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
    preflight(value, limits).map_err(|mut diagnostics| {
        for diagnostic in &mut diagnostics {
            attach_json_declaration_context(diagnostic, value);
        }
        diagnostics
    })?;
    let raw: RawModel = serde_json::from_value(value.clone()).map_err(|e| {
        let mut diagnostic = error("schema_error", e.to_string()).with_pointer(String::new());
        attach_json_declaration_context(&mut diagnostic, value);
        vec![diagnostic]
    })?;
    let mut ds = vec![];
    if raw.language_version != "author-model/0.1" {
        ds.push(contextual_error(
            "version_mismatch",
            "expected author-model/0.1",
            None,
            "/language_version",
            &[],
        ));
    }
    for (key, actual, limit) in [
        ("inputs", raw.inputs.len(), limits.inputs),
        ("rules", raw.rules.len(), limits.rules),
        ("constraints", raw.constraints.len(), limits.constraints),
    ] {
        if actual > limit {
            ds.push(contextual_error(
                "resource_limit",
                format!("{key} declaration limit exceeded"),
                None,
                format!("/{key}"),
                &[],
            ));
        }
    }
    let mut names = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let valid_id = |id: &str| {
        let mut chars = id.chars();
        chars.next().is_some_and(|c| c.is_ascii_alphabetic())
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    if !valid_id(&raw.model_id) {
        ds.push(contextual_error(
            "schema_error",
            format!("invalid model_id {}", raw.model_id),
            None,
            "/model_id",
            &[],
        ));
    }
    for (index, source) in raw.sources.iter().enumerate() {
        if !valid_id(&source.id) {
            ds.push(contextual_error(
                "schema_error",
                format!("invalid source identifier {}", source.id),
                Some(&source.id),
                format!("/sources/{index}/id"),
                std::slice::from_ref(&source.id),
            ));
        }
        if source.text.is_empty() {
            ds.push(contextual_error(
                "schema_error",
                format!("source {} text must not be empty", source.id),
                Some(&source.id),
                format!("/sources/{index}/text"),
                std::slice::from_ref(&source.id),
            ));
        }
    }
    let declarations = declaration_contexts(&raw);
    for declaration in &declarations {
        if !valid_id(&declaration.id) {
            ds.push(contextual_error(
                "schema_error",
                format!("invalid identifier {}", declaration.id),
                Some(&declaration.id),
                format!("{}/id", declaration.pointer),
                &declaration.source_ids,
            ));
        }
    }
    let declared_types = raw
        .inputs
        .iter()
        .enumerate()
        .map(|(index, item)| {
            (
                &item.r#type,
                item.id.as_str(),
                format!("/inputs/{index}/type"),
                item.origin.source_ids.as_slice(),
            )
        })
        .chain(raw.derived.iter().enumerate().map(|(index, item)| {
            (
                &item.r#type,
                item.id.as_str(),
                format!("/derived/{index}/type"),
                item.origin.source_ids.as_slice(),
            )
        }))
        .chain(
            raw.outcomes
                .iter()
                .enumerate()
                .flat_map(|(outcome_index, outcome)| {
                    outcome
                        .fields
                        .iter()
                        .enumerate()
                        .map(move |(field_index, field)| {
                            (
                                &field.r#type,
                                outcome.id.as_str(),
                                format!("/outcomes/{outcome_index}/fields/{field_index}/type"),
                                outcome.origin.source_ids.as_slice(),
                            )
                        })
                }),
        );
    for (ty, declaration_id, pointer, source_ids) in declared_types {
        match ty {
            Type::Enum { name } if !raw.enums.iter().any(|en| en.id == *name) => {
                ds.push(contextual_error(
                    "unknown_reference",
                    format!("unknown enum type {name}"),
                    Some(declaration_id),
                    pointer,
                    source_ids,
                ));
            }
            Type::Int { unit } if !valid_id(unit) => ds.push(contextual_error(
                "schema_error",
                format!("invalid unit {unit}"),
                Some(declaration_id),
                pointer,
                source_ids,
            )),
            _ => {}
        }
    }
    for (enum_index, en) in raw.enums.iter().enumerate() {
        let unique: BTreeSet<_> = en.variants.iter().collect();
        if en.variants.is_empty()
            || unique.len() != en.variants.len()
            || en.variants.iter().any(|variant| !valid_id(variant))
        {
            ds.push(contextual_error(
                "schema_error",
                format!("invalid variants for enum {}", en.id),
                Some(&en.id),
                format!("/enums/{enum_index}/variants"),
                &en.origin.source_ids,
            ));
        }
    }
    for (outcome_index, outcome) in raw.outcomes.iter().enumerate() {
        let fields: BTreeSet<_> = outcome.fields.iter().map(|field| &field.id).collect();
        if fields.len() != outcome.fields.len()
            || outcome.fields.iter().any(|field| !valid_id(&field.id))
        {
            ds.push(contextual_error(
                "schema_error",
                format!("invalid fields for outcome {}", outcome.id),
                Some(&outcome.id),
                format!("/outcomes/{outcome_index}/fields"),
                &outcome.origin.source_ids,
            ));
        }
    }
    for (index, source) in raw.sources.iter().enumerate() {
        if !sources.insert(source.id.clone()) {
            ds.push(contextual_error(
                "duplicate_name",
                format!("duplicate source {}", source.id),
                Some(&source.id),
                format!("/sources/{index}/id"),
                std::slice::from_ref(&source.id),
            ));
        }
    }
    for declaration in &declarations {
        if !names.insert(declaration.id.clone()) {
            ds.push(contextual_error(
                "duplicate_name",
                format!("duplicate declaration {}", declaration.id),
                Some(&declaration.id),
                format!("{}/id", declaration.pointer),
                &declaration.source_ids,
            ));
        }
    }
    for declaration in declarations
        .iter()
        .filter(|item| item.origin_kind.is_some())
    {
        let origin_pointer = format!("{}/origin", declaration.pointer);
        match declaration.origin_kind {
            Some(OriginKind::Source) if declaration.source_ids.is_empty() => {
                ds.push(contextual_error(
                    "source_reference_error",
                    "source origin requires at least one source",
                    Some(&declaration.id),
                    format!("{origin_pointer}/source_ids"),
                    &declaration.source_ids,
                ));
            }
            Some(OriginKind::Assumption)
                if !raw.assumptions.iter().any(|item| item.id == declaration.id) =>
            {
                ds.push(contextual_error(
                    "source_reference_error",
                    "assumption origin is only allowed on assumptions",
                    Some(&declaration.id),
                    format!("{origin_pointer}/kind"),
                    &declaration.source_ids,
                ));
            }
            _ => {}
        }
        for (source_index, source_id) in declaration.source_ids.iter().enumerate() {
            if !sources.contains(source_id) {
                ds.push(contextual_error(
                    "source_reference_error",
                    format!("missing source {source_id}"),
                    Some(&declaration.id),
                    format!("{origin_pointer}/source_ids/{source_index}"),
                    &declaration.source_ids,
                ));
            }
        }
    }
    for (item_index, item) in raw.open_items.iter().enumerate() {
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
            ds.push(contextual_error(
                "schema_error",
                format!("invalid open item {}", item.id),
                Some(&item.id),
                format!(
                    "/open_items/{item_index}/{}",
                    if item.text.is_empty() {
                        "text"
                    } else {
                        "affects"
                    }
                ),
                &item.source_ids,
            ));
        }
        for (source_index, source_id) in item.source_ids.iter().enumerate() {
            if !sources.contains(source_id) {
                ds.push(contextual_error(
                    "source_reference_error",
                    format!("missing source {source_id}"),
                    Some(&item.id),
                    format!("/open_items/{item_index}/source_ids/{source_index}"),
                    &item.source_ids,
                ));
            }
        }
    }
    for (index, assumption) in raw.assumptions.iter().enumerate() {
        if assumption.origin.kind != OriginKind::Assumption {
            ds.push(contextual_error(
                "source_reference_error",
                "assumptions must use assumption origin",
                Some(&assumption.id),
                format!("/assumptions/{index}/origin/kind"),
                &assumption.origin.source_ids,
            ));
        }
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
                &format!("/rules/{index}/then/fields/{}", pointer_segment(field)),
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
    for (index, derived) in raw.derived.iter().enumerate() {
        let pointer = format!("/derived/{index}/expr");
        if infer_in_context(
            &derived.expr,
            &raw,
            false,
            &mut ds,
            &derived.id,
            &pointer,
            &derived.origin.source_ids,
        )
        .as_ref()
            != Some(&derived.r#type)
        {
            ds.push(contextual_error(
                "type_mismatch",
                "derived expression has wrong type",
                Some(&derived.id),
                pointer,
                &derived.origin.source_ids,
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
            ds.push(contextual_error(
                "type_mismatch",
                "predicate must be boolean",
                Some(&p.id),
                pointer,
                &p.origin.source_ids,
            ));
        }
    }
    for (rule_index, r) in raw.rules.iter().enumerate() {
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
            ds.push(contextual_error(
                "type_mismatch",
                "rule guard must be boolean",
                Some(&r.id),
                format!("/rules/{rule_index}/when"),
                &r.origin.source_ids,
            ));
        }
        let Some(o) = raw.outcomes.iter().find(|o| o.id == r.then.outcome) else {
            ds.push(contextual_error(
                "unknown_reference",
                format!("unknown outcome {}", r.then.outcome),
                Some(&r.id),
                format!("/rules/{rule_index}/then/outcome"),
                &r.origin.source_ids,
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
                Some(_) => ds.push(contextual_error(
                    "type_mismatch",
                    format!("field {} has wrong type", f.id),
                    Some(&r.id),
                    format!("/rules/{rule_index}/then/fields/{}", pointer_segment(&f.id)),
                    &r.origin.source_ids,
                )),
                None => ds.push(contextual_error(
                    "missing_outcome_field",
                    format!("missing required field {}", f.id),
                    Some(&r.id),
                    format!("/rules/{rule_index}/then/fields/{}", pointer_segment(&f.id)),
                    &r.origin.source_ids,
                )),
            }
        }
        for field in r.then.fields.keys() {
            if !o.fields.iter().any(|f| f.id == *field) {
                ds.push(contextual_error(
                    "extra_outcome_field",
                    format!("extra outcome field {field}"),
                    Some(&r.id),
                    format!("/rules/{rule_index}/then/fields/{}", pointer_segment(field)),
                    &r.origin.source_ids,
                ));
            }
        }
    }
    for (constraint_index, c) in raw.constraints.iter().enumerate() {
        let target = raw.outcomes.iter().find(|o| o.id == c.for_outcome);
        if target.is_none() {
            ds.push(contextual_error(
                "unknown_reference",
                format!("unknown outcome {}", c.for_outcome),
                Some(&c.id),
                format!("/constraints/{constraint_index}/for_outcome"),
                &c.origin.source_ids,
            ));
        }
        let mut reference_paths = vec![];
        collect_reference_paths(
            &c.must,
            &format!("/constraints/{constraint_index}/must"),
            &mut reference_paths,
        );
        if let Some(target) = target {
            for (path, pointer) in reference_paths.iter().filter_map(|(path, pointer)| {
                path.strip_prefix("outcome.").map(|field| (field, pointer))
            }) {
                if !target.fields.iter().any(|field| field.id == path) {
                    ds.push(contextual_error(
                        "unknown_reference",
                        format!("missing outcome field outcome.{path} on {}", target.id),
                        Some(&c.id),
                        pointer.clone(),
                        &c.origin.source_ids,
                    ));
                }
            }
        }
        let mut scoped = raw.clone();
        scoped
            .outcomes
            .retain(|outcome| outcome.id == c.for_outcome);
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
            ds.push(contextual_error(
                "type_mismatch",
                "constraint must be boolean",
                Some(&c.id),
                format!("/constraints/{constraint_index}/must"),
                &c.origin.source_ids,
            ));
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
            if let Some(cycle_id) = derived_cycle_node(&raw, &pending) {
                let cycle_index = raw
                    .derived
                    .iter()
                    .position(|item| item.id == cycle_id)
                    .unwrap_or(0);
                let cycle = raw.derived.get(cycle_index);
                ds.push(contextual_error(
                    "derived_cycle",
                    format!("derived dependency graph contains a cycle involving {cycle_id}"),
                    cycle.map(|item| item.id.as_str()),
                    format!("/derived/{cycle_index}/expr"),
                    cycle.map_or(&[][..], |item| item.origin.source_ids.as_slice()),
                ));
            }
            break;
        }
        for id in ready {
            pending.remove(&id);
            done.insert(id.clone());
            order.push(id);
        }
    }
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
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
        for name in [
            "examples/models/partial.json",
            "examples/models/repaired-partial.json",
            "examples/models/unsafe-grant.json",
            "examples/models/empty-domain.json",
            "examples/models/duplicate-output.json",
            "examples/models/conflict-tag.json",
            "examples/models/conflict-payload.json",
            "examples/models/conditional-assumption.json",
            "examples/models/complete-proposed.json",
            "examples/models/big-integer.json",
        ] {
            let v: serde_json::Value =
                serde_json::from_slice(&std::fs::read(base.join(name)).unwrap()).unwrap();
            assert!(compile_json(&v, &ModelLimits::default()).is_ok(), "{name}");
        }
        for name in [
            "fixtures/invalid/unsupported-node.json",
            "fixtures/invalid/unknown-reference.json",
            "fixtures/invalid/unit-mismatch.json",
            "fixtures/invalid/invalid-date.json",
            "fixtures/invalid/derived-cycle.json",
        ] {
            let v: serde_json::Value =
                serde_json::from_slice(&std::fs::read(base.join(name)).unwrap()).unwrap();
            assert!(compile_json(&v, &ModelLimits::default()).is_err(), "{name}");
        }
    }

    #[test]
    fn invalid_fixtures_match_authoritative_expected_diagnostic_codes() {
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
        let expectations: serde_json::Value = serde_json::from_slice(
            &std::fs::read(base.join("fixtures/expectations.json")).unwrap(),
        )
        .unwrap();

        for (fixture, expectation) in expectations["invalid"].as_object().unwrap() {
            let value: serde_json::Value = serde_json::from_slice(
                &std::fs::read(base.join(format!("fixtures/invalid/{fixture}.json"))).unwrap(),
            )
            .unwrap();
            let expected_code = expectation["expected_diagnostic"].as_str().unwrap();
            let diagnostics = compile_json(&value, &ModelLimits::default())
                .expect_err(&format!("invalid fixture {fixture} compiled successfully"));
            let diagnostic = diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == expected_code)
                .unwrap_or_else(|| {
                    panic!(
                        "{fixture}: expected diagnostic code {expected_code}, got {:?}",
                        diagnostics
                            .iter()
                            .map(|diagnostic| &diagnostic.code)
                            .collect::<Vec<_>>()
                    )
                });
            if fixture == "unsupported-node" {
                assert!(
                    diagnostic
                        .message
                        .contains("unsupported expression kind python")
                );
                assert_eq!(diagnostic.pointer.as_deref(), Some("/rules/0/when/kind"));
                assert_eq!(
                    diagnostic.declaration_id.as_deref(),
                    Some("GrantApprovedRequest")
                );
                assert!(diagnostic.source_ids.is_empty());
            }
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
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
        let model_value: serde_json::Value = serde_json::from_slice(
            &std::fs::read(base.join("examples/models/unsafe-grant.json")).unwrap(),
        )
        .unwrap();
        let model = compile_json(&model_value, &ModelLimits::default()).unwrap();
        let scenario: serde_json::Value = serde_json::from_slice(
            &std::fs::read(base.join("examples/scenarios/unsafe-expiry.json")).unwrap(),
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
            .join("../../docs/examples/models/partial.json");
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
            .join("../../docs/examples/models/partial.json");
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
        for _ in 0..256 {
            expr = serde_json::json!({"kind":"not", "arg":expr});
        }
        value["constraints"][0]["must"] = expr;
        let errors = compile_json(&value, &ModelLimits::default()).unwrap_err();
        assert_eq!(errors[0].code, "resource_limit");
        assert_eq!(
            errors[0].pointer.as_deref(),
            Some(format!("/constraints/0/must{}", "/arg".repeat(128)).as_str())
        );
    }

    #[test]
    fn declaration_count_limit_precedes_declaration_shape_validation() {
        let mut value = fixture();
        value["inputs"] = serde_json::json!([{"unexpected":true}, {"unexpected":true}]);
        let limits = ModelLimits {
            inputs: 1,
            ..ModelLimits::default()
        };
        let diagnostic = compile_json(&value, &limits).unwrap_err().remove(0);
        assert_eq!(diagnostic.code, "resource_limit");
        assert_eq!(diagnostic.pointer.as_deref(), Some("/inputs"));
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

    #[test]
    fn diagnostics_keep_exact_context_for_overlapping_short_ids() {
        let mut value = fixture();
        value["rules"] = serde_json::json!([
            {"id":"A", "when":{"kind":"int", "value":"1", "unit":"days"}, "then":{"outcome":"Grant", "fields":{"expiry":{"kind":"date", "value":"2025-01-01"}}}, "origin":{"kind":"source", "source_ids":["S1"], "review":"confirmed"}},
            {"id":"AA", "when":{"kind":"int", "value":"1", "unit":"days"}, "then":{"outcome":"Grant", "fields":{"expiry":{"kind":"date", "value":"2025-01-01"}}}, "origin":{"kind":"source", "source_ids":["S2"], "review":"confirmed"}}
        ]);
        let diagnostics = compile_json(&value, &ModelLimits::default()).unwrap_err();
        for (id, pointer, source_id) in
            [("A", "/rules/0/when", "S1"), ("AA", "/rules/1/when", "S2")]
        {
            let diagnostic = diagnostics
                .iter()
                .find(|diagnostic| {
                    diagnostic.code == "type_mismatch"
                        && diagnostic.message == "rule guard must be boolean"
                        && diagnostic.declaration_id.as_deref() == Some(id)
                })
                .unwrap();
            assert_eq!(diagnostic.pointer.as_deref(), Some(pointer));
            assert_eq!(diagnostic.source_ids, [source_id]);
        }
    }

    #[test]
    fn nested_unknown_reference_has_precise_path_and_owner_provenance() {
        let mut value = fixture();
        value["constraints"][0]["must"] = serde_json::json!({
            "kind":"not", "arg":{"kind":"ref", "path":"input.Missing"}
        });
        let diagnostics = compile_json(&value, &ModelLimits::default()).unwrap_err();
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "unknown_reference")
            .unwrap();
        assert_eq!(
            diagnostic.declaration_id.as_deref(),
            Some("ApprovalRequired")
        );
        assert_eq!(
            diagnostic.pointer.as_deref(),
            Some("/constraints/0/must/arg/path")
        );
        assert_eq!(diagnostic.source_ids, ["S1"]);
    }

    #[test]
    fn origin_source_requirement_and_cycle_report_exact_declaration_context() {
        let mut value = fixture();
        value["inputs"][0]["origin"]["source_ids"] = serde_json::json!([]);
        let diagnostics = compile_json(&value, &ModelLimits::default()).unwrap_err();
        let origin = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "source_reference_error")
            .unwrap();
        assert_eq!(origin.declaration_id.as_deref(), Some("sponsor_approval"));
        assert_eq!(
            origin.pointer.as_deref(),
            Some("/inputs/0/origin/source_ids")
        );

        value = fixture();
        value["derived"] = serde_json::json!([
            {"id":"Aardvark", "type":{"kind":"int", "unit":"days"}, "expr":{"kind":"ref", "path":"derived.CycleA"}, "origin":{"kind":"source", "source_ids":["S2"], "review":"confirmed"}},
            {"id":"CycleA", "type":{"kind":"int", "unit":"days"}, "expr":{"kind":"ref", "path":"derived.CycleB"}, "origin":{"kind":"source", "source_ids":["S1"], "review":"confirmed"}},
            {"id":"CycleB", "type":{"kind":"int", "unit":"days"}, "expr":{"kind":"ref", "path":"derived.CycleA"}, "origin":{"kind":"source", "source_ids":["S2"], "review":"confirmed"}}
        ]);
        let diagnostics = compile_json(&value, &ModelLimits::default()).unwrap_err();
        let cycle = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "derived_cycle")
            .unwrap();
        assert_eq!(cycle.declaration_id.as_deref(), Some("CycleA"));
        assert_eq!(cycle.pointer.as_deref(), Some("/derived/1/expr"));
        assert_eq!(cycle.source_ids, ["S1"]);
    }

    #[test]
    fn malformed_nested_if_reports_structural_json_pointer() {
        let mut value = fixture();
        value["constraints"][0]["must"] = serde_json::json!({
            "kind":"if", "condition":{"kind":"bool", "value":true},
            "then":{"kind":"bool", "value":true}, "else":{"kind":"bool", "value":false},
            "extra":true
        });
        let diagnostic = compile_json(&value, &ModelLimits::default())
            .unwrap_err()
            .remove(0);
        assert_eq!(diagnostic.code, "schema_error");
        assert_eq!(
            diagnostic.pointer.as_deref(),
            Some("/constraints/0/must/extra")
        );
    }

    #[test]
    fn raw_model_serialization_round_trips_without_null_optional_metadata() {
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs");
        for name in [
            "examples/models/partial.json",
            "examples/models/repaired-partial.json",
            "examples/models/unsafe-grant.json",
            "examples/models/empty-domain.json",
            "examples/models/duplicate-output.json",
            "examples/models/conflict-tag.json",
            "examples/models/conflict-payload.json",
            "examples/models/conditional-assumption.json",
            "examples/models/complete-proposed.json",
            "examples/models/big-integer.json",
        ] {
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(base.join(name)).unwrap()).unwrap();
            let first = compile_json(&value, &ModelLimits::default()).unwrap();
            let serialized = serde_json::to_value(&first.raw).unwrap();
            let serialized_text = serde_json::to_string(&serialized).unwrap();
            assert!(!serialized_text.contains("\"locator\":null"), "{name}");
            assert!(!serialized_text.contains("\"note\":null"), "{name}");
            let second = compile_json(&serialized, &ModelLimits::default()).unwrap();
            assert_eq!(first.content_hash, second.content_hash, "{name}");
            assert_eq!(first.semantic_hash, second.semantic_hash, "{name}");
        }
    }

    #[test]
    fn nested_type_unit_date_and_variant_errors_point_at_the_expression_node() {
        let expressions = [
            (
                "invalid_date",
                serde_json::json!({"kind":"eq", "left":{"kind":"date", "value":"2025-02-30"}, "right":{"kind":"date", "value":"2025-02-28"}}),
                "/constraints/0/must/left",
            ),
            (
                "unknown_variant",
                serde_json::json!({"kind":"eq", "left":{"kind":"enum", "enum":"ApprovalStatus", "variant":"Missing"}, "right":{"kind":"enum", "enum":"ApprovalStatus", "variant":"Approved"}}),
                "/constraints/0/must/left",
            ),
            (
                "unit_mismatch",
                serde_json::json!({"kind":"eq", "left":{"kind":"int", "value":"1", "unit":"days"}, "right":{"kind":"int", "value":"1", "unit":"hours"}}),
                "/constraints/0/must",
            ),
        ];
        for (code, expression, pointer) in expressions {
            let mut value = fixture();
            value["constraints"][0]["must"] = expression;
            let diagnostics = compile_json(&value, &ModelLimits::default()).unwrap_err();
            let diagnostic = diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == code)
                .unwrap();
            assert_eq!(
                diagnostic.declaration_id.as_deref(),
                Some("ApprovalRequired")
            );
            assert_eq!(diagnostic.pointer.as_deref(), Some(pointer));
            assert_eq!(diagnostic.source_ids, ["S1"]);
        }
    }

    #[test]
    fn structural_schema_cases_have_exact_pointers() {
        let mut cases = Vec::new();
        let mut missing_top_level = fixture();
        missing_top_level
            .as_object_mut()
            .unwrap()
            .remove("model_id");
        cases.push((missing_top_level, "/model_id"));
        let mut extra_top_level = fixture();
        extra_top_level["unexpected"] = serde_json::json!(true);
        cases.push((extra_top_level, "/unexpected"));
        let mut missing_if_branch = fixture();
        missing_if_branch["constraints"][0]["must"] = serde_json::json!({
            "kind":"if", "condition":{"kind":"bool", "value":true},
            "then":{"kind":"bool", "value":true}
        });
        cases.push((missing_if_branch, "/constraints/0/must/else"));
        let mut unknown_type_field = fixture();
        unknown_type_field["inputs"][0]["type"]["unexpected"] = serde_json::json!(true);
        cases.push((unknown_type_field, "/inputs/0/type/unexpected"));
        let mut null_optional = fixture();
        null_optional["sources"][0]["locator"] = serde_json::Value::Null;
        cases.push((null_optional, "/sources/0/locator"));
        for (value, expected_pointer) in cases {
            let diagnostics = compile_json(&value, &ModelLimits::default()).unwrap_err();
            assert_eq!(diagnostics[0].pointer.as_deref(), Some(expected_pointer));
        }
        let mut invalid_identifier = fixture();
        invalid_identifier["model_id"] = serde_json::json!("1invalid");
        let diagnostic = compile_json(&invalid_identifier, &ModelLimits::default())
            .unwrap_err()
            .remove(0);
        assert_eq!(diagnostic.pointer.as_deref(), Some("/model_id"));
    }

    #[test]
    fn outcome_field_completeness_reports_rule_and_field_context() {
        let mut value = fixture();
        value["rules"] = serde_json::json!([
            {"id":"Incomplete", "when":{"kind":"bool", "value":true}, "then":{"outcome":"Grant", "fields":{}}, "origin":{"kind":"source", "source_ids":["S1"], "review":"confirmed"}}
        ]);
        let diagnostics = compile_json(&value, &ModelLimits::default()).unwrap_err();
        let missing = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "missing_outcome_field")
            .unwrap();
        assert_eq!(missing.declaration_id.as_deref(), Some("Incomplete"));
        assert_eq!(
            missing.pointer.as_deref(),
            Some("/rules/0/then/fields/expiry")
        );
        assert_eq!(missing.source_ids, ["S1"]);
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
