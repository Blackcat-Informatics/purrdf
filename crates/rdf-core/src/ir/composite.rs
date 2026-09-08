// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Immutable composition over shared native dictionaries and indexes.

use super::view_accounting::WorkCounter;
use crate::blank_label::{LabelAlphabet, decode_blank_label, encode_blank_label};
use crate::cdt_blank::{cdt_embedded_blanks, rewrite_cdt_blank_terms};
use crate::hash::FastMap;
use crate::{
    BlankScope, DatasetView, DeltaDatasetView, DeltaViewId, GraphMatch, QuadIds, QuadProbePlan,
    QuadRef, RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfStoreCapabilities, TermId, TermRef,
    TermValue, ViewLimits, ViewStats, ViewTermId, ViewWork,
};
use std::collections::{BTreeMap, BTreeSet};
use std::hash::BuildHasher;
use std::sync::{Arc, LazyLock};

type LocalId = DeltaViewId;
type Pattern<I> = (Option<I>, Option<I>, Option<I>, GraphMatch<I>);

// Physical plans depend only on four bound-axis bits. Cache the small, fixed
// set once, including graph-unbound variants needed by placement and the
// all-SPO-bound variants used to suppress duplicates. No RDF values are retained.
fn physical_plan([s, p, o, graph_bound]: [bool; 4]) -> QuadProbePlan {
    static PLANS: LazyLock<[QuadProbePlan; 16]> = LazyLock::new(|| {
        std::array::from_fn(|mask| {
            RdfDataset::probe_plan(
                mask & 1 != 0,
                mask & 2 != 0,
                mask & 4 != 0,
                if mask & 8 == 0 {
                    GraphMatch::Any
                } else {
                    GraphMatch::Default
                },
            )
        })
    });
    PLANS
        [usize::from(s) | usize::from(p) << 1 | usize::from(o) << 2 | usize::from(graph_bound) << 3]
}

#[derive(Debug, Clone, Copy)]
enum SourceGraph {
    Preserve,
    Replace(Option<CompositeViewId>),
}

impl SourceGraph {
    fn named(self) -> Option<CompositeViewId> {
        match self {
            Self::Preserve => None,
            Self::Replace(graph) => graph,
        }
    }
}

/// Placement of every RDF record and graph declaration from one source.
#[derive(Debug, Clone, Default)]
pub enum GraphPlacement {
    /// Retain every original graph, including declaration-only graphs.
    #[default]
    Preserve,
    /// Project every record into the default graph.
    Default,
    /// Project every record into one caller-supplied IRI or blank graph name.
    Named(TermValue),
}

#[derive(Debug, Clone)]
enum Carrier {
    Native(Arc<RdfDataset>),
    Delta(Arc<DeltaDatasetView>),
}

/// One retained immutable source and its explicit graph placement.
#[derive(Debug, Clone)]
pub struct CompositeSource {
    carrier: Carrier,
    placement: GraphPlacement,
    literals: Arc<ReboundLiterals>,
}

impl CompositeSource {
    /// Retain a frozen native dataset, with its graph placement unchanged.
    #[must_use]
    pub fn new(dataset: Arc<RdfDataset>) -> Self {
        Self {
            carrier: Carrier::Native(dataset),
            placement: GraphPlacement::Preserve,
            literals: Arc::default(),
        }
    }
    /// Retain a delta snapshot without compacting its base.
    #[must_use]
    pub fn from_delta(view: Arc<DeltaDatasetView>) -> Self {
        Self {
            carrier: Carrier::Delta(view),
            placement: GraphPlacement::Preserve,
            literals: Arc::default(),
        }
    }
    /// Select graph placement for this source's complete RDF surface.
    #[must_use]
    pub fn with_graph_placement(mut self, placement: GraphPlacement) -> Self {
        self.placement = placement;
        self
    }
    /// Original native source, including source-owned sidecars, when applicable.
    #[must_use]
    pub fn dataset(&self) -> Option<&Arc<RdfDataset>> {
        match &self.carrier {
            Carrier::Native(ds) => Some(ds),
            Carrier::Delta(_) => None,
        }
    }
    /// Original delta snapshot and its base-sidecar owner, when applicable.
    #[must_use]
    pub fn delta(&self) -> Option<&Arc<DeltaDatasetView>> {
        match &self.carrier {
            Carrier::Native(_) => None,
            Carrier::Delta(ds) => Some(ds),
        }
    }
    fn native(&self) -> Option<&RdfDataset> {
        self.dataset().map(AsRef::as_ref)
    }
    fn delta_ref(&self) -> Option<&DeltaDatasetView> {
        self.delta().map(AsRef::as_ref)
    }
    fn term_ids(&self) -> impl Iterator<Item = LocalId> + '_ {
        self.native()
            .into_iter()
            .flat_map(|ds| {
                (0..ds.term_count()).map(|i| {
                    LocalId::Base(TermId::from_index(
                        u32::try_from(i).expect("native index fits u32"),
                    ))
                })
            })
            .chain(
                self.delta_ref()
                    .into_iter()
                    .flat_map(DeltaDatasetView::term_ids),
            )
    }
    fn term_count(&self) -> usize {
        match &self.carrier {
            Carrier::Native(ds) => ds.term_count(),
            Carrier::Delta(ds) => ds.term_count(),
        }
    }
    fn resolve_raw(&self, id: LocalId) -> TermRef<'_, LocalId> {
        match &self.carrier {
            Carrier::Native(ds) => {
                let LocalId::Base(id) = id else {
                    panic!("native source requires a native handle")
                };
                map_term(ds.resolve(id), LocalId::Base, |s| s)
            }
            Carrier::Delta(ds) => ds.resolve(id),
        }
    }
    fn resolve(&self, id: LocalId) -> TermRef<'_, LocalId> {
        match self.resolve_raw(id) {
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => TermRef::Literal {
                lexical: self
                    .literals
                    .values
                    .get(&id)
                    .map_or(lexical, String::as_str),
                datatype,
                language,
                direction,
            },
            term => term,
        }
    }
    fn blank_scopes(&self) -> BTreeSet<BlankScope> {
        let mut result = BTreeSet::new();
        for id in self.term_ids() {
            match self.resolve_raw(id) {
                TermRef::Blank { scope, .. } => {
                    result.insert(scope);
                }
                TermRef::Literal {
                    lexical, datatype, ..
                } => {
                    let TermRef::Iri(datatype) = self.resolve_raw(datatype) else {
                        unreachable!("native datatype is an IRI")
                    };
                    result.extend(
                        cdt_embedded_blanks(lexical, datatype)
                            .into_iter()
                            .map(|(_, scope)| scope),
                    );
                }
                _ => {}
            }
        }
        result
    }
    fn rebind_literals(&mut self, scopes: &BTreeMap<BlankScope, BlankScope>) -> ViewWork {
        let mut rebound = ReboundLiterals::default();
        let mut work = ViewWork::default();
        if scopes.iter().all(|(before, after)| before == after) {
            self.literals = Arc::default();
            return work;
        }
        for id in self.term_ids() {
            let TermRef::Literal {
                lexical, datatype, ..
            } = self.resolve_raw(id)
            else {
                continue;
            };
            let TermRef::Iri(datatype) = self.resolve_raw(datatype) else {
                unreachable!("native datatype is an IRI")
            };
            let rewritten = rewrite_cdt_blank_terms(lexical, datatype, &mut |token| {
                let (label, original) = decode_blank_label(token, LabelAlphabet::BlankNodeLabel);
                let mapped = scopes[&original];
                (mapped != original).then(|| {
                    format!(
                        "_:{}",
                        encode_blank_label(&label, mapped, LabelAlphabet::BlankNodeLabel)
                    )
                })
            });
            if let std::borrow::Cow::Owned(lexical) = rewritten {
                work.copied_text_bytes += lexical.len();
                work.copied_terms += 1;
                rebound.values.insert(id, lexical);
            }
        }
        for (&id, value) in &rebound.values {
            let hash = crate::hash::FastHasher::default().hash_one(value.as_str());
            rebound.index.insert_unique(hash, id, |id| {
                crate::hash::FastHasher::default().hash_one(rebound.values[id].as_str())
            });
        }
        work.copied_index_bytes =
            rebound.values.len() * (size_of::<(LocalId, String)>() + size_of::<LocalId>());
        self.literals = Arc::new(rebound);
        work
    }
    fn lookup_iri(&self, iri: &str) -> Option<LocalId> {
        match &self.carrier {
            Carrier::Native(ds) => ds.term_id_by_iri(iri).map(LocalId::Base),
            Carrier::Delta(ds) => ds.lookup_iri(iri),
        }
    }
    fn lookup_blank(&self, label: &str, scope: BlankScope) -> Option<LocalId> {
        match &self.carrier {
            Carrier::Native(ds) => ds.term_id_by_blank(label, scope).map(LocalId::Base),
            Carrier::Delta(ds) => ds.lookup_blank(label, scope),
        }
    }
    fn lookup_literal(
        &self,
        lexical: &str,
        datatype: &str,
        language: Option<&str>,
        direction: Option<crate::RdfTextDirection>,
    ) -> Option<LocalId> {
        let original = match &self.carrier {
            Carrier::Native(ds) => ds
                .term_id_by_literal(lexical, datatype, language, direction)
                .map(LocalId::Base),
            Carrier::Delta(ds) => ds.lookup_literal(lexical, datatype, language, direction),
        };
        if let Some(id) = original.filter(|id| !self.literals.values.contains_key(id)) {
            return Some(id);
        }
        let hash = crate::hash::FastHasher::default().hash_one(lexical);
        self.literals
            .index
            .find(hash, |&id| {
                let TermRef::Literal {
                    lexical: value,
                    datatype: dt,
                    language: lang,
                    direction: dir,
                } = self.resolve(id)
                else {
                    unreachable!("rebound entry is a literal")
                };
                value == lexical
                    && lang == language
                    && dir == direction
                    && matches!(self.resolve_raw(dt), TermRef::Iri(iri) if iri == datatype)
            })
            .copied()
    }
    fn lookup_triple(&self, s: LocalId, p: LocalId, o: LocalId) -> Option<LocalId> {
        match &self.carrier {
            Carrier::Native(ds) => match (s, p, o) {
                (LocalId::Base(s), LocalId::Base(p), LocalId::Base(o)) => {
                    ds.term_id_by_triple(s, p, o).map(LocalId::Base)
                }
                _ => None,
            },
            Carrier::Delta(ds) => ds.lookup_triple(s, p, o),
        }
    }
    fn metadata_rows(
        &self,
        table: Table,
        subject: Option<LocalId>,
    ) -> impl Iterator<Item = QuadIds<LocalId>> + '_ {
        let reifiers = matches!(table, Table::Reifier);
        let annotations = matches!(table, Table::Annotation);
        let native_subject = match subject {
            Some(LocalId::Base(id)) => Some(id),
            _ => None,
        };
        let native = self
            .native()
            .filter(move |_| !matches!(subject, Some(LocalId::Delta(_))))
            .into_iter()
            .flat_map(move |ds| {
                native_subject
                    .into_iter()
                    .filter(move |_| reifiers)
                    .flat_map(move |s| ds.reifier_quads_of(s))
                    .chain(
                        std::iter::once(ds)
                            .filter(move |_| reifiers && subject.is_none())
                            .flat_map(RdfDataset::reifier_quads),
                    )
                    .chain(
                        native_subject
                            .into_iter()
                            .filter(move |_| annotations)
                            .flat_map(move |s| {
                                ds.annotations_of_with_graph(s)
                                    .map(move |(p, o, g)| QuadIds { s, p, o, g })
                            }),
                    )
                    .chain(
                        std::iter::once(ds)
                            .filter(move |_| annotations && subject.is_none())
                            .flat_map(RdfDataset::annotation_quads),
                    )
                    .map(|q| map_quad(q, LocalId::Base))
            });
        let delta = self.delta_ref().into_iter().flat_map(move |ds| {
            subject
                .into_iter()
                .filter(move |_| reifiers)
                .flat_map(move |s| ds.reifier_quads_of(s))
                .chain(
                    std::iter::once(ds)
                        .filter(move |_| reifiers && subject.is_none())
                        .flat_map(DeltaDatasetView::reifier_quads),
                )
                .chain(
                    subject
                        .into_iter()
                        .filter(move |_| annotations)
                        .flat_map(move |s| {
                            ds.annotations_of_with_graph(s)
                                .map(move |(p, o, g)| QuadIds { s, p, o, g })
                        }),
                )
                .chain(
                    std::iter::once(ds)
                        .filter(move |_| annotations && subject.is_none())
                        .flat_map(DeltaDatasetView::annotation_quads),
                )
        });
        native.chain(delta)
    }
    fn probe(
        &self,
        table: Table,
        plan: QuadProbePlan,
        (s, p, o, g): Pattern<LocalId>,
    ) -> impl Iterator<Item = QuadIds<LocalId>> + '_ {
        let ordinary = matches!(table, Table::Ordinary);
        let native_pattern = local_native_pattern(s, p, o, g);
        let native = self
            .native()
            .filter(move |_| ordinary)
            .into_iter()
            .flat_map(move |ds| {
                native_pattern
                    .into_iter()
                    .flat_map(move |(s, p, o, g)| ds.quads_for_pattern_with_plan(&plan, s, p, o, g))
                    .map(|q| map_quad(q, LocalId::Base))
            });
        let delta = self
            .delta_ref()
            .filter(move |_| ordinary)
            .into_iter()
            .flat_map(move |ds| ds.quads_for_pattern_with_plan(&plan, s, p, o, g));
        native.chain(delta).chain(
            self.metadata_rows(table, s)
                .filter(move |q| matches_pattern(*q, s, p, o, g)),
        )
    }
    fn estimate(
        &self,
        s: Option<LocalId>,
        p: Option<LocalId>,
        o: Option<LocalId>,
        g: GraphMatch<LocalId>,
    ) -> usize {
        match &self.carrier {
            Carrier::Native(ds) => local_native_pattern(s, p, o, g)
                .map_or(0, |(s, p, o, g)| ds.cardinality_estimate(s, p, o, g)),
            Carrier::Delta(ds) => ds.cardinality_estimate(s, p, o, g),
        }
    }
    fn graphs(&self) -> impl Iterator<Item = LocalId> + '_ {
        self.native()
            .into_iter()
            .flat_map(|ds| ds.named_graphs().map(LocalId::Base))
            .chain(
                self.delta_ref()
                    .into_iter()
                    .flat_map(DeltaDatasetView::named_graphs),
            )
    }
    fn retain(&self, stats: &mut ViewStats) {
        match &self.carrier {
            Carrier::Native(ds) => stats.retain(ds),
            Carrier::Delta(ds) => {
                stats.retain(ds.base());
                stats.retain(ds.delta());
                stats.auxiliary_bytes = stats
                    .auxiliary_bytes
                    .saturating_add(ds.stats().auxiliary_bytes);
            }
        }
    }
}

#[derive(Debug, Default)]
struct ReboundLiterals {
    values: FastMap<LocalId, String>,
    index: hashbrown::HashTable<LocalId>,
}

/// An opaque handle local to one composite view. Source-local integer equality
/// never establishes cross-source RDF equality; construction resolves typed values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompositeViewId {
    source: u32,
    local: LocalId,
}
impl ViewTermId for CompositeViewId {
    type JoinKeyAtom = u128;
    fn encode(self) -> u128 {
        (u128::from(self.source) << 34) | u128::from(self.local.encode())
    }
    fn encode_computed(index: u32) -> u128 {
        (1_u128 << 96) | u128::from(index)
    }
}

/// Immutable, indexed set union of retained sources. Construction copies alias
/// handles, without copying base rows or unchanged strings. Composite literals
/// referencing renamed blanks own only their rebound lexical forms. Independent mode
/// remaps blank scopes by source order; shared mode preserves the supplied scope
/// identities. Graph projection is explicit. No pointer or work counter affects RDF.
#[derive(Debug, Clone)]
pub struct CompositeDatasetView {
    sources: Arc<[CompositeSource]>,
    mappings: Arc<[FastMap<LocalId, CompositeViewId>]>,
    reverse: Arc<[FastMap<CompositeViewId, LocalId>]>,
    scopes: Arc<[BTreeMap<BlankScope, BlankScope>]>,
    placement: Arc<[SourceGraph]>,
    user_sources: usize,
    unique_terms: usize,
    stats: ViewStats,
    work: Arc<WorkCounter>,
}

impl CompositeDatasetView {
    /// Compose independently parsed documents, standardizing blanks apart.
    /// # Errors
    /// Refuses invalid graph names, exhausted scope IDs or retention ceilings.
    pub fn new(sources: Vec<Arc<RdfDataset>>, limits: ViewLimits) -> Result<Self, RdfDiagnostic> {
        Self::from_sources(
            sources.into_iter().map(CompositeSource::new).collect(),
            limits,
        )
    }
    /// Compose contributions whose supplied blank scopes already express identity.
    /// # Errors
    /// Refuses invalid graph names or retention ceilings.
    pub fn with_shared_scopes(
        sources: Vec<Arc<RdfDataset>>,
        limits: ViewLimits,
    ) -> Result<Self, RdfDiagnostic> {
        Self::from_shared_sources(
            sources.into_iter().map(CompositeSource::new).collect(),
            limits,
        )
    }
    /// Compose independent documents with explicit graph placement.
    /// # Errors
    /// Refuses invalid graph names, exhausted scope IDs or retention ceilings.
    pub fn from_sources(
        sources: Vec<CompositeSource>,
        limits: ViewLimits,
    ) -> Result<Self, RdfDiagnostic> {
        Self::build(sources, limits, true)
    }
    /// Compose source contributions in an explicitly shared blank identity space.
    /// # Errors
    /// Refuses invalid graph names or retention ceilings.
    pub fn from_shared_sources(
        sources: Vec<CompositeSource>,
        limits: ViewLimits,
    ) -> Result<Self, RdfDiagnostic> {
        Self::build(sources, limits, false)
    }

    fn build(
        mut sources: Vec<CompositeSource>,
        limits: ViewLimits,
        independent: bool,
    ) -> Result<Self, RdfDiagnostic> {
        let user_sources = sources.len();
        let (mut stats, graph_ids, graph_count) = prepare_sources(&mut sources, limits)?;
        let mut construction_work = ViewWork::default();
        let scopes = bind_scopes(
            &mut sources,
            user_sources,
            independent,
            &mut stats,
            &mut construction_work,
            limits,
        )?;
        let (mappings, reverse) = build_aliases(&sources, &scopes, &mut construction_work)?;
        let mut view = Self {
            sources: sources.into(),
            mappings: mappings.into(),
            reverse: reverse.into(),
            scopes: scopes.into(),
            placement: vec![SourceGraph::Preserve; user_sources + usize::from(graph_count > 0)]
                .into(),
            user_sources,
            unique_terms: 0,
            stats,
            work: Arc::default(),
        };
        let mut placement = Vec::with_capacity(view.sources.len());
        for (index, source) in view.sources.iter().enumerate() {
            placement.push(match &source.placement {
                GraphPlacement::Preserve => SourceGraph::Preserve,
                GraphPlacement::Default => SourceGraph::Replace(None),
                GraphPlacement::Named(_) => SourceGraph::Replace(Some(view.map_id(
                    user_sources,
                    LocalId::Base(graph_ids[index].expect("named placement has a graph ID")),
                ))),
            });
        }
        view.placement = placement.into();
        view.unique_terms = view.term_ids().count();
        view.work.add(construction_work);
        if graph_count > 0 {
            view.work.add(ViewWork {
                copied_terms: view.sources[user_sources].term_count(),
                copied_text_bytes: view.sources[user_sources]
                    .native()
                    .expect("graph dictionary")
                    .rdf_text_bytes(),
                freezes: 1,
                ..ViewWork::default()
            });
        }
        Ok(view)
    }

    /// The retained user sources, including their sidecar owners and placement.
    #[must_use]
    pub fn sources(&self) -> &[CompositeSource] {
        &self.sources[..self.user_sources]
    }
    /// Translate a native source handle into this view's canonical identity.
    #[must_use]
    pub fn source_id(&self, source: usize, id: TermId) -> CompositeViewId {
        self.map_id(source, LocalId::Base(id))
    }
    /// Translate a delta source handle into this view's canonical identity.
    #[must_use]
    pub fn delta_source_id(&self, source: usize, id: DeltaViewId) -> CompositeViewId {
        self.map_id(source, id)
    }
    /// All canonical handles, including caller-supplied graph-name terms.
    pub fn term_ids(&self) -> impl Iterator<Item = CompositeViewId> + '_ {
        self.sources
            .iter()
            .enumerate()
            .flat_map(move |(index, source)| {
                source.term_ids().filter_map(move |id| {
                    let mapped = self.map_id(index, id);
                    (mapped.source as usize == index && mapped.local == id).then_some(mapped)
                })
            })
    }
    /// Resolve an owned value only at a caller-requested materialization boundary.
    #[must_use]
    pub fn term_value(&self, id: CompositeViewId) -> TermValue {
        owned_value(self, id)
    }
    /// Retained dimensions and successful work shared by clones.
    #[must_use]
    pub fn stats(&self) -> ViewStats {
        ViewStats {
            work: self.work.get(),
            ..self.stats
        }
    }
    /// Materialize the complete RDF surface at an explicit ownership boundary.
    /// # Errors
    /// Returns admission errors without publishing a partial dataset.
    pub fn materialize(&self) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        let result = crate::ir::pack::dataset_from_view(self)?;
        self.work.add(ViewWork {
            copied_terms: result.term_count(),
            copied_rows: result.rdf_row_count(),
            freezes: 1,
            materializations: 1,
            copied_text_bytes: result.rdf_text_bytes(),
            ..Default::default()
        });
        Ok(result)
    }
    fn map_id(&self, source: usize, local: LocalId) -> CompositeViewId {
        if source == 0 {
            CompositeViewId { source: 0, local }
        } else {
            self.mappings[source][&local]
        }
    }
    fn local_id(&self, source: usize, id: CompositeViewId) -> Option<LocalId> {
        if source == 0 {
            (id.source == 0).then_some(id.local)
        } else {
            self.reverse[source].get(&id).copied()
        }
    }
    fn map_row(&self, index: usize, q: QuadIds<LocalId>) -> QuadIds<CompositeViewId> {
        let mut q = map_quad(q, |id| self.map_id(index, id));
        if let SourceGraph::Replace(graph) = self.placement[index] {
            q.g = graph;
        }
        q
    }
    fn pattern(
        &self,
        index: usize,
        s: Option<CompositeViewId>,
        p: Option<CompositeViewId>,
        o: Option<CompositeViewId>,
        g: GraphMatch<CompositeViewId>,
    ) -> Option<Pattern<LocalId>> {
        let local = |value| match value {
            Some(id) => self.local_id(index, id).map(Some),
            None => Some(None),
        };
        let graph = match self.placement[index] {
            SourceGraph::Replace(placed) => {
                if g.matches(placed) {
                    GraphMatch::Any
                } else {
                    return None;
                }
            }
            SourceGraph::Preserve => match g {
                GraphMatch::Any => GraphMatch::Any,
                GraphMatch::Default => GraphMatch::Default,
                GraphMatch::Named(id) => GraphMatch::Named(self.local_id(index, id)?),
            },
        };
        Some((local(s)?, local(p)?, local(o)?, graph))
    }
    fn contains(&self, index: usize, table: Table, q: QuadIds<CompositeViewId>) -> bool {
        self.pattern(
            index,
            Some(q.s),
            Some(q.p),
            Some(q.o),
            q.g.map_or(GraphMatch::Default, GraphMatch::Named),
        )
        .is_some_and(|(s, p, o, g)| {
            self.sources[index]
                .probe(
                    table,
                    physical_plan([true, true, true, !matches!(g, GraphMatch::Any)]),
                    (s, p, o, g),
                )
                .next()
                .is_some()
        })
    }
    /// Query with a plan prepared for these bound axes and graph constraint.
    ///
    /// The plan is copied into the cursor, which borrows only this view. Results
    /// and iteration order match [`DatasetView::quads_for_pattern`].
    pub fn quads_for_pattern_with_plan(
        &self,
        plan: &QuadProbePlan,
        s: Option<CompositeViewId>,
        p: Option<CompositeViewId>,
        o: Option<CompositeViewId>,
        g: GraphMatch<CompositeViewId>,
    ) -> impl Iterator<Item = QuadIds<CompositeViewId>> + '_ + use<'_> {
        self.probe(Table::Ordinary, *plan, (s, p, o, g))
    }

    fn probe(
        &self,
        table: Table,
        plan: QuadProbePlan,
        (s, p, o, g): Pattern<CompositeViewId>,
    ) -> impl Iterator<Item = QuadIds<CompositeViewId>> + '_ {
        self.sources[..self.user_sources]
            .iter()
            .enumerate()
            .flat_map(move |(index, source)| {
                // Placement removes the logical graph constraint from the physical
                // pattern. A graph-bound prefix would be invalid for that source;
                // retain the caller's plan everywhere the bound axes stay intact.
                let source_plan = if matches!(self.placement[index], SourceGraph::Replace(_))
                    && !matches!(g, GraphMatch::Any)
                {
                    physical_plan([s.is_some(), p.is_some(), o.is_some(), false])
                } else {
                    plan
                };
                self.pattern(index, s, p, o, g)
                    .into_iter()
                    .flat_map(move |pattern| source.probe(table, source_plan, pattern))
                    .filter(move |q| {
                        matches!(self.placement[index], SourceGraph::Preserve)
                            || source
                                .probe(
                                    table,
                                    physical_plan([true, true, true, false]),
                                    (Some(q.s), Some(q.p), Some(q.o), GraphMatch::Any),
                                )
                                .next()
                                .is_some_and(|first| first == *q)
                    })
                    .map(move |q| self.map_row(index, q))
                    .filter(move |q| !(0..index).any(|earlier| self.contains(earlier, table, *q)))
            })
    }
    fn lookup_value(&self, index: usize, value: &TermValue) -> Option<LocalId> {
        let source = &self.sources[index];
        match value {
            TermValue::Iri(iri) => source.lookup_iri(iri),
            TermValue::Blank { label, scope } => self.scopes[index]
                .iter()
                .find_map(|(original, mapped)| (*mapped == *scope).then_some(*original))
                .and_then(|scope| source.lookup_blank(label, scope)),
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => source.lookup_literal(lexical_form, datatype, language.as_deref(), *direction),
            TermValue::Triple { s, p, o } => source.lookup_triple(
                self.lookup_value(index, s)?,
                self.lookup_value(index, p)?,
                self.lookup_value(index, o)?,
            ),
        }
    }
}

fn prepare_sources(
    sources: &mut Vec<CompositeSource>,
    limits: ViewLimits,
) -> Result<(ViewStats, Vec<Option<TermId>>, usize), RdfDiagnostic> {
    let mut stats = ViewStats::default();
    for source in sources.iter() {
        source.retain(&mut stats);
        let graph_bytes = match &source.placement {
            GraphPlacement::Named(TermValue::Iri(iri)) => iri.len(),
            GraphPlacement::Named(TermValue::Blank { label, .. }) => label.len(),
            _ => 0,
        };
        stats.auxiliary_bytes = stats
            .auxiliary_bytes
            .saturating_add(graph_bytes)
            .saturating_add(
                size_of::<CompositeSource>()
                    + 2 * size_of::<FastMap<LocalId, CompositeViewId>>()
                    + size_of::<BTreeMap<BlankScope, BlankScope>>()
                    + size_of::<SourceGraph>()
                    + size_of::<Option<TermId>>(),
            );
    }
    // Conservative construction + retained bookkeeping admission, before maps.
    let alias_terms = sources
        .iter()
        .skip(1)
        .fold(0_usize, |n, s| n.saturating_add(s.term_count()));
    // Two retained maps plus one temporary recursive lookup map. The first
    // source uses identity translation and has no term-sized alias map.
    stats.auxiliary_bytes = stats
        .auxiliary_bytes
        .saturating_add(alias_terms.saturating_mul(
            4 * (2 * size_of::<(LocalId, CompositeViewId)>()
                + size_of::<(LocalId, Option<LocalId>)>()),
        ));
    limits.check(&stats)?;
    let mut graph_builder = RdfDatasetBuilder::new();
    let mut graph_ids = Vec::with_capacity(sources.len());
    let mut graph_count = 0;
    for source in sources.iter() {
        let id = match &source.placement {
            GraphPlacement::Named(TermValue::Iri(iri)) => Some(graph_builder.intern_iri(iri)),
            GraphPlacement::Named(TermValue::Blank { label, scope }) => {
                Some(graph_builder.intern_blank(label, *scope))
            }
            GraphPlacement::Named(_) => {
                return Err(RdfDiagnostic::error(
                    "view-graph-name",
                    "view graph placement requires an IRI or blank node",
                ));
            }
            _ => None,
        };
        if let Some(id) = id {
            graph_builder.declare_named_graph(id);
            graph_count += 1;
        }
        graph_ids.push(id);
    }
    if graph_count > 0 {
        let graphs = graph_builder.freeze()?;
        stats.retain(&graphs);
        stats.auxiliary_bytes = stats
            .auxiliary_bytes
            .saturating_add(graphs.term_count().saturating_mul(
                4 * (2 * size_of::<(LocalId, CompositeViewId)>()
                    + size_of::<(LocalId, Option<LocalId>)>()),
            ))
            .saturating_add(size_of::<CompositeSource>());
        sources.push(CompositeSource::new(graphs));
        limits.check(&stats)?;
    }
    Ok((stats, graph_ids, graph_count))
}

fn bind_scopes(
    sources: &mut [CompositeSource],
    user_sources: usize,
    independent: bool,
    stats: &mut ViewStats,
    construction_work: &mut ViewWork,
    limits: ViewLimits,
) -> Result<Vec<BTreeMap<BlankScope, BlankScope>>, RdfDiagnostic> {
    let mut scopes = Vec::with_capacity(sources.len());
    let mut used = BTreeSet::new();
    // Caller-supplied graph names retain their own scope. Reserve those scopes.
    for source in &sources[..user_sources] {
        if let GraphPlacement::Named(TermValue::Blank { scope, .. }) = &source.placement {
            used.insert(*scope);
        }
    }
    let mut next = 0_u32;
    for (index, source) in sources.iter().enumerate() {
        let original = source.blank_scopes();
        let mut mapping = BTreeMap::new();
        for scope in original {
            let mapped = if !independent || index == user_sources {
                scope
            } else {
                while used.contains(&BlankScope(next)) {
                    next = next.checked_add(1).ok_or_else(|| {
                        RdfDiagnostic::error(
                            "view-blank-scope",
                            "composite blank scope space exhausted",
                        )
                    })?;
                }
                let mapped = BlankScope(next);
                used.insert(mapped);
                mapped
            };
            mapping.insert(scope, mapped);
        }
        stats.auxiliary_bytes = stats.auxiliary_bytes.saturating_add(
            mapping
                .len()
                .saturating_mul(8 * size_of::<(BlankScope, BlankScope)>()),
        );
        limits.check(stats)?;
        construction_work.copied_index_bytes +=
            mapping.len() * size_of::<(BlankScope, BlankScope)>();
        scopes.push(mapping);
    }
    for (index, source) in sources.iter_mut().enumerate() {
        let work = source.rebind_literals(&scopes[index]);
        stats.auxiliary_bytes = stats
            .auxiliary_bytes
            .saturating_add(work.copied_text_bytes)
            .saturating_add(work.copied_index_bytes.saturating_mul(4));
        limits.check(stats)?;
        construction_work.copied_terms += work.copied_terms;
        construction_work.copied_text_bytes += work.copied_text_bytes;
        construction_work.copied_index_bytes += work.copied_index_bytes;
    }
    Ok(scopes)
}

type AliasMaps = (
    Vec<FastMap<LocalId, CompositeViewId>>,
    Vec<FastMap<CompositeViewId, LocalId>>,
);

fn build_aliases(
    sources: &[CompositeSource],
    scopes: &[BTreeMap<BlankScope, BlankScope>],
    construction_work: &mut ViewWork,
) -> Result<AliasMaps, RdfDiagnostic> {
    let mut mappings: Vec<FastMap<LocalId, CompositeViewId>> = Vec::new();
    let mut reverse: Vec<FastMap<CompositeViewId, LocalId>> = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let source_index = u32::try_from(index)
            .map_err(|_| RdfDiagnostic::error("view-source-count", "source count exceeds u32"))?;
        let mut mapping: FastMap<_, _> = if index == 0 {
            FastMap::default()
        } else {
            source
                .term_ids()
                .map(|local| {
                    (
                        local,
                        CompositeViewId {
                            source: source_index,
                            local,
                        },
                    )
                })
                .collect()
        };
        for previous in 0..index {
            // One memo at a time bounds transient storage by this contribution,
            // independent of the number or size of earlier dictionaries.
            let mut memo = FastMap::default();
            for id in source.term_ids() {
                if mapping[&id].source != source_index {
                    continue;
                }
                if let Some(found) = lookup_source_term(
                    source,
                    &sources[previous],
                    id,
                    |scope| {
                        let mapped = scopes[index][&scope];
                        scopes[previous]
                            .iter()
                            .find_map(|(old, new)| (*new == mapped).then_some(*old))
                    },
                    &mut memo,
                ) {
                    let canonical = if previous == 0 {
                        CompositeViewId {
                            source: 0,
                            local: found,
                        }
                    } else {
                        mappings[previous][&found]
                    };
                    mapping.insert(id, canonical);
                }
            }
        }
        let back = mapping
            .iter()
            .map(|(&local, &canonical)| (canonical, local))
            .collect();
        construction_work.copied_index_bytes +=
            mapping.len() * 2 * size_of::<(LocalId, CompositeViewId)>();
        mappings.push(mapping);
        reverse.push(back);
    }
    Ok((mappings, reverse))
}

impl DatasetView for CompositeDatasetView {
    type Id = CompositeViewId;
    type ProbePlan = QuadProbePlan;
    fn quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.quads_for_pattern(None, None, None, GraphMatch::Any)
    }
    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_, Self::Id>> + '_ {
        self.quads().map(|q| QuadRef {
            s: self.resolve(q.s),
            p: self.resolve(q.p),
            o: self.resolve(q.o),
            g: q.g.map(|id| self.resolve(id)),
        })
    }
    fn resolve(&self, id: Self::Id) -> TermRef<'_, Self::Id> {
        let index = id.source as usize;
        map_term(
            self.sources[index].resolve(id.local),
            |local| self.map_id(index, local),
            |scope| self.scopes[index][&scope],
        )
    }
    fn term_id_by_value(&self, value: &TermValue) -> Option<Self::Id> {
        (0..self.sources.len()).find_map(|index| {
            self.lookup_value(index, value)
                .map(|local| self.map_id(index, local))
        })
    }
    fn capabilities(&self) -> RdfStoreCapabilities {
        self.sources
            .iter()
            .fold(RdfStoreCapabilities::default(), |caps, source| {
                caps.union(match &source.carrier {
                    Carrier::Native(ds) => ds.capabilities(),
                    Carrier::Delta(ds) => ds.capabilities(),
                })
            })
    }
    fn term_count(&self) -> usize {
        self.unique_terms
    }
    fn probe_plan(&self, s: bool, p: bool, o: bool, g: GraphMatch<Self::Id>) -> Self::ProbePlan {
        physical_plan([s, p, o, !matches!(g, GraphMatch::Any)])
    }
    fn quads_for_pattern(
        &self,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        let plan = self.probe_plan(s.is_some(), p.is_some(), o.is_some(), g);
        self.probe(Table::Ordinary, plan, (s, p, o, g))
    }
    fn quads_for_pattern_with_plan(
        &self,
        plan: &Self::ProbePlan,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        Self::quads_for_pattern_with_plan(self, plan, s, p, o, g)
    }
    fn cardinality_estimate(
        &self,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> usize {
        self.sources[..self.user_sources].iter().enumerate().fold(
            0_usize,
            |total, (index, source)| {
                total.saturating_add(
                    self.pattern(index, s, p, o, g)
                        .map_or(0, |(s, p, o, g)| source.estimate(s, p, o, g)),
                )
            },
        )
    }
    fn stats_fingerprint(&self) -> u64 {
        (self.stats.retained_rows as u64).rotate_left(32) ^ self.unique_terms as u64
    }
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.probe(
            Table::Reifier,
            physical_plan([false, false, false, false]),
            (None, None, None, GraphMatch::Any),
        )
    }
    fn reifier_quads_of(&self, s: Self::Id) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.probe(
            Table::Reifier,
            physical_plan([true, false, false, false]),
            (Some(s), None, None, GraphMatch::Any),
        )
    }
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.probe(
            Table::Annotation,
            physical_plan([false, false, false, false]),
            (None, None, None, GraphMatch::Any),
        )
    }
    fn annotations_of_with_graph(
        &self,
        s: Self::Id,
    ) -> impl Iterator<Item = (Self::Id, Self::Id, Option<Self::Id>)> + '_ {
        self.probe(
            Table::Annotation,
            physical_plan([true, false, false, false]),
            (Some(s), None, None, GraphMatch::Any),
        )
        .map(|q| (q.p, q.o, q.g))
    }
    fn named_graphs(&self) -> impl Iterator<Item = Self::Id> + '_ {
        self.sources[..self.user_sources]
            .iter()
            .enumerate()
            .flat_map(|(index, source)| {
                source
                    .graphs()
                    .filter_map(move |id| match self.placement[index] {
                        SourceGraph::Preserve => Some(self.map_id(index, id)),
                        SourceGraph::Replace(graph) => graph,
                    })
                    .chain(self.placement[index].named())
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
    }
}

#[derive(Clone, Copy)]
enum Table {
    Ordinary,
    Reifier,
    Annotation,
}
fn matches_pattern<I: Copy + Eq>(
    q: QuadIds<I>,
    s: Option<I>,
    p: Option<I>,
    o: Option<I>,
    g: GraphMatch<I>,
) -> bool {
    s.is_none_or(|id| id == q.s)
        && p.is_none_or(|id| id == q.p)
        && o.is_none_or(|id| id == q.o)
        && g.matches(q.g)
}
fn map_quad<A, B>(q: QuadIds<A>, map: impl Fn(A) -> B) -> QuadIds<B> {
    QuadIds {
        s: map(q.s),
        p: map(q.p),
        o: map(q.o),
        g: q.g.map(map),
    }
}
fn map_term<A, B>(
    term: TermRef<'_, A>,
    map: impl Fn(A) -> B,
    scope: impl Fn(BlankScope) -> BlankScope,
) -> TermRef<'_, B> {
    match term {
        TermRef::Iri(iri) => TermRef::Iri(iri),
        TermRef::Blank { label, scope: old } => TermRef::Blank {
            label,
            scope: scope(old),
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => TermRef::Literal {
            lexical,
            datatype: map(datatype),
            language,
            direction,
        },
        TermRef::Triple { s, p, o } => TermRef::Triple {
            s: map(s),
            p: map(p),
            o: map(o),
        },
    }
}
fn local_native_pattern(
    s: Option<LocalId>,
    p: Option<LocalId>,
    o: Option<LocalId>,
    g: GraphMatch<LocalId>,
) -> Option<Pattern<TermId>> {
    let local = |id| match id {
        None => Some(None),
        Some(LocalId::Base(id)) => Some(Some(id)),
        Some(LocalId::Delta(_)) => None,
    };
    let g = match g {
        GraphMatch::Any => GraphMatch::Any,
        GraphMatch::Default => GraphMatch::Default,
        GraphMatch::Named(LocalId::Base(id)) => GraphMatch::Named(id),
        GraphMatch::Named(LocalId::Delta(_)) => return None,
    };
    Some((local(s)?, local(p)?, local(o)?, g))
}
fn lookup_source_term(
    source: &CompositeSource,
    target: &CompositeSource,
    id: LocalId,
    scope: impl Fn(BlankScope) -> Option<BlankScope> + Copy,
    memo: &mut FastMap<LocalId, Option<LocalId>>,
) -> Option<LocalId> {
    if let Some(found) = memo.get(&id) {
        return *found;
    }
    let found = match source.resolve(id) {
        TermRef::Iri(iri) => target.lookup_iri(iri),
        TermRef::Blank { label, scope: old } => {
            scope(old).and_then(|scope| target.lookup_blank(label, scope))
        }
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let TermRef::Iri(datatype) = source.resolve(datatype) else {
                unreachable!("native literal datatype is IRI")
            };
            target.lookup_literal(lexical, datatype, language, direction)
        }
        TermRef::Triple { s, p, o } => {
            let s = lookup_source_term(source, target, s, scope, memo);
            let p = lookup_source_term(source, target, p, scope, memo);
            let o = lookup_source_term(source, target, o, scope, memo);
            match (s, p, o) {
                (Some(s), Some(p), Some(o)) => target.lookup_triple(s, p, o),
                _ => None,
            }
        }
    };
    memo.insert(id, found);
    found
}
pub(crate) fn owned_value<D: DatasetView>(view: &D, id: D::Id) -> TermValue {
    match view.resolve(id) {
        TermRef::Iri(iri) => TermValue::Iri(iri.to_owned()),
        TermRef::Blank { label, scope } => TermValue::Blank {
            label: label.to_owned(),
            scope,
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let TermRef::Iri(datatype) = view.resolve(datatype) else {
                unreachable!("native literal datatype is IRI")
            };
            TermValue::Literal {
                lexical_form: lexical.to_owned(),
                datatype: datatype.to_owned(),
                language: language.map(str::to_owned),
                direction,
            }
        }
        TermRef::Triple { s, p, o } => TermValue::Triple {
            s: Box::new(owned_value(view, s)),
            p: Box::new(owned_value(view, p)),
            o: Box::new(owned_value(view, o)),
        },
    }
}
