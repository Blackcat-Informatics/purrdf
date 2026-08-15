// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Solution-term identity: the [`SolutionTerm`] bound-value representation and the
//! per-query [`ScratchInterner`] for computed terms.
//!
//! ## The unification rule (the heart of the evaluator's hot path)
//!
//! A variable binding is either a term that exists in the queried dataset
//! ([`SolutionTerm::Existing`], a dataset-local [`TermId`]) or a term *minted*
//! during evaluation ([`SolutionTerm::Computed`], a [`ScratchId`] into the
//! per-query scratch table) — e.g. the result of `CONCAT`, arithmetic, or a
//! `VALUES`/template constant absent from the data.
//!
//! [`ScratchInterner::intern`] **first probes the dataset**
//! (`term_id_by_value`, P4): if the value already exists, the binding is
//! promoted to [`SolutionTerm::Existing`] and never becomes `Computed`. The
//! consequence is load-bearing:
//!
//! - `Existing == Existing` is a raw [`TermId`] integer compare (the BGP/join hot
//!   path — zero string resolution).
//! - `Existing` vs `Computed` is **always unequal by construction**, because any
//!   value present in the dataset would have been promoted at mint time. No
//!   structural fallback is ever needed at join time.
//! - `Computed == Computed` is a [`ScratchId`] compare (the scratch interns by
//!   value, so equal values share an id).
//!
//! So [`SolutionTerm`] stays `Copy + Eq + Hash` and every join-key comparison is a
//! single integer compare. The one lookup cost (`term_id_by_value`) is paid only
//! when a term is *minted* (BIND/VALUES/aggregate output), never in BGP matching.

use purrdf_core::{DatasetView, TermId, TermRef, TermValue, ViewTermId};

use std::hash::{Hash, Hasher};

use hashbrown::HashTable;

/// An id into a [`ScratchInterner`]'s per-query table of computed terms.
///
/// Local to one query evaluation (like [`TermId`] is local to one dataset); never
/// serialized or compared across queries.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct ScratchId(u32);

impl ScratchId {
    #[inline]
    fn from_index(index: usize) -> Self {
        Self(u32::try_from(index).expect("scratch table cannot exceed u32::MAX entries"))
    }

    /// The raw table index behind this id. `pub(crate)` for
    /// [`crate::parallel::portable_row`], which must compare a `Computed` id
    /// against the parent's fork-time `computed_count()` to decide whether it
    /// was already valid in the parent's id space or freshly minted by the
    /// child after the fork.
    #[inline]
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }

    /// The raw `u32` table index, for encoding a `Computed` id into a
    /// [`ViewTermId::JoinKeyAtom`] join key (see [`SolutionTerm::join_key`]).
    #[inline]
    pub(crate) fn raw(self) -> u32 {
        self.0
    }
}

/// One bound value in a solution row.
///
/// See the [module docs](self) for the `Existing`/`Computed` unification rule that
/// keeps this `Copy` and join-comparable by a single integer compare.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SolutionTerm<I = TermId> {
    /// A term that exists in the queried dataset (the view's local id `I`).
    Existing(I),
    /// A term minted during evaluation, held in the [`ScratchInterner`].
    Computed(ScratchId),
}

impl<I: ViewTermId> SolutionTerm<I> {
    /// A total, collision-free encoding used as a single-column hash-join key, so a
    /// one-variable join key is a `Copy` [`ViewTermId::JoinKeyAtom`] with **no per-row
    /// `Vec` allocation**.
    ///
    /// `Existing` ids encode into the id space and `Computed` ids into a disjoint
    /// space (see [`ViewTermId`]'s join-key contract), so two terms encode to the same
    /// atom **iff** they are equal — the same invariant the `Existing`/`Computed`
    /// unification rule already guarantees for join correctness (a value present in the
    /// dataset is never also a `Computed` id). A collision here would be a *wrong join
    /// result*, not a slowdown. For `I = TermId` the atom is the historical packed
    /// `u64` (`Existing` in `[0, 2^32)`, `Computed` in `[2^32, 2^33)`), so the
    /// production hash-join is byte-identical.
    #[inline]
    pub(crate) fn join_key(self) -> I::JoinKeyAtom {
        match self {
            Self::Existing(id) => id.encode(),
            Self::Computed(sid) => I::encode_computed(sid.raw()),
        }
    }
}

/// The `I = TermId` monomorphization must keep its historical layout: `TermId` is a
/// `NonZeroU32`, so the two-variant enum has a spare discriminant value and
/// `Option<SolutionTerm<TermId>>` is niche-packed to the SAME size as
/// `SolutionTerm<TermId>` (no extra word). Genericizing over `I` must not grow the
/// production row cell.
const _: () = assert!(
    size_of::<Option<SolutionTerm<TermId>>>() == size_of::<SolutionTerm<TermId>>(),
    "Option<SolutionTerm<TermId>> must stay niche-packed to SolutionTerm<TermId>'s size"
);

/// A per-query interner for terms computed during evaluation.
///
/// Interns by [`TermValue`] (dataset-independent), de-duplicating equal computed
/// values to one [`ScratchId`]. Stateless with respect to the dataset: the dataset
/// is passed to each operation so the interner does not hold a borrow that would
/// conflict with the evaluator's other dataset access.
#[derive(Clone, Debug, Default)]
pub struct ScratchInterner {
    /// `ScratchId` index → the computed value.
    values: Vec<TermValue>,
    /// Store-once value index: ids only; equality resolves through `values`.
    index: HashTable<ScratchId>,
    /// The running total of [`value_bytes`] over every value ever minted into
    /// `values`, which is what [`Self::minted_bytes`] reports and what the
    /// scratch-arena resource ceiling is charged against.
    minted_bytes: u64,
}

/// A deterministic byte size for one computed value: the payload it owns, plus a fixed
/// per-term constant for the id, the discriminant, and the table slot.
///
/// This is a *deterministic proxy*, not a true heap measurement, and deliberately so.
/// True heap bytes depend on allocator behaviour and on the interner's growth history,
/// so a ceiling compared against them would trip at a different point on a different
/// allocator — which is not a governor. This function is a pure function of the value,
/// so the same query over the same data mints the same number of bytes on every target.
///
/// `pub(crate)` (not private): [`crate::modifier`]'s aggregate phase-1 buffer
/// reuses this exact proxy to charge its own retained [`TermValue`] clones
/// against [`purrdf_core::ResourceDimension::ScratchBytes`] — those clones
/// never pass through [`ScratchInterner::intern`] (they come back out through
/// [`ScratchInterner::value_of`], which mints nothing), so `minted_bytes`
/// alone would never see them; see `crate::modifier::eval_aggregate`'s doc
/// comment for why that retained buffer needs its own charge.
pub(crate) fn value_bytes(value: &TermValue) -> u64 {
    /// The per-term constant: the `ScratchId`, the index slot, and the enum
    /// discriminant, none of which vary with the value's payload.
    const TERM_OVERHEAD: u64 = 32;

    let payload = match value {
        TermValue::Iri(iri) => iri.len() as u64,
        TermValue::Blank { label, .. } => label.len() as u64,
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            ..
        } => {
            (lexical_form.len() + datatype.len() + language.as_ref().map_or(0, String::len)) as u64
        }
        TermValue::Triple { s, p, o } => value_bytes(s)
            .saturating_add(value_bytes(p))
            .saturating_add(value_bytes(o)),
    };
    payload.saturating_add(TERM_OVERHEAD)
}

fn hash_value(value: &TermValue) -> u64 {
    let mut hasher = ahash::AHasher::default();
    value.hash(&mut hasher);
    hasher.finish()
}

impl ScratchInterner {
    /// A fresh, empty interner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern a dataset-independent value to a [`SolutionTerm`], **promoting** it to
    /// [`SolutionTerm::Existing`] if `dataset` already contains the term.
    ///
    /// This is the unification rule: a value already in the data never becomes a
    /// `Computed` id, so cross-case join keys are unequal by construction.
    pub fn intern<D: DatasetView>(&mut self, dataset: &D, value: TermValue) -> SolutionTerm<D::Id> {
        if let Some(id) = dataset.term_id_by_value(&value) {
            return SolutionTerm::Existing(id);
        }
        let hash = hash_value(&value);
        if let Some(&sid) = self
            .index
            .find(hash, |sid| self.values[sid.index()] == value)
        {
            return SolutionTerm::Computed(sid);
        }
        let sid = ScratchId::from_index(self.values.len());
        self.minted_bytes = self.minted_bytes.saturating_add(value_bytes(&value));
        self.values.push(value);
        self.index
            .insert_unique(hash, sid, |sid| hash_value(&self.values[sid.index()]));
        SolutionTerm::Computed(sid)
    }

    /// The deterministic byte size of everything minted into this arena so far: each
    /// value's owned payload plus a fixed per-term constant, which makes the total a pure
    /// function of the values rather than of the allocator.
    ///
    /// This is the meter behind the scratch-arena resource ceiling. It is its own
    /// dimension because arena growth is independent of any row or cell count:
    /// `CONCAT`, `GROUP_CONCAT`, `REPLACE`, and the list constructors mint arbitrarily
    /// large owned values, so a query can exhaust memory with a row count and a cell
    /// count that both sit comfortably inside their ceilings.
    ///
    /// Monotone: de-duplicated interns add nothing, and nothing ever removes a value, so
    /// a caller can charge the difference since its last reading without double-counting.
    #[must_use]
    pub fn minted_bytes(&self) -> u64 {
        self.minted_bytes
    }

    /// Materialize a [`SolutionTerm`] to an owned, dataset-independent [`TermValue`].
    ///
    /// `Existing` ids are resolved from the dataset (recursively for RDF-1.2 triple
    /// terms, expanding the literal datatype id to its IRI string); `Computed` ids
    /// are read from the scratch table. This is the egress boundary used to build
    /// `SparqlResult` rows.
    pub fn value_of<D: DatasetView>(&self, dataset: &D, term: SolutionTerm<D::Id>) -> TermValue {
        match term {
            SolutionTerm::Existing(id) => term_id_to_value(dataset, id),
            SolutionTerm::Computed(sid) => self.values[sid.index()].clone(),
        }
    }

    /// Borrow the computed value behind a [`ScratchId`] (no clone) — the hot-path
    /// twin of [`Self::value_of`] for callers that only need to *inspect* a
    /// computed term (e.g. the comparison fast path in `expr`).
    #[must_use]
    pub fn computed_value(&self, sid: ScratchId) -> &TermValue {
        &self.values[sid.index()]
    }

    /// The number of distinct computed terms minted so far (diagnostics/tests).
    #[must_use]
    pub fn computed_count(&self) -> usize {
        self.values.len()
    }
}

/// Resolve a dataset-local [`TermId`] to an owned, dataset-independent
/// [`TermValue`].
///
/// Recurses through RDF-1.2 triple terms and expands a literal's datatype id to its
/// IRI string, so the result carries no dataset-local ids (the C0.8 boundary).
pub(crate) fn term_id_to_value<D: DatasetView>(dataset: &D, id: D::Id) -> TermValue {
    match dataset.resolve(id) {
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
            let datatype = match dataset.resolve(datatype) {
                TermRef::Iri(iri) => iri.to_owned(),
                // A literal's datatype is always an interned IRI (C0.1).
                other => unreachable!("literal datatype must be an IRI, got {other:?}"),
            };
            TermValue::Literal {
                lexical_form: lexical.to_owned(),
                datatype,
                language: language.map(str::to_owned),
                direction,
            }
        }
        TermRef::Triple { s, p, o } => TermValue::Triple {
            s: Box::new(term_id_to_value(dataset, s)),
            p: Box::new(term_id_to_value(dataset, p)),
            o: Box::new(term_id_to_value(dataset, o)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral};

    fn dataset_with_one_iri() -> std::sync::Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://example.org/s");
        let p = b.intern_iri("https://example.org/p");
        let o = b.intern_iri("https://example.org/o");
        b.push_quad(s, p, o, None);
        b.freeze().expect("freeze")
    }

    #[test]
    fn existing_value_is_promoted_not_computed() {
        let ds = dataset_with_one_iri();
        let mut scratch = ScratchInterner::new();
        let term = scratch.intern(&ds, TermValue::Iri("https://example.org/s".to_owned()));
        // The value is in the dataset → it MUST resolve to an Existing id, and the
        // scratch table stays empty (the promotion rule).
        assert!(matches!(term, SolutionTerm::Existing(_)));
        assert_eq!(scratch.computed_count(), 0);
    }

    #[test]
    fn novel_value_is_computed_and_deduped() {
        let ds = dataset_with_one_iri();
        let mut scratch = ScratchInterner::new();
        let novel = TermValue::Literal {
            lexical_form: "hello world".to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
            language: None,
            direction: None,
        };
        let a = scratch.intern(&ds, novel.clone());
        let b = scratch.intern(&ds, novel);
        // Absent from the dataset → Computed, and the two interns share one id.
        assert!(matches!(a, SolutionTerm::Computed(_)));
        assert_eq!(a, b);
        assert_eq!(scratch.computed_count(), 1);
    }

    #[test]
    fn existing_and_computed_are_never_equal() {
        let ds = dataset_with_one_iri();
        let mut scratch = ScratchInterner::new();
        let existing = scratch.intern(&ds, TermValue::Iri("https://example.org/s".to_owned()));
        let computed = scratch.intern(
            &ds,
            TermValue::Iri("https://example.org/NOT-PRESENT".to_owned()),
        );
        assert!(matches!(existing, SolutionTerm::Existing(_)));
        assert!(matches!(computed, SolutionTerm::Computed(_)));
        // The whole point of the unification rule: cross-case is unequal.
        assert_ne!(existing, computed);
    }

    #[test]
    fn value_of_round_trips_existing_and_computed() {
        let ds = dataset_with_one_iri();
        let mut scratch = ScratchInterner::new();

        let iri = TermValue::Iri("https://example.org/s".to_owned());
        let existing = scratch.intern(&ds, iri.clone());
        assert_eq!(scratch.value_of(&ds, existing), iri);

        let lit = TermValue::Literal {
            lexical_form: "42".to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
            language: None,
            direction: None,
        };
        let computed = scratch.intern(&ds, lit.clone());
        assert_eq!(scratch.value_of(&ds, computed), lit);
    }

    #[test]
    fn value_of_resolves_a_literal_from_the_dataset() {
        // A literal interned in the dataset resolves back to the same value space,
        // exercising the datatype-id → IRI expansion in `term_id_to_value`.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://example.org/s");
        let p = b.intern_iri("https://example.org/p");
        let o = b.intern_literal(RdfLiteral {
            lexical_form: "42".to_owned(),
            datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_owned()),
            language: None,
            direction: None,
        });
        b.push_quad(s, p, o, None);
        let ds = b.freeze().expect("freeze");

        let scratch = ScratchInterner::new();
        let value = scratch.value_of(&ds, SolutionTerm::Existing(o));
        assert_eq!(
            value,
            TermValue::Literal {
                lexical_form: "42".to_owned(),
                datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
                language: None,
                direction: None,
            }
        );
    }
}
