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
//!
//! ## The admission rule (why `intern` is fallible)
//!
//! Minting is also the evaluator's only ingress for a whole `TermValue` built
//! OUTSIDE it — by a custom function, a service resolver, a property function or
//! a custom aggregate — so [`ScratchInterner::intern`] is where a language tag
//! that never met the IR kernel would otherwise reach a results writer. It asks
//! the grammar and returns [`None`] instead: see that method's docs for why the
//! gate belongs at this convergence rather than at each seam, and for the
//! SPARQL 1.1 §17.2 reading of the [`None`].

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

/// Whether `value` is a blank node the QUERY wrote inside a composite literal —
/// the one value [`ScratchInterner::intern`] must not resolve against the dataset.
///
/// A discriminant test on the value the interner is already holding, so it costs a
/// branch and runs strictly cheaper than the term-index probe it guards.
#[inline]
fn is_query_scoped_blank(value: &TermValue) -> bool {
    matches!(value, TermValue::Blank { scope, .. } if *scope == crate::convert::QUERY_BLANK_SCOPE)
}

fn hash_value(value: &TermValue) -> u64 {
    let mut hasher = ahash::AHasher::default();
    value.hash(&mut hasher);
    hasher.finish()
}

/// The profile every language tag in this workspace is judged on — the parser's,
/// the codecs', the IR kernel's, and (since `eval_str_lang`) `STRLANG`'s.
///
/// Naming the same profile here is the whole point: a value that came out of the
/// dataset was admitted by `RdfLiteral::validate_components` on THIS profile, so
/// re-judging it cannot change the verdict, and the gate below therefore costs
/// the internal callers nothing but a scan. Only a value minted outside the
/// kernel — which is to say, by an extension — can fail it.
const LANGTAG_PROFILE: purrdf_iri::langtag::Profile =
    purrdf_iri::langtag::Profile::ConcreteSyntaxLangtagBounded;

/// Whether every language tag `value` carries is one the RDF concrete syntaxes
/// would have lexed, recursing through triple-term components.
///
/// A value that fails this is not a term any writer in the workspace can
/// serialize: the results writers spell a tag as `"x"@<tag>` (TSV), as
/// `"xml:lang": "<tag>"` (JSON) and as `xml:lang="<tag>"` (XML), and every one of
/// those is bytes no reader takes back. `Iri` and `Blank` carry no tag at all, so
/// they are a discriminant test; a literal with no tag is one more branch. The
/// walk is strictly cheaper than the [`hash_value`] the intern path already pays.
fn language_tags_well_formed(value: &TermValue) -> bool {
    match value {
        TermValue::Iri(_) | TermValue::Blank { .. } => true,
        TermValue::Literal { language, .. } => language
            .as_deref()
            .is_none_or(|tag| purrdf_iri::langtag::is_well_formed_with(tag, LANGTAG_PROFILE)),
        TermValue::Triple { s, p, o } => {
            language_tags_well_formed(s)
                && language_tags_well_formed(p)
                && language_tags_well_formed(o)
        }
    }
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
    ///
    /// # The one value that is never promoted
    ///
    /// A blank node at the RESERVED query blank-node scope, `BlankScope(u32::MAX)`,
    /// was written inside a `cdt:List` / `cdt:Map` literal in the QUERY, and a query
    /// is its own blank-node document: such a node is distinct from every node of
    /// the queried data by definition, not by luck of the label. Promotion is
    /// therefore skipped for it, so the separation holds even against a dataset that
    /// (absurdly) interned a blank under the reserved scope. Every other value —
    /// including a blank a composite literal of the DATA names — takes the promotion
    /// path unchanged, which is what keeps a bare `_:b` and the `_:b` embedded in a
    /// `cdt:List` literal of the same document one node.
    ///
    /// No dataset may use that scope; `crate::convert`'s `QUERY_BLANK_SCOPE` states
    /// the reservation and is where query text is bound into it.
    ///
    /// # The one value that is never interned
    ///
    /// This is the arena's **only** door for a whole caller-supplied
    /// [`TermValue`], which makes it the single place a language tag can enter a
    /// solution without having come through the dataset. Four public extension
    /// seams hand the evaluator values it did not build — a `UserFunction`
    /// (`crate::user_fn`), a `ServiceResolver`'s rows (`crate::row_ingest`), a
    /// `PropertyFunction`'s rows (`crate::property_fn_eval`), and a
    /// `CustomAggregate`'s result (`crate::modifier::eval_custom_aggregate`) —
    /// and none of those traits constrains the language string at all. Gating
    /// each of them would be complete only until a fifth is added; gating here is
    /// complete by construction, because there is no other way into `values`.
    ///
    /// A value carrying a tag [`LANGTAG_PROFILE`] refuses is therefore **not
    /// interned** and the call returns [`None`]. That is not a dropped term: it
    /// is the "expression error" of SPARQL 1.1 §17.2, whose specified outcome is
    /// an unbound result, which is exactly what `crate::expr::eval_str_lang`
    /// already returns (`Ok(None)`) when `STRLANG` is handed the same garbage.
    /// Each caller maps the [`None`] onto the unbound outcome its own position
    /// requires; none of them may turn it back into a term.
    ///
    /// The siblings [`Self::intern_iri`], [`Self::intern_blank`] and
    /// [`Self::intern_datatyped`] stay infallible, and are not a second door:
    /// each builds the [`TermValue`] itself out of parts that cannot carry a
    /// language tag, so there is no tag for them to admit. Between them and this
    /// method they are the complete set of entries to the private
    /// [`Self::intern_checked`] body, which owns the one `values.push` in the
    /// crate.
    pub fn intern<D: DatasetView>(
        &mut self,
        dataset: &D,
        value: TermValue,
    ) -> Option<SolutionTerm<D::Id>> {
        if !language_tags_well_formed(&value) {
            return None;
        }
        Some(self.intern_checked(dataset, value))
    }

    /// Intern an IRI. Infallible by construction: an IRI carries no language tag,
    /// so [`Self::intern`]'s gate has nothing to judge.
    pub fn intern_iri<D: DatasetView>(&mut self, dataset: &D, iri: String) -> SolutionTerm<D::Id> {
        self.intern_checked(dataset, TermValue::Iri(iri))
    }

    /// Intern a blank node. Infallible by construction: a blank node carries no
    /// language tag, so [`Self::intern`]'s gate has nothing to judge.
    pub fn intern_blank<D: DatasetView>(
        &mut self,
        dataset: &D,
        label: String,
        scope: purrdf_core::BlankScope,
    ) -> SolutionTerm<D::Id> {
        self.intern_checked(dataset, TermValue::Blank { label, scope })
    }

    /// Intern a datatyped literal from its parts. Infallible by construction: the
    /// value is built here with `language: None`, so [`Self::intern`]'s gate has
    /// nothing to judge. This is how the evaluator mints its own `xsd:string` /
    /// `xsd:integer` / `xsd:boolean` results without inventing a failure branch
    /// that cannot be taken.
    pub fn intern_datatyped<D: DatasetView>(
        &mut self,
        dataset: &D,
        lexical_form: String,
        datatype: String,
    ) -> SolutionTerm<D::Id> {
        self.intern_checked(
            dataset,
            TermValue::Literal {
                lexical_form,
                datatype,
                language: None,
                direction: None,
            },
        )
    }

    /// The promotion + store-once body, on a value whose language tags have
    /// already been judged (or which provably has none).
    fn intern_checked<D: DatasetView>(
        &mut self,
        dataset: &D,
        value: TermValue,
    ) -> SolutionTerm<D::Id> {
        if !is_query_scoped_blank(&value)
            && let Some(id) = dataset.term_id_by_value(&value)
        {
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

    /// Every value in the tests below is an IRI or an untagged literal, so
    /// [`ScratchInterner::intern`]'s language-tag gate has nothing to judge and
    /// cannot return [`None`]. The gate's own two halves are pinned by
    /// [`the_language_tag_gate_refuses_only_ungrammatical_tags`] and by
    /// `tests/language_tag_seams.rs`.
    const WELL_FORMED: &str = "a value with no language tag always interns";

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
        let term = scratch
            .intern(&ds, TermValue::Iri("https://example.org/s".to_owned()))
            .expect(WELL_FORMED);
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
        let a = scratch.intern(&ds, novel.clone()).expect(WELL_FORMED);
        let b = scratch.intern(&ds, novel).expect(WELL_FORMED);
        // Absent from the dataset → Computed, and the two interns share one id.
        assert!(matches!(a, SolutionTerm::Computed(_)));
        assert_eq!(a, b);
        assert_eq!(scratch.computed_count(), 1);
    }

    #[test]
    fn existing_and_computed_are_never_equal() {
        let ds = dataset_with_one_iri();
        let mut scratch = ScratchInterner::new();
        let existing = scratch
            .intern(&ds, TermValue::Iri("https://example.org/s".to_owned()))
            .expect(WELL_FORMED);
        let computed = scratch
            .intern(
                &ds,
                TermValue::Iri("https://example.org/NOT-PRESENT".to_owned()),
            )
            .expect(WELL_FORMED);
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
        let existing = scratch.intern(&ds, iri.clone()).expect(WELL_FORMED);
        assert_eq!(scratch.value_of(&ds, existing), iri);

        let lit = TermValue::Literal {
            lexical_form: "42".to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
            language: None,
            direction: None,
        };
        let computed = scratch.intern(&ds, lit.clone()).expect(WELL_FORMED);
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

    /// One `TermValue` per tag, straight at the choke point, both halves.
    ///
    /// The accepted list is not decoration: `x-purrdf-afrikaans` and
    /// `x-gmeow-english` are tags this workspace's own artifacts carry, and
    /// `en-fr-jura` / `fr-be-fbcl` are tags approved W3C corpora carry, so
    /// refusing any of them would break data that is already valid — the mirror
    /// bug of the escape this gate closes.
    #[test]
    fn the_language_tag_gate_refuses_only_ungrammatical_tags() {
        let ds = dataset_with_one_iri();
        let tagged = |tag: &str| TermValue::Literal {
            lexical_form: "x".to_owned(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
            language: Some(tag.to_owned()),
            direction: None,
        };

        for tag in [
            "en",
            "en-US",
            "zh-Hans-CN",
            "de-CH-x-phonebk",
            "i-enochian",
            "x-purrdf-afrikaans",
            "x-gmeow-english",
            "en-fr-jura",
            "fr-be-fbcl",
            "abcdefgh",
            "en-x-cantbethislong",
        ] {
            let mut scratch = ScratchInterner::new();
            assert!(
                scratch.intern(&ds, tagged(tag)).is_some(),
                "{tag} is a tag real data carries and must still bind"
            );
            assert_eq!(scratch.computed_count(), 1, "{tag} must have been minted");
        }

        for tag in [
            "en us",
            "1",
            "9-9",
            "123-456",
            "en-",
            "-",
            "!!!",
            "abcdefghi",
            "",
        ] {
            let mut scratch = ScratchInterner::new();
            assert!(
                scratch.intern(&ds, tagged(tag)).is_none(),
                "{tag:?} must not become a solution term"
            );
            // Refused means NOT interned: the arena is untouched, so a refusal
            // costs no scratch bytes and mints no id a later row could hit.
            assert_eq!(scratch.computed_count(), 0, "{tag:?} must mint nothing");
            assert_eq!(scratch.minted_bytes(), 0, "{tag:?} must charge nothing");
        }
    }

    /// A triple term is judged through its components: an ungrammatical tag one
    /// level down is still a tag that would reach a writer.
    #[test]
    fn the_gate_recurses_through_triple_terms() {
        let ds = dataset_with_one_iri();
        let quoted = |tag: &str| TermValue::Triple {
            s: Box::new(TermValue::Iri("https://example.org/s".to_owned())),
            p: Box::new(TermValue::Iri("https://example.org/p".to_owned())),
            o: Box::new(TermValue::Literal {
                lexical_form: "x".to_owned(),
                datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                language: Some(tag.to_owned()),
                direction: None,
            }),
        };
        let mut scratch = ScratchInterner::new();
        assert!(scratch.intern(&ds, quoted("en-US")).is_some());
        assert!(scratch.intern(&ds, quoted("en us")).is_none());
    }
}
