// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::{IV256, compress32};
use crate::{
    Backend, SecretArray,
    block::{pad_md, pad_md_length_block},
};

pub(crate) const LANES: usize = 16;
pub(crate) const STRIDE: usize = 512;
pub(crate) const BYTES: usize = LANES * STRIDE;

/// Execution paths for independent short SHA-256 messages.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Sha256BatchBackend {
    /// Hash 16 independent lanes with the x86 AVX-512 kernel.
    X86Avx512,
    /// Hash each message with the portable compressor.
    Portable,
}

impl Backend for Sha256BatchBackend {
    const ALL: &'static [Self] = &[Self::X86Avx512, Self::Portable];

    fn is_available(self) -> bool {
        match self {
            Self::Portable => true,
            Self::X86Avx512 => {
                #[cfg(target_arch = "x86_64")]
                {
                    crate::arch::sha256_batch::sha256_batch_available()
                }
                #[cfg(not(target_arch = "x86_64"))]
                {
                    false
                }
            }
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::X86Avx512 => "x86-avx512",
            Self::Portable => "portable",
        }
    }
}

/// Bounded batch of independent short SHA-256 messages in insertion order.
#[doc(hidden)]
pub struct Sha256Batch {
    bytes: SecretArray<u8, BYTES>,
    blocks: [i32; LANES],
    count: usize,
}

impl core::fmt::Debug for Sha256Batch {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Sha256Batch").finish_non_exhaustive()
    }
}

impl Sha256Batch {
    /// Maximum number of accepted messages between drains.
    pub const CAPACITY: usize = LANES;
    /// Maximum message length, including all input spans but not padding.
    pub const MAX_INPUT_LEN: usize = STRIDE - 9;

    /// An empty batch with inline storage for all lanes.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: SecretArray::new([0; BYTES]),
            blocks: [0; LANES],
            count: 0,
        }
    }

    /// Whether this host can advance independent lanes in parallel.
    #[must_use]
    pub fn is_parallel() -> bool {
        Sha256BatchBackend::selected() == Sha256BatchBackend::X86Avx512
    }

    /// Append one message. A full batch or an oversized message returns
    /// `false` without changing any accepted messages.
    pub fn push<'a>(&mut self, spans: impl IntoIterator<Item = &'a [u8]>) -> bool {
        if self.count == LANES {
            return false;
        }
        let start = self.count * STRIDE;
        let slot = &mut self.bytes[start..start + STRIDE];
        let mut filled = 0;
        for span in spans {
            if span.len() > Self::MAX_INPUT_LEN - filled {
                return false;
            }
            let end = filled + span.len();
            slot[filled..end].copy_from_slice(span);
            filled = end;
        }
        let block = filled / 64 * 64;
        let length = ((filled as u64) * 8).to_be_bytes();
        let mut end = block + 64;
        if pad_md(&mut slot[block..end], filled - block, &length) {
            pad_md_length_block(&mut slot[end..end + 64], &length);
            end += 64;
        }
        self.blocks[self.count] = (end / 64) as i32;
        self.count += 1;
        true
    }

    /// Drain the batch in insertion order. Only the first returned count
    /// entries correspond to inputs.
    pub fn finish_reset(&mut self) -> ([[u8; 32]; LANES], usize) {
        // The selected path has already passed the same feature check.
        self.finish_reset_with_backend(Sha256BatchBackend::selected())
            .expect("the selected batch backend remains available")
    }

    /// Drain with the named backend, or return `None` without modifying the
    /// batch if this build and processor do not support it.
    pub fn finish_reset_with_backend(
        &mut self,
        backend: Sha256BatchBackend,
    ) -> Option<([[u8; 32]; LANES], usize)> {
        match backend {
            Sha256BatchBackend::Portable => Some(self.finish_reset_portable()),
            Sha256BatchBackend::X86Avx512 => {
                #[cfg(target_arch = "x86_64")]
                {
                    let out = crate::arch::sha256_batch::sha256_batch(self)?;
                    Some((out, core::mem::take(&mut self.count)))
                }
                #[cfg(not(target_arch = "x86_64"))]
                {
                    None
                }
            }
        }
    }

    fn finish_reset_portable(&mut self) -> ([[u8; 32]; LANES], usize) {
        let count = core::mem::take(&mut self.count);
        let mut out = [[0; 32]; LANES];
        for (lane, digest) in out[..count].iter_mut().enumerate() {
            let start = lane * STRIDE;
            let end = start + self.blocks[lane] as usize * 64;
            let mut state = IV256;
            compress32(&mut state, &self.bytes[start..end]);
            for (word, bytes) in state.into_iter().zip(digest.as_chunks_mut::<4>().0) {
                *bytes = word.to_be_bytes();
            }
        }
        (out, count)
    }

    /// The accepted prefix has at most LANES entries. Each accepted lane has
    /// one to STRIDE / 64 complete blocks, including padding. All bytes, even
    /// those outside that prefix, remain initialized for unmasked gathers.
    #[cfg(target_arch = "x86_64")]
    pub(crate) fn padded_lanes(&self) -> (&[u8; BYTES], &[i32; LANES], usize) {
        (&self.bytes, &self.blocks, self.count)
    }
}

crate::default_from_new!(Sha256Batch);
