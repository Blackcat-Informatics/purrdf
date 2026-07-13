// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The GTS reader: parse a CBOR Sequence, verify the id/prev chain, fold the
//! log — mirror of `src/purrdf_tools/gts/reader.py`.
//!
//! Implements the Baseline Reader contract (§2.1): chain verification (§9.1),
//! the value-union fold (§7.5), opaque/damaged degradation (§7.6), torn-append
//! detection (§3), and the canonical diagnostics (§2.3). The default
//! [`read`] path carries no content keys: `sig` frames record as
//! `"unverified"` and `encrypt`-class frames degrade to `missing-key` opaque
//! nodes. Callers that hold content keys can use [`read_with_options`].

use std::collections::{HashMap, HashSet};
use std::io::Read;

use ciborium::value::Value;

use crate::codec::{Codec, CodecError, decode_chain, decode_chain_with_decrypt};
use crate::model::{
    AnnotationRow, Diagnostic, Graph, OpaqueNode, Quad, ReifierRow, Signature, StreamableInfo,
    Suppression, Term, TermKind, Triple3,
};
use crate::reader_layout::{IndexRecord, check_index_mmr, layout_check};
use crate::reader_rows::{
    RowDecode, check_quad_positions, decode_annotation_row, decode_reifier_row,
};
use crate::reader_union::union_segments;
use crate::stream::DIGEST as STREAM_DIGEST;
use crate::wire::{
    MAGIC, VERSION, content_id, digest_str, header_id, hex, iter_items, map_get, unwrap_header,
};

pub(crate) fn as_i128(v: &Value) -> Option<i128> {
    if let Value::Integer(i) = v {
        Some(i128::from(*i))
    } else {
        None
    }
}

/// Coerce a value to a non-negative index, else `None` (Python `_as_int`).
pub(crate) fn as_idx(v: &Value) -> Option<usize> {
    as_i128(v).and_then(|n| usize::try_from(n).ok())
}

pub(crate) fn as_text(v: &Value) -> Option<&str> {
    if let Value::Text(t) = v {
        Some(t)
    } else {
        None
    }
}

pub(crate) fn text_or<'a>(v: Option<&'a Value>, default: &'a str) -> &'a str {
    v.and_then(as_text).unwrap_or(default)
}

fn diag_code_for(reason: &str) -> &'static str {
    match reason {
        "missing-key" => "MissingKey",
        _ => "UnknownCodec",
    }
}

fn pub_digest(value: &Value) -> Option<String> {
    let Value::Map(entries) = value else {
        return None;
    };
    match map_get(entries, "digest") {
        Some(Value::Text(text)) if text.starts_with("blake3:") => Some(text.clone()),
        Some(Value::Text(text)) => Some(format!("blake3:{text}")),
        Some(Value::Bytes(bytes)) if bytes.len() == 32 => Some(format!("blake3:{}", hex(bytes))),
        _ => None,
    }
}

fn term_depends_on_anchor(
    graph: &Graph,
    term_id: usize,
    anchor: usize,
    pending: (usize, Triple3),
    seen: &mut HashSet<usize>,
) -> bool {
    if term_id == anchor {
        return true;
    }
    if !seen.insert(term_id) {
        return false;
    }
    let Some(term) = graph.terms.get(term_id) else {
        return false;
    };
    if term.kind != TermKind::Triple {
        return false;
    }
    let Some(reifier) = term.reifier else {
        return false;
    };
    let binding = if reifier == pending.0 {
        Some(pending.1)
    } else {
        graph.reifier(reifier)
    };
    let Some(triple) = binding else {
        return false;
    };
    <[usize; 3]>::from(triple)
        .into_iter()
        .any(|component| term_depends_on_anchor(graph, component, anchor, pending, seen))
}

fn reifier_binding_is_recursive(graph: &Graph, rid: usize, triple: Triple3) -> bool {
    graph
        .terms
        .iter()
        .enumerate()
        .filter(|(_, term)| term.kind == TermKind::Triple && term.reifier == Some(rid))
        .any(|(anchor, _)| {
            [triple.0, triple.1, triple.2].into_iter().any(|component| {
                let mut seen = HashSet::new();
                term_depends_on_anchor(graph, component, anchor, (rid, triple), &mut seen)
            })
        })
}

enum PayloadError {
    /// Missing capability — degrade to an opaque node with this reason.
    Unavailable {
        reason: &'static str,
        detail: String,
    },
    /// Anything else — the frame is damaged.
    Damaged(String),
}

impl From<CodecError> for PayloadError {
    fn from(e: CodecError) -> Self {
        match e {
            CodecError::Unavailable { reason, detail } => Self::Unavailable { reason, detail },
            CodecError::Failed(detail) => Self::Damaged(detail),
        }
    }
}

fn decrypt_codec(
    codec: &Codec,
    data: &[u8],
    content_key: &ContentKeyResolver<'_>,
) -> Result<Vec<u8>, CodecError> {
    if codec.name != "cose-encrypt0" {
        return Err(CodecError::Unavailable {
            reason: "missing-key",
            detail: format!("no decryptor for encrypt codec '{}'", codec.name),
        });
    }
    crate::cose::decrypt0(data, |kid| content_key(kid)).map_err(|err| CodecError::Unavailable {
        reason: "missing-key",
        detail: format!("{} decrypt failed: {err}", codec.name),
    })
}

/// Final state returned by [`read_to_sink`].
///
/// The streaming API emits rows as segment-local events and does not construct
/// the final union [`Graph`]. This result carries the observable file-level
/// state that callers still need for verification and freshness checks.
#[derive(Clone, Debug, Default)]
pub struct StreamingReadResult {
    /// Reader diagnostics in the same order as [`read`] would report them.
    pub diagnostics: Vec<Diagnostic>,
    /// Ordered per-segment head ids.
    pub segment_heads: Vec<Vec<u8>>,
    /// Ordered per-segment profile names.
    pub segment_profiles: Vec<String>,
    /// Ordered per-segment metadata snapshots.
    pub segment_meta: Vec<Vec<(String, Value)>>,
    /// Ordered per-segment streamable-layout state.
    pub segment_streamable: Vec<StreamableInfo>,
    /// Byte offset of a torn trailing CBOR item, if present.
    pub torn: Option<usize>,
}

/// Sink for [`read_to_sink`] events.
///
/// Term ids in events are segment-local ids. A sink that needs a file-level
/// union should intern by value using the same rules as [`read`]; sinks that
/// project rows into a table can usually consume the segment-local ids directly
/// with the segment index as the scope.
pub trait StreamingSink {
    /// Accepted term row.
    fn term(&mut self, _segment_index: usize, _term_id: usize, _term: &Term) {}
    /// Accepted quad row.
    fn quad(&mut self, _segment_index: usize, _quad: Quad) {}
    /// Accepted reifier row.
    fn reifier(&mut self, _segment_index: usize, _reifier: ReifierRow) {}
    /// Accepted annotation row.
    fn annotation(&mut self, _segment_index: usize, _annotation: AnnotationRow) {}
    /// Accepted suppression directive.
    fn suppression(&mut self, _segment_index: usize, _suppression: &Suppression) {}
    /// Accepted inline blob digest and declared metadata.
    fn blob(&mut self, _segment_index: usize, _digest: &str, _meta: Option<&Value>) {}
    /// Opaque frame produced by unknown, encrypted, or damaged payloads.
    fn opaque(&mut self, _segment_index: usize, _opaque: &OpaqueNode) {}
    /// Signature status observed on a frame.
    fn signature(&mut self, _segment_index: usize, _signature: &Signature) {}
    /// Reader diagnostic.
    fn diagnostic(&mut self, _diagnostic: &Diagnostic) {}
    /// Completed segment head.
    fn segment_head(&mut self, _segment_index: usize, _head: &[u8]) {}
    /// Completed segment streamable-layout state.
    fn streamable_layout(&mut self, _segment_index: usize, _info: &StreamableInfo) {}
}

/// Resolves a 32-byte content key by COSE recipient `kid`.
pub type ContentKeyResolver<'a> = dyn Fn(&str) -> Option<[u8; 32]> + 'a;

/// Reader options for keyed/full-reader paths.
#[derive(Clone, Copy, Default)]
pub struct ReadOptions<'a> {
    /// Permit multi-segment `cat` composition.
    pub allow_segments: bool,
    /// Expected last segment head for freshness/truncation checks.
    pub expected_head: Option<&'a [u8]>,
    /// Optional content-key provider for `COSE_Encrypt0` payloads.
    pub content_key: Option<&'a ContentKeyResolver<'a>>,
}

impl std::fmt::Debug for ReadOptions<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadOptions")
            .field("allow_segments", &self.allow_segments)
            .field("expected_head", &self.expected_head)
            .field("content_key", &self.content_key.map(|_| "<resolver>"))
            .finish()
    }
}

impl<'a> ReadOptions<'a> {
    /// Build options matching the legacy [`read`] signature.
    pub fn new(allow_segments: bool, expected_head: Option<&'a [u8]>) -> Self {
        Self {
            allow_segments,
            expected_head,
            content_key: None,
        }
    }

    /// Add a content-key provider for decrypting `COSE_Encrypt0` frames.
    #[must_use]
    pub fn with_content_key(mut self, resolver: &'a ContentKeyResolver<'a>) -> Self {
        self.content_key = Some(resolver);
        self
    }
}

pub(crate) fn push_diagnostic(
    g: &mut Graph,
    sink: &mut Option<&mut dyn StreamingSink>,
    diagnostic: Diagnostic,
) {
    if let Some(sink) = sink.as_deref_mut() {
        sink.diagnostic(&diagnostic);
    }
    g.diagnostics.push(diagnostic);
}

fn push_result_diagnostic(
    result: &mut StreamingReadResult,
    sink: &mut dyn StreamingSink,
    diagnostic: Diagnostic,
) {
    sink.diagnostic(&diagnostic);
    result.diagnostics.push(diagnostic);
}

fn absorb_segment_result(result: &mut StreamingReadResult, segment: &Graph) {
    result
        .diagnostics
        .extend(segment.diagnostics.iter().cloned());
    result
        .segment_heads
        .extend(segment.segment_heads.iter().cloned());
    result
        .segment_profiles
        .extend(segment.segment_profiles.iter().cloned());
    result
        .segment_meta
        .extend(segment.segment_meta.iter().cloned());
    result
        .segment_streamable
        .extend(segment.segment_streamable.iter().cloned());
}

/// Mutable fold state; one per segment (and shared by the snapshot handler).
struct Folder<'g, 's, 'k> {
    g: &'g mut Graph,
    sink: Option<&'s mut dyn StreamingSink>,
    content_key: Option<&'k ContentKeyResolver<'k>>,
    segment_index: usize,
    materialize: bool,
    catalog: HashMap<i128, Codec>,
    // Layout-state bookkeeping (§3.3): intact index frames seen, digests the
    // graph has described via stream:digest so far, and each inline blob's
    // arrival (frame index, digest, was-it-described-at-arrival).
    index_records: Vec<IndexRecord>,
    described: HashSet<String>,
    blob_events: Vec<(usize, String, bool)>,
}

impl Folder<'_, '_, '_> {
    fn with_sink(&mut self, f: impl FnOnce(usize, &mut dyn StreamingSink)) {
        if let Some(sink) = self.sink.as_deref_mut() {
            f(self.segment_index, sink);
        }
    }

    fn diag(&mut self, code: &str, detail: String, index: Option<usize>) {
        push_diagnostic(
            self.g,
            &mut self.sink,
            Diagnostic {
                code: code.to_string(),
                detail,
                frame_index: index,
            },
        );
    }

    fn emit_blob(&mut self, digest: &str) {
        let meta = self
            .g
            .blob_meta
            .iter()
            .find(|(stored, _)| stored == digest)
            .map(|(_, meta)| meta.clone());
        self.with_sink(|segment_index, sink| sink.blob(segment_index, digest, meta.as_ref()));
    }

    fn push_opaque(&mut self, opaque: OpaqueNode) {
        self.with_sink(|segment_index, sink| sink.opaque(segment_index, &opaque));
        if self.materialize {
            self.g.opaque.push(opaque);
        }
    }

    fn push_signature(&mut self, signature: Signature) {
        self.with_sink(|segment_index, sink| sink.signature(segment_index, &signature));
        if self.materialize {
            self.g.signatures.push(signature);
        }
    }

    fn resolve_codecs(&self, ids: &[Value]) -> Result<Vec<Codec>, PayloadError> {
        let mut chain = Vec::with_capacity(ids.len());
        for cid in ids {
            let codec = as_i128(cid).and_then(|c| self.catalog.get(&c));
            match codec {
                Some(c) => chain.push(c.clone()),
                None => {
                    return Err(PayloadError::Unavailable {
                        reason: "unknown-codec",
                        detail: format!("codec id {cid:?} not in catalog"),
                    });
                }
            }
        }
        Ok(chain)
    }

    /// Resolve a frame's logical payload (§6.1); error on missing capability.
    fn payload(&self, frame: &[(Value, Value)], blob: bool) -> Result<Value, PayloadError> {
        let d = map_get(frame, "d");
        if let Some(Value::Array(ids)) = map_get(frame, "x")
            && !ids.is_empty()
        {
            let Some(Value::Bytes(db)) = d else {
                return Err(PayloadError::Damaged(
                    "transformed frame 'd' must be a byte string".to_string(),
                ));
            };
            let chain = self.resolve_codecs(ids)?;
            let decoded = if let Some(content_key) = self.content_key {
                let decrypt = |codec: &Codec, data: &[u8]| decrypt_codec(codec, data, content_key);
                decode_chain_with_decrypt(&chain, db, Some(&decrypt))?
            } else {
                decode_chain(&chain, db)?
            };
            if blob {
                return Ok(Value::Bytes(decoded));
            }
            return ciborium::de::from_reader(&decoded[..])
                .map_err(|e| PayloadError::Damaged(e.to_string()));
        }
        Ok(d.cloned().unwrap_or(Value::Null))
    }

    /// Fold one already-verified frame into the graph.
    ///
    /// Total: a missing capability degrades to an opaque node, and a corrupt
    /// payload degrades to a `damaged` opaque node — the reader never aborts.
    fn fold_frame(&mut self, frame: &[(Value, Value)], index: usize) {
        let ftype = text_or(map_get(frame, "t"), "").to_string();
        if ftype == "blob" {
            self.h_blob_frame(frame, index);
            return;
        }
        let payload = match self.payload(frame, false) {
            Err(PayloadError::Unavailable { reason, detail }) => {
                self.opaque(frame, &ftype, reason);
                self.diag(diag_code_for(reason), detail, Some(index));
                return;
            }
            Err(PayloadError::Damaged(detail)) => {
                self.opaque(frame, &ftype, "damaged");
                self.diag(
                    "DamagedFrame",
                    format!("payload decode failed: {detail}"),
                    Some(index),
                );
                return;
            }
            Ok(p) => p,
        };
        match ftype.as_str() {
            "terms" => self.h_terms(&payload, index),
            "quads" => self.h_quads(&payload, index),
            "reifies" => self.h_reifies(&payload, index),
            "annot" => self.h_annot(&payload, index),
            "meta" => self.h_meta(&payload),
            "suppress" => self.h_suppress(&payload),
            "snapshot" => self.h_snapshot(&payload, index),
            "index" => self.h_index(&payload, index),
            "opaque" => self.h_opaque(&payload),
            _ => {
                self.opaque(frame, &ftype, "unknown-frame-type");
                self.diag(
                    "UnknownFrameType",
                    format!("unsupported frame type {ftype:?}"),
                    Some(index),
                );
            }
        }
    }

    // -- per-type handlers ---------------------------------------------------

    fn h_terms(&mut self, payload: &Value, index: usize) {
        let Value::Array(rows) = payload else { return };
        for raw in rows {
            let Value::Map(entries) = raw else { continue };
            let kind = TermKind::from_wire(map_get(entries, "k").and_then(as_i128));
            let value = map_get(entries, "v").and_then(as_text).map(str::to_string);
            let lang = map_get(entries, "l").and_then(as_text).map(str::to_string);
            let direction = map_get(entries, "dir")
                .and_then(as_text)
                .filter(|value| matches!(*value, "ltr" | "rtl"))
                .map(str::to_string);
            let dt_raw = map_get(entries, "dt").and_then(as_i128);
            let rf_raw = map_get(entries, "rf").and_then(as_i128);
            let tid = self.g.terms.len() as i128;
            let term_id = self.g.terms.len();
            // Sanitise refs: dt MUST name an already-introduced term, and rf
            // normally does too (§7.5). A quoted-triple term may self-bind
            // its reifier (`rf == term_id`) so the term can be used directly
            // as an RDF 1.2 triple term while the following `reifies` frame
            // supplies the SPO binding.
            let sanitize_prior = |r: Option<i128>| match r {
                Some(d) if (0..tid).contains(&d) => Some(d as usize),
                _ => None,
            };
            let dt = sanitize_prior(dt_raw);
            let rf = match rf_raw {
                Some(d) if (0..tid).contains(&d) => Some(d as usize),
                Some(d) if kind == TermKind::Triple && d == tid => Some(d as usize),
                _ => None,
            };
            let dt_out_of_range = matches!(dt_raw, Some(d) if d >= tid);
            let rf_out_of_range =
                matches!(rf_raw, Some(d) if d >= tid && !(kind == TermKind::Triple && d == tid));
            if dt_out_of_range || rf_out_of_range {
                self.diag(
                    "ForwardReference",
                    format!("term {tid} has an out-of-range ref"),
                    Some(index),
                );
            }
            self.g.terms.push(Term {
                kind,
                value,
                datatype: dt,
                lang,
                direction,
                reifier: rf,
            });
            if let Some(sink) = self.sink.as_deref_mut() {
                sink.term(self.segment_index, term_id, &self.g.terms[term_id]);
            }
        }
    }

    fn h_quads(&mut self, payload: &Value, index: usize) {
        let Value::Array(rows) = payload else { return };
        for row in rows {
            let Value::Array(items) = row else { continue };
            if items.len() < 3 {
                continue;
            }
            let (s, p, o) = (as_idx(&items[0]), as_idx(&items[1]), as_idx(&items[2]));
            let has_graph = items.len() >= 4;
            let gslot = if has_graph { as_idx(&items[3]) } else { None };
            if s.is_none() || p.is_none() || o.is_none() || (has_graph && gslot.is_none()) {
                self.diag(
                    "DamagedFrame",
                    "quad has non-integer term ids".to_string(),
                    Some(index),
                );
                continue;
            }
            let (Some(s), Some(p), Some(o)) = (s, p, o) else {
                continue;
            };
            if let Err(detail) = check_quad_positions(self.g, s, p, o, gslot) {
                self.diag("PositionConstraint", detail, Some(index));
                continue;
            }
            let quad = (s, p, o, gslot);
            self.with_sink(|segment_index, sink| sink.quad(segment_index, quad));
            if self.materialize {
                self.g.quads.push(quad);
            }
            // Layout bookkeeping (§3.3): a stream:digest quad describes an
            // upcoming manifestation — record the IOU for the blob check.
            if self.g.terms[p].value.as_deref() == Some(STREAM_DIGEST)
                && let Some(obj) = &self.g.terms[o].value
            {
                self.described.insert(obj.clone());
            }
        }
    }

    fn h_reifies(&mut self, payload: &Value, index: usize) {
        let Value::Array(rows) = payload else {
            self.diag(
                "DamagedFrame",
                "reifies payload must be a row array".to_string(),
                Some(index),
            );
            return;
        };
        for row in rows {
            let (rid, triple, gslot) = match decode_reifier_row(row, self.g) {
                RowDecode::Skip => continue,
                RowDecode::Row(row) => row,
                RowDecode::Damaged(detail) => {
                    self.diag("DamagedFrame", detail.to_string(), Some(index));
                    continue;
                }
                RowDecode::Position(detail) => {
                    self.diag("PositionConstraint", detail, Some(index));
                    continue;
                }
            };
            if let Some(existing) = self.g.reifier(rid)
                && existing != triple
            {
                self.diag(
                    "ConflictingReifier",
                    format!("reifier {rid} rebound"),
                    Some(index),
                );
                continue; // keep the first binding
            }
            if reifier_binding_is_recursive(self.g, rid, triple) {
                self.diag(
                    "DamagedFrame",
                    format!("reifier {rid} creates a recursive quoted-triple binding"),
                    Some(index),
                );
                continue;
            }
            self.g.set_reifier(rid, triple, gslot);
            self.with_sink(|segment_index, sink| sink.reifier(segment_index, (rid, triple, gslot)));
        }
    }

    fn h_annot(&mut self, payload: &Value, index: usize) {
        let Value::Array(rows) = payload else { return };
        for row in rows {
            let annotation = match decode_annotation_row(row, self.g) {
                RowDecode::Skip => continue,
                RowDecode::Row(row) => row,
                RowDecode::Damaged(detail) => {
                    self.diag("DamagedFrame", detail.to_string(), Some(index));
                    continue;
                }
                RowDecode::Position(detail) => {
                    self.diag("PositionConstraint", detail, Some(index));
                    continue;
                }
            };
            self.with_sink(|segment_index, sink| sink.annotation(segment_index, annotation));
            if self.materialize {
                self.g.annotations.push(annotation);
            }
        }
    }

    fn h_blob_frame(&mut self, frame: &[(Value, Value)], index: usize) {
        let d = map_get(frame, "d");
        let pub_meta = map_get(frame, "pub")
            .filter(|value| matches!(value, Value::Map(_)))
            .cloned();

        let chain = match map_get(frame, "x") {
            Some(Value::Array(ids)) if !ids.is_empty() => match self.resolve_codecs(ids) {
                Ok(chain) => chain,
                Err(PayloadError::Unavailable { reason, detail }) => {
                    self.opaque(frame, "blob", reason);
                    self.diag(diag_code_for(reason), detail, Some(index));
                    return;
                }
                Err(PayloadError::Damaged(detail)) => {
                    self.opaque(frame, "blob", "damaged");
                    self.diag(
                        "DamagedFrame",
                        format!("payload decode failed: {detail}"),
                        Some(index),
                    );
                    return;
                }
            },
            _ => Vec::new(),
        };

        if chain.iter().any(|codec| codec.cls == "encrypt") {
            match self.payload(frame, true) {
                Ok(Value::Bytes(bytes)) => {
                    let digest = digest_str(&bytes);
                    if let Some(meta) = pub_meta {
                        self.g.set_blob_meta(digest.clone(), meta);
                    }
                    self.blob_events.push((
                        index,
                        digest.clone(),
                        self.described.contains(&digest),
                    ));
                    if self.materialize {
                        self.g.set_blob(digest.clone(), bytes);
                    }
                    self.emit_blob(&digest);
                }
                Ok(_) => {}
                Err(PayloadError::Unavailable { reason, detail }) => {
                    self.opaque(frame, "blob", reason);
                    self.diag(diag_code_for(reason), detail, Some(index));
                }
                Err(PayloadError::Damaged(detail)) => {
                    self.opaque(frame, "blob", "damaged");
                    self.diag(
                        "DamagedFrame",
                        format!("payload decode failed: {detail}"),
                        Some(index),
                    );
                }
            }
            return;
        }

        if let Some(digest) = pub_meta.as_ref().and_then(pub_digest) {
            if let Some(meta) = pub_meta {
                self.g.set_blob_meta(digest.clone(), meta);
            }
            if let Some(Value::Bytes(raw)) = d
                && self.materialize
            {
                if chain.is_empty() {
                    self.g.set_blob(digest.clone(), raw.clone());
                } else {
                    self.g.set_lazy_blob(digest.clone(), raw.clone(), chain);
                }
            }
            self.blob_events
                .push((index, digest.clone(), self.described.contains(&digest)));
            self.emit_blob(&digest);
            return;
        }

        let Some(Value::Bytes(_)) = d else {
            return;
        };
        match self.payload(frame, true) {
            Ok(Value::Bytes(bytes)) => {
                let digest = digest_str(&bytes);
                if let Some(meta) = pub_meta {
                    self.g.set_blob_meta(digest.clone(), meta);
                }
                self.blob_events
                    .push((index, digest.clone(), self.described.contains(&digest)));
                if self.materialize {
                    self.g.set_blob(digest.clone(), bytes);
                }
                self.emit_blob(&digest);
            }
            Ok(_) => {}
            Err(PayloadError::Unavailable { reason, detail }) => {
                self.opaque(frame, "blob", reason);
                self.diag(diag_code_for(reason), detail, Some(index));
            }
            Err(PayloadError::Damaged(detail)) => {
                self.opaque(frame, "blob", "damaged");
                self.diag(
                    "DamagedFrame",
                    format!("payload decode failed: {detail}"),
                    Some(index),
                );
            }
        }
    }

    fn h_meta(&mut self, payload: &Value) {
        if let Value::Map(entries) = payload {
            for (k, v) in entries {
                let key = as_text(k).map_or_else(|| format!("{k:?}"), str::to_string);
                self.g.set_meta(key, v.clone());
            }
        }
    }

    fn h_suppress(&mut self, payload: &Value) {
        let Value::Map(entries) = payload else { return };
        let Some(Value::Array(targets)) = map_get(entries, "targets") else {
            return;
        };
        let suppression = Suppression {
            targets: targets
                .iter()
                .filter(|t| matches!(t, Value::Map(_)))
                .cloned()
                .collect(),
            reason: map_get(entries, "reason")
                .and_then(as_text)
                .map(str::to_string),
            by: map_get(entries, "by").and_then(as_idx),
        };
        self.with_sink(|segment_index, sink| sink.suppression(segment_index, &suppression));
        if self.materialize {
            self.g.suppressions.push(suppression);
        }
    }

    /// Fold a self-contained snapshot (§10).
    ///
    /// Shifts the snapshot's local term ids into the outer id space and
    /// re-dispatches through the normal handlers, so a snapshot gets the SAME
    /// semantic checks as the equivalent streamed frames.
    fn h_snapshot(&mut self, payload: &Value, index: usize) {
        let Value::Map(entries) = payload else { return };
        let base = self.g.terms.len();
        // Shift a valid local id into the outer space; pass non-ints through
        // so the downstream handler's own checks reject them with diagnostics.
        let sh = |v: &Value| -> Value {
            match as_idx(v) {
                Some(iv) => Value::from((iv + base) as u64),
                None => v.clone(),
            }
        };
        let sh_row = |row: &Value| -> Value {
            match row {
                Value::Array(items) => Value::Array(items.iter().map(sh).collect()),
                other => other.clone(),
            }
        };

        if let Some(Value::Array(snap_terms)) = map_get(entries, "terms") {
            let shifted: Vec<Value> = snap_terms
                .iter()
                .map(|raw| match raw {
                    Value::Map(term_entries) => Value::Map(
                        term_entries
                            .iter()
                            .map(|(k, v)| {
                                if matches!(as_text(k), Some("dt" | "rf")) {
                                    (k.clone(), sh(v))
                                } else {
                                    (k.clone(), v.clone())
                                }
                            })
                            .collect(),
                    ),
                    other => other.clone(),
                })
                .collect();
            self.h_terms(&Value::Array(shifted), index);
        }
        if let Some(Value::Array(quads)) = map_get(entries, "quads") {
            self.h_quads(&Value::Array(quads.iter().map(sh_row).collect()), index);
        }
        match map_get(entries, "reifies") {
            Some(Value::Map(_)) => {
                self.diag(
                    "DamagedFrame",
                    "snapshot reifies payload must be a row array".to_string(),
                    Some(index),
                );
            }
            Some(Value::Array(reifies)) => {
                self.h_reifies(&Value::Array(reifies.iter().map(sh_row).collect()), index);
            }
            _ => {}
        }
        if let Some(Value::Array(annot)) = map_get(entries, "annot") {
            self.h_annot(&Value::Array(annot.iter().map(sh_row).collect()), index);
        }
        if let Some(Value::Map(blobs)) = map_get(entries, "blobs") {
            for (_, b) in blobs {
                if let Value::Bytes(bytes) = b {
                    let digest = digest_str(bytes);
                    if self.materialize {
                        self.g.set_blob(digest.clone(), bytes.clone());
                    }
                    self.emit_blob(&digest);
                }
            }
        }
        if let Some(Value::Map(meta)) = map_get(entries, "meta") {
            for (k, v) in meta {
                let key = as_text(k).map_or_else(|| format!("{k:?}"), str::to_string);
                self.g.set_meta(key, v.clone());
            }
        }
    }

    /// Record an intact `index` frame (§6.2) for the layout check (§3.3).
    ///
    /// The index stays an accelerator for the fold itself; only `count` and
    /// `head` are consumed here, as the covered-region boundary. A payload
    /// without a valid count/head pair is simply not an intact index.
    fn h_index(&mut self, payload: &Value, index: usize) {
        let Value::Map(entries) = payload else { return };
        let count = map_get(entries, "count").and_then(as_idx);
        let head = map_get(entries, "head");
        if let (Some(count), Some(Value::Bytes(head))) = (count, head) {
            let mmr = match map_get(entries, "mmr") {
                Some(Value::Bytes(root)) => Some(root.clone()),
                _ => None,
            };
            self.index_records.push(IndexRecord {
                abs_index: index,
                count,
                head: head.clone(),
                mmr,
            });
        }
    }

    fn h_opaque(&mut self, payload: &Value) {
        if let Value::Map(entries) = payload {
            let id = match map_get(entries, "id") {
                Some(Value::Bytes(b)) => b.clone(),
                _ => Vec::new(),
            };
            self.push_opaque(OpaqueNode {
                id,
                frame_type: text_or(map_get(entries, "type"), "opaque").to_string(),
                reason: text_or(map_get(entries, "reason"), "unknown-codec").to_string(),
                sigstat: text_or(map_get(entries, "sigstat"), "none").to_string(),
                pub_meta: map_get(entries, "pub").cloned(),
                recipients: None,
            });
        }
    }

    // -- helpers ---------------------------------------------------------------

    fn opaque(&mut self, frame: &[(Value, Value)], ftype: &str, reason: &str) {
        let id = match map_get(frame, "id") {
            Some(Value::Bytes(b)) => b.clone(),
            _ => Vec::new(),
        };
        let sigstat = if map_get(frame, "sig").is_some() {
            "unverified"
        } else {
            "none"
        };
        let recipients = match map_get(frame, "to") {
            Some(Value::Array(items)) => Some(
                items
                    .iter()
                    .filter(|t| matches!(t, Value::Map(_)))
                    .cloned()
                    .collect(),
            ),
            _ => None,
        };
        self.push_opaque(OpaqueNode {
            id,
            frame_type: ftype.to_string(),
            reason: reason.to_string(),
            sigstat: sigstat.to_string(),
            pub_meta: map_get(frame, "pub").cloned(),
            recipients,
        });
    }
}

/// §3.1 boundary rule: a map carrying `"gts"` and lacking `"t"`.
fn is_header_item(item: &Value) -> bool {
    let inner = match item {
        Value::Tag(_, inner) => inner.as_ref(),
        other => other,
    };
    if let Value::Map(entries) = inner {
        map_get(entries, "gts").is_some() && map_get(entries, "t").is_none()
    } else {
        false
    }
}

/// Parse the header `"dct"` map (§5): named, uncompressed in-band dictionary
/// bytes that a catalog codec's `"dct"` param references by name.
fn header_dict_table(header: &[(Value, Value)]) -> HashMap<&str, &[u8]> {
    let mut out = HashMap::new();
    if let Some(Value::Map(entries)) = map_get(header, "dct") {
        for (name, bytes) in entries {
            if let (Value::Text(name), Value::Bytes(bytes)) = (name, bytes) {
                out.insert(name.as_str(), bytes.as_slice());
            }
        }
    }
    out
}

/// Build the file-local codec catalog from the header `"cat"` map (§5, §8.5).
///
/// A codec entry that names a `"dct"` dictionary not present in the header
/// `"dct"` map is a hard error (§8.3, fail closed): that catalog id is
/// dropped from the map entirely, so any frame referencing it degrades to an
/// `unknown-codec` opaque node during codec resolution rather than silently
/// decoding without the dictionary (or against the wrong one).
fn catalog_from(header: &[(Value, Value)]) -> HashMap<i128, Codec> {
    let dict_table = header_dict_table(header);
    let mut out = HashMap::new();
    if let Some(Value::Map(raw)) = map_get(header, "cat") {
        for (cid, entry) in raw {
            if let (Some(cid), Value::Map(fields)) = (as_i128(cid), entry) {
                let dct = match map_get(fields, "dct") {
                    Some(Value::Text(name)) => match dict_table.get(name.as_str()) {
                        Some(bytes) => Some((*bytes).to_vec()),
                        // Fail closed: an unresolvable dictionary reference
                        // drops the whole catalog entry, not just the dct.
                        None => continue,
                    },
                    _ => None,
                };
                out.insert(
                    cid,
                    Codec {
                        name: text_or(map_get(fields, "name"), "").to_string(),
                        cls: text_or(map_get(fields, "cls"), "encode").to_string(),
                        dct,
                    },
                );
            }
        }
    }
    out
}

/// Read and fold a GTS file into a [`Graph`].
///
/// Verifies each segment's header genesis hash, every frame's self-`id`, and
/// the per-segment `prev` chain, recording diagnostics; damaged and
/// undecodable frames fold to opaque nodes (§7.6) rather than aborting.
/// Multi-segment files (§3.1) fold per segment and union BY TERM VALUE
/// (term-ids are segment-scoped; blank nodes stay segment-local).
///
/// With `allow_segments = false` the reader emulates a pre-§3.1 reader: a
/// segment boundary is a FATAL `SegmentBoundary` diagnostic and nothing past
/// it is folded (§16, vector 17). `expected_head`, when given, is compared
/// against the LAST segment's head; a mismatch records `TruncatedLog`.
///
/// # Examples
///
/// ```
/// use purrdf_gts::reader::read;
/// use purrdf_gts::writer::Writer;
///
/// let mut writer = Writer::new("purrdf.gts");
/// writer.add_blob(b"nine lives", Some("text/plain"), None);
///
/// let graph = read(&writer.into_bytes(), true, None);
/// assert!(graph.diagnostics.is_empty());
/// assert_eq!(graph.blobs.len(), 1);
///
/// // The reader is total: even empty input folds, recording a diagnostic
/// // instead of aborting.
/// let empty = read(&[], true, None);
/// assert_eq!(empty.diagnostics[0].code, "EmptyFile");
/// ```
pub fn read(data: &[u8], allow_segments: bool, expected_head: Option<&[u8]>) -> Graph {
    read_with_options(data, ReadOptions::new(allow_segments, expected_head))
}

/// Read and fold a GTS file using explicit options.
pub fn read_with_options(data: &[u8], options: ReadOptions<'_>) -> Graph {
    let (items, torn) = iter_items(data);
    if items.is_empty() {
        let mut g = Graph::default();
        g.diagnostics.push(Diagnostic {
            code: "EmptyFile".to_string(),
            detail: "no CBOR items".to_string(),
            frame_index: None,
        });
        return g;
    }

    // Split into segments at header-shaped items (§3.1).
    let bounds: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, (_, item))| is_header_item(item))
        .map(|(i, _)| i)
        .collect();
    if bounds.first() != Some(&0) {
        let mut g = Graph::default();
        g.diagnostics.push(Diagnostic {
            code: "DamagedFrame".to_string(),
            detail: "first item is not a header".to_string(),
            frame_index: Some(0),
        });
        return g;
    }
    if bounds.len() > 1 && !options.allow_segments {
        let mut g = read_segment_with_sink(&items[..bounds[1]], 0, 0, None, options.content_key);
        g.diagnostics.push(Diagnostic {
            code: "SegmentBoundary".to_string(),
            detail: format!(
                "segment boundary at item {} but reader is in pre-segment mode; \
                 remainder of file NOT folded (folding it with file-global \
                 term-ids would silently misfold — §16)",
                bounds[1]
            ),
            frame_index: Some(bounds[1]),
        });
        return g;
    }

    let ends = bounds.iter().skip(1).copied().chain([items.len()]);
    // Each segment owns its term-id namespace. Unioning happens after segment
    // folds by semantic term value, which avoids silently treating equal
    // numeric ids from different segments as equal terms.
    let ranges: Vec<(usize, usize)> = bounds.iter().copied().zip(ends).collect();
    let mut folded = fold_segments(&items, &ranges, options.content_key);

    let mut g = if folded.len() == 1 {
        folded.remove(0)
    } else {
        union_segments(&folded)
    };

    if let Some(expected) = options.expected_head {
        let last_head = g.segment_heads.last().cloned().unwrap_or_default();
        if last_head != expected {
            g.diagnostics.push(Diagnostic {
                code: "TruncatedLog".to_string(),
                detail: "observed head does not match expected head".to_string(),
                frame_index: None,
            });
        }
    }
    if let Some(offset) = torn {
        g.diagnostics.push(Diagnostic {
            code: "TornAppendError".to_string(),
            detail: format!("torn at offset {offset}"),
            frame_index: None,
        });
    }
    g
}

/// Read a GTS file into a [`StreamingSink`] without constructing the final
/// union [`Graph`].
///
/// The same header id, frame id, `prev` chain, payload, and layout checks used
/// by [`read`] are applied while each segment is consumed. Events use
/// segment-local term ids; the returned [`StreamingReadResult`] carries the
/// final diagnostics, segment heads, profiles, and streamable-layout state.
pub fn read_to_sink(
    data: &[u8],
    allow_segments: bool,
    expected_head: Option<&[u8]>,
    sink: &mut dyn StreamingSink,
) -> StreamingReadResult {
    read_to_sink_with_options(data, ReadOptions::new(allow_segments, expected_head), sink)
}

/// Read a GTS file into a [`StreamingSink`] using explicit options.
///
/// This is an evented evidence path, not a promise that memory is independent
/// of segment graph complexity: each segment is still folded enough to apply
/// the same diagnostics and layout checks as [`read_with_options`].
pub fn read_to_sink_with_options(
    data: &[u8],
    options: ReadOptions<'_>,
    sink: &mut dyn StreamingSink,
) -> StreamingReadResult {
    read_to_sink_from_reader(std::io::Cursor::new(data), options, sink)
}

struct StreamingCountingReader<R> {
    inner: R,
    pos: usize,
}

impl<R: Read> Read for StreamingCountingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;
        self.pos += read;
        Ok(read)
    }
}

fn next_stream_item<R: Read>(
    reader: &mut StreamingCountingReader<R>,
) -> Result<Option<Value>, usize> {
    let start = reader.pos;
    match ciborium::de::from_reader::<Value, _>(&mut *reader) {
        Ok(item) => Ok(Some(item)),
        Err(_) if reader.pos == start => Ok(None),
        Err(_) => Err(start),
    }
}

struct ActiveStreamingSegment {
    g: Graph,
    header: Vec<(Value, Value)>,
    expected_prev: Vec<u8>,
    frame_ids: Vec<Vec<u8>>,
    index_offset: usize,
    segment_index: usize,
    valid_header: bool,
    catalog: HashMap<i128, Codec>,
    index_records: Vec<IndexRecord>,
    described: HashSet<String>,
    blob_events: Vec<(usize, String, bool)>,
}

impl ActiveStreamingSegment {
    fn new(
        raw_header: &Value,
        index_offset: usize,
        segment_index: usize,
        sink: &mut dyn StreamingSink,
    ) -> Self {
        let mut g = Graph::default();
        let mut sink_slot = Some(sink);
        let mut header = Vec::new();
        let mut expected_prev = Vec::new();
        let mut valid_header = false;
        match unwrap_header(raw_header) {
            Ok(entries) => {
                header.clone_from(entries);
                valid_header = true;
                let stored_hid: Option<Vec<u8>> = match map_get(&header, "id") {
                    Some(Value::Bytes(b)) => Some(b.clone()),
                    _ => None,
                };
                if stored_hid.as_deref() != Some(&header_id(&header)[..]) {
                    push_diagnostic(
                        &mut g,
                        &mut sink_slot,
                        Diagnostic {
                            code: "DamagedFrame".to_string(),
                            detail: "header self-hash mismatch".to_string(),
                            frame_index: Some(index_offset),
                        },
                    );
                }
                if map_get(&header, "gts").and_then(as_text) != Some(MAGIC)
                    || map_get(&header, "v").and_then(as_i128) != Some(i128::from(VERSION))
                {
                    push_diagnostic(
                        &mut g,
                        &mut sink_slot,
                        Diagnostic {
                            code: "DamagedFrame".to_string(),
                            detail: format!(
                                "unsupported header magic/version {:?}/{:?}",
                                map_get(&header, "gts"),
                                map_get(&header, "v")
                            ),
                            frame_index: Some(index_offset),
                        },
                    );
                }
                expected_prev = stored_hid.unwrap_or_default();
            }
            Err(e) => {
                push_diagnostic(
                    &mut g,
                    &mut sink_slot,
                    Diagnostic {
                        code: "DamagedFrame".to_string(),
                        detail: format!("invalid header: {e}"),
                        frame_index: Some(index_offset),
                    },
                );
            }
        }
        let catalog = if valid_header {
            catalog_from(&header)
        } else {
            HashMap::new()
        };
        Self {
            g,
            header,
            expected_prev,
            frame_ids: Vec::new(),
            index_offset,
            segment_index,
            valid_header,
            catalog,
            index_records: Vec::new(),
            described: HashSet::new(),
            blob_events: Vec::new(),
        }
    }

    fn process_frame<'k>(
        &mut self,
        raw: &Value,
        abs_index: usize,
        sink: &mut dyn StreamingSink,
        content_key: Option<&'k ContentKeyResolver<'k>>,
    ) {
        if !self.valid_header {
            return;
        }
        let catalog = std::mem::take(&mut self.catalog);
        let index_records = std::mem::take(&mut self.index_records);
        let described = std::mem::take(&mut self.described);
        let blob_events = std::mem::take(&mut self.blob_events);
        let mut folder = Folder {
            g: &mut self.g,
            sink: Some(sink),
            content_key,
            segment_index: self.segment_index,
            materialize: false,
            catalog,
            index_records,
            described,
            blob_events,
        };
        if let Value::Map(frame) = raw {
            let stored_id: Option<&Vec<u8>> = match map_get(frame, "id") {
                Some(Value::Bytes(b)) => Some(b),
                _ => None,
            };
            let computed = content_id(frame);
            if stored_id.map(|b| &b[..]) != Some(&computed[..]) {
                folder.diag(
                    "DamagedFrame",
                    "frame self-hash mismatch".to_string(),
                    Some(abs_index),
                );
                let ftype = text_or(map_get(frame, "t"), "").to_string();
                folder.opaque(frame, &ftype, "damaged");
                self.expected_prev = stored_id.cloned().unwrap_or(computed);
                self.frame_ids.push(self.expected_prev.clone());
            } else {
                let prev_ok = matches!(map_get(frame, "prev"),
                    Some(Value::Bytes(b)) if *b == self.expected_prev);
                if !prev_ok {
                    folder.diag(
                        "BrokenChain",
                        "prev does not match".to_string(),
                        Some(abs_index),
                    );
                }
                self.expected_prev.clone_from(&computed);
                self.frame_ids.push(self.expected_prev.clone());
                if let Some(sig) = map_get(frame, "sig") {
                    let (status, cose) = match sig {
                        Value::Bytes(b) => ("unverified", Some(b.clone())),
                        _ => ("invalid", None),
                    };
                    folder.push_signature(Signature {
                        frame_id: computed,
                        kid: None,
                        status: status.to_string(),
                        cose,
                    });
                }
                folder.fold_frame(frame, abs_index);
            }
        } else {
            folder.diag(
                "DamagedFrame",
                "frame is not a map".to_string(),
                Some(abs_index),
            );
            self.frame_ids.push(Vec::new());
        }
        let catalog = std::mem::take(&mut folder.catalog);
        let index_records = std::mem::take(&mut folder.index_records);
        let described = std::mem::take(&mut folder.described);
        let blob_events = std::mem::take(&mut folder.blob_events);
        drop(folder);
        self.catalog = catalog;
        self.index_records = index_records;
        self.described = described;
        self.blob_events = blob_events;
    }

    fn finish_into_result(
        mut self,
        result: &mut StreamingReadResult,
        sink: &mut dyn StreamingSink,
    ) {
        if self.valid_header {
            self.g.segment_heads.push(self.expected_prev);
            if let Some(head) = self.g.segment_heads.last() {
                sink.segment_head(self.segment_index, head);
            }
            let seg_meta = self.g.meta.clone();
            self.g.segment_meta.push(seg_meta);
            self.g
                .segment_profiles
                .push(text_or(map_get(&self.header, "prof"), "generic").to_string());
            let mut sink_slot = Some(sink);
            check_index_mmr(
                &mut self.g,
                &self.index_records,
                &self.frame_ids,
                self.index_offset,
                &mut sink_slot,
            );
            let info = layout_check(
                &mut self.g,
                &self.header,
                &self.index_records,
                &self.blob_events,
                &self.frame_ids,
                self.index_offset,
                &mut sink_slot,
            );
            self.g.segment_streamable.push(info);
            if let Some(sink) = sink_slot
                && let Some(info) = self.g.segment_streamable.last()
            {
                sink.streamable_layout(self.segment_index, info);
            }
        }
        absorb_segment_result(result, &self.g);
    }
}

/// Read a GTS CBOR Sequence from a byte stream into a [`StreamingSink`].
///
/// This additive API consumes one CBOR item at a time and returns only reader
/// sidecar state. It keeps the segment-local term table and validation
/// sidecars needed for correct diagnostics, but it does not materialize the
/// final graph union, folded quads, suppressions, annotations, opaque rows,
/// signatures, or blob payloads.
pub fn read_to_sink_from_reader<R: Read>(
    reader: R,
    options: ReadOptions<'_>,
    sink: &mut dyn StreamingSink,
) -> StreamingReadResult {
    let mut reader = StreamingCountingReader {
        inner: reader,
        pos: 0,
    };
    let mut result = StreamingReadResult::default();
    let mut item_index = 0usize;
    let mut current: Option<ActiveStreamingSegment> = None;
    loop {
        let item = match next_stream_item(&mut reader) {
            Ok(Some(item)) => item,
            Ok(None) => break,
            Err(torn) => {
                result.torn = Some(torn);
                break;
            }
        };
        if is_header_item(&item) {
            if let Some(segment) = current.take() {
                if !options.allow_segments {
                    segment.finish_into_result(&mut result, sink);
                    push_result_diagnostic(
                        &mut result,
                        sink,
                        Diagnostic {
                            code: "SegmentBoundary".to_string(),
                            detail: format!(
                                "segment boundary at item {item_index} but reader is in \
                                 pre-segment mode; remainder of file NOT folded (folding \
                                 it with file-global term-ids would silently misfold — §16)"
                            ),
                            frame_index: Some(item_index),
                        },
                    );
                    return result;
                }
                segment.finish_into_result(&mut result, sink);
            } else if item_index != 0 {
                push_result_diagnostic(
                    &mut result,
                    sink,
                    Diagnostic {
                        code: "DamagedFrame".to_string(),
                        detail: "first item is not a header".to_string(),
                        frame_index: Some(0),
                    },
                );
                return result;
            }
            current = Some(ActiveStreamingSegment::new(
                &item,
                item_index,
                result.segment_heads.len(),
                sink,
            ));
        } else if let Some(segment) = current.as_mut() {
            segment.process_frame(&item, item_index, sink, options.content_key);
        } else {
            push_result_diagnostic(
                &mut result,
                sink,
                Diagnostic {
                    code: "DamagedFrame".to_string(),
                    detail: "first item is not a header".to_string(),
                    frame_index: Some(0),
                },
            );
            return result;
        }
        item_index += 1;
    }

    if item_index == 0 {
        push_result_diagnostic(
            &mut result,
            sink,
            Diagnostic {
                code: "EmptyFile".to_string(),
                detail: "no CBOR items".to_string(),
                frame_index: None,
            },
        );
        return result;
    }
    if let Some(segment) = current {
        segment.finish_into_result(&mut result, sink);
    }

    if let Some(expected) = options.expected_head {
        let last_head = result.segment_heads.last().cloned().unwrap_or_default();
        if last_head != expected {
            push_result_diagnostic(
                &mut result,
                sink,
                Diagnostic {
                    code: "TruncatedLog".to_string(),
                    detail: "observed head does not match expected head".to_string(),
                    frame_index: None,
                },
            );
        }
    }
    if let Some(offset) = result.torn {
        push_result_diagnostic(
            &mut result,
            sink,
            Diagnostic {
                code: "TornAppendError".to_string(),
                detail: format!("torn at offset {offset}"),
                frame_index: None,
            },
        );
    }
    result
}

/// The per-segment view of a file — the input to composition tooling (§14.1):
/// each segment folded independently, plus the file-level torn marker and any
/// fatal pre-segmentation diagnostic.
#[derive(Debug)]
pub struct FileSegments {
    /// One fold per segment, in file order, each carrying its OWN diagnostics.
    pub segments: Vec<Graph>,
    /// Byte offset of a torn trailing item (§3), if any.
    pub torn: Option<usize>,
    /// Set when the file never reaches segmentation (empty, or the first item
    /// is not a header) — `segments` is empty in that case.
    pub fatal: Option<Diagnostic>,
}

/// Fold a file segment-by-segment WITHOUT unioning — the composition ledger
/// view that `gts info`/`gts verify` report per-segment (§14.1).
pub fn read_file_segments(data: &[u8]) -> FileSegments {
    let (items, torn) = iter_items(data);
    if items.is_empty() {
        return FileSegments {
            segments: Vec::new(),
            torn,
            fatal: Some(Diagnostic {
                code: "EmptyFile".to_string(),
                detail: "no CBOR items".to_string(),
                frame_index: None,
            }),
        };
    }
    let bounds: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, (_, item))| is_header_item(item))
        .map(|(i, _)| i)
        .collect();
    if bounds.first() != Some(&0) {
        return FileSegments {
            segments: Vec::new(),
            torn,
            fatal: Some(Diagnostic {
                code: "DamagedFrame".to_string(),
                detail: "first item is not a header".to_string(),
                frame_index: Some(0),
            }),
        };
    }
    let ends = bounds.iter().skip(1).copied().chain([items.len()]);
    let ranges: Vec<(usize, usize)> = bounds.iter().copied().zip(ends).collect();
    let segments = fold_segments(&items, &ranges, None);
    FileSegments {
        segments,
        torn,
        fatal: None,
    }
}

/// Fold each `(start, end)` segment range independently, in file order.
///
/// Segments have independent integrity (§3.1): each carries its own genesis,
/// id/prev chain, signatures, and index, so cross-segment verification is
/// embarrassingly parallel. Folds run on rayon when no content-key resolver is
/// supplied (the resolver closure is not required to be thread-safe); results
/// are collected by segment position, so the folded graphs — and every
/// diagnostic they carry — are identical to a sequential fold regardless of
/// scheduling. On targets without threads rayon executes inline.
fn fold_segments(
    items: &[(usize, Value)],
    ranges: &[(usize, usize)],
    content_key: Option<&ContentKeyResolver<'_>>,
) -> Vec<Graph> {
    if content_key.is_none() && ranges.len() > 1 {
        use rayon::prelude::*;
        ranges
            .par_iter()
            .map(|&(a, b)| read_segment_with_sink(&items[a..b], a, 0, None, None))
            .collect()
    } else {
        ranges
            .iter()
            .map(|&(a, b)| read_segment_with_sink(&items[a..b], a, 0, None, content_key))
            .collect()
    }
}

/// Recompute every frame's BLAKE3 content id concurrently (§9.1).
///
/// Each frame's `"id"` hashes a self-contained byte range, so all frame
/// hashes are recomputed in parallel and the fold that follows reduces to a
/// trivial sequential `"prev"`-equality pass — no accumulating dependency
/// forces single-threaded verification. Ids are collected by frame position
/// (never thread completion order), keeping every downstream diagnostic in
/// exact frame order. Non-map items yield `None`; the fold reports their
/// diagnostics itself.
fn parallel_content_ids(items: &[(usize, Value)]) -> Vec<Option<Vec<u8>>> {
    use rayon::prelude::*;
    items
        .par_iter()
        .map(|(_, raw)| match raw {
            Value::Map(frame) => Some(content_id(frame)),
            _ => None,
        })
        .collect()
}

fn read_segment_with_sink(
    items: &[(usize, Value)],
    index_offset: usize,
    segment_index: usize,
    mut sink: Option<&mut dyn StreamingSink>,
    content_key: Option<&ContentKeyResolver<'_>>,
) -> Graph {
    let mut g = Graph::default();
    let (_, raw_header) = &items[0];
    let header = match unwrap_header(raw_header) {
        Ok(h) => h,
        Err(e) => {
            push_diagnostic(
                &mut g,
                &mut sink,
                Diagnostic {
                    code: "DamagedFrame".to_string(),
                    detail: format!("invalid header: {e}"),
                    frame_index: Some(index_offset),
                },
            );
            return g;
        }
    };
    let stored_hid: Option<Vec<u8>> = match map_get(header, "id") {
        Some(Value::Bytes(b)) => Some(b.clone()),
        _ => None,
    };
    if stored_hid.as_deref() != Some(&header_id(header)[..]) {
        push_diagnostic(
            &mut g,
            &mut sink,
            Diagnostic {
                code: "DamagedFrame".to_string(),
                detail: "header self-hash mismatch".to_string(),
                frame_index: Some(index_offset),
            },
        );
    }
    if map_get(header, "gts").and_then(as_text) != Some(MAGIC)
        || map_get(header, "v").and_then(as_i128) != Some(i128::from(VERSION))
    {
        push_diagnostic(
            &mut g,
            &mut sink,
            Diagnostic {
                code: "DamagedFrame".to_string(),
                detail: format!(
                    "unsupported header magic/version {:?}/{:?}",
                    map_get(header, "gts"),
                    map_get(header, "v")
                ),
                frame_index: Some(index_offset),
            },
        );
    }
    let mut expected_prev: Vec<u8> = stored_hid.unwrap_or_default();
    // per-frame chain ids, by 0-based frame position
    let mut frame_ids: Vec<Vec<u8>> = Vec::new();

    // §9.1 parallel verification: hash every frame's content id up front,
    // concurrently; the fold below then walks the chain sequentially with a
    // cheap `prev`-equality check per frame.
    let mut computed_ids = parallel_content_ids(&items[1..]);

    let (index_records, blob_events, restored_sink) = {
        let catalog = catalog_from(header);
        let mut folder = Folder {
            g: &mut g,
            sink: sink.take(),
            content_key,
            segment_index,
            materialize: true,
            catalog,
            index_records: Vec::new(),
            described: HashSet::new(),
            blob_events: Vec::new(),
        };
        for (index, (_, raw)) in items[1..].iter().enumerate() {
            let abs_index = index + 1 + index_offset;
            let Value::Map(frame) = raw else {
                folder.diag(
                    "DamagedFrame",
                    "frame is not a map".to_string(),
                    Some(abs_index),
                );
                frame_ids.push(Vec::new());
                continue;
            };
            let stored_id: Option<&Vec<u8>> = match map_get(frame, "id") {
                Some(Value::Bytes(b)) => Some(b),
                _ => None,
            };
            let computed = computed_ids[index]
                .take()
                .unwrap_or_else(|| content_id(frame));
            if stored_id.map(|b| &b[..]) != Some(&computed[..]) {
                folder.diag(
                    "DamagedFrame",
                    "frame self-hash mismatch".to_string(),
                    Some(abs_index),
                );
                let ftype = text_or(map_get(frame, "t"), "").to_string();
                folder.opaque(frame, &ftype, "damaged");
                expected_prev = stored_id.cloned().unwrap_or(computed);
                frame_ids.push(expected_prev.clone());
                continue;
            }
            let prev_ok = matches!(map_get(frame, "prev"),
                Some(Value::Bytes(b)) if *b == expected_prev);
            if !prev_ok {
                folder.diag(
                    "BrokenChain",
                    "prev does not match".to_string(),
                    Some(abs_index),
                );
            }
            expected_prev.clone_from(&computed);
            frame_ids.push(expected_prev.clone());
            if let Some(sig) = map_get(frame, "sig") {
                // No key provider in this baseline — a well-formed signature
                // is recorded as "unverified" with its raw COSE bytes retained
                // (compaction carries it detached, §10.1); a malformed one is
                // recorded as "invalid", never silently dropped.
                let (status, cose) = match sig {
                    Value::Bytes(b) => ("unverified", Some(b.clone())),
                    _ => ("invalid", None),
                };
                folder.push_signature(Signature {
                    frame_id: computed.clone(),
                    kid: None,
                    status: status.to_string(),
                    cose,
                });
            }
            folder.fold_frame(frame, abs_index);
        }
        (folder.index_records, folder.blob_events, folder.sink)
    };
    sink = restored_sink;

    if let Some(sink) = sink.as_deref_mut() {
        sink.segment_head(segment_index, &expected_prev);
    }
    g.segment_heads.push(expected_prev);
    let seg_meta = g.meta.clone();
    g.segment_meta.push(seg_meta);
    g.segment_profiles
        .push(text_or(map_get(header, "prof"), "generic").to_string());
    check_index_mmr(&mut g, &index_records, &frame_ids, index_offset, &mut sink);
    let info = layout_check(
        &mut g,
        header,
        &index_records,
        &blob_events,
        &frame_ids,
        index_offset,
        &mut sink,
    );
    if let Some(sink) = sink {
        sink.streamable_layout(segment_index, &info);
    }
    g.segment_streamable.push(info);
    g
}
