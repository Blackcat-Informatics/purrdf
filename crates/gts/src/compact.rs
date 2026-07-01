// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Streamable compaction (GTS-SPEC §10.1): re-author the ordering, only the
//! ordering — mirror of `packages/gts/src/gts/compact.py`.
//!
//! [`compact_streamable`] rewrites an accretive GTS file (or multi-segment
//! composition) into ONE delivery-ordered segment in the streamable layout
//! state (§3.3): a leading streaming index in the `stream` vocabulary
//! (§13.3), the content graph, blobs most-significant-first, and a trailing
//! offset `index` footer. Content signatures ride through untouched; frame
//! signatures are carried *detached* in compaction provenance; the ordering
//! commitment is re-issued — the compactor is the sole attester of the new
//! ordering.
//!
//! The rewrite is byte-deterministic for the same input and parameters
//! (§14.1): blob order is ascending decoded size with digest tie-break, the
//! agent string is a constant, and the timestamp is a parameter — never
//! ambient time.

use std::borrow::Cow;

use ciborium::value::Value;

use crate::model::{Graph, Quad, ReifierRow, Suppression, Term, TermKind};
use crate::reader::{read, read_file_segments};
use crate::stream;
use crate::wire::{digest_str, hex, map_get};
use crate::writer::Writer;

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DATETIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";

/// The input is not safely compactable (§10.1/§14.1 refuse-don't-trust).
#[derive(Debug)]
pub struct CompactRefusedError(pub String);

impl std::fmt::Display for CompactRefusedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CompactRefusedError {}

fn refuse<T>(msg: String) -> Result<T, CompactRefusedError> {
    Err(CompactRefusedError(msg))
}

fn target_text<'a>(target: &'a Value, key: &str) -> Option<&'a str> {
    if let Value::Map(entries) = target {
        if let Some(Value::Text(t)) = map_get(entries, key) {
            return Some(t);
        }
    }
    None
}

/// Verify the input cleanly and return its union fold + single profile.
fn refusal_gate(data: &[u8], seal_original: bool) -> Result<(Graph, String), CompactRefusedError> {
    let fs = read_file_segments(data);
    if let Some(fatal) = &fs.fatal {
        return refuse(format!(
            "input is not a clean GTS file: {}: {}",
            fatal.code, fatal.detail
        ));
    }
    if let Some(torn) = fs.torn {
        return refuse(format!("input has a torn append at byte {torn}"));
    }
    for (idx, seg) in fs.segments.iter().enumerate() {
        if let Some(first) = seg.diagnostics.first() {
            return refuse(format!(
                "segment {idx} does not verify cleanly: {}: {}",
                first.code, first.detail
            ));
        }
    }
    let mut profiles: Vec<&str> = fs
        .segments
        .iter()
        .flat_map(|seg| seg.segment_profiles.iter().map(String::as_str))
        .collect();
    profiles.sort_unstable();
    profiles.dedup();
    if profiles.len() > 1 {
        // Python renders sorted(profiles) as a list repr — keep the text.
        let listed = profiles
            .iter()
            .map(|p| format!("'{p}'"))
            .collect::<Vec<_>>()
            .join(", ");
        return refuse(format!(
            "mixed segment profiles [{listed}] are not compactable (v1)"
        ));
    }
    let profile = profiles.first().copied().unwrap_or("generic").to_string();
    if profile == "evidence" && !seal_original {
        return refuse(
            "an 'evidence' artifact's signed chain IS the artifact; refusing \
             to re-order it without --seal-original (§10.1)"
                .to_string(),
        );
    }
    let g = read(data, true, None);
    for sup in &g.suppressions {
        for target in &sup.targets {
            if target_text(target, "kind") == Some("frame") {
                return refuse(
                    "input carries a frame-addressed suppression; the rewrite \
                     assigns new frame ids, so the target would silently \
                     dangle (§10.1)"
                        .to_string(),
                );
            }
        }
    }
    Ok((g, profile))
}

/// Accumulates the streaming-index terms and quads with stable ids.
#[derive(Default)]
struct GraphBuilder {
    terms: Vec<Term>,
    quads: Vec<Quad>,
}

impl GraphBuilder {
    fn add(&mut self, kind: TermKind, value: &str) -> usize {
        self.terms.push(Term {
            kind,
            value: Some(value.to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        self.terms.len() - 1
    }

    fn literal(&mut self, value: &str, datatype: Option<usize>) -> usize {
        self.terms.push(Term {
            kind: TermKind::Literal,
            value: Some(value.to_string()),
            datatype,
            lang: None,
            direction: None,
            reifier: None,
        });
        self.terms.len() - 1
    }

    fn quad(&mut self, s: usize, p: usize, o: usize) {
        self.quads.push((s, p, o, None));
    }
}

fn blob_decode_refused(digest: &str, err: impl std::fmt::Debug) -> CompactRefusedError {
    CompactRefusedError(format!("cannot decode blob {digest}: {err:?}"))
}

/// Look up a blob's decoded length in the insertion-ordered table.
fn blob_decoded_len(g: &Graph, digest: &str) -> Result<Option<usize>, CompactRefusedError> {
    g.blobs
        .iter()
        .find(|(d, _)| d == digest)
        .map(|(_, entry)| {
            entry
                .decoded_len()
                .map_err(|err| blob_decode_refused(digest, err))
        })
        .transpose()
}

/// Look up a blob's decoded bytes in the insertion-ordered table.
fn blob_bytes<'a>(
    g: &'a Graph,
    digest: &str,
) -> Result<Option<Cow<'a, [u8]>>, CompactRefusedError> {
    g.blobs
        .iter()
        .find(|(d, _)| d == digest)
        .map(|(_, entry)| {
            entry
                .decoded_bytes()
                .map_err(|err| blob_decode_refused(digest, err))
        })
        .transpose()
}

/// A declared text field (`mt`/`rep`) from a blob's `pub` metadata (§12).
fn blob_meta_text(g: &Graph, digest: &str, key: &str) -> Option<String> {
    g.blob_meta
        .iter()
        .find(|(d, _)| d == digest)
        .and_then(|(_, meta)| {
            if let Value::Map(entries) = meta {
                if let Some(Value::Text(t)) = map_get(entries, key) {
                    return Some(t.clone());
                }
            }
            None
        })
}

/// Base64url WITHOUT padding (RFC 4648 §5) — the `stream:cose` literal form.
fn base64url_unpadded(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[n as usize & 63] as char);
        }
    }
    out
}

/// Build the leading streaming index + compaction provenance (§3.3, §13.3).
fn streaming_index(
    g: &Graph,
    blob_order: &[String],
    timestamp: &str,
    sealed_digest: Option<&str>,
    sealed_size: Option<usize>,
) -> Result<GraphBuilder, CompactRefusedError> {
    let mut b = GraphBuilder::default();
    // Fixed vocabulary block — constant ids across engines for determinism.
    let t_type = b.add(TermKind::Iri, RDF_TYPE);
    let t_int = b.add(TermKind::Iri, XSD_INTEGER);
    let t_dt = b.add(TermKind::Iri, XSD_DATETIME);
    let t_manifestation = b.add(TermKind::Iri, stream::MANIFESTATION);
    let t_digest = b.add(TermKind::Iri, stream::DIGEST);
    let t_mt = b.add(TermKind::Iri, stream::MEDIA_TYPE);
    let t_size = b.add(TermKind::Iri, stream::SIZE);
    let t_role = b.add(TermKind::Iri, stream::ROLE);
    let t_order = b.add(TermKind::Iri, stream::ORDER);
    let t_compaction = b.add(TermKind::Iri, stream::COMPACTION);
    let t_agent = b.add(TermKind::Iri, stream::AGENT);
    let t_timestamp = b.add(TermKind::Iri, stream::TIMESTAMP);
    let t_source_head = b.add(TermKind::Iri, stream::SOURCE_HEAD);
    let t_sealed_source = b.add(TermKind::Iri, stream::SEALED_SOURCE);
    let t_detached_sig = b.add(TermKind::Iri, stream::DETACHED_SIGNATURE);
    let t_source_frame = b.add(TermKind::Iri, stream::SOURCE_FRAME);
    let t_cose = b.add(TermKind::Iri, stream::COSE);

    // One Manifestation per promised blob, in delivery order.
    for (order, digest) in blob_order.iter().enumerate() {
        let m = b.add(TermKind::Bnode, &format!("m{order}"));
        let sealed = Some(digest.as_str()) == sealed_digest;
        let size = if sealed {
            sealed_size
        } else {
            blob_decoded_len(g, digest)?
        };
        let mt = if sealed {
            Some("application/vnd.blackcat.gts+cbor-seq".to_string())
        } else {
            blob_meta_text(g, digest, "mt")
        };
        b.quad(m, t_type, t_manifestation);
        let o = b.literal(digest, None);
        b.quad(m, t_digest, o);
        if let Some(mt) = mt {
            let o = b.literal(&mt, None);
            b.quad(m, t_mt, o);
        }
        if let Some(size) = size {
            let o = b.literal(&size.to_string(), Some(t_int));
            b.quad(m, t_size, o);
        }
        let o = b.literal(if sealed { "source" } else { "primary" }, None);
        b.quad(m, t_role, o);
        let o = b.literal(&order.to_string(), Some(t_int));
        b.quad(m, t_order, o);
    }

    // The Compaction provenance node (§10.1).
    let c = b.add(TermKind::Bnode, "c");
    b.quad(c, t_type, t_compaction);
    let o = b.literal(stream::COMPACT_AGENT, None);
    b.quad(c, t_agent, o);
    let o = b.literal(timestamp, Some(t_dt));
    b.quad(c, t_timestamp, o);
    for head in &g.segment_heads {
        let o = b.literal(&format!("blake3:{}", hex(head)), None);
        b.quad(c, t_source_head, o);
    }
    if let Some(sealed) = sealed_digest {
        let o = b.literal(sealed, None);
        b.quad(c, t_sealed_source, o);
    }

    // Detached frame signatures (§10.1): checkable claims about the original log.
    for (j, sig) in g.signatures.iter().filter(|s| s.cose.is_some()).enumerate() {
        let node = b.add(TermKind::Bnode, &format!("s{j}"));
        let cose_b64 = base64url_unpadded(sig.cose.as_deref().unwrap_or(&[]));
        b.quad(node, t_type, t_detached_sig);
        let o = b.literal(&format!("blake3:{}", hex(&sig.frame_id)), None);
        b.quad(node, t_source_frame, o);
        let o = b.literal(&cose_b64, None);
        b.quad(node, t_cose, o);
    }
    Ok(b)
}

/// Shift a term's id references into the output id space.
fn shift_term(t: &Term, base: usize) -> Term {
    Term {
        kind: t.kind,
        value: t.value.clone(),
        datatype: t.datatype.map(|d| d + base),
        lang: t.lang.clone(),
        direction: t.direction.clone(),
        reifier: t.reifier.map(|r| r + base),
    }
}

/// Carry suppressions forward, one output suppression per input (§10.1).
///
/// Re-authoring of the ordering only: each original suppression keeps its own
/// frame with its `reason`/`by` metadata intact — blob targets verbatim
/// (content-addressing is layout-independent), id-addressed targets and `by`
/// shifted into the output id space.
fn shifted_suppressions(g: &Graph, base: usize) -> Vec<Suppression> {
    let mut out: Vec<Suppression> = Vec::new();
    for sup in &g.suppressions {
        let mut targets: Vec<Value> = Vec::new();
        for target in &sup.targets {
            let Value::Map(entries) = target else {
                targets.push(target.clone());
                continue;
            };
            let kind = target_text(target, "kind").unwrap_or("");
            let shifted: Vec<(Value, Value)> = entries
                .iter()
                .map(|(k, v)| {
                    let key = if let Value::Text(t) = k {
                        t.as_str()
                    } else {
                        ""
                    };
                    if (kind == "term" || kind == "reifier") && key == "id" {
                        if let Some(tid) = value_idx(v) {
                            return (k.clone(), Value::from((tid + base) as u64));
                        }
                    } else if kind == "quad" && key == "q" {
                        if let Value::Array(ids) = v {
                            let remapped: Vec<Value> = ids
                                .iter()
                                .map(|x| match value_idx(x) {
                                    Some(tid) => Value::from((tid + base) as u64),
                                    None => x.clone(),
                                })
                                .collect();
                            return (k.clone(), Value::Array(remapped));
                        }
                    }
                    (k.clone(), v.clone())
                })
                .collect();
            targets.push(Value::Map(shifted));
        }
        out.push(Suppression {
            targets,
            reason: sup.reason.clone(),
            by: sup.by.map(|b| b + base),
        });
    }
    out
}

fn value_idx(v: &Value) -> Option<usize> {
    if let Value::Integer(i) = v {
        usize::try_from(i128::from(*i)).ok()
    } else {
        None
    }
}

/// Rewrite a GTS file into one streamable segment (§10.1).
///
/// `data` must verify cleanly (refuse-don't-trust). `timestamp` is the
/// rewrite time recorded as `stream:timestamp` — an explicit parameter so
/// the output is byte-reproducible. `seal_original` carries the verbatim
/// source bytes as a nested GTS blob (§12.1), role `"source"` — REQUIRED
/// for `evidence` input.
pub fn compact_streamable(
    data: &[u8],
    timestamp: &str,
    seal_original: bool,
) -> Result<Vec<u8>, CompactRefusedError> {
    let (mut g, profile) = refusal_gate(data, seal_original)?;

    // Delivery plan: most-significant-first — ascending decoded size, digest
    // tie-break; the sealed original (least significant) always travels last.
    // Decode once here so the streaming index and re-emission reuse cached
    // bytes instead of repeating lazy decode work.
    let mut keyed: Vec<(usize, String)> = g
        .blobs
        .iter_mut()
        .map(|(d, entry)| {
            entry
                .decode()
                .map(|bytes| (bytes.len(), d.clone()))
                .map_err(|err| blob_decode_refused(d, err))
        })
        .collect::<Result<_, _>>()?;
    keyed.sort_unstable();
    let mut blob_order: Vec<String> = keyed.into_iter().map(|(_, d)| d).collect();
    let mut sealed_digest: Option<String> = None;
    if seal_original {
        let sealed = digest_str(data);
        blob_order.retain(|d| *d != sealed);
        blob_order.push(sealed.clone());
        sealed_digest = Some(sealed);
    }

    let index = streaming_index(
        &g,
        &blob_order,
        timestamp,
        sealed_digest.as_deref(),
        sealed_digest.as_ref().map(|_| data.len()),
    )?;
    let base = index.terms.len();

    let mut w = Writer::with_layout(&profile, Some("streamable"));
    // Leading streaming index: the catalog presages everything below it.
    w.add_terms(&index.terms);
    w.add_quads(&index.quads);
    // Content graph, re-emitted from the folded union (ids shifted by `base`).
    if !g.terms.is_empty() {
        let shifted: Vec<Term> = g.terms.iter().map(|t| shift_term(t, base)).collect();
        w.add_terms(&shifted);
    }
    if !g.quads.is_empty() {
        let shifted: Vec<Quad> = g
            .quads
            .iter()
            .map(|&(s, p, o, gr)| (s + base, p + base, o + base, gr.map(|x| x + base)))
            .collect();
        w.add_quads(&shifted);
    }
    if !g.reifiers.is_empty() {
        let shifted: Vec<ReifierRow> = g
            .reifiers
            .iter()
            .map(|&(r, (s, p, o), gr)| {
                (
                    r + base,
                    (s + base, p + base, o + base),
                    gr.map(|x| x + base),
                )
            })
            .collect();
        w.add_reifies(&shifted);
    }
    if !g.annotations.is_empty() {
        let shifted: Vec<(usize, usize, usize, Option<usize>)> = g
            .annotations
            .iter()
            .map(|&(r, p, v, gr)| (r + base, p + base, v + base, gr.map(|x| x + base)))
            .collect();
        w.add_annot(&shifted);
    }
    for sup in shifted_suppressions(&g, base) {
        w.add_suppress(sup.targets, sup.reason.as_deref(), sup.by);
    }
    // Blobs in delivery order; declared metadata rides along.
    for digest in &blob_order {
        if Some(digest.as_str()) == sealed_digest.as_deref() {
            w.add_blob(
                data,
                Some("application/vnd.blackcat.gts+cbor-seq"),
                Some("source"),
            );
            continue;
        }
        let mt = blob_meta_text(&g, digest, "mt");
        let rep = blob_meta_text(&g, digest, "rep");
        let Some(bytes) = blob_bytes(&g, digest)? else {
            continue;
        };
        match bytes {
            Cow::Borrowed(bytes) => {
                w.add_blob(bytes, mt.as_deref(), rep.as_deref());
            }
            Cow::Owned(bytes) => {
                w.add_blob_owned(bytes, mt.as_deref(), rep.as_deref());
            }
        }
    }
    // The re-issued ordering commitment: the compactor is its sole attester.
    w.add_index();
    Ok(w.into_bytes())
}
