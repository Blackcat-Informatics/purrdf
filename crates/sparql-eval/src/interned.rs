// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The **interned** query egress: a borrowed view of a result, visited while the
//! evaluation context is still alive.
//!
//! [`SparqlResult`](purrdf_core::SparqlResult) is the dataset-independent egress
//! model, and building one ENDS the interned id space: every projected cell is
//! turned into an owned [`TermValue`], one `Vec` per row, one `String` per
//! variable name, plus the auxiliary graph of constructed list cells. That is the
//! right shape for a caller who keeps the rows after the engine has gone.
//!
//! It is the wrong shape for a caller that reads one or two columns and drops the
//! rest — SHACL runs one query per focus node and reads `?this` / `?path` /
//! `?value` out of it, so the row `Vec`s and every unread cell are built and
//! discarded. This module is the other door: the evaluation hands a
//! [`InternedOutcome`] to a visitor **inside** the evaluation, so the visitor sees
//! the rows as the evaluator holds them — [`SolutionTerm`] ids, one shared
//! [`VarSchema`](crate::solution::VarSchema) — and pays for exactly the cells it
//! asks for.
//!
//! # Why a visitor and not a returned value
//!
//! A [`SolutionTerm::Computed`] cell names a term minted into this execution's
//! scratch arena, which dies with the [`EvalCtx`]. Interned rows therefore cannot
//! outlive the evaluation, and the only sound shape for "borrow the rows" is a
//! callback that runs before the context is dropped. The visitor's RETURN value
//! is unconstrained, so a caller projects whatever it actually wanted.
//!
//! # This is additive
//!
//! Nothing here changes [`SparqlResult`](purrdf_core::SparqlResult) or any
//! existing entry point. A generic
//! [`SparqlEngine`](purrdf_core::SparqlEngine) consumer keeps the owned egress; a
//! caller that wants the interned one asks for it by name.

use std::sync::Arc;

use purrdf_core::{DatasetView, RdfDataset, TermValue};
use purrdf_sparql_algebra::Variable;

use crate::eval::EvalCtx;
use crate::governed::{BudgetExhausted, RelationIdentity};
use crate::scratch::SolutionTerm;
use crate::solution::{Solution, SolutionSeq};

/// One pre-binding on an interned entry point: a **borrowed** variable name and
/// the term it binds.
///
/// [`SparqlRequest::substitutions`](purrdf_core::SparqlRequest) spells the same
/// thing as `&[(String, TermValue)]`, which charges a caller one freshly allocated
/// `String` per pre-bound variable per REQUEST. For a generic consumer, which
/// issues one query and names its variables once, that is nothing. For SHACL it is
/// the wrong shape for the same reason the owned egress was: a validation runs one
/// query per focus node and pre-binds the same three or four variables in every
/// one of them — `this`, `value`, `shapesGraph`, `currentShape`, a component's
/// parameters. Those names are text out of the SHAPE. They do not vary with the
/// focus node, they are already allocated inside the loaded shapes graph, and
/// re-allocating them per focus node was paying for a copy of a constant.
///
/// So the name borrows. The VALUE stays owned: a focus node's term is genuinely
/// per-focus-node data, and the pre-binding rewrite needs it as an owned
/// [`GroundTerm`](purrdf_sparql_algebra::GroundTerm) in the algebra regardless.
///
/// # This is additive
///
/// [`SparqlRequest`](purrdf_core::SparqlRequest) is untouched and remains the
/// request type for every generic
/// [`SparqlEngine`](purrdf_core::SparqlEngine) consumer, owned substitution list
/// included. This is a different consumer, not a mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prebinding<'a> {
    /// The variable to pre-bind, spelled without the `?`/`$` sigil.
    pub variable: &'a str,
    /// The term to bind it to.
    pub value: TermValue,
}

/// The request an interned entry point takes.
///
/// [`SparqlRequest`](purrdf_core::SparqlRequest)'s shape, differing in exactly one
/// field: the pre-bindings are [`Prebinding`]s, whose variable names borrow. See
/// that type for why.
/// Every field is a borrow, so this is `Copy`: passing one costs no more than
/// passing the query string alone.
#[derive(Clone, Copy, Debug)]
pub struct InternedRequest<'a> {
    /// The query text.
    pub query: &'a str,
    /// The base IRI relative references in the query resolve against.
    pub base_iri: Option<&'a str>,
    /// The variables to pre-bind before evaluating.
    pub substitutions: &'a [Prebinding<'a>],
}

/// A borrowed, still-interned SELECT result.
///
/// The rows are the evaluator's own [`Solution`] rows over the query's shared
/// [`VarSchema`](crate::solution::VarSchema); nothing has been copied out of the
/// id space. [`Self::value_of`] is the one door that leaves it, and it converts a
/// SINGLE cell.
#[derive(Debug)]
pub struct InternedSolutions<'a, 'd, D: DatasetView + Sync> {
    /// The solution bag as the evaluator produced it.
    seq: &'a SolutionSeq<D::Id>,
    /// The live evaluation context the ids resolve against.
    ctx: &'a EvalCtx<'d, D>,
}

impl<'a, 'd, D: DatasetView + Sync> InternedSolutions<'a, 'd, D> {
    /// Wrap `seq` and `ctx`. Crate-internal: the pair is only ever valid inside
    /// the evaluation that produced it, which is what the visitor seam enforces.
    pub(crate) fn new(seq: &'a SolutionSeq<D::Id>, ctx: &'a EvalCtx<'d, D>) -> Self {
        Self { seq, ctx }
    }

    /// The projected variables, in result-column order.
    #[must_use]
    pub fn variables(&self) -> &'a [Variable] {
        self.seq.schema.vars()
    }

    /// The column ordinal of the variable spelled `name` (without the `?`/`$`
    /// sigil), if the result projects it.
    #[must_use]
    pub fn column(&self, name: &str) -> Option<usize> {
        self.seq
            .schema
            .vars()
            .iter()
            .position(|var| var.as_str() == name)
    }

    /// The solution rows, in solution order, as a bag (duplicates preserved).
    #[must_use]
    pub fn rows(&self) -> &'a [Solution<D::Id>] {
        &self.seq.rows
    }

    /// The number of solutions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.seq.rows.len()
    }

    /// Whether the result has no solutions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.seq.rows.is_empty()
    }

    /// The owned [`TermValue`] of one interned cell.
    ///
    /// This is the only conversion out of the id space, and it is per CELL: a
    /// caller that reads one column of a wide result pays for that column alone.
    #[must_use]
    pub fn value_of(&self, term: SolutionTerm<D::Id>) -> TermValue {
        self.ctx.scratch.value_of(self.ctx.dataset, term)
    }

    /// The owned [`TermValue`] of `row`'s `column`-th cell, when the row binds it.
    ///
    /// `None` covers both "not a column of this result" and "a column this row
    /// leaves unbound", which is what every caller of a projected-variable lookup
    /// already treats identically.
    #[must_use]
    pub fn cell(&self, row: &Solution<D::Id>, column: usize) -> Option<TermValue> {
        row.get(column)
            .copied()
            .flatten()
            .map(|term| self.value_of(term))
    }
}

/// A borrowed result of any query form, handed to an interned visitor.
///
/// The exact three-way shape of [`SparqlResult`](purrdf_core::SparqlResult), so a
/// caller that already matches on query form matches on the same three arms.
#[derive(Debug)]
pub enum InternedOutcome<'a, 'd, D: DatasetView + Sync> {
    /// A SELECT result, still interned.
    Solutions(InternedSolutions<'a, 'd, D>),
    /// An ASK result. Booleans carry no rows, so this arm is already owned —
    /// nothing was materialized to produce it.
    Boolean(bool),
    /// A CONSTRUCT/DESCRIBE result. The frozen graph is already built and shared
    /// by `Arc`; a visitor that keeps it clones the handle, not the data.
    Graph(&'a Arc<RdfDataset>),
}

/// The outcome of a governed interned execution.
///
/// The interned twin of [`GovernedOutcome`](crate::GovernedOutcome), differing in
/// exactly one way: the complete arm carries what the VISITOR returned rather than
/// a materialized [`SparqlResult`](purrdf_core::SparqlResult). The exhausted arm
/// is unchanged, partial answers included — a trip is a cold, terminal path, and
/// the certified partial rows are the actionable half of the report.
#[derive(Debug)]
pub enum InternedGoverned<R> {
    /// The execution completed within budget; `value` is the visitor's return.
    Complete {
        /// What the visitor projected out of the interned result.
        value: R,
        /// This execution's resource receipt.
        evidence: purrdf_core::GovernorEvidence,
        /// The property-function registry identity the plan was admitted under.
        relations: RelationIdentity,
    },
    /// A governor stopped the execution; the visitor never ran.
    ///
    /// Boxed because the payload carries both receipts and the certified partial
    /// answers, and every caller of the complete path would otherwise pay that
    /// width on the stack for an arm it does not take.
    BudgetExhausted(Box<BudgetExhausted>),
}
