// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Operation-local graph support and first-cause ownership over compact payloads.

use super::{TripleDataset, TripleIds, TripleOrder, TripleReifierRow};
use crate::dataset_view::{
    DatasetView, FallibleDatasetView, GraphMatch, ViewOperationStatus, WorkspaceReservation,
};
use crate::ir::{QuadIds, TermId, TermRef, TermValue};
use crate::{RdfDiagnostic, RdfStoreCapabilities};
use purrdf_lex::allocation::{Memory, Resident, Shared, SharedCloneError, StorageError};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// Original operational failure of one graph-free read session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TripleReadError {
    /// A named-graph request cannot be represented by this carrier.
    NamedGraphsUnsupported,
    /// Native layout, logical capacity, admission or allocator refusal.
    Storage(StorageError),
    /// Sharing an operation would overflow the original owner's reference count.
    SharedOwnership(SharedCloneError),
}

impl purrdf_hash::error::StructuredError for TripleReadError {
    purrdf_hash::error_visit! {
        NamedGraphsUnsupported => "triple-read-named-graphs-unsupported" {},
        Storage(cause) => "triple-read-storage" { "cause" => Cause(cause) },
        SharedOwnership(cause) => "triple-read-shared-ownership" { "cause" => Cause(cause) },
    }
}

impl std::fmt::Display for TripleReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NamedGraphsUnsupported => {
                f.write_str("the triple-only carrier does not support named graphs")
            }
            Self::Storage(error) => error.fmt(f),
            Self::SharedOwnership(error) => error.fmt(f),
        }
    }
}

purrdf_lex::error_source!(TripleReadError {
    Storage,
    SharedOwnership
});
purrdf_lex::variant_from!(TripleReadError { Storage(StorageError) });

impl From<TripleReadError> for RdfDiagnostic {
    fn from(error: TripleReadError) -> Self {
        match error {
            TripleReadError::Storage(error) => error.into(),
            TripleReadError::NamedGraphsUnsupported => Self::error(
                "rdf-named-graphs-unsupported",
                "the triple-only carrier does not support named graphs",
            ),
            TripleReadError::SharedOwnership(error) => {
                Self::error("rdf-shared-ownership", error.to_string())
            }
        }
    }
}

/// A fresh read operation over one immutable graph-free payload. Forks share
/// this operation's first error; another call to `TripleDataset::read` starts a
/// healthy operation over exactly the same payload.
#[derive(Debug)]
pub struct TripleRead<'a> {
    dataset: &'a TripleDataset,
    failure: Shared<ReadFailure>,
}

#[derive(Debug, Default)]
struct ReadFailure {
    failed: AtomicBool,
    cause: Mutex<Option<TripleReadError>>,
}

impl TripleDataset {
    /// Open a fallible, operation-local read session.
    ///
    /// # Errors
    /// Refuses the concrete control allocation before creating the session.
    pub fn read(&self) -> Result<TripleRead<'_>, TripleReadError> {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        memory.add_bytes(Shared::<ReadFailure>::allocation_layout().size())?;
        let failure =
            Shared::try_new(ReadFailure::default()).map_err(|_| StorageError::AllocationFailed)?;
        Ok(TripleRead {
            dataset: self,
            failure,
        })
    }
}

impl crate::SnapshotStatistics<TermId> for TripleRead<'_> {
    fn source(&self) -> &str {
        crate::SnapshotStatistics::source(&self.dataset.statistics)
    }

    fn fingerprint(&self) -> u64 {
        crate::SnapshotStatistics::fingerprint(&self.dataset.statistics)
    }

    fn pattern(&self, pattern: crate::ProbePattern) -> Option<crate::StatisticCount> {
        self.graph(pattern.g).ok()?;
        crate::SnapshotStatistics::pattern(&self.dataset.statistics, pattern)
    }

    fn predicate(
        &self,
        predicate: TermId,
        graph: GraphMatch,
        stream: crate::StatisticsStream,
    ) -> Option<crate::PredicateStatistics> {
        self.graph(graph).ok()?;
        crate::SnapshotStatistics::predicate(&self.dataset.statistics, predicate, graph, stream)
    }

    fn predicate_object(
        &self,
        predicate: TermId,
        object: TermId,
        graph: GraphMatch,
        stream: crate::StatisticsStream,
    ) -> Option<crate::StatisticCount> {
        self.graph(graph).ok()?;
        crate::SnapshotStatistics::predicate_object(
            &self.dataset.statistics,
            predicate,
            object,
            graph,
            stream,
        )
    }

    fn predicate_object_kind(
        &self,
        predicate: TermId,
        kind: crate::RdfTermKind,
        datatype: Option<TermId>,
        graph: GraphMatch,
        stream: crate::StatisticsStream,
    ) -> Option<crate::StatisticCount> {
        self.graph(graph).ok()?;
        crate::SnapshotStatistics::predicate_object_kind(
            &self.dataset.statistics,
            predicate,
            kind,
            datatype,
            graph,
            stream,
        )
    }

    fn reifier_target_predicate(
        &self,
        predicate: TermId,
        graph: GraphMatch,
    ) -> Option<crate::PredicateStatistics> {
        self.graph(graph).ok()?;
        crate::SnapshotStatistics::reifier_target_predicate(
            &self.dataset.statistics,
            predicate,
            graph,
        )
    }

    fn stream_rows(
        &self,
        graph: GraphMatch,
        stream: crate::StatisticsStream,
    ) -> Option<crate::StatisticCount> {
        self.graph(graph).ok()?;
        crate::SnapshotStatistics::stream_rows(&self.dataset.statistics, graph, stream)
    }

    fn visit_distinct(
        &self,
        predicate: TermId,
        graph: GraphMatch,
        stream: crate::StatisticsStream,
        position: crate::StatisticsPosition,
        visit: &mut dyn FnMut(TermId) -> std::ops::ControlFlow<()>,
    ) -> Result<bool, super::super::pack::bits::PackBitsError> {
        if self.graph(graph).is_err() {
            return Ok(false);
        }
        crate::SnapshotStatistics::visit_distinct(
            &self.dataset.statistics,
            predicate,
            graph,
            stream,
            position,
            visit,
        )
    }
}

impl TripleRead<'_> {
    fn latch(&self, error: TripleReadError) -> TripleReadError {
        let mut failure = self
            .failure
            .cause
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let first = *failure.get_or_insert(error);
        drop(failure);
        self.failure.failed.store(true, Ordering::Release);
        first
    }

    fn ready(&self) -> Result<(), TripleReadError> {
        self.read_error().map_or(Ok(()), Err)
    }

    fn graph(&self, graph: GraphMatch) -> Result<(), TripleReadError> {
        self.ready()?;
        match graph {
            GraphMatch::Named(_) => self.require_named_graphs(),
            GraphMatch::Any | GraphMatch::Default => Ok(()),
        }
    }

    /// Create a parallel/correlated child retaining the same original cause.
    ///
    /// # Errors
    /// Returns the existing cause or a newly latched shared-ownership refusal.
    pub fn fork(&self) -> Result<Self, TripleReadError> {
        self.ready()?;
        let failure = self
            .failure
            .try_clone()
            .map_err(|error| self.latch(TripleReadError::SharedOwnership(error)))?;
        Ok(Self {
            dataset: self.dataset,
            failure,
        })
    }

    fn project(row: TripleIds) -> QuadIds {
        QuadIds {
            s: row.s,
            p: row.p,
            o: row.o,
            g: None,
        }
    }

    fn planned_rows(
        &self,
        plan: TripleOrder,
        pattern: [Option<TermId>; 3],
        graph: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        let rows = self.graph(graph).and_then(|()| {
            self.dataset
                .triples
                .probe_ordered(plan, pattern)
                .map_err(|error| self.latch(error.into()))
        });
        rows.ok()
            .into_iter()
            .flatten()
            .take_while(|_| self.ready().is_ok())
            .map(Self::project)
    }

    fn binding_rows(
        &self,
        reifier: Option<TermId>,
        target: Option<TermId>,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        let rows = self.ready().and_then(|()| {
            self.dataset
                .reifiers_for_pattern(reifier, target)
                .map_err(|error| self.latch(error.into()))
        });
        self.project_bindings(rows)
    }

    fn project_bindings<'a, I: Iterator<Item = TripleReifierRow> + 'a>(
        &'a self,
        rows: Result<I, TripleReadError>,
    ) -> impl Iterator<Item = QuadIds> + 'a {
        let predicate = self
            .dataset
            .dictionary
            .term_id_by_iri(purrdf_iri::vocab::rdf::REIFIES);
        rows.ok()
            .into_iter()
            .flatten()
            .take_while(|_| self.ready().is_ok())
            .filter_map(move |(s, o)| predicate.map(|p| QuadIds { s, p, o, g: None }))
    }

    fn annotation_rows(
        &self,
        pattern: [Option<TermId>; 3],
        graph: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        let rows = self.graph(graph).and_then(|()| {
            self.dataset
                .annotations
                .probe(pattern)
                .map_err(|error| self.latch(error.into()))
        });
        rows.ok()
            .into_iter()
            .flatten()
            .take_while(|_| self.ready().is_ok())
            .map(Self::project)
    }
}

/// Resident workspace has no application ceiling, but every subsequent use
/// still observes the read operation's original first cause.
#[derive(Debug)]
pub struct TripleReservation<'a> {
    read: &'a TripleRead<'a>,
}

impl WorkspaceReservation for TripleReservation<'_> {
    type Error = TripleReadError;
    fn resize(&mut self, _: u64) -> Result<(), Self::Error> {
        self.read.ready()
    }
}

impl DatasetView for TripleRead<'_> {
    type Id = TermId;
    type ReadError = TripleReadError;
    type TermGuard<'a>
        = TermRef<'a>
    where
        Self: 'a;
    type ProbePlan = TripleOrder;

    fn snapshot_identity(&self) -> Result<Option<purrdf_hash::hex::Digest32>, Self::ReadError> {
        if let Some(error) = self.read_error() {
            return Err(error);
        }
        Ok(Some(self.dataset.snapshot_identity))
    }

    fn read_error(&self) -> Option<Self::ReadError> {
        if !self.failure.failed.load(Ordering::Acquire) {
            return None;
        }
        *self
            .failure
            .cause
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn require_named_graphs(&self) -> Result<(), Self::ReadError> {
        Err(self.latch(TripleReadError::NamedGraphsUnsupported))
    }

    fn supports_named_graphs(&self) -> bool {
        false
    }

    fn reserve_workspace(
        &self,
        _: u64,
    ) -> Result<impl WorkspaceReservation<Error = Self::ReadError> + '_, Self::ReadError> {
        self.ready()?;
        Ok(TripleReservation { read: self })
    }

    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.dataset
            .triples()
            .take_while(|_| self.ready().is_ok())
            .map(Self::project)
    }

    fn resolve(&self, id: TermId) -> Result<Self::TermGuard<'_>, Self::ReadError> {
        self.ready()?;
        Ok(self.dataset.resolve(id))
    }

    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, Self::ReadError> {
        self.ready()?;
        Ok(self.dataset.term_id_by_value(value))
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        self.dataset.caps
    }
    fn term_count(&self) -> u64 {
        self.dataset.term_count() as u64
    }
    fn term_bytes_hint(&self) -> Option<u64> {
        Some(self.dataset.dictionary.arena.len() as u64)
    }
    fn len_hint(&self) -> Option<u64> {
        Some(self.dataset.len() as u64)
    }
    crate::dataset_view::frozen_term_nesting_bound_method! {
        /// Compact publication uses the original native term validator and bound.
    }

    fn probe_plan(&self, s: bool, p: bool, o: bool, g: GraphMatch) -> Self::ProbePlan {
        let _ = self.graph(g);
        TripleOrder::selected([s, p, o])
    }

    fn quads_for_pattern(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.planned_rows(
            TripleOrder::selected([s.is_some(), p.is_some(), o.is_some()]),
            [s, p, o],
            g,
        )
    }

    fn quads_for_pattern_with_plan(
        &self,
        plan: &Self::ProbePlan,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.planned_rows(*plan, [s, p, o], g)
    }

    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.binding_rows(None, None)
    }

    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.dataset
            .annotations()
            .take_while(|_| self.ready().is_ok())
            .map(Self::project)
    }

    fn reifier_quads_of(&self, reifier: TermId) -> impl Iterator<Item = QuadIds> + '_ {
        self.binding_rows(Some(reifier), None)
    }

    fn reifier_quads_for_target(
        &self,
        target: TermId,
        graph: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        let _ = self.graph(graph);
        self.binding_rows(None, Some(target))
    }

    fn statement_indexes(&self) -> crate::StatementIndexSupport {
        crate::StatementIndexSupport::INDEXED
    }

    fn statistics(&self) -> Option<&dyn crate::SnapshotStatistics<TermId>> {
        self.ready()
            .ok()
            .map(|()| self as &dyn crate::SnapshotStatistics<TermId>)
    }

    fn source_summary(
        &self,
        predicate: Self::Id,
        graph: GraphMatch<Self::Id>,
        stream: crate::StatisticsStream,
    ) -> Result<Option<crate::source::SourceSummary>, Self::ReadError> {
        self.graph(graph)?;
        Ok(Some(
            self.dataset
                .statistics
                .source_summary(predicate, graph, stream),
        ))
    }

    fn source_statement_count(&self) -> Result<Option<u128>, Self::ReadError> {
        self.ready()?;
        Ok(Some(self.dataset.statistics.statement_count()))
    }

    fn statement_cardinality_estimate(
        &self,
        pattern: crate::ProbePattern,
        components: [Option<TermId>; 3],
    ) -> Option<u64> {
        self.graph(pattern.g).ok()?;
        if self.dataset.reifiers.is_empty() && self.dataset.annotations.rows.is_empty() {
            return Some(0);
        }
        let can_reify = pattern.p.is_none_or(|predicate| {
            Some(predicate)
                == self
                    .dataset
                    .dictionary
                    .term_id_by_iri(purrdf_iri::vocab::rdf::REIFIES)
        });
        let reifiers = if !can_reify || self.dataset.reifiers.is_empty() {
            0
        } else if pattern.s.is_some() || pattern.o.is_some() {
            self.dataset
                .reifiers_for_pattern(pattern.s, pattern.o)
                .map_err(|error| self.latch(error.into()))
                .ok()?
                .size_hint()
                .1?
        } else {
            self.dataset
                .reifiers_for_components(components)
                .map_err(|error| self.latch(error.into()))
                .ok()?
                .len()
        };
        let annotations = self
            .dataset
            .annotations
            .probe([pattern.s, pattern.p, pattern.o])
            .map_err(|error| self.latch(error.into()))
            .ok()?
            .size_hint()
            .1?;
        u64::try_from(reifiers)
            .ok()?
            .checked_add(u64::try_from(annotations).ok()?)
    }

    fn reifier_quads_for_components(
        &self,
        components: [Option<TermId>; 3],
        graph: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        let rows = self.graph(graph).and_then(|()| {
            self.dataset
                .reifiers_for_components(components)
                .map_err(|error| self.latch(error.into()))
        });
        self.project_bindings(rows)
    }

    fn annotation_quads_for_pattern(
        &self,
        predicate: Option<TermId>,
        object: Option<TermId>,
        graph: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.annotation_rows([None, predicate, object], graph)
    }

    fn annotations_of_with_graph(
        &self,
        reifier: TermId,
    ) -> impl Iterator<Item = (TermId, TermId, Option<TermId>)> + '_ {
        self.annotation_rows([Some(reifier), None, None], GraphMatch::Any)
            .map(|quad| (quad.p, quad.o, quad.g))
    }

    fn named_graphs(&self) -> impl Iterator<Item = TermId> + '_ {
        let _ = self.require_named_graphs();
        std::iter::empty()
    }

    fn has_named_graph(&self, _: TermId) -> bool {
        let _ = self.require_named_graphs();
        false
    }

    fn reifier_quads_in_graph(&self, g: GraphMatch) -> impl Iterator<Item = QuadIds> + '_ {
        let _ = self.graph(g);
        self.reifier_quads()
    }

    fn annotation_quads_in_graph(&self, g: GraphMatch) -> impl Iterator<Item = QuadIds> + '_ {
        let _ = self.graph(g);
        self.annotation_quads()
    }
}

impl FallibleDatasetView for TripleRead<'_> {
    type Error = TripleReadError;
    type Evidence = ();
    fn operation_status(&self) -> ViewOperationStatus<Self::Error, Self::Evidence> {
        match self.read_error() {
            Some(error) => ViewOperationStatus::Failed {
                error,
                evidence: (),
            },
            None => ViewOperationStatus::Ready { evidence: () },
        }
    }
}
