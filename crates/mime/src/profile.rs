// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_core::{BaseIri, ContentDigest};
use purrdf_hash::{Domain, frame::frame_le};

use crate::MimeError;

const PROFILE_DOMAIN: Domain = Domain::new(b"purrdf-mime/profile/v1");
const DOCUMENT_DOMAIN: Domain = Domain::new(b"purrdf-mime/document/v1");

/// An offered standard namespace, used only through explicit selection.
pub const STANDARD_NAMESPACE: &str = "https://w3id.org/purrdf/mime#";

pub(crate) const TERMS: [&str; 36] = [
    "Span",
    "Segment",
    "Parameter",
    "Document",
    "Part",
    "Header",
    "Structure",
    "Problem",
    "source",
    "sourceDigest",
    "byteLength",
    "profile",
    "document",
    "byteStart",
    "byteEnd",
    "verbatim",
    "part",
    "parent",
    "ordinal",
    "occurrence",
    "name",
    "value",
    "segment",
    "bodyStart",
    "bodyEnd",
    "mediaType",
    "parameter",
    "parameterName",
    "parameterValue",
    "transferEncoding",
    "kind",
    "related",
    "child",
    "digest",
    "decodedDigest",
    "depth",
];

/// Explicit MIME vocabulary. No codec entry point supplies a fallback namespace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vocabulary {
    base: String,
    terms: [String; TERMS.len()],
}

impl Vocabulary {
    /// Derive this codec's declared local names under a caller-owned namespace.
    ///
    /// # Errors
    /// Refuses an invalid absolute IRI or a namespace not ending in slash/hash.
    pub fn under(base: impl Into<String>) -> Result<Self, MimeError> {
        let base = base.into();
        BaseIri::parse(&base)?;
        if !base.ends_with(['#', '/']) {
            return Err(MimeError::Profile("vocabulary must end in # or /"));
        }
        let terms = TERMS.map(|local| format!("{base}{local}"));
        Ok(Self { base, terms })
    }

    /// Explicitly select the standard vocabulary shipped with this codec.
    ///
    /// # Errors
    /// Returns namespace validation failures, with no implicit replacement.
    pub fn standard() -> Result<Self, MimeError> {
        Self::under(STANDARD_NAMESPACE)
    }

    /// The exact caller-selected namespace.
    pub fn base(&self) -> &str {
        &self.base
    }

    /// Resolve a documented codec term without inventing unknown terms.
    ///
    /// # Errors
    /// Refuses a local name absent from the shipped vocabulary.
    pub fn term(&self, local: &str) -> Result<&str, MimeError> {
        let index = TERMS
            .iter()
            .position(|name| *name == local)
            .ok_or(MimeError::Profile("unknown vocabulary term"))?;
        Ok(&self.terms[index])
    }

    pub(crate) fn iri(&self, local: &str) -> &str {
        self.term(local).expect("shipped MIME term is declared")
    }
}

/// Caller-declared maxima; None means memory is the only limit for that resource.
/// There are no compiled message, header, part, depth or line-length ceilings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Whole source bytes.
    pub source_bytes: Option<u64>,
    /// All header occurrences across all parts.
    pub headers: Option<u64>,
    /// All part occurrences, including the root and nested messages.
    pub parts: Option<u64>,
    /// Nesting depth; the root has depth zero.
    pub depth: Option<u64>,
    /// Physical line payload bytes, excluding its line ending.
    pub line_bytes: Option<u64>,
}

impl Limits {
    /// Explicitly select memory-only admission for every resource.
    pub const fn unbounded() -> Self {
        Self {
            source_bytes: None,
            headers: None,
            parts: None,
            depth: None,
            line_bytes: None,
        }
    }

    pub(crate) fn check(
        limit: Option<u64>,
        actual: u64,
        resource: &'static str,
    ) -> Result<(), MimeError> {
        if let Some(limit) = limit.filter(|limit| actual > *limit) {
            Err(MimeError::Limit { resource, limit })
        } else {
            Ok(())
        }
    }
}

/// Immutable explicit profile whose identity includes every caller-selected knob.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    name: String,
    vocabulary: Vocabulary,
    limits: Limits,
    identity: ContentDigest,
}

impl Profile {
    /// Select a vocabulary and all resource policy; no defaults are inserted.
    ///
    /// # Errors
    /// Refuses an empty caller profile name.
    pub fn new(
        name: impl Into<String>,
        vocabulary: Vocabulary,
        limits: Limits,
    ) -> Result<Self, MimeError> {
        let name = name.into();
        if name.is_empty() {
            return Err(MimeError::Profile("profile name is empty"));
        }
        let mut preimage = PROFILE_DOMAIN.as_bytes().to_vec();
        frame_le(&mut preimage, name.as_bytes());
        frame_le(&mut preimage, vocabulary.base().as_bytes());
        for value in [
            limits.source_bytes,
            limits.headers,
            limits.parts,
            limits.depth,
            limits.line_bytes,
        ] {
            match value {
                None => preimage.push(0),
                Some(maximum) => {
                    preimage.push(1);
                    preimage.extend_from_slice(&maximum.to_le_bytes());
                }
            }
        }
        let identity = ContentDigest::of(&preimage);
        Ok(Self {
            name,
            vocabulary,
            limits,
            identity,
        })
    }

    /// Original caller profile label.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Original caller vocabulary.
    pub const fn vocabulary(&self) -> &Vocabulary {
        &self.vocabulary
    }
    /// Explicit caller limits, unchanged.
    pub const fn limits(&self) -> Limits {
        self.limits
    }
    /// Deterministic semantic profile identity.
    pub const fn identity(&self) -> ContentDigest {
        self.identity
    }

    pub(crate) fn document_id(&self, source: &str, digest: ContentDigest) -> String {
        let mut preimage = DOCUMENT_DOMAIN.as_bytes().to_vec();
        frame_le(&mut preimage, source.as_bytes());
        frame_le(&mut preimage, self.identity.to_hex().as_bytes());
        frame_le(&mut preimage, digest.to_hex().as_bytes());
        format!("urn:purrdf:mime:{}", ContentDigest::of(&preimage).to_hex())
    }
}
