// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! Deferred, not dropped: the auxiliary graph is the one item on that list a
//! result is not complete without, since a constructed list head names cells that
//! exist nowhere in the queried dataset. It moves to
//! [`InternedSolutions::constructed_dataset`], where a visitor asks for it and a
//! query that constructed nothing builds nothing.
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
    pub fn rows(&self) -> &'a [crate::solution::RetainedRow<D::Id>] {
        &self.seq.rows
    }

    /// Explicit longest-first non-overlap over this actual SELECT result.
    /// Domain columns are caller-selected (normally document and language).
    /// All rows attached to a selected identical span survive in original order,
    /// with their original dictionary/evidence/search owners. This is a logical
    /// consumer, independent of whether a physical dictionary shape was used.
    ///
    /// # Errors
    /// Refuses missing columns, unbound/invalid token coordinates, source reads,
    /// or before-growth storage failures. Positions are unsigned token ordinals.
    pub fn longest_first_spans(
        &self,
        domain: &[&str],
        start: &str,
        end: &str,
    ) -> Result<crate::shapes::spans::SpanSelection<'a, D::Id>, crate::EvalError> {
        let workspace = &self.ctx.growth;
        let invalid = || {
            crate::NativeDiagnostic::error(
                crate::NativeDiagnosticKind::Data,
                "span selection requires projected domain columns and bound unsigned integer token coordinates",
                workspace,
            )
        };
        let start = self.column(start).ok_or_else(invalid)?;
        let end = self.column(end).ok_or_else(invalid)?;
        let mut columns = crate::AdmittedVec::new(workspace);
        for name in domain {
            columns.push(self.column(name).ok_or_else(invalid)?)?;
        }
        let mut domains = crate::AdmittedMap::default();
        let mut spans = crate::AdmittedVec::new(workspace);
        for row in &self.seq.rows {
            let key = crate::solution::RetainedRow::from_cells(
                columns.len(),
                columns.iter().map(|&column| row[column]),
                workspace,
            )?;
            let next = domains.len();
            let domain = if let Some(&ordinal) = domains.get(&key) {
                ordinal
            } else {
                domains.insert_admitted(key, next, workspace)?;
                next
            };
            let coordinate = |column: usize| -> Result<u64, crate::EvalError> {
                let cell = row[column].ok_or_else(invalid)?;
                let value = crate::expr::owned_value_of(self.ctx, cell)?;
                let TermValue::Literal {
                    lexical_form,
                    datatype,
                    language: None,
                    direction: None,
                } = &*value
                else {
                    return Err(invalid());
                };
                let datatype = purrdf_xsd::XsdDatatype::from_iri(datatype)
                    .filter(|datatype| datatype.is_integer_family())
                    .ok_or_else(invalid)?;
                let value = crate::parsed_value::ParsedValue::parse(
                    lexical_form,
                    datatype,
                    false,
                    workspace,
                )?
                .ok_or_else(invalid)?;
                let integer = match &*value {
                    purrdf_xsd::XsdValue::Integer { value, .. } => value.as_i128(),
                    _ => None,
                };
                integer
                    .and_then(|value| u64::try_from(value).ok())
                    .ok_or_else(invalid)
            };
            spans.push(crate::shapes::spans::SpanGeometry {
                domain,
                start: coordinate(start)?,
                end: coordinate(end)?,
            })?;
        }
        let ordinals = crate::shapes::spans::longest_first_indices(&spans, workspace)?;
        Ok(crate::shapes::spans::SpanSelection {
            rows: &self.seq.rows,
            ordinals,
        })
    }

    /// The original whole-search declaration, including a reached producer
    /// that yielded no row. Reading this does not replay the query or derive
    /// completeness from the returned row count.
    #[must_use]
    pub fn search_evidence(&self) -> &'a crate::RowEvidence {
        self.seq.rows.search_evidence()
    }

    /// Retain the existential answer under this query's original evidence owner.
    /// An empty answer uses the source's explicit closure declaration; its row
    /// count alone never establishes absence.
    ///
    /// # Errors
    /// Preserves the source, reason-retention and native admission refusal.
    pub fn existential_evidence(&self) -> Result<crate::RowEvidence, crate::EvalError> {
        self.ctx.existential_evidence(
            self.seq
                .rows
                .iter()
                .map(crate::solution::RetainedRow::evidence),
            self.search_evidence(),
        )
    }

    /// Borrow established rows in their original order without constructing a
    /// second result buffer or resolving any cell out of its native id space.
    pub fn established_rows(
        &self,
    ) -> impl Iterator<Item = &'a crate::solution::RetainedRow<D::Id>> {
        self.rows()
            .iter()
            .filter(|row| !row.evidence().is_undecided())
    }

    /// Borrow the separate undecided partition, retaining original bindings,
    /// evidence owners and order. Standard queries have no declared gaps.
    pub fn undecided_rows(&self) -> impl Iterator<Item = &'a crate::solution::RetainedRow<D::Id>> {
        self.rows()
            .iter()
            .filter(|row| row.evidence().is_undecided())
    }

    /// Read RDF 1.2 annotations from this actual answer's original evidence and
    /// reason owners. The caller supplies its row triple terms and vocabulary;
    /// rendering preserves the complete original solution order. Below this
    /// explicit read no ledger cursors or RDF terms are constructed.
    ///
    /// # Errors
    /// Refuses absent evidence declarations, invalid row/content nodes, foreign
    /// evidence owners or original native physical admission failures.
    pub fn with_evidence_ledger<N: purrdf_core::evidence::LedgerNodes, R>(
        &self,
        terms: &[crate::evidence_exec::LedgerRowTerms<'_>],
        nodes: &N,
        vocabulary: &purrdf_core::evidence::LedgerVocabulary<'_>,
        visit: impl FnOnce(&crate::evidence_exec::EvidenceLedger<'_, N>) -> R,
    ) -> Result<R, crate::EvalError> {
        let input = self.evidence_input()?;
        input.with_ledger(self.seq, terms, nodes, vocabulary, &self.ctx.growth, visit)
    }

    /// Merge accepted undecided rows into the established stream in original
    /// answer order, retaining the exact native cells, ranks, scores and reasons.
    /// Each yielded core AcceptedRow exposes its acceptance annotation and the
    /// identity law binding that override to the caller's original row identity.
    ///
    /// # Errors
    /// Refuses noncanonical or absent reasons, foreign owners or an invalid
    /// original dictionary before publishing any merged row.
    pub fn with_accepted_rows<R>(
        &self,
        accepted: &[purrdf_core::evidence::ReasonId],
        visit: impl FnOnce(crate::evidence_exec::AcceptedAnswerRows<'_, D::Id>) -> R,
    ) -> Result<R, crate::EvalError> {
        let input = self.evidence_input()?;
        input.with_accepted_rows(self.seq, accepted, &self.ctx.growth, visit)
    }

    /// Read the explicitly merged answer as RDF 1.2, preserving original order,
    /// reasons and incompleteness while adding visible acceptance annotations.
    /// Unaccepted undecided rows remain in the original partition and are omitted
    /// from this merged ledger. No producer or scoring function is invoked.
    ///
    /// # Errors
    /// Refuses invalid acceptance, RDF declarations or original native failures
    /// before publishing the merged ledger.
    pub fn with_accepted_evidence_ledger<N: purrdf_core::evidence::LedgerNodes, R>(
        &self,
        read: &crate::evidence_exec::AcceptedLedgerRead<'_, N>,
        visit: impl FnOnce(&crate::evidence_exec::EvidenceLedger<'_, N>) -> R,
    ) -> Result<R, crate::EvalError> {
        self.evidence_input()?
            .with_accepted_ledger(self.seq, read, &self.ctx.growth, visit)
    }

    fn evidence_input(
        &self,
    ) -> Result<&crate::evidence_exec::EvidenceInput<'d, D::Id>, crate::EvalError> {
        self.ctx.evidence_input.as_ref().ok_or_else(|| {
            crate::NativeDiagnostic::error(
                crate::NativeDiagnosticKind::Config,
                "evidence reading requires the original evidence source",
                &self.ctx.growth,
            )
        })
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
    pub fn try_value_of(&self, term: SolutionTerm<D::Id>) -> Result<TermValue, crate::EvalError> {
        self.ctx
            .scratch
            .try_value_of(self.ctx.dataset, term)
            .map_err(crate::EvalError::source_read)
    }

    /// The owned [`TermValue`] of `row`'s `column`-th cell, when the row binds it.
    ///
    /// `None` covers both "not a column of this result" and "a column this row
    /// leaves unbound", which is what every caller of a projected-variable lookup
    /// already treats identically.
    pub fn try_cell(
        &self,
        row: &Solution<D::Id>,
        column: usize,
    ) -> Result<Option<TermValue>, crate::EvalError> {
        row.get(column)
            .copied()
            .flatten()
            .map(|term| self.try_value_of(term))
            .transpose()
    }

    /// Copy one bound cell through the native physical allocation factory.
    /// The returned owner keeps the cell's actual payload admitted after this
    /// borrowed result and its evaluation context have been dropped.
    ///
    /// # Errors
    /// Preserves the original dataset read or workspace admission failure.
    pub fn try_cell_owned(
        &self,
        row: &Solution<D::Id>,
        column: usize,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Option<crate::WorkspaceTerm>, crate::EvalError> {
        row.get(column)
            .copied()
            .flatten()
            .map(|term| {
                self.ctx.scratch.try_owned_value_of(
                    self.ctx.dataset,
                    term,
                    workspace,
                    crate::EvalError::source_read,
                )
            })
            .transpose()
    }

    /// The auxiliary graph of quads this query INVENTED, reachable from a term
    /// bound in a surviving row.
    ///
    /// A list constructor (`listSlice`, `listConcat`) binds a cell to the HEAD of
    /// an `rdf:List` whose cells exist nowhere in the queried dataset — they were
    /// minted by this execution. A caller that reads that head out of
    /// [`Self::cell`] and no more holds an identifier pointing into a graph it
    /// cannot see. This is that graph: the very same quads, built the very same
    /// way, that the owned egress hands back as
    /// [`SparqlResult::Solutions`](purrdf_core::SparqlResult)'s `aux` field, so the
    /// two egress doors answer identically about what the query returned.
    ///
    /// # Why an accessor and not a field
    ///
    /// The owned door attaches the graph eagerly because it is building a result
    /// that outlives the evaluation and has no later chance to. A visitor is still
    /// INSIDE the evaluation, so it can simply ask — and the overwhelmingly common
    /// query constructs nothing, so a field populated at construction would charge
    /// every borrowed result, on every row of every SHACL focus node, for an empty
    /// dataset nobody reads. Asked or not, a query that invented no cells walks no
    /// rows here.
    pub fn try_constructed_dataset(&self) -> Result<crate::RetainedGraph, crate::EvalError> {
        let graph = self.ctx.constructed_dataset_of(self.seq)?;
        InternedGraph::new(&graph, &self.ctx.growth).retain()
    }

    /// Materialize a cell of a resident result.
    pub fn value_of(&self, term: SolutionTerm<D::Id>) -> TermValue
    where
        D: DatasetView<ReadError = core::convert::Infallible>,
    {
        self.ctx.scratch.value_of(self.ctx.dataset, term)
    }
    /// Read one cell of a resident result.
    pub fn cell(&self, row: &Solution<D::Id>, column: usize) -> Option<TermValue>
    where
        D: DatasetView<ReadError = core::convert::Infallible>,
    {
        row.get(column)
            .copied()
            .flatten()
            .map(|term| self.value_of(term))
    }
    /// The auxiliary graph of a resident result.
    pub fn constructed_dataset(&self) -> crate::RetainedGraph
    where
        D: DatasetView<ReadError = core::convert::Infallible>,
    {
        self.try_constructed_dataset()
            .expect("resident result terms were admitted by this view")
    }
}

/// A graph borrowed inside the evaluator's read scope. Retaining it transfers
/// its original execution account with the shared frozen dataset, so an Arc
/// handle cannot escape while its admission is destroyed.
///
/// ```compile_fail
/// /// use purrdf_core::RdfDataset;
/// use purrdf_sparql_eval::InternedGraph;
/// fn escape(graph: InternedGraph<'_>) -> Arc<RdfDataset> {
///     Arc::clone(&graph)
/// }
/// ```
///
/// ```
/// use purrdf_sparql_eval::{EvalError, InternedGraph, RetainedGraph};
/// fn retain(graph: InternedGraph<'_>) -> Result<RetainedGraph, EvalError> {
///     graph.retain()
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct InternedGraph<'a> {
    graph: &'a purrdf_core::DatasetHandle,
    workspace: &'a crate::WorkspaceCapability,
}

impl<'a> InternedGraph<'a> {
    pub(crate) const fn new(
        graph: &'a purrdf_core::DatasetHandle,
        workspace: &'a crate::WorkspaceCapability,
    ) -> Self {
        Self { graph, workspace }
    }

    /// Share the frozen dataset with its original physical admission owner.
    /// This shallow clone allocates no graph or shared control.
    #[must_use]
    pub fn dataset_handle(self) -> purrdf_core::DatasetHandle {
        self.graph.clone()
    }

    /// Retain the existing frozen graph and its original account without
    /// copying the dataset. The new shared control is admitted before allocation.
    ///
    /// # Errors
    /// Returns physical admission, overflow or native allocator refusal.
    pub fn retain(self) -> Result<crate::RetainedGraph, crate::EvalError> {
        let result = crate::RetainedSparqlResult::new(
            purrdf_core::SparqlResult::Graph(self.graph.clone()),
            self.workspace.clone(),
            self.workspace,
        )?;
        Ok(crate::RetainedGraph::from_result(result))
    }
}

impl std::ops::Deref for InternedGraph<'_> {
    type Target = RdfDataset;
    fn deref(&self) -> &Self::Target {
        self.graph
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
    /// by its original owner; a visitor retaining it keeps the admission too.
    Graph(InternedGraph<'a>),
}

/// The outcome of a governed interned execution.
///
/// The interned twin of [`GovernedOutcome`](crate::GovernedOutcome), differing in
/// exactly one way: the complete arm carries what the VISITOR returned rather than
/// a materialized [`SparqlResult`](purrdf_core::SparqlResult). The exhausted arm
/// is unchanged, partial answers included — a trip is a cold, terminal path, and
/// the certified partial rows are the actionable half of the report.
///
/// The width ratio between the arms is the design, not an oversight, which is why
/// clippy's `large_enum_variant` is scoped off here. The exhausted arm is boxed
/// because it is cold. The complete arm carries its receipts inline because it
/// is the arm the per-focus-node change path takes on every call, and the
/// `relations` receipt now carries the relation witness — the per-relation ledger
/// of which index generation answered and whether it was whole — so it is the
/// wider of the two. Boxing either receipt to please the ratio would put a heap
/// allocation on the hot arm to save stack width on a cold one.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum InternedGoverned<R> {
    /// The execution completed within budget; `value` is the visitor's return.
    Complete {
        /// What the visitor projected out of the interned result.
        value: R,
        /// This execution's resource receipt.
        evidence: purrdf_core::GovernorEvidence,
        /// The property-function registry identity the plan was admitted under,
        /// carrying the witness this execution's relations wrote.
        relations: RelationIdentity,
        /// Supplied coordinates captured before evaluation begins.
        coordinates: crate::QueryCoordinates,
    },
    /// A governor stopped the execution; the visitor never ran.
    ///
    /// Boxed because the payload carries its receipts and the certified partial
    /// answers, and every caller of the complete path would otherwise pay that
    /// width on the stack for an arm it does not take.
    BudgetExhausted(Box<BudgetExhausted>),
}

impl<R> InternedGoverned<R> {
    /// Supplied coordinates, or a refusal before context construction.
    #[must_use]
    pub const fn coordinates(&self) -> &crate::QueryCoordinates {
        match self {
            Self::Complete { coordinates, .. } => coordinates,
            Self::BudgetExhausted(exhausted) => &exhausted.coordinates,
        }
    }
}
