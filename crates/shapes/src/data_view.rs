// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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

    pub(crate) fn with_statement_projection(mut self) -> Self {
        self.statements_projected = true;
        self
    }

    fn new(source: Source, projected: bool) -> Self {
        Self {
            source,
            projected,
            statements_projected: projected,
            materialized: OnceLock::new(),
            materializations: AtomicUsize::new(0),
        }
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
        let mut seen = FastSet::default();
        self.raw_probe(*plan, s, p, o, source_graph)
            .chain(overlays)
            .filter_map(move |mut q| {
                if !allowed {
                    return None;
                }
                if self.projected {
                    q.g = None;
                }
                if self.statements_projected && !seen.insert(q) {
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
            Source::Composite(dense) => Either::Right(boxed(
                overlay_probe(
                    dense.source.as_ref(),
                    s.map(|id| dense.source_id(id)),
                    p.map(|id| dense.source_id(id)),
                    o.map(|id| dense.source_id(id)),
                    dense.graph(g),
                )
                .map(|q| dense.quad(q)),
            )),
            Source::Delta(dense) => Either::Right(boxed(
                overlay_probe(
                    dense.source.as_ref(),
                    s.map(|id| dense.source_id(id)),
                    p.map(|id| dense.source_id(id)),
                    o.map(|id| dense.source_id(id)),
                    dense.graph(g),
                )
                .map(|q| dense.quad(q)),
            )),
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
