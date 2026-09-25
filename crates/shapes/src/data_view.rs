// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared native carriers for SHACL, with compact validation-local term handles.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use ::purrdf::ir::{
    CompositeDatasetView, DeltaDatasetView, QuadProbePlan, ViewLimits, import::DatasetImporter,
};
use ::purrdf::{
    BlankScope, DatasetView, FastMap, FastSet, GraphMatch, QuadIds, QuadRef, RdfDataset,
    RdfDatasetBuilder, RdfStoreCapabilities, RdfTextDirection, TermId, TermRef, TermValue,
};
use smallvec::SmallVec;

/// Native term lookup used by SHACL traversal, without an owned RDF row boundary.
/// Implementations preserve one validation-local `TermId` namespace.
pub trait ShaclRead: DatasetView<Id = TermId> + Sync {
    /// Look up an IRI by borrowed spelling.
    fn term_id_by_iri(&self, iri: &str) -> Option<TermId>;
    /// Look up a scope-qualified blank node.
    fn term_id_by_blank(&self, label: &str, scope: BlankScope) -> Option<TermId>;
    /// Look up the complete RDF 1.2 literal identity.
    fn term_id_by_literal(
        &self,
        lexical: &str,
        datatype: &str,
        language: Option<&str>,
        direction: Option<RdfTextDirection>,
    ) -> Option<TermId>;
    /// Look up a quoted triple within this view's term namespace.
    fn term_id_by_triple(&self, s: TermId, p: TermId, o: TermId) -> Option<TermId>;
}

impl ShaclRead for RdfDataset {
    fn term_id_by_iri(&self, iri: &str) -> Option<TermId> {
        self.term_id_by_iri(iri)
    }
    fn term_id_by_blank(&self, label: &str, scope: BlankScope) -> Option<TermId> {
        self.term_id_by_blank(label, scope)
    }
    fn term_id_by_literal(
        &self,
        lexical: &str,
        datatype: &str,
        language: Option<&str>,
        direction: Option<RdfTextDirection>,
    ) -> Option<TermId> {
        self.term_id_by_literal(lexical, datatype, language, direction)
    }
    fn term_id_by_triple(&self, s: TermId, p: TermId, o: TermId) -> Option<TermId> {
        self.term_id_by_triple(s, p, o)
    }
}

impl<T: ShaclRead + Send> ShaclRead for Arc<T> {
    fn term_id_by_iri(&self, iri: &str) -> Option<TermId> {
        self.as_ref().term_id_by_iri(iri)
    }
    fn term_id_by_blank(&self, label: &str, scope: BlankScope) -> Option<TermId> {
        self.as_ref().term_id_by_blank(label, scope)
    }
    fn term_id_by_literal(
        &self,
        lexical: &str,
        datatype: &str,
        language: Option<&str>,
        direction: Option<RdfTextDirection>,
    ) -> Option<TermId> {
        self.as_ref()
            .term_id_by_literal(lexical, datatype, language, direction)
    }
    fn term_id_by_triple(&self, s: TermId, p: TermId, o: TermId) -> Option<TermId> {
        self.as_ref().term_id_by_triple(s, p, o)
    }
}

/// Explicit work and retention dimensions for a prepared SHACL carrier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShaclViewStats {
    /// Compact term-handle mappings retained by this adapter.
    pub mapped_terms: usize,
    /// Conservative auxiliary allocation bound charged at construction.
    pub auxiliary_bytes: usize,
    /// Owned datasets materialized through a compatibility or function boundary.
    pub materializations: usize,
}

#[derive(Debug)]
struct Dense<D: DatasetView> {
    source: Arc<D>,
    ids: Box<[D::Id]>,
    local_ids: FastMap<D::Id, TermId>,
    auxiliary_bytes: usize,
}

impl<D: DatasetView> Dense<D> {
    fn new(
        source: Arc<D>,
        ids: impl FnOnce(&D) -> Box<[D::Id]>,
        limits: ViewLimits,
    ) -> Result<Self, String> {
        let count = source.term_count();
        let auxiliary_bytes = count
            .checked_mul(size_of::<D::Id>() * 3 + size_of::<TermId>() * 2 + 16)
            .ok_or("SHACL term mapping size overflow")?;
        if count > limits.max_terms
            || count >= u32::MAX as usize
            || auxiliary_bytes > limits.max_auxiliary_bytes
        {
            return Err("SHACL view exceeds term or auxiliary retention limit".to_owned());
        }
        let ids = ids(&source);
        let local_ids = ids
            .iter()
            .copied()
            .enumerate()
            .map(|(index, id)| {
                (
                    id,
                    TermId::from_index(u32::try_from(index).expect("admitted mapping fits")),
                )
            })
            .collect();
        Ok(Self {
            source,
            ids,
            local_ids,
            auxiliary_bytes,
        })
    }

    fn id(&self, source: D::Id) -> TermId {
        self.local_ids[&source]
    }
    fn source_id(&self, id: TermId) -> D::Id {
        self.ids[id.index()]
    }
    fn graph(&self, graph: GraphMatch) -> GraphMatch<D::Id> {
        match graph {
            GraphMatch::Any => GraphMatch::Any,
            GraphMatch::Default => GraphMatch::Default,
            GraphMatch::Named(id) => GraphMatch::Named(self.source_id(id)),
        }
    }
    fn quad(&self, row: QuadIds<D::Id>) -> QuadIds {
        QuadIds {
            s: self.id(row.s),
            p: self.id(row.p),
            o: self.id(row.o),
            g: row.g.map(|id| self.id(id)),
        }
    }
    fn resolve(&self, id: TermId) -> TermRef<'_> {
        match self.source.resolve(self.source_id(id)) {
            TermRef::Iri(iri) => TermRef::Iri(iri),
            TermRef::Blank { label, scope } => TermRef::Blank { label, scope },
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => TermRef::Literal {
                lexical,
                datatype: self.id(datatype),
                language,
                direction,
            },
            TermRef::Triple { s, p, o } => TermRef::Triple {
                s: self.id(s),
                p: self.id(p),
                o: self.id(o),
            },
        }
    }
}

#[derive(Debug)]
enum Source {
    Native(Arc<RdfDataset>),
    Composite(Dense<CompositeDatasetView>),
    Delta(Dense<DeltaDatasetView>),
}

/// Shared SHACL data view over native, composite or mutation-snapshot carriers.
///
/// The adapter maps compact term handles while borrowing original term payloads
/// and indexes. `projected` views flatten graphs and expose RDF 1.2 overlays as
/// ordinary SHACL data triples. The original carrier is retained by `Arc`; owned
/// materialization happens only when a caller asks for the compatibility dataset.
#[derive(Debug)]
pub struct ShaclDatasetView {
    source: Source,
    projected: bool,
    statements_projected: bool,
    /// Whether this source can put the SAME projected row on the wire twice, and
    /// therefore whether a probe has to dedup at all. Decided once, here, from
    /// facts about the carrier; see [`Self::source_can_duplicate`].
    source_can_duplicate: bool,
    materialized: OnceLock<Arc<RdfDataset>>,
    materializations: AtomicUsize,
}

impl ShaclDatasetView {
    /// Borrow one immutable native dataset, preserving its graphs and local IDs.
    #[must_use]
    pub fn native(source: Arc<RdfDataset>) -> Self {
        Self::new(Source::Native(source), false)
    }

    /// Read a native dataset through SHACL's graph-union and statement projection.
    #[must_use]
    pub fn project(source: Arc<RdfDataset>) -> Self {
        Self::new(Source::Native(source), true)
    }

    /// Read an admitted composite carrier without rebuilding its base dictionaries.
    /// `projected` selects SHACL's graph-union and statement projection.
    ///
    /// # Errors
    /// Refuses a validation-local handle mapping exceeding the supplied limits.
    pub fn composite(
        source: Arc<CompositeDatasetView>,
        projected: bool,
        limits: ViewLimits,
    ) -> Result<Self, String> {
        let dense = Dense::new(source, |source| source.term_ids().collect(), limits)?;
        Ok(Self::new(Source::Composite(dense), projected))
    }

    /// Read an admitted mutation snapshot, retaining its shared immutable base.
    /// `projected` selects SHACL's graph-union and statement projection.
    ///
    /// An unprojected delta view is a legitimate graph-scoped read of the
    /// snapshot, but it is NOT a carrier the incremental change path can be sound
    /// over: that path's guarantee is stated over the projected union, so
    /// `PreparedValidator::affected_focus_node_ids` refuses a binding whose
    /// [`Self::statements_projected`] answers `false`. Use
    /// `PreparedShapes::bind_delta_with_shapes_graph` for the change path.
    ///
    /// # Errors
    /// Refuses a validation-local handle mapping exceeding the supplied limits.
    pub fn delta(
        source: Arc<DeltaDatasetView>,
        projected: bool,
        limits: ViewLimits,
    ) -> Result<Self, String> {
        let dense = Dense::new(source, |source| source.term_ids().collect(), limits)?;
        Ok(Self::new(Source::Delta(dense), projected))
    }

    /// Whether reads through this view union the RDF 1.2 statement tables onto
    /// the plain stream — SHACL's statement projection.
    ///
    /// This is a property of the view, not of the carrier: a reifier or
    /// annotation row is READ here when this answers `true`, and is invisible to
    /// [`Self::quads_for_pattern_with_plan`] when it answers `false`. Exposed
    /// because it decides how wide this view's read surface is, and a consumer
    /// whose correctness argument is stated over that surface — the incremental
    /// change path is the one in this crate — has to be able to CHECK it rather
    /// than assume the constructor it was handed chose the wide one.
    #[must_use]
    pub const fn statements_projected(&self) -> bool {
        self.statements_projected
    }

    pub(crate) fn with_statement_projection(mut self) -> Self {
        self.statements_projected = true;
        self
    }

    fn new(source: Source, projected: bool) -> Self {
        let source_can_duplicate = Self::source_can_duplicate(&source);
        Self {
            source,
            projected,
            statements_projected: projected,
            source_can_duplicate,
            materialized: OnceLock::new(),
            materializations: AtomicUsize::new(0),
        }
    }

    /// Whether the statement projection over `source` can yield one row twice.
    ///
    /// There are exactly three ways it can, and a carrier that admits none of
    /// them needs no dedup — which matters because dedup is not free: a set that
    /// must hold a whole probe's rows is an allocation on the first row and a
    /// reallocation every time it doubles, charged to EVERY probe, and SHACL
    /// probes once per focus node per path step.
    ///
    /// 1. **The graph union.** Projection drops each row's graph slot, so one
    ///    triple asserted in two graphs collapses onto one row twice. Impossible
    ///    with no named graph.
    /// 2. **The RDF 1.2 overlay stream.** Reifier and annotation quads are
    ///    chained onto the base probe and may restate a row it already produced.
    ///    Impossible with no reifier and no annotation quad.
    /// 3. **The carrier itself.** A native [`RdfDataset`] answers a pattern from
    ///    an index over a deduplicated quad set, so its rows are distinct by
    ///    construction. A composite or delta carrier unions several bases, and
    ///    whether those bases overlap is not this adapter's fact to assume — so
    ///    those keep deduping unconditionally.
    ///
    /// Evaluated once per view, against the RAW source: [`Self::named_graphs`]
    /// and [`Self::reifier_quads`] are already projected (they answer empty under
    /// projection), so asking THEM would answer the wrong question and quietly
    /// switch dedup off for the carriers that need it most.
    fn source_can_duplicate(source: &Source) -> bool {
        let Source::Native(native) = source else {
            return true;
        };
        native.named_graphs().next().is_some()
            || native.reifier_quads().next().is_some()
            || native.annotation_quads().next().is_some()
    }

    /// Return an immutable owned dataset at an explicit compatibility boundary.
    /// Repeated requests share one materialization; ordinary validation never
    /// calls this method. Native unprojected views return their original `Arc`.
    #[must_use]
    pub fn materialized(&self) -> &Arc<RdfDataset> {
        if let Source::Native(source) = &self.source
            && !self.projected
            && !self.statements_projected
        {
            return source;
        }
        self.materialized.get_or_init(|| {
            let mut builder = RdfDatasetBuilder::new();
            DatasetImporter::new(&mut builder, self).append();
            let dataset = builder
                .freeze()
                .expect("known valid immutable carrier projection remains valid");
            self.materializations.fetch_add(1, Ordering::Relaxed);
            dataset
        })
    }

    /// The mutation snapshot this view reads, when it reads one.
    ///
    /// Exposed so the incremental change path can prove the delta a caller hands
    /// it is the delta this binding was built over. The alternative — trusting the
    /// caller — would answer with ids from one dataset about changes in another,
    /// and every one of those ids would be a valid index into the wrong table.
    pub(crate) fn delta_source(&self) -> Option<&Arc<DeltaDatasetView>> {
        match &self.source {
            Source::Delta(dense) => Some(&dense.source),
            Source::Native(_) | Source::Composite(_) => None,
        }
    }

    /// This view's own handle for a snapshot term, or `None` when the term is not
    /// one this view maps.
    pub(crate) fn local_delta_id(&self, id: ::purrdf::ir::DeltaViewId) -> Option<TermId> {
        match &self.source {
            Source::Delta(dense) => dense.local_ids.get(&id).copied(),
            Source::Native(_) | Source::Composite(_) => None,
        }
    }

    /// Retained handle mapping and explicit materialization work.
    #[must_use]
    pub fn stats(&self) -> ShaclViewStats {
        let (mapped_terms, auxiliary_bytes) = match &self.source {
            Source::Native(_) => (0, 0),
            Source::Composite(dense) => (dense.ids.len(), dense.auxiliary_bytes),
            Source::Delta(dense) => (dense.ids.len(), dense.auxiliary_bytes),
        };
        ShaclViewStats {
            mapped_terms,
            auxiliary_bytes,
            materializations: self.materializations.load(Ordering::Relaxed),
        }
    }

    /// Probe with a plan prepared by [`DatasetView::probe_plan`]. The source
    /// pattern, including any graph-union transformation, is planned once and
    /// the supplied plan is forwarded to the retained native indexes.
    pub fn quads_for_pattern_with_plan(
        &self,
        plan: &QuadProbePlan,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ + use<'_> {
        let source_graph = if self.projected { GraphMatch::Any } else { g };
        let allowed = !self.projected || !matches!(g, GraphMatch::Named(_));
        let overlays = self
            .statements_projected
            .then(|| self.raw_overlay_probe(s, p, o, source_graph))
            .into_iter()
            .flatten();
        let dedup = self.statements_projected && self.source_can_duplicate;
        let mut seen = ProjectionDedup::default();
        self.raw_probe(*plan, s, p, o, source_graph)
            .chain(overlays)
            .filter_map(move |mut q| {
                if !allowed {
                    return None;
                }
                if self.projected {
                    q.g = None;
                }
                if dedup && !seen.insert(q) {
                    return None;
                }
                Some(q)
            })
    }
    fn raw_probe(
        &self,
        plan: QuadProbePlan,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        match &self.source {
            Source::Native(source) => Either::Left(
                source
                    .as_ref()
                    .quads_for_pattern_with_plan(&plan, s, p, o, g),
            ),
            Source::Composite(dense) => Either::Right(boxed(
                dense
                    .source
                    .as_ref()
                    .quads_for_pattern_with_plan(
                        &plan,
                        s.map(|id| dense.source_id(id)),
                        p.map(|id| dense.source_id(id)),
                        o.map(|id| dense.source_id(id)),
                        dense.graph(g),
                    )
                    .map(|row| dense.quad(row)),
            )),
            Source::Delta(dense) => Either::Right(boxed(
                dense
                    .source
                    .as_ref()
                    .quads_for_pattern_with_plan(
                        &plan,
                        s.map(|id| dense.source_id(id)),
                        p.map(|id| dense.source_id(id)),
                        o.map(|id| dense.source_id(id)),
                        dense.graph(g),
                    )
                    .map(|row| dense.quad(row)),
            )),
        }
    }
    fn raw_overlay_probe(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        match &self.source {
            Source::Native(source) => Either::Left(overlay_probe(source.as_ref(), s, p, o, g)),
            Source::Composite(dense) => Either::Right(dense_overlay_probe(dense, s, p, o, g)),
            Source::Delta(dense) => Either::Right(dense_overlay_probe(dense, s, p, o, g)),
        }
    }
    fn raw_reifiers(&self) -> impl Iterator<Item = QuadIds> + '_ {
        match &self.source {
            Source::Native(source) => Either::Left(source.reifier_quads()),
            Source::Composite(dense) => {
                Either::Right(boxed(dense.source.reifier_quads().map(|q| dense.quad(q))))
            }
            Source::Delta(dense) => {
                Either::Right(boxed(dense.source.reifier_quads().map(|q| dense.quad(q))))
            }
        }
    }
    fn raw_annotations(&self) -> impl Iterator<Item = QuadIds> + '_ {
        match &self.source {
            Source::Native(source) => Either::Left(source.annotation_quads()),
            Source::Composite(dense) => Either::Right(boxed(
                dense.source.annotation_quads().map(|q| dense.quad(q)),
            )),
            Source::Delta(dense) => Either::Right(boxed(
                dense.source.annotation_quads().map(|q| dense.quad(q)),
            )),
        }
    }
}

/// First-seen membership over the rows one projected probe has already yielded,
/// allocation-free while that row set is small.
///
/// The statement projection has to dedup: the graph union collapses the same
/// triple asserted in two graphs onto one row, and the RDF 1.2 overlay stream can
/// restate a row the base probe already produced. A `HashSet` is the right shape
/// for that, and it is also one heap allocation on its FIRST insert — charged to
/// every probe that matches even a single quad. SHACL evaluates a path and a
/// class membership per focus node, so that was one allocation per probe per
/// focus node for a set that almost always holds one or two rows.
///
/// So membership is answered by a linear scan over an inline row buffer until it
/// reaches [`Self::LINEAR_MAX`], and only a probe that really is wide pays for a
/// table. Both regimes answer the same question in the same order, so the rows a
/// probe yields and the order it yields them in are unchanged; the asymptotics
/// are unchanged too, because the scanned regime is bounded by a constant.
#[derive(Default)]
struct ProjectionDedup {
    /// The rows admitted so far, while the scan is still the cheaper answer.
    inline: SmallVec<[QuadIds; Self::LINEAR_MAX]>,
    /// The hashed set, once the row set has outgrown the scan. `None` until then,
    /// and a `None` here has never allocated.
    hashed: Option<FastSet<QuadIds>>,
}

impl ProjectionDedup {
    /// The row count past which probing switches from a linear scan to a hash
    /// lookup. A scan of this many 16-byte `Copy` rows is a few cache lines and
    /// beats hashing one; past it the scan would cost more than the allocation it
    /// avoids.
    const LINEAR_MAX: usize = 16;

    /// Whether `quad` is the first occurrence of that row in this probe.
    fn insert(&mut self, quad: QuadIds) -> bool {
        if let Some(set) = &mut self.hashed {
            return set.insert(quad);
        }
        if self.inline.contains(&quad) {
            return false;
        }
        if self.inline.len() < Self::LINEAR_MAX {
            self.inline.push(quad);
            return true;
        }
        let mut set: FastSet<QuadIds> = FastSet::with_capacity_and_hasher(
            self.inline.len() * 2,
            ::purrdf::FastHasher::default(),
        );
        set.extend(self.inline.iter().copied());
        let fresh = set.insert(quad);
        self.hashed = Some(set);
        fresh
    }
}

fn overlay_probe<D: DatasetView>(
    source: &D,
    s: Option<D::Id>,
    p: Option<D::Id>,
    o: Option<D::Id>,
    g: GraphMatch<D::Id>,
) -> impl Iterator<Item = QuadIds<D::Id>> + '_ {
    let reifiers = s
        .into_iter()
        .flat_map(|subject| source.reifier_quads_of(subject))
        .chain(
            source
                .reifier_quads()
                .take(if s.is_none() { usize::MAX } else { 0 }),
        );
    let annotations = s
        .into_iter()
        .flat_map(|subject| {
            source
                .annotations_of_with_graph(subject)
                .map(move |(p, o, g)| QuadIds {
                    s: subject,
                    p,
                    o,
                    g,
                })
        })
        .chain(
            source
                .annotation_quads()
                .take(if s.is_none() { usize::MAX } else { 0 }),
        );
    reifiers.chain(annotations).filter(move |q| {
        p.is_none_or(|id| q.p == id) && o.is_none_or(|id| q.o == id) && g.matches(q.g)
    })
}

// Composite and delta iterators contain several indexed source alternatives.
// Erasing just those branches bounds callers' stack frames while native probes
// keep their allocation-free iterator representation.
/// The RDF 1.2 overlay probe of a dense (composite or delta) carrier, boxed.
///
/// Out of line, one instance per carrier type, so each carrier's overlay
/// iterator is built in its own stack frame. Built inline in
/// `raw_overlay_probe`, both carriers' iterators, each a deep nest of
/// chained and flattened pattern scans, occupied that one frame together,
/// although a call only ever builds one of them.
#[inline(never)]
fn dense_overlay_probe<D: DatasetView>(
    dense: &Dense<D>,
    s: Option<TermId>,
    p: Option<TermId>,
    o: Option<TermId>,
    g: GraphMatch,
) -> Box<dyn Iterator<Item = QuadIds> + '_> {
    boxed(
        overlay_probe(
            dense.source.as_ref(),
            s.map(|id| dense.source_id(id)),
            p.map(|id| dense.source_id(id)),
            o.map(|id| dense.source_id(id)),
            dense.graph(g),
        )
        .map(|q| dense.quad(q)),
    )
}

fn boxed<'a, T>(iter: impl Iterator<Item = T> + 'a) -> Box<dyn Iterator<Item = T> + 'a> {
    Box::new(iter)
}

enum Either<A, B> {
    Left(A),
    Right(B),
}
impl<T, A: Iterator<Item = T>, B: Iterator<Item = T>> Iterator for Either<A, B> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self {
            Self::Left(iter) => iter.next(),
            Self::Right(iter) => iter.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Left(iter) => iter.size_hint(),
            Self::Right(iter) => iter.size_hint(),
        }
    }
}

impl DatasetView for ShaclDatasetView {
    type Id = TermId;
    type ProbePlan = QuadProbePlan;
    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.quads_for_pattern(None, None, None, GraphMatch::Any)
    }
    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_>> + '_ {
        self.quads().map(|q| QuadRef {
            s: self.resolve(q.s),
            p: self.resolve(q.p),
            o: self.resolve(q.o),
            g: q.g.map(|id| self.resolve(id)),
        })
    }
    fn resolve(&self, id: TermId) -> TermRef<'_> {
        match &self.source {
            Source::Native(source) => source.resolve(id),
            Source::Composite(dense) => dense.resolve(id),
            Source::Delta(dense) => dense.resolve(id),
        }
    }
    fn quads_for_pattern(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        let plan = self.probe_plan(s.is_some(), p.is_some(), o.is_some(), g);
        self.quads_for_pattern_with_plan(&plan, s, p, o, g)
    }
    fn term_id_by_value(&self, value: &TermValue) -> Option<TermId> {
        match &self.source {
            Source::Native(source) => source.term_id_by_value(value),
            Source::Composite(dense) => dense.source.term_id_by_value(value).map(|id| dense.id(id)),
            Source::Delta(dense) => dense.source.term_id_by_value(value).map(|id| dense.id(id)),
        }
    }
    fn capabilities(&self) -> RdfStoreCapabilities {
        match &self.source {
            Source::Native(source) => source.capabilities(),
            Source::Composite(dense) => dense.source.capabilities(),
            Source::Delta(dense) => dense.source.capabilities(),
        }
    }
    fn probe_plan(&self, s: bool, p: bool, o: bool, g: GraphMatch) -> QuadProbePlan {
        // The union projection removes the physical graph constraint. Plan for
        // that source pattern once, before the plan is reused across probe rows.
        let source_graph = if self.projected { GraphMatch::Any } else { g };
        RdfDataset::probe_plan(s, p, o, source_graph)
    }
    fn quads_for_pattern_with_plan(
        &self,
        plan: &QuadProbePlan,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        Self::quads_for_pattern_with_plan(self, plan, s, p, o, g)
    }
    fn len_hint(&self) -> Option<usize> {
        if self.projected || self.statements_projected {
            return None;
        }
        match &self.source {
            Source::Native(source) => Some(source.quad_count()),
            Source::Composite(dense) => dense.source.len_hint(),
            Source::Delta(dense) => dense.source.len_hint(),
        }
    }
    fn cardinality_estimate(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> usize {
        if self.projected || self.statements_projected {
            return self.quads_for_pattern(s, p, o, g).count();
        }
        match &self.source {
            Source::Native(source) => source.cardinality_estimate(s, p, o, g),
            Source::Composite(dense) => dense.source.cardinality_estimate(
                s.map(|id| dense.source_id(id)),
                p.map(|id| dense.source_id(id)),
                o.map(|id| dense.source_id(id)),
                dense.graph(g),
            ),
            Source::Delta(dense) => dense.source.cardinality_estimate(
                s.map(|id| dense.source_id(id)),
                p.map(|id| dense.source_id(id)),
                o.map(|id| dense.source_id(id)),
                dense.graph(g),
            ),
        }
    }
    fn stats_fingerprint(&self) -> u64 {
        let source = match &self.source {
            Source::Native(source) => source.stats_fingerprint(),
            Source::Composite(dense) => dense.source.stats_fingerprint(),
            Source::Delta(dense) => dense.source.stats_fingerprint(),
        };
        source ^ (u64::from(self.projected) << 62) ^ (u64::from(self.statements_projected) << 63)
    }
    fn term_count(&self) -> usize {
        match &self.source {
            Source::Native(source) => source.term_count(),
            Source::Composite(dense) => dense.ids.len(),
            Source::Delta(dense) => dense.ids.len(),
        }
    }
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.raw_reifiers().take(if self.statements_projected {
            0
        } else {
            usize::MAX
        })
    }
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.raw_annotations().take(if self.statements_projected {
            0
        } else {
            usize::MAX
        })
    }
    fn annotations_of_with_graph(
        &self,
        reifier: TermId,
    ) -> impl Iterator<Item = (TermId, TermId, Option<TermId>)> + '_ {
        self.annotation_quads()
            .filter(move |q| q.s == reifier)
            .map(|q| (q.p, q.o, q.g))
    }
    fn named_graphs(&self) -> impl Iterator<Item = TermId> + '_ {
        let graphs = match &self.source {
            Source::Native(source) => Either::Left(source.named_graphs()),
            Source::Composite(dense) => {
                Either::Right(boxed(dense.source.named_graphs().map(|id| dense.id(id))))
            }
            Source::Delta(dense) => {
                Either::Right(boxed(dense.source.named_graphs().map(|id| dense.id(id))))
            }
        };
        graphs.filter(|_| !self.projected)
    }
}

impl ShaclRead for ShaclDatasetView {
    fn term_id_by_iri(&self, iri: &str) -> Option<TermId> {
        match &self.source {
            Source::Native(source) => source.term_id_by_iri(iri),
            _ => self.term_id_by_value(&TermValue::iri(iri)),
        }
    }
    fn term_id_by_blank(&self, label: &str, scope: BlankScope) -> Option<TermId> {
        if let Source::Native(source) = &self.source {
            return source.term_id_by_blank(label, scope);
        }
        self.term_id_by_value(&TermValue::Blank {
            label: label.to_owned(),
            scope,
        })
    }
    fn term_id_by_literal(
        &self,
        lexical: &str,
        datatype: &str,
        language: Option<&str>,
        direction: Option<RdfTextDirection>,
    ) -> Option<TermId> {
        if let Source::Native(source) = &self.source {
            return source.term_id_by_literal(lexical, datatype, language, direction);
        }
        self.term_id_by_value(&TermValue::Literal {
            lexical_form: lexical.to_owned(),
            datatype: datatype.to_owned(),
            language: language.map(str::to_owned),
            direction,
        })
    }
    fn term_id_by_triple(&self, s: TermId, p: TermId, o: TermId) -> Option<TermId> {
        if let Source::Native(source) = &self.source {
            return source.term_id_by_triple(s, p, o);
        }
        self.term_id_by_value(&TermValue::Triple {
            s: Box::new(crate::term::term_id_to_native(self, s).to_term_value()),
            p: Box::new(crate::term::term_id_to_native(self, p).to_term_value()),
            o: Box::new(crate::term::term_id_to_native(self, o).to_term_value()),
        })
    }
}
