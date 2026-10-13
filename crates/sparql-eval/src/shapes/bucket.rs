// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::{Builtin, ShapeOffer};
use crate::AdmittedVec;
use crate::eval::EvalCtx;
use crate::governor::lift::Evaluated;
use crate::solution::{RetainedRow, SolutionSeq};
use crate::{EvalError, WorkspaceCapability};
use purrdf_core::DatasetView;
use purrdf_sparql_algebra::{Expression, Function, GraphPattern};
use purrdf_xsd::XsdDatatype;

type Partition<I> = AdmittedVec<(usize, RetainedRow<I>, AdmittedVec<usize>)>;

pub(crate) fn offer(
    pattern: &GraphPattern,
    _workspace: &WorkspaceCapability,
) -> Result<Option<ShapeOffer>, EvalError> {
    let GraphPattern::Group {
        inner, variables, ..
    } = pattern
    else {
        return Ok(None);
    };
    let mut current = &**inner;
    while let GraphPattern::Extend {
        inner,
        variable,
        expression,
    } = current
    {
        if variables.contains(variable)
            && let Expression::FunctionCall(Function::Custom(iri), arguments) = expression
            && arguments.len() == 1
            && matches!(
                XsdDatatype::from_iri(iri.as_str()),
                Some(XsdDatatype::Date | XsdDatatype::GYearMonth)
            )
        {
            return Ok(Some(ShapeOffer::builtin(
                "typed-bucketed-series",
                Builtin::Bucket,
            )));
        }
        current = inner;
    }
    Ok(None)
}
pub(crate) fn execute<D: DatasetView + Sync>(
    node: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let previous = ctx.bucketed_group;
    ctx.bucketed_group = Some(std::ptr::from_ref(node) as usize);
    let result = crate::eval::eval_literal_node(node, ctx);
    ctx.bucketed_group = previous;
    result
}

/// Native identity equality is the grouping law after the original typed cast.
/// Sorting never merges different date timezone spellings or unbound keys.
pub(crate) fn partition<I: purrdf_core::ViewTermId>(
    seq: &SolutionSeq<I>,
    columns: &[Option<usize>],
    workspace: &WorkspaceCapability,
) -> Result<Partition<I>, EvalError> {
    let mut ordered = AdmittedVec::with_capacity(seq.rows.len(), workspace)?;
    for (ordinal, row) in seq.rows.iter().enumerate() {
        let key = RetainedRow::from_cells(
            columns.len(),
            columns
                .iter()
                .map(|column| column.and_then(|column| row[column])),
            workspace,
        )?;
        ordered.push((ordinal, key))?;
    }
    ordered.as_mut_slice().sort_unstable_by(|left, right| {
        for (left, right) in left.1.iter().zip(right.1.iter()) {
            let order = compare_cell(*left, *right);
            if order != core::cmp::Ordering::Equal {
                return order;
            }
        }
        left.0.cmp(&right.0)
    });
    let mut groups: Partition<I> = AdmittedVec::new(workspace);
    for (ordinal, key) in ordered {
        if let Some((_, previous, members)) = groups.as_mut_slice().last_mut()
            && *previous == key
        {
            members.push(ordinal)?;
        } else {
            let mut members = AdmittedVec::new(workspace);
            members.push(ordinal)?;
            groups.push((ordinal, key, members))?;
        }
    }
    groups
        .as_mut_slice()
        .sort_unstable_by_key(|(first, _, _)| *first);
    // Ordinals are restored too: the aggregate's first error, rounding and
    // SAMPLE/FOLD/custom accumulator order remain exactly the original ones.
    for (_, _, members) in groups.as_mut_slice() {
        members.as_mut_slice().sort_unstable();
    }
    for (ordinal, (first, _, _)) in groups.as_mut_slice().iter_mut().enumerate() {
        *first = ordinal;
    }
    Ok(groups)
}

fn compare_cell<I: Ord>(
    left: Option<crate::scratch::SolutionTerm<I>>,
    right: Option<crate::scratch::SolutionTerm<I>>,
) -> core::cmp::Ordering {
    use crate::scratch::SolutionTerm::{Computed, Existing};
    match (left, right) {
        (None, None) => core::cmp::Ordering::Equal,
        (None, Some(_)) => core::cmp::Ordering::Less,
        (Some(_), None) => core::cmp::Ordering::Greater,
        (Some(Existing(left)), Some(Existing(right))) => left.cmp(&right),
        (Some(Computed(left)), Some(Computed(right))) => left.cmp(&right),
        (Some(Existing(_)), Some(Computed(_))) => core::cmp::Ordering::Less,
        (Some(Computed(_)), Some(Existing(_))) => core::cmp::Ordering::Greater,
    }
}
