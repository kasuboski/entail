use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
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
    Not { arg: Box<Expr> },
    #[serde(rename = "and")]
    And { args: Vec<Expr> },
    #[serde(rename = "or")]
    Or { args: Vec<Expr> },
    #[serde(rename = "eq")]
    Eq { left: Box<Expr>, right: Box<Expr> },
    #[serde(rename = "ne")]
    Ne { left: Box<Expr>, right: Box<Expr> },
    #[serde(rename = "lt")]
    Lt { left: Box<Expr>, right: Box<Expr> },
    #[serde(rename = "le")]
    Le { left: Box<Expr>, right: Box<Expr> },
    #[serde(rename = "gt")]
    Gt { left: Box<Expr>, right: Box<Expr> },
    #[serde(rename = "ge")]
    Ge { left: Box<Expr>, right: Box<Expr> },
    #[serde(rename = "add")]
    Add { left: Box<Expr>, right: Box<Expr> },
    #[serde(rename = "sub")]
    Sub { left: Box<Expr>, right: Box<Expr> },
    #[serde(rename = "scale")]
    Scale { factor: String, arg: Box<Expr> },
    #[serde(rename = "if")]
    If {
        condition: Box<Expr>,
        #[serde(rename = "then")]
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
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
#[derive(Clone, Debug)]
pub struct CompiledModel {
    pub raw: RawModel,
    pub derived_order: Vec<String>,
    pub content_hash: String,
    pub semantic_hash: String,
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
        .filter(|d| d.format("%Y-%m-%d").to_string() == s)
}
fn expr_count(e: &Expr, depth: usize, count: &mut usize, maxdepth: usize) -> bool {
    *count = count.saturating_add(1);
    if depth > maxdepth {
        return false;
    }
    let child = |x: &Expr, c: &mut usize| expr_count(x, depth + 1, c, maxdepth);
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
                refs(x, out)
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
            refs(right, out)
        }
        Expr::If {
            condition,
            then_expr,
            else_expr,
        } => {
            refs(condition, out);
            refs(then_expr, out);
            refs(else_expr, out)
        }
        _ => {}
    }
}
fn check_refs(e: &Expr, raw: &RawModel, allow_outcome: bool, ds: &mut Vec<Diagnostic>) {
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
            ds.push(error(
                "unknown_reference",
                format!("unknown or out-of-scope reference {path}"),
            ));
        }
    }
}
fn infer(e: &Expr, raw: &RawModel, allow_outcome: bool, ds: &mut Vec<Diagnostic>) -> Option<Type> {
    let child = |x: &Expr, d: &mut Vec<Diagnostic>| infer(x, raw, allow_outcome, d);
    match e {
        Expr::Bool { .. } => Some(Type::Bool),
        Expr::Int { value, unit } => {
            if !canonical_integer(value) {
                ds.push(error(
                    "type_mismatch",
                    "integer literal must be canonical decimal",
                ));
            }
            Some(Type::Int { unit: unit.clone() })
        }
        Expr::Date { value } => {
            if parse_date(value).is_none() {
                ds.push(error(
                    "invalid_date",
                    format!("invalid date literal {value}"),
                ));
            }
            Some(Type::Date)
        }
        Expr::Enum { enum_name, variant } => {
            if !raw
                .enums
                .iter()
                .any(|x| x.id == *enum_name && x.variants.contains(variant))
            {
                ds.push(error(
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
            if child(arg, ds) != Some(Type::Bool) {
                ds.push(error("type_mismatch", "not requires bool"));
            }
            Some(Type::Bool)
        }
        Expr::And { args } | Expr::Or { args } => {
            if args.is_empty() {
                ds.push(error(
                    "type_mismatch",
                    "and/or requires at least one argument",
                ));
            }
            for x in args {
                if child(x, ds) != Some(Type::Bool) {
                    ds.push(error("type_mismatch", "and/or arguments must be bool"));
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
            let a = child(left, ds);
            let b = child(right, ds);
            if a != b {
                if matches!((&a,&b),(Some(Type::Int{unit:u}),Some(Type::Int{unit:v})) if u!=v) {
                    ds.push(error("unit_mismatch", "integer units differ"));
                } else {
                    ds.push(error("type_mismatch", "comparison operands differ in type"));
                }
            }
            if matches!(
                e,
                Expr::Lt { .. } | Expr::Le { .. } | Expr::Gt { .. } | Expr::Ge { .. }
            ) && !matches!(a, Some(Type::Int { .. } | Type::Date))
            {
                ds.push(error(
                    "type_mismatch",
                    "ordered comparison requires int or date",
                ));
            }
            Some(Type::Bool)
        }
        Expr::Add { left, right } | Expr::Sub { left, right } => {
            let a = child(left, ds);
            let b = child(right, ds);
            if a != b {
                if matches!((&a,&b),(Some(Type::Int{unit:u}),Some(Type::Int{unit:v})) if u!=v) {
                    ds.push(error("unit_mismatch", "integer units differ"));
                } else {
                    ds.push(error("type_mismatch", "arithmetic operands differ"));
                }
            }
            if !matches!(a, Some(Type::Int { .. })) {
                ds.push(error("type_mismatch", "arithmetic requires integers"));
            }
            a
        }
        Expr::Scale { factor, arg } => {
            if !canonical_integer(factor) {
                ds.push(error(
                    "type_mismatch",
                    "scale factor must be canonical integer",
                ));
            }
            let a = child(arg, ds);
            if !matches!(a, Some(Type::Int { .. })) {
                ds.push(error("type_mismatch", "scale requires integer"));
            }
            a
        }
        Expr::If {
            condition,
            then_expr,
            else_expr,
        } => {
            if child(condition, ds) != Some(Type::Bool) {
                ds.push(error("type_mismatch", "if condition must be bool"));
            }
            let a = child(then_expr, ds);
            let b = child(else_expr, ds);
            if a != b {
                ds.push(error("type_mismatch", "if branches differ"));
            }
            a
        }
    }
}

pub fn compile_json(
    value: &serde_json::Value,
    limits: &ModelLimits,
) -> Result<CompiledModel, Vec<Diagnostic>> {
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
                ds.push(error("schema_error", format!("invalid unit {unit}")))
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
    let mut add = |id: &str| {
        if !names.insert(id.to_string()) {
            ds.push(error(
                "duplicate_name",
                format!("duplicate declaration {id}"),
            ));
        }
    };
    for e in &raw.enums {
        add(&e.id)
    }
    for i in &raw.inputs {
        add(&i.id)
    }
    for d in &raw.derived {
        add(&d.id)
    }
    for o in &raw.outcomes {
        add(&o.id)
    }
    for p in &raw.domain {
        add(&p.id)
    }
    for p in &raw.assumptions {
        add(&p.id)
    }
    for r in &raw.rules {
        add(&r.id)
    }
    for c in &raw.constraints {
        add(&c.id)
    }
    for o in &raw.open_items {
        add(&o.id)
    }
    drop(add);
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
        all.push(&d.expr)
    }
    for p in &raw.domain {
        all.push(&p.expr)
    }
    for p in &raw.assumptions {
        all.push(&p.expr)
    }
    for r in &raw.rules {
        all.push(&r.when);
        all.extend(r.then.fields.values())
    }
    for c in &raw.constraints {
        all.push(&c.must)
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
    for d in &raw.derived {
        check_refs(&d.expr, &raw, false, &mut ds);
    }
    for p in &raw.domain {
        check_refs(&p.expr, &raw, false, &mut ds);
    }
    for p in &raw.assumptions {
        check_refs(&p.expr, &raw, false, &mut ds);
    }
    for r in &raw.rules {
        check_refs(&r.when, &raw, false, &mut ds);
        for e in r.then.fields.values() {
            check_refs(e, &raw, false, &mut ds);
        }
    }
    for c in &raw.constraints {
        check_refs(&c.must, &raw, true, &mut ds);
    }
    for d in &raw.derived {
        if infer(&d.expr, &raw, false, &mut ds).as_ref() != Some(&d.r#type) {
            ds.push(error(
                "type_mismatch",
                format!("derived {} expression has wrong type", d.id),
            ));
        }
    }
    for p in raw.domain.iter().chain(raw.assumptions.iter()) {
        if infer(&p.expr, &raw, false, &mut ds) != Some(Type::Bool) {
            ds.push(error(
                "type_mismatch",
                format!("predicate {} must be boolean", p.id),
            ));
        }
    }
    for r in &raw.rules {
        if infer(&r.when, &raw, false, &mut ds) != Some(Type::Bool) {
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
                Some(e) if infer(e, &raw, false, &mut ds).as_ref() == Some(&f.r#type) => {}
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
        if !raw.outcomes.iter().any(|o| o.id == c.for_outcome) {
            ds.push(error(
                "unknown_reference",
                format!("constraint {} names unknown outcome", c.id),
            ));
        }
        if infer(&c.must, &raw, true, &mut ds) != Some(Type::Bool) {
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
            ds.push(error(
                "derived_cycle",
                "derived dependency graph contains a cycle",
            ));
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
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(m) => {
                m.remove("origin");
                m.remove("note");
                for x in m.values_mut() {
                    strip(x)
                }
            }
            serde_json::Value::Array(a) => {
                for x in a {
                    strip(x)
                }
            }
            _ => {}
        }
    }
    strip(&mut semantic);
    Ok(CompiledModel {
        raw,
        derived_order: order,
        content_hash,
        semantic_hash: hash(&semantic),
    })
}

fn value_type(v: &Value) -> Option<Type> {
    match v {
        Value::Bool { .. } => Some(Type::Bool),
        Value::Int { unit, .. } => Some(Type::Int { unit: unit.clone() }),
        Value::Date { .. } => Some(Type::Date),
        Value::Enum { enum_name, .. } => Some(Type::Enum {
            name: enum_name.clone(),
        }),
    }
}
pub fn validate_inputs(model: &CompiledModel, inputs: &InputValues) -> Result<(), Diagnostic> {
    let decl: BTreeMap<_, _> = model
        .raw
        .inputs
        .iter()
        .map(|x| (x.id.as_str(), &x.r#type))
        .collect();
    for (id, t) in &decl {
        let Some(v) = inputs.get(*id) else {
            return Err(error("missing_input", format!("missing input {id}")));
        };
        if value_type(v).as_ref() != Some(*t) {
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
                    .raw
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
pub fn eval_expr(
    expr: &Expr,
    inputs: &InputValues,
    derived: &BTreeMap<String, Value>,
    outcome_fields: &BTreeMap<String, Value>,
) -> Result<Value, Diagnostic> {
    let ev = |e: &Expr| eval_expr(e, inputs, derived, outcome_fields);
    let boolean = |v: Value| {
        if let Value::Bool { value } = v {
            Ok(value)
        } else {
            Err(error("type_mismatch", "expected boolean"))
        }
    };
    let int = |v: Value| {
        if let Value::Int { value, unit } = v {
            BigInt::parse_bytes(value.as_bytes(), 10)
                .map(|n| (n, unit))
                .ok_or_else(|| error("invalid_value", "invalid integer"))
        } else {
            Err(error("type_mismatch", "expected integer"))
        }
    };
    let b = |x| Ok(Value::Bool { value: x });
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
        Expr::Not { arg } => b(!boolean(ev(arg)?)?),
        Expr::And { args } => {
            for a in args {
                if !boolean(ev(a)?)? {
                    return b(false);
                }
            }
            b(true)
        }
        Expr::Or { args } => {
            for a in args {
                if boolean(ev(a)?)? {
                    return b(true);
                }
            }
            b(false)
        }
        Expr::Eq { left, right } => b(ev(left)? == ev(right)?),
        Expr::Ne { left, right } => b(ev(left)? != ev(right)?),
        Expr::Lt { left, right }
        | Expr::Le { left, right }
        | Expr::Gt { left, right }
        | Expr::Ge { left, right } => {
            let l = ev(left)?;
            let r = ev(right)?;
            let ord = match (&l, &r) {
                (Value::Int { value: a, unit: u }, Value::Int { value: c, unit: v }) if u == v => {
                    BigInt::parse_bytes(a.as_bytes(), 10)
                        .and_then(|x| BigInt::parse_bytes(c.as_bytes(), 10).map(|y| x.cmp(&y)))
                }
                (Value::Date { value: a }, Value::Date { value: c }) => {
                    parse_date(a).zip(parse_date(c)).map(|(x, y)| x.cmp(&y))
                }
                _ => None,
            }
            .ok_or_else(|| error("type_mismatch", "incompatible ordered values"))?;
            b(match expr {
                Expr::Lt { .. } => ord.is_lt(),
                Expr::Le { .. } => !ord.is_gt(),
                Expr::Gt { .. } => ord.is_gt(),
                _ => !ord.is_lt(),
            })
        }
        Expr::Add { left, right } | Expr::Sub { left, right } => {
            let (a, u) = int(ev(left)?)?;
            let (c, v) = int(ev(right)?)?;
            if u != v {
                return Err(error("unit_mismatch", "integer units differ"));
            }
            let n = if matches!(expr, Expr::Add { .. }) {
                a + c
            } else {
                a - c
            };
            Ok(Value::Int {
                value: n.to_string(),
                unit: u,
            })
        }
        Expr::Scale { factor, arg } => {
            let (n, u) = int(ev(arg)?)?;
            if !canonical_integer(factor) {
                return Err(error("invalid_value", "invalid scale factor"));
            }
            let f = BigInt::parse_bytes(factor.as_bytes(), 10)
                .ok_or_else(|| error("invalid_value", "invalid scale factor"))?;
            Ok(Value::Int {
                value: (n * f).to_string(),
                unit: u,
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
pub fn compute_derived(
    model: &CompiledModel,
    inputs: &InputValues,
) -> Result<BTreeMap<String, Value>, Diagnostic> {
    validate_inputs(model, inputs)?;
    let mut vals = BTreeMap::new();
    for id in &model.derived_order {
        let d = model
            .raw
            .derived
            .iter()
            .find(|x| x.id == *id)
            .ok_or_else(|| error("internal_error", "missing derived declaration"))?;
        let v = eval_expr(&d.expr, inputs, &vals, &BTreeMap::new())?;
        if value_type(&v).as_ref() != Some(&d.r#type) {
            return Err(error(
                "type_mismatch",
                format!("derived {id} type mismatch"),
            ));
        }
        vals.insert(id.clone(), v);
    }
    Ok(vals)
}
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
            .raw
            .assumptions
            .iter()
            .find(|x| x.id == *id)
            .ok_or_else(|| error("unknown_reference", format!("unknown assumption {id}")))?;
        out.push(p.clone());
    }
    Ok(out)
}
pub fn in_domain(
    model: &CompiledModel,
    inputs: &InputValues,
    derived: &BTreeMap<String, Value>,
    assumptions: &[String],
) -> Result<bool, Diagnostic> {
    for p in &model.raw.domain {
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
    for r in &model.raw.rules {
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
            .raw
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
        assert_ne!(first.content_hash, second.content_hash);
        assert_eq!(first.semantic_hash, second.semantic_hash);
    }
}
