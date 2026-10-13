// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The PurRDF `rdf:List` SPARQL extension functions.
//!
//! These bind the FnO list primitives — `listLength`, `listGet`, `listIndexOf`,
//! `listSlice`, `listConcat`, `listContains` — to executable SPARQL extension
//! functions, so a query can spell `ext:listLength(?list)` under whatever extension-function
//! namespace the caller configures. They are
//! recognized at parse time as members of the closed
//! [`PurrdfFn`](purrdf_sparql_algebra::PurrdfFn) registry and dispatched from the
//! `Function::Purrdf` arm of [`crate::expr`].
//!
//! Two shapes:
//!
//! * **Scalar readers** (`listLength`/`listGet`/`listIndexOf`/`listContains`) walk
//!   the `rdf:first`/`rdf:rest` chain in the dataset and return a single term. These
//!   mirror the reasoning-layer recursion (conformance case
//!   `goal-rdf-list-functions`) — parity is the contract.
//! * **Constructors** (`listSlice`/`listConcat`) invent a fresh `rdf:List`. Because
//!   a SPARQL expression returns one term, the new cells are emitted into the
//!   per-query constructed-quads buffer on [`EvalCtx`] and surface at the result
//!   boundary (CONSTRUCT output and the SELECT auxiliary graph). See
//!   [`materialize_list`].
//!
//! The walk is cycle-guarded: a cyclic or torn `rdf:List` is malformed input and
//! hard-fails with the native data diagnostic rather than looping forever.

use purrdf_core::collections::{
    ListVocab, SoleObject, try_build_rdf_list_with_copy, walk_rdf_list_with_memory,
};
use purrdf_core::{BlankScope, DatasetView, ListError, ListErrorKind, TermValue};
use purrdf_lex::allocation::Memory;
use purrdf_sparql_algebra::PurrdfFn;
use purrdf_xsd::{XsdDatatype, XsdValue};

use crate::error::{EvalError, NativeDiagnostic, NativeDiagnosticKind};
use crate::eval::EvalCtx;
use crate::expr::{arg, intern_boolean, xsd_to_term};
use crate::parsed_value::ParsedValue;
use crate::scratch::SolutionTerm;
use crate::workspace::{AdmittedVec, LexicalFrame, WorkspaceDebug};
use crate::{WorkspaceCapability, WorkspaceTerm};

use purrdf_iri::vocab::rdf::FIRST as RDF_FIRST;
use purrdf_iri::vocab::rdf::NIL as RDF_NIL;
use purrdf_iri::vocab::rdf::REST as RDF_REST;

/// Evaluate the same six list functions through their original native owners.
pub(crate) fn dispatch<D: DatasetView + Sync>(
    func: PurrdfFn,
    vals: &[Option<TermValue>],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    match func {
        PurrdfFn::ListLength => list_length(ctx, vals),
        PurrdfFn::ListGet => list_get(ctx, vals),
        PurrdfFn::ListIndexOf => list_index_of(ctx, vals),
        PurrdfFn::ListContains => list_contains(ctx, vals),
        PurrdfFn::ListSlice => list_slice(ctx, vals),
        PurrdfFn::ListConcat => list_concat(ctx, vals),
        PurrdfFn::HeldIn => Err(NativeDiagnostic::error(
            NativeDiagnosticKind::Internal,
            "heldIn is not an rdf:List function",
            &ctx.growth,
        )),
    }
}

/// Return the cardinality without narrowing a native usize through i64.
fn intern_count<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    count: usize,
) -> Result<SolutionTerm<D::Id>, EvalError> {
    let value = i128::try_from(count).map_err(|_| EvalError::WorkspaceBoundOverflow)?;
    xsd_to_term(
        ctx,
        &XsdValue::Integer {
            value,
            datatype: XsdDatatype::Integer,
        },
    )
}

/// A valid list's exact number of members.
fn list_length<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let Some(head) = arg(vals, 0) else {
        return Ok(None);
    };
    match walk(ctx, head)? {
        Some(members) => intern_count(ctx, members.len()).map(Some),
        None => Ok(None),
    }
}

/// Move the selected member with its original lexical/box admission.
fn list_get<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let (Some(head), Some(index)) = (arg(vals, 0), arg(vals, 1)) else {
        return Ok(None);
    };
    let Some(index) = as_index(index, &ctx.growth)? else {
        return Ok(None);
    };
    let Some(members) = walk(ctx, head)? else {
        return Ok(None);
    };
    let Ok(index) = usize::try_from(index) else {
        return Ok(None);
    };
    match members.into_iter().nth(index) {
        Some(value) => ctx.intern_workspace_term(value),
        None => Ok(None),
    }
}

/// RDF identity comparisons retain the original term comparison work-list.
fn list_index_of<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let (Some(head), Some(value)) = (arg(vals, 0), arg(vals, 1)) else {
        return Ok(None);
    };
    let Some(members) = walk(ctx, head)? else {
        return Ok(None);
    };
    for (position, member) in members.iter().enumerate() {
        if ctx.growth.terms_equal(member, value)? {
            return intern_count(ctx, position).map(Some);
        }
    }
    Ok(None)
}

fn list_contains<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let (Some(head), Some(value)) = (arg(vals, 0), arg(vals, 1)) else {
        return Ok(None);
    };
    let Some(members) = walk(ctx, head)? else {
        return Ok(None);
    };
    let mut contains = false;
    for member in &members {
        if ctx.growth.terms_equal(member, value)? {
            contains = true;
            break;
        }
    }
    intern_boolean(ctx, contains).map(Some)
}

/// Clamping a nonnegative i64 beyond the target's usize range yields the end
/// of the list, as the original half-open slice law requires.
fn clamped_index(index: i64, len: usize) -> usize {
    usize::try_from(index.max(0)).unwrap_or(usize::MAX).min(len)
}

fn list_slice<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let (Some(head), Some(start), Some(end)) = (arg(vals, 0), arg(vals, 1), arg(vals, 2)) else {
        return Ok(None);
    };
    let (Some(start), Some(end)) = (as_index(start, &ctx.growth)?, as_index(end, &ctx.growth)?)
    else {
        return Ok(None);
    };
    let Some(members) = walk(ctx, head)? else {
        return Ok(None);
    };
    let lo = clamped_index(start, members.len());
    let hi = clamped_index(end, members.len()).max(lo);
    let Some(value) = materialize_list(ctx, members.into_iter().skip(lo).take(hi - lo))? else {
        return Ok(None);
    };
    ctx.intern_workspace_term(value)
}

fn list_concat<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let (Some(a), Some(b)) = (arg(vals, 0), arg(vals, 1)) else {
        return Ok(None);
    };
    // The original left-then-right walk order is observable on source failure.
    let (Some(mut left), Some(right)) = (walk(ctx, a)?, walk(ctx, b)?) else {
        return Ok(None);
    };
    left.try_extend(right)?;
    let Some(value) = materialize_list(ctx, left)? else {
        return Ok(None);
    };
    ctx.intern_workspace_term(value)
}

/// Use the sole core construction algorithm, supplying before-copy native term
/// owners and fallible metadata destinations. Partial cell emission is rolled
/// back on every stop/refusal, so an incomplete auxiliary list cannot escape.
fn materialize_list<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    members: impl IntoIterator<Item = WorkspaceTerm>,
) -> Result<Option<WorkspaceTerm>, EvalError> {
    enum Aborted {
        Stopped,
        Failed(EvalError),
    }
    let workspace = ctx.growth.clone();
    let vocab = ListVocab {
        first: workspace.iri(RDF_FIRST)?,
        rest: workspace.iri(RDF_REST)?,
        nil: workspace.iri(RDF_NIL)?,
    };
    let committed = ctx.constructed.len();
    let mut constructed = std::mem::take(&mut ctx.constructed);
    let head = try_build_rdf_list_with_copy(
        members,
        &vocab,
        |_| {
            let Some(label) = ctx.try_mint_blank_text("lc").map_err(Aborted::Failed)? else {
                return Err(Aborted::Stopped);
            };
            workspace
                .blank(label.as_str(), BlankScope::DEFAULT)
                .map_err(Aborted::Failed)
        },
        |value| workspace.clone_term(value).map_err(Aborted::Failed),
        |cell, predicate, object| {
            constructed
                .push_admitted((cell, predicate, object), &workspace)
                .map_err(Aborted::Failed)
        },
    );
    if head.is_err() {
        constructed.truncate(committed);
    }
    ctx.constructed = constructed;
    match head {
        Ok(head) => Ok(Some(head)),
        Err(Aborted::Stopped) => Ok(None),
        Err(Aborted::Failed(error)) => Err(error),
    }
}

/// The original integer-derived-only index law, with shared native parse owner.
fn as_index(value: &TermValue, workspace: &WorkspaceCapability) -> Result<Option<i64>, EvalError> {
    let TermValue::Literal {
        lexical_form,
        datatype,
        ..
    } = value
    else {
        return Ok(None);
    };
    let Some(datatype) = XsdDatatype::from_iri(datatype) else {
        return Ok(None);
    };
    let Some(value) = ParsedValue::parse(lexical_form, datatype, false, workspace)? else {
        return Ok(None);
    };
    Ok(match &*value {
        XsdValue::Integer { value, .. } => i64::try_from(*value).ok(),
        _ => None,
    })
}

/// Read dataset lists first and this query's minted lists second, unchanged.
fn walk<D: DatasetView + Sync>(
    ctx: &EvalCtx<'_, D>,
    head: &TermValue,
) -> Result<Option<AdmittedVec<WorkspaceTerm>>, EvalError> {
    if is_nil(head) {
        return Ok(Some(AdmittedVec::new(&ctx.growth)));
    }
    if let Some(members) = walk_dataset(ctx, head)? {
        return Ok(Some(members));
    }
    walk_constructed(ctx, head)
}

/// Keep the strict walk's actual native ID vector alive until every stored
/// member has been copied through the source's before-allocation term door.
fn walk_dataset<D: DatasetView + Sync>(
    ctx: &EvalCtx<'_, D>,
    head: &TermValue,
) -> Result<Option<AdmittedVec<WorkspaceTerm>>, EvalError> {
    let Some(head_id) = ctx
        .dataset
        .term_id_by_value(head)
        .map_err(|error| ctx.workspace.source_error(error))?
    else {
        return Ok(None);
    };
    let [first_value, rest_value, nil_value] = [
        ctx.growth.iri(RDF_FIRST)?,
        ctx.growth.iri(RDF_REST)?,
        ctx.growth.iri(RDF_NIL)?,
    ];
    let first = ctx
        .dataset
        .term_id_by_value(&first_value)
        .map_err(|error| ctx.workspace.source_error(error))?;
    let rest = ctx
        .dataset
        .term_id_by_value(&rest_value)
        .map_err(|error| ctx.workspace.source_error(error))?;
    let nil = ctx
        .dataset
        .term_id_by_value(&nil_value)
        .map_err(|error| ctx.workspace.source_error(error))?;
    let scope = ctx.active_dataset.scope_for(ctx.active_graph);
    let mut frame = LexicalFrame::new(&ctx.growth);
    let mut memory = Memory::new(&mut frame);
    let walked = ctx
        .dataset
        .checked_read(|dataset| {
            let objects = |cell, predicate: Option<D::Id>| {
                let mut sole = SoleObject::None;
                if let Some(predicate) = predicate {
                    scope.for_each_quad(dataset, Some(cell), Some(predicate), None, |q| {
                        sole = sole.and(q.o);
                    });
                }
                sole
            };
            walk_rdf_list_with_memory(
                head_id,
                nil,
                |cell| objects(cell, first),
                |cell| objects(cell, rest),
                &mut memory,
            )
        })
        .map_err(|error| ctx.workspace.source_error(error))?
        .map_err(|error| {
            memory
                .admission_mut()
                .storage_error(error, "rdf:List member IDs")
        })?;
    let Some(ids) = list_members(head_id, walked, |error| {
        NativeDiagnostic::error(
            NativeDiagnosticKind::Data,
            format_args!("{error}"),
            &ctx.growth,
        )
    })?
    else {
        return Ok(None);
    };
    let mut members = AdmittedVec::with_capacity(ids.len(), &ctx.growth)?;
    for id in &ids {
        let value = ctx.scratch.try_owned_value_of(
            ctx.dataset,
            SolutionTerm::Existing(*id),
            &ctx.growth,
            |error| ctx.workspace.source_error(error),
        )?;
        members.push(value)?;
    }
    memory.release_vec(ids).map_err(|error| {
        memory
            .admission_mut()
            .storage_error(error, "rdf:List member IDs")
    })?;
    Ok(Some(members))
}

/// Only list construction writes this buffer: its cell/rest identities are
/// minted flat blanks or rdf:nil. The strict iterator's identity/cycle checks
/// therefore cannot grow a nested term work-list. Member equality still uses
/// the independently admitted native term comparator above.
fn walk_constructed<D: DatasetView + Sync>(
    ctx: &EvalCtx<'_, D>,
    head: &TermValue,
) -> Result<Option<AdmittedVec<WorkspaceTerm>>, EvalError> {
    let [first, rest, nil] = [
        ctx.growth.iri(RDF_FIRST)?,
        ctx.growth.iri(RDF_REST)?,
        ctx.growth.iri(RDF_NIL)?,
    ];
    let objects = |cell: &TermValue, predicate: &TermValue| {
        SoleObject::of(
            ctx.constructed
                .iter()
                .filter(|(s, p, _)| s == cell && p == predicate)
                .map(|(_, _, o)| o),
        )
    };
    let mut frame = LexicalFrame::new(&ctx.growth);
    let mut memory = Memory::new(&mut frame);
    let walked = walk_rdf_list_with_memory(
        head,
        Some(&*nil),
        |cell| objects(cell, &first),
        |cell| objects(cell, &rest),
        &mut memory,
    )
    .map_err(|error| {
        memory
            .admission_mut()
            .storage_error(error, "constructed list members")
    })?;
    let Some(terms) = list_members(head, walked, |error| {
        let node = WorkspaceDebug::new(error.node, &ctx.growth);
        let diagnostic = NativeDiagnostic::error(
            NativeDiagnosticKind::Data,
            format_args!(
                "malformed rdf:List: {} (at {:?}, after {} member(s))",
                error.kind,
                node,
                error.members.len()
            ),
            &ctx.growth,
        );
        node.take_failure().unwrap_or(diagnostic)
    })?
    else {
        return Ok(None);
    };
    let mut members = AdmittedVec::with_capacity(terms.len(), &ctx.growth)?;
    for term in &terms {
        members.push(ctx.growth.clone_term(term)?)?;
    }
    memory.release_vec(terms).map_err(|error| {
        memory
            .admission_mut()
            .storage_error(error, "constructed list members")
    })?;
    Ok(Some(members))
}

/// Missing rdf:first at the head is unbound; every other strict-walk fault
/// keeps the original malformed-list message and native data classification.
fn list_members<Id: Copy + Eq>(
    head: Id,
    walked: Result<Vec<Id>, ListError<Id>>,
    diagnostic: impl FnOnce(&ListError<Id>) -> EvalError,
) -> Result<Option<Vec<Id>>, EvalError> {
    match walked {
        Ok(members) => Ok(Some(members)),
        Err(error)
            if error.kind == ListErrorKind::MissingFirst
                && error.node == head
                && error.members.is_empty() =>
        {
            Ok(None)
        }
        Err(error) => Err(diagnostic(&error)),
    }
}

fn is_nil(value: &TermValue) -> bool {
    matches!(value, TermValue::Iri(iri) if iri == RDF_NIL)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{DatasetHandle, RdfDataset, RdfDatasetBuilder, TermValue};

    use crate::error::EvalError;
    use crate::eval::{EvalCtx, Outcome, evaluate_query};

    /// The three-element list `(x y z)` rooted at `ex:l0`, plus an anchor triple
    /// `ex:q ex:list ex:l0` so a BGP can bind the head.
    fn list_ds() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let first = b.intern_iri(super::RDF_FIRST);
        let rest = b.intern_iri(super::RDF_REST);
        let nil = b.intern_iri(super::RDF_NIL);
        let l0 = b.intern_iri("http://ex/l0");
        let l1 = b.intern_iri("http://ex/l1");
        let l2 = b.intern_iri("http://ex/l2");
        let x = b.intern_iri("http://ex/x");
        let y = b.intern_iri("http://ex/y");
        let z = b.intern_iri("http://ex/z");
        let q = b.intern_iri("http://ex/q");
        let list = b.intern_iri("http://ex/list");
        b.push_quad(l0, first, x, None);
        b.push_quad(l0, rest, l1, None);
        b.push_quad(l1, first, y, None);
        b.push_quad(l1, rest, l2, None);
        b.push_quad(l2, first, z, None);
        b.push_quad(l2, rest, nil, None);
        b.push_quad(q, list, l0, None);
        b.freeze().expect("freeze")
    }

    /// The caller-configured extension-function namespace these tests parse with
    /// (the `g:` prefix in `PREFIX` below binds to the same namespace).
    fn ext_options() -> purrdf_sparql_algebra::ParserOptions {
        purrdf_sparql_algebra::ParserOptions {
            extension_fn_namespaces: vec!["https://example.org/ext/".to_owned()],
            property_fn_namespaces: Vec::new(),
            property_fn_iris: Vec::new(),
        }
    }

    /// Run `query` and return sorted stringified rows (a multiset comparison).
    fn rows(ds: &RdfDataset, query: &str) -> Vec<Vec<String>> {
        use purrdf_sparql_algebra::SparqlParser;
        let parsed = SparqlParser::new()
            .parse_query_with(query, &ext_options())
            .expect("parse");
        let mut ctx = EvalCtx::new(ds);
        match evaluate_query(&parsed, &mut ctx).expect("eval") {
            Outcome::Solutions(seq) => {
                let mut out: Vec<Vec<String>> = seq
                    .rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|c| match c {
                                None => "UNBOUND".to_owned(),
                                Some(t) => match ctx.scratch.value_of(ctx.dataset, *t) {
                                    TermValue::Iri(i) => format!("<{i}>"),
                                    TermValue::Literal { lexical_form, .. } => lexical_form,
                                    TermValue::Blank { label, .. } => format!("_:{label}"),
                                    TermValue::Triple { .. } => "<<triple>>".to_owned(),
                                },
                            })
                            .collect()
                    })
                    .collect();
                out.sort();
                out
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// Evaluate a query expected to hard-fail, returning the error.
    fn eval_err(ds: &RdfDataset, query: &str) -> EvalError {
        use purrdf_sparql_algebra::SparqlParser;
        let parsed = SparqlParser::new()
            .parse_query_with(query, &ext_options())
            .expect("parse");
        let mut ctx = EvalCtx::new(ds);
        evaluate_query(&parsed, &mut ctx).expect_err("expected a hard failure")
    }

    const PREFIX: &str = "PREFIX g: <https://example.org/ext/> ";

    #[test]
    fn list_length_counts_members() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?n WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listLength(?l) AS ?n) }}"
        );
        assert_eq!(rows(&ds, &q), vec![vec!["3".to_owned()]]);
    }

    /// A list whose edges are asserted in two graphs merged by `FROM` is one list:
    /// the same `rdf:first` in both graphs is one edge. A cell with two distinct
    /// `rdf:rest` objects across the merge is malformed.
    #[test]
    fn a_list_read_through_a_from_merge_counts_each_edge_once() {
        let build = |second_rest: bool| {
            let mut b = RdfDatasetBuilder::new();
            let first = b.intern_iri(super::RDF_FIRST);
            let rest = b.intern_iri(super::RDF_REST);
            let nil = b.intern_iri(super::RDF_NIL);
            let l0 = b.intern_iri("http://ex/l0");
            let l1 = b.intern_iri("http://ex/l1");
            let x = b.intern_iri("http://ex/x");
            let y = b.intern_iri("http://ex/y");
            let g1 = b.intern_iri("http://ex/g1");
            let g2 = b.intern_iri("http://ex/g2");
            b.push_quad(l0, first, x, Some(g1));
            b.push_quad(l0, rest, l1, Some(g1));
            b.push_quad(l0, first, x, Some(g2));
            b.push_quad(l1, first, y, Some(g2));
            b.push_quad(l1, rest, nil, Some(g2));
            if second_rest {
                b.push_quad(l1, rest, l0, Some(g1));
            }
            b.freeze().expect("freeze")
        };
        let q = format!(
            "{PREFIX} SELECT ?n FROM <http://ex/g1> FROM <http://ex/g2> WHERE {{ \
             BIND(g:listLength(<http://ex/l0>) AS ?n) }}"
        );
        assert_eq!(rows(&build(false), &q), vec![vec!["2".to_owned()]]);
        let err = eval_err(&build(true), &q);
        assert!(
            err.to_string().contains("more than one rdf:rest"),
            "got {err}"
        );
    }

    #[test]
    fn list_length_of_nil_is_zero() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?n WHERE {{ \
             BIND(g:listLength(<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>) AS ?n) }}"
        );
        assert_eq!(rows(&ds, &q), vec![vec!["0".to_owned()]]);
    }

    #[test]
    fn list_get_returns_indexed_member() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?x WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listGet(?l, 1) AS ?x) }}"
        );
        assert_eq!(rows(&ds, &q), vec![vec!["<http://ex/y>".to_owned()]]);
    }

    #[test]
    fn list_get_out_of_range_is_unbound() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?x WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listGet(?l, 5) AS ?x) }}"
        );
        assert_eq!(rows(&ds, &q), vec![vec!["UNBOUND".to_owned()]]);
    }

    #[test]
    fn list_index_of_finds_value() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?n WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listIndexOf(?l, <http://ex/z>) AS ?n) }}"
        );
        assert_eq!(rows(&ds, &q), vec![vec!["2".to_owned()]]);
    }

    #[test]
    fn list_index_of_absent_is_unbound() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?n WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listIndexOf(?l, <http://ex/absent>) AS ?n) }}"
        );
        assert_eq!(rows(&ds, &q), vec![vec!["UNBOUND".to_owned()]]);
    }

    #[test]
    fn list_contains_true_and_false() {
        let ds = list_ds();
        let q_true = format!(
            "{PREFIX} SELECT ?b WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listContains(?l, <http://ex/y>) AS ?b) }}"
        );
        assert_eq!(rows(&ds, &q_true), vec![vec!["true".to_owned()]]);
        let q_false = format!(
            "{PREFIX} SELECT ?b WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listContains(?l, <http://ex/absent>) AS ?b) }}"
        );
        assert_eq!(rows(&ds, &q_false), vec![vec!["false".to_owned()]]);
    }

    #[test]
    fn unknown_extension_function_is_a_parse_error() {
        // The extension-function surface is a CLOSED registry: an unrecognized
        // IRI under a configured namespace in call position fails fast at parse
        // time and never reaches evaluation.
        use purrdf_sparql_algebra::SparqlParser;
        let q = format!("{PREFIX} SELECT ?x WHERE {{ BIND(g:notAListFunction(1) AS ?x) }}");
        let err = SparqlParser::new()
            .parse_query_with(&q, &ext_options())
            .expect_err("closed registry must reject an unknown extension function");
        assert!(
            err.to_string().contains("unknown extension function"),
            "got {err}"
        );
    }

    #[test]
    fn unknown_custom_function_still_hard_fails() {
        // A custom IRI outside the configured namespace parses to
        // `Function::Custom` and hard-fails at eval.
        let ds = list_ds();
        let q = "SELECT ?x WHERE { BIND(<http://other.example/notAFunction>(1) AS ?x) }";
        let err = eval_err(&ds, q);
        assert!(matches!(err, EvalError::Unsupported { .. }), "got {err:?}");
    }

    #[test]
    fn cyclic_list_is_a_hard_data_error() {
        // l0 -> first x, rest l1 ; l1 -> first y, rest l0  (a cycle, no rdf:nil).
        let mut b = RdfDatasetBuilder::new();
        let first = b.intern_iri(super::RDF_FIRST);
        let rest = b.intern_iri(super::RDF_REST);
        let nil = b.intern_iri(super::RDF_NIL); // present so the walk starts
        let l0 = b.intern_iri("http://ex/l0");
        let l1 = b.intern_iri("http://ex/l1");
        let x = b.intern_iri("http://ex/x");
        let y = b.intern_iri("http://ex/y");
        let z = b.intern_iri("http://ex/z");
        b.push_quad(l0, first, x, None);
        b.push_quad(l0, rest, l1, None);
        b.push_quad(l1, first, y, None);
        b.push_quad(l1, rest, l0, None);
        // A well-formed terminator elsewhere so rdf:nil is interned.
        b.push_quad(z, rest, nil, None);
        let ds = b.freeze().expect("freeze");

        let q = format!("{PREFIX} SELECT ?n WHERE {{ BIND(g:listLength(<http://ex/l0>) AS ?n) }}");
        let err = eval_err(&ds, &q);
        assert!(
            matches!(&err, EvalError::NativeDiagnostic(diagnostic) if diagnostic.kind() == crate::error::NativeDiagnosticKind::Data),
            "got {err:?}"
        );
        assert!(err.to_string().contains("rdf:rest chain returns"));
    }

    #[test]
    fn list_membership_is_term_exact_not_value_space() {
        // The single member is "1"^^xsd:integer. listIndexOf/listContains match by
        // structural (lexical + datatype) term identity — the SAME equality the
        // logic oracle uses (Prolog unification), which is the parity contract — and
        // NOT SPARQL value-space: "1"^^xsd:decimal is numerically equal but a
        // distinct term, so it does not match.
        use purrdf_core::RdfLiteral;
        use purrdf_xsd::datatype::XSD_DECIMAL;
        use purrdf_xsd::datatype::XSD_INTEGER;
        let mut b = RdfDatasetBuilder::new();
        let first = b.intern_iri(super::RDF_FIRST);
        let rest = b.intern_iri(super::RDF_REST);
        let nil = b.intern_iri(super::RDF_NIL);
        let l0 = b.intern_iri("http://ex/l0");
        let one_int = b.intern_literal(RdfLiteral::typed("1", XSD_INTEGER));
        b.push_quad(l0, first, one_int, None);
        b.push_quad(l0, rest, nil, None);
        let ds = b.freeze().expect("freeze");

        // The exact term is a member at index 0.
        let q_exact = format!(
            "{PREFIX} SELECT ?b ?n WHERE {{ \
             BIND(g:listContains(<http://ex/l0>, \"1\"^^<{XSD_INTEGER}>) AS ?b) \
             BIND(g:listIndexOf(<http://ex/l0>, \"1\"^^<{XSD_INTEGER}>) AS ?n) }}"
        );
        assert_eq!(
            rows(&ds, &q_exact),
            vec![vec!["true".to_owned(), "0".to_owned()]]
        );

        // A value-equal but structurally distinct term (different datatype) does not
        // match: listContains is false, listIndexOf is unbound.
        let q_distinct = format!(
            "{PREFIX} SELECT ?b ?n WHERE {{ \
             BIND(g:listContains(<http://ex/l0>, \"1\"^^<{XSD_DECIMAL}>) AS ?b) \
             BIND(g:listIndexOf(<http://ex/l0>, \"1\"^^<{XSD_DECIMAL}>) AS ?n) }}"
        );
        assert_eq!(
            rows(&ds, &q_distinct),
            vec![vec!["false".to_owned(), "UNBOUND".to_owned()]]
        );
    }

    #[test]
    fn torn_list_missing_rest_is_a_hard_data_error() {
        // l0 -> first x, rest l1 ; l1 -> first y  (no rdf:rest on the 2nd cell).
        let mut b = RdfDatasetBuilder::new();
        let first = b.intern_iri(super::RDF_FIRST);
        let rest = b.intern_iri(super::RDF_REST);
        let nil = b.intern_iri(super::RDF_NIL);
        let l0 = b.intern_iri("http://ex/l0");
        let l1 = b.intern_iri("http://ex/l1");
        let x = b.intern_iri("http://ex/x");
        let y = b.intern_iri("http://ex/y");
        let z = b.intern_iri("http://ex/z");
        b.push_quad(l0, first, x, None);
        b.push_quad(l0, rest, l1, None);
        b.push_quad(l1, first, y, None);
        // l1 has no rdf:rest — a torn list. Intern rdf:nil elsewhere so the walk starts.
        b.push_quad(z, rest, nil, None);
        let ds = b.freeze().expect("freeze");

        let q = format!("{PREFIX} SELECT ?n WHERE {{ BIND(g:listLength(<http://ex/l0>) AS ?n) }}");
        let err = eval_err(&ds, &q);
        assert!(
            matches!(&err, EvalError::NativeDiagnostic(diagnostic) if diagnostic.kind() == crate::error::NativeDiagnosticKind::Data),
            "got {err:?}"
        );
        assert!(err.to_string().contains("has no rdf:rest"), "got {err}");
    }

    #[test]
    fn torn_list_interior_missing_first_is_a_hard_data_error() {
        // l0 -> first x, rest l1 ; l1 -> rest nil  (no rdf:first on the interior cell).
        let mut b = RdfDatasetBuilder::new();
        let first = b.intern_iri(super::RDF_FIRST);
        let rest = b.intern_iri(super::RDF_REST);
        let nil = b.intern_iri(super::RDF_NIL);
        let l0 = b.intern_iri("http://ex/l0");
        let l1 = b.intern_iri("http://ex/l1");
        let x = b.intern_iri("http://ex/x");
        b.push_quad(l0, first, x, None);
        b.push_quad(l0, rest, l1, None);
        // l1 has rdf:rest but no rdf:first — torn, and `members` is already non-empty
        // (so this is a torn interior cell, not a non-list head).
        b.push_quad(l1, rest, nil, None);
        let ds = b.freeze().expect("freeze");

        let q = format!("{PREFIX} SELECT ?n WHERE {{ BIND(g:listLength(<http://ex/l0>) AS ?n) }}");
        let err = eval_err(&ds, &q);
        assert!(
            matches!(&err, EvalError::NativeDiagnostic(diagnostic) if diagnostic.kind() == crate::error::NativeDiagnosticKind::Data),
            "got {err:?}"
        );
        assert!(err.to_string().contains("has no rdf:first"), "got {err}");
    }

    #[test]
    fn multi_edge_dataset_cell_is_a_hard_data_error() {
        // A cell carrying two rdf:first quads is ambiguous — hard-fail rather than
        // pick an iteration-order-dependent branch.
        let mut b = RdfDatasetBuilder::new();
        let first = b.intern_iri(super::RDF_FIRST);
        let rest = b.intern_iri(super::RDF_REST);
        let nil = b.intern_iri(super::RDF_NIL);
        let l0 = b.intern_iri("http://ex/l0");
        let x = b.intern_iri("http://ex/x");
        let y = b.intern_iri("http://ex/y");
        b.push_quad(l0, first, x, None);
        b.push_quad(l0, first, y, None); // a second rdf:first — malformed
        b.push_quad(l0, rest, nil, None);
        let ds = b.freeze().expect("freeze");

        let q = format!("{PREFIX} SELECT ?n WHERE {{ BIND(g:listLength(<http://ex/l0>) AS ?n) }}");
        let err = eval_err(&ds, &q);
        assert!(
            matches!(&err, EvalError::NativeDiagnostic(diagnostic) if diagnostic.kind() == crate::error::NativeDiagnosticKind::Data),
            "got {err:?}"
        );
        assert!(
            err.to_string().contains("more than one rdf:first"),
            "got {err}"
        );
    }

    #[test]
    fn constructed_list_is_readable_within_the_same_query() {
        // A list minted by listSlice/listConcat must be walkable by another list
        // function in the SAME query: its cells live only in the per-query buffer, so
        // walk() consults the constructed buffer as well as the dataset.
        let ds = list_ds();

        // listLength(listSlice((x y z), 1, 3)) = |(y z)| = 2
        let q_len = format!(
            "{PREFIX} SELECT ?n WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listLength(g:listSlice(?l, 1, 3)) AS ?n) }}"
        );
        assert_eq!(rows(&ds, &q_len), vec![vec!["2".to_owned()]]);

        // listGet(listConcat(L, L), 3) = first member of the second copy = x
        let q_get = format!(
            "{PREFIX} SELECT ?x WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listGet(g:listConcat(?l, ?l), 3) AS ?x) }}"
        );
        assert_eq!(rows(&ds, &q_get), vec![vec!["<http://ex/x>".to_owned()]]);
    }

    // ── constructing functions: listSlice / listConcat ───────────────────────

    use purrdf_core::{SparqlRequest, SparqlResult, TermRef};

    use crate::engine::NativeSparqlEngine;

    const RDF_NIL_STR: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>";

    #[test]
    fn list_cell_reservations_obey_scratch_bytes_and_discard_incomplete_cells() {
        use crate::{GovernedOutcome, GovernorState, QueryGovernors};
        use purrdf_core::{ResourceDimension, TrippedGovernor};

        let ds = list_ds();
        let env = crate::extension_env::ExtensionEnv::over_options(ext_options())
            .expect("declared list namespace");
        let engine = NativeSparqlEngine::new();
        let query = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listSlice(?l, 0, 2) AS ?s) }}"
        );
        let run = |governors: &QueryGovernors| {
            engine
                .query_governed(
                    &ds,
                    SparqlRequest {
                        query: &query,
                        base_iri: None,
                        substitutions: &[],
                    },
                    QueryOptions {
                        env: &env,
                        ..QueryOptions::EMPTY
                    },
                    governors,
                )
                .expect("list allocation exhaustion is a typed outcome")
        };
        let measured = run(&QueryGovernors::METERED);
        // Two integer arguments, two retained lc1/lc2 labels, one owned head.
        let argument_bytes = 2 * (1 + purrdf_xsd::datatype::XSD_INTEGER.len() as u64 + 32);
        let required = argument_bytes + 3 * (3 + 32);
        assert_eq!(
            measured
                .evidence()
                .consumed
                .get(ResourceDimension::ScratchBytes),
            required
        );
        let stopped = run(&QueryGovernors::UNBOUNDED.with_max_scratch_bytes(argument_bytes + 69));
        assert_eq!(
            stopped.tripped(),
            Some(TrippedGovernor::Budget {
                dimension: ResourceDimension::ScratchBytes,
                limit: argument_bytes + 69,
                consumed: argument_bytes + 70,
            })
        );
        for limit in [required, required + 1] {
            let GovernedOutcome::Complete {
                result: SparqlResult::Solutions { rows, aux, .. },
                ..
            } = run(&QueryGovernors::UNBOUNDED.with_max_scratch_bytes(limit))
            else {
                panic!("inclusive scratch budget admits a complete list");
            };
            assert_eq!(rows.len(), 1);
            assert_eq!(aux.quad_count(), 4);
            assert_eq!(
                members_of(&aux, &head_str(&rows)),
                vec!["<http://ex/x>", "<http://ex/y>"]
            );
        }

        // Exercise the in-flight buffer independently of egress withholding:
        // the first cell emits rdf:first before the second cell's mint trips.
        let state = Arc::new(GovernorState::new(
            &QueryGovernors::UNBOUNDED.with_max_scratch_bytes(69),
        ));
        let mut ctx = EvalCtx::new(ds.as_ref()).with_governors(Arc::clone(&state));
        let members = vec![
            ctx.growth.iri("http://ex/x").unwrap(),
            ctx.growth.iri("http://ex/y").unwrap(),
        ];
        let result =
            super::materialize_list(&mut ctx, members).expect("a mint trip is not an EvalError");
        assert!(result.is_none());
        assert!(
            ctx.constructed.is_empty(),
            "no torn list survives the failed constructor"
        );
        assert_eq!(ctx.expression_barrier.observed(), state.tripped());
    }

    /// Resolve a dataset to sorted `(s, p, o)` string triples.
    fn triples(ds: &RdfDataset) -> Vec<(String, String, String)> {
        let term = |id| match ds.resolve(id) {
            TermRef::Iri(i) => format!("<{i}>"),
            TermRef::Blank { label, .. } => format!("_:{label}"),
            TermRef::Literal { lexical, .. } => lexical.to_owned(),
            TermRef::Triple { .. } => "<<triple>>".to_owned(),
        };
        let mut out: Vec<_> = ds
            .quads()
            .map(|q| (term(q.s), term(q.p), term(q.o)))
            .collect();
        out.sort();
        out
    }

    /// Walk a constructed `rdf:List` from `head`, returning member object strings.
    fn members_of(ds: &RdfDataset, head: &str) -> Vec<String> {
        let first = format!("<{}>", super::RDF_FIRST);
        let rest = format!("<{}>", super::RDF_REST);
        let ts = triples(ds);
        let mut members = Vec::new();
        let mut cur = head.to_owned();
        while cur != RDF_NIL_STR {
            let f = ts
                .iter()
                .find(|(s, p, _)| s == &cur && p == &first)
                .map(|(_, _, o)| o.clone());
            let r = ts
                .iter()
                .find(|(s, p, _)| s == &cur && p == &rest)
                .map(|(_, _, o)| o.clone());
            match (f, r) {
                (Some(f), Some(r)) => {
                    members.push(f);
                    cur = r;
                }
                _ => break,
            }
        }
        members
    }

    /// Run a SELECT/ASK and return its rows plus the auxiliary constructed graph.
    fn run_constructed(
        ds: &Arc<RdfDataset>,
        query: &str,
    ) -> (Vec<Vec<Option<TermValue>>>, DatasetHandle) {
        let env = crate::extension_env::ExtensionEnv::over_options(ext_options())
            .expect("environment over declared parser options");
        let engine = NativeSparqlEngine::new();
        let res = engine
            .query_with_options_view(
                ds,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    env: &env,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("query");
        match res {
            SparqlResult::Solutions { rows, aux, .. } => (rows, aux),
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// Run a CONSTRUCT and return its output graph.
    fn run_graph(ds: &Arc<RdfDataset>, query: &str) -> DatasetHandle {
        let env = crate::extension_env::ExtensionEnv::over_options(ext_options())
            .expect("environment over declared parser options");
        let engine = NativeSparqlEngine::new();
        match engine
            .query_with_options_view(
                ds,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    env: &env,
                    ..QueryOptions::EMPTY
                },
            )
            .expect("query")
        {
            SparqlResult::Graph(g) => g,
            other => panic!("expected a graph, got {other:?}"),
        }
    }

    /// The single SELECT head cell as a comparable string (`<iri>` or `_:label`).
    fn head_str(rows: &[Vec<Option<TermValue>>]) -> String {
        match &rows[0][0] {
            Some(TermValue::Iri(i)) => format!("<{i}>"),
            Some(TermValue::Blank { label, .. }) => format!("_:{label}"),
            other => panic!("expected a list head term, got {other:?}"),
        }
    }

    #[test]
    fn list_slice_surfaces_subrange_in_aux_graph() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listSlice(?l, 1, 3) AS ?s) }}"
        );
        let (rows, aux) = run_constructed(&ds, &q);
        let head = head_str(&rows);
        assert!(head.starts_with("_:"), "head must be a fresh blank: {head}");
        assert_eq!(
            members_of(&aux, &head),
            vec!["<http://ex/y>".to_owned(), "<http://ex/z>".to_owned()]
        );
        // A 2-member list is exactly 4 cell quads.
        assert_eq!(aux.quad_count(), 4);
    }

    #[test]
    fn list_slice_empty_range_is_nil() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listSlice(?l, 2, 2) AS ?s) }}"
        );
        let (rows, aux) = run_constructed(&ds, &q);
        assert_eq!(head_str(&rows), RDF_NIL_STR);
        assert_eq!(aux.quad_count(), 0);
    }

    #[test]
    fn list_slice_clamps_out_of_bounds_and_inverted_ranges() {
        let ds = list_ds();
        // end past the list end → clamps to the full tail [1, len).
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listSlice(?l, 1, 99) AS ?s) }}"
        );
        let (rows, aux) = run_constructed(&ds, &q);
        assert_eq!(
            members_of(&aux, &head_str(&rows)),
            vec!["<http://ex/y>".to_owned(), "<http://ex/z>".to_owned()]
        );
        // inverted range (start > end) → empty.
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listSlice(?l, 2, 1) AS ?s) }}"
        );
        let (rows, _) = run_constructed(&ds, &q);
        assert_eq!(head_str(&rows), RDF_NIL_STR);
    }

    #[test]
    fn list_concat_appends_members() {
        let ds = list_ds();
        // concat the list with itself → [x, y, z, x, y, z].
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listConcat(?l, ?l) AS ?s) }}"
        );
        let (rows, aux) = run_constructed(&ds, &q);
        assert_eq!(
            members_of(&aux, &head_str(&rows)),
            vec![
                "<http://ex/x>".to_owned(),
                "<http://ex/y>".to_owned(),
                "<http://ex/z>".to_owned(),
                "<http://ex/x>".to_owned(),
                "<http://ex/y>".to_owned(),
                "<http://ex/z>".to_owned(),
            ]
        );
    }

    #[test]
    fn list_concat_with_nil_is_identity_and_nil_nil_is_nil() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listConcat(?l, <{}>) AS ?s) }}",
            super::RDF_NIL
        );
        let (rows, aux) = run_constructed(&ds, &q);
        assert_eq!(
            members_of(&aux, &head_str(&rows)),
            vec![
                "<http://ex/x>".to_owned(),
                "<http://ex/y>".to_owned(),
                "<http://ex/z>".to_owned(),
            ]
        );
        // nil ++ nil → nil (no cells).
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ BIND(g:listConcat(<{nil}>, <{nil}>) AS ?s) }}",
            nil = super::RDF_NIL
        );
        let (rows, aux) = run_constructed(&ds, &q);
        assert_eq!(head_str(&rows), RDF_NIL_STR);
        assert_eq!(aux.quad_count(), 0);
    }

    #[test]
    fn list_slice_materializes_into_construct_output() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} CONSTRUCT {{ <http://ex/out> <http://ex/has> ?s }} \
             WHERE {{ ?q <http://ex/list> ?l . BIND(g:listSlice(?l, 0, 2) AS ?s) }}"
        );
        let graph = run_graph(&ds, &q);
        // The head is the object of ex:out ex:has — find it, then walk the cells.
        let ts = triples(&graph);
        let head = ts
            .iter()
            .find(|(s, p, _)| s == "<http://ex/out>" && p == "<http://ex/has>")
            .map(|(_, _, o)| o.clone())
            .expect("the binding triple is present");
        assert_eq!(
            members_of(&graph, &head),
            vec!["<http://ex/x>".to_owned(), "<http://ex/y>".to_owned()]
        );
        // binding triple (1) + two cells (4) = 5 quads.
        assert_eq!(graph.quad_count(), 5);
    }

    #[test]
    fn pruned_row_does_not_leak_constructed_cells_into_aux() {
        // A list is minted on a row that FILTER then removes. Its cells were buffered,
        // but no row survives, so they must NOT surface in the SELECT aux graph (the
        // row↔aux contract — no orphaned cells).
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listSlice(?l, 0, 2) AS ?s) FILTER(1 > 2) }}"
        );
        let (rows, aux) = run_constructed(&ds, &q);
        assert!(rows.is_empty(), "all rows are filtered out, got {rows:?}");
        assert_eq!(
            aux.quad_count(),
            0,
            "orphaned cells leaked: {:?}",
            triples(&aux)
        );
    }

    #[test]
    fn pruned_row_does_not_leak_constructed_cells_into_construct() {
        // Same contract for CONSTRUCT: a list minted on a filtered row contributes no
        // orphaned cells to the output graph.
        let ds = list_ds();
        let q = format!(
            "{PREFIX} CONSTRUCT {{ <http://ex/out> <http://ex/has> ?s }} \
             WHERE {{ ?q <http://ex/list> ?l . BIND(g:listSlice(?l, 0, 2) AS ?s) FILTER(1 > 2) }}"
        );
        let graph = run_graph(&ds, &q);
        assert_eq!(
            graph.quad_count(),
            0,
            "orphaned cells leaked: {:?}",
            triples(&graph)
        );
    }

    // ── both egress doors: owned `SparqlResult` vs borrowed `InternedOutcome` ──

    use crate::engine::QueryOptions;
    use crate::interned::{InternedOutcome, InternedRequest};

    /// [`run_constructed`] through the INTERNED egress door: the first projected
    /// column of every surviving row, stringified the way [`head_str`] spells the
    /// owned door's, plus the auxiliary constructed graph the borrowed result
    /// exposes.
    fn run_constructed_interned(
        ds: &Arc<RdfDataset>,
        query: &str,
    ) -> (Vec<String>, crate::RetainedGraph) {
        let env = crate::extension_env::ExtensionEnv::over_options(ext_options())
            .expect("environment over declared parser options");
        let engine = NativeSparqlEngine::new();
        engine
            .query_interned_view(
                ds.as_ref(),
                InternedRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions {
                    env: &env,
                    ..QueryOptions::EMPTY
                },
                |outcome| match outcome {
                    InternedOutcome::Solutions(solutions) => {
                        let heads = solutions
                            .rows()
                            .iter()
                            .map(|row| match solutions.cell(row, 0) {
                                Some(TermValue::Iri(i)) => format!("<{i}>"),
                                Some(TermValue::Blank { label, .. }) => format!("_:{label}"),
                                other => panic!("expected a list head term, got {other:?}"),
                            })
                            .collect();
                        (heads, solutions.constructed_dataset())
                    }
                    InternedOutcome::Boolean(_) | InternedOutcome::Graph(_) => {
                        panic!("expected solutions")
                    }
                },
            )
            .expect("interned query")
    }

    #[test]
    fn both_doors_agree_on_the_constructed_graph_of_a_list_slice() {
        // The head `listSlice` binds names cells that exist nowhere in the queried
        // dataset — this execution minted them. The owned door carries them out as
        // `aux`. The borrowed door must carry the SAME quads, or a caller that reads
        // the head and no more holds an identifier pointing into a graph it cannot
        // see.
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listSlice(?l, 1, 3) AS ?s) }}"
        );

        let (owned_rows, owned_aux) = run_constructed(&ds, &q);
        let (interned_heads, interned_aux) = run_constructed_interned(&ds, &q);

        assert_eq!(
            interned_heads,
            vec![head_str(&owned_rows)],
            "the two doors disagree about the head term"
        );
        assert_eq!(
            triples(&interned_aux),
            triples(&owned_aux),
            "the two doors disagree about the constructed quads"
        );
        // Not vacuous: the query really does construct, and the borrowed door's graph
        // is the walkable list, not merely a graph of equal size.
        assert_eq!(
            members_of(&interned_aux, &interned_heads[0]),
            vec!["<http://ex/y>".to_owned(), "<http://ex/z>".to_owned()]
        );
        assert_eq!(interned_aux.quad_count(), 4);
    }

    #[test]
    fn both_doors_agree_on_the_constructed_graph_of_a_list_concat() {
        // `listConcat` over a freshly sliced list: the intermediate slice's cells are
        // buffered but unreachable from the surviving row, so BOTH doors must prune
        // them — the borrowed door runs the same reachability walk, not a laxer one.
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listConcat(g:listSlice(?l, 1, 3), ?l) AS ?s) }}"
        );

        let (owned_rows, owned_aux) = run_constructed(&ds, &q);
        let (interned_heads, interned_aux) = run_constructed_interned(&ds, &q);

        assert_eq!(interned_heads, vec![head_str(&owned_rows)]);
        assert_eq!(
            triples(&interned_aux),
            triples(&owned_aux),
            "the two doors disagree about the constructed quads"
        );
        assert_eq!(
            members_of(&interned_aux, &interned_heads[0]),
            vec![
                "<http://ex/y>".to_owned(),
                "<http://ex/z>".to_owned(),
                "<http://ex/x>".to_owned(),
                "<http://ex/y>".to_owned(),
                "<http://ex/z>".to_owned(),
            ]
        );
    }

    #[test]
    fn both_doors_agree_that_a_pruned_row_constructs_nothing() {
        // The row↔aux contract, asserted on the borrowed door: a list minted on a row
        // that FILTER then removes leaves no orphaned cells in EITHER graph.
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listSlice(?l, 0, 2) AS ?s) FILTER(1 > 2) }}"
        );

        let (owned_rows, owned_aux) = run_constructed(&ds, &q);
        let (interned_heads, interned_aux) = run_constructed_interned(&ds, &q);

        assert_eq!(
            owned_rows.len(),
            0,
            "all rows are filtered out: {owned_rows:?}"
        );
        assert_eq!(
            interned_heads.len(),
            0,
            "all rows are filtered out: {interned_heads:?}"
        );
        assert_eq!(owned_aux.quad_count(), 0);
        assert_eq!(
            interned_aux.quad_count(),
            0,
            "orphaned cells leaked through the borrowed door: {:?}",
            triples(&interned_aux)
        );
    }

    #[test]
    fn a_query_that_constructs_nothing_has_an_empty_interned_constructed_graph() {
        // The negative case: no list constructor anywhere in the query. The borrowed
        // door must answer with an empty graph — not an error, and not an absent one —
        // exactly as the owned door does.
        let ds = list_ds();
        let q = format!("{PREFIX} SELECT ?l WHERE {{ ?q <http://ex/list> ?l }}");

        let (_, owned_aux) = run_constructed(&ds, &q);
        let (interned_heads, interned_aux) = run_constructed_interned(&ds, &q);

        assert_eq!(interned_heads, vec!["<http://ex/l0>".to_owned()]);
        assert_eq!(owned_aux.quad_count(), 0);
        assert_eq!(interned_aux.quad_count(), 0);
        assert_eq!(triples(&interned_aux), triples(&owned_aux));
    }
}
