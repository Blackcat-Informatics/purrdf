// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The declared law: the vocabulary a caller supplies, the profile that
//! binds it to the split constants, and the source document the law is
//! applied to.
//!
//! Nothing here is dialect-specific. A profile states which IRIs are
//! emitted and how an oversize unit is cut; the Markdown dialect reader
//! ([`crate::dialect`]) never reads it except for those two constants.

use purrdf_core::BaseIri;
use purrdf_core::embedding::{ChunkingContractId, derive_chunking_contract_id};

use crate::error::MarkdownError;
use crate::{DIGEST_ALGORITHM, MIN_MAX_BYTES, claims};

/// The namespace the specification designates for this crate's terms:
/// the one namespace [`Vocabulary::standard`] derives everything under.
///
/// It is not a default and it is not a fallback. Nothing reaches for it
/// unless a caller names it, by calling [`Vocabulary::standard`] or by
/// writing it out — see that constructor for why the import/export
/// surface is deliberately opinionated while [`Vocabulary::under`]
/// stays open.
pub const STANDARD_NAMESPACE: &str = "https://w3id.org/purrdf/markdown#";

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
/// all of them from one base, [`Vocabulary::standard`] fills them from
/// the namespace the specification designates, and
/// [`Vocabulary::validate`] is applied before any claim is rendered.
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

    /// The vocabulary under [`STANDARD_NAMESPACE`]: exactly
    /// `Vocabulary::under("https://w3id.org/purrdf/markdown#")`, term
    /// for term.
    ///
    /// PurRDF mints no ontology, and this is not one: every IRI here is
    /// still the local names of [`Vocabulary::under`] appended to a
    /// base, and nothing in the crate reaches for this base on its own.
    /// What it is, is an **opinion about interchange**. Two deployments
    /// that each derive their own namespace publish two graphs no
    /// consumer can join without a mapping, and the mapping is the part
    /// that never gets written. So the specification designates one
    /// namespace for documents meant to be *exchanged*, and this
    /// constructor is the only place the crate spells it. A deployment
    /// that is sovereign over its own terms keeps [`Vocabulary::under`]
    /// and loses nothing: the law, the identities, and the split are
    /// the same either way, and the profile's contract id tells the two
    /// apart.
    ///
    /// # Errors
    ///
    /// [`MarkdownError::InvalidVocabulary`] cannot arise from the
    /// designated namespace — it is absolute and every derived name is
    /// absolute with it — but the result is a `Result` all the same, so
    /// that this constructor answers exactly as
    /// [`Vocabulary::under`] does and no caller learns two shapes.
    pub fn standard() -> Result<Self, MarkdownError> {
        Self::under(STANDARD_NAMESPACE)
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
            if iri.is_empty() || iri.chars().any(claims::iri_forbids) || !is_absolute_iri(iri) {
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
    /// one too, and must be provably under it — see
    /// [`slice_markdown`](crate::slice_markdown) for the containment
    /// rule and [`MarkdownError::InvalidAnchor`] for what a failure
    /// names. The lift is **pure concatenation**, never RFC-3986
    /// reference resolution: resolution would dissolve a fragment
    /// base's `#` and fold a dot segment away, so `base ++ anchor` and
    /// the IRI the caller declared would part company. Concatenation is
    /// why the anchors have to be checked rather than trusted.
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
            max_bytes: crate::DEFAULT_MAX_BYTES,
            overlap: crate::DEFAULT_OVERLAP,
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

/// The profile answers for itself before any document does: its
/// vocabulary states IRIs, its name is one line of its own identity,
/// its two constants describe a split that terminates and stays inside
/// the bound, and its canon base, when it declares one, is a base.
///
/// The parsed base comes back rather than being thrown away, so the
/// anchor lift mints under the same base this checked, once.
pub(crate) fn validate_profile(profile: &Profile) -> Result<Option<BaseIri>, MarkdownError> {
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

/// Whether a string is an absolute IRI under the workspace law: it
/// parses as an RFC-3987 IRI and carries a scheme. The one question,
/// asked of the one law, through the kernel that will intern the
/// answer.
pub(crate) fn is_absolute_iri(candidate: &str) -> bool {
    BaseIri::parse(candidate).is_ok()
}

/// A character no line-oriented preimage can carry: the C0 controls,
/// newline and tab among them, and the delete.
const fn is_control(c: char) -> bool {
    (c as u32) < 0x20 || c == '\u{7f}'
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

    #[test]
    fn the_standard_vocabulary_is_the_designated_namespace_and_states_itself_as_that_base() {
        let standard = Vocabulary::standard().expect("the designated namespace derives one");
        assert_eq!(standard.common_base(), Some(STANDARD_NAMESPACE));
        assert_eq!(
            standard.stage_lines(),
            format!("vocabulary {STANDARD_NAMESPACE}\n")
        );
    }
}
