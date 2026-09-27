// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The object-safe streaming interface every hasher in this crate implements.

/// The longest output of any [`Digest`] in this crate (SHA3-512), so a caller
/// dispatching through `&mut dyn Digest` can finalize into a stack buffer.
pub const MAX_OUTPUT_LEN: usize = 64;

/// A streaming hash function, usable as `&mut dyn Digest`.
///
/// Each hasher also has inherent `update`/`finalize` methods and a one-shot
/// `digest` function; this trait exists for callers that pick the algorithm
/// at run time.
///
/// ```
/// use purrdf_hash::{Digest, MAX_OUTPUT_LEN};
/// use purrdf_hash::{md5::Md5, sha1::Sha1, sha3::Sha3_256};
///
/// for hasher in [
///     &mut Md5::new() as &mut dyn Digest,
///     &mut Sha1::new(),
///     &mut Sha3_256::new(),
/// ] {
///     hasher.update(b"a");
///     hasher.update(b"bc");
///     let mut out = [0u8; MAX_OUTPUT_LEN];
///     let len = hasher.finalize_reset(&mut out);
///     assert_eq!(len, hasher.output_len());
/// }
/// ```
pub trait Digest {
    /// The number of bytes [`finalize_reset`](Self::finalize_reset) writes.
    fn output_len(&self) -> usize;

    /// Absorb `data`.
    fn update(&mut self, data: &[u8]);

    /// Write the digest of everything absorbed since the last reset into the
    /// first [`output_len`](Self::output_len) bytes of `out`, return that
    /// length, and reset the hasher to its initial state.
    ///
    /// # Panics
    ///
    /// If `out` is shorter than [`output_len`](Self::output_len).
    fn finalize_reset(&mut self, out: &mut [u8]) -> usize;

    /// Discard everything absorbed and return to the initial state.
    fn reset(&mut self);
}
