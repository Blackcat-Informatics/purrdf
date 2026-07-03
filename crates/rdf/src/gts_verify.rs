// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Full content-chain verification bridge for `purrdf`.
//!
//! [`verify_content_chain`] is the single, hard-failing entry point that ties a
//! frozen [`RdfDataset`](crate::RdfDataset)'s content-addressed terms back to a
//! signed GTS file. It performs three checks, in order, and propagates the first
//! failure as an [`RdfDiagnostic`] — nothing is swallowed:
//!
//! 1. **COSE signatures** ([`purrdf_gts::verify::verify_file`]): the file's
//!    embedded transport key must resolve and every signed frame must verify.
//! 2. **Expected-head chain replay** ([`purrdf_gts::reader::read`] with an
//!    `expected_head`): the log must fold without diagnostics AND the last
//!    segment head must equal the caller's `expected_head` (a mismatch surfaces
//!    as a reader `TruncatedLog` diagnostic).
//! 3. **Digest inclusion**: every content-addressed term's cached BLAKE3 digest
//!    (from `rdf-core`'s content-id side table) must be an included blob id or
//!    segment-head id in the verified chain.
//!
//! Inclusion is checked against the file's blob ids (`graph.blobs` keys, already
//! spelled `blake3:<hex>`) and its segment-head ids (rendered `blake3:<hex>`).
//! It deliberately does **not** use the MMR proof API: `mmr::prove_file` operates
//! on frame ids, and a content-addressed blob digest is not necessarily an MMR
//! leaf, so an MMR check here would produce false failures for legitimately
//! included blobs. The blob/segment-head membership test is exact and total.

use std::collections::BTreeSet;

use purrdf_gts::reader;
use purrdf_gts::verify::verify_file;

use crate::gts_core::diagnostics_to_error;
use crate::{RdfDataset, RdfDiagnostic};

/// The result of a successful [`verify_content_chain`] pass.
///
/// Every field reflects a check that PASSED; the function only returns `Ok`
/// when the signatures verified, the chain replayed against the expected head,
/// and every content-addressed term's digest was included in the file.
#[derive(Clone, Debug)]
pub struct ContentChainVerification {
    /// Number of COSE frame signatures cryptographically valid under the
    /// resolved transport key (from [`purrdf_gts::verify::VerificationResult::valid`]).
    pub signatures_valid: usize,
    /// Number of valid signatures whose signer is trusted by the default trust
    /// policy (from [`purrdf_gts::verify::VerificationResult::trusted`]).
    pub signatures_trusted: usize,
    /// Number of the dataset's content-addressed terms whose digest was found in
    /// the verified chain (equals the dataset's content-id count on success).
    pub digests_included: usize,
    /// Always `true` on success: the replayed log's last segment head matched
    /// the caller-supplied `expected_head`.
    pub head_matched: bool,
}

/// Fully verify that `dataset`'s content-addressed terms are backed by the signed
/// GTS file `gts_bytes` whose expected last-segment head is `expected_head`.
///
/// See the [module docs](self) for the exact checks. Hard-fails on the first
/// problem — a bad signature, a fold diagnostic, an expected-head mismatch, or a
/// content-addressed term whose digest is not included in the file.
///
/// # Errors
///
/// Returns an [`RdfDiagnostic`] when any stage fails:
/// - `gts-verify-signature` — COSE verification did not succeed.
/// - `gts-fold-diagnostic` — the reader reported diagnostics (includes an
///   expected-head mismatch, surfaced as `TruncatedLog`).
/// - `gts-verify-digest-inclusion` — one or more content-addressed terms are not
///   included in the verified chain (the detail names each missing term + digest,
///   in sorted `TermId` order).
pub fn verify_content_chain(
    dataset: &RdfDataset,
    gts_bytes: &[u8],
    expected_head: &[u8],
) -> Result<ContentChainVerification, RdfDiagnostic> {
    // 1. COSE signatures: resolve the embedded transport key and verify every
    //    signed frame. `ok` is false whenever any error was recorded.
    let verification = verify_file(gts_bytes);
    if !verification.ok {
        let detail = if verification.errors.is_empty() {
            "signature verification did not succeed".to_owned()
        } else {
            verification.errors.join("; ")
        };
        return Err(RdfDiagnostic::error(
            "gts-verify-signature",
            "GTS COSE signature verification failed",
        )
        .with_detail(detail));
    }

    // 2. Expected-head chain replay: fold every segment and enforce the head.
    //    Any reader diagnostic (including a head mismatch) is a hard failure.
    let graph = reader::read(gts_bytes, true, Some(expected_head));
    if !graph.diagnostics.is_empty() {
        return Err(diagnostics_to_error(&graph));
    }
    let head_matched = true;

    // 3. Digest inclusion: the file's content ids are its blob ids (already
    //    `blake3:<hex>`) plus its segment-head ids (rendered `blake3:<hex>`).
    let mut included: BTreeSet<String> = BTreeSet::new();
    for (digest, _entry) in &graph.blobs {
        included.insert(digest.clone());
    }
    for head in &graph.segment_heads {
        included.insert(format!("blake3:{}", hex_lower(head)));
    }

    // `content_ids()` yields sorted-by-`TermId` pairs, so both the match count
    // and the missing list are deterministic without an extra sort.
    let mut digests_included = 0usize;
    let mut missing: Vec<String> = Vec::new();
    for (term_id, digest) in dataset.content_ids() {
        let content_id = format!("blake3:{}", digest.to_hex());
        if included.contains(&content_id) {
            digests_included += 1;
        } else {
            missing.push(format!("term#{} {content_id}", term_id.index()));
        }
    }
    if !missing.is_empty() {
        return Err(RdfDiagnostic::error(
            "gts-verify-digest-inclusion",
            format!(
                "{} content-addressed term(s) are not included in the verified chain",
                missing.len()
            ),
        )
        .with_detail(missing.join("; ")));
    }

    Ok(ContentChainVerification {
        signatures_valid: verification.valid,
        signatures_trusted: verification.trusted,
        digests_included,
        head_matched,
    })
}

/// Lowercase-hex render of raw bytes (segment-head ids carry no `blake3:` prefix).
fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ciborium::value::Value;
    use purrdf_core::ir::RdfDatasetBuilder;
    use purrdf_core::{ContentIdScheme, RdfLiteral};
    use purrdf_gts::openpgp::parse_secret_signing_key;
    use purrdf_gts::wire::digest_str;
    use purrdf_gts::writer::Writer;

    use super::{verify_content_chain, RdfDataset};

    const KID: &str = "did:example:test";
    const BLOB_PAYLOAD: &[u8] = b"content-addressed payload for the GTS verify bridge";

    fn secret_armor() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vectors/openpgp/test_key.sec.asc"
        ))
        .expect("secret key fixture")
    }

    fn public_armor() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vectors/openpgp/test_key.pub.asc"
        ))
        .expect("public key fixture")
    }

    /// Build a GTS file: optionally signed with the fixture transport key, always
    /// carrying one inline blob. Returns `(bytes, head)`.
    fn build_file(sign: bool) -> (Vec<u8>, Vec<u8>) {
        let mut writer = Writer::new("purrdf.gts");
        if sign {
            let (signing_key, kid) = parse_secret_signing_key(&secret_armor(), Some(KID))
                .expect("parse fixture secret key")
                .into_parts();
            writer.sign_with(signing_key, &kid);
            writer.add_meta(Value::Map(vec![(
                Value::Text("gts:transportKey".to_owned()),
                Value::Map(vec![
                    (Value::Text("kid".to_owned()), Value::Text(kid)),
                    (Value::Text("gpg".to_owned()), Value::Text(public_armor())),
                ]),
            )]));
        }
        writer.add_blob(BLOB_PAYLOAD, Some("text/plain"), Some("doc"));
        let bytes = writer.to_bytes();
        let head = writer.head().to_vec();
        (bytes, head)
    }

    /// Freeze a content-addressing dataset that references `content_iri` as the
    /// subject of a single quad.
    fn dataset_referencing(content_iri: &str) -> Arc<RdfDataset> {
        let scheme = ContentIdScheme::new("blake3:").expect("valid scheme");
        let mut builder = RdfDatasetBuilder::with_content_addressing(scheme, None);
        let s = builder.intern_iri(content_iri);
        let p = builder.intern_iri("https://example.org/p");
        let o = builder.intern_literal(RdfLiteral::simple("o"));
        builder.push_quad(s, p, o, None);
        builder.freeze().expect("freeze content-addressing dataset")
    }

    #[test]
    fn verifies_signed_chain_with_included_digest() {
        let (bytes, head) = build_file(true);
        let dataset = dataset_referencing(&digest_str(BLOB_PAYLOAD));

        let result = verify_content_chain(&dataset, &bytes, &head)
            .expect("fully valid content chain verifies");
        assert_eq!(
            result.digests_included, 1,
            "the one blob digest is included"
        );
        assert!(result.head_matched, "the expected head matched");
        assert!(
            result.signatures_valid >= 1,
            "at least one COSE signature was cryptographically valid"
        );
    }

    #[test]
    fn wrong_expected_head_is_rejected() {
        let (bytes, mut head) = build_file(true);
        let dataset = dataset_referencing(&digest_str(BLOB_PAYLOAD));
        // Flip a byte of the head so the replay's head check fails.
        head[0] ^= 0xff;

        let err = verify_content_chain(&dataset, &bytes, &head)
            .expect_err("a wrong expected head must fail");
        assert_eq!(err.code, "gts-fold-diagnostic");
    }

    #[test]
    fn missing_digest_names_the_term() {
        let (bytes, head) = build_file(true);
        // A digest that is NOT the blob's and NOT a segment head.
        let absent = format!("blake3:{}", "ab".repeat(32));
        let dataset = dataset_referencing(&absent);

        let err = verify_content_chain(&dataset, &bytes, &head)
            .expect_err("a digest absent from the file must fail");
        assert_eq!(err.code, "gts-verify-digest-inclusion");
        let detail = err.detail.unwrap_or_default();
        assert!(
            detail.contains("term#0") && detail.contains(&absent),
            "detail names the missing term and digest: {detail}"
        );
    }

    #[test]
    fn unsigned_file_fails_the_signature_gate() {
        let (bytes, head) = build_file(false);
        let dataset = dataset_referencing(&digest_str(BLOB_PAYLOAD));

        let err = verify_content_chain(&dataset, &bytes, &head)
            .expect_err("an unsigned file must fail the signature gate");
        assert_eq!(err.code, "gts-verify-signature");
    }

    #[test]
    fn tampered_bytes_are_rejected() {
        let (mut bytes, head) = build_file(true);
        let dataset = dataset_referencing(&digest_str(BLOB_PAYLOAD));
        // Corrupt the final frame. Depending on where the flip lands this fails
        // either the signature gate (an invalid COSE frame) or the replay gate (a
        // frame that degrades to opaque / breaks the chain) — both are hard fails
        // and nothing is swallowed.
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;

        let err = verify_content_chain(&dataset, &bytes, &head)
            .expect_err("tampered bytes must fail verification");
        assert!(
            matches!(
                err.code.as_str(),
                "gts-verify-signature" | "gts-fold-diagnostic"
            ),
            "tampering is caught at a verification gate, got {}",
            err.code
        );
    }
}
