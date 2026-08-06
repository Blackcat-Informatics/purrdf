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
//! [`ReservedVocabulary`]. Refusal is chosen over injective escaping because the
//! property a consumer has to audit ("these bytes cannot be forged") is then a
//! single total rule over the input rather than a proof about an escaping function.
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

use super::dataset::{RdfDataset, TermRef};
use super::skolem::{TermMapper, rebuild_dataset};
use super::term::{BlankScope, TermId};

/// `xsd:string` — the implicit datatype that N-Quads writes bare (no `^^<…>`).
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// The IRI namespace the RDF 1.2 overlay lowers into, reserved by this profile.
///
/// **No term of an input dataset may be an IRI in this namespace, in any position.**
/// A dataset that carries one is refused with [`ReservedVocabulary`]; see
/// [`CANON_PROFILE_ID`] for why refusal rather than convention is the contract.
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
    "038f7431e845e63c8bb2122cdfa2c9968f40c17ae7cb6b9e458bbb5cb11375b7";

/// The version of [`CANON_PROFILE_ID`] this build implements.
///
/// Incremented by any change to the canonical bytes a given dataset produces, to
/// the reserved vocabulary, to the refusal rule, or to the bounds — i.e. by any
/// change that could move a consumer's minted identity. A change that cannot move
/// output (a refactor, a faster search, a clearer diagnostic) does NOT increment
/// it, which is what makes the number worth pinning.
pub const CANON_PROFILE_VERSION: u32 = 1;

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

/// The result of canonicalizing a dataset.
#[derive(Clone, Debug)]
pub struct Canonicalized {
    /// The canonical N-Quads document: every line `'\n'`-terminated, the set of
    /// lines sorted bytewise ascending and deduplicated. Blanks render as their
    /// canonical `_:c14nN` label. Includes the reified/annotated overlay (via the
    /// reserved `urn:purrdf:rdfc:` sentinels).
    pub nquads: String,
    /// Each blank [`TermId`] mapped to its canonical label (`"c14n0"`, …) WITHOUT
    /// the leading `_:`.
    ///
    /// This map is the PRINCIPLED blank-label assignment: the labels are issued
    /// purely from graph structure, so they are isomorphism-invariant (two
    /// isomorphic datasets assign corresponding blanks the same label), and their
    /// alphabet is ASCII alphanumerics only — legal under every constrained
    /// egress alphabet (`BLANK_NODE_LABEL`, XML `NCName`; see
    /// [`crate::blank_label`]). [`canonical_relabel`] applies it as a dataset
    /// rewrite.
    pub labels: BTreeMap<TermId, Box<str>>,
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
    CanonState::new(ds, hash).run()
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
    CanonState::new(ds, hash).run_fallible()
}

/// Relabel every blank node of `ds` to its canonical `c14n{n}` label at
/// [`BlankScope::DEFAULT`], returning a NEW frozen dataset with all other
/// terms, quads, reifiers, annotations, named-graph declarations, and quad
/// source locations preserved.
///
/// This is the "make any dataset serializable in every alphabet" recourse: the
/// serializers hard-fail on a blank label that is illegal in the target
/// syntax's alphabet (see [`crate::blank_label`]) rather than relabel silently,
/// and this operation is the caller-invoked, principled fix. The `c14n{n}`
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
/// `predecessors`/`predecessor_chain`) reset on the output: a frozen dataset
/// does not expose its `ContentIdScheme`, so the rewrite cannot re-establish
/// content addressing; callers that need those indexes rebuild them with
/// their own scheme configuration.
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
    let canonical = try_canonicalize(ds)?;
    // Declaration-only blank graphs are invisible to canonicalization (they own
    // no statement), so continue the canonical numbering over them in a
    // value-deterministic order.
    let mut unseen: Vec<(&str, BlankScope, TermId)> = ds
        .named_graphs()
        .filter(|g| !canonical.labels.contains_key(g))
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
            let label = format!("{CANON_PREFIX}{}", canonical.labels.len() + i);
            (id, label.into_boxed_str())
        })
        .collect();
    rebuild_dataset(
        ds,
        &mut CanonicalRelabeler {
            labels: &canonical.labels,
            extra,
        },
    )
}

/// The [`canonical_relabel`] mapper: blanks take their issued `c14n{n}` label
/// at [`BlankScope::DEFAULT`]; every other term passes through unchanged.
struct CanonicalRelabeler<'a> {
    /// The canonicalization's issued labels ([`Canonicalized::labels`]).
    labels: &'a BTreeMap<TermId, Box<str>>,
    /// Continuation labels for declaration-only blank graphs.
    extra: BTreeMap<TermId, Box<str>>,
}

impl TermMapper for CanonicalRelabeler<'_> {
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
}

/// Whether `ds` is admissible to canonicalization under profile
/// [`CANON_PROFILE_ID`] — i.e. carries no IRI in [`RESERVED_NAMESPACE`].
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
    reserved_vocabulary(ds).map_or(Ok(()), Err)
}

/// The count of distinct blank nodes in `ds` (incl. blanks nested inside triple
/// terms). A cheap structural pre-reject used by [`super::compare`].
#[must_use]
pub fn blank_count(ds: &RdfDataset) -> usize {
    let mut set: BTreeSet<TermId> = BTreeSet::new();
    collect_components(ds, &mut |comp| {
        comp.for_each_blank(ds, &mut |b| {
            set.insert(b);
        });
    });
    set.len()
}

/// A statement normalized to a quad shape for uniform hashing and serialization.
/// Predicate/graph slots may be a reserved sentinel IRI (the overlay rows).
#[derive(Clone, Copy)]
enum Component {
    /// A genuine dataset quad.
    Quad {
        s: TermId,
        p: TermId,
        o: TermId,
        g: Option<TermId>,
    },
    /// A reifier binding `r <urn:purrdf:rdfc:reifies> t` in graph `g` (`None` =
    /// default graph — the graph slot then stays empty, byte-identical to the
    /// pre-graph-dimension form).
    Reifier {
        r: TermId,
        t: TermId,
        g: Option<TermId>,
    },
    /// An annotation `r p o` in the reserved annotation graph, itself scoped to graph
    /// `g` (`None` = default graph).
    Annotation {
        r: TermId,
        p: TermId,
        o: TermId,
        g: Option<TermId>,
    },
}

/// One quad slot: a dataset term, a synthetic sentinel IRI (overlay predicate) that
/// has no [`TermId`], or the annotation-overlay graph marker (the reserved annotation
/// sentinel plus the annotation's own named graph, if any).
#[derive(Clone, Copy)]
enum Slot {
    Term(TermId),
    Sentinel(&'static str),
    /// The annotation overlay's graph position: the reserved annotation sentinel and,
    /// for a named-graph annotation, the graph term. `None` renders exactly as the
    /// bare sentinel (byte-identical to the default-graph form); `Some(g)` appends the
    /// real graph term so a named-graph annotation stays lossless and distinct from a
    /// genuine quad. The graph term keeps its [`TermId`] so a blank-node graph still
    /// participates in canonical labeling.
    AnnotationGraph(Option<TermId>),
}

impl Component {
    /// The four quad slots `(s, p, o, g)` of this component in canonical shape.
    fn slots(self) -> (Slot, Slot, Slot, Option<Slot>) {
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
        }
    }

    /// Invoke `f` for every blank [`TermId`] appearing anywhere in this component
    /// (recursing into triple terms).
    fn for_each_blank(self, ds: &RdfDataset, f: &mut impl FnMut(TermId)) {
        let (s, p, o, g) = self.slots();
        for slot in [Some(s), Some(p), Some(o), g].into_iter().flatten() {
            match slot {
                Slot::Term(id) | Slot::AnnotationGraph(Some(id)) => blanks_in_term(ds, id, f),
                Slot::Sentinel(_) | Slot::AnnotationGraph(None) => {}
            }
        }
    }
}

/// Invoke `f` for every blank [`TermId`] reachable at `id` (recursing triple terms).
fn blanks_in_term(ds: &RdfDataset, id: TermId, f: &mut impl FnMut(TermId)) {
    match ds.resolve(id) {
        TermRef::Blank { .. } => f(id),
        TermRef::Triple { s, p, o } => {
            blanks_in_term(ds, s, f);
            blanks_in_term(ds, p, f);
            blanks_in_term(ds, o, f);
        }
        _ => {}
    }
}

/// Drive `f` over every [`Component`] of the dataset (quads, reifiers, annotations).
fn collect_components(ds: &RdfDataset, f: &mut impl FnMut(Component)) {
    for q in ds.quads() {
        f(Component::Quad {
            s: q.s,
            p: q.p,
            o: q.o,
            g: q.g,
        });
    }
    for (r, t, g) in ds.reifiers_with_graph() {
        f(Component::Reifier { r, t, g });
    }
    for (r, p, o, g) in ds.annotations_with_graph() {
        f(Component::Annotation { r, p, o, g });
    }
}

/// How a blank renders during serialization.
#[derive(Clone, Copy)]
enum BlankRender<'a> {
    /// Hash First Degree Quads (§4.6): the focus blank → `_:a`, every other → `_:z`.
    FirstDegree { focus: TermId },
    /// Final output (§4.4 step 7): each blank → its issued `_:c14nN` label.
    Canonical { issuer: &'a IdIssuer },
}

impl BlankRender<'_> {
    /// The `_:`-less label a blank renders to under this strategy.
    fn label(&self, id: TermId) -> String {
        match self {
            BlankRender::FirstDegree { focus } => {
                if id == *focus {
                    "a".to_owned()
                } else {
                    "z".to_owned()
                }
            }
            BlankRender::Canonical { issuer } => issuer
                .issued_for(id)
                .expect("every blank has a canonical id at output time")
                .to_owned(),
        }
    }
}

/// The RDFC-1.0 "identifier issuer": mints prefixed ids (`c14n0`, `b0`, …) in a
/// stable order, remembering each blank's id and the issuance order.
#[derive(Clone)]
struct IdIssuer {
    prefix: &'static str,
    issued: BTreeMap<TermId, Box<str>>,
    order: Vec<TermId>,
}

impl IdIssuer {
    fn new(prefix: &'static str) -> Self {
        Self {
            prefix,
            issued: BTreeMap::new(),
            order: Vec::new(),
        }
    }

    /// Issue (or return the already-issued) id for `b`.
    fn issue(&mut self, b: TermId) -> &str {
        if !self.issued.contains_key(&b) {
            let id = format!("{}{}", self.prefix, self.order.len()).into_boxed_str();
            self.issued.insert(b, id);
            self.order.push(b);
        }
        self.issued.get(&b).expect("just inserted")
    }

    fn issued_for(&self, b: TermId) -> Option<&str> {
        self.issued.get(&b).map(Box::as_ref)
    }

    fn has(&self, b: TermId) -> bool {
        self.issued.contains_key(&b)
    }

    /// The blanks in issuance order.
    fn order(&self) -> &[TermId] {
        &self.order
    }
}

/// Per-dataset canonicalization state.
struct CanonState<'a> {
    ds: &'a RdfDataset,
    /// Every blank, in ascending [`TermId`] order (the deterministic reference set).
    blanks: Vec<TermId>,
    /// The components each blank participates in (its "quads", RDFC-1.0 §4.4).
    incident: BTreeMap<TermId, Vec<Component>>,
    /// First-degree hash (§4.6) of each blank, computed once.
    first_degree: BTreeMap<TermId, HashHex>,
    /// The durable canonical issuer.
    canonical: IdIssuer,
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

/// The input dataset carries an IRI in the profile's [`RESERVED_NAMESPACE`], which
/// canonicalization refuses rather than lower alongside its own sentinels.
///
/// Accepting such a dataset would let a genuine reifier/annotation structure and a
/// literal assertion of its lowered form canonicalize to identical bytes — an
/// identity collision, and for a content-addressed store an identity-forgery
/// primitive. See the module documentation for why the rule is refusal at the
/// namespace rather than escaping at the two sentinel spellings.
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
fn reserved_in_term(ds: &RdfDataset, id: TermId) -> Option<Box<str>> {
    match ds.resolve(id) {
        TermRef::Iri(iri) => iri.starts_with(RESERVED_NAMESPACE).then(|| Box::from(iri)),
        TermRef::Literal { datatype, .. } => reserved_in_term(ds, datatype),
        TermRef::Triple { s, p, o } => reserved_in_term(ds, s)
            .or_else(|| reserved_in_term(ds, p))
            .or_else(|| reserved_in_term(ds, o)),
        TermRef::Blank { .. } => None,
    }
}

/// The dataset's reserved-namespace violation, or `None` if it is admissible.
///
/// Returns the LEAST `(position, iri)` rather than the first one encountered. The
/// difference only shows on a dataset carrying several violations — which is already
/// refused either way — but "first encountered" would mean statement order, and
/// statement order is interning order, which differs between backends holding the
/// same dataset. The refusal was always total; this makes the DIAGNOSTIC total too,
/// so a corpus can pin the reported position and a consumer comparing two
/// implementations' rejections is comparing something well defined.
fn reserved_vocabulary(ds: &RdfDataset) -> Option<ReservedVocabulary> {
    let mut worst: Option<ReservedVocabulary> = None;
    collect_components(ds, &mut |comp| {
        let (s, p, o, g) = comp.slots();
        for (slot, position) in [
            (Some(s), TermPosition::Subject),
            (Some(p), TermPosition::Predicate),
            (Some(o), TermPosition::Object),
            (g, TermPosition::Graph),
        ] {
            // `Slot::Sentinel` is the overlay's OWN lowering, not caller input, and
            // `AnnotationGraph(None)` carries no term at all — neither is a violation.
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

impl<'a> CanonState<'a> {
    fn new(ds: &'a RdfDataset, hash: CanonHash) -> Self {
        let mut blank_set: BTreeSet<TermId> = BTreeSet::new();
        let mut incident: BTreeMap<TermId, Vec<Component>> = BTreeMap::new();
        collect_components(ds, &mut |comp| {
            // Record incidence for each distinct blank in the component (a blank that
            // appears in two positions of one quad still lists that quad once).
            let mut seen: BTreeSet<TermId> = BTreeSet::new();
            comp.for_each_blank(ds, &mut |b| {
                blank_set.insert(b);
                if seen.insert(b) {
                    incident.entry(b).or_default().push(comp);
                }
            });
        });
        let blanks: Vec<TermId> = blank_set.into_iter().collect();
        Self {
            ds,
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
    fn run(self) -> Canonicalized {
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
    fn run_fallible(mut self) -> Result<Canonicalized, CanonError> {
        if let Some(violation) = reserved_vocabulary(self.ds) {
            return Err(CanonError::ReservedVocabulary(violation));
        }
        let blank_count = self.blanks.len();
        match self.run_inner() {
            Ok(()) => {}
            Err(Exhausted) => {
                return Err(CanonError::BudgetExceeded(BudgetExceeded { blank_count }));
            }
        }
        let nquads = self.serialize_canonical();
        let labels = self
            .canonical
            .issued
            .iter()
            .map(|(&id, label)| (id, label.clone()))
            .collect();
        Ok(Canonicalized { nquads, labels })
    }

    fn run_inner(&mut self) -> Result<(), Exhausted> {
        // §4.4 step 3: first-degree hash of every blank, grouped by hash.
        let mut by_hash: BTreeMap<HashHex, Vec<TermId>> = BTreeMap::new();
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
            let group = by_hash.get(&h).expect("ambiguous hash present").clone();
            // 5.2–5.3: for each not-yet-canonical blank, run hashNDegreeQuads against a
            // fresh temporary issuer seeded with that blank.
            let mut hash_paths: Vec<(HashHex, IdIssuer)> = Vec::new();
            for b in group {
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
    fn hash_first_degree(&self, b: TermId) -> HashHex {
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
        identifier: TermId,
        mut issuer: IdIssuer,
    ) -> Result<(HashHex, IdIssuer), Exhausted> {
        self.budget = self.budget.checked_sub(1).ok_or(Exhausted)?;

        // §4.8 step 3: map related-blank hash → the related blanks bearing it.
        let mut hn: BTreeMap<HashHex, Vec<TermId>> = BTreeMap::new();
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
            let mut chosen_issuer: Option<IdIssuer> = None;

            // §4.8 step 5.4: every permutation of the related list, identity first.
            for perm in permutations(related_list) {
                // Charge the poison budget PER PERMUTATION: a related group of size k
                // contributes k! permutations, so this — not the recursive-call count —
                // is the dominant cost on a pathologically symmetric graph (e.g. a
                // 10-blank clique). Counting it here bounds the actual work.
                self.budget = self.budget.checked_sub(1).ok_or(Exhausted)?;
                let mut issuer_copy = issuer.clone();
                let mut path = String::new();
                let mut recursion: Vec<TermId> = Vec::new();
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
        comp: Component,
        focus: TermId,
        issuer: &IdIssuer,
        f: &mut impl FnMut(TermId, HashHex),
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
        slot: Slot,
        position: &str,
        predicate: &Slot,
        focus: TermId,
        issuer: &IdIssuer,
        f: &mut impl FnMut(TermId, HashHex),
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
            _ => {}
        }
    }

    /// Hash Related Blank Node (RDFC-1.0 §4.7).
    fn hash_related_blank_node(
        &self,
        related: TermId,
        position: &str,
        predicate: &Slot,
        issuer: &IdIssuer,
    ) -> HashHex {
        let mut input = String::new();
        input.push_str(position);
        if position != "g" && !position.starts_with("g.") {
            input.push('<');
            input.push_str(&self.predicate_iri(predicate));
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

    /// The IRI value of a predicate slot (a real IRI term or a sentinel).
    fn predicate_iri(&self, predicate: &Slot) -> String {
        match predicate {
            Slot::Sentinel(iri) => (*iri).to_owned(),
            Slot::Term(id) => match self.ds.resolve(*id) {
                TermRef::Iri(iri) => iri.to_owned(),
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
        collect_components(self.ds, &mut |comp| {
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
    fn write_component(&self, comp: Component, render: BlankRender<'_>, out: &mut String) {
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

    fn write_slot(&self, slot: Slot, render: BlankRender<'_>, out: &mut String) {
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
    /// language / direction are emitted **verbatim** (never normalized).
    fn write_term(&self, id: TermId, render: BlankRender<'_>, out: &mut String) {
        match self.ds.resolve(id) {
            TermRef::Iri(iri) => {
                out.push('<');
                write_iri_escaped(iri, out);
                out.push('>');
            }
            TermRef::Blank { .. } => {
                out.push_str("_:");
                out.push_str(&render.label(id));
            }
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                out.push('"');
                write_literal_escaped(lexical, out);
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
    use crate::ir::RdfDatasetBuilder;
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
    // Reserved vocabulary: the overlay's sentinels cannot be forged from input
    // -----------------------------------------------------------------------

    /// The attack the refusal rule exists to stop, built end to end.
    ///
    /// Dataset A carries a genuine reifier, which the overlay lowers to a row spelled
    /// `r <urn:purrdf:rdfc:reifies> <<(…)>>`. Dataset B carries no reifier at all —
    /// it simply ASSERTS that row as an ordinary quad. Before the refusal rule the two
    /// canonicalized to identical bytes, so a consumer minting identity from those
    /// bytes would give a structurally different dataset the same identity.
    ///
    /// The assertion is deliberately two-sided. It is not enough that B is refused:
    /// the test also builds A's canonical bytes and confirms they are exactly the ones
    /// B would have had to produce, so it fails if the lowering is ever changed in a
    /// way that makes the fixture stop reproducing the collision — a test that passed
    /// because it stopped testing anything would be worse than no test.
    #[test]
    fn a_literally_asserted_sentinel_row_cannot_forge_a_reifier_structure() {
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
        let forged = b.freeze().expect("valid");

        // The forgery is refused, and refused for BEING a forgery attempt.
        match try_canonicalize(&forged) {
            Err(CanonError::ReservedVocabulary(err)) => {
                assert_eq!(&*err.iri, SENTINEL_REIFIES);
                assert_eq!(err.position, TermPosition::Predicate);
            }
            other => panic!("the forged dataset must be refused; got {other:?}"),
        }

        // And the collision was real: had it not been refused, these are the bytes it
        // would have produced — byte-identical to the genuine structure's.
        assert!(
            lowered.contains("<urn:purrdf:rdfc:reifies>") && !lowered.is_empty(),
            "genuine lowering: {lowered}"
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
            let mut b = RdfDatasetBuilder::new();
            let (s, pred, o, g) = (
                iri(&mut b, "s"),
                iri(&mut b, "p"),
                iri(&mut b, "o"),
                iri(&mut b, "g"),
            );
            let bad = b.intern_iri(sentinel);
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
                    assert_eq!(&*err.iri, sentinel);
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

    /// The profile identity a consumer pins is readable from the API, and the reserved
    /// namespace really is the prefix of the sentinels the overlay lowers into — the
    /// one relationship the whole refusal argument rests on.
    #[test]
    fn the_profile_identity_and_the_reserved_namespace_are_consistent() {
        assert_eq!(CANON_PROFILE_ID, "purrdf-rdfc12");
        assert_eq!(CANON_PROFILE_VERSION, 1);
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
        use crate::blank_label::{LabelAlphabet, is_valid_label};
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
                LabelAlphabet::Unconstrained,
            ] {
                assert!(
                    is_valid_label(label, alphabet),
                    "{label:?} must be legal under {alphabet:?}"
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
}
