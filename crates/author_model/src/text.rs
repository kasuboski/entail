//! Readable, line-oriented frontend for the author model.

// Parser cursor arithmetic is bounded by the corresponding token/string checks;
// the formatter also intentionally appends small preformatted fragments.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::format_push_string,
    clippy::too_many_lines
)]

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    CompiledModel, Constraint, DerivedDecl, Diagnostic, EnumDecl, Expr, FieldDecl, InputDecl,
    ModelLimits, OpenItem, Origin, OriginKind, OutcomeDecl, Predicate, RawModel, Review, Rule,
    RuleConclusion, Source, Type, compile_json,
};

/// Parse readable model text into its typed, unvalidated representation.
///
/// # Errors
/// Returns a syntax diagnostic when a declaration is malformed.
pub fn parse_text(source: &str) -> Result<RawModel, Vec<Diagnostic>> {
    Parser::new(source).parse()
}

/// Parse and validate a readable model.
///
/// # Errors
/// Returns syntax or semantic diagnostics.
pub fn compile_text(source: &str, limits: &ModelLimits) -> Result<CompiledModel, Vec<Diagnostic>> {
    let raw = parse_text(source)?;
    let value = serde_json::to_value(raw).map_err(|error| vec![diagnostic(error.to_string())])?;
    compile_json(&value, limits)
}

/// Render a raw model in canonical readable syntax.
///
/// # Errors
/// Returns a formatting diagnostic if an expression cannot be rendered.
pub fn format_text(model: &RawModel) -> Result<String, Diagnostic> {
    let mut out = format!(
        "decision {}\nversion {}\n",
        model.model_id,
        quote(&model.language_version)
    );
    for source in &model.sources {
        out.push_str(&format!(
            "source {} text {}",
            source.id,
            quote(&source.text)
        ));
        if let Some(locator) = &source.locator {
            out.push_str(&format!(" locator {}", quote(locator)));
        }
        out.push('\n');
    }
    for item in &model.enums {
        out.push_str(&format!(
            "type {} = {} {}\n",
            item.id,
            item.variants.join(" | "),
            origin_text(&item.origin)
        ));
    }
    for item in &model.inputs {
        out.push_str(&format!(
            "input {}: {} {}\n",
            item.id,
            type_text(&item.r#type),
            origin_text(&item.origin)
        ));
    }
    for item in &model.derived {
        out.push_str(&format!(
            "derived {}: {} = {} {}\n",
            item.id,
            type_text(&item.r#type),
            expr_text(&item.expr)?,
            origin_text(&item.origin)
        ));
    }
    for item in &model.outcomes {
        let fields = item
            .fields
            .iter()
            .map(|field| format!("{}: {}", field.id, type_text(&field.r#type)))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "outcome {}({fields}) {}\n",
            item.id,
            origin_text(&item.origin)
        ));
    }
    for item in &model.domain {
        out.push_str(&format!(
            "domain {} require {} {}\n",
            item.id,
            expr_text(&item.expr)?,
            origin_text(&item.origin)
        ));
    }
    for item in &model.assumptions {
        out.push_str(&format!(
            "assumption {} require {} {}\n",
            item.id,
            expr_text(&item.expr)?,
            origin_text(&item.origin)
        ));
    }
    for item in &model.rules {
        let status = match item.origin.review {
            Review::Proposed => "proposed",
            Review::Confirmed => "confirmed",
        };
        out.push_str(&format!(
            "rule {}\n  when {}\n  conclude {}({})\n  status {}\n  {}\n",
            item.id,
            expr_text(&item.when)?,
            item.then.outcome,
            fields_text(&item.then.fields)?,
            status,
            origin_text(&item.origin)
        ));
    }
    for item in &model.constraints {
        let fields = model
            .outcomes
            .iter()
            .find(|outcome| outcome.id == item.for_outcome)
            .map(|outcome| {
                outcome
                    .fields
                    .iter()
                    .map(|field| field.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        out.push_str(&format!(
            "constraint {}\n  when outcome is {}({fields})\n  require {}\n",
            item.id,
            item.for_outcome,
            expr_text(&item.must)?
        ));
        for source_id in &item.origin.source_ids {
            if let Some(source) = model.sources.iter().find(|source| source.id == *source_id) {
                let label =
                    source
                        .locator
                        .as_deref()
                        .filter(|locator| {
                            !model.sources.iter().any(|candidate| {
                                candidate.id == *locator && candidate.id != source.id
                            }) && model
                                .sources
                                .iter()
                                .filter(|candidate| candidate.locator.as_deref() == Some(*locator))
                                .count()
                                == 1
                        })
                        .unwrap_or(&source.id);
                out.push_str(&format!("  source {}\n", source_label(label)));
            }
        }
        out.push_str(&format!("  {}\n", origin_text(&item.origin)));
    }
    for item in &model.open_items {
        out.push_str(&format!(
            "unresolved {} affects {} sources [{}]\n  {}\n",
            item.id,
            item.affects,
            item.source_ids.join(", "),
            quote(&item.text)
        ));
    }
    Ok(out)
}

fn fields_text(fields: &BTreeMap<String, Expr>) -> Result<String, Diagnostic> {
    fields
        .iter()
        .map(|(key, value)| Ok(format!("{key} = {}", expr_text(value)?)))
        .collect::<Result<Vec<_>, Diagnostic>>()
        .map(|values| values.join(", "))
}

fn type_text(ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".to_owned(),
        Type::Enum { name } if matches!(name.as_str(), "Bool" | "Date") => format!("Enum[{name}]"),
        Type::Enum { name } => name.clone(),
        Type::Int { unit } => format!("Int[{unit}]"),
        Type::Date => "Date".to_owned(),
    }
}

fn origin_text(origin: &Origin) -> String {
    let kind = match origin.kind {
        OriginKind::Source => "source",
        OriginKind::Proposal => "proposal",
        OriginKind::Assumption => "assumption",
    };
    let review = match origin.review {
        Review::Proposed => "proposed",
        Review::Confirmed => "confirmed",
    };
    let mut result = format!(
        "origin {kind} review {review} sources [{}]",
        origin.source_ids.join(", ")
    );
    if let Some(note) = &origin.note {
        result.push_str(&format!(" note {}", quote(note)));
    }
    result
}

fn expr_text(expr: &Expr) -> Result<String, Diagnostic> {
    let binary = |op: &str, a: &Expr, b: &Expr| -> Result<String, Diagnostic> {
        Ok(format!("({} {op} {})", expr_text(a)?, expr_text(b)?))
    };
    Ok(match expr {
        Expr::Bool { value } => value.to_string(),
        Expr::Int { value, unit } => format!("{value}[{unit}]"),
        Expr::Date { value } => format!("date({})", quote(value)),
        Expr::Enum { enum_name, variant } => format!("{enum_name}::{variant}"),
        Expr::Ref { path } => path.clone(),
        Expr::Not { arg } => format!("not ({})", expr_text(arg)?),
        Expr::And { args } => join_expr(args, "and")?,
        Expr::Or { args } => join_expr(args, "or")?,
        Expr::Eq { left, right } => binary("==", left, right)?,
        Expr::Ne { left, right } => binary("!=", left, right)?,
        Expr::Lt { left, right } => binary("<", left, right)?,
        Expr::Le { left, right } => binary("<=", left, right)?,
        Expr::Gt { left, right } => binary(">", left, right)?,
        Expr::Ge { left, right } => binary(">=", left, right)?,
        Expr::Add { left, right } => binary("+", left, right)?,
        Expr::Sub { left, right } => binary("-", left, right)?,
        Expr::Scale { factor, arg } => format!("scale({factor}, {})", expr_text(arg)?),
        Expr::ApplyRate { rate, quantity } => {
            format!("apply_rate({}, {})", expr_text(rate)?, expr_text(quantity)?)
        }
        Expr::If {
            condition,
            then_expr,
            else_expr,
        } => format!(
            "(if {} then {} else {})",
            expr_text(condition)?,
            expr_text(then_expr)?,
            expr_text(else_expr)?
        ),
    })
}

fn join_expr(args: &[Expr], operator: &str) -> Result<String, Diagnostic> {
    args.iter()
        .map(expr_text)
        .collect::<Result<Vec<_>, _>>()
        .map(|values| format!("{operator}({})", values.join(", ")))
}

fn quote(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
}

fn source_label(value: &str) -> String {
    if !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
        })
    {
        value.to_owned()
    } else {
        quote(value)
    }
}

fn diagnostic(message: String) -> Diagnostic {
    Diagnostic {
        code: "syntax_error".to_owned(),
        message,
        pointer: None,
        declaration_id: None,
        source_ids: Vec::new(),
    }
}

struct Parser {
    lines: Vec<String>,
    line_numbers: Vec<usize>,
    current_line: Cell<usize>,
}

impl Parser {
    fn new(source: &str) -> Self {
        let (line_numbers, lines) = source
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, line.trim()))
            .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
            .map(|(index, line)| (index, line.to_owned()))
            .unzip();
        Self {
            lines,
            line_numbers,
            current_line: Cell::new(0),
        }
    }

    fn parse(self) -> Result<RawModel, Vec<Diagnostic>> {
        self.parse_inner().map_err(|mut diagnostics| {
            let index = self.current_line.get();
            let line_number = self.line_numbers.get(index).copied().unwrap_or(1);
            for diagnostic in &mut diagnostics {
                diagnostic.pointer = Some(format!("text:line:{line_number}"));
                if let Some(line) = self.lines.get(index) {
                    diagnostic.declaration_id = line
                        .split_whitespace()
                        .nth(1)
                        .map(|name| name.trim_end_matches(':').to_owned());
                }
            }
            diagnostics
        })
    }

    fn parse_inner(&self) -> Result<RawModel, Vec<Diagnostic>> {
        let mut model = RawModel {
            language_version: String::new(),
            model_id: String::new(),
            sources: Vec::new(),
            enums: Vec::new(),
            inputs: Vec::new(),
            derived: Vec::new(),
            outcomes: Vec::new(),
            domain: Vec::new(),
            assumptions: Vec::new(),
            rules: Vec::new(),
            constraints: Vec::new(),
            open_items: Vec::new(),
        };
        let mut rule_blocks = Vec::new();
        let mut constraint_blocks = Vec::new();
        let mut i = 0;
        while i < self.lines.len() {
            self.current_line.set(i);
            let line = self.lines.get(i).map(String::as_str).unwrap_or_default();
            if let Some(value) = line.strip_prefix("decision ") {
                if !model.model_id.is_empty() {
                    return Err(vec![diagnostic(
                        "duplicate decision declaration".to_owned(),
                    )]);
                }
                value.clone_into(&mut model.model_id);
            } else if let Some(value) = line.strip_prefix("version ") {
                if !model.language_version.is_empty() {
                    return Err(vec![diagnostic("duplicate version declaration".to_owned())]);
                }
                model.language_version = parse_string(value)
                    .ok_or_else(|| vec![diagnostic("invalid version string".to_owned())])?;
            } else if let Some(value) = line.strip_prefix("source ") {
                model.sources.push(parse_source(value).ok_or_else(|| {
                    vec![diagnostic(format!("invalid source declaration: {line}"))]
                })?);
            } else if let Some(value) = line.strip_prefix("type ") {
                model.enums.push(parse_enum(value).ok_or_else(|| {
                    vec![diagnostic(format!("invalid enum declaration: {line}"))]
                })?);
            } else if let Some(value) = line.strip_prefix("input ") {
                model.inputs.push(parse_input(value).ok_or_else(|| {
                    vec![diagnostic(format!("invalid input declaration: {line}"))]
                })?);
            } else if let Some(value) = line.strip_prefix("derived ") {
                model.derived.push(parse_derived(value).ok_or_else(|| {
                    vec![diagnostic(format!("invalid derived declaration: {line}"))]
                })?);
            } else if let Some(value) = line.strip_prefix("outcome ") {
                model.outcomes.push(parse_outcome(value).ok_or_else(|| {
                    vec![diagnostic(format!("invalid outcome declaration: {line}"))]
                })?);
            } else if let Some(value) = line.strip_prefix("domain ") {
                model.domain.push(parse_predicate(value).ok_or_else(|| {
                    vec![diagnostic(format!("invalid domain declaration: {line}"))]
                })?);
            } else if let Some(value) = line.strip_prefix("assumption ") {
                model
                    .assumptions
                    .push(parse_assumption(value).ok_or_else(|| {
                        vec![diagnostic(format!(
                            "invalid assumption declaration: {line}"
                        ))]
                    })?);
            } else if let Some(value) = line.strip_prefix("unresolved ") {
                model.open_items.push(
                    parse_open(
                        value,
                        self.lines
                            .get(i + 1)
                            .map(String::as_str)
                            .unwrap_or_default(),
                    )
                    .ok_or_else(|| {
                        vec![diagnostic(format!(
                            "invalid unresolved declaration: {line}"
                        ))]
                    })?,
                );
                i += 1;
            } else if let Some(value) = line.strip_prefix("rule ") {
                let end = (i + 1..self.lines.len())
                    .find(|index| {
                        self.lines
                            .get(*index)
                            .is_some_and(|s| s.starts_with("origin ") || s.starts_with("  origin "))
                    })
                    .ok_or_else(|| vec![diagnostic(format!("rule {value} is missing origin"))])?;
                let block = self
                    .lines
                    .get(i + 1..=end)
                    .ok_or_else(|| vec![diagnostic("invalid rule block range".to_owned())])?;
                rule_blocks.push((value.to_owned(), block.to_vec(), i));
                i = end;
            } else if let Some(value) = line.strip_prefix("constraint ") {
                let end = (i + 1..self.lines.len())
                    .find(|index| {
                        self.lines
                            .get(*index)
                            .is_some_and(|s| s.starts_with("origin ") || s.starts_with("  origin "))
                    })
                    .ok_or_else(|| {
                        vec![diagnostic(format!("constraint {value} is missing origin"))]
                    })?;
                let block = self
                    .lines
                    .get(i + 1..=end)
                    .ok_or_else(|| vec![diagnostic("invalid constraint block range".to_owned())])?;
                constraint_blocks.push((value.to_owned(), block.to_vec(), i));
                i = end;
            } else {
                return Err(vec![diagnostic(format!("unknown declaration: {line}"))]);
            }
            i += 1;
        }
        if model.model_id.is_empty() || model.language_version.is_empty() {
            return Err(vec![diagnostic(
                "model requires both `decision ID` and `version \"author-model/0.1\"` declarations"
                    .to_owned(),
            )]);
        }
        for (id, block, index) in rule_blocks {
            self.current_line.set(index);
            model.rules.push(
                parse_rule(&id, &block, &model.outcomes)
                    .ok_or_else(|| vec![diagnostic(format!("invalid rule block: {id}"))])?,
            );
        }
        for (id, block, index) in constraint_blocks {
            self.current_line.set(index);
            model.constraints.push(
                parse_constraint(&id, &block, &model.sources, &model.outcomes)
                    .ok_or_else(|| vec![diagnostic(format!("invalid constraint block: {id}"))])?,
            );
        }
        resolve_names(&mut model);
        Ok(model)
    }
}

// Declaration parsers follow below; they use JSON string literals so escaping is unambiguous.

fn split_origin(input: &str) -> Option<(&str, Origin)> {
    for (index, _) in input.match_indices(" origin ") {
        let body = input.get(..index)?;
        let origin = input.get(index + 1..)?;
        if let Some(parsed) = parse_origin(origin) {
            return Some((body, parsed));
        }
    }
    None
}

fn parse_origin(input: &str) -> Option<Origin> {
    let mut parts = input.splitn(6, ' ');
    if parts.next()? != "origin" {
        return None;
    }
    let kind = match parts.next()? {
        "source" => OriginKind::Source,
        "proposal" => OriginKind::Proposal,
        "assumption" => OriginKind::Assumption,
        _ => return None,
    };
    if parts.next()? != "review" {
        return None;
    }
    let review = match parts.next()? {
        "proposed" => Review::Proposed,
        "confirmed" => Review::Confirmed,
        _ => return None,
    };
    if parts.next()? != "sources" {
        return None;
    }
    let rest = parts.next()?;
    let close = rest.find(']')?;
    let list = rest.strip_prefix('[')?.get(..close - 1)?;
    let source_ids = list
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    let suffix = rest.get(close + 1..)?.trim();
    let note = if suffix.is_empty() {
        None
    } else {
        Some(parse_string(suffix.strip_prefix("note ")?)?)
    };
    Some(Origin {
        kind,
        source_ids,
        review,
        note,
    })
}

fn parse_source(input: &str) -> Option<Source> {
    let (id, rest) = input.split_once(" text ")?;
    let (text, tail) = parse_string_prefix(rest)?;
    let locator = if let Some(value) = tail.trim().strip_prefix("locator ") {
        Some(parse_string(value)?)
    } else if tail.trim().is_empty() {
        None
    } else {
        return None;
    };
    Some(Source {
        id: id.to_owned(),
        text,
        locator,
    })
}

fn parse_enum(input: &str) -> Option<EnumDecl> {
    let (body, origin) = split_origin(input)?;
    let (id, variants) = body.split_once(" = ")?;
    Some(EnumDecl {
        id: id.to_owned(),
        variants: variants
            .split('|')
            .map(str::trim)
            .map(str::to_owned)
            .collect(),
        origin,
    })
}

fn parse_input(input: &str) -> Option<InputDecl> {
    let (body, origin) = split_origin(input)?;
    let (id, ty) = body.split_once(": ")?;
    Some(InputDecl {
        id: id.to_owned(),
        r#type: parse_type(ty)?,
        origin,
    })
}

fn parse_derived(input: &str) -> Option<DerivedDecl> {
    let (body, origin) = split_origin(input)?;
    let (signature, expr) = body.split_once(" = ")?;
    let (id, ty) = signature.split_once(": ")?;
    Some(DerivedDecl {
        id: id.to_owned(),
        r#type: parse_type(ty)?,
        expr: parse_expr(expr)?,
        origin,
    })
}

fn parse_type(input: &str) -> Option<Type> {
    if input.is_empty() {
        return None;
    }
    if let Some(name) = input
        .strip_prefix("Enum[")
        .and_then(|value| value.strip_suffix(']'))
    {
        return Some(Type::Enum {
            name: name.to_owned(),
        });
    }
    if input == "Bool" {
        return Some(Type::Bool);
    }
    if input == "Date" {
        return Some(Type::Date);
    }
    if let Some(unit) = input
        .strip_prefix("Int[")
        .and_then(|value| value.strip_suffix(']'))
    {
        return Some(Type::Int {
            unit: unit.to_owned(),
        });
    }
    Some(Type::Enum {
        name: input.to_owned(),
    })
}

fn parse_outcome(input: &str) -> Option<OutcomeDecl> {
    let (body, origin) = split_origin(input)?;
    let (id, tail) = body.split_once('(')?;
    let fields = tail.strip_suffix(')')?;
    let fields = split_commas(fields)
        .into_iter()
        .map(|item| {
            let (name, ty) = item.split_once(": ")?;
            Some(FieldDecl {
                id: name.trim().to_owned(),
                r#type: parse_type(ty.trim())?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(OutcomeDecl {
        id: id.to_owned(),
        fields,
        origin,
    })
}

fn parse_predicate(input: &str) -> Option<Predicate> {
    let (body, origin) = split_origin(input)?;
    let (id, expr) = body.split_once(" require ")?;
    Some(Predicate {
        id: id.to_owned(),
        expr: parse_expr(expr)?,
        origin,
    })
}

fn parse_assumption(input: &str) -> Option<Predicate> {
    parse_predicate(input)
}

fn resolve_names(model: &mut RawModel) {
    let inputs = model
        .inputs
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    let derived = model
        .derived
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    let variants = model
        .enums
        .iter()
        .flat_map(|decl| {
            decl.variants
                .iter()
                .map(move |variant| (variant.clone(), decl.id.clone()))
        })
        .fold(
            BTreeMap::<String, Vec<String>>::new(),
            |mut map, (variant, enum_name)| {
                map.entry(variant).or_default().push(enum_name);
                map
            },
        );
    let outcome_fields = model
        .outcomes
        .iter()
        .flat_map(|outcome| outcome.fields.iter().map(|field| field.id.clone()))
        .collect::<BTreeSet<_>>();
    for item in &mut model.derived {
        resolve_expr(
            &mut item.expr,
            &inputs,
            &derived,
            &variants,
            &outcome_fields,
            false,
        );
    }
    for item in &mut model.domain {
        resolve_expr(
            &mut item.expr,
            &inputs,
            &derived,
            &variants,
            &outcome_fields,
            false,
        );
    }
    for item in &mut model.assumptions {
        resolve_expr(
            &mut item.expr,
            &inputs,
            &derived,
            &variants,
            &outcome_fields,
            false,
        );
    }
    for item in &mut model.rules {
        resolve_expr(
            &mut item.when,
            &inputs,
            &derived,
            &variants,
            &outcome_fields,
            false,
        );
        for expr in item.then.fields.values_mut() {
            resolve_expr(expr, &inputs, &derived, &variants, &outcome_fields, false);
        }
    }
    for item in &mut model.constraints {
        let outcome_fields = model
            .outcomes
            .iter()
            .find(|outcome| outcome.id == item.for_outcome)
            .into_iter()
            .flat_map(|outcome| outcome.fields.iter().map(|field| field.id.clone()))
            .collect::<BTreeSet<_>>();
        resolve_expr(
            &mut item.must,
            &inputs,
            &derived,
            &variants,
            &outcome_fields,
            true,
        );
    }
}

fn resolve_expr(
    expr: &mut Expr,
    inputs: &BTreeSet<String>,
    derived: &BTreeSet<String>,
    variants: &BTreeMap<String, Vec<String>>,
    outcome_fields: &BTreeSet<String>,
    allow_outcome: bool,
) {
    match expr {
        Expr::Ref { path } if !path.contains('.') => {
            if allow_outcome && outcome_fields.contains(path) {
                *path = format!("outcome.{path}");
            } else if inputs.contains(path) {
                *path = format!("input.{path}");
            } else if derived.contains(path) {
                *path = format!("derived.{path}");
            } else if let Some(enum_name) = variants
                .get(path)
                .filter(|names| names.len() == 1)
                .and_then(|names| names.first())
            {
                *expr = Expr::Enum {
                    enum_name: enum_name.clone(),
                    variant: path.clone(),
                };
            } else {
                *path = format!("input.{path}");
            }
        }
        Expr::Not { arg } | Expr::Scale { arg, .. } => resolve_expr(
            arg,
            inputs,
            derived,
            variants,
            outcome_fields,
            allow_outcome,
        ),
        Expr::ApplyRate { rate, quantity } => {
            resolve_expr(
                rate,
                inputs,
                derived,
                variants,
                outcome_fields,
                allow_outcome,
            );
            resolve_expr(
                quantity,
                inputs,
                derived,
                variants,
                outcome_fields,
                allow_outcome,
            );
        }
        Expr::And { args } | Expr::Or { args } => {
            for arg in args {
                resolve_expr(
                    arg,
                    inputs,
                    derived,
                    variants,
                    outcome_fields,
                    allow_outcome,
                );
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
            resolve_expr(
                left,
                inputs,
                derived,
                variants,
                outcome_fields,
                allow_outcome,
            );
            resolve_expr(
                right,
                inputs,
                derived,
                variants,
                outcome_fields,
                allow_outcome,
            );
        }
        Expr::If {
            condition,
            then_expr,
            else_expr,
        } => {
            resolve_expr(
                condition,
                inputs,
                derived,
                variants,
                outcome_fields,
                allow_outcome,
            );
            resolve_expr(
                then_expr,
                inputs,
                derived,
                variants,
                outcome_fields,
                allow_outcome,
            );
            resolve_expr(
                else_expr,
                inputs,
                derived,
                variants,
                outcome_fields,
                allow_outcome,
            );
        }
        _ => {}
    }
}

fn parse_open(input: &str, text_line: &str) -> Option<OpenItem> {
    let (id, rest) = input.split_once(" affects ")?;
    let (affects, sources) = rest.split_once(" sources [")?;
    let sources = sources.strip_suffix(']')?;
    let text = parse_string(text_line.trim())?;
    Some(OpenItem {
        id: id.to_owned(),
        text,
        source_ids: sources
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
        affects: affects.to_owned(),
    })
}

fn parse_rule(id: &str, lines: &[String], outcomes: &[OutcomeDecl]) -> Option<Rule> {
    let mut conclusion = None;
    let mut origin = None;
    let mut status = None;
    let mut guard_text = None;
    for line in lines {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("when ") {
            if guard_text.replace(value.to_owned()).is_some() {
                return None;
            }
        } else if let Some(value) = line.strip_prefix("conclude ") {
            if conclusion.is_some() {
                return None;
            }
            conclusion = Some(parse_conclusion(value, outcomes)?);
        } else if let Some(value) = line.strip_prefix("status ") {
            if status.replace(value).is_some() {
                return None;
            }
        } else if line.starts_with("origin ") {
            if origin.is_some() {
                return None;
            }
            origin = Some(parse_origin(line)?);
        } else if line.starts_with("and ") || line.starts_with("or ") {
            if conclusion.is_some() || status.is_some() {
                return None;
            }
            let text = guard_text.as_mut()?;
            text.push(' ');
            text.push_str(line);
        } else {
            return None;
        }
    }
    let guard = guard_text.as_deref().and_then(parse_expr);
    let origin = origin?;
    let expected_status = match origin.review {
        Review::Proposed => "proposed",
        Review::Confirmed => "confirmed",
    };
    if status? != expected_status {
        return None;
    }
    Some(Rule {
        id: id.to_owned(),
        when: guard?,
        then: conclusion?,
        origin,
    })
}

fn parse_conclusion(input: &str, outcomes: &[OutcomeDecl]) -> Option<RuleConclusion> {
    let (outcome, fields) = input.split_once('(')?;
    let fields = fields.strip_suffix(')')?;
    let mut parsed = BTreeMap::new();
    let arguments = split_commas(fields);
    let named = arguments.first().is_some_and(|field| field.contains(" = "));
    let declaration = outcomes.iter().find(|decl| decl.id == outcome);
    for (index, field) in arguments.iter().enumerate() {
        let (name, value) = if named {
            field.split_once(" = ")?
        } else {
            (declaration?.fields.get(index)?.id.as_str(), *field)
        };
        if parsed
            .insert(name.trim().to_owned(), parse_expr(value.trim())?)
            .is_some()
        {
            return None;
        }
    }
    if !named && declaration.is_some_and(|decl| decl.fields.len() != arguments.len()) {
        return None;
    }
    Some(RuleConclusion {
        outcome: outcome.to_owned(),
        fields: parsed,
    })
}

fn parse_constraint(
    id: &str,
    lines: &[String],
    sources: &[Source],
    outcomes: &[OutcomeDecl],
) -> Option<Constraint> {
    let mut outcome = None;
    let mut outcome_fields = Vec::new();
    let mut must = None;
    let mut origin = None;
    let mut source_labels = BTreeSet::new();
    for line in lines {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("when outcome is ") {
            let (constructor, fields) = value.split_once('(')?;
            let fields = fields.strip_suffix(')')?;
            if outcome.replace(constructor.to_owned()).is_some() {
                return None;
            }
            outcome_fields = split_commas(fields)
                .into_iter()
                .map(str::to_owned)
                .collect();
        } else if let Some(value) = line.strip_prefix("require ") {
            if must.is_some() {
                return None;
            }
            must = Some(parse_expr(value)?);
        } else if let Some(value) = line.strip_prefix("source ") {
            source_labels.insert(parse_string(value).unwrap_or_else(|| value.to_owned()));
        } else if line.starts_with("origin ") {
            if origin.is_some() {
                return None;
            }
            origin = Some(parse_origin(line)?);
        } else {
            return None;
        }
    }
    let for_outcome = outcome?;
    if !outcome_fields.is_empty() {
        let expected = outcomes
            .iter()
            .find(|decl| decl.id == for_outcome)?
            .fields
            .iter()
            .map(|field| field.id.as_str())
            .collect::<Vec<_>>();
        if expected.len() != outcome_fields.len()
            || expected
                .iter()
                .zip(&outcome_fields)
                .any(|(left, right)| left != right)
        {
            return None;
        }
    }
    let origin = origin?;
    let labeled_source_ids = source_labels
        .iter()
        .map(|label| {
            sources
                .iter()
                .find(|source| source.id == *label)
                .or_else(|| {
                    let mut matches = sources
                        .iter()
                        .filter(|source| source.locator.as_deref() == Some(label.as_str()));
                    let first = matches.next()?;
                    matches.next().is_none().then_some(first)
                })
                .map(|source| source.id.as_str())
        })
        .collect::<Option<BTreeSet<_>>>()?;
    if labeled_source_ids
        != origin
            .source_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
    {
        return None;
    }
    Some(Constraint {
        id: id.to_owned(),
        for_outcome,
        must: must?,
        origin,
    })
}

fn parse_string(input: &str) -> Option<String> {
    serde_json::from_str(input).ok()
}

fn parse_string_prefix(input: &str) -> Option<(String, &str)> {
    let bytes = input.as_bytes();
    if bytes.first().copied()? != b'"' {
        return None;
    }
    let mut escaped = false;
    for (index, byte) in bytes.iter().copied().enumerate().skip(1) {
        if escaped {
            escaped = false;
        } else if byte == b'\\' {
            escaped = true;
        } else if byte == b'"' {
            return Some((
                serde_json::from_str(input.get(..=index)?).ok()?,
                input.get(index + 1..)?,
            ));
        }
    }
    None
}

fn split_commas(input: &str) -> Vec<&str> {
    let mut depth = 0_i32;
    let mut quoted = false;
    let mut escaped = false;
    let mut start = 0;
    let mut result = Vec::new();
    for (index, character) in input.char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
        } else {
            match character {
                '"' => quoted = true,
                '(' | '[' => depth += 1,
                ')' | ']' => depth -= 1,
                ',' if depth == 0 => {
                    result.push(input.get(start..index).unwrap_or_default().trim());
                    start = index + 1;
                }
                _ => {}
            }
        }
    }
    let last = input.get(start..).unwrap_or_default().trim();
    if !last.is_empty() {
        result.push(last);
    }
    result
}

fn parse_expr(input: &str) -> Option<Expr> {
    let tokens = lex_expr(input)?;
    if tokens.len() > 500_000 {
        return None;
    }
    let mut parser = ExprParser {
        tokens,
        index: 0,
        nesting: 0,
        tree_depth: 0,
    };
    let expr = parser.expression(0)?;
    (parser.index == parser.tokens.len()).then_some(expr)
}

fn lex_expr(input: &str) -> Option<Vec<String>> {
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }
        if ch == '"' {
            let mut escaped = false;
            let mut end = None;
            for (index, c) in chars.by_ref() {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    end = Some(index);
                    break;
                }
            }
            let end = end? + 1;
            tokens.push(input.get(start..end)?.to_owned());
            continue;
        }
        if ch == '-'
            && chars.peek().is_some_and(|(_, next)| next.is_ascii_digit())
            && tokens.last().is_none_or(|previous| {
                matches!(
                    previous.as_str(),
                    "(" | "["
                        | ","
                        | "="
                        | "+"
                        | "-"
                        | "<"
                        | ">"
                        | "<="
                        | ">="
                        | "=="
                        | "!="
                        | "then"
                        | "else"
                        | "and"
                        | "or"
                        | "not"
                )
            })
        {
            let mut end = start + 1;
            while let Some((index, next)) = chars.peek().copied() {
                if next.is_ascii_digit() {
                    chars.next();
                    end = index + next.len_utf8();
                } else {
                    break;
                }
            }
            tokens.push(input.get(start..end)?.to_owned());
            continue;
        }
        if ch.is_ascii_alphanumeric() || ch == '_' {
            let mut end = start + ch.len_utf8();
            while let Some((index, next)) = chars.peek().copied() {
                if next.is_ascii_alphanumeric() || next == '_' {
                    chars.next();
                    end = index + next.len_utf8();
                } else {
                    break;
                }
            }
            tokens.push(input.get(start..end)?.to_owned());
            continue;
        }
        let rest = input.get(start..)?;
        let op = ["==", "!=", "<=", ">=", "::"]
            .iter()
            .find(|candidate| rest.starts_with(**candidate));
        if let Some(op) = op {
            for _ in 1..op.len() {
                chars.next();
            }
            tokens.push((*op).to_owned());
        } else if "()[],.+-<>/".contains(ch) {
            tokens.push(ch.to_string());
        } else {
            return None;
        }
    }
    Some(tokens)
}

struct ExprParser {
    tokens: Vec<String>,
    index: usize,
    nesting: usize,
    tree_depth: usize,
}

impl ExprParser {
    fn expression(&mut self, minimum: u8) -> Option<Expr> {
        if self.nesting >= 128 {
            return None;
        }
        self.nesting += 1;
        let result = self.expression_inner(minimum);
        self.nesting -= 1;
        result
    }

    fn expression_inner(&mut self, minimum: u8) -> Option<Expr> {
        let mut left = self.prefix()?;
        let mut left_depth = self.tree_depth;
        if left_depth > 128 {
            return None;
        }
        while let Some(token) = self.tokens.get(self.index) {
            let op = token.as_str();
            let (precedence, kind) = match op {
                "or" => (1, 0),
                "and" => (2, 1),
                "==" => (3, 2),
                "!=" => (3, 3),
                "<" => (3, 4),
                "<=" => (3, 5),
                ">" => (3, 6),
                ">=" => (3, 7),
                "+" => (4, 8),
                "-" => (4, 9),
                _ => break,
            };
            if precedence < minimum {
                break;
            }
            self.index += 1;
            let right = self.expression(precedence + 1)?;
            left_depth = left_depth.max(self.tree_depth) + 1;
            if left_depth > 128 {
                return None;
            }
            left = match kind {
                0 => Expr::Or {
                    args: vec![left, right],
                },
                1 => Expr::And {
                    args: vec![left, right],
                },
                2 => Expr::Eq {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                3 => Expr::Ne {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                4 => Expr::Lt {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                5 => Expr::Le {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                6 => Expr::Gt {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                7 => Expr::Ge {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                8 => Expr::Add {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                _ => Expr::Sub {
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        self.tree_depth = left_depth;
        Some(left)
    }

    fn prefix(&mut self) -> Option<Expr> {
        let token = self.tokens.get(self.index)?.clone();
        self.index += 1;
        self.tree_depth = 1;
        if self
            .tokens
            .get(self.index)
            .is_some_and(|next| matches!(next.as_str(), "::" | "."))
        {
            return self.atom(token);
        }
        match token.as_str() {
            "not" => {
                let arg = self.expression(5)?;
                self.tree_depth += 1;
                Some(Expr::Not { arg: Box::new(arg) })
            }
            "(" => {
                let value = self.expression(0)?;
                self.take(")").then_some(value)
            }
            "true" => Some(Expr::Bool { value: true }),
            "false" => Some(Expr::Bool { value: false }),
            "date" => {
                if !self.take("(") {
                    return None;
                }
                let value = parse_string(self.tokens.get(self.index)?)?;
                self.index += 1;
                self.take(")").then_some(Expr::Date { value })
            }
            "if" => {
                let condition = self.expression(0)?;
                let condition_depth = self.tree_depth;
                if !self.take("then") {
                    return None;
                }
                let then_expr = self.expression(0)?;
                let then_depth = self.tree_depth;
                if !self.take("else") {
                    return None;
                }
                let else_expr = self.expression(0)?;
                self.tree_depth = self.tree_depth.max(condition_depth).max(then_depth) + 1;
                Some(Expr::If {
                    condition: Box::new(condition),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                })
            }
            "scale" => {
                if !self.take("(") {
                    return None;
                }
                let factor = self.tokens.get(self.index)?.clone();
                self.index += 1;
                if !self.take(",") {
                    return None;
                }
                let arg = self.expression(0)?;
                if !self.take(")") {
                    return None;
                }
                self.tree_depth += 1;
                Some(Expr::Scale {
                    factor,
                    arg: Box::new(arg),
                })
            }
            "apply_rate" => {
                if !self.take("(") {
                    return None;
                }
                let rate = self.expression(0)?;
                if !self.take(",") {
                    return None;
                }
                let quantity = self.expression(0)?;
                if !self.take(")") {
                    return None;
                }
                self.tree_depth += 1;
                Some(Expr::ApplyRate {
                    rate: Box::new(rate),
                    quantity: Box::new(quantity),
                })
            }
            "and" | "or" => {
                if !self.take("(") {
                    return None;
                }
                let mut args = vec![self.expression(0)?];
                let mut deepest = self.tree_depth;
                while self.take(",") {
                    args.push(self.expression(0)?);
                    deepest = deepest.max(self.tree_depth);
                }
                self.tree_depth = deepest + 1;
                if !self.take(")") {
                    return None;
                }
                if token == "and" {
                    Some(Expr::And { args })
                } else {
                    Some(Expr::Or { args })
                }
            }
            _ => self.atom(token),
        }
    }

    fn atom(&mut self, token: String) -> Option<Expr> {
        if token
            .strip_prefix('-')
            .unwrap_or(&token)
            .chars()
            .all(|character| character.is_ascii_digit())
        {
            if !self.take("[") {
                return None;
            }
            let mut unit = self.tokens.get(self.index)?.clone();
            self.index += 1;
            if self.take("/") {
                unit.push('/');
                unit.push_str(self.tokens.get(self.index)?);
                self.index += 1;
            }
            if !self.take("]") {
                return None;
            }
            return Some(Expr::Int { value: token, unit });
        }
        if self.take("::") {
            let variant = self.tokens.get(self.index)?.clone();
            self.index += 1;
            return Some(Expr::Enum {
                enum_name: token,
                variant,
            });
        }
        if self.take(".") {
            let member = self.tokens.get(self.index)?.clone();
            self.index += 1;
            return Some(Expr::Ref {
                path: format!("{token}.{member}"),
            });
        }
        token
            .chars()
            .next()
            .filter(char::is_ascii_alphabetic)
            .and_then(|_| {
                token
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                    .then_some(Expr::Ref { path: token })
            })
    }

    fn take(&mut self, expected: &str) -> bool {
        if self
            .tokens
            .get(self.index)
            .is_some_and(|token| token == expected)
        {
            self.index += 1;
            true
        } else {
            false
        }
    }
}
