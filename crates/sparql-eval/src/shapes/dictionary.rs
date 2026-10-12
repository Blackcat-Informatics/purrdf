// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact phrase-adapter and original expanded positional dictionary certificates.

use super::{Builtin, ShapeOffer};
use crate::eval::EvalCtx;
use crate::governor::{
    ChargePoint,
    lift::{Evaluated, Lift, Truncation},
};
use crate::property_fn::{OccurrenceDictionary, OccurrenceDictionaryInput, ServiceLevel};
use crate::property_fn_eval::OccurrenceCallPlans;
use crate::solution::{RowsBuilder, SolutionSeq, VarSchema};
use crate::workspace::SharedWorkspace;
use crate::{AdmittedMap, AdmittedVec, EvalError, WorkspaceCapability};
use purrdf_core::{DatasetView, ViewTermId};
use purrdf_lex::allocation::SharedText;
use purrdf_sparql_algebra::{
    ArithmeticOperator, Expression, GraphPattern, PropertyFunctionCall, TermPattern, Variable,
};

pub(crate) fn offer(
    pattern: &GraphPattern,
    relations: &crate::PropertyFunctionRegistry,
    has_evidence_source: bool,
    workspace: &WorkspaceCapability,
) -> Result<Option<ShapeOffer>, EvalError> {
    if let GraphPattern::Lateral { right, .. } = pattern
        && let GraphPattern::PropertyFunction(call) = &**right
        && let Some(relation) = relations.resolve(call.iri.as_str())
    {
        let capability = crate::contain::declaration_contained_admitted(
            "property function",
            call.iri.as_str(),
            "dictionary shape",
            || relation.dictionary_shape(),
            workspace,
        )?;
        if capability.is_some() {
            return Ok(Some(ShapeOffer::builtin(
                "dictionary-match",
                Builtin::Dictionary,
            )));
        }
    }
    // The original adjacency FILTER calls the source for every original row,
    // including ordinary false rows. A known positional mismatch is not an
    // evidence-source declaration that no expression gap exists.
    if has_evidence_source {
        return Ok(None);
    }
    let (roots, union) = match pattern {
        GraphPattern::Union { arms } => (arms.as_slice(), true),
        other => (std::slice::from_ref(other), false),
    };
    let mut arms = AdmittedVec::new(workspace);
    let mut inputs = AdmittedVec::new(workspace);
    let mut entries = AdmittedVec::new(workspace);
    let mut iri: Option<&str> = None;
    for (ordinal, pattern) in roots.iter().enumerate() {
        let branch = union.then_some(ordinal);
        let Some((values, calls, inner_path, values_path)) = certificate(pattern, workspace)?
        else {
            return Ok(None);
        };
        let GraphPattern::Values {
            variables,
            bindings,
        } = values
        else {
            return Ok(None);
        };
        for call in &calls {
            if iri.is_some_and(|iri| iri != call.iri) {
                return Ok(None);
            }
            iri = Some(call.iri.as_str());
        }
        let schema = VarSchema::from_vars_admitted(variables.iter().cloned(), workspace)?;
        let plans = OccurrenceCallPlans::compile(&calls, &schema, workspace)?;
        let arm_ordinal = arms.len();
        for (row_ordinal, row) in bindings.iter().enumerate() {
            let mut payload = AdmittedVec::with_capacity(row.len(), workspace)?;
            for value in row {
                payload.push(
                    value
                        .as_ref()
                        .map(|value| {
                            crate::convert::ground_term_to_workspace_value(value, workspace)
                        })
                        .transpose()?,
                )?;
            }
            let mut tokens = AdmittedVec::with_capacity(calls.len(), workspace)?;
            for call in &calls {
                let token = match &call.object_args[0] {
                    TermPattern::Variable(variable) => {
                        let Some(column) =
                            variables.iter().position(|candidate| candidate == variable)
                        else {
                            return Ok(None);
                        };
                        let Some(Some(value)) = payload.get(column) else {
                            return Ok(None);
                        };
                        workspace.clone_term(value)?
                    }
                    TermPattern::Literal(_) => {
                        crate::convert::ground_term_pattern_to_workspace_value(
                            &call.object_args[0],
                            "dictionary token",
                            workspace,
                        )?
                    }
                    _ => return Ok(None),
                };
                tokens.push(token)?;
            }
            entries.push((arm_ordinal, row_ordinal))?;
            inputs.push(OccurrenceDictionaryInput { payload, tokens })?;
        }
        arms.push(Arm {
            branch,
            inner_path,
            values_path,
            plans,
        })?;
    }
    let Some(iri) = iri else {
        return Ok(None);
    };
    let Some(relation) = relations.resolve(iri) else {
        return Ok(None);
    };
    let capability = crate::contain::declaration_contained_admitted(
        "property function",
        iri,
        "complete occurrence shape",
        || relation.occurrence_shape(),
        workspace,
    )?;
    if capability.is_none() {
        return Ok(None);
    }
    let prepared = crate::contain::call_contained_admitted(
        "property function",
        iri,
        "compile occurrence dictionary",
        || relation.prepare_occurrence_dictionary(&inputs, workspace),
        workspace,
    )?;
    let Some(prepared) = prepared else {
        return Ok(None);
    };
    Ok(Some(ShapeOffer::builtin(
        "dictionary-match",
        Builtin::ExpandedDictionary(ExpandedDictionary {
            arms,
            entries,
            prepared,
            iri: workspace.authored_text(iri)?,
        }),
    )))
}

struct Arm {
    branch: Option<usize>,
    inner_path: AdmittedVec<usize>,
    values_path: AdmittedVec<usize>,
    plans: OccurrenceCallPlans,
}
pub(crate) struct ExpandedDictionary {
    arms: AdmittedVec<Arm>,
    entries: AdmittedVec<(usize, usize)>,
    prepared: Box<dyn OccurrenceDictionary>,
    iri: SharedText,
}

type Certificate<'a> = (
    &'a GraphPattern,
    AdmittedVec<&'a PropertyFunctionCall>,
    AdmittedVec<usize>,
    AdmittedVec<usize>,
);
fn certificate<'a>(
    pattern: &'a GraphPattern,
    workspace: &WorkspaceCapability,
) -> Result<Option<Certificate<'a>>, EvalError> {
    let candidate = match pattern {
        GraphPattern::Filter { inner, .. } => &**inner,
        other => other,
    };
    if !matches!(candidate, GraphPattern::Lateral { .. }) {
        return Ok(None);
    }
    let mut inner_path = AdmittedVec::new(workspace);
    let (mut inner, filter) = match pattern {
        GraphPattern::Filter { inner, expr } => {
            inner_path.push(0)?;
            (&**inner, Some(expr))
        }
        other => (other, None),
    };
    let mut values_path = AdmittedVec::new(workspace);
    for &ordinal in &inner_path {
        values_path.push(ordinal)?;
    }
    let mut calls = AdmittedVec::new(workspace);
    while let GraphPattern::Lateral { left, right } = inner {
        let GraphPattern::PropertyFunction(call) = &**right else {
            return Ok(None);
        };
        calls.push(call)?;
        values_path.push(0)?;
        inner = left;
    }
    let GraphPattern::Values { variables, .. } = inner else {
        return Ok(None);
    };
    if calls.is_empty() {
        return Ok(None);
    }
    calls.as_mut_slice().reverse();
    let mut positions: AdmittedVec<&Variable> = AdmittedVec::with_capacity(calls.len(), workspace)?;
    let mut domain = None;
    for call in &calls {
        let (
            [TermPattern::Variable(document)],
            [
                token,
                TermPattern::Variable(language),
                TermPattern::Variable(position),
            ],
        ) = (call.subject_args.as_slice(), call.object_args.as_slice())
        else {
            return Ok(None);
        };
        if !matches!(token, TermPattern::Variable(_) | TermPattern::Literal(_))
            || variables.contains(document)
            || variables.contains(language)
            || variables.contains(position)
            || document == language
            || document == position
            || language == position
            || positions.contains(&position)
        {
            return Ok(None);
        }
        if let Some((prior_document, prior_language)) = domain {
            if prior_document != document || prior_language != language {
                return Ok(None);
            }
        } else {
            domain = Some((document, language));
        }
        positions.push(position)?;
    }
    if !adjacency(filter, &positions, workspace)? {
        return Ok(None);
    }
    Ok(Some((inner, calls, inner_path, values_path)))
}

fn adjacency(
    filter: Option<&Expression>,
    positions: &[&Variable],
    workspace: &WorkspaceCapability,
) -> Result<bool, EvalError> {
    let Some(filter) = filter else {
        return Ok(positions.len() == 1);
    };
    let mut found = AdmittedVec::with_capacity(positions.len().saturating_sub(1), workspace)?;
    for _ in 1..positions.len() {
        found.push(false)?;
    }
    let mut pending = AdmittedVec::new(workspace);
    pending.push(filter)?;
    while let Some(expression) = pending.pop() {
        if let Expression::And(parts) = expression {
            for part in parts {
                pending.push(part)?;
            }
            continue;
        }
        let Expression::Equal(a, b) = expression else {
            return Ok(false);
        };
        let mut gap = None;
        for (plain, offset) in [(&**a, &**b), (&**b, &**a)] {
            let Expression::Variable(next) = plain else {
                continue;
            };
            let Expression::Arithmetic(first, rest) = offset else {
                continue;
            };
            let Expression::Variable(previous) = &**first else {
                continue;
            };
            let [(ArithmeticOperator::Add, Expression::Literal(literal))] = &**rest else {
                continue;
            };
            let Some(datatype) = purrdf_xsd::XsdDatatype::from_iri(literal.datatype().as_str())
            else {
                continue;
            };
            let Some(value) = crate::parsed_value::ParsedValue::parse(
                literal.value(),
                datatype,
                false,
                workspace,
            )?
            else {
                continue;
            };
            if !value.is_exact_numeric()
                || super::window::compare(
                    &value,
                    &purrdf_xsd::XsdValue::Integer {
                        value: purrdf_xsd::exact::Integer::ONE,
                        datatype: purrdf_xsd::XsdDatatype::Integer,
                    },
                    workspace,
                )? != core::cmp::Ordering::Equal
            {
                continue;
            }
            gap = positions
                .windows(2)
                .position(|pair| pair[0] == previous && pair[1] == next);
        }
        let Some(gap) = gap else {
            return Ok(false);
        };
        found.as_mut_slice()[gap] = true;
    }
    Ok(found.iter().all(|&found| found))
}

fn follow<'a>(
    mut pattern: &'a GraphPattern,
    path: &[usize],
) -> Result<&'a GraphPattern, EvalError> {
    for &ordinal in path {
        pattern = match (pattern, ordinal) {
            (GraphPattern::Filter { inner, .. }, 0) => inner,
            (GraphPattern::Lateral { left, .. }, 0) => left,
            _ => return Err(EvalError::WorkspaceBoundOverflow),
        };
    }
    Ok(pattern)
}
fn arm_root<'a>(root: &'a GraphPattern, arm: &Arm) -> Result<&'a GraphPattern, EvalError> {
    match (root, arm.branch) {
        (GraphPattern::Union { arms }, Some(ordinal)) => {
            arms.get(ordinal).ok_or(EvalError::WorkspaceBoundOverflow)
        }
        (_, None) => Ok(root),
        _ => Err(EvalError::WorkspaceBoundOverflow),
    }
}

/// Immutable temporary bags indexed only while their original AST is alive.
/// They retain the original rows and complete search handles through every fork.
pub(crate) struct Materialization<I: ViewTermId> {
    bags: AdmittedMap<usize, SolutionSeq<I>>,
}
impl<I: ViewTermId> Materialization<I> {
    pub(crate) fn get(&self, pattern: &GraphPattern) -> Option<&SolutionSeq<I>> {
        self.bags.get(&(std::ptr::from_ref(pattern) as usize))
    }
}

pub(crate) fn execute<D: DatasetView + Sync>(
    root: &GraphPattern,
    spec: &ExpandedDictionary,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let growth = ctx.growth.clone();
    let schema = crate::eval::syntactic_schema_admitted(root, &growth)?;
    let mut inputs = AdmittedVec::with_capacity(spec.arms.len(), &growth)?;
    let mut outputs = AdmittedVec::with_capacity(spec.arms.len(), &growth)?;
    let mut lift = Lift::at_admitted(root, &growth)?;
    for (ordinal, arm) in spec.arms.iter().enumerate() {
        let values = follow(arm_root(root, arm)?, &arm.values_path)?;
        let Some(input) = lift.absorb(ordinal, crate::eval::eval_evaluated(values, ctx)?) else {
            return Ok(lift.finish(SolutionSeq::empty(schema)));
        };
        inputs.push(input)?;
        outputs.push(RowsBuilder::new(&growth))?;
    }
    if spec.entries.is_empty() {
        // No invocation was reached, including in the retained search ledger.
        return install(root, spec, &inputs, outputs, ctx);
    }
    if let Err(tripped) = ctx.charge(ChargePoint::PropertyFunctionInvocation) {
        return Ok(Evaluated::Truncated(Truncation::barred_at(
            root, tripped, schema,
        )));
    }
    if let Some(tripped) = ctx.stop_check() {
        return Ok(Evaluated::Truncated(Truncation::barred_at(
            root, tripped, schema,
        )));
    }
    let mut matches = crate::contain::call_contained_admitted(
        "property function",
        &spec.iri,
        "scan occurrence dictionary",
        || spec.prepared.scan(&growth),
        &growth,
    )?;
    let work = crate::contain::declaration_contained_admitted(
        "property function",
        &spec.iri,
        "occurrence dictionary work",
        || matches.work(),
        &growth,
    )?;
    ctx.shape_state
        .work
        .add(work.source_visits, work.transitions)?;
    let fuel = work
        .source_visits
        .checked_add(work.transitions)
        .ok_or(EvalError::WorkspaceBoundOverflow)?;
    if let Err(tripped) = ctx.charge_occurrences(ChargePoint::PropertyFunctionWork, fuel) {
        return Ok(Evaluated::Truncated(Truncation::barred_at(
            root, tripped, schema,
        )));
    }
    if ctx.witnessing {
        let generation = crate::contain::declaration_contained_admitted(
            "property function",
            &spec.iri,
            "occurrence dictionary generation",
            || matches.generation(),
            &growth,
        )?;
        ctx.witness
            .try_record(&spec.iri, &generation, &ServiceLevel::Undeclared, &growth)?;
    }
    while let Some((entry, rows)) = crate::contain::call_contained_admitted(
        "property function",
        &spec.iri,
        "pull occurrence dictionary",
        || matches.next(&growth),
        &growth,
    )? {
        if let Some(tripped) = ctx.stop_check() {
            return Ok(Evaluated::Truncated(Truncation::barred_at(
                root, tripped, schema,
            )));
        }
        let &(arm, input) = spec
            .entries
            .get(entry)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        let mu = inputs[arm]
            .rows
            .get(input)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        let ingest = crate::row_ingest::GovernedRowIngest::new(
            ctx,
            spec.arms[arm].plans.schema.len(),
            Some(ChargePoint::PropertyFunctionRow),
        );
        if let crate::row_ingest::IngestVerdict::Abandoned(tripped) =
            ingest.admit(ctx, outputs[arm].len())
        {
            let tripped = tripped.ok_or(EvalError::WorkspaceBoundOverflow)?;
            return Ok(Evaluated::Truncated(Truncation::barred_at(
                root, tripped, schema,
            )));
        }
        if let Some(row) = spec.arms[arm].plans.bind(mu, &rows, ctx)? {
            outputs.as_mut_slice()[arm].push_row(row)?;
        }
    }
    install(root, spec, &inputs, outputs, ctx)
}

fn install<D: DatasetView + Sync>(
    root: &GraphPattern,
    spec: &ExpandedDictionary,
    inputs: &AdmittedVec<SolutionSeq<D::Id>>,
    outputs: AdmittedVec<RowsBuilder<D::Id>>,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut bags = AdmittedMap::default();
    for ((arm, input), output) in spec.arms.iter().zip(inputs).zip(outputs) {
        let mut rows = output.finish()?;
        // Complete native occurrence has uniform producer evidence. The WHOLE
        // driving search handle is retained, including the empty-output case.
        rows.retain_search_evidence(input.rows.search_evidence().clone());
        let inner = follow(arm_root(root, arm)?, &arm.inner_path)?;
        bags.insert_admitted(
            std::ptr::from_ref(inner) as usize,
            SolutionSeq {
                schema: arm.plans.schema.clone(),
                rows,
            },
            &ctx.growth,
        )?;
    }
    // A one-token region can be the offered root itself. Dispatching it
    // literally would bypass the overlay and reopen the original producer.
    if let Some(seq) = bags.get(&(std::ptr::from_ref(root) as usize)) {
        return Ok(Evaluated::Complete(seq.clone()));
    }
    let materialized = SharedWorkspace::new_admitted(Materialization { bags }, &ctx.growth)?;
    let previous = ctx.expanded_dictionary.replace(materialized);
    let result = crate::eval::eval_literal_node(root, ctx);
    ctx.expanded_dictionary = previous;
    result
}
