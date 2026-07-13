// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! GTS streamable-compaction certificates (GTS-SPEC §10.1/§10.2, Task 5).
//!
//! A [`CompactionCertificate`] is the correctness-critical proof that a
//! streamable compaction (`purrdf_gts::compact::compact_streamable`) preserved
//! the *content* of a GTS log while re-authoring only its ordering and
//! packaging. The certificate binds a **content projection** — the folded
//! graph with every compaction-provenance quad removed — so a pre-compaction
//! log and its post-compaction pack canonicalize to byte-identical RDFC-1.0
//! N-Quads whenever the compaction was faithful.
//!
//! Three entry points:
//! - [`refold_digest`] / [`content_projection`]: the projection + digest that
//!   both the certifying authoring wrapper and independent verifiers compute.
//! - [`verify_compaction`]: an independent, from-bytes check of a claimed
//!   compaction against a verifier's own keyring — never trusts the pack's own
//!   embedded digest without recomputing it from the *pre* bytes too.
//! - [`compact_and_certify`]: the certifying authoring wrapper — folds,
//!   computes the pre digest (poison-guarded), compacts, and certifies that
//!   the post digest agrees.
//!
//! [`compose`] chains two certificates end-to-end (`A→B`, `B→C` ⇒ `A→C`) for
//! multi-hop compaction pipelines, and [`CompactionCertificate`]'s canonical
//! CBOR round-trips so a certificate can be carried and replayed independent
//! of this crate's in-memory shape.

use std::collections::{HashMap, HashSet};

use ciborium::value::{Integer, Value};
use ed25519_dalek::{SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

use purrdf_gts::compact::{self, CompactionParams, DictStrategy};
use purrdf_gts::cose::{self, SigStatus};
use purrdf_gts::model::{Graph, Suppression};
use purrdf_gts::reader::read;
use purrdf_gts::stream;
use purrdf_gts::verify::verify_file_with_keyring;
use purrdf_gts::wire;

use crate::gts::dataset_from_gts_graph;
use crate::gts_core::diagnostics_to_error;
use crate::{BudgetExceeded, CanonHash, RdfDiagnostic, canonicalize_with, try_canonicalize_with};

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// Blank-node count above which [`refold_digest`] refuses to canonicalize the
/// content projection (a cheap structural pre-reject; RDFC-1.0's n-degree
/// search is combinatorial in pathologically symmetric blank graphs, and a
/// certificate/verification path must never be a resource-exhaustion vector).
pub const POISON_BLANK_LIMIT: usize = 100_000;

/// Errors from certificate authoring, verification, and (de)serialization.
#[derive(Debug)]
pub enum CertifyError {
    /// The compaction pipeline refused the input (`purrdf_gts::compact::CompactRefusedError`).
    Refused(String),
    /// The GTS→dataset bridge, or a fold, reported a diagnostic.
    Dataset(RdfDiagnostic),
    /// The content projection's blank-node count exceeds [`POISON_BLANK_LIMIT`]
    /// — refused rather than risking a non-terminating RDFC-1.0 run.
    Poison(usize),
    /// An UNTRUSTED (verification-path) canonicalization exceeded RDFC-1.0's
    /// n-degree search call budget — a pathologically symmetric blank graph
    /// that [`Self::Poison`]'s cheap blank-node-COUNT pre-reject did not catch
    /// (a small but highly symmetric graph can blow the call budget while
    /// staying far under [`POISON_BLANK_LIMIT`]). [`verify_compaction`] routes
    /// through the fallible canonicalizer and surfaces this instead of letting
    /// the RDFC-1.0 engine panic on adversarial verifier input.
    CanonBudgetExceeded(BudgetExceeded),
    /// The certificate's canonical CBOR encoding is malformed.
    Cbor(String),
    /// An internal invariant was violated (e.g. a freshly authored
    /// compaction's post refold digest disagreeing with the pre digest it was
    /// authored from — a compaction-authoring bug, never a caller error).
    Invariant(String),
}

impl std::fmt::Display for CertifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(msg) => write!(f, "compaction refused: {msg}"),
            Self::Dataset(diag) => write!(f, "GTS dataset bridge failed: {diag}"),
            Self::Poison(count) => write!(
                f,
                "refusing to canonicalize a content projection with {count} blank node(s) \
                 (exceeds the {POISON_BLANK_LIMIT} poison guard)"
            ),
            Self::CanonBudgetExceeded(err) => write!(f, "compaction verification failed: {err}"),
            Self::Cbor(msg) => write!(f, "compaction certificate CBOR error: {msg}"),
            Self::Invariant(msg) => {
                write!(f, "compaction certificate invariant violated: {msg}")
            }
        }
    }
}

impl std::error::Error for CertifyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Dataset(diag) => Some(diag),
            Self::CanonBudgetExceeded(err) => Some(err),
            Self::Refused(_) | Self::Poison(_) | Self::Cbor(_) | Self::Invariant(_) => None,
        }
    }
}

impl From<RdfDiagnostic> for CertifyError {
    fn from(diag: RdfDiagnostic) -> Self {
        Self::Dataset(diag)
    }
}

impl From<compact::CompactRefusedError> for CertifyError {
    fn from(err: compact::CompactRefusedError) -> Self {
        Self::Refused(err.to_string())
    }
}

// ---------------------------------------------------------------------------
// Part 1 — content projection + refold digest
// ---------------------------------------------------------------------------

/// The term-ids that are the subject of at least one `rdf:type` quad naming a
/// compaction-provenance class (`stream:Compaction`, `stream:Manifestation`,
/// `stream:DetachedSignature`).
///
/// Provenance nodes are the subjects of ALL their own quads (the streaming
/// index/provenance authoring in `purrdf_gts::compact` never lets a content
/// node reuse a provenance node's id), so dropping every quad whose subject is
/// in this set removes provenance completely and leaves the content graph
/// untouched.
fn provenance_subject_ids(g: &Graph) -> crate::FastSet<usize> {
    let mut ids = crate::FastSet::default();
    for &(s, p, o, _) in &g.quads {
        if g.terms.get(p).and_then(|t| t.value.as_deref()) != Some(RDF_TYPE) {
            continue;
        }
        let is_provenance_class = match g.terms.get(o).and_then(|t| t.value.as_deref()) {
            Some(v) => {
                v == stream::COMPACTION
                    || v == stream::MANIFESTATION
                    || v == stream::DETACHED_SIGNATURE
            }
            None => false,
        };
        if is_provenance_class {
            ids.insert(s);
        }
    }
    ids
}

/// Project `g` down to its content: every quad whose subject is a
/// compaction-provenance node (`stream:Compaction`, `stream:Manifestation`,
/// `stream:DetachedSignature`) is dropped; everything else — including
/// reifiers, annotations, and blobs, which provenance never carries — passes
/// through unchanged. Terms are never pruned: a leftover unreferenced
/// provenance term produces no N-Quads line on its own.
///
/// This is the projection that makes a pre-compaction log and its
/// post-compaction pack canonicalize identically: the pack's ONLY graph-level
/// addition versus the source is this same provenance, which the projection
/// removes on both sides of the comparison.
#[must_use]
pub fn content_projection(g: &Graph) -> Graph {
    let provenance = provenance_subject_ids(g);
    Graph {
        terms: g.terms.clone(),
        quads: g
            .quads
            .iter()
            .copied()
            .filter(|&(s, _, _, _)| !provenance.contains(&s))
            .collect(),
        reifiers: g.reifiers.clone(),
        annotations: g.annotations.clone(),
        blobs: g.blobs.clone(),
        blob_meta: g.blob_meta.clone(),
        meta: g.meta.clone(),
        suppressions: g.suppressions.clone(),
        opaque: g.opaque.clone(),
        signatures: g.signatures.clone(),
        diagnostics: g.diagnostics.clone(),
        segment_heads: g.segment_heads.clone(),
        segment_profiles: g.segment_profiles.clone(),
        segment_meta: g.segment_meta.clone(),
        segment_streamable: g.segment_streamable.clone(),
    }
}

/// The RDFC-1.0 (SHA-256) digest of `g`'s content projection, as lowercase hex.
///
/// This is the "content-refold digest": two folded graphs — one straight from
/// a source log, one from a streamable-compacted pack of that same log —
/// yield the SAME digest iff the compaction preserved content (§10.1 refold
/// equivalence). The digest domain is deliberately RDFC-1.0's canonical
/// N-Quads text, not a structural hash over the IR, so it agrees with any
/// other RDFC-1.0-conformant engine.
///
/// # Errors
/// Returns [`CertifyError::Dataset`] when the projection cannot be bridged
/// into a frozen dataset, and [`CertifyError::Poison`] when its blank-node
/// count exceeds [`POISON_BLANK_LIMIT`] — refused BEFORE canonicalization is
/// attempted (refuse-don't-trust: never runs RDFC-1.0's combinatorial
/// n-degree search against untrusted, potentially adversarial input).
pub fn refold_digest(g: &Graph) -> Result<String, CertifyError> {
    canonical_digest(&content_projection(g))
}

/// The RDFC-1.0 (SHA-256) digest of an already-projected graph, as lowercase hex.
///
/// Shared by [`refold_digest`] (over [`content_projection`]) and
/// [`effective_digest`] (over [`effective_projection`]) — the two digest
/// domains differ only in which projection feeds this canonicalize+hash step.
///
/// # Errors
/// Returns [`CertifyError::Dataset`] when `projected` cannot be bridged into a
/// frozen dataset, and [`CertifyError::Poison`] when its blank-node count
/// exceeds [`POISON_BLANK_LIMIT`] — refused BEFORE canonicalization is
/// attempted (refuse-don't-trust: never runs RDFC-1.0's combinatorial
/// n-degree search against untrusted, potentially adversarial input).
fn canonical_digest(projected: &Graph) -> Result<String, CertifyError> {
    let dataset = dataset_from_gts_graph(projected)?;
    let blanks = crate::ir::canon::blank_count(&dataset);
    if blanks > POISON_BLANK_LIMIT {
        return Err(CertifyError::Poison(blanks));
    }
    let canonical = canonicalize_with(&dataset, CanonHash::Sha256);
    let digest = Sha256::digest(canonical.nquads.as_bytes());
    Ok(wire::hex(digest.as_slice()))
}

/// The fallible twin of [`canonical_digest`], for UNTRUSTED input
/// ([`verify_compaction`] and everything it calls): identical up through the
/// cheap [`POISON_BLANK_LIMIT`] blank-COUNT pre-reject, but canonicalizes
/// through [`try_canonicalize_with`] so a pathologically symmetric graph that
/// stays under the count limit yet exhausts RDFC-1.0's n-degree search call
/// budget surfaces as [`CertifyError::CanonBudgetExceeded`] instead of
/// panicking. Byte-identical `Ok` digest to [`canonical_digest`] on any input
/// both accept — determinism is unaffected, only the failure mode on
/// adversarial input changes from abort to a returned error.
fn try_canonical_digest(projected: &Graph) -> Result<String, CertifyError> {
    let dataset = dataset_from_gts_graph(projected)?;
    let blanks = crate::ir::canon::blank_count(&dataset);
    if blanks > POISON_BLANK_LIMIT {
        return Err(CertifyError::Poison(blanks));
    }
    let canonical = try_canonicalize_with(&dataset, CanonHash::Sha256)
        .map_err(CertifyError::CanonBudgetExceeded)?;
    let digest = Sha256::digest(canonical.nquads.as_bytes());
    Ok(wire::hex(digest.as_slice()))
}

/// The fallible twin of [`refold_digest`], for UNTRUSTED input — see
/// [`try_canonical_digest`].
///
/// # Errors
/// Returns [`CertifyError::Dataset`], [`CertifyError::Poison`], or
/// [`CertifyError::CanonBudgetExceeded`] — never panics, regardless of how
/// adversarially symmetric `g`'s content projection is.
fn try_refold_digest(g: &Graph) -> Result<String, CertifyError> {
    try_canonical_digest(&content_projection(g))
}

/// The fallible twin of [`effective_digest`], for UNTRUSTED input — see
/// [`try_canonical_digest`].
///
/// # Errors
/// Returns [`CertifyError::Dataset`], [`CertifyError::Poison`], or
/// [`CertifyError::CanonBudgetExceeded`] — never panics, regardless of how
/// adversarially symmetric `g`'s effective projection is.
fn try_effective_digest(g: &Graph) -> Result<String, CertifyError> {
    try_canonical_digest(&effective_projection(g))
}

/// A term-id resolved to its own `value` string, or `None` when the id is
/// out of range or the resolved term carries no value.
///
/// `ciborium::Value` is not `Hash`/`Eq` — term values are always plain
/// strings (`Term::value: Option<String>`), so resolving straight to
/// `String` lets [`term_suppressed_values`]/[`quad_suppressed_targets`] use
/// ordinary hash sets instead of a CBOR-aware comparator.
fn resolved_term_value(g: &Graph, id: usize) -> Option<String> {
    g.terms.get(id).and_then(|t| t.value.clone())
}

/// The kind text of a suppress-target map, or `None` when `target` is not a
/// map or carries no `"kind"` text field.
fn target_kind(target: &Value) -> Option<&str> {
    let Value::Map(entries) = target else {
        return None;
    };
    entries.iter().find_map(|(k, v)| match (k, v) {
        (Value::Text(k), Value::Text(v)) if k == "kind" => Some(v.as_str()),
        _ => None,
    })
}

/// Term VALUES hidden by every `term`-kind suppression target in `g`
/// (GTS-SPEC §11: a `term` target hides the term value AND every quad in
/// which that value appears, in ANY position).
fn term_suppressed_values(g: &Graph) -> HashSet<String> {
    let mut hidden = HashSet::new();
    for sup in &g.suppressions {
        for t in &sup.targets {
            if target_kind(t) != Some("term") {
                continue;
            }
            let Value::Map(entries) = t else { continue };
            if let Some(Value::Integer(i)) = wire::map_get(entries, "id")
                && let Some(id) = as_usize(i)
                && let Some(value) = resolved_term_value(g, id)
            {
                hidden.insert(value);
            }
        }
    }
    hidden
}

/// One resolved `quad`-kind suppression target: `(subject, predicate,
/// object, graph?)` VALUES — `graph` is `None` when the target's `"q"` array
/// omits the (optional) 4th element, matching a default-graph quad exactly
/// like the base wire encoding (GTS-SPEC §11).
type ValueQuad = (String, String, String, Option<String>);

/// Every `quad`-kind suppression target in `g`, resolved to term VALUES.
fn quad_suppressed_targets(g: &Graph) -> HashSet<ValueQuad> {
    let mut hidden = HashSet::new();
    for sup in &g.suppressions {
        for t in &sup.targets {
            if target_kind(t) != Some("quad") {
                continue;
            }
            let Value::Map(entries) = t else { continue };
            let Some(Value::Array(ids)) = wire::map_get(entries, "q") else {
                continue;
            };
            if ids.len() < 3 {
                continue;
            }
            let resolved: Vec<Option<String>> = ids
                .iter()
                .map(|v| match v {
                    Value::Integer(i) => as_usize(i).and_then(|id| resolved_term_value(g, id)),
                    _ => None,
                })
                .collect();
            // A malformed/out-of-range component makes the target
            // unresolvable — skip it rather than guess (refuse-don't-trust).
            if resolved[..3].iter().any(Option::is_none) {
                continue;
            }
            let graph = resolved.get(3).cloned().unwrap_or(None);
            if ids.len() > 3 && graph.is_none() {
                continue;
            }
            hidden.insert((
                resolved[0].clone().expect("checked above"),
                resolved[1].clone().expect("checked above"),
                resolved[2].clone().expect("checked above"),
                graph,
            ));
        }
    }
    hidden
}

/// Project `g` down to its EFFECTIVE (post-suppression) view: start from
/// [`content_projection`], then remove every base quad hidden by a `term`- or
/// `quad`-kind suppression (GTS-SPEC §11).
///
/// `blob`- and `frame`-kind suppressions never remove base quads: a `blob`
/// suppression hides content-addressed bytes that never appear as N-Quads
/// text, and a `frame`-kind suppression is refused pre-compaction (§10.1), so
/// it can never reach a fold this function is applied to. `reifier`-kind
/// suppressions hide a reifier BINDING (an `rdf:reifies` side-table entry),
/// not a base quad — filtering it out here would require also dropping the
/// matching reifier row and every annotation keyed off it, which is a
/// materially different (and separately trustworthy) operation from the
/// value-wise quad removal this function performs; scoping this function to
/// the two kinds that actually change the base-quad N-Quads digest (`term`,
/// `quad`) keeps it exact rather than approximately-right.
#[must_use]
pub fn effective_projection(g: &Graph) -> Graph {
    let mut projected = content_projection(g);
    let hidden_terms = term_suppressed_values(g);
    let hidden_quads = quad_suppressed_targets(g);
    if hidden_terms.is_empty() && hidden_quads.is_empty() {
        return projected;
    }
    let hides = |v: &Option<String>| v.as_ref().is_some_and(|v| hidden_terms.contains(v));
    projected.quads.retain(|&(s, p, o, gr)| {
        let sv = resolved_term_value(g, s);
        let pv = resolved_term_value(g, p);
        let ov = resolved_term_value(g, o);
        let gv = gr.and_then(|id| resolved_term_value(g, id));
        if hides(&sv) || hides(&pv) || hides(&ov) || hides(&gv) {
            return false;
        }
        match (sv, pv, ov) {
            (Some(sv), Some(pv), Some(ov)) => !hidden_quads.contains(&(sv, pv, ov, gv)),
            // A quad with an unresolvable component can never match a
            // resolved suppression target — keep it (nothing to hide it).
            _ => true,
        }
    });
    projected
}

/// The RDFC-1.0 (SHA-256) digest of `g`'s EFFECTIVE (post-suppression)
/// projection, as lowercase hex — see [`effective_projection`].
///
/// Where [`refold_digest`] proves value-wise RAW retention (every byte stays
/// present), `effective_digest` proves the suppression↔compaction commuting
/// square: a pre-compaction fold and its post-compaction pack yield the SAME
/// effective digest whenever the compaction carried every suppression
/// forward faithfully — the view a suppression-aware consumer actually sees
/// is preserved, not just the underlying bytes.
///
/// # Errors
/// Returns [`CertifyError::Dataset`] when the projection cannot be bridged
/// into a frozen dataset, and [`CertifyError::Poison`] when its blank-node
/// count exceeds [`POISON_BLANK_LIMIT`].
pub fn effective_digest(g: &Graph) -> Result<String, CertifyError> {
    canonical_digest(&effective_projection(g))
}

// ---------------------------------------------------------------------------
// Part 2 — verify_compaction
// ---------------------------------------------------------------------------

/// The outcome of independently verifying a claimed streamable compaction.
// Six independent §10.1 preservation facets, each a genuinely independent
// pass/fail axis a caller inspects individually (not a state machine — every
// combination is meaningful, e.g. content-equivalent but seam-broken).
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompactionReport {
    /// The pre- and post-compaction content projections canonicalize to the
    /// SAME RDFC-1.0 N-Quads (§10.1 refold equivalence).
    pub refold_equivalent: bool,
    /// The post-compaction pack (plus any accretive tail) folds without
    /// reader diagnostics — an intact, untorn hash chain.
    pub seam_chain_ok: bool,
    /// The pre-compaction detached-signature set is bound under the post's
    /// `stream:detachedSignatureRoot` MMR commitment, with each signature's
    /// selective inclusion proof verifying against it (vacuously true when
    /// pre carries no detached signatures AND post emits no root).
    pub signatures_bound: bool,
    /// Every carried `stream:DetachedSignature` in the post pack
    /// cryptographically verifies against the supplied keyring (vacuously
    /// true when post carries none).
    pub signatures_verify: bool,
    /// The pack's own MANDATORY packaging (index/head) signature is present
    /// and cryptographically valid under the supplied keyring.
    pub packaging_sig_ok: bool,
    /// TRUE iff BOTH halves of §10.1 suppression preservation hold: (raw)
    /// every suppression present in the pre-compaction fold is carried
    /// forward into the post-compaction fold (compared by resolved term
    /// VALUE, not by id — ids are re-based by the rewrite), AND (effective)
    /// [`effective_digest`] agrees between pre and post — the
    /// suppression↔compaction commuting square: the view a suppression-aware
    /// consumer actually sees is preserved, not merely the underlying bytes.
    pub suppressions_ok: bool,
}

impl CompactionReport {
    /// True iff every check passed.
    #[must_use]
    pub fn all_ok(&self) -> bool {
        self.refold_equivalent
            && self.seam_chain_ok
            && self.signatures_bound
            && self.signatures_verify
            && self.packaging_sig_ok
            && self.suppressions_ok
    }
}

/// Find the term-id whose `value` is exactly `value`.
fn term_id_with_value(g: &Graph, value: &str) -> Option<usize> {
    g.terms
        .iter()
        .position(|t| t.value.as_deref() == Some(value))
}

/// Whether any quad uses `predicate_iri` as its predicate.
fn has_predicate(g: &Graph, predicate_iri: &str) -> bool {
    let Some(p) = term_id_with_value(g, predicate_iri) else {
        return false;
    };
    g.quads.iter().any(|&(_, pred, _, _)| pred == p)
}

/// The literal value of the object of the first quad using `predicate_iri`,
/// scoped to a specific subject when `subject` is `Some`.
fn literal_object(g: &Graph, subject: Option<usize>, predicate_iri: &str) -> Option<String> {
    let p = term_id_with_value(g, predicate_iri)?;
    g.quads
        .iter()
        .find(|&&(s, pred, _, _)| pred == p && subject.is_none_or(|want| s == want))
        .and_then(|&(_, _, o, _)| g.terms.get(o))
        .and_then(|t| t.value.clone())
}

/// Subject ids of every node whose `rdf:type` is `class_iri`.
fn subjects_of_type(g: &Graph, class_iri: &str) -> Vec<usize> {
    let Some(rdf_type) = term_id_with_value(g, RDF_TYPE) else {
        return Vec::new();
    };
    let Some(class) = term_id_with_value(g, class_iri) else {
        return Vec::new();
    };
    g.quads
        .iter()
        .filter(|&&(_, p, o, _)| p == rdf_type && o == class)
        .map(|&(s, _, _, _)| s)
        .collect()
}

/// §10.1 signature preservation: the pre-compaction detached-signature set is
/// bound under the post's `stream:detachedSignatureRoot` MMR commitment.
fn signatures_bound_ok(pre: &Graph, post: &Graph) -> bool {
    let pre_leaves = compact::detached_signature_leaves(pre);
    if pre_leaves.is_empty() {
        return !has_predicate(post, stream::DETACHED_SIGNATURE_ROOT);
    }
    let Some(root_literal) = literal_object(post, None, stream::DETACHED_SIGNATURE_ROOT) else {
        return false;
    };
    let Ok(parsed_root) = purrdf_gts::mmr::parse_hex_32(&root_literal) else {
        return false;
    };
    let expected_root = purrdf_gts::mmr::root(&pre_leaves);
    if parsed_root != expected_root {
        return false;
    }
    for sig in pre.signatures.iter().filter(|s| s.cose.is_some()) {
        let cose_bytes = sig.cose.as_deref().expect("filtered to carry cose bytes");
        let Some(proof) = compact::detached_signature_proof(pre, &sig.frame_id, cose_bytes) else {
            return false;
        };
        if proof.root != expected_root || purrdf_gts::mmr::verify_proof(&proof).is_err() {
            return false;
        }
    }
    true
}

/// Every carried `stream:DetachedSignature` in `post` cryptographically
/// verifies against `keyring`.
fn signatures_verify_ok(post: &Graph, keyring: &HashMap<String, VerifyingKey>) -> bool {
    let nodes = subjects_of_type(post, stream::DETACHED_SIGNATURE);
    for node in nodes {
        let Some(cose_b64) = literal_object(post, Some(node), stream::COSE) else {
            return false;
        };
        let Ok(cose_bytes) = compact::base64url_decode(&cose_b64) else {
            return false;
        };
        let Some(source_frame) = literal_object(post, Some(node), stream::SOURCE_FRAME) else {
            return false;
        };
        let Ok(frame_id) = purrdf_gts::mmr::parse_hex_32(&source_frame) else {
            return false;
        };
        let Some((kid, _, _)) = cose::parse(&cose_bytes) else {
            return false;
        };
        let Some(key) = keyring.get(&kid) else {
            return false;
        };
        if cose::verify_sig(&cose_bytes, &frame_id, key) != SigStatus::Valid {
            return false;
        }
    }
    true
}

/// A term-id resolved to its own `value` (the RDF identity a compaction
/// rewrite preserves verbatim even as it shifts ids), or `Value::Null` when
/// the id is out of range.
fn term_value(g: &Graph, id: usize) -> Value {
    g.terms
        .get(id)
        .and_then(|t| t.value.clone())
        .map_or(Value::Null, Value::Text)
}

fn as_usize(i: &Integer) -> Option<usize> {
    usize::try_from(i128::from(*i)).ok()
}

/// Resolve every term-id embedded in a suppression target map to its term
/// VALUE instead of its (rewrite-shifted) id, so the same logical target
/// compares equal across a pre/post id re-basing. `term`/`reifier` targets
/// resolve their `"id"`; `quad` targets resolve their `"q"` array; every other
/// shape (`frame`, `blob`, …) is content-addressed or unresolvable and passes
/// through verbatim.
fn resolve_target(g: &Graph, target: &Value) -> Value {
    let Value::Map(entries) = target else {
        return target.clone();
    };
    let kind = entries.iter().find_map(|(k, v)| match (k, v) {
        (Value::Text(k), Value::Text(v)) if k == "kind" => Some(v.as_str()),
        _ => None,
    });
    let resolved: Vec<(Value, Value)> = entries
        .iter()
        .map(|(k, v)| {
            let key = if let Value::Text(t) = k {
                Some(t.as_str())
            } else {
                None
            };
            let value = match (kind, key, v) {
                (Some("term" | "reifier"), Some("id"), Value::Integer(i)) => {
                    as_usize(i).map_or_else(|| v.clone(), |id| term_value(g, id))
                }
                (Some("quad"), Some("q"), Value::Array(ids)) => Value::Array(
                    ids.iter()
                        .map(|item| match item {
                            Value::Integer(i) => {
                                as_usize(i).map_or_else(|| item.clone(), |id| term_value(g, id))
                            }
                            other => other.clone(),
                        })
                        .collect(),
                ),
                _ => v.clone(),
            };
            (k.clone(), value)
        })
        .collect();
    Value::Map(resolved)
}

/// A value-wise, id-rebasing-independent identity for a suppression.
type SuppressionSignature = (Vec<Value>, Option<String>, Option<Value>);

fn suppression_signature(g: &Graph, sup: &Suppression) -> SuppressionSignature {
    let targets = sup.targets.iter().map(|t| resolve_target(g, t)).collect();
    let by = sup.by.map(|id| term_value(g, id));
    (targets, sup.reason.clone(), by)
}

/// Every suppression present in `pre`'s fold is carried forward in `post`'s
/// fold (RAW retention, compared value-wise) — the first of the two halves
/// [`suppressions_ok`] requires.
fn suppression_retention_ok(pre: &Graph, post: &Graph) -> bool {
    let post_signatures: Vec<SuppressionSignature> = post
        .suppressions
        .iter()
        .map(|s| suppression_signature(post, s))
        .collect();
    pre.suppressions.iter().all(|sup| {
        let signature = suppression_signature(pre, sup);
        post_signatures.contains(&signature)
    })
}

/// TRUE iff BOTH: `pre`'s suppressions are retained value-wise in `post`
/// (RAW), AND `pre`/`post` agree on [`effective_digest`] (EFFECTIVE) — the
/// suppression↔compaction commuting square (see [`CompactionReport::suppressions_ok`]).
///
/// Called only from [`verify_compaction`] over UNTRUSTED bytes, so the
/// effective-digest agreement is computed via [`try_effective_digest`]
/// (fail-closed on a poison-budget graph) rather than [`effective_digest`]
/// (which panics for trusted callers).
///
/// # Errors
/// Returns [`CertifyError`] only when an effective projection genuinely
/// cannot be canonicalized (dataset bridge failure, the blank-count poison
/// guard, or RDFC-1.0 call-budget exhaustion) — never for a suppression
/// mismatch, which surfaces as `Ok(false)`.
fn suppressions_ok(pre: &Graph, post: &Graph) -> Result<bool, CertifyError> {
    let retained = suppression_retention_ok(pre, post);
    let effective_equivalent = try_effective_digest(pre)? == try_effective_digest(post)?;
    Ok(retained && effective_equivalent)
}

/// Independently verify a claimed streamable compaction from raw bytes.
///
/// Folds `pre_bytes` and `post_bytes` and checks all six independent facets of
/// §10.1 preservation (see [`CompactionReport`]). Never trusts the pack's own
/// embedded `stream:contentRefoldDigest` — the refold digest is always
/// recomputed from both `pre_bytes` and `post_bytes` here.
///
/// `pre_bytes`/`post_bytes` are UNTRUSTED (an independent verifier's whole
/// point is to not trust the claimant), so every digest this function
/// computes over them goes through the fallible RDFC-1.0 canonicalizer
/// ([`try_refold_digest`]/[`try_effective_digest`], backing
/// [`try_canonicalize_with`]) rather than the panicking
/// [`canonicalize_with`]: a pathologically symmetric blank graph that stays
/// under [`POISON_BLANK_LIMIT`]'s cheap blank-COUNT pre-reject yet exhausts
/// RDFC-1.0's n-degree search call budget surfaces as
/// [`CertifyError::CanonBudgetExceeded`], never a process-aborting panic —
/// this function must never be a resource-exhaustion (or crash) vector.
///
/// A post pack with a broken hash chain (a corrupted frame) still returns
/// `Ok` with `seam_chain_ok: false` rather than erroring — the reader
/// degrades a damaged frame to an opaque node rather than aborting (§7.6), so
/// the remaining checks stay meaningful over whatever content survives.
///
/// # Errors
/// Returns [`CertifyError`] only when a content projection genuinely cannot be
/// canonicalized: the GTS→dataset bridge fails, the blank-count poison guard
/// trips, or the RDFC-1.0 call budget is exhausted on a symmetric-poison
/// graph.
// The keyring mirrors the caller's key store, not a hot lookup path worth
// generalizing over `BuildHasher` — matches `verify_file_with_keyring`.
#[allow(clippy::implicit_hasher)]
pub fn verify_compaction(
    pre_bytes: &[u8],
    post_bytes: &[u8],
    keyring: &HashMap<String, VerifyingKey>,
) -> Result<CompactionReport, CertifyError> {
    let pre = read(pre_bytes, true, None);
    let post = read(post_bytes, true, None);

    let refold_equivalent = try_refold_digest(&pre)? == try_refold_digest(&post)?;
    let seam_chain_ok = post.diagnostics.is_empty();
    let signatures_bound = signatures_bound_ok(&pre, &post);
    let signatures_verify = signatures_verify_ok(&post, keyring);
    let packaging_sig_ok = verify_file_with_keyring(post_bytes, keyring).valid >= 1;
    let suppressions_ok = suppressions_ok(&pre, &post)?;

    Ok(CompactionReport {
        refold_equivalent,
        seam_chain_ok,
        signatures_bound,
        signatures_verify,
        packaging_sig_ok,
        suppressions_ok,
    })
}

// ---------------------------------------------------------------------------
// Part 4 — CompactionCertificate + canonical CBOR (Part 3 `compose` follows)
// ---------------------------------------------------------------------------

const FIELD_PRE: &str = "pre_refold_digest";
const FIELD_POST: &str = "post_refold_digest";
const FIELD_ROOTS: &str = "detached_sig_roots";
const FIELD_KIDS: &str = "packaging_kids";

/// A portable, independently-checkable claim that a streamable compaction
/// preserved content (§10.1/§10.2): the pre/post content-refold digests, the
/// accreted detached-signature MMR roots, and the packaging key ids involved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompactionCertificate {
    /// RDFC-1.0 digest of the pre-compaction content projection.
    pub pre_refold_digest: String,
    /// RDFC-1.0 digest of the post-compaction content projection.
    pub post_refold_digest: String,
    /// Hex `stream:detachedSignatureRoot` value(s) bound by the compaction(s)
    /// this certificate covers (a `Vec` so [`compose`] can accrete them).
    pub detached_sig_roots: Vec<String>,
    /// Packaging (index/head) signature key id(s) involved.
    pub packaging_kids: Vec<String>,
}

fn text_field(entries: &[(Value, Value)], key: &str) -> Result<String, CertifyError> {
    match wire::map_get(entries, key) {
        Some(Value::Text(text)) => Ok(text.clone()),
        Some(_) => Err(CertifyError::Cbor(format!("{key:?} must be a text string"))),
        None => Err(CertifyError::Cbor(format!(
            "certificate CBOR missing {key:?}"
        ))),
    }
}

fn text_array_field(entries: &[(Value, Value)], key: &str) -> Result<Vec<String>, CertifyError> {
    match wire::map_get(entries, key) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::Text(text) => Ok(text.clone()),
                _ => Err(CertifyError::Cbor(format!(
                    "{key:?} must be an array of text strings"
                ))),
            })
            .collect(),
        Some(_) => Err(CertifyError::Cbor(format!("{key:?} must be an array"))),
        None => Err(CertifyError::Cbor(format!(
            "certificate CBOR missing {key:?}"
        ))),
    }
}

impl CompactionCertificate {
    /// Encode as deterministic CBOR (RFC 8949 §4.2 canonical map ordering via
    /// `purrdf_gts::wire::canonical`) — byte-identical across repeated calls
    /// on an equal certificate.
    #[must_use]
    pub fn to_canonical_cbor(&self) -> Vec<u8> {
        let value = Value::Map(vec![
            (
                FIELD_PRE.into(),
                Value::Text(self.pre_refold_digest.clone()),
            ),
            (
                FIELD_POST.into(),
                Value::Text(self.post_refold_digest.clone()),
            ),
            (
                FIELD_ROOTS.into(),
                Value::Array(
                    self.detached_sig_roots
                        .iter()
                        .cloned()
                        .map(Value::Text)
                        .collect(),
                ),
            ),
            (
                FIELD_KIDS.into(),
                Value::Array(
                    self.packaging_kids
                        .iter()
                        .cloned()
                        .map(Value::Text)
                        .collect(),
                ),
            ),
        ]);
        wire::canonical(&value)
    }

    /// Decode the form written by [`Self::to_canonical_cbor`].
    ///
    /// # Errors
    /// Returns [`CertifyError::Cbor`] when `bytes` is not valid CBOR, is not a
    /// map, or is missing/misshapes a required field.
    pub fn from_canonical_cbor(bytes: &[u8]) -> Result<Self, CertifyError> {
        let value: Value = ciborium::de::from_reader(bytes)
            .map_err(|err| CertifyError::Cbor(format!("cannot parse certificate CBOR: {err}")))?;
        let Value::Map(entries) = value else {
            return Err(CertifyError::Cbor(
                "certificate CBOR root must be a map".to_string(),
            ));
        };
        Ok(Self {
            pre_refold_digest: text_field(&entries, FIELD_PRE)?,
            post_refold_digest: text_field(&entries, FIELD_POST)?,
            detached_sig_roots: text_array_field(&entries, FIELD_ROOTS)?,
            packaging_kids: text_array_field(&entries, FIELD_KIDS)?,
        })
    }
}

// ---------------------------------------------------------------------------
// Part 3 — compose
// ---------------------------------------------------------------------------

/// Chain two compaction certificates end-to-end: `Some` iff `a`'s post digest
/// equals `b`'s pre digest (`a: X→Y`, `b: Y→Z` ⇒ `X→Z`). Accretes the detached-
/// signature roots (`Vec` of both, in order) and unions the packaging kids
/// (deduplicated, first-seen order).
#[must_use]
pub fn compose(
    a: &CompactionCertificate,
    b: &CompactionCertificate,
) -> Option<CompactionCertificate> {
    if a.post_refold_digest != b.pre_refold_digest {
        return None;
    }
    let mut detached_sig_roots = a.detached_sig_roots.clone();
    detached_sig_roots.extend(b.detached_sig_roots.iter().cloned());

    let mut packaging_kids = a.packaging_kids.clone();
    for kid in &b.packaging_kids {
        if !packaging_kids.contains(kid) {
            packaging_kids.push(kid.clone());
        }
    }

    Some(CompactionCertificate {
        pre_refold_digest: a.pre_refold_digest.clone(),
        post_refold_digest: b.post_refold_digest.clone(),
        detached_sig_roots,
        packaging_kids,
    })
}

// ---------------------------------------------------------------------------
// Part 4 — certifying authoring wrapper
// ---------------------------------------------------------------------------

/// Compact `pre_bytes` into a streamable pack AND certify that the rewrite
/// preserved content — the sole authoring entry point that both performs and
/// certifies a compaction, so the two can never drift apart.
///
/// Computes the pre-compaction content-refold digest FIRST (running the
/// poison guard, refusing an uncanonicalizable input BEFORE authoring),
/// embeds it in the pack as `stream:contentRefoldDigest` (a proof-carrying
/// pack, §10.1), then recomputes the digest from the freshly authored pack
/// and asserts the two agree — an internal invariant, since a fresh
/// compaction disagreeing with the digest it was authored from is a
/// compaction-authoring bug, never a caller error.
///
/// # Errors
/// Returns [`CertifyError::Dataset`]/[`CertifyError::Poison`] when the source
/// cannot be canonicalized, [`CertifyError::Refused`] when
/// `compact_streamable` refuses the input, and [`CertifyError::Invariant`] if
/// the freshly authored pack's digest ever disagrees with the one it was
/// authored from (should be unreachable; surfaced rather than swallowed).
pub fn compact_and_certify(
    pre_bytes: &[u8],
    strategy: DictStrategy,
    timestamp: &str,
    seal_original: bool,
    packaging_signer: (SigningKey, String),
) -> Result<(Vec<u8>, CompactionCertificate), CertifyError> {
    let pre_fold = read(pre_bytes, true, None);
    if !pre_fold.diagnostics.is_empty() {
        return Err(diagnostics_to_error(&pre_fold).into());
    }
    let digest = refold_digest(&pre_fold)?;

    let (key, kid) = packaging_signer;
    let post_bytes = compact::compact_streamable(
        pre_bytes,
        CompactionParams {
            timestamp,
            seal_original,
            strategy,
            content_digest: Some(&digest),
            packaging_signer: Some((key, kid.clone())),
        },
    )?;

    let post_fold = read(&post_bytes, true, None);
    if !post_fold.diagnostics.is_empty() {
        return Err(diagnostics_to_error(&post_fold).into());
    }
    let post_digest = refold_digest(&post_fold)?;
    if post_digest != digest {
        return Err(CertifyError::Invariant(format!(
            "compaction changed the content-refold digest: pre={digest} post={post_digest}"
        )));
    }

    let detached_sig_roots = literal_object(&post_fold, None, stream::DETACHED_SIGNATURE_ROOT)
        .into_iter()
        .collect();

    Ok((
        post_bytes,
        CompactionCertificate {
            pre_refold_digest: digest,
            post_refold_digest: post_digest,
            detached_sig_roots,
            packaging_kids: vec![kid],
        },
    ))
}
