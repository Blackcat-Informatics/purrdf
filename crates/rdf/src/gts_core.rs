// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;

use ciborium::value::Value;
use purrdf_gts::model::Graph;

use crate::{
    RdfBlobOrigin, RdfBlobRecord, RdfDiagnostic, RdfLocation, RdfLookaside, RdfLookasideKind,
    RdfLookasideResource, RdfMetadataEntry, RdfMetadataValue, RdfOpaqueNodeRecord,
    RdfSegmentRecord, RdfSignatureRecord, RdfSuppressionRecord,
};

pub fn lookaside_from_graph(graph: &Graph) -> RdfLookaside {
    let metadata = graph
        .meta
        .iter()
        .map(|(key, value)| {
            RdfMetadataEntry::new("gts:file", key.clone(), metadata_value_from_cbor(value))
        })
        .chain(
            graph
                .segment_meta
                .iter()
                .enumerate()
                .flat_map(|(segment_index, entries)| {
                    entries.iter().map(move |(key, value)| {
                        RdfMetadataEntry::new(
                            format!("gts:segment:{segment_index}"),
                            key.clone(),
                            metadata_value_from_cbor(value),
                        )
                    })
                }),
        )
        .collect();

    let segments = segment_records(graph);
    let blobs = blob_records(graph);
    let resources = resource_records(graph);
    let suppressions = graph
        .suppressions
        .iter()
        .map(|suppression| RdfSuppressionRecord {
            reason: suppression.reason.clone(),
            by: suppression.by.map(|term_id| term_display(graph, term_id)),
            targets: suppression
                .targets
                .iter()
                .map(metadata_value_from_cbor)
                .collect(),
        })
        .collect();
    let opaque_nodes = graph
        .opaque
        .iter()
        .map(|opaque| RdfOpaqueNodeRecord {
            id: hex_bytes(&opaque.id),
            frame_type: opaque.frame_type.clone(),
            reason: opaque.reason.clone(),
            signature_status: opaque.sigstat.clone(),
            public_metadata: opaque.pub_meta.as_ref().map(metadata_value_from_cbor),
        })
        .collect();
    let signatures = graph
        .signatures
        .iter()
        .map(|signature| RdfSignatureRecord {
            frame_id: hex_bytes(&signature.frame_id),
            key_id: signature.kid.clone(),
            status: signature.status.clone(),
            has_cose: signature.cose.is_some(),
        })
        .collect();

    RdfLookaside {
        resources,
        metadata,
        segments,
        blobs,
        suppressions,
        opaque_nodes,
        signatures,
    }
}

fn segment_records(graph: &Graph) -> Vec<RdfSegmentRecord> {
    let max_segments = graph
        .segment_heads
        .len()
        .max(graph.segment_profiles.len())
        .max(graph.segment_streamable.len());
    (0..max_segments)
        .map(|index| {
            let streamable = graph.segment_streamable.get(index);
            RdfSegmentRecord {
                index,
                head: graph.segment_heads.get(index).map(|head| hex_bytes(head)),
                profile: graph.segment_profiles.get(index).cloned(),
                claimed_streamable: streamable.is_some_and(|info| info.claimed),
                covered: streamable.map_or(0, |info| info.covered),
                tail: streamable.map_or(0, |info| info.tail),
            }
        })
        .collect()
}

fn blob_records(graph: &Graph) -> Vec<RdfBlobRecord> {
    let blob_meta = blob_metadata_index(graph);
    // Origin file identity (segment heads) shared by every blob read from this
    // folded graph. Computed once; the fold does not retain per-blob frame
    // provenance, so the reference is file-level.
    let origin = blob_origin(graph);
    graph
        .blobs
        .iter()
        .map(|(digest, entry)| {
            let metadata = blob_metadata(&blob_meta, digest);
            RdfBlobRecord {
                digest: digest.clone(),
                media_type: metadata_text(&metadata, "mt"),
                representation: metadata_text(&metadata, "rep"),
                // `cached_bytes` measures only an already-decoded entry; it never
                // forces a lazy decode. A transformed (Lazy) blob — potentially
                // multi-terabyte — therefore reports `None` rather than decoding
                // the whole payload just to learn its length.
                decoded_len: entry.cached_bytes().map(<[u8]>::len),
                metadata,
                origin: origin.clone(),
            }
        })
        .collect()
}

/// The content-addressed origin reference for blobs in this folded graph: the
/// file-level segment-head ids (hex). `None` when the graph declares no segment
/// heads (e.g. a hand-built graph).
fn blob_origin(graph: &Graph) -> Option<RdfBlobOrigin> {
    if graph.segment_heads.is_empty() {
        return None;
    }
    Some(RdfBlobOrigin {
        source_segments: graph
            .segment_heads
            .iter()
            .map(|head| hex_bytes(head))
            .collect(),
    })
}

fn resource_records(graph: &Graph) -> Vec<RdfLookasideResource> {
    let blob_meta = blob_metadata_index(graph);
    graph
        .blobs
        .iter()
        .map(|(digest, _)| {
            let metadata = blob_metadata(&blob_meta, digest);
            let kind = lookaside_kind_from_metadata(&metadata);
            let mut resource = RdfLookasideResource::new(kind).with_digest(digest.clone());
            resource.media_type = metadata_text(&metadata, "mt");
            resource.path = metadata_text(&metadata, "path");
            resource.iri = metadata_text(&metadata, "iri");
            resource.name = metadata_text(&metadata, "name")
                .or_else(|| metadata_text(&metadata, "label"))
                .or_else(|| metadata_text(&metadata, "role"));
            resource.graph_name = metadata_text(&metadata, "graph");
            resource.metadata = metadata;
            resource
        })
        .collect()
}

fn blob_metadata_index(graph: &Graph) -> BTreeMap<&str, &Value> {
    graph
        .blob_meta
        .iter()
        .map(|(digest, value)| (digest.as_str(), value))
        .collect()
}

fn blob_metadata(
    blob_meta: &BTreeMap<&str, &Value>,
    digest: &str,
) -> BTreeMap<String, RdfMetadataValue> {
    blob_meta
        .get(digest)
        .map(|value| match metadata_value_from_cbor(value) {
            RdfMetadataValue::Map(map) => map,
            value => {
                let mut map = BTreeMap::new();
                map.insert("value".to_owned(), value);
                map
            }
        })
        .unwrap_or_default()
}

fn lookaside_kind_from_metadata(metadata: &BTreeMap<String, RdfMetadataValue>) -> RdfLookasideKind {
    // Borrow the metadata text directly (`as_text`) rather than `metadata_text`'s
    // owned clone: these hints are inspected and discarded, never stored.
    for key in ["kind", "role", "domain", "type"] {
        if let Some(value) = metadata.get(key).and_then(RdfMetadataValue::as_text) {
            return RdfLookasideKind::from_hint(value);
        }
    }
    if let Some(media_type) = metadata.get("mt").and_then(RdfMetadataValue::as_text) {
        let lower = media_type.to_ascii_lowercase();
        if lower.contains("shacl") {
            return RdfLookasideKind::Shacl;
        }
        if lower.contains("shex") {
            return RdfLookasideKind::Shex;
        }
        if lower.contains("sparql") {
            return RdfLookasideKind::Query;
        }
        if lower.contains("json") && lower.contains("schema") {
            return RdfLookasideKind::Schema;
        }
        if lower.contains("markdown") || lower.contains("html") {
            return RdfLookasideKind::Docs;
        }
    }
    RdfLookasideKind::Blob
}

fn metadata_text(metadata: &BTreeMap<String, RdfMetadataValue>, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(RdfMetadataValue::as_text)
        .map(str::to_owned)
}

fn metadata_value_from_cbor(value: &Value) -> RdfMetadataValue {
    match value {
        Value::Integer(integer) => RdfMetadataValue::Integer(i128::from(*integer)),
        Value::Bytes(bytes) => RdfMetadataValue::Bytes(bytes.clone()),
        Value::Float(value) => RdfMetadataValue::Float(*value),
        Value::Text(value) => RdfMetadataValue::Text(value.clone()),
        Value::Bool(value) => RdfMetadataValue::Bool(*value),
        Value::Null => RdfMetadataValue::Null,
        Value::Tag(tag, value) => RdfMetadataValue::Tagged {
            tag: *tag,
            value: Box::new(metadata_value_from_cbor(value)),
        },
        Value::Array(values) => {
            RdfMetadataValue::Array(values.iter().map(metadata_value_from_cbor).collect())
        }
        Value::Map(entries) => RdfMetadataValue::Map(
            entries
                .iter()
                .map(|(key, value)| (metadata_key_from_cbor(key), metadata_value_from_cbor(value)))
                .collect(),
        ),
        other => RdfMetadataValue::Opaque(format!("{other:?}")),
    }
}

fn metadata_key_from_cbor(value: &Value) -> String {
    match value {
        Value::Text(value) => value.clone(),
        other => format!("{other:?}"),
    }
}

fn term_display(graph: &Graph, term_id: usize) -> String {
    graph
        .terms
        .get(term_id)
        .and_then(|term| term.value.clone())
        .unwrap_or_else(|| format!("term#{term_id}"))
}

fn hex_bytes(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Fold GTS bytes into a graph and fail if the reader produced diagnostics.
pub fn read_graph(bytes: &[u8], allow_segments: bool) -> Result<Graph, RdfDiagnostic> {
    let graph = purrdf_gts::reader::read(bytes, allow_segments, None);
    if graph.diagnostics.is_empty() {
        Ok(graph)
    } else {
        Err(diagnostics_to_error(&graph))
    }
}

/// Fold all GTS segments into a graph and fail on any reader diagnostic.
pub fn read_all_segments(bytes: &[u8]) -> Result<Graph, RdfDiagnostic> {
    read_graph(bytes, true)
}

pub(crate) fn diagnostics_to_error(graph: &Graph) -> RdfDiagnostic {
    let joined = graph
        .diagnostics
        .iter()
        .map(|d| format!("{}: {}", d.code, d.detail))
        .collect::<Vec<_>>()
        .join("; ");
    let mut diagnostic = RdfDiagnostic::error(
        "gts-fold-diagnostic",
        format!(
            "GTS fold reported {} diagnostic(s)",
            graph.diagnostics.len()
        ),
    )
    .with_detail(joined);
    if let Some(frame_index) = graph.diagnostics.iter().find_map(|d| d.frame_index) {
        diagnostic = diagnostic
            .with_location(RdfLocation::logical("gts:reader").with_gts_frame(frame_index));
    }
    diagnostic
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gts_resolve::term_from_id;
    use crate::RdfTerm;
    use purrdf_gts::model::{Term, TermKind};

    fn private_lang_named_graph() -> Graph {
        let mut graph = Graph::default();
        graph.terms.push(Term {
            kind: TermKind::Iri,
            value: Some("https://example.org/s".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        graph.terms.push(Term {
            kind: TermKind::Iri,
            value: Some("https://example.org/p".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        graph.terms.push(Term {
            kind: TermKind::Literal,
            value: Some("hallo".to_owned()),
            datatype: None,
            lang: Some("x-purrdf-afrikaans".to_owned()),
            direction: None,
            reifier: None,
        });
        graph.terms.push(Term {
            kind: TermKind::Iri,
            value: Some("https://example.org/graph".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        graph
            .meta
            .push(("producer".to_owned(), Value::Text("purrdf-test".to_owned())));
        graph.segment_profiles.push("rdf12".to_owned());
        graph.quads.push((0, 1, 2, Some(3)));
        graph
    }

    #[test]
    fn blob_is_preserved_as_content_addressed_reference() {
        // A blob read from a GTS graph is preserved as a content-addressed
        // reference: the blob_id digest + an origin file id — never the payload
        // bytes (which may be multi-terabyte). This is the by-reference model
        // behind the `blob-bytes-absent` intentional loss.
        let mut graph = Graph::default();
        graph.segment_heads.push(vec![0xab, 0xcd]);
        graph.set_blob("blake3:deadbeef".to_owned(), b"payload".to_vec());

        let lookaside = lookaside_from_graph(&graph);
        assert_eq!(lookaside.blobs.len(), 1);
        let blob = &lookaside.blobs[0];
        // blob_id reference.
        assert_eq!(blob.digest, "blake3:deadbeef");
        // origin file reference (segment-head hex).
        let origin = blob.origin.as_ref().expect("origin reference present");
        assert_eq!(origin.source_segments, vec!["abcd".to_owned()]);
    }

    #[test]
    fn gts_import_preserves_named_graph_and_private_language_tag() {
        let graph = private_lang_named_graph();
        let bundle = crate::import_gts_graph(graph).expect("GTS graph should import cleanly");
        let quads: Vec<_> = bundle.dataset.owned_quads().collect();
        assert_eq!(quads.len(), 1);
        assert!(quads[0].graph_name.is_some());
        let lookaside = &bundle.envelope.lookaside;
        assert_eq!(lookaside.metadata.len(), 1);
        assert_eq!(lookaside.segments.len(), 1);
        match &quads[0].object {
            RdfTerm::Literal(literal) => {
                assert_eq!(literal.language.as_deref(), Some("x-purrdf-afrikaans"));
            }
            other => panic!("expected literal object, got {other:?}"),
        }
    }

    #[test]
    fn read_graph_rejects_malformed_bytes() {
        let result = read_all_segments(b"not a valid gts file");
        assert!(result.is_err(), "bad GTS bytes must fail");
        assert_eq!(result.unwrap_err().code, "gts-fold-diagnostic");
    }

    #[test]
    fn cyclic_triple_terms_hit_nesting_limit() {
        let mut graph = Graph::default();
        graph.terms.push(Term {
            kind: TermKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: Some(0),
        });
        graph.terms.push(Term {
            kind: TermKind::Iri,
            value: Some("https://example.org/p".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        graph.terms.push(Term {
            kind: TermKind::Iri,
            value: Some("https://example.org/o".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        graph.reifiers.push((0, (0, 1, 2), None));

        let err = term_from_id(&graph, 0, RdfLocation::logical("test"))
            .expect_err("cyclic triple term should hit nesting limit");
        assert_eq!(err.code, "gts-term-nesting-limit");
    }

    #[test]
    fn iri_terms_require_non_empty_values() {
        for value in [None, Some("")] {
            let mut graph = Graph::default();
            graph.terms.push(Term {
                kind: TermKind::Iri,
                value: value.map(str::to_owned),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            });

            let err = term_from_id(&graph, 0, RdfLocation::logical("test"))
                .expect_err("invalid GTS IRI term should fail");
            assert_eq!(err.code, "gts-iri-missing-value");
        }
    }
}
