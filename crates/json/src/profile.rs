// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use purrdf_core::{BaseIri, ContentDigest};

use crate::JsonError;

/// The explicitly offered standard vocabulary. [`Vocabulary::standard`] is an
/// opt-in constructor; no API silently supplies a namespace.
pub const STANDARD_NAMESPACE: &str = "https://w3id.org/purrdf/json#";

pub(crate) const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub(crate) const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
pub(crate) const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
pub(crate) const TERMS: [&str; 20] = [
    "Document",
    "Value",
    "Structure",
    "source",
    "sourceDigest",
    "byteLength",
    "profile",
    "document",
    "byteStart",
    "byteEnd",
    "text",
    "path",
    "kind",
    "occurrence",
    "parent",
    "ordinal",
    "digest",
    "verbatimText",
    "verbatim",
    "size",
];

/// Immutable vocabulary derived under a caller-supplied absolute namespace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vocabulary {
    base: String,
    terms: [String; TERMS.len()],
}

impl Vocabulary {
    /// Derive terms by appending the documented local names to `base`. A slash or
    /// hash terminator is required so terms and node identities remain distinct.
    pub fn under(base: impl Into<String>) -> Result<Self, JsonError> {
        let base = base.into();
        BaseIri::parse(&base).map_err(JsonError::Iri)?;
        if !base.ends_with(['#', '/']) {
            return Err(JsonError::Profile("namespace must end in # or /"));
        }
        let terms = TERMS.map(|term| format!("{base}{term}"));
        for term in &terms {
            BaseIri::parse(term).map_err(JsonError::Iri)?;
        }
        Ok(Self { base, terms })
    }

    /// Explicitly select the vocabulary offered with this codec.
    pub fn standard() -> Result<Self, JsonError> {
        Self::under(STANDARD_NAMESPACE)
    }

    /// Namespace from which this vocabulary derives every term.
    pub fn base(&self) -> &str {
        &self.base
    }

    /// Resolve one of this codec's documented local names. Unknown terms refuse.
    pub fn term(&self, local: &str) -> Result<&str, JsonError> {
        let index = TERMS
            .iter()
            .position(|term| *term == local)
            .ok_or(JsonError::Profile("unknown vocabulary local name"))?;
        Ok(&self.terms[index])
    }

    pub(crate) fn iri(&self, local: &str) -> &str {
        self.term(local)
            .expect("codec term belongs to the vocabulary")
    }
}

/// Resource bounds bound into the profile identity on every target. Values are
/// explicit; changing any of them changes the profile and document identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// Maximum input bytes; must be between 1 and `u32::MAX`.
    pub max_source_bytes: u64,
    /// Maximum number of value occurrences; must be between 1 and 1,048,576.
    pub max_values: u32,
    /// Maximum nested container depth; must be between 1 and 128.
    pub max_depth: u16,
    /// Maximum aggregate bytes of stored RFC 6901 paths; at most `u32::MAX`.
    pub max_pointer_bytes: u64,
}

impl Bounds {
    /// The explicitly named standard limits: 16 MiB source, 1,048,576 values,
    /// depth 128, and 64 MiB aggregate pointer text.
    pub const fn standard() -> Self {
        Self {
            max_source_bytes: 16 * 1024 * 1024,
            max_values: 1_048_576,
            max_depth: 128,
            max_pointer_bytes: 64 * 1024 * 1024,
        }
    }

    fn validate(self) -> Result<(), JsonError> {
        if self.max_source_bytes == 0
            || self.max_source_bytes > u64::from(u32::MAX)
            || self.max_values == 0
            || self.max_values > 1_048_576
            || self.max_depth == 0
            || self.max_depth > 128
            || self.max_pointer_bytes == 0
            || self.max_pointer_bytes > u64::from(u32::MAX)
        {
            return Err(JsonError::Profile(
                "bounds exceed the portable codec limits",
            ));
        }
        Ok(())
    }
}

/// An immutable, validated language and emission contract. This version accepts
/// RFC 8259 JSON encoded in UTF-8 without BOM, with paired surrogate escapes in
/// member names; scalar values retain all escape spellings, including unpaired
/// surrogate escapes. Duplicate object keys remain distinct occurrences.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    name: String,
    version: u32,
    vocabulary: Vocabulary,
    bounds: Bounds,
    identity: ContentDigest,
}

impl Profile {
    /// Bind a caller's nonempty profile name and nonzero version to this codec's
    /// language, vocabulary and resource limits.
    pub fn new(
        name: impl Into<String>,
        version: u32,
        vocabulary: Vocabulary,
        bounds: Bounds,
    ) -> Result<Self, JsonError> {
        bounds.validate()?;
        let name = name.into();
        if name.is_empty() || name.len() > 1024 || version == 0 {
            return Err(JsonError::Profile(
                "profile name must be 1..=1024 bytes and version nonzero",
            ));
        }
        let mut profile = Self {
            name,
            version,
            vocabulary,
            bounds,
            identity: ContentDigest::of(&[]),
        };
        profile.identity = ContentDigest::of(&profile.canonical_bytes());
        Ok(profile)
    }

    /// Explicitly choose the shipped language, vocabulary and bounds.
    pub fn standard() -> Result<Self, JsonError> {
        Self::new(
            "purrdf-json-ordered",
            1,
            Vocabulary::standard()?,
            Bounds::standard(),
        )
    }

    /// SHA-256 of the complete, framed canonical profile description.
    pub const fn identity(&self) -> ContentDigest {
        self.identity
    }
    /// Caller-supplied vocabulary.
    pub const fn vocabulary(&self) -> &Vocabulary {
        &self.vocabulary
    }
    /// Validated resource bounds.
    pub const fn bounds(&self) -> Bounds {
        self.bounds
    }
    /// Caller-assigned profile name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Caller-assigned profile version.
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// The exact portable identity preimage, as specified in `SPEC.md`.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for field in [
            "purrdf-json-profile-v1",
            "rfc8259-utf8-no-bom-scalar-member-names",
            "preorder-occurrences-rfc6901-duplicates",
            "scalar-lexical-cover-sha256-sized-containers-fragment-v-s",
            &self.name,
            self.vocabulary.base(),
        ] {
            frame(&mut bytes, field.as_bytes());
        }
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.bounds.max_source_bytes.to_le_bytes());
        bytes.extend_from_slice(&self.bounds.max_values.to_le_bytes());
        bytes.extend_from_slice(&self.bounds.max_depth.to_le_bytes());
        bytes.extend_from_slice(&self.bounds.max_pointer_bytes.to_le_bytes());
        for term in TERMS {
            frame(&mut bytes, term.as_bytes());
        }
        bytes
    }

    pub(crate) fn document_id(&self, source: &str, digest: &ContentDigest) -> String {
        let mut bytes = Vec::new();
        frame(&mut bytes, b"purrdf-json-document-v1");
        frame(&mut bytes, source.as_bytes());
        bytes.extend_from_slice(self.identity.as_bytes());
        bytes.extend_from_slice(digest.as_bytes());
        let namespace = self.vocabulary.base();
        let root = namespace
            .split_once('#')
            .map_or(namespace, |(prefix, _)| prefix);
        let separator = if root.ends_with('/') { "" } else { "/" };
        format!(
            "{root}{separator}document-{}",
            ContentDigest::of(&bytes).to_hex()
        )
    }
}

fn frame(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    output.extend_from_slice(bytes);
}
