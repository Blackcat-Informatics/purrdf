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
///
/// One honest caveat: this namespace is not publicly dereferenceable. It
/// is a stable identifier, not a URL that resolves today; a redirect for
/// it is being registered, and no date is promised. Nothing here depends
/// on that — RDF does not require an IRI to dereference, so the terms
/// derived from this base, the node identities minted under it, and the
/// contract id computed over both are what they would be either way.
pub const STANDARD_NAMESPACE: &str = "https://w3id.org/purrdf/markdown#";

/// The parameter encoding [`Profile::purremb_chunking_stage`] declares:
/// the stage's parameters are exactly the bytes of
/// [`Profile::chunking_bytes`], which are UTF-8 plain text, one fact of
/// the law per line.
///
/// It is stated as a constant because a producer in another language
/// reproducing that stage has to write this same string: the encoding
/// identifier is inside the canonical stage block, so a different
/// spelling of it is a different chunking id.
pub const PURREMB_PARAMETER_ENCODING: &str = "text/plain;charset=utf-8";

/// What a term of the set **is** where it is written: the one
/// distinction an OWL or SHACL description of a namespace rests on,
/// since no vocabulary can describe one IRI as an `owl:DatatypeProperty`
/// and an `rdfs:Datatype` at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum TermRole {
    /// Written as the object of an `rdf:type` line, and nowhere else.
    Class,
    /// Written in the predicate position of a line.
    Predicate,
    /// Written as the datatype IRI of a typed literal.
    Datatype,
    /// No term at all: the base node IRIs are minted under, which names
    /// individuals rather than describing them.
    NodeBase,
}

/// The term set: what each term is, and the local name
/// [`Vocabulary::under`] appends to a base to derive it, in the order
/// the profile's stage description lists them.
///
/// Every local name in the table is distinct, so **every IRI a derived
/// vocabulary states wears exactly one role** — the designated
/// namespace's IRIs among them, which is what lets an OWL or SHACL
/// description of that namespace be written at all.
///
/// That is why the datatype of a heading is `headingText` rather than
/// `heading`, and the datatype of a lineage `lineagePath` rather than
/// `lineage`: the bare names are predicates, and each datatype name says
/// which lexical space its literals live in, exactly as `digest`,
/// `media`, `profile`, `anchor` and `path` do for theirs.
const TERMS: [(TermRole, &str); 36] = [
    (TermRole::Class, "Document"),
    (TermRole::Class, "Section"),
    (TermRole::Class, "Movement"),
    (TermRole::Class, "Unit"),
    (TermRole::Class, "Citation"),
    (TermRole::Predicate, "sourceDigest"),
    (TermRole::Predicate, "mediaType"),
    (TermRole::Predicate, "byteLength"),
    (TermRole::Predicate, "sliceProfile"),
    (TermRole::Predicate, "title"),
    (TermRole::Predicate, "document"),
    (TermRole::Predicate, "parent"),
    (TermRole::Predicate, "level"),
    (TermRole::Predicate, "ordinal"),
    (TermRole::Predicate, "heading"),
    (TermRole::Predicate, "byteStart"),
    (TermRole::Predicate, "byteEnd"),
    (TermRole::Predicate, "scalarStart"),
    (TermRole::Predicate, "scalarEnd"),
    (TermRole::Predicate, "text"),
    (TermRole::Predicate, "contentDigest"),
    (TermRole::Predicate, "section"),
    (TermRole::Predicate, "verse"),
    (TermRole::Predicate, "lineage"),
    (TermRole::Predicate, "continues"),
    (TermRole::Predicate, "cites"),
    (TermRole::Predicate, "canonSource"),
    (TermRole::Predicate, "unit"),
    (TermRole::Datatype, "digest"),
    (TermRole::Datatype, "media"),
    (TermRole::Datatype, "profile"),
    (TermRole::Datatype, "headingText"),
    (TermRole::Datatype, "lineagePath"),
    (TermRole::Datatype, "anchor"),
    (TermRole::Datatype, "path"),
    (TermRole::NodeBase, ""),
];

/// The local names a derived vocabulary appends to its base, in table
/// order, separated by one space: the second half of the compact stage
/// line, and the field-by-field statement of which term this crate's
/// [`Vocabulary::under`] derives where.
///
/// The node base's empty name is not among them; `vocabulary <base>` is
/// the line that states it.
fn derived_terms() -> String {
    TERMS
        .iter()
        .map(|(_, local)| *local)
        .filter(|local| !local.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

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
    /// The class of a citation node: one concordance row's lift onto one
    /// unit. Every citation node states it, whatever the row lifted, so
    /// a node that reifies nothing still says what it is.
    pub citation_class: String,
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
    /// The unit a citation node is an edge of: the back-edge every
    /// citation node states, whatever its row lifted. It is what makes a
    /// row that named sources and no anchors reachable from the unit it
    /// lifted onto — such a node reifies nothing, so without it nothing
    /// in the graph would point at it at all.
    pub in_unit: String,
    /// Datatype of a digest literal (`sha256:<hex>`).
    pub dt_digest: String,
    /// Datatype of a media type literal.
    pub dt_media: String,
    /// Datatype of a profile literal (`<name>:<contract id hex>`).
    pub dt_profile: String,
    /// Datatype of a heading or title literal. A derived vocabulary
    /// names it `headingText`, never `heading`: the bare name is the
    /// predicate [`Self::heading`], and one IRI cannot be described as
    /// both a property and a datatype.
    pub dt_heading: String,
    /// Datatype of a lineage literal. A derived vocabulary names it
    /// `lineagePath`, never `lineage`, for the reason
    /// [`Self::dt_heading`] gives — and because the literal *is* a path:
    /// the heading stack joined with ` > `.
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
            citation_class: iri("Citation"),
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
            in_unit: iri("unit"),
            dt_digest: iri("digest"),
            dt_media: iri("media"),
            dt_profile: iri("profile"),
            dt_heading: iri("headingText"),
            dt_lineage: iri("lineagePath"),
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
    fn fields(&self) -> [(&'static str, &str); 36] {
        [
            ("Document", &self.document_class),
            ("Section", &self.section_class),
            ("Movement", &self.movement_class),
            ("Unit", &self.unit_class),
            ("Citation", &self.citation_class),
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
            ("unit", &self.in_unit),
            ("digest", &self.dt_digest),
            ("media", &self.dt_media),
            ("profile", &self.dt_profile),
            ("headingText", &self.dt_heading),
            ("lineagePath", &self.dt_lineage),
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
            .zip(TERMS)
            .all(|((_, iri), (_, local))| *iri == format!("{base}{local}"));
        derived.then_some(base)
    }

    /// The vocabulary as the stage description states it: the base and
    /// the terms derived under it when every IRI derives from one, else
    /// one line per IRI.
    ///
    /// The compact form states **both** facts because the base alone
    /// does not determine the IRIs. The local names are this crate's,
    /// and a producer that derived other ones under the same base — as
    /// this crate itself did when the datatype of a heading was
    /// `heading` rather than `headingText` — would emit a different
    /// graph. These bytes are the preimage of
    /// [`Profile::contract_id`], which promises that cannot happen
    /// unseen, so the names are written out and a change to any of them
    /// re-mints the id. The explicit form needs no such line: it writes
    /// every IRI out already.
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
            |base| format!("vocabulary {base}\nterms {}\n", derived_terms()),
        )
    }
}

/// The slicing profile: its name and version, its vocabulary, the byte
/// bound and overlap that act only on an oversize unit, and the
/// optional canon IRI base for concordance anchors.
///
/// **Every** one of them is inside [`Profile::contract_id`], the canon
/// base included: a declared base flips every citation object from a
/// typed literal to a minted IRI, so a profile that declared one and a
/// profile that did not emit different bytes, and an id they shared
/// would be a claim neither could keep. Only the clauses that decide
/// where a unit *starts and ends* are inside
/// [`Profile::chunking_id`] — see [`Profile::chunking_bytes`] and
/// [`Profile::emission_bytes`] for the two preimages and why they are
/// two.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    /// The profile's declared name, the first line of its identity. It
    /// is a line of [`Self::chunking_bytes`], so it carries no control
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

    /// The **chunking preimage**: the clauses that decide where a unit
    /// starts and ends and what bytes are in it, and nothing else, one
    /// fact per line — the profile's name and version, the dialect
    /// grammar that opens and closes a unit, and the split law with its
    /// two constants.
    ///
    /// This is the preimage of [`Self::chunking_id`] and the parameters
    /// of [`Self::purremb_chunking_stage`], and it is deliberately
    /// *narrower* than the law. A consumer that embedded this document's
    /// units holds vectors addressed by the chunking id; those vectors
    /// are stale exactly when a unit's bytes move, and at no other time.
    /// Renaming a predicate, swapping a whole vocabulary or declaring a
    /// canon base moves not one boundary, so none of them belongs here
    /// and none of them costs a consumer a re-embedding. What is here is
    /// what re-cuts the text.
    ///
    /// The concordance law is outside it for the same reason, with one
    /// clause excepted: a `|` line closes the open unit and is the
    /// content of no unit *whether or not a concordance is in force*, so
    /// adding or removing the trigger heading cannot move a boundary.
    /// That clause is stated here; the rest of §4 — what a row lifts,
    /// how its cells are read, how an anchor is minted — decides only
    /// what is emitted, and is stated in [`Self::emission_bytes`].
    ///
    /// The preimage is line-oriented and unframed: a newline ends one
    /// fact and begins the next, and no field is length-prefixed or
    /// quoted. That is why [`Self::name`] is refused a control
    /// character — a newline inside the name would state further facts
    /// of the law rather than name it, and two profiles could then
    /// describe themselves the same way. The refusal is a bound of the
    /// identity format, not a taste in names.
    #[must_use]
    pub fn chunking_bytes(&self) -> Vec<u8> {
        format!(
            "{name}\n\
             version {version}\n\
             section ATX ^#{{1,6}}[ \\t] ; title trimmed, trailing # trimmed ; seven hashes are prose\n\
             movement ^\u{2042}[ \\t] ; name trimmed, one surrounding * or _ pair removed\n\
             unit verse ^\\d+\\.[ \\t] ; the number is a u64 or the line is prose\n\
             unit paragraph blank-line ; rule ^(-{{3,}}|\\*{{3,}})$ and table row ^\\| close a unit\n\
             a table row is the content of no unit, a concordance in force or not\n\
             structure is read on a line's trimmed text ; CRLF states its LF twin's structure\n\
             a heading, a movement or a verse is read behind up to 3 leading spaces\n\
             4 or more leading spaces, or a tab in that run, is no marker but ordinary content\n\
             U+FEFF at byte zero is encoding, elsewhere content ; every span counts the document's own bytes\n\
             max_bytes {max_bytes}\n\
             overlap {overlap}\n\
             split newline > scalar boundary ; overlap snaps backward to newline\n\
             the cut lands on that newline and no piece carries it\n\
             with no newline the cut is the last scalar boundary at or before the bound\n\
             a continuation snaps backward to a line start, else to a scalar boundary, never before the unit's start\n\
             never inside a scalar ; never across a heading\n",
            name = self.name,
            version = self.version,
            max_bytes = self.max_bytes,
            overlap = self.overlap,
        )
        .into_bytes()
    }

    /// The **emission preimage**: [`Self::chunking_bytes`] verbatim,
    /// then every further clause that decides a byte of the emitted
    /// graph — the vocabulary, the canon base, the section-level rule,
    /// the concordance lift, the identity formulas and the emission law.
    ///
    /// This is the preimage of [`Self::contract_id`], and it states the
    /// whole law because that id is the handle a consumer keeps: **two
    /// runs that agree on it must agree on every byte they emit**. A
    /// clause left out of this description is a clause a producer could
    /// change while the id stood still, and a consumer holding that id
    /// would have no way to learn of it. So the emission law is in here
    /// beside the split law: extending the vocabulary or moving the
    /// reification shape re-mints the id, which is the design, not a
    /// cost.
    ///
    /// The canon base is in here by name, and its **absence** is stated
    /// rather than implied: a profile that declares none writes `canon
    /// base none`, so the line is present either way and no reader has
    /// to infer a missing fact from a missing line. It has to be here
    /// because it is not a garnish on the graph — under a declared base
    /// every citation object is a minted IRI and under none it is a
    /// typed literal, and two profiles that differed only there emitted
    /// different bytes while sharing an id, which is the one thing this
    /// id promises not to do.
    ///
    /// The vocabulary is in here as the base *and* the local names
    /// derived under it, for the same reason: a base names a namespace,
    /// not a term set, and the names are this crate's rather than the
    /// caller's. Writing them out is what makes the id notice a renamed
    /// or an added term — the change that moves a byte of the graph and
    /// not one unit boundary.
    ///
    /// The whole of [`Self::chunking_bytes`] is a **prefix** of these
    /// bytes, which is the relation the two ids rest on: the emission
    /// preimage cannot drop a chunking clause, so an emission id can
    /// never stand still while a boundary moves.
    ///
    /// It is line-oriented and unframed for the same reasons, and under
    /// the same bound on the name. The canon base needs no such bound:
    /// an admitted profile's base is an absolute IRI, and the IRI
    /// grammar admits no control character, so no base can state a
    /// further fact of the law.
    #[must_use]
    pub fn emission_bytes(&self) -> Vec<u8> {
        let mut bytes = self.chunking_bytes();
        bytes.extend_from_slice(
            format!(
                "{vocabulary}\
                 {canon_base}\
                 a movement's level is one under the nearest heading ; movements are siblings\n\
                 concordance section heading Concordance, ASCII case-insensitive\n\
                 a table row lifts when a concordance section is in force at any depth, by containment\n\
                 concordance row | verses | canon sources | anchors |\n\
                 cells are delimited by unescaped | only ; \\| is a pipe, \\\\ is a backslash, any other \\ is content\n\
                 verse range n, n-n, or n\u{2013}n ; both endpoints u64, the first at or under the last\n\
                 a cell lifts its backticked names only, in the order written\n\
                 a row that lifts nothing here and a row too malformed to read are data, never a refusal\n\
                 anchor lift is base ++ anchor ; absolute, and under the base by relativization\n\
                 unit id H(kind, source id, chunking id, byte start, byte end, alg, alg(span))\n\
                 section id H(kind, source id, chunking id, heading span, alg(heading line))\n\
                 citation id H(kind, source id, chunking id, row span, alg(row line, unit id, reified terms))\n\
                 alg {DIGEST_ALGORITHM}\n\
                 emit document type, sourceDigest, mediaType, byteLength, sliceProfile, title\n\
                 emit section type, document, parent, level, ordinal, heading, byteStart, byteEnd\n\
                 emit unit type, text, document, section, ordinal, verse, lineage, continues\n\
                 emit unit byteStart, byteEnd, scalarStart, scalarEnd, contentDigest\n\
                 emit unit cites anchor, once per anchor of every row that lifted onto it\n\
                 emit citation type and citation unit for every citation node, whatever its row lifted\n\
                 emit citation rdf:reifies <<( unit cites anchor )>> and citation canonSource path\n\
                 emit an offset as xsd:integer and a content digest as lowercase hex xsd:hexBinary\n\
                 emit one claim per node as N-Triples lines, sorted bytewise and de-duplicated\n\
                 escaping is purrdf-core's canonical writer ; C0 and DEL as \\uXXXX\n",
                vocabulary = self.vocabulary.stage_lines(),
                canon_base = self.canon_base.as_ref().map_or_else(
                    || "canon base none\n".to_owned(),
                    |base| format!("canon base {base}\n"),
                ),
            )
            .as_bytes(),
        );
        bytes
    }

    /// The identity of the **chunking law alone**, derived over
    /// [`Self::chunking_bytes`]: the id that enters every node preimage
    /// ([`unit_iri`](crate::unit_iri) and its siblings) and the id a
    /// `.purremb` consumer's embeddings are addressed under.
    ///
    /// It is the node preimage's third field rather than
    /// [`Self::contract_id`] on purpose. A node's identity answers *what
    /// this is*: this span of these bytes of this document, cut by this
    /// law. Declaring a canon base, or renaming a predicate, says
    /// nothing about what the unit is — it says more about it, in more
    /// terms — and if the emission id were the field, adding one
    /// citation vocabulary would re-mint every unit and section in the
    /// corpus and orphan every reference a consumer had already stored.
    /// With the chunking id there, the two graphs describe the *same*
    /// nodes and merge, which is what RDF is for; the `sliceProfile`
    /// literal on the document node still tells the two laws apart.
    ///
    /// A citation node is the one node whose identity has to notice the
    /// canon base, because what it reifies changes with it — and it
    /// notices it where it belongs, in the minted anchors inside its own
    /// content digest ([`citation_iri`](crate::citation_iri)), not in a
    /// profile id shared with nodes that did not change.
    #[must_use]
    pub fn chunking_id(&self) -> ChunkingContractId {
        derive_chunking_contract_id(&self.chunking_bytes())
    }

    /// The profile's identity under the chunking-contract domain of
    /// `purrdf-core`: **this crate's own law id**, derived over the
    /// line-oriented preimage of [`Self::emission_bytes`], and so over
    /// the whole law — the canon base among it.
    ///
    /// # What it promises
    ///
    /// Two runs that agree on this id agree on every byte they emit.
    /// That is the whole of its use, and it is why the preimage is the
    /// emission preimage and not the chunking one: a producer could
    /// otherwise rename a term or declare a canon base, emit a different
    /// graph, and still hand a consumer the id it was handed before.
    ///
    /// # It is not a PURREMB family chunking id
    ///
    /// The type is shared with PURREMB and the id is not. A PURREMB
    /// [`EmbeddingFamily`](purrdf_core::embedding::EmbeddingFamily)
    /// derives its `chunking_id` over
    /// [`AppliedStage::canonical_bytes`], which is TLV-framed: tagged,
    /// length-delimited, eight-byte aligned. This id is derived over an
    /// unframed run of newline-separated lines, and over a *longer* law
    /// than the stage carries besides. The preimages cannot coincide, so
    /// **no family contract can ever reproduce this id**, and the ids of
    /// one profile are always three different values — which
    /// [`Self::purremb_chunking_stage`] exists to make usable rather
    /// than merely true.
    ///
    /// Wire this id where the *law* is meant: the `sliceProfile`
    /// literal on the document node, a cache key over which law emitted
    /// which graph. Wire [`Self::chunking_id`] where a *node* is meant,
    /// and [`Self::purremb_chunking_stage`] into a family's `chunking`
    /// stage. A `.purremb` consumer that writes this id into a family
    /// expecting a stage id has named an id that family's own contract
    /// cannot derive, and every chunk target checked against the family
    /// will disagree with it.
    #[must_use]
    pub fn contract_id(&self) -> ChunkingContractId {
        derive_chunking_contract_id(&self.emission_bytes())
    }

    /// The profile's **chunking law** as a PURREMB chunking stage: the
    /// identity an
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
    /// * `digest`: the SHA-256 of [`Self::chunking_bytes`], a manifest
    ///   digest of the exact chunking law applied, which is the whole of
    ///   what this stage *is*.
    /// * `parameter_encoding`: [`PURREMB_PARAMETER_ENCODING`].
    /// * `parameters`: [`Self::chunking_bytes`], verbatim — the clauses
    ///   that decide where a unit starts and ends, and only those.
    ///
    /// A chunking stage carries the chunking law and nothing else, which
    /// is what makes it honest labelling rather than a label. The
    /// parameters are the exact bytes of [`Self::chunking_bytes`], so a
    /// family's derived id moves when a boundary could move and stands
    /// still when it could not: a consumer's vectors survive a renamed
    /// predicate and a newly declared canon base, and are invalidated by
    /// a changed bound, which is the truth about them. The wider law is
    /// not lost — it is in [`Self::contract_id`], the id the graph
    /// itself states — and because [`Self::chunking_bytes`] is a prefix
    /// of [`Self::emission_bytes`], this stage can never stand still
    /// while a chunk boundary moves.
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
    /// handed. It is still not [`Self::chunking_id`], which is derived
    /// over the same bytes unframed: the framing differs, so the values
    /// differ, and the node preimage's id is never a family's id either.
    /// [`Self::contract_id`] stays where the graph states it and never
    /// enters a family.
    #[must_use]
    pub fn purremb_chunking_stage(&self) -> AppliedStage {
        let parameters = self.chunking_bytes();
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

    /// The literal recorded on the document node: `<name>:<hex>`, the
    /// hex of [`Self::contract_id`] — the id that answers for every byte
    /// of the graph the literal sits in, canon base included.
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
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;

    /// The property that makes the designated namespace describable:
    /// every IRI a derived vocabulary states is written in **one** role,
    /// so no OWL or SHACL description of it has to call one IRI a
    /// property and a datatype at once.
    ///
    /// It is asked of the whole term set rather than of the two terms
    /// that once doubled up (`heading` and `lineage` were each a
    /// predicate and a datatype), so a term added later that reuses a
    /// name fails here — in this crate, on the run that added it —
    /// rather than in a reasoner somebody else is holding.
    #[test]
    fn every_derived_iri_wears_exactly_one_role() {
        for base in ["urn:test:", STANDARD_NAMESPACE] {
            let vocabulary = Vocabulary::under(base).expect("a vocabulary");
            let mut roles: BTreeMap<&str, BTreeSet<TermRole>> = BTreeMap::new();
            let mut names: BTreeSet<&str> = BTreeSet::new();
            for ((field, iri), (role, local)) in vocabulary.fields().into_iter().zip(TERMS) {
                assert_eq!(
                    field, local,
                    "the field table and the term table name the same term at each position"
                );
                assert_eq!(
                    iri,
                    format!("{base}{local}"),
                    "and the field carries that term derived under the base"
                );
                roles.entry(iri).or_default().insert(role);
                names.insert(local);
            }
            assert_eq!(
                names.len(),
                TERMS.len(),
                "every local name of the set is its own"
            );
            for (iri, wears) in &roles {
                assert_eq!(
                    wears.len(),
                    1,
                    "{iri} is written in more than one role: {wears:?}"
                );
            }
            assert_eq!(
                roles.len(),
                TERMS.len(),
                "and every term of the set is its own IRI"
            );
        }
    }

    #[test]
    fn a_vocabulary_under_one_base_states_itself_as_that_base() {
        let v = Vocabulary::under("urn:test:").expect("a vocabulary");
        assert_eq!(v.common_base(), Some("urn:test:"));
        assert_eq!(
            v.stage_lines(),
            format!("vocabulary urn:test:\nterms {}\n", derived_terms())
        );
        // The terms line is the term set itself, so a renamed local name
        // is a different law and a different id.
        assert!(v.stage_lines().contains(" ordinal heading byteStart "));
        assert!(
            v.stage_lines()
                .ends_with(" profile headingText lineagePath anchor path\n")
        );
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
            format!(
                "vocabulary {STANDARD_NAMESPACE}\nterms {}\n",
                derived_terms()
            )
        );
    }
}
