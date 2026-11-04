// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::YamlError;
use purrdf_core::{BaseIri, ContentDigest};
use purrdf_hash::{Domain, frame::frame_le};
use purrdf_lex::{
    allocation::{Admission, Memory, Resident, StorageError},
    yaml::SyntaxStreamLimits,
};

/// Explicitly offered vocabulary; never supplied implicitly by analysis.
pub const STANDARD_NAMESPACE: &str = "https://w3id.org/purrdf/yaml#";
pub(crate) const SPECIFICATION: &str = "yaml-1.2.2-acyclic-lexical-stream-cover-v1";
const PROFILE_DOMAIN: Domain = Domain::new(b"purrdf-yaml-profile-v1");
const STREAM_DOMAIN: Domain = Domain::new(b"purrdf-yaml-stream-v1");
pub(crate) const MAX_IDENTIFIER_BYTES: usize = 4096;
pub(crate) const TERMS: &[&str] = &[
    "Stream",
    "Document",
    "Node",
    "Item",
    "Entry",
    "Trivia",
    "Directive",
    "Escape",
    "CoverSpan",
    "source",
    "sourceDigest",
    "lexBlake3",
    "byteLength",
    "profile",
    "profileName",
    "profileVersion",
    "vocabulary",
    "specification",
    "stream",
    "document",
    "ordinal",
    "byteStart",
    "byteEnd",
    "prologueStart",
    "prologueEnd",
    "startMarkerStart",
    "startMarkerEnd",
    "endMarkerStart",
    "endMarkerEnd",
    "root",
    "node",
    "item",
    "entry",
    "key",
    "value",
    "parent",
    "kind",
    "style",
    "text",
    "rawStart",
    "rawEnd",
    "rawText",
    "anchorStart",
    "anchorEnd",
    "anchorText",
    "tagStart",
    "tagEnd",
    "tagText",
    "tagKind",
    "tagName",
    "explicitKeyStart",
    "explicitKeyEnd",
    "blockStart",
    "blockEnd",
    "chomping",
    "indentIndicator",
    "contentIndent",
    "target",
    "attachment",
    "attachmentNode",
    "decoded",
    "escapeKind",
    "verbatim",
    "verbatimText",
    "sha256Digest",
    "blake3Digest",
];

/// Cached terms derived under an explicit caller namespace.
#[derive(Debug)]
pub struct Vocabulary {
    base: String,
    terms: Vec<String>,
}
impl Vocabulary {
    /// Validate and cache a namespace ending in slash or hash, at most 2,048 bytes.
    pub fn under(base: &str) -> Result<Self, YamlError> {
        if base.len() > 2048 || !base.ends_with(['/', '#']) {
            return Err(YamlError::Profile(
                "namespace must end in / or # and fit 2,048 bytes",
            ));
        }
        validate_iri(base)?;
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut terms = Vec::new();
        for local in TERMS {
            let text = memory.format(&format_args!("{base}{local}"))?;
            validate_iri(&text)?;
            memory.push(&mut terms, text)?;
        }
        Ok(Self {
            base: memory.string(base)?,
            terms,
        })
    }
    /// Explicitly opt into the shipped vocabulary.
    pub fn standard() -> Result<Self, YamlError> {
        Self::under(STANDARD_NAMESPACE)
    }
    /// The exact namespace.
    pub fn base(&self) -> &str {
        &self.base
    }
    /// A known vocabulary term; unknown locals are configuration failures.
    pub fn term(&self, local: &str) -> Result<&str, YamlError> {
        TERMS
            .iter()
            .position(|&term| term == local)
            .map(|index| self.terms[index].as_str())
            .ok_or(YamlError::Profile("unknown vocabulary local name"))
    }
}

/// Separate explicit phase ceilings, all included in profile identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// Complete source/occurrence/parser admission through Lex.
    pub syntax: SyntaxStreamLimits,
    /// Maximum distinct partition spans.
    pub max_cover_spans: usize,
    /// Partition, identity and coverage verification scratch, additional to Lex.
    pub max_partition_bytes: usize,
    /// Maximum projected or selected metadata statements.
    pub max_statements: usize,
    /// Sum of lexical/IRI component bytes over metadata statements, before deduplication.
    pub max_payload_bytes: usize,
    /// Additional RDF builder, index, temporary text and freeze storage.
    pub max_projection_bytes: usize,
    /// Selected metadata collection and shared cover reconstruction storage.
    pub max_decode_bytes: usize,
}
impl Bounds {
    /// Explicit standard bounds: 16 MiB input, one million occurrences/documents,
    /// depth 128, 128 MiB Lex, 64 MiB partition, two million RDF statements,
    /// and 512 MiB payload/projection plus 256 MiB decode storage.
    pub const fn standard() -> Self {
        Self {
            syntax: SyntaxStreamLimits {
                syntax: purrdf_lex::yaml::SyntaxLimits::DEFAULT,
                max_documents: 1 << 20,
            },
            max_cover_spans: (2 << 20) + 1,
            max_partition_bytes: 64 << 20,
            max_statements: 2 << 20,
            max_payload_bytes: 512 << 20,
            max_projection_bytes: 512 << 20,
            max_decode_bytes: 256 << 20,
        }
    }
}

/// Versioned, immutable source grammar, vocabulary and phase-admission law.
#[derive(Debug)]
pub struct Profile {
    name: String,
    version: u32,
    vocabulary: Vocabulary,
    bounds: Bounds,
    identity: ContentDigest,
}
impl Profile {
    /// Bind a nonempty caller name/version, vocabulary and portable phase bounds.
    /// Zero resource ceilings are legitimate refusal policies.
    pub fn new(
        name: &str,
        version: u32,
        vocabulary: Vocabulary,
        bounds: Bounds,
    ) -> Result<Self, YamlError> {
        if name.is_empty() || name.len() > 1024 || version == 0 {
            return Err(YamlError::Profile(
                "nonempty name within 1,024 bytes and nonzero version required",
            ));
        }
        let numbers = bounds.numbers();
        if numbers.iter().any(|&number| number > u64::from(u32::MAX)) {
            return Err(YamlError::Profile(
                "phase bounds must fit portable 32-bit counts",
            ));
        }
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut fields = Vec::new();
        for field in [
            PROFILE_DOMAIN.as_bytes(),
            SPECIFICATION.as_bytes(),
            name.as_bytes(),
            vocabulary.base().as_bytes(),
        ] {
            memory.push(&mut fields, field)?;
        }
        for &term in TERMS {
            memory.push(&mut fields, term.as_bytes())?;
        }
        let mut bytes = framed(&fields, &mut memory)?;
        let required = bytes
            .len()
            .checked_add(4 + numbers.len() * 8 + 1)
            .ok_or(StorageError::SizeOverflow)?;
        memory.reserve(&mut bytes, required)?;
        bytes.extend_from_slice(&version.to_le_bytes());
        for number in numbers {
            bytes.extend_from_slice(&number.to_le_bytes());
        }
        bytes.push(u8::from(bounds.syntax.syntax.aliases));
        let identity = ContentDigest::of(&bytes);
        Ok(Self {
            name: memory.string(name)?,
            version,
            vocabulary,
            bounds,
            identity,
        })
    }
    /// Explicitly select the shipped schema, vocabulary and standard bounds.
    pub fn standard() -> Result<Self, YamlError> {
        Self::new(
            "purrdf-yaml-source",
            1,
            Vocabulary::standard()?,
            Bounds::standard(),
        )
    }
    /// SHA-256 of the complete versioned profile law.
    pub const fn identity(&self) -> ContentDigest {
        self.identity
    }
    /// Caller profile name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Caller profile version.
    pub const fn version(&self) -> u32 {
        self.version
    }
    /// Cached caller vocabulary.
    pub const fn vocabulary(&self) -> &Vocabulary {
        &self.vocabulary
    }
    /// Exact phase bounds.
    pub const fn bounds(&self) -> Bounds {
        self.bounds
    }
    pub(crate) fn stream_id<S: Admission + ?Sized>(
        &self,
        source: &str,
        digest: ContentDigest,
        memory: &mut Memory<'_, S>,
    ) -> Result<String, YamlError> {
        let bytes = framed(
            &[
                STREAM_DOMAIN.as_bytes(),
                source.as_bytes(),
                self.identity.as_bytes(),
                digest.as_bytes(),
            ],
            memory,
        )?;
        let identity = ContentDigest::of(&bytes);
        memory.release_vec(bytes)?;
        Ok(memory.format(&format_args!("urn:purrdf:yaml:{identity}"))?)
    }
}

impl Bounds {
    fn numbers(self) -> [u64; 11] {
        [
            self.syntax.syntax.max_input_bytes as u64,
            self.syntax.syntax.max_allocation_bytes as u64,
            self.syntax.syntax.max_occurrences,
            self.syntax.syntax.max_depth as u64,
            self.syntax.max_documents as u64,
            self.max_cover_spans as u64,
            self.max_partition_bytes as u64,
            self.max_statements as u64,
            self.max_payload_bytes as u64,
            self.max_projection_bytes as u64,
            self.max_decode_bytes as u64,
        ]
    }
}
pub(crate) fn validate_iri(text: &str) -> Result<(), YamlError> {
    if text.len() > MAX_IDENTIFIER_BYTES {
        return Err(YamlError::Limit {
            resource: "identifier bytes",
            limit: MAX_IDENTIFIER_BYTES,
        });
    }
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    BaseIri::parse_with_memory(text, &mut memory)?;
    Ok(())
}
fn framed<S: Admission + ?Sized>(
    fields: &[&[u8]],
    memory: &mut Memory<'_, S>,
) -> Result<Vec<u8>, StorageError> {
    let length = fields.iter().try_fold(0_usize, |size, field| {
        size.checked_add(8)
            .and_then(|size| size.checked_add(field.len()))
            .ok_or(StorageError::SizeOverflow)
    })?;
    let mut bytes = Vec::new();
    memory.reserve(&mut bytes, length)?;
    for field in fields {
        frame_le(&mut bytes, field);
    }
    Ok(bytes)
}
