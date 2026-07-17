// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Canonical corpus and RDF 1.2 embedding subjects, sets, relations, and spans.

use core::cmp::Ordering;

use crate::{ContentDigest, RdfTextDirection};

use super::contract::{TlvWireType, push_tlv, validate_tlv_block};
use super::error::EmbeddingError;
use super::identity::{
    ChunkingContractId, FamilyId, TargetId, TargetIdentityDigest, TargetSetId,
    derive_relation_role_digest, derive_target_id, derive_target_identity_digest,
    derive_target_set_id,
};

/// Stable PURREMB v1 target-kind codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum TargetKind {
    /// External content-addressed corpus manifest.
    Corpus = 1,
    /// External UTF-8 document.
    Document = 2,
    /// Content-addressed span within a document.
    Chunk = 3,
    /// Complete certified RDF 1.2 dataset.
    RdfDataset = 16,
    /// Default or named RDF graph.
    RdfGraph = 17,
    /// RDF 1.2 statement in graph scope.
    RdfStatement = 18,
    /// RDF 1.2 reifier binding.
    RdfReifier = 19,
    /// RDF 1.2 statement annotation.
    RdfAnnotation = 20,
    /// RDF 1.2 term, including a recursive triple term.
    RdfTerm = 21,
    /// Caller-supplied extension kind.
    Extension = 0x8000_0000,
}

impl TargetKind {
    /// Stable wire code.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}

impl TryFrom<u32> for TargetKind {
    type Error = EmbeddingError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Corpus),
            2 => Ok(Self::Document),
            3 => Ok(Self::Chunk),
            16 => Ok(Self::RdfDataset),
            17 => Ok(Self::RdfGraph),
            18 => Ok(Self::RdfStatement),
            19 => Ok(Self::RdfReifier),
            20 => Ok(Self::RdfAnnotation),
            21 => Ok(Self::RdfTerm),
            0x8000_0000 => Ok(Self::Extension),
            value => Err(EmbeddingError::UnsupportedCode {
                field: "target kind",
                value,
            }),
        }
    }
}

/// Canonical target plus optional retained identity bytes and pack-local ordinal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingTarget {
    /// Stable target identity.
    pub id: TargetId,
    /// Digest of the complete canonical identity block.
    pub identity_digest: TargetIdentityDigest,
    /// Stable target kind.
    pub kind: TargetKind,
    /// Retained canonical identity block, or `None` for digest-only disclosure.
    pub canonical_identity: Option<Vec<u8>>,
    /// Verified source-local acceleration hint.
    pub source_local_ordinal: Option<u64>,
}

impl EmbeddingTarget {
    /// Builds a target from one canonical identity block.
    pub fn from_canonical_identity(
        kind: TargetKind,
        canonical_identity: Vec<u8>,
        retain_identity: bool,
        source_local_ordinal: Option<u64>,
    ) -> Result<Self, EmbeddingError> {
        validate_tlv_block(&canonical_identity, 0)?;
        validate_ordinal(kind, source_local_ordinal)?;
        let identity_digest = derive_target_identity_digest(kind.code(), &canonical_identity);
        let id = derive_target_id(kind.code(), identity_digest);
        Ok(Self {
            id,
            identity_digest,
            kind,
            canonical_identity: retain_identity.then_some(canonical_identity),
            source_local_ordinal,
        })
    }

    /// Builds a digest-only target received from a trusted canonical producer.
    pub fn from_digest(
        kind: TargetKind,
        identity_digest: TargetIdentityDigest,
        source_local_ordinal: Option<u64>,
    ) -> Result<Self, EmbeddingError> {
        validate_ordinal(kind, source_local_ordinal)?;
        Ok(Self {
            id: derive_target_id(kind.code(), identity_digest),
            identity_digest,
            kind,
            canonical_identity: None,
            source_local_ordinal,
        })
    }
}

impl Ord for EmbeddingTarget {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for EmbeddingTarget {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Corpus-manifest subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusTarget {
    /// Exact external corpus-manifest SHA-256.
    pub manifest_digest: ContentDigest,
    /// Caller media type or canonical format identifier.
    pub manifest_media_type: String,
    /// Digest of the stable caller logical corpus identifier.
    pub logical_id_digest: ContentDigest,
}

impl CorpusTarget {
    /// Converts this subject to a canonical embedding target.
    pub fn into_target(self, retain_identity: bool) -> Result<EmbeddingTarget, EmbeddingError> {
        ensure_nonempty(&self.manifest_media_type, "corpus manifest media type")?;
        let mut block = Vec::new();
        digest_field(&mut block, 1, self.manifest_digest.as_bytes())?;
        utf8_field(&mut block, 2, &self.manifest_media_type)?;
        digest_field(&mut block, 3, self.logical_id_digest.as_bytes())?;
        EmbeddingTarget::from_canonical_identity(TargetKind::Corpus, block, retain_identity, None)
    }
}

/// External UTF-8 document subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTarget {
    /// Parent corpus target.
    pub corpus_id: TargetId,
    /// SHA-256 of exact UTF-8 document bytes.
    pub content_digest: ContentDigest,
    /// Digest of the stable caller logical document identifier.
    pub logical_id_digest: ContentDigest,
    /// Caller media type or canonical format identifier.
    pub media_type: String,
    /// Exact document byte length.
    pub byte_length: u64,
    /// Unicode scalar-value count.
    pub scalar_count: u64,
}

impl DocumentTarget {
    /// Constructs document metadata from exact UTF-8 content.
    pub fn from_content(
        corpus_id: TargetId,
        logical_id_digest: ContentDigest,
        media_type: impl Into<String>,
        bytes: &[u8],
    ) -> Result<Self, EmbeddingError> {
        let text = core::str::from_utf8(bytes)
            .map_err(|_| EmbeddingError::InvalidUtf8("document content"))?;
        Ok(Self {
            corpus_id,
            content_digest: ContentDigest::of(bytes),
            logical_id_digest,
            media_type: media_type.into(),
            byte_length: u64::try_from(bytes.len()).expect("slice length fits u64"),
            scalar_count: u64::try_from(text.chars().count()).expect("char count fits u64"),
        })
    }

    /// Verifies exact external document bytes against this identity.
    pub fn verify_content(&self, bytes: &[u8]) -> Result<(), EmbeddingError> {
        let actual_length = u64::try_from(bytes.len()).expect("slice length fits u64");
        if actual_length != self.byte_length {
            return Err(EmbeddingError::ContentMismatch("document byte length"));
        }
        if ContentDigest::of(bytes) != self.content_digest {
            return Err(EmbeddingError::ContentMismatch("document SHA-256"));
        }
        let text = core::str::from_utf8(bytes)
            .map_err(|_| EmbeddingError::InvalidUtf8("document content"))?;
        let scalar_count = u64::try_from(text.chars().count()).expect("char count fits u64");
        if scalar_count != self.scalar_count {
            return Err(EmbeddingError::ContentMismatch("document scalar count"));
        }
        Ok(())
    }

    /// Converts this subject to a canonical embedding target.
    pub fn into_target(self, retain_identity: bool) -> Result<EmbeddingTarget, EmbeddingError> {
        ensure_nonempty(&self.media_type, "document media type")?;
        let mut block = Vec::new();
        digest_field(&mut block, 1, self.corpus_id.as_bytes())?;
        digest_field(&mut block, 2, self.content_digest.as_bytes())?;
        digest_field(&mut block, 3, self.logical_id_digest.as_bytes())?;
        utf8_field(&mut block, 4, &self.media_type)?;
        u64_field(&mut block, 5, self.byte_length)?;
        u64_field(&mut block, 6, self.scalar_count)?;
        EmbeddingTarget::from_canonical_identity(TargetKind::Document, block, retain_identity, None)
    }
}

/// Content-addressed document chunk with byte and scalar coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextChunkTarget {
    /// Parent document target.
    pub document_id: TargetId,
    /// Canonical chunking contract.
    pub chunking_id: ChunkingContractId,
    /// SHA-256 of exact chunk bytes.
    pub content_digest: ContentDigest,
    /// Inclusive byte start in the parent document.
    pub byte_start: u64,
    /// Exclusive byte end in the parent document.
    pub byte_end: u64,
    /// Inclusive Unicode scalar start in the parent document.
    pub scalar_start: u64,
    /// Exclusive Unicode scalar end in the parent document.
    pub scalar_end: u64,
}

impl TextChunkTarget {
    /// Creates and verifies a chunk against exact parent UTF-8 bytes.
    pub fn from_document(
        document_id: TargetId,
        chunking_id: ChunkingContractId,
        document_bytes: &[u8],
        byte_start: u64,
        byte_end: u64,
    ) -> Result<Self, EmbeddingError> {
        let document = core::str::from_utf8(document_bytes)
            .map_err(|_| EmbeddingError::InvalidUtf8("document content"))?;
        let start = usize::try_from(byte_start)
            .map_err(|_| EmbeddingError::ArithmeticOverflow("chunk byte start"))?;
        let end = usize::try_from(byte_end)
            .map_err(|_| EmbeddingError::ArithmeticOverflow("chunk byte end"))?;
        if start >= end || end > document_bytes.len() {
            return Err(EmbeddingError::InvalidSpan {
                context: "chunk bytes",
                offset: byte_start,
                length: byte_end.saturating_sub(byte_start),
            });
        }
        if !document.is_char_boundary(start) || !document.is_char_boundary(end) {
            return Err(EmbeddingError::ContentMismatch("chunk UTF-8 boundary"));
        }
        let scalar_start =
            u64::try_from(document[..start].chars().count()).expect("char count fits u64");
        let scalar_count =
            u64::try_from(document[start..end].chars().count()).expect("char count fits u64");
        Ok(Self {
            document_id,
            chunking_id,
            content_digest: ContentDigest::of(&document_bytes[start..end]),
            byte_start,
            byte_end,
            scalar_start,
            scalar_end: scalar_start + scalar_count,
        })
    }

    /// Verifies this span against exact parent UTF-8 bytes.
    pub fn verify_document(&self, document_bytes: &[u8]) -> Result<(), EmbeddingError> {
        let reconstructed = Self::from_document(
            self.document_id,
            self.chunking_id,
            document_bytes,
            self.byte_start,
            self.byte_end,
        )?;
        if &reconstructed != self {
            return Err(EmbeddingError::ContentMismatch(
                "chunk coordinates or digest",
            ));
        }
        Ok(())
    }

    /// Converts this subject to a canonical embedding target.
    pub fn into_target(self, retain_identity: bool) -> Result<EmbeddingTarget, EmbeddingError> {
        if self.byte_start >= self.byte_end || self.scalar_start >= self.scalar_end {
            return Err(EmbeddingError::InvalidSpan {
                context: "chunk",
                offset: self.byte_start,
                length: self.byte_end.saturating_sub(self.byte_start),
            });
        }
        let mut block = Vec::new();
        digest_field(&mut block, 1, self.document_id.as_bytes())?;
        digest_field(&mut block, 2, self.chunking_id.as_bytes())?;
        digest_field(&mut block, 3, self.content_digest.as_bytes())?;
        u64_field(&mut block, 4, self.byte_start)?;
        u64_field(&mut block, 5, self.byte_end)?;
        u64_field(&mut block, 6, self.scalar_start)?;
        u64_field(&mut block, 7, self.scalar_end)?;
        EmbeddingTarget::from_canonical_identity(TargetKind::Chunk, block, retain_identity, None)
    }
}

/// Certified RDF dataset subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RdfDatasetTarget {
    /// Independently certified RDFC SHA-256.
    pub rdfc_digest: [u8; 32],
}

impl RdfDatasetTarget {
    /// Converts this subject to a canonical embedding target.
    pub fn into_target(self, retain_identity: bool) -> Result<EmbeddingTarget, EmbeddingError> {
        let mut block = Vec::new();
        digest_field(&mut block, 1, &self.rdfc_digest)?;
        EmbeddingTarget::from_canonical_identity(
            TargetKind::RdfDataset,
            block,
            retain_identity,
            None,
        )
    }
}

/// Canonical RDF 1.2 term identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RdfTermTarget {
    /// Absolute IRI bytes.
    Iri(String),
    /// Dataset-scoped canonical blank-node label without `_:`.
    Blank {
        /// Certified dataset target.
        dataset_id: TargetId,
        /// Canonical blank label.
        canonical_label: String,
    },
    /// Lexically preserving RDF literal.
    Literal {
        /// Lexical form.
        lexical: String,
        /// Expanded datatype IRI.
        datatype: String,
        /// Lowercase language tag.
        language: Option<String>,
        /// RDF 1.2 base direction.
        direction: Option<RdfTextDirection>,
    },
    /// Recursive RDF 1.2 triple term by component target IDs.
    Triple {
        /// Subject term target.
        subject: TargetId,
        /// Predicate IRI target.
        predicate: TargetId,
        /// Object term target.
        object: TargetId,
    },
}

impl RdfTermTarget {
    /// Converts this term to a canonical embedding target.
    pub fn into_target(
        self,
        retain_identity: bool,
        source_local_ordinal: Option<u64>,
    ) -> Result<EmbeddingTarget, EmbeddingError> {
        let mut block = Vec::new();
        match self {
            Self::Iri(iri) => {
                validate_absolute_iri(&iri)?;
                u32_field(&mut block, 1, 1)?;
                utf8_field(&mut block, 2, &iri)?;
            }
            Self::Blank {
                dataset_id,
                canonical_label,
            } => {
                ensure_nonempty(&canonical_label, "canonical blank label")?;
                if canonical_label.starts_with("_:") {
                    return Err(EmbeddingError::Malformed(
                        "canonical blank label includes the _: prefix",
                    ));
                }
                u32_field(&mut block, 1, 2)?;
                digest_field(&mut block, 2, dataset_id.as_bytes())?;
                utf8_field(&mut block, 3, &canonical_label)?;
            }
            Self::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                ensure_nonempty(&datatype, "literal datatype IRI")?;
                validate_absolute_iri(&datatype)?;
                if let Some(language) = &language {
                    validate_language_tag(language)?;
                }
                if direction.is_some() && language.is_none() {
                    return Err(EmbeddingError::Malformed(
                        "literal direction requires a language tag",
                    ));
                }
                u32_field(&mut block, 1, 3)?;
                utf8_field(&mut block, 2, &lexical)?;
                utf8_field(&mut block, 3, &datatype)?;
                if let Some(language) = language {
                    utf8_field(&mut block, 4, &language)?;
                }
                let direction = match direction {
                    None => 0,
                    Some(RdfTextDirection::Ltr) => 1,
                    Some(RdfTextDirection::Rtl) => 2,
                };
                u32_field(&mut block, 5, direction)?;
            }
            Self::Triple {
                subject,
                predicate,
                object,
            } => {
                u32_field(&mut block, 1, 4)?;
                digest_field(&mut block, 2, subject.as_bytes())?;
                digest_field(&mut block, 3, predicate.as_bytes())?;
                digest_field(&mut block, 4, object.as_bytes())?;
            }
        }
        EmbeddingTarget::from_canonical_identity(
            TargetKind::RdfTerm,
            block,
            retain_identity,
            source_local_ordinal,
        )
    }
}

/// Default or named RDF graph target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RdfGraphTarget {
    /// Parent dataset target.
    pub dataset_id: TargetId,
    /// Named-graph term; `None` denotes the default graph.
    pub graph_name: Option<TargetId>,
}

impl RdfGraphTarget {
    /// Converts this graph to a canonical embedding target.
    pub fn into_target(self, retain_identity: bool) -> Result<EmbeddingTarget, EmbeddingError> {
        let mut block = Vec::new();
        digest_field(&mut block, 1, self.dataset_id.as_bytes())?;
        u32_field(&mut block, 2, u32::from(self.graph_name.is_some()))?;
        if let Some(graph_name) = self.graph_name {
            digest_field(&mut block, 3, graph_name.as_bytes())?;
        }
        EmbeddingTarget::from_canonical_identity(TargetKind::RdfGraph, block, retain_identity, None)
    }
}

/// RDF 1.2 statement target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RdfStatementTarget {
    /// Graph target.
    pub graph: TargetId,
    /// Subject term target.
    pub subject: TargetId,
    /// Predicate term target.
    pub predicate: TargetId,
    /// Object term target.
    pub object: TargetId,
}

impl RdfStatementTarget {
    /// Converts this statement to a canonical embedding target.
    pub fn into_target(
        self,
        retain_identity: bool,
        ordinal: Option<u64>,
    ) -> Result<EmbeddingTarget, EmbeddingError> {
        let block = four_digest_block(self.graph, self.subject, self.predicate, self.object)?;
        EmbeddingTarget::from_canonical_identity(
            TargetKind::RdfStatement,
            block,
            retain_identity,
            ordinal,
        )
    }
}

/// RDF 1.2 reifier-binding target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RdfReifierTarget {
    /// Graph target.
    pub graph: TargetId,
    /// Reified statement target.
    pub statement: TargetId,
    /// Reifier term target.
    pub reifier: TargetId,
}

impl RdfReifierTarget {
    /// Converts this binding to a canonical embedding target.
    pub fn into_target(
        self,
        retain_identity: bool,
        ordinal: Option<u64>,
    ) -> Result<EmbeddingTarget, EmbeddingError> {
        let mut block = Vec::new();
        digest_field(&mut block, 1, self.graph.as_bytes())?;
        digest_field(&mut block, 2, self.statement.as_bytes())?;
        digest_field(&mut block, 3, self.reifier.as_bytes())?;
        EmbeddingTarget::from_canonical_identity(
            TargetKind::RdfReifier,
            block,
            retain_identity,
            ordinal,
        )
    }
}

/// RDF 1.2 annotation target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RdfAnnotationTarget {
    /// Graph target.
    pub graph: TargetId,
    /// Reifier target.
    pub reifier: TargetId,
    /// Annotation predicate term target.
    pub predicate: TargetId,
    /// Annotation object term target.
    pub object: TargetId,
}

impl RdfAnnotationTarget {
    /// Converts this annotation to a canonical embedding target.
    pub fn into_target(
        self,
        retain_identity: bool,
        ordinal: Option<u64>,
    ) -> Result<EmbeddingTarget, EmbeddingError> {
        let block = four_digest_block(self.graph, self.reifier, self.predicate, self.object)?;
        EmbeddingTarget::from_canonical_identity(
            TargetKind::RdfAnnotation,
            block,
            retain_identity,
            ordinal,
        )
    }
}

/// Caller-defined extension target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionTarget {
    /// Stable extension-kind identifier.
    pub kind_identifier: String,
    /// Stable canonical payload-encoding identifier.
    pub payload_encoding: String,
    /// Canonical payload bytes.
    pub payload: Vec<u8>,
}

impl ExtensionTarget {
    /// Converts this extension to a canonical embedding target.
    pub fn into_target(self, retain_identity: bool) -> Result<EmbeddingTarget, EmbeddingError> {
        ensure_nonempty(&self.kind_identifier, "extension target kind identifier")?;
        ensure_nonempty(&self.payload_encoding, "extension target payload encoding")?;
        let mut block = Vec::new();
        utf8_field(&mut block, 1, &self.kind_identifier)?;
        utf8_field(&mut block, 2, &self.payload_encoding)?;
        bytes_field(&mut block, 3, &self.payload)?;
        digest_field(&mut block, 4, ContentDigest::of(&self.payload).as_bytes())?;
        EmbeddingTarget::from_canonical_identity(
            TargetKind::Extension,
            block,
            retain_identity,
            None,
        )
    }
}

/// Canonical, nonempty target row set shared by one or more matrices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSet {
    /// Stable target-set identity.
    pub id: TargetSetId,
    /// Strictly sorted target IDs in matrix row order.
    pub targets: Vec<TargetId>,
}

impl TargetSet {
    /// Sorts, validates, and derives a target set.
    pub fn new(mut targets: Vec<TargetId>) -> Result<Self, EmbeddingError> {
        if targets.is_empty() {
            return Err(EmbeddingError::Missing("target-set row"));
        }
        targets.sort_unstable();
        if targets.windows(2).any(|window| window[0] == window[1]) {
            return Err(EmbeddingError::Duplicate("target-set target"));
        }
        Ok(Self {
            id: derive_target_set_id(&targets),
            targets,
        })
    }

    /// O(1) row-to-target lookup.
    #[must_use]
    pub fn target(&self, row: usize) -> Option<TargetId> {
        self.targets.get(row).copied()
    }

    /// Allocation-free reverse lookup.
    #[must_use]
    pub fn row_for_target(&self, target: TargetId) -> Option<usize> {
        self.targets.binary_search(&target).ok()
    }
}

/// Built-in or caller-defined structural relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetRelation {
    /// Parent or statement-like endpoint.
    pub subject: TargetId,
    /// Child or component endpoint.
    pub object: TargetId,
    /// Relation kind.
    pub kind: RelationKind,
    /// Exact caller role for the extension kind.
    pub extension_role: Option<Vec<u8>>,
}

impl TargetRelation {
    /// Constructs a built-in relation.
    #[must_use]
    pub const fn builtin(subject: TargetId, kind: RelationKind, object: TargetId) -> Self {
        Self {
            subject,
            object,
            kind,
            extension_role: None,
        }
    }

    /// Constructs a caller-defined relation.
    pub fn extension(
        subject: TargetId,
        object: TargetId,
        role: Vec<u8>,
    ) -> Result<Self, EmbeddingError> {
        let text = core::str::from_utf8(&role)
            .map_err(|_| EmbeddingError::InvalidUtf8("extension relation role"))?;
        ensure_nonempty(text, "extension relation role")?;
        Ok(Self {
            subject,
            object,
            kind: RelationKind::Extension,
            extension_role: Some(role),
        })
    }

    /// Canonical role digest; built-in roles use zero.
    #[must_use]
    pub fn role_digest(&self) -> [u8; 32] {
        self.extension_role
            .as_deref()
            .map_or([0; 32], derive_relation_role_digest)
    }
}

impl Ord for TargetRelation {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.subject, self.kind, self.object, self.role_digest()).cmp(&(
            other.subject,
            other.kind,
            other.object,
            other.role_digest(),
        ))
    }
}

impl PartialOrd for TargetRelation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// PURREMB v1 structural relation kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum RelationKind {
    /// Corpus contains document.
    CorpusDocument = 1,
    /// Document contains chunk.
    DocumentChunk = 2,
    /// Dataset contains graph.
    DatasetGraph = 16,
    /// Graph contains statement.
    GraphStatement = 17,
    /// Statement subject term.
    StatementSubject = 18,
    /// Statement predicate term.
    StatementPredicate = 19,
    /// Statement object term.
    StatementObject = 20,
    /// Statement has reifier binding.
    StatementReifier = 21,
    /// Reifier binding uses term.
    ReifierTerm = 22,
    /// Reifier has annotation.
    ReifierAnnotation = 23,
    /// Annotation predicate term.
    AnnotationPredicate = 24,
    /// Annotation object term.
    AnnotationObject = 25,
    /// Named graph name term.
    GraphName = 26,
    /// Triple-term subject component.
    TripleTermSubject = 32,
    /// Triple-term predicate component.
    TripleTermPredicate = 33,
    /// Triple-term object component.
    TripleTermObject = 34,
    /// Caller-defined role.
    Extension = 0x8000_0000,
}

impl RelationKind {
    /// Stable wire code.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }
}

/// Family-scoped tokenizer span for a document or chunk target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TokenSpan {
    /// Embedding family whose tokenizer produced the span.
    pub family_id: FamilyId,
    /// Document or chunk target.
    pub target_id: TargetId,
    /// Inclusive token start in the full document tokenization.
    pub token_start: u64,
    /// Exclusive token end.
    pub token_end: u64,
    /// Actual token count presented to the model.
    pub model_input_token_count: u64,
    /// Whether input was left-truncated.
    pub left_truncated: bool,
    /// Whether input was right-truncated.
    pub right_truncated: bool,
    /// Whether the count includes special tokens.
    pub includes_special_tokens: bool,
}

impl TokenSpan {
    /// Validates token interval invariants.
    pub fn validate(self) -> Result<Self, EmbeddingError> {
        if self.token_start > self.token_end || self.model_input_token_count == 0 {
            return Err(EmbeddingError::InvalidSpan {
                context: "token span",
                offset: self.token_start,
                length: self.token_end.saturating_sub(self.token_start),
            });
        }
        Ok(self)
    }

    /// Stable v1 flag bits.
    #[must_use]
    pub fn flags(self) -> u32 {
        u32::from(self.left_truncated)
            | (u32::from(self.right_truncated) << 1)
            | (u32::from(self.includes_special_tokens) << 2)
    }
}

fn validate_ordinal(kind: TargetKind, ordinal: Option<u64>) -> Result<(), EmbeddingError> {
    if ordinal.is_some()
        && !matches!(
            kind,
            TargetKind::RdfTerm
                | TargetKind::RdfStatement
                | TargetKind::RdfReifier
                | TargetKind::RdfAnnotation
        )
    {
        return Err(EmbeddingError::Malformed(
            "source ordinal is not valid for this target kind",
        ));
    }
    if ordinal == Some(u64::MAX) {
        return Err(EmbeddingError::Malformed(
            "u64::MAX is reserved for an absent source ordinal",
        ));
    }
    if kind == TargetKind::RdfTerm && ordinal == Some(0) {
        return Err(EmbeddingError::Malformed(
            "RDF term source ordinals are one-based",
        ));
    }
    Ok(())
}

fn four_digest_block(
    first: TargetId,
    second: TargetId,
    third: TargetId,
    fourth: TargetId,
) -> Result<Vec<u8>, EmbeddingError> {
    let mut block = Vec::new();
    digest_field(&mut block, 1, first.as_bytes())?;
    digest_field(&mut block, 2, second.as_bytes())?;
    digest_field(&mut block, 3, third.as_bytes())?;
    digest_field(&mut block, 4, fourth.as_bytes())?;
    Ok(block)
}

fn validate_language_tag(language: &str) -> Result<(), EmbeddingError> {
    let mut subtags = language.split('-');
    let primary = subtags.next().unwrap_or_default();
    if primary.is_empty()
        || primary.len() > 8
        || !primary.bytes().all(|byte| byte.is_ascii_lowercase())
    {
        return Err(EmbeddingError::Malformed("invalid lowercase language tag"));
    }

    let mut private_use = primary == "x";
    let mut saw_subtag = false;
    let mut ends_with_private_marker = private_use;
    for subtag in subtags {
        saw_subtag = true;
        let alphanumeric = !subtag.is_empty()
            && subtag
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
        if !alphanumeric || (!private_use && subtag.len() > 8) {
            return Err(EmbeddingError::Malformed("invalid lowercase language tag"));
        }
        if subtag == "x" {
            private_use = true;
            ends_with_private_marker = true;
        } else {
            ends_with_private_marker = false;
        }
    }
    if (primary == "x" && !saw_subtag) || ends_with_private_marker {
        return Err(EmbeddingError::Malformed("invalid lowercase language tag"));
    }
    Ok(())
}

fn validate_absolute_iri(iri: &str) -> Result<(), EmbeddingError> {
    let parsed =
        purrdf_iri::parse(iri).map_err(|_| EmbeddingError::Malformed("invalid RDF IRI"))?;
    if parsed.scheme().is_none() {
        return Err(EmbeddingError::Malformed("RDF IRI is not absolute"));
    }
    Ok(())
}

fn ensure_nonempty(value: &str, field: &'static str) -> Result<(), EmbeddingError> {
    if value.is_empty() {
        return Err(EmbeddingError::Missing(field));
    }
    Ok(())
}

fn digest_field(output: &mut Vec<u8>, tag: u16, value: &[u8; 32]) -> Result<(), EmbeddingError> {
    push_tlv(output, tag, TlvWireType::Digest32, true, value)
}

fn utf8_field(output: &mut Vec<u8>, tag: u16, value: &str) -> Result<(), EmbeddingError> {
    push_tlv(output, tag, TlvWireType::Utf8, true, value.as_bytes())
}

fn bytes_field(output: &mut Vec<u8>, tag: u16, value: &[u8]) -> Result<(), EmbeddingError> {
    push_tlv(output, tag, TlvWireType::Bytes, true, value)
}

fn u32_field(output: &mut Vec<u8>, tag: u16, value: u32) -> Result<(), EmbeddingError> {
    push_tlv(output, tag, TlvWireType::U32, true, &value.to_le_bytes())
}

fn u64_field(output: &mut Vec<u8>, tag: u16, value: u64) -> Result<(), EmbeddingError> {
    push_tlv(output, tag, TlvWireType::U64, true, &value.to_le_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_and_digest_only_targets_have_the_same_id() {
        let corpus = CorpusTarget {
            manifest_digest: ContentDigest::of(b"manifest"),
            manifest_media_type: "application/example".to_owned(),
            logical_id_digest: ContentDigest::of(b"corpus-id"),
        };
        let retained = corpus.clone().into_target(true).expect("retained target");
        let digest_only =
            EmbeddingTarget::from_digest(retained.kind, retained.identity_digest, None)
                .expect("digest target");
        assert_eq!(retained.id, digest_only.id);
        assert!(retained.canonical_identity.is_some());
        assert!(digest_only.canonical_identity.is_none());
    }

    #[test]
    fn document_and_chunk_verify_external_utf8_content() {
        let corpus_id = TargetId::from_raw([1; 32]);
        let document_bytes = "alpha βeta gamma".as_bytes();
        let document = DocumentTarget::from_content(
            corpus_id,
            ContentDigest::of(b"doc-id"),
            "text/plain;charset=utf-8",
            document_bytes,
        )
        .expect("document");
        document
            .verify_content(document_bytes)
            .expect("content verifies");
        let start = "alpha ".len() as u64;
        let end = "alpha βeta".len() as u64;
        let chunk = TextChunkTarget::from_document(
            TargetId::from_raw([2; 32]),
            ChunkingContractId::from_raw([3; 32]),
            document_bytes,
            start,
            end,
        )
        .expect("chunk");
        chunk
            .verify_document(document_bytes)
            .expect("chunk verifies");
        assert_eq!(chunk.scalar_end - chunk.scalar_start, 4);
    }

    #[test]
    fn rdf_triple_terms_and_directional_literals_are_first_class() {
        let literal = RdfTermTarget::Literal {
            lexical: "chat".to_owned(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
            language: Some("fr".to_owned()),
            direction: Some(RdfTextDirection::Ltr),
        }
        .into_target(true, Some(1))
        .expect("literal target");
        let triple = RdfTermTarget::Triple {
            subject: TargetId::from_raw([1; 32]),
            predicate: TargetId::from_raw([2; 32]),
            object: literal.id,
        }
        .into_target(true, None)
        .expect("triple target");
        assert_ne!(literal.id, triple.id);
    }

    #[test]
    fn target_sets_sort_and_reject_duplicates() {
        let a = TargetId::from_raw([1; 32]);
        let b = TargetId::from_raw([2; 32]);
        let set = TargetSet::new(vec![b, a]).expect("set");
        assert_eq!(set.targets, vec![a, b]);
        assert_eq!(set.row_for_target(b), Some(1));
        assert!(TargetSet::new(vec![a, a]).is_err());
    }

    #[test]
    fn relation_sorting_includes_extension_role_digest() {
        let subject = TargetId::from_raw([1; 32]);
        let object = TargetId::from_raw([2; 32]);
        let a = TargetRelation::extension(subject, object, b"a".to_vec()).expect("role a");
        let b = TargetRelation::extension(subject, object, b"b".to_vec()).expect("role b");
        assert_ne!(a.role_digest(), b.role_digest());
        assert_ne!(a.cmp(&b), Ordering::Equal);
    }

    #[test]
    fn rdf_term_ordinals_are_one_based() {
        let error = EmbeddingTarget::from_digest(
            TargetKind::RdfTerm,
            TargetIdentityDigest::from_raw([4; 32]),
            Some(0),
        )
        .expect_err("zero is not a unified PackId");
        assert!(matches!(error, EmbeddingError::Malformed(_)));

        EmbeddingTarget::from_digest(
            TargetKind::RdfStatement,
            TargetIdentityDigest::from_raw([5; 32]),
            Some(0),
        )
        .expect("statement row ordinals are zero-based");
    }

    #[test]
    fn language_tags_use_lowercase_bcp47_shape() {
        for valid in ["en", "zh-hant", "en-us", "x-purrdf-private-extension"] {
            validate_language_tag(valid).expect("valid lowercase language tag");
        }
        for invalid in ["", "EN", "en--us", "en-abcdefghi", "x", "en-x"] {
            assert!(
                validate_language_tag(invalid).is_err(),
                "accepted {invalid:?}"
            );
        }
    }

    #[test]
    fn rdf_iris_must_be_absolute_and_well_formed() {
        RdfTermTarget::Iri("https://example.org/猫".to_owned())
            .into_target(true, None)
            .expect("absolute RFC 3987 IRI");
        assert!(
            RdfTermTarget::Iri("relative/path".to_owned())
                .into_target(true, None)
                .is_err()
        );
        assert!(
            RdfTermTarget::Iri("https://example.org/<bad>".to_owned())
                .into_target(true, None)
                .is_err()
        );
    }
}
