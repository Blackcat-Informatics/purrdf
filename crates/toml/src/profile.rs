// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::TomlError;
use purrdf_core::{BaseIri, ContentDigest};
use purrdf_hash::{Domain, frame::frame_le};

/// The grammar accepted by this profile.
pub const SPECIFICATION_VERSION: &str = "1.1.0";
/// Explicitly offered namespace; never selected by an implicit default.
pub const STANDARD_NAMESPACE: &str = "https://w3id.org/purrdf/toml#";
const PROFILE_DOMAIN: Domain = Domain::new(b"purrdf-toml-profile-v1");
const DOCUMENT_DOMAIN: Domain = Domain::new(b"purrdf-toml-document-v1");

pub(crate) const TERMS: &[&str] = &[
    "Document",
    "Value",
    "Statement",
    "Key",
    "Structure",
    "source",
    "sourceDigest",
    "byteLength",
    "profile",
    "document",
    "byteStart",
    "byteEnd",
    "verbatim",
    "verbatimText",
    "path",
    "kind",
    "occurrence",
    "parent",
    "ordinal",
    "key",
    "value",
    "node",
    "header",
    "digest",
    "size",
    "OffsetDateTime",
    "LocalDateTime",
    "LocalDate",
    "LocalTime",
];

/// Absolute namespace under which every vocabulary term is derived.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vocabulary(String);

impl Vocabulary {
    /// Validate the caller namespace, ending with a slash or hash.
    pub fn under(base: impl Into<String>) -> Result<Self, TomlError> {
        let base = base.into();
        BaseIri::parse(&base)?;
        if !base.ends_with(['/', '#']) {
            return Err(TomlError::Profile("namespace must end with / or #"));
        }
        for local in TERMS {
            BaseIri::parse(&format!("{base}{local}"))?;
        }
        Ok(Self(base))
    }
    /// Deliberately opt into the shipped vocabulary.
    pub fn standard() -> Result<Self, TomlError> {
        Self::under(STANDARD_NAMESPACE)
    }
    /// The exact configured namespace.
    pub fn base(&self) -> &str {
        &self.0
    }
    /// Resolve a documented vocabulary local name; unknown names fail.
    pub fn term(&self, local: &str) -> Result<String, TomlError> {
        if !TERMS.contains(&local) {
            return Err(TomlError::Profile("unknown vocabulary term"));
        }
        Ok(format!("{}{local}", self.0))
    }
}

/// Explicit portable resource contract, included in profile identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// Maximum original source bytes.
    pub max_source_bytes: u64,
    /// Maximum semantic nodes and, separately, authored statements.
    pub max_nodes: u32,
    /// Maximum semantic nesting including dotted keys and array elements.
    pub max_depth: u32,
    /// Maximum retained model/path/key/scalar work in bytes.
    pub max_model_bytes: u64,
}

impl Bounds {
    /// Explicit standard contract: 16 MiB input, one million nodes, depth 256,
    /// and 256 MiB model admission. No parser uses native recursive descent.
    pub const fn standard() -> Self {
        Self {
            max_source_bytes: 16 * 1024 * 1024,
            max_nodes: 1_048_576,
            max_depth: 256,
            max_model_bytes: 256 * 1024 * 1024,
        }
    }
}

/// Immutable language, vocabulary, version and admission law.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    vocabulary: Vocabulary,
    bounds: Bounds,
    name: String,
    version: u32,
    identity: ContentDigest,
}

impl Profile {
    /// Bind caller-owned naming and resource policy to the shipped grammar.
    pub fn new(
        name: impl Into<String>,
        version: u32,
        vocabulary: Vocabulary,
        bounds: Bounds,
    ) -> Result<Self, TomlError> {
        let name = name.into();
        if name.is_empty()
            || name.len() > 1024
            || version == 0
            || bounds.max_source_bytes == 0
            || bounds.max_source_bytes > u64::from(u32::MAX)
            || bounds.max_nodes == 0
            || bounds.max_depth == 0
            || bounds.max_model_bytes == 0
            || bounds.max_model_bytes > u64::from(u32::MAX)
        {
            return Err(TomlError::Profile(
                "nonempty name, nonzero version and portable nonzero bounds required",
            ));
        }
        let mut bytes = Vec::new();
        for field in [
            PROFILE_DOMAIN.as_bytes(),
            b"toml-1.1.0-flat-occurrences-scalar-cover-v1",
            name.as_bytes(),
            vocabulary.base().as_bytes(),
        ] {
            frame_le(&mut bytes, field);
        }
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(&bounds.max_source_bytes.to_le_bytes());
        bytes.extend_from_slice(&bounds.max_nodes.to_le_bytes());
        bytes.extend_from_slice(&bounds.max_depth.to_le_bytes());
        bytes.extend_from_slice(&bounds.max_model_bytes.to_le_bytes());
        for local in TERMS {
            frame_le(&mut bytes, local.as_bytes());
        }
        Ok(Self {
            vocabulary,
            bounds,
            name,
            version,
            identity: ContentDigest::of(&bytes),
        })
    }
    /// Explicitly select the shipped profile and its bounds.
    pub fn standard() -> Result<Self, TomlError> {
        Self::new(
            "purrdf-toml-lossless",
            1,
            Vocabulary::standard()?,
            Bounds::standard(),
        )
    }
    /// Content identity of the full profile law.
    pub const fn identity(&self) -> ContentDigest {
        self.identity
    }
    /// Configured vocabulary.
    pub const fn vocabulary(&self) -> &Vocabulary {
        &self.vocabulary
    }
    /// Declared resource contract.
    pub const fn bounds(&self) -> Bounds {
        self.bounds
    }
    /// Caller profile name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Caller profile version.
    pub const fn version(&self) -> u32 {
        self.version
    }
    pub(crate) fn document_id(&self, source: &str, digest: ContentDigest) -> String {
        let mut bytes = Vec::new();
        frame_le(&mut bytes, DOCUMENT_DOMAIN.as_bytes());
        frame_le(&mut bytes, source.as_bytes());
        bytes.extend_from_slice(self.identity.as_bytes());
        bytes.extend_from_slice(digest.as_bytes());
        let base = self
            .vocabulary
            .base()
            .split_once('#')
            .map_or_else(|| self.vocabulary.base(), |(base, _)| base);
        format!(
            "{}/document-{}",
            base.trim_end_matches('/'),
            ContentDigest::of(&bytes).to_hex()
        )
    }
}
