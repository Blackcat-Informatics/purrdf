// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bounded nested-GTS discovery for Full Reader callers.

use crate::FastSet;

use purrdf_lex::cbor::Value;

use crate::model::{BlobEntry, Diagnostic, Graph};
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
    let mut seen = FastSet::default();
    let mut usage = crate::reader::GtsReadUsage::default();
    let mut nested_decoded = 0usize;
    let mut subgraphs = Vec::new();
    let nested_limits = NestedLimits {
        max_depth,
        decoded_bytes: max_decoded_bytes,
        read: limits,
    };
    let graph = visit_with_limits(
        data,
        0,
        &mut seen,
        &mut subgraphs,
        nested_limits,
        &mut usage,
        &mut nested_decoded,
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
    decoded_bytes: usize,
    read: crate::reader::GtsReadLimits,
}

fn visit_with_limits(
    data: &[u8],
    depth: usize,
    seen: &mut FastSet<String>,
    subgraphs: &mut Vec<(String, Graph)>,
    limits: NestedLimits,
    usage: &mut crate::reader::GtsReadUsage,
    nested_decoded: &mut usize,
) -> Result<Graph, crate::reader::GtsReadLimitError> {
    let artifact_limits = remaining_limits(limits.read, *usage);
    let artifact_usage = crate::reader::check_read_limits(
        data,
        crate::reader::ReadOptions::new(true, None),
        artifact_limits,
    )
    .map_err(|error| aggregate_limit_error(*usage, error, limits.read))?;
    accumulate_usage(usage, artifact_usage, limits.read)?;
    let mut graph = read(data, true, None);
    let nested_digests: Vec<String> = graph
        .blob_meta
        .iter()
        .filter(|(_, meta)| blob_media_type(meta) == Some(GTS_MEDIA_TYPE))
        .map(|(digest, _)| digest.clone())
        .collect();
    for digest in nested_digests {
        charge_nested_work(usage, limits.read, 1)?;
        if seen.contains(&digest) {
            continue;
        }
        if depth >= limits.max_depth {
            let required = depth.saturating_add(1);
            return Err(crate::reader::GtsReadLimitError {
                resource: crate::reader::GtsReadResource::NestedDepth,
                required,
                limit: limits.max_depth,
                usage: *usage,
            });
        }
        let Some(entry) = graph.blob_entry(&digest) else {
            continue;
        };
        let remaining_work = limits.read.work.saturating_sub(usage.work);
        let measurement_limit = limits.read.decoded_blob_bytes.min(remaining_work);
        let measured = match measure_nested_entry(entry, measurement_limit, remaining_work) {
            Ok(measured) => measured,
            Err(MeasureError::Work { required }) => {
                let required = usage.work.saturating_add(required);
                let mut refused_usage = *usage;
                refused_usage.work = required;
                return Err(crate::reader::GtsReadLimitError {
                    resource: crate::reader::GtsReadResource::Work,
                    required,
                    limit: limits.read.work,
                    usage: refused_usage,
                });
            }
            Err(MeasureError::Codec { cause, work }) => {
                charge_nested_work(usage, limits.read, work)?;
                let error = NestedError::DecodeStream {
                    digest,
                    cause: cause.into(),
                };
                push_nested_diagnostic(
                    &mut graph,
                    usage,
                    limits.read,
                    Diagnostic {
                        cause: None,
                        code: "DamagedFrame".to_owned(),
                        detail: error.to_string(),
                        frame_index: None,
                    }
                    .with_native_cause(error),
                )?;
                continue;
            }
        };
        let decoded_len = measured.decoded;
        charge_nested_work(usage, limits.read, measured.work)?;
        let required_nested = nested_decoded.saturating_add(decoded_len);
        if required_nested > limits.decoded_bytes {
            return Err(crate::reader::GtsReadLimitError {
                resource: crate::reader::GtsReadResource::NestedDecodedBytes,
                required: required_nested,
                limit: limits.decoded_bytes,
                usage: *usage,
            });
        }
        *nested_decoded = required_nested;
        let required_input = usage.encoded_input_bytes.saturating_add(decoded_len);
        if required_input > limits.read.encoded_input_bytes {
            let mut refused_usage = *usage;
            refused_usage.encoded_input_bytes = required_input;
            return Err(crate::reader::GtsReadLimitError {
                resource: crate::reader::GtsReadResource::EncodedInputBytes,
                required: required_input,
                limit: limits.read.encoded_input_bytes,
                usage: refused_usage,
            });
        }
        let bytes = match materialize_nested_entry(entry, limits.read, usage) {
            Ok(bytes) => bytes,
            Err(MaterializeError::Limit(error)) => return Err(error),
            Err(MaterializeError::Codec(cause)) => {
                let error = NestedError::Decode { digest, cause };
                push_nested_diagnostic(
                    &mut graph,
                    usage,
                    limits.read,
                    Diagnostic {
                        cause: None,
                        code: "DamagedFrame".to_owned(),
                        detail: error.to_string(),
                        frame_index: None,
                    }
                    .with_native_cause(error),
                )?;
                continue;
            }
        };
        if bytes.len() != decoded_len {
            let error = NestedError::DecodedSize {
                digest,
                actual: bytes.len(),
                remaining: decoded_len,
            };
            push_nested_diagnostic(
                &mut graph,
                usage,
                limits.read,
                Diagnostic {
                    cause: None,
                    code: "DamagedFrame".to_owned(),
                    detail: error.to_string(),
                    frame_index: None,
                }
                .with_native_cause(error),
            )?;
            continue;
        }
        seen.insert(digest.clone());
        let child = visit_with_limits(
            &bytes,
            depth + 1,
            seen,
            subgraphs,
            limits,
            usage,
            nested_decoded,
        )?;
        if child.segment_heads.is_empty() {
            let error = NestedError::Unreadable {
                digest,
                diagnostics: child.diagnostics,
            };
            push_nested_diagnostic(
                &mut graph,
                usage,
                limits.read,
                Diagnostic {
                    cause: None,
                    code: "DamagedFrame".to_owned(),
                    detail: error.to_string(),
                    frame_index: None,
                }
                .with_native_cause(error),
            )?;
            continue;
        }
        subgraphs.push((digest, child));
    }
    Ok(graph)
}

struct MeasuredEntry {
    decoded: usize,
    work: usize,
}

enum MeasureError {
    Codec { cause: std::io::Error, work: usize },
    Work { required: usize },
}

fn measure_nested_entry(
    entry: &BlobEntry,
    decoded_limit: usize,
    work_limit: usize,
) -> Result<MeasuredEntry, MeasureError> {
    use std::io::Read as _;

    match entry {
        BlobEntry::Bytes(bytes) if bytes.len() > decoded_limit => Err(MeasureError::Codec {
            cause: purrdf_hash::error::io_error(
                std::io::ErrorKind::InvalidData,
                crate::codec::CodecError::Limit {
                    limit: decoded_limit,
                },
            ),
            work: 0,
        }),
        BlobEntry::Bytes(bytes) => Ok(MeasuredEntry {
            decoded: bytes.len(),
            work: bytes.len(),
        }),
        BlobEntry::Lazy { raw, chain } => {
            let (mut reader, work) = crate::codec::decode_chain_reader_with_work_limit(
                chain,
                std::io::Cursor::new(raw.as_slice()),
                decoded_limit,
                work_limit,
            )
            .map_err(|error| {
                if error.work.refused() {
                    MeasureError::Work {
                        required: error.work.total(),
                    }
                } else {
                    MeasureError::Codec {
                        cause: error.cause,
                        work: error.work.total(),
                    }
                }
            })?;
            let mut scratch = [0; 8192];
            let mut decoded = 0usize;
            loop {
                match reader.read(&mut scratch) {
                    Ok(0) => {
                        return Ok(MeasuredEntry {
                            decoded,
                            work: if work.total() == 0 {
                                decoded
                            } else {
                                work.total()
                            },
                        });
                    }
                    Ok(count) => decoded = decoded.saturating_add(count),
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(_error) if work.refused() => {
                        return Err(MeasureError::Work {
                            required: work.total(),
                        });
                    }
                    Err(error) => {
                        return Err(MeasureError::Codec {
                            cause: error,
                            work: work.total(),
                        });
                    }
                }
            }
        }
    }
}

enum MaterializeError {
    Codec(crate::codec::CodecError),
    Limit(crate::reader::GtsReadLimitError),
}

fn materialize_nested_entry(
    entry: &BlobEntry,
    limits: crate::reader::GtsReadLimits,
    usage: &mut crate::reader::GtsReadUsage,
) -> Result<Vec<u8>, MaterializeError> {
    match entry {
        BlobEntry::Bytes(bytes) => {
            charge_nested_work(usage, limits, bytes.len()).map_err(MaterializeError::Limit)?;
            Ok(bytes.clone())
        }
        BlobEntry::Lazy { raw, chain } => {
            let transformed = chain.iter().any(|codec| codec.name != "identity");
            let decoded = match crate::codec::decode_chain_with_decrypt_bounded_observed(
                chain,
                raw,
                None,
                |produced| {
                    if let Some(bytes) = produced {
                        charge_nested_work(usage, limits, bytes)
                            .map_err(MaterializeError::Limit)?;
                    }
                    Ok(limits
                        .decoded_blob_bytes
                        .min(limits.work.saturating_sub(usage.work)))
                },
                MaterializeError::Codec,
            ) {
                Err(MaterializeError::Codec(cause))
                    if error_is_codec_limit(&cause)
                        && limits.work.saturating_sub(usage.work) < limits.decoded_blob_bytes =>
                {
                    let remaining = limits.work.saturating_sub(usage.work);
                    let required = usage.work.saturating_add(remaining.saturating_add(1));
                    let mut refused_usage = *usage;
                    refused_usage.work = required;
                    return Err(MaterializeError::Limit(crate::reader::GtsReadLimitError {
                        resource: crate::reader::GtsReadResource::Work,
                        required,
                        limit: limits.work,
                        usage: refused_usage,
                    }));
                }
                outcome => outcome?,
            };
            if !transformed {
                charge_nested_work(usage, limits, decoded.len())
                    .map_err(MaterializeError::Limit)?;
            }
            Ok(decoded)
        }
    }
}

fn error_is_codec_limit(error: &(dyn std::error::Error + 'static)) -> bool {
    let mut current = Some(error);
    while let Some(cause) = current {
        if matches!(
            cause.downcast_ref::<crate::codec::CodecError>(),
            Some(crate::codec::CodecError::Limit { .. })
        ) {
            return true;
        }
        current = cause.source();
    }
    false
}

fn remaining_limits(
    limits: crate::reader::GtsReadLimits,
    usage: crate::reader::GtsReadUsage,
) -> crate::reader::GtsReadLimits {
    crate::reader::GtsReadLimits {
        encoded_input_bytes: limits
            .encoded_input_bytes
            .saturating_sub(usage.encoded_input_bytes),
        items: limits.items.saturating_sub(usage.items),
        encoded_frame_bytes: limits.encoded_frame_bytes,
        decoded_frame_bytes: limits.decoded_frame_bytes,
        decoded_blob_bytes: limits.decoded_blob_bytes,
        total_decoded_bytes: limits
            .total_decoded_bytes
            .saturating_sub(usage.total_decoded_bytes),
        terms: limits.terms.saturating_sub(usage.terms),
        rows: limits.rows.saturating_sub(usage.rows),
        metadata_entries: limits
            .metadata_entries
            .saturating_sub(usage.metadata_entries),
        diagnostics: limits.diagnostics.saturating_sub(usage.diagnostics),
        work: limits.work.saturating_sub(usage.work),
    }
}

fn aggregate_limit_error(
    prefix: crate::reader::GtsReadUsage,
    mut error: crate::reader::GtsReadLimitError,
    limits: crate::reader::GtsReadLimits,
) -> crate::reader::GtsReadLimitError {
    use crate::reader::GtsReadResource;

    let (required, limit) = match error.resource {
        GtsReadResource::EncodedInputBytes => (
            prefix.encoded_input_bytes.saturating_add(error.required),
            limits.encoded_input_bytes,
        ),
        GtsReadResource::Items => (prefix.items.saturating_add(error.required), limits.items),
        GtsReadResource::EncodedFrameBytes => (error.required, limits.encoded_frame_bytes),
        GtsReadResource::DecodedFrameBytes => (error.required, limits.decoded_frame_bytes),
        GtsReadResource::DecodedBlobBytes => (error.required, limits.decoded_blob_bytes),
        GtsReadResource::TotalDecodedBytes => (
            prefix.total_decoded_bytes.saturating_add(error.required),
            limits.total_decoded_bytes,
        ),
        GtsReadResource::Terms => (prefix.terms.saturating_add(error.required), limits.terms),
        GtsReadResource::Rows => (prefix.rows.saturating_add(error.required), limits.rows),
        GtsReadResource::MetadataEntries => (
            prefix.metadata_entries.saturating_add(error.required),
            limits.metadata_entries,
        ),
        GtsReadResource::Diagnostics => (
            prefix.diagnostics.saturating_add(error.required),
            limits.diagnostics,
        ),
        GtsReadResource::Work => (prefix.work.saturating_add(error.required), limits.work),
        GtsReadResource::NestedDecodedBytes | GtsReadResource::NestedDepth => {
            (error.required, error.limit)
        }
    };
    error.required = required;
    error.limit = limit;
    error.usage = merge_usage(prefix, error.usage);
    error
}

fn charge_nested_work(
    usage: &mut crate::reader::GtsReadUsage,
    limits: crate::reader::GtsReadLimits,
    additional: usize,
) -> Result<(), crate::reader::GtsReadLimitError> {
    usage.work = usage.work.saturating_add(additional);
    if usage.work > limits.work {
        return Err(crate::reader::GtsReadLimitError {
            resource: crate::reader::GtsReadResource::Work,
            required: usage.work,
            limit: limits.work,
            usage: *usage,
        });
    }
    Ok(())
}

fn push_nested_diagnostic(
    graph: &mut Graph,
    usage: &mut crate::reader::GtsReadUsage,
    limits: crate::reader::GtsReadLimits,
    diagnostic: Diagnostic,
) -> Result<(), crate::reader::GtsReadLimitError> {
    charge_nested_work(usage, limits, 1)?;
    usage.diagnostics = usage.diagnostics.saturating_add(1);
    if usage.diagnostics > limits.diagnostics {
        return Err(crate::reader::GtsReadLimitError {
            resource: crate::reader::GtsReadResource::Diagnostics,
            required: usage.diagnostics,
            limit: limits.diagnostics,
            usage: *usage,
        });
    }
    graph.diagnostics.push(diagnostic);
    Ok(())
}

fn accumulate_usage(
    total: &mut crate::reader::GtsReadUsage,
    next: crate::reader::GtsReadUsage,
    limits: crate::reader::GtsReadLimits,
) -> Result<(), crate::reader::GtsReadLimitError> {
    *total = merge_usage(*total, next);

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

fn merge_usage(
    mut total: crate::reader::GtsReadUsage,
    next: crate::reader::GtsReadUsage,
) -> crate::reader::GtsReadUsage {
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
    total
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
