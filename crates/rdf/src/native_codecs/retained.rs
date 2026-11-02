// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native source and dataset payloads retain their original admission owner.

use core::convert::Infallible;

use purrdf_core::dataset_view::{
    DatasetView, GraphMatch, OwnedWorkspaceReservation, WorkspaceReservation,
};
use purrdf_core::{
    QuadIds, RdfDataset, RdfStoreCapabilities, SnapshotStatistics, StatisticsStream, TermId,
    TermRef, TermValue,
};
use purrdf_iri::BaseScope;
use purrdf_lex::allocation::{Admission, OrderedTable, Resident};
use purrdf_lex::control::{Progress, Unmetered};

use super::acquisition::{SourceFailure, SourceRun, TextError, resident_error};
use super::parse::{ParseFailure, ParseOutcome};
use super::source_occurrence::{
    SourceOccurrence, SourceOriginId, SourceStatementKind, SourceTermOrigin,
};
use super::span::SpanTable;

/// Inline first cause and the original directive payload; no failure-only box
/// or error conversion bound is needed under the original account.
pub(crate) struct RetainedFailure<E> {
    cause: TextError<E>,
    document_base: BaseScope,
    document_prefixes: OrderedTable<String, String>,
}

impl<E> RetainedFailure<E> {
    pub(crate) fn new(
        cause: TextError<E>,
        document_base: BaseScope,
        document_prefixes: OrderedTable<String, String>,
    ) -> Self {
        Self {
            cause,
            document_base,
            document_prefixes,
        }
    }
}

/// The frozen payload stays by value inside its original admission owner.
pub(crate) struct RetainedOutcome {
    pub(crate) dataset: RdfDataset,
    pub(crate) spans: Option<SpanTable>,
    pub(crate) document_base: BaseScope,
    pub(crate) document_prefixes: OrderedTable<String, String>,
}

/// One unique owner. Dataset/error payloads die first, then original source
/// storage, then the sole original context's admission guard inside `source`.
/// Nothing on the generic path can extract or clone the concrete dataset.
pub struct RetainedParse<'a, A: Admission, C: Progress> {
    output: Result<RetainedOutcome, RetainedFailure<C::Error>>,
    source: SourceRun<'a, A, C>,
}

purrdf_hash::debug_non_exhaustive!(['a, A: Admission, C: Progress] RetainedParse<'a, A, C> {});

impl<'a, A: Admission, C: Progress> RetainedParse<'a, A, C> {
    /// All supplied controlled payloads must already belong to `source`'s
    /// original account. This constructor does not adopt resident allocations.
    pub(crate) fn new(
        source: SourceRun<'a, A, C>,
        output: Result<RetainedOutcome, RetainedFailure<C::Error>>,
    ) -> Self {
        Self { output, source }
    }

    /// Borrow the frozen dataset without detaching its concrete native owner.
    /// No query workspace authority is supplied by source acquisition.
    pub fn dataset(
        &self,
    ) -> Option<impl DatasetView<Id = TermId, ReadError = BorrowedReadError> + '_> {
        self.output.as_ref().ok().map(|output| BorrowedDataset {
            dataset: &output.dataset,
        })
    }

    /// Borrow the original source bytes and exact source occurrences.
    pub fn source(&self) -> BorrowedSource<'_> {
        BorrowedSource {
            bytes: self.source.bytes(),
            spans: self
                .output
                .as_ref()
                .ok()
                .and_then(|output| output.spans.as_ref()),
        }
    }

    pub(crate) fn cause(&self) -> Option<&TextError<C::Error>> {
        self.output.as_ref().err().map(|failure| &failure.cause)
    }

    /// The native document's final base, borrowed under its original owner.
    pub fn document_base_iri(&self) -> Option<&str> {
        match &self.output {
            Ok(output) => output
                .document_base
                .current()
                .map(|base| base.iri().as_str()),
            Err(failure) => failure
                .document_base
                .current()
                .map(|base| base.iri().as_str()),
        }
    }

    /// Declared prefix bindings in key order, without copying their payloads.
    pub fn document_prefixes(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        let prefixes = match &self.output {
            Ok(output) => &output.document_prefixes,
            Err(failure) => &failure.document_prefixes,
        };
        prefixes
            .range::<str, _>(..)
            .map(|(label, iri)| (label.as_str(), iri.as_str()))
    }
}

/// A refused parse retains its first typed failure, full input and original
/// account. It exposes no partially constructed dataset.
pub struct ParseRefusal<'a, A: Admission, C: Progress> {
    owner: RefusalOwner<'a, A, C>,
}

#[expect(
    clippy::large_enum_variant,
    reason = "The original parsed payloads and admission owner must survive refusal without a new allocation."
)]
enum RefusalOwner<'a, A: Admission, C: Progress> {
    Source(SourceFailure<'a, A, C>),
    Parsed(RetainedParse<'a, A, C>),
}

purrdf_hash::debug_non_exhaustive!(['a, A: Admission, C: Progress] ParseRefusal<'a, A, C> {});

impl<'a, A: Admission, C: Progress> ParseRefusal<'a, A, C> {
    pub(super) fn from_source(failure: SourceFailure<'a, A, C>) -> Self {
        Self {
            owner: RefusalOwner::Source(failure),
        }
    }

    #[expect(
        clippy::result_large_err,
        reason = "Refusal retains the original parsed payloads and account inline without allocating after refusal."
    )]
    pub(super) fn classify(
        owner: RetainedParse<'a, A, C>,
    ) -> Result<RetainedParse<'a, A, C>, Self> {
        if owner.cause().is_some() {
            Err(Self {
                owner: RefusalOwner::Parsed(owner),
            })
        } else {
            Ok(owner)
        }
    }

    /// Borrow the unchanged native or caller failure without formatting it.
    pub fn cause(&self) -> &TextError<C::Error> {
        match &self.owner {
            RefusalOwner::Source(owner) => owner.cause(),
            // classify admits only owners whose original failure is present.
            RefusalOwner::Parsed(owner) => owner
                .cause()
                .expect("refused owner retains its first cause"),
        }
    }

    /// Full original input, including when its owned source copy was refused.
    pub fn input(&self) -> &[u8] {
        match &self.owner {
            RefusalOwner::Source(owner) => owner.input(),
            RefusalOwner::Parsed(owner) => owner.source.bytes(),
        }
    }

    /// The original base in force at refusal, if source initialization completed.
    pub fn document_base_iri(&self) -> Option<&str> {
        match &self.owner {
            RefusalOwner::Source(_) => None,
            RefusalOwner::Parsed(owner) => owner.document_base_iri(),
        }
    }

    /// Original declarations encountered before refusal, without allocation.
    pub fn document_prefixes(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        let parsed = match &self.owner {
            RefusalOwner::Source(_) => None,
            RefusalOwner::Parsed(owner) => Some(owner),
        };
        parsed
            .into_iter()
            .flat_map(RetainedParse::document_prefixes)
    }
}

impl<A: Admission, C: Progress> core::fmt::Display for ParseRefusal<'_, A, C>
where
    C::Error: core::fmt::Display,
{
    fn fmt(&self, output: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.cause().fmt(output)
    }
}

impl<A: Admission, C: Progress> std::error::Error for ParseRefusal<'_, A, C>
where
    C::Error: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.cause())
    }
}

impl RetainedParse<'_, Resident, Unmetered> {
    /// Only the pre-existing resident API may detach its documented owned
    /// dataset/diagnostic. Controlled owners have no corresponding extraction.
    pub(crate) fn into_legacy(self) -> Result<ParseOutcome, Box<ParseFailure>> {
        match self.output {
            Ok(output) => Ok(ParseOutcome {
                dataset: std::sync::Arc::new(output.dataset),
                spans: output.spans,
                document_base: output.document_base.current().cloned(),
                document_prefixes: resident_prefixes(output.document_prefixes),
            }),
            Err(failure) => Err(Box::new(ParseFailure {
                diagnostic: resident_error(failure.cause),
                document_base: failure.document_base.current().cloned(),
                document_prefixes: resident_prefixes(failure.document_prefixes),
            })),
        }
    }
}

/// Only the resident convenience API exports a detached directive vector.
fn resident_prefixes(mut prefixes: OrderedTable<String, String>) -> Vec<(String, String)> {
    let mut output = Vec::with_capacity(prefixes.len());
    while let Some(pair) = prefixes.pop_first() {
        output.push(pair);
    }
    output
}

/// Borrowed source data without exposing cloneable side-table owners.
#[derive(Debug)]
pub struct BorrowedSource<'a> {
    bytes: &'a [u8],
    spans: Option<&'a SpanTable>,
}

impl BorrowedSource<'_> {
    /// Borrow the source bytes whose offsets the native occurrences address.
    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }

    /// Visit exact occurrences in native encounter order, including duplicates.
    pub fn occurrences(&self) -> impl Iterator<Item = &SourceOccurrence> + '_ {
        self.spans
            .into_iter()
            .flat_map(|spans| spans.source_occurrences().iter())
    }

    /// Borrow the source spelling or native generation rule of an occurrence.
    pub fn origin(&self, id: SourceOriginId) -> Option<&SourceTermOrigin> {
        self.spans
            .and_then(|spans| spans.source_occurrences().origin(id))
    }

    /// The native RDF statement layer chosen for an emission ordinal.
    pub fn statement_kind(&self, statement: usize) -> Option<SourceStatementKind> {
        self.spans
            .and_then(|spans| spans.source_occurrences().statement_kind(statement))
    }
}

/// Borrowed dataset inspection has no query workspace provider. This is a
/// capability refusal, distinct from the retained acquisition's original cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowedReadError {
    /// Source acquisition supplies no query workspace admission account.
    WorkspaceUnavailable,
}

impl core::fmt::Display for BorrowedReadError {
    fn fmt(&self, output: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        output.write_str("retained source inspection has no query workspace authority")
    }
}

impl std::error::Error for BorrowedReadError {}

impl From<Infallible> for BorrowedReadError {
    fn from(impossible: Infallible) -> Self {
        match impossible {}
    }
}

/// There is no successful workspace permit to construct or detach from this
/// inspection facade, even for a zero-byte query preflight.
struct NoWorkspace(Infallible);

impl WorkspaceReservation for NoWorkspace {
    type Error = BorrowedReadError;
    fn resize(&mut self, _bytes: u64) -> Result<(), Self::Error> {
        Err(BorrowedReadError::WorkspaceUnavailable)
    }
}

/// A borrowed trait facade. Concrete cursors and the concrete dataset stay
/// private, including the cursor method that can clone a detached dataset.
/// Native borrowed reads and sealed metadata are forwarded. Pattern inspection
/// scans the existing arrays and never initializes a lazy index. Query workspace
/// admission is deliberately unavailable: callers must use a separately
/// admitted query session, and must not interpret this facade as an unbounded
/// resident provider or as the acquisition account's query authority.
struct BorrowedDataset<'a> {
    dataset: &'a RdfDataset,
}

impl DatasetView for BorrowedDataset<'_> {
    type Id = TermId;
    type ReadError = BorrowedReadError;
    type TermGuard<'a>
        = TermRef<'a>
    where
        Self: 'a;
    type ProbePlan = ();

    /// Deny the scoped permit, including the zero-byte preflight used by
    /// callers that have no live-budget certificate. No resident no-op guard
    /// is created and no acquisition grant is lent or replaced.
    fn reserve_workspace(
        &self,
        _bytes: u64,
    ) -> Result<impl WorkspaceReservation<Error = Self::ReadError> + '_, Self::ReadError> {
        Err::<NoWorkspace, _>(BorrowedReadError::WorkspaceUnavailable)
    }

    /// No independently retained query account can be extracted from the
    /// uniquely owned acquisition. Consumers must refuse retained execution.
    fn reserve_owned_workspace(
        &self,
        _bytes: u64,
    ) -> Option<Result<OwnedWorkspaceReservation<Self::ReadError>, Self::ReadError>> {
        None
    }

    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.dataset.quads()
    }

    fn resolve(&self, id: TermId) -> Result<Self::TermGuard<'_>, Self::ReadError> {
        DatasetView::resolve(self.dataset, id).map_err(BorrowedReadError::from)
    }

    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, Self::ReadError> {
        DatasetView::term_id_by_value(self.dataset, value).map_err(BorrowedReadError::from)
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        self.dataset.capabilities()
    }

    fn probe_plan(&self, _s: bool, _p: bool, _o: bool, _g: GraphMatch) {}

    fn quads_for_pattern_with_plan(
        &self,
        _plan: &(),
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.quads_for_pattern(s, p, o, g)
    }

    fn term_count(&self) -> u64 {
        DatasetView::term_count(self.dataset)
    }

    fn len_hint(&self) -> Option<u64> {
        DatasetView::len_hint(self.dataset)
    }

    fn term_bytes_hint(&self) -> Option<u64> {
        DatasetView::term_bytes_hint(self.dataset)
    }

    fn triple_term_nesting_bound(&self) -> Option<usize> {
        DatasetView::triple_term_nesting_bound(self.dataset)
    }

    fn named_graphs(&self) -> impl Iterator<Item = TermId> + '_ {
        self.dataset.named_graphs()
    }

    fn has_named_graph(&self, graph: TermId) -> bool {
        self.dataset.has_named_graph(graph)
    }

    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.dataset.reifier_quads()
    }

    fn reifier_quads_of(&self, reifier: TermId) -> impl Iterator<Item = QuadIds> + '_ {
        self.dataset.reifier_quads_of(reifier)
    }

    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.dataset.annotation_quads()
    }

    fn annotations_of_with_graph(
        &self,
        reifier: TermId,
    ) -> impl Iterator<Item = (TermId, TermId, Option<TermId>)> + '_ {
        self.dataset.annotations_of_with_graph(reifier)
    }

    fn stats_fingerprint(&self) -> u64 {
        self.dataset.stats_fingerprint()
    }

    fn snapshot_identity(&self) -> Result<Option<purrdf_hash::hex::Digest32>, Self::ReadError> {
        DatasetView::snapshot_identity(self.dataset).map_err(BorrowedReadError::from)
    }

    fn statistics(&self) -> Option<&dyn SnapshotStatistics<TermId>> {
        DatasetView::statistics(self.dataset)
    }

    fn source_statement_count(&self) -> Result<Option<u128>, Self::ReadError> {
        DatasetView::source_statement_count(self.dataset).map_err(BorrowedReadError::from)
    }

    fn source_summary(
        &self,
        predicate: TermId,
        graph: GraphMatch,
        stream: StatisticsStream,
    ) -> Result<Option<purrdf_core::source::SourceSummary>, Self::ReadError> {
        DatasetView::source_summary(self.dataset, predicate, graph, stream)
            .map_err(BorrowedReadError::from)
    }
}
