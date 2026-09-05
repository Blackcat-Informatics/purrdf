// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! In-memory data model for the folded graph — mirror of
//! `src/purrdf_tools/gts/model.py`.
//!
//! A [`Term`] is a single RDF term carried by integer id (§7.1). The folded
//! [`Graph`] is the deterministic replay of the append-only frame log (§7.5):
//! terms, quads, reifiers, annotations, content-addressed blobs, metadata,
//! suppressions, opaque nodes, signatures, and reader diagnostics. `reifiers` and `meta`
//! are insertion-ordered maps
//! (Python `dict` semantics): re-binding an existing key replaces the value
//! but keeps the original position.

use std::borrow::Cow;
use std::slice;
use std::vec;

use ciborium::value::Value;

use crate::codec::{Codec, CodecError, decode_chain};

/// Well-known `xsd:string` datatype IRI used by the literal-defaulting rule (§7.1).
pub const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
/// Well-known `rdf:langString` datatype IRI implied by a language tag (§7.1).
pub const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
/// Well-known `rdf:dirLangString` datatype IRI implied by a base direction (§7.1).
pub const RDF_DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";

/// Return whether `direction` is a valid RDF 1.2 base direction token.
pub fn is_literal_direction(direction: &str) -> bool {
    matches!(direction, "ltr" | "rtl")
}

/// The kind of an RDF term, matching the wire `"k"` field (§7.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TermKind {
    /// An IRI (wire `"k"` = 0, and the default for unknown kinds).
    Iri = 0,
    /// A literal with lexical form, datatype, and optional language/direction.
    Literal = 1,
    /// A blank node with a scope-local label.
    Bnode = 2,
    /// An RDF 1.2 quoted triple term, optionally carrying a reifier.
    Triple = 3,
}

impl TermKind {
    /// Parse the wire `"k"` value; an unknown kind defaults to IRI (§7.1).
    pub fn from_wire(k: Option<i128>) -> Self {
        match k {
            Some(1) => Self::Literal,
            Some(2) => Self::Bnode,
            Some(3) => Self::Triple,
            _ => Self::Iri,
        }
    }
}

/// An RDF term identified by append-order id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    /// Wire term kind (`"k"`), with unknown or absent wire values folded as IRI.
    pub kind: TermKind,
    /// IRI string, literal lexical form, or blank-node label (scope-local).
    pub value: Option<String>,
    /// Term-id of the literal's datatype IRI, when explicit.
    pub datatype: Option<usize>,
    /// Literal language tag (BCP 47).
    pub lang: Option<String>,
    /// RDF 1.2 literal base direction (`"ltr"` or `"rtl"`) for language-tagged strings.
    pub direction: Option<String>,
    /// Term-id of the reifier of a quoted triple (`kind == Triple`).
    ///
    /// This is the ORIGINAL, indirect spelling of a quoted triple's components:
    /// the term names a reifier id and the `reifies` frame supplies that id's
    /// `(s, p, o)`. It is retained for files written that way, and is the
    /// fallback whenever [`Self::triple`] is absent.
    pub reifier: Option<usize>,
    /// The quoted triple's own `(subject, predicate, object)` term ids
    /// (`kind == Triple`), read from the wire `"tt"` field.
    ///
    /// `rdf:reifies` is NOT a functional property — one reifier id may bind
    /// several distinct triples — so routing a triple TERM's identity through a
    /// reifier id cannot express RDF 1.2. A self-describing triple term does:
    /// `tt` names the components directly and the `reifies` frame is left to be
    /// what RDF 1.2 says it is, a multi-valued statement layer. When present,
    /// this takes precedence over [`Self::reifier`].
    pub triple: Option<Triple3>,
}

impl Term {
    /// Apply `f` to every TERM ID this term carries, leaving value-bearing
    /// columns untouched.
    ///
    /// The id columns are `datatype`, `reifier`, and each component of `triple`.
    /// `kind`, `value`, `lang` and `direction` are values, not references into the
    /// term table, and are copied through unchanged.
    ///
    /// # Use this instead of a struct-update expression
    ///
    /// If you relocate terms — appending a frame into an open segment, merging
    /// term tables, compacting — shift ids with this rather than
    /// `Term { datatype: …, reifier: …, ..t.clone() }`. That form keeps compiling
    /// when a new id column is added and carries it through UNSHIFTED: a typed
    /// literal whose datatype id stayed frame-local resolves to whatever term
    /// happens to sit at that index, and a self-describing triple term resolves to
    /// three unrelated terms. Nothing fails; the bytes are just wrong.
    ///
    /// The body below destructures `Term` exhaustively, with no `..` rest pattern,
    /// so adding an id column is a compile error HERE — in the one place that
    /// knows whether the new column is an id — rather than a silent wrong answer
    /// at every call site.
    ///
    /// ```
    /// use purrdf_gts::model::{Term, TermKind};
    ///
    /// let t = Term {
    ///     kind: TermKind::Triple,
    ///     value: None,
    ///     datatype: Some(1),
    ///     lang: None,
    ///     direction: None,
    ///     reifier: Some(2),
    ///     triple: Some((3, 4, 5)),
    /// };
    /// let shifted = t.map_term_ids(|id| id + 10);
    /// assert_eq!(shifted.datatype, Some(11));
    /// assert_eq!(shifted.reifier, Some(12));
    /// assert_eq!(shifted.triple, Some((13, 14, 15)));
    /// ```
    #[must_use]
    pub fn map_term_ids(&self, f: impl Fn(usize) -> usize) -> Self {
        // EXHAUSTIVE on purpose — see the doc comment. Do not add `..`.
        let Self {
            kind,
            value,
            datatype,
            lang,
            direction,
            reifier,
            triple,
        } = self;
        Self {
            kind: *kind,
            value: value.clone(),
            datatype: datatype.map(&f),
            lang: lang.clone(),
            direction: direction.clone(),
            reifier: reifier.map(&f),
            triple: triple.map(|ids| map_triple_ids(ids, &f)),
        }
    }
}

/// A quad of term-ids; the graph slot is `None` for the default graph.
pub type Quad = (usize, usize, usize, Option<usize>);
/// A `(subject, predicate, object)` triple of term-ids.
pub type Triple3 = (usize, usize, usize);
/// A reifier row: `(reifier, (subject, predicate, object), graph?)`.
pub type ReifierRow = (usize, Triple3, Option<usize>);
/// An annotation row: `(reifier, predicate, value, graph?)`.
pub type AnnotationRow = (usize, usize, usize, Option<usize>);

// ── Relocating term ids ──────────────────────────────────────────────────────
//
// Term ids are SEGMENT-SCOPED (§7.5). Anything that relocates terms — appending
// a frame into an open segment, merging two term tables, compacting — has to
// shift every id-bearing column by the same base. Which columns those are is
// knowledge that belongs to this module, because this module is what adds them.
//
// Re-deriving that list at each call site is the hazard. A relocating remap is
// idiomatically written as a struct-update expression:
//
// ```ignore
// Term { datatype: t.datatype.map(shift), reifier: t.reifier.map(shift), ..t.clone() }
// ```
//
// which keeps compiling when a new id column arrives and carries it through
// UNSHIFTED — a typed literal whose datatype id stayed frame-local resolves to
// whatever term happens to sit at that index, and a self-describing triple term
// resolves to three unrelated terms. Silent, and green on every gate.
//
// The helpers below are the single enumeration. Each destructures its shape
// EXHAUSTIVELY, with no `..` rest pattern, so adding an id column to `Term` is a
// compile error here — in the one place that knows what to do about it — rather
// than a silent wrong answer at every consumer.

/// Apply `f` to every term id in a [`Quad`], graph slot included.
#[must_use]
pub fn map_quad_ids(quad: Quad, f: impl Fn(usize) -> usize) -> Quad {
    let (s, p, o, g) = quad;
    (f(s), f(p), f(o), g.map(&f))
}

/// Apply `f` to every term id in a [`Triple3`].
#[must_use]
pub fn map_triple_ids(triple: Triple3, f: impl Fn(usize) -> usize) -> Triple3 {
    let (s, p, o) = triple;
    (f(s), f(p), f(o))
}

/// Apply `f` to every term id in a [`ReifierRow`] — the reifier, its triple, and
/// the graph the declaration was asserted in.
#[must_use]
pub fn map_reifier_row_ids(row: ReifierRow, f: impl Fn(usize) -> usize) -> ReifierRow {
    let (reifier, triple, graph) = row;
    (f(reifier), map_triple_ids(triple, &f), graph.map(&f))
}

/// Apply `f` to every term id in an [`AnnotationRow`].
#[must_use]
pub fn map_annotation_row_ids(row: AnnotationRow, f: impl Fn(usize) -> usize) -> AnnotationRow {
    let (reifier, predicate, value, graph) = row;
    (f(reifier), f(predicate), f(value), graph.map(&f))
}

/// A quad with term ids resolved to borrowed [`Term`] values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuadTerms<'a> {
    /// Resolved subject term.
    pub subject: &'a Term,
    /// Resolved predicate term.
    pub predicate: &'a Term,
    /// Resolved object term.
    pub object: &'a Term,
    /// Resolved graph-name term, or `None` for the default graph.
    pub graph_name: Option<&'a Term>,
}

/// Borrowing iterator returned by [`Graph::quad_terms`].
#[derive(Debug)]
pub struct QuadTermsIter<'a> {
    graph: &'a Graph,
    inner: slice::Iter<'a, Quad>,
}

impl<'a> Iterator for QuadTermsIter<'a> {
    type Item = QuadTerms<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let &(subject, predicate, object, graph_name) = self.inner.next()?;
        Some(QuadTerms {
            subject: &self.graph.terms[subject],
            predicate: &self.graph.terms[predicate],
            object: &self.graph.terms[object],
            graph_name: graph_name.map(|tid| &self.graph.terms[tid]),
        })
    }
}

/// A frame the reader could not decode, surfaced rather than dropped (§7.6).
#[derive(Clone, Debug, PartialEq)]
pub struct OpaqueNode {
    /// Frame content id when available.
    pub id: Vec<u8>,
    /// Wire frame `"t"` value, or a placeholder when the frame is too damaged.
    pub frame_type: String,
    /// `"unknown-codec"` | `"missing-key"` | `"damaged"`.
    pub reason: String,
    /// `"none"` | `"valid"` | `"invalid"` | `"unverified"`.
    pub sigstat: String,
    /// Public frame metadata retained for diagnostics and policy decisions.
    pub pub_meta: Option<Value>,
    /// Recipient metadata rows from encrypted frames.
    pub recipients: Option<Vec<Value>>,
}

/// A recorded `suppress` directive (§11) — a display/precedence overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct Suppression {
    /// Target maps (`{"kind": "term"|"quad"|"reifier"|"frame"|"blob", ...}`).
    pub targets: Vec<Value>,
    /// Optional author-provided reason for suppression.
    pub reason: Option<String>,
    /// Optional term-id identifying the suppressing actor.
    pub by: Option<usize>,
}

/// A machine-observable reader diagnostic (§2.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable diagnostic code, matching the cross-engine diagnostic vocabulary.
    pub code: String,
    /// Human-readable detail string.
    pub detail: String,
    /// Absolute CBOR item index when the problem belongs to a frame.
    pub frame_index: Option<usize>,
}

/// The verification outcome for a signed frame (§9.2).
///
/// `cose` retains the raw COSE_Sign1 bytes so streamable compaction (§10.1)
/// can carry the signature *detached* — forever verifiable against
/// `frame_id` even after the frame itself is re-authored into a new chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    /// Frame id the COSE_Sign1 signature authenticates.
    pub frame_id: Vec<u8>,
    /// COSE key id, when present.
    pub kid: Option<String>,
    /// `"valid"` | `"invalid"` | `"unverified"`.
    pub status: String,
    /// Raw COSE_Sign1 bytes, retained for detached-signature transport.
    pub cose: Option<Vec<u8>>,
}

/// One segment's layout state (§3.3).
///
/// `covered`/`head` come from the segment's last intact `index` frame;
/// `tail` counts the legal unpresaged frames after it ("streamable through
/// frame *covered*, accretive tail of *tail* frame(s)"). For an unclaimed
/// (accretive) segment all fields are their zero values.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StreamableInfo {
    /// Whether the segment header explicitly claimed `layout = "streamable"`.
    pub claimed: bool,
    /// Number of frames covered by the last intact index footer.
    pub covered: usize,
    /// Number of legal accretive frames after the covered prefix.
    pub tail: usize,
    /// Head id declared by the last intact index footer.
    pub head: Option<Vec<u8>>,
}

/// Half-open byte range `[start, end)` into a GTS file, shared by the
/// replication inventory and other byte-provenance callers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ByteRange {
    /// Start byte offset.
    pub start: usize,
    /// End byte offset, exclusive.
    pub end: usize,
}

impl ByteRange {
    /// Number of bytes spanned by this range.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Whether this range spans zero bytes.
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

/// One content-addressed blob entry in a folded graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlobEntry {
    /// Decoded blob bytes.
    Bytes(Vec<u8>),
    /// Wire bytes plus the transform chain needed to decode them on access.
    ///
    /// Lazy entries preserve reader totality for transformed blob frames:
    /// callers that never ask for the bytes still get a complete folded graph,
    /// while callers that do ask receive the same codec errors the eager path
    /// would have produced.
    Lazy {
        /// Transformed wire bytes exactly as stored in the frame.
        raw: Vec<u8>,
        /// Transform chain to apply (in reverse) when the bytes are requested.
        chain: Vec<Codec>,
    },
}

impl BlobEntry {
    /// Store already-decoded blob bytes.
    pub fn bytes(data: Vec<u8>) -> Self {
        Self::Bytes(data)
    }

    /// Store transformed wire bytes for deferred decoding.
    pub fn lazy(raw: Vec<u8>, chain: Vec<Codec>) -> Self {
        Self::Lazy { raw, chain }
    }

    /// True when this entry still holds transformed wire bytes.
    pub fn is_lazy(&self) -> bool {
        matches!(self, Self::Lazy { .. })
    }

    /// Return cached decoded bytes without forcing a lazy decode.
    pub fn cached_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            Self::Lazy { .. } => None,
        }
    }

    /// Decode and cache this entry, returning the decoded bytes.
    pub fn decode(&mut self) -> Result<&[u8], CodecError> {
        if let Self::Lazy { raw, chain } = self {
            let decoded = decode_chain(chain, raw)?;
            *self = Self::Bytes(decoded);
        }
        match self {
            Self::Bytes(bytes) => Ok(bytes),
            Self::Lazy { .. } => unreachable!("lazy entry was decoded above"),
        }
    }

    /// Return decoded bytes without mutating the entry.
    pub fn decoded_bytes(&self) -> Result<Cow<'_, [u8]>, CodecError> {
        match self {
            Self::Bytes(bytes) => Ok(Cow::Borrowed(bytes)),
            Self::Lazy { raw, chain } => decode_chain(chain, raw).map(Cow::Owned),
        }
    }

    /// Return decoded bytes as an owned vector.
    pub fn decoded_vec(&self) -> Result<Vec<u8>, CodecError> {
        match self.decoded_bytes()? {
            Cow::Borrowed(bytes) => Ok(bytes.to_vec()),
            Cow::Owned(bytes) => Ok(bytes),
        }
    }

    /// Return the decoded byte length, decoding transiently if needed.
    pub fn decoded_len(&self) -> Result<usize, CodecError> {
        Ok(self.decoded_bytes()?.len())
    }
}

/// The folded result of a GTS log.
#[derive(Default, Debug)]
pub struct Graph {
    /// Terms in append/fold order. Quad ids index into this vector.
    pub terms: Vec<Term>,
    /// RDF quad rows using term ids; `None` graph slots mean default graph.
    pub quads: Vec<Quad>,
    /// Reifier rows, insertion-ordered; `None` graph slots mean default graph.
    pub reifiers: Vec<ReifierRow>,
    /// Annotation rows, insertion-ordered; `None` graph slots mean default graph.
    pub annotations: Vec<AnnotationRow>,
    /// `blake3:<hex>` digest → inline bytes, insertion-ordered.
    pub blobs: Vec<(String, BlobEntry)>,
    /// Declared blob metadata by digest — the blob frame's `"pub"` map
    /// (`mt`, `rep`, …) retained through the fold so tooling can list
    /// contents and assert media types without re-walking frames (§12).
    pub blob_meta: Vec<(String, Value)>,
    /// File-level shallow-merged metadata, insertion-ordered.
    pub meta: Vec<(String, Value)>,
    /// Suppression overlays, preserved in append/fold order.
    pub suppressions: Vec<Suppression>,
    /// Opaque frame records for unknown, encrypted, or damaged payloads.
    pub opaque: Vec<OpaqueNode>,
    /// Signature observations retained independently of verification policy.
    pub signatures: Vec<Signature>,
    /// Reader diagnostics collected during fold.
    pub diagnostics: Vec<Diagnostic>,
    /// Ordered per-segment head ids (§3.1) — the file's composite identity.
    pub segment_heads: Vec<Vec<u8>>,
    /// Per-segment header profiles; the effective requirement set is the
    /// union (§3.1, §13).
    pub segment_profiles: Vec<String>,
    /// Per-segment folded meta, preserved alongside the file-level merge.
    pub segment_meta: Vec<Vec<(String, Value)>>,
    /// Per-segment layout state (§3.3), in file order — the
    /// declared-vs-computed streamable claim, its covered boundary, and the
    /// accretive tail.
    pub segment_streamable: Vec<StreamableInfo>,
}

impl Graph {
    /// Consume the graph and yield its raw quad-id rows without cloning them.
    ///
    /// Existing direct access through [`Self::quads`] remains available for
    /// callers that need the full folded graph.
    pub fn into_quads(self) -> vec::IntoIter<Quad> {
        self.quads.into_iter()
    }

    /// Iterate over quads with term ids resolved to borrowed [`Term`] values.
    ///
    /// Resolution happens one row at a time. Like existing index-based access,
    /// this panics if a manually constructed graph contains invalid term ids;
    /// reader-produced graphs sanitize quad term ids during the fold.
    pub fn quad_terms(&self) -> QuadTermsIter<'_> {
        QuadTermsIter {
            graph: self,
            inner: self.quads.iter(),
        }
    }

    /// Look up the FIRST triple a reifier id binds.
    ///
    /// `rdf:reifies` is not functional, so a reifier id may bind several
    /// triples; use [`Self::reifier_bindings`] when every binding matters. This
    /// accessor exists for the legacy, indirect triple-term spelling
    /// ([`Term::reifier`]), which can only name one.
    pub fn reifier(&self, rid: usize) -> Option<Triple3> {
        self.reifiers
            .iter()
            .find(|(r, _, _)| *r == rid)
            .map(|(_, spo, _)| *spo)
    }

    /// Every `(triple, graph?)` binding recorded for one reifier id, in fold
    /// order.
    pub fn reifier_bindings(
        &self,
        rid: usize,
    ) -> impl Iterator<Item = (Triple3, Option<usize>)> + '_ {
        self.reifiers
            .iter()
            .filter(move |(r, _, _)| *r == rid)
            .map(|&(_, spo, g)| (spo, g))
    }

    /// How many distinct triples one reifier id binds.
    pub fn reifier_binding_count(&self, rid: usize) -> usize {
        let mut seen: Vec<Triple3> = Vec::new();
        for (spo, _) in self.reifier_bindings(rid) {
            if !seen.contains(&spo) {
                seen.push(spo);
            }
        }
        seen.len()
    }

    /// The `(s, p, o)` of a quoted-triple term.
    ///
    /// Prefers the term's own self-describing [`Term::triple`] (wire `"tt"`);
    /// falls back to the legacy indirection through [`Term::reifier`] for terms
    /// written before the triple term carried its own components. This is the
    /// ONE place a quoted triple's components are resolved from a folded graph.
    ///
    /// # Termination
    ///
    /// Resolving a component of the returned triple, and so on recursively, is
    /// what every consumer of a quoted triple does — the segment union, the
    /// canonical writer, every projection to text. That walk terminates on a
    /// reader-produced graph and MUST NOT be assumed to on any other: `tt` and
    /// `dt` ids are sanitized to name strictly earlier terms, and the fold
    /// REFUSES a `reifies` row that would let a triple term reach itself. A
    /// [`Graph`] assembled by hand carries no such guarantee, and an unguarded
    /// walk over a self-reaching one blows the stack, which aborts rather than
    /// panics. Callers that accept caller-built graphs must bound their own
    /// recursion.
    pub fn triple_of(&self, term_id: usize) -> Option<Triple3> {
        let term = self.terms.get(term_id)?;
        // A self-bound triple term may leave `rf` implicit, in which case the
        // binding is keyed by the term's own id (§7.1).
        self.term_triple(term).or_else(|| {
            (term.kind == TermKind::Triple && term.reifier.is_none())
                .then(|| self.reifier(term_id))
                .flatten()
        })
    }

    /// [`Self::triple_of`] for an already-borrowed [`Term`].
    pub fn term_triple(&self, term: &Term) -> Option<Triple3> {
        if term.kind != TermKind::Triple {
            return None;
        }
        term.triple
            .or_else(|| term.reifier.and_then(|rid| self.reifier(rid)))
    }

    /// Record a reifier row unless the identical row is already present.
    pub fn set_reifier(&mut self, rid: usize, spo: Triple3, graph_name: Option<usize>) {
        if !self
            .reifiers
            .iter()
            .any(|&(r, existing, g)| r == rid && existing == spo && g == graph_name)
        {
            self.reifiers.push((rid, spo, graph_name));
        }
    }

    /// Set a meta key, replacing in place (Python dict assignment).
    pub fn set_meta(&mut self, key: String, value: Value) {
        if let Some(slot) = self.meta.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = value;
        } else {
            self.meta.push((key, value));
        }
    }

    /// Record a blob's declared metadata, replacing in place.
    pub fn set_blob_meta(&mut self, digest: String, meta: Value) {
        if let Some(slot) = self.blob_meta.iter_mut().find(|(d, _)| *d == digest) {
            slot.1 = meta;
        } else {
            self.blob_meta.push((digest, meta));
        }
    }

    /// Store a blob entry under its digest, replacing in place.
    pub fn set_blob_entry(&mut self, digest: String, entry: BlobEntry) {
        if let Some(slot) = self.blobs.iter_mut().find(|(d, _)| *d == digest) {
            slot.1 = entry;
        } else {
            self.blobs.push((digest, entry));
        }
    }

    /// Store decoded inline blob bytes under their digest, replacing in place.
    pub fn set_blob(&mut self, digest: String, data: Vec<u8>) {
        self.set_blob_entry(digest, BlobEntry::bytes(data));
    }

    /// Store transformed inline blob bytes for lazy decoding.
    pub fn set_lazy_blob(&mut self, digest: String, raw: Vec<u8>, chain: Vec<Codec>) {
        self.set_blob_entry(digest, BlobEntry::lazy(raw, chain));
    }

    /// Look up a blob entry without decoding it.
    pub fn blob_entry(&self, digest: &str) -> Option<&BlobEntry> {
        self.blobs
            .iter()
            .find(|(d, _)| d == digest)
            .map(|(_, entry)| entry)
    }

    /// Look up a blob and decode/cache it on demand.
    pub fn blob_bytes(&mut self, digest: &str) -> Result<Option<&[u8]>, CodecError> {
        match self.blobs.iter_mut().find(|(d, _)| d == digest) {
            Some((_, entry)) => entry.decode().map(Some),
            None => Ok(None),
        }
    }

    /// Look up a blob and return decoded owned bytes.
    pub fn blob_bytes_cloned(&mut self, digest: &str) -> Result<Option<Vec<u8>>, CodecError> {
        Ok(self.blob_bytes(digest)?.map(<[u8]>::to_vec))
    }

    /// Return every blob as decoded owned bytes, preserving insertion order.
    pub fn decoded_blobs(&mut self) -> Result<Vec<(String, Vec<u8>)>, CodecError> {
        let mut out = Vec::with_capacity(self.blobs.len());
        for (digest, entry) in &mut self.blobs {
            out.push((digest.clone(), entry.decode()?.to_vec()));
        }
        Ok(out)
    }

    /// The effective datatype IRI of a literal, applying §7.1 defaulting.
    ///
    /// The fold sanitizes `datatype` ids, but `Graph` is constructible by
    /// callers — an out-of-range id falls back to `xsd:string`, never panics.
    pub fn datatype_iri(&self, t: &Term) -> String {
        self.datatype_iri_str(t).to_string()
    }

    /// Borrowing form of [`Self::datatype_iri`]: the same §7.1 defaulting, but
    /// the IRI is returned as a slice of the datatype term (or a well-known
    /// constant) so read-only callers such as the canonical term-identity walk
    /// pay no per-literal `String` allocation.
    pub(crate) fn datatype_iri_str(&self, t: &Term) -> &str {
        if let Some(dt) = t.datatype {
            return self
                .terms
                .get(dt)
                .and_then(|term| term.value.as_deref())
                .unwrap_or(XSD_STRING);
        }
        if t.lang.is_some()
            && matches!(t.direction.as_deref(), Some(direction) if is_literal_direction(direction))
        {
            RDF_DIR_LANG_STRING
        } else if t.lang.is_some() {
            RDF_LANG_STRING
        } else {
            XSD_STRING
        }
    }
}

impl IntoIterator for Graph {
    type Item = Quad;
    type IntoIter = vec::IntoIter<Quad>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_quads()
    }
}

#[cfg(test)]
mod term_id_remap_tests {
    //! Term ids are segment-scoped, so relocating terms has to shift EVERY
    //! id-bearing column. These pin which columns those are, from the outside.

    use super::{
        AnnotationRow, Quad, ReifierRow, Term, TermKind, Triple3, map_annotation_row_ids,
        map_quad_ids, map_reifier_row_ids, map_triple_ids,
    };

    /// A term carrying every id column at once, so a dropped column is visible.
    fn loaded() -> Term {
        Term {
            kind: TermKind::Triple,
            value: Some("lex".to_owned()),
            datatype: Some(1),
            lang: Some("en".to_owned()),
            direction: Some("ltr".to_owned()),
            reifier: Some(2),
            triple: Some((3, 4, 5)),
        }
    }

    #[test]
    fn every_id_column_shifts_and_no_value_column_does() {
        let shifted = loaded().map_term_ids(|id| id + 10);

        assert_eq!(shifted.datatype, Some(11));
        assert_eq!(shifted.reifier, Some(12));
        // `triple` is the column a second, hand-written copy of this enumeration
        // in this crate had silently dropped — it set `triple: None` — which is
        // the whole reason the remap lives on the type now.
        assert_eq!(shifted.triple, Some((13, 14, 15)));

        // Values are not references into the term table and must NOT move.
        let original = loaded();
        assert_eq!(shifted.kind, original.kind);
        assert_eq!(shifted.value, original.value);
        assert_eq!(shifted.lang, original.lang);
        assert_eq!(shifted.direction, original.direction);
    }

    #[test]
    fn a_term_with_no_ids_is_carried_through_unchanged() {
        // THE NEIGHBOURING CASE. A plain IRI term has no id columns at all, and a
        // remap that touched it — or refused it — would corrupt the common case
        // while the loaded one above still passed.
        let plain = Term {
            kind: TermKind::Iri,
            value: Some("http://example.org/s".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        };
        let shifted = plain.map_term_ids(|id| id + 10);
        assert_eq!(shifted, plain, "no id columns means nothing to shift");
    }

    #[test]
    fn the_identity_remap_is_the_identity() {
        // Guards against a helper that shifts something twice, or shifts a value
        // column by coincidence of type: with `f = identity` the term must be
        // byte-equal however many columns it carries.
        let t = loaded();
        assert_eq!(t.map_term_ids(|id| id), t);
    }

    #[test]
    fn the_tuple_shapes_shift_every_slot_including_the_graph() {
        // The graph slot IS a term id — the statement layer is keyed per graph, so
        // a relocation that left it frame-local would move rows into the wrong
        // named graph rather than dangling, which is the quieter failure.
        let quad: Quad = (1, 2, 3, Some(4));
        assert_eq!(map_quad_ids(quad, |id| id + 10), (11, 12, 13, Some(14)));
        assert_eq!(
            map_quad_ids((1, 2, 3, None), |id| id + 10),
            (11, 12, 13, None)
        );

        let triple: Triple3 = (1, 2, 3);
        assert_eq!(map_triple_ids(triple, |id| id + 10), (11, 12, 13));

        let reifier: ReifierRow = (1, (2, 3, 4), Some(5));
        assert_eq!(
            map_reifier_row_ids(reifier, |id| id + 10),
            (11, (12, 13, 14), Some(15))
        );

        let annotation: AnnotationRow = (1, 2, 3, Some(4));
        assert_eq!(
            map_annotation_row_ids(annotation, |id| id + 10),
            (11, 12, 13, Some(14))
        );
    }
}
