// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bounded nested-GTS discovery for Full Reader callers.

use crate::FastSet;

use purrdf_lex::cbor::Value;

use crate::model::{Diagnostic, Graph};
use crate::reader::read;
use crate::wire::map_get;

mod error;
use error::NestedError;

/// Media type identifying a blob as a nested GTS file (§16).
pub const GTS_MEDIA_TYPE: &str = "application/vnd.blackcat.gts+cbor-seq";

/// A root fold plus nested folds addressed by containing blob digest.
#[derive(Debug)]
pub struct NestedReadResult {
    /// The folded root graph.
    pub graph: Graph,
    /// Nested folds, keyed by the digest of the blob that contained them.
    pub subgraphs: Vec<(String, Graph)>,
    /// Diagnostics from the root fold and every nested fold, in order.
    pub diagnostics: Vec<Diagnostic>,
}

impl NestedReadResult {
    /// Look up a nested fold by its containing blob digest.
    pub fn subgraph(&self, digest: &str) -> Option<&Graph> {
        purrdf_lex::assoc::get(&self.subgraphs, digest)
    }
}

/// Read a GTS file and boundedly recurse into nested-GTS blobs.
///
/// Baseline readers treat nested GTS as ordinary blobs. Full Reader callers can
/// use this helper to expose subgraphs by blob digest while enforcing the
/// recursion and decoded-size budgets required by §12.1/§18.
pub fn read_nested(data: &[u8], max_depth: usize, max_decoded_bytes: usize) -> NestedReadResult {
    let mut remaining = max_decoded_bytes;
    let mut seen = FastSet::default();
    let mut subgraphs = Vec::new();
    let graph = visit(
        data,
        0,
        max_depth,
        &mut remaining,
        &mut seen,
        &mut subgraphs,
    );
    let mut diagnostics = graph.diagnostics.clone();
    for (_, subgraph) in &subgraphs {
        diagnostics.extend(subgraph.diagnostics.iter().cloned());
    }
    NestedReadResult {
        graph,
        subgraphs,
        diagnostics,
    }
}

/// Read nested GTS artifacts after admitting every artifact the recursive walk
/// can reach under one aggregate whole-read contract.
///
/// # Errors
/// Returns the first resource refusal without a partial nested result.
pub fn read_nested_with_limits(
    data: &[u8],
    max_depth: usize,
    max_decoded_bytes: usize,
    limits: crate::reader::GtsReadLimits,
) -> Result<NestedReadResult, crate::reader::GtsReadLimitError> {
    let mut remaining = max_decoded_bytes;
    let mut seen = FastSet::default();
    let mut usage = crate::reader::GtsReadUsage::default();
    let mut subgraphs = Vec::new();
    let nested_limits = NestedLimits {
        max_depth,
        read: limits,
    };
    let graph = visit_with_limits(
        data,
        0,
        &mut remaining,
        &mut seen,
        &mut subgraphs,
        nested_limits,
        &mut usage,
    )?;
    let mut diagnostics = graph.diagnostics.clone();
    for (_, subgraph) in &subgraphs {
        diagnostics.extend(subgraph.diagnostics.iter().cloned());
    }
    Ok(NestedReadResult {
        graph,
        subgraphs,
        diagnostics,
    })
}

#[derive(Clone, Copy)]
struct NestedLimits {
    max_depth: usize,
    read: crate::reader::GtsReadLimits,
}

fn visit_with_limits(
    data: &[u8],
    depth: usize,
    remaining: &mut usize,
    seen: &mut FastSet<String>,
    subgraphs: &mut Vec<(String, Graph)>,
    limits: NestedLimits,
    usage: &mut crate::reader::GtsReadUsage,
) -> Result<Graph, crate::reader::GtsReadLimitError> {
    let artifact_usage = crate::reader::check_read_limits(
        data,
        crate::reader::ReadOptions::new(true, None),
        limits.read,
    )?;
    accumulate_usage(usage, artifact_usage, limits.read)?;
    let mut graph = read(data, true, None);
    let nested_digests: Vec<String> = graph
        .blob_meta
        .iter()
        .filter(|(_, meta)| blob_media_type(meta) == Some(GTS_MEDIA_TYPE))
        .map(|(digest, _)| digest.clone())
        .collect();
    for digest in nested_digests {
        if seen.contains(&digest) {
            continue;
        }
        if depth >= limits.max_depth {
            let error = NestedError::Depth {
                digest,
                depth,
                limit: limits.max_depth,
            };
            graph.diagnostics.push(
                Diagnostic {
                    cause: None,
                    code: "RecursionLimit".to_owned(),
                    detail: error.to_string(),
                    frame_index: None,
                }
                .with_native_cause(error),
            );
            continue;
        }
        let Some(entry) = graph.blob_entry(&digest) else {
            continue;
        };
        let decoded_len = match entry.decoded_len_bounded(limits.read.decoded_blob_bytes) {
            Ok(len) => len,
            Err(cause) => {
                let error = NestedError::DecodeStream {
                    digest,
                    cause: cause.into(),
                };
                graph.diagnostics.push(
                    Diagnostic {
                        cause: None,
                        code: "DamagedFrame".to_owned(),
                        detail: error.to_string(),
                        frame_index: None,
                    }
                    .with_native_cause(error),
                );
                continue;
            }
        };
        if decoded_len > *remaining {
            let error = NestedError::DecodedSize {
                digest,
                actual: decoded_len,
                remaining: *remaining,
            };
            graph.diagnostics.push(
                Diagnostic {
                    cause: None,
                    code: "RecursionLimit".to_owned(),
                    detail: error.to_string(),
                    frame_index: None,
                }
                .with_native_cause(error),
            );
            continue;
        }
        let bytes = match graph.blob_bytes_cloned_bounded(&digest, decoded_len) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => continue,
            Err(cause) => {
                let error = NestedError::Decode { digest, cause };
                graph.diagnostics.push(
                    Diagnostic {
                        cause: None,
                        code: "DamagedFrame".to_owned(),
                        detail: error.to_string(),
                        frame_index: None,
                    }
                    .with_native_cause(error),
                );
                continue;
            }
        };
        *remaining -= decoded_len;
        seen.insert(digest.clone());
        let child =
            visit_with_limits(&bytes, depth + 1, remaining, seen, subgraphs, limits, usage)?;
        if child.segment_heads.is_empty() {
            let error = NestedError::Unreadable {
                digest,
                diagnostics: child.diagnostics,
            };
            graph.diagnostics.push(
                Diagnostic {
                    cause: None,
                    code: "DamagedFrame".to_owned(),
                    detail: error.to_string(),
                    frame_index: None,
                }
                .with_native_cause(error),
            );
            continue;
        }
        subgraphs.push((digest, child));
    }
    Ok(graph)
}

fn accumulate_usage(
    total: &mut crate::reader::GtsReadUsage,
    next: crate::reader::GtsReadUsage,
    limits: crate::reader::GtsReadLimits,
) -> Result<(), crate::reader::GtsReadLimitError> {
    total.encoded_input_bytes = total
        .encoded_input_bytes
        .saturating_add(next.encoded_input_bytes);
    total.items = total.items.saturating_add(next.items);
    total.largest_encoded_frame_bytes = total
        .largest_encoded_frame_bytes
        .max(next.largest_encoded_frame_bytes);
    total.largest_decoded_frame_bytes = total
        .largest_decoded_frame_bytes
        .max(next.largest_decoded_frame_bytes);
    total.largest_decoded_blob_bytes = total
        .largest_decoded_blob_bytes
        .max(next.largest_decoded_blob_bytes);
    total.total_decoded_bytes = total
        .total_decoded_bytes
        .saturating_add(next.total_decoded_bytes);
    total.terms = total.terms.saturating_add(next.terms);
    total.rows = total.rows.saturating_add(next.rows);
    total.metadata_entries = total.metadata_entries.saturating_add(next.metadata_entries);
    total.diagnostics = total.diagnostics.saturating_add(next.diagnostics);
    total.work = total.work.saturating_add(next.work);

    for (resource, required, limit) in [
        (
            crate::reader::GtsReadResource::EncodedInputBytes,
            total.encoded_input_bytes,
            limits.encoded_input_bytes,
        ),
        (
            crate::reader::GtsReadResource::Items,
            total.items,
            limits.items,
        ),
        (
            crate::reader::GtsReadResource::EncodedFrameBytes,
            total.largest_encoded_frame_bytes,
            limits.encoded_frame_bytes,
        ),
        (
            crate::reader::GtsReadResource::DecodedFrameBytes,
            total.largest_decoded_frame_bytes,
            limits.decoded_frame_bytes,
        ),
        (
            crate::reader::GtsReadResource::DecodedBlobBytes,
            total.largest_decoded_blob_bytes,
            limits.decoded_blob_bytes,
        ),
        (
            crate::reader::GtsReadResource::TotalDecodedBytes,
            total.total_decoded_bytes,
            limits.total_decoded_bytes,
        ),
        (
            crate::reader::GtsReadResource::Terms,
            total.terms,
            limits.terms,
        ),
        (
            crate::reader::GtsReadResource::Rows,
            total.rows,
            limits.rows,
        ),
        (
            crate::reader::GtsReadResource::MetadataEntries,
            total.metadata_entries,
            limits.metadata_entries,
        ),
        (
            crate::reader::GtsReadResource::Diagnostics,
            total.diagnostics,
            limits.diagnostics,
        ),
        (
            crate::reader::GtsReadResource::Work,
            total.work,
            limits.work,
        ),
    ] {
        if required > limit {
            return Err(crate::reader::GtsReadLimitError {
                resource,
                required,
                limit,
                usage: *total,
            });
        }
    }
    Ok(())
}

fn visit(
    data: &[u8],
    depth: usize,
    max_depth: usize,
    remaining: &mut usize,
    seen: &mut FastSet<String>,
    subgraphs: &mut Vec<(String, Graph)>,
) -> Graph {
    let mut graph = read(data, true, None);
    let nested_digests: Vec<String> = graph
        .blob_meta
        .iter()
        .filter_map(|(digest, meta)| {
            if blob_media_type(meta) == Some(GTS_MEDIA_TYPE) {
                Some(digest.clone())
            } else {
                None
            }
        })
        .collect();

    for digest in nested_digests {
        if seen.contains(&digest) {
            continue;
        }
        if depth >= max_depth {
            let error = NestedError::Depth {
                digest,
                depth,
                limit: max_depth,
            };
            graph.diagnostics.push(
                Diagnostic {
                    cause: None,
                    code: "RecursionLimit".to_string(),
                    detail: error.to_string(),
                    frame_index: None,
                }
                .with_native_cause(error),
            );
            continue;
        }
        let nested_bytes = match graph.blob_bytes_cloned(&digest) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => continue,
            Err(err) => {
                let error = NestedError::Decode { digest, cause: err };
                graph.diagnostics.push(
                    Diagnostic {
                        cause: None,
                        code: "DamagedFrame".to_string(),
                        detail: error.to_string(),
                        frame_index: None,
                    }
                    .with_native_cause(error),
                );
                continue;
            }
        };
        let nested_len = nested_bytes.len();
        if nested_len > *remaining {
            let error = NestedError::DecodedSize {
                digest,
                actual: nested_len,
                remaining: *remaining,
            };
            graph.diagnostics.push(
                Diagnostic {
                    cause: None,
                    code: "RecursionLimit".to_string(),
                    detail: error.to_string(),
                    frame_index: None,
                }
                .with_native_cause(error),
            );
            continue;
        }
        *remaining -= nested_len;
        seen.insert(digest.clone());
        let child = visit(
            &nested_bytes,
            depth + 1,
            max_depth,
            remaining,
            seen,
            subgraphs,
        );
        if child.segment_heads.is_empty() {
            let error = NestedError::Unreadable {
                digest,
                diagnostics: child.diagnostics,
            };
            graph.diagnostics.push(
                Diagnostic {
                    cause: None,
                    code: "DamagedFrame".to_string(),
                    detail: error.to_string(),
                    frame_index: None,
                }
                .with_native_cause(error),
            );
            continue;
        }
        subgraphs.push((digest, child));
    }
    graph
}

fn blob_media_type(meta: &Value) -> Option<&str> {
    match meta {
        Value::Map(entries) => match map_get(entries, "mt") {
            Some(Value::Text(mt)) => Some(mt.as_str()),
            _ => None,
        },
        _ => None,
    }
}
