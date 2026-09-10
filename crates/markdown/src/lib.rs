// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf-markdown` — a Markdown document becomes an RDF 1.2 graph along
//! its own structure.
//!
//! The whole law this crate applies is written out, clause by clause, in
//! the specification that ships beside it:
//! [`crates/markdown/SPEC.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/markdown/SPEC.md).
//! What follows is that law in summary.
//!
//! # Stand-off first, claims second
//!
//! The slicer is a stand-off markup engine: the document's bytes are
//! never touched, and everything the crate knows is a typed annotation
//! over a **range** of them. [`analyze`] returns that layer as a
//! [`Document`] — sections, units, citations, and the rows that lifted
//! nothing — and [`render`] projects it into claims. [`slice_markdown`]
//! is the two of them in a row, and remains the whole surface a caller
//! who only wants triples needs.
//!
//! The order is the point. The model is the finding; the graph is one
//! view of it. A consumer that wants to ask which unit covers byte
//! 4,821 ([`Document::unit_at`]), which units a byte range touches
//! ([`Document::covering`]), how two spans lie against each other
//! ([`span_relation`], the thirteen Allen interval relations), what a
//! unit's exact quote and surrounding context are ([`Unit::anchor`]),
//! or which concordance rows named verses this document does not carry
//! ([`Document::unmatched_citations`]) asks the model, and never parses
//! a triple to do it.
//!
//! # The dialect
//!
//! One document node; one section node per ATX heading (`#` through
//! `######`) and per movement marker (`⁂ *name*`, U+2042, which sits one
//! level under the nearest heading so consecutive movements are
//! siblings); and one unit node per numbered verse (`12. text`) or
//! blank-line paragraph. Horizontal rules and table rows are structure,
//! not units. A unit carries exactly one plain `xsd:string` literal, the
//! verbatim byte span of the source, plus its byte span, its scalar
//! span, its content digest, its ordinal, its section, and its heading
//! lineage as typed literals, so a text index sees one text per unit and
//! a reader can always get back to the exact bytes — and prove, from the
//! digest, that the bytes it got back are the bytes the node was minted
//! over.
//!
//! Structure is recognized on a line's trimmed text, so a document
//! written with CRLF endings slices into the structure its LF twin
//! slices into while every span still carries the document's own bytes,
//! the `\r` among them; and a verse number is a `u64`, so a longer run
//! of digits is no verse number at all and the line stays a paragraph. A
//! byte order mark opening the document is a statement about the
//! encoding, so the first line's structure is read after it while every
//! span still counts the document's own bytes; anywhere else the mark is
//! ordinary content.
//!
//! Everything in that paragraph is Markdown, and it is all in one
//! module — the crate's `dialect`, internal and documented at length in
//! the source. The split, the identities, the containment lattice, and
//! the projection know nothing about a heading or a verse, and the next
//! structured format plugs in at that seam.
//!
//! # The split
//!
//! A unit over `max_bytes` splits into pieces of at most that many
//! bytes, with no exception. Each cut falls at the last newline at or
//! before the bound — the cut lands *on* that newline and no piece
//! carries it — else at the last scalar boundary at or before the
//! bound. A continuation reaches back `overlap` bytes, snapped backward
//! to a line start, else to a scalar boundary, and never before the
//! unit's own start. A cut never falls inside a scalar and a split never
//! crosses a heading. `max_bytes` is at least [`MIN_MAX_BYTES`] — the
//! widest UTF-8 scalar — and `overlap` sits strictly under it.
//!
//! # The concordance
//!
//! A `## Concordance` table (`| Verses | Canon source | Anchors |`)
//! lifts into citation triples from each verse in a range to its
//! anchors. Each row's lift onto one verse is also its own
//! content-addressed **citation node**, which `rdf:reifies` the RDF 1.2
//! triple term `<<( <unit> <cites> <anchor> )>>` and carries that row's
//! source paths — so a verse two rows cover keeps each row's paths
//! beside that row's anchors, which a flat projection could not say. A
//! concordance may cover a canon wider than the document that carries
//! it: a row whose verses are all elsewhere lifts nothing here and is
//! **not** an error — it is reported as data, along with any row too
//! malformed to read at all.
//!
//! # Identity
//!
//! The output is a pure, deterministic function of the bytes and a
//! declared [`Profile`]: the same input slices to byte-identical claims
//! on every target, and every parameter of the law is inside the
//! profile's identity, so a change re-mints it rather than drifting.
//!
//! A unit's IRI is `<node base>unit:sha256:<hex>`, the hex being the
//! SHA-256 of a length-prefixed preimage of the kind, the source id,
//! the profile's contract id, the byte span, the digest algorithm tag,
//! and the SHA-256 of the span's bytes. Two byte-identical paragraphs in
//! one document are two nodes, because the span is inside the identity;
//! an insertion moves every later span and re-mints every later unit
//! while leaving their text untouched; a same-length substitution
//! re-mints only the unit it touches and moves no boundary. The digest
//! algorithm is inside both the preimage and the IRI, so another
//! producer can mint the same shape under another algorithm without
//! redefining the preimage.
//!
//! A citation node is minted the same way over the pair it names — the
//! concordance row's own line span, and a digest of that line together
//! with the unit's IRI — so a row that lifts onto two verses is two
//! nodes and a unit that re-mints re-mints every citation of it.
//!
//! # It mints no vocabulary
//!
//! PurRDF is not an ontology. Every class, predicate, and datatype IRI
//! this crate emits, and the base under which it mints node IRIs, is
//! **caller-supplied configuration** in a [`Vocabulary`]; there is no
//! default namespace and no fabricated fallback. [`Vocabulary::under`]
//! derives a whole vocabulary from one base IRI with the crate's local
//! names, and a caller who wants other IRIs sets the fields directly.
//! [`Vocabulary::standard`] derives it from the one namespace the
//! specification designates for documents meant to be *exchanged* — an
//! opinion about interchange, asked for by name, never reached for on a
//! caller's behalf. The vocabulary is part of the profile's identity:
//! the same law under two vocabularies is two profiles.
//!
//! Caller-supplied is not unchecked. Every IRI a caller states — each
//! field of the vocabulary, the source id, the canon base, and every
//! IRI a concordance anchor mints under that base — must be an
//! **absolute** IRI under the workspace IRI law, which this crate
//! reaches through `purrdf-core` so that it is asking the same question
//! the kernel will ask when it interns the result. A relative IRI would
//! otherwise slice whole and be refused a stage later, far from the
//! field that wrote it.

#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]

mod claims;
mod dialect;
mod error;
mod identity;
mod model;
mod profile;
mod split;

use purrdf_core::{BaseIri, parse_iri};

pub use crate::claims::render;
pub use crate::error::MarkdownError;
pub use crate::identity::{citation_iri, section_iri, unit_iri};
pub use crate::model::{
    CONTEXT_BYTES, Citation, ContentAnchor, Document, MalformedRow, RowDefect, Section, Span,
    SpanRelation, Unit, span_relation,
};
pub use crate::profile::{Profile, STANDARD_NAMESPACE, SourceDocument, Vocabulary};

/// The default byte bound: a unit over this many bytes splits.
pub const DEFAULT_MAX_BYTES: usize = 2048;
/// The least byte bound a profile may declare: the widest UTF-8 scalar
/// is four bytes, and no piece is ever over the bound, so a smaller
/// bound could not be honoured without cutting inside a scalar.
pub const MIN_MAX_BYTES: usize = 4;
/// The default overlap: the continuation of a split reaches back this
/// many bytes, snapped backward to a newline.
pub const DEFAULT_OVERLAP: usize = 128;
/// The digest algorithm tag carried inside every unit and section
/// identity, in both the preimage and the IRI.
pub const DIGEST_ALGORITHM: &str = "sha256";

/// `rdf:type`, one of the four IRIs the slicer emits that are the
/// standard's, not the caller's.
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// `rdf:reifies`, the RDF 1.2 predicate binding a citation node to the
/// triple term it reifies. Like [`RDF_TYPE`] it is the standard's IRI
/// and never a vocabulary field: reification is a shape of the data
/// model, not a term a deployment gets to rename.
pub const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
/// `xsd:integer`, the datatype of every ordinal, level, byte offset, and
/// scalar offset.
pub const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// `xsd:hexBinary`, the datatype of a unit's content digest stated as
/// data. The lexical form is the same lowercase hex the unit's IRI
/// carries, so a consumer compares the two without re-encoding either.
pub const XSD_HEX_BINARY: &str = "http://www.w3.org/2001/XMLSchema#hexBinary";

/// Which node a claim describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimKind {
    /// The document node.
    Document,
    /// A heading or movement section.
    Section,
    /// A verse, a paragraph, or one piece of an oversize unit.
    Unit,
}

/// One node's triples, rendered as sorted N-Triples lines (valid
/// Turtle; declare `text/turtle` to a consumer).
///
/// [`Self::subject`] names the node the claim is *about*. A unit's claim
/// also carries the triples of the citation nodes minted for the rows
/// that lifted onto it — a citation is an edge of the unit and nothing
/// asks after it on its own, so it travels with the unit rather than
/// becoming a claim a consumer has to join back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
    /// The triples, one per line, sorted bytewise, each ending in `\n`.
    pub turtle: String,
    /// The node's IRI.
    pub subject: String,
    /// The node's kind.
    pub kind: ClaimKind,
    /// The node's byte span in the source (absent for the document).
    pub span: Option<(u64, u64)>,
}

/// Reads a Markdown document into the typed stand-off model under a
/// profile: the seam every refusal is stated at.
///
/// This is where the crate answers for its inputs — the profile, the
/// source id, the encoding, and every concordance anchor the document
/// names under a declared canon base — so that everything downstream of
/// it is total. [`render`] cannot fail because a [`Document`] cannot
/// exist without having passed here.
///
/// The result is deterministic in the bytes and the profile, and it
/// holds no copy of the text: it borrows the bytes it was analyzed
/// over, and every span indexes them.
///
/// # The anchor lift is checked, not trusted
///
/// With a canon base declared, every anchor of the concordance is
/// walked before a model exists, and `base ++ anchor` — the very string
/// the projection will write — must be an absolute IRI under the
/// workspace law **and** lie under the base. Containment is
/// [`BaseIri::relativize`], the one containment surface the law
/// offers: the minted IRI is under the base exactly when a relative
/// spelling of it against that base exists, which is decided by
/// round-tripping that spelling back through RFC-3986 resolution
/// rather than by comparing strings. That rule is chosen because it is
/// the only one that answers for a **fragment** base:
/// `https://example.org/canon#` ++ `tide-line` relativizes to
/// `#tide-line` and is admitted, while a prefix test would have had to
/// special-case the `#` and a resolution test would have thrown it
/// away. It is also what closes the two escapes a character blacklist
/// cannot see, both of them spelled in perfectly lawful IRI
/// characters: an anchor that climbs out of a path base (`../x` under
/// `https://example.org/canon/` resolves to `https://example.org/x`,
/// which has no relative spelling against the base) and an anchor that
/// extends the base's own host into a different authority
/// (`.elsewhere.example/x` concatenated onto `https://example.org`
/// lands on `example.org.elsewhere.example`). Both are refused.
///
/// Nothing about the anchor's *bytes* is refused: an anchor is checked
/// only against the IRI it would mint, so the law is IRI-lawfulness
/// and never ASCII. Without a canon base nothing is minted and no
/// anchor is refused at all — it is a typed literal, and arbitrary
/// bytes are legal there.
///
/// # Errors
///
/// The profile is answered for first.
/// [`MarkdownError::InvalidVocabulary`] when one of its IRIs is empty,
/// cannot be written as an IRI reference, or is not absolute;
/// [`MarkdownError::InvalidProfileName`] when its name carries a
/// control character, which the line-oriented stage description of
/// [`Profile::stage_bytes`] cannot frame;
/// [`MarkdownError::InvalidMaxBytes`] when its byte bound is under
/// [`MIN_MAX_BYTES`], a bound no split could honour without cutting
/// inside a scalar; [`MarkdownError::InvalidOverlap`] when its overlap
/// is not strictly under that bound;
/// [`MarkdownError::InvalidCanonBase`] when it declares a canon base
/// that is empty or not an absolute IRI.
///
/// Then the document. [`MarkdownError::EmptySourceId`] when the id is
/// empty, which would be written `<>` and name no document;
/// [`MarkdownError::InvalidSourceId`] when the id cannot be written
/// inside `<` and `>`; [`MarkdownError::RelativeSourceId`] when it can
/// but carries no scheme; [`MarkdownError::InvalidUtf8`] when the
/// bytes are not UTF-8.
///
/// Then, only under a declared canon base, the concordance:
/// [`MarkdownError::InvalidAnchor`] when an anchor mints no lawful IRI
/// under that base, or mints one outside it, naming the anchor and the
/// verse range of the row that wrote it.
pub fn analyze<'a>(
    doc: &SourceDocument<'a>,
    profile: &Profile,
) -> Result<Document<'a>, MarkdownError> {
    let canon = profile::validate_profile(profile)?;
    if doc.id.is_empty() {
        return Err(MarkdownError::EmptySourceId);
    }
    if let Some(found) = doc.id.chars().find(|c| claims::iri_forbids(*c)) {
        return Err(MarkdownError::InvalidSourceId { found });
    }
    if !profile::is_absolute_iri(doc.id) {
        return Err(MarkdownError::RelativeSourceId {
            id: doc.id.to_owned(),
        });
    }
    let text = std::str::from_utf8(doc.bytes).map_err(|e| MarkdownError::InvalidUtf8 {
        valid_up_to: e.valid_up_to(),
    })?;
    let reading = dialect::markdown::read(text);
    if let Some(base) = &canon {
        validate_anchors(&reading, base)?;
    }
    Ok(Document::assemble(doc.id, text, &reading, profile))
}

/// Slices a Markdown document into claims under a profile: [`analyze`]
/// then [`render`].
///
/// The result is deterministic in the bytes and the profile. Claims
/// come in document order: the document node first, then sections and
/// units interleaved as they occur.
///
/// Every refusal is stated before a byte of the document is read, and
/// nothing is ever dropped quietly: a document either slices whole or
/// names why it could not. A caller who wants more of the document than
/// the graph carries — the containment lattice, a unit's content anchor,
/// the rows that lifted nothing and the rows too malformed to read —
/// calls [`analyze`] and keeps the model.
///
/// # Errors
///
/// Exactly [`analyze`]'s, in exactly that order: the projection adds
/// none.
pub fn slice_markdown(
    doc: &SourceDocument<'_>,
    profile: &Profile,
) -> Result<Vec<Claim>, MarkdownError> {
    let document = analyze(doc, profile)?;
    Ok(render(&document, profile))
}

/// Every anchor the concordance names, checked against the IRI it
/// would mint, before a model exists.
///
/// The whole table is walked, not only the rows whose verses this
/// document happens to carry: a row that names an anchor no IRI can be
/// minted from is a defect of the document either way, and refusing it
/// where it is written is what keeps a later edit — one that adds the
/// missing verse — from turning a silent row into a sudden refusal.
fn validate_anchors(reading: &dialect::Reading, base: &BaseIri) -> Result<(), MarkdownError> {
    for row in &reading.rows {
        for anchor in &row.anchors {
            // The projection's own string, built the projection's own way.
            let minted = format!("{}{anchor}", base.as_str());
            let under_base = parse_iri(&minted).is_ok_and(|iri| base.relativize(&iri).is_some());
            if !under_base {
                return Err(MarkdownError::InvalidAnchor {
                    anchor: anchor.clone(),
                    verses: (row.first, row.last),
                });
            }
        }
    }
    Ok(())
}
