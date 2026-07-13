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
//! Two producers share that contract:
//!
//! - [`raw_content_dict`] keeps a canonical trailing window of the corpus — no
//!   training, no randomness; trivially wasm-clean and deterministic.
//! - [`trained_dict`] runs pure-Rust FastCOVER
//!   ([`structured_zstd::dictionary`]). FastCOVER's reservoir sampler draws from
//!   `fastrand`'s ambient thread-local RNG and exposes no seed parameter, so we
//!   seed that global deterministically from the corpus immediately before
//!   training, on the single training thread. The seed is derived from the corpus
//!   bytes, never from ambient state, so repeated builds on the authoring
//!   platform are byte-identical. The trained bytes are pinned in-band, so every
//!   reader — native or wasm — decodes the same dictionary.

use structured_zstd::dictionary::{FastCoverOptions, train_fastcover_raw_from_slice};

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

/// Build a deterministic **raw-content** dictionary from `corpus`.
///
/// A zstd raw dictionary is history the compressor sees *before* the payload; the
/// bytes nearest the end are the cheapest matches, so we keep the trailing
/// `target_len` bytes of the canonical concatenation (or the whole thing when it
/// is already smaller). No randomness is involved, so this is deterministic and
/// wasm-clean by construction.
#[must_use]
pub fn raw_content_dict(corpus: &[&[u8]], target_len: usize) -> Vec<u8> {
    let concat = canonical_concat(corpus);
    if concat.len() <= target_len {
        return concat;
    }
    concat[concat.len() - target_len..].to_vec()
}

/// Build a deterministic **FastCOVER-trained** dictionary from `corpus`.
///
/// The ambient `fastrand` global is seeded from the corpus immediately before
/// training so the reservoir sampler is reproducible; training is single-threaded
/// so the seeded thread-local is the one the sampler reads.
///
/// # Errors
/// Returns [`DictError`] when the corpus is empty or too small for FastCOVER to
/// train a dictionary of the requested size.
pub fn trained_dict(corpus: &[&[u8]], target_len: usize) -> Result<Vec<u8>, DictError> {
    let concat = canonical_concat(corpus);
    if concat.is_empty() {
        return Err(DictError("empty corpus".to_owned()));
    }
    fastrand::seed(derive_seed(&concat));
    match train_fastcover_raw_from_slice(&concat, target_len, &FastCoverOptions::default()) {
        Ok((dict, _tuned)) => Ok(dict),
        Err(err) => Err(DictError(err.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn raw_content_dict_is_deterministic_and_bounded() {
        let owned = sample_corpus();
        let corpus = as_slices(&owned);
        let a = raw_content_dict(&corpus, 4096);
        let b = raw_content_dict(&corpus, 4096);
        assert_eq!(a, b, "raw-content dict must be byte-reproducible");
        assert!(!a.is_empty());
        assert!(a.len() <= 4096, "must respect the target length bound");
    }

    #[test]
    fn raw_content_dict_is_order_independent() {
        let owned = sample_corpus();
        let forward = as_slices(&owned);
        let mut reversed = forward.clone();
        reversed.reverse();
        assert_eq!(
            raw_content_dict(&forward, 4096),
            raw_content_dict(&reversed, 4096),
            "canonical ordering must ignore caller iteration order"
        );
    }

    #[test]
    fn trained_dict_is_deterministic() {
        let owned = sample_corpus();
        let corpus = as_slices(&owned);
        let a = trained_dict(&corpus, 4096).expect("training should succeed");
        let b = trained_dict(&corpus, 4096).expect("training should succeed");
        assert_eq!(a, b, "seeded FastCOVER must be byte-reproducible");
        assert!(!a.is_empty());
    }

    #[test]
    fn trained_dict_is_order_independent() {
        let owned = sample_corpus();
        let forward = as_slices(&owned);
        let mut reversed = forward.clone();
        reversed.reverse();
        assert_eq!(
            trained_dict(&forward, 4096).expect("train"),
            trained_dict(&reversed, 4096).expect("train"),
            "the trained dict must be a pure function of the sample multiset"
        );
    }

    #[test]
    fn trained_dict_rejects_empty_corpus() {
        let err = trained_dict(&[], 4096).expect_err("empty corpus must be rejected");
        assert_eq!(err, DictError("empty corpus".to_owned()));
    }
}
