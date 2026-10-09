// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable mutation snapshots over shared native indexes.

use std::collections::BTreeSet;
use std::sync::Arc;

use super::RecordKind;
use crate::RdfStoreCapabilities;
use crate::dataset_view::{DatasetView, GraphMatch, ViewTermId};
use crate::hash::{FastMap, FastSet};
use crate::ir::cursor::{Cursor, optional};
use crate::ir::{QuadIds, QuadProbePlan, RdfDataset, TermId, TermRef, TermValue};

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
/// A named graph whose declaration the mutable branch withdrew and that holds no
/// row is not enumerated by [`DatasetView::named_graphs`]. A remembered slot stays
/// present when its last row leaves; implicit-mode removal withdraws it. This view
/// carries the resulting presence, independently of the branch's selected policy.
///
/// The base remains available through [`Self::base`] for source locations and
/// non-RDF sidecars that `DatasetView` does not expose. Materializing this view
/// with a generic RDF importer does not transfer those sidecars automatically.
#[derive(Debug, Clone)]
pub struct DeltaDatasetView {
    base: Arc<RdfDataset>,
    delta: Arc<RdfDataset>,
    suppressed: Arc<FastSet<(RecordKind, QuadIds)>>,
    /// Generated base-origin records replay through their original indexed segment.
    /// This is owner-produced provenance, not a reader-side classification rule.
    converted: Arc<FastSet<(RecordKind, QuadIds)>>,
    has_converted_ordinary: bool,
    has_converted_annotations: bool,
    delta_ids: Arc<[DeltaViewId]>,
    base_to_delta: Arc<FastMap<TermId, TermId>>,
    duplicate_reifiers: Arc<FastSet<QuadIds>>,
    duplicate_annotations: Arc<FastSet<QuadIds>>,
    /// Base named graphs the mutation emptied that hold no row of this snapshot.
    /// Probed by membership only; never iterated for order.
    withdrawn_graphs: Arc<FastSet<TermId>>,
    stats: crate::ViewStats,
    work: Arc<super::super::view_accounting::WorkCounter>,
}

impl DeltaDatasetView {
    /// The construction charge of one term of the frozen delta: its slot in the
    /// delta id table plus the hash entries that map it to a base id, at four times
    /// their payload for table slack.
    pub(super) const AUXILIARY_BYTES_PER_DELTA_TERM: usize = size_of::<DeltaViewId>()
        + 4 * (size_of::<(TermId, TermId)>() + size_of::<(TermId, Option<TermId>)>());

    pub(super) fn new(
        base: Arc<RdfDataset>,
        delta: Arc<RdfDataset>,
        suppressed: FastSet<(RecordKind, QuadIds)>,
        converted: FastSet<(RecordKind, QuadIds)>,
        withdrawn_graphs: Arc<FastSet<TermId>>,
        limits: crate::ViewLimits,
    ) -> Result<Self, crate::RdfDiagnostic> {
        let mut stats = crate::ViewStats::default();
        stats.retain(&base);
        stats.retain(&delta);
        stats.auxiliary_bytes = delta
            .as_ref()
            .term_count()
            .saturating_mul(Self::AUXILIARY_BYTES_PER_DELTA_TERM)
            .saturating_add(
                suppressed
                    .len()
                    .saturating_add(converted.len())
                    .saturating_add(delta.rdf_row_count())
                    .saturating_mul(4 * size_of::<(RecordKind, QuadIds)>()),
            );
        limits.check(&stats)?;
        let mut lookup = FastMap::default();
        let mut base_to_delta = FastMap::default();
        let delta_ids = (0..delta.as_ref().term_count())
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
        let mut has_converted_ordinary = false;
        let mut has_converted_annotations = false;
        for (kind, _) in &converted {
            match kind {
                RecordKind::Ordinary => has_converted_ordinary = true,
                RecordKind::Annotation => has_converted_annotations = true,
                RecordKind::Reifier => unreachable!("classification does not create declarations"),
            }
        }
        let mut view = Self {
            base,
            delta,
            suppressed: Arc::new(suppressed),
            converted: Arc::new(converted),
            has_converted_ordinary,
            has_converted_annotations,
            delta_ids,
            base_to_delta: Arc::new(base_to_delta),
            duplicate_reifiers: Arc::default(),
            duplicate_annotations: Arc::default(),
            withdrawn_graphs,
            stats,
            work: Arc::default(),
        };
        view.work.add(crate::ViewWork {
            copied_terms: view.delta.as_ref().term_count(),
            copied_rows: view.delta.rdf_row_count(),
            freezes: 1,
            materializations: 0,
            copied_text_bytes: view.delta.rdf_text_bytes(),
            copied_index_bytes: view.delta_ids.len() * size_of::<DeltaViewId>()
                + view.base_to_delta.len() * size_of::<(TermId, TermId)>()
                + (view.suppressed.len() + view.converted.len())
                    * size_of::<(RecordKind, QuadIds)>(),
        });
        // Defend the set-union seam against overlapping statement records by
        // probing native subject indexes, without collecting base-sized tables.
        view.duplicate_reifiers = Arc::new(
            view.delta
                .reifier_quads()
                .filter(|q| {
                    view.base_quad(view.map_delta(*q)).is_some_and(|base_q| {
                        view.base.reifier_quads_of(base_q.s).any(|row| {
                            row == base_q && !view.suppressed.contains(&(RecordKind::Reifier, row))
                        })
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
                            .any(|(p, o, g)| {
                                (p, o, g) == (base_q.p, base_q.o, base_q.g)
                                    && !view.suppressed.contains(&(RecordKind::Annotation, base_q))
                            })
                    })
                })
                .collect(),
        );
        view.work.add(crate::ViewWork {
            copied_index_bytes: (view.duplicate_reifiers.len() + view.duplicate_annotations.len())
                * size_of::<(RecordKind, QuadIds)>(),
            ..Default::default()
        });
        Ok(view)
    }

    /// Canonical handles of every term retained by this snapshot.
    pub fn term_ids(&self) -> impl Iterator<Item = DeltaViewId> + '_ {
        (0..self.base.as_ref().term_count())
            .map(|index| {
                DeltaViewId::Base(TermId::from_index(
                    u32::try_from(index).expect("native index fits u32"),
                ))
            })
            .chain(self.added_term_ids())
    }

    /// The handles of the terms this snapshot adds to its base, in the order
    /// [`Self::term_ids`] lists them after the base's: the delta's own terms, read in
    /// time proportional to the delta rather than to the base.
    pub fn added_term_ids(&self) -> impl Iterator<Item = DeltaViewId> + '_ {
        self.delta_ids
            .iter()
            .copied()
            .filter(|id| matches!(id, DeltaViewId::Delta(_)))
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
        // Rebuilt under the base's configuration, as `MutableDataset::freeze` is.
        let result =
            crate::ir::pack::certify::dataset_from_view_into(self, self.base.rebuild_builder())?;
        self.work.add(crate::ViewWork {
            copied_terms: result.as_ref().term_count(),
            copied_rows: result.rdf_row_count(),
            freezes: 1,
            materializations: 1,
            copied_text_bytes: result.rdf_text_bytes(),
            ..Default::default()
        });
        Ok(result)
    }

    /// Whether the mutation withdrew the base graph `graph`: it emptied the graph,
    /// which now holds no row of this snapshot, so the snapshot does not enumerate
    /// it. An O(1) membership probe of the set the mutation kept current.
    fn is_withdrawn_graph(&self, graph: TermId) -> bool {
        self.withdrawn_graphs.contains(&graph)
    }

    fn effective_named_graphs(&self) -> impl Iterator<Item = DeltaViewId> + '_ {
        self.base
            .named_graphs()
            .filter(|&graph| !self.is_withdrawn_graph(graph))
            .map(DeltaViewId::Base)
            .chain(self.delta.named_graphs().map(|id| self.delta_id(id)))
    }

    pub(super) fn base_quad_is_ordinary(&self, q: QuadIds) -> bool {
        !self.suppressed.contains(&(RecordKind::Ordinary, q))
    }

    fn base_annotation_is_retained(&self, q: QuadIds) -> bool {
        !self.suppressed.contains(&(RecordKind::Annotation, q))
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

    /// Every row this snapshot CHANGES relative to its base, in this view's own
    /// [`DeltaViewId`] space: each added row, then each suppressed one.
    ///
    /// The point of the id space is that a consumer can join these rows against
    /// reads of this same view without a term-value round trip. [`Self::delta`]
    /// alone cannot serve that: its ids are delta-LOCAL, so a term the base
    /// already interned carries a different number there than it does here.
    ///
    /// The three added-row streams are chained because the delta keeps RDF 1.2
    /// reifier and annotation rows in tables of their own, and a consumer asking
    /// "what moved?" that saw only the plain rows would miss a statement whose
    /// only change was an annotation. Rows may therefore repeat — a row present in
    /// two of those tables is yielded twice, and an added row identical to a
    /// suppressed one is yielded on both sides. That is deliberate: this is a
    /// CHANGE set, its consumers deduplicate whatever they accumulate from it, and
    /// deduplicating here would cost a base-sized set to answer a question nobody
    /// asked.
    ///
    /// # What "changes" means here: the SURFACE, not the table
    ///
    /// The guarantee is over this snapshot's RDF surface — plain rows and BOTH
    /// statement tables together, which is what an RDF 1.2 consumer reads. Every
    /// row that JOINS or LEAVES that surface is named here.
    ///
    /// It is deliberately NOT a guarantee about any one table, because the RDF 1.2
    /// overlay moves rows BETWEEN tables without touching the surface: declaring a
    /// reifier for `(r, g)` demotes the base rows about `r` in `g` from the plain
    /// table into `r`'s annotations (`base_quad_is_ordinary` and
    /// `base_quad_is_annotation`), and suppressing a base reifier promotes them
    /// back (`base_annotation_is_ordinary`). A reclassified row is still read, at
    /// the same subject and predicate, by any consumer that unions the overlay onto
    /// the plain stream — so it did not change, and only the reifier declaration
    /// itself did. That declaration IS named, as an added or a suppressed row.
    ///
    /// A consumer that reads [`DatasetView::quads`] ALONE, without the
    /// statement tables, sees a narrower surface than this set describes, and for
    /// that consumer a demotion does look like a disappearance. The only caller
    /// today is the incremental SHACL change path, and it reads the projected
    /// union — not by happy accident but because it REFUSES to expand a change
    /// otherwise: its data view carries a statement-projection predicate and its
    /// expansion entry point hard-fails a delta binding that answers `false`,
    /// naming the projecting constructor to use instead. That guard is what keeps
    /// this paragraph true, since the narrow view is constructible through a
    /// public argument. A consumer introduced later that genuinely wants the
    /// narrow surface needs this stream extended with the reclassified rows rather
    /// than a filter downstream, because a change set that is short by a row
    /// cannot be told from a graph that did not change.
    /// `changed_quads_names_every_row_the_rdf12_overlay_moves_on_or_off_the_surface`
    /// holds the line as stated.
    pub fn changed_quads(&self) -> impl Iterator<Item = QuadIds<DeltaViewId>> + '_ {
        self.delta
            .quads_for_pattern(None, None, None, GraphMatch::Any)
            .chain(self.delta.reifier_quads())
            .chain(self.delta.annotation_quads())
            .map(|q| self.map_delta(q))
            .chain(
                self.suppressed
                    .iter()
                    .map(|(_, q)| q.map_ids(DeltaViewId::Base)),
            )
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
        q.map_ids(|id| self.delta_id(id))
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
        // Each layer contributes its one cursor or none (`optional`), and the
        // demoted statement rows one arm, so the cursor holds no inactive branch
        // and no `flat_map` front/back pair.
        let base = optional(self.local_pattern(s, p, o, g, Layer::Base).map(|q| {
            self.base
                .as_ref()
                .quads_for_pattern_with_plan(&plan, q.s, q.p, q.o, q.g)
        }))
        .filter(|q| self.base_quad_is_ordinary(*q))
        .map(|q| q.map_ids(DeltaViewId::Base));
        let delta = optional(self.local_pattern(s, p, o, g, Layer::Delta).map(|q| {
            self.delta
                .as_ref()
                .quads_for_pattern_with_plan(&plan, q.s, q.p, q.o, q.g)
        }))
        .filter(|q| !self.delta_is_converted(RecordKind::Ordinary, *q))
        .map(|q| self.map_delta(q));
        base.chain(self.converted_ordinary(s, p, o, g)).chain(delta)
    }

    fn delta_is_converted(&self, kind: RecordKind, q: QuadIds) -> bool {
        self.base_quad(self.map_delta(q))
            .is_some_and(|base| self.converted.contains(&(kind, base)))
    }

    fn converted_ordinary(
        &self,
        s: Option<DeltaViewId>,
        p: Option<DeltaViewId>,
        o: Option<DeltaViewId>,
        g: GraphMatch<DeltaViewId>,
    ) -> impl Iterator<Item = QuadIds<DeltaViewId>> + '_ {
        type Unused = std::iter::Empty<QuadIds>;
        let rows = match (self.has_converted_ordinary, s) {
            (false, _) => Cursor::<_, _, Unused, Unused>::Empty,
            (true, None) => Cursor::Second(self.base.annotation_quads()),
            (true, Some(id)) => match self.local_id(id, Layer::Base) {
                Some(subject) => Cursor::First(self.base.annotations_of_with_graph(subject).map(
                    move |(p, o, g)| QuadIds {
                        s: subject,
                        p,
                        o,
                        g,
                    },
                )),
                None => Cursor::Empty,
            },
        };
        rows.filter(|q| self.converted.contains(&(RecordKind::Ordinary, *q)))
            .map(|q| q.map_ids(DeltaViewId::Base))
            .filter(move |q| {
                s.is_none_or(|v| q.s == v)
                    && p.is_none_or(|v| q.p == v)
                    && o.is_none_or(|v| q.o == v)
                    && g.matches(q.g)
            })
    }

    fn converted_annotations(
        &self,
        s: Option<DeltaViewId>,
        g: GraphMatch<DeltaViewId>,
    ) -> impl Iterator<Item = QuadIds<DeltaViewId>> + '_ {
        optional(
            self.has_converted_annotations
                .then(|| self.local_pattern(s, None, None, g, Layer::Base))
                .flatten()
                .map(|q| self.base.as_ref().quads_for_pattern(q.s, q.p, q.o, q.g)),
        )
        .filter(|q| self.converted.contains(&(RecordKind::Annotation, *q)))
        .map(|q| q.map_ids(DeltaViewId::Base))
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
            g: self.local_graph(g, layer)?,
        })
    }

    /// One graph constraint translated into `layer`'s own handle space, or `None`
    /// when that layer's dictionary does not hold the named graph at all.
    ///
    /// `None` is a proof of emptiness, not an error: every row a layer yields carries
    /// a graph slot drawn from that layer's dictionary, so a graph the dictionary
    /// never interned cannot appear in any of its rows. A caller may therefore skip
    /// the whole layer instead of scanning it.
    fn local_graph(&self, g: GraphMatch<DeltaViewId>, layer: Layer) -> Option<GraphMatch> {
        match g {
            GraphMatch::Any => Some(GraphMatch::Any),
            GraphMatch::Default => Some(GraphMatch::Default),
            GraphMatch::Named(id) => self.local_id(id, layer).map(GraphMatch::Named),
        }
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

impl DatasetView for DeltaDatasetView {
    type Id = DeltaViewId;
    type ReadError = std::convert::Infallible;
    type TermGuard<'a>
        = TermRef<'a, Self::Id>
    where
        Self: 'a;
    type ProbePlan = QuadProbePlan;

    /// The wider bound of the base and the delta.
    fn triple_term_nesting_bound(&self) -> Option<usize> {
        crate::dataset_view::widest_nesting_bound([
            self.base.triple_term_nesting_bound(),
            self.delta.triple_term_nesting_bound(),
        ])
    }

    fn quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.base
            .quads()
            .filter(|q| self.base_quad_is_ordinary(*q))
            .map(|q| q.map_ids(DeltaViewId::Base))
            .chain(self.converted_ordinary(None, None, None, GraphMatch::Any))
            .chain(
                self.delta
                    .quads()
                    .filter(|q| !self.delta_is_converted(RecordKind::Ordinary, *q))
                    .map(|q| self.map_delta(q)),
            )
    }

    fn resolve(&self, id: Self::Id) -> Result<Self::TermGuard<'_>, Self::ReadError> {
        Ok({
            match id {
                DeltaViewId::Base(id) => self.base.as_ref().resolve(id).map_ids(DeltaViewId::Base),
                DeltaViewId::Delta(id) => self
                    .delta
                    .as_ref()
                    .resolve(id)
                    .map_ids(|id| self.delta_id(id)),
            }
        })
    }

    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<Self::Id>, Self::ReadError> {
        Ok({
            self.base
                .as_ref()
                .term_id_by_value(value)
                .map(DeltaViewId::Base)
                .or_else(|| {
                    self.delta
                        .as_ref()
                        .term_id_by_value(value)
                        .map(|id| self.delta_id(id))
                })
        })
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        let mut capabilities = self.base.capabilities().union(self.delta.capabilities());
        capabilities.named_graphs = self.effective_named_graphs().next().is_some();
        capabilities
    }

    fn len_hint(&self) -> Option<u64> {
        None
    }

    fn term_count(&self) -> u64 {
        u64::try_from({
            self.base.as_ref().term_count() + self.delta.as_ref().term_count()
                - self.base_to_delta.len()
        })
        .expect("bounded local count fits u64")
    }

    fn stats_fingerprint(&self) -> u64 {
        // Cost-ranking discriminator only, never content or cache authority.
        (self.stats.retained_rows as u64).rotate_left(32) ^ self.term_count()
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
    ) -> u64 {
        u64::try_from({
            // Cost ranking is an upper bound. Native index estimates need no result
            // scan; retained metadata bounds any annotations exposed by deletion.
            let base = self.local_pattern(s, p, o, g, Layer::Base).map_or(0, |q| {
                self.base
                    .as_ref()
                    .cardinality_estimate(q.s, q.p, q.o, q.g)
                    .saturating_add(self.base.rdf_row_count() - self.base.quad_count())
            });
            let delta = self.local_pattern(s, p, o, g, Layer::Delta).map_or(0, |q| {
                self.delta.as_ref().cardinality_estimate(q.s, q.p, q.o, q.g)
            });
            base.saturating_add(delta)
        })
        .expect("bounded local count fits u64")
    }

    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.base
            .reifier_quads()
            .filter(|q| !self.suppressed.contains(&(RecordKind::Reifier, *q)))
            .map(|q| q.map_ids(DeltaViewId::Base))
            .chain(
                self.delta
                    .reifier_quads()
                    .filter(|q| !self.duplicate_reifiers.contains(q))
                    .map(|q| self.map_delta(q)),
            )
    }

    fn reifier_quads_of(&self, reifier: Self::Id) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        optional(
            self.local_id(reifier, Layer::Base)
                .map(|id| self.base.reifier_quads_of(id)),
        )
        .filter(|q| !self.suppressed.contains(&(RecordKind::Reifier, *q)))
        .map(|q| q.map_ids(DeltaViewId::Base))
        .chain(
            optional(
                self.local_id(reifier, Layer::Delta)
                    .map(|id| self.delta.reifier_quads_of(id)),
            )
            .filter(|q| !self.duplicate_reifiers.contains(q))
            .map(|q| self.map_delta(q)),
        )
    }

    /// Narrows each layer through its OWN [`reifier_quads_in_graph`](DatasetView::reifier_quads_in_graph)
    /// before the overlay's masks (`suppressed` on the base, `duplicate_reifiers` on
    /// the delta) are applied, so a layer that can skip storage units for `g` still
    /// does; a layer whose dictionary cannot name `g` drops that whole arm via
    /// `local_graph`. The trait default would scan both layers' whole reifier tables.
    fn reifier_quads_in_graph(
        &self,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        // The overlay is a per-row mask, so it composes with the graph seam rather
        // than erasing it: narrow each layer through its OWN `reifier_quads_in_graph`
        // — so a layer that can skip storage units for `g` still does — and keep the
        // unkeyed override's masks (`suppressed` on the base, `duplicate_reifiers` on
        // the delta) on top, unchanged. A layer whose dictionary cannot name `g` at
        // all holds no row in `g`, so `local_graph` returning `None` drops that whole
        // arm. Chain order, and the order within each arm, are the unkeyed
        // override's, so this is the same multiset in the same order as
        // `reifier_quads().filter(|q| g.matches(q.g))`.
        optional(
            self.local_graph(g, Layer::Base)
                .map(|graph| self.base.reifier_quads_in_graph(graph)),
        )
        .filter(|q| !self.suppressed.contains(&(RecordKind::Reifier, *q)))
        .map(|q| q.map_ids(DeltaViewId::Base))
        .chain(
            optional(
                self.local_graph(g, Layer::Delta)
                    .map(|graph| self.delta.reifier_quads_in_graph(graph)),
            )
            .filter(|q| !self.duplicate_reifiers.contains(q))
            .map(|q| self.map_delta(q)),
        )
    }

    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.base
            .annotation_quads()
            .filter(|q| self.base_annotation_is_retained(*q))
            .map(|q| q.map_ids(DeltaViewId::Base))
            .chain(self.converted_annotations(None, GraphMatch::Any))
            .chain(
                self.delta
                    .annotation_quads()
                    .filter(|q| {
                        !self.duplicate_annotations.contains(q)
                            && !self.delta_is_converted(RecordKind::Annotation, *q)
                    })
                    .map(|q| self.map_delta(q)),
            )
    }

    /// See [`reifier_quads_in_graph`](DatasetView::reifier_quads_in_graph) above: the
    /// same layer-local narrowing and masks, over the ANNOTATION stream — including the
    /// middle arm for base quads promoted to annotations by a delta-added reifier.
    fn annotation_quads_in_graph(
        &self,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        // See `reifier_quads_in_graph` above: the same layer-local narrowing and the
        // same masks, over the ANNOTATION stream. The middle arm — base quads PROMOTED
        // to annotations because the delta added a reifier for them — reads the base's
        // ordinary table, which carries no graph seam of its own, so it keeps the
        // definitional row filter; `local_graph` still drops the whole base arm when
        // the base cannot name `g`.
        optional(self.local_graph(g, Layer::Base).map(|graph| {
            self.base
                .annotation_quads_in_graph(graph)
                .filter(|q| self.base_annotation_is_retained(*q))
        }))
        .map(|q| q.map_ids(DeltaViewId::Base))
        .chain(self.converted_annotations(None, g))
        .chain(
            optional(
                self.local_graph(g, Layer::Delta)
                    .map(|graph| self.delta.annotation_quads_in_graph(graph)),
            )
            .filter(|q| {
                !self.duplicate_annotations.contains(q)
                    && !self.delta_is_converted(RecordKind::Annotation, *q)
            })
            .map(|q| self.map_delta(q)),
        )
    }

    fn annotations_of_with_graph(
        &self,
        reifier: Self::Id,
    ) -> impl Iterator<Item = (Self::Id, Self::Id, Option<Self::Id>)> + '_ {
        let base = optional(self.local_id(reifier, Layer::Base).map(|subject| {
            self.base
                .annotations_of_with_graph(subject)
                .map(move |(p, o, g)| QuadIds {
                    s: subject,
                    p,
                    o,
                    g,
                })
                .filter(|q| self.base_annotation_is_retained(*q))
        }))
        .map(|q| q.map_ids(DeltaViewId::Base));
        let delta = optional(self.local_id(reifier, Layer::Delta).map(|subject| {
            self.delta
                .annotations_of_with_graph(subject)
                .map(move |(p, o, g)| QuadIds {
                    s: subject,
                    p,
                    o,
                    g,
                })
        }))
        .filter(|q| {
            !self.duplicate_annotations.contains(q)
                && !self.delta_is_converted(RecordKind::Annotation, *q)
        })
        .map(|q| self.map_delta(q));
        base.chain(self.converted_annotations(Some(reifier), GraphMatch::Any))
            .chain(delta)
            .map(|q| (q.p, q.o, q.g))
    }

    fn named_graphs(&self) -> impl Iterator<Item = Self::Id> + '_ {
        self.effective_named_graphs()
            .collect::<BTreeSet<_>>()
            .into_iter()
    }

    /// Membership in [`named_graphs`](DatasetView::named_graphs): `graph` names a
    /// graph of either layer, each asked through its own sorted graph set, with a base
    /// graph an operation emptied withdrawn exactly as the enumeration withdraws it. A
    /// graph repopulated since is a delta graph, so it answers through the delta.
    fn has_named_graph(&self, graph: Self::Id) -> bool {
        self.local_id(graph, Layer::Base)
            .is_some_and(|id| self.base.has_named_graph(id) && !self.is_withdrawn_graph(id))
            || self
                .local_id(graph, Layer::Delta)
                .is_some_and(|id| self.delta.has_named_graph(id))
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
    use crate::TermBox;
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
                .map(|id| materialized.term_value(id).unwrap())
                .collect::<FastSet<_>>(),
            frozen
                .named_graphs()
                .map(|id| frozen.term_value(id))
                .collect()
        );
    }

    /// Every row of one dataset's RDF surface — plain rows and BOTH statement
    /// tables — as owned values, which is the set an RDF 1.2 consumer that unions
    /// the overlay onto the plain stream actually reads.
    fn base_surface(base: &RdfDataset) -> BTreeSet<String> {
        base.quads()
            .chain(base.reifier_quads())
            .chain(base.annotation_quads())
            .map(|q| {
                row_key(&QuadValues {
                    s: base.term_value(q.s),
                    p: base.term_value(q.p),
                    o: base.term_value(q.o),
                    g: q.g.map(|g| base.term_value(g)),
                })
            })
            .collect()
    }

    /// The same surface, read through a snapshot.
    fn view_surface(view: &DeltaDatasetView) -> BTreeSet<String> {
        view.quads()
            .chain(view.reifier_quads())
            .chain(view.annotation_quads())
            .map(|q| {
                row_key(&QuadValues {
                    s: view.term_value(q.s),
                    p: view.term_value(q.p),
                    o: view.term_value(q.o),
                    g: q.g.map(|g| view.term_value(g)),
                })
            })
            .collect()
    }

    /// A value row as a comparable key. `QuadValues` is neither `Hash` nor `Ord`,
    /// and its derived `Debug` spells every component of every nested term, so two
    /// rows share a key exactly when they are the same row.
    fn row_key(q: &QuadValues) -> String {
        format!("{q:?}")
    }

    /// THE CHANGE-SET LAW, stated over the surface rather than over any one table:
    /// every row that joins or leaves the RDF surface must be NAMED by
    /// [`DeltaDatasetView::changed_quads`].
    ///
    /// This is the property a change-set consumer depends on, and it is stronger
    /// than "the added and suppressed tables are returned": the RDF 1.2 overlay
    /// RECLASSIFIES rows — an added reifier for `(r, g)` moves the base rows about
    /// `r` from the plain table into the annotation table, and suppressing one
    /// moves them back — so a table-by-table reading of "what changed" is not the
    /// same question. A row that merely changed tables did not join or leave the
    /// surface and is not required here; a row that is absent from the surface
    /// after being present in it (or the reverse) is, and a change set that omits
    /// one is a silent drop, because nothing downstream can tell a short answer
    /// from a quiet graph.
    fn assert_changed_quads_covers_the_surface_difference(
        base: &Arc<RdfDataset>,
        mutation: &MutableDataset,
        case: &str,
    ) {
        let view = mutation.snapshot_view().unwrap();
        let changed: BTreeSet<String> = view
            .changed_quads()
            .map(|q| {
                row_key(&QuadValues {
                    s: view.term_value(q.s),
                    p: view.term_value(q.p),
                    o: view.term_value(q.o),
                    g: q.g.map(|g| view.term_value(g)),
                })
            })
            .collect();
        let before = base_surface(base);
        let after = view_surface(&view);
        assert_ne!(
            before, after,
            "{case}: the delta moved no row at all, so this case proves nothing"
        );
        for row in before.symmetric_difference(&after) {
            assert!(
                changed.contains(row),
                "{case}: {row:?} joined or left the RDF surface and changed_quads does not \
                 name it — a consumer filtering the change set by predicate would never \
                 re-examine it"
            );
        }
        // The owned freeze of the same mutation is the independent oracle for what
        // the surface became: a snapshot that disagrees with it would make the
        // comparison above a comparison of one implementation with itself.
        assert_eq!(
            after,
            base_surface(&mutation.freeze().unwrap()),
            "{case}: the snapshot surface and its owned freeze must agree"
        );
    }

    /// A base carrying a reifier declaration and one annotation on it, plus an
    /// unrelated plain row, in the default graph.
    fn reified_base() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://example.org/s");
        let p = b.intern_iri("https://example.org/p");
        let o = b.intern_iri("https://example.org/o");
        let r = b.intern_iri("https://example.org/r");
        let other = b.intern_iri("https://example.org/other");
        let triple = b.intern_triple(s, p, o);
        b.push_quad(s, p, o, None);
        b.push_quad(other, p, o, None);
        b.push_reifier_in_graph(r, triple, None);
        b.push_annotation_in_graph(r, p, other, None);
        b.freeze().unwrap()
    }

    /// The reifier declaration of [`reified_base`], as a value row.
    fn reifier_row() -> QuadValues {
        QuadValues {
            s: iri("r"),
            p: TermValue::Iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies".into()),
            o: TermValue::Triple {
                s: TermBox::new(iri("s")),
                p: TermBox::new(iri("p")),
                o: TermBox::new(iri("o")),
            },
            g: None,
        }
    }

    #[test]
    fn changed_quads_names_every_row_the_rdf12_overlay_moves_on_or_off_the_surface() {
        // 1. An ADDED reifier: it demotes the base rows about its subject out of
        //    the plain table. They stay on the surface as that reifier's
        //    annotations, so the only row that JOINED is the declaration itself.
        let base = reified_base();
        let mut mutation = MutableDataset::new(Arc::clone(&base));
        assert!(
            mutation
                .insert(QuadValues {
                    s: iri("other"),
                    ..reifier_row()
                })
                .unwrap()
        );
        assert_changed_quads_covers_the_surface_difference(&base, &mutation, "added reifier");

        // 2. A SUPPRESSED reifier: it promotes that reifier's annotations back
        //    into the plain table. Again a move, not a departure — the annotation
        //    is still on the surface, and only the declaration left it.
        let mut mutation = MutableDataset::new(Arc::clone(&base));
        assert!(mutation.remove(&reifier_row()));
        assert_changed_quads_covers_the_surface_difference(&base, &mutation, "suppressed reifier");

        // 3. Suppressing a reifier AND its annotation: the annotation really does
        //    leave, and it is named because it is suppressed in its own right.
        let mut mutation = MutableDataset::new(Arc::clone(&base));
        assert!(mutation.remove(&reifier_row()));
        assert!(mutation.remove(&QuadValues {
            s: iri("r"),
            p: iri("p"),
            o: iri("other"),
            g: None,
        }));
        assert_changed_quads_covers_the_surface_difference(
            &base,
            &mutation,
            "suppressed reifier and annotation",
        );

        // 4. A plain row added about an EXISTING reifier: it is admitted as that
        //    reifier's annotation rather than as a plain row, so the table it
        //    lands in is not the one a caller named — it is on the surface either
        //    way, and named either way.
        let mut mutation = MutableDataset::new(Arc::clone(&base));
        assert!(
            mutation
                .insert(QuadValues {
                    s: iri("r"),
                    p: iri("p"),
                    o: iri("s"),
                    g: None,
                })
                .unwrap()
        );
        assert_changed_quads_covers_the_surface_difference(
            &base,
            &mutation,
            "annotation of an existing reifier",
        );

        // 5. A reifier added for a subject that already carries plain rows AND an
        //    identical annotation, which is the one shape in which a demoted row
        //    is dropped rather than re-filed. It is still on the surface, via the
        //    annotation that masked it.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://example.org/s");
        let p = b.intern_iri("https://example.org/p");
        let o = b.intern_iri("https://example.org/o");
        let triple = b.intern_triple(s, p, o);
        b.push_quad(s, p, o, None);
        b.push_reifier_in_graph(s, triple, None);
        b.push_annotation_in_graph(s, p, o, None);
        let masked = b.freeze().unwrap();
        let mut mutation = MutableDataset::new(Arc::clone(&masked));
        assert!(
            mutation
                .insert(QuadValues {
                    s: iri("s"),
                    p: iri("p"),
                    o: iri("other"),
                    g: None,
                })
                .unwrap()
        );
        assert_changed_quads_covers_the_surface_difference(
            &masked,
            &mutation,
            "reifier masking an identical annotation",
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
            first.term_id_by_value(&iri("b")).unwrap(),
            base.as_ref()
                .term_id_by_value(&iri("b"))
                .map(DeltaViewId::Base)
        );
        let new_id = first.term_id_by_value(&iri("new")).unwrap().unwrap();
        assert!(matches!(new_id, DeltaViewId::Delta(_)));
        assert!(new_id.encode() < DeltaViewId::encode_computed(0));
        assert!(mutation.insert(row("a", "b")).unwrap());
        assert!(mutation.remove(&row("b", "new")));
        let second = mutation.snapshot_view().unwrap();
        assert_surface_equal(&second, &mutation.freeze().unwrap());
        assert_eq!(first.quads().count(), 1);
        assert_eq!(second.quads().count(), 1);
        assert!(first.term_id_by_value(&iri("new")).unwrap().is_some());
        assert!(second.term_id_by_value(&iri("new")).unwrap().is_none());
        assert_ne!(
            first.term_value(first.quads().next().unwrap().o),
            second.term_value(second.quads().next().unwrap().o),
        );
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
            .map(|s| view.term_id_by_value(&iri(s)).unwrap().unwrap())
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
                        assert!(view.cardinality_estimate(s, p, o, g) >= scan.len() as u64);
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
                    s: base.term_value(q.s).unwrap(),
                    p: base.term_value(q.p).unwrap(),
                    o: base.term_value(q.o).unwrap(),
                    g: q.g.map(|id| base.term_value(id).unwrap()),
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
        let r = view.term_id_by_value(&iri("reifier")).unwrap().unwrap();
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
