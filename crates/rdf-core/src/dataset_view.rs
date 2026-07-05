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

use crate::collections::{container_member_index, RdfListError};
use crate::collections::{RDF_ALT, RDF_BAG, RDF_FIRST, RDF_NIL, RDF_REST, RDF_SEQ, RDF_TYPE};
use crate::ir::{QuadIds, QuadRef, RdfDataset, TermId, TermRef, TermValue};
use crate::RdfStoreCapabilities;

mod sealed {
    pub trait Sealed {}

    impl Sealed for crate::ir::RdfDataset {}
    impl Sealed for crate::ir::MutableDataset {}
}

/// How a pattern query matches the graph slot of a quad.
///
/// Storage keeps `g: Option<TermId>` where `None` is the default graph, so
/// `Option<TermId>` alone cannot distinguish *any graph* from *the default graph* —
/// hence this dedicated three-way match. Deliberately exhaustive (NOT
/// `#[non_exhaustive]`): a quad's graph is either the default or exactly one named
/// graph, so the three cases are closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphMatch {
    /// Match quads in any graph (default or named).
    Any,
    /// Match only quads in the default graph (`g == None`).
    Default,
    /// Match only quads in the named graph identified by this id.
    Named(TermId),
}

impl GraphMatch {
    /// Whether a quad's stored graph slot (`None` = default graph) matches.
    #[inline]
    #[must_use]
    pub fn matches(self, graph: Option<TermId>) -> bool {
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
pub trait DatasetView: sealed::Sealed {
    /// Iterate every quad as `Copy` [`QuadIds`] (dataset-local term ids).
    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_;

    /// Iterate every quad as a borrowed, resolved [`QuadRef`] (no allocation).
    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_>> + '_;

    /// Resolve a dataset-local [`TermId`] to its borrowed [`TermRef`].
    fn resolve(&self, id: TermId) -> TermRef<'_>;

    /// Quads matching an optional `(s, p, o)` id pattern and a [`GraphMatch`].
    ///
    /// The default is an id-equality linear scan (no string resolution); backends
    /// with access-pattern indexes (P4) override this with an indexed lookup.
    /// Callers resolve term *values* to ids first (`term_id_by_value`, P4).
    fn quads_for_pattern(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.quads().filter(move |q| {
            // Closure params named `id` (not s/p/o) to avoid shadowing the outer
            // `Option<TermId>` filters with the unwrapped `TermId`.
            s.is_none_or(|id| q.s == id)
                && p.is_none_or(|id| q.p == id)
                && o.is_none_or(|id| q.o == id)
                && g.matches(q.g)
        })
    }

    /// Resolve a term **value** to its dataset-local [`TermId`], without minting.
    ///
    /// A value interned nowhere in this view yields `None` (it names no term), so a
    /// structural walk keyed on a not-present IRI simply finds nothing — absence is
    /// an empty match, never an error. Backends with a reverse value index (P4) use
    /// it; others may scan.
    fn term_id_by_value(&self, value: &TermValue) -> Option<TermId>;

    /// The capabilities this view's backing data exposes (C7).
    fn capabilities(&self) -> RdfStoreCapabilities;

    /// A size hint for the number of quads, if known.
    fn len_hint(&self) -> Option<usize> {
        None
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
    fn rdf_list(&self, head: TermId, graph: GraphMatch) -> Result<Vec<TermId>, RdfListError> {
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
        let mut seen: BTreeSet<TermId> = BTreeSet::new();
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
    fn rdf_container_members(&self, head: TermId, graph: GraphMatch) -> Vec<TermId> {
        let mut indexed: Vec<(u64, TermId)> = Vec::new();
        for q in self.quads_for_pattern(Some(head), None, None, graph) {
            if let TermRef::Iri(iri) = self.resolve(q.p) {
                if let Some(n) = container_member_index(iri) {
                    indexed.push((n, q.o));
                }
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
    fn members(&self, head: TermId, graph: GraphMatch) -> Result<Vec<TermId>, RdfListError> {
        // Collection shape wins: an `rdf:first` edge marks a cons cell.
        if let Some(first_p) = self.term_id_by_value(&TermValue::iri(RDF_FIRST)) {
            if self
                .quads_for_pattern(Some(head), Some(first_p), None, graph)
                .next()
                .is_some()
            {
                return self.rdf_list(head, graph);
            }
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
        id: TermId,
        first_p: TermId,
        rest_p: Option<TermId>,
        graph: GraphMatch,
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
    fn is_typed_container(&self, head: TermId, graph: GraphMatch) -> bool {
        let Some(type_p) = self.term_id_by_value(&TermValue::iri(RDF_TYPE)) else {
            return false;
        };
        let seq = self.term_id_by_value(&TermValue::iri(RDF_SEQ));
        let bag = self.term_id_by_value(&TermValue::iri(RDF_BAG));
        let alt = self.term_id_by_value(&TermValue::iri(RDF_ALT));
        // No container class interned ⇒ nothing can be typed as one.
        if seq.is_none() && bag.is_none() && alt.is_none() {
            return false;
        }
        self.quads_for_pattern(Some(head), Some(type_p), None, graph)
            .any(|q| Some(q.o) == seq || Some(q.o) == bag || Some(q.o) == alt)
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
        // Delegate to the inherent lazy reverse value index (no minting).
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
        assert!(GraphMatch::Any.matches(None) && GraphMatch::Any.matches(Some(g)));
        assert!(GraphMatch::Default.matches(None) && !GraphMatch::Default.matches(Some(g)));
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
