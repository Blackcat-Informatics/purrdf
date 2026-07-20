// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The static, allocation-free **read view** over an RDF dataset (purrdf P2,
//! ). See [`docs/design/purrdf-backend-contract.md`](../../../docs/design/purrdf-backend-contract.md).
//!
//! [`DatasetView`] is the id-based, borrowed read interface: it yields `Copy`
//! [`QuadIds`] and borrowed [`QuadRef`]s (no per-quad allocation, no term-string clones), and offers
//! [`DatasetView::quads_for_pattern`] keyed on dataset-local [`TermId`]s plus a
//! [`GraphMatch`]. The default `quads_for_pattern` is a linear scan; backends with
//! access-pattern indexes (P4) override it.
//!
//! This is the **static** trait layer (generic `impl DatasetView`, RPITIT — not
//! object-safe). Per the backend contract (C1), backend selection is compile-time
//! and single, so the erased `&mut dyn` layer is deferred; this trait carries no
//! object-safety obligation.

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::RdfStoreCapabilities;
use crate::collections::{RDF_ALT, RDF_BAG, RDF_FIRST, RDF_NIL, RDF_REST, RDF_SEQ, RDF_TYPE};
use crate::collections::{RdfListError, container_member_index};
use crate::ir::{QuadIds, QuadProbePlan, QuadRef, RdfDataset, TermId, TermRef, TermValue};

mod sealed {
    pub trait Sealed {}

    impl Sealed for crate::ir::MutableDataset {}
}

/// The associated id type of a [`DatasetView`]. An id is meaningful only within the
/// view that minted it (C0.8); these bounds are exactly what the evaluator's
/// join/index machinery needs of an id (`Copy` to pass by value, `Eq`/`Ord`/`Hash`
/// to key joins and index probes, `Send`/`Sync`/`'static` to cross the query
/// boundary). The production id is [`TermId`]; a paged/global backend mints its own.
///
/// # Join-key encoding
///
/// The evaluator hash-joins on a single [`JoinKeyAtom`](Self::JoinKeyAtom): a `Copy`,
/// totally-ordered atom that packs *either* a dataset id (via [`encode`](Self::encode))
/// *or* an evaluator-minted computed-term id (via
/// [`encode_computed`](Self::encode_computed)) into ONE key space, with the two
/// disjoint by construction so an id never collides with a computed key (which would
/// be a wrong join result, not a slowdown). For [`TermId`] the atom is the historical
/// packed `u64` — dataset ids in `[0, 2^32)`, computed ids in `[2^32, 2^33)` — so the
/// production hash-join is bit-identical to before the id-generic seam. A wider id
/// (e.g. `GlobalTermId`) uses a wider atom (`u128`) so the same disjointness holds at
/// its width.
pub trait ViewTermId:
    Copy + Eq + Ord + core::hash::Hash + core::fmt::Debug + Send + Sync + 'static
{
    /// A `Copy`, totally-ordered atom that encodes a dataset id OR a computed-term
    /// id into one hash-join key space (see the trait docs). For [`TermId`] this is
    /// `u64`; for a wider id it is `u128`.
    type JoinKeyAtom: Copy + Eq + Ord + core::hash::Hash + Send + Sync + 'static;

    /// Encode this dataset id into the join-key space. The image of `encode` over all
    /// ids MUST be disjoint from the image of [`encode_computed`](Self::encode_computed)
    /// over all scratch indices, so an `Existing` binding never shares a key with a
    /// `Computed` one.
    fn encode(self) -> Self::JoinKeyAtom;

    /// Encode an evaluator-minted computed-term scratch index (a `u32`) into the join-key
    /// space, disjoint from every [`encode`](Self::encode) image (see the trait docs).
    fn encode_computed(scratch_index: u32) -> Self::JoinKeyAtom;
}

impl ViewTermId for TermId {
    type JoinKeyAtom = u64;

    #[inline]
    fn encode(self) -> u64 {
        let ix = self.index() as u64;
        debug_assert!(ix < (1 << 32), "TermId index must fit u32");
        ix
    }

    #[inline]
    fn encode_computed(scratch_index: u32) -> u64 {
        // Dataset ids occupy `[0, 2^32)`; computed ids occupy `[2^32, 2^33)`. Bit 32
        // is the disjointness tag — byte-identical to the historical `join_key_u64`.
        (1 << 32) | u64::from(scratch_index)
    }
}

/// How a pattern query matches the graph slot of a quad.
///
/// Storage keeps `g: Option<TermId>` where `None` is the default graph, so
/// `Option<TermId>` alone cannot distinguish *any graph* from *the default graph* —
/// hence this dedicated three-way match. Deliberately exhaustive (NOT
/// `#[non_exhaustive]`): a quad's graph is either the default or exactly one named
/// graph, so the three cases are closed.
///
/// Generic over the id type `Id` (defaulting to [`TermId`]) so it names a graph in
/// any [`DatasetView`]'s id space; the bare spelling `GraphMatch` continues to name
/// the `RdfDataset` (`TermId`) instantiation everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphMatch<Id = TermId> {
    /// Match quads in any graph (default or named).
    Any,
    /// Match only quads in the default graph (`g == None`).
    Default,
    /// Match only quads in the named graph identified by this id.
    Named(Id),
}

impl<Id: Copy + PartialEq> GraphMatch<Id> {
    /// Whether a quad's stored graph slot (`None` = default graph) matches.
    #[inline]
    #[must_use]
    pub fn matches(self, graph: Option<Id>) -> bool {
        match self {
            Self::Any => true,
            Self::Default => graph.is_none(),
            Self::Named(id) => graph == Some(id),
        }
    }
}

/// How a **write-side** pattern query matches the graph slot of a quad — the
/// value-based twin of [`GraphMatch`].
///
/// The read view ([`DatasetView`]) names a graph by its dataset-local [`TermId`],
/// which every graph in a frozen dataset has. The mutable write view
/// ([`DatasetMut`]), however, straddles a frozen base and an in-memory delta: a
/// *delta-only* named graph (one introduced after branching) has NO base `TermId`,
/// so a `TermId`-keyed graph filter cannot express it. Worse, it would be
/// inconsistent with the `s`/`p`/`o` slots, which `DatasetMut` already matches by
/// *value*. So the write side names a graph by [`TermValue`] too: the implementer
/// resolves the value to its internal handle WITHOUT minting (a value interned
/// nowhere matches nothing — an empty filter, exactly like a bound `s`/`p`/`o`
/// value that misses). This makes both base-named AND delta-only-named graphs
/// expressible. Deliberately exhaustive (NOT `#[non_exhaustive]`), like
/// [`GraphMatch`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphMatchValue<'a> {
    /// Match quads in any graph (default or named).
    Any,
    /// Match only quads in the default graph.
    Default,
    /// Match only quads in the named graph identified by this term value.
    Named(&'a TermValue),
}

/// A static, allocation-free read view over an RDF dataset (purrdf backend
/// contract, C2/C3/C6). All methods are infallible for a frozen, validated dataset.
pub trait DatasetView {
    /// The dataset-local id type this view mints and reads in (C0.8). For the
    /// production [`RdfDataset`] this is [`TermId`]; a paged/global backend supplies
    /// its own id, which is meaningful only within this view.
    type Id: ViewTermId;

    /// The opaque, loop-invariant probe plan a pattern query precomputes once (from
    /// which axes a pattern binds) and reuses across the probe rows of one
    /// index-nested-loop join slot (see [`Self::probe_plan`] /
    /// [`Self::quads_for_pattern_with_plan`]). A backend with no access-pattern index
    /// uses the unit plan `()`.
    ///
    /// `Send + Sync` because the plan is computed once per join slot and then
    /// captured by value into every parallel probe worker (see the BGP join loop):
    /// the loop-invariant plan crosses the fork boundary, so it must be shareable.
    type ProbePlan: Copy + Send + Sync;

    /// Iterate every quad as `Copy` [`QuadIds`] (dataset-local term ids).
    fn quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_;

    /// Iterate every quad as a borrowed, resolved [`QuadRef`] (no allocation).
    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_, Self::Id>> + '_;

    /// Resolve a dataset-local id to its borrowed [`TermRef`].
    fn resolve(&self, id: Self::Id) -> TermRef<'_, Self::Id>;

    /// Quads matching an optional `(s, p, o)` id pattern and a [`GraphMatch`].
    ///
    /// The default is an id-equality linear scan (no string resolution); backends
    /// with access-pattern indexes (P4) override this with an indexed lookup.
    /// Callers resolve term *values* to ids first (`term_id_by_value`, P4).
    fn quads_for_pattern(
        &self,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.quads().filter(move |q| {
            // Closure params named `id` (not s/p/o) to avoid shadowing the outer
            // `Option<Self::Id>` filters with the unwrapped id.
            s.is_none_or(|id| q.s == id)
                && p.is_none_or(|id| q.p == id)
                && o.is_none_or(|id| q.o == id)
                && g.matches(q.g)
        })
    }

    /// Resolve a term **value** to its dataset-local id, without minting.
    ///
    /// A value interned nowhere in this view yields `None` (it names no term), so a
    /// structural walk keyed on a not-present IRI simply finds nothing — absence is
    /// an empty match, never an error. Backends with a reverse value index (P4) use
    /// it; others may scan.
    fn term_id_by_value(&self, value: &TermValue) -> Option<Self::Id>;

    /// The capabilities this view's backing data exposes (C7).
    fn capabilities(&self) -> RdfStoreCapabilities;

    /// A size hint for the number of quads, if known.
    fn len_hint(&self) -> Option<usize> {
        None
    }

    /// Precompute the loop-invariant [`ProbePlan`](Self::ProbePlan) for a pattern of
    /// the given bound-axis shape and graph constraint, to be reused across the probe
    /// rows of an index-nested-loop join slot via
    /// [`Self::quads_for_pattern_with_plan`]. An index-free backend returns the unit
    /// plan; [`RdfDataset`] returns its permutation-and-prefix plan.
    fn probe_plan(
        &self,
        s_bound: bool,
        p_bound: bool,
        o_bound: bool,
        g: GraphMatch<Self::Id>,
    ) -> Self::ProbePlan;

    /// Quads matching `(s, p, o, g)` under a caller-precomputed
    /// [`ProbePlan`](Self::ProbePlan). Behaviourally identical to
    /// [`Self::quads_for_pattern`] — the plan only skips the per-row permutation
    /// selection; the yielded quads and their order are unchanged. An index-free
    /// backend ignores the (unit) plan and forwards to `quads_for_pattern`.
    fn quads_for_pattern_with_plan(
        &self,
        plan: &Self::ProbePlan,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_;

    /// An upper-bound cardinality estimate for `(s, p, o, g)`, FOR COST RANKING ONLY
    /// (never an exact `COUNT`). The default materializes the pattern and counts it;
    /// a backend with index bounds (P4) overrides this with an `O(log n)` estimate.
    fn cardinality_estimate(
        &self,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> usize {
        self.quads_for_pattern(s, p, o, g).count()
    }

    /// The number of distinct interned terms this view addresses.
    fn term_count(&self) -> usize;

    /// A cheap, deterministic size fingerprint for a dataset-aware cache key (e.g. a
    /// join-order cache). A *cache discriminator*, not a content digest. The default
    /// is `0` (no discrimination); [`RdfDataset`] hashes its quad and term counts.
    fn stats_fingerprint(&self) -> u64 {
        0
    }

    /// The RDF 1.2 reifier side-table AS resolved virtual triples: each
    /// `(reifier, triple-term)` binding becomes a `(reifier, rdf:reifies, triple-term)`
    /// quad carrying the declaration's own graph slot. Capability-gated: a backend
    /// with no reifier layer yields nothing (the default).
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        std::iter::empty()
    }

    /// The RDF 1.2 annotation side-table AS resolved virtual triples: each
    /// `(reifier, predicate, object)` annotation becomes a quad carrying its own graph
    /// slot. Capability-gated: a backend with no annotation layer yields nothing.
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        std::iter::empty()
    }

    /// The `(predicate, object, graph)` annotations declared for `reifier` (`graph`
    /// `None` ⇒ default graph). Capability-gated: a backend with no annotation layer
    /// yields nothing (the default ignores `reifier`).
    fn annotations_of_with_graph(
        &self,
        reifier: Self::Id,
    ) -> impl Iterator<Item = (Self::Id, Self::Id, Option<Self::Id>)> + '_ {
        let _ = reifier;
        std::iter::empty()
    }

    /// Every named graph this view addresses, in ascending id order (sorted,
    /// deduplicated). Drives `GRAPH ?g` enumeration, so the order is
    /// result-observable and must be deterministic. The default derives the set
    /// from the quads the view can see; a backend that also tracks explicitly
    /// declared *empty* named graphs (e.g. [`RdfDataset`]) overrides this to
    /// include them.
    fn named_graphs(&self) -> impl Iterator<Item = Self::Id> + '_ {
        let set: BTreeSet<Self::Id> = self.quads().filter_map(|q| q.g).collect();
        set.into_iter()
    }

    /// Materialize an `rdf:first`/`rdf:rest`/`rdf:nil` Collection whose head is
    /// `head`, scoped to `graph`. Returns members in list order.
    ///
    /// Cycle-guarded: a revisited cell terminates the walk gracefully (`Ok`,
    /// truncated at the cycle), matching the reference GTS walker. A MALFORMED cell
    /// — a cons cell with no `rdf:first`, more than one `rdf:first`, or an
    /// `rdf:rest` to a term that is neither `rdf:nil` nor a cons cell — is a hard
    /// error ([`RdfListError`]): this walker is also a validator. A head that is
    /// `rdf:nil` or is not a list (carries neither `rdf:first` nor `rdf:rest`)
    /// yields an empty `Vec`.
    ///
    /// If the `rdf:first` IRI is not interned in this view at all, no Collection can
    /// exist, so the result is an empty `Vec` (`Ok`), not an error.
    fn rdf_list(
        &self,
        head: Self::Id,
        graph: GraphMatch<Self::Id>,
    ) -> Result<Vec<Self::Id>, RdfListError> {
        // No `rdf:first` in the term table ⇒ no cons cell can exist here.
        let Some(first_p) = self.term_id_by_value(&TermValue::iri(RDF_FIRST)) else {
            return Ok(Vec::new());
        };
        // `rdf:rest`/`rdf:nil` may be absent; the walk handles that inline (a
        // missing `rdf:rest` edge simply ends the list, absent `rdf:nil` matches no
        // terminator).
        let rest_p = self.term_id_by_value(&TermValue::iri(RDF_REST));
        let nil = self.term_id_by_value(&TermValue::iri(RDF_NIL));

        let mut out = Vec::new();
        // Cycle guard: revisited cell terminates the walk (mirrors the reference
        // GTS `rdf_list` seen-set).
        let mut seen: BTreeSet<Self::Id> = BTreeSet::new();
        let mut current = head;
        loop {
            if Some(current) == nil {
                break;
            }
            if !seen.insert(current) {
                break;
            }

            // Gather this cell's `rdf:first` objects: zero or many is malformed
            // (the defensive multi-edge detection of the reference list walker).
            let mut first_obj = None;
            let mut first_count = 0usize;
            for q in self.quads_for_pattern(Some(current), Some(first_p), None, graph) {
                first_obj = Some(q.o);
                first_count += 1;
            }
            // The single `rdf:rest` object, if any.
            let mut rest_obj = None;
            if let Some(rest_p) = rest_p {
                for q in self.quads_for_pattern(Some(current), Some(rest_p), None, graph) {
                    rest_obj = Some(q.o);
                }
            }

            // Neither edge ⇒ not a cons cell. Only `head` can reach this branch
            // (interior cells are only entered through a rest edge validated to
            // point at `rdf:nil` or a cons cell); it means `head` is simply not a
            // list ⇒ an empty Vec.
            if first_count == 0 && rest_obj.is_none() {
                break;
            }
            if first_count == 0 {
                return Err(RdfListError::MissingFirst);
            }
            if first_count > 1 {
                return Err(RdfListError::MultipleFirst);
            }
            out.push(first_obj.expect("first_count == 1 implies a first object"));

            // A cons cell with no `rdf:rest` edge ends the list (a truncated, but
            // not malformed, tail). Otherwise validate the rest target before
            // following it.
            let Some(next) = rest_obj else {
                break;
            };
            if Some(next) != nil && !self.is_cons_cell(next, first_p, rest_p, graph) {
                return Err(RdfListError::DanglingRest);
            }
            current = next;
        }
        Ok(out)
    }

    /// RDF Container members `rdf:_1`..`rdf:_n` of `head` in numeric order, scoped
    /// to `graph`. Gaps are skipped; ordering is by the numeric suffix, NOT dataset
    /// order.
    fn rdf_container_members(&self, head: Self::Id, graph: GraphMatch<Self::Id>) -> Vec<Self::Id> {
        let mut indexed: Vec<(u64, Self::Id)> = Vec::new();
        for q in self.quads_for_pattern(Some(head), None, None, graph) {
            if let TermRef::Iri(iri) = self.resolve(q.p)
                && let Some(n) = container_member_index(iri)
            {
                indexed.push((n, q.o));
            }
        }
        // Order by the numeric suffix (`_2` before `_10`), tolerating gaps.
        indexed.sort_by_key(|&(n, _)| n);
        indexed.into_iter().map(|(_, o)| o).collect()
    }

    /// Dispatch by shape: a `head` carrying an `rdf:first` is walked as a Collection
    /// ([`rdf_list`](Self::rdf_list)); a `head` typed `rdf:Seq`/`rdf:Bag`/`rdf:Alt`
    /// or carrying any `rdf:_n` property is walked as a Container
    /// ([`rdf_container_members`](Self::rdf_container_members)). A head matching
    /// neither yields an empty `Vec`.
    fn members(
        &self,
        head: Self::Id,
        graph: GraphMatch<Self::Id>,
    ) -> Result<Vec<Self::Id>, RdfListError> {
        // Collection shape wins: an `rdf:first` edge marks a cons cell.
        if let Some(first_p) = self.term_id_by_value(&TermValue::iri(RDF_FIRST))
            && self
                .quads_for_pattern(Some(head), Some(first_p), None, graph)
                .next()
                .is_some()
        {
            return self.rdf_list(head, graph);
        }
        // Container shape: any `rdf:_n` membership property present.
        let members = self.rdf_container_members(head, graph);
        if !members.is_empty() {
            return Ok(members);
        }
        // A head explicitly typed as a container is a container even with no member
        // properties yet — walking it yields the (empty) member set.
        if self.is_typed_container(head, graph) {
            return Ok(members);
        }
        Ok(Vec::new())
    }

    /// Whether `id` is an RDF Collection cons cell in `graph`: it carries an
    /// `rdf:first` or an `rdf:rest` edge. Internal helper for the list walker's
    /// dangling-rest validation.
    #[doc(hidden)]
    fn is_cons_cell(
        &self,
        id: Self::Id,
        first_p: Self::Id,
        rest_p: Option<Self::Id>,
        graph: GraphMatch<Self::Id>,
    ) -> bool {
        if self
            .quads_for_pattern(Some(id), Some(first_p), None, graph)
            .next()
            .is_some()
        {
            return true;
        }
        rest_p.is_some_and(|rest_p| {
            self.quads_for_pattern(Some(id), Some(rest_p), None, graph)
                .next()
                .is_some()
        })
    }

    /// Whether `head` is typed `rdf:Seq`/`rdf:Bag`/`rdf:Alt` in `graph`. Internal
    /// helper for [`members`](Self::members) container dispatch.
    #[doc(hidden)]
    fn is_typed_container(&self, head: Self::Id, graph: GraphMatch<Self::Id>) -> bool {
        let Some(type_p) = self.term_id_by_value(&TermValue::iri(RDF_TYPE)) else {
            return false;
        };
        // One reverse lookup (`rdf:type`); the container classes are matched by
        // resolving each type object's IRI, not by three extra id probes.
        self.quads_for_pattern(Some(head), Some(type_p), None, graph)
            .any(|q| {
                matches!(self.resolve(q.o), TermRef::Iri(iri) if iri == RDF_SEQ || iri == RDF_BAG || iri == RDF_ALT)
            })
    }
}

/// An atomic checkpoint of an operationally fallible dataset view.
///
/// [`Ready`](Self::Ready) means no operational failure has been observed *at this
/// checkpoint*. It becomes a completeness certificate only when an execution engine
/// samples it after evaluation has stopped. [`Failed`](Self::Failed) carries both the
/// sticky root cause and the deterministic evidence accumulated before that failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewOperationStatus<Error, Evidence> {
    /// The view has not observed an operational failure.
    Ready {
        /// Deterministic resource/request evidence accumulated so far.
        evidence: Evidence,
    },
    /// The operation has irreversibly failed; further reads yield no data.
    Failed {
        /// The first operational root cause observed by the view.
        error: Error,
        /// Deterministic resource/request evidence at the failure boundary.
        evidence: Evidence,
    },
}

impl<Error, Evidence> ViewOperationStatus<Error, Evidence> {
    /// Borrow the evidence carried by either status variant.
    #[must_use]
    pub const fn evidence(&self) -> &Evidence {
        match self {
            Self::Ready { evidence } | Self::Failed { evidence, .. } => evidence,
        }
    }

    /// Borrow the sticky operational error, if one has occurred.
    #[must_use]
    pub const fn error(&self) -> Option<&Error> {
        match self {
            Self::Ready { .. } => None,
            Self::Failed { error, .. } => Some(error),
        }
    }
}

/// A [`DatasetView`] whose backing data can fail during lazy reads.
///
/// Implementations preserve the infallible iterator shape required by the evaluator:
/// the first operational failure becomes sticky, every iterator stops yielding, and
/// [`operation_status`](Self::operation_status) exposes that root cause. An execution
/// boundary must sample the status before evaluation and again after all evaluation
/// and result materialization; it may publish a result as complete only when the final
/// checkpoint is [`ViewOperationStatus::Ready`]. Internal partial rows are never a
/// completeness signal.
pub trait FallibleDatasetView: DatasetView {
    /// The typed operational root cause.
    type Error: std::error::Error + Clone + Send + Sync + 'static;
    /// Deterministic request and resource evidence.
    type Evidence: Clone + std::fmt::Debug + PartialEq + Eq + Send + Sync + 'static;

    /// Take an atomic checkpoint of current operational status and evidence.
    fn operation_status(&self) -> ViewOperationStatus<Self::Error, Self::Evidence>;
}

impl<T: FallibleDatasetView> FallibleDatasetView for Arc<T> {
    type Error = T::Error;
    type Evidence = T::Evidence;

    #[inline]
    fn operation_status(&self) -> ViewOperationStatus<Self::Error, Self::Evidence> {
        (**self).operation_status()
    }
}

/// The **write companion** to [`DatasetView`] — the mutation surface a copy-on-write
/// or backed-by-store dataset exposes (purrdf P5; backend contract C4).
///
/// Where [`DatasetView`] reads in dataset-local [`TermId`]s, `DatasetMut` mutates by
/// **value**: its [`Quad`](DatasetMut::Quad) associated type is an owned, dataset-
/// independent quad (each component a [`TermValue`]). A mutable dataset that straddles
/// a frozen base and an in-memory delta has no single id space its caller could name a
/// brand-new term in (C0.8), so a value is the only well-defined mutation identity. The
/// implementer resolves each value to its internal handle (a base hit, or a freshly
/// minted delta id) — see [`MutableDataset`](crate::ir::mutable::MutableDataset).
///
/// All four methods operate on the **effective** set. `insert`/`remove` return whether
/// the effective set actually changed (so callers can detect no-ops); `contains` and
/// `quads_for_pattern` reflect the effective set after any sequence of mutations.
pub trait DatasetMut: sealed::Sealed {
    /// The owned, dataset-independent quad value this dataset is mutated with.
    type Quad;

    /// Insert a quad into the effective set. Returns `true` iff the effective set
    /// changed (a quad already present is a no-op returning `false`).
    fn insert(&mut self, quad: Self::Quad) -> bool;

    /// Remove a quad from the effective set. Returns `true` iff the effective set
    /// changed (removing an absent quad is a no-op returning `false`).
    fn remove(&mut self, quad: &Self::Quad) -> bool;

    /// Whether the quad is in the effective set.
    fn contains(&self, quad: &Self::Quad) -> bool;

    /// The effective quads matching an optional `(s, p, o)` value pattern and a
    /// [`GraphMatchValue`]. Returns owned value-quads (the mutable view has no stable
    /// id space to borrow into across the base/delta boundary). A bound value — in
    /// any of `s`/`p`/`o` OR the graph slot — interned in neither the base nor the
    /// delta matches nothing.
    ///
    /// The graph filter is value-based (`GraphMatchValue`, NOT the read side's
    /// `TermId`-based `GraphMatch`) so a delta-only named graph, which has no base
    /// `TermId`, is still expressible — consistent with the value-based `s`/`p`/`o`.
    fn quads_for_pattern(
        &self,
        s: Option<&TermValue>,
        p: Option<&TermValue>,
        o: Option<&TermValue>,
        g: GraphMatchValue<'_>,
    ) -> Vec<Self::Quad>;
}

/// The production read view: the immutable value-interned [`RdfDataset`] (C1).
impl DatasetView for RdfDataset {
    type Id = TermId;
    type ProbePlan = QuadProbePlan;

    #[inline]
    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        // Inherent methods take method-resolution priority over trait methods, so
        // these delegate to `RdfDataset`'s own impls (no recursion).
        Self::quads(self)
    }

    #[inline]
    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_>> + '_ {
        Self::quad_refs(self)
    }

    #[inline]
    fn resolve(&self, id: TermId) -> TermRef<'_> {
        Self::resolve(self, id)
    }

    #[inline]
    fn quads_for_pattern(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        // Indexed override (P4b): lazy permutation indexes + a bound-set ->
        // permutation -> partition_point dispatch, byte-identical to the trait's
        // default linear scan (differential proptest in `ir/dataset.rs`).
        Self::quads_for_pattern_indexed(self, s, p, o, g)
    }

    #[inline]
    fn term_id_by_value(&self, value: &TermValue) -> Option<TermId> {
        // Delegate to the retained store-once term index (no minting).
        Self::term_id_by_value(self, value)
    }

    #[inline]
    fn capabilities(&self) -> RdfStoreCapabilities {
        Self::capabilities(self)
    }

    #[inline]
    fn len_hint(&self) -> Option<usize> {
        Some(Self::quad_count(self))
    }

    #[inline]
    fn probe_plan(
        &self,
        s_bound: bool,
        p_bound: bool,
        o_bound: bool,
        g: GraphMatch,
    ) -> QuadProbePlan {
        // Inherent `probe_plan` is a value-independent associated fn (no `&self`).
        Self::probe_plan(s_bound, p_bound, o_bound, g)
    }

    #[inline]
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

    #[inline]
    fn cardinality_estimate(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> usize {
        Self::cardinality_estimate(self, s, p, o, g)
    }

    #[inline]
    fn term_count(&self) -> usize {
        Self::term_count(self)
    }

    #[inline]
    fn stats_fingerprint(&self) -> u64 {
        Self::stats_fingerprint(self)
    }

    #[inline]
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        Self::reifier_quads(self)
    }

    #[inline]
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        Self::annotation_quads(self)
    }

    #[inline]
    fn annotations_of_with_graph(
        &self,
        reifier: TermId,
    ) -> impl Iterator<Item = (TermId, TermId, Option<TermId>)> + '_ {
        Self::annotations_of_with_graph(self, reifier)
    }

    #[inline]
    fn named_graphs(&self) -> impl Iterator<Item = TermId> + '_ {
        // The inherent set includes explicitly-declared empty named graphs, which
        // the quads-derived default would miss, so delegate to it verbatim.
        Self::named_graphs(self)
    }
}

/// A shared [`Arc`]-wrapped read view is itself a read view: every method delegates
/// to the inner `T`, so `Arc<RdfDataset>` (the engine's `Dataset` handle) plugs into
/// the generic evaluator directly, byte-for-byte identical to the bare `RdfDataset`.
/// Every method — including the ones `RdfDataset` overrides for its indexed read path
/// and RDF 1.2 side-tables — is forwarded, so no default (e.g. an empty reifier layer)
/// can silently diverge from the inner view.
impl<T: DatasetView> DatasetView for Arc<T> {
    type Id = T::Id;
    type ProbePlan = T::ProbePlan;

    #[inline]
    fn quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        (**self).quads()
    }

    #[inline]
    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_, Self::Id>> + '_ {
        (**self).quad_refs()
    }

    #[inline]
    fn resolve(&self, id: Self::Id) -> TermRef<'_, Self::Id> {
        (**self).resolve(id)
    }

    #[inline]
    fn quads_for_pattern(
        &self,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        (**self).quads_for_pattern(s, p, o, g)
    }

    #[inline]
    fn term_id_by_value(&self, value: &TermValue) -> Option<Self::Id> {
        (**self).term_id_by_value(value)
    }

    #[inline]
    fn capabilities(&self) -> RdfStoreCapabilities {
        (**self).capabilities()
    }

    #[inline]
    fn len_hint(&self) -> Option<usize> {
        (**self).len_hint()
    }

    #[inline]
    fn probe_plan(
        &self,
        s_bound: bool,
        p_bound: bool,
        o_bound: bool,
        g: GraphMatch<Self::Id>,
    ) -> Self::ProbePlan {
        (**self).probe_plan(s_bound, p_bound, o_bound, g)
    }

    #[inline]
    fn quads_for_pattern_with_plan(
        &self,
        plan: &Self::ProbePlan,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        (**self).quads_for_pattern_with_plan(plan, s, p, o, g)
    }

    #[inline]
    fn cardinality_estimate(
        &self,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> usize {
        (**self).cardinality_estimate(s, p, o, g)
    }

    #[inline]
    fn term_count(&self) -> usize {
        (**self).term_count()
    }

    #[inline]
    fn stats_fingerprint(&self) -> u64 {
        (**self).stats_fingerprint()
    }

    #[inline]
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        (**self).reifier_quads()
    }

    #[inline]
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        (**self).annotation_quads()
    }

    #[inline]
    fn annotations_of_with_graph(
        &self,
        reifier: Self::Id,
    ) -> impl Iterator<Item = (Self::Id, Self::Id, Option<Self::Id>)> + '_ {
        (**self).annotations_of_with_graph(reifier)
    }

    #[inline]
    fn named_graphs(&self) -> impl Iterator<Item = Self::Id> + '_ {
        (**self).named_graphs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::RdfDatasetBuilder;

    fn iri(b: &mut RdfDatasetBuilder, n: &str) -> TermId {
        b.intern_iri(&format!("http://example.org/{n}"))
    }

    #[test]
    fn graph_match_three_way() {
        let mut b = RdfDatasetBuilder::new();
        let g = iri(&mut b, "g");
        assert!(GraphMatch::<TermId>::Any.matches(None) && GraphMatch::Any.matches(Some(g)));
        assert!(
            GraphMatch::<TermId>::Default.matches(None) && !GraphMatch::Default.matches(Some(g))
        );
        assert!(GraphMatch::Named(g).matches(Some(g)) && !GraphMatch::Named(g).matches(None));
    }

    #[test]
    fn quads_for_pattern_filters_by_id_and_graph() {
        let mut b = RdfDatasetBuilder::new();
        let s = iri(&mut b, "s");
        let p = iri(&mut b, "p");
        let o1 = iri(&mut b, "o1");
        let o2 = iri(&mut b, "o2");
        let g = iri(&mut b, "g");
        b.push_quad(s, p, o1, None); // default graph
        b.push_quad(s, p, o2, Some(g)); // named graph g
        let ds = b.freeze().expect("freeze");

        // Whole-dataset (Any matches everything).
        assert_eq!(
            ds.quads_for_pattern(None, None, None, GraphMatch::Any)
                .count(),
            2
        );
        assert_eq!(ds.len_hint(), Some(2));
        // Object filter.
        assert_eq!(
            ds.quads_for_pattern(None, None, Some(o1), GraphMatch::Any)
                .count(),
            1
        );
        // Default graph only.
        assert_eq!(
            ds.quads_for_pattern(None, None, None, GraphMatch::Default)
                .count(),
            1
        );
        // Named graph only.
        assert_eq!(
            ds.quads_for_pattern(None, None, None, GraphMatch::Named(g))
                .count(),
            1
        );
        // s+p match both quads.
        assert_eq!(
            ds.quads_for_pattern(Some(s), Some(p), None, GraphMatch::Any)
                .count(),
            2
        );
        // A non-matching subject yields nothing.
        assert_eq!(
            ds.quads_for_pattern(Some(o1), None, None, GraphMatch::Any)
                .count(),
            0
        );
        // The trait read view agrees with the inherent iterators.
        assert_eq!(DatasetView::quads(&*ds).count(), 2);
        assert_eq!(DatasetView::quad_refs(&*ds).count(), 2);
    }
}
