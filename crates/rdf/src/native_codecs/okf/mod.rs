// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native Open Knowledge Format (OKF) Markdown-bundle codec.
//!
//! OKF is a deterministic in-memory bundle of UTF-8 Markdown documents with YAML
//! frontmatter. This module deliberately owns no filesystem API: callers can map
//! [`OkfBundle`](crate::native_codecs::okf::OkfBundle) entries to directories, archives, browser
//! storage, or another transport without making the release crate non-wasm. The RDF vocabulary and
//! document base are mandatory caller configuration; PurRDF provides no namespace
//! default and mints no vocabulary IRI.

mod reader;
mod writer;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub use reader::lift_okf_bundle;
pub use writer::{OkfWriteOutcome, OkfWriter, write_okf_bundle};

use crate::LossLedger;

/// Maximum number of documents accepted by one in-memory bundle.
pub const MAX_OKF_DOCUMENTS: usize = 65_536;
/// Maximum UTF-8 byte length of one Markdown document.
pub const MAX_OKF_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
/// Maximum aggregate UTF-8 byte length of one bundle.
pub const MAX_OKF_BUNDLE_BYTES: usize = 256 * 1024 * 1024;
/// Maximum UTF-8 byte length of one relative bundle path.
pub const MAX_OKF_PATH_BYTES: usize = 4_096;
/// Maximum YAML-frontmatter byte length of one document.
pub const MAX_OKF_FRONTMATTER_BYTES: usize = 1024 * 1024;
/// Maximum number of YAML nodes in one document's frontmatter.
pub const MAX_OKF_YAML_NODES: usize = 65_536;
/// Maximum YAML nesting depth in one document's frontmatter.
pub const MAX_OKF_YAML_DEPTH: usize = 64;
/// Maximum number of Markdown links scanned from one document body.
pub const MAX_OKF_LINKS_PER_DOCUMENT: usize = 65_536;

const RESERVED_PROFILE_KEYS: &[&str] = &[
    "body",
    "json",
    "linkOccurrence",
    "linkText",
    "links",
    "path",
];

/// A typed hard failure from OKF configuration, parsing, lifting, or writing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OkfError {
    detail: String,
}

impl OkfError {
    pub(super) fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    /// The stable human-readable error detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for OkfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for OkfError {}

/// Mandatory caller-owned OKF vocabulary and frontmatter profile.
///
/// There is intentionally no [`Default`] implementation. The caller must choose
/// the namespace from which profile predicate IRIs are derived, the base used for
/// documents without an explicit `resource`, and the exact set of frontmatter keys
/// the application recognizes. `type` is always required; unrecognized authored
/// keys hard-fail instead of leaking into an invented vocabulary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OkfConfig {
    namespace: String,
    document_base_iri: String,
    recognized_keys: BTreeSet<String>,
    predicates: BTreeMap<String, String>,
    path_predicate: String,
    body_predicate: String,
    links_predicate: String,
    link_text_predicate: String,
    link_occurrence_predicate: String,
    json_datatype: String,
}

impl OkfConfig {
    /// Validate and construct a mandatory OKF profile.
    ///
    /// # Errors
    ///
    /// Returns [`OkfError`] when either IRI is not absolute, a namespace/base does
    /// not end in a safe joining delimiter (`#`, `/`, or `:`), `type` is absent,
    /// or a recognized key is unsafe/reserved.
    pub fn new<I, S>(
        namespace: impl Into<String>,
        document_base_iri: impl Into<String>,
        recognized_keys: I,
    ) -> Result<Self, OkfError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let namespace = namespace.into();
        let document_base_iri = document_base_iri.into();
        validate_join_base("OKF namespace", &namespace)?;
        validate_join_base("OKF document base IRI", &document_base_iri)?;

        let recognized_keys: BTreeSet<String> =
            recognized_keys.into_iter().map(Into::into).collect();
        if !recognized_keys.contains("type") {
            return Err(OkfError::new(
                "OKF recognized-frontmatter profile must include the required `type` key",
            ));
        }
        if recognized_keys.len() > MAX_OKF_YAML_NODES {
            return Err(OkfError::new(format!(
                "OKF recognized-frontmatter profile has {} keys; limit is {MAX_OKF_YAML_NODES}",
                recognized_keys.len()
            )));
        }

        let mut predicates = BTreeMap::new();
        for key in &recognized_keys {
            validate_profile_key(key)?;
            if RESERVED_PROFILE_KEYS.contains(&key.as_str()) {
                return Err(OkfError::new(format!(
                    "OKF frontmatter key `{key}` is reserved for the RDF bundle profile"
                )));
            }
            let iri = format!("{namespace}{key}");
            validate_absolute_iri("OKF predicate", &iri)?;
            predicates.insert(key.clone(), iri);
        }

        let system_iri = |local: &str| -> Result<String, OkfError> {
            let iri = format!("{namespace}{local}");
            validate_absolute_iri("OKF system predicate", &iri)?;
            Ok(iri)
        };
        let path_predicate = system_iri("path")?;
        let body_predicate = system_iri("body")?;
        let links_predicate = system_iri("links")?;
        let link_text_predicate = system_iri("linkText")?;
        let link_occurrence_predicate = system_iri("linkOccurrence")?;
        let json_datatype = system_iri("json")?;

        Ok(Self {
            namespace,
            document_base_iri,
            recognized_keys,
            predicates,
            path_predicate,
            body_predicate,
            links_predicate,
            link_text_predicate,
            link_occurrence_predicate,
            json_datatype,
        })
    }

    /// The caller-supplied vocabulary namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// The caller-supplied document base used when `resource` is absent.
    pub fn document_base_iri(&self) -> &str {
        &self.document_base_iri
    }

    /// Recognized frontmatter keys in deterministic lexical order.
    pub fn recognized_keys(&self) -> impl ExactSizeIterator<Item = &str> {
        self.recognized_keys.iter().map(String::as_str)
    }

    /// The configured predicate IRI for a recognized frontmatter key.
    pub fn predicate_iri(&self, key: &str) -> Option<&str> {
        self.predicates.get(key).map(String::as_str)
    }

    /// Profile predicate carrying a document's normalized bundle path.
    pub fn path_predicate(&self) -> &str {
        &self.path_predicate
    }

    /// Profile predicate carrying the authoritative Markdown body literal.
    pub fn body_predicate(&self) -> &str {
        &self.body_predicate
    }

    /// Profile predicate carrying a relative Markdown-link edge.
    pub fn links_predicate(&self) -> &str {
        &self.links_predicate
    }

    /// Annotation predicate carrying a Markdown link's visible text.
    pub fn link_text_predicate(&self) -> &str {
        &self.link_text_predicate
    }

    /// Annotation predicate carrying a Markdown link's one-based occurrence.
    pub fn link_occurrence_predicate(&self) -> &str {
        &self.link_occurrence_predicate
    }

    /// Datatype IRI used for canonical JSON representations of structured YAML.
    pub fn json_datatype(&self) -> &str {
        &self.json_datatype
    }

    pub(super) fn recognizes(&self, key: &str) -> bool {
        self.recognized_keys.contains(key)
    }

    pub(super) fn key_for_predicate<'a>(&'a self, iri: &str) -> Option<&'a str> {
        let local = iri.strip_prefix(&self.namespace)?;
        self.recognized_keys.get(local).map(String::as_str)
    }
}

/// A deterministic, validated in-memory OKF Markdown bundle.
///
/// Paths are normalized POSIX-relative `.md` names and iteration is lexical by
/// path. Documents are UTF-8 [`String`] values, so invalid body/frontmatter bytes
/// cannot enter the codec. Construction enforces the public resource limits.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OkfBundle {
    documents: BTreeMap<String, String>,
    total_bytes: usize,
}

impl OkfBundle {
    /// Construct an empty bundle.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a bundle from path/document pairs.
    ///
    /// # Errors
    ///
    /// Returns [`OkfError`] on a duplicate/unsafe path or a resource-limit breach.
    pub fn from_documents<I, P, D>(documents: I) -> Result<Self, OkfError>
    where
        I: IntoIterator<Item = (P, D)>,
        P: Into<String>,
        D: Into<String>,
    {
        let mut bundle = Self::new();
        for (path, document) in documents {
            bundle.insert(path, document)?;
        }
        Ok(bundle)
    }

    /// Insert one document while preserving all path and size invariants.
    ///
    /// # Errors
    ///
    /// Returns [`OkfError`] on a duplicate/unsafe path or a resource-limit breach.
    pub fn insert(
        &mut self,
        path: impl Into<String>,
        document: impl Into<String>,
    ) -> Result<(), OkfError> {
        let path = path.into();
        let document = document.into();
        validate_relative_markdown_path(&path)?;
        if self.documents.contains_key(&path) {
            return Err(OkfError::new(format!(
                "duplicate OKF document path `{path}`"
            )));
        }
        if self.documents.len() >= MAX_OKF_DOCUMENTS {
            return Err(OkfError::new(format!(
                "OKF bundle exceeds the {MAX_OKF_DOCUMENTS}-document limit"
            )));
        }
        if document.len() > MAX_OKF_DOCUMENT_BYTES {
            return Err(OkfError::new(format!(
                "OKF document `{path}` is {} bytes; limit is {MAX_OKF_DOCUMENT_BYTES}",
                document.len()
            )));
        }
        let total = self
            .total_bytes
            .checked_add(document.len())
            .ok_or_else(|| OkfError::new("OKF bundle byte count overflow"))?;
        if total > MAX_OKF_BUNDLE_BYTES {
            return Err(OkfError::new(format!(
                "OKF bundle is {total} bytes; limit is {MAX_OKF_BUNDLE_BYTES}"
            )));
        }
        self.total_bytes = total;
        self.documents.insert(path, document);
        Ok(())
    }

    /// Borrow a document by its normalized path.
    pub fn get(&self, path: &str) -> Option<&str> {
        self.documents.get(path).map(String::as_str)
    }

    /// Documents in deterministic lexical path order.
    pub fn documents(&self) -> impl ExactSizeIterator<Item = (&str, &str)> {
        self.documents
            .iter()
            .map(|(path, document)| (path.as_str(), document.as_str()))
    }

    /// Number of documents in the bundle.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Whether the bundle contains no documents.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Aggregate UTF-8 byte length of all documents.
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

/// Report from lifting an OKF bundle through an RDF event sink.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OkfReadOutcome {
    /// Always-computed runtime loss ledger.
    pub losses: LossLedger,
    /// Number of concept documents emitted into the sink.
    pub documents: usize,
    /// Number of frontmatter-less navigation indexes explicitly skipped.
    pub navigation_pages: usize,
    /// Whether the sink requested early cancellation. A cancelled drive is not
    /// finished, matching the [`purrdf_events::RdfEventSource`] contract.
    pub cancelled: bool,
}

pub(super) fn validate_relative_markdown_path(path: &str) -> Result<(), OkfError> {
    if path.is_empty() {
        return Err(OkfError::new("OKF document path must not be empty"));
    }
    if path.len() > MAX_OKF_PATH_BYTES {
        return Err(OkfError::new(format!(
            "OKF document path is {} bytes; limit is {MAX_OKF_PATH_BYTES}",
            path.len()
        )));
    }
    if path.starts_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(OkfError::new(format!("unsafe OKF relative path `{path}`")));
    }
    if std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        != Some("md")
    {
        return Err(OkfError::new(format!(
            "OKF document path `{path}` must end in `.md`"
        )));
    }
    Ok(())
}

pub(super) fn validate_absolute_iri(label: &str, value: &str) -> Result<(), OkfError> {
    let iri = purrdf_iri::parse(value)
        .map_err(|error| OkfError::new(format!("invalid {label} `{value}`: {error}")))?;
    if !iri.has_scheme() {
        return Err(OkfError::new(format!(
            "{label} `{value}` must be an absolute IRI"
        )));
    }
    Ok(())
}

fn validate_join_base(label: &str, value: &str) -> Result<(), OkfError> {
    validate_absolute_iri(label, value)?;
    if !value.ends_with(['#', '/', ':']) {
        return Err(OkfError::new(format!(
            "{label} `{value}` must end in `#`, `/`, or `:` before local names are appended"
        )));
    }
    Ok(())
}

fn validate_profile_key(key: &str) -> Result<(), OkfError> {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return Err(OkfError::new("OKF frontmatter keys must not be empty"));
    };
    if !(first.is_ascii_alphabetic() || first == '_')
        || !chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        return Err(OkfError::new(format!(
            "unsafe OKF frontmatter key `{key}`; expected an ASCII identifier"
        )));
    }
    Ok(())
}

pub(super) fn minted_document_iri(config: &OkfConfig, path: &str) -> Result<String, OkfError> {
    let iri = format!("{}{}", config.document_base_iri, percent_encode_path(path));
    validate_absolute_iri("minted OKF document IRI", &iri)?;
    Ok(iri)
}

fn percent_encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(char::from(byte));
            }
            _ => {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                out.push('%');
                out.push(char::from(HEX[usize::from(byte >> 4)]));
                out.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
        }
    }
    out
}
