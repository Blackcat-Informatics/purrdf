// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Where the canonical lines go, in their final order.
//!
//! One producer yields each line of the canonical document, sorted and
//! deduplicated, and a sink consumes it. The document sink is the result
//! builder, which keeps the bytes. A digest sink feeds the same bytes to a hash
//! and keeps nothing, so a run that only needs a digest never reserves or
//! builds the document.

use sha2::{Digest, Sha256};

use purrdf_lex::allocation::StorageError;

use super::CanonError;
use super::control::SearchRun;
use super::result::ResultBuilder;
use crate::ViewTermId;

/// Bytes hashed between two polls of the caller's stop signal.
const CHUNK: usize = 64 * 1024;

/// A consumer of the canonical lines, called once per line in document order.
pub(super) trait LineSink<E> {
    /// The document's total length, known before its first line.
    fn begin(&mut self, total: usize) -> Result<(), CanonError<E>>;

    /// The next line, `'\n'`-terminated.
    fn line(&mut self, line: &str) -> Result<(), CanonError<E>>;
}

impl<Id: ViewTermId, E: Send + Sync + 'static> LineSink<E> for ResultBuilder<Id, E> {
    fn begin(&mut self, total: usize) -> Result<(), CanonError<E>> {
        self.reserve_text(total)
    }

    fn line(&mut self, line: &str) -> Result<(), CanonError<E>> {
        self.append(line)
    }
}

/// The sink of a run that stops before any line is spelled.
pub(super) struct Discard;

impl<E> LineSink<E> for Discard {
    fn begin(&mut self, _: usize) -> Result<(), CanonError<E>> {
        Ok(())
    }

    fn line(&mut self, _: &str) -> Result<(), CanonError<E>> {
        Ok(())
    }
}

/// A running 256-bit hash.
pub(super) trait Absorb {
    fn absorb(&mut self, bytes: &[u8]);
    fn digest(self) -> [u8; 32];
}

impl Absorb for Sha256 {
    fn absorb(&mut self, bytes: &[u8]) {
        Digest::update(self, bytes);
    }

    fn digest(self) -> [u8; 32] {
        self.finalize().into()
    }
}

impl Absorb for purrdf_hash::blake3::Hasher {
    fn absorb(&mut self, bytes: &[u8]) {
        self.update(bytes);
    }

    fn digest(self) -> [u8; 32] {
        *self.finalize().as_bytes()
    }
}

/// The digest sink: every line is copied into one reused 64 KiB buffer, and each
/// full buffer is hashed and emptied; no line and no document is kept.
///
/// The stop signal is polled before each 64 KiB of the document is hashed, so a
/// long document stays interruptible however its lines fall, and the hasher is
/// fed in large blocks rather than one small update per line.
pub(super) struct Streamed<'run, H> {
    hasher: H,
    run: &'run SearchRun<'run>,
    block: Box<[u8]>,
    filled: usize,
}

impl<'run, H: Absorb> Streamed<'run, H> {
    pub(super) fn new<E>(hasher: H, run: &'run SearchRun<'run>) -> Result<Self, CanonError<E>> {
        let mut block = Vec::new();
        block
            .try_reserve_exact(CHUNK)
            .map_err(|_| CanonError::Storage(StorageError::AllocationFailed))?;
        block.resize(CHUNK, 0);
        Ok(Self {
            hasher,
            run,
            block: block.into_boxed_slice(),
            filled: 0,
        })
    }

    fn flush<E>(&mut self) -> Result<(), CanonError<E>> {
        self.run.check_stop().map_err(|_| CanonError::Stopped)?;
        self.hasher.absorb(&self.block[..self.filled]);
        self.filled = 0;
        Ok(())
    }

    /// Hash what remains, poll once more and return the digest of everything delivered.
    pub(super) fn finish<E>(mut self) -> Result<[u8; 32], CanonError<E>> {
        if self.filled != 0 {
            self.flush()?;
        }
        self.run.check_stop().map_err(|_| CanonError::Stopped)?;
        Ok(self.hasher.digest())
    }
}

impl<H: Absorb, E> LineSink<E> for Streamed<'_, H> {
    fn begin(&mut self, _: usize) -> Result<(), CanonError<E>> {
        Ok(())
    }

    fn line(&mut self, line: &str) -> Result<(), CanonError<E>> {
        let mut rest = line.as_bytes();
        // Most lines end inside the block.
        if rest.len() < CHUNK - self.filled {
            self.block[self.filled..self.filled + rest.len()].copy_from_slice(rest);
            self.filled += rest.len();
            return Ok(());
        }
        while !rest.is_empty() {
            let (now, later) = rest.split_at(rest.len().min(CHUNK - self.filled));
            self.block[self.filled..self.filled + now.len()].copy_from_slice(now);
            self.filled += now.len();
            rest = later;
            if self.filled == CHUNK {
                self.flush()?;
            }
        }
        Ok(())
    }
}
