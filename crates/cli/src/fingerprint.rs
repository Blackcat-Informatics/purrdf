// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Full identities and native human comparison aids remain separate values.
use crate::error::CliError;
use purrdf_core::visual_fingerprint::{self, Request};
use purrdf_lex::allocation::{Admission, StorageError};
use std::cell::Cell;
use std::io::Write as _;

/// Caller ceiling that refused, before native output publication.
#[derive(Clone, Copy, Debug)]
pub struct Exhaustion {
    /// Native resource.
    pub resource: &'static str,
    // Preserve exact u128 counts without imposing their alignment on CliError.
    actual: [u8; 16],
    limit: [u8; 16],
}
impl Exhaustion {
    /// Exact requested native amount.
    pub fn actual(self) -> u128 {
        u128::from_le_bytes(self.actual)
    }
    /// Exact caller ceiling.
    pub fn limit(self) -> u128 {
        u128::from_le_bytes(self.limit)
    }
}
/// Original presentation or output refusal.
#[derive(Debug)]
pub enum Failure {
    /// Original native cause and actionable caller ceiling when tripped.
    Native {
        /// Unchanged renderer or hexadecimal refusal.
        cause: visual_fingerprint::Error,
        /// Original caller limit finding.
        exhaustion: Option<Exhaustion>,
    },
    /// Completed presentation could not be written.
    Io(std::io::Error),
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native { cause, exhaustion } => {
                cause.fmt(f)?;
                if let Some(limit) = exhaustion {
                    write!(
                        f,
                        "; {} requires {}, limit {}",
                        limit.resource,
                        limit.actual(),
                        limit.limit()
                    )?;
                }
                Ok(())
            }
            Self::Io(cause) => cause.fmt(f),
        }
    }
}
impl std::error::Error for Failure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Native { cause, .. } => cause,
            Self::Io(cause) => cause,
        })
    }
}
struct Limits<'a> {
    bytes: usize,
    work: u64,
    used: u128,
    refusal: &'a Cell<Option<Exhaustion>>,
}
impl Limits<'_> {
    fn check(&self, resource: &'static str, actual: u128, limit: u128) -> Result<(), StorageError> {
        if actual > limit {
            self.refusal.set(Some(Exhaustion {
                resource,
                actual: actual.to_le_bytes(),
                limit: limit.to_le_bytes(),
            }));
            Err(StorageError::AdmissionFailed)
        } else {
            Ok(())
        }
    }
    fn failure(&self, cause: visual_fingerprint::Error) -> CliError {
        CliError::Fingerprint(Failure::Native {
            cause,
            exhaustion: self.refusal.get(),
        })
    }
}
impl Admission for Limits<'_> {
    fn progress_enabled(&self) -> bool {
        true
    }
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        self.check("rendered storage bytes", bytes as u128, self.bytes as u128)
    }
    fn checkpoint(&mut self, units: u64) -> Result<(), StorageError> {
        self.used += u128::from(units);
        self.check("native work", self.used, u128::from(self.work))
    }
}
pub(crate) fn run(
    digest: &str,
    symbols: usize,
    max_bytes: usize,
    max_work: u64,
) -> Result<(), CliError> {
    let refusal = Cell::new(None);
    let mut limits = Limits {
        bytes: max_bytes,
        work: max_work,
        used: 0,
        refusal: &refusal,
    };
    let request = Request::new(symbols, "BLAKE3 256").map_err(|cause| limits.failure(cause))?;
    let digest = visual_fingerprint::parse_digest(digest, &mut limits)
        .map_err(|cause| limits.failure(cause))?;
    let presentation = visual_fingerprint::present(digest, request, limits).map_err(|cause| {
        CliError::Fingerprint(Failure::Native {
            cause,
            exhaustion: refusal.get(),
        })
    })?;
    let rendering = presentation.rendering();
    let output = writeln!(
        std::io::stdout().lock(),
        "digest {digest}\nscheme {}\nsymbols {}\nemoji {}\nspoken {}\nrandomart\n{}",
        rendering.scheme(),
        rendering.symbols(),
        rendering.emoji(),
        rendering.spoken(),
        rendering.randomart()
    );
    let cleanup = presentation.release();
    output.map_err(|cause| CliError::Fingerprint(Failure::Io(cause)))?;
    cleanup.map_err(|cause| {
        CliError::Fingerprint(Failure::Native {
            cause: cause.into(),
            exhaustion: refusal.get(),
        })
    })
}
