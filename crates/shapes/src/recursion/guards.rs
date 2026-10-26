// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Sufficient, request-independent proofs of exclusive node-shape guards.

use crate::model::rdf;
use crate::report::Severity;
use crate::shapes::{ClosedMode, Constraint, ModelOwner, NodeKindValue, Path, Shape};

/// A sufficient proof that xone's conformance dependency admits a positive union
/// extension. This is not a solver for arbitrary SHACL formulas, and it does not
/// replace xone's result-counting rule in a report.
///
/// All guards have the xone's own unannotated severity. If that severity counts
/// against conformance, the guards exclude simultaneous operand conformance. If
/// it does not, the xone's result itself cannot affect conformance. Thus the
/// proof holds for every severity policy without caching a request's answers.
pub(crate) fn xone_positive(
    source: &Shape,
    operands: &[Shape],
    owner: Option<&ModelOwner>,
) -> bool {
    if operands.len() < 2 || !clean(source, &source.severity) {
        return false;
    }
    for (index, left) in operands.iter().enumerate() {
        let Some((left, left_owner)) = original(left, owner, &source.severity) else {
            return false;
        };
        for right in &operands[index + 1..] {
            let Some((right, right_owner)) = original(right, owner, &source.severity) else {
                return false;
            };
            if kinds(left) & kinds(right) != 0
                && !required_excluded(left, left_owner, right, right_owner, &source.severity)
                && !required_excluded(right, right_owner, left, left_owner, &source.severity)
            {
                return false;
            }
        }
    }
    true
}

fn clean(shape: &Shape, severity: &Severity) -> bool {
    !shape.deactivated
        && shape.deactivated_when.is_empty()
        && shape.constraint_annotations.is_empty()
        && shape.severity == *severity
        && shape.as_property().is_none()
}

fn original<'a>(
    shape: &'a Shape,
    owner: Option<&'a ModelOwner>,
    severity: &Severity,
) -> Option<(&'a Shape, Option<&'a ModelOwner>)> {
    if !clean(shape, severity) {
        return None;
    }
    let owner = shape.owner.as_deref().or(owner);
    let body = if super::model::plain_handle(shape) {
        owner?.definition(&shape.id)?
    } else {
        shape
    };
    clean(body, severity).then_some((body, owner))
}

fn kinds(shape: &Shape) -> u8 {
    shape
        .constraints
        .iter()
        .fold(0b1111, |allowed, constraint| {
            let Constraint::NodeKind(kinds) = constraint else {
                return allowed;
            };
            allowed
                & kinds.iter().fold(0, |mask, kind| {
                    mask | match kind {
                        NodeKindValue::Iri => 0b0001,
                        NodeKindValue::BlankNode => 0b0010,
                        NodeKindValue::Literal => 0b0100,
                        NodeKindValue::BlankNodeOrIri => 0b0011,
                        NodeKindValue::BlankNodeOrLiteral => 0b0110,
                        NodeKindValue::IriOrLiteral => 0b0101,
                        NodeKindValue::TripleTerm => 0b1000,
                    }
                })
        })
}

fn excludes(shape: &Shape, owner: Option<&ModelOwner>, predicate: &str) -> bool {
    shape.constraints.iter().any(|constraint| {
        matches!(constraint, Constraint::Closed { ignored, mode: ClosedMode::Declared }
            if !ignored.iter().any(|ignored| ignored.as_str() == predicate))
    }) && shape.properties_in(owner).all(|property| {
        property.is_ok_and(|property| match &property.property.path {
            Path::Predicate(allowed) => allowed.as_str() != predicate,
            // Conservatively decline composite paths, without implementing a
            // second version of the production permitted-predicate derivation.
            _ => false,
        })
    })
}

fn required_excluded(
    required: &Shape,
    required_owner: Option<&ModelOwner>,
    closed: &Shape,
    closed_owner: Option<&ModelOwner>,
    severity: &Severity,
) -> bool {
    // A conforming nonempty SHACL list necessarily has both outgoing edges.
    if required
        .constraints
        .iter()
        .any(|constraint| matches!(constraint, Constraint::MinListLength(min) if *min > 0))
        && (excludes(closed, closed_owner, rdf::FIRST) || excludes(closed, closed_owner, rdf::REST))
    {
        return true;
    }
    required.properties_in(required_owner).any(|property| {
        property.is_ok_and(|view| {
            let property = view.property;
            let Path::Predicate(predicate) = &property.path else {
                return false;
            };
            !view.deactivated()
                && view.severity() == severity
                && property.deactivated_when.is_empty()
                && view
                    .edge
                    .is_none_or(|edge| edge.deactivated_when.is_empty())
                && property.constraint_annotations.is_empty()
                && property.values.is_none()
                && property.default_value.is_none()
                && property
                    .constraints
                    .iter()
                    .any(|constraint| matches!(constraint, Constraint::MinCount(min) if *min > 0))
                && excludes(closed, closed_owner, predicate.as_str())
        })
    })
}
