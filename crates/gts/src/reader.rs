// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The GTS reader: parse a CBOR Sequence, verify the id/prev chain, fold the
//! log — mirror of `src/purrdf_tools/gts/reader.py`.
//!
//! Implements the Baseline Reader contract (§2.1): chain verification (§9.1),
//! the value-union fold (§7.5), opaque/damaged degradation (§7.6), torn-append
//! detection (§3), and the canonical diagnostics (§2.3). The default
//! [`read`] path carries no content keys: `sig` frames record as
//! `"unverified"` and `encrypt`-class frames degrade to `missing-key` opaque
//! nodes. Callers that hold content keys can use [`read_with_options`].

use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::{FastMap, FastSet};
use std::io::Read;
use std::sync::Arc;

use crate::term_table::TermsRef;

mod fold_error;
use fold_error::FoldError;

mod append_error;
pub use append_error::AppendError;

mod payload_error;
use payload_error::{PayloadCause, PayloadError};

mod reifier_index;
use reifier_index::ReifierIndex;

use purrdf_lex::cbor::{self, BorrowedItem, EncodedItem, Value, ValueRef};
mod encoded_frame;
use encoded_frame::{Payload, StreamItem};

use crate::codec::{
    Codec, CodecError, decode_chain, decode_chain_bounded, decode_chain_with_decrypt_bounded,
};
use crate::model::{
    AnnotationRow, BlobEntry, ByteRange, Diagnostic, Graph, OpaqueNode, Quad, ReifierRow,
    Signature, StreamableInfo, Suppression, Term, TermKind, language_tag_refusal,
};
use crate::reader_index::DigestIndex;
use crate::reader_layout::{IndexRecord, check_index_mmr, layout_check};
use crate::reader_rows::{
    RowDecode, RowError, TermIdBase, check_quad_positions, decode_annotation_row,
    decode_reifier_row, numeric_row,
};
use crate::reader_union::union_segments;
use crate::stream::DIGEST as STREAM_DIGEST;
use crate::wire::{
    MAGIC, VERSION, content_id, digest_label, digest_str, header_id, map_get, unwrap_header,
};
use crate::wire::{TornAppendError, iter_items_with_error};

pub(crate) fn as_i128(v: &Value) -> Option<i128> {
    v.as_integer().map(i128::from)
}

/// Coerce a value to a non-negative index, else `None` (Python `_as_int`).
pub(crate) fn as_idx(v: &Value) -> Option<usize> {
    as_i128(v).and_then(|n| usize::try_from(n).ok())
}

pub(crate) fn text_or<'a>(v: Option<&'a Value>, default: &'a str) -> &'a str {
    v.and_then(Value::as_text).unwrap_or(default)
}

/// Borrow raw blob bytes and move decoded transform output without copying.
fn payload_bytes(value: Payload<'_>) -> Result<Option<Cow<'_, [u8]>>, PayloadError> {
    match value {
        Payload::Value(value) => value
            .map(ValueRef::bytes)
            .transpose()
            .map(Option::flatten)
            .map_err(|cause| PayloadError::Damaged(PayloadCause::Cbor(cause))),
        Payload::Blob(bytes) => Ok(Some(bytes)),
        Payload::Encoded(_) => unreachable!("blob decoding returns bytes, never an RDF item"),
    }
}

fn diag_code_for(reason: &str) -> &'static str {
    match reason {
        "missing-key" => "MissingKey",
        _ => "UnknownCodec",
    }
}

/// Resolve public blob metadata's digest using the reader's identity convention.
///
/// Accepts 64 lowercase hexadecimal digits with or without the `blake3:` prefix,
/// or a 32-byte digest value.
/// The original metadata is borrowed and unchanged. This only resolves the
/// declaration; callers authenticating payloads must compare the result with the
/// decoded content digest and reject ambiguous metadata keys separately.
#[must_use]
pub fn public_blob_digest(value: &Value) -> Option<String> {
    let Value::Map(entries) = value else {
        return None;
    };
    match map_get(entries, "digest") {
        Some(Value::Text(text)) => {
            let hex = text.strip_prefix("blake3:").unwrap_or(text);
            purrdf_hash::hex::decode_32_canonical(hex).map(|digest| digest_label(&digest))
        }
        Some(Value::Bytes(bytes)) if bytes.len() == 32 => Some(digest_label(bytes)),
        _ => None,
    }
}

/// Diagnostic code for a payload refused by a caller-supplied byte ceiling.
///
/// Distinct from `DamagedFrame` so a consumer can tell "I declined to spend the
/// bytes" from "the container is corrupt" without parsing the detail string.
/// Only a sink that supplies a blob byte limit can provoke it.
pub const BLOB_BUDGET_DIAGNOSTIC: &str = "BlobBudget";

/// Diagnostic code for a whole non-blob frame refused by a caller-supplied
/// byte ceiling.
///
/// Deliberately distinct from [`BLOB_BUDGET_DIAGNOSTIC`]: declining one blob
/// payload leaves the RDF dataset intact, whereas dropping a frame removes rows
/// the caller was relying on. A consumer that treats the first as recoverable
/// must not silently treat the second the same way.
pub const FRAME_BUDGET_DIAGNOSTIC: &str = "FrameBudget";

/// Identity for a payload a ceiling refused, and whether this reader proved it.
///
/// With no transform chain the wire bytes are the decoded bytes, so hashing them
/// is exact and costs no memory — that identity is *proved*. Otherwise the only
/// candidate is the container's own `pub.digest`, which cannot be checked
/// against a payload that was never decoded. The distinction matters downstream:
/// acting on an unverified claim lets a container assert a refused payload's
/// identity, and so decide how a selector resolves.
fn refused_identity(
    chain: &[Codec],
    d: Option<&Value>,
    pub_meta: Option<&Value>,
) -> (Option<String>, bool) {
    if chain.is_empty()
        && let Some(raw) = d.and_then(Value::as_bytes)
    {
        return (Some(digest_str(raw)), true);
    }
    (pub_meta.and_then(public_blob_digest), false)
}

fn decrypt_codec(
    codec: &Codec,
    data: &[u8],
    content_key: &ContentKeyResolver<'_>,
    limit: usize,
) -> Result<Vec<u8>, CodecError> {
    if codec.name != "cose-encrypt0" {
        return Err(CodecError::Unavailable {
            reason: "missing-key",
            detail: crate::codec::CodecUnavailable::NoDecryptor(codec.name.clone()),
        });
    }
    crate::cose::decrypt0_bounded(data, content_key, limit).map_err(|err| match err {
        crate::cose::BoundedDecrypt0Error::Crypto(err) => CodecError::Unavailable {
            reason: "missing-key",
            detail: crate::codec::CodecUnavailable::Decrypt {
                codec: codec.name.clone(),
                cause: err,
            },
        },
        crate::cose::BoundedDecrypt0Error::Limit => CodecError::Limit { limit },
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
    pub torn: Option<u64>,
}

/// Physical provenance of one streamed frame: where its bytes live and what it
/// hashes to.
///
/// Fired once per frame by [`read_to_sink_from_reader`] (and its callers)
/// before that frame's rows are folded, so index builders can record byte
/// offsets without re-parsing the source. The field semantics mirror
/// `replication::FrameInventory`, the offline equivalent computed from a
/// fully-buffered file.
#[derive(Clone, Debug)]
pub struct FrameContext<'a> {
    /// Zero-based segment index.
    pub segment_index: u64,
    /// Zero-based frame index within the segment (excludes the header).
    pub frame_index: u64,
    /// Frame content-id: the stored `"id"` when present and self-hash-valid,
    /// else the recomputed id; empty for a non-map placeholder frame.
    pub content_id: &'a [u8],
    /// `[start, end)` byte range of this frame in the source.
    pub range: ByteRange,
    /// Wire `"t"` value; `"<non-map>"` for a non-map placeholder frame.
    pub frame_type: &'a str,
    /// True when both the id self-hash and the `prev`-chain link hold.
    pub valid: bool,
}

impl FrameContext<'_> {
    /// Recompute the content-id over `source[range]` and compare it to
    /// [`Self::content_id`].
    ///
    /// Returns `false` when the range is out of bounds, the bytes fail to
    /// decode as CBOR, or the decoded value is not a map — a damaged frame
    /// never verifies.
    pub fn verify(&self, source: &[u8]) -> bool {
        let (Ok(start), Ok(end)) = (
            usize::try_from(self.range.start),
            usize::try_from(self.range.end),
        ) else {
            return false;
        };
        let Some(bytes) = source.get(start..end) else {
            return false;
        };
        let Ok((item, used)) = cbor::borrowed_prefix(bytes, cbor::Limits::DEFAULT) else {
            return false;
        };
        if used != bytes.len() {
            return false;
        }
        crate::wire::content_id_encoded(item).is_ok_and(|id| id == self.content_id)
    }
}

/// Borrowed payload and public metadata for one accepted inline blob occurrence.
///
/// This event carries no RDF term ids. Its segment index and the preceding
/// [`StreamingSink::frame`] event identify its exact physical source. A later
/// occurrence may replace a digest's public metadata; absent metadata preserves
/// earlier metadata under the ordinary cross-segment union contract.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct BlobPayload<'a> {
    /// Zero-based segment containing this occurrence.
    pub segment_index: u64,
    /// Declared content digest, or the computed digest when not declared.
    pub digest: &'a str,
    /// Public metadata available at this occurrence, independent of RDF rows.
    pub metadata: Option<&'a Value>,
    /// True only when this occurrence explicitly declares public metadata.
    /// False distinguishes inherited segment metadata from a new declaration.
    pub metadata_declared: bool,
    /// Decoded bytes for an empty codec chain, otherwise transformed wire bytes.
    /// `None` exposes a metadata-only occurrence without inventing a payload.
    pub bytes: Option<&'a [u8]>,
    /// Whether the occurrence contains a payload field at all. A present field
    /// with `bytes == None` is malformed, distinct from a metadata-only update.
    pub payload_present: bool,
    /// Original blob byte-string length before any eager decoding. For a snapshot
    /// entry this is the embedded blob length, excluding its enclosing RDF frame.
    /// Metadata-only occurrences have length zero.
    pub encoded_len: usize,
    /// Resolved transform chain for `bytes`; apply in reverse to decode.
    pub codecs: &'a [Codec],
}

/// Typed reason a reader could not provide one inline blob occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlobRefusalKind {
    /// A caller-supplied payload byte ceiling.
    ByteBudget,
    /// A recognized codec/key capability is unavailable.
    UnavailableCapability,
}

/// Recover the original typed refusal from a native payload diagnostic.
/// Storage failures and diagnostics without their original cause return `None`.
#[must_use]
pub fn blob_refusal_kind(diagnostic: &Diagnostic) -> Option<BlobRefusalKind> {
    let crate::model::DiagnosticCause::Native(cause) = diagnostic.cause.as_ref()? else {
        return None;
    };
    let error: &(dyn std::error::Error + 'static) = &***cause;
    error.downcast_ref::<PayloadError>()?.blob_refusal_kind()
}

/// One inline blob occurrence refused by a byte ceiling or missing capability.
///
/// Reported so a selecting consumer still learns a candidate was present. It
/// carries everything decidable without spending the bytes: the container's
/// declared public metadata (so a representation selector can still match), the
/// physical source, and the encoded length that was rejected.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub struct BlobRefusal<'a> {
    /// Original typed admission reason.
    pub kind: BlobRefusalKind,
    /// Zero-based segment containing this occurrence.
    pub segment_index: u64,
    /// Declared content digest, when the container published one.
    ///
    /// `None` for a payload with no public digest: refusing happens before the
    /// decode that would compute it, so no identity is available. Such a blob is
    /// matchable by representation and provenance, never by digest.
    pub digest: Option<&'a str>,
    /// Public metadata available at this occurrence, independent of RDF rows.
    pub metadata: Option<&'a Value>,
    /// Whether this reader computed [`Self::digest`] from the payload itself.
    ///
    /// False means the value is the container's unverified claim, which no
    /// consumer should treat as identity: the payload was refused before any
    /// decode, so nothing checked it.
    pub digest_computed: bool,
    /// Original blob byte-string length that was refused.
    pub encoded_len: usize,
    /// Which ceiling fired, in the reader's own words.
    pub detail: &'a str,
}

/// Sink for [`read_to_sink`] events.
///
/// Term ids in events are segment-local ids. A sink that needs a file-level
/// union should intern by value using the same rules as [`read`]; sinks that
/// project rows into a table can usually consume the segment-local ids directly
/// with the segment index as the scope.
pub trait StreamingSink {
    /// Per-frame provenance, fired once per frame before its rows are processed.
    fn frame(&mut self, _ctx: FrameContext<'_>) {}
    /// Accepted term row, shared with the reader's segment validation table.
    /// The reader releases its owner after segment validation; a sink may keep
    /// this handle without copying the term's strings.
    fn term(&mut self, _segment_index: u64, _term_id: usize, _term: Arc<Term>) {}
    /// Accepted quad row.
    fn quad(&mut self, _segment_index: u64, _quad: Quad) {}
    /// Accepted reifier row.
    fn reifier(&mut self, _segment_index: u64, _reifier: ReifierRow) {}
    /// Accepted annotation row.
    fn annotation(&mut self, _segment_index: u64, _annotation: AnnotationRow) {}
    /// Accepted suppression directive.
    fn suppression(&mut self, _segment_index: u64, _suppression: &Suppression) {}
    /// Accepted inline blob digest and declared metadata.
    fn blob(&mut self, _segment_index: u64, _digest: &str, _meta: Option<&Value>) {}
    /// Borrowed blob bytes with their public metadata and codec chain.
    ///
    /// The default is inert: existing sinks neither copy nor decode payloads.
    /// Called after the legacy metadata-only [`Self::blob`] event. The metadata
    /// is repeated here so consumers need not pair callbacks by arrival order.
    fn blob_payload(&mut self, _payload: BlobPayload<'_>) {}
    /// A blob this sink's own byte ceiling refused, before it was decoded.
    ///
    /// Fires instead of [`Self::blob_payload`], and only for a sink that set a
    /// ceiling. A refused payload is reported rather than dropped so a selecting
    /// consumer can still see that a candidate existed: silently omitting it
    /// would let a ceiling change which blob a selector resolves to, turning a
    /// retention bound into a selection rule. The digest is absent whenever the
    /// container did not declare one, because refusing precedes hashing.
    fn blob_refused(&mut self, _refusal: BlobRefusal<'_>) {}
    /// Optional encoded-byte bound before eager blob decoding, including decryption.
    ///
    /// Existing sinks leave the reader behavior unchanged. This does not bound
    /// ordinary RDF frame decoding or reader frame buffers.
    fn blob_encoded_limit(&self) -> Option<usize> {
        None
    }
    /// Optional bound for each intermediate and final eager blob decode output.
    ///
    /// Existing sinks leave the reader behavior unchanged. Bounded collectors
    /// apply their per-blob limit before decoding without a public digest or
    /// decrypting with a content key. This does not bound ordinary RDF frame
    /// decoding, encoded COSE parsing buffers, or codec dictionary storage.
    fn blob_decode_limit(&self) -> Option<usize> {
        None
    }
    /// Optional bound for decoding an ordinary (non-blob) frame's payload.
    ///
    /// A `snapshot` frame carries embedded blob bytes *inside* an RDF frame, so
    /// a blob ceiling alone leaves the real bomb unbounded: the memory is spent
    /// decoding the enclosing frame, long before any embedded entry is seen.
    /// Sinks that set no bound keep the previous unbounded behavior.
    fn frame_decode_limit(&self) -> Option<usize> {
        None
    }
    /// Opaque frame produced by unknown, encrypted, or damaged payloads.
    fn opaque(&mut self, _segment_index: u64, _opaque: &OpaqueNode) {}
    /// Signature status observed on a frame.
    fn signature(&mut self, _segment_index: u64, _signature: &Signature) {}
    /// Reader diagnostic.
    fn diagnostic(&mut self, _diagnostic: &Diagnostic) {}
    /// Completed segment head.
    fn segment_head(&mut self, _segment_index: u64, _head: &[u8]) {}
    /// Completed segment streamable-layout state.
    fn streamable_layout(&mut self, _segment_index: u64, _info: &StreamableInfo) {}
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
    // Present only for streaming folds; materialized folds write Graph::terms.
    shared_terms: Option<&'g mut Vec<Arc<Term>>>,
    sink: Option<&'s mut dyn StreamingSink>,
    content_key: Option<&'k ContentKeyResolver<'k>>,
    segment_index: u64,
    materialize: bool,
    // Only streamable segments can use these facts to qualify packaging.
    // Observe every quad in those segments, including facts that poison a shape.
    streamable: bool,
    provenance: crate::compact::ProvenanceSubjects,
    catalog: FastMap<i128, Codec>,
    blob_index: DigestIndex,
    blob_meta_index: DigestIndex,
    // Layout-state bookkeeping (§3.3): intact index frames seen, digests the
    // graph has described via stream:digest so far, and each inline blob's
    // arrival (frame index, digest, was-it-described-at-arrival).
    index_records: Vec<IndexRecord>,
    described: FastSet<String>,
    blob_events: Vec<(u64, String, bool)>,
    // Reifier ids seen bound to more than one triple, with the frame index of
    // the rebinding row. Legitimate on its own (§7.1 `tt`); only ambiguous for a
    // legacy `rf`-only quoted-triple term, which is checked once the segment is
    // fully folded (see `report_ambiguous_reifiers`).
    rebound_reifiers: Vec<(usize, u64)>,
    // Anchors and first bindings by reifier id, plus distinct rows only when
    // materializing `g.reifiers`, so a `reifies` row never scans either table.
    reifier_index: ReifierIndex,
}

impl Folder<'_, '_, '_> {
    fn terms(&self) -> TermsRef<'_> {
        match self.shared_terms.as_deref() {
            Some(terms) => TermsRef::Shared(terms),
            None => TermsRef::Owned(&self.g.terms),
        }
    }

    /// Report the ONE shape a multi-bound reifier id can still break: a
    /// quoted-triple term that carries no `tt` of its own and resolves its
    /// components through that id, so the file asks for a single term with two
    /// different meanings. The bindings themselves are kept (nothing is
    /// dropped); the term resolves through the FIRST binding, and the
    /// diagnostic makes the ambiguity loud rather than silent.
    ///
    /// Run after the whole segment is folded so it does not depend on whether
    /// the `terms` frame happened to precede the `reifies` frame.
    fn report_ambiguous_reifiers(&mut self) {
        let ambiguous: Vec<(usize, u64)> = std::mem::take(&mut self.rebound_reifiers)
            .into_iter()
            .filter(|&(rid, _)| {
                let terms = self.terms();
                self.reifier_index.anchors(rid).iter().any(|&tid| {
                    let term = &terms[tid];
                    term.triple.is_none() && term.reifier == Some(rid)
                })
            })
            .collect();
        for (rid, index) in ambiguous {
            self.diag(FoldError::ConflictingReifier { reifier: rid }, index);
        }
    }

    fn with_sink(&mut self, f: impl FnOnce(u64, &mut dyn StreamingSink)) {
        if let Some(sink) = self.sink.as_deref_mut() {
            f(self.segment_index, sink);
        }
    }

    fn diag(&mut self, error: FoldError, index: u64) {
        let detail = error.to_string();
        self.diag_native(error.diagnostic_code(), error, detail, index);
    }

    fn diag_native(
        &mut self,
        code: &'static str,
        error: impl purrdf_hash::error::NativeError + 'static,
        detail: String,
        index: u64,
    ) {
        let diagnostic = Diagnostic {
            code: code.into(),
            detail,
            frame_index: Some(index),
            cause: None,
        }
        .with_native_cause(error);
        push_diagnostic(self.g, &mut self.sink, diagnostic);
    }

    /// Report a payload the sink's own ceiling refused, before any decode.
    fn emit_blob_refusal(
        &mut self,
        kind: BlobRefusalKind,
        digest: Option<&str>,
        digest_computed: bool,
        declared_metadata: Option<&Value>,
        encoded_len: usize,
        detail: &str,
    ) {
        let segment_index = self.segment_index;
        let Some(sink) = self.sink.as_deref_mut() else {
            return;
        };
        sink.blob_refused(BlobRefusal {
            kind,
            segment_index,
            digest,
            digest_computed,
            metadata: declared_metadata,
            encoded_len,
            detail,
        });
    }

    fn emit_blob(
        &mut self,
        digest: &str,
        bytes: Option<&[u8]>,
        codecs: &[Codec],
        declared_metadata: Option<&Value>,
        payload_present: bool,
        encoded_len: usize,
    ) {
        // Only a sink can observe the meta, so look it up only when one is
        // present — and borrow it (`self.g` shared, `self.sink` mut are
        // disjoint fields) instead of deep-cloning the CBOR map per blob.
        let Some(sink) = self.sink.as_deref_mut() else {
            return;
        };
        let meta = self.blob_meta_index.get(&self.g.blob_meta, digest);
        sink.blob(self.segment_index, digest, meta);
        sink.blob_payload(BlobPayload {
            segment_index: self.segment_index,
            digest,
            metadata: declared_metadata.or(meta),
            metadata_declared: declared_metadata.is_some(),
            bytes,
            payload_present,
            encoded_len,
            codecs,
        });
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
                        detail: PayloadCause::MissingCatalog(cid.clone()),
                    });
                }
            }
        }
        Ok(chain)
    }

    /// Resolve a frame's logical payload (§6.1); error on missing capability.
    fn payload<'frame>(
        &self,
        frame: &'frame [(Value, Value)],
        blob: bool,
        data: Option<ValueRef<'frame>>,
    ) -> Result<Payload<'frame>, PayloadError> {
        let d = data;
        let bytes = d
            .map(ValueRef::bytes)
            .transpose()
            .map(Option::flatten)
            .map_err(|cause| PayloadError::Damaged(PayloadCause::Cbor(cause)))?;
        if blob
            && let Some(bytes) = bytes.as_deref()
            && let Some(limit) = self
                .sink
                .as_deref()
                .and_then(StreamingSink::blob_encoded_limit)
            && bytes.len() > limit
        {
            return Err(PayloadError::Budget(PayloadCause::EncodedBlobLimit {
                actual: bytes.len(),
                limit,
            }));
        }
        if let Some(Value::Array(ids)) = map_get(frame, "x")
            && !ids.is_empty()
        {
            let Some(db) = bytes.as_deref() else {
                return Err(PayloadError::Damaged(PayloadCause::TransformedNotBytes));
            };
            let chain = self.resolve_codecs(ids)?;
            // A blob frame is bounded by the blob ceiling; every other frame by
            // the frame ceiling, which is what keeps an embedded snapshot blob
            // map from decoding without limit.
            let limit = self.sink.as_deref().and_then(|sink| {
                if blob {
                    sink.blob_decode_limit()
                } else {
                    sink.frame_decode_limit()
                }
            });
            let decoded = if let Some(content_key) = self.content_key {
                let limit = limit.unwrap_or(usize::MAX);
                let decrypt =
                    |codec: &Codec, data: &[u8]| decrypt_codec(codec, data, content_key, limit);
                decode_chain_with_decrypt_bounded(&chain, db, Some(&decrypt), limit)?
            } else if let Some(limit) = limit {
                decode_chain_bounded(&chain, db, limit)?
            } else {
                decode_chain(&chain, db)?
            };
            if blob {
                return Ok(Payload::Blob(Cow::Owned(decoded)));
            }
            return EncodedItem::from_bytes(decoded, cbor::Limits::DEFAULT)
                .map(Payload::Encoded)
                .map_err(|e| PayloadError::Damaged(PayloadCause::Cbor(e)));
        }
        if blob
            && let Some(bytes) = bytes.as_deref()
            && let Some(limit) = self
                .sink
                .as_deref()
                .and_then(StreamingSink::blob_decode_limit)
            && bytes.len() > limit
        {
            return Err(PayloadError::Budget(PayloadCause::DecodedBlobLimit {
                actual: bytes.len(),
                limit,
            }));
        }
        Ok(Payload::Value(d))
    }

    /// Fold one already-verified frame into the graph.
    ///
    /// Total: a missing capability degrades to an opaque node, and a corrupt
    /// payload degrades to a `damaged` opaque node — the reader never aborts.
    fn fold_frame(&mut self, frame: &[(Value, Value)], index: u64, data: Option<ValueRef<'_>>) {
        // Borrowed from `frame` (a parameter, not `self`), so the `&mut self`
        // handler calls below are unaffected and no per-frame `String` is made.
        let ftype = text_or(map_get(frame, "t"), "");
        if ftype == "blob" {
            self.h_blob_frame(frame, index);
            return;
        }
        let payload = match self.payload(frame, false, data) {
            Err(error) => {
                self.opaque(frame, ftype, error.opaque_reason());
                let detail = error.diagnostic_detail();
                self.diag_native(
                    error.diagnostic_code(FRAME_BUDGET_DIAGNOSTIC),
                    error,
                    detail,
                    index,
                );
                return;
            }
            Ok(p) => p,
        };
        let folded = (|| -> Result<(), cbor::DecodeError> {
            match ftype {
                "terms" => self.h_terms(payload.view(), index, TermIdBase(0))?,
                "quads" => self.h_quads(payload.view(), index, TermIdBase(0))?,
                "reifies" => self.h_reifies(payload.view(), index, TermIdBase(0))?,
                "annot" => self.h_annot(payload.view(), index, TermIdBase(0))?,
                "meta" => self.h_meta(payload.view().to_value()?.as_ref()),
                "suppress" => self.h_suppress(payload.view().to_value()?.as_ref()),
                "snapshot" => self.h_snapshot(payload.view(), index)?,
                "index" => self.h_index(payload.view().to_value()?.as_ref(), index),
                "opaque" => self.h_opaque(payload.view().to_value()?.as_ref()),
                _ => {
                    self.opaque(frame, ftype, "unknown-frame-type");
                    self.diag(
                        FoldError::UnknownFrameType {
                            frame_type: ftype.to_string(),
                        },
                        index,
                    );
                }
            }
            Ok(())
        })();
        if let Err(cause) = folded {
            let error = PayloadError::Damaged(PayloadCause::Cbor(cause));
            self.opaque(frame, ftype, error.opaque_reason());
            let detail = error.diagnostic_detail();
            self.diag_native(
                error.diagnostic_code(FRAME_BUDGET_DIAGNOSTIC),
                error,
                detail,
                index,
            );
        }
    }

    // -- per-type handlers ---------------------------------------------------

    fn damaged_row(&mut self, error: RowError, index: u64) {
        let message = error.to_string();
        self.diag_native("DamagedFrame", error, message, index);
    }

    fn term_reference(
        &mut self,
        value: Option<&Value>,
        base: TermIdBase,
        index: u64,
    ) -> Option<i128> {
        let value = value?;
        match base.integer(value) {
            Ok(value) => value,
            Err(error) => {
                // A bad reference must not renumber later positional terms.
                self.damaged_row(error, index);
                None
            }
        }
    }

    fn h_terms(
        &mut self,
        payload: ValueRef<'_>,
        index: u64,
        base: TermIdBase,
    ) -> Result<(), cbor::DecodeError> {
        let Some(rows) = payload.array()? else {
            return Ok(());
        };
        for raw in rows {
            let raw = raw?.to_value()?;
            let Value::Map(entries) = raw.as_ref() else {
                continue;
            };
            let kind = TermKind::from_wire(map_get(entries, "k").and_then(as_i128));
            let value = map_get(entries, "v")
                .and_then(Value::as_text)
                .map(str::to_string);
            // This row's id, read before any decode that can raise a diagnostic:
            // diagnostics leave the term table unchanged,
            // so the id a refusal quotes is the id this row goes on to take.
            let tid = self.terms().len() as i128;
            let term_id = self.terms().len();
            // THE language-tag decode point. `"l"` is the only field of the wire
            // format that becomes a `LANGTAG` token downstream, and this is the
            // only place it is read: `h_snapshot` re-enters through this very
            // function, `reader_union` and `compact` copy already-decoded
            // `Term`s, and the event bridge (`event_stream::stream_events`)
            // drives `read_to_sink_with_options`, i.e. this loop. So a tag that
            // does not pass here reaches neither the fold view's N-Quads token
            // (`purrdf_rdf::gts_view`'s `render_literal`) nor any `GtsEventSink`.
            //
            // Ask the grammar, on the profile every RDF codec and the IR kernel
            // already name (`model::LANGUAGE_TAG_PROFILE`). A refusal is NOT a
            // quiet coercion: it is a `DamagedFrame` diagnostic — the registry's
            // code for a malformed payload, whose stated reader behaviour is
            // "isolate the damaged item ..., surface a diagnostic, and fold
            // survivors" (GTS-CONFORMANCE.md §6) — that quotes the term id, the
            // production that rejected the tag, and the tag verbatim, so nothing
            // about the refusal has to be guessed. The damaged item is the tag;
            // the survivor is the literal's lexical form, which folds on under
            // §7.1's `xsd:string` default. This is the same shape the sibling
            // `"dir"` field already has directly below, where a value outside
            // `"ltr" / "rtl"` is likewise not carried through.
            //
            // The Baseline Reader cannot answer a malformed tag any other way:
            // `read` returns a `Graph`, not a `Result` (permissive-read "never
            // panic ..., return graph state plus diagnostics", §7), and dropping
            // the whole row is impossible because term ids are positional —
            // `tid` is `self.terms().len()`, so skipping a row would renumber
            // every later term in the segment.
            // `lang_refused` distinguishes "there was no tag" from "there was a
            // tag and the grammar refused it". The base-direction arm below
            // needs that distinction: the two cases must not fold to the same
            // outcome, because only one of them is a defect in the input.
            let (lang, lang_refused) = match map_get(entries, "l").and_then(Value::as_text) {
                Some(tag) => match language_tag_refusal(tag) {
                    Some(code) => {
                        self.diag(
                            FoldError::Language {
                                term: tid,
                                tag: tag.to_string(),
                                refusal: code,
                            },
                            index,
                        );
                        (None, true)
                    }
                    None => (Some(tag.to_string()), false),
                },
                None => (None, false),
            };
            // Same reasoning as the tag arm above, and the same failure shape. A
            // value outside the closed `ltr`/`rtl` space cannot be carried
            // through, and dropping it without saying so is worse than it looks:
            // the term keeps its lexical form and its language, so an
            // `rdf:dirLangString` quietly becomes an `rdf:langString` and the
            // graph asserts a literal the container does not contain — hence the
            // diagnostic. The writer filters direction before emitting it, so a
            // value here can only have come from a foreign container, which is
            // precisely the case a diagnostic is for.
            let direction = match map_get(entries, "dir").and_then(Value::as_text) {
                Some(value) if crate::model::is_literal_direction(value) => Some(value.to_string()),
                Some(value) => {
                    self.diag(
                        FoldError::Direction {
                            term: tid,
                            direction: value.to_string(),
                        },
                        index,
                    );
                    None
                }
                None => None,
            };
            // A refused tag takes the base direction down with it. `direction`
            // is only ever the second half of an `rdf:dirLangString`, so once
            // the language is gone it describes nothing: carrying it on would
            // build a term with a direction and no language, which is outside
            // RDF 1.2's value space. Leaving it standing is not a harmless
            // remnant either — it makes the importer downstream refuse with
            // `gts-direction-without-language`, naming a defect the container
            // never had and pointing away from the tag that actually failed.
            //
            // This is a consequence of a refusal, not a second input error, so
            // it gets its own diagnostic rather than riding along silently on
            // the tag's: dropping carried data without saying so is the
            // documented mirror of over-refusal, and it costs a reader exactly
            // the field it would need to understand the first diagnostic.
            //
            // Only a *refused* tag triggers this. A `"dir"` with no `"l"` at
            // all is a genuine defect in the container, and the importer's
            // `gts-direction-without-language` is then the correct name for it
            // — so that case is left alone to be reported accurately.
            let direction = match (lang_refused, direction) {
                (true, Some(direction)) => {
                    self.diag(
                        FoldError::RefusedLanguageDirection {
                            term: tid,
                            direction,
                        },
                        index,
                    );
                    None
                }
                (_, direction) => direction,
            };
            let dt_raw = self.term_reference(map_get(entries, "dt"), base, index);
            let rf_raw = self.term_reference(map_get(entries, "rf"), base, index);
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
            // `tt` — the quoted triple's OWN components (§7.1). Every id must
            // name an already-introduced term, exactly like `dt`; a triple term
            // therefore cannot contain itself, which is what makes `tt` the
            // acyclic-by-construction spelling of a quoted triple.
            let tt_raw = match map_get(entries, "tt") {
                Some(Value::Array(ids)) if ids.len() == 3 => Some(
                    [&ids[0], &ids[1], &ids[2]]
                        .map(|value| self.term_reference(Some(value), base, index)),
                ),
                _ => None,
            };
            let tt = match tt_raw {
                Some([Some(s), Some(p), Some(o)])
                    if kind == TermKind::Triple
                        && [s, p, o].iter().all(|d| (0..tid).contains(d)) =>
                {
                    Some((s as usize, p as usize, o as usize))
                }
                _ => None,
            };
            let tt_out_of_range = tt.is_none()
                && matches!(tt_raw, Some(ids) if ids.iter().any(|d| matches!(d, Some(d) if *d >= tid)));
            if dt_out_of_range || rf_out_of_range || tt_out_of_range {
                self.diag(
                    FoldError::ForwardReference {
                        term: tid,
                        datatype: dt_raw,
                        reifier: rf_raw,
                        triple: tt_raw,
                    },
                    index,
                );
            }
            let term = Term {
                kind,
                value,
                datatype: dt,
                lang,
                direction,
                reifier: rf,
                triple: tt,
            };
            if let Some(terms) = self.shared_terms.as_deref_mut() {
                let term = Arc::new(term);
                terms.push(Arc::clone(&term));
                self.reifier_index.observe_term(term_id, &term);
                if let Some(sink) = self.sink.as_deref_mut() {
                    sink.term(self.segment_index, term_id, term);
                }
            } else {
                self.g.terms.push(term);
                self.reifier_index
                    .observe_term(term_id, &self.g.terms[term_id]);
            }
        }
        Ok(())
    }

    fn h_quads(
        &mut self,
        payload: ValueRef<'_>,
        index: u64,
        base: TermIdBase,
    ) -> Result<(), cbor::DecodeError> {
        let Some(rows) = payload.array()? else {
            return Ok(());
        };
        for row in rows {
            let Some(items) = numeric_row::<4>(row?)? else {
                continue;
            };
            if items.width < 3 {
                continue;
            }
            let raw = match base.row::<4>(items.values) {
                Ok(raw) => raw,
                Err(error) => {
                    self.damaged_row(error, index);
                    continue;
                }
            };
            let [s, p, o, gslot] = raw.map(|value| value.and_then(|id| usize::try_from(id).ok()));
            let has_graph = items.width >= 4;
            if s.is_none() || p.is_none() || o.is_none() || (has_graph && gslot.is_none()) {
                let error = RowError::QuadIds {
                    values: raw[..items.width.min(4)].to_vec(),
                    width: items.width,
                };
                self.damaged_row(error, index);
                continue;
            }
            let (Some(s), Some(p), Some(o)) = (s, p, o) else {
                continue;
            };
            if let Err(detail) = check_quad_positions(self.terms(), s, p, o, gslot) {
                let message = detail.to_string();
                self.diag_native("PositionConstraint", *detail, message, index);
                continue;
            }
            let quad = (s, p, o, gslot);
            if self.streamable {
                let terms = match self.shared_terms.as_deref() {
                    Some(terms) => TermsRef::Shared(terms),
                    None => TermsRef::Owned(&self.g.terms),
                };
                self.provenance.observe_terms(terms, s, p, o);
            }
            self.with_sink(|segment_index, sink| sink.quad(segment_index, quad));
            if self.materialize {
                self.g.quads.push(quad);
            }
            // Layout bookkeeping (§3.3): a stream:digest quad describes an
            // upcoming manifestation — record the IOU for the blob check.
            if self.terms()[p].value.as_deref() == Some(STREAM_DIGEST)
                && let Some(obj) = self.terms()[o].value.clone()
            {
                self.described.insert(obj);
            }
        }
        Ok(())
    }

    fn h_reifies(
        &mut self,
        payload: ValueRef<'_>,
        index: u64,
        base: TermIdBase,
    ) -> Result<(), cbor::DecodeError> {
        let Some(rows) = payload.array()? else {
            self.diag(FoldError::ReifiesArray { snapshot: false }, index);
            return Ok(());
        };
        for row in rows {
            let (rid, triple, gslot) =
                match decode_reifier_row(numeric_row::<5>(row?)?, self.terms(), base) {
                    RowDecode::Skip => continue,
                    RowDecode::Row(row) => row,
                    RowDecode::Damaged(detail) => {
                        self.damaged_row(*detail, index);
                        continue;
                    }
                    RowDecode::Position(detail) => {
                        let message = detail.to_string();
                        self.diag_native("PositionConstraint", *detail, message, index);
                        continue;
                    }
                };
            // `rdf:reifies` is NOT functional: one reifier id legitimately
            // binds several distinct triple terms (`r rdf:reifies <<s p o1>>`
            // and `r rdf:reifies <<s p o2>>` are both assertable, in the same
            // graph or in different ones). Every binding is therefore recorded;
            // the only thing a rebind can still break is the LEGACY indirect
            // spelling of a quoted-triple term (one that names this reifier id
            // through `rf` and carries no `tt` of its own), whose meaning would
            // become double-valued. That shape is flagged after the fold, once
            // every term and every binding of the segment has been seen.
            if matches!(self.reifier_index.first_binding(rid), Some(existing) if existing != triple)
            {
                self.rebound_reifiers.push((rid, index));
            }
            if self
                .reifier_index
                .binding_is_recursive(self.terms(), rid, triple)
            {
                self.diag(
                    FoldError::RecursiveReifier {
                        reifier: rid,
                        triple,
                    },
                    index,
                );
                continue;
            }
            self.reifier_index
                .record(self.g, (rid, triple, gslot), self.materialize);
            self.with_sink(|segment_index, sink| sink.reifier(segment_index, (rid, triple, gslot)));
        }
        Ok(())
    }

    fn h_annot(
        &mut self,
        payload: ValueRef<'_>,
        index: u64,
        base: TermIdBase,
    ) -> Result<(), cbor::DecodeError> {
        let Some(rows) = payload.array()? else {
            return Ok(());
        };
        for row in rows {
            let annotation =
                match decode_annotation_row(numeric_row::<4>(row?)?, self.terms(), base) {
                    RowDecode::Skip => continue,
                    RowDecode::Row(row) => row,
                    RowDecode::Damaged(detail) => {
                        self.damaged_row(*detail, index);
                        continue;
                    }
                    RowDecode::Position(detail) => {
                        let message = detail.to_string();
                        self.diag_native("PositionConstraint", *detail, message, index);
                        continue;
                    }
                };
            self.with_sink(|segment_index, sink| sink.annotation(segment_index, annotation));
            if self.materialize {
                self.g.annotations.push(annotation);
            }
        }
        Ok(())
    }

    fn h_blob_frame(&mut self, frame: &[(Value, Value)], index: u64) {
        let d = map_get(frame, "d");
        let declared_metadata = map_get(frame, "pub");
        let declared_digest = declared_metadata.and_then(public_blob_digest);
        let malformed = if d.is_some_and(|value| !matches!(value, Value::Bytes(_))) {
            Some(PayloadCause::BlobNotBytes)
        } else if matches!(map_get(frame, "x"), Some(Value::Array(ids)) if !ids.is_empty())
            && d.is_none()
        {
            Some(PayloadCause::TransformedNotBytes)
        } else if d.is_none() && declared_digest.is_none() {
            Some(PayloadCause::BlobMissingIdentity)
        } else {
            None
        };
        if let Some(cause) = malformed {
            let error = PayloadError::Damaged(cause);
            let detail = error.diagnostic_detail();
            self.opaque(frame, "blob", error.opaque_reason());
            self.diag_native(
                error.diagnostic_code(BLOB_BUDGET_DIAGNOSTIC),
                error,
                detail,
                index,
            );
            return;
        }
        let pub_meta = declared_metadata
            .filter(|value| matches!(value, Value::Map(_)))
            .cloned();
        let encoded_len = d.and_then(Value::as_bytes).map_or(0, <[u8]>::len);
        let chain = match map_get(frame, "x") {
            Some(Value::Array(ids)) if !ids.is_empty() => match self.resolve_codecs(ids) {
                Ok(chain) => chain,
                Err(error) => {
                    self.opaque(frame, "blob", error.opaque_reason());
                    let detail = error.diagnostic_detail();
                    self.diag_native(
                        error.diagnostic_code(BLOB_BUDGET_DIAGNOSTIC),
                        error,
                        detail,
                        index,
                    );
                    return;
                }
            },
            _ => Vec::new(),
        };

        if chain.iter().any(|codec| codec.cls == "encrypt") {
            match self
                .payload(frame, true, map_get(frame, "d").map(ValueRef::Owned))
                .and_then(payload_bytes)
            {
                Ok(Some(bytes)) => {
                    let digest = digest_str(&bytes);
                    if let Some(meta) = pub_meta {
                        self.blob_meta_index
                            .set(&mut self.g.blob_meta, digest.clone(), meta);
                    }
                    self.blob_events.push((
                        index,
                        digest.clone(),
                        self.described.contains(&digest),
                    ));
                    self.emit_blob(
                        &digest,
                        Some(&bytes),
                        &[],
                        declared_metadata,
                        true,
                        encoded_len,
                    );
                    if self.materialize {
                        self.blob_index.set(
                            &mut self.g.blobs,
                            digest.clone(),
                            BlobEntry::bytes(bytes.into_owned()),
                        );
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    let detail = error.diagnostic_detail();
                    if let Some(kind) = error.blob_refusal_kind() {
                        let (refused_digest, digest_computed) =
                            refused_identity(&chain, d, pub_meta.as_ref());
                        self.emit_blob_refusal(
                            kind,
                            refused_digest.as_deref(),
                            digest_computed,
                            declared_metadata,
                            encoded_len,
                            &detail,
                        );
                    }
                    self.opaque(frame, "blob", error.opaque_reason());
                    self.diag_native(
                        error.diagnostic_code(BLOB_BUDGET_DIAGNOSTIC),
                        error,
                        detail,
                        index,
                    );
                }
            }
            return;
        }

        if let Some(digest) = declared_digest {
            if let Some(meta) = pub_meta {
                self.blob_meta_index
                    .set(&mut self.g.blob_meta, digest.clone(), meta);
            }
            self.emit_blob(
                &digest,
                d.and_then(Value::as_bytes),
                &chain,
                declared_metadata,
                d.is_some(),
                encoded_len,
            );
            if let Some(Value::Bytes(raw)) = d
                && self.materialize
            {
                if chain.is_empty() {
                    self.blob_index.set(
                        &mut self.g.blobs,
                        digest.clone(),
                        BlobEntry::bytes(raw.clone()),
                    );
                } else {
                    self.blob_index.set(
                        &mut self.g.blobs,
                        digest.clone(),
                        BlobEntry::lazy(raw.clone(), chain),
                    );
                }
            }
            self.blob_events
                .push((index, digest.clone(), self.described.contains(&digest)));
            return;
        }

        let Some(Value::Bytes(_)) = d else {
            return;
        };
        match self
            .payload(frame, true, map_get(frame, "d").map(ValueRef::Owned))
            .and_then(payload_bytes)
        {
            Ok(Some(bytes)) => {
                let digest = digest_str(&bytes);
                if let Some(meta) = pub_meta {
                    self.blob_meta_index
                        .set(&mut self.g.blob_meta, digest.clone(), meta);
                }
                self.blob_events
                    .push((index, digest.clone(), self.described.contains(&digest)));
                self.emit_blob(
                    &digest,
                    Some(&bytes),
                    &[],
                    declared_metadata,
                    true,
                    encoded_len,
                );
                if self.materialize {
                    self.blob_index.set(
                        &mut self.g.blobs,
                        digest.clone(),
                        BlobEntry::bytes(bytes.into_owned()),
                    );
                }
            }
            Ok(_) => {}
            Err(error) => {
                let detail = error.diagnostic_detail();
                if let Some(kind) = error.blob_refusal_kind() {
                    let (refused_digest, digest_computed) =
                        refused_identity(&chain, d, pub_meta.as_ref());
                    self.emit_blob_refusal(
                        kind,
                        refused_digest.as_deref(),
                        digest_computed,
                        declared_metadata,
                        encoded_len,
                        &detail,
                    );
                }
                self.opaque(frame, "blob", error.opaque_reason());
                self.diag_native(
                    error.diagnostic_code(BLOB_BUDGET_DIAGNOSTIC),
                    error,
                    detail,
                    index,
                );
            }
        }
    }

    fn h_meta(&mut self, payload: &Value) {
        if let Value::Map(entries) = payload {
            for (k, v) in entries {
                let key = k.as_text().map_or_else(|| format!("{k:?}"), str::to_string);
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
                .and_then(Value::as_text)
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
    /// Decode local term ids against the captured outer base through the same
    /// handlers and semantic checks as equivalent streamed frames. The source
    /// payload remains borrowed; rebasing never builds another CBOR tree.
    fn h_snapshot(&mut self, payload: ValueRef<'_>, index: u64) -> Result<(), cbor::DecodeError> {
        if !payload.is_map() {
            return Ok(());
        }
        let base = TermIdBase(self.terms().len());
        if let Some(terms) = payload.get("terms")? {
            self.h_terms(terms, index, base)?;
        }
        if let Some(quads) = payload.get("quads")? {
            self.h_quads(quads, index, base)?;
        }
        if let Some(reifies) = payload.get("reifies")? {
            if reifies.is_map() {
                self.diag(FoldError::ReifiesArray { snapshot: true }, index);
            } else if reifies.array()?.is_some() {
                self.h_reifies(reifies, index, base)?;
            }
        }
        if let Some(annot) = payload.get("annot")? {
            self.h_annot(annot, index, base)?;
        }
        if let Some(blobs) = payload.get("blobs")?
            && let Some(blobs) = blobs.map()?
        {
            let ceiling = self
                .sink
                .as_deref()
                .and_then(StreamingSink::blob_decode_limit);
            for entry in blobs {
                let (_, value) = entry?;
                if let Some(bytes) = value.bytes()? {
                    let digest = digest_str(&bytes);
                    if let Some(limit) = ceiling
                        && bytes.len() > limit
                    {
                        let error = PayloadError::Budget(PayloadCause::SnapshotBlobLimit {
                            actual: bytes.len(),
                            limit,
                        });
                        let detail = error.diagnostic_detail();
                        self.emit_blob_refusal(
                            BlobRefusalKind::ByteBudget,
                            Some(&digest),
                            true,
                            None,
                            bytes.len(),
                            &detail,
                        );
                        self.diag_native(
                            error.diagnostic_code(BLOB_BUDGET_DIAGNOSTIC),
                            error,
                            detail,
                            index,
                        );
                        continue;
                    }
                    if self.materialize {
                        self.blob_index.set(
                            &mut self.g.blobs,
                            digest.clone(),
                            BlobEntry::bytes(bytes.to_vec()),
                        );
                    }
                    self.emit_blob(&digest, Some(&bytes), &[], None, true, bytes.len());
                }
            }
        }
        if let Some(meta) = payload.get("meta")?
            && let Some(meta) = meta.map()?
        {
            for entry in meta {
                let (key, value) = entry?;
                let key = match key.text()? {
                    Some(text) => text.into_owned(),
                    None => format!("{:?}", key.to_value()?.as_ref()),
                };
                self.g.set_meta(key, value.to_value()?.into_owned());
            }
        }
        Ok(())
    }

    /// Record an intact `index` frame (§6.2) for the layout check (§3.3).
    ///
    /// The index stays an accelerator for the fold itself; only `count` and
    /// `head` are consumed here, as the covered-region boundary. A payload
    /// without a valid count/head pair is simply not an intact index.
    fn h_index(&mut self, payload: &Value, index: u64) {
        let Value::Map(entries) = payload else { return };
        let count = map_get(entries, "count")
            .and_then(as_i128)
            .and_then(|count| u64::try_from(count).ok());
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
pub(crate) fn is_header_item(item: &Value) -> bool {
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
fn header_dict_table(header: &[(Value, Value)]) -> FastMap<&str, &[u8]> {
    let mut out = FastMap::default();
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
fn catalog_from(header: &[(Value, Value)]) -> FastMap<i128, Codec> {
    let dict_table = header_dict_table(header);
    let mut out = FastMap::default();
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
                        // §8.5 `level?`: the authoring compression level, made
                        // observable so a profile can gate on it.
                        level: map_get(fields, "level")
                            .and_then(as_i128)
                            .and_then(|level| i32::try_from(level).ok()),
                    },
                );
            }
        }
    }
    out
}

/// One catalog row exactly as it appears on the wire (§5, §8.5), keeping the
/// dictionary NAME the entry references rather than the resolved bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogRow {
    /// File-local catalog id.
    pub id: i64,
    /// Canonical codec name (§8.5 registry).
    pub name: String,
    /// `"encode"` | `"compress"` | `"encrypt"`.
    pub cls: String,
    /// Header `"dct"` key this entry binds to, if any.
    pub dct: Option<String>,
    /// Declared `level` parameter, if any.
    pub level: Option<i32>,
}

/// Everything a writer needs to APPEND to an existing segment rather than
/// starting a new one: the on-wire catalog, the in-band dictionaries, and the
/// current head id the next frame must reference as `"prev"`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentAppendState {
    /// The last segment's catalog rows, ids included.
    pub catalog: Vec<CatalogRow>,
    /// The last segment's in-band dictionaries by name (§5 header `"dct"`).
    pub dicts: BTreeMap<String, Vec<u8>>,
    /// The id of the last item in the file — the `"prev"` of the next frame.
    pub head: Vec<u8>,
}

/// Recover [`SegmentAppendState`] from an existing GTS file.
///
/// Reads the LAST segment header (§3.1) and the id of the file's last item, so
/// a subsequent writer continues that one append-only chain instead of minting
/// a second header. Nothing is folded and no chain is verified here — this is
/// the authoring-side inverse of the header parse, not a reader.
///
/// # Errors
/// Returns an error when the file is empty, ends in a torn append, carries no
/// segment header, or its last item has no `"id"` — appending onto any of those
/// would silently break the chain.
pub fn segment_append_state(data: &[u8]) -> Result<SegmentAppendState, AppendError> {
    let (items, torn) = iter_items_with_error(data);
    if let Some(error) = torn {
        return Err(AppendError::Torn {
            offset: usize::try_from(error.offset)
                .expect("buffered item offsets fit the address space"),
            cause: error.cause,
        });
    }
    let Some(last_header) = items.iter().rposition(|(_, item)| is_header_item(item)) else {
        return Err(AppendError::MissingHeader);
    };
    let header = unwrap_header(&items[last_header].1).map_err(AppendError::Header)?;

    let mut catalog: Vec<CatalogRow> = Vec::new();
    if let Some(Value::Map(raw)) = map_get(header, "cat") {
        for (cid, entry) in raw {
            let (Some(cid), Value::Map(fields)) = (as_i128(cid), entry) else {
                return Err(AppendError::CatalogEntry {
                    id: cid.clone(),
                    entry: entry.clone(),
                });
            };
            let level = match map_get(fields, "level") {
                None => None,
                Some(value) => {
                    let raw = as_i128(value).ok_or_else(|| AppendError::CatalogLevel {
                        id: cid,
                        value: value.clone(),
                    })?;
                    Some(i32::try_from(raw).map_err(|_| AppendError::LevelRange {
                        id: cid,
                        actual: raw,
                    })?)
                }
            };
            catalog.push(CatalogRow {
                id: i64::try_from(cid).map_err(|_| AppendError::IdRange { actual: cid })?,
                name: text_or(map_get(fields, "name"), "").to_string(),
                cls: text_or(map_get(fields, "cls"), "encode").to_string(),
                dct: map_get(fields, "dct")
                    .and_then(Value::as_text)
                    .map(str::to_string),
                level,
            });
        }
    }
    catalog.sort_by_key(|row| row.id);

    let mut dicts: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    if let Some(Value::Map(entries)) = map_get(header, "dct") {
        for (name, bytes) in entries {
            let (Value::Text(name), Value::Bytes(bytes)) = (name, bytes) else {
                return Err(AppendError::DictionaryEntry {
                    name: name.clone(),
                    bytes: bytes.clone(),
                });
            };
            dicts.insert(name.clone(), bytes.clone());
        }
    }
    // Fail closed: every dictionary a catalog entry names must be present, or
    // frames appended against that entry would be undecodable.
    for row in &catalog {
        if let Some(name) = &row.dct
            && !dicts.contains_key(name)
        {
            return Err(AppendError::MissingDictionary {
                id: row.id,
                name: name.clone(),
            });
        }
    }

    let last = items.last().expect("a header item was found above");
    let entries = unwrap_header(&last.1).map_err(AppendError::Header)?;
    let head = match map_get(entries, "id") {
        Some(Value::Bytes(id)) => id.clone(),
        actual => {
            return Err(AppendError::MissingHead {
                actual: actual.cloned(),
            });
        }
    };
    Ok(SegmentAppendState {
        catalog,
        dicts,
        head,
    })
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
    let (items, torn) = iter_items_with_error(data);
    if items.is_empty() {
        let mut g = Graph::default();
        let diagnostic = Diagnostic {
            cause: None,
            code: "EmptyFile".to_string(),
            detail: "no CBOR items".to_string(),
            frame_index: None,
        };
        g.diagnostics.push(match torn {
            Some(error) => diagnostic.with_native_cause(error),
            None => diagnostic.with_native_cause(FoldError::EmptyFile),
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
        g.diagnostics.push(
            Diagnostic {
                cause: None,
                code: "DamagedFrame".to_string(),
                detail: "first item is not a header".to_string(),
                frame_index: Some(0),
            }
            .with_native_cause(FoldError::FirstHeader),
        );
        return g;
    }
    if bounds.len() > 1 && !options.allow_segments {
        let mut g = read_segment(&items[..bounds[1]], 0, options.content_key);
        g.diagnostics.push(
            Diagnostic {
                cause: None,
                code: "SegmentBoundary".to_string(),
                detail: format!(
                    "segment boundary at item {} but reader is in pre-segment mode; \
                 remainder of file NOT folded (folding it with file-global \
                 term-ids would silently misfold — §16)",
                    bounds[1]
                ),
                frame_index: Some(bounds[1] as u64),
            }
            .with_native_cause(FoldError::SegmentBoundary {
                item: bounds[1] as u64,
            }),
        );
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
            g.diagnostics.push(
                Diagnostic {
                    cause: None,
                    code: "TruncatedLog".to_string(),
                    detail: "observed head does not match expected head".to_string(),
                    frame_index: None,
                }
                .with_native_cause(FoldError::ExpectedHead {
                    actual: last_head,
                    expected: expected.to_vec(),
                }),
            );
        }
    }
    if let Some(error) = torn {
        g.diagnostics.push(
            Diagnostic {
                cause: None,
                code: "TornAppendError".to_string(),
                detail: error.to_string(),
                frame_index: None,
            }
            .with_native_cause(error),
        );
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
    pos: u64,
    address_exhausted: Option<(u64, usize)>,
}

impl<R: Read> Read for StreamingCountingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;
        self.pos = match self.pos.checked_add(read as u64) {
            Some(position) => position,
            None => {
                self.address_exhausted = Some((self.pos, read));
                return Err(purrdf_hash::error::io_error(
                    std::io::ErrorKind::Other,
                    FoldError::ByteOffset {
                        position: self.pos,
                        read,
                    },
                ));
            }
        };
        Ok(read)
    }
}

fn next_stream_item<R: Read>(
    reader: &mut StreamingCountingReader<R>,
) -> Result<Option<(StreamItem, u64, u64)>, TornAppendError> {
    let start = reader.pos;
    match cbor::read_encoded_from(&mut *reader, cbor::Limits::DEFAULT)
        .and_then(|item| item.map(encoded_frame::classify).transpose())
    {
        Ok(Some(item)) => Ok(Some((item, start, reader.pos))),
        Ok(None) => Ok(None),
        Err(cause) => Err(TornAppendError {
            offset: start,
            cause,
        }),
    }
}

struct ActiveStreamingSegment {
    g: Graph,
    // These owners outlive every validation callback in the segment, then drop
    // before the resolver flushes it on the next segment's first RDF event.
    shared_terms: Vec<Arc<Term>>,
    header: Vec<(Value, Value)>,
    expected_prev: Vec<u8>,
    frame_ids: Vec<Vec<u8>>,
    index_offset: u64,
    segment_index: u64,
    valid_header: bool,
    provenance: crate::compact::ProvenanceSubjects,
    catalog: FastMap<i128, Codec>,
    blob_index: DigestIndex,
    blob_meta_index: DigestIndex,
    index_records: Vec<IndexRecord>,
    described: FastSet<String>,
    blob_events: Vec<(u64, String, bool)>,
    // Carried across this segment's frames (each frame gets a fresh `Folder`)
    // so the ambiguity check runs once the whole segment has been seen.
    rebound_reifiers: Vec<(usize, u64)>,
    // Carried across frames for the same reason: it indexes the whole
    // segment's terms and reifier rows, not one frame's.
    reifier_index: ReifierIndex,
}

impl ActiveStreamingSegment {
    fn new(
        raw_header: Value,
        index_offset: u64,
        segment_index: u64,
        sink: &mut dyn StreamingSink,
    ) -> Self {
        let mut g = Graph::default();
        let mut sink_slot = Some(sink);
        let mut header = Vec::new();
        let mut expected_prev = Vec::new();
        let mut valid_header = false;
        match crate::wire::into_header(raw_header) {
            Ok(entries) => {
                header = entries;
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
                            cause: None,
                            code: "DamagedFrame".to_string(),
                            detail: "header self-hash mismatch".to_string(),
                            frame_index: Some(index_offset),
                        }
                        .with_native_cause(FoldError::HeaderHash {
                            stored: stored_hid.clone(),
                            computed: header_id(&header),
                        }),
                    );
                }
                if map_get(&header, "gts").and_then(Value::as_text) != Some(MAGIC)
                    || map_get(&header, "v").and_then(as_i128) != Some(i128::from(VERSION))
                {
                    push_diagnostic(
                        &mut g,
                        &mut sink_slot,
                        Diagnostic {
                            cause: None,
                            code: "DamagedFrame".to_string(),
                            detail: format!(
                                "unsupported header magic/version {:?}/{:?}",
                                map_get(&header, "gts"),
                                map_get(&header, "v")
                            ),
                            frame_index: Some(index_offset),
                        }
                        .with_native_cause(FoldError::HeaderVersion {
                            magic: map_get(&header, "gts").cloned(),
                            version: map_get(&header, "v").cloned(),
                        }),
                    );
                }
                expected_prev = stored_hid.unwrap_or_default();
            }
            Err(e) => {
                push_diagnostic(
                    &mut g,
                    &mut sink_slot,
                    Diagnostic {
                        cause: None,
                        code: "DamagedFrame".to_string(),
                        detail: format!("invalid header: {e}"),
                        frame_index: Some(index_offset),
                    }
                    .with_native_cause(e),
                );
            }
        }
        let catalog = if valid_header {
            catalog_from(&header)
        } else {
            FastMap::default()
        };
        Self {
            g,
            shared_terms: Vec::new(),
            header,
            expected_prev,
            frame_ids: Vec::new(),
            index_offset,
            segment_index,
            valid_header,
            provenance: crate::compact::ProvenanceSubjects::default(),
            catalog,
            blob_index: DigestIndex::default(),
            blob_meta_index: DigestIndex::default(),
            index_records: Vec::new(),
            described: FastSet::default(),
            blob_events: Vec::new(),
            rebound_reifiers: Vec::new(),
            reifier_index: ReifierIndex::default(),
        }
    }

    fn process_frame<'k>(
        &mut self,
        raw: BorrowedItem<'_>,
        abs_index: u64,
        frame_start: u64,
        frame_end: u64,
        sink: &mut dyn StreamingSink,
        content_key: Option<&'k ContentKeyResolver<'k>>,
    ) -> Result<(), cbor::DecodeError> {
        if !self.valid_header {
            return Ok(());
        }
        let mut parsed = encoded_frame::Frame::new(raw)?;
        let raw = &parsed.metadata;
        // Captured before any push below — the zero-based index of THIS
        // frame within the segment (mirrors `collect_frames` in
        // `replication.rs`).
        let frame_index = self.frame_ids.len() as u64;
        let catalog = std::mem::take(&mut self.catalog);
        let blob_index = std::mem::take(&mut self.blob_index);
        let blob_meta_index = std::mem::take(&mut self.blob_meta_index);
        let index_records = std::mem::take(&mut self.index_records);
        let described = std::mem::take(&mut self.described);
        let blob_events = std::mem::take(&mut self.blob_events);
        let rebound_reifiers = std::mem::take(&mut self.rebound_reifiers);
        let reifier_index = std::mem::take(&mut self.reifier_index);
        let mut folder = Folder {
            g: &mut self.g,
            shared_terms: Some(&mut self.shared_terms),
            sink: Some(sink),
            content_key,
            segment_index: self.segment_index,
            materialize: false,
            streamable: map_get(&self.header, "layout").and_then(Value::as_text)
                == Some("streamable"),
            provenance: std::mem::take(&mut self.provenance),
            catalog,
            blob_index,
            blob_meta_index,
            index_records,
            described,
            blob_events,
            rebound_reifiers,
            reifier_index,
        };
        // `sink.frame()` is fired for THIS frame before any of its rows are
        // folded (the `StreamingSink::frame` contract), so index builders can
        // attribute the rows that follow to this frame's provenance.
        let range = ByteRange {
            start: frame_start,
            end: frame_end,
        };
        if let Value::Map(frame) = raw {
            let stored_id: Option<&Vec<u8>> = match map_get(frame, "id") {
                Some(Value::Bytes(b)) => Some(b),
                _ => None,
            };
            let computed = parsed
                .computed
                .take()
                .expect("an encoded map has its canonical content ID");
            let ftype = text_or(map_get(frame, "t"), "").to_string();
            if stored_id.map(|b| &b[..]) != Some(&computed[..]) {
                let ctx_id = stored_id.cloned().unwrap_or_else(|| computed.clone());
                folder.with_sink(|segment_index, sink| {
                    sink.frame(FrameContext {
                        segment_index,
                        frame_index,
                        content_id: &ctx_id,
                        range,
                        frame_type: &ftype,
                        valid: false,
                    });
                });
                folder.diag(
                    FoldError::SelfHash {
                        stored: stored_id.cloned(),
                        computed: computed.clone(),
                    },
                    abs_index,
                );
                folder.opaque(frame, &ftype, "damaged");
                self.expected_prev = stored_id.cloned().unwrap_or(computed);
                self.frame_ids.push(self.expected_prev.clone());
            } else {
                let prev_ok = matches!(map_get(frame, "prev"),
                    Some(Value::Bytes(b)) if *b == self.expected_prev);
                folder.with_sink(|segment_index, sink| {
                    sink.frame(FrameContext {
                        segment_index,
                        frame_index,
                        content_id: &computed,
                        range,
                        frame_type: &ftype,
                        valid: prev_ok,
                    });
                });
                if !prev_ok {
                    folder.diag(
                        FoldError::Previous {
                            actual: map_get(frame, "prev")
                                .and_then(Value::as_bytes)
                                .map(<[u8]>::to_vec),
                            expected: self.expected_prev.clone(),
                        },
                        abs_index,
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
                        frame_id: computed.clone(),
                        kid: None,
                        status: status.to_string(),
                        cose,
                        packaging: crate::compact::packaging_role(
                            &folder.provenance,
                            text_or(map_get(frame, "t"), ""),
                            folder.streamable,
                        ),
                    });
                }
                let data = if parsed.borrowed_data {
                    parsed.data
                } else {
                    map_get(frame, "d").map(ValueRef::Owned)
                };
                folder.fold_frame(frame, abs_index, data);
            }
        } else {
            folder.with_sink(|segment_index, sink| {
                sink.frame(FrameContext {
                    segment_index,
                    frame_index,
                    content_id: &[],
                    range,
                    frame_type: "<non-map>",
                    valid: false,
                });
            });
            folder.diag(FoldError::FrameMap, abs_index);
            self.frame_ids.push(Vec::new());
        }
        let catalog = std::mem::take(&mut folder.catalog);
        let blob_index = std::mem::take(&mut folder.blob_index);
        let blob_meta_index = std::mem::take(&mut folder.blob_meta_index);
        let index_records = std::mem::take(&mut folder.index_records);
        let described = std::mem::take(&mut folder.described);
        let blob_events = std::mem::take(&mut folder.blob_events);
        let rebound_reifiers = std::mem::take(&mut folder.rebound_reifiers);
        let reifier_index = std::mem::take(&mut folder.reifier_index);
        self.provenance = std::mem::take(&mut folder.provenance);
        drop(folder);
        self.catalog = catalog;
        self.blob_index = blob_index;
        self.blob_meta_index = blob_meta_index;
        self.index_records = index_records;
        self.described = described;
        self.blob_events = blob_events;
        self.rebound_reifiers = rebound_reifiers;
        self.reifier_index = reifier_index;
        Ok(())
    }

    fn finish_into_result(
        mut self,
        result: &mut StreamingReadResult,
        sink: &mut dyn StreamingSink,
    ) {
        if self.valid_header {
            // Every frame of this segment has been folded, so a multi-bound
            // reifier id can now be judged against the segment's full term
            // table (see `Folder::report_ambiguous_reifiers`).
            if !self.rebound_reifiers.is_empty() {
                let mut folder = Folder {
                    g: &mut self.g,
                    shared_terms: Some(&mut self.shared_terms),
                    sink: Some(sink),
                    content_key: None,
                    segment_index: self.segment_index,
                    materialize: false,
                    streamable: map_get(&self.header, "layout").and_then(Value::as_text)
                        == Some("streamable"),
                    provenance: std::mem::take(&mut self.provenance),
                    catalog: FastMap::default(),
                    blob_index: std::mem::take(&mut self.blob_index),
                    blob_meta_index: std::mem::take(&mut self.blob_meta_index),
                    index_records: Vec::new(),
                    described: FastSet::default(),
                    blob_events: Vec::new(),
                    rebound_reifiers: std::mem::take(&mut self.rebound_reifiers),
                    reifier_index: std::mem::take(&mut self.reifier_index),
                };
                folder.report_ambiguous_reifiers();
            }
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
        address_exhausted: None,
    };
    let mut result = StreamingReadResult::default();
    let mut torn_cause = None;
    let mut item_index = 0u64;
    let mut current: Option<ActiveStreamingSegment> = None;
    loop {
        let (item, frame_start, frame_end) = match next_stream_item(&mut reader) {
            Ok(Some(item)) => item,
            Ok(None) => break,
            Err(torn) => {
                if let Some((position, read)) = reader.address_exhausted {
                    push_result_diagnostic(
                        &mut result,
                        sink,
                        Diagnostic {
                            cause: None,
                            code: "ResourceLimit".to_owned(),
                            detail: "GTS source byte offset exceeds u64::MAX".to_owned(),
                            frame_index: Some(item_index),
                        }
                        .with_native_cause(FoldError::ByteOffset { position, read }),
                    );
                    return result;
                }
                result.torn = Some(torn.offset);
                torn_cause = Some(torn);
                break;
            }
        };
        if let StreamItem::Header(item) = item {
            if let Some(segment) = current.take() {
                if !options.allow_segments {
                    segment.finish_into_result(&mut result, sink);
                    push_result_diagnostic(
                        &mut result,
                        sink,
                        Diagnostic {
                            cause: None,
                            code: "SegmentBoundary".to_string(),
                            detail: format!(
                                "segment boundary at item {item_index} but reader is in \
                                 pre-segment mode; remainder of file NOT folded (folding \
                                 it with file-global term-ids would silently misfold — §16)"
                            ),
                            frame_index: Some(item_index),
                        }
                        .with_native_cause(FoldError::SegmentBoundary { item: item_index }),
                    );
                    return result;
                }
                segment.finish_into_result(&mut result, sink);
            } else if item_index != 0 {
                push_result_diagnostic(
                    &mut result,
                    sink,
                    Diagnostic {
                        cause: None,
                        code: "DamagedFrame".to_string(),
                        detail: "first item is not a header".to_string(),
                        frame_index: Some(0),
                    }
                    .with_native_cause(FoldError::FirstHeader),
                );
                return result;
            }
            current = Some(ActiveStreamingSegment::new(
                item,
                item_index,
                result.segment_heads.len() as u64,
                sink,
            ));
        } else if let Some(segment) = current.as_mut() {
            let StreamItem::Frame(item) = item else {
                unreachable!("headers are handled before frame dispatch");
            };
            if let Err(cause) = segment.process_frame(
                item.view(),
                item_index,
                frame_start,
                frame_end,
                sink,
                options.content_key,
            ) {
                result.torn = Some(frame_start);
                torn_cause = Some(TornAppendError {
                    offset: frame_start,
                    cause,
                });
                break;
            }
        } else {
            push_result_diagnostic(
                &mut result,
                sink,
                Diagnostic {
                    cause: None,
                    code: "DamagedFrame".to_string(),
                    detail: "first item is not a header".to_string(),
                    frame_index: Some(0),
                }
                .with_native_cause(FoldError::FirstHeader),
            );
            return result;
        }
        item_index = match item_index.checked_add(1) {
            Some(next) => next,
            None => {
                push_result_diagnostic(
                    &mut result,
                    sink,
                    Diagnostic {
                        cause: None,
                        code: "ResourceLimit".to_owned(),
                        detail: "GTS item ordinal exceeds u64::MAX".to_owned(),
                        frame_index: Some(item_index),
                    }
                    .with_native_cause(FoldError::ItemOrdinal { item: item_index }),
                );
                return result;
            }
        };
    }

    if item_index == 0 {
        let diagnostic = Diagnostic {
            cause: None,
            code: "EmptyFile".to_string(),
            detail: "no CBOR items".to_string(),
            frame_index: None,
        };
        push_result_diagnostic(
            &mut result,
            sink,
            match torn_cause {
                Some(error) => diagnostic.with_native_cause(error),
                None => diagnostic.with_native_cause(FoldError::EmptyFile),
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
                    cause: None,
                    code: "TruncatedLog".to_string(),
                    detail: "observed head does not match expected head".to_string(),
                    frame_index: None,
                }
                .with_native_cause(FoldError::ExpectedHead {
                    actual: last_head,
                    expected: expected.to_vec(),
                }),
            );
        }
    }
    if let Some(error) = torn_cause {
        push_result_diagnostic(
            &mut result,
            sink,
            Diagnostic {
                cause: None,
                code: "TornAppendError".to_string(),
                detail: error.to_string(),
                frame_index: None,
            }
            .with_native_cause(error),
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
    /// Original refusal at a torn trailing item (§3), if any.
    pub torn: Option<TornAppendError>,
    /// Set when the file never reaches segmentation (empty, or the first item
    /// is not a header) — `segments` is empty in that case.
    pub fatal: Option<Diagnostic>,
}

/// Fold a file segment-by-segment WITHOUT unioning — the composition ledger
/// view that `gts info`/`gts verify` report per-segment (§14.1).
pub fn read_file_segments(data: &[u8]) -> FileSegments {
    let (items, torn) = iter_items_with_error(data);
    if items.is_empty() {
        let diagnostic = Diagnostic {
            cause: None,
            code: "EmptyFile".to_string(),
            detail: "no CBOR items".to_string(),
            frame_index: None,
        };
        let diagnostic = match &torn {
            Some(error) => diagnostic.with_native_cause(error.clone()),
            None => diagnostic.with_native_cause(FoldError::EmptyFile),
        };
        return FileSegments {
            segments: Vec::new(),
            torn,
            fatal: Some(diagnostic),
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
            fatal: Some(
                Diagnostic {
                    cause: None,
                    code: "DamagedFrame".to_string(),
                    detail: "first item is not a header".to_string(),
                    frame_index: Some(0),
                }
                .with_native_cause(FoldError::FirstHeader),
            ),
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
            .map(|&(a, b)| read_segment(&items[a..b], a as u64, None))
            .collect()
    } else {
        ranges
            .iter()
            .map(|&(a, b)| read_segment(&items[a..b], a as u64, content_key))
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

fn read_segment(
    items: &[(usize, Value)],
    index_offset: u64,
    content_key: Option<&ContentKeyResolver<'_>>,
) -> Graph {
    let mut g = Graph::default();
    let mut sink = None;
    let (_, raw_header) = &items[0];
    let header = match unwrap_header(raw_header) {
        Ok(h) => h,
        Err(e) => {
            push_diagnostic(
                &mut g,
                &mut sink,
                Diagnostic {
                    cause: None,
                    code: "DamagedFrame".to_string(),
                    detail: format!("invalid header: {e}"),
                    frame_index: Some(index_offset),
                }
                .with_native_cause(e),
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
                cause: None,
                code: "DamagedFrame".to_string(),
                detail: "header self-hash mismatch".to_string(),
                frame_index: Some(index_offset),
            }
            .with_native_cause(FoldError::HeaderHash {
                stored: stored_hid.clone(),
                computed: header_id(header),
            }),
        );
    }
    if map_get(header, "gts").and_then(Value::as_text) != Some(MAGIC)
        || map_get(header, "v").and_then(as_i128) != Some(i128::from(VERSION))
    {
        push_diagnostic(
            &mut g,
            &mut sink,
            Diagnostic {
                cause: None,
                code: "DamagedFrame".to_string(),
                detail: format!(
                    "unsupported header magic/version {:?}/{:?}",
                    map_get(header, "gts"),
                    map_get(header, "v")
                ),
                frame_index: Some(index_offset),
            }
            .with_native_cause(FoldError::HeaderVersion {
                magic: map_get(header, "gts").cloned(),
                version: map_get(header, "v").cloned(),
            }),
        );
    }
    let mut expected_prev: Vec<u8> = stored_hid.unwrap_or_default();
    // per-frame chain ids, by 0-based frame position
    let mut frame_ids: Vec<Vec<u8>> = Vec::new();

    // §9.1 parallel verification: hash every frame's content id up front,
    // concurrently; the fold below then walks the chain sequentially with a
    // cheap `prev`-equality check per frame.
    let mut computed_ids = parallel_content_ids(&items[1..]);

    let (index_records, blob_events) = {
        let catalog = catalog_from(header);
        let mut folder = Folder {
            g: &mut g,
            shared_terms: None,
            sink: None,
            content_key,
            segment_index: 0,
            materialize: true,
            streamable: map_get(header, "layout").and_then(Value::as_text) == Some("streamable"),
            provenance: crate::compact::ProvenanceSubjects::default(),
            catalog,
            blob_index: DigestIndex::default(),
            blob_meta_index: DigestIndex::default(),
            index_records: Vec::new(),
            described: FastSet::default(),
            blob_events: Vec::new(),
            rebound_reifiers: Vec::new(),
            reifier_index: ReifierIndex::default(),
        };
        for (index, (_, raw)) in items[1..].iter().enumerate() {
            let abs_index = (index as u64)
                .checked_add(1)
                .and_then(|index| index.checked_add(index_offset))
                .expect("bounded file item count fits u64");
            let Value::Map(frame) = raw else {
                folder.diag(FoldError::FrameMap, abs_index);
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
                    FoldError::SelfHash {
                        stored: stored_id.cloned(),
                        computed: computed.clone(),
                    },
                    abs_index,
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
                    FoldError::Previous {
                        actual: map_get(frame, "prev")
                            .and_then(Value::as_bytes)
                            .map(<[u8]>::to_vec),
                        expected: expected_prev.clone(),
                    },
                    abs_index,
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
                    packaging: crate::compact::packaging_role(
                        &folder.provenance,
                        text_or(map_get(frame, "t"), ""),
                        folder.streamable,
                    ),
                });
            }
            folder.fold_frame(frame, abs_index, map_get(frame, "d").map(ValueRef::Owned));
        }
        folder.report_ambiguous_reifiers();
        (folder.index_records, folder.blob_events)
    };
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
    g.segment_streamable.push(info);
    g
}

#[cfg(test)]
mod transformed_payload_tests {
    use purrdf_lex::cbor::Value;

    use super::read;
    use crate::wire::canonical;
    use crate::writer::Writer;

    #[test]
    fn streaming_offsets_cross_u32_and_refuse_terminal_overflow() {
        use super::{StreamingCountingReader, next_stream_item};
        assert_eq!(
            size_of::<Result<Option<(Value, u64, u64)>, super::TornAppendError>>(),
            size_of::<Result<Option<(Value, u64, u64)>, u64>>()
        );
        let mut reader = StreamingCountingReader {
            inner: std::io::Cursor::new([1u8]),
            pos: (1 << 32) + 7,
            address_exhausted: None,
        };
        let (_, start, end) = next_stream_item(&mut reader).unwrap().unwrap();
        assert_eq!(start, (1 << 32) + 7);
        assert_eq!(end, (1 << 32) + 8);
        let mut reader = StreamingCountingReader {
            inner: std::io::Cursor::new([1u8]),
            pos: u64::MAX,
            address_exhausted: None,
        };
        assert_eq!(next_stream_item(&mut reader).unwrap_err().offset, u64::MAX);
        assert_eq!(reader.address_exhausted, Some((u64::MAX, 1)));
        assert_eq!(reader.pos, u64::MAX);
    }

    #[test]
    fn global_frame_range_refuses_addresses_outside_the_local_buffer() {
        let context = super::FrameContext {
            segment_index: 1 << 32,
            frame_index: 1 << 32,
            content_id: &[],
            range: super::ByteRange {
                start: (1 << 32) + 1,
                end: (1 << 32) + 2,
            },
            frame_type: "terms",
            valid: false,
        };
        assert!(!context.verify(&[1]));
        assert_eq!(context.range.len(), 1);
    }

    /// A file whose one `terms` frame carries `bytes` through the `gzip`
    /// transform, so the reader decodes them as the frame's payload item.
    fn gzip_terms_file(bytes: Vec<u8>) -> Vec<u8> {
        let mut writer = Writer::new("purrdf.gts");
        writer.add_frame(
            "terms",
            None,
            Some(bytes),
            Some(&["gzip".to_string()]),
            None,
        );
        writer.into_bytes()
    }

    fn one_iri_payload() -> Vec<u8> {
        canonical(&Value::Array(vec![Value::Map(vec![
            (Value::from("k"), Value::from(0_u8)),
            (Value::from("v"), Value::from("https://example.org/s")),
        ])]))
    }

    #[test]
    fn a_transformed_payload_that_is_exactly_one_item_folds() {
        let graph = read(&gzip_terms_file(one_iri_payload()), false, None);
        assert_eq!(graph.diagnostics, Vec::new());
        assert_eq!(graph.terms.len(), 1);
        assert_eq!(
            graph.terms[0].value.as_deref(),
            Some("https://example.org/s")
        );
    }

    #[test]
    fn bytes_after_a_transformed_payload_item_damage_the_frame() {
        let mut bytes = one_iri_payload();
        bytes.push(0x00);
        let graph = read(&gzip_terms_file(bytes), false, None);
        assert_eq!(graph.terms, Vec::new());
        assert!(
            graph
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "DamagedFrame"),
            "{:?}",
            graph.diagnostics
        );
    }
}
