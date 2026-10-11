// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A restriction certificate consumed by the original indexed BGP kernel.
//! The original FILTER still decides every emitted row. Only exact numeric
//! ordinal comparisons are used to omit candidates that cannot pass it.

use super::{Builtin, ShapeOffer};
use crate::eval::EvalCtx;
use crate::governor::lift::Evaluated;
use crate::native_numeric::NumericFrame;
use crate::parsed_value::ParsedValue;
use crate::{AdmittedVec, EvalError, WorkspaceCapability};
use core::cmp::Ordering;
use purrdf_core::{DatasetView, TermValue};
use purrdf_sparql_algebra::{
    ArithmeticOperator, Expression, Function, GraphPattern, TermPattern, Variable,
};
use purrdf_xsd::{XsdDatatype, XsdValue};

/// `left - right <[=] bound`, optionally under ABS. Values and limits are
/// exact XSD integers/decimals, with no ordinal narrowing or chosen ceiling.
#[derive(Clone, Debug)]
pub(crate) struct WindowSpec {
    pub(crate) left: Variable,
    pub(crate) right: Variable,
    bound: ParsedValue,
    absolute: bool,
    strict: bool,
    /// An eager EXISTS residual must still run for every original pair, even
    /// when this numeric conjunct is false. Its inner is separately indexed.
    pub(crate) restrict: bool,
}

pub(crate) fn offer(
    pattern: &GraphPattern,
    workspace: &WorkspaceCapability,
) -> Result<Option<ShapeOffer>, EvalError> {
    let GraphPattern::Filter { inner, expr } = pattern else {
        return Ok(None);
    };
    let GraphPattern::Bgp { patterns } = &**inner else {
        return Ok(None);
    };
    let Some(mut spec) = find_window(expr, workspace)? else {
        return Ok(None);
    };
    // An opaque/stateful residual has its original invocation order and search
    // scope. It is never speculatively skipped by an access restriction.
    let pure = pure_ordinal_predicate(expr, workspace)?;
    if !pure && !indexed_barrier_predicate(expr, workspace)? {
        return Ok(None);
    }
    spec.restrict = pure;
    let mut left_subject = None;
    let mut right_subject = None;
    for pattern in patterns {
        if !matches!(
            pattern.predicate,
            purrdf_sparql_algebra::NamedNodePattern::NamedNode(_)
        ) || !matches!(pattern.subject, TermPattern::Variable(_))
            || !matches!(
                pattern.object,
                TermPattern::Variable(_) | TermPattern::Literal(_) | TermPattern::NamedNode(_)
            )
        {
            return Ok(None);
        }
        if let TermPattern::Variable(object) = &pattern.object {
            if *object == spec.left {
                left_subject = Some(&pattern.subject);
            }
            if *object == spec.right {
                right_subject = Some(&pattern.subject);
            }
        }
    }
    let (Some(left), Some(right)) = (left_subject, right_subject) else {
        return Ok(None);
    };
    // The same sequence is an actual shared graph binding, not a lexical
    // assumption about entities, IDs or positions.
    let connected = patterns.iter().any(|a| &a.subject == left && patterns.iter().any(|b| &b.subject == right && a.predicate == b.predicate && matches!((&a.object, &b.object), (TermPattern::Variable(a), TermPattern::Variable(b)) if a == b)));
    if !connected {
        return Ok(None);
    }
    Ok(Some(ShapeOffer::builtin(
        "ordered-window",
        Builtin::Window(spec),
    )))
}

pub(crate) fn pure_ordinal_predicate(
    expression: &Expression,
    workspace: &WorkspaceCapability,
) -> Result<bool, EvalError> {
    let mut pending = AdmittedVec::new(workspace);
    pending.push(expression)?;
    while let Some(expression) = pending.pop() {
        match expression {
            Expression::Variable(_) | Expression::Literal(_) | Expression::Bound(_) => {}
            Expression::And(parts) => {
                for part in parts {
                    pending.push(part)?;
                }
            }
            Expression::Equal(a, b)
            | Expression::SameTerm(a, b)
            | Expression::Less(a, b)
            | Expression::LessOrEqual(a, b)
            | Expression::Greater(a, b)
            | Expression::GreaterOrEqual(a, b) => {
                pending.push(a)?;
                pending.push(b)?;
            }
            Expression::Arithmetic(first, rest) => {
                pending.push(first)?;
                for (operator, operand) in rest {
                    if !matches!(
                        operator,
                        ArithmeticOperator::Add | ArithmeticOperator::Subtract
                    ) {
                        return Ok(false);
                    }
                    pending.push(operand)?;
                }
            }
            Expression::FunctionCall(Function::Abs, args) => {
                for arg in args {
                    pending.push(arg)?;
                }
            }
            Expression::UnaryMinus(inner) | Expression::UnaryPlus(inner) => pending.push(inner)?,
            _ => return Ok(false),
        }
    }
    Ok(true)
}

fn indexed_barrier_predicate(
    expression: &Expression,
    workspace: &WorkspaceCapability,
) -> Result<bool, EvalError> {
    let mut pending = AdmittedVec::new(workspace);
    pending.push(expression)?;
    let mut found = false;
    while let Some(expression) = pending.pop() {
        if let Expression::And(parts) = expression {
            for part in parts {
                pending.push(part)?;
            }
        } else if let Expression::Not(inner) = expression
            && let Expression::Exists(pattern) = &**inner
        {
            if !super::barrier::is_between(pattern, workspace)? {
                return Ok(false);
            }
            found = true;
        } else if !pure_ordinal_predicate(expression, workspace)? {
            return Ok(false);
        }
    }
    Ok(found)
}

fn find_window(
    expression: &Expression,
    workspace: &WorkspaceCapability,
) -> Result<Option<WindowSpec>, EvalError> {
    let mut frame = crate::workspace::LexicalFrame::new(workspace);
    let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
    let mut pending = purrdf_lex::walk::WorkList::<_, 16>::with(expression);
    while let Some(expression) = pending.pop() {
        if let Expression::And(parts) = expression {
            for part in parts.iter().rev() {
                pending
                    .try_push_admitted(part, &mut memory)
                    .map_err(|error| {
                        memory
                            .admission_mut()
                            .storage_error(error, "ordinal window certificate")
                    })?;
            }
            continue;
        }
        let (difference, bound, strict, reverse) = match expression {
            Expression::Less(a, b) => (&**a, &**b, true, false),
            Expression::LessOrEqual(a, b) => (&**a, &**b, false, false),
            Expression::Greater(a, b) => (&**a, &**b, true, true),
            Expression::GreaterOrEqual(a, b) => (&**a, &**b, false, true),
            _ => continue,
        };
        let Some(bound) = constant(bound, workspace)? else {
            continue;
        };
        let (difference, absolute) = match difference {
            Expression::FunctionCall(Function::Abs, args) if args.len() == 1 => (&args[0], true),
            other => (other, false),
        };
        if absolute && reverse {
            continue;
        }
        let Expression::Arithmetic(first, rest) = difference else {
            continue;
        };
        if rest.len() != 1 {
            continue;
        }
        let (ArithmeticOperator::Subtract, Expression::Variable(right)) = &rest[0] else {
            continue;
        };
        let Expression::Variable(left) = &**first else {
            continue;
        };
        let (left, right, bound) = if reverse {
            (right, left, negate(&bound, workspace)?)
        } else {
            (left, right, bound)
        };
        return Ok(Some(WindowSpec {
            left: left.clone(),
            right: right.clone(),
            bound,
            absolute,
            strict,
            restrict: true,
        }));
    }
    Ok(None)
}

fn constant(
    expression: &Expression,
    workspace: &WorkspaceCapability,
) -> Result<Option<ParsedValue>, EvalError> {
    let (literal, negative) = match expression {
        Expression::Literal(literal) => (literal, false),
        Expression::UnaryMinus(inner) => match &**inner {
            Expression::Literal(literal) => (literal, true),
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };
    let Some(datatype) = XsdDatatype::from_iri(literal.datatype().as_str()) else {
        return Ok(None);
    };
    let Some(value) = ParsedValue::parse(literal.value(), datatype, false, workspace)? else {
        return Ok(None);
    };
    if !value.is_exact_numeric() {
        return Ok(None);
    }
    Ok(Some(if negative {
        negate(&value, workspace)?
    } else {
        value
    }))
}

pub(crate) fn parsed(
    value: &TermValue,
    workspace: &WorkspaceCapability,
) -> Result<Option<ParsedValue>, EvalError> {
    let TermValue::Literal {
        lexical_form,
        datatype,
        language: None,
        direction: None,
    } = value
    else {
        return Ok(None);
    };
    let Some(datatype) = XsdDatatype::from_iri(datatype) else {
        return Ok(None);
    };
    let value = ParsedValue::parse(lexical_form, datatype, false, workspace)?;
    Ok(value.filter(|value| value.is_exact_numeric()))
}

pub(crate) fn compare(
    a: &XsdValue,
    b: &XsdValue,
    workspace: &WorkspaceCapability,
) -> Result<Ordering, EvalError> {
    let mut frame = NumericFrame::new(workspace);
    let comparison =
        purrdf_xsd::numeric::numeric_cmp_admitted(a, b, &mut |layout| frame.admit(layout));
    frame
        .finish_scalar(comparison)?
        .ok_or(EvalError::WorkspaceBoundOverflow)
}

fn operation(
    a: &XsdValue,
    b: &XsdValue,
    operator: purrdf_xsd::numeric::NumericBinaryOperator,
    workspace: &WorkspaceCapability,
) -> Result<ParsedValue, EvalError> {
    let mut frame = NumericFrame::new(workspace);
    let result = purrdf_xsd::numeric::numeric_binary_admitted(
        a,
        b,
        operator,
        purrdf_xsd::exact::DivisionPolicy::Exact,
        &mut |layout| frame.admit(layout),
    );
    frame
        .finish_value(result)?
        .map_err(|_| EvalError::WorkspaceBoundOverflow)
}
fn negate(value: &XsdValue, workspace: &WorkspaceCapability) -> Result<ParsedValue, EvalError> {
    operation(
        &XsdValue::Integer {
            value: 0,
            datatype: XsdDatatype::Integer,
        },
        value,
        purrdf_xsd::numeric::NumericBinaryOperator::Subtract,
        workspace,
    )
}

pub(crate) struct Range {
    pub(crate) lower: Option<ParsedValue>,
    pub(crate) upper: Option<ParsedValue>,
    pub(crate) strict: bool,
}
impl WindowSpec {
    pub(crate) fn range(
        &self,
        other: &XsdValue,
        new_left: bool,
        workspace: &WorkspaceCapability,
    ) -> Result<Range, EvalError> {
        use purrdf_xsd::numeric::NumericBinaryOperator::{Add, Subtract};
        let lower = (self.absolute || !new_left)
            .then(|| operation(other, &self.bound, Subtract, workspace))
            .transpose()?;
        let upper = (self.absolute || new_left)
            .then(|| operation(other, &self.bound, Add, workspace))
            .transpose()?;
        Ok(Range {
            lower,
            upper,
            strict: self.strict,
        })
    }
}

pub(crate) fn execute<D: DatasetView + Sync>(
    node: &GraphPattern,
    spec: &WindowSpec,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let GraphPattern::Filter { inner, expr } = node else {
        return Err(EvalError::WorkspaceBoundOverflow);
    };
    let GraphPattern::Bgp { patterns } = &**inner else {
        return Err(EvalError::WorkspaceBoundOverflow);
    };
    let previous = ctx.window_bgp.take();
    let sequential = ctx.options.force_sequential;
    let mut selected = spec.clone();
    // Source-declared expression gaps are reached on ordinary false rows too.
    // Keep every original pair until the original FILTER owner decides it.
    selected.restrict &= ctx.evidence_input.is_none();
    // The barrier cache is owned by this ordered FILTER context. Reached eager
    // residuals keep literal pair order and share one domain preparation.
    if !selected.restrict {
        ctx.options.force_sequential = true;
    }
    ctx.window_bgp = Some((patterns.as_ptr() as usize, selected));
    // The original native FILTER/BGP checkpoints, lifting, residual evaluation
    // and publication remain the only path. The marker changes candidate access.
    let result = crate::expr::eval_filter(node, expr, inner, ctx);
    ctx.window_bgp = previous;
    ctx.options.force_sequential = sequential;
    result
}
