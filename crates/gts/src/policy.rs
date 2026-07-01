// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Trust and profile-policy checks layered above core GTS validity.
//!
//! The reader verifies bytes, hashes, chains, signatures, and decryptability.
//! This module keeps deployment trust and profile conformance separate from
//! those mechanics: a signature status of `"valid"` means the COSE signature
//! verified under a resolved key, not that the signer is authorized or that the
//! signed claim is true.

use std::collections::{BTreeSet, HashSet};

use ciborium::value::Value;

use crate::model::{Graph, Signature, TermKind};
use crate::stream::{SEALED_SOURCE, STREAM_NS};
use crate::wire::map_get;

/// Vocabulary namespace used by the optional-standard `files` profile.
pub const FILES_NS: &str = "https://w3id.org/gts/files#";
/// Default high-privacy recipient-key-id pattern for the `opaque` profile.
pub const DEFAULT_PSEUDONYMOUS_KID_PATTERN: &str = "^anon:[0-9a-fA-F]{32,}$";

const PROFILE_VOCABS: &[(&str, &str)] = &[("files", FILES_NS)];

/// Deployment trust anchors and high-privacy recipient-id rules.
///
/// The policy is deliberately deployment-local. It authorizes signers and
/// recipient identifier shapes after the core reader has already parsed the
/// file and, if requested, verified COSE signatures. Core GTS validity never
/// depends on this struct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustPolicy {
    /// Signer `kid` values trusted by the deployment.
    pub trusted_signers: HashSet<String>,
    /// Require at least one cryptographically valid signature from
    /// [`Self::trusted_signers`] for profiles that require signatures.
    pub require_trusted_signer: bool,
    /// Pattern accepted for high-privacy opaque-profile recipient `kid` values.
    ///
    /// The default shape is implemented without a regex dependency. Custom
    /// values support exact string matching or anchored literal full matches
    /// such as `^did:example:recipient$`.
    pub pseudonymous_kid_pattern: String,
}

impl Default for TrustPolicy {
    fn default() -> Self {
        Self {
            trusted_signers: HashSet::new(),
            require_trusted_signer: false,
            pseudonymous_kid_pattern: DEFAULT_PSEUDONYMOUS_KID_PATTERN.to_string(),
        }
    }
}

impl TrustPolicy {
    /// Build a deployment policy from trusted signer ids.
    pub fn new<I, S>(trusted_signers: I, require_trusted_signer: bool) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            trusted_signers: trusted_signers.into_iter().map(Into::into).collect(),
            require_trusted_signer,
            ..Self::default()
        }
    }

    /// True when `kid` is a deployment-trusted signer.
    pub fn is_trusted(&self, kid: Option<&str>) -> bool {
        kid.is_some_and(|kid| self.trusted_signers.contains(kid))
    }

    /// True when `kid` satisfies the high-privacy opaque-profile shape.
    ///
    /// The default pattern is implemented as the documented
    /// `anon:[0-9a-fA-F]{32,}` shape without pulling in a regex dependency. If
    /// `pseudonymous_kid_pattern` is customized, dependency-free matching
    /// supports exact strings and anchored literal full-match patterns such as
    /// `^did:example:recipient$`.
    pub fn is_pseudonymous_recipient(&self, kid: &str) -> bool {
        if self.pseudonymous_kid_pattern != DEFAULT_PSEUDONYMOUS_KID_PATTERN {
            return custom_pseudonymous_pattern_matches(&self.pseudonymous_kid_pattern, kid);
        }
        let Some(hex) = kid.strip_prefix("anon:") else {
            return false;
        };
        hex.len() >= 32 && hex.bytes().all(|b| b.is_ascii_hexdigit())
    }
}

fn custom_pseudonymous_pattern_matches(pattern: &str, kid: &str) -> bool {
    let Some(inner) = pattern
        .strip_prefix('^')
        .and_then(|value| value.strip_suffix('$'))
    else {
        return kid == pattern;
    };
    let Some(literal) = anchored_literal_pattern(inner) else {
        return false;
    };
    kid == literal
}

fn anchored_literal_pattern(inner: &str) -> Option<String> {
    let mut literal = String::new();
    let mut escaped = false;
    for ch in inner.chars() {
        if escaped {
            literal.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if matches!(
            ch,
            '.' | '[' | ']' | '{' | '}' | '(' | ')' | '*' | '+' | '?' | '|' | '^' | '$'
        ) {
            return None;
        }
        literal.push(ch);
    }
    if escaped {
        literal.push('\\');
    }
    Some(literal)
}

/// A signature's cryptographic status plus deployment-trust result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignatureTrust {
    /// Content id of the signed frame.
    pub frame_id: Vec<u8>,
    /// Key id declared by the signature, if present.
    pub kid: Option<String>,
    /// Cryptographic verification status: `valid`, `invalid`, or `unverified`.
    pub status: String,
    /// True only when the signature is valid and the signer is trusted by the
    /// supplied [`TrustPolicy`].
    pub trusted: bool,
}

/// Severity assigned to a profile-policy finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// A profile-aware publication or verification tool should fail.
    Error,
    /// The graph is readable, but the profile signal is incomplete or weak.
    Warning,
    /// Informational result.
    Info,
}

impl Severity {
    /// Stable lowercase rendering used by CLI diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

/// One profile or trust-policy finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileFinding {
    /// Stable diagnostic code.
    pub code: String,
    /// Error, warning, or info severity.
    pub severity: Severity,
    /// Human-readable explanation.
    pub detail: String,
    /// Profile that produced the finding, when profile-specific.
    pub profile: Option<String>,
    /// Segment index in a multi-segment file, when evaluated segment-by-segment.
    pub segment_index: Option<usize>,
}

impl ProfileFinding {
    fn new(
        code: &str,
        severity: Severity,
        detail: impl Into<String>,
        profile: Option<&str>,
        segment_index: Option<usize>,
    ) -> Self {
        Self {
            code: code.to_string(),
            severity,
            detail: detail.into(),
            profile: profile.map(str::to_string),
            segment_index,
        }
    }
}

/// Evaluate deployment trust for already-verified signature statuses.
///
/// This does not verify COSE signatures. Callers should run the reader and
/// signature verifier first, then pass the folded graph here to classify valid
/// signatures against deployment trust anchors.
pub fn signature_trust(graph: &Graph, policy: Option<&TrustPolicy>) -> Vec<SignatureTrust> {
    let default_policy;
    let policy = match policy {
        Some(policy) => policy,
        None => {
            default_policy = TrustPolicy::default();
            &default_policy
        }
    };
    graph
        .signatures
        .iter()
        .map(|sig| SignatureTrust {
            frame_id: sig.frame_id.clone(),
            kid: sig.kid.clone(),
            status: sig.status.clone(),
            trusted: sig.status == "valid" && policy.is_trusted(sig.kid.as_deref()),
        })
        .collect()
}

/// Run supported profile checks without changing core reader validity.
///
/// Findings are policy-layer diagnostics. They never make a GTS byte stream
/// unreadable, but profile-aware tools can treat [`Severity::Error`] as a
/// publication or verification failure.
pub fn evaluate_profile_policy(
    graph: &Graph,
    policy: Option<&TrustPolicy>,
    segment_index: Option<usize>,
) -> Vec<ProfileFinding> {
    let default_policy;
    let policy = match policy {
        Some(policy) => policy,
        None => {
            default_policy = TrustPolicy::default();
            &default_policy
        }
    };
    let declared: BTreeSet<String> = if graph.segment_profiles.is_empty() {
        std::iter::once("generic".to_string()).collect()
    } else {
        graph.segment_profiles.iter().cloned().collect()
    };

    let mut findings = Vec::new();
    findings.extend(profile_vocab_findings(graph, &declared, segment_index));
    findings.extend(stream_vocab_findings(graph, segment_index));
    for profile in declared {
        if matches!(profile.as_str(), "evidence" | "opaque") {
            findings.extend(signature_policy_findings(
                graph,
                &profile,
                policy,
                segment_index,
            ));
        }
        if profile == "evidence" {
            findings.extend(evidence_head_findings(graph, segment_index));
        }
        if profile == "opaque" {
            findings.extend(opaque_recipient_findings(graph, policy, segment_index));
        }
    }
    findings
}

fn signature_policy_findings(
    graph: &Graph,
    profile: &str,
    policy: &TrustPolicy,
    segment_index: Option<usize>,
) -> Vec<ProfileFinding> {
    let mut findings = Vec::new();
    if graph.signatures.is_empty() {
        if profile == "evidence" && has_sealed_source(graph) {
            return findings;
        }
        return vec![ProfileFinding::new(
            "ProfileSignatureRequired",
            Severity::Error,
            format!("profile '{profile}' requires signed frames"),
            Some(profile),
            segment_index,
        )];
    }

    let invalid = graph
        .signatures
        .iter()
        .filter(|sig| sig.status == "invalid")
        .count();
    if invalid > 0 {
        findings.push(ProfileFinding::new(
            "ProfileSignatureInvalid",
            Severity::Error,
            format!("profile '{profile}' has {invalid} invalid signature(s)"),
            Some(profile),
            segment_index,
        ));
    }

    let unverified = graph
        .signatures
        .iter()
        .filter(|sig| sig.status == "unverified")
        .count();
    if unverified > 0 {
        findings.push(ProfileFinding::new(
            "ProfileSignatureUnverified",
            Severity::Error,
            format!("profile '{profile}' has {unverified} unresolved signature(s)"),
            Some(profile),
            segment_index,
        ));
    }

    let trust = signature_trust(graph, Some(policy));
    let valid: Vec<&SignatureTrust> = trust.iter().filter(|sig| sig.status == "valid").collect();
    let trusted = valid.iter().any(|sig| sig.trusted);
    if policy.require_trusted_signer && !trusted {
        findings.push(ProfileFinding::new(
            "ProfileSignerUntrusted",
            Severity::Error,
            format!("profile '{profile}' has no deployment-trusted valid signer"),
            Some(profile),
            segment_index,
        ));
    } else if !valid.is_empty() && policy.trusted_signers.is_empty() {
        findings.push(ProfileFinding::new(
            "ProfileSignerTrustNotEvaluated",
            Severity::Warning,
            format!(
                "profile '{profile}' signatures are cryptographically valid; \
                 no deployment trust policy was supplied"
            ),
            Some(profile),
            segment_index,
        ));
    }
    findings
}

fn evidence_head_findings(graph: &Graph, segment_index: Option<usize>) -> Vec<ProfileFinding> {
    if has_sealed_source(graph) {
        return Vec::new();
    }
    let heads: HashSet<&[u8]> = graph.segment_heads.iter().map(Vec::as_slice).collect();
    let valid_heads = signed_heads(&graph.signatures, "valid");
    let signed_heads = if valid_heads.is_empty() {
        signed_heads(&graph.signatures, "unverified")
    } else {
        valid_heads
    };
    if !heads.is_empty() && heads.is_disjoint(&signed_heads) {
        return vec![ProfileFinding::new(
            "EvidenceHeadCommitmentRequired",
            Severity::Error,
            "profile 'evidence' requires a signed segment head commitment",
            Some("evidence"),
            segment_index,
        )];
    }
    Vec::new()
}

fn signed_heads<'a>(signatures: &'a [Signature], status: &str) -> HashSet<&'a [u8]> {
    signatures
        .iter()
        .filter(|sig| sig.status == status)
        .map(|sig| sig.frame_id.as_slice())
        .collect()
}

fn has_sealed_source(graph: &Graph) -> bool {
    graph
        .quads
        .iter()
        .any(|&(_s, p, _o, _g)| term_iri_value(graph, p) == Some(SEALED_SOURCE))
}

fn opaque_recipient_findings(
    graph: &Graph,
    policy: &TrustPolicy,
    segment_index: Option<usize>,
) -> Vec<ProfileFinding> {
    let mut findings = Vec::new();
    for node in &graph.opaque {
        if let Some(recipients) = &node.recipients {
            for recipient in recipients {
                let kid = match recipient {
                    Value::Map(entries) => map_get(entries, "kid"),
                    _ => None,
                };
                match kid {
                    Some(Value::Text(kid)) if policy.is_pseudonymous_recipient(kid) => {}
                    Some(Value::Text(kid)) => findings.push(ProfileFinding::new(
                        "OpaqueRecipientKidPublic",
                        Severity::Error,
                        format!(
                            "opaque-profile high-privacy recipient kid must match {:?}, got {:?}",
                            policy.pseudonymous_kid_pattern, kid
                        ),
                        Some("opaque"),
                        segment_index,
                    )),
                    _ => findings.push(ProfileFinding::new(
                        "OpaqueRecipientKidMissing",
                        Severity::Error,
                        "opaque-profile recipient lacks a string kid",
                        Some("opaque"),
                        segment_index,
                    )),
                }
            }
        }
    }
    findings
}

fn namespace(iri: &str) -> &str {
    if let Some(i) = iri.rfind('#') {
        &iri[..=i]
    } else if let Some(i) = iri.rfind('/') {
        &iri[..=i]
    } else {
        iri
    }
}

fn term_iri_value(graph: &Graph, tid: usize) -> Option<&str> {
    graph
        .terms
        .get(tid)
        .and_then(|term| match (term.kind, term.value.as_deref()) {
            (TermKind::Iri, Some(value)) => Some(value),
            _ => None,
        })
}

fn used_vocabs(graph: &Graph) -> HashSet<&'static str> {
    let mut out = HashSet::new();
    let term_vocabs: Vec<Option<&'static str>> = graph
        .terms
        .iter()
        .map(|term| match (term.kind, term.value.as_deref()) {
            (TermKind::Iri, Some(iri)) => {
                let ns = namespace(iri);
                PROFILE_VOCABS
                    .iter()
                    .find(|&&(_, vocab)| ns == vocab)
                    .map(|&(_, vocab)| vocab)
            }
            _ => None,
        })
        .collect();
    for &(s, p, o, g) in &graph.quads {
        for tid in [Some(s), Some(p), Some(o), g].into_iter().flatten() {
            if let Some(&Some(vocab)) = term_vocabs.get(tid) {
                out.insert(vocab);
            }
        }
    }
    out
}

fn profile_vocab_findings(
    graph: &Graph,
    declared: &BTreeSet<String>,
    segment_index: Option<usize>,
) -> Vec<ProfileFinding> {
    let mut findings = Vec::new();
    let used = used_vocabs(graph);
    for &(profile, vocab) in PROFILE_VOCABS {
        let declares = declared.contains(profile);
        let uses = used.contains(vocab);
        if uses && !declares {
            findings.push(ProfileFinding::new(
                "ProfileVocabularyUndeclared",
                Severity::Error,
                format!("segment uses {vocab} vocabulary but does not declare '{profile}'"),
                Some(profile),
                segment_index,
            ));
        }
        if declares && !uses {
            findings.push(ProfileFinding::new(
                "ProfileVocabularyUnused",
                Severity::Warning,
                format!("segment declares '{profile}' but uses no {vocab} vocabulary"),
                Some(profile),
                segment_index,
            ));
        }
    }
    findings
}

fn stream_vocab_findings(graph: &Graph, segment_index: Option<usize>) -> Vec<ProfileFinding> {
    let claimed = graph.segment_streamable.iter().any(|info| info.claimed);
    if claimed {
        return Vec::new();
    }
    let term_uses_stream: Vec<bool> = graph
        .terms
        .iter()
        .map(|term| match (term.kind, term.value.as_deref()) {
            (TermKind::Iri, Some(iri)) => iri.starts_with(STREAM_NS),
            _ => false,
        })
        .collect();
    let uses = graph.quads.iter().any(|&(s, p, o, g)| {
        [Some(s), Some(p), Some(o), g]
            .into_iter()
            .flatten()
            .any(|tid| term_uses_stream.get(tid).copied().unwrap_or(false))
    });
    if !uses {
        return Vec::new();
    }
    vec![ProfileFinding::new(
        "StreamVocabularyWithoutLayout",
        Severity::Warning,
        format!(
            "segment uses {STREAM_NS} vocabulary but does not claim layout \
             'streamable' (§13.3)"
        ),
        Some("stream"),
        segment_index,
    )]
}
