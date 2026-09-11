// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Immutable mutation snapshots over shared native indexes.

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::RdfStoreCapabilities;
use crate::dataset_view::{DatasetView, GraphMatch, ViewTermId};
use crate::hash::{FastMap, FastSet};
use crate::ir::{QuadIds, QuadProbePlan, QuadRef, RdfDataset, TermId, TermRef, TermValue};

/// A term in one mutation snapshot. Equal values shared by both layers always
/// use `Base`; a `Delta` ID therefore names a value absent from the base.
/// Handles belong to the snapshot that returned them, never another dataset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeltaViewId {
    /// An existing value in the shared base.
    Base(TermId),
    /// A new value in the immutable delta.
    Delta(TermId),
}

impl ViewTermId for DeltaViewId {
    type JoinKeyAtom = u64;

    fn encode(self) -> u64 {
        match self {
            Self::Base(id) => id.index() as u64,
            Self::Delta(id) => (1 << 32) | id.index() as u64,
        }
    }

    fn encode_computed(scratch_index: u32) -> u64 {
        (1 << 33) | u64::from(scratch_index)
    }
}

/// An immutable RDF read view of `(base - suppressed) + delta`.
///
/// Publication copies only the delta and suppression set. Reads borrow the two
/// native dictionaries and probe their existing indexes. The sparse alias map
/// grows with delta terms, never with the base. Reifier bindings and annotations
/// retain their separate RDF 1.2 tables and original graph scopes.
///
/// The base remains available through [`Self::base`] for source locations and
/// non-RDF sidecars that `DatasetView` does not expose. Materializing this view
/// with a generic RDF importer does not transfer those sidecars automatically.
#[derive(Debug, Clone)]
pub struct DeltaDatasetView {
    base: Arc<RdfDataset>,
    delta: Arc<RdfDataset>,
    suppressed: Arc<FastSet<QuadIds>>,
    delta_ids: Arc<[DeltaViewId]>,
    base_to_delta: Arc<FastMap<TermId, TermId>>,
    duplicate_reifiers: Arc<FastSet<QuadIds>>,
    duplicate_annotations: Arc<FastSet<QuadIds>>,
    has_added_reifiers: bool,
    has_suppressed_reifiers: bool,
    stats: crate::ViewStats,
    work: Arc<super::super::view_accounting::WorkCounter>,
}

impl DeltaDatasetView {
    pub(super) fn new(
        base: Arc<RdfDataset>,
        delta: Arc<RdfDataset>,
        suppressed: FastSet<QuadIds>,
        limits: crate::ViewLimits,
    ) -> Result<Self, crate::RdfDiagnostic> {
        let mut stats = crate::ViewStats::default();
        stats.retain(&base);
        stats.retain(&delta);
        stats.auxiliary_bytes = delta
            .term_count()
            .saturating_mul(
                size_of::<DeltaViewId>()
                    + 4 * (size_of::<(TermId, TermId)>() + size_of::<(TermId, Option<TermId>)>()),
            )
            .saturating_add(
                suppressed
                    .len()
                    .saturating_add(delta.rdf_row_count())
                    .saturating_mul(4 * size_of::<QuadIds>()),
            );
        limits.check(&stats)?;
        let mut lookup = FastMap::default();
        let mut base_to_delta = FastMap::default();
        let delta_ids = (0..delta.term_count())
            .map(|index| {
                let id =
                    TermId::from_index(u32::try_from(index).expect("native term index fits u32"));
                if let Some(base_id) =
                    crate::ir::import::lookup_native_term(&delta, &base, id, Some, &mut lookup)
                {
                    base_to_delta.insert(base_id, id);
                    DeltaViewId::Base(base_id)
                } else {
                    DeltaViewId::Delta(id)
                }
            })
            .collect();
        let has_added_reifiers = delta.reifier_quads().next().is_some();
        let has_suppressed_reifiers = suppressed
            .iter()
            .any(|q| base.reifier_quads_of(q.s).any(|row| row == *q));
        let mut view = Self {
            has_added_reifiers,
            has_suppressed_reifiers,
            base,
            delta,
            suppressed: Arc::new(suppressed),
            delta_ids,
            base_to_delta: Arc::new(base_to_delta),
            duplicate_reifiers: Arc::default(),
            duplicate_annotations: Arc::default(),
            stats,
            work: Arc::default(),
        };
        view.work.add(crate::ViewWork {
            copied_terms: view.delta.term_count(),
            copied_rows: view.delta.rdf_row_count(),
            freezes: 1,
            materializations: 0,
            copied_text_bytes: view.delta.rdf_text_bytes(),
            copied_index_bytes: view.delta_ids.len() * size_of::<DeltaViewId>()
                + view.base_to_delta.len() * size_of::<(TermId, TermId)>()
                + view.suppressed.len() * size_of::<QuadIds>(),
        });
        // Defend the set-union seam against overlapping statement records by
        // probing native subject indexes, without collecting base-sized tables.
        view.duplicate_reifiers = Arc::new(
            view.delta
                .reifier_quads()
                .filter(|q| {
                    view.base_quad(view.map_delta(*q)).is_some_and(|base_q| {
                        view.base
                            .reifier_quads_of(base_q.s)
                            .any(|row| row == base_q)
                    })
                })
                .collect(),
        );
        view.duplicate_annotations = Arc::new(
            view.delta
                .annotation_quads()
                .filter(|q| {
                    view.base_quad(view.map_delta(*q)).is_some_and(|base_q| {
                        view.base
                            .annotations_of_with_graph(base_q.s)
                            .any(|(p, o, g)| (p, o, g) == (base_q.p, base_q.o, base_q.g))
                    })
                })
                .collect(),
        );
        view.work.add(crate::ViewWork {
            copied_index_bytes: (view.duplicate_reifiers.len() + view.duplicate_annotations.len())
                * size_of::<QuadIds>(),
            ..Default::default()
        });
        Ok(view)
    }

    /// Canonical handles of every term retained by this snapshot.
    pub fn term_ids(&self) -> impl Iterator<Item = DeltaViewId> + '_ {
        (0..self.base.term_count())
            .map(|index| {
                DeltaViewId::Base(TermId::from_index(
                    u32::try_from(index).expect("native index fits u32"),
                ))
            })
            .chain(
                self.delta_ids
                    .iter()
                    .copied()
                    .filter(|id| matches!(id, DeltaViewId::Delta(_))),
            )
    }

    /// Resolve the complete owned RDF value at an explicit consumer boundary.
    #[must_use]
    pub fn term_value(&self, id: DeltaViewId) -> TermValue {
        crate::ir::composite::owned_value(self, id)
    }

    /// Current retention dimensions and successful work across all view clones.
    #[must_use]
    pub fn stats(&self) -> crate::ViewStats {
        crate::ViewStats {
            work: self.work.get(),
            ..self.stats
        }
    }

    /// Explicit RDF materialization boundary; source sidecars remain on the base.
    ///
    /// # Errors
    /// Returns native admission errors without publishing a partial dataset.
    pub fn materialize(&self) -> Result<Arc<RdfDataset>, crate::RdfDiagnostic> {
        let result = crate::ir::pack::dataset_from_view(self)?;
        self.work.add(crate::ViewWork {
            copied_terms: result.term_count(),
            copied_rows: result.rdf_row_count(),
            freezes: 1,
            materializations: 1,
            copied_text_bytes: result.rdf_text_bytes(),
            ..Default::default()
        });
        Ok(result)
    }

    fn has_reifier(&self, subject: DeltaViewId, graph: Option<DeltaViewId>) -> bool {
        self.reifier_quads_of(subject).any(|q| q.g == graph)
    }

    pub(super) fn base_quad_is_ordinary(&self, q: QuadIds) -> bool {
        !self.suppressed.contains(&q)
            && (!self.has_added_reifiers
                || !self.has_reifier(DeltaViewId::Base(q.s), q.g.map(DeltaViewId::Base)))
    }

    fn base_annotation_is_ordinary(&self, q: QuadIds) -> bool {
        self.has_suppressed_reifiers
            && !self.suppressed.contains(&q)
            && self.base.reifier_quads_of(q.s).any(|row| row.g == q.g)
            && !self.has_reifier(DeltaViewId::Base(q.s), q.g.map(DeltaViewId::Base))
    }
    fn demoted_annotation_is_unique(&self, q: QuadIds) -> bool {
        self.base_annotation_is_ordinary(q)
            && self
                .base
                .quads_for_pattern(
                    Some(q.s),
                    Some(q.p),
                    Some(q.o),
                    q.g.map_or(GraphMatch::Default, GraphMatch::Named),
                )
                .next()
                .is_none()
    }
    fn base_annotation_is_retained(&self, q: QuadIds) -> bool {
        !self.suppressed.contains(&q) && !self.base_annotation_is_ordinary(q)
    }
    fn base_quad_is_annotation(&self, q: QuadIds) -> bool {
        self.has_added_reifiers
            && !self.suppressed.contains(&q)
            && self.has_reifier(DeltaViewId::Base(q.s), q.g.map(DeltaViewId::Base))
            && !self
                .base
                .annotations_of_with_graph(q.s)
                .any(|(p, o, g)| (p, o, g) == (q.p, q.o, q.g))
    }

    /// Original immutable base, including its locations and non-RDF sidecars.
    #[must_use]
    pub fn base(&self) -> &Arc<RdfDataset> {
        &self.base
    }

    /// The admitted added rows, before overlap with base statement metadata is
    /// removed by this view. Its term IDs are delta-local, not view IDs.
    #[must_use]
    pub fn delta(&self) -> &Arc<RdfDataset> {
        &self.delta
    }

    pub(crate) fn lookup_iri(&self, iri: &str) -> Option<DeltaViewId> {
        self.base
            .term_id_by_iri(iri)
            .map(DeltaViewId::Base)
            .or_else(|| self.delta.term_id_by_iri(iri).map(|id| self.delta_id(id)))
    }

    pub(crate) fn lookup_blank(
        &self,
        label: &str,
        scope: crate::BlankScope,
    ) -> Option<DeltaViewId> {
        self.base
            .term_id_by_blank(label, scope)
            .map(DeltaViewId::Base)
            .or_else(|| {
                self.delta
                    .term_id_by_blank(label, scope)
                    .map(|id| self.delta_id(id))
            })
    }

    pub(crate) fn lookup_literal(
        &self,
        lexical: &str,
        datatype: &str,
        language: Option<&str>,
        direction: Option<crate::RdfTextDirection>,
    ) -> Option<DeltaViewId> {
        self.base
            .term_id_by_literal(lexical, datatype, language, direction)
            .map(DeltaViewId::Base)
            .or_else(|| {
                self.delta
                    .term_id_by_literal(lexical, datatype, language, direction)
                    .map(|id| self.delta_id(id))
            })
    }

    pub(crate) fn lookup_triple(
        &self,
        s: DeltaViewId,
        p: DeltaViewId,
        o: DeltaViewId,
    ) -> Option<DeltaViewId> {
        for (layer, ds) in [(Layer::Base, &self.base), (Layer::Delta, &self.delta)] {
            if let (Some(s), Some(p), Some(o)) = (
                self.local_id(s, layer),
                self.local_id(p, layer),
                self.local_id(o, layer),
            ) && let Some(id) = ds.term_id_by_triple(s, p, o)
            {
                return Some(match layer {
                    Layer::Base => DeltaViewId::Base(id),
                    Layer::Delta => self.delta_id(id),
                });
            }
        }
        None
    }

    fn delta_id(&self, id: TermId) -> DeltaViewId {
        self.delta_ids[id.index()]
    }

    fn map_delta(&self, q: QuadIds) -> QuadIds<DeltaViewId> {
        map_quad(q, |id| self.delta_id(id))
    }

    /// Query with a plan prepared for these bound axes and graph constraint.
    ///
    /// The plan is copied into the cursor, which borrows only this view. Results
    /// and iteration order match [`DatasetView::quads_for_pattern`].
    pub fn quads_for_pattern_with_plan(
        &self,
        plan: &QuadProbePlan,
        s: Option<DeltaViewId>,
        p: Option<DeltaViewId>,
        o: Option<DeltaViewId>,
        g: GraphMatch<DeltaViewId>,
    ) -> impl Iterator<Item = QuadIds<DeltaViewId>> + '_ + use<'_> {
        self.probe(*plan, s, p, o, g)
    }

    fn probe(
        &self,
        plan: QuadProbePlan,
        s: Option<DeltaViewId>,
        p: Option<DeltaViewId>,
        o: Option<DeltaViewId>,
        g: GraphMatch<DeltaViewId>,
    ) -> impl Iterator<Item = QuadIds<DeltaViewId>> + '_ + use<'_> {
        let base = self
            .local_pattern(s, p, o, g, Layer::Base)
            .into_iter()
            .flat_map(move |q| {
                self.base
                    .as_ref()
                    .quads_for_pattern_with_plan(&plan, q.s, q.p, q.o, q.g)
            })
            .filter(|q| self.base_quad_is_ordinary(*q))
            .map(|q| map_quad(q, DeltaViewId::Base));
        let delta = self
            .local_pattern(s, p, o, g, Layer::Delta)
            .into_iter()
            .flat_map(move |q| {
                self.delta
                    .as_ref()
                    .quads_for_pattern_with_plan(&plan, q.s, q.p, q.o, q.g)
            })
            .map(|q| self.map_delta(q));
        let demoted = self
            .has_suppressed_reifiers
            .then_some(self.base.as_ref())
            .into_iter()
            .flat_map(move |ds| {
                let subject = s.and_then(|id| self.local_id(id, Layer::Base));
                subject
                    .into_iter()
                    .flat_map(move |subject| {
                        ds.annotations_of_with_graph(subject)
                            .map(move |(p, o, g)| QuadIds {
                                s: subject,
                                p,
                                o,
                                g,
                            })
                    })
                    .chain(
                        std::iter::once(ds)
                            .filter(move |_| s.is_none())
                            .flat_map(RdfDataset::annotation_quads),
                    )
            })
            .filter(|q| self.demoted_annotation_is_unique(*q))
            .map(|q| map_quad(q, DeltaViewId::Base))
            .filter(move |q| {
                s.is_none_or(|v| q.s == v)
                    && p.is_none_or(|v| q.p == v)
                    && o.is_none_or(|v| q.o == v)
                    && g.matches(q.g)
            });
        base.chain(demoted).chain(delta)
    }

    fn local_id(&self, id: DeltaViewId, layer: Layer) -> Option<TermId> {
        match (id, layer) {
            (DeltaViewId::Base(id), Layer::Base) | (DeltaViewId::Delta(id), Layer::Delta) => {
                Some(id)
            }
            (DeltaViewId::Base(id), Layer::Delta) => self.base_to_delta.get(&id).copied(),
            (DeltaViewId::Delta(_), Layer::Base) => None,
        }
    }

    fn base_quad(&self, q: QuadIds<DeltaViewId>) -> Option<QuadIds> {
        let id = |id| self.local_id(id, Layer::Base);
        Some(QuadIds {
            s: id(q.s)?,
            p: id(q.p)?,
            o: id(q.o)?,
            g: match q.g {
                Some(g) => Some(id(g)?),
                None => None,
            },
        })
    }

    fn local_pattern(
        &self,
        s: Option<DeltaViewId>,
        p: Option<DeltaViewId>,
        o: Option<DeltaViewId>,
        g: GraphMatch<DeltaViewId>,
        layer: Layer,
    ) -> Option<Pattern> {
        let map = |id: Option<DeltaViewId>| match id {
            Some(id) => self.local_id(id, layer).map(Some),
            None => Some(None),
        };
        Some(Pattern {
            s: map(s)?,
            p: map(p)?,
            o: map(o)?,
            g: match g {
                GraphMatch::Any => GraphMatch::Any,
                GraphMatch::Default => GraphMatch::Default,
                GraphMatch::Named(id) => GraphMatch::Named(self.local_id(id, layer)?),
            },
        })
    }
}

#[derive(Clone, Copy)]
enum Layer {
    Base,
    Delta,
}

struct Pattern {
    s: Option<TermId>,
    p: Option<TermId>,
    o: Option<TermId>,
    g: GraphMatch,
}

fn map_quad(q: QuadIds, map: impl Fn(TermId) -> DeltaViewId) -> QuadIds<DeltaViewId> {
    QuadIds {
        s: map(q.s),
        p: map(q.p),
        o: map(q.o),
        g: q.g.map(map),
    }
}

fn map_term(term: TermRef<'_>, map: impl Fn(TermId) -> DeltaViewId) -> TermRef<'_, DeltaViewId> {
    match term {
        TermRef::Iri(iri) => TermRef::Iri(iri),
        TermRef::Blank { label, scope } => TermRef::Blank { label, scope },
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

impl DatasetView for DeltaDatasetView {
    type Id = DeltaViewId;
    type ProbePlan = QuadProbePlan;

    fn quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.base
            .quads()
            .filter(|q| self.base_quad_is_ordinary(*q))
            .chain(
                self.has_suppressed_reifiers
                    .then_some(self.base.as_ref())
                    .into_iter()
                    .flat_map(RdfDataset::annotation_quads)
                    .filter(|q| self.demoted_annotation_is_unique(*q)),
            )
            .map(|q| map_quad(q, DeltaViewId::Base))
            .chain(self.delta.quads().map(|q| self.map_delta(q)))
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
        match id {
            DeltaViewId::Base(id) => map_term(self.base.resolve(id), DeltaViewId::Base),
            DeltaViewId::Delta(id) => map_term(self.delta.resolve(id), |id| self.delta_id(id)),
        }
    }

    fn term_id_by_value(&self, value: &TermValue) -> Option<Self::Id> {
        self.base
            .term_id_by_value(value)
            .map(DeltaViewId::Base)
            .or_else(|| {
                self.delta
                    .term_id_by_value(value)
                    .map(|id| self.delta_id(id))
            })
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        self.base.capabilities().union(self.delta.capabilities())
    }

    fn len_hint(&self) -> Option<usize> {
        None
    }

    fn term_count(&self) -> usize {
        self.base.term_count() + self.delta.term_count() - self.base_to_delta.len()
    }

    fn stats_fingerprint(&self) -> u64 {
        // Cost-ranking discriminator only, never content or cache authority.
        (self.stats.retained_rows as u64).rotate_left(32) ^ self.term_count() as u64
    }

    fn probe_plan(
        &self,
        s_bound: bool,
        p_bound: bool,
        o_bound: bool,
        g: GraphMatch<Self::Id>,
    ) -> Self::ProbePlan {
        // Native plan selection depends on boundness, not a graph's identity.
        let graph = if matches!(g, GraphMatch::Any) {
            GraphMatch::Any
        } else {
            GraphMatch::Default
        };
        RdfDataset::probe_plan(s_bound, p_bound, o_bound, graph)
    }

    fn quads_for_pattern(
        &self,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        let plan = self.probe_plan(s.is_some(), p.is_some(), o.is_some(), g);
        self.probe(plan, s, p, o, g)
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
        // Cost ranking is an upper bound. Native index estimates need no result
        // scan; retained metadata bounds any annotations exposed by deletion.
        let base = self.local_pattern(s, p, o, g, Layer::Base).map_or(0, |q| {
            self.base
                .cardinality_estimate(q.s, q.p, q.o, q.g)
                .saturating_add(self.base.rdf_row_count() - self.base.quad_count())
        });
        let delta = self
            .local_pattern(s, p, o, g, Layer::Delta)
            .map_or(0, |q| self.delta.cardinality_estimate(q.s, q.p, q.o, q.g));
        base.saturating_add(delta)
    }

    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.base
            .reifier_quads()
            .filter(|q| !self.suppressed.contains(q))
            .map(|q| map_quad(q, DeltaViewId::Base))
            .chain(
                self.delta
                    .reifier_quads()
                    .filter(|q| !self.duplicate_reifiers.contains(q))
                    .map(|q| self.map_delta(q)),
            )
    }

    fn reifier_quads_of(&self, reifier: Self::Id) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.local_id(reifier, Layer::Base)
            .into_iter()
            .flat_map(|id| self.base.reifier_quads_of(id))
            .filter(|q| !self.suppressed.contains(q))
            .map(|q| map_quad(q, DeltaViewId::Base))
            .chain(
                self.local_id(reifier, Layer::Delta)
                    .into_iter()
                    .flat_map(|id| self.delta.reifier_quads_of(id))
                    .filter(|q| !self.duplicate_reifiers.contains(q))
                    .map(|q| self.map_delta(q)),
            )
    }

    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.base
            .annotation_quads()
            .filter(|q| self.base_annotation_is_retained(*q))
            .chain(
                self.has_added_reifiers
                    .then_some(self.base.as_ref())
                    .into_iter()
                    .flat_map(RdfDataset::quads)
                    .filter(|q| self.base_quad_is_annotation(*q)),
            )
            .map(|q| map_quad(q, DeltaViewId::Base))
            .chain(
                self.delta
                    .annotation_quads()
                    .filter(|q| !self.duplicate_annotations.contains(q))
                    .map(|q| self.map_delta(q)),
            )
    }

    fn annotations_of_with_graph(
        &self,
        reifier: Self::Id,
    ) -> impl Iterator<Item = (Self::Id, Self::Id, Option<Self::Id>)> + '_ {
        let base = self
            .local_id(reifier, Layer::Base)
            .into_iter()
            .flat_map(move |subject| {
                self.base
                    .annotations_of_with_graph(subject)
                    .map(move |(p, o, g)| QuadIds {
                        s: subject,
                        p,
                        o,
                        g,
                    })
                    .filter(|q| self.base_annotation_is_retained(*q))
                    .chain(
                        self.base
                            .quads_for_pattern(Some(subject), None, None, GraphMatch::Any)
                            .filter(|q| self.base_quad_is_annotation(*q)),
                    )
            })
            .map(|q| map_quad(q, DeltaViewId::Base));
        let delta = self
            .local_id(reifier, Layer::Delta)
            .into_iter()
            .flat_map(move |subject| {
                self.delta
                    .annotations_of_with_graph(subject)
                    .map(move |(p, o, g)| QuadIds {
                        s: subject,
                        p,
                        o,
                        g,
                    })
            })
            .filter(|q| !self.duplicate_annotations.contains(q))
            .map(|q| self.map_delta(q));
        base.chain(delta).map(|q| (q.p, q.o, q.g))
    }

    fn named_graphs(&self) -> impl Iterator<Item = Self::Id> + '_ {
        self.base
            .named_graphs()
            .map(DeltaViewId::Base)
            .chain(self.delta.named_graphs().map(|id| self.delta_id(id)))
            .collect::<BTreeSet<_>>()
            .into_iter()
    }
}

/// A snapshot borrows two frozen dictionaries and its own copied delta — nothing
/// it reads can fail to arrive — so every checkpoint is
/// [`Ready`](crate::ViewOperationStatus::Ready), before and after iteration alike.
impl crate::FallibleDatasetView for DeltaDatasetView {
    type Error = std::convert::Infallible;
    type Evidence = ();

    #[inline]
    fn operation_status(&self) -> crate::ViewOperationStatus<Self::Error, Self::Evidence> {
        crate::ViewOperationStatus::Ready { evidence: () }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset_view::DatasetMut;
    use crate::ir::mutable::QuadValues;
    use crate::ir::pack::dataset_from_view;
    use crate::ir::{MutableDataset, RdfDatasetBuilder};
    use crate::model::{RdfLiteral, RdfTextDirection};

    fn iri(local: &str) -> TermValue {
        TermValue::Iri(format!("https://example.org/{local}"))
    }
    fn row(s: &str, o: &str) -> QuadValues {
        QuadValues {
            s: iri(s),
            p: iri("p"),
            o: iri(o),
            g: None,
        }
    }

    fn assert_surface_equal(view: &DeltaDatasetView, frozen: &RdfDataset) {
        let materialized = dataset_from_view(view).unwrap();
        assert_eq!(
            materialized.owned_quads().collect::<FastSet<_>>(),
            frozen.owned_quads().collect()
        );
        assert_eq!(
            materialized.owned_reifiers().collect::<FastSet<_>>(),
            frozen.owned_reifiers().collect()
        );
        assert_eq!(
            materialized.owned_annotations().collect::<FastSet<_>>(),
            frozen.owned_annotations().collect()
        );
        assert_eq!(
            materialized
                .named_graphs()
                .map(|id| materialized.term_value(id))
                .collect::<FastSet<_>>(),
            frozen
                .named_graphs()
                .map(|id| frozen.term_value(id))
                .collect()
        );
    }

    #[test]
    fn snapshot_shares_base_and_isolates_suppression_reinsertion_and_new_terms() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://example.org/a");
        let p = b.intern_iri("https://example.org/p");
        let o = b.intern_iri("https://example.org/b");
        b.push_quad(s, p, o, None);
        let base = b.freeze().unwrap();
        let mut mutation = MutableDataset::new(Arc::clone(&base));
        assert!(mutation.remove(&row("a", "b")));
        assert!(mutation.insert(row("b", "new")).unwrap());
        let first = mutation.snapshot_view().unwrap();
        assert!(Arc::ptr_eq(first.base(), &base));
        assert_eq!(first.delta().quad_count(), 1);
        assert_surface_equal(&first, &mutation.freeze().unwrap());
        assert_eq!(
            first.term_id_by_value(&iri("b")),
            base.term_id_by_value(&iri("b")).map(DeltaViewId::Base)
        );
        let new_id = first.term_id_by_value(&iri("new")).unwrap();
        assert!(matches!(new_id, DeltaViewId::Delta(_)));
        assert!(new_id.encode() < DeltaViewId::encode_computed(0));
        assert!(mutation.insert(row("a", "b")).unwrap());
        assert!(mutation.remove(&row("b", "new")));
        let second = mutation.snapshot_view().unwrap();
        assert_surface_equal(&second, &mutation.freeze().unwrap());
        assert_eq!(first.quads().count(), 1);
        assert_eq!(second.quads().count(), 1);
        assert!(first.term_id_by_value(&iri("new")).is_some());
        assert!(second.term_id_by_value(&iri("new")).is_none());
        assert_ne!(first.quad_refs().next(), second.quad_refs().next());
    }

    #[test]
    fn indexed_patterns_match_complete_scan_for_shared_and_delta_ids() {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("https://example.org/p");
        let graph = b.intern_iri("https://example.org/g");
        for i in 0..20 {
            let s = b.intern_iri(&format!("https://example.org/s{i}"));
            b.push_quad(s, p, s, if i % 2 == 0 { Some(graph) } else { None });
        }
        let base = b.freeze().unwrap();
        let mut mutation = MutableDataset::new(base);
        mutation.remove(&row("s1", "s1"));
        mutation.insert(row("s1", "new")).unwrap();
        mutation
            .insert(QuadValues {
                g: Some(iri("new-graph")),
                ..row("new", "s0")
            })
            .unwrap();
        let view = mutation.snapshot_view().unwrap();
        let terms: Vec<_> = ["s0", "s1", "p", "new", "g", "new-graph"]
            .map(|s| view.term_id_by_value(&iri(s)).unwrap())
            .into_iter()
            .collect();
        let choices: Vec<_> = std::iter::once(None)
            .chain(terms.iter().copied().map(Some))
            .collect();
        let graphs: Vec<_> = [GraphMatch::Any, GraphMatch::Default]
            .into_iter()
            .chain(terms.iter().copied().map(GraphMatch::Named))
            .collect();
        for &s in &choices {
            for &p in &choices {
                for &o in &choices {
                    for &g in &graphs {
                        let scan: Vec<_> = view
                            .quads()
                            .filter(|q| {
                                s.is_none_or(|id| q.s == id)
                                    && p.is_none_or(|id| q.p == id)
                                    && o.is_none_or(|id| q.o == id)
                                    && g.matches(q.g)
                            })
                            .collect();
                        let plan = view.probe_plan(s.is_some(), p.is_some(), o.is_some(), g);
                        let indexed: Vec<_> = view
                            .quads_for_pattern_with_plan(&plan, s, p, o, g)
                            .collect();
                        assert_eq!(
                            indexed.iter().copied().collect::<FastSet<_>>(),
                            scan.iter().copied().collect()
                        );
                        assert_eq!(
                            indexed,
                            view.quads_for_pattern(s, p, o, g).collect::<Vec<_>>()
                        );
                        assert!(view.cardinality_estimate(s, p, o, g) >= scan.len());
                    }
                }
            }
        }
        assert_surface_equal(&view, &mutation.freeze().unwrap());
    }

    #[test]
    fn rdf12_tables_deduplicate_overlaps_and_keep_graphs_and_direction() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://example.org/s");
        let p = b.intern_iri("https://example.org/p");
        let literal = b.intern_literal(RdfLiteral {
            direction: Some(RdfTextDirection::Rtl),
            ..RdfLiteral::language_tagged("مرحبا", "ar")
        });
        let quoted = b.intern_triple(s, p, literal);
        let nested = b.intern_triple(s, p, quoted);
        let r = b.intern_iri("https://example.org/reifier");
        let g = b.intern_iri("https://example.org/context");
        let empty = b.intern_iri("https://example.org/empty");
        b.declare_named_graph(empty);
        b.push_quad(s, p, nested, Some(g));
        b.push_reifier_in_graph(r, quoted, Some(g));
        b.push_annotation_in_graph(r, p, literal, Some(g));
        let base = b.freeze().unwrap();
        let mut mutation = MutableDataset::new(Arc::clone(&base));
        for q in base.reifier_quads().chain(base.annotation_quads()) {
            mutation
                .insert(QuadValues {
                    s: base.term_value(q.s),
                    p: base.term_value(q.p),
                    o: base.term_value(q.o),
                    g: q.g.map(|id| base.term_value(id)),
                })
                .unwrap();
        }
        mutation
            .insert(QuadValues {
                s: iri("reifier"),
                p: iri("extra"),
                o: iri("new"),
                g: Some(iri("context")),
            })
            .unwrap();
        let view = mutation.snapshot_view().unwrap();
        assert_eq!(view.reifier_quads().count(), 1);
        assert_eq!(view.annotation_quads().count(), 2);
        let r = view.term_id_by_value(&iri("reifier")).unwrap();
        assert_eq!(view.reifier_quads_of(r).count(), 1);
        assert_eq!(view.annotations_of_with_graph(r).count(), 2);
        assert_eq!(view.named_graphs().count(), 2);
        assert_surface_equal(&view, &mutation.freeze().unwrap());
    }

    #[test]
    fn snapshot_rejects_invalid_delta_positions_before_exposing_a_view() {
        let mut mutation = MutableDataset::new(RdfDatasetBuilder::new().freeze().unwrap());
        mutation
            .insert(QuadValues {
                s: iri("s"),
                p: TermValue::Blank {
                    label: "bad-predicate".into(),
                    scope: crate::BlankScope::DEFAULT,
                },
                o: iri("o"),
                g: None,
            })
            .unwrap();
        assert!(mutation.snapshot_view().is_err());
        assert!(mutation.freeze().is_err());
    }
}
