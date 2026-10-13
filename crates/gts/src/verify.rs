// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! High-level embedded-key verification helpers.
//!
//! This module mirrors the Python `gts.verify` surface: it discovers an
//! embedded `gts:transportKey`, resolves its OpenPGP Ed25519 public key, verifies
//! every COSE_Sign1 frame, and returns a data result suitable for libraries and
//! CLIs. Trust-policy findings remain separate from cryptographic validity.

use std::collections::HashMap;
use std::hash::BuildHasher;

use purrdf_ed25519::VerifyingKey;
use purrdf_lex::cbor::Value;

use crate::FastMap;
use crate::cose::{Algorithm, VerifyingKeyRef, verify_signatures_with_resolver};
use crate::emojihash::{emojihash, emojihash_labels, randomart};
use crate::model::{Diagnostic, Graph};
use crate::openpgp::parse_transport_key;
use crate::policy::{
    ProfileFinding, Severity, TrustPolicy, evaluate_profile_policy, signature_trust,
};
use crate::reader::read;

/// Explicit algorithm-tagged public key, independent of discovery and trust.
#[derive(Clone, Debug)]
pub enum VerificationKey {
    /// Existing Ed25519 public key.
    Ed25519(VerifyingKey),
    /// Dedicated composite public key.
    Composite(Box<crate::cose::composite::VerifyingKey>),
}

impl VerificationKey {
    /// Borrow the key through the sole typed COSE verifier.
    #[must_use]
    pub fn as_ref(&self) -> VerifyingKeyRef<'_> {
        match self {
            Self::Ed25519(key) => VerifyingKeyRef::Ed25519(key),
            Self::Composite(key) => VerifyingKeyRef::Composite(key),
        }
    }
}

purrdf_lex::variant_from!(VerificationKey {
    Ed25519(VerifyingKey),
    Composite(Box<crate::cose::composite::VerifyingKey>),
});

/// Caller-supplied resolution of an exact optional COSE identifier. The
/// declared algorithm is authenticated only after verification succeeds.
pub trait SignatureKeyring {
    /// Return an explicitly configured key; wrong algorithm/key type fails
    /// verification. `None` identifier is distinct from `Some(b"")`.
    fn resolve(&self, kid: Option<&[u8]>, algorithm: Algorithm) -> Option<VerifyingKeyRef<'_>>;
}

/// Fixed-hasher opaque-id keyring. An absent-id key is configured separately,
/// never fabricated from an empty identifier or used as a fallback.
#[derive(Clone, Debug, Default)]
pub struct Keyring {
    keys: FastMap<Vec<u8>, VerificationKey>,
    without_id: Option<VerificationKey>,
}

impl Keyring {
    /// Insert or replace the public key associated with exact opaque id bytes.
    /// Composite keys are supplied in a `Box`, keeping each keyring entry small.
    pub fn insert(
        &mut self,
        kid: impl AsRef<[u8]>,
        key: impl Into<VerificationKey>,
    ) -> Option<VerificationKey> {
        self.keys.insert(kid.as_ref().to_vec(), key.into())
    }

    /// Explicitly configure signatures that have no kid header. This does not
    /// resolve any present identifier, including an empty one.
    pub fn insert_without_id(
        &mut self,
        key: impl Into<VerificationKey>,
    ) -> Option<VerificationKey> {
        self.without_id.replace(key.into())
    }
}

impl SignatureKeyring for Keyring {
    fn resolve(&self, kid: Option<&[u8]>, _: Algorithm) -> Option<VerifyingKeyRef<'_>> {
        match kid {
            Some(kid) => self.keys.get(kid),
            None => self.without_id.as_ref(),
        }
        .map(VerificationKey::as_ref)
    }
}

// The existing text/Ed25519 keyring remains a convenience over the same core.
impl<S: BuildHasher> SignatureKeyring for HashMap<String, VerifyingKey, S> {
    fn resolve(&self, kid: Option<&[u8]>, _: Algorithm) -> Option<VerifyingKeyRef<'_>> {
        let kid = core::str::from_utf8(kid?).ok()?;
        self.get(kid).map(VerifyingKeyRef::Ed25519)
    }
}

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

purrdf_hash::default_from_new!(VerifyOptions);

impl VerifyOptions {
    /// Release-style defaults, and the [`Default`]: embedded key lookup and signatures
    /// required.
    #[must_use]
    pub fn new() -> Self {
        Self {
            armored_key: None,
            require_signatures: true,
            trust_policy: TrustPolicy::default(),
        }
    }

    /// Use an out-of-band trusted public key instead of embedded metadata.
    #[must_use]
    pub fn with_armored_key(mut self, armored: impl Into<String>) -> Self {
        self.armored_key = Some(armored.into());
        self
    }

    /// Set whether unsigned files are accepted.
    #[must_use]
    pub fn require_signatures(mut self, value: bool) -> Self {
        self.require_signatures = value;
        self
    }

    /// Apply deployment-level signer/profile trust rules.
    #[must_use]
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
    let value = purrdf_lex::assoc::get(&graph.meta, "gts:transportKey")?;
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
    verify_file_with_options(data, &VerifyOptions::new())
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
                return verify_graph_with_keyring(first, &Keyring::default(), options);
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

    let graph = graph.unwrap_or_else(|| read(data, true, None));
    // Fold the resolved single key into a one-entry keyring so single-key and
    // rotation-capable verification share the same core (no parallel resolver
    // mechanism).
    debug_assert!(
        errors.is_empty(),
        "every earlier error path returns before reaching the shared keyring core"
    );
    let mut keyring = FastMap::with_capacity_and_hasher(1, purrdf_hash::fixed::FixedState::new());
    keyring.insert(kid.clone(), public);
    let mut result = verify_graph_with_keyring(graph, &keyring, options);
    result.kid = Some(kid);
    result.fingerprint = Some(fingerprint);
    result.emojihash = Some(emojihash(&raw_public, 11));
    result.emojihash_labels = Some(emojihash_labels(&raw_public, 11));
    result.randomart = Some(randomart(&raw_public, "GTS transport"));
    result
}

/// Verify a GTS file's embedded signatures against a rotation-capable keyring.
///
/// Every COSE `kid` present in the folded signatures is resolved independently
/// against `keyring`, so a file whose frames were signed under different keys
/// over time (key rotation) verifies as long as each `kid` used is present.
/// This is the core [`verify_file_with_options`] also uses internally, folded
/// down to its single resolved key.
pub fn verify_file_with_keyring<K: SignatureKeyring + ?Sized>(
    data: &[u8],
    keyring: &K,
) -> VerificationResult {
    let options = VerifyOptions::default();
    verify_graph_with_keyring(read(data, true, None), keyring, &options)
}

fn verify_graph_with_keyring<K: SignatureKeyring + ?Sized>(
    mut graph: Graph,
    keyring: &K,
    options: &VerifyOptions,
) -> VerificationResult {
    let result = verify_against_keyring(&mut graph, keyring, options);
    VerificationResult {
        ok: result.ok,
        kid: None,
        fingerprint: None,
        emojihash: None,
        emojihash_labels: None,
        randomart: None,
        frames: result.signed,
        signed: result.signed,
        valid: result.valid,
        trusted: result.trusted,
        invalid: result.invalid,
        unverified: result.unverified,
        errors: result.errors,
        diagnostics: graph.diagnostics,
        profile_findings: result.profile_findings,
    }
}

/// Counts, errors, and trust findings from resolving each embedded signature
/// against `keyring` — shared by [`verify_file_with_options`] (a one-entry
/// keyring around its resolved transport key) and [`verify_file_with_keyring`]
/// (an explicit rotation-capable keyring).
struct KeyringVerification {
    signed: usize,
    valid: usize,
    invalid: usize,
    unverified: usize,
    trusted: usize,
    errors: Vec<String>,
    profile_findings: Vec<ProfileFinding>,
    ok: bool,
}

fn verify_against_keyring<K: SignatureKeyring + ?Sized>(
    graph: &mut Graph,
    keyring: &K,
    options: &VerifyOptions,
) -> KeyringVerification {
    verify_signatures_with_resolver(&mut graph.signatures, |candidate, algorithm| {
        keyring.resolve(candidate, algorithm)
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
    let trusts = signature_trust(graph, Some(&options.trust_policy));
    let trusted = trusts.iter().filter(|item| item.trusted).count();
    let profile_findings = evaluate_profile_policy(graph, Some(&options.trust_policy), None);

    let mut errors = integrity_errors(&graph.diagnostics);
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

    KeyringVerification {
        signed,
        valid,
        invalid,
        unverified,
        trusted,
        errors,
        profile_findings,
        ok,
    }
}

fn integrity_errors(diagnostics: &[Diagnostic]) -> Vec<String> {
    // Damaged frames can be withheld from the folded signature list. Valid
    // signatures on survivors cannot establish file integrity. MissingKey and
    // UnknownCodec still permit authentication of opaque ciphertext/payloads.
    diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.code.as_str(),
                "DamagedFrame"
                    | "EmptyFile"
                    | "BrokenChain"
                    | "TruncatedLog"
                    | "TornAppendError"
                    | "ResourceLimit"
            )
        })
        .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.detail))
        .collect()
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
