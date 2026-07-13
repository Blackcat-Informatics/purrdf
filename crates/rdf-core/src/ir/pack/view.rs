// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The [`DatasetView`] seam over a [`super::container::PackView`]:
//! [`PackId`] wraps the dictionary's unified, 1-based
//! [`super::dict::PackTermId`] as a [`ViewTermId`], and `impl DatasetView for
//! PackView<'_>` answers every read-side query directly from the pack's decoded
//! dictionary + borrowed bitmap-triples + borrowed side-tables, with NO
//! materialization step (no intermediate `RdfDataset`) — a pack file opened with
//! [`super::container::PackView::from_bytes`] is queryable by the SPARQL evaluator
//! exactly like any other backend.
//!
//! # `PackId`
//!
//! Mirrors [`crate::ir::global::GlobalTermId`] one level down: the pack dictionary
//! already mints ids as `NonZeroU64` in effect (unified ids are `1..=n_terms`, id `0`
//! is never assigned — see `PackDict::entry`'s panic message), so
//! `PackId` wraps that guarantee in the type rather than re-deriving it, giving
//! `Option<PackId>` the same 8-byte niche as `Option<GlobalTermId>`.
//!
//! # Mapping `TermRef`/`GraphMatch`/quad rows
//!
//! Every query surface `PackDict`/`TriplesRef`/`SideTablesRef` expose is keyed on
//! the bare `u64` [`super::dict::PackTermId`] (an internal, dictionary-local
//! representation with no niche — a decode step, not a caller-facing identity). This
//! module is the ONE place that crosses from that internal `u64` space into the
//! [`DatasetView`]-facing [`PackId`] space: `map_term_ref` rewrites a resolved
//! [`TermRef`]'s id-carrying variants, `map_quad` rewrites a raw `(s, p, o, g)`
//! tuple into a [`QuadIds<PackId>`], and `unified_graph` rewrites a caller's
//! [`GraphMatch<PackId>`] back down to the `u64` space the triples/side readers
//! expect.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU64;

use crate::RdfStoreCapabilities;
use crate::dataset_view::{DatasetView, GraphMatch, ViewTermId};
use crate::ir::{QuadIds, QuadRef, TermRef, TermValue};

use super::container::PackView;
use super::dict::PackTermId;

/// The [`DatasetView`] id a [`PackView`]-backed read mints: a thin, niche-optimized
/// wrapper around the pack dictionary's unified [`PackTermId`] (see the
/// [module docs](self)). Meaningful only within the [`PackView`] that resolved it
/// (C0.8) — a durable identifier must resolve the term to its RDF value rather than
/// retain a `PackId`.
///
/// `#[repr(transparent)]` keeps the FFI layout a plain `u64`; the `NonZeroU64` inner
/// value gives `Option<PackId>` the same 8-byte niche as `Option<GlobalTermId>`
/// (every pack dictionary id is `1..=n_terms`, so `0` is a safe, never-minted
/// sentinel — see `PackDict::entry`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(transparent)]
pub struct PackId(NonZeroU64);

impl PackId {
    /// Wrap a raw unified pack id, or `None` if `id` is `0` (never a valid unified
    /// id — see `PackDict::entry`).
    #[inline]
    #[must_use]
    pub fn new(id: u64) -> Option<Self> {
        NonZeroU64::new(id).map(Self)
    }

    /// The wrapped unified pack id.
    #[inline]
    #[must_use]
    pub fn get(self) -> u64 {
        self.0.get()
    }

    /// Wrap an already-decoded, guaranteed-nonzero unified id handed back by
    /// [`super::dict::PackDict`]/[`super::triples::TriplesRef`]/
    /// [`super::side::SideTablesRef`] — every id those readers ever return is `>= 1`
    /// by construction (a successfully-opened pack's decoders validate every
    /// internal id reference once, at open time; see [`PackView::from_bytes`]), so
    /// this is an internal, panic-free-in-practice shorthand for [`Self::new`]
    /// callers that already hold that invariant, not a public fallible entry point.
    #[inline]
    #[must_use]
    fn from_unified(id: PackTermId) -> Self {
        Self(NonZeroU64::new(id).expect("pack: a decoded unified id is never 0"))
    }

    /// The wrapped unified pack id, for handing back to a `PackDict`/`TriplesRef`/
    /// `SideTablesRef` query method. An internal alias of [`Self::get`] — kept
    /// distinct so call sites read as "cross back into the raw pack id space" rather
    /// than "read the wrapped integer".
    #[inline]
    #[must_use]
    fn as_unified(self) -> PackTermId {
        self.0.get()
    }
}

// The `NonZeroU64` niche mirrors `GlobalTermId`'s (see that type's ring-fence
// asserts in `ir/global.rs`): a caller storing `Option<PackId>` (every quad's graph
// slot) pays no extra word for it.
const _: () = assert!(size_of::<PackId>() == 8);
const _: () = assert!(size_of::<Option<PackId>>() == 8);

/// `PackId` is a valid [`DatasetView`] id: its [`ViewTermId::JoinKeyAtom`] is `u128`,
/// mirroring [`crate::ir::global::GlobalTermId`]'s scheme at the same width — a
/// dataset id occupies the low 64 bits (`[0, 2^64)`, since the wrapped `PackTermId`
/// is `u64`-bounded) and a computed-term id sets bit 64 (`[2^64, 2^64 + 2^32)`), so
/// the two spaces are disjoint by construction.
impl ViewTermId for PackId {
    type JoinKeyAtom = u128;

    #[inline]
    fn encode(self) -> u128 {
        u128::from(self.get())
    }

    #[inline]
    fn encode_computed(scratch_index: u32) -> u128 {
        (1u128 << 64) | u128::from(scratch_index)
    }
}

/// Rewrite a resolved [`TermRef`] out of the `u64`-keyed [`PackTermId`] space into
/// the caller-facing [`PackId`] space: only the id-carrying variants (a literal's
/// `datatype`, a triple term's `s`/`p`/`o`) change; the borrowed string payloads are
/// untouched (same lifetime in, same lifetime out).
fn map_term_ref(term: TermRef<'_, PackTermId>) -> TermRef<'_, PackId> {
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
            datatype: PackId::from_unified(datatype),
            language,
            direction,
        },
        TermRef::Triple { s, p, o } => TermRef::Triple {
            s: PackId::from_unified(s),
            p: PackId::from_unified(p),
            o: PackId::from_unified(o),
        },
    }
}

/// Rewrite a raw unified-id `(s, p, o, g)` row — the shape every
/// [`super::triples::TriplesRef`]/[`super::side::SideTablesRef`] query returns —
/// into a [`QuadIds<PackId>`].
fn map_quad(
    (s, p, o, g): (PackTermId, PackTermId, PackTermId, Option<PackTermId>),
) -> QuadIds<PackId> {
    QuadIds {
        s: PackId::from_unified(s),
        p: PackId::from_unified(p),
        o: PackId::from_unified(o),
        g: g.map(PackId::from_unified),
    }
}

/// Rewrite a caller's [`GraphMatch<PackId>`] down to the `u64`-keyed
/// [`GraphMatch<PackTermId>`] the triples/side readers expect. `Any`/`Default` carry
/// no id and pass through unchanged; `Named` unwraps its `PackId`.
fn unified_graph(g: GraphMatch<PackId>) -> GraphMatch<PackTermId> {
    match g {
        GraphMatch::Any => GraphMatch::Any,
        GraphMatch::Default => GraphMatch::Default,
        GraphMatch::Named(id) => GraphMatch::Named(id.as_unified()),
    }
}

/// A [`PackView`] answers [`DatasetView`] directly over its decoded dictionary +
/// borrowed bitmap-triples + borrowed side-tables — no materialization step. See
/// the [module docs](self) for the id/`TermRef`/`GraphMatch` mapping this impl
/// performs at every boundary.
impl DatasetView for PackView<'_> {
    type Id = PackId;
    type ProbePlan = ();

    fn quads(&self) -> impl Iterator<Item = QuadIds<PackId>> + '_ {
        self.triples().all_quads().map(map_quad)
    }

    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_, PackId>> + '_ {
        self.quads().map(move |q| QuadRef {
            s: self.resolve(q.s),
            p: self.resolve(q.p),
            o: self.resolve(q.o),
            g: q.g.map(|g| self.resolve(g)),
        })
    }

    fn resolve(&self, id: PackId) -> TermRef<'_, PackId> {
        map_term_ref(self.dict().resolve(id.as_unified()))
    }

    fn quads_for_pattern(
        &self,
        s: Option<PackId>,
        p: Option<PackId>,
        o: Option<PackId>,
        g: GraphMatch<PackId>,
    ) -> impl Iterator<Item = QuadIds<PackId>> + '_ {
        self.triples()
            .pattern(
                s.map(PackId::as_unified),
                p.map(PackId::as_unified),
                o.map(PackId::as_unified),
                unified_graph(g),
            )
            .map(map_quad)
    }

    fn term_id_by_value(&self, value: &TermValue) -> Option<PackId> {
        self.dict().id_by_value(value).and_then(PackId::new)
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        Self::capabilities(self)
    }

    fn len_hint(&self) -> Option<usize> {
        // `cardinality_upper_bound` on the fully-unbound, any-graph pattern sums each
        // partition's own `n_triples` (see `partition_upper_bound`'s `(None, None,
        // None)` arm) — an EXACT quad count here, not merely an upper bound, and
        // cheap (`O(partitions)`, no materialization).
        Some(
            self.triples()
                .cardinality_upper_bound(None, None, None, GraphMatch::Any),
        )
    }

    fn probe_plan(
        &self,
        _s_bound: bool,
        _p_bound: bool,
        _o_bound: bool,
        _g: GraphMatch<PackId>,
    ) -> Self::ProbePlan {
    }

    fn quads_for_pattern_with_plan(
        &self,
        _plan: &(),
        s: Option<PackId>,
        p: Option<PackId>,
        o: Option<PackId>,
        g: GraphMatch<PackId>,
    ) -> impl Iterator<Item = QuadIds<PackId>> + '_ {
        // The unit plan carries nothing; forward to the pattern query, exactly like
        // `PagedDataset`'s index-free override.
        self.quads_for_pattern(s, p, o, g)
    }

    fn cardinality_estimate(
        &self,
        s: Option<PackId>,
        p: Option<PackId>,
        o: Option<PackId>,
        g: GraphMatch<PackId>,
    ) -> usize {
        self.triples().cardinality_upper_bound(
            s.map(PackId::as_unified),
            p.map(PackId::as_unified),
            o.map(PackId::as_unified),
            unified_graph(g),
        )
    }

    fn term_count(&self) -> usize {
        self.dict().n_terms() as usize
    }

    fn stats_fingerprint(&self) -> u64 {
        // Mirror `RdfDataset`/`PagedDataset`'s coarse fingerprint: hash (quad count,
        // distinct term count). A cache discriminator only, not a content digest.
        let mut h = DefaultHasher::new();
        self.len_hint().hash(&mut h);
        self.term_count().hash(&mut h);
        h.finish()
    }

    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<PackId>> + '_ {
        self.side().reifier_quads().map(map_quad)
    }

    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<PackId>> + '_ {
        self.side().annotation_quads().map(map_quad)
    }

    fn annotations_of_with_graph(
        &self,
        reifier: PackId,
    ) -> impl Iterator<Item = (PackId, PackId, Option<PackId>)> + '_ {
        self.side()
            .annotations_of_with_graph(reifier.as_unified())
            .map(|(p, o, g)| {
                (
                    PackId::from_unified(p),
                    PackId::from_unified(o),
                    g.map(PackId::from_unified),
                )
            })
    }

    fn named_graphs(&self) -> impl Iterator<Item = PackId> + '_ {
        self.triples().named_graph_ids().map(PackId::from_unified)
    }
}

/// `PackView<'static>` implements [`DatasetView`] and is `Send + Sync` — the two
/// properties the RDFC-digest verifier (`verify_pack`) and the SPARQL-over-pack
/// evaluator (in `purrdf-sparql-eval`) rely on to plug a `PackView` straight
/// into the generic evaluator. `Send`/`Sync` are auto traits with no
/// lifetime-specific opt-out in
/// this type (every field is a borrow or an owned arena), so checking the
/// `'static` instantiation certifies every shorter-lived `PackView<'a>` too.
/// Compile-time only: this function is never called, so it costs nothing at
/// runtime.
const _: fn() = || {
    fn assert_dataset_view<T: DatasetView>() {}
    fn assert_send_sync<T: Send + Sync>() {}
    assert_dataset_view::<PackView<'static>>();
    assert_send_sync::<PackView<'static>>();
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_id_niche_matches_global_term_id() {
        assert_eq!(size_of::<PackId>(), 8);
        assert_eq!(size_of::<Option<PackId>>(), 8);
    }

    #[test]
    fn pack_id_new_rejects_zero() {
        assert!(PackId::new(0).is_none());
        assert_eq!(PackId::new(1).map(PackId::get), Some(1));
        assert_eq!(PackId::new(42).map(PackId::get), Some(42));
    }

    #[test]
    fn pack_id_join_key_atoms_are_disjoint() {
        let id = PackId::new(7).expect("nonzero");
        let dataset_atom = id.encode();
        let computed_atom = PackId::encode_computed(7);
        assert!(
            dataset_atom < (1u128 << 64),
            "dataset ids occupy the low 64 bits"
        );
        assert!(
            computed_atom >= (1u128 << 64),
            "computed ids set bit 64, disjoint from any dataset id"
        );
        assert_ne!(dataset_atom, computed_atom);
    }
}
