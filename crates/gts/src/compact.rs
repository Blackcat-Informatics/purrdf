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

use crate::dict;
use crate::mmr;
use crate::model::{Graph, Quad, ReifierRow, Suppression, Term, TermKind};
use crate::reader::{read, read_file_segments};
use crate::stream;
use crate::wire::{blake3_256, digest_str, hex, map_get};
use crate::writer::{Writer, WriterOptions};

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DATETIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";

/// In-band pack dictionary name pinned in the header `"dct"` map (§5).
const DICT_NAME: &str = "pack";
/// Target size of the pinned pack dictionary. FastCOVER/raw-content truncate the
/// content to fit; a fixed value keeps compaction byte-deterministic.
const DICT_TARGET_LEN: usize = 16 * 1024;

/// Which in-band dictionary a pack pins for its `zstd` `dct` codec (§8.5).
///
/// Both are byte-deterministic; the trained strategy is the production default
/// (Req 3 headline is "trained"), and the raw-content strategy is a named
/// alternate. The choice affects only the pinned dictionary bytes, never the
/// fold — a reader decodes the in-band dictionary either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DictStrategy {
    /// FastCOVER-trained dictionary (the production default).
    Trained,
    /// Raw-content dictionary (canonical trailing window of the corpus).
    RawContent,
    /// No pack dictionary — plain `zstd` frames (used when there is no content
    /// blob corpus to train on).
    None,
}

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
    if let Value::Map(entries) = target
        && let Some(Value::Text(t)) = map_get(entries, key)
    {
        return Some(t);
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
            if let Value::Map(entries) = meta
                && let Some(Value::Text(t)) = map_get(entries, key)
            {
                return Some(t.clone());
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

/// Decode a base64url WITHOUT padding (RFC 4648 §5) string — the inverse of
/// [`base64url_unpadded`], used to recover a `stream:cose` literal's raw
/// COSE_Sign1 bytes (Task 5 signature verification).
///
/// # Errors
/// HARD-ERRORS on a padding character (`=`) or any byte outside the unpadded
/// alphabet (`A-Za-z0-9-_`) — a malformed literal must fail loudly rather than
/// silently decode to truncated or garbage bytes (refuse-don't-trust).
pub fn base64url_decode(s: &str) -> Result<Vec<u8>, String> {
    fn sextet(byte: u8) -> Result<u32, String> {
        match byte {
            b'A'..=b'Z' => Ok(u32::from(byte - b'A')),
            b'a'..=b'z' => Ok(u32::from(byte - b'a') + 26),
            b'0'..=b'9' => Ok(u32::from(byte - b'0') + 52),
            b'-' => Ok(62),
            b'_' => Ok(63),
            b'=' => Err("unexpected padding character '=' in unpadded base64url".to_string()),
            other => Err(format!(
                "byte {other:#04x} is outside the base64url (RFC 4648 §5) alphabet"
            )),
        }
    }
    let bytes = s.as_bytes();
    if bytes.len() % 4 == 1 {
        return Err(format!(
            "base64url input length {} leaves a single trailing character (invalid)",
            bytes.len()
        ));
    }
    let mut out = Vec::with_capacity((bytes.len() / 4 + 1) * 3);
    let mut chunks = bytes.chunks_exact(4);
    for chunk in &mut chunks {
        let n = (sextet(chunk[0])? << 18)
            | (sextet(chunk[1])? << 12)
            | (sextet(chunk[2])? << 6)
            | sextet(chunk[3])?;
        out.push((n >> 16) as u8);
        out.push((n >> 8) as u8);
        out.push(n as u8);
    }
    match chunks.remainder() {
        [] => {}
        [a, b] => {
            let n = (sextet(*a)? << 18) | (sextet(*b)? << 12);
            out.push((n >> 16) as u8);
        }
        [a, b, c] => {
            let n = (sextet(*a)? << 18) | (sextet(*b)? << 12) | (sextet(*c)? << 6);
            out.push((n >> 16) as u8);
            out.push((n >> 8) as u8);
        }
        _ => unreachable!("chunks_exact(4) remainder is always shorter than 4"),
    }
    Ok(out)
}

/// Whether `g` is itself a previously-compacted pack — carries a
/// `stream:Compaction` provenance node — as opposed to a raw authored tail.
///
/// A pack's `g.signatures` (the raw per-frame `"sig"` observations folded by
/// the reader) contains ONLY the mandatory packaging head signature on the
/// re-issued index footer (§10.1: `compact_streamable` never re-signs a
/// carried content frame). Distinguishing a repack input from a raw tail lets
/// [`sorted_detached_pairs`] exclude that packaging observation from the
/// authorship union — it must never leak into the detached-authorship root.
fn is_repack_input(g: &Graph) -> bool {
    let Some(rdf_type) = g
        .terms
        .iter()
        .position(|t| t.value.as_deref() == Some(RDF_TYPE))
    else {
        return false;
    };
    let Some(compaction) = g
        .terms
        .iter()
        .position(|t| t.value.as_deref() == Some(stream::COMPACTION))
    else {
        return false;
    };
    g.quads
        .iter()
        .any(|&(_, p, o, _)| p == rdf_type && o == compaction)
}

/// The literal object value(s) of every quad `(subject, predicate_iri, ?)` in
/// `g`, for every `subject` typed `stream:DetachedSignature`.
///
/// Parses the input graph's OWN carried `stream:DetachedSignature` provenance
/// nodes back into `(frame_id, cose)` byte pairs — the authorship signatures
/// accumulated by any PRIOR compaction(s), so a repack's detached root keeps
/// binding the ORIGINAL author frame sigs, not just whatever
/// `compact_streamable` observed fresh on this input. A node missing either
/// literal, or carrying one that fails to decode, is skipped — a malformed
/// provenance shape is unresolvable, not a license to guess (refuse-don't-trust,
/// mirrored from `purrdf_rdf::gts_certify`'s suppression-target resolution).
fn carried_detached_pairs(g: &Graph) -> Vec<(Vec<u8>, Vec<u8>)> {
    let Some(rdf_type) = g
        .terms
        .iter()
        .position(|t| t.value.as_deref() == Some(RDF_TYPE))
    else {
        return Vec::new();
    };
    let Some(detached_class) = g
        .terms
        .iter()
        .position(|t| t.value.as_deref() == Some(stream::DETACHED_SIGNATURE))
    else {
        return Vec::new();
    };
    let Some(source_frame_pred) = g
        .terms
        .iter()
        .position(|t| t.value.as_deref() == Some(stream::SOURCE_FRAME))
    else {
        return Vec::new();
    };
    let Some(cose_pred) = g
        .terms
        .iter()
        .position(|t| t.value.as_deref() == Some(stream::COSE))
    else {
        return Vec::new();
    };
    let nodes = g
        .quads
        .iter()
        .filter(|&&(_, p, o, _)| p == rdf_type && o == detached_class)
        .map(|&(s, _, _, _)| s);
    let mut out = Vec::new();
    for node in nodes {
        let frame_lit = g
            .quads
            .iter()
            .find(|&&(s, p, _, _)| s == node && p == source_frame_pred)
            .and_then(|&(_, _, o, _)| g.terms.get(o))
            .and_then(|t| t.value.as_deref());
        let cose_lit = g
            .quads
            .iter()
            .find(|&&(s, p, _, _)| s == node && p == cose_pred)
            .and_then(|&(_, _, o, _)| g.terms.get(o))
            .and_then(|t| t.value.as_deref());
        let (Some(frame_lit), Some(cose_lit)) = (frame_lit, cose_lit) else {
            continue;
        };
        let (Ok(frame_id), Ok(cose)) = (mmr::parse_hex_32(frame_lit), base64url_decode(cose_lit))
        else {
            continue;
        };
        out.push((frame_id, cose));
    }
    out
}

/// Sorted, deduplicated `(frame_id, cose)` pairs over every detached
/// AUTHORSHIP signature in `g` — the union of:
///  - the fresh per-frame COSE folded onto `g.signatures` when `g` is a raw
///    authored tail (never a repack input — see [`is_repack_input`], which
///    excludes a pack's own mandatory packaging observation), and
///  - the carried `stream:DetachedSignature` provenance already present in
///    `g` (accumulated by any prior compaction — see
///    [`carried_detached_pairs`]).
///
/// A frame may carry multiple co-signatures under key rotation, so `frame_id`
/// alone is not a unique key — the `cose` tie-break is required for a stable,
/// deterministic leaf order. The union is deduplicated so an identical pair
/// surviving both sources (impossible today, but not an invariant this
/// function should assume) contributes exactly one leaf.
fn sorted_detached_pairs(g: &Graph) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut pairs: Vec<(Vec<u8>, Vec<u8>)> = if is_repack_input(g) {
        Vec::new()
    } else {
        g.signatures
            .iter()
            .filter_map(|s| {
                s.cose
                    .as_deref()
                    .map(|cose| (s.frame_id.clone(), cose.to_vec()))
            })
            .collect()
    };
    pairs.extend(carried_detached_pairs(g));
    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

/// The MMR leaves committed by `stream:detachedSignatureRoot`: one
/// `blake3(frame_id || cose)` hash per detached signature, in sorted
/// `(frame_id, cose)` order (§10.1 signature preservation).
///
/// Public so a certificate consumer (GTS-SPEC §10.2) can independently derive
/// the same leaf set and prove membership without re-deriving the sort.
pub fn detached_signature_leaves(g: &Graph) -> Vec<Vec<u8>> {
    sorted_detached_pairs(g)
        .into_iter()
        .map(|(frame_id, cose)| detached_signature_leaf(&frame_id, &cose))
        .collect()
}

/// `blake3(frame_id || cose)` — the leaf preimage for one detached signature.
fn detached_signature_leaf(frame_id: &[u8], cose: &[u8]) -> Vec<u8> {
    let mut preimage = Vec::with_capacity(frame_id.len() + cose.len());
    preimage.extend_from_slice(frame_id);
    preimage.extend_from_slice(cose);
    blake3_256(&preimage)
}

/// A selective per-frame authorship proof: the detached inclusion proof for
/// one `(frame_id, cose)` leaf under [`detached_signature_leaves`]'s root.
///
/// Returns `None` when no detached signature matches `(frame_id, cose)`.
pub fn detached_signature_proof(g: &Graph, frame_id: &[u8], cose: &[u8]) -> Option<mmr::Proof> {
    let pairs = sorted_detached_pairs(g);
    let leaf_index = pairs
        .iter()
        .position(|(f, c)| f.as_slice() == frame_id && c.as_slice() == cose)?;
    let leaves: Vec<Vec<u8>> = pairs
        .into_iter()
        .map(|(f, c)| detached_signature_leaf(&f, &c))
        .collect();
    mmr::prove(&leaves, leaf_index)
}

/// Build the leading streaming index + compaction provenance (§3.3, §13.3).
fn streaming_index(
    g: &Graph,
    blob_order: &[String],
    timestamp: &str,
    sealed_digest: Option<&str>,
    sealed_size: Option<usize>,
    content_digest: Option<&str>,
) -> Result<GraphBuilder, CompactRefusedError> {
    // The detached-authorship union (§10.1): fresh frame COSE from
    // `g.signatures` (when `g` is a raw tail — never a repack's own
    // packaging observation) UNIONED with the carried `stream:DetachedSignature`
    // provenance already present in `g` (accumulated by any prior
    // compaction). Computed once, up front, so the boolean drives the fixed
    // vocabulary block's id assignment identically to the pairs used below.
    let detached_pairs = sorted_detached_pairs(g);

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
    // The content-refold digest term rides the fixed block only when embedded,
    // mirroring how the sealed-source quad is emitted conditionally.
    let t_content_digest =
        content_digest.map(|_| b.add(TermKind::Iri, stream::CONTENT_REFOLD_DIGEST));
    // The detached-signature root term rides the fixed block only when the
    // union carries at least one detached signature (empty set ⇒ no quad,
    // keeping `signatures_bound` vacuously true for unsigned tails).
    let t_detached_root =
        (!detached_pairs.is_empty()).then(|| b.add(TermKind::Iri, stream::DETACHED_SIGNATURE_ROOT));

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
    // Proof-carrying pack: embed the RDFC-1.0 content digest so a repack
    // certifies without the pre-compaction bytes. Excluded from the content
    // projection at verification time, so it cannot perturb the equivalence.
    if let (Some(t_cd), Some(digest)) = (t_content_digest, content_digest) {
        let o = b.literal(digest, None);
        b.quad(c, t_cd, o);
    }

    // Detached frame signatures (§10.1): checkable claims about the original
    // log — the FULL authorship union, so a re-emitted pack carries forward
    // every original author signature across any number of repacks, not just
    // the ones freshly observed on this input.
    for (j, (frame_id, cose)) in detached_pairs.iter().enumerate() {
        let node = b.add(TermKind::Bnode, &format!("s{j}"));
        let cose_b64 = base64url_unpadded(cose);
        b.quad(node, t_type, t_detached_sig);
        let o = b.literal(&format!("blake3:{}", hex(frame_id)), None);
        b.quad(node, t_source_frame, o);
        let o = b.literal(&cose_b64, None);
        b.quad(node, t_cose, o);
    }

    // One MMR root binding the whole detached-signature set under a single
    // commitment (§10.1 signature preservation) — omitted entirely when the
    // set is empty (no zero-count root emitted for an unsigned tail).
    if let Some(t_root) = t_detached_root {
        let leaves: Vec<Vec<u8>> = detached_pairs
            .iter()
            .map(|(frame_id, cose)| detached_signature_leaf(frame_id, cose))
            .collect();
        let root = mmr::root(&leaves);
        let o = b.literal(&format!("blake3:{}", hex(&root)), None);
        b.quad(c, t_root, o);
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
                    } else if kind == "quad"
                        && key == "q"
                        && let Value::Array(ids) = v
                    {
                        let remapped: Vec<Value> = ids
                            .iter()
                            .map(|x| match value_idx(x) {
                                Some(tid) => Value::from((tid + base) as u64),
                                None => x.clone(),
                            })
                            .collect();
                        return (k.clone(), Value::Array(remapped));
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

/// Build the in-band pack dictionary over the batched content-blob corpus.
///
/// The corpus is every content blob's decoded bytes (the sealed original — the
/// whole source log — is excluded, it is not "content"). A pack with no content
/// blobs has no corpus, so it pins no dictionary and its frames use plain `zstd`.
/// The producers are order-independent, so the emitted `dct` bytes equal
/// `trained_dict`/`raw_content_dict` of the corpus regardless of iteration order.
fn build_pack_dict(
    g: &Graph,
    blob_order: &[String],
    sealed_digest: Option<&str>,
    strategy: DictStrategy,
) -> Result<Option<(String, Vec<u8>)>, CompactRefusedError> {
    if strategy == DictStrategy::None {
        return Ok(None);
    }
    let mut corpus: Vec<Vec<u8>> = Vec::new();
    for digest in blob_order {
        if Some(digest.as_str()) == sealed_digest {
            continue;
        }
        if let Some(bytes) = blob_bytes(g, digest)? {
            corpus.push(bytes.into_owned());
        }
    }
    if corpus.is_empty() {
        return Ok(None);
    }
    let refs: Vec<&[u8]> = corpus.iter().map(Vec::as_slice).collect();
    let bytes = match strategy {
        DictStrategy::Trained => dict::trained_dict(&refs, DICT_TARGET_LEN),
        DictStrategy::RawContent => dict::raw_content_dict(&refs, DICT_TARGET_LEN),
        DictStrategy::None => unreachable!("handled above"),
    }
    .map_err(|err| CompactRefusedError(format!("cannot build the pack dictionary: {err}")))?;
    Ok(Some((DICT_NAME.to_string(), bytes)))
}

/// Parameters for [`compact_streamable`].
// `SigningKey`'s `Debug` impl redacts the secret scalar, so deriving is safe here.
#[derive(Debug)]
pub struct CompactionParams<'a> {
    /// The rewrite time recorded as `stream:timestamp` — an explicit
    /// parameter so the output is byte-reproducible.
    pub timestamp: &'a str,
    /// Carry the verbatim source bytes as a nested GTS blob (§12.1), role
    /// `"source"` — REQUIRED for `evidence` input.
    pub seal_original: bool,
    /// The in-band pack dictionary strategy trained over the batched
    /// content-blob corpus and pinned per pack ([`DictStrategy::Trained`] is
    /// the production default).
    pub strategy: DictStrategy,
    /// When supplied by the certifying authoring wrapper, embedded as
    /// `stream:contentRefoldDigest` provenance so a repack certifies without
    /// the pre-compaction bytes.
    pub content_digest: Option<&'a str>,
    /// `(key, kid)` used to sign ONLY the re-issued ordering commitment (the
    /// trailing `index` footer) — the MANDATORY packaging signature attesting
    /// ordering/packaging, never frame authorship. Carried detached
    /// authorship signatures live in provenance quads instead (§10.1); this
    /// is a distinct, separately-verifiable claim.
    ///
    /// REQUIRED, not optional: R1 makes the packaging head signature
    /// mandatory, so a pack with no packaging signature must be
    /// unrepresentable through this API rather than merely discouraged — the
    /// field is a plain tuple, not an `Option`, precisely so an unsigned pack
    /// cannot be constructed by a caller that forgets to supply a signer.
    pub packaging_signer: (ed25519_dalek::SigningKey, String),
}

/// Rewrite a GTS file into one streamable segment (§10.1).
///
/// `data` must verify cleanly (refuse-don't-trust). See [`CompactionParams`]
/// for the rewrite parameters.
///
/// # Errors
/// Returns [`CompactRefusedError`] when the input is not safely compactable, a
/// blob cannot be decoded, the pack dictionary cannot be built, or the writer
/// rejects the configuration.
pub fn compact_streamable(
    data: &[u8],
    params: CompactionParams<'_>,
) -> Result<Vec<u8>, CompactRefusedError> {
    let CompactionParams {
        timestamp,
        seal_original,
        strategy,
        content_digest,
        packaging_signer,
    } = params;
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
    let sealed_digest: Option<String> = if seal_original {
        let sealed = digest_str(data);
        blob_order.retain(|d| *d != sealed);
        blob_order.push(sealed.clone());
        Some(sealed)
    } else {
        None
    };

    // Pack dictionary trained over the batched content-blob corpus (the sealed
    // original — the whole source — is excluded). The dict producers are
    // order-independent, so blob-delivery order need not be threaded here.
    let dict = build_pack_dict(&g, &blob_order, sealed_digest.as_deref(), strategy)?;

    let index = streaming_index(
        &g,
        &blob_order,
        timestamp,
        sealed_digest.as_deref(),
        sealed_digest.as_ref().map(|_| data.len()),
        content_digest,
    )?;
    let base = index.terms.len();

    let mut w = Writer::with_options(
        &profile,
        WriterOptions {
            layout: Some("streamable".to_string()),
            dict,
            ..WriterOptions::default()
        },
    )
    .map_err(|err| CompactRefusedError(format!("cannot configure the pack writer: {err}")))?;
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
    // Blobs in delivery order; declared metadata rides along. When a pack
    // dictionary is pinned, content-blob frames are re-emitted through the
    // `zstd` transform so they are actually compressed against `dict` above —
    // an unused in-band dictionary would otherwise be dead weight. The sealed
    // original (the nested source GTS) is never dict-compressed: it carries
    // its own framing and is excluded from the dictionary training corpus.
    let content_transform: Vec<String> = if strategy == DictStrategy::None {
        Vec::new()
    } else {
        vec!["zstd".to_string()]
    };
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
        let owned = match bytes {
            Cow::Borrowed(bytes) => bytes.to_vec(),
            Cow::Owned(bytes) => bytes,
        };
        if content_transform.is_empty() {
            w.add_blob_owned(owned, mt.as_deref(), rep.as_deref());
        } else {
            w.add_blob_transformed(owned, mt.as_deref(), rep.as_deref(), &content_transform);
        }
    }
    // The MANDATORY packaging head signature: sign ONLY the re-issued
    // ordering commitment below, never the frames already appended above.
    // This attests ordering/packaging — the compactor is its sole attester —
    // distinct from the carried detached authorship signatures (§10.1).
    // `packaging_signer` is a required field (not `Option`), so this always
    // runs: an unsigned pack is unrepresentable through this API.
    let (key, kid) = packaging_signer;
    w.sign_with(key, &kid);
    w.add_index();
    Ok(w.into_bytes())
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::SigningKey;

    use super::*;
    use crate::reader::read;

    /// A fixed, deterministic Ed25519 signing key (RFC 8032 signing is
    /// deterministic per key + message, so tests stay byte-reproducible) —
    /// mirrors `crates/gts/tests/compaction_signatures.rs::fixed_key`.
    fn fixed_key(byte: u8) -> SigningKey {
        SigningKey::from_bytes(&[byte; 32])
    }

    /// A source GTS file whose content blobs share structure — the corpus a
    /// pack dictionary trains on.
    fn source_with_blobs() -> Vec<u8> {
        let mut w = Writer::new("purrdf.gts");
        for i in 0..64u32 {
            let blob = format!(
                "<https://example.org/s{}> <https://example.org/p> \"claim {} about cats\" .\n",
                i % 37,
                i
            )
            .into_bytes();
            w.add_blob_owned(blob, Some("text/plain"), None);
        }
        w.into_bytes()
    }

    fn digest_quad_present(bytes: &[u8]) -> bool {
        let g = read(bytes, true, None);
        g.terms
            .iter()
            .any(|t| t.value.as_deref() == Some(stream::CONTENT_REFOLD_DIGEST))
    }

    /// A `CompactionParams` with the shared test defaults: fixed timestamp, no
    /// source seal, a fixed packaging signer (the field is mandatory — see
    /// [`CompactionParams::packaging_signer`]).
    fn params(strategy: DictStrategy, content_digest: Option<&str>) -> CompactionParams<'_> {
        CompactionParams {
            timestamp: "2026-01-01T00:00:00Z",
            seal_original: false,
            strategy,
            content_digest,
            packaging_signer: (fixed_key(99), "pack-test".to_string()),
        }
    }

    #[test]
    fn compaction_is_byte_deterministic_with_a_trained_dict() {
        let source = source_with_blobs();
        let a = compact_streamable(&source, params(DictStrategy::Trained, None))
            .expect("compaction succeeds");
        let b = compact_streamable(&source, params(DictStrategy::Trained, None))
            .expect("compaction succeeds");
        assert_eq!(a, b, "a trained-dict compaction must be byte-reproducible");
    }

    #[test]
    fn compacted_pack_folds_cleanly_and_blobs_decode_through_the_dict() {
        let source = source_with_blobs();
        let packed = compact_streamable(&source, params(DictStrategy::Trained, None))
            .expect("compaction succeeds");
        let g = read(&packed, true, None);
        assert!(
            g.diagnostics.is_empty(),
            "compacted pack must fold cleanly (dict resolves): {:?}",
            g.diagnostics
        );
        assert_eq!(g.blobs.len(), 64, "every content blob survives the repack");
        for (_, entry) in &g.blobs {
            entry
                .decoded_vec()
                .expect("dict-compressed blob must decode against the pinned in-band dictionary");
        }
    }

    #[test]
    fn the_dictionary_is_invisible_to_the_fold() {
        let source = source_with_blobs();
        let trained = compact_streamable(&source, params(DictStrategy::Trained, None))
            .expect("trained compaction");
        let undicted = compact_streamable(&source, params(DictStrategy::None, None))
            .expect("undicted compaction");
        assert_ne!(
            trained, undicted,
            "pinning a dictionary changes the header bytes"
        );
        let a = read(&trained, true, None);
        let b = read(&undicted, true, None);
        let a_blobs: Vec<Vec<u8>> = a
            .blobs
            .iter()
            .map(|(_, e)| e.decoded_vec().unwrap())
            .collect();
        let b_blobs: Vec<Vec<u8>> = b
            .blobs
            .iter()
            .map(|(_, e)| e.decoded_vec().unwrap())
            .collect();
        assert_eq!(
            a_blobs, b_blobs,
            "the pack dictionary is a compression detail, invisible to the fold"
        );
    }

    #[test]
    fn embedded_content_digest_appears_only_when_supplied() {
        let source = source_with_blobs();
        let without =
            compact_streamable(&source, params(DictStrategy::Trained, None)).expect("compaction");
        assert!(
            !digest_quad_present(&without),
            "no content-refold digest without one supplied"
        );
        let with = compact_streamable(
            &source,
            params(DictStrategy::Trained, Some("blake3:0123456789abcdef")),
        )
        .expect("compaction");
        assert!(
            digest_quad_present(&with),
            "the supplied content-refold digest must be embedded as provenance"
        );
    }

    #[test]
    fn base64url_round_trips_through_the_encoder_for_every_remainder_length() {
        for len in 0..=17usize {
            let data: Vec<u8> = (0..len).map(|i| (i * 37 + 5) as u8).collect();
            let encoded = base64url_unpadded(&data);
            let decoded = base64url_decode(&encoded)
                .unwrap_or_else(|err| panic!("length {len} round trip must decode: {err}"));
            assert_eq!(decoded, data, "length {len} round trip must be lossless");
        }
    }

    // -----------------------------------------------------------------
    // Task 6, Part A — adversarial `shifted_suppressions`
    // coverage across all five suppress-target kinds (GTS-SPEC §11).
    // -----------------------------------------------------------------

    fn iri(v: &str) -> Term {
        Term {
            kind: TermKind::Iri,
            value: Some(v.to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        }
    }

    fn bnode(v: &str) -> Term {
        Term {
            kind: TermKind::Bnode,
            value: Some(v.to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        }
    }

    /// Build a `suppress-target` map: `{"kind": kind, ...extra}`.
    fn target(kind: &str, extra: Vec<(&str, Value)>) -> Value {
        let mut entries: Vec<(Value, Value)> = vec![("kind".into(), kind.into())];
        entries.extend(extra.into_iter().map(|(k, v)| (k.into(), v)));
        Value::Map(entries)
    }

    fn target_id(t: &Value) -> Option<usize> {
        let Value::Map(entries) = t else { return None };
        value_idx(map_get(entries, "id")?)
    }

    fn target_q(t: &Value) -> Option<Vec<Value>> {
        let Value::Map(entries) = t else { return None };
        match map_get(entries, "q")? {
            Value::Array(items) => Some(items.clone()),
            _ => None,
        }
    }

    fn target_digest(t: &Value) -> Option<String> {
        let Value::Map(entries) = t else { return None };
        match map_get(entries, "digest")? {
            Value::Text(s) => Some(s.clone()),
            _ => None,
        }
    }

    fn find_term_id(g: &Graph, value: &str) -> Option<usize> {
        g.terms
            .iter()
            .position(|t| t.value.as_deref() == Some(value))
    }

    #[test]
    fn term_suppression_is_carried_forward_value_wise_and_non_dangling() {
        let mut w = Writer::new("generic");
        w.add_terms(&[
            iri("https://example.org/s"), // 0
            iri("https://example.org/p"), // 1
            iri("https://example.org/o"), // 2 — the suppressed term
        ]);
        w.add_quads(&[(0, 1, 2, None)]);
        w.add_suppress(
            vec![target("term", vec![("id", Value::from(2u64))])],
            Some("pii"),
            None,
        );
        let source = w.into_bytes();

        let packed = compact_streamable(&source, params(DictStrategy::None, None))
            .expect("compaction succeeds");
        let g = read(&packed, true, None);
        assert!(
            g.diagnostics.is_empty(),
            "pack must fold cleanly: {:?}",
            g.diagnostics
        );

        // The suppressed term's VALUE is retained — suppression is a display
        // overlay, never a deletion (GTS-SPEC §11).
        let oid =
            find_term_id(&g, "https://example.org/o").expect("suppressed term value retained");
        // The quad it appeared in is likewise retained verbatim.
        let sid = find_term_id(&g, "https://example.org/s").expect("subject retained");
        let pid = find_term_id(&g, "https://example.org/p").expect("predicate retained");
        assert!(
            g.quads.contains(&(sid, pid, oid, None)),
            "the quad naming the suppressed term must still be present (never deleted)"
        );

        let sup = g
            .suppressions
            .iter()
            .find(|s| {
                s.targets
                    .iter()
                    .any(|t| target_text(t, "kind") == Some("term"))
            })
            .expect("term suppression carried forward into the pack");
        let t = sup
            .targets
            .iter()
            .find(|t| target_text(t, "kind") == Some("term"))
            .unwrap();
        assert_eq!(
            target_id(t),
            Some(oid),
            "the carried term suppression must resolve to the SAME term value in the pack"
        );
    }

    #[test]
    fn quad_suppression_is_carried_forward_value_wise_and_non_dangling() {
        let mut w = Writer::new("generic");
        w.add_terms(&[
            iri("https://example.org/s2"), // 0
            iri("https://example.org/p2"), // 1
            iri("https://example.org/o2"), // 2
        ]);
        w.add_quads(&[(0, 1, 2, None)]);
        w.add_suppress(
            vec![target(
                "quad",
                vec![(
                    "q",
                    Value::Array(vec![
                        Value::from(0u64),
                        Value::from(1u64),
                        Value::from(2u64),
                    ]),
                )],
            )],
            None,
            None,
        );
        let source = w.into_bytes();

        let packed = compact_streamable(&source, params(DictStrategy::None, None))
            .expect("compaction succeeds");
        let g = read(&packed, true, None);
        assert!(
            g.diagnostics.is_empty(),
            "pack must fold cleanly: {:?}",
            g.diagnostics
        );

        let sid = find_term_id(&g, "https://example.org/s2").expect("subject retained");
        let pid = find_term_id(&g, "https://example.org/p2").expect("predicate retained");
        let oid = find_term_id(&g, "https://example.org/o2").expect("object retained");
        assert!(
            g.quads.contains(&(sid, pid, oid, None)),
            "the targeted quad must still be present (never deleted)"
        );

        let sup = g
            .suppressions
            .iter()
            .find(|s| {
                s.targets
                    .iter()
                    .any(|t| target_text(t, "kind") == Some("quad"))
            })
            .expect("quad suppression carried forward into the pack");
        let t = sup
            .targets
            .iter()
            .find(|t| target_text(t, "kind") == Some("quad"))
            .unwrap();
        let q = target_q(t).expect("quad target carries a \"q\" array");
        let ids: Vec<usize> = q
            .iter()
            .map(|v| value_idx(v).expect("q element is an id"))
            .collect();
        assert_eq!(
            ids,
            vec![sid, pid, oid],
            "the carried quad suppression must resolve to the SAME (s,p,o) values in the pack"
        );
    }

    #[test]
    fn reifier_suppression_is_carried_forward_value_wise_and_non_dangling() {
        let mut w = Writer::new("generic");
        w.add_terms(&[
            iri("https://example.org/s3"), // 0
            iri("https://example.org/p3"), // 1
            iri("https://example.org/o3"), // 2
            bnode("rf1"),                  // 3 — the reifier
        ]);
        w.add_quads(&[(0, 1, 2, None)]);
        w.add_reifies(&[(3, (0, 1, 2), None)]);
        w.add_suppress(
            vec![target("reifier", vec![("id", Value::from(3u64))])],
            None,
            None,
        );
        let source = w.into_bytes();

        let packed = compact_streamable(&source, params(DictStrategy::None, None))
            .expect("compaction succeeds");
        let g = read(&packed, true, None);
        assert!(
            g.diagnostics.is_empty(),
            "pack must fold cleanly: {:?}",
            g.diagnostics
        );

        let sid = find_term_id(&g, "https://example.org/s3").expect("subject retained");
        let pid = find_term_id(&g, "https://example.org/p3").expect("predicate retained");
        let oid = find_term_id(&g, "https://example.org/o3").expect("object retained");
        let rid = g
            .reifiers
            .iter()
            .find(|&&(_, spo, _)| spo == (sid, pid, oid))
            .map(|&(r, _, _)| r)
            .expect("the reifier binding must still be present (never deleted)");

        let sup = g
            .suppressions
            .iter()
            .find(|s| {
                s.targets
                    .iter()
                    .any(|t| target_text(t, "kind") == Some("reifier"))
            })
            .expect("reifier suppression carried forward into the pack");
        let t = sup
            .targets
            .iter()
            .find(|t| target_text(t, "kind") == Some("reifier"))
            .unwrap();
        assert_eq!(
            target_id(t),
            Some(rid),
            "the carried reifier suppression must resolve to the SAME reifier binding in the pack"
        );
    }

    #[test]
    fn blob_suppression_is_carried_verbatim_and_the_blob_bytes_are_retained() {
        let mut w = Writer::new("generic");
        let data = b"classified cat photograph".to_vec();
        let digest = digest_str(&data);
        w.add_blob_owned(data.clone(), Some("text/plain"), None);
        w.add_suppress(
            vec![target("blob", vec![("digest", digest.clone().into())])],
            Some("classified"),
            None,
        );
        let source = w.into_bytes();

        let packed = compact_streamable(&source, params(DictStrategy::None, None))
            .expect("compaction succeeds");
        let g = read(&packed, true, None);
        assert!(
            g.diagnostics.is_empty(),
            "pack must fold cleanly: {:?}",
            g.diagnostics
        );

        // The suppressed blob's bytes are PRESENT — suppression hides, it
        // never deletes.
        let (_, entry) = g
            .blobs
            .iter()
            .find(|(d, _)| *d == digest)
            .expect("the suppressed blob is retained in the pack, not deleted");
        assert_eq!(
            entry.decoded_vec().expect("blob decodes"),
            data,
            "the retained blob bytes must be byte-identical to the source"
        );

        let sup = g
            .suppressions
            .iter()
            .find(|s| {
                s.targets
                    .iter()
                    .any(|t| target_text(t, "kind") == Some("blob"))
            })
            .expect("blob suppression carried forward into the pack");
        let t = sup
            .targets
            .iter()
            .find(|t| target_text(t, "kind") == Some("blob"))
            .unwrap();
        assert_eq!(
            target_digest(t),
            Some(digest),
            "a blob suppression's digest must be carried verbatim (content-addressing is \
             layout-independent)"
        );
    }

    #[test]
    fn frame_suppression_refuses_compaction() {
        let mut w = Writer::new("generic");
        w.add_terms(&[iri("https://example.org/s4")]);
        w.add_suppress(
            vec![target("frame", vec![("id", Value::Bytes(vec![7u8; 32]))])],
            None,
            None,
        );
        let source = w.into_bytes();

        let err = compact_streamable(&source, params(DictStrategy::None, None))
            .expect_err("a frame-addressed suppression must refuse compaction (§10.1)");
        assert!(
            err.to_string().contains("frame-addressed suppression"),
            "refusal message should name the cause: {err}"
        );
    }

    #[test]
    fn base64url_decode_rejects_padding_and_out_of_alphabet_bytes() {
        assert!(
            base64url_decode("aGVsbG8=").is_err(),
            "a trailing '=' padding character must be rejected"
        );
        assert!(
            base64url_decode("+++=").is_err(),
            "standard-alphabet '+' is not in the base64url alphabet"
        );
        assert!(
            base64url_decode("a/b/").is_err(),
            "standard-alphabet '/' is not in the base64url alphabet"
        );
        assert!(
            base64url_decode("a").is_err(),
            "a single trailing character cannot decode to a whole byte"
        );
    }
}
