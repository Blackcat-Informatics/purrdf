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

/// How one source's blank node scopes bind to the composite's blank identity space.
///
/// Blank node identity is scoped, never global: the same label in two documents
/// names two different nodes unless a caller states otherwise. The binding is that
/// statement, made per source, so one composite can hold a delta that co-refers
/// with its own base beside an independently parsed contribution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScopeBinding {
    /// Standardize this source's blank scopes apart from every other source.
    #[default]
    Independent,
    /// Retain the supplied blank scopes. Equal label and scope name one node,
    /// whichever co-referent source contributed the occurrence.
    Shared,
}

#[derive(Debug, Clone)]
enum Carrier {
    Native(Arc<RdfDataset>),
    Delta(Arc<DeltaDatasetView>),
    Selected(Arc<SelectedGraphs>),
}

/// One retained immutable source with its explicit graph placement and blank
/// scope binding.
#[derive(Debug, Clone)]
pub struct CompositeSource {
    carrier: Carrier,
    placement: GraphPlacement,
    binding: ScopeBinding,
    literals: Arc<ReboundLiterals>,
}

impl CompositeSource {
    /// Retain a frozen native dataset, with its graph placement unchanged.
    #[must_use]
    pub fn new(dataset: Arc<RdfDataset>) -> Self {
        Self {
            carrier: Carrier::Native(dataset),
            placement: GraphPlacement::Preserve,
            binding: ScopeBinding::Independent,
            literals: Arc::default(),
        }
    }
    /// Retain a delta snapshot without compacting its base.
    #[must_use]
    pub fn from_delta(view: Arc<DeltaDatasetView>) -> Self {
        Self {
            carrier: Carrier::Delta(view),
            placement: GraphPlacement::Preserve,
            binding: ScopeBinding::Independent,
            literals: Arc::default(),
        }
    }
    /// Retain a SELECTION of one composite's named graphs as a composable source,
    /// without flattening it and without copying a row, a string or a source handle.
    ///
    /// ## One selection, all four layers
    ///
    /// Selection is BY GRAPH and it is a single statement over everything a graph
    /// holds: an ordinary quad, a reifier declaration, an annotation row and the
    /// graph DECLARATION itself are each visible through this source exactly when
    /// the graph they belong to was named, and invisible through every accessor
    /// when it was not. A selected graph that holds no rows survives as what it was
    /// — a declaration — so [`DatasetView::named_graphs`] over the composite this
    /// source joins reports exactly the selected names.
    ///
    /// Rows in the retained composite's DEFAULT graph belong to no named graph and
    /// are therefore never selected: `graphs` names graphs, and the default graph
    /// has no name to give.
    ///
    /// ## Selection is not placement
    ///
    /// Select first, place after. A selected source accepts
    /// [`with_graph_placement`](Self::with_graph_placement) and
    /// [`with_scope_binding`](Self::with_scope_binding) exactly like a native or
    /// delta source; with the default [`GraphPlacement::Preserve`] the selected
    /// graphs keep their own names.
    ///
    /// ## Nothing is re-scoped, and nothing is re-owned
    ///
    /// Blank identity is answered by the retained composite in its own canonical
    /// space and handed through verbatim, so occurrences that co-refer inside the
    /// composite still co-refer inside the selection. The composite this source is
    /// later composed INTO is what decides whether that space is shared with its
    /// siblings, through the ordinary [`ScopeBinding`] this source carries. The
    /// underlying owners are RETAINED, not copied, so a
    /// [`RetentionLedger`](crate::RetentionLedger) that already sees one of them
    /// reports it once between the two carriers.
    ///
    /// # Errors
    /// `view-graph-name` if a selection entry is neither an IRI nor a blank node;
    /// `view-graph-selection` if an entry names a graph this composite does not
    /// hold (a declaration-only graph DOES count as held); `view-retention-limit`
    /// if the retained owners plus this projection's own bookkeeping exceed
    /// `limits`.
    pub fn from_selection(
        view: Arc<CompositeDatasetView>,
        graphs: impl IntoIterator<Item = TermValue>,
        limits: ViewLimits,
    ) -> Result<Self, RdfDiagnostic> {
        Ok(Self {
            carrier: Carrier::Selected(Arc::new(SelectedGraphs::project(view, graphs, limits)?)),
            placement: GraphPlacement::Preserve,
            binding: ScopeBinding::Independent,
            literals: Arc::default(),
        })
    }
    /// Select graph placement for this source's complete RDF surface.
    #[must_use]
    pub fn with_graph_placement(mut self, placement: GraphPlacement) -> Self {
        self.placement = placement;
        self
    }
    /// Select whether this source's blank scopes are standardized apart or
    /// co-referent with the composite's base blank identity space.
    #[must_use]
    pub fn with_scope_binding(mut self, binding: ScopeBinding) -> Self {
        self.binding = binding;
        self
    }
    /// The blank scope binding this source contributes under.
    #[must_use]
    pub const fn scope_binding(&self) -> ScopeBinding {
        self.binding
    }
    /// Original native source, including source-owned sidecars, when applicable.
    #[must_use]
    pub fn dataset(&self) -> Option<&Arc<RdfDataset>> {
        match &self.carrier {
            Carrier::Native(ds) => Some(ds),
            Carrier::Delta(_) | Carrier::Selected(_) => None,
        }
    }
    /// Original delta snapshot and its base-sidecar owner, when applicable.
    #[must_use]
    pub fn delta(&self) -> Option<&Arc<DeltaDatasetView>> {
        match &self.carrier {
            Carrier::Native(_) | Carrier::Selected(_) => None,
            Carrier::Delta(ds) => Some(ds),
        }
    }
    /// Original composite and the graph names selected from it, when applicable.
    /// The names are deduplicated and in canonical value order, so two selections
    /// that named the same graphs report the same list whatever order they were
    /// written in.
    #[must_use]
    pub fn selection(&self) -> Option<(&Arc<CompositeDatasetView>, &[TermValue])> {
        self.selected()
            .map(|selection| (&selection.view, &*selection.names))
    }
    fn native(&self) -> Option<&RdfDataset> {
        self.dataset().map(AsRef::as_ref)
    }
    fn delta_ref(&self) -> Option<&DeltaDatasetView> {
        self.delta().map(AsRef::as_ref)
    }
    fn selected(&self) -> Option<&Arc<SelectedGraphs>> {
        match &self.carrier {
            Carrier::Selected(selection) => Some(selection),
            Carrier::Native(_) | Carrier::Delta(_) => None,
        }
    }
    fn term_ids(&self) -> impl Iterator<Item = LocalId> + '_ {
        // A selection speaks the same dense `Base(index)` vocabulary a native
        // source does: its table is the projected handles, in the retained
        // composite's own ascending id order, so the local numbering is a function
        // of that view and of nothing transient.
        let dense = match &self.carrier {
            Carrier::Native(ds) => ds.term_count(),
            Carrier::Selected(selection) => selection.ids.len(),
            Carrier::Delta(_) => 0,
        };
        (0..dense)
            .map(|i| {
                LocalId::Base(TermId::from_index(
                    u32::try_from(i).expect("source index fits u32"),
                ))
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
            Carrier::Selected(selection) => selection.ids.len(),
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
            Carrier::Selected(selection) => {
                let inner = selection
                    .inner(id)
                    .expect("selected source requires one of its own handles");
                // The scope arm is the identity ON PURPOSE: the retained composite
                // has already resolved this term into its canonical blank space and
                // selection is not a renaming of it.
                map_term(
                    selection.view.resolve(inner),
                    |id| selection.local(id),
                    |scope| scope,
                )
            }
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
            Carrier::Selected(selection) => selection.lookup(&TermValue::iri(iri)),
        }
    }
    fn lookup_blank(&self, label: &str, scope: BlankScope) -> Option<LocalId> {
        match &self.carrier {
            Carrier::Native(ds) => ds.term_id_by_blank(label, scope).map(LocalId::Base),
            Carrier::Delta(ds) => ds.lookup_blank(label, scope),
            Carrier::Selected(selection) => selection.lookup(&TermValue::Blank {
                label: label.to_owned(),
                scope,
            }),
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
            Carrier::Selected(selection) => selection.lookup(&TermValue::Literal {
                lexical_form: lexical.to_owned(),
                datatype: datatype.to_owned(),
                language: language.map(str::to_owned),
                direction,
            }),
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
            Carrier::Selected(selection) => {
                // A triple term is addressed by VALUE here, as everywhere else on
                // this seam: the retained composite owns the component identities
                // and only it can say which of its handles the whole term is.
                let component = |id: LocalId| {
                    selection
                        .inner(id)
                        .map(|inner| Box::new(owned_value(&*selection.view, inner)))
                };
                selection.lookup(&TermValue::Triple {
                    s: component(s)?,
                    p: component(p)?,
                    o: component(o)?,
                })
            }
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
        // ONE selection over both statement layers: a row is visible exactly when
        // its OWN graph slot was selected, which is the same test the ordinary
        // quads run — the statement layer is keyed per graph in this IR, so a
        // reifier declared in two graphs contributes only the selected one's row.
        // Every pull out of the retained composite is type-erased for the same
        // reason `probe_graph` is: a selection's rows come FROM a composite, so
        // a transparent pull here would put the composite's opaque iterator
        // inside itself.
        type ErasedRows<'a> = Box<dyn Iterator<Item = QuadIds<CompositeViewId>> + 'a>;
        let selected = self.selected().into_iter().flat_map(move |selection| {
            let view: &CompositeDatasetView = &selection.view;
            let narrowed = subject.and_then(|id| selection.inner(id));
            narrowed
                .into_iter()
                .filter(move |_| reifiers)
                .flat_map(move |s| -> ErasedRows<'_> { Box::new(view.reifier_quads_of(s)) })
                .chain(
                    std::iter::once(view)
                        .filter(move |_| reifiers && subject.is_none())
                        .flat_map(|view| -> ErasedRows<'_> { Box::new(view.reifier_quads()) }),
                )
                .chain(narrowed.into_iter().filter(move |_| annotations).flat_map(
                    move |s| -> ErasedRows<'_> {
                        Box::new(
                            view.annotations_of_with_graph(s)
                                .map(move |(p, o, g)| QuadIds { s, p, o, g }),
                        )
                    },
                ))
                .chain(
                    std::iter::once(view)
                        .filter(move |_| annotations && subject.is_none())
                        .flat_map(|view| -> ErasedRows<'_> { Box::new(view.annotation_quads()) }),
                )
                .filter(move |q| q.g.is_some_and(|graph| selection.graphs.contains(&graph)))
                .map(move |q| map_quad(q, |id| selection.local(id)))
        });
        native.chain(delta).chain(selected)
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
        // The projection reads the retained composite ONE selected graph at a time,
        // so the physical pattern is graph-bound however the logical one was
        // spelled. The caller's plan describes a different graph-boundness and a
        // graph-bound prefix chosen for it would name the wrong rows, so this axis
        // is re-planned here — the same reason placement re-plans its sources.
        let selected_plan = physical_plan([s.is_some(), p.is_some(), o.is_some(), true]);
        let selected = self
            .selected()
            .filter(move |_| ordinary)
            .into_iter()
            .flat_map(move |selection| {
                selection
                    .probe_pattern((s, p, o, g))
                    .into_iter()
                    .flat_map(move |(s, p, o, only)| {
                        selection.targets(only).flat_map(move |graph| {
                            selection.probe_graph(selected_plan, s, p, o, graph)
                        })
                    })
                    .map(move |q| map_quad(q, |id| selection.local(id)))
            });
        native.chain(delta).chain(selected).chain(
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
            Carrier::Selected(selection) => {
                selection
                    .probe_pattern((s, p, o, g))
                    .map_or(0, |(s, p, o, only)| {
                        selection.targets(only).fold(0_usize, |total, graph| {
                            total.saturating_add(selection.view.cardinality_estimate(
                                s,
                                p,
                                o,
                                GraphMatch::Named(graph),
                            ))
                        })
                    })
            }
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
            .chain(self.selected().into_iter().flat_map(|selection| {
                // Exactly the selected names, declaration-only ones included.
                selection
                    .graphs
                    .iter()
                    .map(move |&graph| selection.local(graph))
            }))
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
            Carrier::Selected(selection) => {
                // The SAME owners the retained composite holds, charged the way it
                // charges them — a selection keeps every one of them alive and is
                // honest about that. Deduplication across carriers is the
                // RetentionLedger's question, not admission's: `ViewStats` is
                // deliberately per-view and unconditional, so discounting a base
                // because the inner view also names it would understate this
                // source's own ceilings.
                for source in &*selection.view.sources {
                    source.retain(stats);
                }
                stats.auxiliary_bytes = stats
                    .auxiliary_bytes
                    .saturating_add(selection.view.stats.auxiliary_bytes)
                    .saturating_add(selection.ids.len().saturating_mul(selection_term_bytes()));
            }
        }
    }
}

#[derive(Debug, Default)]
struct ReboundLiterals {
    values: FastMap<LocalId, String>,
    index: hashbrown::HashTable<LocalId>,
}

/// One composite view projected onto a chosen set of its named graphs.
///
/// The projection is a RENAMING, never a rebuild. [`ids`](Self::ids) is the dense
/// table of the retained view's canonical handles the selected rows reach, so a
/// selected source speaks the ordinary `LocalId::Base(index)` vocabulary every
/// other source speaks, while every VALUE — IRI text, blank label and blank scope
/// alike — is still answered by the retained view itself. No dictionary is built,
/// no string is copied and no row is moved.
///
/// # Why co-reference survives
///
/// Two selected occurrences that name one node inside the composite resolve
/// through ONE canonical handle there, so they land on one entry of this table and
/// remain one term here. Nothing in this projection reads, mints or renumbers a
/// [`BlankScope`]: the scopes it hands out are the retained view's canonical ones
/// verbatim. The composite this source is composed INTO is therefore the only
/// thing that can rename them, through the ordinary [`ScopeBinding`] every source
/// carries — [`Shared`](ScopeBinding::Shared) keeps them, so the selection
/// co-refers with anything else composed over the same identity space, and
/// [`Independent`](ScopeBinding::Independent) standardizes the WHOLE selection
/// apart under one injective map, which preserves internal co-reference by
/// construction.
#[derive(Debug)]
struct SelectedGraphs {
    /// The retained composite. Its sources — and therefore its owners — are kept
    /// alive by this handle; none of them is copied.
    view: Arc<CompositeDatasetView>,
    /// The selected graph names, deduplicated, in canonical value order.
    names: Arc<[TermValue]>,
    /// The selected graphs' canonical handles in [`view`](Self::view), ascending.
    graphs: BTreeSet<CompositeViewId>,
    /// Local index -> canonical handle, ascending.
    ids: Vec<CompositeViewId>,
    /// The inverse of [`ids`](Self::ids).
    index: FastMap<CompositeViewId, TermId>,
    /// What the SELECTED subset exposes — not what the whole composite does.
    capabilities: RdfStoreCapabilities,
}

impl SelectedGraphs {
    fn project(
        view: Arc<CompositeDatasetView>,
        graphs: impl IntoIterator<Item = TermValue>,
        limits: ViewLimits,
    ) -> Result<Self, RdfDiagnostic> {
        let inner: &CompositeDatasetView = &view;
        // A declaration-only graph is HELD, so it is selectable: the whole point of
        // carrying a selection forward is that it says the same thing the composite
        // said about those graphs, and "this graph exists and is empty" is one of
        // the things it said.
        let held: BTreeSet<CompositeViewId> = inner.named_graphs().collect();
        let mut selected: BTreeSet<CompositeViewId> = BTreeSet::new();
        let mut names: Vec<TermValue> = Vec::new();
        for name in graphs {
            if !matches!(name, TermValue::Iri(_) | TermValue::Blank { .. }) {
                return Err(RdfDiagnostic::error(
                    "view-graph-name",
                    "view graph selection requires an IRI or blank node",
                ));
            }
            let Some(graph) = inner.term_id_by_value(&name).filter(|id| held.contains(id)) else {
                return Err(RdfDiagnostic::error(
                    "view-graph-selection",
                    "view graph selection names a graph this composite does not hold",
                ));
            };
            if selected.insert(graph) {
                names.push(name);
            }
        }
        names.sort();

        // Every handle a selected row reaches, the selected graph NAMES included so
        // an empty selected graph still has a name to declare.
        let mut ids: BTreeSet<CompositeViewId> = selected.clone();
        let mut reifiers = false;
        let mut annotations = false;
        for &graph in &selected {
            for q in inner.quads_for_pattern(None, None, None, GraphMatch::Named(graph)) {
                ids.extend([q.s, q.p, q.o]);
                ids.extend(q.g);
            }
        }
        for q in inner.reifier_quads() {
            if q.g.is_some_and(|graph| selected.contains(&graph)) {
                reifiers = true;
                ids.extend([q.s, q.p, q.o]);
                ids.extend(q.g);
            }
        }
        for q in inner.annotation_quads() {
            if q.g.is_some_and(|graph| selected.contains(&graph)) {
                annotations = true;
                ids.extend([q.s, q.p, q.o]);
                ids.extend(q.g);
            }
        }

        // A literal's datatype and a triple term's components are referenced BY
        // HANDLE, so the projection must close over them or `resolve` would name a
        // term this table does not hold. Bounded: the set only grows and can never
        // exceed the retained view's own term count.
        let mut queue: Vec<CompositeViewId> = ids.iter().copied().collect();
        let mut quoted = false;
        while let Some(id) = queue.pop() {
            match inner.resolve(id) {
                TermRef::Literal { datatype, .. } => {
                    if ids.insert(datatype) {
                        queue.push(datatype);
                    }
                }
                TermRef::Triple { s, p, o } => {
                    quoted = true;
                    for component in [s, p, o] {
                        if ids.insert(component) {
                            queue.push(component);
                        }
                    }
                }
                TermRef::Iri(_) | TermRef::Blank { .. } => {}
            }
        }

        let ids: Vec<CompositeViewId> = ids.into_iter().collect();
        if u32::try_from(ids.len()).is_err() {
            return Err(RdfDiagnostic::error(
                "view-term-count",
                "selected term count exceeds the source handle space",
            ));
        }
        let index: FastMap<CompositeViewId, TermId> = ids
            .iter()
            .enumerate()
            .map(|(local, &id)| {
                (
                    id,
                    TermId::from_index(u32::try_from(local).expect("checked against u32 above")),
                )
            })
            .collect();

        let mut stats = ViewStats::default();
        for source in &*inner.sources {
            source.retain(&mut stats);
        }
        stats.auxiliary_bytes = stats
            .auxiliary_bytes
            .saturating_add(inner.stats.auxiliary_bytes)
            .saturating_add(ids.len().saturating_mul(selection_term_bytes()))
            .saturating_add(
                selected
                    .len()
                    .saturating_mul(4 * size_of::<CompositeViewId>()),
            )
            .saturating_add(names.iter().map(graph_name_bytes).sum::<usize>());
        limits.check(&stats)?;

        // Honest for the SUBSET: the statement layers are claimed only when a
        // selected graph actually declares one, and the named-graph layer only when
        // something was selected. The three sidecar flags are the retained view's
        // unchanged — they describe owners this source keeps alive, and a selection
        // neither adds nor removes one.
        let capabilities = RdfStoreCapabilities {
            named_graphs: !selected.is_empty(),
            quoted_triples: quoted,
            reifiers,
            annotations,
            ..inner.capabilities()
        };

        Ok(Self {
            view,
            names: names.into(),
            graphs: selected,
            ids,
            index,
            capabilities,
        })
    }

    /// The retained view's handle one of this source's local handles names, or
    /// `None` when the handle names nothing this projection holds.
    fn inner(&self, id: LocalId) -> Option<CompositeViewId> {
        match id {
            LocalId::Base(id) => self.ids.get(id.index()).copied(),
            LocalId::Delta(_) => None,
        }
    }

    /// This source's local handle for one retained-view handle.
    fn local(&self, id: CompositeViewId) -> LocalId {
        LocalId::Base(
            self.index
                .get(&id)
                .copied()
                .expect("the projection closes over every handle a selected row reaches"),
        )
    }

    /// Resolve a term value against the retained view, admitting it only when the
    /// selection actually holds the resulting handle.
    fn lookup(&self, value: &TermValue) -> Option<LocalId> {
        self.view
            .term_id_by_value(value)
            .and_then(|id| self.index.get(&id).copied())
            .map(LocalId::Base)
    }

    /// One pattern translated into the retained view's handles, paired with the
    /// selected graph it may read: `Some(graph)` for a single graph, `None` for
    /// every selected graph.
    ///
    /// `None` for the whole result means the pattern names nothing here — a bound
    /// axis this projection does not hold, a graph outside the selection, or the
    /// default graph, which selection by graph name never admits.
    fn probe_pattern(
        &self,
        (s, p, o, g): Pattern<LocalId>,
    ) -> Option<(
        Option<CompositeViewId>,
        Option<CompositeViewId>,
        Option<CompositeViewId>,
        Option<CompositeViewId>,
    )> {
        let axis = |value: Option<LocalId>| match value {
            None => Some(None),
            Some(id) => self.inner(id).map(Some),
        };
        let graph = match g {
            GraphMatch::Any => None,
            GraphMatch::Default => return None,
            GraphMatch::Named(id) => {
                let graph = self.inner(id)?;
                if !self.graphs.contains(&graph) {
                    return None;
                }
                Some(graph)
            }
        };
        Some((axis(s)?, axis(p)?, axis(o)?, graph))
    }

    /// The selected graphs one probe visits, ascending — deterministic, and a
    /// single graph when the pattern bound one.
    fn targets(&self, only: Option<CompositeViewId>) -> impl Iterator<Item = CompositeViewId> + '_ {
        self.graphs
            .iter()
            .copied()
            .filter(move |graph| only.is_none_or(|want| want == *graph))
    }

    /// Probe ONE selected graph of the retained composite. The plan is taken by
    /// value so the returned cursor borrows the view alone. The cursor is
    /// type-erased: a selection reads THROUGH the composite's own probe, so a
    /// transparent return type here would make the composite's opaque iterator
    /// contain itself (a selection is a composite source whose rows come from a
    /// composite). One boxed hop per selected-graph probe severs that cycle and
    /// costs nothing on the native and delta arms, which never take it.
    fn probe_graph(
        &self,
        plan: QuadProbePlan,
        s: Option<CompositeViewId>,
        p: Option<CompositeViewId>,
        o: Option<CompositeViewId>,
        graph: CompositeViewId,
    ) -> Box<dyn Iterator<Item = QuadIds<CompositeViewId>> + '_> {
        Box::new(CompositeDatasetView::quads_for_pattern_with_plan(
            &self.view,
            &plan,
            s,
            p,
            o,
            GraphMatch::Named(graph),
        ))
    }
}

/// Per selected term: the dense handle table plus its reverse index, the latter
/// charged at four times its payload for hash-table slack — the same conservative
/// convention the alias maps are charged under.
fn selection_term_bytes() -> usize {
    size_of::<CompositeViewId>() + 4 * size_of::<(CompositeViewId, TermId)>()
}

/// The caller-owned bytes one selected graph name retains.
fn graph_name_bytes(name: &TermValue) -> usize {
    match name {
        TermValue::Iri(iri) => iri.len(),
        TermValue::Blank { label, .. } => label.len(),
        TermValue::Literal { .. } | TermValue::Triple { .. } => 0,
    }
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
    mappings: Arc<[Arc<AliasMap>]>,
    reverse: Arc<[Arc<ReverseMap>]>,
    scopes: Arc<[BTreeMap<BlankScope, BlankScope>]>,
    placement: Arc<[SourceGraph]>,
    prefix: PrefixState,
    user_sources: usize,
    unique_terms: usize,
    stats: ViewStats,
    work: Arc<WorkCounter>,
}

/// Everything an appended source needs of the sources already composed: which
/// blank scopes are spoken for, how far the standardizing counter has walked, and
/// the accounting owed by the user sources alone.
///
/// The derived graph dictionary is deliberately absent from [`stats`](Self::stats)
/// and [`unique_terms`](Self::unique_terms) — it is rebuilt from the full placement
/// list every time, so what it RETAINS is never charged twice. [`work`](Self::work)
/// is the opposite case and carries it: the dictionary's freeze and copied terms
/// are work that was genuinely performed, and dropping them at the prefix boundary
/// is what once made an append chain report one freeze and one dictionary's copied
/// terms however many appends it had run.
#[derive(Debug, Clone)]
struct PrefixState {
    reserved: Arc<BTreeSet<BlankScope>>,
    assigned: Arc<BTreeSet<BlankScope>>,
    next: u32,
    /// Retention owed by the user sources alone — dictionary EXCLUDED.
    stats: ViewStats,
    /// Every counter the whole composition, dictionary INCLUDED, has charged.
    work: ViewWork,
    /// Canonical term count of the user sources alone — dictionary EXCLUDED.
    unique_terms: usize,
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
        Self::from_bound_sources(rebind(sources, ScopeBinding::Independent), limits)
    }
    /// Compose source contributions in an explicitly shared blank identity space.
    /// # Errors
    /// Refuses invalid graph names or retention ceilings.
    pub fn from_shared_sources(
        sources: Vec<CompositeSource>,
        limits: ViewLimits,
    ) -> Result<Self, RdfDiagnostic> {
        Self::from_bound_sources(rebind(sources, ScopeBinding::Shared), limits)
    }
    /// Compose sources that each state their own blank scope binding.
    ///
    /// This is the general form the two modal constructors are endpoints of:
    /// [`from_sources`](Self::from_sources) declares every source
    /// [`Independent`](ScopeBinding::Independent) and
    /// [`from_shared_sources`](Self::from_shared_sources) declares every source
    /// [`Shared`](ScopeBinding::Shared), overriding whatever binding the supplied
    /// sources carried. Mixed lists are the point: a delta may co-refer with the
    /// base it branched from while a third contribution stays standardized apart.
    ///
    /// A shared source's supplied scopes are reserved before any standardization
    /// runs, so an independent source is never renamed *onto* a co-referent scope.
    ///
    /// # Errors
    /// Refuses invalid graph names, exhausted scope IDs or retention ceilings.
    pub fn from_bound_sources(
        sources: Vec<CompositeSource>,
        limits: ViewLimits,
    ) -> Result<Self, RdfDiagnostic> {
        let blanks: Vec<BTreeSet<BlankScope>> =
            sources.iter().map(CompositeSource::blank_scopes).collect();
        let mut prefix = Prefix::default();
        for (source, blank) in sources.iter().zip(&blanks) {
            prefix.reserved.extend(reservations(source, blank));
        }
        for (source, blank) in sources.into_iter().zip(blanks) {
            prefix.append(source, blank, limits)?;
        }
        prefix.assemble(limits)
    }

    /// Compose one more source onto this view, aliasing only the new contribution
    /// against the identity space the retained sources already agreed on.
    ///
    /// The result is the view [`from_bound_sources`](Self::from_bound_sources)
    /// would have built from this view's sources followed by `source`: the same
    /// rows, the same canonical identity, the same named graphs and the same
    /// retention.
    ///
    /// ## What appending actually saves
    ///
    /// Not asymptotically more than it can. Appending still aliases `source`
    /// against EVERY retained source, so one append costs
    /// O(retained sources × `source`'s terms) and a chain of N appends stays
    /// quadratic in N — the same number of source pairs a single N-source
    /// composition visits. What appending does not do is re-alias the pairs the
    /// retained sources already settled: rebuilding a k-source view from scratch
    /// costs O(k² × terms) every time, so accumulating BY REBUILD would pay that
    /// again at every step. The saving is that cubic-in-N rebuild cost, not the
    /// quadratic composition cost itself.
    ///
    /// ## What it costs on the counters
    ///
    /// [`ViewWork`] records work actually PERFORMED, so an append chain reports
    /// more of it than one from-scratch build of the same source list: each
    /// append that carries an IRI- or blank-named [`GraphPlacement`] freezes its
    /// own derived graph dictionary, and every one of those freezes is charged.
    /// The counters are monotone non-decreasing across a chain and are never an
    /// RDF identity input; [`stats`](Self::stats)'s retention figures, which are,
    /// match the from-scratch build exactly.
    ///
    /// Appending does not mutate this view; both remain usable and independent.
    ///
    /// # Errors
    /// Refuses invalid graph names, exhausted scope IDs or retention ceilings —
    /// the same refusals, with the same diagnostics, as composing from scratch.
    pub fn extend(
        &self,
        source: CompositeSource,
        limits: ViewLimits,
    ) -> Result<Self, RdfDiagnostic> {
        let blank = source.blank_scopes();
        // Reserving a scope that standardization already handed out would shift
        // every earlier renaming, so the incremental prefix no longer describes
        // the composition being asked for. Recompose instead of answering wrongly.
        if self
            .prefix
            .assigned
            .is_disjoint(&reservations(&source, &blank))
        {
            let mut prefix = self.retained_prefix();
            prefix.append(source, blank, limits)?;
            return prefix.assemble(limits);
        }
        let mut sources = self.sources().to_vec();
        sources.push(source);
        Self::from_bound_sources(sources, limits)
    }

    fn retained_prefix(&self) -> Prefix {
        Prefix {
            sources: self.sources[..self.user_sources].to_vec(),
            scopes: self.scopes[..self.user_sources].to_vec(),
            mappings: self.mappings[..self.user_sources].to_vec(),
            reverse: self.reverse[..self.user_sources].to_vec(),
            reserved: (*self.prefix.reserved).clone(),
            assigned: (*self.prefix.assigned).clone(),
            next: self.prefix.next,
            stats: self.prefix.stats,
            work: self.prefix.work,
            unique_terms: self.prefix.unique_terms,
        }
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

type AliasMap = FastMap<LocalId, CompositeViewId>;
type ReverseMap = FastMap<CompositeViewId, LocalId>;

/// Per-source retained bookkeeping: the source handle, both alias maps, the scope
/// map, the resolved placement and the derived graph handle.
fn source_bookkeeping_bytes() -> usize {
    size_of::<CompositeSource>()
        + 2 * size_of::<AliasMap>()
        + size_of::<BTreeMap<BlankScope, BlankScope>>()
        + size_of::<SourceGraph>()
        + size_of::<Option<TermId>>()
}

/// Two retained alias maps plus one temporary recursive lookup map, each charged
/// at four times its payload for hash-table slack.
fn alias_term_bytes() -> usize {
    4 * (2 * size_of::<(LocalId, CompositeViewId)>() + size_of::<(LocalId, Option<LocalId>)>())
}

fn rebind(sources: Vec<CompositeSource>, binding: ScopeBinding) -> Vec<CompositeSource> {
    sources
        .into_iter()
        .map(|source| source.with_scope_binding(binding))
        .collect()
}

/// The scopes one source forbids standardization from handing out: a caller-supplied
/// blank graph name keeps its own scope, and a co-referent source keeps all of them.
fn reservations(source: &CompositeSource, blank: &BTreeSet<BlankScope>) -> BTreeSet<BlankScope> {
    let mut reserved = BTreeSet::new();
    if let GraphPlacement::Named(TermValue::Blank { scope, .. }) = &source.placement {
        reserved.insert(*scope);
    }
    if matches!(source.binding, ScopeBinding::Shared) {
        reserved.extend(blank.iter().copied());
    }
    reserved
}

/// The user-source half of a composition, in the order the sources were supplied.
/// Composing from scratch appends every source to an empty prefix; extending an
/// existing view appends one source to the prefix that view retained.
#[derive(Debug, Default)]
struct Prefix {
    sources: Vec<CompositeSource>,
    scopes: Vec<BTreeMap<BlankScope, BlankScope>>,
    mappings: Vec<Arc<AliasMap>>,
    reverse: Vec<Arc<ReverseMap>>,
    reserved: BTreeSet<BlankScope>,
    assigned: BTreeSet<BlankScope>,
    next: u32,
    stats: ViewStats,
    work: ViewWork,
    unique_terms: usize,
}

impl Prefix {
    fn append(
        &mut self,
        mut source: CompositeSource,
        blank: BTreeSet<BlankScope>,
        limits: ViewLimits,
    ) -> Result<(), RdfDiagnostic> {
        let index = self.sources.len();
        source.retain(&mut self.stats);
        let graph_bytes = match &source.placement {
            GraphPlacement::Named(TermValue::Iri(iri)) => iri.len(),
            GraphPlacement::Named(TermValue::Blank { label, .. }) => label.len(),
            _ => 0,
        };
        self.stats.auxiliary_bytes = self
            .stats
            .auxiliary_bytes
            .saturating_add(graph_bytes)
            .saturating_add(source_bookkeeping_bytes());
        if index > 0 {
            // The first source uses identity translation and owns no alias map.
            self.stats.auxiliary_bytes = self
                .stats
                .auxiliary_bytes
                .saturating_add(source.term_count().saturating_mul(alias_term_bytes()));
        }
        limits.check(&self.stats)?;

        self.reserved.extend(reservations(&source, &blank));
        let shared = matches!(source.binding, ScopeBinding::Shared);
        let mut mapping = BTreeMap::new();
        for scope in blank {
            let mapped = if shared {
                scope
            } else {
                while self.reserved.contains(&BlankScope(self.next)) {
                    self.next = self.next.checked_add(1).ok_or_else(|| {
                        RdfDiagnostic::error(
                            "view-blank-scope",
                            "composite blank scope space exhausted",
                        )
                    })?;
                }
                let mapped = BlankScope(self.next);
                self.reserved.insert(mapped);
                self.assigned.insert(mapped);
                mapped
            };
            mapping.insert(scope, mapped);
        }
        self.stats.auxiliary_bytes = self.stats.auxiliary_bytes.saturating_add(
            mapping
                .len()
                .saturating_mul(8 * size_of::<(BlankScope, BlankScope)>()),
        );
        limits.check(&self.stats)?;
        self.work.copied_index_bytes += mapping.len() * size_of::<(BlankScope, BlankScope)>();

        let work = source.rebind_literals(&mapping);
        self.stats.auxiliary_bytes = self
            .stats
            .auxiliary_bytes
            .saturating_add(work.copied_text_bytes)
            .saturating_add(work.copied_index_bytes.saturating_mul(4));
        limits.check(&self.stats)?;
        self.work.copied_terms += work.copied_terms;
        self.work.copied_text_bytes += work.copied_text_bytes;
        self.work.copied_index_bytes += work.copied_index_bytes;

        self.sources.push(source);
        self.scopes.push(mapping);
        self.unique_terms += alias_last(
            &self.sources,
            &self.scopes,
            &mut self.mappings,
            &mut self.reverse,
            &mut self.work,
        )?;
        Ok(())
    }

    fn assemble(self, limits: ViewLimits) -> Result<CompositeDatasetView, RdfDiagnostic> {
        let Self {
            mut sources,
            mut scopes,
            mut mappings,
            mut reverse,
            reserved,
            assigned,
            next,
            stats: base_stats,
            work: base_work,
            unique_terms: base_unique_terms,
        } = self;
        let user_sources = sources.len();
        let mut graph_builder = RdfDatasetBuilder::new();
        let mut graph_ids = Vec::with_capacity(user_sources);
        let mut graph_count = 0_usize;
        for source in &sources {
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
        let mut stats = base_stats;
        let mut work = base_work;
        let mut unique_terms = base_unique_terms;
        if graph_count > 0 {
            let graphs = graph_builder.freeze()?;
            stats.retain(&graphs);
            stats.auxiliary_bytes = stats
                .auxiliary_bytes
                .saturating_add(graphs.term_count().saturating_mul(alias_term_bytes()))
                .saturating_add(size_of::<CompositeSource>());
            limits.check(&stats)?;
            // Charge the dictionary's own freeze and copied terms into the SAME
            // running total the prefix carries forward, so an append chain reports
            // the freeze each append actually paid for rather than the last one's
            // alone. Charging it here rather than onto the finished view is the
            // whole of that fix: the counters are identical for a single
            // from-scratch construction, which adds this once either way.
            work.copied_terms += graphs.term_count();
            work.copied_text_bytes += graphs.rdf_text_bytes();
            work.freezes += 1;
            // The derived dictionary owns the caller's own graph names, so its
            // scopes are the supplied ones verbatim — already reserved above.
            let dictionary = CompositeSource::new(graphs).with_scope_binding(ScopeBinding::Shared);
            let mapping: BTreeMap<_, _> = dictionary
                .blank_scopes()
                .into_iter()
                .map(|scope| (scope, scope))
                .collect();
            stats.auxiliary_bytes = stats.auxiliary_bytes.saturating_add(
                mapping
                    .len()
                    .saturating_mul(8 * size_of::<(BlankScope, BlankScope)>()),
            );
            limits.check(&stats)?;
            work.copied_index_bytes += mapping.len() * size_of::<(BlankScope, BlankScope)>();
            sources.push(dictionary);
            scopes.push(mapping);
            unique_terms += alias_last(&sources, &scopes, &mut mappings, &mut reverse, &mut work)?;
        }
        let mut view = CompositeDatasetView {
            sources: sources.into(),
            mappings: mappings.into(),
            reverse: reverse.into(),
            scopes: scopes.into(),
            placement: vec![SourceGraph::Preserve; user_sources + usize::from(graph_count > 0)]
                .into(),
            prefix: PrefixState {
                reserved: Arc::new(reserved),
                assigned: Arc::new(assigned),
                next,
                stats: base_stats,
                // The FULL accumulated work, dictionary included — see `PrefixState`.
                work,
                unique_terms: base_unique_terms,
            },
            user_sources,
            unique_terms,
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
        // One charge, and it already includes the derived dictionary's freeze.
        view.work.add(work);
        Ok(view)
    }
}

/// Alias the last pushed source against every source composed before it, and
/// report how many of its terms stay canonical in their own right.
fn alias_last(
    sources: &[CompositeSource],
    scopes: &[BTreeMap<BlankScope, BlankScope>],
    mappings: &mut Vec<Arc<AliasMap>>,
    reverse: &mut Vec<Arc<ReverseMap>>,
    work: &mut ViewWork,
) -> Result<usize, RdfDiagnostic> {
    let index = sources.len() - 1;
    let source = &sources[index];
    let source_index = u32::try_from(index)
        .map_err(|_| RdfDiagnostic::error("view-source-count", "source count exceeds u32"))?;
    let mut mapping: AliasMap = if index == 0 {
        AliasMap::default()
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
    let back: ReverseMap = mapping
        .iter()
        .map(|(&local, &canonical)| (canonical, local))
        .collect();
    work.copied_index_bytes += mapping.len() * 2 * size_of::<(LocalId, CompositeViewId)>();
    // The first source translates by identity and owns no map, so every one of its
    // terms is canonical; later sources keep the ones no earlier source answered for.
    let canonical = if index == 0 {
        source.term_ids().count()
    } else {
        mapping
            .values()
            .filter(|id| id.source == source_index)
            .count()
    };
    mappings.push(Arc::new(mapping));
    reverse.push(Arc::new(back));
    Ok(canonical)
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
                    // The SELECTED subset's own claim, not the retained view's.
                    Carrier::Selected(selection) => selection.capabilities,
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

/// A composite faults nowhere a frozen source would not: it retains its sources,
/// reads through their resident indexes and mints no request of its own, so every
/// checkpoint is [`Ready`](crate::ViewOperationStatus::Ready) — the same standing
/// [`RdfDataset`] reports, preserved through composition rather than lost at it.
impl crate::FallibleDatasetView for CompositeDatasetView {
    type Error = std::convert::Infallible;
    type Evidence = ();

    #[inline]
    fn operation_status(&self) -> crate::ViewOperationStatus<Self::Error, Self::Evidence> {
        crate::ViewOperationStatus::Ready { evidence: () }
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
