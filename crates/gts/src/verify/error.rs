// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native signature-verification findings, before human presentation.

use crate::{model::Diagnostic, openpgp::OpenPgpError};
use core::fmt;

/// A refusal to import the configured transport key.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum TransportKeyError {
    /// The original OpenPGP parser refusal.
    OpenPgp(OpenPgpError),
    /// The decoded public key is not a valid Ed25519 point.
    PublicKey {
        /// Original public (never secret) key bytes.
        key: [u8; 32],
        /// Original point-validation refusal.
        source: purrdf_ed25519::SignatureError,
    },
}

impl fmt::Display for TransportKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenPgp(error) => error.fmt(f),
            Self::PublicKey { source, .. } => source.fmt(f),
        }
    }
}

purrdf_lex::error_source!(TransportKeyError => {
    Self::OpenPgp(error) => error,
    Self::PublicKey { source, .. } => source,
});

impl purrdf_hash::error::StructuredError for TransportKeyError {
    purrdf_hash::error_visit! {
        OpenPgp(cause) => "gts-transport-key-openpgp" { "cause" => Cause(cause) },
        PublicKey { key, source } => "gts-transport-key-public" { "key" => Bytes(key), "cause" => Cause(source) },
    }
}

/// An original cryptographic or file-integrity verification finding.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum VerificationError {
    /// An explicitly configured trusted public verification key could not be
    /// loaded. The payload contains public material and structural diagnostics,
    /// never secret signing material.
    TrustedPublicKey(TransportKeyError),
    /// The file's declared transport key could not be loaded.
    EmbeddedKey {
        /// Original declared key identifier.
        kid: String,
        /// Original key-import refusal.
        cause: TransportKeyError,
    },
    /// The file declares no transport key.
    MissingTransportKey,
    /// The reader's complete original integrity diagnostic.
    Integrity(Diagnostic),
    /// Signatures failed cryptographic verification.
    Invalid {
        /// Number of invalid signatures.
        count: usize,
    },
    /// Signatures could not be checked because no key resolved.
    Unverified {
        /// Number of unresolved signatures.
        count: usize,
    },
    /// Signatures were required, but none were present.
    Unsigned,
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TrustedPublicKey(cause) => write!(f, "cannot load trusted key: {cause}"),
            Self::EmbeddedKey { cause, .. } => {
                write!(f, "cannot load embedded transport key: {cause}")
            }
            Self::MissingTransportKey => f.write_str("no gts:transportKey found in file metadata"),
            Self::Integrity(diagnostic) => write!(f, "{}: {}", diagnostic.code, diagnostic.detail),
            Self::Invalid { count } => write!(f, "{count} signature(s) invalid"),
            Self::Unverified { count } => {
                write!(f, "{count} signature(s) unverified (no key resolved)")
            }
            Self::Unsigned => f.write_str("no signed frames found"),
        }
    }
}

purrdf_lex::error_source!(VerificationError => {
    Self::TrustedPublicKey(cause) | Self::EmbeddedKey { cause, .. } => cause,
    Self::Integrity(diagnostic) => diagnostic,
});

impl purrdf_hash::error::StructuredError for VerificationError {
    purrdf_hash::error_visit! {
        TrustedPublicKey(cause) => "gts-verify-trusted-key" { "cause" => Cause(cause) },
        EmbeddedKey { kid, cause } => "gts-verify-embedded-key" { "kid" => Text(kid), "cause" => Cause(cause) },
        MissingTransportKey => "gts-verify-missing-transport-key" {},
        Integrity(diagnostic) => "gts-verify-integrity" { "diagnostic" => Record(diagnostic) },
        Invalid { count } => "gts-verify-invalid-signatures" { "count" => Unsigned(*count as u128) },
        Unverified { count } => "gts-verify-unresolved-signatures" { "count" => Unsigned(*count as u128) },
        Unsigned => "gts-verify-unsigned" {},
    }
}
