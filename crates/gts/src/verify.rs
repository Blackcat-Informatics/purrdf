// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! High-level embedded-key verification helpers.
//!
//! This module mirrors the Python `gts.verify` surface: it discovers an
//! embedded `gts:transportKey`, resolves its OpenPGP Ed25519 public key, verifies
//! every COSE_Sign1 frame, and returns a data result suitable for libraries and
//! CLIs. Trust-policy findings remain separate from cryptographic validity.

use ciborium::value::Value;
use ed25519_dalek::VerifyingKey;

use crate::cose::verify_signatures;
use crate::emojihash::{emojihash, emojihash_labels, randomart};
use crate::model::{Diagnostic, Graph};
use crate::openpgp::parse_transport_key;
use crate::policy::{
    evaluate_profile_policy, signature_trust, ProfileFinding, Severity, TrustPolicy,
};
use crate::reader::read;

/// The embedded `gts:transportKey` metadata value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddedTransportKey {
    /// The key id used by COSE signatures in this file.
    pub kid: String,
    /// ASCII-armored OpenPGP Ed25519 public-key certificate.
    pub gpg: String,
}

/// Options for [`verify_file_with_options`].
///
/// Cryptographic validity and deployment trust are intentionally separated:
/// signatures are first checked against a resolved OpenPGP Ed25519 key, then
/// [`TrustPolicy`] decides whether valid signers and declared profiles are
/// acceptable for the caller.
#[derive(Clone, Debug)]
pub struct VerifyOptions {
    /// Optional out-of-band armored OpenPGP public key. When absent, the file's
    /// embedded `gts:transportKey` is used.
    pub armored_key: Option<String>,
    /// Treat a file with no signed frames as a verification failure.
    pub require_signatures: bool,
    /// Optional deployment trust policy layered above cryptographic validity.
    pub trust_policy: TrustPolicy,
}

impl Default for VerifyOptions {
    fn default() -> Self {
        Self {
            armored_key: None,
            require_signatures: true,
            trust_policy: TrustPolicy::default(),
        }
    }
}

impl VerifyOptions {
    /// Release-style defaults: embedded key lookup and signatures required.
    pub fn strict() -> Self {
        Self::default()
    }

    /// Use an out-of-band trusted public key instead of embedded metadata.
    pub fn with_armored_key(mut self, armored: impl Into<String>) -> Self {
        self.armored_key = Some(armored.into());
        self
    }

    /// Set whether unsigned files are accepted.
    pub fn require_signatures(mut self, value: bool) -> Self {
        self.require_signatures = value;
        self
    }

    /// Apply deployment-level signer/profile trust rules.
    pub fn trust_policy(mut self, policy: TrustPolicy) -> Self {
        self.trust_policy = policy;
        self
    }
}

/// Outcome of verifying a GTS file's signatures and profile trust policy.
#[derive(Clone, Debug, Default)]
pub struct VerificationResult {
    /// True when no cryptographic errors, unresolved signatures, or profile
    /// policy errors were found under the supplied options.
    pub ok: bool,
    /// Key id used for verification, either embedded or derived from the
    /// out-of-band OpenPGP fingerprint.
    pub kid: Option<String>,
    /// Uppercase OpenPGP v4 fingerprint of the resolved transport key.
    pub fingerprint: Option<String>,
    /// Emoji visual hash of the raw Ed25519 public key.
    pub emojihash: Option<String>,
    /// Speakable labels corresponding to [`Self::emojihash`].
    pub emojihash_labels: Option<String>,
    /// OpenSSH-style randomart of the raw Ed25519 public key.
    pub randomart: Option<String>,
    /// Number of signed frames inspected. This mirrors Python's result shape.
    pub frames: usize,
    /// Number of COSE_Sign1 frame signatures present.
    pub signed: usize,
    /// Number of signatures cryptographically valid under the resolved key.
    pub valid: usize,
    /// Number of valid signatures whose signer is trusted by the policy.
    pub trusted: usize,
    /// Number of signatures that failed cryptographic verification.
    pub invalid: usize,
    /// Number of signatures whose key id could not be resolved.
    pub unverified: usize,
    /// Human-readable verification errors.
    pub errors: Vec<String>,
    /// Reader diagnostics produced while folding the file.
    pub diagnostics: Vec<Diagnostic>,
    /// Profile and trust-policy findings layered above core verification.
    pub profile_findings: Vec<ProfileFinding>,
}

/// Return an OpenPGP fingerprint grouped for human comparison.
pub fn format_fingerprint(fingerprint: &str) -> String {
    let compact: String = fingerprint.chars().filter(|c| !c.is_whitespace()).collect();
    let compact = compact.to_uppercase();
    if compact.is_empty() || !compact.bytes().all(|b| b.is_ascii_hexdigit()) {
        return fingerprint.to_string();
    }
    compact
        .as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).expect("hex is ascii"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Return the embedded `gts:transportKey` meta value if well-formed.
pub fn extract_transport_key(graph: &Graph) -> Option<EmbeddedTransportKey> {
    let value = graph
        .meta
        .iter()
        .find(|(k, _)| k == "gts:transportKey")
        .map(|(_, v)| v)?;
    let Value::Map(entries) = value else {
        return None;
    };
    let mut kid = None;
    let mut gpg = None;
    for (key, value) in entries {
        if let (Value::Text(key), Value::Text(text)) = (key, value) {
            match key.as_str() {
                "kid" => kid = Some(text.clone()),
                "gpg" => gpg = Some(text.clone()),
                _ => {}
            }
        }
    }
    Some(EmbeddedTransportKey {
        kid: kid?,
        gpg: gpg?,
    })
}

/// Verify a GTS file with strict defaults: embedded key lookup and signatures required.
pub fn verify_file(data: &[u8]) -> VerificationResult {
    verify_file_with_options(data, &VerifyOptions::strict())
}

/// Verify a GTS file's embedded signatures with explicit options.
pub fn verify_file_with_options(data: &[u8], options: &VerifyOptions) -> VerificationResult {
    let mut errors = Vec::new();

    let (kid, public, raw_public, fingerprint, graph): (
        String,
        VerifyingKey,
        [u8; 32],
        String,
        Option<Graph>,
    ) = if let Some(armored) = options.armored_key.as_deref() {
        match provider_from_armor(armored, None) {
            Ok((kid, public, raw_public, fingerprint)) => {
                (kid, public, raw_public, fingerprint, None)
            }
            Err(err) => {
                errors.push(format!("cannot load trusted key: {err}"));
                return VerificationResult {
                    ok: false,
                    errors,
                    ..VerificationResult::default()
                };
            }
        }
    } else {
        let first = read(data, true, None);
        let Some(transport) = extract_transport_key(&first) else {
            if !options.require_signatures && first.signatures.is_empty() {
                return VerificationResult {
                    ok: true,
                    frames: first.signatures.len(),
                    diagnostics: first.diagnostics,
                    ..VerificationResult::default()
                };
            }
            errors.push("no gts:transportKey found in file metadata".to_string());
            return VerificationResult {
                ok: false,
                errors,
                diagnostics: first.diagnostics,
                frames: first.signatures.len(),
                signed: first.signatures.len(),
                ..VerificationResult::default()
            };
        };
        match provider_from_armor(&transport.gpg, Some(&transport.kid)) {
            Ok((kid, public, raw_public, fingerprint)) => {
                (kid, public, raw_public, fingerprint, Some(first))
            }
            Err(err) => {
                errors.push(format!("cannot load embedded transport key: {err}"));
                return VerificationResult {
                    ok: false,
                    kid: Some(transport.kid),
                    errors,
                    diagnostics: first.diagnostics,
                    frames: first.signatures.len(),
                    signed: first.signatures.len(),
                    ..VerificationResult::default()
                };
            }
        }
    };

    let mut graph = graph.unwrap_or_else(|| read(data, true, None));
    verify_signatures(&mut graph.signatures, |candidate| {
        (candidate == kid).then_some(public)
    });

    let signed = graph.signatures.len();
    let valid = graph
        .signatures
        .iter()
        .filter(|sig| sig.status == "valid")
        .count();
    let invalid = graph
        .signatures
        .iter()
        .filter(|sig| sig.status == "invalid")
        .count();
    let unverified = graph
        .signatures
        .iter()
        .filter(|sig| sig.status == "unverified")
        .count();
    let trusts = signature_trust(&graph, Some(&options.trust_policy));
    let trusted = trusts.iter().filter(|item| item.trusted).count();
    let profile_findings = evaluate_profile_policy(&graph, Some(&options.trust_policy), None);

    if invalid > 0 {
        errors.push(format!("{invalid} signature(s) invalid"));
    }
    if unverified > 0 {
        errors.push(format!(
            "{unverified} signature(s) unverified (no key resolved)"
        ));
    }
    if options.require_signatures && signed == 0 {
        errors.push("no signed frames found".to_string());
    }

    let has_profile_error = profile_findings
        .iter()
        .any(|finding| finding.severity == Severity::Error);
    let ok = errors.is_empty() && invalid == 0 && unverified == 0 && !has_profile_error;

    VerificationResult {
        ok,
        kid: Some(kid),
        fingerprint: Some(fingerprint),
        emojihash: Some(emojihash(&raw_public, 11)),
        emojihash_labels: Some(emojihash_labels(&raw_public, 11)),
        randomart: Some(randomart(&raw_public, "GTS transport")),
        frames: signed,
        signed,
        valid,
        trusted,
        invalid,
        unverified,
        errors,
        diagnostics: graph.diagnostics,
        profile_findings,
    }
}

fn provider_from_armor(
    armored: &str,
    kid: Option<&str>,
) -> Result<(String, VerifyingKey, [u8; 32], String), String> {
    let parsed = parse_transport_key(armored).map_err(|e| e.to_string())?;
    let public = VerifyingKey::from_bytes(&parsed.raw_public).map_err(|e| e.to_string())?;
    let resolved_kid = kid.unwrap_or(&parsed.fingerprint).to_string();
    Ok((resolved_kid, public, parsed.raw_public, parsed.fingerprint))
}
