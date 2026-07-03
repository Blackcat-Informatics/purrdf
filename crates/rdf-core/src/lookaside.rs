// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;

use crate::RdfLocation;

/// Structured non-triple material that travels with an RDF store.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RdfLookaside {
    pub resources: Vec<RdfLookasideResource>,
    pub metadata: Vec<RdfMetadataEntry>,
    pub segments: Vec<RdfSegmentRecord>,
    pub blobs: Vec<RdfBlobRecord>,
    pub suppressions: Vec<RdfSuppressionRecord>,
    pub opaque_nodes: Vec<RdfOpaqueNodeRecord>,
    pub signatures: Vec<RdfSignatureRecord>,
}

impl RdfLookaside {
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
            && self.metadata.is_empty()
            && self.segments.is_empty()
            && self.blobs.is_empty()
            && self.suppressions.is_empty()
            && self.opaque_nodes.is_empty()
            && self.signatures.is_empty()
    }

    pub fn resources_of_kind(
        &self,
        kind: RdfLookasideKind,
    ) -> impl Iterator<Item = &RdfLookasideResource> {
        self.resources
            .iter()
            .filter(move |resource| resource.kind == kind)
    }

    /// Every decodable target across all `suppress` directives (§11), flattened.
    ///
    /// A target is a `{"kind": ..., "id": ...}` map; this is a linear scan over
    /// `suppressions[*].targets` that decodes each one into a typed
    /// [`SuppressionTarget`]. A target whose `"kind"` is not one of the
    /// integer-id-addressed kinds below, whose `"id"` is missing/not an
    /// integer, or whose `"id"` is negative or does not fit `usize` is skipped
    /// — that is filtering out an undecodable target, not swallowing an error.
    ///
    /// `frame`- and `blob`-kind targets also exist in the underlying model
    /// (GTS §11) but are addressed by frame id bytes / content digest rather
    /// than an integer id, so they have no [`SuppressionTargetKind`] here and
    /// are always skipped by this decoder.
    ///
    /// The `by` field is intentionally never read here: it is a C0.8 display
    /// hint (an actor label), not an id to resolve.
    pub fn suppression_targets(&self) -> impl Iterator<Item = SuppressionTarget> + '_ {
        self.suppressions
            .iter()
            .flat_map(|suppression| suppression.targets.iter())
            .filter_map(|target| {
                let RdfMetadataValue::Map(entries) = target else {
                    return None;
                };
                let kind = match entries.get("kind")?.as_text()? {
                    "term" => SuppressionTargetKind::Term,
                    "quad" => SuppressionTargetKind::Quad,
                    "reifier" => SuppressionTargetKind::Reifier,
                    _ => return None,
                };
                let RdfMetadataValue::Integer(raw) = entries.get("id")? else {
                    return None;
                };
                let id = usize::try_from(*raw).ok()?;
                Some(SuppressionTarget { kind, id })
            })
    }
}

/// A decoded [`RdfSuppressionRecord`] target: `{"kind": ..., "id": n}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuppressionTarget {
    pub kind: SuppressionTargetKind,
    pub id: usize,
}

/// The integer-id-addressed suppression target kinds. `frame` and `blob`
/// targets exist in the underlying model but are addressed by frame id bytes
/// / content digest rather than an integer id, so they have no variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuppressionTargetKind {
    Term,
    Quad,
    Reifier,
}

/// Known companion/index kinds. Unknown domains remain representable.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[non_exhaustive]
pub enum RdfLookasideKind {
    Shacl,
    Shex,
    Docs,
    Logic,
    /// Materialized reasoning output (inferred closures, entailments, reasoner
    /// reports) that travels with the dataset as a typed sidecar — the lookaside
    /// home for the reasoning lane the pipeline bundle carries.
    Reasoning,
    /// The Horn/relational-core projection of the logic layer (the
    /// NNF→Skolem→Horn floor) carried as a typed sidecar alongside its source graph.
    RelationalCore,
    Schema,
    Query,
    Mapping,
    Projection,
    Ontology,
    #[default]
    Metadata,
    Blob,
    Other(String),
}

impl RdfLookasideKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Shacl => "shacl",
            Self::Shex => "shex",
            Self::Docs => "docs",
            Self::Logic => "logic",
            Self::Reasoning => "reasoning",
            Self::RelationalCore => "relational-core",
            Self::Schema => "schema",
            Self::Query => "query",
            Self::Mapping => "mapping",
            Self::Projection => "projection",
            Self::Ontology => "ontology",
            Self::Metadata => "metadata",
            Self::Blob => "blob",
            Self::Other(value) => value.as_str(),
        }
    }

    pub fn from_hint(value: &str) -> Self {
        let lower = value.to_ascii_lowercase();
        match lower.as_str() {
            "shacl" | "shape" | "shapes" => Self::Shacl,
            "shex" => Self::Shex,
            "doc" | "docs" | "documentation" | "ontology-docs" => Self::Docs,
            "logic" | "rule" | "rules" => Self::Logic,
            "reasoning" | "reason" | "inferred" | "entailment" | "entailments" => Self::Reasoning,
            "relational-core" | "relational_core" | "relationalcore" | "horn" => {
                Self::RelationalCore
            }
            "schema" | "schemas" | "json-schema" => Self::Schema,
            "query" | "queries" | "sparql" => Self::Query,
            "mapping" | "mappings" => Self::Mapping,
            "projection" | "projections" => Self::Projection,
            "ontology" | "owl" => Self::Ontology,
            "metadata" | "meta" => Self::Metadata,
            "blob" | "blobs" => Self::Blob,
            _ => Self::Other(value.to_owned()),
        }
    }
}

/// A typed sidecar resource such as SHACL, ShEx, docs, logic, schemas, or queries.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RdfLookasideResource {
    pub kind: RdfLookasideKind,
    pub iri: Option<String>,
    pub name: Option<String>,
    pub graph_name: Option<String>,
    pub media_type: Option<String>,
    pub content_digest: Option<String>,
    pub path: Option<String>,
    pub location: Option<RdfLocation>,
    pub metadata: BTreeMap<String, RdfMetadataValue>,
}

impl RdfLookasideResource {
    pub fn new(kind: RdfLookasideKind) -> Self {
        Self {
            kind,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[must_use]
    pub fn with_iri(mut self, iri: impl Into<String>) -> Self {
        self.iri = Some(iri.into());
        self
    }

    #[must_use]
    pub fn with_graph_name(mut self, graph_name: impl Into<String>) -> Self {
        self.graph_name = Some(graph_name.into());
        self
    }

    #[must_use]
    pub fn with_media_type(mut self, media_type: impl Into<String>) -> Self {
        self.media_type = Some(media_type.into());
        self
    }

    #[must_use]
    pub fn with_digest(mut self, digest: impl Into<String>) -> Self {
        self.content_digest = Some(digest.into());
        self
    }

    #[must_use]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    #[must_use]
    pub fn with_location(mut self, location: RdfLocation) -> Self {
        self.location = Some(location);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RdfMetadataEntry {
    pub scope: String,
    pub key: String,
    pub value: RdfMetadataValue,
    pub location: Option<RdfLocation>,
}

impl RdfMetadataEntry {
    pub fn new(scope: impl Into<String>, key: impl Into<String>, value: RdfMetadataValue) -> Self {
        Self {
            scope: scope.into(),
            key: key.into(),
            value,
            location: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RdfMetadataValue {
    Null,
    Bool(bool),
    Integer(i128),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
    Array(Vec<Self>),
    Map(BTreeMap<String, Self>),
    Tagged { tag: u64, value: Box<Self> },
    Opaque(String),
}

impl RdfMetadataValue {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdfSegmentRecord {
    pub index: usize,
    pub head: Option<String>,
    pub profile: Option<String>,
    pub claimed_streamable: bool,
    pub covered: usize,
    pub tail: usize,
}

/// Where a blob's payload bytes can be fetched from.
///
/// The payload is **never** held in the RDF IR — it may be arbitrarily large
/// (multi-terabyte data dumps). This content-addressed reference — the blob_id
/// digest (on [`RdfBlobRecord`]) plus the origin file identity here — is what a
/// streaming materializer uses to copy bytes origin→destination on demand.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RdfBlobOrigin {
    /// Origin file identity: the GTS segment-head id(s) (hex) the blob was read
    /// from. A folded read records the file-level segment set; the originating
    /// frame index is not recoverable from a fold and is intentionally omitted.
    pub source_segments: Vec<String>,
}

/// A content-addressed reference to a blob that travels with an RDF store.
///
/// Carries the blob_id ([`digest`](Self::digest)) and declared metadata — but
/// **never** the payload bytes ( made the bytes reachable; the IR
/// deliberately does not materialize them). The bytes are recovered by streaming
/// from [`origin`](Self::origin) when a destination is materialized.
#[derive(Debug, Clone, PartialEq)]
pub struct RdfBlobRecord {
    pub digest: String,
    pub media_type: Option<String>,
    pub representation: Option<String>,
    pub decoded_len: Option<usize>,
    pub metadata: BTreeMap<String, RdfMetadataValue>,
    /// Content-addressed origin for streaming the payload on demand. `None` when
    /// the source file identity is unknown (e.g. a hand-built store).
    pub origin: Option<RdfBlobOrigin>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RdfSuppressionRecord {
    pub reason: Option<String>,
    pub by: Option<String>,
    pub targets: Vec<RdfMetadataValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RdfOpaqueNodeRecord {
    pub id: String,
    pub frame_type: String,
    pub reason: String,
    pub signature_status: String,
    pub public_metadata: Option<RdfMetadataValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdfSignatureRecord {
    pub frame_id: String,
    pub key_id: Option<String>,
    pub status: String,
    pub has_cose: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasoning_and_relational_core_kinds_round_trip() {
        assert_eq!(RdfLookasideKind::Reasoning.as_str(), "reasoning");
        assert_eq!(RdfLookasideKind::RelationalCore.as_str(), "relational-core");
        // `from_hint` resolves the canonical alias and a few synonyms.
        assert_eq!(
            RdfLookasideKind::from_hint("reasoning"),
            RdfLookasideKind::Reasoning
        );
        assert_eq!(
            RdfLookasideKind::from_hint("entailment"),
            RdfLookasideKind::Reasoning
        );
        assert_eq!(
            RdfLookasideKind::from_hint("relational-core"),
            RdfLookasideKind::RelationalCore
        );
        assert_eq!(
            RdfLookasideKind::from_hint("horn"),
            RdfLookasideKind::RelationalCore
        );
        // An unknown hint still falls through to `Other`, preserving openness.
        assert_eq!(
            RdfLookasideKind::from_hint("not-a-kind"),
            RdfLookasideKind::Other("not-a-kind".to_owned())
        );
    }

    #[test]
    fn resources_of_kind_filters_by_new_variants() {
        let mut la = RdfLookaside::default();
        la.resources
            .push(RdfLookasideResource::new(RdfLookasideKind::Reasoning).with_name("closure"));
        la.resources
            .push(RdfLookasideResource::new(RdfLookasideKind::Logic).with_name("rules"));
        let reasoning: Vec<_> = la.resources_of_kind(RdfLookasideKind::Reasoning).collect();
        assert_eq!(reasoning.len(), 1);
        assert_eq!(reasoning[0].name.as_deref(), Some("closure"));
    }

    fn target_map(kind: &str, id: RdfMetadataValue) -> RdfMetadataValue {
        RdfMetadataValue::Map(BTreeMap::from([
            ("kind".to_owned(), RdfMetadataValue::Text(kind.to_owned())),
            ("id".to_owned(), id),
        ]))
    }

    #[test]
    fn suppression_targets_decodes_known_kinds_and_skips_the_rest() {
        let mut la = RdfLookaside::default();
        la.suppressions.push(RdfSuppressionRecord {
            reason: Some("test".to_owned()),
            by: Some("agent:1".to_owned()),
            targets: vec![
                target_map("term", RdfMetadataValue::Integer(3)),
                target_map("quad", RdfMetadataValue::Integer(7)),
                // Unknown kind: skipped.
                target_map("frame", RdfMetadataValue::Integer(9)),
                // Negative id: skipped.
                target_map("reifier", RdfMetadataValue::Integer(-1)),
                // Overflowing id: skipped.
                target_map("reifier", RdfMetadataValue::Integer(i128::MAX)),
                // Not a map at all: skipped.
                RdfMetadataValue::Text("not-a-target".to_owned()),
            ],
        });

        let targets: Vec<_> = la.suppression_targets().collect();
        assert_eq!(
            targets,
            vec![
                SuppressionTarget {
                    kind: SuppressionTargetKind::Term,
                    id: 3,
                },
                SuppressionTarget {
                    kind: SuppressionTargetKind::Quad,
                    id: 7,
                },
            ]
        );
    }
}
