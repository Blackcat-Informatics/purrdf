// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The declared law: the vocabulary a caller supplies, the profile that
//! binds it to the split constants, and the source document the law is
//! applied to.
//!
//! Nothing here is dialect-specific. A profile states which IRIs are
//! emitted and how an oversize unit is cut; the Markdown dialect reader
//! ([`crate::dialect`]) never reads it except for those two constants.

use purrdf_core::embedding::{
    AppliedStage, ChunkingContractId, StageImplementation, derive_chunking_contract_id,
};
use purrdf_core::{BaseIri, ContentDigest, IriError};

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

/// The parameter encoding [`Profile::purremb_chunking_stage`] declares:
/// the stage's parameters are exactly the bytes of
/// [`Profile::stage_bytes`], which are UTF-8 plain text, one fact of the
/// law per line.
///
/// It is stated as a constant because a producer in another language
/// reproducing that stage has to write this same string: the encoding
/// identifier is inside the canonical stage block, so a different
/// spelling of it is a different chunking id.
pub const PURREMB_PARAMETER_ENCODING: &str = "text/plain;charset=utf-8";

/// The local names [`Vocabulary::under`] appends to a base, in the
/// order the profile's stage description lists them.
const LOCAL_NAMES: [&str; 34] = [
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
    "scalarStart",
    "scalarEnd",
    "text",
    "contentDigest",
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
    /// How many Unicode scalars of the document precede a unit's start.
    pub scalar_start: String,
    /// How many precede its end.
    pub scalar_end: String,
    /// A unit's verbatim text: the only plain literal it carries.
    pub text: String,
    /// A unit's content digest, as lowercase hex typed `xsd:hexBinary`:
    /// the very digest inside the unit's identity, stated as data.
    pub content_digest: String,
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
    /// The canon source path a citation node names, typed
    /// [`Self::dt_path`]. It annotates the citation node — the row's own
    /// edge — and never the unit, so a verse two rows cover keeps each
    /// row's paths beside that row's anchors.
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
    /// whole vocabulary of relative IRIs; and
    /// [`MarkdownError::MalformedVocabulary`] when a derived name is no
    /// IRI reference at all, carrying the law's finding about the first
    /// such field.
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
            scalar_start: iri("scalarStart"),
            scalar_end: iri("scalarEnd"),
            text: iri("text"),
            content_digest: iri("contentDigest"),
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
    /// Neither [`MarkdownError::InvalidVocabulary`] nor
    /// [`MarkdownError::MalformedVocabulary`] can arise from the
    /// designated namespace — it is absolute and every derived name is
    /// absolute with it — but the result is a `Result` all the same, so
    /// that this constructor answers exactly as
    /// [`Vocabulary::under`] does and no caller learns two shapes.
    pub fn standard() -> Result<Self, MarkdownError> {
        Self::under(STANDARD_NAMESPACE)
    }

    /// Every field, in the order the stage description lists them, with
    /// the node base last.
    fn fields(&self) -> [(&'static str, &str); 34] {
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
            ("scalarStart", &self.scalar_start),
            ("scalarEnd", &self.scalar_end),
            ("text", &self.text),
            ("contentDigest", &self.content_digest),
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
    /// The two halves of that question are answered separately, because
    /// they are separate facts about the field: a value the law reads
    /// whole and finds scheme-less is relative, and a value it cannot
    /// read is malformed, which is a finding only the law can word.
    ///
    /// # Errors
    ///
    /// [`MarkdownError::InvalidVocabulary`], naming the field, when the
    /// value is empty, cannot sit inside an IRI reference, or is an IRI
    /// reference carrying no scheme;
    /// [`MarkdownError::MalformedVocabulary`], naming the field and
    /// carrying the law's own finding, when it is no IRI reference at
    /// all.
    pub fn validate(&self) -> Result<(), MarkdownError> {
        for (name, iri) in self.fields() {
            let field = if name.is_empty() { "node base" } else { name };
            if iri.is_empty() || iri.chars().any(claims::iri_forbids) {
                return Err(MarkdownError::InvalidVocabulary {
                    field,
                    iri: iri.to_owned(),
                });
            }
            if let Err(defect) = absolute_iri(iri) {
                return Err(match defect {
                    NotAbsolute::SchemeLess => MarkdownError::InvalidVocabulary {
                        field,
                        iri: iri.to_owned(),
                    },
                    NotAbsolute::Malformed(cause) => MarkdownError::MalformedVocabulary {
                        field,
                        iri: iri.to_owned(),
                        cause,
                    },
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
    /// **the whole law**, one fact per line — the name, the version, the
    /// vocabulary, the dialect grammar, the split law and its constants,
    /// the concordance law, the identity formulas, and the emission law.
    ///
    /// It states the whole law because the contract id is the handle a
    /// consumer keeps: two runs that agree on it must agree on every
    /// byte they emit. A clause left out of this description is a clause
    /// a producer could change while the id stood still, and a consumer
    /// holding that id would have no way to learn it. So the emission
    /// law is in here beside the split law: extending the vocabulary or
    /// moving the reification shape re-mints the id, which is the
    /// design, not a cost.
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
            "{name}\n\
             version {version}\n\
             {vocabulary}\
             section ATX ^#{{1,6}}[ \\t] ; title trimmed, trailing # trimmed ; seven hashes are prose\n\
             movement ^\u{2042}[ \\t] ; name trimmed, one surrounding * or _ pair removed\n\
             a movement's level is one under the nearest heading ; movements are siblings\n\
             unit verse ^\\d+\\.[ \\t] ; the number is a u64 or the line is prose\n\
             unit paragraph blank-line ; rule ^(-{{3,}}|\\*{{3,}})$ and table row ^\\| close a unit\n\
             structure is read on a line's trimmed text ; CRLF states its LF twin's structure\n\
             U+FEFF at byte zero is encoding, elsewhere content ; every span counts the document's own bytes\n\
             max_bytes {max_bytes}\n\
             overlap {overlap}\n\
             split newline > scalar boundary ; overlap snaps backward to newline\n\
             the cut lands on that newline and no piece carries it\n\
             with no newline the cut is the last scalar boundary at or before the bound\n\
             a continuation snaps backward to a line start, else to a scalar boundary, never before the unit's start\n\
             never inside a scalar ; never across a heading\n\
             concordance section heading Concordance, ASCII case-insensitive\n\
             a table row lifts when a concordance section is in force at any depth, by containment\n\
             concordance row | verses | canon sources | anchors |\n\
             cells are delimited by unescaped | only ; \\| is a pipe, \\\\ is a backslash, any other \\ is content\n\
             verse range n, n-n, or n\u{2013}n ; both endpoints u64, the first at or under the last\n\
             a cell lifts its backticked names only, in the order written\n\
             a row that lifts nothing here and a row too malformed to read are data, never a refusal\n\
             anchor lift is base ++ anchor ; absolute, and under the base by relativization\n\
             unit id H(source id, profile id, byte start, byte end, alg, alg(span))\n\
             section id H(kind, source id, profile id, heading span, alg(heading line))\n\
             citation id H(kind, source id, profile id, row span, alg(row line, unit id))\n\
             alg {DIGEST_ALGORITHM}\n\
             emit document type, sourceDigest, mediaType, byteLength, sliceProfile, title\n\
             emit section type, document, parent, level, ordinal, heading, byteStart, byteEnd\n\
             emit unit type, text, document, section, ordinal, verse, lineage, continues\n\
             emit unit byteStart, byteEnd, scalarStart, scalarEnd, contentDigest\n\
             emit unit cites anchor, once per anchor of every row that lifted onto it\n\
             emit citation rdf:reifies <<( unit cites anchor )>> and citation canonSource path\n\
             emit an offset as xsd:integer and a content digest as lowercase hex xsd:hexBinary\n\
             emit one claim per node as N-Triples lines, sorted bytewise and de-duplicated\n\
             escaping is purrdf-core's canonical writer ; C0 and DEL as \\uXXXX\n",
            name = self.name,
            version = self.version,
            vocabulary = self.vocabulary.stage_lines(),
            max_bytes = self.max_bytes,
            overlap = self.overlap,
        )
        .into_bytes()
    }

    /// The profile's identity under the chunking-contract domain of
    /// `purrdf-core`: **this crate's own law id**, derived over the
    /// line-oriented preimage of [`Self::stage_bytes`].
    ///
    /// # It is not a PURREMB family chunking id
    ///
    /// The type is shared with PURREMB and the id is not. A PURREMB
    /// [`EmbeddingFamily`](purrdf_core::embedding::EmbeddingFamily)
    /// derives its `chunking_id` over
    /// [`AppliedStage::canonical_bytes`], which is TLV-framed: tagged,
    /// length-delimited, eight-byte aligned. This id is derived over an
    /// unframed run of newline-separated lines. The two preimages cannot
    /// coincide, so **no family contract can ever reproduce this id**,
    /// and the two ids of one profile are always different — which
    /// [`Self::purremb_chunking_stage`] exists to make usable rather
    /// than merely true.
    ///
    /// Wire this id where the *law* is meant: the `sliceProfile`
    /// literal on the document node, the third field of every node
    /// identity ([`unit_iri`](crate::unit_iri) and its siblings), a
    /// cache key over which law sliced which bytes. Wire
    /// [`Self::purremb_chunking_stage`] into a family's `chunking`
    /// stage. A `.purremb` consumer that writes this id into a family
    /// expecting a stage id has named an id that family's own contract
    /// cannot derive, and every chunk target checked against the family
    /// will disagree with it.
    #[must_use]
    pub fn contract_id(&self) -> ChunkingContractId {
        derive_chunking_contract_id(&self.stage_bytes())
    }

    /// The profile as a PURREMB chunking stage: the second identity,
    /// and the one an
    /// [`EmbeddingFamilyContract`](purrdf_core::embedding::EmbeddingFamilyContract)
    /// **can** reproduce.
    ///
    /// The stage is always [`AppliedStage::Applied`] — this crate always
    /// chunks; there is no profile under which it does not — and its
    /// four fields are:
    ///
    /// * `identifier`: [`STANDARD_NAMESPACE`], naming whose chunking law
    ///   this is. It mints nothing new: it is the one namespace the
    ///   specification already designates.
    /// * `digest`: the SHA-256 of [`Self::stage_bytes`], a manifest
    ///   digest of the exact law applied, which is the whole of what
    ///   this stage *is*.
    /// * `parameter_encoding`: [`PURREMB_PARAMETER_ENCODING`].
    /// * `parameters`: [`Self::stage_bytes`], verbatim — so every clause
    ///   inside the law id of [`Self::contract_id`] is inside this stage
    ///   too, and neither id can move while the other stands still.
    ///
    /// # Which id a consumer uses where
    ///
    /// Put this stage in the `chunking` field of the family contract.
    /// The family's derived `chunking_id` is then exactly
    /// `derive_chunking_contract_id(&stage.canonical_bytes()?)`, and
    /// **that** is the id every
    /// [`TextChunkTarget`](purrdf_core::embedding::TextChunkTarget)
    /// minted from this document carries — the id
    /// [`Unit::text_chunk_target`](crate::Unit::text_chunk_target) is
    /// handed. [`Self::contract_id`] stays where the graph states it and
    /// never enters a family.
    #[must_use]
    pub fn purremb_chunking_stage(&self) -> AppliedStage {
        let parameters = self.stage_bytes();
        let digest = ContentDigest::of(&parameters);
        AppliedStage::Applied(
            StageImplementation::new(
                STANDARD_NAMESPACE,
                digest,
                PURREMB_PARAMETER_ENCODING,
                parameters,
            )
            .expect("both identifiers are non-empty crate constants carrying no NUL"),
        )
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
            absolute_iri(base).map_err(|defect| match defect {
                // An empty base, an unreadable one and a scheme-less one
                // all leave nothing to mint under, and they are still
                // three different things to have written: the first two
                // are findings of the law, which states which of them it
                // is and where.
                NotAbsolute::SchemeLess => MarkdownError::InvalidCanonBase { base: base.clone() },
                NotAbsolute::Malformed(cause) => MarkdownError::MalformedCanonBase {
                    base: base.clone(),
                    cause,
                },
            })
        })
        .transpose()
}

/// Why a caller's string is not an absolute IRI: the one distinction
/// every seam of this crate draws, drawn in one place.
///
/// The two cases are different facts about a caller's input and have
/// different remedies. A string the IRI grammar reads whole and finds
/// scheme-less is an ordinary, bounded mistake: it *is* an IRI
/// reference, it just names something relative to whatever holds the
/// claims, and the remedy is to write it absolute. A string the grammar
/// cannot read at all is not that, and nothing this crate knows says
/// which byte of it is wrong — only the law that read it does.
///
/// Collapsing the two is what a boolean forces, and it is how
/// `https://example.org/%` — a truncated percent-encoding, and a string
/// that plainly carries a scheme — came to be refused for *having no
/// scheme*, sending its author to look for a defect that is not there.
#[derive(Debug)]
pub(crate) enum NotAbsolute {
    /// The string is a lawful IRI reference and simply carries no
    /// scheme.
    SchemeLess,
    /// The string is no IRI reference at all, and this is what the
    /// workspace IRI law found. It is carried verbatim to the seam that
    /// asked, never reworded there.
    Malformed(IriError),
}

/// The candidate as an absolute IRI under the workspace law: it parses
/// as an RFC-3987 IRI and carries a scheme. The one question, asked of
/// the one law, through the kernel that will intern the answer — and
/// the answer handed back, both when it is a base to mint under and
/// when it is a refusal to state.
pub(crate) fn absolute_iri(candidate: &str) -> Result<BaseIri, NotAbsolute> {
    BaseIri::parse(candidate).map_err(|cause| match cause {
        // A base is the parsed IRI plus the scheme check, in that order,
        // so these two — and only these two — are findings about a
        // string the grammar read whole. Every other finding is about a
        // string it could not read at all, whether or not the bytes in
        // front of the defect look like a scheme.
        IriError::NonAbsoluteBase(_) | IriError::MissingScheme => NotAbsolute::SchemeLess,
        cause => NotAbsolute::Malformed(cause),
    })
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

    /// The distinction every seam of the crate rests on, asked of the
    /// classifier that owns it: a string the law reads whole and finds
    /// scheme-less is scheme-less, and a string it cannot read is
    /// malformed — with the law's own finding, which is the part a
    /// boolean threw away.
    #[test]
    fn not_absolute_tells_a_scheme_less_iri_from_one_that_is_no_iri() {
        for relative in ["not-absolute", "doc/field-guide", "#fragment", "canon#"] {
            assert!(matches!(
                absolute_iri(relative),
                Err(NotAbsolute::SchemeLess)
            ));
        }
        for (malformed, code) in [
            ("", "iri-empty"),
            ("https://example.org/%", "iri-bad-percent-encoding"),
            ("http://[not-an-ipv6", "iri-bad-authority"),
            ("ht~tp://example.org/", "iri-bad-scheme"),
            ("1http://example.org/", "iri-disallowed-char"),
        ] {
            let Err(NotAbsolute::Malformed(cause)) = absolute_iri(malformed) else {
                panic!("{malformed:?} is no IRI reference at all");
            };
            assert_eq!(cause.diagnostic_code(), code);
        }
        // The base comes back for the caller that mints under it.
        let base = absolute_iri("https://example.org/canon#").expect("an absolute base");
        assert_eq!(base.as_str(), "https://example.org/canon#");
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
