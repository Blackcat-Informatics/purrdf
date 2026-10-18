// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native key-envelope refusals. No secret key bytes are retained in an error.

use core::fmt;

macro_rules! rules {
    ($($variant:ident => ($code:literal, $message:literal)),+ $(,)?) => {
        /// A structural rule the supplied OpenPGP envelope violated.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum OpenPgpRule { $(#[doc = $message] $variant),+ }
        impl fmt::Display for OpenPgpRule {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self { $(Self::$variant => f.write_str($message)),+ }
            }
        }
        impl std::error::Error for OpenPgpRule {}
        impl purrdf_hash::error::StructuredError for OpenPgpRule {
            purrdf_hash::error_visit! { $($variant => $code {}),+ }
        }
    };
}

rules! {
    MissingBegin => ("gts-openpgp-missing-begin", "missing armor BEGIN line"),
    MissingEnd => ("gts-openpgp-missing-end", "missing armor END line"),
    EmptyArmor => ("gts-openpgp-empty-armor", "empty armor body"),
    InvalidBase64 => ("gts-openpgp-invalid-base64", "invalid base64 armor body"),
    TruncatedMpiLength => ("gts-openpgp-truncated-mpi-length", "truncated MPI length"),
    TruncatedMpiPayload => ("gts-openpgp-truncated-mpi-payload", "truncated MPI payload"),
    TruncatedPacketHeader => ("gts-openpgp-truncated-header", "truncated packet header"),
    InvalidPacketTag => ("gts-openpgp-invalid-tag", "invalid packet tag octet"),
    TruncatedNewLength => ("gts-openpgp-truncated-new-length", "truncated new-format length octet"),
    TruncatedNewLength2 => ("gts-openpgp-truncated-new-length-2", "truncated new-format 2-octet length"),
    TruncatedNewLength4 => ("gts-openpgp-truncated-new-length-4", "truncated new-format 4-octet length"),
    PartialBodyLength => ("gts-openpgp-partial-length", "partial body lengths are not supported"),
    TruncatedOldLength => ("gts-openpgp-truncated-old-length", "truncated old-format length octet"),
    TruncatedOldLength2 => ("gts-openpgp-truncated-old-length-2", "truncated old-format 2-octet length"),
    TruncatedOldLength4 => ("gts-openpgp-truncated-old-length-4", "truncated old-format 4-octet length"),
    IndeterminateLength => ("gts-openpgp-indeterminate-length", "indeterminate-length packets are not supported"),
    PacketBeyondInput => ("gts-openpgp-packet-beyond-input", "packet body exceeds input"),
    TruncatedPublicPacket => ("gts-openpgp-truncated-public", "truncated public-key packet"),
    TruncatedOid => ("gts-openpgp-truncated-oid", "truncated OID"),
    TruncatedSecretPacket => ("gts-openpgp-truncated-secret", "truncated secret-key packet"),
    SecretStructure => ("gts-openpgp-secret-structure", "unsupported secret-key packet structure"),
    KeyMismatch => ("gts-openpgp-key-mismatch", "OpenPGP secret key does not match public key material"),
    MissingPublicPacket => ("gts-openpgp-missing-public", "no public-key packet found"),
    MissingSecretPacket => ("gts-openpgp-missing-secret", "no secret-key packet found"),
}

/// An original OpenPGP key parse failure with typed native fields.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum OpenPgpError {
    /// A structural envelope/packet rule.
    Rule(OpenPgpRule),
    /// The public packet is too short or does not declare v4.
    PublicHeader {
        /// Original public packet byte count.
        actual: usize,
        /// Declared version, absent when the packet is empty.
        version: Option<u8>,
    },
    /// The packet declares an algorithm other than EdDSA.
    Algorithm {
        /// Original algorithm byte.
        actual: u8,
    },
    /// The supplied public curve OID is unsupported.
    Oid {
        /// Original public OID bytes.
        actual: Vec<u8>,
    },
    /// The supplied public MPI has an unsupported byte count.
    PublicMpiLength {
        /// Original MPI byte count.
        actual: usize,
    },
    /// The supplied secret MPI exceeds the seed width.
    SecretMpiLength {
        /// Original MPI byte count, including any leading zero.
        actual: usize,
        /// Byte count after the optional zero is removed.
        normalized: usize,
    },
    /// The native importer does not decrypt secret-key envelopes.
    EncryptedSecret {
        /// Original S2K usage byte.
        usage: u8,
    },
    /// The secret packet's checksum does not match its contents.
    SecretChecksum {
        /// Checksum declared in the supplied envelope.
        expected: u16,
        /// Checksum computed over the supplied encoded MPI.
        actual: u16,
    },
}

impl fmt::Display for OpenPgpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rule(rule) => rule.fmt(f),
            Self::PublicHeader { .. } => f.write_str("only OpenPGP v4 public keys are supported"),
            Self::Algorithm { actual } => write!(f, "unsupported public-key algorithm {actual}"),
            Self::Oid { actual } => write!(
                f,
                "unsupported curve OID {}",
                purrdf_hash::hex::Lower(actual)
            ),
            Self::PublicMpiLength { actual } => {
                write!(f, "unexpected Ed25519 public MPI length {actual}")
            }
            Self::SecretMpiLength { actual, .. } => {
                write!(f, "unexpected Ed25519 secret MPI length {actual}")
            }
            Self::EncryptedSecret { .. } => f.write_str(
                "encrypted secret keys are not supported; export an unencrypted Ed25519 secret key",
            ),
            Self::SecretChecksum { .. } => f.write_str("OpenPGP secret-key checksum mismatch"),
        }
    }
}

impl PartialEq for OpenPgpError {
    fn eq(&self, other: &Self) -> bool {
        // Preserve the previous message-based identity without formatting.
        match (self, other) {
            (Self::Rule(a), Self::Rule(b)) => a == b,
            (Self::PublicHeader { .. }, Self::PublicHeader { .. })
            | (Self::EncryptedSecret { .. }, Self::EncryptedSecret { .. })
            | (Self::SecretChecksum { .. }, Self::SecretChecksum { .. }) => true,
            (Self::Algorithm { actual: a }, Self::Algorithm { actual: b }) => a == b,
            (Self::Oid { actual: a }, Self::Oid { actual: b }) => a == b,
            (Self::PublicMpiLength { actual: a }, Self::PublicMpiLength { actual: b })
            | (Self::SecretMpiLength { actual: a, .. }, Self::SecretMpiLength { actual: b, .. }) => {
                a == b
            }
            _ => false,
        }
    }
}
impl Eq for OpenPgpError {}
purrdf_lex::error_source!(OpenPgpError { Rule });

impl purrdf_hash::error::StructuredError for OpenPgpError {
    purrdf_hash::error_visit! {
        Rule(rule) => "gts-openpgp-rule" { "rule" => Cause(rule) },
        PublicHeader { actual, version } => "gts-openpgp-public-header" {
            "actual" => ByteCount(*actual as u128),
            "version" => Value(version.map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Unsigned(u128::from(value)))),
            "expected_version" => Unsigned(4), "minimum_bytes" => ByteCount(6),
        },
        Algorithm { actual } => "gts-openpgp-algorithm" { "actual" => Unsigned(u128::from(*actual)), "expected" => Unsigned(u128::from(super::ED25519_ALGO)) },
        Oid { actual } => "gts-openpgp-oid" { "actual" => Bytes(actual), "expected" => Bytes(&super::ED25519_OID) },
        PublicMpiLength { actual } => "gts-openpgp-public-mpi-length" { "actual" => ByteCount(*actual as u128) },
        SecretMpiLength { actual, normalized } => "gts-openpgp-secret-mpi-length" {
            "actual" => ByteCount(*actual as u128), "normalized" => ByteCount(*normalized as u128), "maximum" => ByteCount(32),
        },
        EncryptedSecret { usage } => "gts-openpgp-encrypted-secret" { "usage" => Unsigned(u128::from(*usage)) },
        SecretChecksum { expected, actual } => "gts-openpgp-secret-checksum" {
            "expected" => Unsigned(u128::from(*expected)), "actual" => Unsigned(u128::from(*actual)),
        },
    }
}
