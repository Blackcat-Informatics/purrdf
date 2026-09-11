// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native **full W3C RDFC-1.0** RDF Dataset Canonicalization, oxigraph-free.
//!
//! This module is the canonicalization authority for the purrdf family. It
//! replaces `oxrdf`'s `Dataset::canonicalize` ( oxigraph eviction) and
//! supersedes the simplified FNV signature comparator that `compare.rs` used to
//! carry: it implements the real algorithm — *Hash First Degree Quads* (§4.6),
//! initial canonical assignment (§4.4), and *Hash N-Degree Quads* (§4.8) with
//! *Hash Related Blank Node* (§4.7) and **permutation backtracking** — so it
//! resolves blank-node automorphisms instead of conceding a false negative.
//!
//! ## What it produces
//!
//! [`canonicalize`] assigns every blank node a **stable canonical label**
//! (`c14n0`, `c14n1`, …) purely from graph structure and emits the **canonical
//! N-Quads** form (lines bytewise-sorted, deduplicated). Two datasets are
//! RDF-isomorphic **iff** their canonical N-Quads strings are byte-equal — an
//! exact oracle (no false positives *and* no false negatives), which is what
//! [`super::compare::datasets_isomorphic`] is rebuilt on.
//!
//! ## The working seam is VIEW-generic
//!
//! Nothing in the algorithm needs a materialized [`RdfDataset`]: it reads quads,
//! the RDF 1.2 side tables and the term table, all of which
//! [`DatasetView`] already offers. So the whole seam —
//! component collection, the blank sweep, the admissibility sweep, the first- and
//! n-degree hashes and the canonical writer — is generic over `D: DatasetView` and
//! keyed on `D::Id`, and the view entry points ([`canonicalize_view`],
//! [`try_canonicalize_view`], [`check_admissible_view`], [`blank_count_view`],
//! [`canonicalize_graph_view`]) run it directly on a composite or delta view with
//! **no intermediate dataset built anywhere**.
//!
//! The `&RdfDataset` entry points ([`canonicalize`], [`canonicalize_with`],
//! [`try_canonicalize`], [`try_canonicalize_with`], [`check_admissible`],
//! [`blank_count`]) are thin wrappers over that same core — `RdfDataset` IS a
//! `DatasetView`, so they are the `D = RdfDataset` instantiation and their output is
//! byte-identical to what it was before the seam existed, not merely equivalent.
//!
//! Identity is therefore comparable ACROSS view kinds: a composite view and the flat
//! dataset holding the same content canonicalize to the same bytes, so
//! [`super::compare::datasets_isomorphic`] compares them without materializing
//! either.
//!
//! ## SUBSUME + EXTEND: the RDF-1.2 overlay
//!
//! RDFC-1.0 is specified over triples/quads. purrdf's IR additionally carries a
//! **reifier** overlay (`reifier → triple-term` bindings) and an **annotation**
//! overlay (`reifier, predicate, object`), plus quoted **triple terms**. This
//! implementation folds all three into both the hashing and the canonical output
//! by normalizing every statement into a quad shape, using sentinel IRIs drawn
//! from the reserved [`RESERVED_NAMESPACE`]:
//!
//! - reifier `(r, t)` → `r <urn:purrdf:rdfc:reifies> t .` (`t` is the triple term)
//! - annotation `(r, p, o)` → `r p o <urn:purrdf:rdfc:annotation> .`
//!
//! Because the sentinels are disjoint from genuine quads, the **reifier COUNT**
//! and **annotation presence** stay observable in the canonical form — preserving
//! the lossless identity contract (two datasets differing only in reifier
//! count or an annotation compare UNEQUAL). RDFC-1.0 canonicalizes blank labels
//! **only**: literal lexical forms, datatypes, language tags and base directions
//! are emitted verbatim (`0.70` ≠ `0.7`, `@en--ltr` ≠ `@en--rtl`).
//!
//! ### The sentinels are reserved by REFUSAL, not by assertion
//!
//! Disjointness is what makes the overlay lossless, and nothing about the IRI
//! syntax delivers it: `urn:purrdf:rdfc:reifies` is a perfectly legal IRI that a
//! dataset may assert as an ordinary predicate. Were such a dataset canonicalized,
//! a genuine reifier structure and a literal assertion of its lowered form would
//! produce the SAME canonical bytes — and for a content-addressed consumer that
//! mints identity from those bytes, two structurally different datasets sharing a
//! digest is an identity-forgery primitive, not a curiosity.
//!
//! So the disjointness is enforced rather than assumed: a dataset carrying ANY IRI
//! in [`RESERVED_NAMESPACE`], in any position, is REFUSED — see
//! [`ReservedVocabulary`] — except in the one shape this module itself emits, which
//! is folded back (next section). Refusal is chosen over injective escaping because
//! the property a consumer has to audit ("these bytes cannot be forged") is then a
//! single total rule over the input rather than a proof about an escaping function.
//!
//! ### …except the canonicalizer's OWN output, which is FOLDED back
//!
//! The refusal is a rule about an input dataset, and the canonical document is an
//! input: parse the N-Quads this module emits and the statement layer comes back as
//! plain quads bearing the sentinels, because that is precisely what the lowering
//! wrote. Read literally, the rule therefore refuses the canonicalizer's own output —
//! `canon(canon(g))` would not merely differ, it would not complete — and an identity
//! that cannot be re-derived from the bytes it was minted from is not an identity a
//! consumer can check.
//!
//! So a quad in EXACTLY the shape the lowering emits is FOLDED back into the
//! statement layer at ingestion instead of being refused:
//!
//! - `r <urn:purrdf:rdfc:reifies> t [g] .`, with `r` an IRI or blank node and `t` a
//!   triple term, is read as the reifier binding `(r, t, g)`.
//! - `r p o <urn:purrdf:rdfc:annotation> .`, with `r` an IRI or blank node and `p` an
//!   IRI, is read as the default-graph annotation `(r, p, o)`.
//!
//! Nothing else moves. A sentinel predicate over a non-triple object, a sentinel in
//! subject, object or datatype position, the annotation sentinel anywhere but a lone
//! graph slot, and every other IRI in the namespace are refused exactly as before —
//! and the fold smuggles nothing past the sweep, because the folded row's remaining
//! slots are swept like any other (a reserved IRI nested inside the triple term still
//! refuses).
//!
//! The fold is not a hole in the anti-forgery argument, it is that argument applied
//! in the other direction. The refusal exists because two datasets with DIFFERENT
//! content must not share canonical bytes; a quad in the exact emitted shape has the
//! SAME content as the reifier or annotation row it spells — it is that row, written
//! down — so giving the two one digest is the lossless overlay working as specified.
//! The fold is total and shape-exact in both directions, so it is injective on
//! content: every folded quad denotes exactly one statement-layer row, every row is
//! spelled by exactly one quad shape, and a dataset that carries a row BOTH ways
//! carries it once (the duplicate spelling is dropped, not counted twice).
//!
//! One shape is out of reach, and it is the emitter's doing rather than the fold's: a
//! NAMED-graph annotation lowers to a five-token line (`r p o <…:annotation> <g> .`)
//! which is not an N-Quads quad at all, so no quad can carry it and none is folded.
//! The graph-scoped entry point ([`canonicalize_graph_view`]) erases the graph slot,
//! so its output is always quad-shaped and folds in full.
//!
//! ## Termination (poison guard)
//!
//! The n-degree search is NP-hard in the worst case (pathologically symmetric
//! blank graphs). Per the project no-optionality / hard-fail rule there is no
//! knob: a fixed [`RDFC_CALL_LIMIT`] bounds recursion and the routine `panic!`s
//! with a diagnostic on exhaustion rather than degrading.
//!
//! ## This is NOT RDFC-1.0
//!
//! The overlay means a dataset carrying reifiers or annotations canonicalizes to
//! bytes an RDFC-1.0 implementation would not produce, and the refusal rule means
//! this implementation rejects inputs RDFC-1.0 accepts. On the RDF 1.1 subset —
//! no reifiers, no annotations, no triple terms, no reserved IRIs — the two agree
//! byte for byte, which is what the vendored W3C `rdf-canon` suite gates.
//!
//! Everything beyond that subset belongs to a NAMED, VERSIONED profile so a
//! consumer can pin it: see [`CANON_PROFILE_ID`] / [`CANON_PROFILE_VERSION`], and
//! `docs/RDF12-CANON-PROFILE.md` for the normative specification. A digest taken
//! over this output must never be labelled "RDFC-1.0".

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use sha2::{Digest, Sha256, Sha384};

use super::dataset::{QuadIds, RdfDataset, TermRef};
use super::skolem::{TermMapper, rebuild_dataset};
use super::term::{BlankScope, TermId, TermValue};
use crate::content_store::ContentDigest;
use crate::dataset_view::{
    DatasetView, DrainCheckpoint, DrainFailure, FallibleDatasetView, GraphMatch, ViewTermId,
    checkpointed_drain,
};

/// `xsd:string` — the implicit datatype that N-Quads writes bare (no `^^<…>`).
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// The IRI namespace the RDF 1.2 overlay lowers into, reserved by this profile.
///
/// **No term of an input dataset may be an IRI in this namespace, in any position,
/// except where it spells one of the overlay's own lowered rows.** A dataset that
/// carries one anywhere else is refused with [`ReservedVocabulary`]; see
/// [`CANON_PROFILE_ID`] for why refusal rather than convention is the contract, and
/// the module documentation for the two folded shapes and why folding them is the
/// anti-forgery argument rather than an exception to it.
///
/// The rule is stated over the NAMESPACE rather than over the two sentinel spellings
/// below, and that is the load-bearing choice. An enumeration would have to be
/// re-audited every time the overlay grows a row, and the audit is exactly the step
/// that gets skipped; a namespace rule is a single sentence a reader can check against
/// the whole module. It also costs nothing to widen: no vocabulary is published here,
/// so nothing legitimate is being excluded.
pub const RESERVED_NAMESPACE: &str = "urn:purrdf:rdfc:";

/// Sentinel predicate for a reifier binding in the canonical form (§ overlay).
const SENTINEL_REIFIES: &str = "urn:purrdf:rdfc:reifies";
/// Sentinel graph for an annotation row in the canonical form (§ overlay).
const SENTINEL_ANNOTATION_GRAPH: &str = "urn:purrdf:rdfc:annotation";
/// `rdf:reifies` — the REAL predicate a reifier binding denotes once lowered to the
/// [`CanonPresentation::FlatAssertion`] shape. A base quad that spells a reifier row
/// via [`SENTINEL_REIFIES`] carries the SENTINEL id in its predicate slot, not this
/// one, and a view holding the reifier ONLY that way may never have interned this
/// IRI at all — so it is rendered as literal text ([`Component::FlatReifier`]) rather
/// than resolved through an interned [`TermId`], the same mechanism
/// [`SENTINEL_REIFIES`] itself already uses for the overlay shape.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
/// The canonical blank-label prefix (`c14n0`, `c14n1`, …) mandated by RDFC-1.0.
const CANON_PREFIX: &str = "c14n";
/// The temporary-issuer prefix used inside the n-degree search (RDFC-1.0 §4.5/4.8).
const TEMP_PREFIX: &str = "b";
/// The fixed recursion/permutation call budget for the n-degree search. Generous
/// for every non-adversarial dataset; exhaustion means a pathologically symmetric
/// blank graph and is a hard `panic!` (no knob, no degraded fallback — `.goals`).
///
/// Public because it is part of the profile's CONTRACT, not an implementation
/// detail: a consumer pinning [`CANON_PROFILE_ID`] is pinning the bound at which
/// canonicalization refuses, and a bound stated only in prose is one the consumer
/// cannot check against the code it actually linked.
pub const RDFC_CALL_LIMIT: u64 = 1_000_000;

/// The identifier of the canonicalization profile this module implements.
///
/// A consumer that mints identity from canonical bytes must be able to pin WHAT
/// produced them. Pinning a revision ("whatever `canon.rs` did at rev Z") does not
/// survive a refactor and says nothing about which behaviours are load-bearing, so
/// the algorithm — RDFC-1.0 base, the RDF 1.2 overlay lowering, the reserved
/// vocabulary, the refusal rule, the bounds — is specified under this stable name
/// in `docs/RDF12-CANON-PROFILE.md` and versioned by [`CANON_PROFILE_VERSION`].
///
/// This is an IDENTIFIER, not a vocabulary term: it is a bare token rather than an
/// IRI precisely so that nothing can dereference it, assert with it, or mistake it
/// for an ontology PurRDF does not publish.
pub const CANON_PROFILE_ID: &str = "purrdf-rdfc12";

/// The content-addressed identity of this profile's normative vector corpus.
///
/// The SHA-256 of the corpus's freeze manifest
/// (`scripts/conformance-frozen/vectors-rdf12-canon.sha256`), which in turn covers
/// every payload byte under `vectors/rdf12-canon/`. Defining it over the manifest
/// rather than over a bespoke traversal means a consumer can reproduce it with one
/// `sha256sum` and without running any of this crate's code — a digest only its
/// author can compute is not one anybody can independently check.
///
/// A consumer pins [`CANON_PROFILE_ID`], [`CANON_PROFILE_VERSION`] and this value
/// together: the first two say which algorithm was agreed, and this says which
/// evidence was agreed to demonstrate it.
pub const CANON_CORPUS_DIGEST: &str =
    "b9f367a47ebbf389f76efddb083040dae77b1987f96709bebb8af9fdb5818d3d";

/// The version of [`CANON_PROFILE_ID`] this build implements.
///
/// Incremented by any change to the canonical bytes a given dataset produces, to
/// the reserved vocabulary, to the refusal rule, or to the bounds — i.e. by any
/// change that could move a consumer's minted identity. A change that cannot move
/// output (a refactor, a faster search, a clearer diagnostic) does NOT increment
/// it, which is what makes the number worth pinning.
///
/// **v1 → v2**: the refusal rule narrowed. The two quad shapes this module's own
/// lowering emits — `r <urn:purrdf:rdfc:reifies> t [g] .` over a triple term, and
/// `r p o <urn:purrdf:rdfc:annotation> .` over an IRI predicate — are FOLDED back
/// into the statement layer instead of refused, which is what makes canonicalization
/// idempotent over its own output. Every other use of [`RESERVED_NAMESPACE`] refuses
/// exactly as in v1, and no input that v1 admitted changed bytes; the increment is
/// owed because two inputs v1 refused now canonicalize, and a refusal is part of the
/// contract a consumer pinned.
pub const CANON_PROFILE_VERSION: u32 = 2;

/// The identifier of the RDFC-1.0 overlay presentation: the RDF 1.2 statement layer
/// (reifiers, annotations) rendered through this profile's reserved sentinel IRIs
/// ([`RESERVED_NAMESPACE`]) — see [`CanonPresentation::Overlay`] for the exact shape.
/// Every entry point in this module named WITHOUT a `flat` infix ([`canonicalize`],
/// [`try_canonicalize_view`], [`canonicalize_graph_view`], …) pins this presentation;
/// it is the only presentation this module offered before
/// [`CANON_PRESENTATION_FLAT_ASSERTION_ID`] existed.
///
/// A consumer minting identity from this module's output pins FOUR coordinates
/// together: [`CANON_PROFILE_ID`] and [`CANON_PROFILE_VERSION`] say which algorithm
/// and which admissibility rule were agreed, this identifier (paired with
/// [`CANON_PRESENTATION_OVERLAY_VERSION`]) says which SHAPE the statement layer takes
/// in the output, and [`CanonHash`] is the last free choice. Like
/// [`CANON_PROFILE_ID`], this is a bare token rather than an IRI, so nothing can
/// dereference it or mistake it for vocabulary PurRDF does not publish.
pub const CANON_PRESENTATION_OVERLAY_ID: &str = "overlay";

/// The version of [`CANON_PRESENTATION_OVERLAY_ID`] this build implements.
///
/// Incremented by any change to the bytes the overlay presentation produces for a
/// given admitted view — never by a change that only affects
/// [`CANON_PRESENTATION_FLAT_ASSERTION_ID`]'s output, which versions independently.
/// What is ADMITTED (the reserved vocabulary and the refusal rule) is shared between
/// the two presentations and versioned once, by [`CANON_PROFILE_VERSION`].
pub const CANON_PRESENTATION_OVERLAY_VERSION: u32 = 1;

/// The identifier of the flat assertion presentation: the RDF 1.2 statement layer
/// lowered to ORDINARY quads carrying each row's own real predicate (`rdf:reifies`
/// for a reifier binding, the annotation's own predicate for an annotation row) —
/// no sentinel is ever minted. This holds for a row held NATIVELY (the side tables)
/// exactly as for a row a base quad merely SPELLS in the overlay's own sentinel
/// shape: the fold that reads a sentinel-spelled base quad back as a statement-layer
/// row (module documentation, "…except the canonicalizer's OWN output") runs before
/// presentation is applied, so a sentinel-spelled base quad lowers to the same
/// ordinary quad its natively-held twin does, under this presentation, just as it
/// renders through the sentinel under [`CanonPresentation::Overlay`] — the two
/// spellings of one row co-canonicalize under EITHER presentation, never only one.
/// See [`CanonPresentation::FlatAssertion`] for the exact shape and the id-level
/// dedup law, and `docs/RDF12-CANON-PROFILE.md` §3.3 for the normative
/// specification. [`try_canonicalize_flat_view`] and its `_flat_` siblings pin this
/// presentation.
///
/// Paired with [`CANON_PRESENTATION_FLAT_ASSERTION_VERSION`] as the third pinned
/// coordinate alongside [`CANON_PROFILE_ID`]/[`CANON_PROFILE_VERSION`] — see
/// [`CANON_PRESENTATION_OVERLAY_ID`] for the shared rationale.
pub const CANON_PRESENTATION_FLAT_ASSERTION_ID: &str = "flat-assertion";

/// The version of [`CANON_PRESENTATION_FLAT_ASSERTION_ID`] this build implements.
///
/// Incremented by any change to the bytes the flat presentation produces for a given
/// admitted view — never by a change that only affects
/// [`CANON_PRESENTATION_OVERLAY_ID`]'s output, which versions independently. What is
/// ADMITTED is shared between the two presentations and versioned once, by
/// [`CANON_PROFILE_VERSION`]: a dataset the profile refuses is refused under either
/// presentation, so this version cannot move by itself.
pub const CANON_PRESENTATION_FLAT_ASSERTION_VERSION: u32 = 1;

/// The RDFC-1.0 hash algorithm. SHA-256 is the default; SHA-384 is the spec's
/// alternative (RDFC-1.0 §3, exercised by W3C suite `test075`). EXTEND beyond
/// `oxrdf`, which only offered SHA-256.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanonHash {
    /// SHA-256 (the RDFC-1.0 default).
    Sha256,
    /// SHA-384.
    Sha384,
}

/// A digest rendered as fixed-capacity lowercase ASCII hex (`Copy`, so it sorts and
/// keys a `BTreeMap` without heap allocation). Holds SHA-256 (64 hex chars) or
/// SHA-384 (96 hex chars); within one canonicalization every hash shares an
/// algorithm, hence a length.
#[derive(Clone, Copy)]
struct HashHex {
    buf: [u8; 96],
    len: u8,
}

impl HashHex {
    /// The hex digits as `&str` (always valid ASCII hex by construction).
    #[inline]
    fn as_str(&self) -> &str {
        // SAFETY: bytes `[0, len)` are ASCII hex digits written by `hex_of`.
        unsafe { std::str::from_utf8_unchecked(&self.buf[..self.len as usize]) }
    }
}

impl PartialEq for HashHex {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}
impl Eq for HashHex {}
impl PartialOrd for HashHex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for HashHex {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

/// Lowercase-hex a raw digest (32 bytes for SHA-256, 48 for SHA-384) into a [`HashHex`].
fn hex_of(digest: &[u8]) -> HashHex {
    let mut buf = [0u8; 96];
    const LUT: &[u8; 16] = b"0123456789abcdef";
    for (i, byte) in digest.iter().enumerate() {
        buf[2 * i] = LUT[(byte >> 4) as usize];
        buf[2 * i + 1] = LUT[(byte & 0x0f) as usize];
    }
    HashHex {
        buf,
        len: (digest.len() * 2) as u8,
    }
}

/// Hash `bytes` under the selected algorithm, returning its lowercase hex.
fn digest_hex(hash: CanonHash, bytes: &[u8]) -> HashHex {
    match hash {
        CanonHash::Sha256 => hex_of(&Sha256::digest(bytes)),
        CanonHash::Sha384 => hex_of(&Sha384::digest(bytes)),
    }
}

/// Hash a sequence of already-serialized lines under the selected algorithm, feeding
/// each line into one running digest (Hash First Degree Quads, §4.6).
fn hash_lines(hash: CanonHash, lines: &[String]) -> HashHex {
    match hash {
        CanonHash::Sha256 => {
            let mut h = Sha256::new();
            for line in lines {
                h.update(line.as_bytes());
            }
            hex_of(&h.finalize())
        }
        CanonHash::Sha384 => {
            let mut h = Sha384::new();
            for line in lines {
                h.update(line.as_bytes());
            }
            hex_of(&h.finalize())
        }
    }
}

/// The result of canonicalizing a dataset or a view of one.
///
/// Generic over the id type `Id` — the [`Id`](DatasetView::Id) of the view
/// that produced it — and **defaulted to [`TermId`]**, so the bare spelling
/// `Canonicalized` continues to name the `&RdfDataset` result everywhere.
#[derive(Clone, Debug)]
pub struct Canonicalized<Id = TermId> {
    /// The canonical N-Quads document: every line `'\n'`-terminated, the set of
    /// lines sorted bytewise ascending and deduplicated. Blanks render as their
    /// canonical `_:c14nN` label. Includes the reified/annotated overlay (via the
    /// reserved `urn:purrdf:rdfc:` sentinels).
    ///
    /// This field carries NO view-local ids, which is what makes it comparable
    /// across view kinds: the canonical bytes of a composite view and of the flat
    /// dataset holding the same content are equal.
    pub nquads: String,
    /// Each blank id mapped to its canonical label (`"c14n0"`, …) WITHOUT
    /// the leading `_:`. The ids are those of the view that was canonicalized and
    /// are meaningful only within it.
    ///
    /// This map is the PRINCIPLED blank-label assignment: the labels are issued
    /// purely from graph structure, so they are isomorphism-invariant (two
    /// isomorphic datasets assign corresponding blanks the same label), and their
    /// alphabet is ASCII alphanumerics only — legal under every constrained
    /// egress alphabet (`BLANK_NODE_LABEL`, XML `NCName`; see
    /// [`crate::blank_label`]). [`canonical_relabel`] applies it as a dataset
    /// rewrite.
    pub labels: BTreeMap<Id, Box<str>>,
}

/// Canonicalize `ds` under profile [`CANON_PROFILE_ID`] (RDFC-1.0 with SHA-256,
/// extended by the RDF 1.2 overlay).
///
/// Deterministic and oxigraph-free.
///
/// # Panics
/// **Trusted callers only.** Hard-`panic!`s on either refusal: an n-degree search
/// exceeding [`RDFC_CALL_LIMIT`] on a pathologically symmetric blank graph, or a
/// dataset carrying an IRI in [`RESERVED_NAMESPACE`]. Both are properties of
/// ADVERSARIAL input, so a caller who cannot vouch for the dataset's provenance
/// wants [`try_canonicalize`], which returns them as values.
#[must_use]
pub fn canonicalize(ds: &RdfDataset) -> Canonicalized {
    canonicalize_with(ds, CanonHash::Sha256)
}

/// Canonicalize `ds` under profile [`CANON_PROFILE_ID`] with an explicit hash
/// algorithm ([`CanonHash::Sha384`] is RDFC-1.0's SHA-384 variant). See
/// [`canonicalize`].
///
/// # Panics
/// Trusted callers only (`.goals` no-knob contract): hard-`panic!`s on poison-budget
/// exhaustion and on reserved-vocabulary input alike — see [`try_canonicalize_with`]
/// for the fallible equivalent.
#[must_use]
pub fn canonicalize_with(ds: &RdfDataset, hash: CanonHash) -> Canonicalized {
    // A thin wrapper, not a second implementation: `RdfDataset` IS a `DatasetView`,
    // so this is the `D = RdfDataset` instantiation of the one core and its bytes
    // are identical to the view form's by construction rather than by agreement.
    canonicalize_view(ds, hash)
}

/// Canonicalize `ds` under profile [`CANON_PROFILE_ID`], returning a typed
/// [`CanonError`] instead of panicking.
///
/// **This is the entry point for UNTRUSTED input** — an independent
/// certificate-verification path folding caller-supplied bytes, or any consumer
/// minting identity from the result. Both refusals fail closed with a value the
/// caller can propagate: a pathologically symmetric blank graph never aborts the
/// process, and a dataset carrying the profile's reserved vocabulary is refused
/// rather than canonicalized into bytes another dataset could forge.
///
/// Byte-identical output to [`canonicalize`] on success — same algorithm, same
/// n-quads, same labels; only the refusal behavior differs. See [`canonicalize`]
/// for trusted callers, which panics instead.
///
/// # Errors
/// [`CanonError::ReservedVocabulary`] if any term is an IRI in
/// [`RESERVED_NAMESPACE`]; [`CanonError::BudgetExceeded`] if the n-degree search's
/// call/permutation budget ([`RDFC_CALL_LIMIT`]) is exhausted first.
pub fn try_canonicalize(ds: &RdfDataset) -> Result<Canonicalized, CanonError> {
    try_canonicalize_with(ds, CanonHash::Sha256)
}

/// Fallible, explicit-hash-algorithm form of [`try_canonicalize`]. See
/// [`canonicalize_with`] for the panicking (trusted-caller) equivalent.
///
/// # Errors
/// [`CanonError::ReservedVocabulary`] if any term is an IRI in
/// [`RESERVED_NAMESPACE`]; [`CanonError::BudgetExceeded`] if the n-degree search's
/// call/permutation budget ([`RDFC_CALL_LIMIT`]) is exhausted first.
pub fn try_canonicalize_with(
    ds: &RdfDataset,
    hash: CanonHash,
) -> Result<Canonicalized, CanonError> {
    try_canonicalize_view(ds, hash)
}

/// Canonicalize any [`DatasetView`] under profile [`CANON_PROFILE_ID`] with an
/// explicit hash algorithm — the view-generic form of [`canonicalize_with`], and the
/// function that one delegates to.
///
/// Nothing is materialized: the algorithm reads `view`'s quads, its RDF 1.2 reifier
/// and annotation rows and its term table in place, so a composite or delta view
/// canonicalizes without ever building an [`RdfDataset`]. The result's
/// [`nquads`](Canonicalized::nquads) carry no view-local ids, so a composite view and
/// the flat dataset holding the same content produce byte-equal canonical documents;
/// [`labels`](Canonicalized::labels) is keyed on `view`'s OWN ids.
///
/// The hash algorithm is explicit here rather than split across a `_with` twin: the
/// `_with` pair on the `&RdfDataset` side exists to keep its long-standing default
/// spelling, and duplicating that split over the view surface would double it for no
/// added expressiveness.
///
/// # Panics
/// **Trusted callers only**, exactly like [`canonicalize_with`]: hard-`panic!`s on
/// poison-budget exhaustion and on reserved-vocabulary input alike. See
/// [`try_canonicalize_view`] for the fallible equivalent.
#[must_use]
pub fn canonicalize_view<D: DatasetView>(view: &D, hash: CanonHash) -> Canonicalized<D::Id> {
    CanonState::new(view, CanonScope::Dataset, CanonPresentation::Overlay, hash).run()
}

/// Canonicalize any [`DatasetView`], returning a typed [`CanonError`] instead of
/// panicking — the view-generic form of [`try_canonicalize_with`], and the function
/// that one delegates to.
///
/// **This is the entry point for UNTRUSTED input**, for the reasons
/// [`try_canonicalize`] gives; byte-identical `Ok` output to [`canonicalize_view`].
///
/// # Errors
/// [`CanonError::ReservedVocabulary`] if any term of `view` is an IRI in
/// [`RESERVED_NAMESPACE`]; [`CanonError::BudgetExceeded`] if the n-degree search's
/// call/permutation budget ([`RDFC_CALL_LIMIT`]) is exhausted first.
pub fn try_canonicalize_view<D: DatasetView>(
    view: &D,
    hash: CanonHash,
) -> Result<Canonicalized<D::Id>, CanonError> {
    CanonState::new(view, CanonScope::Dataset, CanonPresentation::Overlay, hash).run_fallible()
}

/// Canonicalize the subgraph of `view` asserted in the named graph `graph` (an IRI),
/// with the graph slot erased — the per-graph identity a carrier digest pins.
///
/// Selection is graph-FAITHFUL and emission graph-ERASING, uniformly across both
/// layers, exactly as
/// [`RdfDataset::project_named_graph`](super::dataset::RdfDataset::project_named_graph)
/// defines it: a base quad contributes when its graph slot is `graph`, and a reifier
/// declaration or annotation row contributes when **its own** graph slot is `graph`;
/// each is canonicalized with no graph name. The statement layer is keyed per graph in
/// this IR, so one reifier id may be declared and annotated independently in two
/// graphs and only the rows belonging to `graph` are admitted — the digest is a
/// function of `graph`'s content ALONE.
///
/// The difference from the projection route is that no projection exists: the rows are
/// selected through a [`GraphMatch::Named`] pattern probe and the side tables' own
/// graph slots, so the output is byte-identical to canonicalizing the materialized
/// projection without building it. A `graph` this view interns nowhere, or interns but
/// never uses as a graph name, names no rows and canonicalizes to the empty document —
/// absence is an empty subgraph, never an error.
///
/// # Panics
/// Trusted callers only, exactly like [`canonicalize_view`]; see
/// [`try_canonicalize_graph_view`] for the fallible equivalent.
#[must_use]
pub fn canonicalize_graph_view<D: DatasetView>(
    view: &D,
    graph: &str,
    hash: CanonHash,
) -> Canonicalized<D::Id> {
    match graph_scope(view, graph) {
        Some(scope) => CanonState::new(view, scope, CanonPresentation::Overlay, hash).run(),
        None => empty_canonicalized(),
    }
}

/// Canonicalize one named graph of `view`, returning a typed [`CanonError`] instead of
/// panicking. See [`canonicalize_graph_view`] for the selection rule.
///
/// # Errors
/// [`CanonError::ReservedVocabulary`] if any term of the selected subgraph is an IRI
/// in [`RESERVED_NAMESPACE`]; [`CanonError::BudgetExceeded`] on a pathologically
/// symmetric blank graph. The sweep is scoped to the subgraph, so a reserved IRI in
/// ANOTHER graph does not refuse this one — the projection route has exactly that
/// property, and a per-graph digest that refused on a neighbour's content would not be
/// a function of `graph` alone.
pub fn try_canonicalize_graph_view<D: DatasetView>(
    view: &D,
    graph: &str,
    hash: CanonHash,
) -> Result<Canonicalized<D::Id>, CanonError> {
    match graph_scope(view, graph) {
        Some(scope) => {
            CanonState::new(view, scope, CanonPresentation::Overlay, hash).run_fallible()
        }
        None => Ok(empty_canonicalized()),
    }
}

/// The SHA-256 [`ContentDigest`] of one named graph's canonical form — the value a
/// per-graph content handle pins, taken over [`canonicalize_graph_view`]'s bytes.
///
/// # Panics
/// Trusted callers only, exactly like [`canonicalize_graph_view`]; see
/// [`try_graph_digest_view`] for the fallible equivalent.
#[must_use]
pub fn graph_digest_view<D: DatasetView>(view: &D, graph: &str) -> ContentDigest {
    ContentDigest::of(
        canonicalize_graph_view(view, graph, CanonHash::Sha256)
            .nquads
            .as_bytes(),
    )
}

/// The SHA-256 [`ContentDigest`] of one named graph's canonical form, returning a
/// typed [`CanonError`] instead of panicking. Byte-identical `Ok` output to
/// [`graph_digest_view`].
///
/// # Errors
/// Exactly [`try_canonicalize_graph_view`]'s refusals, unchanged.
pub fn try_graph_digest_view<D: DatasetView>(
    view: &D,
    graph: &str,
) -> Result<ContentDigest, CanonError> {
    Ok(ContentDigest::of(
        try_canonicalize_graph_view(view, graph, CanonHash::Sha256)?
            .nquads
            .as_bytes(),
    ))
}

// -----------------------------------------------------------------------------
// The flat-presentation, fault-aware entry points.
//
// Every function below composes two things the overlay-pinned family above never
// had to: the [`FlatAssertion`](CanonPresentation::FlatAssertion) presentation, and
// [`checkpointed_drain`]'s two-checkpoint completeness law over a
// [`FallibleDatasetView`]. A canonical run internally drains `view` several times
// over one call — component collection runs once to build [`CanonState`], once more
// for the reserved-vocabulary sweep, and once more to serialize the result (three
// passes, not one) — and [`checkpointed_drain`] does not checkpoint each pass
// individually: it samples [`FallibleDatasetView::operation_status`] once before the
// FIRST of those passes and once after the LAST, bracketing the whole run rather than
// each pass on its own. That is sufficient rather than a gap, because a view's fault
// is STICKY once raised (`FallibleDatasetView`'s own contract: "the first operational
// failure becomes sticky, every iterator stops yielding"), so a fault raised during
// ANY internal pass is still observable at the final sample — the AFTER checkpoint is
// what refuses a view that faulted BETWEEN internal passes, not only one already
// broken before the run began.
// -----------------------------------------------------------------------------

/// The `Result` shape [`try_canonicalize_flat_view`] and
/// [`try_canonicalize_flat_graph_view`] share. Named purely to satisfy clippy's
/// `type_complexity` lint on a signature that nests one generic inside another —
/// the type itself hides nothing a caller could not already see spelled out.
type FlatViewResult<D> = Result<
    Canonicalized<<D as DatasetView>::Id>,
    ViewCanonError<<D as FallibleDatasetView>::Error, <D as FallibleDatasetView>::Evidence>,
>;

/// Canonicalize any [`FallibleDatasetView`] under profile [`CANON_PROFILE_ID`] in the
/// [`FlatAssertion`](CanonPresentation::FlatAssertion) presentation
/// ([`CANON_PRESENTATION_FLAT_ASSERTION_ID`] /
/// [`CANON_PRESENTATION_FLAT_ASSERTION_VERSION`]), with an explicit hash algorithm —
/// the flat-presentation, fault-aware sibling of [`try_canonicalize_view`].
///
/// Where [`try_canonicalize_view`] emits the RDFC-1.0 overlay — a reifier or
/// annotation row rendered through the profile's reserved sentinel IRIs — this entry
/// point emits every statement-layer row as an ORDINARY quad carrying its row's own
/// real predicate (`rdf:reifies`, or the annotation's own predicate); a row that
/// already exists as a genuine base quad is emitted exactly once (the id-level dedup
/// law). What is ADMITTED is unchanged from the overlay — only what is EMITTED
/// differs — so the refusals below are exactly [`try_canonicalize_view`]'s.
///
/// [`Canonicalized::nquads`] is isomorphism-invariant, exactly as under the overlay:
/// two isomorphic views produce byte-equal flat documents. [`Canonicalized::labels`]
/// is invariant only UP TO AUTOMORPHISM — where a graph carries a nontrivial
/// automorphism, the n-degree search's tie-break among otherwise-equivalent
/// candidates is enumeration-order-dependent, so two isomorphic-but-not-identical
/// views may assign automorphic blanks different (but structurally equivalent)
/// labels. Label issuance does not read the presentation axis at all, so this is the
/// same caveat the overlay carries, unchanged.
///
/// See the module-level note above this function for why bracketing the run's THREE
/// internal drain passes with only two checkpoints is complete rather than a gap.
///
/// # Implementor obligation
/// Like every consumer generic over `D: DatasetView` that takes a whole pass over a
/// view (see the termination and snapshot-ingestion obligations documented on
/// [`DatasetView`] itself), this entry point trusts that `view` presents
/// structurally valid rows. A view that never passed freeze validation and hands
/// this function a dangling or cyclic reference has committed a contract violation
/// of its own making — one this function can neither detect nor recover from.
///
/// # Errors
/// [`ViewCanonError::Refused`] wrapping [`CanonError::ReservedVocabulary`] if any
/// term of `view` is an IRI in [`RESERVED_NAMESPACE`]; wrapping
/// [`CanonError::BudgetExceeded`] if the n-degree search's call/permutation budget
/// ([`RDFC_CALL_LIMIT`]) is exhausted first. [`ViewCanonError::NotReady`] if either
/// checkpoint observes the view's backing data at fault.
pub fn try_canonicalize_flat_view<D: FallibleDatasetView>(
    view: &D,
    hash: CanonHash,
) -> FlatViewResult<D> {
    match checkpointed_drain(view, |v| {
        CanonState::new(
            v,
            CanonScope::Dataset,
            CanonPresentation::FlatAssertion,
            hash,
        )
        .run_fallible()
    }) {
        Ok(Ok(canonicalized)) => Ok(canonicalized),
        Ok(Err(refused)) => Err(ViewCanonError::Refused(refused)),
        Err(failure) => Err(failure.into()),
    }
}

/// Canonicalize one named graph of any [`FallibleDatasetView`] in the
/// [`FlatAssertion`](CanonPresentation::FlatAssertion) presentation — the graph
/// scope of [`canonicalize_graph_view`] composed with
/// [`try_canonicalize_flat_view`]'s presentation and fault-checkpointing. Selection
/// is exactly [`canonicalize_graph_view`]'s rule (graph-FAITHFUL selection,
/// graph-ERASING emission); a `graph` the view interns nowhere, or interns but never
/// uses as a graph name, canonicalizes to the empty document rather than an error,
/// exactly as there. See [`try_canonicalize_flat_view`] for the presentation, the
/// labels-up-to-automorphism caveat, and the checkpoint-brackets-the-whole-run law
/// this entry point shares.
///
/// # Errors
/// Exactly [`try_canonicalize_flat_view`]'s refusals, scoped to the selected
/// subgraph: a reserved IRI in ANOTHER graph does not refuse this one.
pub fn try_canonicalize_flat_graph_view<D: FallibleDatasetView>(
    view: &D,
    graph: &str,
    hash: CanonHash,
) -> FlatViewResult<D> {
    match checkpointed_drain(view, |v| match graph_scope(v, graph) {
        Some(scope) => {
            CanonState::new(v, scope, CanonPresentation::FlatAssertion, hash).run_fallible()
        }
        None => Ok(empty_canonicalized()),
    }) {
        Ok(Ok(canonicalized)) => Ok(canonicalized),
        Ok(Err(refused)) => Err(ViewCanonError::Refused(refused)),
        Err(failure) => Err(failure.into()),
    }
}

/// Whether any [`FallibleDatasetView`] is admissible to canonicalization under
/// profile [`CANON_PROFILE_ID`] — the flat-presentation, fault-aware sibling of
/// [`check_admissible_view`]. Admissibility does not depend on presentation (see
/// [`CanonPresentation::FlatAssertion`]'s documentation: only what is EMITTED
/// differs, never what is ADMITTED), so this predicate agrees with
/// [`check_admissible_view`] on every input; it exists so a flat-presentation caller
/// can screen a view before deciding whether to canonicalize it, exactly as
/// [`check_admissible_view`] does for the overlay. See
/// [`try_canonicalize_flat_view`] for the checkpoint-brackets-the-whole-run law this
/// entry point shares.
///
/// # Errors
/// [`ViewCanonError::Refused`] wrapping [`CanonError::ReservedVocabulary`] naming the
/// least offending `(position, iri)`. [`ViewCanonError::NotReady`] if either
/// checkpoint observes the view's backing data at fault.
pub fn check_admissible_flat_view<D: FallibleDatasetView>(
    view: &D,
) -> Result<(), ViewCanonError<D::Error, D::Evidence>> {
    match checkpointed_drain(view, |v| {
        reserved_vocabulary(v, CanonScope::Dataset, CanonPresentation::FlatAssertion)
    }) {
        Ok(None) => Ok(()),
        Ok(Some(violation)) => Err(ViewCanonError::Refused(CanonError::ReservedVocabulary(
            violation,
        ))),
        Err(failure) => Err(failure.into()),
    }
}

/// The SHA-256 [`ContentDigest`] of `view`'s WHOLE-DATASET flat canonical
/// form — the [`FlatAssertion`](CanonPresentation::FlatAssertion) sibling of
/// [`graph_digest_view`]/[`try_graph_digest_view`], taken over the whole dataset
/// rather than one named graph (this module offers no flat PER-GRAPH digest entry
/// point). `hash` selects only the RDFC label-issuance algorithm inside the
/// canonicalization; the digest over the resulting document is unconditionally
/// SHA-256, exactly as every other [`ContentDigest`] in this module. Equal to
/// [`ContentDigest::of`] applied to [`try_canonicalize_flat_view`]'s
/// [`Canonicalized::nquads`] under the same `hash`, because that is exactly what
/// this function does.
///
/// # Errors
/// Exactly [`try_canonicalize_flat_view`]'s refusals, unchanged.
pub fn try_flat_digest_view<D: FallibleDatasetView>(
    view: &D,
    hash: CanonHash,
) -> Result<ContentDigest, ViewCanonError<D::Error, D::Evidence>> {
    Ok(ContentDigest::of(
        try_canonicalize_flat_view(view, hash)?.nquads.as_bytes(),
    ))
}

/// The [`CanonScope`] naming `graph` in `view`, or `None` when `view` interns no such
/// IRI (so no quad or statement row can carry it as a graph name).
fn graph_scope<D: DatasetView>(view: &D, graph: &str) -> Option<CanonScope<D::Id>> {
    view.term_id_by_value(&TermValue::iri(graph))
        .map(CanonScope::Graph)
}

/// The canonical form of an empty selection: the empty document, no labels.
fn empty_canonicalized<Id>() -> Canonicalized<Id> {
    Canonicalized {
        nquads: String::new(),
        labels: BTreeMap::new(),
    }
}

/// Relabel every blank node of `ds` to its canonical `c14n{n}` label at
/// [`BlankScope::DEFAULT`], returning a NEW frozen dataset with all other
/// terms, quads, reifiers, annotations, named-graph declarations, and quad
/// source locations preserved.
///
/// This is the "choose the labels yourself" recourse. Serialization is already
/// total — the serializers escape a blank label illegal in the target syntax's
/// alphabet (see [`crate::blank_label`]) — but the escape is a mechanical
/// rewrite of whatever the caller happened to hold, while this operation issues
/// labels from graph structure, before egress, so the document carries a
/// principled labeling the caller picked rather than an escape. The `c14n{n}`
/// labels come from [`try_canonicalize`], so they are ASCII alphanumerics
/// (legal as `BLANK_NODE_LABEL` and as an XML `NCName` alike) and
/// isomorphism-invariant — two isomorphic inputs relabel to the same canonical
/// labeling, and relabeling is idempotent (relabeling a relabeled dataset
/// changes nothing up to canonical bytes).
///
/// Collapsing every blank to [`BlankScope::DEFAULT`] is sound because canonical
/// labels are already unique across the whole dataset, so no two distinct
/// blanks can collide in the single scope.
///
/// The non-serialized derived side tables (`content_ids`,
/// `predecessors`/`predecessor_chain`) survive the rewrite: `ds`'s
/// [`ContentIdScheme`](crate::ContentIdScheme) (and derivation predicate, if
/// configured) is carried forward onto the output via the crate-private
/// `rebuild_dataset` helper (see [`super::skolem`]), so content addressing
/// re-derives from the (here, unchanged) IRI bytes and the predecessor index
/// resolves over the carried-forward annotation table exactly as it did on
/// `ds`.
///
/// A blank node that canonicalization cannot observe — a blank DECLARED as a
/// named graph that owns no quads (declaration-only) — still gets a fresh
/// `c14n{n}` label, continuing the canonical numbering in ascending
/// `(label, scope)` value order, so the output never leaks a hostile label.
///
/// # Errors
/// Propagates [`try_canonicalize`]'s refusals unchanged:
/// [`CanonError::ReservedVocabulary`] if any term is an IRI in
/// [`RESERVED_NAMESPACE`]; [`CanonError::BudgetExceeded`] on a pathologically
/// symmetric blank graph (adversarial input never panics here).
pub fn canonical_relabel(ds: &RdfDataset) -> Result<RdfDataset, CanonError> {
    relabel_recording(ds, |_, _| {})
}

/// A native canonical relabeling paired with the term mapping established by
/// its rebuild. Constructed by [`canonical_relabel_with_mapping`].
///
/// The mapping is local to the exact input dataset and this output. It is not a
/// persistent identifier, provenance certificate, or assertion of semantic
/// equivalence beyond the relabeling operation.
#[derive(Debug)]
pub struct CanonicalRelabeling {
    /// The canonical native dataset; never rendered or reparsed.
    dataset: RdfDataset,
    /// Source-indexed output IDs; dictionary-only unused terms remain absent.
    terms: Box<[Option<TermId>]>,
}

impl CanonicalRelabeling {
    /// Borrow the rewritten dataset, including its RDF 1.2 record surfaces.
    #[must_use]
    pub const fn dataset(&self) -> &RdfDataset {
        &self.dataset
    }

    /// Map a term from the **exact input dataset** into [`Self::dataset`].
    ///
    /// Returns `None` for unused dictionary entries that were not reached by
    /// the rebuild, or IDs beyond the input dictionary. Datatype terms, nested
    /// triple components, composite embedded blanks and already-interned IRIs,
    /// reifier/annotation terms and declaration-only graphs are included.
    ///
    /// Like other [`TermId`] APIs, this cannot detect a same-index ID belonging
    /// to another dataset. Neither input nor output IDs may be persisted as
    /// portable identities. Resolve them and bind any durable records to the
    /// identity of the exact source and output datasets.
    #[must_use]
    pub fn map_term(&self, source: TermId) -> Option<TermId> {
        self.terms.get(source.index()).copied().flatten()
    }

    /// Consume the result, discarding its dataset-local mapping.
    /// Translate source handles before calling this method if needed.
    #[must_use]
    pub fn into_dataset(self) -> RdfDataset {
        self.dataset
    }
}

/// Canonically relabel a native dataset and retain its exact term mapping.
///
/// Shares [`canonical_relabel`]'s one canonical-label search and one native
/// rebuild. No canonical document is rendered, parsed or searched again to
/// recover term correspondence. Recording uses four bytes per input term and
/// constant-time lookup; the existing [`canonical_relabel`] allocates no map.
/// All native records and side tables preserved by that function are preserved
/// here as well. Mapping is observational: it never skips positional checks.
///
/// # Errors
/// Returns exactly [`canonical_relabel`]'s admission and search-budget refusals.
pub fn canonical_relabel_with_mapping(ds: &RdfDataset) -> Result<CanonicalRelabeling, CanonError> {
    let mut terms = vec![None; ds.term_count()].into_boxed_slice();
    let dataset = relabel_recording(ds, |source, target| {
        let source = match source {
            RelabelSource::Term(id) => Some(id),
            RelabelSource::IndirectIri(iri) => ds.term_id_by_iri(iri),
        };
        if let Some(source) = source {
            let slot = &mut terms[source.index()];
            assert!(
                slot.is_none_or(|previous| previous == target),
                "a canonical term rewrite must agree across occurrences"
            );
            *slot = Some(target);
        }
    })?;
    Ok(CanonicalRelabeling { dataset, terms })
}

/// A source term reached either directly or inside a composite literal.
enum RelabelSource<'a> {
    /// A source dictionary ID encountered by the native rebuild.
    Term(TermId),
    /// A lexical or implicit reifier IRI; only the mapping consumer looks it up.
    IndirectIri(&'a str),
}

/// Shared label search and native traversal. The no-op recorder monomorphizes
/// away, including the embedded-IRI lookup needed only by the mapped result.
fn relabel_recording(
    ds: &RdfDataset,
    record: impl FnMut(RelabelSource<'_>, TermId),
) -> Result<RdfDataset, CanonError> {
    // The typed consumer needs the issued labels, not a serialized document.
    // Keep the exact admission/search algorithm shared with text canonicalization
    // and move its label table without rendering or cloning it.
    let labels = CanonState::new(
        ds,
        CanonScope::Dataset,
        CanonPresentation::Overlay,
        CanonHash::Sha256,
    )
    .issue_labels()?
    .canonical
    .issued;
    // Declaration-only blank graphs are invisible to canonicalization (they own
    // no statement), so continue the canonical numbering over them in a
    // value-deterministic order.
    let mut unseen: Vec<(&str, BlankScope, TermId)> = ds
        .named_graphs()
        .filter(|g| !labels.contains_key(g))
        .filter_map(|g| match ds.resolve(g) {
            TermRef::Blank { label, scope } => Some((label, scope, g)),
            _ => None,
        })
        .collect();
    unseen.sort_unstable();
    let extra: BTreeMap<TermId, Box<str>> = unseen
        .iter()
        .enumerate()
        .map(|(i, &(_, _, id))| {
            let label = format!("{CANON_PREFIX}{}", labels.len() + i);
            (id, label.into_boxed_str())
        })
        .collect();
    rebuild_dataset(
        ds,
        &mut CanonicalRelabeler {
            labels: &labels,
            extra,
            record,
        },
    )
}

/// The [`canonical_relabel`] mapper: blanks take their issued `c14n{n}` label
/// at [`BlankScope::DEFAULT`]; every other term passes through unchanged.
struct CanonicalRelabeler<'a, R> {
    /// The canonicalization's issued labels ([`Canonicalized::labels`]).
    labels: &'a BTreeMap<TermId, Box<str>>,
    /// Continuation labels for declaration-only blank graphs.
    extra: BTreeMap<TermId, Box<str>>,
    /// Receives actual term pairs during the same rebuild traversal.
    record: R,
}

impl<R: FnMut(RelabelSource<'_>, TermId)> TermMapper for CanonicalRelabeler<'_, R> {
    type Error = CanonError;

    fn map_blank(
        &mut self,
        builder: &mut super::builder::RdfDatasetBuilder,
        id: TermId,
        _label: &str,
        _scope: BlankScope,
    ) -> Result<TermId, CanonError> {
        let label = self
            .labels
            .get(&id)
            .or_else(|| self.extra.get(&id))
            .expect("every blank node holds a canonical or continuation label");
        Ok(builder.intern_blank(label, BlankScope::DEFAULT))
    }

    fn map_iri(
        &mut self,
        builder: &mut super::builder::RdfDatasetBuilder,
        iri: &str,
        _iri_only: bool,
    ) -> Result<TermId, CanonError> {
        Ok(builder.intern_iri(iri))
    }

    fn record_term(&mut self, source: TermId, target: TermId) {
        (self.record)(RelabelSource::Term(source), target);
    }

    fn record_embedded_iri(&mut self, iri: &str, target: TermId) {
        (self.record)(RelabelSource::IndirectIri(iri), target);
    }

    fn record_reifier_predicate(&mut self, builder: &super::builder::RdfDatasetBuilder) {
        if let Some(target) = builder.reifies_predicate() {
            (self.record)(
                RelabelSource::IndirectIri("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"),
                target,
            );
        }
    }
}

/// Whether `ds` is admissible to canonicalization under profile
/// [`CANON_PROFILE_ID`] — i.e. carries no IRI in [`RESERVED_NAMESPACE`] outside the
/// overlay's own lowered shapes, which are folded back rather than refused (module
/// documentation).
///
/// Exposed separately so a dataset can be screened at ADMISSION, before it is
/// stored, rather than only at the moment identity is minted. A store that admits
/// an inadmissible dataset has not been compromised — canonicalization will still
/// refuse it — but it has accepted bytes it can never canonicalize, and finding
/// that out at write time is strictly better than at read time.
///
/// # Errors
/// [`ReservedVocabulary`] naming the least offending `(position, iri)`.
pub fn check_admissible(ds: &RdfDataset) -> Result<(), ReservedVocabulary> {
    check_admissible_view(ds)
}

/// Whether any [`DatasetView`] is admissible to canonicalization under profile
/// [`CANON_PROFILE_ID`] — the view-generic form of [`check_admissible`], and the
/// function that one delegates to.
///
/// Screens a composite or delta view in place, so a view can be admitted or refused
/// before anyone decides whether to materialize it.
///
/// # Errors
/// [`ReservedVocabulary`] naming the least offending `(position, iri)`.
pub fn check_admissible_view<D: DatasetView>(view: &D) -> Result<(), ReservedVocabulary> {
    reserved_vocabulary(view, CanonScope::Dataset, CanonPresentation::Overlay).map_or(Ok(()), Err)
}

/// The count of distinct blank nodes in `ds` (incl. blanks nested inside triple
/// terms). A cheap structural pre-reject used by [`super::compare`].
#[must_use]
pub fn blank_count(ds: &RdfDataset) -> usize {
    blank_count_view(ds)
}

/// The count of distinct blank nodes any [`DatasetView`] carries (incl. blanks nested
/// inside triple terms and inside composite literals) — the view-generic form of
/// [`blank_count`], and the function that one delegates to.
///
/// Label-independent by construction: it counts view ids, so two views whose blanks
/// differ only in `(label, scope)` count the same, and two sources of a composite that
/// happen to share a local label count as the two distinct nodes they are.
#[must_use]
pub fn blank_count_view<D: DatasetView>(view: &D) -> usize {
    let mut set: BTreeSet<D::Id> = BTreeSet::new();
    collect_components(
        view,
        CanonScope::Dataset,
        CanonPresentation::Overlay,
        &mut |comp| {
            comp.for_each_blank(view, &mut |b| {
                set.insert(b);
            });
        },
    );
    set.len()
}

/// Which statements of a view canonicalization admits.
///
/// A scope is a SELECTION, never a rewrite: the same components are hashed and written
/// either way, so the whole-dataset and per-graph paths share one implementation and
/// cannot drift apart in their handling of the RDF 1.2 overlay.
#[derive(Clone, Copy)]
enum CanonScope<Id> {
    /// Every quad, reifier row and annotation row the view carries, each keeping its
    /// own graph slot.
    Dataset,
    /// Only the rows whose OWN graph slot is this named graph, each emitted with the
    /// graph slot erased — see [`canonicalize_graph_view`].
    Graph(Id),
}

/// How the RDF 1.2 statement layer (reifiers, annotations) is EXPRESSED in the
/// component collector's output — a second axis, orthogonal to the internal
/// canonicalization scope's SELECTION axis: scope picks which rows are admitted,
/// presentation picks how an admitted statement-layer row is shaped.
///
/// Exhaustive, deliberately with NO [`Default`]: every caller states which one it
/// means, so a presentation can never be reached by omission. Public as an
/// identity/documentation vocabulary ONLY: it names the two presentations and
/// anchors their normative docs, but NO function — public or private — accepts it as
/// a parameter, so a caller never threads this enum across an API boundary. A
/// caller SELECTS a presentation by which entry point it calls instead: each
/// presentation gets its own explicitly-named entry points. [`canonicalize`],
/// [`try_canonicalize_view`], [`canonicalize_graph_view`] and their kin pin
/// [`Overlay`](Self::Overlay); [`try_canonicalize_flat_view`],
/// [`try_canonicalize_flat_graph_view`], [`check_admissible_flat_view`] and
/// [`try_flat_digest_view`] pin [`FlatAssertion`](Self::FlatAssertion). See
/// [`CANON_PRESENTATION_OVERLAY_ID`] / [`CANON_PRESENTATION_FLAT_ASSERTION_ID`] for
/// the stable, versioned identifiers a consumer pins for each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonPresentation {
    /// The RDFC-1.0 overlay this module has always emitted: a reifier row becomes a
    /// `Component::Reifier` and an annotation row becomes a `Component::Annotation`,
    /// each rendered through the profile's reserved sentinel IRIs
    /// (`SENTINEL_REIFIES`/`SENTINEL_ANNOTATION_GRAPH`).
    Overlay,
    /// The flat assertion presentation: a reifier or annotation row lowers to an
    /// ORDINARY quad carrying its row's own real predicate id (`rdf:reifies`, or the
    /// annotation's own predicate) — no sentinel is ever minted. A lowered row whose
    /// `(s, p, o, g)` already exists as a genuine base quad is emitted exactly once
    /// (see `already_asserted`'s flat dedup law). What is ADMITTED is unchanged
    /// from [`Overlay`](Self::Overlay) — only what is EMITTED differs.
    ///
    /// **The law holds for BOTH ways a row can reach this presentation.** A row held
    /// NATIVELY (the side tables) lowers through `emit_reifier_row` /
    /// `emit_annotation_row` to a `Component::Quad` carrying a real interned
    /// predicate id. A row a base quad merely SPELLS — the overlay's own sentinel
    /// shape, recognized by `fold_sentinel_row` and read back as the same
    /// statement-layer row (module documentation, "…except the canonicalizer's OWN
    /// output") — lowers through `lower_folded_row` to the SAME ordinary shape: a
    /// reifier spelling lowers to `Component::FlatReifier` (the predicate rendered
    /// as literal text, because the view holding only the spelled quad may never have
    /// interned `rdf:reifies` at all — see `RDF_REIFIES`), and an annotation
    /// spelling lowers directly to a `Component::Quad` (its predicate slot already
    /// held a real interned id, taken straight from the spelling quad). Either way the
    /// bytes rendered are identical to the row's natively-held twin, so the two
    /// spellings co-canonicalize under this presentation exactly as they already did
    /// under [`Overlay`](Self::Overlay) — never only one of the two. The dedup law
    /// above extends accordingly: a row present as a native side-table entry, as a
    /// sentinel-spelled base quad, and as a real-predicate base quad, in any
    /// combination, is still emitted exactly once (`lower_folded_row` drops a
    /// spelled row already covered by a real-predicate base quad, and
    /// `already_native` drops one already covered by a native row).
    ///
    /// Its production consumers are [`try_canonicalize_flat_view`],
    /// [`try_canonicalize_flat_graph_view`], [`check_admissible_flat_view`] and
    /// [`try_flat_digest_view`].
    FlatAssertion,
}

/// A statement normalized to a quad shape for uniform hashing and serialization.
/// Predicate/graph slots may be a reserved sentinel IRI (the overlay rows).
///
/// Keyed on the id type of the view it was collected from, so the same component
/// machinery serves a flat dataset, a composite view and a delta view alike.
#[derive(Clone, Copy)]
enum Component<Id> {
    /// A genuine dataset quad.
    Quad { s: Id, p: Id, o: Id, g: Option<Id> },
    /// A reifier binding `r <urn:purrdf:rdfc:reifies> t` in graph `g` (`None` =
    /// default graph — the graph slot then stays empty, byte-identical to the
    /// pre-graph-dimension form).
    Reifier { r: Id, t: Id, g: Option<Id> },
    /// An annotation `r p o` in the reserved annotation graph, itself scoped to graph
    /// `g` (`None` = default graph).
    Annotation { r: Id, p: Id, o: Id, g: Option<Id> },
    /// A reifier binding lowered to the [`CanonPresentation::FlatAssertion`] shape
    /// from a base quad that merely SPELLED it (the overlay's sentinel shape,
    /// recognized by [`fold_sentinel_row`]): the ordinary quad `r rdf:reifies t
    /// [g] .`, with the predicate rendered as literal text ([`RDF_REIFIES`]) rather
    /// than an interned [`TermId`], because the view holding only the spelled quad
    /// may never have interned the real `rdf:reifies` IRI at all.
    ///
    /// Renders BYTE-IDENTICALLY to the [`Component::Quad`] a natively-held reifier
    /// row lowers to via [`emit_reifier_row`] — [`write_slot`] resolves a
    /// [`Slot::Term`] IRI and writes a [`Slot::Sentinel`] literal through the exact
    /// same `<…>`-escaping call, so two slots carrying the same IRI TEXT are
    /// indistinguishable in the output regardless of which one is interned (pinned by
    /// [`tests::a_flat_reifier_slot_renders_identically_whether_interned_or_literal`]).
    /// A reifier row lowered from the NATIVE side table never takes this variant —
    /// only a base quad's spelling does — so the two paths never compete for the
    /// same output line by construction; they merely happen to render to it.
    FlatReifier { r: Id, t: Id, g: Option<Id> },
}

/// One quad slot: a dataset term, a synthetic sentinel IRI (overlay predicate) that
/// has no [`TermId`], or the annotation-overlay graph marker (the reserved annotation
/// sentinel plus the annotation's own named graph, if any).
#[derive(Clone, Copy)]
enum Slot<Id> {
    Term(Id),
    Sentinel(&'static str),
    /// The annotation overlay's graph position: the reserved annotation sentinel and,
    /// for a named-graph annotation, the graph term. `None` renders exactly as the
    /// bare sentinel (byte-identical to the default-graph form); `Some(g)` appends the
    /// real graph term so a named-graph annotation stays lossless and distinct from a
    /// genuine quad. The graph term keeps its id so a blank-node graph still
    /// participates in canonical labeling.
    AnnotationGraph(Option<Id>),
}

impl<Id: ViewTermId> Component<Id> {
    /// The four quad slots `(s, p, o, g)` of this component in canonical shape.
    fn slots(self) -> (Slot<Id>, Slot<Id>, Slot<Id>, Option<Slot<Id>>) {
        match self {
            Self::Quad { s, p, o, g } => (
                Slot::Term(s),
                Slot::Term(p),
                Slot::Term(o),
                g.map(Slot::Term),
            ),
            Self::Reifier { r, t, g } => (
                Slot::Term(r),
                Slot::Sentinel(SENTINEL_REIFIES),
                Slot::Term(t),
                // The reifier's graph reuses the (previously always-empty) graph slot,
                // so a default-graph reifier (`g == None`) is byte-identical to before.
                g.map(Slot::Term),
            ),
            Self::Annotation { r, p, o, g } => (
                Slot::Term(r),
                Slot::Term(p),
                Slot::Term(o),
                Some(Slot::AnnotationGraph(g)),
            ),
            Self::FlatReifier { r, t, g } => (
                Slot::Term(r),
                Slot::Sentinel(RDF_REIFIES),
                Slot::Term(t),
                g.map(Slot::Term),
            ),
        }
    }

    /// Invoke `f` for every blank id appearing anywhere in this component
    /// (recursing into triple terms).
    fn for_each_blank<D: DatasetView<Id = Id>>(self, ds: &D, f: &mut impl FnMut(Id)) {
        let (s, p, o, g) = self.slots();
        for slot in [Some(s), Some(p), Some(o), g].into_iter().flatten() {
            match slot {
                Slot::Term(id) | Slot::AnnotationGraph(Some(id)) => blanks_in_term(ds, id, f),
                Slot::Sentinel(_) | Slot::AnnotationGraph(None) => {}
            }
        }
    }
}

/// Invoke `f` for every blank id reachable at `id` — recursing triple
/// terms, and descending into the lexical form of a composite (`cdt:List` /
/// `cdt:Map`) literal.
///
/// A blank node embedded in a composite literal is a blank node OF THE GRAPH
/// (see [`crate::cdt_blank`]), so it must participate in canonical labeling like
/// any other. Leaving it out would make two datasets that differ only by a
/// consistent renaming of such a node canonicalize differently, and would let
/// [`canonical_relabel`] emit a dangling label.
fn blanks_in_term<D: DatasetView>(ds: &D, id: D::Id, f: &mut impl FnMut(D::Id)) {
    match ds.resolve(id) {
        TermRef::Blank { .. } => f(id),
        TermRef::Triple { s, p, o } => {
            blanks_in_term(ds, s, f);
            blanks_in_term(ds, p, f);
            blanks_in_term(ds, o, f);
        }
        TermRef::Literal {
            lexical, datatype, ..
        } => {
            for id in composite_blanks(ds, lexical, datatype) {
                f(id);
            }
        }
        TermRef::Iri(_) => {}
    }
}

/// The view ids of the blank nodes a composite literal's lexical form names,
/// in occurrence order and deduplicated.
///
/// Empty unless `datatype` is one of the two composite IRIs, so an ordinary
/// literal costs one datatype resolve and two string comparisons.
///
/// Every `(label, scope)` a composite literal names was interned when the
/// literal was
/// ([`intern_literal`](super::builder::RdfDatasetBuilder::intern_literal) does
/// it), so the lookup normally succeeds; a pair the view does not hold names
/// no node of this graph and is skipped rather than fabricated.
///
/// The lookup goes through [`DatasetView::term_id_by_value`], which resolves WITHOUT
/// minting — the same non-minting resolve the flat path's `term_id_by_blank` performs,
/// so an embedded pair naming no node stays absent rather than becoming one.
fn composite_blanks<D: DatasetView>(ds: &D, lexical: &str, datatype: D::Id) -> Vec<D::Id> {
    let TermRef::Iri(iri) = ds.resolve(datatype) else {
        return Vec::new();
    };
    if !crate::cdt_blank::is_cdt_datatype(iri) {
        return Vec::new();
    }
    let mut seen = BTreeSet::new();
    crate::cdt_blank::cdt_embedded_blanks(lexical, iri)
        .into_iter()
        .filter_map(|(label, scope)| ds.term_id_by_value(&TermValue::Blank { label, scope }))
        .filter(|id| seen.insert(*id))
        .collect()
}

/// The ids, in ONE view's own id space, of the overlay's two sentinel IRIs.
///
/// Looked up once per component sweep and compared by id afterwards, so a view that
/// interns neither — every dataset that has never been through a canonical document —
/// pays two value resolves for the whole sweep and nothing at all per quad. A view
/// that does intern one pays one `Option` comparison per quad, and the quad's terms
/// are resolved only once that comparison has already matched.
#[derive(Clone, Copy)]
struct Sentinels<Id> {
    /// The id of `urn:purrdf:rdfc:reifies`, if this view interns it.
    reifies: Option<Id>,
    /// The id of `urn:purrdf:rdfc:annotation`, if this view interns it.
    annotation: Option<Id>,
}

impl<Id: ViewTermId> Sentinels<Id> {
    /// Resolve both sentinels against `ds` WITHOUT minting — the same non-minting
    /// resolve [`graph_scope`] performs, and with the same consequence: an IRI the
    /// view interns nowhere names no term of it, so no quad can carry it and no quad
    /// can be in a folded shape.
    ///
    /// A view that somehow held one IRI under two ids would fold only the id this
    /// resolve names; the other would reach the admissibility sweep, which compares
    /// IRIs by VALUE, and be refused. That failure direction is the safe one — a
    /// missed fold refuses, it never admits a second spelling.
    fn of<D: DatasetView<Id = Id>>(ds: &D) -> Self {
        Self {
            reifies: ds.term_id_by_value(&TermValue::iri(SENTINEL_REIFIES)),
            annotation: ds.term_id_by_value(&TermValue::iri(SENTINEL_ANNOTATION_GRAPH)),
        }
    }
}

/// Whether `id` resolves to a term legal in an ASSERTED subject position — an IRI or
/// a blank node.
///
/// The quad subject and the statement layer's reifier slot carry the same rule at
/// freeze time (`require_asserted_subject`), so on a frozen dataset it holds already;
/// it is checked anyway because the fold's whole safety argument is that the shape it
/// recognizes is EXACTLY the shape the lowering emits, and a shape test that assumes
/// away one of its conjuncts is not exact.
fn is_asserted_subject<D: DatasetView>(ds: &D, id: D::Id) -> bool {
    matches!(ds.resolve(id), TermRef::Iri(_) | TermRef::Blank { .. })
}

/// The statement-layer row a base quad SPELLS when it is in exactly the shape this
/// module's own lowering emits, or `None` when it is an ordinary quad.
///
/// This is the ingestion half of the overlay: [`Component::slots`] writes a reifier
/// out as `r <…:reifies> t [g]` and a default-graph annotation as `r p o <…:annotation>`,
/// and this reads those two shapes back. The two are inverses by construction, which
/// is what makes canonicalization idempotent over its own output.
///
/// `q.g` is the graph slot the scope will EMIT, not necessarily the one the quad
/// stores: under [`CanonScope::Graph`] the slot is erased before the fold sees it, so
/// canonicalizing the graph literally named `<urn:purrdf:rdfc:annotation>` keeps
/// behaving as it always has (the name is erased, so nothing is folded and nothing is
/// refused) rather than acquiring a meaning from a name the caller chose.
fn fold_sentinel_row<D: DatasetView>(
    ds: &D,
    sentinels: Sentinels<D::Id>,
    q: QuadIds<D::Id>,
) -> Option<Component<D::Id>> {
    if sentinels.reifies == Some(q.p) {
        return (is_asserted_subject(ds, q.s) && matches!(ds.resolve(q.o), TermRef::Triple { .. }))
            .then_some(Component::Reifier {
                r: q.s,
                t: q.o,
                g: q.g,
            });
    }
    if let Some(annotation) = sentinels.annotation
        && q.g == Some(annotation)
    {
        return (is_asserted_subject(ds, q.s) && matches!(ds.resolve(q.p), TermRef::Iri(_)))
            .then_some(Component::Annotation {
                r: q.s,
                p: q.p,
                o: q.o,
                // The lowering spends the graph slot on the sentinel itself, so the
                // shape it emits is the DEFAULT-graph annotation and nothing else; a
                // named-graph annotation lowers to a five-token line no quad can hold.
                g: None,
            });
    }
    None
}

/// Whether `scope` already admits `folded` from the view's OWN side tables.
///
/// A dataset may carry one statement-layer row both natively and as the quad that
/// spells it. The fold's claim is that those are the same row, so the canonical form
/// must hold it once: the duplicate spelling is dropped here rather than counted
/// twice. Dropping (rather than, say, refusing the pair) is what keeps the claim
/// symmetric — a dataset and the same dataset with one row spelled twice are the same
/// content, hence the same bytes.
///
/// Only reached when a fold actually fired, so a view carrying no sentinel-shaped quad
/// never walks a side table here.
fn already_native<D: DatasetView>(
    ds: &D,
    scope: CanonScope<D::Id>,
    folded: Component<D::Id>,
) -> bool {
    match folded {
        Component::Reifier { r, t, g } => ds
            .reifier_quads()
            .any(|row| row.s == r && row.o == t && scope_emits(scope, row.g, g)),
        Component::Annotation { r, p, o, g } => ds
            .annotation_quads()
            .any(|row| row.s == r && row.p == p && row.o == o && scope_emits(scope, row.g, g)),
        // The fold never produces either of these directly: a quad is what it
        // declines to fold, and `FlatReifier` is produced only by `lower_folded_row`,
        // AFTER this check has already run on the `Reifier` it lowers.
        Component::Quad { .. } | Component::FlatReifier { .. } => false,
    }
}

/// Whether a side-table row whose OWN graph slot is `row` is admitted by `scope` and
/// emitted with the graph slot `emitted` — the comparison [`already_native`] needs,
/// stated once so the two scopes cannot drift apart in it.
fn scope_emits<Id: ViewTermId>(
    scope: CanonScope<Id>,
    row: Option<Id>,
    emitted: Option<Id>,
) -> bool {
    match scope {
        CanonScope::Dataset => row == emitted,
        CanonScope::Graph(graph) => emitted.is_none() && row == Some(graph),
    }
}

/// The component a base quad contributes under `scope`/`presentation` — itself, the
/// statement-layer row it spells (shaped for `presentation`), or nothing at all when
/// it spells a row already covered elsewhere (natively, under [`already_native`], or
/// — under [`CanonPresentation::FlatAssertion`] only — by a genuine real-predicate
/// base quad, under [`lower_folded_row`]).
///
/// `graph_probe` is the row's OWN graph, stated as a pattern, for
/// [`lower_folded_row`]'s real-predicate dedup probe — passed separately from `q.g`
/// because under [`CanonScope::Graph`] the caller has already ERASED `q.g` to `None`
/// (the scope's emission rule) before this function ever sees it, while the probe
/// must still name the scope's actual graph. See [`collect_components`]'s two call
/// sites, which mirror exactly how [`emit_reifier_row`]/[`emit_annotation_row`]
/// already split `graph_probe` from `emitted_g` for the same reason.
fn base_quad_component<D: DatasetView>(
    ds: &D,
    scope: CanonScope<D::Id>,
    presentation: CanonPresentation,
    graph_probe: GraphMatch<D::Id>,
    sentinels: Sentinels<D::Id>,
    q: QuadIds<D::Id>,
) -> Option<Component<D::Id>> {
    let Some(folded) = fold_sentinel_row(ds, sentinels, q) else {
        return Some(Component::Quad {
            s: q.s,
            p: q.p,
            o: q.o,
            g: q.g,
        });
    };
    if already_native(ds, scope, folded) {
        return None;
    }
    match presentation {
        CanonPresentation::Overlay => Some(folded),
        CanonPresentation::FlatAssertion => lower_folded_row(ds, graph_probe, folded),
    }
}

/// Lower a statement-layer row [`fold_sentinel_row`] recognized from a base quad's
/// overlay-sentinel spelling to the ordinary shape [`CanonPresentation::FlatAssertion`]
/// emits for it — or `None` when a genuine real-predicate base quad ALREADY asserts
/// the same `(s, p, o, g)`, so THAT quad (visited separately by
/// [`collect_components`]'s own `ds.quads()`/pattern walk, and never itself folded,
/// since its predicate is the real one, not the sentinel) is the row's sole emitter.
///
/// This is the FLAT-presentation half of the dedup law [`already_native`] states for
/// the overlay: a row may be spelled as a native side-table entry, as a
/// sentinel-spelled base quad, and/or as a real-predicate base quad, in any
/// combination, and must be emitted exactly once. `already_native` (checked by the
/// caller before this function runs) covers the native-row combinations; this
/// function covers the one combination that is possible only between two base
/// quads, which `already_native` cannot see because neither side of it is a
/// side-table row.
///
/// A reifier row's real predicate is looked up WITHOUT minting
/// ([`DatasetView::term_id_by_value`]): a view that never interned `rdf:reifies` can
/// hold no real-predicate base quad naming it, so the probe is skipped rather than
/// forced, and the row lowers straight to [`Component::FlatReifier`]. An annotation
/// row's predicate is already a real interned id — it came straight from the
/// spelling quad's own predicate slot, never from a sentinel — so no lookup is
/// needed there at all.
fn lower_folded_row<D: DatasetView>(
    ds: &D,
    graph_probe: GraphMatch<D::Id>,
    folded: Component<D::Id>,
) -> Option<Component<D::Id>> {
    match folded {
        Component::Reifier { r, t, g } => {
            let dup = ds
                .term_id_by_value(&TermValue::iri(RDF_REIFIES))
                .is_some_and(|p| already_asserted(ds, graph_probe, r, p, t));
            (!dup).then_some(Component::FlatReifier { r, t, g })
        }
        Component::Annotation { r, p, o, g } => {
            // NOT `graph_probe`: for the annotation fold the sentinel occupies the
            // quad's OWN graph slot, so `graph_probe` names that sentinel graph —
            // matching the spelling quad against itself. The row the fold denotes is
            // always the DEFAULT-graph annotation (`fold_sentinel_row` hard-codes
            // `g: None`), so the real-predicate counterpart this checks against must
            // be probed in the default graph, unconditionally.
            let dup = already_asserted(ds, GraphMatch::Default, r, p, o);
            (!dup).then_some(Component::Quad { s: r, p, o, g })
        }
        // `fold_sentinel_row` never constructs either of these; only `Reifier` and
        // `Annotation` are foldable shapes.
        Component::Quad { .. } | Component::FlatReifier { .. } => {
            unreachable!("fold_sentinel_row never produces this shape")
        }
    }
}

/// Drive `f` over every [`Component`] the `scope` admits (quads, reifiers,
/// annotations), read straight off the view's accessors.
///
/// The RDF 1.2 rows come from [`DatasetView::reifier_quads`] /
/// [`DatasetView::annotation_quads`], the virtual-quad shape of the side tables:
/// `(reifier, rdf:reifies, triple-term, graph)` and `(reifier, predicate, object,
/// graph)`. For [`RdfDataset`] those yield exactly the rows, in exactly the order, that
/// its `reifiers_with_graph` / `annotations_with_graph` tables hold — which is what
/// makes the flat wrappers byte-identical to the pre-seam implementation.
///
/// Under [`CanonScope::Graph`] the selection is a [`GraphMatch::Named`] pattern probe
/// for the base quads and the side tables' own graph slots for the overlay rows, and
/// every admitted component is emitted with its graph slot ERASED — the projection's
/// rule, applied without building a projection.
///
/// This is also where a base quad that SPELLS a statement-layer row is folded back
/// into one ([`base_quad_component`]). Placing the fold here rather than at an entry
/// point is what makes it reach everything: the flat wrappers, the view-generic entry
/// points, the per-graph scope, the admissibility sweep and the blank sweep all read
/// the dataset through this one function, so none of them can disagree about what the
/// input contains.
fn collect_components<D: DatasetView>(
    ds: &D,
    scope: CanonScope<D::Id>,
    presentation: CanonPresentation,
    f: &mut impl FnMut(Component<D::Id>),
) {
    let sentinels = Sentinels::of(ds);
    match scope {
        CanonScope::Dataset => {
            for q in ds.quads() {
                let graph_probe = graph_match_of(q.g);
                if let Some(comp) =
                    base_quad_component(ds, scope, presentation, graph_probe, sentinels, q)
                {
                    f(comp);
                }
            }
            for q in ds.reifier_quads() {
                emit_reifier_row(ds, presentation, graph_match_of(q.g), q, q.g, f);
            }
            for q in ds.annotation_quads() {
                emit_annotation_row(ds, presentation, graph_match_of(q.g), q, q.g, f);
            }
        }
        CanonScope::Graph(graph) => {
            for q in ds.quads_for_pattern(None, None, None, GraphMatch::Named(graph)) {
                // The graph slot is erased FIRST, so the fold reads the quad as this
                // scope will emit it (see [`fold_sentinel_row`]) — but the dedup probe
                // still needs the scope's ACTUAL graph, so it is passed separately
                // rather than derived from the erased slot (see
                // [`base_quad_component`]'s doc comment).
                let q = QuadIds { g: None, ..q };
                if let Some(comp) = base_quad_component(
                    ds,
                    scope,
                    presentation,
                    GraphMatch::Named(graph),
                    sentinels,
                    q,
                ) {
                    f(comp);
                }
            }
            for q in ds.reifier_quads().filter(|q| q.g == Some(graph)) {
                emit_reifier_row(ds, presentation, GraphMatch::Named(graph), q, None, f);
            }
            for q in ds.annotation_quads().filter(|q| q.g == Some(graph)) {
                emit_annotation_row(ds, presentation, GraphMatch::Named(graph), q, None, f);
            }
        }
    }
}

/// The [`GraphMatch`] naming exactly the quads whose graph slot is `g` — [`Default`]
/// (the default graph) for `None`, [`Named`] for `Some`. The lookup
/// [`already_asserted`]'s dedup probe needs to state a reifier/annotation row's OWN
/// graph slot as a pattern, once, rather than at every call site.
///
/// [`Default`]: GraphMatch::Default
/// [`Named`]: GraphMatch::Named
fn graph_match_of<Id: ViewTermId>(g: Option<Id>) -> GraphMatch<Id> {
    g.map_or(GraphMatch::Default, GraphMatch::Named)
}

/// Whether the view's OWN base quads already assert `(s, p, o)` in `graph_probe` —
/// the [`CanonPresentation::FlatAssertion`] dedup law's test: a side-table row lowered
/// to an ordinary [`Component::Quad`] must not be emitted a second time when a genuine
/// base quad already spells the same `(s, p, o, g)`.
///
/// Checked AT THE ID LEVEL against [`DatasetView::quads_for_pattern`] — never against
/// rendered/serialized text — so a duplicate never reaches [`CanonState`]'s incident
/// map in the first place; a component that never exists cannot perturb a first-degree
/// hash the way a component built and only deduplicated at the text layer could.
fn already_asserted<D: DatasetView>(
    ds: &D,
    graph_probe: GraphMatch<D::Id>,
    s: D::Id,
    p: D::Id,
    o: D::Id,
) -> bool {
    ds.quads_for_pattern(Some(s), Some(p), Some(o), graph_probe)
        .next()
        .is_some()
}

/// Emit one reifier side-table virtual quad `q` (from
/// [`DatasetView::reifier_quads`]) under `presentation`: the
/// [`CanonPresentation::Overlay`] sentinel shape ([`Component::Reifier`]), or under
/// [`CanonPresentation::FlatAssertion`] the ordinary quad `(q.s, q.p, q.o,
/// emitted_g)` — `q.p` is already the row's real `rdf:reifies` id, so nothing is
/// resolved or minted here. Dropped instead when [`already_asserted`] finds a base
/// quad already spelling it (the flat dedup law).
fn emit_reifier_row<D: DatasetView>(
    ds: &D,
    presentation: CanonPresentation,
    graph_probe: GraphMatch<D::Id>,
    q: QuadIds<D::Id>,
    emitted_g: Option<D::Id>,
    f: &mut impl FnMut(Component<D::Id>),
) {
    match presentation {
        CanonPresentation::Overlay => f(Component::Reifier {
            r: q.s,
            t: q.o,
            g: emitted_g,
        }),
        CanonPresentation::FlatAssertion => {
            if !already_asserted(ds, graph_probe, q.s, q.p, q.o) {
                f(Component::Quad {
                    s: q.s,
                    p: q.p,
                    o: q.o,
                    g: emitted_g,
                });
            }
        }
    }
}

/// Emit one annotation side-table virtual quad `q` (from
/// [`DatasetView::annotation_quads`]) under `presentation`: the
/// [`CanonPresentation::Overlay`] sentinel shape ([`Component::Annotation`]), or under
/// [`CanonPresentation::FlatAssertion`] the ordinary quad `(q.s, q.p, q.o,
/// emitted_g)` the row's own real predicate id already names. Dropped instead when
/// [`already_asserted`] finds a base quad already spelling it (the flat dedup law).
fn emit_annotation_row<D: DatasetView>(
    ds: &D,
    presentation: CanonPresentation,
    graph_probe: GraphMatch<D::Id>,
    q: QuadIds<D::Id>,
    emitted_g: Option<D::Id>,
    f: &mut impl FnMut(Component<D::Id>),
) {
    match presentation {
        CanonPresentation::Overlay => f(Component::Annotation {
            r: q.s,
            p: q.p,
            o: q.o,
            g: emitted_g,
        }),
        CanonPresentation::FlatAssertion => {
            if !already_asserted(ds, graph_probe, q.s, q.p, q.o) {
                f(Component::Quad {
                    s: q.s,
                    p: q.p,
                    o: q.o,
                    g: emitted_g,
                });
            }
        }
    }
}

/// How a blank renders during serialization.
#[derive(Clone, Copy)]
enum BlankRender<'a, Id> {
    /// Hash First Degree Quads (§4.6): the focus blank → `_:a`, every other → `_:z`.
    FirstDegree { focus: Id },
    /// Final output (§4.4 step 7): each blank → its issued `_:c14nN` label.
    Canonical { issuer: &'a IdIssuer<Id> },
}

impl<'a, Id: ViewTermId> BlankRender<'a, Id> {
    /// The `_:`-less label a blank renders to under this strategy. Borrowed: the
    /// first-degree labels are static and the canonical ones live in the issuer,
    /// so no `String` is minted per blank occurrence written.
    fn label(self, id: Id) -> &'a str {
        match self {
            BlankRender::FirstDegree { focus } => {
                if id == focus {
                    "a"
                } else {
                    "z"
                }
            }
            BlankRender::Canonical { issuer } => issuer
                .issued_for(id)
                .expect("every blank has a canonical id at output time"),
        }
    }
}

/// The RDFC-1.0 "identifier issuer": mints prefixed ids (`c14n0`, `b0`, …) in a
/// stable order, remembering each blank's id and the issuance order.
#[derive(Clone)]
struct IdIssuer<Id> {
    prefix: &'static str,
    issued: BTreeMap<Id, Box<str>>,
    order: Vec<Id>,
}

impl<Id: ViewTermId> IdIssuer<Id> {
    fn new(prefix: &'static str) -> Self {
        Self {
            prefix,
            issued: BTreeMap::new(),
            order: Vec::new(),
        }
    }

    /// Issue (or return the already-issued) id for `b`.
    fn issue(&mut self, b: Id) -> &str {
        // One tree descent for both outcomes (`entry`), instead of the
        // contains + insert + get triple this replaced. The id is still numbered
        // from `order.len()` BEFORE the push, so the issued sequence is unchanged.
        let next = self.order.len();
        match self.issued.entry(b) {
            std::collections::btree_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::btree_map::Entry::Vacant(e) => {
                self.order.push(b);
                e.insert(format!("{}{next}", self.prefix).into_boxed_str())
            }
        }
    }

    fn issued_for(&self, b: Id) -> Option<&str> {
        self.issued.get(&b).map(Box::as_ref)
    }

    fn has(&self, b: Id) -> bool {
        self.issued.contains_key(&b)
    }

    /// The blanks in issuance order.
    fn order(&self) -> &[Id] {
        &self.order
    }
}

/// Per-run canonicalization state over ONE view.
///
/// Generic over `D: DatasetView` and keyed on `D::Id`: the working tables below are
/// scratch, and the view itself is read in place — no [`RdfDataset`] is built here for
/// any view, including the composite and delta ones.
struct CanonState<'a, D: DatasetView> {
    ds: &'a D,
    /// Which statements of `ds` this run admits.
    scope: CanonScope<D::Id>,
    /// How this run's statement-layer rows are expressed — see [`CanonPresentation`].
    presentation: CanonPresentation,
    /// Every blank, in ascending id order (the deterministic reference set).
    blanks: Vec<D::Id>,
    /// The components each blank participates in (its "quads", RDFC-1.0 §4.4).
    incident: BTreeMap<D::Id, Vec<Component<D::Id>>>,
    /// First-degree hash (§4.6) of each blank, computed once.
    first_degree: BTreeMap<D::Id, HashHex>,
    /// The durable canonical issuer.
    canonical: IdIssuer<D::Id>,
    /// The hash algorithm for this run (RDFC-1.0 §3).
    hash: CanonHash,
    /// Remaining recursion/permutation budget (poison guard).
    budget: u64,
}

/// Internal early-unwind carrier for the poison-budget guard (no payload —
/// [`CanonState::run_fallible`] attaches the blank count when it surfaces the
/// public [`BudgetExceeded`] to a caller).
struct Exhausted;

/// The n-degree search's call/permutation budget (`RDFC_CALL_LIMIT`) was
/// exhausted before the dataset canonicalized — a pathologically symmetric
/// blank graph (adversarial input, not a legitimate large dataset: a
/// non-symmetric graph of any size stays well under budget). Returned by
/// [`try_canonicalize`]/[`try_canonicalize_with`] instead of the panic that
/// [`canonicalize`]/[`canonicalize_with`] raise for trusted callers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BudgetExceeded {
    /// The number of distinct blank nodes in the input that triggered
    /// exhaustion (diagnostic only).
    pub blank_count: usize,
}

impl std::fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RDFC-1.0 canonicalization exceeded its call budget ({RDFC_CALL_LIMIT}) on a \
             pathologically symmetric blank graph ({} blanks); the input is adversarial and \
             cannot be canonicalized deterministically within bounds",
            self.blank_count
        )
    }
}

impl std::error::Error for BudgetExceeded {}

/// The quad position a refused reserved IRI was found in.
///
/// A reserved IRI nested inside a triple term reports the position that triple term
/// itself occupies — the refusal is about the statement, and naming the outer slot
/// is what lets a caller find the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum TermPosition {
    /// The subject slot.
    Subject,
    /// The predicate slot.
    Predicate,
    /// The object slot.
    Object,
    /// The graph slot.
    Graph,
}

impl std::fmt::Display for TermPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Subject => "subject",
            Self::Predicate => "predicate",
            Self::Object => "object",
            Self::Graph => "graph",
        })
    }
}

/// The input dataset carries an IRI in the profile's [`RESERVED_NAMESPACE`] somewhere
/// other than in one of the overlay's own lowered shapes, which canonicalization
/// refuses rather than lower alongside its own sentinels.
///
/// Accepting such a dataset would let a structure with DIFFERENT content canonicalize
/// to a genuine reifier/annotation structure's bytes — an identity collision, and for
/// a content-addressed store an identity-forgery primitive. The two exact shapes the
/// lowering emits are the one case where the content is not different: those are
/// folded back into the statement layer instead of refused, which is what makes
/// canonicalization idempotent over its own output. See the module documentation for
/// both halves of that argument, and for why the rule is refusal at the namespace
/// rather than escaping at the two sentinel spellings.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReservedVocabulary {
    /// The offending IRI, in full.
    pub iri: Box<str>,
    /// The quad position it was found in.
    pub position: TermPosition,
}

impl std::fmt::Display for ReservedVocabulary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the dataset carries the reserved IRI <{}> in the {} position; \
             <{RESERVED_NAMESPACE}…> is reserved by canonicalization profile \
             {CANON_PROFILE_ID} v{CANON_PROFILE_VERSION} for the RDF 1.2 overlay and \
             cannot appear in an input dataset",
            self.iri, self.position
        )
    }
}

impl std::error::Error for ReservedVocabulary {}

/// Why canonicalization refused.
///
/// Both variants are refusals of ADVERSARIAL input, and they are separate variants
/// rather than one opaque error because they oblige a caller differently: a
/// [`BudgetExceeded`] dataset is well-formed and merely uncanonicalizable within
/// bounds, while a [`ReservedVocabulary`] dataset is one whose acceptance would have
/// been an identity collision. A consumer auditing a rejection needs to tell those
/// apart without parsing a message.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CanonError {
    /// The n-degree search's call/permutation budget was exhausted.
    BudgetExceeded(BudgetExceeded),
    /// The input carries an IRI in the profile's reserved namespace.
    ReservedVocabulary(ReservedVocabulary),
}

impl std::fmt::Display for CanonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BudgetExceeded(err) => err.fmt(f),
            Self::ReservedVocabulary(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for CanonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BudgetExceeded(err) => Some(err),
            Self::ReservedVocabulary(err) => Some(err),
        }
    }
}

impl From<BudgetExceeded> for CanonError {
    fn from(err: BudgetExceeded) -> Self {
        Self::BudgetExceeded(err)
    }
}

impl From<ReservedVocabulary> for CanonError {
    fn from(err: ReservedVocabulary) -> Self {
        Self::ReservedVocabulary(err)
    }
}

/// The refusal surfaced by a [`FallibleDatasetView`]-generic view-canon entry point:
/// [`try_canonicalize_flat_view`], [`try_canonicalize_flat_graph_view`],
/// [`check_admissible_flat_view`] and [`try_flat_digest_view`].
///
/// Two DIFFERENT kinds of refusal, and a caller must be able to tell them apart
/// without parsing a message. [`Refused`](Self::Refused) is exactly [`CanonError`] —
/// the input itself is inadmissible (reserved vocabulary) or the n-degree search
/// exhausted its budget, the same two refusals [`try_canonicalize_view`] already
/// returns for an infallible view. [`NotReady`](Self::NotReady) is the OTHER kind
/// entirely: [`checkpointed_drain`]'s two-checkpoint completeness law observed the
/// view's OWN backing data at fault, either already broken before a single row was
/// read or faulted somewhere over the run — so nothing about the DATASET was ever
/// judged, because the view could not even be read to completion. Collapsing the two
/// into one opaque error would make that distinction a string a caller has to parse
/// back out of a message; keeping them apart means one obliges the caller to fix
/// their INPUT and the other to retry or repair their VIEW.
///
/// [`NotReady`](Self::NotReady) carries the fault TYPED, not erased to a `String`:
/// `error`/`evidence` are exactly [`FallibleDatasetView::Error`] /
/// [`FallibleDatasetView::Evidence`], the same typed pair
/// [`FallibleDatasetView::operation_status`] itself reports. A caller that already
/// handles the view's own typed error elsewhere can match on that SAME type here
/// instead of re-deriving it from rendered text, so the audit record stays
/// machine-readable end to end.
///
/// Deliberately exhaustive (NOT `#[non_exhaustive]`), matching [`CanonError`]: a
/// consumer that had to prepare for a hidden third variant could never write an
/// exhaustive match, and every refusal this module's flat-presentation entry points
/// can produce is named above.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewCanonError<E, Ev> {
    /// [`checkpointed_drain`] observed the view's backing data at fault — see
    /// [`DrainCheckpoint`] for which of its two checkpoints reports `checkpoint`.
    NotReady {
        /// Which checkpoint observed the fault.
        checkpoint: DrainCheckpoint,
        /// The view's own typed operational root cause.
        error: E,
        /// The view's own deterministic evidence at the failure boundary.
        evidence: Ev,
    },
    /// Canonicalization refused the input itself: reserved vocabulary, or n-degree
    /// budget exhaustion. Exactly [`try_canonicalize_view`]'s refusals.
    Refused(CanonError),
}

impl<E: std::fmt::Display, Ev: std::fmt::Debug> std::fmt::Display for ViewCanonError<E, Ev> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotReady {
                checkpoint,
                error,
                evidence,
            } => write!(
                f,
                "the view was not ready at the {checkpoint:?} checkpoint: {error} \
                 (evidence: {evidence:?})"
            ),
            Self::Refused(err) => err.fmt(f),
        }
    }
}

impl<E: std::error::Error + 'static, Ev: std::fmt::Debug> std::error::Error
    for ViewCanonError<E, Ev>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotReady { error, .. } => Some(error),
            Self::Refused(err) => Some(err),
        }
    }
}

impl<E, Ev> From<DrainFailure<E, Ev>> for ViewCanonError<E, Ev> {
    fn from(failure: DrainFailure<E, Ev>) -> Self {
        Self::NotReady {
            checkpoint: failure.checkpoint,
            error: failure.error,
            evidence: failure.evidence,
        }
    }
}

/// The reserved IRI reachable at `id`, recursing into triple terms and literal
/// datatypes, or `None`.
///
/// The datatype slot is swept even though the overlay never lowers a sentinel into
/// one: the rule a consumer audits is "no reserved IRI anywhere", and a rule with a
/// carve-out for the one position that happens to be safe today is a rule nobody can
/// check. Sweeping it costs a comparison on a term already resolved.
///
/// Recursion mirrors [`blanks_in_term`], which already walks the same nesting on the
/// same input: term ids are issued bottom-up so the structure is a DAG, and the depth
/// it can reach is the depth the parser admitted before this function ever ran.
fn reserved_in_term<D: DatasetView>(ds: &D, id: D::Id) -> Option<Box<str>> {
    match ds.resolve(id) {
        TermRef::Iri(iri) => iri.starts_with(RESERVED_NAMESPACE).then(|| Box::from(iri)),
        TermRef::Literal { datatype, .. } => reserved_in_term(ds, datatype),
        TermRef::Triple { s, p, o } => reserved_in_term(ds, s)
            .or_else(|| reserved_in_term(ds, p))
            .or_else(|| reserved_in_term(ds, o)),
        TermRef::Blank { .. } => None,
    }
}

/// The view's reserved-namespace violation within `scope`, or `None` if it is
/// admissible.
///
/// Returns the LEAST `(position, iri)` rather than the first one encountered. The
/// difference only shows on a dataset carrying several violations — which is already
/// refused either way — but "first encountered" would mean statement order, and
/// statement order is interning order, which differs between backends holding the
/// same dataset. The refusal was always total; this makes the DIAGNOSTIC total too,
/// so a corpus can pin the reported position and a consumer comparing two
/// implementations' rejections is comparing something well defined.
fn reserved_vocabulary<D: DatasetView>(
    ds: &D,
    scope: CanonScope<D::Id>,
    presentation: CanonPresentation,
) -> Option<ReservedVocabulary> {
    let mut worst: Option<ReservedVocabulary> = None;
    collect_components(ds, scope, presentation, &mut |comp| {
        let (s, p, o, g) = comp.slots();
        for (slot, position) in [
            (Some(s), TermPosition::Subject),
            (Some(p), TermPosition::Predicate),
            (Some(o), TermPosition::Object),
            (g, TermPosition::Graph),
        ] {
            // `Slot::Sentinel` is the overlay's OWN lowering — of a row this view
            // holds natively, or of one a base quad spelled and `collect_components`
            // folded back — and `AnnotationGraph(None)` carries no term at all;
            // neither is a violation. Every OTHER slot of a folded row still arrives
            // here as `Slot::Term`, so the fold cannot smuggle a reserved IRI past
            // this sweep by hiding it in a triple term or an annotation predicate.
            let Some(Slot::Term(id) | Slot::AnnotationGraph(Some(id))) = slot else {
                continue;
            };
            let Some(iri) = reserved_in_term(ds, id) else {
                continue;
            };
            let found = ReservedVocabulary { iri, position };
            if worst
                .as_ref()
                .is_none_or(|w| (found.position, &found.iri) < (w.position, &w.iri))
            {
                worst = Some(found);
            }
        }
    });
    worst
}

impl<'a, D: DatasetView> CanonState<'a, D> {
    fn new(
        ds: &'a D,
        scope: CanonScope<D::Id>,
        presentation: CanonPresentation,
        hash: CanonHash,
    ) -> Self {
        let mut blank_set: BTreeSet<D::Id> = BTreeSet::new();
        let mut incident: BTreeMap<D::Id, Vec<Component<D::Id>>> = BTreeMap::new();
        collect_components(ds, scope, presentation, &mut |comp| {
            // Record incidence for each distinct blank in the component (a blank that
            // appears in two positions of one quad still lists that quad once).
            let mut seen: BTreeSet<D::Id> = BTreeSet::new();
            comp.for_each_blank(ds, &mut |b| {
                blank_set.insert(b);
                if seen.insert(b) {
                    incident.entry(b).or_default().push(comp);
                }
            });
        });
        let blanks: Vec<D::Id> = blank_set.into_iter().collect();
        Self {
            ds,
            scope,
            presentation,
            blanks,
            incident,
            first_degree: BTreeMap::new(),
            canonical: IdIssuer::new(CANON_PREFIX),
            hash,
            budget: RDFC_CALL_LIMIT,
        }
    }

    /// Run the full algorithm, panicking on poison-budget exhaustion (trusted
    /// callers — [`canonicalize`]/[`canonicalize_with`]).
    fn run(self) -> Canonicalized<D::Id> {
        match self.run_fallible() {
            Ok(canonicalized) => canonicalized,
            Err(err) => panic!("{err}"),
        }
    }

    /// Run the full algorithm, returning [`CanonError`] instead of panicking
    /// (untrusted callers — [`try_canonicalize`]/[`try_canonicalize_with`]).
    /// Byte-identical `Ok` output to [`Self::run`].
    ///
    /// The reserved-vocabulary sweep runs FIRST, before any hashing. That ordering
    /// is deliberate: a dataset that is both inadmissible and pathologically
    /// symmetric must be refused for the reason that makes it dangerous, and it
    /// must be refused without spending the poison budget deciding so.
    fn run_fallible(self) -> Result<Canonicalized<D::Id>, CanonError> {
        let canonical = self.issue_labels()?;
        let nquads = canonical.serialize_canonical();
        let labels = canonical.canonical.issued;
        Ok(Canonicalized { nquads, labels })
    }

    /// Admit and issue labels once for both typed relabeling and canonical text.
    fn issue_labels(mut self) -> Result<Self, CanonError> {
        if let Some(violation) = reserved_vocabulary(self.ds, self.scope, self.presentation) {
            return Err(CanonError::ReservedVocabulary(violation));
        }
        let blank_count = self.blanks.len();
        match self.run_inner() {
            Ok(()) => {}
            Err(Exhausted) => {
                return Err(CanonError::BudgetExceeded(BudgetExceeded { blank_count }));
            }
        }
        Ok(self)
    }

    fn run_inner(&mut self) -> Result<(), Exhausted> {
        // §4.4 step 3: first-degree hash of every blank, grouped by hash.
        let mut by_hash: BTreeMap<HashHex, Vec<D::Id>> = BTreeMap::new();
        for &b in &self.blanks {
            let h = self.hash_first_degree(b);
            self.first_degree.insert(b, h);
            by_hash.entry(h).or_default().push(b);
        }

        // §4.4 step 4: issue canonical ids to uniquely-hashed blanks, ascending hash.
        // Defer hash-colliding groups to the n-degree pass.
        let mut ambiguous: Vec<HashHex> = Vec::new();
        for (h, group) in &by_hash {
            if group.len() == 1 {
                self.canonical.issue(group[0]);
            } else {
                ambiguous.push(*h);
            }
        }

        // §4.4 step 5: resolve each ambiguous group via the n-degree search.
        for h in ambiguous {
            // `by_hash` is a local, not part of `self`, so the group can be walked
            // by reference across the `&mut self` calls below (no `Vec` clone).
            let group = by_hash.get(&h).expect("ambiguous hash present");
            // 5.2–5.3: for each not-yet-canonical blank, run hashNDegreeQuads against a
            // fresh temporary issuer seeded with that blank.
            let mut hash_paths: Vec<(HashHex, IdIssuer<D::Id>)> = Vec::new();
            for &b in group {
                if self.canonical.has(b) {
                    continue;
                }
                let mut temp = IdIssuer::new(TEMP_PREFIX);
                temp.issue(b);
                let (result_hash, result_issuer) = self.hash_n_degree(b, temp)?;
                hash_paths.push((result_hash, result_issuer));
            }
            // 5.5: promote the temp issuers' bindings into the canonical issuer, the
            // groups taken in ascending result-hash order, each issuer in its own
            // issuance order.
            hash_paths.sort_by_key(|(h, _)| *h);
            for (_h, issuer) in hash_paths {
                for &b in issuer.order() {
                    self.canonical.issue(b);
                }
            }
        }
        Ok(())
    }

    /// Hash First Degree Quads (RDFC-1.0 §4.6) for blank `b`.
    fn hash_first_degree(&self, b: D::Id) -> HashHex {
        let render = BlankRender::FirstDegree { focus: b };
        let mut lines: Vec<String> = self
            .incident
            .get(&b)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .map(|comp| {
                let mut s = String::new();
                self.write_component(*comp, render, &mut s);
                s
            })
            .collect();
        lines.sort_unstable();
        hash_lines(self.hash, &lines)
    }

    /// Hash N-Degree Quads (RDFC-1.0 §4.8): the gossip-path permutation search.
    fn hash_n_degree(
        &mut self,
        identifier: D::Id,
        mut issuer: IdIssuer<D::Id>,
    ) -> Result<(HashHex, IdIssuer<D::Id>), Exhausted> {
        self.budget = self.budget.checked_sub(1).ok_or(Exhausted)?;

        // §4.8 step 3: map related-blank hash → the related blanks bearing it.
        let mut hn: BTreeMap<HashHex, Vec<D::Id>> = BTreeMap::new();
        let components = self.incident.get(&identifier).cloned().unwrap_or_default();
        for comp in &components {
            self.related_blanks(*comp, identifier, &issuer, &mut |related, related_hash| {
                hn.entry(related_hash).or_default().push(related);
            });
        }

        let mut data_to_hash = String::new();
        // §4.8 step 5: for each related hash, ascending.
        for (related_hash, related_list) in &hn {
            data_to_hash.push_str(related_hash.as_str());
            let mut chosen_path: Option<String> = None;
            let mut chosen_issuer: Option<IdIssuer<D::Id>> = None;

            // §4.8 step 5.4: every permutation of the related list, identity first.
            for perm in permutations(related_list) {
                // Charge the poison budget PER PERMUTATION: a related group of size k
                // contributes k! permutations, so this — not the recursive-call count —
                // is the dominant cost on a pathologically symmetric graph (e.g. a
                // 10-blank clique). Counting it here bounds the actual work.
                self.budget = self.budget.checked_sub(1).ok_or(Exhausted)?;
                let mut issuer_copy = issuer.clone();
                let mut path = String::new();
                let mut recursion: Vec<D::Id> = Vec::new();
                let mut pruned = false;

                // 5.4.4
                for related in &perm {
                    if let Some(id) = self.canonical.issued_for(*related) {
                        path.push_str("_:");
                        path.push_str(id);
                    } else {
                        if !issuer_copy.has(*related) {
                            recursion.push(*related);
                        }
                        path.push_str("_:");
                        path.push_str(issuer_copy.issue(*related));
                    }
                    // 5.4.4.3: prune if this partial path can no longer win.
                    if let Some(best) = &chosen_path
                        && path.len() >= best.len()
                        && path.as_str() > best.as_str()
                    {
                        pruned = true;
                        break;
                    }
                }
                if pruned {
                    continue;
                }

                // 5.4.5: recurse into newly-seen related blanks in path order.
                for related in &recursion {
                    let (rec_hash, rec_issuer) =
                        self.hash_n_degree(*related, issuer_copy.clone())?;
                    path.push_str("_:");
                    path.push_str(issuer_copy.issue(*related));
                    path.push('<');
                    path.push_str(rec_hash.as_str());
                    path.push('>');
                    issuer_copy = rec_issuer;
                    if let Some(best) = &chosen_path
                        && path.len() >= best.len()
                        && path.as_str() > best.as_str()
                    {
                        pruned = true;
                        break;
                    }
                }
                if pruned {
                    continue;
                }

                // 5.4.6: keep the lexicographically least path (first wins ties).
                if chosen_path
                    .as_ref()
                    .is_none_or(|best| path.as_str() < best.as_str())
                {
                    chosen_path = Some(path);
                    chosen_issuer = Some(issuer_copy);
                }
            }

            // 5.5–5.6: fold the winning path and adopt its issuer.
            data_to_hash.push_str(chosen_path.as_deref().unwrap_or(""));
            if let Some(winner) = chosen_issuer {
                issuer = winner;
            }
        }

        Ok((digest_hex(self.hash, data_to_hash.as_bytes()), issuer))
    }

    /// §4.8 step 3 + §4.7: for each related blank of `comp` (other than `focus`),
    /// invoke `f(related, hash_related_blank_node(related, …))`.
    fn related_blanks(
        &self,
        comp: Component<D::Id>,
        focus: D::Id,
        issuer: &IdIssuer<D::Id>,
        f: &mut impl FnMut(D::Id, HashHex),
    ) {
        let (s, p, o, g) = comp.slots();
        // Standard quad positions whose blanks are "related": subject, object, graph.
        // (Predicates are always IRIs / sentinels — never blank.) Blanks nested
        // inside a triple-term slot recurse with a position-path tag (RDF-1.2 ext).
        self.related_in_slot(s, "s", &p, focus, issuer, f);
        self.related_in_slot(o, "o", &p, focus, issuer, f);
        if let Some(g) = g {
            self.related_in_slot(g, "g", &p, focus, issuer, f);
        }
    }

    /// Walk a slot for related blanks, recursing triple terms with a position path.
    fn related_in_slot(
        &self,
        slot: Slot<D::Id>,
        position: &str,
        predicate: &Slot<D::Id>,
        focus: D::Id,
        issuer: &IdIssuer<D::Id>,
        f: &mut impl FnMut(D::Id, HashHex),
    ) {
        // The annotation-overlay graph marker carries a real graph term whose blanks
        // are "related" exactly like any graph-slot term.
        let id = match slot {
            Slot::Term(id) | Slot::AnnotationGraph(Some(id)) => id,
            Slot::Sentinel(_) | Slot::AnnotationGraph(None) => return,
        };
        match self.ds.resolve(id) {
            TermRef::Blank { .. } => {
                if id != focus {
                    let h = self.hash_related_blank_node(id, position, predicate, issuer);
                    f(id, h);
                }
            }
            TermRef::Triple { s, p, o } => {
                // Nested-triple blanks get a position path so role inside the quoted
                // triple is distinguished (RDF-1.2 extension; never hit by the W3C suite).
                self.related_in_slot(
                    Slot::Term(s),
                    &format!("{position}.s"),
                    predicate,
                    focus,
                    issuer,
                    f,
                );
                self.related_in_slot(
                    Slot::Term(p),
                    &format!("{position}.p"),
                    predicate,
                    focus,
                    issuer,
                    f,
                );
                self.related_in_slot(
                    Slot::Term(o),
                    &format!("{position}.o"),
                    predicate,
                    focus,
                    issuer,
                    f,
                );
            }
            TermRef::Literal {
                lexical, datatype, ..
            } => {
                // A blank embedded in a composite literal is related exactly as a
                // blank in that slot would be. The position path gets a `.cdt`
                // segment so the role "inside a composite value" is distinguished
                // from the slot's own term, and the segment is label-independent,
                // which is what canonicality requires.
                for related in composite_blanks(self.ds, lexical, datatype) {
                    if related != focus {
                        let h = self.hash_related_blank_node(
                            related,
                            &format!("{position}.cdt"),
                            predicate,
                            issuer,
                        );
                        f(related, h);
                    }
                }
            }
            TermRef::Iri(_) => {}
        }
    }

    /// Hash Related Blank Node (RDFC-1.0 §4.7).
    fn hash_related_blank_node(
        &self,
        related: D::Id,
        position: &str,
        predicate: &Slot<D::Id>,
        issuer: &IdIssuer<D::Id>,
    ) -> HashHex {
        let mut input = String::new();
        input.push_str(position);
        if position != "g" && !position.starts_with("g.") {
            input.push('<');
            input.push_str(self.predicate_iri(predicate));
            input.push('>');
        }
        if let Some(id) = self.canonical.issued_for(related) {
            input.push_str("_:");
            input.push_str(id);
        } else if let Some(id) = issuer.issued_for(related) {
            input.push_str("_:");
            input.push_str(id);
        } else {
            input.push_str(self.first_degree[&related].as_str());
        }
        digest_hex(self.hash, input.as_bytes())
    }

    /// The IRI value of a predicate slot (a real IRI term or a sentinel). Borrowed
    /// from the sentinel table or the dataset arena: this sits inside Hash Related
    /// Blank Node, the innermost loop of the n-degree search, so it mints nothing.
    fn predicate_iri(&self, predicate: &Slot<D::Id>) -> &str {
        match predicate {
            Slot::Sentinel(iri) => iri,
            Slot::Term(id) => match self.ds.resolve(*id) {
                TermRef::Iri(iri) => iri,
                other => unreachable!("predicate must be an IRI, got {other:?}"),
            },
            Slot::AnnotationGraph(_) => {
                unreachable!("the annotation-graph marker is never a predicate slot")
            }
        }
    }

    /// §4.4 step 7: serialize every component with canonical labels, sorted + deduped.
    fn serialize_canonical(&self) -> String {
        let render = BlankRender::Canonical {
            issuer: &self.canonical,
        };
        let mut lines: BTreeSet<String> = BTreeSet::new();
        collect_components(self.ds, self.scope, self.presentation, &mut |comp| {
            let mut s = String::new();
            self.write_component(comp, render, &mut s);
            lines.insert(s);
        });
        let mut out = String::new();
        for line in &lines {
            out.push_str(line);
        }
        out
    }

    /// Write one component as a canonical N-Quads line (`s p o [g] .\n`).
    fn write_component(
        &self,
        comp: Component<D::Id>,
        render: BlankRender<'_, D::Id>,
        out: &mut String,
    ) {
        let (s, p, o, g) = comp.slots();
        self.write_slot(s, render, out);
        out.push(' ');
        self.write_slot(p, render, out);
        out.push(' ');
        self.write_slot(o, render, out);
        if let Some(g) = g {
            out.push(' ');
            self.write_slot(g, render, out);
        }
        out.push_str(" .\n");
    }

    fn write_slot(&self, slot: Slot<D::Id>, render: BlankRender<'_, D::Id>, out: &mut String) {
        match slot {
            Slot::Sentinel(iri) => {
                out.push('<');
                write_iri_escaped(iri, out);
                out.push('>');
            }
            Slot::Term(id) => self.write_term(id, render, out),
            Slot::AnnotationGraph(g) => {
                // `None`: bare annotation sentinel — byte-identical to the pre-graph
                // form. `Some(g)`: sentinel then the graph term, so a named-graph
                // annotation stays lossless and never collides with a genuine quad
                // (which never carries two graph tokens). Not re-parsed — this string
                // is only hashed / byte-compared as the canonical oracle.
                out.push('<');
                write_iri_escaped(SENTINEL_ANNOTATION_GRAPH, out);
                out.push('>');
                if let Some(g) = g {
                    out.push(' ');
                    self.write_term(g, render, out);
                }
            }
        }
    }

    /// Write a term in canonical N-Quads form. Literal lexical forms / datatypes /
    /// language / direction are emitted **verbatim** (never normalized), with one
    /// exception that is not a normalization: a composite (`cdt:List` /
    /// `cdt:Map`) literal's embedded `BLANK_NODE_LABEL` tokens are rendered
    /// through `render`, exactly as the same blank node written as a term is.
    ///
    /// Without that the canonical form would still carry the INPUT labels of
    /// blank nodes that happen to live inside a literal, so two isomorphic
    /// datasets would serialize differently and the oracle would report a false
    /// negative. Every other byte of the lexical form is untouched.
    fn write_term(&self, id: D::Id, render: BlankRender<'_, D::Id>, out: &mut String) {
        match self.ds.resolve(id) {
            TermRef::Iri(iri) => {
                out.push('<');
                write_iri_escaped(iri, out);
                out.push('>');
            }
            TermRef::Blank { .. } => {
                out.push_str("_:");
                out.push_str(render.label(id));
            }
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let rendered = self.render_composite_lexical(lexical, datatype, render);
                out.push('"');
                write_literal_escaped(&rendered, out);
                out.push('"');
                if let Some(lang) = language {
                    out.push('@');
                    out.push_str(lang);
                    if let Some(dir) = direction {
                        out.push_str("--");
                        out.push_str(dir.as_str());
                    }
                } else {
                    let dt = match self.ds.resolve(datatype) {
                        TermRef::Iri(iri) => iri,
                        other => unreachable!("literal datatype must be an IRI, got {other:?}"),
                    };
                    if dt != XSD_STRING {
                        out.push_str("^^<");
                        write_iri_escaped(dt, out);
                        out.push('>');
                    }
                }
            }
            TermRef::Triple { s, p, o } => {
                // RDF-1.2 triple term: `<<( <s> <p> <o> )>>` (the form oxigraph/Jena parse).
                out.push_str("<<( ");
                self.write_term(s, render, out);
                out.push(' ');
                self.write_term(p, render, out);
                out.push(' ');
                self.write_term(o, render, out);
                out.push_str(" )>>");
            }
        }
    }

    /// A composite literal's lexical form with every embedded blank label
    /// replaced by the label `render` issues for that node; any other lexical
    /// form, borrowed unchanged.
    fn render_composite_lexical<'l>(
        &self,
        lexical: &'l str,
        datatype: D::Id,
        render: BlankRender<'_, D::Id>,
    ) -> std::borrow::Cow<'l, str> {
        let TermRef::Iri(iri) = self.ds.resolve(datatype) else {
            return std::borrow::Cow::Borrowed(lexical);
        };
        crate::cdt_blank::rewrite_cdt_blank_terms(lexical, iri, &mut |label| {
            let (label, scope) = crate::blank_label::decode_blank_label(
                label,
                crate::blank_label::LabelAlphabet::BlankNodeLabel,
            );
            let id = self.ds.term_id_by_value(&TermValue::Blank {
                label: label.into_owned(),
                scope,
            })?;
            Some(format!("_:{}", render.label(id)))
        })
    }
}

/// A **lazy** generator of every permutation of a slice (identity first, then
/// lexicographic position order). Lazy generation matters for the poison case: a
/// 9-element related group has 9! = 362 880 permutations, so collecting them all
/// upfront would allocate a factorial-sized `Vec<Vec<_>>` per n-degree call. Yielding
/// one small `Vec` at a time keeps the call-budget guard the only bound on cost.
struct Permutations<T> {
    items: Vec<T>,
    idx: Vec<usize>,
    first: bool,
    done: bool,
}

impl<T: Copy> Iterator for Permutations<T> {
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Vec<T>> {
        if self.done {
            return None;
        }
        if self.first {
            self.first = false;
        } else if !next_permutation(&mut self.idx) {
            self.done = true;
            return None;
        }
        Some(self.idx.iter().map(|&i| self.items[i]).collect())
    }
}

/// Lazily generate every permutation of `items` (identity first; see [`Permutations`]).
fn permutations<T: Copy>(items: &[T]) -> Permutations<T> {
    Permutations {
        items: items.to_vec(),
        idx: (0..items.len()).collect(),
        first: true,
        // An empty slice still yields exactly one (empty) permutation.
        done: false,
    }
}

/// In-place next lexicographic permutation of `a`; `false` if `a` was the last.
fn next_permutation(a: &mut [usize]) -> bool {
    let n = a.len();
    if n < 2 {
        return false;
    }
    let mut i = n - 1;
    while i > 0 && a[i - 1] >= a[i] {
        i -= 1;
    }
    if i == 0 {
        return false;
    }
    let mut j = n - 1;
    while a[j] <= a[i - 1] {
        j -= 1;
    }
    a.swap(i - 1, j);
    a[i..].reverse();
    true
}

/// Escape an IRI for `<…>` N-Quads form: control chars (C0, the space character, DEL,
/// and the C1 block `0x80-0x9F`) and the reserved delimiter set become `\uXXXX`
/// (canonical N-Triples IRIREF rules). Clean ASCII IRIs pass through unchanged.
fn write_iri_escaped(iri: &str, out: &mut String) {
    for ch in iri.chars() {
        match ch {
            c if c.is_control() || c == ' ' => write_u_escape(c, out),
            '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\' => {
                write_u_escape(ch, out);
            }
            _ => out.push(ch),
        }
    }
}

/// Escape a literal lexical form for a `"…"` N-Quads string, matching the canonical
/// N-Triples ECHAR set; other C0 control characters become `\uXXXX`.
fn write_literal_escaped(value: &str, out: &mut String) {
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            // Canonical N-Quads escapes C0 controls and U+007F (DEL) as \uXXXX; every
            // other character (incl. all non-ASCII, including the C1 block) is emitted
            // verbatim as UTF-8 — the W3C RDFC-1.0 test suite fixtures (e.g. test060)
            // pin the C1 block passing through raw in literals, unlike IRIs where the
            // IRIREF grammar forbids the full control range.
            c if (c as u32) < 0x20 || c as u32 == 0x7f => write_u_escape(c, out),
            c => out.push(c),
        }
    }
}

/// Write `\uXXXX` (or `\UXXXXXXXX` beyond the BMP) for `ch`.
fn write_u_escape(ch: char, out: &mut String) {
    let cp = ch as u32;
    if cp <= 0xFFFF {
        let _ = write!(out, "\\u{cp:04X}");
    } else {
        let _ = write!(out, "\\U{cp:08X}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RdfStoreCapabilities;
    use crate::dataset_view::ViewOperationStatus;
    use crate::ir::RdfDatasetBuilder;
    use crate::ir::dataset::QuadRef;
    use crate::{RdfLiteral, RdfTextDirection};
    use std::sync::Arc;

    fn iri(b: &mut RdfDatasetBuilder, n: &str) -> TermId {
        b.intern_iri(&format!("http://example.org/{n}"))
    }

    fn canon(ds: &RdfDataset) -> String {
        canonicalize(ds).nquads
    }

    #[test]
    fn all_ground_fast_path_sorts_quads() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o1, o2) = (
            iri(&mut b, "s"),
            iri(&mut b, "p"),
            iri(&mut b, "o1"),
            iri(&mut b, "o2"),
        );
        b.push_quad(s, p, o2, None);
        b.push_quad(s, p, o1, None);
        let ds = b.freeze().expect("valid");
        assert_eq!(
            canon(&ds),
            "<http://example.org/s> <http://example.org/p> <http://example.org/o1> .\n\
             <http://example.org/s> <http://example.org/p> <http://example.org/o2> .\n"
        );
        assert!(canonicalize(&ds).labels.is_empty(), "no blanks → no labels");
    }

    // -----------------------------------------------------------------------
    // Reserved vocabulary: the overlay's sentinels cannot be forged from input,
    // and the overlay's OWN lowered shapes fold back instead of being refused
    // -----------------------------------------------------------------------

    /// The shape the refusal rule used to stop, and what it means now.
    ///
    /// Dataset A carries a genuine reifier, which the overlay lowers to a row spelled
    /// `r <urn:purrdf:rdfc:reifies> <<(…)>>`. Dataset B carries no reifier at all — it
    /// simply ASSERTS that row as an ordinary quad. That is not two structures sharing
    /// one digest, it is one structure written two ways: B's quad denotes exactly the
    /// binding A declares, so the two carry the same content and sharing a digest is
    /// the lossless overlay working, not a collision. Canonicalization folds B's quad
    /// back into the statement layer and the pair co-canonicalizes, byte for byte.
    ///
    /// The assertion is deliberately two-sided. It is not enough that B is admitted:
    /// the test also confirms A's bytes still travel through the sentinel, so it fails
    /// if the lowering is ever changed in a way that makes the fixture stop exercising
    /// the fold at all — a test that passed because it stopped testing anything would
    /// be worse than no test. What the refusal rule still stops — a quad that is NOT
    /// in the emitted shape — is pinned by the twins below.
    #[test]
    fn a_literally_asserted_sentinel_row_folds_into_the_reifier_it_denotes() {
        // A: a genuine reifier.
        let mut b = RdfDatasetBuilder::new();
        let (s, pred, o, r) = (
            iri(&mut b, "s"),
            iri(&mut b, "p"),
            iri(&mut b, "o"),
            iri(&mut b, "r"),
        );
        let triple = b.intern_triple(s, pred, o);
        b.push_reifier(r, triple);
        let genuine = b.freeze().expect("valid");
        let lowered = canon(&genuine);
        assert!(
            lowered.contains("<urn:purrdf:rdfc:reifies>"),
            "the fixture must actually exercise the lowering: {lowered}"
        );

        // B: no reifier — the lowered row asserted literally as an ordinary quad.
        let mut b = RdfDatasetBuilder::new();
        let (s, o, r) = (iri(&mut b, "s"), iri(&mut b, "o"), iri(&mut b, "r"));
        let pred = iri(&mut b, "p");
        let sentinel = b.intern_iri(SENTINEL_REIFIES);
        let triple = b.intern_triple(s, pred, o);
        b.push_quad(r, sentinel, triple, None);
        let spelled = b.freeze().expect("valid");
        assert_eq!(
            spelled.reifier_quads().count(),
            0,
            "B must carry no statement layer of its own — the quad IS the input"
        );

        let folded = try_canonicalize(&spelled).expect("the emitted shape must fold, not refuse");
        assert_eq!(
            folded.nquads, lowered,
            "the spelled row and the row it spells must canonicalize to the same bytes"
        );

        // C: the same dataset with the row spelled BOTH ways. One row, held twice, is
        // still one row — so the bytes may not move, and in particular the duplicate
        // may not be counted twice.
        let mut b = RdfDatasetBuilder::new();
        let (s, pred, o, r) = (
            iri(&mut b, "s"),
            iri(&mut b, "p"),
            iri(&mut b, "o"),
            iri(&mut b, "r"),
        );
        let triple = b.intern_triple(s, pred, o);
        let sentinel = b.intern_iri(SENTINEL_REIFIES);
        b.push_reifier(r, triple);
        b.push_quad(r, sentinel, triple, None);
        let both = b.freeze().expect("valid");
        assert_eq!(
            try_canonicalize(&both)
                .expect("a row spelled both ways must fold, not refuse")
                .nquads,
            lowered,
            "spelling one row twice must not change the canonical form"
        );
    }

    /// The rule is over the NAMESPACE, not over the two sentinel spellings. An IRI
    /// nobody has minted yet is refused just the same, so growing the overlay cannot
    /// silently reopen the hole.
    #[test]
    fn any_iri_in_the_reserved_namespace_is_refused_not_only_the_two_sentinels() {
        let mut b = RdfDatasetBuilder::new();
        let (s, o) = (iri(&mut b, "s"), iri(&mut b, "o"));
        let unminted = b.intern_iri("urn:purrdf:rdfc:no-such-sentinel-exists-yet");
        b.push_quad(s, unminted, o, None);
        let ds = b.freeze().expect("valid");
        assert!(
            matches!(
                try_canonicalize(&ds),
                Err(CanonError::ReservedVocabulary(_))
            ),
            "an unminted name in the reserved namespace must still be refused"
        );
    }

    /// Every position, including the two an attacker reaches only through nesting:
    /// inside a triple term, and inside a literal's datatype slot.
    #[test]
    fn the_reserved_namespace_is_refused_in_every_position() {
        let sentinel = SENTINEL_ANNOTATION_GRAPH;

        // Subject / predicate / object / graph, each in turn.
        for position in [
            TermPosition::Subject,
            TermPosition::Predicate,
            TermPosition::Object,
            TermPosition::Graph,
        ] {
            // A LONE annotation sentinel in the graph slot is the overlay's own
            // default-graph annotation row and folds back (see the fold tests), so
            // probing the position rule with it THERE would be probing the fold
            // instead. Every position is still probed — this one with a reserved name
            // the overlay does not lower into, which is the position rule itself.
            let probe = if position == TermPosition::Graph {
                "urn:purrdf:rdfc:not-a-sentinel"
            } else {
                sentinel
            };
            let mut b = RdfDatasetBuilder::new();
            let (s, pred, o, g) = (
                iri(&mut b, "s"),
                iri(&mut b, "p"),
                iri(&mut b, "o"),
                iri(&mut b, "g"),
            );
            let bad = b.intern_iri(probe);
            match position {
                TermPosition::Subject => b.push_quad(bad, pred, o, None),
                TermPosition::Predicate => b.push_quad(s, bad, o, None),
                TermPosition::Object => b.push_quad(s, pred, bad, None),
                TermPosition::Graph => b.push_quad(s, pred, o, Some(bad)),
            }
            let _ = g;
            let ds = b.freeze().expect("valid");
            match try_canonicalize(&ds) {
                Err(CanonError::ReservedVocabulary(err)) => {
                    assert_eq!(err.position, position, "position must be reported exactly");
                    assert_eq!(&*err.iri, probe);
                }
                other => panic!("{position:?} must be refused; got {other:?}"),
            }
        }

        // Nested inside a triple term: reported at the slot the triple term occupies.
        let mut b = RdfDatasetBuilder::new();
        let (s, pred, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        let bad = b.intern_iri(sentinel);
        let quoted = b.intern_triple(s, bad, o);
        b.push_quad(s, pred, quoted, None);
        let ds = b.freeze().expect("valid");
        match try_canonicalize(&ds) {
            Err(CanonError::ReservedVocabulary(err)) => {
                assert_eq!(err.position, TermPosition::Object);
                assert_eq!(&*err.iri, sentinel);
            }
            other => panic!("a nested reserved IRI must be refused; got {other:?}"),
        }

        // Inside a literal's datatype. The overlay never lowers a sentinel here, so
        // this position is safe today — it is swept anyway, because a rule with a
        // carve-out for whichever position happens to be harmless is one no consumer
        // can audit, and tomorrow's overlay may not leave it harmless.
        let mut b = RdfDatasetBuilder::new();
        let (s, pred) = (iri(&mut b, "s"), iri(&mut b, "p"));
        let lit = b.intern_literal(RdfLiteral::typed("5", sentinel));
        b.push_quad(s, pred, lit, None);
        let ds = b.freeze().expect("valid");
        assert!(
            matches!(
                try_canonicalize(&ds),
                Err(CanonError::ReservedVocabulary(_))
            ),
            "a reserved IRI in a datatype slot must be refused"
        );
    }

    /// Which violation is NAMED must not depend on statement order, because statement
    /// order is interning order and two backends holding the same dataset need not
    /// agree on it. The refusal was always total; this pins the diagnostic.
    #[test]
    fn the_reported_violation_is_the_least_one_not_the_first_encountered() {
        let build = |reverse: bool| {
            let mut b = RdfDatasetBuilder::new();
            let (s, pred, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let bad_subject = b.intern_iri("urn:purrdf:rdfc:zzz");
            let bad_object = b.intern_iri("urn:purrdf:rdfc:aaa");
            let rows: [(TermId, TermId, TermId); 2] =
                [(bad_subject, pred, o), (s, pred, bad_object)];
            let order: [usize; 2] = if reverse { [1, 0] } else { [0, 1] };
            for i in order {
                let (a, c, d) = rows[i];
                b.push_quad(a, c, d, None);
            }
            b.freeze().expect("valid")
        };

        let forward = try_canonicalize(&build(false)).expect_err("refused");
        let reversed = try_canonicalize(&build(true)).expect_err("refused");
        assert_eq!(
            forward, reversed,
            "the named violation must not depend on statement order"
        );
        match forward {
            // Subject sorts before Object, so the subject occurrence wins even though
            // its IRI ("zzz") sorts after the object's ("aaa") — position is the
            // primary key, which is what makes the answer independent of both orders.
            CanonError::ReservedVocabulary(err) => {
                assert_eq!(err.position, TermPosition::Subject);
                assert_eq!(&*err.iri, "urn:purrdf:rdfc:zzz");
            }
            other => panic!("expected a reserved-vocabulary refusal; got {other:?}"),
        }
    }

    /// The sweep runs BEFORE the poison budget, so a dataset that is both inadmissible
    /// and pathologically symmetric is refused for the reason that makes it dangerous
    /// — and refused without spending the budget to find out.
    #[test]
    fn reserved_vocabulary_is_reported_ahead_of_the_poison_budget() {
        let mut b = RdfDatasetBuilder::new();
        let pred = iri(&mut b, "p");
        let bad = b.intern_iri(SENTINEL_REIFIES);
        // A wide symmetric blank ring: every blank has identical first-degree
        // structure, which is what drives the n-degree search.
        let blanks: Vec<TermId> = (0..24)
            .map(|i| b.intern_blank(&format!("b{i}"), BlankScope(0)))
            .collect();
        for w in blanks.windows(2) {
            b.push_quad(w[0], pred, w[1], None);
        }
        b.push_quad(blanks[blanks.len() - 1], pred, blanks[0], None);
        b.push_quad(blanks[0], bad, blanks[1], None);
        let ds = b.freeze().expect("valid");
        assert!(
            matches!(
                try_canonicalize(&ds),
                Err(CanonError::ReservedVocabulary(_))
            ),
            "the inadmissibility must be reported, not masked by budget exhaustion"
        );
    }

    /// `check_admissible` is the same predicate the canonicalizer applies, exposed for
    /// screening at write time. If the two could disagree, screening would be theatre.
    #[test]
    fn check_admissible_agrees_with_the_canonicalizer_on_both_answers() {
        let mut b = RdfDatasetBuilder::new();
        let (s, pred, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        b.push_quad(s, pred, o, None);
        let clean = b.freeze().expect("valid");
        assert!(check_admissible(&clean).is_ok());
        assert!(try_canonicalize(&clean).is_ok());

        let mut b = RdfDatasetBuilder::new();
        let (s, o) = (iri(&mut b, "s"), iri(&mut b, "o"));
        let bad = b.intern_iri(SENTINEL_REIFIES);
        b.push_quad(s, bad, o, None);
        let dirty = b.freeze().expect("valid");
        let screened = check_admissible(&dirty).expect_err("inadmissible");
        match try_canonicalize(&dirty) {
            Err(CanonError::ReservedVocabulary(err)) => assert_eq!(
                err, screened,
                "screening and canonicalization must name the same violation"
            ),
            other => panic!("expected a refusal; got {other:?}"),
        }
    }

    /// The trusted entry point hard-fails on the same input the fallible one refuses.
    /// Its contract is "trusted callers only"; a caller who cannot vouch for the bytes
    /// is meant to be using `try_canonicalize`, and this is what makes that real
    /// rather than advisory.
    #[test]
    #[should_panic(expected = "reserved IRI")]
    fn the_trusted_entry_point_panics_on_reserved_vocabulary() {
        let mut b = RdfDatasetBuilder::new();
        let (s, o) = (iri(&mut b, "s"), iri(&mut b, "o"));
        let bad = b.intern_iri(SENTINEL_REIFIES);
        b.push_quad(s, bad, o, None);
        let ds = b.freeze().expect("valid");
        let _ = canonicalize(&ds);
    }

    /// The overlay's OWN lowering is not input and must not trip the sweep — otherwise
    /// the rule would refuse every dataset carrying a reifier, which is most of the
    /// reason this module exists.
    #[test]
    fn the_overlays_own_sentinels_do_not_trip_the_sweep() {
        let mut b = RdfDatasetBuilder::new();
        let (s, pred, o, r) = (
            iri(&mut b, "s"),
            iri(&mut b, "p"),
            iri(&mut b, "o"),
            iri(&mut b, "r"),
        );
        let triple = b.intern_triple(s, pred, o);
        b.push_reifier(r, triple);
        b.push_annotation(r, pred, o);
        let ds = b.freeze().expect("valid");
        let out = try_canonicalize(&ds).expect("a genuine overlay must canonicalize");
        assert!(out.nquads.contains("<urn:purrdf:rdfc:reifies>"));
        assert!(out.nquads.contains("<urn:purrdf:rdfc:annotation>"));
    }

    // -----------------------------------------------------------------------
    // Idempotence: canonicalization admits its own output and reproduces it
    // -----------------------------------------------------------------------

    /// Which way a fixture spells its RDF 1.2 statement layer.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Spelling {
        /// The reifier and annotation rows pushed into the side tables — the dataset
        /// a producer builds.
        Native,
        /// Each of those rows pushed as the plain quad its canonical line IS — the
        /// dataset a reader of the canonical document builds.
        Lowered,
    }

    /// A fixture carrying reifiers, an annotation and scoped blanks, in either
    /// spelling.
    ///
    /// [`Spelling::Lowered`] STANDS IN FOR PARSING the canonical document back:
    /// `purrdf-core` holds no N-Quads parser (the parsers live above this crate, so
    /// reaching for one here would invert the dependency), and the tests below tie
    /// the stand-in to the actual bytes rather than asserting it on trust — the
    /// lowered form carries no statement layer at all, holds exactly one quad per
    /// canonical line, and its terms are the ones those lines name. That is what a
    /// faithful parse of those bytes produces, up to blank-node labels — and
    /// canonicalization is invariant under those by construction, which
    /// `isomorphic_blank_relabeling_is_byte_equal` pins.
    ///
    /// Both reifier subject shapes the lowering can emit are covered: a blank node
    /// (default graph) and an IRI (named graph).
    fn statement_layer_fixture(spelling: Spelling) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let (p, q, o) = (iri(&mut b, "p"), iri(&mut b, "q"), iri(&mut b, "o"));
        let g = iri(&mut b, "g");
        let shared = b.intern_blank("n", BlankScope::DEFAULT);
        let scoped = b.intern_blank("n", BlankScope(4));
        b.push_quad(shared, p, o, None);
        b.push_quad(scoped, p, o, Some(g));
        let triple = b.intern_triple(shared, p, o);
        let blank_reifier = b.intern_blank("r", BlankScope(7));
        let iri_reifier = iri(&mut b, "nr");
        match spelling {
            Spelling::Native => {
                b.push_reifier_in_graph(blank_reifier, triple, None);
                b.push_reifier_in_graph(iri_reifier, triple, Some(g));
                b.push_annotation_in_graph(blank_reifier, q, o, None);
            }
            Spelling::Lowered => {
                let reifies = b.intern_iri(SENTINEL_REIFIES);
                let annotation = b.intern_iri(SENTINEL_ANNOTATION_GRAPH);
                b.push_quad(blank_reifier, reifies, triple, None);
                b.push_quad(iri_reifier, reifies, triple, Some(g));
                b.push_quad(blank_reifier, q, o, Some(annotation));
            }
        }
        b.freeze().expect("valid")
    }

    /// `canon(canon(g)) == canon(g)`, bytes and digest alike — the property whose
    /// absence meant the canonicalizer could not read back the document it had just
    /// written, and so could not re-derive an identity it had just minted.
    ///
    /// The second pass runs over the LOWERED spelling, which is the dataset those
    /// bytes parse to; the assertions below it pin that correspondence to the bytes
    /// themselves rather than to the fixture's good intentions.
    #[test]
    fn canonicalization_is_idempotent_over_its_own_output() {
        let native = statement_layer_fixture(Spelling::Native);
        let reread = statement_layer_fixture(Spelling::Lowered);

        let first = canonicalize(&native);
        assert!(
            first.nquads.contains("<urn:purrdf:rdfc:reifies>")
                && first.nquads.contains("<urn:purrdf:rdfc:annotation>")
                && first.nquads.contains("_:c14n"),
            "the fixture must reach both sentinels and the labeler: {}",
            first.nquads
        );

        // The stand-in really is the document: no statement layer of its own, and one
        // quad for every canonical line.
        assert_eq!(reread.reifier_quads().count(), 0);
        assert_eq!(reread.annotation_quads().count(), 0);
        assert_eq!(
            reread.quads().count(),
            first.nquads.lines().count(),
            "the re-read dataset must hold exactly the canonical document's rows"
        );

        let second = try_canonicalize(&reread).expect("the canonical document must be admissible");
        assert_eq!(
            second.nquads, first.nquads,
            "canonicalizing the canonical document must reproduce it byte for byte"
        );
        assert_eq!(
            ContentDigest::of(second.nquads.as_bytes()),
            ContentDigest::of(first.nquads.as_bytes()),
            "equal bytes must mint equal identity"
        );

        // And a THIRD pass changes nothing either — idempotence, not a one-off.
        assert_eq!(canonicalize(&reread).nquads, second.nquads);
    }

    /// The same property through the graph-scoped entry point.
    ///
    /// A per-graph canonical document is graph-ERASING, so its rows come back in the
    /// default graph — including the annotation row, whose named-graph form is the one
    /// shape the lowering writes as a five-token line no quad can hold. Re-reading that
    /// document and canonicalizing the whole of it must reproduce the per-graph bytes,
    /// which is what a carrier that pins a per-graph digest re-derives.
    #[test]
    fn graph_scoped_canonicalization_is_idempotent_over_its_own_output() {
        let mut b = RdfDatasetBuilder::new();
        let (p, q, o) = (iri(&mut b, "p"), iri(&mut b, "q"), iri(&mut b, "o"));
        let g = iri(&mut b, "g");
        let member = b.intern_blank("m", BlankScope(2));
        b.push_quad(member, p, o, Some(g));
        let triple = b.intern_triple(member, p, o);
        let reifier = iri(&mut b, "r");
        b.push_reifier_in_graph(reifier, triple, Some(g));
        b.push_annotation_in_graph(reifier, q, o, Some(g));
        // A neighbouring default-graph row that the scope must not admit.
        b.push_quad(o, p, o, None);
        let native = b.freeze().expect("valid");

        let scoped = canonicalize_graph_view(&*native, "http://example.org/g", CanonHash::Sha256);
        assert!(
            scoped.nquads.contains("<urn:purrdf:rdfc:reifies>")
                && scoped.nquads.contains("<urn:purrdf:rdfc:annotation> ."),
            "the per-graph document must carry both lowered rows, graph-erased: {}",
            scoped.nquads
        );

        // The document, read back: every row in the default graph, the statement
        // layer spelled as the plain quads those lines are.
        let mut b = RdfDatasetBuilder::new();
        let (p, q, o) = (iri(&mut b, "p"), iri(&mut b, "q"), iri(&mut b, "o"));
        let member = b.intern_blank("m", BlankScope::DEFAULT);
        let triple = b.intern_triple(member, p, o);
        let reifier = iri(&mut b, "r");
        let reifies = b.intern_iri(SENTINEL_REIFIES);
        let annotation = b.intern_iri(SENTINEL_ANNOTATION_GRAPH);
        b.push_quad(member, p, o, None);
        b.push_quad(reifier, reifies, triple, None);
        b.push_quad(reifier, q, o, Some(annotation));
        let reread = b.freeze().expect("valid");
        assert_eq!(
            reread.quads().count(),
            scoped.nquads.lines().count(),
            "the re-read dataset must hold exactly the per-graph document's rows"
        );

        assert_eq!(
            try_canonicalize(&reread)
                .expect("a per-graph canonical document must be admissible")
                .nquads,
            scoped.nquads,
            "re-canonicalizing a per-graph document must reproduce it byte for byte"
        );
        assert_eq!(
            graph_digest_view(&*native, "http://example.org/g"),
            ContentDigest::of(
                try_canonicalize(&reread)
                    .expect("admissible")
                    .nquads
                    .as_bytes()
            ),
            "the per-graph digest must be re-derivable from the document it covers"
        );
    }

    /// The soundness statement of the fold: a genuine statement-layer row and the
    /// plain quad that spells it are the same content, so they canonicalize to the
    /// same bytes — reifiers and annotations alike, and through every entry point.
    #[test]
    fn a_spelled_row_and_the_row_it_spells_canonicalize_identically() {
        let native = statement_layer_fixture(Spelling::Native);
        let spelled = statement_layer_fixture(Spelling::Lowered);

        assert_eq!(canonicalize(&native).nquads, canonicalize(&spelled).nquads);
        assert_eq!(
            canonicalize_with(&native, CanonHash::Sha384).nquads,
            canonicalize_with(&spelled, CanonHash::Sha384).nquads,
            "the fold is in the ingestion seam, so it is hash-algorithm-independent"
        );
        assert_eq!(
            canonicalize_view(&*native, CanonHash::Sha256).nquads,
            canonicalize_view(&*spelled, CanonHash::Sha256).nquads,
            "the view-generic entry point folds too — one seam, not two"
        );
        assert_eq!(
            blank_count(&native),
            blank_count(&spelled),
            "the blank sweep reads the folded rows, so it agrees across spellings"
        );
        assert!(
            check_admissible(&spelled).is_ok(),
            "screening must admit exactly what canonicalization admits"
        );
        assert!(
            datasets_are_byte_equal(&native, &spelled),
            "the two spellings must be one identity"
        );
    }

    /// Whether two datasets mint the same identity under this profile.
    fn datasets_are_byte_equal(a: &RdfDataset, b: &RdfDataset) -> bool {
        ContentDigest::of(canonicalize(a).nquads.as_bytes())
            == ContentDigest::of(canonicalize(b).nquads.as_bytes())
    }

    /// The fold is SHAPE-EXACT, and these are its near misses: each differs from an
    /// emitted shape in exactly one respect, and each must still be refused.
    ///
    /// This is the half of the rule that stops forgery, so it is asserted against the
    /// neighbours rather than against a distant counterexample: a fold that were even
    /// slightly wider would admit one of these, and admitting one of these WOULD be a
    /// collision — none of them denotes the row it resembles.
    #[test]
    fn the_fold_is_shape_exact_and_its_near_misses_still_refuse() {
        // The sentinel predicate over an object that is NOT a triple term: it denotes
        // no reifier binding, because a reifier binds a triple term and nothing else.
        let mut b = RdfDatasetBuilder::new();
        let (r, o) = (iri(&mut b, "r"), iri(&mut b, "o"));
        let reifies = b.intern_iri(SENTINEL_REIFIES);
        b.push_quad(r, reifies, o, None);
        let ds = b.freeze().expect("valid");
        match try_canonicalize(&ds) {
            Err(CanonError::ReservedVocabulary(err)) => {
                assert_eq!(&*err.iri, SENTINEL_REIFIES);
                assert_eq!(err.position, TermPosition::Predicate);
            }
            other => panic!("a non-triple object must still be refused; got {other:?}"),
        }

        // A different name in the reserved namespace as the predicate: the rule is
        // over the namespace, and only the two lowered shapes are folded.
        let mut b = RdfDatasetBuilder::new();
        let (r, p, o) = (iri(&mut b, "r"), iri(&mut b, "p"), iri(&mut b, "o"));
        let unminted = b.intern_iri("urn:purrdf:rdfc:reifies-ish");
        let triple = b.intern_triple(r, p, o);
        b.push_quad(r, unminted, triple, None);
        let ds = b.freeze().expect("valid");
        assert!(
            matches!(
                try_canonicalize(&ds),
                Err(CanonError::ReservedVocabulary(_))
            ),
            "only the sentinel spelling folds, not a neighbour in the namespace"
        );

        // The sentinel in OBJECT position: no lowered row ever puts it there.
        let mut b = RdfDatasetBuilder::new();
        let (r, p) = (iri(&mut b, "r"), iri(&mut b, "p"));
        let reifies = b.intern_iri(SENTINEL_REIFIES);
        b.push_quad(r, p, reifies, None);
        let ds = b.freeze().expect("valid");
        match try_canonicalize(&ds) {
            Err(CanonError::ReservedVocabulary(err)) => {
                assert_eq!(err.position, TermPosition::Object);
            }
            other => panic!("the sentinel in object position must refuse; got {other:?}"),
        }

        // The annotation sentinel as a PREDICATE rather than as a lone graph slot.
        let mut b = RdfDatasetBuilder::new();
        let (r, o) = (iri(&mut b, "r"), iri(&mut b, "o"));
        let annotation = b.intern_iri(SENTINEL_ANNOTATION_GRAPH);
        b.push_quad(r, annotation, o, None);
        let ds = b.freeze().expect("valid");
        assert!(
            matches!(
                try_canonicalize(&ds),
                Err(CanonError::ReservedVocabulary(_))
            ),
            "the annotation sentinel is a graph marker, never a predicate"
        );

        // A folded row may not smuggle a reserved IRI past the sweep in the slots the
        // fold does NOT consume: the triple term is still swept.
        let mut b = RdfDatasetBuilder::new();
        let (r, p) = (iri(&mut b, "r"), iri(&mut b, "p"));
        let reifies = b.intern_iri(SENTINEL_REIFIES);
        let hidden = b.intern_iri("urn:purrdf:rdfc:hidden");
        let triple = b.intern_triple(r, p, hidden);
        b.push_quad(r, reifies, triple, None);
        let ds = b.freeze().expect("valid");
        match try_canonicalize(&ds) {
            Err(CanonError::ReservedVocabulary(err)) => {
                assert_eq!(&*err.iri, "urn:purrdf:rdfc:hidden");
                assert_eq!(err.position, TermPosition::Object);
            }
            other => panic!("a reserved IRI inside a folded row must refuse; got {other:?}"),
        }

        // Same for the annotation fold's predicate and object slots.
        let mut b = RdfDatasetBuilder::new();
        let r = iri(&mut b, "r");
        let annotation = b.intern_iri(SENTINEL_ANNOTATION_GRAPH);
        let hidden = b.intern_iri("urn:purrdf:rdfc:hidden");
        b.push_quad(r, hidden, r, Some(annotation));
        let ds = b.freeze().expect("valid");
        match try_canonicalize(&ds) {
            Err(CanonError::ReservedVocabulary(err)) => {
                assert_eq!(&*err.iri, "urn:purrdf:rdfc:hidden");
                assert_eq!(err.position, TermPosition::Predicate);
            }
            other => panic!("a reserved annotation predicate must refuse; got {other:?}"),
        }

        // The neighbouring VALID case, so the refusals above are not passing by
        // refusing everything: the emitted shapes themselves still fold.
        assert!(
            try_canonicalize(&statement_layer_fixture(Spelling::Lowered)).is_ok(),
            "the exact emitted shapes must remain admissible"
        );
    }

    /// The profile identity a consumer pins is readable from the API, and the reserved
    /// namespace really is the prefix of the sentinels the overlay lowers into — the
    /// one relationship the whole refusal argument rests on.
    #[test]
    fn the_profile_identity_and_the_reserved_namespace_are_consistent() {
        assert_eq!(CANON_PROFILE_ID, "purrdf-rdfc12");
        assert_eq!(CANON_PROFILE_VERSION, 2);
        assert!(SENTINEL_REIFIES.starts_with(RESERVED_NAMESPACE));
        assert!(SENTINEL_ANNOTATION_GRAPH.starts_with(RESERVED_NAMESPACE));
    }

    #[test]
    fn empty_dataset_canonicalizes_to_empty() {
        let ds = RdfDatasetBuilder::new().freeze().expect("valid");
        assert_eq!(canon(&ds), "");
    }

    #[test]
    fn literal_forms_are_verbatim() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p) = (iri(&mut b, "s"), iri(&mut b, "p"));
        // A typed literal whose lexical form MUST NOT be normalized (0.70 != 0.7).
        let lit = b.intern_literal(RdfLiteral::typed(
            "0.70",
            "http://www.w3.org/2001/XMLSchema#decimal",
        ));
        b.push_quad(s, p, lit, None);
        let ds = b.freeze().expect("valid");
        assert!(
            canon(&ds).contains("\"0.70\"^^<http://www.w3.org/2001/XMLSchema#decimal>"),
            "lexical form preserved: {}",
            canon(&ds)
        );
    }

    #[test]
    fn xsd_string_is_bare_and_directional_literal_renders() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, q) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "q"));
        let plain = b.intern_literal(RdfLiteral::simple("hi"));
        let rtl = b.intern_literal(RdfLiteral {
            lexical_form: "مرحبا".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        });
        b.push_quad(s, p, plain, None);
        b.push_quad(s, q, rtl, None);
        let out = canon(&ds_of(b));
        assert!(out.contains("\"hi\" ."), "xsd:string bare: {out}");
        assert!(
            out.contains("\"مرحبا\"@ar--rtl ."),
            "directional literal: {out}"
        );
    }

    fn ds_of(b: RdfDatasetBuilder) -> Arc<RdfDataset> {
        b.freeze().expect("valid")
    }

    #[test]
    fn isomorphic_blank_relabeling_is_byte_equal() {
        use super::super::term::BlankScope;
        let build = |l: &str, scope: u32| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let p = iri(&mut b, "p");
            let o = iri(&mut b, "o");
            let blank = b.intern_blank(l, BlankScope(scope));
            b.push_quad(blank, p, o, None);
            b.freeze().expect("valid")
        };
        let a = build("x", 0);
        let c = build("totally-different", 9);
        assert_eq!(
            canon(&a),
            canon(&c),
            "blank label/scope must not affect canon"
        );
    }

    /// The symmetric two-blank ring the OLD FNV comparator false-negatived: now it
    /// canonicalizes deterministically and two relabelings are byte-equal.
    #[test]
    fn symmetric_ring_resolves_deterministically() {
        use super::super::term::BlankScope;
        let build = |l1: &str, l2: &str| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (p, q) = (iri(&mut b, "p"), iri(&mut b, "q"));
            let x = b.intern_blank(l1, BlankScope(0));
            let y = b.intern_blank(l2, BlankScope(0));
            b.push_quad(x, p, y, None);
            b.push_quad(y, q, x, None);
            b.freeze().expect("valid")
        };
        let a = build("x", "y");
        let c = build("m", "n");
        let ca = canon(&a);
        assert_eq!(
            ca,
            canon(&c),
            "relabeled ring must canonicalize identically"
        );
        assert!(
            ca.contains("_:c14n0") && ca.contains("_:c14n1"),
            "stable labels: {ca}"
        );
    }

    #[test]
    fn self_loop_canonicalizes() {
        use super::super::term::BlankScope;
        let mut b = RdfDatasetBuilder::new();
        let p = iri(&mut b, "p");
        let x = b.intern_blank("x", BlankScope::DEFAULT);
        b.push_quad(x, p, x, None);
        let ds = b.freeze().expect("valid");
        assert_eq!(canon(&ds), "_:c14n0 <http://example.org/p> _:c14n0 .\n");
    }

    /// Differently-wired blank graphs must NOT be byte-equal.
    #[test]
    fn different_wiring_differs() {
        use super::super::term::BlankScope;
        let build = |neighbour: &str| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (p, link, s) = (iri(&mut b, "p"), iri(&mut b, "link"), iri(&mut b, "s"));
            let blank = b.intern_blank("b", BlankScope::DEFAULT);
            let nb = iri(&mut b, neighbour);
            b.push_quad(s, p, blank, None);
            b.push_quad(blank, link, nb, None);
            b.freeze().expect("valid")
        };
        assert_ne!(canon(&build("o1")), canon(&build("o2")));
    }

    /// Reifier COUNT is observable in the canonical form (the headline gate).
    #[test]
    fn reifier_count_shows_in_canon() {
        let build = |reifiers: &[&str]| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let triple = b.intern_triple(s, p, o);
            b.push_quad(s, p, o, None);
            for r in reifiers {
                let rid = iri(&mut b, r);
                b.push_reifier(rid, triple);
            }
            b.freeze().expect("valid")
        };
        let one = canon(&build(&["r1"]));
        let two = canon(&build(&["r1", "r2"]));
        assert_ne!(one, two, "two reifiers must differ from one");
        assert!(
            one.contains("<urn:purrdf:rdfc:reifies> <<( "),
            "reifier sentinel: {one}"
        );
        assert!(
            two.contains(
                "<<( <http://example.org/s> <http://example.org/p> <http://example.org/o> )>>"
            ),
            "triple term rendered: {two}"
        );
    }

    /// Annotation presence is observable.
    #[test]
    fn annotation_shows_in_canon() {
        let build = |annotated: bool| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let triple = b.intern_triple(s, p, o);
            let r = iri(&mut b, "r");
            b.push_quad(s, p, o, None);
            b.push_reifier(r, triple);
            if annotated {
                let (ap, ao) = (iri(&mut b, "ap"), iri(&mut b, "ao"));
                b.push_annotation(r, ap, ao);
            }
            b.freeze().expect("valid")
        };
        let with = canon(&build(true));
        let without = canon(&build(false));
        assert_ne!(with, without);
        assert!(
            with.contains("<urn:purrdf:rdfc:annotation> ."),
            "annotation graph sentinel: {with}"
        );
    }

    #[test]
    fn blank_count_counts_distinct_including_nested() {
        use super::super::term::BlankScope;
        let mut b = RdfDatasetBuilder::new();
        let p = iri(&mut b, "p");
        let x = b.intern_blank("x", BlankScope::DEFAULT);
        let y = b.intern_blank("y", BlankScope::DEFAULT);
        b.push_quad(x, p, y, None);
        b.push_quad(y, p, x, None);
        let ds = b.freeze().expect("valid");
        assert_eq!(blank_count(&ds), 2);
    }

    #[test]
    fn permutations_are_lexicographic_identity_first() {
        let perms: Vec<Vec<u32>> = permutations(&[10u32, 20, 30]).collect();
        assert_eq!(perms.len(), 6);
        assert_eq!(perms[0], vec![10, 20, 30], "identity first");
        assert_eq!(perms[5], vec![30, 20, 10], "reverse last");
        // A single-element slice yields exactly one permutation.
        assert_eq!(permutations(&[7u32]).count(), 1);
    }

    /// purrdf-EXT n-degree path: a symmetric blank pair reachable ONLY through
    /// quoted triple-term slots — the `.s`/`.o` position paths in
    /// [`CanonState::related_in_slot`] that the W3C suite never exercises. The
    /// automorphism must resolve deterministically (two relabelings byte-equal),
    /// and an asymmetric sibling must canonicalize differently.
    #[test]
    fn nested_triple_term_symmetry_resolves_deterministically() {
        use super::super::term::BlankScope;
        // <base> <ref> <<( x <link> y )>> .
        // <base> <ref> <<( y <link> x )>> .   — symmetric under x<->y, the symmetry
        // mediated entirely by blanks nested inside triple terms (no top-level blank
        // edge), so resolving it forces the triple-term-recursing n-degree search.
        let build = |l1: &str, l2: &str| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (base, refp, link) = (iri(&mut b, "base"), iri(&mut b, "ref"), iri(&mut b, "link"));
            let x = b.intern_blank(l1, BlankScope(0));
            let y = b.intern_blank(l2, BlankScope(0));
            let t1 = b.intern_triple(x, link, y);
            let t2 = b.intern_triple(y, link, x);
            b.push_quad(base, refp, t1, None);
            b.push_quad(base, refp, t2, None);
            b.freeze().expect("valid")
        };
        let ca = canon(&build("x", "y"));
        assert_eq!(
            ca,
            canon(&build("m", "n")),
            "nested-triple-term automorphism must canonicalize identically regardless of input labels"
        );
        assert!(
            ca.contains("_:c14n0") && ca.contains("_:c14n1"),
            "two stable nested blank labels: {ca}"
        );
        assert!(ca.contains("<<("), "triple terms rendered: {ca}");

        // Break the symmetry: give x one extra ground edge nested in a triple term.
        // x and y are no longer automorphic, so the canon output must differ.
        let asym = {
            let mut b = RdfDatasetBuilder::new();
            let (base, refp, link, tag) = (
                iri(&mut b, "base"),
                iri(&mut b, "ref"),
                iri(&mut b, "link"),
                iri(&mut b, "tag"),
            );
            let x = b.intern_blank("x", BlankScope(0));
            let y = b.intern_blank("y", BlankScope(0));
            let t1 = b.intern_triple(x, link, y);
            let t2 = b.intern_triple(y, link, x);
            let t3 = b.intern_triple(x, link, tag);
            b.push_quad(base, refp, t1, None);
            b.push_quad(base, refp, t2, None);
            b.push_quad(base, refp, t3, None);
            b.freeze().expect("valid")
        };
        assert_ne!(
            ca,
            canon(&asym),
            "an asymmetric nested-triple graph must not canonicalize to the symmetric one"
        );
    }

    // -----------------------------------------------------------------------
    // canonical_relabel: the caller-invoked recourse for egress-illegal labels
    // -----------------------------------------------------------------------

    /// The blank `(label, scope)` pairs the dataset's TERM TABLE carries.
    fn output_blanks(ds: &RdfDataset) -> BTreeSet<(String, BlankScope)> {
        (0..ds.term_count())
            .filter_map(|i| match ds.resolve(TermId::from_index(i as u32)) {
                TermRef::Blank { label, scope } => Some((label.to_owned(), scope)),
                _ => None,
            })
            .collect()
    }

    /// A dataset whose blanks carry labels illegal in every constrained egress
    /// alphabet, across every blank surface (quads, graph name, quoted triple,
    /// reifier, annotation) and across scopes.
    fn hostile_blank_dataset() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let p = iri(&mut b, "p");
        let o = iri(&mut b, "o");
        let bad = b.intern_blank("bad label", BlankScope::DEFAULT);
        let ctl = b.intern_blank("ctl\u{1}byte", BlankScope::DEFAULT);
        let uni = b.intern_blank("日本 空白", BlankScope(3));
        let bg = b.intern_blank("graph blank", BlankScope(2));
        b.push_quad(bad, p, ctl, None);
        b.push_quad(uni, p, o, Some(bg));
        let qs = b.intern_blank("quoted subject", BlankScope::DEFAULT);
        let triple = b.intern_triple(qs, p, o);
        b.push_quad(bad, p, triple, None);
        let r = b.intern_blank("reifier blank", BlankScope(5));
        b.push_reifier(r, triple);
        b.push_annotation(r, p, ctl);
        b.freeze().expect("valid")
    }

    #[test]
    fn canonical_relabel_output_labels_are_exactly_the_c14n_set() {
        use crate::blank_label::{LabelAlphabet, escape_label, is_valid_label};
        let ds = hostile_blank_dataset();
        let canonical = canonicalize(&ds);
        let out = canonical_relabel(&ds).expect("relabel");
        let expected: BTreeSet<(String, BlankScope)> = canonical
            .labels
            .values()
            .map(|l| (l.to_string(), BlankScope::DEFAULT))
            .collect();
        let got = output_blanks(&out);
        assert_eq!(got, expected, "output blanks must be exactly the c14n set");
        for (label, scope) in &got {
            assert_eq!(*scope, BlankScope::DEFAULT);
            // Legal under EVERY constrained egress alphabet (the serializer
            // gates for Turtle-family BLANK_NODE_LABEL and RDF/XML NCName).
            for alphabet in [
                LabelAlphabet::BlankNodeLabel,
                LabelAlphabet::NcName,
                LabelAlphabet::XmlText,
            ] {
                assert!(
                    is_valid_label(label, alphabet),
                    "{label:?} must be legal under {alphabet:?}"
                );
                // …so the egress escape is the IDENTITY on a canonical label:
                // relabeling before egress is the way to a byte-stable
                // re-serialization.
                assert_eq!(
                    escape_label(label, alphabet),
                    label.as_str(),
                    "the escape must not touch a canonical label under {alphabet:?}"
                );
            }
        }
    }

    #[test]
    fn canonical_relabel_is_idempotent_and_isomorphism_preserving() {
        let ds = hostile_blank_dataset();
        let once = canonical_relabel(&ds).expect("relabel once");
        let twice = canonical_relabel(&once).expect("relabel twice");
        assert_eq!(
            canonicalize(&once).nquads,
            canonicalize(&ds).nquads,
            "relabeling must preserve the isomorphism class"
        );
        assert_eq!(
            canonicalize(&twice).nquads,
            canonicalize(&once).nquads,
            "relabeling a relabeled dataset must change nothing (canonical bytes)"
        );
        assert_eq!(
            output_blanks(&twice),
            output_blanks(&once),
            "relabel twice = once, label for label"
        );
    }

    #[test]
    fn canonical_relabel_covers_a_declaration_only_blank_graph() {
        use crate::blank_label::{LabelAlphabet, is_valid_label};
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        let seen = b.intern_blank("seen blank", BlankScope::DEFAULT);
        b.push_quad(seen, p, o, None);
        let _ = s;
        // A blank named graph that owns no quads: invisible to canonicalization,
        // still relabeled (continuation numbering).
        let empty_graph = b.intern_blank("empty graph blank", BlankScope(4));
        b.declare_named_graph(empty_graph);
        let ds = b.freeze().expect("valid");
        let out = canonical_relabel(&ds).expect("relabel");
        let got = output_blanks(&out);
        assert_eq!(got.len(), 2, "both blanks survive: {got:?}");
        for (label, scope) in &got {
            assert_eq!(*scope, BlankScope::DEFAULT);
            assert!(label.starts_with(CANON_PREFIX), "{label:?}");
            assert!(is_valid_label(label, LabelAlphabet::BlankNodeLabel));
            assert!(is_valid_label(label, LabelAlphabet::NcName));
        }
        assert!(
            out.named_graphs().count() >= 1,
            "the declaration survives the rewrite"
        );
    }

    // -----------------------------------------------------------------------
    // The view seam: the flat entry points ARE the core, at `D = RdfDataset`
    // -----------------------------------------------------------------------

    /// A dataset exercising every surface the seam had to carry over: blanks in
    /// two scopes, a quoted triple term, reifier and annotation rows in both the
    /// default graph and a named one, and a declaration-only named graph.
    fn seam_dataset() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let (p, q, o) = (iri(&mut b, "p"), iri(&mut b, "q"), iri(&mut b, "o"));
        let g = iri(&mut b, "g");
        let shared = b.intern_blank("n", BlankScope::DEFAULT);
        let scoped = b.intern_blank("n", BlankScope(4));
        b.push_quad(shared, p, o, None);
        b.push_quad(shared, q, o, None);
        b.push_quad(scoped, p, o, Some(g));
        let triple = b.intern_triple(shared, p, o);
        b.push_quad(scoped, q, triple, Some(g));
        let reifier = b.intern_blank("r", BlankScope(7));
        b.push_reifier_in_graph(reifier, triple, Some(g));
        b.push_annotation_in_graph(reifier, q, o, Some(g));
        let default_reifier = iri(&mut b, "dr");
        b.push_reifier_in_graph(default_reifier, triple, None);
        b.push_annotation_in_graph(default_reifier, q, o, None);
        let declared_only = b.intern_blank("declared only", BlankScope(9));
        b.declare_named_graph(declared_only);
        b.freeze().expect("valid")
    }

    /// The `&RdfDataset` entry points are wrappers, not a second implementation:
    /// the bytes and the issued labels must be the view core's, identically. If
    /// they could differ, every consumer pinning the flat output would be pinning
    /// something the view path does not reproduce.
    #[test]
    fn the_flat_entry_points_are_the_view_core_at_rdfdataset() {
        let ds = seam_dataset();
        let flat = canonicalize_with(&ds, CanonHash::Sha256);
        let view = canonicalize_view(&*ds, CanonHash::Sha256);
        assert_eq!(flat.nquads, view.nquads);
        assert_eq!(flat.labels, view.labels);
        assert!(
            flat.nquads.contains("<urn:purrdf:rdfc:reifies>")
                && flat.nquads.contains("<urn:purrdf:rdfc:annotation>")
                && flat.nquads.contains("_:c14n"),
            "the fixture must reach the overlay and the labeler: {}",
            flat.nquads
        );
        // The defaulted type parameter: `Canonicalized` still names the `TermId`
        // instantiation, so existing call sites keep their key type unchanged.
        let flat: Canonicalized = flat;
        let _: &BTreeMap<TermId, Box<str>> = &flat.labels;
        // SHA-384 travels the same seam.
        assert_eq!(
            canonicalize_with(&ds, CanonHash::Sha384).nquads,
            canonicalize_view(&*ds, CanonHash::Sha384).nquads
        );
        // And so do the screening surfaces.
        assert_eq!(blank_count(&ds), blank_count_view(&*ds));
        assert!(check_admissible(&ds).is_ok() && check_admissible_view(&*ds).is_ok());
    }

    /// The per-graph canonicalization is the named-graph PROJECTION's canonical
    /// form, reached without building the projection. Both halves matter: the
    /// bytes must match for a graph that exists, and a graph that does not exist
    /// must canonicalize to the empty document rather than to anything else —
    /// the flat projection route already answers that way.
    #[test]
    fn graph_scoped_canonicalization_matches_the_named_graph_projection() {
        let ds = seam_dataset();
        let graph = "http://example.org/g";
        let projected = canonicalize(&ds.project_named_graph(graph)).nquads;
        assert!(!projected.is_empty(), "the graph must hold content");
        assert_eq!(
            canonicalize_graph_view(&*ds, graph, CanonHash::Sha256).nquads,
            projected
        );
        assert_eq!(
            graph_digest_view(&*ds, graph),
            ContentDigest::of(projected.as_bytes())
        );
        // The default graph's reifier over the same triple term belongs to the
        // default graph alone, so it never reaches `<g>`'s projection.
        assert!(
            !projected.contains("<http://example.org/dr>"),
            "{projected}"
        );
        // A graph the dataset never names selects nothing, exactly as projecting
        // it flat does.
        let absent = "http://example.org/no-such-graph";
        assert_eq!(canonicalize(&ds.project_named_graph(absent)).nquads, "");
        assert_eq!(
            canonicalize_graph_view(&*ds, absent, CanonHash::Sha256).nquads,
            ""
        );
    }

    /// A reserved IRI in ONE graph refuses that graph and leaves its neighbour
    /// admissible. The second half is the point: a per-graph digest that refused
    /// on another graph's content would not be a function of its own graph, and a
    /// refusal that spread would be the over-refusal mirror of a silent drop.
    #[test]
    fn a_graph_scoped_refusal_does_not_spread_to_a_neighbouring_graph() {
        let mut b = RdfDatasetBuilder::new();
        let (p, o) = (iri(&mut b, "p"), iri(&mut b, "o"));
        let (dirty, clean) = (iri(&mut b, "dirty"), iri(&mut b, "clean"));
        let bad = b.intern_iri(SENTINEL_REIFIES);
        b.push_quad(o, bad, o, Some(dirty));
        b.push_quad(o, p, o, Some(clean));
        let ds = b.freeze().expect("valid");
        assert!(matches!(
            try_canonicalize_graph_view(&*ds, "http://example.org/dirty", CanonHash::Sha256),
            Err(CanonError::ReservedVocabulary(_))
        ));
        let neighbour =
            try_canonicalize_graph_view(&*ds, "http://example.org/clean", CanonHash::Sha256)
                .expect("a clean graph must still canonicalize");
        assert_eq!(
            neighbour.nquads,
            canonicalize(&ds.project_named_graph("http://example.org/clean")).nquads
        );
        // The whole dataset IS refused, because the offending row is in it.
        assert!(matches!(
            try_canonicalize_view(&*ds, CanonHash::Sha256),
            Err(CanonError::ReservedVocabulary(_))
        ));
    }

    #[test]
    fn canonical_relabel_propagates_canonicalization_refusals() {
        let mut b = RdfDatasetBuilder::new();
        let (s, o) = (iri(&mut b, "s"), iri(&mut b, "o"));
        let bad = b.intern_iri(SENTINEL_REIFIES);
        b.push_quad(s, bad, o, None);
        let ds = b.freeze().expect("valid");
        assert!(
            matches!(
                canonical_relabel(&ds),
                Err(CanonError::ReservedVocabulary(_))
            ),
            "the relabel recourse must refuse exactly what canonicalization refuses"
        );
    }

    // -----------------------------------------------------------------------
    // The presentation axis: `Overlay` is byte-frozen, `FlatAssertion` lowers
    // the statement layer to ordinary quads and deduplicates against the base
    // quad set at the id level.
    // -----------------------------------------------------------------------

    /// Build the shared presentation-axis fixture: one reifier (a blank subject),
    /// one default-graph annotation on it, and one wholly ordinary quad — enough to
    /// exercise both statement-layer row kinds plus a genuine base quad in the same
    /// run.
    fn presentation_fixture() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let (s, pred, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        let reifier = b.intern_blank("r", BlankScope::DEFAULT);
        let triple = b.intern_triple(s, pred, o);
        b.push_reifier(reifier, triple);
        let conf = iri(&mut b, "confidence");
        let score = b.intern_literal(RdfLiteral::typed(
            "0.9",
            "http://www.w3.org/2001/XMLSchema#decimal",
        ));
        b.push_annotation(reifier, conf, score);
        let (os, op, oo) = (iri(&mut b, "os"), iri(&mut b, "op"), iri(&mut b, "oo"));
        b.push_quad(os, op, oo, None);
        b.freeze().expect("valid")
    }

    /// Threading the presentation axis through `collect_components` must not move a
    /// single byte of the OVERLAY path's output. The whole existing suite re-running
    /// unchanged (`cargo test -p purrdf-core`) is the same guarantee at crate scale;
    /// this test pins one concrete statement-bearing fixture (a reifier plus an
    /// annotation) against the literal bytes canonicalization produced before the
    /// presentation axis existed, so a future change to this module has one
    /// self-contained diff to check.
    #[test]
    fn the_overlay_presentation_is_byte_identical_to_its_pre_axis_output() {
        let ds = presentation_fixture();
        assert_eq!(
            canonicalize(&ds).nquads,
            "<http://example.org/os> <http://example.org/op> <http://example.org/oo> .\n\
             _:c14n0 <http://example.org/confidence> \"0.9\"^^<http://www.w3.org/2001/XMLSchema#decimal> <urn:purrdf:rdfc:annotation> .\n\
             _:c14n0 <urn:purrdf:rdfc:reifies> <<( <http://example.org/s> <http://example.org/p> <http://example.org/o> )>> .\n"
        );
    }

    /// `FlatAssertion` lowers the reifier and annotation rows to ORDINARY quad lines
    /// carrying their real predicate ids — no sentinel spelling, and the genuine base
    /// quad this fixture also carries stays exactly as it was. Reached only through a
    /// crate-internal call: no public entry point in this module offers this
    /// presentation.
    #[test]
    fn the_flat_presentation_lowers_the_statement_layer_to_ordinary_quads() {
        let ds = presentation_fixture();
        let flat = CanonState::new(
            &ds,
            CanonScope::Dataset,
            CanonPresentation::FlatAssertion,
            CanonHash::Sha256,
        )
        .run();
        assert_eq!(
            flat.nquads,
            "<http://example.org/os> <http://example.org/op> <http://example.org/oo> .\n\
             _:c14n0 <http://example.org/confidence> \"0.9\"^^<http://www.w3.org/2001/XMLSchema#decimal> .\n\
             _:c14n0 <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <http://example.org/s> <http://example.org/p> <http://example.org/o> )>> .\n"
        );
        assert!(
            !flat.nquads.contains(RESERVED_NAMESPACE),
            "no reserved-namespace IRI may appear in the flat presentation's output: {}",
            flat.nquads
        );
    }

    /// The flat dedup law: a side-table row whose `(s, p, o, g)` already exists as a
    /// genuine base quad is emitted exactly once — checked AT THE ID LEVEL (a raw
    /// [`collect_components`] walk, not the rendered-text `BTreeSet` `serialize_canonical`
    /// also deduplicates through) so a would-be duplicate component never reaches
    /// [`CanonState`]'s incident map. Exercised in both the default graph and a named
    /// graph, matching the two graph slots [`collect_components`]'s two `CanonScope`
    /// arms handle separately.
    #[test]
    fn the_flat_dedup_law_emits_a_doubly_spelled_row_exactly_once() {
        fn reifies_iri(b: &mut RdfDatasetBuilder) -> TermId {
            b.intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies")
        }
        fn component_count(ds: &RdfDataset) -> usize {
            let mut n = 0;
            collect_components(
                ds,
                CanonScope::Dataset,
                CanonPresentation::FlatAssertion,
                &mut |_| n += 1,
            );
            n
        }

        // Default graph: the row spelled once (a reifier declaration alone) vs. the
        // same row spelled twice (the declaration AND the literal quad it denotes)
        // must yield the SAME component count and the SAME canonical bytes.
        let mut once = RdfDatasetBuilder::new();
        let (s, pred, o) = (
            iri(&mut once, "s"),
            iri(&mut once, "p"),
            iri(&mut once, "o"),
        );
        let r = once.intern_blank("r", BlankScope::DEFAULT);
        let triple = once.intern_triple(s, pred, o);
        once.push_reifier(r, triple);
        let once = once.freeze().expect("valid");

        let mut twice = RdfDatasetBuilder::new();
        let (s, pred, o) = (
            iri(&mut twice, "s"),
            iri(&mut twice, "p"),
            iri(&mut twice, "o"),
        );
        let r = twice.intern_blank("r", BlankScope::DEFAULT);
        let triple = twice.intern_triple(s, pred, o);
        twice.push_reifier(r, triple);
        let reifies = reifies_iri(&mut twice);
        twice.push_quad(r, reifies, triple, None);
        let twice = twice.freeze().expect("valid");

        assert_eq!(
            component_count(&twice),
            component_count(&once),
            "a row spelled twice must contribute exactly the components a row spelled \
             once contributes — no duplicate may reach the incident map"
        );
        let flat_once = CanonState::new(
            &once,
            CanonScope::Dataset,
            CanonPresentation::FlatAssertion,
            CanonHash::Sha256,
        )
        .run();
        let flat_twice = CanonState::new(
            &twice,
            CanonScope::Dataset,
            CanonPresentation::FlatAssertion,
            CanonHash::Sha256,
        )
        .run();
        assert_eq!(
            flat_twice.nquads, flat_once.nquads,
            "spelling one default-graph row twice must not change the flat canonical form"
        );

        // Named graph: the same pair, with the row (and its duplicate) declared in a
        // named graph instead of the default graph.
        let mut once_g = RdfDatasetBuilder::new();
        let (s, pred, o) = (
            iri(&mut once_g, "s"),
            iri(&mut once_g, "p"),
            iri(&mut once_g, "o"),
        );
        let r = once_g.intern_blank("r", BlankScope::DEFAULT);
        let triple = once_g.intern_triple(s, pred, o);
        let g = iri(&mut once_g, "g");
        once_g.push_reifier_in_graph(r, triple, Some(g));
        let once_g = once_g.freeze().expect("valid");

        let mut twice_g = RdfDatasetBuilder::new();
        let (s, pred, o) = (
            iri(&mut twice_g, "s"),
            iri(&mut twice_g, "p"),
            iri(&mut twice_g, "o"),
        );
        let r = twice_g.intern_blank("r", BlankScope::DEFAULT);
        let triple = twice_g.intern_triple(s, pred, o);
        let g = iri(&mut twice_g, "g");
        twice_g.push_reifier_in_graph(r, triple, Some(g));
        let reifies = reifies_iri(&mut twice_g);
        twice_g.push_quad(r, reifies, triple, Some(g));
        let twice_g = twice_g.freeze().expect("valid");

        assert_eq!(
            component_count(&twice_g),
            component_count(&once_g),
            "the same dedup law must hold when the doubly-spelled row is in a named graph"
        );
        let flat_once_g = CanonState::new(
            &once_g,
            CanonScope::Dataset,
            CanonPresentation::FlatAssertion,
            CanonHash::Sha256,
        )
        .run();
        let flat_twice_g = CanonState::new(
            &twice_g,
            CanonScope::Dataset,
            CanonPresentation::FlatAssertion,
            CanonHash::Sha256,
        )
        .run();
        assert_eq!(
            flat_twice_g.nquads, flat_once_g.nquads,
            "spelling one named-graph row twice must not change the flat canonical form"
        );
    }

    // -----------------------------------------------------------------------
    // The flat presentation must not leak the overlay's reserved sentinel for a
    // base quad that merely SPELLS a statement-layer row, and the two spellings of
    // one row must co-canonicalize under FlatAssertion exactly as they already did
    // under Overlay.
    // -----------------------------------------------------------------------

    /// The flat canonical bytes of `ds`, panicking on refusal (test convenience,
    /// mirroring [`canon`] for the flat presentation).
    fn flat_canon(ds: &RdfDataset) -> String {
        try_canonicalize_flat_view(ds, CanonHash::Sha256)
            .expect("admissible")
            .nquads
    }

    /// The flat counterpart of
    /// [`a_literally_asserted_sentinel_row_folds_into_the_reifier_it_denotes`]: a base
    /// quad spelling a reifier row via [`SENTINEL_REIFIES`] must lower to the SAME
    /// `rdf:reifies` row the native reifier lowers to — never to a row still carrying
    /// the sentinel. This is the exact shape of the CLI bug this fix closes: `ex:r
    /// <urn:purrdf:rdfc:reifies> <<( ex:s ex:p ex:o )>>` flat-canonicalizing WITH the
    /// sentinel while the native reifier emits `rdf:reifies` — two identities for one
    /// row.
    #[test]
    fn a_literally_asserted_sentinel_row_folds_into_the_flat_reifier_row_it_denotes() {
        // A: a genuine (native) reifier.
        let mut b = RdfDatasetBuilder::new();
        let (s, pred, o, r) = (
            iri(&mut b, "s"),
            iri(&mut b, "p"),
            iri(&mut b, "o"),
            iri(&mut b, "r"),
        );
        let triple = b.intern_triple(s, pred, o);
        b.push_reifier(r, triple);
        let genuine = b.freeze().expect("valid");
        let lowered = flat_canon(&genuine);
        assert!(
            lowered.contains(RDF_REIFIES),
            "the fixture must actually exercise the real-predicate lowering: {lowered}"
        );
        assert!(
            !lowered.contains(RESERVED_NAMESPACE),
            "the native reifier's flat form must never carry the sentinel: {lowered}"
        );

        // B: no native reifier — the sentinel-SPELLED row asserted literally as an
        // ordinary base quad (exactly the adversary's CLI demonstration).
        let mut b = RdfDatasetBuilder::new();
        let (s, o, r) = (iri(&mut b, "s"), iri(&mut b, "o"), iri(&mut b, "r"));
        let pred = iri(&mut b, "p");
        let sentinel = b.intern_iri(SENTINEL_REIFIES);
        let triple = b.intern_triple(s, pred, o);
        b.push_quad(r, sentinel, triple, None);
        let spelled = b.freeze().expect("valid");
        assert_eq!(
            spelled.reifier_quads().count(),
            0,
            "B must carry no statement layer of its own — the quad IS the input"
        );

        let folded = flat_canon(&spelled);
        assert_eq!(
            folded, lowered,
            "the sentinel-spelled row and the native reifier it denotes must produce \
             the SAME flat bytes — the sentinel must not leak, and the two \
             spellings must not split identity"
        );
        assert!(
            !folded.contains(RESERVED_NAMESPACE),
            "the spelled row's flat form must never carry the sentinel either: {folded}"
        );

        // C: the row spelled BOTH ways (native + sentinel-spelled base quad) is still
        // exactly one row.
        let mut b = RdfDatasetBuilder::new();
        let (s, pred, o, r) = (
            iri(&mut b, "s"),
            iri(&mut b, "p"),
            iri(&mut b, "o"),
            iri(&mut b, "r"),
        );
        let triple = b.intern_triple(s, pred, o);
        let sentinel = b.intern_iri(SENTINEL_REIFIES);
        b.push_reifier(r, triple);
        b.push_quad(r, sentinel, triple, None);
        let both = b.freeze().expect("valid");
        assert_eq!(
            flat_canon(&both),
            lowered,
            "spelling one row twice (native + sentinel-spelled) must not change the \
             flat canonical form"
        );
    }

    /// The annotation-side twin of
    /// [`a_literally_asserted_sentinel_row_folds_into_the_flat_reifier_row_it_denotes`]:
    /// a base quad spelling an annotation row via [`SENTINEL_ANNOTATION_GRAPH`] (the
    /// sentinel used as the quad's GRAPH) must lower to the SAME `(r, p, o)` row the
    /// native annotation lowers to, with no graph token at all — never to a row still
    /// carrying the annotation sentinel as a fourth token.
    #[test]
    fn a_literally_asserted_sentinel_row_folds_into_the_flat_annotation_row_it_denotes() {
        // A: a genuine (native) annotation.
        let mut b = RdfDatasetBuilder::new();
        let (r, pred, o) = (iri(&mut b, "r"), iri(&mut b, "p"), iri(&mut b, "o"));
        b.push_annotation(r, pred, o);
        let genuine = b.freeze().expect("valid");
        let lowered = flat_canon(&genuine);
        assert_eq!(
            lowered, "<http://example.org/r> <http://example.org/p> <http://example.org/o> .\n",
            "the fixture must actually exercise the annotation lowering"
        );
        assert!(!lowered.contains(RESERVED_NAMESPACE));

        // B: no native annotation — the sentinel-SPELLED row asserted literally (the
        // sentinel as the quad's graph name, exactly as the adversary's CLI
        // demonstration spells it).
        let mut b = RdfDatasetBuilder::new();
        let (r, pred, o) = (iri(&mut b, "r"), iri(&mut b, "p"), iri(&mut b, "o"));
        let sentinel = b.intern_iri(SENTINEL_ANNOTATION_GRAPH);
        b.push_quad(r, pred, o, Some(sentinel));
        let spelled = b.freeze().expect("valid");
        assert_eq!(
            spelled.annotation_quads().count(),
            0,
            "B must carry no statement layer of its own — the quad IS the input"
        );

        let folded = flat_canon(&spelled);
        assert_eq!(
            folded, lowered,
            "the sentinel-spelled row and the native annotation it denotes must \
             produce the SAME flat bytes — the annotation-graph sentinel must not \
             leak into flat output as a graph name"
        );
        assert!(!folded.contains(RESERVED_NAMESPACE));

        // C: both ways at once.
        let mut b = RdfDatasetBuilder::new();
        let (r, pred, o) = (iri(&mut b, "r"), iri(&mut b, "p"), iri(&mut b, "o"));
        let sentinel = b.intern_iri(SENTINEL_ANNOTATION_GRAPH);
        b.push_annotation(r, pred, o);
        b.push_quad(r, pred, o, Some(sentinel));
        let both = b.freeze().expect("valid");
        assert_eq!(
            flat_canon(&both),
            lowered,
            "spelling one annotation row twice must not change the flat canonical form"
        );
    }

    /// The byte-identity the fold's soundness rests on: a [`Slot::Term`] resolving to
    /// the real `rdf:reifies` IRI and a [`Slot::Sentinel`] literal carrying the exact
    /// same IRI text render to IDENTICAL bytes. This is what makes
    /// [`Component::FlatReifier`] (predicate always rendered as literal text, because
    /// a view holding only a sentinel-spelled quad may never have interned
    /// `rdf:reifies`) a safe stand-in for a natively-lowered [`Component::Quad`]
    /// (predicate rendered from an interned id) regardless of which one the view
    /// happens to intern.
    #[test]
    fn a_flat_reifier_slot_renders_identically_whether_interned_or_literal() {
        let mut b = RdfDatasetBuilder::new();
        let (s, pred, o, r) = (
            iri(&mut b, "s"),
            iri(&mut b, "p"),
            iri(&mut b, "o"),
            iri(&mut b, "r"),
        );
        let triple = b.intern_triple(s, pred, o);
        // Intern the real predicate too, so the `Slot::Term` rendering has a genuine
        // id to resolve — proving the comparison, not merely a vacuous one where the
        // id path was never exercised.
        let real_reifies = b.intern_iri(RDF_REIFIES);
        let ds = b.freeze().expect("valid");

        let state = CanonState::new(
            &*ds,
            CanonScope::Dataset,
            CanonPresentation::FlatAssertion,
            CanonHash::Sha256,
        );
        let render = BlankRender::Canonical {
            issuer: &state.canonical,
        };

        let mut via_id = String::new();
        state.write_component(
            Component::Quad {
                s: r,
                p: real_reifies,
                o: triple,
                g: None,
            },
            render,
            &mut via_id,
        );

        let mut via_literal = String::new();
        state.write_component(
            Component::FlatReifier {
                r,
                t: triple,
                g: None,
            },
            render,
            &mut via_literal,
        );

        assert_eq!(
            via_id, via_literal,
            "a Slot::Term resolving to the real rdf:reifies IRI and a Slot::Sentinel \
             literal carrying the same IRI text must render byte-identically"
        );
    }

    /// The dedup law's combination `already_native` cannot see by itself: no native
    /// side-table row at all, only TWO base quads spelling the same row two different
    /// ways — the overlay sentinel spelling, and the row's own real predicate/graph.
    /// [`lower_folded_row`] is what closes this combination for the flat presentation.
    #[test]
    fn the_flat_dedup_law_covers_sentinel_and_real_predicate_base_quads_without_a_native_row() {
        // Reifier.
        let mut b = RdfDatasetBuilder::new();
        let (s, pred, o, r) = (
            iri(&mut b, "s"),
            iri(&mut b, "p"),
            iri(&mut b, "o"),
            iri(&mut b, "r"),
        );
        let triple = b.intern_triple(s, pred, o);
        let sentinel = b.intern_iri(SENTINEL_REIFIES);
        b.push_quad(r, sentinel, triple, None);
        let real = b.intern_iri(RDF_REIFIES);
        b.push_quad(r, real, triple, None);
        let ds = b.freeze().expect("valid");
        assert_eq!(
            ds.reifier_quads().count(),
            0,
            "no native row in this fixture"
        );

        let bytes = flat_canon(&ds);
        let line_count = bytes.lines().filter(|l| !l.is_empty()).count();
        assert_eq!(
            line_count, 1,
            "a row spelled ONLY as a sentinel-spelled quad and a real-predicate quad \
             (no native side-table row) must still be exactly one flat row: {bytes}"
        );
        assert!(bytes.contains(RDF_REIFIES));
        assert!(!bytes.contains(RESERVED_NAMESPACE));

        // Annotation.
        let mut b = RdfDatasetBuilder::new();
        let (r, pred, o) = (iri(&mut b, "r"), iri(&mut b, "p"), iri(&mut b, "o"));
        let sentinel = b.intern_iri(SENTINEL_ANNOTATION_GRAPH);
        b.push_quad(r, pred, o, Some(sentinel));
        b.push_quad(r, pred, o, None);
        let ds = b.freeze().expect("valid");
        assert_eq!(
            ds.annotation_quads().count(),
            0,
            "no native row in this fixture"
        );

        let bytes = flat_canon(&ds);
        let line_count = bytes.lines().filter(|l| !l.is_empty()).count();
        assert_eq!(
            line_count, 1,
            "the annotation twin of the same law: {bytes}"
        );
        assert!(!bytes.contains(RESERVED_NAMESPACE));
    }

    /// The same dedup law, holding when both base-quad spellings live in a NAMED
    /// graph rather than the default graph — the case that exercises
    /// [`base_quad_component`]'s `graph_probe`/erased-`g` split under
    /// [`CanonScope::Graph`]: a probe that (incorrectly) fell back to the default
    /// graph after erasure would fail to find the real-predicate quad and emit two
    /// lines instead of one.
    #[test]
    fn the_flat_dedup_law_holds_for_sentinel_and_real_predicate_base_quads_in_a_named_graph() {
        let mut b = RdfDatasetBuilder::new();
        let (s, pred, o, r) = (
            iri(&mut b, "s"),
            iri(&mut b, "p"),
            iri(&mut b, "o"),
            iri(&mut b, "r"),
        );
        let triple = b.intern_triple(s, pred, o);
        let g = iri(&mut b, "g");
        let sentinel = b.intern_iri(SENTINEL_REIFIES);
        b.push_quad(r, sentinel, triple, Some(g));
        let real = b.intern_iri(RDF_REIFIES);
        b.push_quad(r, real, triple, Some(g));
        let ds = b.freeze().expect("valid");

        let scoped =
            try_canonicalize_flat_graph_view(&*ds, "http://example.org/g", CanonHash::Sha256)
                .expect("admissible");
        let line_count = scoped.nquads.lines().filter(|l| !l.is_empty()).count();
        assert_eq!(
            line_count, 1,
            "the same dedup law must hold when both spellings live in a named graph: {}",
            scoped.nquads
        );
        assert!(scoped.nquads.contains(RDF_REIFIES));
        assert!(!scoped.nquads.contains(RESERVED_NAMESPACE));
        // Graph-erasing emission: the per-graph document carries no graph token.
        assert_eq!(
            scoped.nquads,
            format!(
                "<http://example.org/r> <{RDF_REIFIES}> <<( <http://example.org/s> \
                 <http://example.org/p> <http://example.org/o> )>> .\n"
            )
        );
    }

    /// All three spellings at once (native side-table row + sentinel-spelled base
    /// quad + real-predicate base quad) must still yield exactly one row — the full
    /// combination the fix's dedup laws jointly cover: `already_native` drops the
    /// sentinel-spelled quad (a native row exists), and the native row's own
    /// pre-existing `already_asserted` check drops IT in turn (a real-predicate quad
    /// already asserts it), leaving the real-predicate quad as the row's sole
    /// emitter.
    #[test]
    fn the_flat_dedup_law_covers_all_three_spellings_combined() {
        // Reifier.
        let mut b = RdfDatasetBuilder::new();
        let (s, pred, o, r) = (
            iri(&mut b, "s"),
            iri(&mut b, "p"),
            iri(&mut b, "o"),
            iri(&mut b, "r"),
        );
        let triple = b.intern_triple(s, pred, o);
        b.push_reifier(r, triple);
        let sentinel = b.intern_iri(SENTINEL_REIFIES);
        b.push_quad(r, sentinel, triple, None);
        let real = b.intern_iri(RDF_REIFIES);
        b.push_quad(r, real, triple, None);
        let all_three = b.freeze().expect("valid");

        let mut solo = RdfDatasetBuilder::new();
        let (s, pred, o, r) = (
            iri(&mut solo, "s"),
            iri(&mut solo, "p"),
            iri(&mut solo, "o"),
            iri(&mut solo, "r"),
        );
        let triple = solo.intern_triple(s, pred, o);
        solo.push_reifier(r, triple);
        let solo = solo.freeze().expect("valid");

        assert_eq!(
            flat_canon(&all_three),
            flat_canon(&solo),
            "native + sentinel-spelled + real-predicate must still yield exactly one row"
        );

        fn component_count(ds: &RdfDataset) -> usize {
            let mut n = 0;
            collect_components(
                ds,
                CanonScope::Dataset,
                CanonPresentation::FlatAssertion,
                &mut |_| n += 1,
            );
            n
        }
        assert_eq!(
            component_count(&all_three),
            component_count(&solo),
            "the three spellings must contribute exactly the components the lone \
             native row contributes"
        );

        // Annotation.
        let mut b = RdfDatasetBuilder::new();
        let (r, pred, o) = (iri(&mut b, "r"), iri(&mut b, "p"), iri(&mut b, "o"));
        b.push_annotation(r, pred, o);
        let sentinel = b.intern_iri(SENTINEL_ANNOTATION_GRAPH);
        b.push_quad(r, pred, o, Some(sentinel));
        b.push_quad(r, pred, o, None);
        let all_three_ann = b.freeze().expect("valid");

        let mut solo_ann = RdfDatasetBuilder::new();
        let (r, pred, o) = (
            iri(&mut solo_ann, "r"),
            iri(&mut solo_ann, "p"),
            iri(&mut solo_ann, "o"),
        );
        solo_ann.push_annotation(r, pred, o);
        let solo_ann = solo_ann.freeze().expect("valid");

        assert_eq!(
            flat_canon(&all_three_ann),
            flat_canon(&solo_ann),
            "annotation: native + sentinel-spelled + real-quad must still yield \
             exactly one row"
        );
        assert_eq!(component_count(&all_three_ann), component_count(&solo_ann));
    }

    /// Admission does not depend on presentation (module contract): the exact same
    /// near misses the overlay refuses are refused, typed, under flat too — paired
    /// with the neighbouring VALID shape (the exact folded shape) staying admitted,
    /// per the over-refusal rule.
    #[test]
    fn flat_presentation_refuses_the_same_near_misses_the_overlay_does() {
        // Invalid: the reifies sentinel over a NON-triple object — refused under both.
        let mut b = RdfDatasetBuilder::new();
        let (r, o) = (iri(&mut b, "r"), iri(&mut b, "o"));
        let reifies = b.intern_iri(SENTINEL_REIFIES);
        b.push_quad(r, reifies, o, None);
        let ds = b.freeze().expect("valid");
        assert!(matches!(
            try_canonicalize(&ds),
            Err(CanonError::ReservedVocabulary(_))
        ));
        assert!(matches!(
            try_canonicalize_flat_view(&*ds, CanonHash::Sha256),
            Err(ViewCanonError::Refused(CanonError::ReservedVocabulary(_)))
        ));

        // The neighbouring VALID case: the exact folded shape (a triple-term object)
        // is ADMITTED under flat too, and lowers instead of refusing.
        let mut b = RdfDatasetBuilder::new();
        let (s, pred, o, r) = (
            iri(&mut b, "s"),
            iri(&mut b, "p"),
            iri(&mut b, "o"),
            iri(&mut b, "r"),
        );
        let triple = b.intern_triple(s, pred, o);
        let reifies = b.intern_iri(SENTINEL_REIFIES);
        b.push_quad(r, reifies, triple, None);
        let ds = b.freeze().expect("valid");
        let flat = try_canonicalize_flat_view(&*ds, CanonHash::Sha256)
            .expect("the exact folded shape must be admitted under flat, too");
        assert!(flat.nquads.contains(RDF_REIFIES));
        assert!(!flat.nquads.contains(RESERVED_NAMESPACE));
    }

    // -----------------------------------------------------------------------
    // The public flat-presentation, fault-aware entry points: `try_canonicalize_flat_view`,
    // `try_canonicalize_flat_graph_view`, `check_admissible_flat_view`,
    // `try_flat_digest_view`.
    // -----------------------------------------------------------------------

    /// The refused case: the reserved-vocabulary rule is unchanged by presentation,
    /// so both flat entry points that admit-or-refuse must refuse it typed, exactly
    /// as the overlay family does. The neighbouring VALID case pins the mirror
    /// half: an IRI adjacent to, but outside, [`RESERVED_NAMESPACE`] — used as a
    /// GRAPH name, the position [`the_fold_is_shape_exact_and_its_near_misses_still_refuse`]
    /// never exercises — must be admitted and appear in the output.
    #[test]
    fn flat_entry_points_refuse_reserved_vocabulary_typed_and_admit_a_neighbouring_iri() {
        let mut b = RdfDatasetBuilder::new();
        let (r, o) = (iri(&mut b, "r"), iri(&mut b, "o"));
        let bad = b.intern_iri(SENTINEL_REIFIES);
        b.push_quad(r, bad, o, None);
        let ds = b.freeze().expect("valid");

        match try_canonicalize_flat_view(&*ds, CanonHash::Sha256) {
            Err(ViewCanonError::Refused(CanonError::ReservedVocabulary(err))) => {
                assert_eq!(&*err.iri, SENTINEL_REIFIES);
            }
            other => panic!("expected a typed reserved-vocabulary refusal; got {other:?}"),
        }
        assert!(
            matches!(
                check_admissible_flat_view(&*ds),
                Err(ViewCanonError::Refused(CanonError::ReservedVocabulary(_)))
            ),
            "check_admissible_flat_view must refuse the same input the same way"
        );

        // The neighbour: outside `RESERVED_NAMESPACE` (`urn:purrdf:rdfc:`) even
        // though it shares two path segments with it.
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        let neighbour_graph = b.intern_iri("urn:purrdf:other:annotation");
        b.push_quad(s, p, o, Some(neighbour_graph));
        let ds = b.freeze().expect("valid");

        let admitted = try_canonicalize_flat_view(&*ds, CanonHash::Sha256)
            .expect("an IRI outside the reserved namespace must be admitted");
        assert!(
            admitted.nquads.contains("<urn:purrdf:other:annotation>"),
            "the neighbouring graph name must appear in the output: {}",
            admitted.nquads
        );
        assert!(check_admissible_flat_view(&*ds).is_ok());
    }

    /// The exact W3C RDFC-1.0 `test074` poison shape, reproduced natively: `n`
    /// mutually symmetric blank nodes, every ORDERED pair (including self-loops)
    /// linked by the same predicate — a complete symmetric digraph. `n = 10` is the
    /// documented minimal shape (see `crates/rdf/tests/gts_certify.rs`'s
    /// `poison_symmetric_source`, and `crates/rdf/tests/rdfc_w3c.rs`'s heavy-offgate
    /// `test074`) that exceeds [`RDFC_CALL_LIMIT`] (a single ambiguous hash group of
    /// 10 mutually-symmetric blanks contributes up to `10! = 3,628,800` permutations
    /// to one `hash_n_degree` call) while its blank COUNT stays nowhere near any
    /// count-based pre-reject a caller might apply upstream of this module.
    fn poison_symmetric_dataset(n: usize) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let p = iri(&mut b, "p");
        let blanks: Vec<TermId> = (0..n)
            .map(|i| b.intern_blank(&format!("e{i}"), BlankScope::DEFAULT))
            .collect();
        for &a in &blanks {
            for &c in &blanks {
                b.push_quad(a, p, c, None);
            }
        }
        b.freeze().expect("valid")
    }

    /// A LEGITIMATE, non-symmetric neighbour of [`poison_symmetric_dataset`]:
    /// `pairs` independent symmetric blank-node PAIRS, each tied to its own unique
    /// ground anchor so pairs are mutually distinguishable (no CROSS-pair symmetry)
    /// while remaining internally ambiguous — RDFC-1.0's simplest automorphism
    /// shape (see [`symmetric_ring_resolves_deterministically`]). Each pair's
    /// n-degree resolution is O(1), so the total search stays far under
    /// [`RDFC_CALL_LIMIT`] regardless of `pairs`: a graph that merely LOOKS
    /// expensive (many blanks) must not be refused for the reason the genuinely
    /// poisoned shape above is — the over-refusal mirror of the poison case.
    fn large_non_symmetric_dataset(pairs: u32) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let link = iri(&mut b, "link");
        for i in 0..pairs {
            let anchor = iri(&mut b, &format!("anchor{i}"));
            let x = b.intern_blank(&format!("x{i}"), BlankScope::DEFAULT);
            let y = b.intern_blank(&format!("y{i}"), BlankScope::DEFAULT);
            b.push_quad(anchor, link, x, None);
            b.push_quad(anchor, link, y, None);
            b.push_quad(x, link, y, None);
            b.push_quad(y, link, x, None);
        }
        b.freeze().expect("valid")
    }

    /// Budget exhaustion refuses typed through the public flat entry point, exactly
    /// as [`try_canonicalize_view`] does for the overlay; the paired valid
    /// neighbour — legitimately large, but not globally symmetric — stays green
    /// through the SAME entry point.
    #[test]
    fn try_canonicalize_flat_view_refuses_budget_exhaustion_and_a_large_neighbour_stays_green() {
        let poison = poison_symmetric_dataset(10);
        match try_canonicalize_flat_view(&*poison, CanonHash::Sha256) {
            Err(ViewCanonError::Refused(CanonError::BudgetExceeded(err))) => {
                assert_eq!(err.blank_count, 10);
            }
            other => panic!("expected a typed budget-exceeded refusal; got {other:?}"),
        }

        let large = large_non_symmetric_dataset(200);
        let admitted = try_canonicalize_flat_view(&*large, CanonHash::Sha256)
            .expect("a large non-symmetric graph must not be refused for budget reasons");
        assert_ne!(admitted.nquads, "");
        assert_eq!(
            admitted.labels.len(),
            400,
            "every pair's two blanks get a label"
        );
    }

    /// `try_canonicalize_flat_view` accepts `RdfDataset` directly AND `Arc<RdfDataset>`
    /// — both satisfy `FallibleDatasetView` (the latter via the blanket `Arc<T>`
    /// impl), both with `Error = Infallible`, and both must produce byte-identical
    /// output for the same content.
    #[test]
    fn try_canonicalize_flat_view_accepts_both_rdfdataset_and_arc_rdfdataset() {
        let arc_ds = presentation_fixture();
        let owned_ds = Arc::try_unwrap(presentation_fixture())
            .expect("a freshly built, singly-owned dataset must unwrap");
        let via_arc =
            try_canonicalize_flat_view(&arc_ds, CanonHash::Sha256).expect("admissible fixture");
        let via_owned =
            try_canonicalize_flat_view(&owned_ds, CanonHash::Sha256).expect("admissible fixture");
        assert_eq!(via_arc.nquads, via_owned.nquads);
    }

    /// A two-graph fixture: graph `gA` carries a reifier, an annotation on it, and
    /// an ordinary quad (the same three-row shape [`presentation_fixture`]
    /// exercises); graph `gB` carries unrelated content the graph scope must not
    /// admit.
    fn two_graph_flat_fixture() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let ga = iri(&mut b, "gA");
        let gb = iri(&mut b, "gB");
        let (s, pred, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        let reifier = b.intern_blank("r", BlankScope::DEFAULT);
        let triple = b.intern_triple(s, pred, o);
        b.push_reifier_in_graph(reifier, triple, Some(ga));
        let conf = iri(&mut b, "confidence");
        let score = b.intern_literal(RdfLiteral::typed(
            "0.9",
            "http://www.w3.org/2001/XMLSchema#decimal",
        ));
        b.push_annotation_in_graph(reifier, conf, score, Some(ga));
        let (os, op, oo) = (iri(&mut b, "os"), iri(&mut b, "op"), iri(&mut b, "oo"));
        b.push_quad(os, op, oo, Some(ga));
        let (bs, bp, bo) = (iri(&mut b, "bs"), iri(&mut b, "bp"), iri(&mut b, "bo"));
        b.push_quad(bs, bp, bo, Some(gb));
        b.freeze().expect("valid")
    }

    /// Composition sanity: `try_canonicalize_flat_graph_view` on graph A equals
    /// `try_canonicalize_flat_view` on a dataset containing only graph A's rows
    /// (`project_named_graph`, already graph-erased) — the same relationship the
    /// overlay family's `graph_scoped_canonicalization_matches_the_named_graph_projection`
    /// pins, held under the flat presentation instead.
    #[test]
    fn flat_graph_scope_is_flat_dataset_scope_composed_with_projection() {
        let ds = two_graph_flat_fixture();
        let graph = "http://example.org/gA";
        let projected = ds.project_named_graph(graph);
        let via_projection =
            try_canonicalize_flat_view(&projected, CanonHash::Sha256).expect("admissible");
        let via_graph_scope =
            try_canonicalize_flat_graph_view(&*ds, graph, CanonHash::Sha256).expect("admissible");
        assert_eq!(via_graph_scope.nquads, via_projection.nquads);
        assert!(
            !via_graph_scope.nquads.contains("<http://example.org/bs>"),
            "graph B's content must not leak into graph A's scope: {}",
            via_graph_scope.nquads
        );
    }

    /// `try_flat_digest_view` is exactly hashing `try_canonicalize_flat_view`'s
    /// `nquads` under the same algorithm — the flat sibling of the law
    /// `graph_digest_view`/`try_graph_digest_view` state for the overlay's per-graph
    /// digest, held here over the whole-dataset flat digest instead.
    #[test]
    fn try_flat_digest_view_hashes_the_flat_canonical_document() {
        let ds = presentation_fixture();
        let flat = try_canonicalize_flat_view(&*ds, CanonHash::Sha256).expect("admissible fixture");
        let digest = try_flat_digest_view(&*ds, CanonHash::Sha256).expect("admissible fixture");
        assert_eq!(digest, ContentDigest::of(flat.nquads.as_bytes()));

        // SHA-384 travels the same seam — and selects ONLY the label-issuance
        // algorithm: the digest over the resulting document is still the
        // unconditional SHA-256 `ContentDigest`, so a Sha384-labelled run's
        // digest length and value are those of `ContentDigest::of`, never a
        // SHA-384 of anything.
        let flat384 =
            try_canonicalize_flat_view(&*ds, CanonHash::Sha384).expect("admissible fixture");
        let digest384 = try_flat_digest_view(&*ds, CanonHash::Sha384).expect("admissible fixture");
        assert_eq!(digest384, ContentDigest::of(flat384.nquads.as_bytes()));
    }

    // -----------------------------------------------------------------------
    // Fault closure through the PUBLIC flat entry points: a `FallibleDatasetView`
    // whose backing data faults must never let a `Canonicalized` escape, and must
    // report which checkpoint saw it — the same law
    // `dataset_view::tests::checkpointed_drain` pins directly, exercised here at
    // this module's own public boundary.
    // -----------------------------------------------------------------------

    /// The typed root cause a [`FlatProbeView`] reports once faulted — the same
    /// controllable-status probe pattern `dataset_view`'s own `checkpointed_drain`
    /// tests use (`ProbeView`/`ProbeFault`, private to that module's own test
    /// suite), reproduced here because these public entry points are THIS module's
    /// own fault-closure boundary and need their own instance of the same harness.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FlatProbeFault(&'static str);

    impl std::fmt::Display for FlatProbeFault {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.0)
        }
    }

    impl std::error::Error for FlatProbeFault {}

    /// A [`FallibleDatasetView`] wrapping a real [`RdfDataset`] (so a run reads
    /// genuine content, including the RDF 1.2 side tables) whose
    /// [`operation_status`](FallibleDatasetView::operation_status) is driven by two
    /// independent controls: `pre_faulted`, fixed at construction (the view was
    /// already broken before anyone drained it), and `faulted_by_read`, a `Cell`
    /// flipped by the FIRST read accessor a run touches (the view faults PARTWAY
    /// THROUGH). The entry points under test own their drain closure internally, so
    /// unlike `dataset_view`'s `ProbeView` tests — which inject the fault from the
    /// closure the TEST supplies — the fault here has to be a side effect of being
    /// READ.
    struct FlatProbeView {
        inner: Arc<RdfDataset>,
        pre_faulted: bool,
        faulted_by_read: std::cell::Cell<bool>,
    }

    impl FlatProbeView {
        /// Already `Failed` at construction: the FIRST checkpoint observes the
        /// fault, so the drain closure must never run at all.
        fn already_failed(inner: Arc<RdfDataset>) -> Self {
            Self {
                inner,
                pre_faulted: true,
                faulted_by_read: std::cell::Cell::new(false),
            }
        }

        /// `Ready` at construction, but the FIRST read accessor a run touches flips
        /// it to `Failed` — `Ready` at the first checkpoint, `Failed` by the second.
        fn faults_on_first_read(inner: Arc<RdfDataset>) -> Self {
            Self {
                inner,
                pre_faulted: false,
                faulted_by_read: std::cell::Cell::new(false),
            }
        }

        fn mark_read(&self) {
            self.faulted_by_read.set(true);
        }
    }

    impl DatasetView for FlatProbeView {
        type Id = TermId;
        type ProbePlan = ();

        fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
            self.mark_read();
            self.inner.quads()
        }

        fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_>> + '_ {
            self.mark_read();
            DatasetView::quad_refs(&*self.inner)
        }

        fn resolve(&self, id: TermId) -> TermRef<'_> {
            self.inner.resolve(id)
        }

        fn term_id_by_value(&self, value: &TermValue) -> Option<TermId> {
            self.inner.term_id_by_value(value)
        }

        fn capabilities(&self) -> RdfStoreCapabilities {
            self.inner.capabilities()
        }

        fn probe_plan(&self, _s: bool, _p: bool, _o: bool, _g: GraphMatch) {}

        fn quads_for_pattern_with_plan(
            &self,
            _plan: &(),
            s: Option<TermId>,
            p: Option<TermId>,
            o: Option<TermId>,
            g: GraphMatch,
        ) -> impl Iterator<Item = QuadIds> + '_ {
            self.quads_for_pattern(s, p, o, g)
        }

        fn term_count(&self) -> usize {
            self.inner.term_count()
        }

        fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
            self.mark_read();
            self.inner.reifier_quads()
        }

        fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
            self.mark_read();
            self.inner.annotation_quads()
        }
    }

    impl FallibleDatasetView for FlatProbeView {
        type Error = FlatProbeFault;
        type Evidence = u32;

        fn operation_status(&self) -> ViewOperationStatus<FlatProbeFault, u32> {
            if self.pre_faulted || self.faulted_by_read.get() {
                ViewOperationStatus::Failed {
                    error: FlatProbeFault("the flat probe view faulted"),
                    evidence: 7,
                }
            } else {
                ViewOperationStatus::Ready { evidence: 0 }
            }
        }
    }

    /// A view already `Failed` at the FIRST checkpoint is refused before the run
    /// ever starts — the `Before` checkpoint's error and evidence are returned.
    #[test]
    fn try_canonicalize_flat_view_refuses_a_view_already_failed_before_the_drain() {
        let view = FlatProbeView::already_failed(presentation_fixture());
        match try_canonicalize_flat_view(&view, CanonHash::Sha256) {
            Err(ViewCanonError::NotReady {
                checkpoint,
                error,
                evidence,
            }) => {
                assert_eq!(checkpoint, DrainCheckpoint::Before);
                assert_eq!(error, FlatProbeFault("the flat probe view faulted"));
                assert_eq!(evidence, 7);
            }
            other => panic!("expected NotReady at the Before checkpoint; got {other:?}"),
        }
    }

    /// A view `Ready` at the first checkpoint but faulted by the second — the run
    /// faulted partway through — is refused with the `After` checkpoint's error and
    /// evidence, and NO `Canonicalized` escapes even though the run completed.
    #[test]
    fn try_canonicalize_flat_view_refuses_a_view_that_faults_mid_run_and_publishes_nothing() {
        let view = FlatProbeView::faults_on_first_read(presentation_fixture());
        match try_canonicalize_flat_view(&view, CanonHash::Sha256) {
            Err(ViewCanonError::NotReady {
                checkpoint,
                error,
                evidence,
            }) => {
                assert_eq!(checkpoint, DrainCheckpoint::After);
                assert_eq!(error, FlatProbeFault("the flat probe view faulted"));
                assert_eq!(evidence, 7);
            }
            other => panic!(
                "expected a NotReady refusal at the After checkpoint, with no \
                 Canonicalized escaping; got {other:?}"
            ),
        }
    }

    /// The same fault closure through `check_admissible_flat_view`, which drains
    /// the reserved-vocabulary sweep alone rather than the full canonical run.
    #[test]
    fn check_admissible_flat_view_refuses_a_view_that_faults_mid_run() {
        let view = FlatProbeView::faults_on_first_read(presentation_fixture());
        match check_admissible_flat_view(&view) {
            Err(ViewCanonError::NotReady { checkpoint, .. }) => {
                assert_eq!(checkpoint, DrainCheckpoint::After);
            }
            other => panic!("expected NotReady at the After checkpoint; got {other:?}"),
        }
    }
}
