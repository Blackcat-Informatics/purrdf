// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A fixed-key, non-cryptographic hasher for in-memory tables.
//!
//! [`FixedHasher`] is a [`Hasher`](core::hash::Hasher) and [`FixedState`] the [`BuildHasher`]
//! that makes it, for `HashMap<K, V, FixedState>` and `HashSet<T, FixedState>`.
//!
//! ```
//! use core::hash::{BuildHasher, Hash, Hasher};
//! use std::collections::HashMap;
//!
//! use purrdf_hash::fixed::{FixedHasher, FixedState};
//!
//! let mut hasher = FixedHasher::default();
//! "http://example.org/s".hash(&mut hasher);
//! let once = hasher.finish();
//! assert_eq!(FixedState::new().hash_one("http://example.org/s"), once);
//!
//! let mut map: HashMap<&str, u32, FixedState> = HashMap::with_hasher(FixedState::new());
//! map.insert("http://example.org/p", 1);
//! assert_eq!(map["http://example.org/p"], 1);
//! ```
//!
//! # Fixed keys, and what they do not promise
//!
//! The keys are compile-time constants derived from `⌊2^64/φ⌋`. There is no
//! run-time seeding, so equal inputs hash equal across runs and processes of
//! one build, on every target including `wasm32-unknown-unknown`, which has
//! no entropy source. The value is still **a property of the build**. A
//! build whose target enables AES hashes byte slices longer than 16 bytes
//! with AES rounds, and every other build uses folded multiplies. Because of
//! that, a hash must never be persisted, sent anywhere or used as a content
//! address. Use a digest from this crate for any of those.
//!
//! The keys are public, so the hasher offers no resistance to inputs chosen
//! to collide. It is for tables over trusted or ordinary data.
//!
//! # The function
//!
//! The state is one 64-bit accumulator. Each write folds its input into the
//! accumulator with a *folded multiply*: the low and high halves of a
//! 128-bit product XOR-ed together, with a fixed odd key as the multiplier.
//! The paths are:
//!
//! * integers are one fold;
//! * slices of 0–16 bytes are two independent folds plus a length term;
//! * slices of 17–32 bytes use two folded products or two AES lanes;
//! * slices over 32 bytes use four independent lanes (folds, or AES rounds
//!   on an AES build);
//! * [`finish`](core::hash::Hasher::finish) is one final fold.
//!
//! On 32-bit targets the 128-bit product is built from 32-bit multiplies
//! with identical output.

use core::fmt;
use core::hash::BuildHasher;
use core::marker::PhantomData;

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
mod aes;
mod fold;
mod keys;
mod portable;

use fold::fold;
use keys::{FIN_M, FIN_X, K_A, K_B, LEN_M, LEN_X, SEED};

/// How a path compresses a slice of more than 16 bytes to two words.
pub(crate) trait Compress {
    /// The path's name, as the tests and bench print it.
    const NAME: &'static str;
    /// The portable terminal path folds its two compressed words separately.
    const TERMINAL_FOLD_LANES: bool;
    /// Two words standing for `bytes`, which is longer than 16 bytes.
    fn compress_long(bytes: &[u8]) -> (u64, u64);
}

/// Folded multiplies on every length.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Portable;

impl Compress for Portable {
    const NAME: &'static str = "portable";
    const TERMINAL_FOLD_LANES: bool = true;
    #[inline]
    fn compress_long(bytes: &[u8]) -> (u64, u64) {
        portable::compress_long(bytes)
    }
}

/// AES rounds on long slices; exists only in builds whose target enables AES.
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
#[derive(Clone, Copy, Debug)]
pub(crate) struct Aes;

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
impl Compress for Aes {
    const NAME: &'static str = "aes";
    const TERMINAL_FOLD_LANES: bool = false;
    #[inline]
    fn compress_long(bytes: &[u8]) -> (u64, u64) {
        aes::compress_long(bytes)
    }
}

/// The path this build's [`FixedHasher`] runs.
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
pub(crate) type Selected = Aes;

/// The path this build's [`FixedHasher`] runs.
#[cfg(not(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
)))]
pub(crate) type Selected = Portable;

/// The accumulator and its update rules, over one compression path.
pub(crate) struct Engine<P> {
    acc: u64,
    path: PhantomData<P>,
}

impl<P> Clone for Engine<P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P> Copy for Engine<P> {}

impl<P: Compress> fmt::Debug for Engine<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Engine")
            .field("path", &P::NAME)
            .field("acc", &format_args!("{:#018x}", self.acc))
            .finish()
    }
}

/// The length term: outside every multiply, so no choice of bytes can cancel
/// a difference in length linearly.
#[inline]
const fn length_term(len: usize) -> u64 {
    fold(len as u64 ^ LEN_X, LEN_M)
}

impl<P: Compress> Engine<P> {
    pub(crate) const fn new() -> Self {
        Self {
            acc: SEED,
            path: PhantomData,
        }
    }

    #[inline]
    pub(crate) const fn word(&mut self, word: u64) {
        self.acc = fold(self.acc ^ word, K_A);
    }

    #[inline]
    const fn pair(&mut self, first: u64, second: u64, extra: u64) {
        self.acc = fold(self.acc ^ first, K_A) ^ fold(second, K_B) ^ extra;
    }

    #[inline]
    pub(crate) fn wide(&mut self, word: u128) {
        self.pair(word as u64, (word >> 64) as u64, 0);
    }

    #[inline]
    pub(crate) fn bytes(&mut self, bytes: &[u8]) {
        let (first, second) = if bytes.len() <= 16 {
            portable::pack_short(bytes)
        } else {
            P::compress_long(bytes)
        };
        self.pair(first, second, length_term(bytes.len()));
    }

    #[inline]
    pub(crate) const fn finish(self) -> u64 {
        fold(self.acc ^ FIN_X, FIN_M)
    }

    #[inline]
    pub(crate) fn terminal(tag: u8, bytes: &[u8]) -> u64 {
        if bytes.len() <= 16 {
            // Small key spaces need the two-stage streaming finalization to
            // satisfy the exhaustive two-byte avalanche check.
            let mut hasher = Self::new();
            hasher.word(u64::from(tag));
            hasher.bytes(bytes);
            return hasher.finish();
        }
        let (first, second) = P::compress_long(bytes);
        // A tag and one byte slice are the entire key: there is no later
        // field to separate. Each compressed word has already mixed its own
        // input. The portable path folds them separately before one final
        // fold; the AES path folds their combined words once.
        let domain = SEED
            ^ FIN_X
            ^ u64::from(tag).wrapping_mul(K_A)
            ^ (bytes.len() as u64).wrapping_mul(LEN_M);
        if P::TERMINAL_FOLD_LANES {
            // Keep the two portable lanes nonlinear until after the folds:
            // rotating and XOR-ing their raw tail words first would let
            // sparse differences cancel before any final mixer could see them.
            let lanes = fold(first ^ domain, K_A) ^ fold(second ^ domain.rotate_left(31), K_B);
            fold(lanes, FIN_M)
        } else {
            fold(first ^ second.rotate_left(29) ^ domain, FIN_M)
        }
    }
}

macro_rules! hasher {
    ($(#[$meta:meta])* $name:ident, $path:ty) => {
        $(#[$meta])*
        #[derive(Clone, Debug)]
        pub struct $name(crate::fixed::Engine<$path>);

        impl $name {
            /// Hash a tagged byte slice that is the whole table key.
            ///
            /// This path avoids the extra state transitions of the streaming
            /// [`Hasher`](core::hash::Hasher). Its result is build-specific,
            /// non-cryptographic and must never be persisted or exposed as a
            /// content identity.
            #[must_use]
            #[inline]
            pub fn hash_terminal(tag: u8, bytes: &[u8]) -> u64 {
                crate::fixed::Engine::<$path>::terminal(tag, bytes)
            }
        }

        impl Default for $name {
            #[inline]
            fn default() -> Self {
                Self(crate::fixed::Engine::new())
            }
        }

        impl core::hash::Hasher for $name {
            #[inline]
            fn finish(&self) -> u64 {
                self.0.finish()
            }

            #[inline]
            fn write(&mut self, bytes: &[u8]) {
                self.0.bytes(bytes);
            }

            #[inline]
            fn write_u8(&mut self, value: u8) {
                self.0.word(u64::from(value));
            }

            #[inline]
            fn write_u16(&mut self, value: u16) {
                self.0.word(u64::from(value));
            }

            #[inline]
            fn write_u32(&mut self, value: u32) {
                self.0.word(u64::from(value));
            }

            #[inline]
            fn write_u64(&mut self, value: u64) {
                self.0.word(value);
            }

            #[inline]
            fn write_u128(&mut self, value: u128) {
                self.0.wide(value);
            }

            /// Zero-extended to 64 bits, so a `usize` hashes the same on
            /// 32- and 64-bit targets.
            #[inline]
            fn write_usize(&mut self, value: usize) {
                self.0.word(value as u64);
            }
        }
    };
}
pub(crate) use hasher;

hasher!(
    /// The fixed-key table hasher. See the [module documentation](self).
    ///
    /// `FixedHasher::default()` always starts from the same compile-time
    /// state. Integers of every width are zero-extended to 64 bits, so
    /// `write_u8(1)` and `write_u64(1)` feed the same word.
    FixedHasher,
    Selected
);

/// Builds [`FixedHasher`]s: the `S` of `HashMap<K, V, S>`.
///
/// Every `FixedState` is equal and builds the same hasher, so maps that hold
/// one can be cloned and compared freely.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FixedState {
    _keys: (),
}

impl FixedState {
    /// The builder. It is the same as [`FixedState::default`].
    #[must_use]
    pub const fn new() -> Self {
        Self { _keys: () }
    }
}

impl BuildHasher for FixedState {
    type Hasher = FixedHasher;

    #[inline]
    fn build_hasher(&self) -> FixedHasher {
        FixedHasher::default()
    }
}

/// The name of the path this build's [`FixedHasher`] runs.
pub(crate) const SELECTED_NAME: &str = <Selected as Compress>::NAME;
