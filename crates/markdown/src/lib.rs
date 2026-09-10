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
//! never across a heading.
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

use purrdf_core::ContentDigest;
use purrdf_core::embedding::{ChunkingContractId, derive_chunking_contract_id};

/// The default byte bound: a unit over this many bytes splits.
pub const DEFAULT_MAX_BYTES: usize = 2048;
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
    /// [`MarkdownError::InvalidVocabulary`] when the base is empty or
    /// cannot sit inside an IRI reference.
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

    /// Refuses an empty IRI, or one carrying a character that cannot sit
    /// inside an IRI reference.
    ///
    /// # Errors
    ///
    /// [`MarkdownError::InvalidVocabulary`], naming the field.
    pub fn validate(&self) -> Result<(), MarkdownError> {
        for (name, iri) in self.fields() {
            let field = if name.is_empty() { "node base" } else { name };
            if iri.is_empty() || iri.chars().any(render::iri_forbids) {
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
    /// The profile's declared name, the first line of its identity.
    pub name: String,
    /// The profile's declared version.
    pub version: u32,
    /// The IRIs the slicer emits.
    pub vocabulary: Vocabulary,
    /// A unit over this many bytes splits.
    pub max_bytes: usize,
    /// The continuation of a split reaches back this many bytes,
    /// snapped backward to a newline.
    pub overlap: usize,
    /// When set, a concordance anchor becomes the IRI `base ++ anchor`;
    /// when absent, a typed anchor literal, never a guess.
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
pub enum MarkdownError {
    /// The bytes are not UTF-8; the offset is where decoding stopped.
    InvalidUtf8 {
        /// The byte offset of the first invalid sequence.
        valid_up_to: usize,
    },
    /// The source id cannot be written as an IRI reference.
    InvalidSourceId {
        /// The offending character.
        found: char,
    },
    /// A vocabulary IRI is empty or cannot be written as an IRI
    /// reference.
    InvalidVocabulary {
        /// The vocabulary field.
        field: &'static str,
        /// The offending value.
        iri: String,
    },
}

impl std::fmt::Display for MarkdownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUtf8 { valid_up_to } => {
                write!(f, "source bytes are not UTF-8 after byte {valid_up_to}")
            }
            Self::InvalidSourceId { found } => {
                write!(
                    f,
                    "source id cannot be an IRI reference: contains {found:?}"
                )
            }
            Self::InvalidVocabulary { field, iri } => {
                write!(
                    f,
                    "vocabulary field {field} is not an IRI reference: {iri:?}"
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
/// # Errors
///
/// [`MarkdownError::InvalidVocabulary`] when the profile's vocabulary
/// does not validate; [`MarkdownError::InvalidUtf8`] when the bytes are
/// not UTF-8; [`MarkdownError::InvalidSourceId`] when the id cannot be
/// written inside `<` and `>`.
pub fn slice_markdown(
    doc: &SourceDocument<'_>,
    profile: &Profile,
) -> Result<Vec<Claim>, MarkdownError> {
    profile.vocabulary.validate()?;
    if let Some(found) = doc.id.chars().find(|c| render::iri_forbids(*c)) {
        return Err(MarkdownError::InvalidSourceId { found });
    }
    let text = std::str::from_utf8(doc.bytes).map_err(|e| MarkdownError::InvalidUtf8 {
        valid_up_to: e.valid_up_to(),
    })?;
    let contract = profile.contract_id();
    let structure = parse::parse(text, profile);
    Ok(render::render(doc, text, profile, &contract, &structure))
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
