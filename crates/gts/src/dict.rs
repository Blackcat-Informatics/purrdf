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
//!   ([`structured_zstd::dictionary`]). FastCOVER's reservoir sampler draws from
//!   `fastrand`'s ambient thread-local RNG and exposes no seed parameter, so we
//!   seed that global deterministically from the corpus immediately before
//!   training, on the single training thread. The seed is derived from the corpus
//!   bytes, never from ambient state, so repeated builds on the authoring
//!   platform are byte-identical; the finalized bytes are pinned in-band, so
//!   every reader — native or wasm — decodes the same dictionary.

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

/// Build a deterministic **FastCOVER-trained** finalized dictionary from `corpus`.
///
/// The ambient `fastrand` global is seeded from the corpus immediately before
/// training so the reservoir sampler is reproducible; training is single-threaded
/// so the seeded thread-local is the one the sampler reads. The trainer finalizes
/// the raw content into a full dictionary binary.
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
    let mut out = Vec::new();
    create_fastcover_dict_from_source(
        concat.as_slice(),
        &mut out,
        target_len,
        &FastCoverOptions::default(),
        FinalizeOptions::default(),
    )
    .map_err(|err| DictError(err.to_string()))?;
    Ok(out)
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
        let a = trained_dict(&corpus, 4096).expect("training should succeed");
        let b = trained_dict(&corpus, 4096).expect("training should succeed");
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
            trained_dict(&forward, 4096).expect("train"),
            trained_dict(&reversed, 4096).expect("train"),
            "the trained dict must be a pure function of the sample multiset"
        );
    }

    #[test]
    fn producers_reject_empty_corpus() {
        assert_eq!(
            trained_dict(&[], 4096).expect_err("empty corpus must be rejected"),
            DictError("empty corpus".to_owned())
        );
        assert_eq!(
            raw_content_dict(&[], 4096).expect_err("empty corpus must be rejected"),
            DictError("empty corpus".to_owned())
        );
    }
}
