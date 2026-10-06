// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Chronological sealed layers, a mutable value head, and owned guarded snapshots.

use std::collections::BTreeSet;
use std::convert::Infallible;
use std::num::NonZeroUsize;
use std::sync::{Arc, OnceLock};

use super::summary::PageStream;
use super::{
    InMemoryPageProvider, PageFault, PageGeneration, PageId, PageMaterialization, PageProvider,
    PagedDataset, PagedFreezeError, PagedQueryError, PagedQueryEvidence, PagedQueryLimits,
    PagedQueryView,
};
use crate::ir::mutable::{StatementKind, classify_statement};
use crate::{
    DatasetMut, DatasetView, FallibleDatasetView, FastSet, GlobalDictionary, GlobalTermId,
    GraphMatch, MutableDataset, PackBuilder, PackError, QuadIds, QuadValues, RdfDataset,
    RdfDatasetBuilder, RdfDiagnostic, RdfStoreCapabilities, TermFactory, TermRef, TermValue,
    ViewOperationStatus,
};

/// A pinned sealed source, in chronological order (oldest first).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StackSource {
    /// Its original provider generation, not a composed hash.
    pub generation: PageGeneration,
    /// Its original dense page count, including zero-page layers.
    pub page_count: u64,
}

/// The source-qualified address of a physical page request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackPageOrigin {
    /// A sealed layer and its original dense page ordinal.
    Sealed {
        /// Chronological layer ordinal.
        layer: u64,
        /// Page ordinal within that layer.
        page: PageId,
    },
    /// The snapshot's frozen resident head, charged like one reference page.
    Head,
}

/// Exact immutable stack identity and aggregate operation evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PagedStackEvidence {
    /// Every sealed descriptor, including sources that no probe touches.
    pub sources: Arc<[StackSource]>,
    /// Head value rows pinned by this snapshot.
    pub head: Arc<[QuadValues]>,
    /// Graph declarations made in the head.
    pub head_graphs: Arc<[TermValue]>,
    /// Explicit declarations retained across head seals, one set per sealed layer
    /// followed by the head; physical declaration-only graphs are in the source seal.
    pub graph_declarations: Arc<[Arc<[TermValue]>]>,
    /// One removal set for each sealed layer, followed by the head's set.
    pub removals: Arc<[Arc<[QuadValues]>]>,
    /// The shared page/byte receipt, with physical dense routing page IDs.
    pub pages: PagedQueryEvidence,
    /// Qualified addresses in precisely the receipt's first-request order.
    pub requested_origins: Vec<StackPageOrigin>,
    /// Reference byte charge of the head page (zero if no head page exists).
    pub head_bytes: u64,
}

/// A construction or mutable membership failure; absence never substitutes for error.
#[derive(Debug)]
pub enum PagedStackError {
    /// Invalid RDF row or head declaration.
    Invalid(RdfDiagnostic),
    /// Sealed metadata construction failed.
    Freeze(PagedFreezeError),
    /// Effective membership could not be established.
    Read(PagedQueryError),
    /// The deterministic head seal failed.
    Seal(CanonicalPagedError<Infallible>),
    /// A logical count or resident allocation cannot be represented.
    Capacity,
    /// At least one sealed base is required.
    MissingBase,
    /// Publishing an external delta would reorder pending head mutations.
    PendingHead,
}
impl std::fmt::Display for PagedStackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(e) => write!(f, "invalid stack head: {e}"),
            Self::Freeze(e) => write!(f, "stack metadata construction: {e}"),
            Self::Read(e) => write!(f, "stack membership: {e}"),
            Self::Seal(e) => write!(f, "stack head seal: {e}"),
            Self::Capacity => f.write_str("stack exceeds resident address capacity"),
            Self::MissingBase => f.write_str("attach at least one sealed base generation"),
            Self::PendingHead => f.write_str("seal the pending head before appending a generation"),
        }
    }
}
impl std::error::Error for PagedStackError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Invalid(error) => Some(error),
            Self::Freeze(error) => Some(error),
            Self::Read(error) => Some(error),
            Self::Seal(error) => Some(error),
            Self::Capacity | Self::MissingBase | Self::PendingHead => None,
        }
    }
}

#[derive(Debug)]
struct Layer {
    dataset: Arc<PagedDataset>,
    removals: BTreeSet<QuadValues>,
    declared_graphs: BTreeSet<TermValue>,
}

/// A consumer-owned chronological stack with one mutable in-memory value head.
///
/// The stack provides no persistence or automatic compaction policy. Snapshot
/// publication never reads old page contents. Reuse one published snapshot for
/// multiple operations: rebuilding its dictionary/translations costs linear work
/// in all retained dictionary values and translation entries, not only the delta.
#[derive(Debug)]
pub struct PagedStack {
    layers: Vec<Layer>,
    head: BTreeSet<QuadValues>,
    removals: BTreeSet<QuadValues>,
    graphs: BTreeSet<TermValue>,
    sealed_snapshot: OnceLock<PagedStackSnapshot>,
}

impl PagedStack {
    /// Attach one or more independently sealed bases, oldest first. Repeated
    /// facts across bases are effective set members, not a violation of in-layer G3.
    ///
    /// # Errors
    /// Refuses an empty base list or a provider whose sealed descriptor has drifted.
    pub fn new(bases: Vec<Arc<PagedDataset>>) -> Result<Self, PagedStackError> {
        if bases.is_empty() {
            return Err(PagedStackError::MissingBase);
        }
        let stack = Self {
            layers: bases
                .into_iter()
                .map(|dataset| Layer {
                    dataset,
                    removals: BTreeSet::new(),
                    declared_graphs: BTreeSet::new(),
                })
                .collect(),
            head: BTreeSet::new(),
            removals: BTreeSet::new(),
            graphs: BTreeSet::new(),
            sealed_snapshot: OnceLock::new(),
        };
        stack.check_sources()?;
        Ok(stack)
    }

    fn check_sources(&self) -> Result<(), PagedStackError> {
        for (source, layer) in self.layers.iter().enumerate() {
            layer
                .dataset
                .provider
                .check_snapshot(layer.dataset.generation(), layer.dataset.page_count())
                .map_err(|error| {
                    PagedStackError::Read(PagedQueryError::SourceSnapshot {
                        source: source as u64,
                        error: Box::new(error),
                    })
                })?;
        }
        Ok(())
    }

    /// Append a sealed generation with value removals that suppress older layers.
    /// Additions in this generation win over its own removals. A nonempty head
    /// must first be sealed, so appending cannot reorder already accepted mutations.
    ///
    /// # Errors
    /// Refuses pending head changes, source drift, or invalid removal row values.
    pub fn append(
        &mut self,
        dataset: Arc<PagedDataset>,
        removals: Vec<QuadValues>,
    ) -> Result<(), PagedStackError> {
        self.check_sources()?;
        if !self.head.is_empty() || !self.removals.is_empty() || !self.graphs.is_empty() {
            return Err(PagedStackError::PendingHead);
        }
        dataset
            .provider
            .check_snapshot(dataset.generation(), dataset.page_count())
            .map_err(PagedStackError::Read)?;
        let mut normalized = BTreeSet::new();
        for row in removals {
            normalized.insert(normalize_row(&row)?);
        }
        self.layers
            .try_reserve(1)
            .map_err(|_| PagedStackError::Capacity)?;
        self.layers.push(Layer {
            dataset,
            removals: normalized,
            declared_graphs: BTreeSet::new(),
        });
        self.sealed_snapshot.take();
        Ok(())
    }

    /// Test effective RDF-surface membership with a typed operational result.
    ///
    /// # Errors
    /// Returns source/admission failures instead of an absent boolean.
    pub fn contains(&self, row: &QuadValues) -> Result<bool, PagedStackError> {
        self.contains_with_limits(row, PagedQueryLimits::UNBOUNDED)
    }

    /// Membership under an explicit whole-operation I/O allowance. Head presence
    /// and removals are checked before lower pages. Sealed routing metadata is reused
    /// throughout a mutable batch, but each membership operation owns a fresh cache.
    ///
    /// # Errors
    /// Invalid values or source/resource faults prevent a membership answer.
    pub fn contains_with_limits(
        &self,
        row: &QuadValues,
        limits: PagedQueryLimits,
    ) -> Result<bool, PagedStackError> {
        self.check_sources()?;
        let row = normalize_row(row)?;
        if self.head.contains(&row) {
            return Ok(true);
        }
        if self.removals.contains(&row) {
            return Ok(false);
        }
        if self.sealed_snapshot.get().is_none() {
            let snapshot = self.snapshot_with_head(false)?;
            let _ = self.sealed_snapshot.set(snapshot);
        }
        let snapshot = self
            .sealed_snapshot
            .get()
            .expect("sealed routing metadata initialized");
        let view = snapshot.query_view(limits);
        let found = view.contains_values(&row).map_err(PagedStackError::Read)?;
        if let Some(error) = view.read_error() {
            return Err(PagedStackError::Read(error));
        }
        Ok(found)
    }

    /// Insert by RDF value. An insertion above a previous removal survives it.
    ///
    /// # Errors
    /// Invalid RDF or a failed lower membership read leaves the head unchanged.
    // Match the value-consuming mutation boundary; canonicalization replaces the
    // supplied representation before ownership enters the head.
    #[allow(clippy::needless_pass_by_value)]
    pub fn insert(&mut self, row: QuadValues) -> Result<bool, PagedStackError> {
        let row = normalize_row(&row)?;
        if self.contains(&row)? {
            return Ok(false);
        }
        self.removals.remove(&row);
        self.head.insert(row);
        Ok(true)
    }

    /// Remove from the effective surface, including every older copy.
    ///
    /// # Errors
    /// Failed membership reads leave both the head and removal set unchanged.
    pub fn remove(&mut self, row: &QuadValues) -> Result<bool, PagedStackError> {
        let row = normalize_row(row)?;
        if !self.contains(&row)? {
            return Ok(false);
        }
        self.head.remove(&row);
        self.removals.insert(row);
        Ok(true)
    }

    /// Declare a graph independently of its rows; declaration-only graphs survive
    /// removals and folds. Only IRI or scoped blank graph names are valid.
    ///
    /// # Errors
    /// Returns invalid RDF or source drift without changing the head.
    // Declarations share the value-consuming writer boundary with insertion.
    #[allow(clippy::needless_pass_by_value)]
    pub fn declare_named_graph(&mut self, graph: TermValue) -> Result<bool, PagedStackError> {
        self.check_sources()?;
        let mut builder = RdfDatasetBuilder::new();
        let id = builder.intern_value(&graph);
        builder.declare_named_graph(id);
        let frozen = builder.freeze().map_err(PagedStackError::Invalid)?;
        Ok(self.graphs.insert(frozen.as_ref().term_value(id)))
    }

    fn frozen_head(&self) -> Result<Arc<RdfDataset>, PagedStackError> {
        let mut mutation = MutableDataset::new(
            RdfDatasetBuilder::new()
                .freeze()
                .map_err(PagedStackError::Invalid)?,
        );
        for row in &self.head {
            mutation.insert(row.clone()).map_err(|e| {
                PagedStackError::Read(PagedQueryError::InvalidData {
                    page: PageId(0),
                    message: e.to_string(),
                })
            })?;
        }
        let head = mutation.freeze().map_err(PagedStackError::Invalid)?;
        if self.graphs.is_empty() {
            return Ok(head);
        }
        let mut builder = RdfDatasetBuilder::new();
        crate::ir::import::DatasetImporter::new(&mut builder, head.as_ref()).append();
        for graph in &self.graphs {
            let id = builder.intern_value(graph);
            builder.declare_named_graph(id);
        }
        builder.freeze().map_err(PagedStackError::Invalid)
    }

    /// Pin an immutable owned snapshot without materializing a lower page.
    ///
    /// # Errors
    /// Refuses invalid head data, descriptor drift, or metadata count exhaustion.
    pub fn snapshot(&self) -> Result<PagedStackSnapshot, PagedStackError> {
        self.snapshot_with_head(true)
    }

    fn snapshot_with_head(
        &self,
        include_head: bool,
    ) -> Result<PagedStackSnapshot, PagedStackError> {
        self.check_sources()?;
        let has_head = include_head && (!self.head.is_empty() || !self.graphs.is_empty());
        let mut sources: Vec<Arc<PagedDataset>> = self
            .layers
            .iter()
            .map(|layer| Arc::clone(&layer.dataset))
            .collect();
        let mut head_bytes = 0;
        if has_head {
            let head = PageMaterialization::in_memory(self.frozen_head()?, PageGeneration::INITIAL);
            head_bytes = head.byte_len;
            sources.push(Arc::new(
                PagedDataset::from_provider(Arc::new(InMemoryPageProvider::with_byte_lengths(
                    vec![(head.dataset, head_bytes)],
                    PageGeneration::INITIAL,
                )))
                .map_err(PagedStackError::Freeze)?,
            ));
        }
        let descriptors: Arc<[StackSource]> = self
            .layers
            .iter()
            .map(|layer| StackSource {
                generation: layer.dataset.generation(),
                page_count: layer.dataset.page_count(),
            })
            .collect();
        let mut dictionary = GlobalDictionary::new();
        let mut parts = Vec::new();
        let mut origins = Vec::new();
        let mut empty_graphs = BTreeSet::new();
        let mut ranges = Vec::new();
        for (layer, source) in sources.iter().enumerate() {
            let first = parts.len();
            let mut remap = Vec::new();
            remap
                .try_reserve(source.dictionary.len())
                .map_err(|_| PagedStackError::Capacity)?;
            for i in 0..source.dictionary.len() {
                let value = source
                    .dictionary
                    .term_value(GlobalTermId::from_index(i as u64));
                remap.push(
                    dictionary
                        .try_reintern_validated(&value)
                        .map_err(|()| PagedStackError::Capacity)?,
                );
            }
            for slot in &source.pages {
                let translation = slot.translation.remap(|id| remap[id.index() as usize]);
                for graph in translation.summary().declared_graphs() {
                    if [
                        PageStream::Base,
                        PageStream::Reifier,
                        PageStream::Annotation,
                    ]
                    .iter()
                    .all(|&stream| translation.summary().graph_rows(graph, stream) == 0)
                    {
                        empty_graphs.insert(translation.to_global(graph));
                    }
                }
                parts
                    .try_reserve(1)
                    .map_err(|_| PagedStackError::Capacity)?;
                parts.push(super::PagePart {
                    translation,
                    capabilities: slot.caps,
                    quad_count: slot.quad_count,
                    byte_len: slot.byte_len,
                });
                origins.push(if layer == self.layers.len() {
                    StackPageOrigin::Head
                } else {
                    StackPageOrigin::Sealed {
                        layer: layer as u64,
                        page: slot.id,
                    }
                });
            }
            ranges.push(first..parts.len());
        }
        let empty_removals = BTreeSet::new();
        let empty_graph_values = BTreeSet::new();
        let graph_declarations: Arc<[Arc<[TermValue]>]> = self
            .layers
            .iter()
            .map(|layer| &layer.declared_graphs)
            .chain(std::iter::once(if include_head {
                &self.graphs
            } else {
                &empty_graph_values
            }))
            .map(|graphs| graphs.iter().cloned().collect::<Arc<[_]>>())
            .collect();
        for graph in self
            .layers
            .iter()
            .flat_map(|layer| &layer.declared_graphs)
            .chain(self.graphs.iter().filter(|_| include_head))
        {
            empty_graphs.insert(
                dictionary
                    .try_reintern_validated(graph)
                    .map_err(|()| PagedStackError::Capacity)?,
            );
        }
        let head_removals = if include_head {
            &self.removals
        } else {
            &empty_removals
        };
        let removal_values: Arc<[Arc<[QuadValues]>]> = self
            .layers
            .iter()
            .map(|layer| &layer.removals)
            .chain(std::iter::once(head_removals))
            .map(|set| set.iter().cloned().collect::<Arc<[_]>>())
            .collect();
        let removals: Vec<FastSet<QuadIds<GlobalTermId>>> = removal_values
            .iter()
            .map(|set| {
                set.iter()
                    .map(|row| {
                        Ok(QuadIds {
                            s: dictionary
                                .try_reintern_validated(&row.s)
                                .map_err(|()| PagedStackError::Capacity)?,
                            p: dictionary
                                .try_reintern_validated(&row.p)
                                .map_err(|()| PagedStackError::Capacity)?,
                            o: dictionary
                                .try_reintern_validated(&row.o)
                                .map_err(|()| PagedStackError::Capacity)?,
                            g: row
                                .g
                                .as_ref()
                                .map(|g| {
                                    dictionary
                                        .try_reintern_validated(g)
                                        .map_err(|()| PagedStackError::Capacity)
                                })
                                .transpose()?,
                        })
                    })
                    .collect::<Result<_, PagedStackError>>()
            })
            .collect::<Result<_, _>>()?;
        let provider = Arc::new(StackProvider {
            sources,
            origins: origins.clone(),
        });
        let physical =
            PagedDataset::from_parts(dictionary, provider, PageGeneration::INITIAL, parts)
                .map_err(PagedStackError::Freeze)?;
        let snapshot = PagedStackSnapshot {
            physical,
            origins,
            removals,
            empty_graphs,
            ranges,
            sources: descriptors,
            head: if include_head {
                self.head.iter().cloned().collect()
            } else {
                Arc::from([])
            },
            head_graphs: if include_head {
                self.graphs.iter().cloned().collect()
            } else {
                Arc::from([])
            },
            removal_values,
            graph_declarations,
            head_bytes,
        };
        self.check_sources()?;
        Ok(snapshot)
    }

    /// Seal only the mutable batch and publish a fresh empty head. Even a
    /// removal-only batch adds a zero-page layer that retains its ordered tombstones.
    ///
    /// # Errors
    /// Source drift or head encoding failures publish no layer and retain the head.
    pub fn seal_head(&mut self, page_rows: NonZeroUsize) -> Result<(), PagedStackError> {
        self.check_sources()?;
        let dataset = canonical_paged_seal(self.frozen_head()?.as_ref(), page_rows)
            .map_err(PagedStackError::Seal)?;
        self.check_sources()?;
        self.layers
            .try_reserve(1)
            .map_err(|_| PagedStackError::Capacity)?;
        self.layers.push(Layer {
            dataset: Arc::new(dataset),
            removals: std::mem::take(&mut self.removals),
            declared_graphs: std::mem::take(&mut self.graphs),
        });
        self.head.clear();
        self.graphs.clear();
        self.sealed_snapshot.take();
        Ok(())
    }

    /// Fold the effective surface into a new canonical base; publication belongs
    /// to the consumer. This explicitly paid operation reads surviving candidates.
    ///
    /// # Errors
    /// A failed source, limit, or encoder returns no partial artifact.
    pub fn compact(
        &self,
        limits: PagedQueryLimits,
        page_rows: NonZeroUsize,
    ) -> Result<PagedDataset, CanonicalPagedError<PagedQueryError>> {
        let snapshot = self
            .snapshot()
            .map_err(|error| CanonicalPagedError::Construction(Box::new(error)))?;
        snapshot.compact(limits, page_rows)
    }
}

fn normalize_row(row: &QuadValues) -> Result<QuadValues, PagedStackError> {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_value(&row.s);
    let p = builder.intern_value(&row.p);
    let o = builder.intern_value(&row.o);
    let g = row.g.as_ref().map(|g| builder.intern_value(g));
    builder.push_quad(s, p, o, g);
    let frozen = builder.freeze().map_err(PagedStackError::Invalid)?;
    Ok(QuadValues {
        s: frozen.as_ref().term_value(s),
        p: frozen.as_ref().term_value(p),
        o: frozen.as_ref().term_value(o),
        g: g.map(|g| frozen.as_ref().term_value(g)),
    })
}

struct StackProvider {
    sources: Vec<Arc<PagedDataset>>,
    origins: Vec<StackPageOrigin>,
}
impl PageProvider for StackProvider {
    fn page_count(&self) -> u64 {
        self.origins.len() as u64
    }
    fn generation(&self) -> PageGeneration {
        PageGeneration::INITIAL
    }
    fn check_snapshot(&self, _: PageGeneration, _: u64) -> Result<(), PagedQueryError> {
        for (source, dataset) in self.sources.iter().enumerate() {
            dataset
                .provider
                .check_snapshot(dataset.generation(), dataset.page_count())
                .map_err(|error| PagedQueryError::SourceSnapshot {
                    source: source as u64,
                    error: Box::new(error),
                })?;
        }
        Ok(())
    }
    fn materialize(&self, page: PageId) -> Result<PageMaterialization, PageFault> {
        let origin = self
            .origins
            .get(page.0 as usize)
            .ok_or_else(|| PageFault::invalid_data(page, "unknown stack page"))?;
        let (layer, original) = match *origin {
            StackPageOrigin::Sealed { layer, page } => (layer as usize, page),
            StackPageOrigin::Head => (self.sources.len() - 1, PageId(0)),
        };
        let source = &self.sources[layer];
        let mut materialization = source.provider.materialize(original).map_err(|mut fault| {
            fault.message = format!(
                "source {layer}, original page {}: {}",
                original.0, fault.message
            );
            fault.page = page;
            fault
        })?;
        if materialization.generation != source.generation() {
            return Err(PageFault::stale_generation(
                page,
                source.generation(),
                materialization.generation,
            ));
        }
        materialization.generation = PageGeneration::INITIAL;
        Ok(materialization)
    }
}

/// Immutable metadata and copied head/removals, retained independently of its writer.
#[derive(Debug)]
pub struct PagedStackSnapshot {
    physical: PagedDataset,
    origins: Vec<StackPageOrigin>,
    removals: Vec<FastSet<QuadIds<GlobalTermId>>>,
    empty_graphs: BTreeSet<GlobalTermId>,
    ranges: Vec<std::ops::Range<usize>>,
    sources: Arc<[StackSource]>,
    head: Arc<[QuadValues]>,
    head_graphs: Arc<[TermValue]>,
    removal_values: Arc<[Arc<[QuadValues]>]>,
    graph_declarations: Arc<[Arc<[TermValue]>]>,
    head_bytes: u64,
}
impl PagedStackSnapshot {
    /// Start one operation sharing a single physical cache, budget and sticky latch.
    #[must_use]
    pub fn query_view(&self, limits: PagedQueryLimits) -> PagedStackQueryView<'_> {
        PagedStackQueryView {
            snapshot: self,
            physical: self.physical.query_view(limits),
        }
    }
    /// The snapshot-local value dictionary; IDs never identify a writer or source.
    #[must_use]
    pub const fn dictionary(&self) -> &GlobalDictionary {
        &self.physical.dictionary
    }
    /// Number of sealed layers, including removal-only generations.
    #[must_use]
    pub fn sealed_depth(&self) -> usize {
        self.sources.len()
    }
    /// Fold through the guarded logical read path and canonical eager partitioner.
    ///
    /// # Errors
    /// Source or resource errors refuse the complete fold.
    pub fn compact(
        &self,
        limits: PagedQueryLimits,
        page_rows: NonZeroUsize,
    ) -> Result<PagedDataset, CanonicalPagedError<PagedQueryError>> {
        canonical_paged_seal(&self.query_view(limits), page_rows)
    }
}

/// The logical, fallible RDF 1.2 view of one complete pinned stack.
#[derive(Debug)]
pub struct PagedStackQueryView<'a> {
    snapshot: &'a PagedStackSnapshot,
    physical: PagedQueryView<'a>,
}
impl PagedStackQueryView<'_> {
    fn layer_of(&self, page: PageId) -> usize {
        match self.snapshot.origins[page.0 as usize] {
            StackPageOrigin::Sealed { layer, .. } => layer as usize,
            StackPageOrigin::Head => self.snapshot.sources.len(),
        }
    }

    fn visible(&self, page: PageId, row: QuadIds<GlobalTermId>) -> bool {
        self.visible_at(page, row, self.snapshot.removals.len() - 1)
    }

    fn visible_at(&self, page: PageId, row: QuadIds<GlobalTermId>, prefix: usize) -> bool {
        let layer = self.layer_of(page);
        if layer > prefix {
            return false;
        }
        !self
            .snapshot
            .removals
            .iter()
            .take(prefix + 1)
            .skip(layer + 1)
            .any(|set| set.contains(&row))
    }

    fn declarations(
        &self,
        subject: GlobalTermId,
        graph: Option<GlobalTermId>,
        prefix: usize,
    ) -> bool {
        let end = self
            .snapshot
            .ranges
            .get(prefix)
            .map_or(self.snapshot.physical.pages.len(), |range| range.end);
        self.physical
            .stream_pattern_range(
                PageStream::Reifier,
                Some(subject),
                None,
                None,
                graph.map_or(GraphMatch::Default, GraphMatch::Named),
                0..end,
            )
            .any(|(page, row)| self.visible_at(page, row, prefix))
    }

    fn external_declaration(
        &self,
        subject: GlobalTermId,
        graph: Option<GlobalTermId>,
        page: PageId,
    ) -> bool {
        let origin = self.snapshot.origins[page.0 as usize];
        self.physical
            .stream_pattern(
                PageStream::Reifier,
                Some(subject),
                None,
                None,
                graph.map_or(GraphMatch::Default, GraphMatch::Named),
            )
            .any(|(other, row)| {
                let other_origin = self.snapshot.origins[other.0 as usize];
                let same_source = match (origin, other_origin) {
                    (
                        StackPageOrigin::Sealed { layer: a, .. },
                        StackPageOrigin::Sealed { layer: b, .. },
                    ) => a == b,
                    (StackPageOrigin::Head, StackPageOrigin::Head) => true,
                    _ => false,
                };
                !same_source && self.visible(other, row)
            })
    }

    fn logical_rows(
        &self,
        kind: StatementKind,
        s: Option<GlobalTermId>,
        p: Option<GlobalTermId>,
        o: Option<GlobalTermId>,
        g: GraphMatch<GlobalTermId>,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        let mut seen = FastSet::default();
        self.snapshot
            .ranges
            .iter()
            .rev()
            .flat_map(move |range| {
                [PageStream::Annotation, PageStream::Base]
                    .into_iter()
                    .flat_map(move |stream| {
                        self.physical
                            .stream_pattern_range(stream, s, p, o, g, range.clone())
                            .map(move |(page, row)| (stream, page, row))
                    })
            })
            .filter_map(move |(stream, page, row)| {
                if self.physical.failed() || !self.visible(page, row) || !seen.insert(row) {
                    return None;
                }
                let effective = if stream == PageStream::Base {
                    self.external_declaration(row.s, row.g, page)
                } else {
                    self.declarations(row.s, row.g, self.snapshot.removals.len() - 1)
                };
                let original = stream == PageStream::Annotation
                    && self.declarations(row.s, row.g, self.layer_of(page));
                let physical = if stream == PageStream::Base {
                    StatementKind::Ordinary
                } else {
                    StatementKind::Annotation
                };
                let classified = classify_statement(physical, true, original, effective);
                (!self.physical.failed() && classified == Some(kind)).then_some(row)
            })
    }

    fn reifiers(
        &self,
        s: Option<GlobalTermId>,
        g: GraphMatch<GlobalTermId>,
    ) -> impl Iterator<Item = QuadIds<GlobalTermId>> + '_ {
        let mut seen = FastSet::default();
        self.physical
            .stream_pattern(PageStream::Reifier, s, None, None, g)
            .filter_map(move |(page, row)| {
                (!self.physical.failed() && self.visible(page, row) && seen.insert(row))
                    .then_some(row)
            })
    }

    fn contains_values(&self, row: &QuadValues) -> Result<bool, PagedQueryError> {
        let dictionary = self.snapshot.dictionary();
        let (Some(s), Some(p), Some(o)) = (
            dictionary.term_id_by_value(&row.s),
            dictionary.term_id_by_value(&row.p),
            dictionary.term_id_by_value(&row.o),
        ) else {
            return Ok(false);
        };
        let graph = match &row.g {
            None => GraphMatch::Default,
            Some(g) => {
                let Some(g) = dictionary.term_id_by_value(g) else {
                    return Ok(false);
                };
                GraphMatch::Named(g)
            }
        };
        let found = self
            .quads_for_pattern(Some(s), Some(p), Some(o), graph)
            .next()
            .is_some()
            || self.reifiers(Some(s), graph).any(|q| q.p == p && q.o == o)
            || self
                .logical_rows(StatementKind::Annotation, Some(s), Some(p), Some(o), graph)
                .next()
                .is_some();
        if let Some(error) = self.read_error() {
            return Err(error);
        }
        Ok(found)
    }
}
impl DatasetView for PagedStackQueryView<'_> {
    type Id = GlobalTermId;
    type ReadError = PagedQueryError;
    type TermGuard<'a>
        = TermRef<'a, GlobalTermId>
    where
        Self: 'a;
    type ProbePlan = ();
    fn read_error(&self) -> Option<Self::ReadError> {
        self.physical.read_error()
    }
    fn resolve(&self, id: Self::Id) -> Result<Self::TermGuard<'_>, Self::ReadError> {
        self.physical.resolve(id)
    }
    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<Self::Id>, Self::ReadError> {
        self.physical.term_id_by_value(value)
    }
    fn capabilities(&self) -> RdfStoreCapabilities {
        let mut caps = self.physical.capabilities();
        caps.annotations |= caps.reifiers;
        caps
    }
    fn len_hint(&self) -> Option<u64> {
        None
    }
    fn term_count(&self) -> u64 {
        self.physical.term_count()
    }
    fn stats_fingerprint(&self) -> u64 {
        self.physical.stats_fingerprint()
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
        self.logical_rows(StatementKind::Ordinary, s, p, o, g)
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
    fn cardinality_estimate(
        &self,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> u64 {
        self.physical
            .stream_estimate(PageStream::Base, s, p, o, g)
            .saturating_add(
                self.physical
                    .stream_estimate(PageStream::Annotation, s, p, o, g),
            )
    }
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.reifiers(None, GraphMatch::Any)
    }
    fn reifier_quads_of(&self, reifier: Self::Id) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.reifiers(Some(reifier), GraphMatch::Any)
    }
    fn reifier_quads_in_graph(
        &self,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.reifiers(None, g)
    }
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.annotation_quads_in_graph(GraphMatch::Any)
    }
    fn annotation_quads_in_graph(
        &self,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.logical_rows(StatementKind::Annotation, None, None, None, g)
    }
    fn annotations_of_with_graph(
        &self,
        reifier: Self::Id,
    ) -> impl Iterator<Item = (Self::Id, Self::Id, Option<Self::Id>)> + '_ {
        self.logical_rows(
            StatementKind::Annotation,
            Some(reifier),
            None,
            None,
            GraphMatch::Any,
        )
        .map(|q| (q.p, q.o, q.g))
    }
    fn named_graphs(&self) -> impl Iterator<Item = Self::Id> + '_ {
        self.physical
            .named_graphs()
            .filter(move |&graph| self.has_named_graph(graph))
    }
    fn has_named_graph(&self, graph: Self::Id) -> bool {
        if self.read_error().is_some() {
            return false;
        }
        if self.snapshot.empty_graphs.contains(&graph) {
            return true;
        }
        if self.snapshot.removals.iter().all(FastSet::is_empty) {
            return self.physical.has_named_graph(graph);
        }
        let g = GraphMatch::Named(graph);
        let found = self.quads_for_pattern(None, None, None, g).next().is_some()
            || self.reifier_quads_in_graph(g).next().is_some()
            || self.annotation_quads_in_graph(g).next().is_some();
        self.read_error().is_none() && found
    }
}
impl FallibleDatasetView for PagedStackQueryView<'_> {
    type Error = PagedQueryError;
    type Evidence = PagedStackEvidence;
    fn operation_status(&self) -> ViewOperationStatus<Self::Error, Self::Evidence> {
        let (error, pages) = match self.physical.operation_status() {
            ViewOperationStatus::Ready { evidence } => (None, evidence),
            ViewOperationStatus::Failed { error, evidence } => (Some(error), evidence),
        };
        let requested_origins = pages
            .requested_pages
            .iter()
            .map(|page| self.snapshot.origins[page.0 as usize])
            .collect();
        let evidence = PagedStackEvidence {
            sources: Arc::clone(&self.snapshot.sources),
            head: Arc::clone(&self.snapshot.head),
            head_graphs: Arc::clone(&self.snapshot.head_graphs),
            graph_declarations: Arc::clone(&self.snapshot.graph_declarations),
            removals: Arc::clone(&self.snapshot.removal_values),
            pages,
            requested_origins,
            head_bytes: self.snapshot.head_bytes,
        };
        match error {
            None => ViewOperationStatus::Ready { evidence },
            Some(error) => ViewOperationStatus::Failed { error, evidence },
        }
    }
}

/// Failure of canonical eager sealing or a guarded fold; no partial base is returned.
#[derive(Debug)]
pub enum CanonicalPagedError<E> {
    /// The source operation failed, retaining its original typed cause.
    Read(E),
    /// A typed recursive term lookup failure (including source read errors).
    Term(crate::TermLookupError<E>),
    /// A row failed the native RDF 1.2 validation boundary.
    Invalid(RdfDiagnostic),
    /// Native sealed page construction failed.
    Freeze(PagedFreezeError),
    /// The actual deterministic page encoder failed.
    Pack(PackError),
    /// Resident capacity was exhausted.
    Capacity,
    /// Snapshot construction failed before a read operation could start.
    Construction(Box<PagedStackError>),
}
impl<E: std::fmt::Display> std::fmt::Display for CanonicalPagedError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(e) => write!(f, "canonical source read: {e}"),
            Self::Term(e) => write!(f, "canonical term lookup: {e}"),
            Self::Invalid(e) => write!(f, "canonical RDF row: {e}"),
            Self::Freeze(e) => write!(f, "canonical page seal: {e}"),
            Self::Pack(e) => write!(f, "canonical page carrier: {e}"),
            Self::Capacity => f.write_str("canonical fold exceeds resident address capacity"),
            Self::Construction(e) => write!(f, "canonical snapshot construction: {e}"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for CanonicalPagedError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(error) => Some(error),
            Self::Term(error) => Some(error),
            Self::Invalid(error) => Some(error),
            Self::Freeze(error) => Some(error),
            Self::Pack(error) => Some(error),
            Self::Construction(error) => Some(error.as_ref()),
            Self::Capacity => None,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum CanonicalRecord {
    Ordinary(QuadValues),
    Reifier(QuadValues),
    Annotation(QuadValues),
    Graph(TermValue),
}

impl CanonicalRecord {
    fn replay(self, builder: &mut RdfDatasetBuilder) {
        match self {
            Self::Graph(graph) => {
                let g = builder.intern_value(&graph);
                builder.declare_named_graph(g);
            }
            row => {
                let (kind, row) = match row {
                    Self::Ordinary(row) => (0, row),
                    Self::Reifier(row) => (1, row),
                    Self::Annotation(row) => (2, row),
                    Self::Graph(_) => unreachable!(),
                };
                let s = builder.intern_value(&row.s);
                let p = builder.intern_value(&row.p);
                let o = builder.intern_value(&row.o);
                let g = row.g.as_ref().map(|g| builder.intern_value(g));
                match kind {
                    0 => builder.push_quad(s, p, o, g),
                    1 => builder.push_reifier_in_graph(s, o, g),
                    _ => builder.push_annotation_in_graph(s, p, o, g),
                }
            }
        }
    }
}

/// Canonical eager seal shared by head publication and whole-stack compaction.
/// Records sort by typed table (ordinary, reifier, annotation, graph declaration),
/// then by `(subject,predicate,object,graph)` value; graph records sort by value.
/// Each record, including a graph declaration, consumes one row of the positive
/// page bound. The completely empty surface produces zero pages. Actual page
/// charges are the byte lengths of the deterministic `PackBuilder` carriers.
///
/// # Errors
/// Source checkpoints and term reads, native validation, capacity, and carrier
/// encoding failures return no partially built dataset.
pub fn canonical_paged_seal<D: FallibleDatasetView>(
    view: &D,
    page_rows: NonZeroUsize,
) -> Result<PagedDataset, CanonicalPagedError<D::Error>> {
    checkpoint(view)?;
    let mut records = BTreeSet::new();
    let mut populated_graphs = BTreeSet::new();
    for (kind, rows) in [
        (
            0,
            Box::new(view.quads()) as Box<dyn Iterator<Item = QuadIds<D::Id>>>,
        ),
        (
            1,
            Box::new(view.reifier_quads()) as Box<dyn Iterator<Item = QuadIds<D::Id>>>,
        ),
        (
            2,
            Box::new(view.annotation_quads()) as Box<dyn Iterator<Item = QuadIds<D::Id>>>,
        ),
    ] {
        for q in rows {
            let row = QuadValues {
                s: view.term_value(q.s).map_err(CanonicalPagedError::Term)?,
                p: view.term_value(q.p).map_err(CanonicalPagedError::Term)?,
                o: view.term_value(q.o).map_err(CanonicalPagedError::Term)?,
                g: q.g
                    .map(|g| view.term_value(g))
                    .transpose()
                    .map_err(CanonicalPagedError::Term)?,
            };
            populated_graphs.extend(row.g.iter().cloned());
            records.insert(match kind {
                0 => CanonicalRecord::Ordinary(row),
                1 => CanonicalRecord::Reifier(row),
                _ => CanonicalRecord::Annotation(row),
            });
        }
        checkpoint(view)?;
    }
    for graph in view.named_graphs() {
        let graph = view.term_value(graph).map_err(CanonicalPagedError::Term)?;
        if !populated_graphs.contains(&graph) {
            records.insert(CanonicalRecord::Graph(graph));
        }
    }
    checkpoint(view)?;
    let mut pages = Vec::new();
    let mut records = records.into_iter().peekable();
    while records.peek().is_some() {
        let mut builder = RdfDatasetBuilder::new();
        for record in records.by_ref().take(page_rows.get()) {
            record.replay(&mut builder);
        }
        let page = builder.freeze().map_err(CanonicalPagedError::Invalid)?;
        let bytes = PackBuilder::build_bytes(&page).map_err(CanonicalPagedError::Pack)?;
        pages
            .try_reserve(1)
            .map_err(|_| CanonicalPagedError::Capacity)?;
        pages.push((page, bytes.len() as u64));
    }
    checkpoint(view)?;
    let sealed = PagedDataset::from_provider(Arc::new(InMemoryPageProvider::with_byte_lengths(
        pages,
        PageGeneration::INITIAL,
    )))
    .map_err(CanonicalPagedError::Freeze)?;
    checkpoint(view)?;
    Ok(sealed.compact())
}
fn checkpoint<D: FallibleDatasetView>(view: &D) -> Result<(), CanonicalPagedError<D::Error>> {
    match view.operation_status() {
        ViewOperationStatus::Ready { .. } => Ok(()),
        ViewOperationStatus::Failed { error, .. } => Err(CanonicalPagedError::Read(error)),
    }
}
