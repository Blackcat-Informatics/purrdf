// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Certified workspace admission for a bounded operational read session.
//!
//! Plan preparation belongs to its caller's bounded plan cache. This admission
//! covers execution, simultaneous intermediate bags, and owned result staging.
//! The returned guard must stay alive through the engine's complete result drain.

use purrdf_core::{DatasetView, GraphMatch, RdfDiagnostic, WorkspaceReservation};
use purrdf_sparql_algebra::{GraphPattern, NamedNodePattern, Query, TermPattern};

use crate::EvalError;

fn overflow() -> EvalError {
    EvalError::WorkspaceBoundOverflow
}
fn add(left: u64, right: u64) -> Result<u64, EvalError> {
    left.checked_add(right).ok_or_else(overflow)
}
fn mul(left: u64, right: u64) -> Result<u64, EvalError> {
    left.checked_mul(right).ok_or_else(overflow)
}
fn logical(value: usize) -> Result<u64, EvalError> {
    u64::try_from(value).map_err(|_| overflow())
}
fn diagnostic(error: &EvalError) -> RdfDiagnostic {
    RdfDiagnostic::error(
        super::eval_diagnostic_code(error, "native-sparql-query-eval"),
        error.to_string(),
    )
}

/// Fixed governor/report scaffolding is admitted before constructing its heap
/// owner. Resident monomorphs retain the same zero-sized no-op reservation.
pub(super) fn reserve_reporting<D: DatasetView>(
    view: &D,
) -> Result<impl WorkspaceReservation<Error = D::ReadError> + '_, D::ReadError> {
    view.reserve_workspace(if view.storage_live_budget().is_some() {
        8192
    } else {
        0
    })
}

/// Admit before converting even a query constant to an owned lookup key. Every
/// subsequent execution allocation is covered by the grown reservation.
/// Resident views retain their zero-sized reservation and ordinary fast path.
pub(super) fn reserve<'a, D: DatasetView>(
    view: &'a D,
    query: &Query,
    has_substitutions: bool,
    options: super::QueryOptions<'_>,
) -> Result<impl WorkspaceReservation<Error = D::ReadError> + 'a, RdfDiagnostic> {
    let _reporting =
        reserve_reporting(view).map_err(|error| diagnostic(&EvalError::source_read(error)))?;
    check_inputs(view, has_substitutions, options)?;
    reserve_checked(view, query).map_err(|error| diagnostic(&error))
}

/// Refuse known unpriced caller inputs before request parameter metadata can
/// allocate. The caller retains its reporting admission through this projection.
pub(super) fn check_inputs<D: DatasetView>(
    view: &D,
    has_substitutions: bool,
    options: super::QueryOptions<'_>,
) -> Result<(), RdfDiagnostic> {
    if view.storage_live_budget().is_some()
        && (!options.functions.is_empty()
            || !options.property_functions().is_empty()
            || !options.aggregates().is_empty()
            || options.bnode_mint_prefix.is_some()
            || options.focus_graph.is_some()
            || options.call_depth != 0
            || options.remote.is_some()
            || options.load.is_some())
    {
        return Err(diagnostic(&EvalError::WorkspaceUnpriced(
            "a configured extension context",
        )));
    }
    if has_substitutions && view.storage_live_budget().is_some() {
        return Err(diagnostic(&EvalError::WorkspaceUnpriced(
            "prebound substitutions",
        )));
    }
    Ok(())
}

fn reserve_checked<'a, D: DatasetView>(
    view: &'a D,
    query: &Query,
) -> Result<impl WorkspaceReservation<Error = D::ReadError> + 'a, EvalError> {
    if view.storage_live_budget().is_none() {
        return view.reserve_workspace(0).map_err(EvalError::source_read);
    }
    let Query::Select {
        pattern, dataset, ..
    } = query
    else {
        return Err(EvalError::WorkspaceUnpriced("the query form"));
    };
    if !dataset.default.is_empty() || !dataset.named.is_empty() {
        return Err(EvalError::WorkspaceUnpriced("an explicit active dataset"));
    }
    let GraphPattern::Project { inner, variables } = pattern else {
        return Err(EvalError::WorkspaceUnpriced("solution modifiers"));
    };
    let GraphPattern::Bgp { patterns } = &**inner else {
        return Err(EvalError::WorkspaceUnpriced(
            "algebra outside a basic graph pattern",
        ));
    };
    let term_bytes = view
        .max_owned_term_bytes()
        .ok_or(EvalError::WorkspaceUnpriced(
            "a dataset without a certified owned-term bound",
        ))?;
    let count = logical(patterns.len())?;
    let width = add(mul(count, 3)?, logical(variables.len())?)?.max(1);
    let base_bytes = logical(query.base_iri().map_or(0, |base| base.as_str().len()))?;
    let mut authored_bytes = variables.iter().try_fold(base_bytes, |total, variable| {
        add(total, logical(variable.as_str().len())?)
    })?;
    let mut name_bytes = variables.iter().try_fold(0_u64, |maximum, variable| {
        Ok::<_, EvalError>(maximum.max(logical(variable.as_str().len())?))
    })?;
    for pattern in patterns {
        for term in [&pattern.subject, &pattern.object] {
            let bytes = match term {
                TermPattern::NamedNode(node) => logical(node.as_str().len())?,
                TermPattern::Literal(literal) => add(
                    add(
                        logical(literal.value().len())?,
                        logical(literal.datatype().as_str().len())?,
                    )?,
                    logical(literal.language().map_or(0, str::len))?,
                )?,
                TermPattern::Variable(variable) => {
                    name_bytes = name_bytes.max(logical(variable.as_str().len())?);
                    logical(variable.as_str().len())?
                }
                TermPattern::BlankNode(blank) => {
                    name_bytes = name_bytes.max(add(logical(blank.as_str().len())?, 32)?);
                    logical(blank.as_str().len())?
                }
                TermPattern::Triple(_) => {
                    return Err(EvalError::WorkspaceUnpriced("a nested triple pattern"));
                }
            };
            authored_bytes = add(authored_bytes, bytes)?;
        }
        let predicate_bytes = match &pattern.predicate {
            NamedNodePattern::NamedNode(node) => logical(node.as_str().len())?,
            NamedNodePattern::Variable(variable) => {
                name_bytes = name_bytes.max(logical(variable.as_str().len())?);
                logical(variable.as_str().len())?
            }
        };
        authored_bytes = add(authored_bytes, predicate_bytes)?;
    }
    // Query lookup keys, schema/compiled-pattern scaffolding, contexts and charge
    // tables. The eightfold text allowance covers lexical binding expansion and
    // overlapping input/key copies; all terms here are non-recursive.
    let base = add(
        32_768,
        add(mul(authored_bytes, 8)?, mul(add(count, width)?, 512)?)?,
    )?;
    let reservation = view
        .reserve_workspace(base)
        .map_err(EvalError::source_read)?;
    let mut rows = 1_u64;
    for pattern in patterns {
        let (subject, missing_subject) = term_id(view, &pattern.subject)?;
        let (object, missing_object) = term_id(view, &pattern.object)?;
        let (predicate, missing_predicate) = match &pattern.predicate {
            NamedNodePattern::Variable(_) => (None, false),
            NamedNodePattern::NamedNode(node) => {
                let value = crate::convert::named_node_to_value(node);
                let id = view
                    .term_id_by_value(&value)
                    .map_err(EvalError::source_read)?;
                (id, id.is_none())
            }
        };
        let cardinality = if missing_subject || missing_object || missing_predicate {
            0
        } else {
            view.cardinality_estimate(subject, predicate, object, GraphMatch::Default)
        };
        if let Some(error) = view.read_error() {
            return Err(EvalError::source_read(error));
        }
        // Each prefix in any join order has at most this product's rows. A zero
        // pattern contributes one to keep all earlier prefixes covered as well.
        rows = mul(rows, cardinality.max(1))?;
    }
    let stages = add(count, 4)?;
    let cells = mul(mul(stages, rows)?, width)?;
    // A cell allowance includes dense rows, both sides of Vec capacity growth,
    // bindings, schema maps and projection indices. Four simultaneous expanded
    // owned values cover scratch/key/result copies and growing owned vectors.
    let cell_bytes = add(256, add(mul(name_bytes, 4)?, mul(term_bytes, 4)?)?)?;
    let bags = mul(cells, cell_bytes)?;
    // Selinger's planner uses at most 256 DP states (through eight patterns),
    // then greedy O(N²) work. Price both bounds without depending on dictionary
    // cardinality: unrelated stored terms do not become operator scratch.
    let planner = add(32_768, mul(mul(count, count)?, add(128, mul(width, 32)?)?)?)?;
    let full = add(base, add(bags, planner)?)?;
    drop(reservation);
    view.reserve_workspace(full).map_err(EvalError::source_read)
}

fn term_id<D: DatasetView>(
    view: &D,
    term: &TermPattern,
) -> Result<(Option<D::Id>, bool), EvalError> {
    match term {
        TermPattern::Variable(_) | TermPattern::BlankNode(_) => Ok((None, false)),
        TermPattern::NamedNode(_) | TermPattern::Literal(_) => {
            let value = crate::convert::ground_term_pattern_to_value(term, "workspace admission")?;
            let id = view
                .term_id_by_value(&value)
                .map_err(EvalError::source_read)?;
            Ok((id, id.is_none()))
        }
        TermPattern::Triple(_) => Err(EvalError::WorkspaceUnpriced("a nested triple pattern")),
    }
}
