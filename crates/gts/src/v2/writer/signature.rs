// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt;
use std::io::{self, Write};

use purrdf_lex::cbor::head;

use super::{
    Buffer, Digest, FrameType, SegmentWriteError, SegmentWriter, write_commit_content,
    write_frame_body,
};
use crate::cose::{SigningKeyRef, composite};
use crate::mldsa65;
use crate::v2::signature::{
    COMPOSITE, ED25519, SignatureRecord, key_identifier, signature_message,
};
use crate::v2::{HeldKey, SignatureInputs, SignatureStatus};

/// Failure preparing a signature list before a commit is appended.
#[derive(Debug)]
#[non_exhaustive]
pub enum SignatureWriteError {
    /// The same algorithm and public key were supplied twice.
    DuplicateKey {
        /// GTS registry algorithm code.
        algorithm: u64,
        /// Identity of the duplicated key.
        key_id: Digest,
    },
    /// The frame or signature bytes could not be staged.
    Preparation(io::Error),
    /// The native signing operation failed.
    Signing(composite::Error),
    /// The native self-verification operation could not complete.
    Verification(mldsa65::Error),
    /// The new signature failed the wire verification policy.
    Rejected(SignatureStatus),
}

impl fmt::Display for SignatureWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateKey { algorithm, key_id } => write!(
                f,
                "duplicate GTS signature key {} for algorithm {algorithm}",
                purrdf_hash::hex::encode(key_id)
            ),
            Self::Preparation(error) => {
                write!(f, "could not prepare the GTS signed commit ({error})")
            }
            Self::Signing(error) => write!(f, "could not sign the GTS commit ({error})"),
            Self::Verification(error) => {
                write!(f, "could not verify the GTS commit signature ({error})")
            }
            Self::Rejected(status) => {
                write!(f, "GTS commit signature self-check returned {status:?}")
            }
        }
    }
}

purrdf_lex::error_source!(SignatureWriteError {
    Preparation,
    Signing,
    Verification
});

impl purrdf_hash::error::StructuredError for SignatureWriteError {
    purrdf_hash::error_visit! {
        DuplicateKey { algorithm, key_id } => "gts-v2-duplicate-signing-key" {
            "algorithm" => Unsigned(u128::from(*algorithm)), "key_id" => Bytes(key_id),
        },
        Preparation(cause) => "gts-v2-signature-preparation" { "cause" => Cause(cause) },
        Signing(cause) => "gts-v2-signing-operation" { "cause" => Cause(cause) },
        Verification(cause) => "gts-v2-signature-check-operation" { "cause" => Cause(cause) },
        Rejected(_status) => "gts-v2-signature-self-check" {},
    }
}

purrdf_lex::variant_from!(SignatureWriteError {
    Preparation(io::Error), Signing(composite::Error), Verification(mldsa65::Error),
});
purrdf_lex::variant_from!(SegmentWriteError { Signature(SignatureWriteError) });

impl<W: Write> SegmentWriter<W> {
    /// Append a commit with deterministic, individually self-verified signatures.
    ///
    /// Supply dedicated component keys for composite signers. The signature
    /// records are written in deterministic order. An empty signer list
    /// writes an unsigned commit.
    /// # Errors
    /// A preparation failure leaves the current head and sink intact. A
    /// failed sink write prevents further appends.
    pub fn add_signed_commit(
        &mut self,
        signers: &[SigningKeyRef<'_>],
    ) -> Result<Digest, SegmentWriteError> {
        if signers.is_empty() {
            return self.add_commit();
        }

        let predecessor = self.append_head()?;
        let commit = SignedCommit::prepare(&self.opener, &predecessor, &self.range, signers)?;
        self.append_with(|sink, _| {
            sink.write_all(commit.body())?;
            sink.write_all(commit.signatures())?;
            Ok(commit.identifier)
        })
    }
}

pub(super) struct SignedCommit {
    pub(super) identifier: Digest,
    body: [u8; 121],
    body_length: usize,
    signatures: Buffer,
}

impl SignedCommit {
    pub(super) fn prepare(
        opener: &Digest,
        predecessor: &Digest,
        range: &crate::v2::Range,
        signers: &[SigningKeyRef<'_>],
    ) -> Result<Self, SegmentWriteError> {
        let count = range.count().get() - 1;
        let root = range.root();
        // Even the longest count occupies nine bytes. With fixed digests and
        // an empty payload, the frame body ends within 121 bytes.
        let mut body = [0; 121];
        let mut tail = body.as_mut_slice();
        let identifier = write_frame_body(&mut tail, predecessor, FrameType::Commit, &[], |out| {
            write_commit_content(out, count, &root)
        })
        .map_err(SignatureWriteError::Preparation)?;
        let remaining = tail.len();
        let signatures = prepare_signatures(signers, opener, &identifier)?;
        Ok(Self {
            body_length: body.len() - remaining,
            body,
            identifier,
            signatures,
        })
    }

    pub(super) fn body(&self) -> &[u8] {
        &self.body[..self.body_length]
    }

    pub(super) fn signatures(&self) -> &[u8] {
        &self.signatures.0
    }
}

struct Signer<'a> {
    algorithm: u64,
    key_id: Digest,
    key: SigningKeyRef<'a>,
}

impl<'a> Signer<'a> {
    fn from_key(key: SigningKeyRef<'a>) -> Self {
        let (algorithm, key_id) = match key {
            SigningKeyRef::Ed25519(key) => (
                ED25519,
                key_identifier(ED25519, key.verifying_key().as_bytes()),
            ),
            SigningKeyRef::Composite(key) => (
                COMPOSITE,
                key_identifier(COMPOSITE, &key.verifying_key().to_bytes()),
            ),
        };
        Self {
            algorithm,
            key_id,
            key,
        }
    }

    fn sign(
        &self,
        header: &Digest,
        frame: &Digest,
        output: &mut Buffer,
    ) -> Result<(), SignatureWriteError> {
        let message = signature_message(self.algorithm, &self.key_id, header, frame);
        match self.key {
            SigningKeyRef::Ed25519(key) => self.verify_record(
                header,
                frame,
                key.verifying_key().as_bytes(),
                &key.sign(&message).to_bytes(),
                output,
            ),
            SigningKeyRef::Composite(key) => self.verify_record(
                header,
                frame,
                &key.verifying_key().to_bytes(),
                &key.sign_deterministic(&message)?.to_bytes(),
                output,
            ),
        }
    }

    fn verify_record(
        &self,
        header: &Digest,
        frame: &Digest,
        public_key: &[u8],
        value: &[u8],
        output: &mut Buffer,
    ) -> Result<(), SignatureWriteError> {
        let held = HeldKey {
            key_id: self.key_id,
            algorithm: self.algorithm,
            public_key,
        };
        let record = SignatureRecord {
            algorithm: self.algorithm,
            key_id: &self.key_id,
            value,
        };
        let status = SignatureInputs::verify_held(&[held], &record, header, frame)?;
        if status != SignatureStatus::Valid {
            return Err(SignatureWriteError::Rejected(status));
        }
        head::write_head(output, head::ARRAY, 3)?;
        head::write_head(output, head::UNSIGNED, self.algorithm)?;
        head::write_bytes(output, &self.key_id)?;
        head::write_bytes(output, value)?;
        Ok(())
    }
}

fn prepare_signatures(
    keys: &[SigningKeyRef<'_>],
    header: &Digest,
    frame: &Digest,
) -> Result<Buffer, SignatureWriteError> {
    let mut signers = Vec::new();
    signers
        .try_reserve_exact(keys.len())
        .map_err(io::Error::other)?;
    signers.extend(keys.iter().copied().map(Signer::from_key));
    signers.sort_unstable_by_key(|signer| (signer.algorithm, signer.key_id));
    for pair in signers.windows(2) {
        if pair[0].algorithm == pair[1].algorithm && pair[0].key_id == pair[1].key_id {
            return Err(SignatureWriteError::DuplicateKey {
                algorithm: pair[0].algorithm,
                key_id: pair[0].key_id,
            });
        }
    }
    let mut output = Buffer::default();
    head::write_head(&mut output, head::ARRAY, keys.len() as u64)?;
    for signer in &signers {
        signer.sign(header, frame, &mut output)?;
    }
    Ok(output)
}
