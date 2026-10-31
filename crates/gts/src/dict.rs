// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! - [`trained_dict`] runs deterministic pure Rust FastCOVER over the canonical
//!   corpus bytes, then finalizes the dictionary. The trainer has no random
//!   generator or seed parameter.

use purrdf_zstd::decoding::Dictionary;
use purrdf_zstd::dictionary::{
    FastCoverOptions, FinalizeOptions, create_fastcover_dict_from_source, finalize_raw_dict,
};

/// A dictionary could not be built from the supplied corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DictError {
    /// All supplied samples were empty.
    EmptyCorpus {
        /// Native authoring operation.
        operation: DictOperation,
        /// Original number of supplied samples.
        samples: usize,
        /// Requested finalized dictionary bytes.
        target_len: usize,
    },
    /// Explanation from the external dictionary implementation.
    External {
        /// Native operation that called the external implementation.
        operation: DictOperation,
        /// Original input byte count.
        input_bytes: usize,
        /// Requested output bytes, absent for dictionary decoding.
        target_len: Option<usize>,
        /// Original opaque external explanation.
        message: String,
    },
}

/// The dictionary operation that refused its input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictOperation {
    /// Canonical trailing-window construction.
    Raw,
    /// Deterministic FastCOVER construction over the canonical corpus.
    Train,
    /// Parse an existing finalized dictionary.
    Decode,
}

impl DictOperation {
    /// Stable operation code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw-content",
            Self::Train => "train",
            Self::Decode => "decode",
        }
    }
}

impl DictError {
    /// Original human-readable refusal, without its display prefix.
    #[must_use]
    pub fn detail(&self) -> &str {
        match self {
            Self::EmptyCorpus { .. } => "empty corpus",
            Self::External { message, .. } => message,
        }
    }
}

impl core::fmt::Display for DictError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "dictionary construction failed: {}", self.detail())
    }
}

impl std::error::Error for DictError {}

impl purrdf_hash::error::StructuredError for DictError {
    purrdf_hash::error_visit! {
        EmptyCorpus { operation, samples, target_len } => "gts-dictionary-empty-corpus" {
            "operation" => Code(operation.as_str()), "samples" => Unsigned(*samples as u128),
            "target_len" => ByteCount(*target_len as u128),
        },
        External { operation, input_bytes, target_len, message } => "gts-dictionary-external" {
            "operation" => Code(operation.as_str()), "input_bytes" => ByteCount(*input_bytes as u128),
            "target_len" => Value(target_len.map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::ByteCount(value as u128))),
            "message" => Message(message),
        },
    }
}

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
        return Err(DictError::EmptyCorpus {
            operation: DictOperation::Raw,
            samples: corpus.len(),
            target_len,
        });
    }
    finalize_raw_dict(&concat, &concat, target_len, FinalizeOptions::default()).map_err(|err| {
        DictError::External {
            operation: DictOperation::Raw,
            input_bytes: concat.len(),
            target_len: Some(target_len),
            message: err.to_string(),
        }
    })
}

/// Build a deterministic FastCOVER-trained finalized dictionary from `corpus`.
///
/// Training reads only the canonical corpus bytes and fixed FastCOVER options.
/// The result is independent of ambient random state.
///
/// # Errors
/// Returns [`DictError`] when the corpus is empty or too small for FastCOVER to
/// train a dictionary of the requested size.
pub fn trained_dict(corpus: &[&[u8]], target_len: usize) -> Result<Vec<u8>, DictError> {
    let concat = canonical_concat(corpus);
    if concat.is_empty() {
        return Err(DictError::EmptyCorpus {
            operation: DictOperation::Train,
            samples: corpus.len(),
            target_len,
        });
    }
    let mut out = Vec::new();
    let result = create_fastcover_dict_from_source(
        concat.as_slice(),
        &mut out,
        target_len,
        &FastCoverOptions::default(),
        FinalizeOptions::default(),
    );
    result.map_err(|err| DictError::External {
        operation: DictOperation::Train,
        input_bytes: concat.len(),
        target_len: Some(target_len),
        message: err.to_string(),
    })?;
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
        .map_err(|err| DictError::External {
            operation: DictOperation::Decode,
            input_bytes: dict.len(),
            target_len: None,
            message: err.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_zstd::decoding::Dictionary;

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
        assert_ne!(a, [] as [u8; 0]);
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
        assert_eq!(a, b, "FastCOVER must be byte-reproducible");
        assert_ne!(a, [] as [u8; 0]);
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
            trained_dict(&[], 4096).expect_err("empty corpus is rejected"),
            DictError::EmptyCorpus {
                operation: DictOperation::Train,
                samples: 0,
                target_len: 4096
            }
        );
        assert_eq!(
            raw_content_dict(&[], 4096).expect_err("empty corpus must be rejected"),
            DictError::EmptyCorpus {
                operation: DictOperation::Raw,
                samples: 0,
                target_len: 4096
            }
        );
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
