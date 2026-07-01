// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Bounded nested-GTS discovery for Full Reader callers.

use std::collections::HashSet;

use ciborium::value::Value;

use crate::model::{Diagnostic, Graph};
use crate::reader::read;
use crate::wire::map_get;

pub const GTS_MEDIA_TYPE: &str = "application/vnd.blackcat.gts+cbor-seq";

/// A root fold plus nested folds addressed by containing blob digest.
#[derive(Debug)]
pub struct NestedReadResult {
    pub graph: Graph,
    pub subgraphs: Vec<(String, Graph)>,
    pub diagnostics: Vec<Diagnostic>,
}

impl NestedReadResult {
    pub fn subgraph(&self, digest: &str) -> Option<&Graph> {
        self.subgraphs
            .iter()
            .find(|(d, _)| d == digest)
            .map(|(_, graph)| graph)
    }
}

/// Read a GTS file and boundedly recurse into nested-GTS blobs.
///
/// Baseline readers treat nested GTS as ordinary blobs. Full Reader callers can
/// use this helper to expose subgraphs by blob digest while enforcing the
/// recursion and decoded-size budgets required by §12.1/§18.
pub fn read_nested(data: &[u8], max_depth: usize, max_decoded_bytes: usize) -> NestedReadResult {
    let mut remaining = max_decoded_bytes;
    let mut seen = HashSet::new();
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

fn visit(
    data: &[u8],
    depth: usize,
    max_depth: usize,
    remaining: &mut usize,
    seen: &mut HashSet<String>,
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
            graph.diagnostics.push(Diagnostic {
                code: "RecursionLimit".to_string(),
                detail: format!("nested GTS blob {digest} exceeds max depth {max_depth}"),
                frame_index: None,
            });
            continue;
        }
        let nested_bytes = match graph.blob_bytes_cloned(&digest) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => continue,
            Err(err) => {
                graph.diagnostics.push(Diagnostic {
                    code: "DamagedFrame".to_string(),
                    detail: format!("nested GTS blob {digest} decode failed: {err:?}"),
                    frame_index: None,
                });
                continue;
            }
        };
        let nested_len = nested_bytes.len();
        if nested_len > *remaining {
            graph.diagnostics.push(Diagnostic {
                code: "RecursionLimit".to_string(),
                detail: format!(
                    "nested GTS decoded-size budget exceeded at {digest}: {} > {}",
                    nested_len, *remaining
                ),
                frame_index: None,
            });
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
            graph.diagnostics.push(Diagnostic {
                code: "DamagedFrame".to_string(),
                detail: format!("nested GTS blob {digest} could not be parsed"),
                frame_index: None,
            });
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
