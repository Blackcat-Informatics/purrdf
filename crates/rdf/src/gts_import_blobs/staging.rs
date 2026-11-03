// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Provisional payload writes, with both native digests computed in one pass.

use purrdf_gts::codec::{Codec, CodecError};
use purrdf_gts::model::{DiagnosticCause, retain_native_cause};
use std::io::{self, Read};

use super::{
    CollectedPayload, GtsBlobOmissionKind, GtsBlobStager, GtsStagedPayload, RdfDiagnostic,
};

pub(super) enum PayloadFailure {
    Codec(CodecError),
    Read(io::Error),
    Sink(RdfDiagnostic),
}

fn omission_kind(error: &CodecError) -> Option<GtsBlobOmissionKind> {
    if matches!(error, CodecError::Limit { .. }) {
        Some(GtsBlobOmissionKind::ByteBudget)
    } else if error.is_capability_unavailable() {
        Some(GtsBlobOmissionKind::UnavailableCapability)
    } else {
        None
    }
}

impl PayloadFailure {
    fn omission_kind(&self) -> Option<GtsBlobOmissionKind> {
        match self {
            Self::Codec(error) => omission_kind(error),
            Self::Read(error) => {
                let mut source: Option<&(dyn std::error::Error + 'static)> = Some(error);
                while let Some(error) = source {
                    if let Some(error) = error.downcast_ref::<CodecError>() {
                        return omission_kind(error);
                    }
                    source = error.source();
                }
                None
            }
            Self::Sink(_) => None,
        }
    }

    pub(super) fn into_cause(
        self,
    ) -> Result<(Option<GtsBlobOmissionKind>, DiagnosticCause), RdfDiagnostic> {
        let kind = self.omission_kind();
        let cause = match self {
            Self::Codec(error) => retain_native_cause(error)?,
            Self::Read(error) => retain_native_cause(purrdf_hash::error::IoFailure::from(error))?,
            Self::Sink(error) => return Err(error),
        };
        Ok((kind, DiagnosticCause::Native(cause)))
    }
}

pub(super) fn stage_payload(
    sink: &mut dyn GtsBlobStager,
    codecs: &[Codec],
    encoded: impl Read,
    limit: usize,
) -> Result<(CollectedPayload, String), PayloadFailure> {
    let mut source = purrdf_gts::codec::decode_chain_reader(codecs, encoded, limit)
        .map_err(PayloadFailure::Read)?;
    let handle = sink.begin().map_err(PayloadFailure::Sink)?;
    let mut blake3 = purrdf_hash::blake3::Hasher::new();
    let mut sha256 = purrdf_core::ContentHasher::default();
    let mut len = 0usize;
    let mut scratch = [0; 8192];
    let result: Result<_, PayloadFailure> = (|| {
        loop {
            let count = match source.read(&mut scratch) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                result => result.map_err(PayloadFailure::Read)?,
            };
            if count == 0 {
                break;
            }
            sink.write(handle, &scratch[..count])
                .map_err(PayloadFailure::Sink)?;
            blake3.update(&scratch[..count]);
            sha256.update(&scratch[..count]);
            len = len
                .checked_add(count)
                .ok_or_else(|| PayloadFailure::Codec(CodecError::Limit { limit }))?;
        }
        Ok((
            CollectedPayload::Staged(GtsStagedPayload {
                handle,
                len,
                sha256: sha256.finish(),
            }),
            purrdf_gts::wire::digest_label(blake3.finalize().as_bytes()),
        ))
    })();
    if let Err(error) = &result
        && error.omission_kind().is_some()
    {
        sink.discard(handle).map_err(PayloadFailure::Sink)?;
    }
    result
}
