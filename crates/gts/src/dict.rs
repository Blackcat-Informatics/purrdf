// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic in-band pack dictionaries for the GTS `zstd` `dct` codec
//! (GTS-SPEC §5 header `"dct"`, §8.5 `zstd` `dct` parameter).
//!
//! A compacted pack pins its dictionary **uncompressed and in-band**; the reader
//! decodes dict-compressed frames against those exact bytes, so a dictionary must
//! be a **pure function of the batched corpus** — the GTS writer is byte
//! deterministic, and a nondeterministic dictionary would break that invariant.
//!
//! Both producers emit a **finalized** zstd dictionary (magic number, a non-zero
//! dict-id, entropy tables, offset history, then content); the raw content alone
//! cannot prime a zstd encoder (`set_dictionary_from_bytes` rejects a zero id or
//! zero repeat offsets), so finalization is mandatory for the dictionary to be
//! usable on both the encode and decode paths. The two producers differ only in
//! how the dictionary *content* is selected:
//!
//! - [`raw_content_dict`] keeps a canonical trailing window of the corpus — no
//!   training, no randomness; trivially wasm-clean and deterministic.
//! - [`trained_dict`] runs pure-Rust FastCOVER
//!   ([`structured_zstd::dictionary`]) under an **explicit [`DictSeed`]**.
//!
//! # Why the seed is an explicit parameter
//!
//! FastCOVER's reservoir sampler draws from `fastrand`'s thread-local RNG and
//! upstream `FastCoverOptions` exposes no seed field, so the sampler's stream is
//! determined by ambient thread-local state. [`trained_dict`] closes that hole
//! from both ends:
//!
//! 1. The seed is a **required argument** ([`DictSeed`]) — never implicit, so a
//!    caller that wants a specific dictionary says so, and two corpora that
//!    should share a dictionary can be trained under the same seed.
//! 2. The thread-local seed is **saved before and restored after** training
//!    (`fastrand::get_seed`/`fastrand::seed` round-trip the generator state
//!    exactly), so the call leaves no ambient trace on the caller's RNG stream
//!    and observes none of the caller's.
//!
//! Because `fastrand`'s generator is thread-*local*, two threads training
//! concurrently never share it, and (1)+(2) make each call a pure function of
//! `(corpus, target_len, seed)` regardless of what any other `fastrand` user on
//! any thread — including the same one, before or after — is doing. That is the
//! property [`trained_dict`]'s determinism claim actually needs, and it is
//! covered by a concurrent test rather than a single-threaded caveat.
//!
//! MEASURED against `structured-zstd` 0.0.49: its reservoir sampler's skip loop
//! reads into a zero-length buffer and therefore terminates immediately, so the
//! sample it produces is the leading window of the source and the RNG draws do
//! not currently reach the dictionary bytes — two seeds yield the same
//! dictionary today. The seed parameter is NOT decorative because of that: it
//! is what makes this module's determinism independent of that upstream detail.
//! If a later `structured-zstd` makes the sampler actually sample, the output
//! stays a pure function of `(corpus, target_len, seed)` instead of silently
//! becoming a function of whatever the calling thread last did with `fastrand`.

use structured_zstd::decoding::Dictionary;
use structured_zstd::dictionary::{
    FastCoverOptions, FinalizeOptions, create_fastcover_dict_from_source, finalize_raw_dict,
};

/// A dictionary could not be built from the supplied corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictError(pub String);

impl core::fmt::Display for DictError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "dictionary construction failed: {}", self.0)
    }
}

impl std::error::Error for DictError {}

/// Canonical, order-independent concatenation of the corpus.
///
/// Samples are sorted bytewise so the result is a pure function of the sample
/// *multiset*, not of the caller's iteration order. Duplicates are retained —
/// repetition is exactly the signal a dictionary should capture.
fn canonical_concat(corpus: &[&[u8]]) -> Vec<u8> {
    let mut ordered: Vec<&[u8]> = corpus.to_vec();
    ordered.sort_unstable();
    let total: usize = ordered.iter().map(|s| s.len()).sum();
    let mut out = Vec::with_capacity(total);
    for sample in ordered {
        out.extend_from_slice(sample);
    }
    out
}

/// Derive a deterministic 64-bit seed from the canonical corpus bytes.
fn derive_seed(concat: &[u8]) -> u64 {
    let hash = blake3::hash(concat);
    let bytes = hash.as_bytes();
    // BLAKE3 output is 32 bytes; the first eight are ample entropy for a seed.
    u64::from_le_bytes(
        bytes[..8]
            .try_into()
            .expect("BLAKE3 digest is 32 bytes, so 8 are always available"),
    )
}

/// Build a deterministic **raw-content** finalized dictionary from `corpus`.
///
/// The dictionary content is the canonical trailing window of the corpus (a zstd
/// raw dictionary is history the compressor sees before the payload, so the bytes
/// nearest the end are the cheapest matches); `finalize_raw_dict` truncates it to
/// the budget and layers on the magic, deterministic FNV dict-id, entropy tables
/// and offset history. No randomness is involved, so this is deterministic and
/// wasm-clean by construction.
///
/// # Errors
/// Returns [`DictError`] when the corpus is empty or `target_len` is too small to
/// hold the finalized header and offset history.
pub fn raw_content_dict(corpus: &[&[u8]], target_len: usize) -> Result<Vec<u8>, DictError> {
    let concat = canonical_concat(corpus);
    if concat.is_empty() {
        return Err(DictError("empty corpus".to_owned()));
    }
    finalize_raw_dict(&concat, &concat, target_len, FinalizeOptions::default())
        .map_err(|err| DictError(err.to_string()))
}

/// The seed FastCOVER's reservoir sampler runs under — always explicit.
///
/// There is no `Default`: a dictionary's bytes are a function of its seed, and
/// picking one silently is exactly the ambient-state coupling this type exists
/// to remove.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DictSeed {
    /// Derive the seed from the canonical corpus bytes (BLAKE3-256, first eight
    /// bytes little-endian). The dictionary is then a pure function of the
    /// sample multiset alone — the seed carries no information the corpus does
    /// not already fix, which is what a content-addressed pack wants.
    FromCorpus,
    /// An explicit caller-chosen seed. Two different corpora trained under the
    /// same seed draw the same sampler stream.
    Explicit(u64),
}

impl DictSeed {
    /// Resolve to the concrete `fastrand` seed for `concat`.
    fn resolve(self, concat: &[u8]) -> u64 {
        match self {
            Self::FromCorpus => derive_seed(concat),
            Self::Explicit(seed) => seed,
        }
    }
}

/// Restores the thread-local `fastrand` seed on drop, so a panic inside
/// FastCOVER cannot leave the caller's generator re-seeded.
struct RestoreFastrandSeed(u64);

impl Drop for RestoreFastrandSeed {
    fn drop(&mut self) {
        fastrand::seed(self.0);
    }
}

/// Build a deterministic **FastCOVER-trained** finalized dictionary from
/// `corpus` under an explicit `seed`.
///
/// The thread-local `fastrand` generator is seeded from `seed` for the duration
/// of training and restored afterwards (see the module docs), so the result is a
/// pure function of `(corpus, target_len, seed)` — reproducible under
/// concurrency and alongside other `fastrand` users. The trainer finalizes the
/// raw content into a full dictionary binary.
///
/// # Errors
/// Returns [`DictError`] when the corpus is empty or too small for FastCOVER to
/// train a dictionary of the requested size.
pub fn trained_dict(
    corpus: &[&[u8]],
    target_len: usize,
    seed: DictSeed,
) -> Result<Vec<u8>, DictError> {
    let concat = canonical_concat(corpus);
    if concat.is_empty() {
        return Err(DictError("empty corpus".to_owned()));
    }
    let mut out = Vec::new();
    let result = {
        let _restore = RestoreFastrandSeed(fastrand::get_seed());
        fastrand::seed(seed.resolve(&concat));
        create_fastcover_dict_from_source(
            concat.as_slice(),
            &mut out,
            target_len,
            &FastCoverOptions::default(),
            FinalizeOptions::default(),
        )
    };
    result.map_err(|err| DictError(err.to_string()))?;
    Ok(out)
}

/// The `Dictionary_ID` a finalized dictionary declares — the same value every
/// frame primed by it carries in its zstd frame header.
///
/// # Errors
/// Returns [`DictError`] when `dict` is not a parseable finalized zstd
/// dictionary.
pub fn dictionary_id(dict: &[u8]) -> Result<u32, DictError> {
    Dictionary::decode_dict(dict)
        .map(|parsed| parsed.id)
        .map_err(|err| DictError(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use structured_zstd::decoding::Dictionary;

    /// A corpus with enough repeated structure for FastCOVER to train on.
    fn sample_corpus() -> Vec<Vec<u8>> {
        (0..400u32)
            .map(|i| {
                format!(
                    "<https://example.org/s{}> <https://example.org/p> \"claim {} about cats\" .\n",
                    i % 37,
                    i
                )
                .into_bytes()
            })
            .collect()
    }

    fn as_slices(owned: &[Vec<u8>]) -> Vec<&[u8]> {
        owned.iter().map(Vec::as_slice).collect()
    }

    /// A finalized dictionary must parse via the zstd decoder — this is exactly
    /// the check that would fail on a bare raw-content blob.
    fn assert_is_valid_finalized_dict(dict: &[u8]) {
        Dictionary::decode_dict(dict).expect("output must be a valid finalized zstd dictionary");
    }

    #[test]
    fn raw_content_dict_is_deterministic_and_valid() {
        let owned = sample_corpus();
        let corpus = as_slices(&owned);
        let a = raw_content_dict(&corpus, 4096).expect("build");
        let b = raw_content_dict(&corpus, 4096).expect("build");
        assert_eq!(a, b, "raw-content dict must be byte-reproducible");
        assert!(!a.is_empty());
        assert!(a.len() <= 4096, "must respect the target length bound");
        assert_is_valid_finalized_dict(&a);
    }

    #[test]
    fn raw_content_dict_is_order_independent() {
        let owned = sample_corpus();
        let forward = as_slices(&owned);
        let mut reversed = forward.clone();
        reversed.reverse();
        assert_eq!(
            raw_content_dict(&forward, 4096).expect("build"),
            raw_content_dict(&reversed, 4096).expect("build"),
            "canonical ordering must ignore caller iteration order"
        );
    }

    #[test]
    fn trained_dict_is_deterministic_and_valid() {
        let owned = sample_corpus();
        let corpus = as_slices(&owned);
        let a = trained_dict(&corpus, 4096, DictSeed::FromCorpus).expect("training should succeed");
        let b = trained_dict(&corpus, 4096, DictSeed::FromCorpus).expect("training should succeed");
        assert_eq!(a, b, "seeded FastCOVER must be byte-reproducible");
        assert!(!a.is_empty());
        assert_is_valid_finalized_dict(&a);
    }

    #[test]
    fn trained_dict_is_order_independent() {
        let owned = sample_corpus();
        let forward = as_slices(&owned);
        let mut reversed = forward.clone();
        reversed.reverse();
        assert_eq!(
            trained_dict(&forward, 4096, DictSeed::FromCorpus).expect("train"),
            trained_dict(&reversed, 4096, DictSeed::FromCorpus).expect("train"),
            "the trained dict must be a pure function of the sample multiset"
        );
    }

    #[test]
    fn producers_reject_empty_corpus() {
        assert_eq!(
            trained_dict(&[], 4096, DictSeed::FromCorpus).expect_err("empty corpus is rejected"),
            DictError("empty corpus".to_owned())
        );
        assert_eq!(
            raw_content_dict(&[], 4096).expect_err("empty corpus must be rejected"),
            DictError("empty corpus".to_owned())
        );
    }

    /// The property that actually matters: the trained bytes do not depend on
    /// the caller's ambient `fastrand` state under ANY seed choice.
    ///
    /// This is deliberately NOT written as "two seeds give two dictionaries":
    /// see the module docs — upstream's reservoir currently consumes the RNG
    /// only vestigially, so that assertion would encode an upstream
    /// implementation detail rather than this module's contract.
    #[test]
    fn trained_bytes_ignore_the_callers_ambient_rng_state() {
        let owned = sample_corpus();
        let corpus = as_slices(&owned);

        fastrand::seed(1);
        let under_seed_one = trained_dict(&corpus, 4096, DictSeed::Explicit(7)).expect("train");
        fastrand::seed(0xFFFF_FFFF_FFFF_FFFF);
        let _ = fastrand::u64(..);
        let under_other_state = trained_dict(&corpus, 4096, DictSeed::Explicit(7)).expect("train");
        assert_eq!(
            under_seed_one, under_other_state,
            "an explicitly-seeded trained dict must not observe ambient RNG state"
        );

        // The declared seed is what governs, and it round-trips reproducibly.
        assert_eq!(
            trained_dict(&corpus, 4096, DictSeed::FromCorpus).expect("train"),
            trained_dict(
                &corpus,
                4096,
                DictSeed::Explicit(derive_seed(&canonical_concat(&corpus)))
            )
            .expect("train"),
            "FromCorpus must equal the explicit seed it derives"
        );
    }

    /// (g) The seeded trainer is reproducible when called concurrently from
    /// several threads WHILE other `fastrand` users churn the ambient
    /// thread-local generator — the property the old "single-threaded, seed the
    /// global" caveat could not offer.
    #[test]
    fn trained_dict_is_reproducible_under_concurrency_with_other_fastrand_users() {
        let owned = sample_corpus();
        let expected = {
            let corpus = as_slices(&owned);
            trained_dict(&corpus, 4096, DictSeed::Explicit(12345)).expect("train")
        };

        let noise = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let noisemakers: Vec<_> = (0..3)
            .map(|_| {
                let noise = std::sync::Arc::clone(&noise);
                std::thread::spawn(move || {
                    while noise.load(std::sync::atomic::Ordering::Relaxed) {
                        fastrand::seed(fastrand::u64(..));
                        let _ = fastrand::f64();
                    }
                })
            })
            .collect();

        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let owned = &owned;
                    scope.spawn(move || {
                        // Churn this thread's OWN generator first: `trained_dict`
                        // must observe none of it and leave none of its own.
                        fastrand::seed(0x5EED_5EED);
                        let before = fastrand::u64(..);
                        fastrand::seed(0x5EED_5EED);
                        let corpus = as_slices(owned);
                        let dict =
                            trained_dict(&corpus, 4096, DictSeed::Explicit(12345)).expect("train");
                        (dict, before, fastrand::u64(..))
                    })
                })
                .collect();
            for handle in handles {
                let (dict, before, after) = handle.join().expect("training thread");
                assert_eq!(
                    dict, expected,
                    "a seeded trained dict must be byte-identical across threads"
                );
                assert_eq!(
                    before, after,
                    "training must restore the caller's thread-local fastrand state"
                );
            }
        });

        noise.store(false, std::sync::atomic::Ordering::Relaxed);
        for handle in noisemakers {
            handle.join().expect("noise thread");
        }
    }

    #[test]
    fn dictionary_id_reads_the_finalized_header() {
        let owned = sample_corpus();
        let corpus = as_slices(&owned);
        let dict = raw_content_dict(&corpus, 4096).expect("build");
        let id = dictionary_id(&dict).expect("a finalized dictionary carries an id");
        assert_ne!(
            id, 0,
            "a usable zstd dictionary has a non-zero Dictionary_ID"
        );
        assert!(dictionary_id(b"not a dictionary").is_err());
    }
}
