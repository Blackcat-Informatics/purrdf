// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf-markdown` — a Markdown document becomes an RDF 1.2 graph along
//! its own structure.
//!
//! The slicer reads a Markdown document and emits one claim per node of
//! the document's structure: one document node; one section node per
//! ATX heading and per movement marker (`⁂ *name*`); and one unit node
//! per numbered verse (`12. text`) or blank-line paragraph. A unit
//! carries exactly one plain `xsd:string` literal, the verbatim byte
//! span of the source, plus its byte span, ordinal, section, and
//! heading lineage as typed literals, so a text index sees one text per
//! unit and a reader can always get back to the exact bytes. A
//! `## Concordance` table (`| Verses | Canon source | Anchors |`) lifts
//! into citation triples from each verse in a range to its anchors and
//! source paths. A unit over a byte bound splits at a newline, else at a
//! scalar boundary, with a snapped overlap, never inside a scalar and
//! never across a heading. A byte order mark opening the document is a
//! statement about the encoding, so the first line's structure is read
//! after it while every span still counts the document's own bytes;
//! anywhere else the mark is ordinary content.
//!
//! The output is a pure, deterministic function of the bytes and a
//! declared [`Profile`]: the same input slices to byte-identical claims
//! on every target, and every parameter of the law is inside the
//! profile's identity, so a change re-mints it rather than drifting.
//!
//! # It mints no vocabulary
//!
//! PurRDF is not an ontology. Every class, predicate, and datatype IRI
//! this crate emits, and the base under which it mints node IRIs, is
//! **caller-supplied configuration** in a [`Vocabulary`]; there is no
//! default namespace and no fabricated fallback. [`Vocabulary::under`]
//! derives a whole vocabulary from one base IRI with the crate's local
//! names, and a caller who wants other IRIs sets the fields directly.
//! The vocabulary is part of the profile's identity: the same law under
//! two vocabularies is two profiles.
//!
//! Caller-supplied is not unchecked. Every IRI a caller states — each
//! field of the vocabulary, the source id, the canon base, and every
//! IRI a concordance anchor mints under that base — must be an
//! **absolute** IRI under the workspace IRI law, which this crate
//! reaches through `purrdf-core` so that it is asking the same question
//! the kernel will ask when it interns the result. A relative IRI would
//! otherwise slice whole and be refused a stage later, far from the
//! field that wrote it.
//!
//! # Identity
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

#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]

mod parse;
mod render;

use purrdf_core::embedding::{ChunkingContractId, derive_chunking_contract_id};
use purrdf_core::{BaseIri, ContentDigest, parse_iri};

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

/// `rdf:type`, the one IRI the slicer emits that is the standard's, not
/// the caller's.
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// `xsd:integer`, the datatype of every ordinal, level, and byte offset.
pub const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// The local names [`Vocabulary::under`] appends to a base, in the
/// order the profile's stage description lists them.
const LOCAL_NAMES: [&str; 31] = [
    "Document",
    "Section",
    "Movement",
    "Unit",
    "sourceDigest",
    "mediaType",
    "byteLength",
    "sliceProfile",
    "title",
    "document",
    "parent",
    "level",
    "ordinal",
    "heading",
    "byteStart",
    "byteEnd",
    "text",
    "section",
    "verse",
    "lineage",
    "continues",
    "cites",
    "canonSource",
    "digest",
    "media",
    "profile",
    "heading",
    "lineage",
    "anchor",
    "path",
    "",
];

/// Every IRI the slicer emits, supplied by the caller. The fields are
/// public so a caller can set any of them; [`Vocabulary::under`] fills
/// all of them from one base, and [`Vocabulary::validate`] is applied
/// before any claim is rendered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vocabulary {
    /// The base node IRIs are minted under: a unit is
    /// `<node_base>unit:sha256:<hex>`, a section
    /// `<node_base>section:sha256:<hex>`.
    pub node_base: String,
    /// The class of the document node.
    pub document_class: String,
    /// The class of a heading section.
    pub section_class: String,
    /// The class of a movement section.
    pub movement_class: String,
    /// The class of a unit (a verse or a paragraph).
    pub unit_class: String,
    /// The document's source digest, typed [`Self::dt_digest`].
    pub source_digest: String,
    /// The document's media type, typed [`Self::dt_media`].
    pub media_type: String,
    /// The document's byte length.
    pub byte_length: String,
    /// The profile the document was sliced under, typed
    /// [`Self::dt_profile`].
    pub slice_profile: String,
    /// The document's title, typed [`Self::dt_heading`].
    pub title: String,
    /// The document a section or unit belongs to.
    pub in_document: String,
    /// A section's parent section.
    pub parent: String,
    /// A section's level.
    pub level: String,
    /// Document order among sections, or among units.
    pub ordinal: String,
    /// A section's heading text, typed [`Self::dt_heading`].
    pub heading: String,
    /// The first byte of a span.
    pub byte_start: String,
    /// One past the last byte of a span.
    pub byte_end: String,
    /// A unit's verbatim text: the only plain literal it carries.
    pub text: String,
    /// The innermost section in force at a unit's start.
    pub in_section: String,
    /// A numbered verse's number.
    pub verse: String,
    /// The heading stack at a unit's start, typed [`Self::dt_lineage`].
    pub lineage: String,
    /// The previous piece of an oversize unit.
    pub continues: String,
    /// A verse's citation of a canon anchor.
    pub cites: String,
    /// A verse's canon source path, typed [`Self::dt_path`].
    pub canon_source: String,
    /// Datatype of a digest literal (`sha256:<hex>`).
    pub dt_digest: String,
    /// Datatype of a media type literal.
    pub dt_media: String,
    /// Datatype of a profile literal (`<name>:<contract id hex>`).
    pub dt_profile: String,
    /// Datatype of a heading or title literal.
    pub dt_heading: String,
    /// Datatype of a lineage literal.
    pub dt_lineage: String,
    /// Datatype of a canon anchor literal (when no canon base is set).
    pub dt_anchor: String,
    /// Datatype of a canon source path literal.
    pub dt_path: String,
}

impl Vocabulary {
    /// Every IRI of the vocabulary derived from one base by the crate's
    /// local names (`<base>Document`, `<base>byteStart`, ...), with the
    /// base itself as the node base.
    ///
    /// # Errors
    ///
    /// [`MarkdownError::InvalidVocabulary`] when the base is empty,
    /// cannot sit inside an IRI reference, or does not make every
    /// derived name an absolute IRI — a base without a scheme derives a
    /// whole vocabulary of relative IRIs.
    pub fn under(base: &str) -> Result<Self, MarkdownError> {
        let iri = |local: &str| format!("{base}{local}");
        let vocabulary = Self {
            node_base: base.to_owned(),
            document_class: iri("Document"),
            section_class: iri("Section"),
            movement_class: iri("Movement"),
            unit_class: iri("Unit"),
            source_digest: iri("sourceDigest"),
            media_type: iri("mediaType"),
            byte_length: iri("byteLength"),
            slice_profile: iri("sliceProfile"),
            title: iri("title"),
            in_document: iri("document"),
            parent: iri("parent"),
            level: iri("level"),
            ordinal: iri("ordinal"),
            heading: iri("heading"),
            byte_start: iri("byteStart"),
            byte_end: iri("byteEnd"),
            text: iri("text"),
            in_section: iri("section"),
            verse: iri("verse"),
            lineage: iri("lineage"),
            continues: iri("continues"),
            cites: iri("cites"),
            canon_source: iri("canonSource"),
            dt_digest: iri("digest"),
            dt_media: iri("media"),
            dt_profile: iri("profile"),
            dt_heading: iri("heading"),
            dt_lineage: iri("lineage"),
            dt_anchor: iri("anchor"),
            dt_path: iri("path"),
        };
        vocabulary.validate()?;
        Ok(vocabulary)
    }

    /// Every field, in the order the stage description lists them, with
    /// the node base last.
    fn fields(&self) -> [(&'static str, &str); 31] {
        [
            ("Document", &self.document_class),
            ("Section", &self.section_class),
            ("Movement", &self.movement_class),
            ("Unit", &self.unit_class),
            ("sourceDigest", &self.source_digest),
            ("mediaType", &self.media_type),
            ("byteLength", &self.byte_length),
            ("sliceProfile", &self.slice_profile),
            ("title", &self.title),
            ("document", &self.in_document),
            ("parent", &self.parent),
            ("level", &self.level),
            ("ordinal", &self.ordinal),
            ("heading", &self.heading),
            ("byteStart", &self.byte_start),
            ("byteEnd", &self.byte_end),
            ("text", &self.text),
            ("section", &self.in_section),
            ("verse", &self.verse),
            ("lineage", &self.lineage),
            ("continues", &self.continues),
            ("cites", &self.cites),
            ("canonSource", &self.canon_source),
            ("digest", &self.dt_digest),
            ("media", &self.dt_media),
            ("profile", &self.dt_profile),
            ("heading", &self.dt_heading),
            ("lineage", &self.dt_lineage),
            ("anchor", &self.dt_anchor),
            ("path", &self.dt_path),
            ("", &self.node_base),
        ]
    }

    /// Refuses an empty IRI, one carrying a character that cannot sit
    /// inside an IRI reference, and one that is not **absolute** under
    /// the workspace IRI law.
    ///
    /// Absoluteness is not a nicety here. A vocabulary of relative IRIs
    /// renders claims the kernel refuses the moment it tries to intern
    /// them, so a slicer that accepted one would hand back a document's
    /// whole graph and let it fail a stage later, far from the field
    /// that caused it. The question asked is the kernel's own, through
    /// `purrdf-core`'s re-export of the law: parse as an RFC-3987 IRI,
    /// and carry a scheme.
    ///
    /// # Errors
    ///
    /// [`MarkdownError::InvalidVocabulary`], naming the field.
    pub fn validate(&self) -> Result<(), MarkdownError> {
        for (name, iri) in self.fields() {
            let field = if name.is_empty() { "node base" } else { name };
            if iri.is_empty() || iri.chars().any(render::iri_forbids) || !is_absolute_iri(iri) {
                return Err(MarkdownError::InvalidVocabulary {
                    field,
                    iri: iri.to_owned(),
                });
            }
        }
        Ok(())
    }

    /// The one base every IRI is derived from by the local names, when
    /// there is one: the compact form of the stage description.
    fn common_base(&self) -> Option<&str> {
        let base = self.node_base.as_str();
        let fields = self.fields();
        let derived = fields
            .iter()
            .zip(LOCAL_NAMES)
            .all(|((_, iri), local)| *iri == format!("{base}{local}"));
        derived.then_some(base)
    }

    /// The vocabulary as the stage description states it: one line
    /// naming the base when every IRI derives from it, else one line per
    /// IRI.
    fn stage_lines(&self) -> String {
        self.common_base().map_or_else(
            || {
                use std::fmt::Write as _;
                let mut out = String::from("vocabulary explicit\n");
                for (name, iri) in self.fields() {
                    let name = if name.is_empty() { "node-base" } else { name };
                    let _ = writeln!(out, "  {name} {iri}");
                }
                out
            },
            |base| format!("vocabulary {base}\n"),
        )
    }
}

/// The slicing profile: its name and version, its vocabulary, the byte
/// bound and overlap that act only on an oversize unit, and the
/// optional canon IRI base for concordance anchors. The name, the
/// version, the vocabulary, the bound, and the overlap are inside the
/// contract id; the canon base is an option of the consumer, not a
/// parameter of the law, and stays outside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    /// The profile's declared name, the first line of its identity. It
    /// is a line of [`Self::stage_bytes`], so it carries no control
    /// character: see that method for why the format binds the name.
    pub name: String,
    /// The profile's declared version.
    pub version: u32,
    /// The IRIs the slicer emits.
    pub vocabulary: Vocabulary,
    /// A unit over this many bytes splits, and no piece is ever over
    /// this many bytes. A bound under [`MIN_MAX_BYTES`] is refused: it
    /// could not be honoured without cutting inside a scalar.
    pub max_bytes: usize,
    /// The continuation of a split reaches back this many bytes,
    /// snapped backward to a newline. It sits strictly under
    /// [`Self::max_bytes`]; an overlap at or over the bound would let a
    /// continuation reach back over the whole piece it continues.
    pub overlap: usize,
    /// When set, a concordance anchor becomes the IRI `base ++ anchor`;
    /// when absent, a typed anchor literal, never a guess.
    ///
    /// A base that is set must be non-empty and an absolute IRI under
    /// the workspace IRI law, and every anchor it mints under must be
    /// one too, and must be provably under it — see [`slice_markdown`]
    /// for the containment rule and
    /// [`MarkdownError::InvalidAnchor`] for what a failure names. The
    /// lift is **pure concatenation**, never RFC-3986 reference
    /// resolution: resolution would dissolve a fragment base's `#` and
    /// fold a dot segment away, so `base ++ anchor` and the IRI the
    /// caller declared would part company. Concatenation is why the
    /// anchors have to be checked rather than trusted.
    pub canon_base: Option<String>,
}

impl Profile {
    /// A profile under a vocabulary with the default bound and overlap
    /// and no canon base.
    #[must_use]
    pub fn new(name: &str, version: u32, vocabulary: Vocabulary) -> Self {
        Self {
            name: name.to_owned(),
            version,
            vocabulary,
            max_bytes: DEFAULT_MAX_BYTES,
            overlap: DEFAULT_OVERLAP,
            canon_base: None,
        }
    }

    /// The canonical stage description the contract id is derived over:
    /// the name, the version, the vocabulary, the split law, and the
    /// constants, one fact per line.
    ///
    /// The preimage is line-oriented and unframed: a newline ends one
    /// fact and begins the next, and no field is length-prefixed or
    /// quoted. That is why [`Self::name`] is refused a control
    /// character — a newline inside the name would state further facts
    /// of the law rather than name it, and two profiles could then
    /// describe themselves the same way. The refusal is a bound of the
    /// identity format, not a taste in names.
    #[must_use]
    pub fn stage_bytes(&self) -> Vec<u8> {
        format!(
            "{}\n\
             version {}\n\
             {}\
             section ATX ^#{{1,6}}\\s ; movement ^\u{2042}\\s\n\
             unit verse ^\\d+\\.\\s ; paragraph blank-line\n\
             max_bytes {}\n\
             overlap {}\n\
             split newline > scalar boundary ; overlap snaps backward to newline\n\
             never inside a scalar ; never across a heading\n\
             unit id H(source id, profile id, byte start, byte end, alg, alg(span))\n\
             alg {DIGEST_ALGORITHM}\n",
            self.name,
            self.version,
            self.vocabulary.stage_lines(),
            self.max_bytes,
            self.overlap
        )
        .into_bytes()
    }

    /// The profile's identity under the chunking-contract domain of
    /// `purrdf-core`.
    #[must_use]
    pub fn contract_id(&self) -> ChunkingContractId {
        derive_chunking_contract_id(&self.stage_bytes())
    }

    /// The literal recorded on the document node: `<name>:<hex>`.
    #[must_use]
    pub fn label(&self) -> String {
        format!("{}:{}", self.name, self.contract_id().to_hex())
    }
}

/// One source document: its IRI and its exact bytes.
#[derive(Clone, Copy, Debug)]
pub struct SourceDocument<'a> {
    /// The document's IRI; it is the subject of the document node.
    pub id: &'a str,
    /// The exact bytes, which must be UTF-8.
    pub bytes: &'a [u8],
}

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

/// Why a document could not be sliced.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownError {
    /// The bytes are not UTF-8; the offset is where decoding stopped.
    InvalidUtf8 {
        /// The byte offset of the first invalid sequence.
        valid_up_to: usize,
    },
    /// The source id is empty. It would be written `<>`, a relative IRI
    /// reference that names whatever document happens to hold the
    /// claims, so every node the slicer minted would point at nothing
    /// fixed.
    EmptySourceId,
    /// The source id cannot be written as an IRI reference.
    InvalidSourceId {
        /// The offending character.
        found: char,
    },
    /// The source id is writable but not absolute: it carries no
    /// scheme, so it names a document only relative to whatever holds
    /// the claims, and every node minted over it inherits that.
    RelativeSourceId {
        /// The id as declared.
        id: String,
    },
    /// A vocabulary IRI is empty, cannot be written as an IRI
    /// reference, or is not absolute.
    InvalidVocabulary {
        /// The vocabulary field.
        field: &'static str,
        /// The offending value.
        iri: String,
    },
    /// The profile's byte bound is under [`MIN_MAX_BYTES`], so no piece
    /// could be both within the bound and outside a scalar.
    InvalidMaxBytes {
        /// The declared bound.
        max_bytes: usize,
        /// The least bound the split law can honour.
        least: usize,
    },
    /// The profile's overlap is at or over its byte bound, so a
    /// continuation could reach back over the whole piece it continues
    /// and advance by as little as a byte.
    InvalidOverlap {
        /// The declared overlap.
        overlap: usize,
        /// The declared bound it must sit strictly under.
        max_bytes: usize,
    },
    /// The profile's name carries a control character, which the
    /// line-oriented stage description cannot frame.
    InvalidProfileName {
        /// The offending character.
        found: char,
    },
    /// The profile declares a canon base that is empty or is not an
    /// absolute IRI, so nothing concatenated onto it could be one
    /// either.
    InvalidCanonBase {
        /// The base as declared.
        base: String,
    },
    /// A concordance anchor does not mint a lawful IRI under the
    /// declared canon base: `base ++ anchor` is not an absolute IRI, or
    /// it is one that lies outside the base. The verse range is the
    /// row's, so a long concordance names the row to fix rather than
    /// leaving it to be searched for.
    InvalidAnchor {
        /// The anchor as the concordance wrote it, between backticks.
        anchor: String,
        /// The first and last verse of the row that named it.
        verses: (u64, u64),
    },
}

impl std::fmt::Display for MarkdownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUtf8 { valid_up_to } => {
                write!(f, "source bytes are not UTF-8 after byte {valid_up_to}")
            }
            Self::EmptySourceId => {
                write!(f, "source id is empty: <> names no document")
            }
            Self::InvalidSourceId { found } => {
                write!(
                    f,
                    "source id cannot be an IRI reference: contains {found:?}"
                )
            }
            Self::RelativeSourceId { id } => {
                write!(f, "source id is not an absolute IRI: {id:?} has no scheme")
            }
            Self::InvalidVocabulary { field, iri } => {
                write!(
                    f,
                    "vocabulary field {field} is not an absolute IRI: {iri:?}"
                )
            }
            Self::InvalidMaxBytes { max_bytes, least } => {
                write!(
                    f,
                    "profile max_bytes {max_bytes} is under {least}, the widest UTF-8 scalar"
                )
            }
            Self::InvalidOverlap { overlap, max_bytes } => {
                write!(
                    f,
                    "profile overlap {overlap} is not under max_bytes {max_bytes}"
                )
            }
            Self::InvalidProfileName { found } => {
                write!(
                    f,
                    "profile name cannot hold a control character: contains {found:?}"
                )
            }
            Self::InvalidCanonBase { base } => {
                write!(f, "canon base is not an absolute IRI: {base:?}")
            }
            Self::InvalidAnchor { anchor, verses } => {
                write!(
                    f,
                    "concordance anchor {anchor:?} for verses {}\u{2013}{} mints no lawful IRI under the canon base",
                    verses.0, verses.1
                )
            }
        }
    }
}

impl std::error::Error for MarkdownError {}

/// Slices a Markdown document into claims under a profile.
///
/// The result is deterministic in the bytes and the profile. Claims
/// come in document order: the document node first, then sections and
/// units interleaved as they occur.
///
/// Every refusal is stated before a byte of the document is read, and
/// nothing is ever dropped quietly: a document either slices whole or
/// names why it could not.
///
/// # The anchor lift is checked, not trusted
///
/// With a canon base declared, every anchor of the concordance is
/// walked before a claim is rendered, and `base ++ anchor` — the very
/// string the renderer will write — must be an absolute IRI under the
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
pub fn slice_markdown(
    doc: &SourceDocument<'_>,
    profile: &Profile,
) -> Result<Vec<Claim>, MarkdownError> {
    let canon = validate_profile(profile)?;
    if doc.id.is_empty() {
        return Err(MarkdownError::EmptySourceId);
    }
    if let Some(found) = doc.id.chars().find(|c| render::iri_forbids(*c)) {
        return Err(MarkdownError::InvalidSourceId { found });
    }
    if !is_absolute_iri(doc.id) {
        return Err(MarkdownError::RelativeSourceId {
            id: doc.id.to_owned(),
        });
    }
    let text = std::str::from_utf8(doc.bytes).map_err(|e| MarkdownError::InvalidUtf8 {
        valid_up_to: e.valid_up_to(),
    })?;
    let contract = profile.contract_id();
    let structure = parse::parse(text, profile);
    if let Some(base) = &canon {
        validate_anchors(&structure, base)?;
    }
    Ok(render::render(doc, text, profile, &contract, &structure))
}

/// The profile answers for itself before any document does: its
/// vocabulary states IRIs, its name is one line of its own identity,
/// its two constants describe a split that terminates and stays inside
/// the bound, and its canon base, when it declares one, is a base.
///
/// The parsed base comes back rather than being thrown away, so the
/// anchor lift mints under the same base this checked, once.
fn validate_profile(profile: &Profile) -> Result<Option<BaseIri>, MarkdownError> {
    profile.vocabulary.validate()?;
    if let Some(found) = profile.name.chars().find(|c| is_control(*c)) {
        return Err(MarkdownError::InvalidProfileName { found });
    }
    if profile.max_bytes < MIN_MAX_BYTES {
        return Err(MarkdownError::InvalidMaxBytes {
            max_bytes: profile.max_bytes,
            least: MIN_MAX_BYTES,
        });
    }
    if profile.overlap >= profile.max_bytes {
        return Err(MarkdownError::InvalidOverlap {
            overlap: profile.overlap,
            max_bytes: profile.max_bytes,
        });
    }
    profile
        .canon_base
        .as_ref()
        .map(|base| {
            // An empty base and an unparseable one are the same defect:
            // there is no IRI to mint under.
            BaseIri::parse(base).map_err(|_| MarkdownError::InvalidCanonBase { base: base.clone() })
        })
        .transpose()
}

/// Every anchor the concordance names, checked against the IRI it
/// would mint, before a claim is rendered.
///
/// The whole table is walked, not only the rows whose verses this
/// document happens to carry: a row that names an anchor no IRI can be
/// minted from is a defect of the document either way, and refusing it
/// where it is written is what keeps a later edit — one that adds the
/// missing verse — from turning a silent row into a sudden refusal.
fn validate_anchors(structure: &parse::Structure, base: &BaseIri) -> Result<(), MarkdownError> {
    for citation in &structure.citations {
        for anchor in &citation.anchors {
            // The renderer's own string, built the renderer's own way.
            let minted = format!("{}{anchor}", base.as_str());
            let under_base = parse_iri(&minted).is_ok_and(|iri| base.relativize(&iri).is_some());
            if !under_base {
                return Err(MarkdownError::InvalidAnchor {
                    anchor: anchor.clone(),
                    verses: (citation.first, citation.last),
                });
            }
        }
    }
    Ok(())
}

/// Whether a string is an absolute IRI under the workspace law: it
/// parses as an RFC-3987 IRI and carries a scheme. The one question,
/// asked of the one law, through the kernel that will intern the
/// answer.
fn is_absolute_iri(candidate: &str) -> bool {
    BaseIri::parse(candidate).is_ok()
}

/// A character no line-oriented preimage can carry: the C0 controls,
/// newline and tab among them, and the delete.
const fn is_control(c: char) -> bool {
    (c as u32) < 0x20 || c == '\u{7f}'
}

/// The identity of a unit, with the digest algorithm inside both the
/// preimage and the IRI. A consumer re-derives it from the bytes at a
/// span to prove a hit is bound to its source.
#[must_use]
pub fn unit_iri(
    vocabulary: &Vocabulary,
    source_id: &str,
    contract: &ChunkingContractId,
    byte_start: u64,
    byte_end: u64,
    span: &[u8],
) -> String {
    node_iri(
        vocabulary, "unit", source_id, contract, byte_start, byte_end, span,
    )
}

/// The identity of a section, over its heading line's bytes.
#[must_use]
pub fn section_iri(
    vocabulary: &Vocabulary,
    source_id: &str,
    contract: &ChunkingContractId,
    byte_start: u64,
    byte_end: u64,
    heading_line: &[u8],
) -> String {
    node_iri(
        vocabulary,
        "section",
        source_id,
        contract,
        byte_start,
        byte_end,
        heading_line,
    )
}

fn node_iri(
    vocabulary: &Vocabulary,
    kind: &str,
    source_id: &str,
    contract: &ChunkingContractId,
    byte_start: u64,
    byte_end: u64,
    bytes: &[u8],
) -> String {
    let mut preimage = Vec::new();
    push_field(&mut preimage, kind.as_bytes());
    push_field(&mut preimage, source_id.as_bytes());
    push_field(&mut preimage, contract.as_bytes());
    push_field(&mut preimage, &byte_start.to_le_bytes());
    push_field(&mut preimage, &byte_end.to_le_bytes());
    push_field(&mut preimage, DIGEST_ALGORITHM.as_bytes());
    push_field(&mut preimage, ContentDigest::of(bytes).as_bytes());
    format!(
        "{}{kind}:{DIGEST_ALGORITHM}:{}",
        vocabulary.node_base,
        ContentDigest::of(&preimage).to_hex()
    )
}

/// Length-prefixed field: no two field sequences share a preimage.
fn push_field(out: &mut Vec<u8>, field: &[u8]) {
    out.extend_from_slice(&(field.len() as u64).to_le_bytes());
    out.extend_from_slice(field);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vocabulary_under_one_base_states_itself_as_that_base() {
        let v = Vocabulary::under("urn:test:").expect("a vocabulary");
        assert_eq!(v.common_base(), Some("urn:test:"));
        assert_eq!(v.stage_lines(), "vocabulary urn:test:\n");
        let mut custom = v;
        custom.text = "urn:other:body".to_owned();
        assert_eq!(custom.common_base(), None);
        assert!(custom.stage_lines().starts_with("vocabulary explicit\n"));
        assert!(custom.stage_lines().contains("  text urn:other:body\n"));
    }

    #[test]
    fn an_empty_or_unwritable_iri_refuses_and_names_the_field() {
        assert!(matches!(
            Vocabulary::under(""),
            Err(MarkdownError::InvalidVocabulary { .. })
        ));
        let mut v = Vocabulary::under("urn:test:").expect("a vocabulary");
        v.cites = "urn:with space".to_owned();
        assert_eq!(
            v.validate(),
            Err(MarkdownError::InvalidVocabulary {
                field: "cites",
                iri: "urn:with space".to_owned(),
            })
        );
    }
}
