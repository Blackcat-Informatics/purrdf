// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Effective chronological streams over source-local, lazily addressed values.

use super::super::SegmentedTermGuard;
use super::{
    DatasetView, GlobalTermId, HashTable, QueryStorage, SegmentedError, SegmentedOperation,
    SegmentedStackQuery, SegmentedStackSnapshot, SegmentedStackSource, StorageError, TermId,
    TermValue, WorkspaceReservation, hash_of, lock_read_state, reserve_table_with_memory,
};
use crate::ir::mutable::{StatementKind, classify_statement};
use crate::{
    FallibleDatasetView, GraphMatch, ProbePattern, QuadIds, RdfStoreCapabilities, TermGuard,
    TermRef, ViewOperationStatus,
};

#[derive(Debug)]
enum SourceGuard<'a> {
    Sealed(SegmentedTermGuard),
    Resident(TermRef<'a>),
}

impl SourceGuard<'_> {
    fn term(&self) -> TermRef<'_, u64> {
        match self {
            Self::Sealed(guard) => guard.term().map_ids(GlobalTermId::index),
            Self::Resident(term) => term.map_ids(local_index),
        }
    }
}

/// Original source pin with translated datatype or triple component identities.
/// Strings remain borrowed from that source for the guard's entire lifetime.
#[derive(Debug)]
pub struct SegmentedStackTermGuard<'a> {
    source: SourceGuard<'a>,
    children: [GlobalTermId; 3],
}

impl TermGuard<GlobalTermId> for SegmentedStackTermGuard<'_> {
    fn term(&self) -> TermRef<'_, GlobalTermId> {
        let mut position = 0;
        self.source.term().map_ids(|_| {
            let id = self.children[position];
            position += 1;
            id
        })
    }
}

fn local_index(id: TermId) -> u64 {
    u64::from(id.raw() - 1)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stream {
    Ordinary,
    Reifier,
    Annotation,
}

struct Seen<T> {
    entries: HashTable<T>,
    storage: QueryStorage,
}

impl<T: Eq + std::hash::Hash> Seen<T> {
    fn new(operation: &SegmentedOperation) -> Result<Self, SegmentedError> {
        Ok(Self {
            entries: HashTable::new(),
            storage: QueryStorage::new(operation)?,
        })
    }

    fn insert(&mut self, value: T) -> Result<bool, SegmentedError> {
        let hash = hash_of(&value);
        if self.entries.find(hash, |other| *other == value).is_some() {
            return Ok(false);
        }
        let next = self
            .entries
            .len()
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?;
        self.storage
            .run(|memory| reserve_table_with_memory(&mut self.entries, next, hash_of, memory))?;
        self.entries.insert_unique(hash, value, hash_of);
        Ok(true)
    }
}

impl SegmentedStackQuery<'_> {
    pub(super) fn source_at(&self, layer: usize) -> SegmentedStackSource {
        if layer == self.sessions.len() {
            SegmentedStackSource::Head
        } else {
            SegmentedStackSource::Layer(layer)
        }
    }

    pub(super) fn source_count(&self) -> usize {
        self.sessions.len() + usize::from(self.snapshot.head.is_some())
    }

    pub(super) fn translate_pattern<D: DatasetView>(
        &self,
        source: &D,
        origin: SegmentedStackSource,
        pattern: ProbePattern<GlobalTermId>,
        local: impl Fn(u64) -> Result<D::Id, SegmentedError>,
        error: impl Fn(D::ReadError) -> SegmentedError,
    ) -> Result<Option<ProbePattern<D::Id>>, SegmentedError> {
        let convert = |id: Option<GlobalTermId>| {
            id.map(|id| {
                // IDs originally imported from this source already carry its
                // exact local address. Do not materialize and reverse-search
                // that same value again for each annotation membership probe.
                let original = {
                    let identities = lock_read_state(&self.identities);
                    usize::try_from(id.index())
                        .ok()
                        .and_then(|index| identities.representatives.get(index))
                        .map(|representative| representative.term)
                };
                if let Some(term) = original.filter(|term| term.source == origin) {
                    return local(term.index).map(Some);
                }
                let value = self.term_value(id)?;
                source.term_id_by_value(value.value()).map_err(&error)
            })
            .transpose()
        };
        let (s, p, o) = (
            convert(pattern.s)?,
            convert(pattern.p)?,
            convert(pattern.o)?,
        );
        if s == Some(None) || p == Some(None) || o == Some(None) {
            return Ok(None);
        }
        let g = match pattern.g {
            GraphMatch::Any => GraphMatch::Any,
            GraphMatch::Default => GraphMatch::Default,
            GraphMatch::Named(id) => {
                let Some(Some(id)) = convert(Some(id))? else {
                    return Ok(None);
                };
                GraphMatch::Named(id)
            }
        };
        Ok(Some(ProbePattern {
            s: s.flatten(),
            p: p.flatten(),
            o: o.flatten(),
            g,
        }))
    }

    fn rows_from<'a, D: DatasetView, R: Iterator<Item = QuadIds<D::Id>> + 'a>(
        &'a self,
        dataset: &'a D,
        origin: SegmentedStackSource,
        rows: impl Fn(ProbePattern<D::Id>) -> R + 'a,
        pattern: ProbePattern<GlobalTermId>,
        ids: (
            impl Fn(D::Id) -> u64 + Copy + 'a,
            impl Fn(u64) -> Result<D::Id, SegmentedError>,
        ),
        error: impl Fn(D::ReadError) -> SegmentedError,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + 'a {
        let (index, local) = ids;
        let pattern = self
            .translate_pattern(dataset, origin, pattern, local, error)
            .or_else(|error| self.operation.fail(error))
            .ok()
            .flatten();
        pattern.into_iter().flat_map(rows).map_while(move |row| {
            let translate = || {
                Ok(QuadIds {
                    s: self.intern_source_term(origin, index(row.s))?,
                    p: self.intern_source_term(origin, index(row.p))?,
                    o: self.intern_source_term(origin, index(row.o))?,
                    g: row
                        .g
                        .map(|id| self.intern_source_term(origin, index(id)))
                        .transpose()?,
                })
            };
            translate().or_else(|error| self.operation.fail(error)).ok()
        })
    }

    fn physical_rows(
        &self,
        origin: SegmentedStackSource,
        stream: Stream,
        pattern: ProbePattern<GlobalTermId>,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        self.physical_rows_range(origin, stream, pattern, None)
    }

    fn physical_rows_range<'a>(
        &'a self,
        origin: SegmentedStackSource,
        stream: Stream,
        pattern: ProbePattern<GlobalTermId>,
        values: Option<&'a crate::AnnotationValueRange<'a>>,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + 'a {
        let session = match origin {
            SegmentedStackSource::Layer(index) => self.sessions.get(index),
            _ => None,
        };
        session
            .into_iter()
            // The authenticated table count decides emptiness before a bound
            // pattern needs any dictionary or reverse-index reads.
            .filter(move |source| {
                self.operation.ready().is_ok() && source.has_stream_rows(stream as usize)
            })
            .flat_map(move |source| {
                self.rows_from(
                    source,
                    origin,
                    move |pattern| {
                        source.stream_range(
                            stream as usize,
                            (pattern.s, pattern.p, pattern.o, pattern.g),
                            values,
                        )
                    },
                    pattern,
                    (GlobalTermId::index, |index| {
                        GlobalTermId::from_index(index)
                            .map_err(|error| StorageError::from(error).into())
                    }),
                    |error| error,
                )
            })
            .chain(
                self.resident(origin)
                    .into_iter()
                    .filter(move |source| {
                        self.operation.ready().is_ok()
                            && match stream {
                                Stream::Ordinary => source.quad_count() != 0,
                                Stream::Reifier => source.reifiers().next().is_some(),
                                Stream::Annotation => source.annotations().next().is_some(),
                            }
                    })
                    .flat_map(move |source| {
                        self.rows_from(
                            source,
                            origin,
                            move |pattern| {
                                let dataset = source;
                                let matches = move |row: &QuadIds| {
                                    pattern.s.is_none_or(|id| row.s == id)
                                        && pattern.p.is_none_or(|id| row.p == id)
                                        && pattern.o.is_none_or(|id| row.o == id)
                                        && pattern.g.matches(row.g)
                                };
                                (stream == Stream::Ordinary)
                                    .then_some(())
                                    .into_iter()
                                    .flat_map(move |()| {
                                        dataset.quads_for_pattern(
                                            pattern.s, pattern.p, pattern.o, pattern.g,
                                        )
                                    })
                                    .chain(
                                        (stream == Stream::Reifier)
                                            .then_some(())
                                            .into_iter()
                                            .flat_map(move |()| {
                                                pattern
                                                    .s
                                                    .into_iter()
                                                    .flat_map(move |subject| {
                                                        dataset.reifier_quads_of(subject)
                                                    })
                                                    .chain(
                                                        pattern
                                                            .s
                                                            .is_none()
                                                            .then_some(())
                                                            .into_iter()
                                                            .flat_map(move |()| {
                                                                dataset.reifier_quads_in_graph(
                                                                    pattern.g,
                                                                )
                                                            }),
                                                    )
                                                    .filter(matches)
                                            }),
                                    )
                                    .chain(
                                        (stream == Stream::Annotation)
                                            .then_some(())
                                            .into_iter()
                                            .flat_map(move |()| {
                                                pattern
                                                    .s
                                                    .into_iter()
                                                    .flat_map(move |subject| {
                                                        dataset
                                                            .annotations_of_with_graph(subject)
                                                            .map(move |(p, o, g)| QuadIds {
                                                                s: subject,
                                                                p,
                                                                o,
                                                                g,
                                                            })
                                                    })
                                                    .chain(
                                                        pattern
                                                            .s
                                                            .is_none()
                                                            .then_some(())
                                                            .into_iter()
                                                            .flat_map(move |()| {
                                                                dataset.annotation_quads_in_graph(
                                                                    pattern.g,
                                                                )
                                                            }),
                                                    )
                                                    .filter(matches)
                                            }),
                                    )
                            },
                            pattern,
                            (local_index, |index| {
                                let index = u32::try_from(index)
                                    .map_err(|_| SegmentedError::AddressExhausted)?;
                                TermId::from_index(index)
                                    .map_err(|error| StorageError::from(error).into())
                            }),
                            |error| match error {},
                        )
                    }),
            )
    }

    fn visible_at(&self, layer: usize, row: QuadIds<GlobalTermId>, prefix: usize) -> bool {
        if layer > prefix {
            return false;
        }
        let pattern = ProbePattern {
            s: Some(row.s),
            p: Some(row.p),
            o: Some(row.o),
            g: row.g.map_or(GraphMatch::Default, GraphMatch::Named),
        };
        !(layer + 1..=prefix).any(|newer| {
            let origin = if newer == self.sessions.len() {
                SegmentedStackSource::HeadRemovals
            } else {
                SegmentedStackSource::Removals(newer)
            };
            [Stream::Ordinary, Stream::Reifier, Stream::Annotation]
                .into_iter()
                .any(|stream| self.physical_rows(origin, stream, pattern).next().is_some())
        })
    }

    fn declarations(
        &self,
        subject: GlobalTermId,
        graph: Option<GlobalTermId>,
        prefix: usize,
    ) -> bool {
        let pattern = ProbePattern {
            s: Some(subject),
            p: None,
            o: None,
            g: graph.map_or(GraphMatch::Default, GraphMatch::Named),
        };
        (0..=prefix).any(|layer| {
            self.physical_rows(self.source_at(layer), Stream::Reifier, pattern)
                .any(|row| self.visible_at(layer, row, prefix))
        })
    }

    fn logical_rows(
        &self,
        kind: StatementKind,
        pattern: ProbePattern<GlobalTermId>,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        self.logical_rows_to(Some(kind), pattern, self.source_count())
    }

    fn logical_rows_to(
        &self,
        kind: Option<StatementKind>,
        pattern: ProbePattern<GlobalTermId>,
        sources: usize,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        self.logical_rows_to_range(kind, pattern, sources, None)
    }

    fn logical_rows_to_range<'a>(
        &'a self,
        kind: Option<StatementKind>,
        pattern: ProbePattern<GlobalTermId>,
        sources: usize,
        values: Option<&'a crate::AnnotationValueRange<'a>>,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + 'a {
        let mut seen = Seen::new(&self.operation);
        let end = sources.saturating_sub(1);
        (0..sources)
            .rev()
            .flat_map(move |layer| {
                [Stream::Annotation, Stream::Ordinary]
                    .into_iter()
                    .flat_map(move |stream| {
                        self.physical_rows_range(self.source_at(layer), stream, pattern, values)
                            .map(move |row| (layer, stream, row))
                    })
            })
            .filter_map(move |(layer, stream, row)| {
                let mut select = || {
                    self.operation.ready()?;
                    if !self.visible_at(layer, row, end) {
                        return Ok(None);
                    }
                    let seen = seen.as_mut().map_err(|error| error.clone())?;
                    if !seen.insert(row)? {
                        return Ok(None);
                    }
                    let Some(kind) = kind else {
                        return Ok(Some(row));
                    };
                    let ordinary = stream == Stream::Ordinary;
                    let effective = !ordinary && self.declarations(row.s, row.g, end);
                    let original = !ordinary && self.declarations(row.s, row.g, layer);
                    self.operation.ready()?;
                    let physical = if ordinary {
                        StatementKind::Ordinary
                    } else {
                        StatementKind::Annotation
                    };
                    Ok(
                        (classify_statement(physical, false, original, effective) == kind)
                            .then_some(row),
                    )
                };
                select()
                    .or_else(|error| self.operation.fail(error))
                    .ok()
                    .flatten()
            })
    }

    fn reifiers(
        &self,
        pattern: ProbePattern<GlobalTermId>,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        self.reifiers_to(pattern, self.source_count())
    }

    fn reifiers_to(
        &self,
        pattern: ProbePattern<GlobalTermId>,
        sources: usize,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        let mut seen = Seen::new(&self.operation);
        let end = sources.saturating_sub(1);
        (0..sources)
            .rev()
            .flat_map(move |layer| {
                self.physical_rows(self.source_at(layer), Stream::Reifier, pattern)
                    .map(move |row| (layer, row))
            })
            .filter_map(move |(layer, row)| {
                let mut select = || {
                    self.operation.ready()?;
                    if !self.visible_at(layer, row, end) {
                        return Ok(None);
                    }
                    self.operation.ready()?;
                    Ok(seen
                        .as_mut()
                        .map_err(|error| error.clone())?
                        .insert(row)?
                        .then_some(row))
                };
                select()
                    .or_else(|error| self.operation.fail(error))
                    .ok()
                    .flatten()
            })
    }

    pub(super) fn source_rows(
        &self,
        family: crate::source::SourceFamily,
        pattern: ProbePattern<GlobalTermId>,
        sources: usize,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        use crate::source::SourceFamily;
        (family == SourceFamily::Statements)
            .then_some(())
            .into_iter()
            .flat_map(move |()| self.logical_rows_to(None, pattern, sources))
            .chain(
                (family == SourceFamily::Reifiers)
                    .then_some(())
                    .into_iter()
                    .flat_map(move |()| self.reifiers_to(pattern, sources)),
            )
    }

    pub(super) fn physical_source_rows(
        &self,
        origin: SegmentedStackSource,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        [Stream::Ordinary, Stream::Reifier, Stream::Annotation]
            .into_iter()
            .flat_map(move |stream| {
                self.physical_rows(
                    origin,
                    stream,
                    ProbePattern {
                        s: None,
                        p: None,
                        o: None,
                        g: GraphMatch::Any,
                    },
                )
            })
    }
}

impl SegmentedStackQuery<'_> {
    fn declaration_source(&self, layer: usize) -> SegmentedStackSource {
        if layer == self.sessions.len() {
            SegmentedStackSource::HeadDeclarations
        } else {
            SegmentedStackSource::Declarations(layer)
        }
    }

    fn declared_graphs(&self) -> impl Iterator<Item = GlobalTermId> + '_ {
        (0..self.source_count()).flat_map(move |layer| {
            let origin = self.declaration_source(layer);
            self.resident(origin).into_iter().flat_map(move |source| {
                source
                    .named_graphs()
                    .map_while(move |id| self.intern_source_term(origin, local_index(id)).ok())
            })
        })
    }
}

impl DatasetView for SegmentedStackQuery<'_> {
    type Id = GlobalTermId;
    type ReadError = SegmentedError;
    type TermGuard<'a>
        = SegmentedStackTermGuard<'a>
    where
        Self: 'a;
    type ProbePlan = ();

    fn snapshot_identity(&self) -> Result<Option<purrdf_hash::hex::Digest32>, Self::ReadError> {
        self.check_ready()?;
        Ok(Some(self.snapshot.content_identity()))
    }

    fn source_version(&self) -> Result<Option<crate::source::SourceVersion>, Self::ReadError> {
        self.capture_source_version()
            .map(Some)
            .or_else(|error| self.operation.fail(error))
    }

    fn visit_source_changes<B>(
        &self,
        previous: &crate::source::SourceVersion,
        visit: impl FnMut(
            crate::source::SourceSide,
            crate::source::SourceFamily,
            crate::source::SourceRowRef<'_>,
        ) -> std::ops::ControlFlow<B>,
    ) -> Result<crate::source::SourceVisit<B>, Self::ReadError> {
        self.observed_source_changes(previous, visit)
            .or_else(|error| self.operation.fail(error))
    }

    fn statistics(&self) -> Option<&dyn crate::SnapshotStatistics<Self::Id>> {
        self.check_ready().ok().map(|()| self as _)
    }

    fn read_error(&self) -> Option<Self::ReadError> {
        self.check_ready().err()
    }

    fn storage_live_budget(&self) -> Option<u64> {
        Some(self.operation.live_limit())
    }

    fn max_owned_term_bytes(&self) -> Option<u64> {
        let bound = self.sessions.iter().try_fold(0, |widest: u64, session| {
            session
                .max_owned_term_bytes()
                .map(|bound| widest.max(bound))
        })?;
        let bound = self.snapshot.layers.iter().fold(bound, |widest, layer| {
            widest.max(layer.metadata.max_owned_term_bytes())
        });
        Some(
            self.snapshot
                .head
                .as_ref()
                .map_or(bound, |head| bound.max(head.max_owned_term_bytes)),
        )
    }

    fn triple_term_nesting_bound(&self) -> Option<usize> {
        let metadata = self
            .snapshot
            .layers
            .iter()
            .map(|layer| &layer.metadata)
            .chain(self.snapshot.head.iter().map(|head| &head.metadata));
        let resident_bounds = metadata
            .flat_map(|metadata| {
                metadata
                    .removals()
                    .into_iter()
                    .chain(metadata.declarations())
            })
            .map(DatasetView::triple_term_nesting_bound)
            .chain(
                self.snapshot
                    .head
                    .iter()
                    .map(|head| head.dataset.triple_term_nesting_bound()),
            );
        crate::dataset_view::widest_nesting_bound(
            self.sessions
                .iter()
                .map(DatasetView::triple_term_nesting_bound)
                .chain(resident_bounds),
        )
    }

    fn reserve_owned_workspace(
        &self,
        bytes: u64,
    ) -> Option<Result<crate::OwnedWorkspaceReservation<Self::ReadError>, Self::ReadError>> {
        Some(self.operation.reserve_owned_workspace(bytes))
    }

    fn reserve_workspace(
        &self,
        bytes: u64,
    ) -> Result<impl WorkspaceReservation<Error = Self::ReadError> + '_, Self::ReadError> {
        self.operation.reserve_workspace(bytes)
    }

    fn resolve(&self, id: Self::Id) -> Result<Self::TermGuard<'_>, Self::ReadError> {
        let resolve = || {
            self.operation.ready()?;
            let term = {
                let ids = lock_read_state(&self.identities);
                usize::try_from(id.index())
                    .ok()
                    .and_then(|index| ids.representatives.get(index))
                    .map(|entry| entry.term)
            }
            .ok_or(SegmentedError::Corrupt("foreign query term ID"))?;
            let source = match term.source {
                SegmentedStackSource::Layer(index) => {
                    SourceGuard::Sealed(self.sessions[index].resolve(
                        GlobalTermId::from_index(term.index).map_err(StorageError::from)?,
                    )?)
                }
                other => {
                    let dataset = self
                        .resident(other)
                        .ok_or(SegmentedError::Corrupt("missing resident source"))?;
                    let local = TermId::from_index(
                        u32::try_from(term.index).map_err(|_| SegmentedError::AddressExhausted)?,
                    )
                    .map_err(StorageError::from)?;
                    SourceGuard::Resident(dataset.resolve(local))
                }
            };
            let mut children = [id; 3];
            let mut position = 0;
            let mut failure = None;
            source.term().map_ids(|index| {
                if failure.is_none() {
                    match self.intern_source_term(term.source, index) {
                        Ok(child) => children[position] = child,
                        Err(error) => failure = Some(error),
                    }
                }
                position += 1;
            });
            if let Some(error) = failure {
                return Err(error);
            }
            Ok(SegmentedStackTermGuard { source, children })
        };
        resolve().or_else(|error| self.operation.fail(error))
    }

    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<Self::Id>, Self::ReadError> {
        let lookup = || {
            self.operation.ready()?;
            if let Some(id) = self.find_touched(value)? {
                return Ok(Some(id));
            }
            for (index, source) in self.sessions.iter().enumerate() {
                if let Some(id) = source.term_id_by_value(value)? {
                    return self
                        .intern_source_term(SegmentedStackSource::Layer(index), id.index())
                        .map(Some);
                }
            }
            if let Some(head) = self.resident(SegmentedStackSource::Head)
                && let Some(id) = head.term_id_by_value(value)
            {
                return self
                    .intern_source_term(SegmentedStackSource::Head, local_index(id))
                    .map(Some);
            }
            for layer in 0..self.source_count() {
                let origin = self.declaration_source(layer);
                if let Some(source) = self.resident(origin)
                    && let Some(id) = source.term_id_by_value(value)
                {
                    return self.intern_source_term(origin, local_index(id)).map(Some);
                }
            }
            Ok(None)
        };
        lookup().or_else(|error| self.operation.fail(error))
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        let mut caps = self
            .sessions
            .iter()
            .fold(RdfStoreCapabilities::plain_rdf(), |caps, source| {
                caps.union(source.capabilities())
            });
        if let Some(head) = self.resident(SegmentedStackSource::Head) {
            caps = caps.union(head.capabilities());
        }
        caps.annotations |= caps.reifiers;
        caps.named_graphs |= (0..self.source_count()).any(|layer| {
            self.resident(self.declaration_source(layer))
                .is_some_and(|source| source.named_graphs().next().is_some())
        });
        caps
    }

    fn term_count(&self) -> u64 {
        self.touched_terms() as u64
    }

    fn quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.quads_for_pattern(None, None, None, GraphMatch::Any)
    }

    fn quads_for_pattern(
        &self,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.logical_rows(StatementKind::Ordinary, ProbePattern { s, p, o, g })
    }

    fn probe_plan(&self, _: bool, _: bool, _: bool, _: GraphMatch<Self::Id>) {}

    fn quads_for_pattern_with_plan(
        &self,
        (): &(),
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.quads_for_pattern(s, p, o, g)
    }

    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.reifier_quads_in_graph(GraphMatch::Any)
    }

    fn reifier_quads_of(&self, reifier: Self::Id) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.reifiers(ProbePattern {
            s: Some(reifier),
            p: None,
            o: None,
            g: GraphMatch::Any,
        })
    }

    fn reifier_quads_in_graph(
        &self,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.reifiers(ProbePattern {
            s: None,
            p: None,
            o: None,
            g,
        })
    }

    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.annotation_quads_in_graph(GraphMatch::Any)
    }

    fn annotations_of_with_graph(
        &self,
        reifier: Self::Id,
    ) -> impl Iterator<Item = (Self::Id, Self::Id, Option<Self::Id>)> + '_ {
        self.logical_rows(
            StatementKind::Annotation,
            ProbePattern {
                s: Some(reifier),
                p: None,
                o: None,
                g: GraphMatch::Any,
            },
        )
        .map(|row| (row.p, row.o, row.g))
    }

    fn annotation_quads_in_graph(
        &self,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.logical_rows(
            StatementKind::Annotation,
            ProbePattern {
                s: None,
                p: None,
                o: None,
                g,
            },
        )
    }

    fn annotation_quads_for_range<'a>(
        &'a self,
        predicate: Option<Self::Id>,
        object: Option<Self::Id>,
        graph: GraphMatch<Self::Id>,
        range: &'a crate::AnnotationValueRange<'a>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + 'a {
        self.logical_rows_to_range(
            Some(StatementKind::Annotation),
            ProbePattern {
                s: None,
                p: predicate,
                o: object,
                g: graph,
            },
            self.source_count(),
            Some(range),
        )
    }

    fn named_graphs(&self) -> impl Iterator<Item = Self::Id> + '_ {
        let mut seen = Seen::new(&self.operation);
        self.declared_graphs()
            .chain(
                self.quads()
                    .chain(self.reifier_quads())
                    .chain(self.annotation_quads())
                    .filter_map(|row| row.g),
            )
            .filter_map(move |id| {
                let mut insert = || {
                    self.operation.ready()?;
                    Ok(seen
                        .as_mut()
                        .map_err(|error| error.clone())?
                        .insert(id)?
                        .then_some(id))
                };
                insert()
                    .or_else(|error| self.operation.fail(error))
                    .ok()
                    .flatten()
            })
    }
}

impl FallibleDatasetView for SegmentedStackQuery<'_> {
    type Error = SegmentedError;
    type Evidence = super::super::SegmentedEvidence<SegmentedStackSnapshot>;

    fn operation_status(&self) -> ViewOperationStatus<Self::Error, Self::Evidence> {
        let result = self.check_ready();
        let evidence = self.operation.evidence(self.snapshot.clone());
        match result {
            Ok(()) => ViewOperationStatus::Ready { evidence },
            Err(error) => ViewOperationStatus::Failed { error, evidence },
        }
    }
}
