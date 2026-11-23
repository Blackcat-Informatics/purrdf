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
/// can reach under one per-artifact whole-read contract.
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
    admit_nested(data, 0, max_depth, &mut remaining, &mut seen, limits)?;
    Ok(read_nested(data, max_depth, max_decoded_bytes))
}

fn admit_nested(
    data: &[u8],
    depth: usize,
    max_depth: usize,
    remaining: &mut usize,
    seen: &mut FastSet<String>,
    limits: crate::reader::GtsReadLimits,
) -> Result<(), crate::reader::GtsReadLimitError> {
    let mut graph =
        crate::reader::read_with_limits(data, crate::reader::ReadOptions::new(true, None), limits)?;
    if depth >= max_depth {
        return Ok(());
    }
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
        let Ok(Some(bytes)) = graph.blob_bytes_cloned(&digest) else {
            continue;
        };
        if bytes.len() > *remaining {
            continue;
        }
        *remaining -= bytes.len();
        seen.insert(digest);
        admit_nested(&bytes, depth + 1, max_depth, remaining, seen, limits)?;
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
