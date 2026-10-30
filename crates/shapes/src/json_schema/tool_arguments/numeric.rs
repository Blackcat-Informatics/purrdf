// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Plain numeric carriers over the shared XSD and JSON number authorities.

use super::{Ctx, ToolPolicy, ToolRepresentationError, Value, native_object, native_value};
use crate::json_model::Map;
use crate::json_schema::numeric_order::{BoundNumber, Decimal, Facet, Rel, Threshold};
use crate::shapes::{Constraint, Shape};
use crate::term::Term;
use purrdf_lex::allocation::StorageError;
use purrdf_lex::json::Number;
use purrdf_xsd::datatype::{XSD_DECIMAL, XSD_DOUBLE, XSD_FLOAT, XSD_INTEGER};
use purrdf_xsd::{XsdDatatype, XsdValue};

/// The original RDF datatype selected for a plain numeric value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::json_schema) enum Kind {
    Integer = 1,
    Decimal,
    Float,
    Double,
}

impl Kind {
    const fn datatype(self) -> XsdDatatype {
        match self {
            Self::Integer => XsdDatatype::Integer,
            Self::Decimal => XsdDatatype::Decimal,
            Self::Float => XsdDatatype::Float,
            Self::Double => XsdDatatype::Double,
        }
    }
    const fn ieee(self) -> bool {
        matches!(self, Self::Float | Self::Double)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(in crate::json_schema) struct Scope {
    kind: Option<Kind>,
    finite: bool,
}

pub(in crate::json_schema) fn kind(datatype: &str) -> Option<Kind> {
    match datatype {
        XSD_INTEGER => Some(Kind::Integer),
        XSD_DECIMAL => Some(Kind::Decimal),
        XSD_FLOAT => Some(Kind::Float),
        XSD_DOUBLE => Some(Kind::Double),
        _ => None,
    }
}

pub(in crate::json_schema) fn cache_key(ctx: &mut Ctx<'_>, target: &Shape) -> u8 {
    let Some(policy) = &mut ctx.tool else {
        return 0;
    };
    let scope = policy.numeric;
    let mut own = None;
    let mut datatype = false;
    let mut ambiguous = false;
    for constraint in &target.constraints {
        if !policy.charge(1) {
            break;
        }
        if let Constraint::Datatype(types) = constraint {
            datatype = true;
            for ty in types {
                if !policy.charge(1) {
                    break;
                }
                let candidate = kind(ty.as_str());
                ambiguous |=
                    candidate.is_none() || own.is_some_and(|known| Some(known) != candidate);
                own = candidate.or(own);
            }
        }
    }
    if !datatype {
        for constraint in &target.constraints {
            if !policy.charge(1) {
                break;
            }
            if let Constraint::In(terms) = constraint {
                for term in terms {
                    if !policy.charge(1) {
                        break;
                    }
                    let candidate = match term {
                        Term::Literal(literal) => kind(literal.datatype_str()),
                        _ => None,
                    };
                    ambiguous |=
                        own.is_some_and(|known| candidate.is_some() && Some(known) != candidate);
                    own = candidate.or(own);
                }
            }
        }
    }
    // Only an independently selected integer target makes these incoming
    // contexts equivalent. An alias may select a different datatype later.
    let incoming = if !ambiguous && own == Some(Kind::Integer) && scope.kind.is_none() {
        Some(Kind::Integer)
    } else {
        scope.kind
    };
    incoming.map_or(0, |kind| kind as u8) | if scope.finite { 8 } else { 0 }
}

pub(in crate::json_schema) fn with_scope<T>(
    ctx: &mut Ctx<'_>,
    constraints: &[&Constraint],
    shape: &str,
    key: &str,
    property: bool,
    operation: impl FnOnce(&mut Ctx<'_>) -> T,
) -> T {
    if ctx.tool.is_none() {
        return operation(ctx);
    }
    let location = super::format_text(ctx, format_args!("{shape} / {key}"));
    let Some(policy) = &mut ctx.tool else {
        return operation(ctx);
    };
    let previous_location = std::mem::replace(&mut policy.numeric_location, location);
    let previous = policy.numeric;
    let mut scope = if property { Scope::default() } else { previous };
    let mut selected = None;
    let (mut finite, mut scalar, mut constant) = (false, false, false);
    for constraint in constraints {
        if !policy.charge(1) {
            break;
        }
        match constraint {
            Constraint::Datatype(types) => {
                for datatype in types {
                    if !policy.charge(1) {
                        break;
                    }
                    let candidate = kind(datatype.as_str());
                    if candidate.is_some() && selected.is_some_and(|known| Some(known) != candidate)
                    {
                        policy.refuse(
                            &format!("{shape} / {key}"),
                            ToolRepresentationError::NumericDomain,
                        );
                    }
                    selected = candidate.or(selected);
                }
            }
            Constraint::In(_) => finite = true,
            Constraint::MaxCount(1) => scalar = true,
            Constraint::HasValue(_) => constant = true,
            _ => {}
        }
    }
    if selected.is_none() {
        for constraint in constraints {
            if !policy.charge(1) {
                break;
            }
            if let Constraint::In(terms) = constraint {
                for term in terms {
                    if !policy.charge(1) {
                        break;
                    }
                    let candidate = match term {
                        Term::Literal(literal) => kind(literal.datatype_str()),
                        _ => None,
                    };
                    if candidate.is_some() && selected.is_some_and(|known| Some(known) != candidate)
                    {
                        policy.refuse(
                            &format!("{shape} / {key}"),
                            ToolRepresentationError::NumericDomain,
                        );
                    }
                    selected = candidate.or(selected);
                }
            }
        }
    }
    if scope.kind.is_none() {
        scope.kind = selected;
    }
    scope.finite |= finite || (scalar && constant);
    policy.numeric = scope;
    let result = operation(ctx);
    if let Some(policy) = &mut ctx.tool {
        policy.numeric = previous;
        policy.numeric_location = previous_location;
    }
    result
}

pub(in crate::json_schema) fn with_finite_scope<T>(
    ctx: &mut Ctx<'_>,
    constraints: &[Constraint],
    operation: impl FnOnce(&mut Ctx<'_>) -> T,
) -> T {
    let Some(policy) = &mut ctx.tool else {
        return operation(ctx);
    };
    let previous = policy.numeric;
    for constraint in constraints {
        if !policy.charge(1) {
            break;
        }
        if matches!(constraint, Constraint::In(_) | Constraint::HasValue(_)) {
            policy.numeric.finite = true;
        }
    }
    let answer = operation(ctx);
    if let Some(policy) = &mut ctx.tool {
        policy.numeric = previous;
    }
    answer
}

pub(in crate::json_schema) fn requires_logical_proof(ctx: &Ctx<'_>) -> bool {
    ctx.tool
        .as_ref()
        .and_then(|policy| policy.numeric.kind)
        .is_some_and(|kind| kind != Kind::Integer)
}

pub(in crate::json_schema) fn refuse_logic(ctx: &mut Ctx<'_>, shape: &str, key: &str) {
    if let Some(policy) = &mut ctx.tool {
        policy.refuse(
            &format!("{shape} / {key}"),
            ToolRepresentationError::NumericLogic,
        );
    }
}

fn parse(term: &Term, policy: &mut ToolPolicy) -> Option<XsdValue> {
    let Term::Literal(literal) = term else {
        return None;
    };
    let kind = kind(literal.datatype_str())?;
    if !policy.charge(u64::try_from(literal.value().len()).unwrap_or(u64::MAX)) {
        return None;
    }
    match policy.native(|memory| {
        purrdf_xsd::value::try_parse_with_memory(literal.value(), kind.datatype(), true, memory)
    })? {
        purrdf_xsd::value::ParsedValue::Value(value) => Some(value),
        purrdf_xsd::value::ParsedValue::Invalid(_) => None,
    }
}

fn valid_value(value: &XsdValue) -> Result<(), ToolRepresentationError> {
    match value {
        XsdValue::Double(value) if !value.is_finite() => {
            Err(ToolRepresentationError::NumericConstant)
        }
        XsdValue::Float(value) if !value.is_finite() => {
            Err(ToolRepresentationError::NumericConstant)
        }
        XsdValue::Double(value) if *value == 0.0 && value.is_sign_negative() => {
            Err(ToolRepresentationError::SignedZero)
        }
        XsdValue::Float(value) if *value == 0.0 && value.is_sign_negative() => {
            Err(ToolRepresentationError::SignedZero)
        }
        _ => Ok(()),
    }
}

pub(in crate::json_schema) fn admit_term(
    term: &Term,
    location: &str,
    policy: &mut ToolPolicy,
) -> bool {
    let Term::Literal(literal) = term else {
        return false;
    };
    let candidate = kind(literal.datatype_str());
    if candidate.is_some()
        && policy
            .numeric
            .kind
            .is_some_and(|selected| Some(selected) != candidate)
    {
        policy.refuse(location, ToolRepresentationError::NumericDomain);
        return false;
    }
    if !candidate.is_some_and(|kind| kind != Kind::Integer) {
        return super::scalar_term(term);
    }
    let Some(value) = parse(term, policy) else {
        policy.refuse(location, ToolRepresentationError::NumericConstant);
        return false;
    };
    if let Err(reason) = valid_value(&value) {
        policy.refuse(location, reason);
        return false;
    }
    let Some(canonical) = render(&value, policy, false) else {
        return false;
    };
    if canonical != literal.value() {
        policy.refuse(location, ToolRepresentationError::NumericIdentity);
        return false;
    }
    true
}

pub(in crate::json_schema) fn admit_bound(
    term: &Term,
    location: &str,
    policy: &mut ToolPolicy,
) -> bool {
    let reason = match parse(term, policy) {
        Some(value) => valid_value(&value).err(),
        None => Some(ToolRepresentationError::NumericConstant),
    };
    if let Some(reason) = reason {
        policy.refuse(location, reason);
        return false;
    }
    true
}

fn limb_error(error: purrdf_xsd::bigint::LimbScratchError) -> StorageError {
    match error {
        purrdf_xsd::bigint::LimbScratchError::SizeOverflow => StorageError::SizeOverflow,
        purrdf_xsd::bigint::LimbScratchError::AllocationFailed => StorageError::AllocationFailed,
        purrdf_xsd::bigint::LimbScratchError::Storage(error) => error,
        _ => StorageError::AdmissionFailed,
    }
}

fn invariant(policy: &mut ToolPolicy) {
    let location = policy.numeric_location.clone();
    policy.refuse(&location, ToolRepresentationError::NumericInvariant);
}

/// Preserve native physical causes separately from semantic numeric invariants.
fn converted(
    value: Result<purrdf_xsd::exact::Decimal, purrdf_xsd::exact::ExactOperationError>,
) -> Result<Option<purrdf_xsd::exact::Decimal>, StorageError> {
    match value {
        Ok(value) => Ok(Some(value)),
        Err(purrdf_xsd::exact::ExactOperationError::Storage(error)) => Err(limb_error(error)),
        Err(purrdf_xsd::exact::ExactOperationError::Value(_)) => Ok(None),
    }
}

fn render(value: &XsdValue, policy: &mut ToolPolicy, dyadic: bool) -> Option<String> {
    let answer = policy.native(|memory| {
        let decimal = match value {
            XsdValue::Float(value) if dyadic => {
                let decimal = converted(purrdf_xsd::exact::Decimal::try_from_f32_admitted(
                    *value,
                    &mut |layout| {
                        memory
                            .add_bytes(layout.required_bytes())
                            .map_err(purrdf_xsd::bigint::LimbScratchError::Storage)
                    },
                ))?;
                let Some(decimal) = decimal else {
                    return Ok(None);
                };
                Some(decimal)
            }
            XsdValue::Double(value) if dyadic => {
                let decimal = converted(purrdf_xsd::exact::Decimal::try_from_f64_admitted(
                    *value,
                    &mut |layout| {
                        memory
                            .add_bytes(layout.required_bytes())
                            .map_err(purrdf_xsd::bigint::LimbScratchError::Storage)
                    },
                ))?;
                let Some(decimal) = decimal else {
                    return Ok(None);
                };
                Some(decimal)
            }
            _ => None,
        };
        let prepared = if let Some(decimal) = &decimal {
            Some(
                purrdf_xsd::numeric::prepare_decimal_text(decimal, &mut |layout| {
                    memory
                        .add_bytes(layout.required_bytes())
                        .map_err(purrdf_xsd::bigint::LimbScratchError::Storage)
                })
                .map_err(limb_error)?,
            )
        } else {
            purrdf_xsd::numeric::prepare_numeric_text(value, &mut |layout| {
                memory
                    .add_bytes(layout.required_bytes())
                    .map_err(purrdf_xsd::bigint::LimbScratchError::Storage)
            })
            .map_err(limb_error)?
        };
        let Some(prepared) = prepared else {
            return Ok(None);
        };
        let mut text = String::new();
        memory.reserve_string(&mut text, prepared.len())?;
        prepared
            .write_to(&mut text)
            .map_err(|_| StorageError::SizeOverflow)?;
        Ok(Some(text))
    });
    match answer {
        Some(Some(text)) => Some(text),
        Some(None) => {
            invariant(policy);
            None
        }
        None => None,
    }
}

/// Emit the mathematical value, never the shortest IEEE round-trip approximation.
fn number(value: &XsdValue, policy: &mut ToolPolicy) -> Option<Value> {
    if !policy.native_nodes(1) {
        return None;
    }
    let text = render(value, policy, true)?;
    match Number::from_lexeme(text) {
        Ok(number) => Some(Value::Number(number)),
        Err(_) => {
            invariant(policy);
            None
        }
    }
}

pub(in crate::json_schema) fn term_value(term: &Term, ctx: &mut Ctx<'_>) -> Value {
    let Some(policy) = &mut ctx.tool else {
        return Value::Null;
    };
    let Some(value) = parse(term, policy) else {
        if policy.error.is_none() {
            invariant(policy);
        }
        return Value::Null;
    };
    number(&value, policy).unwrap_or(Value::Null)
}

pub(in crate::json_schema) fn datatype_schema(
    kind: Kind,
    shape: &str,
    key: &str,
    ctx: &mut Ctx<'_>,
) -> Value {
    let scope = ctx.tool.as_ref().expect("plain numeric mode").numeric;
    if scope.kind.is_some_and(|selected| selected != kind) {
        return native_value(ctx, false);
    }
    let name = if kind == Kind::Integer {
        "integer"
    } else {
        "number"
    };
    if kind.ieee() && !scope.finite {
        super::record_loss(
            ctx,
            "plain:ieee-representability",
            shape,
            &format!(
                "property {key} requires original SHACL validation of finite IEEE representability; JSON number membership is not that check"
            ),
        );
        let comment = native_value(
            ctx,
            "original SHACL validation of finite IEEE representability is required",
        );
        let name = native_value(ctx, name);
        return native_object(ctx, [("$comment", comment), ("type", name)]);
    }
    let name = native_value(ctx, name);
    native_object(ctx, [("type", name)])
}

pub(in crate::json_schema) fn exact(
    constraint: &Constraint,
    siblings: &[Constraint],
    ctx: &mut Ctx<'_>,
    location: (&str, &str),
) -> Option<bool> {
    let scope = ctx.tool.as_ref()?.numeric;
    let numeric = match constraint {
        Constraint::Datatype(types) => types
            .iter()
            .any(|datatype| kind(datatype.as_str()).is_some()),
        Constraint::MinInclusive(_)
        | Constraint::MaxInclusive(_)
        | Constraint::MinExclusive(_)
        | Constraint::MaxExclusive(_) => true,
        Constraint::In(terms) => terms.iter().any(|term| {
            matches!(term,
            Term::Literal(literal) if kind(literal.datatype_str()).is_some())
        }),
        Constraint::HasValue(term) => matches!(term,
            Term::Literal(literal) if kind(literal.datatype_str()).is_some()),
        Constraint::Pattern { .. } if scope.kind.is_some() => {
            let (mut finite, mut scalar, mut constant) = (false, false, false);
            for sibling in siblings {
                if !super::step(ctx) {
                    return Some(false);
                }
                match sibling {
                    Constraint::In(_) => finite = true,
                    Constraint::MaxCount(1) => scalar = true,
                    Constraint::HasValue(_) => constant = true,
                    _ => {}
                }
            }
            return Some(finite || (scalar && constant));
        }
        _ => return None,
    };
    if !numeric {
        return None;
    }
    if scope.kind.is_none() || (scope.kind.is_some_and(Kind::ieee) && !scope.finite) {
        ctx.tool.as_mut().expect("plain proof").refuse(
            &format!("{} / {}", location.0, location.1),
            ToolRepresentationError::NumericLogic,
        );
        return Some(false);
    }
    Some(true)
}

fn decimal(value: &XsdValue, policy: &mut ToolPolicy) -> Option<Decimal> {
    let text = render(value, policy, true)?;
    let parsed = policy.native(|memory| Decimal::parse_with_memory(&text, false, memory));
    match parsed {
        Some(Some(decimal)) => Some(decimal),
        Some(None) => {
            invariant(policy);
            None
        }
        None => None,
    }
}

fn admitted_threshold(
    facet: Facet,
    bound: BoundNumber<'_>,
    policy: &mut ToolPolicy,
) -> Option<Threshold> {
    let result =
        policy.native(
            |memory| match Threshold::for_bound_with_memory(facet, bound, memory) {
                Ok(threshold) => Ok(Some(threshold)),
                Err(purrdf_xsd::exact::ExactOperationError::Storage(error)) => {
                    Err(limb_error(error))
                }
                Err(purrdf_xsd::exact::ExactOperationError::Value(_)) => Ok(None),
            },
        );
    match result {
        Some(Some(threshold)) => Some(threshold),
        Some(None) => {
            invariant(policy);
            None
        }
        None => None,
    }
}

fn threshold(facet: Facet, term: &Term, kind: Kind, policy: &mut ToolPolicy) -> Option<Threshold> {
    let value = parse(term, policy)?;
    if matches!(kind, Kind::Integer | Kind::Decimal) {
        return match value {
            XsdValue::Float(value) => admitted_threshold(facet, BoundNumber::Float(value), policy),
            XsdValue::Double(value) => {
                admitted_threshold(facet, BoundNumber::Double(value), policy)
            }
            _ => Some(Threshold::for_exact(facet, decimal(&value, policy)?)),
        };
    }
    let promoted = match value {
        XsdValue::Float(value) => XsdValue::Double(f64::from(value)),
        XsdValue::Double(value) => XsdValue::Double(value),
        _ => {
            let text = render(&value, policy, false)?;
            let parsed = match kind {
                Kind::Float => purrdf_xsd::parse_float_xsd10(&text).map(XsdValue::Float),
                Kind::Double => purrdf_xsd::parse_double_xsd10(&text).map(XsdValue::Double),
                _ => {
                    invariant(policy);
                    return None;
                }
            };
            let Ok(parsed) = parsed else {
                invariant(policy);
                return None;
            };
            parsed
        }
    };
    if let Err(reason) = valid_value(&promoted) {
        let location = policy.numeric_location.clone();
        policy.refuse(&location, reason);
        return None;
    }
    Some(Threshold::for_exact(facet, decimal(&promoted, policy)?))
}

fn merge_threshold(threshold: Threshold, target: &mut Map<String, Value>, ctx: &mut Ctx<'_>) {
    let Threshold::Cmp(relation, decimal) = threshold else {
        if matches!(threshold, Threshold::None) {
            let falsehood = native_value(ctx, false);
            let rejected = super::native_array(ctx, [falsehood]);
            super::insert!(ctx; target,"allOf", rejected);
        }
        return;
    };
    let keyword = match relation {
        Rel::Ge => "minimum",
        Rel::Gt => "exclusiveMinimum",
        Rel::Le => "maximum",
        Rel::Lt => "exclusiveMaximum",
    };
    let Some(policy) = &mut ctx.tool else {
        return;
    };
    if let Some(previous) = target.get(keyword).and_then(Value::as_number) {
        let previous =
            policy.native(|memory| Decimal::parse_with_memory(previous.lexeme(), false, memory));
        let Some(Some(previous)) = previous else {
            if policy.error.is_none() {
                invariant(policy);
            }
            return;
        };
        let stricter = if matches!(relation, Rel::Ge | Rel::Gt) {
            decimal > previous
        } else {
            decimal < previous
        };
        if !stricter {
            return;
        }
    }
    let Some(text) = policy.native(|memory| decimal.plain_text(memory)) else {
        return;
    };
    let Ok(number) = Number::from_lexeme(text) else {
        invariant(policy);
        return;
    };
    if !policy.native_nodes(1) {
        return;
    }
    super::insert!(ctx; target,keyword, Value::Number(number));
}

pub(in crate::json_schema) fn compile_bounds(
    bounds: &[(Facet, &Term)],
    target: &mut Map<String, Value>,
    ctx: &mut Ctx<'_>,
    location: (&str, &str),
) {
    let Some(kind) = ctx.tool.as_ref().and_then(|policy| policy.numeric.kind) else {
        ctx.tool.as_mut().expect("plain bounds").refuse(
            &format!("{} / {}", location.0, location.1),
            ToolRepresentationError::NumericDomain,
        );
        return;
    };
    let name = native_value(
        ctx,
        if kind == Kind::Integer {
            "integer"
        } else {
            "number"
        },
    );
    super::insert!(ctx; target,"type", name);
    for &(facet, term) in bounds {
        let policy = ctx.tool.as_mut().expect("plain numeric bounds");
        if !policy.charge(1) {
            return;
        }
        let Some(threshold) = threshold(facet, term, kind, policy) else {
            return;
        };
        merge_threshold(threshold, target, ctx);
    }
}

/// A positive same-value witness widens only the JSON carrier, never an exactness proof.
pub(in crate::json_schema) fn number_carrier(
    groups: &[Vec<Value>],
    conjuncts: &[Value],
    ctx: &mut Ctx<'_>,
) -> bool {
    let Some(policy) = &mut ctx.tool else {
        return false;
    };
    if policy
        .numeric
        .kind
        .is_some_and(|kind| kind != Kind::Integer)
    {
        return true;
    }
    if policy.numeric.kind == Some(Kind::Integer) {
        return false;
    }
    let mut visited = Vec::new();
    let wider = groups
        .iter()
        .flatten()
        .chain(conjuncts)
        .any(|value| positive_number(value, &ctx.catalog_defs, &mut visited, policy));
    policy.native(|memory| memory.release_vec(visited));
    wider
}

fn positive_number<'a>(
    value: &'a Value,
    definitions: &'a Map<String, (Value, String)>,
    visited: &mut Vec<&'a str>,
    policy: &mut ToolPolicy,
) -> bool {
    if !policy.charge(1) {
        return true;
    }
    if purrdf_stack::is_low() {
        policy
            .error
            .get_or_insert(super::ToolSchemaError::StackMargin {
                location: "numeric carrier traversal".to_owned(),
            });
        return true;
    }
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.get("type").is_some_and(|ty| {
        ty.as_str() == Some("number")
            || ty.as_array().is_some_and(|types| {
                types
                    .iter()
                    .any(|ty| policy.charge(1) && ty.as_str() == Some("number"))
            })
    }) {
        return true;
    }
    for keyword in ["allOf", "anyOf", "oneOf"] {
        if let Some(children) = object.get(keyword).and_then(Value::as_array)
            && children
                .iter()
                .any(|child| positive_number(child, definitions, visited, policy))
        {
            return true;
        }
    }
    let Some(key) = object
        .get("$ref")
        .and_then(Value::as_str)
        .and_then(|reference| reference.strip_prefix("#/$defs/"))
        .filter(|key| key.starts_with("ShapeRef"))
    else {
        return false;
    };
    if !policy.charge(key.len() as u64) {
        return true;
    }
    for seen in visited.iter() {
        if !policy.charge(seen.len().min(key.len()) as u64 + 1) {
            return true;
        }
        if *seen == key {
            return true;
        }
    }
    // A definition still being compiled is unknown. Widening the carrier is
    // conservative; this says nothing about its datatype or residual checks.
    let Some((body, _)) = definitions.get(key) else {
        return true;
    };
    if policy.native(|memory| memory.push(visited, key)).is_none() {
        return true;
    }
    let wider = positive_number(body, definitions, visited, policy);
    visited.pop();
    wider
}

/// Preserve existing integer-only carrier bytes in the absence of a wider witness.
pub(in crate::json_schema) fn default_types(number: bool, ctx: &mut Ctx<'_>) -> Value {
    if number {
        super::emitted_json!(ctx; ["string", "number", "boolean", "object"])
    } else {
        super::emitted_json!(ctx; ["string", "integer", "boolean", "object"])
    }
}
