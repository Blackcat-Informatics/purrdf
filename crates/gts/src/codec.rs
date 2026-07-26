// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The GTS transform catalog (§8) — mirror of `src/purrdf_tools/gts/codec.py`.
//!
//! Each catalog entry is a codec with a canonical `name` and a `cls` of
//! `encode`, `compress` or `encrypt`. The baseline implements the core
//! `identity`/`gzip`/`zstd` codecs; an unknown codec or an `encrypt` codec
//! (no keys in the baseline) degrades to an opaque node (§7.6, §8.3).

use std::borrow::Cow;
use std::fmt;
use std::io::{Read, Write};

use structured_zstd::decoding::{FrameDecoder, errors::FrameDecoderError};
use structured_zstd::encoding::{
    CompressionLevel, FrameCompressor, MatchGeneratorDriver, compress_to_vec,
};

/// The one-shot, dictionary-primed encoder context type. `FrameCompressor`'s
/// source/drain type parameters are unused on the `compress_independent_frame`
/// path, so they are pinned to a concrete never-read reader/writer.
type DictCompressor = FrameCompressor<std::io::Empty, std::io::Sink, MatchGeneratorDriver>;

/// A catalog entry (§5, §8.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Codec {
    /// Canonical codec name from the registry (e.g. `"gzip"`, `"zstd"`).
    pub name: String,
    /// `"encode"` | `"compress"` | `"encrypt"`.
    pub cls: String,
    /// Resolved raw dictionary bytes for a `zstd`/`zstd-rsyncable`/`lzma2`
    /// `dct` codec (header `"dct"` map value the catalog entry's `"dct"` name
    /// resolved to); `None` for non-dict codecs (§5, §8.5).
    pub dct: Option<Vec<u8>>,
    /// The declared compression `level` parameter (§8.5 `zstd`/`zstd-rsyncable`
    /// `level?`). Decoding never needs it — zstd is self-describing — but
    /// declaring it on the wire makes the authoring level an OBSERVABLE fact a
    /// downstream policy can gate on, instead of an unverifiable claim.
    pub level: Option<i32>,
}

impl Codec {
    /// Build a non-dict, no-declared-level catalog entry.
    pub fn new(name: impl Into<String>, cls: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cls: cls.into(),
            dct: None,
            level: None,
        }
    }
}

/// Why a transform chain could not be reversed.
#[derive(Debug)]
pub enum CodecError {
    /// A missing capability: `reason` is `"unknown-codec"` or `"missing-key"`
    /// — the frame degrades to an opaque node with that reason (§8.3).
    Unavailable {
        /// Opaque-node reason token: `"unknown-codec"` or `"missing-key"`.
        reason: &'static str,
        /// Human-readable detail naming the codec that could not be applied.
        detail: String,
    },
    /// The codec is known but the data is corrupt — the frame is damaged.
    Failed(String),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable { reason, detail } => write!(f, "{reason}: {detail}"),
            Self::Failed(detail) => f.write_str(detail),
        }
    }
}

impl std::error::Error for CodecError {}

const DEFAULT_ZSTD_LEVEL: CompressionLevel = CompressionLevel::Fastest;

/// Encoder options for transform chains.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EncodeOptions<'a> {
    /// Optional per-frame zstd compression level used by `zstd` and by each
    /// independent `zstd-rsyncable` block. `None` preserves the previous Rust
    /// writer default, roughly zstd level 1.
    pub zstd_level: Option<i32>,
    /// Optional raw dictionary bytes priming the zstd-family transform (§5
    /// header `"dct"`, §8.5 `zstd`/`zstd-rsyncable` `dct` parameter).
    ///
    /// `zstd` primes its single frame with the dictionary; `zstd-rsyncable`
    /// primes EACH independent block with the SAME dictionary, so the blocks
    /// stay mutually independent and the block-boundary/delta property is
    /// preserved exactly (§8.4, §8.5). A dictionary paired with a chain that
    /// carries no zstd-family transform is a hard error — there is nothing to
    /// prime, and silently ignoring it would be a capability lie.
    pub dict: Option<&'a [u8]>,
}

fn zstd_level(level: Option<i32>) -> CompressionLevel {
    level.map_or(DEFAULT_ZSTD_LEVEL, CompressionLevel::Level)
}

fn decode_one(codec: &Codec, data: &[u8]) -> Result<Vec<u8>, CodecError> {
    if codec.cls == "encrypt" {
        return Err(CodecError::Unavailable {
            reason: "missing-key",
            detail: format!("no key for encrypt codec '{}'", codec.name),
        });
    }
    match codec.name.as_str() {
        "identity" => Ok(data.to_vec()),
        "gzip" => {
            let mut out = Vec::new();
            flate2::read::GzDecoder::new(data)
                .read_to_end(&mut out)
                .map_err(|e| CodecError::Failed(format!("gzip decode failed: {e}")))?;
            Ok(out)
        }
        "zstd" | "zstd-rsyncable" => {
            let mut decoder = FrameDecoder::new();
            if let Some(dict) = codec.dct.as_deref() {
                decoder
                    .add_dict_from_bytes(dict)
                    .map_err(|e| CodecError::Failed(format!("zstd dictionary load failed: {e}")))?;
            }
            // Start with a generous expansion factor and grow until the frame fits.
            let mut capacity = data.len().saturating_mul(4).max(4096);
            loop {
                let mut out = Vec::new();
                out.try_reserve(capacity).map_err(|e| {
                    CodecError::Failed(format!("zstd decode failed: output allocation failed: {e}"))
                })?;
                match decoder.decode_all_to_vec(data, &mut out) {
                    Ok(()) => return Ok(out),
                    Err(FrameDecoderError::TargetTooSmall) => {
                        capacity = capacity.checked_mul(2).ok_or_else(|| {
                            CodecError::Failed(
                                "zstd decode failed: decoded output is too large for this platform"
                                    .into(),
                            )
                        })?;
                    }
                    Err(e) => return Err(CodecError::Failed(format!("zstd decode failed: {e}"))),
                }
            }
        }
        other => Err(CodecError::Unavailable {
            reason: "unknown-codec",
            detail: format!("unknown codec '{other}'"),
        }),
    }
}

const RSYNCABLE_BLOCK_SIZE: usize = 65_536;

fn encode_zstd(data: &[u8], level: Option<i32>) -> Vec<u8> {
    compress_to_vec(data, zstd_level(level))
}

/// A dictionary-primed encoder context, reused across every block of one
/// rsyncable payload (the dictionary parse and table setup are paid once).
fn dict_compressor(level: Option<i32>, dict: &[u8]) -> Result<DictCompressor, CodecError> {
    let mut cctx: DictCompressor = FrameCompressor::new(zstd_level(level));
    cctx.set_dictionary_from_bytes(dict)
        .map_err(|e| CodecError::Failed(format!("zstd dictionary load failed: {e}")))?;
    Ok(cctx)
}

fn encode_zstd_with_dict(
    data: &[u8],
    level: Option<i32>,
    dict: &[u8],
) -> Result<Vec<u8>, CodecError> {
    Ok(dict_compressor(level, dict)?.compress_independent_frame(data))
}

/// Encode `data` as independent `RSYNCABLE_BLOCK_SIZE` zstd frames (§8.4).
///
/// When `dict` is present EVERY block is primed with the SAME dictionary —
/// priming is history the encoder sees *before* the block, never state carried
/// *between* blocks, so the blocks remain mutually independent and the cut
/// points stay at exactly the same uncompressed offsets as the undicted
/// encoding. That is the whole point: the dictionary buys density without
/// costing the delta-transfer property.
fn encode_zstd_rsyncable(
    data: &[u8],
    level: Option<i32>,
    dict: Option<&[u8]>,
) -> Result<Vec<u8>, CodecError> {
    let mut out = Vec::with_capacity(data.len());
    match dict {
        None => {
            for block in data.chunks(RSYNCABLE_BLOCK_SIZE) {
                out.extend(encode_zstd(block, level));
            }
        }
        Some(dict) => {
            // One encoder context primes the dictionary once and emits an
            // INDEPENDENT frame per block; nothing carries across blocks.
            let mut cctx = dict_compressor(level, dict)?;
            let mut block_out = Vec::new();
            for block in data.chunks(RSYNCABLE_BLOCK_SIZE) {
                // `compress_independent_frame_into` replaces the buffer today,
                // but make the frame-isolation requirement explicit at this
                // call site instead of depending on that reuse contract.
                block_out.clear();
                cctx.compress_independent_frame_into(block, &mut block_out);
                out.extend_from_slice(&block_out);
            }
        }
    }
    Ok(out)
}

fn encode_one(name: &str, data: &[u8], options: EncodeOptions<'_>) -> Result<Vec<u8>, CodecError> {
    match name {
        "identity" => Ok(data.to_vec()),
        "gzip" => {
            let mut encoder = flate2::GzBuilder::new()
                .mtime(0)
                .write(Vec::new(), flate2::Compression::default());
            encoder
                .write_all(data)
                .map_err(|e| CodecError::Failed(format!("gzip encode failed: {e}")))?;
            encoder
                .finish()
                .map_err(|e| CodecError::Failed(format!("gzip encode failed: {e}")))
        }
        "zstd" => match options.dict {
            Some(dict) => encode_zstd_with_dict(data, options.zstd_level, dict),
            None => Ok(encode_zstd(data, options.zstd_level)),
        },
        "zstd-rsyncable" => encode_zstd_rsyncable(data, options.zstd_level, options.dict),
        other => Err(CodecError::Unavailable {
            reason: "unknown-codec",
            detail: format!("writer cannot encode with codec '{other}'"),
        }),
    }
}

/// Encode `data` through codec names in array order with explicit options (§8.2).
pub fn encode_chain_with_options(
    chain: &[String],
    data: &[u8],
    options: EncodeOptions<'_>,
) -> Result<Vec<u8>, CodecError> {
    if options.zstd_level.is_some()
        && !chain
            .iter()
            .any(|name| matches!(name.as_str(), "zstd" | "zstd-rsyncable"))
    {
        return Err(CodecError::Failed(
            "zstd_level requires a zstd or zstd-rsyncable transform".into(),
        ));
    }
    if options.dict.is_some()
        && !chain
            .iter()
            .any(|name| matches!(name.as_str(), "zstd" | "zstd-rsyncable"))
    {
        return Err(CodecError::Failed(
            "dict requires a zstd or zstd-rsyncable transform".into(),
        ));
    }
    let mut current = Cow::Borrowed(data);
    for name in chain {
        current = Cow::Owned(encode_one(name, current.as_ref(), options)?);
    }
    Ok(current.into_owned())
}

/// Encode `data` through codec names in array order (§8.2).
pub fn encode_chain(chain: &[String], data: &[u8]) -> Result<Vec<u8>, CodecError> {
    encode_chain_with_options(chain, data, EncodeOptions::default())
}

/// One `zstd-rsyncable` block (or the single frame of a plain `zstd` payload)
/// as observed on the wire, without decompressing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZstdBlockInfo {
    /// Declared uncompressed length of the block (§8.4 cut point).
    pub content_len: u64,
    /// Compressed length of this frame in the payload byte stream.
    pub compressed_len: usize,
    /// `Dictionary_ID` named by the frame header, `None` when the block was
    /// encoded without a dictionary.
    pub dictionary_id: Option<u32>,
}

/// Walk an encoded `zstd`/`zstd-rsyncable` payload frame by frame, reporting
/// each block's declared cut point and priming dictionary WITHOUT decompressing.
///
/// This is the observation surface behind the rsyncable guarantee: a policy can
/// check that a payload's cut points sit on the declared `block_size` grid and
/// that every block names the pack's pinned dictionary (§8.4, §8.5).
///
/// # Errors
/// Returns [`CodecError::Failed`] when the byte stream is not an exact sequence
/// of zstd frames, or a frame omits its `Frame_Content_Size`.
pub fn zstd_block_layout(payload: &[u8]) -> Result<Vec<ZstdBlockInfo>, CodecError> {
    use structured_zstd::decoding::{
        FrameContentSize, find_frame_compressed_size, read_frame_header_info,
    };

    let mut out = Vec::new();
    let mut rest = payload;
    while !rest.is_empty() {
        let header = read_frame_header_info(rest, false)
            .map_err(|e| CodecError::Failed(format!("zstd frame header is unreadable: {e}")))?;
        let compressed_len = find_frame_compressed_size(rest)
            .map_err(|e| CodecError::Failed(format!("zstd frame is truncated: {e:?}")))?;
        if compressed_len == 0 || compressed_len > rest.len() {
            return Err(CodecError::Failed(format!(
                "zstd frame reports invalid compressed length {compressed_len} for {} remaining \
                 bytes",
                rest.len()
            )));
        }
        let FrameContentSize::Known(content_len) = header.content_size else {
            return Err(CodecError::Failed(
                "zstd frame omits its Frame_Content_Size, so its cut point is unobservable".into(),
            ));
        };
        out.push(ZstdBlockInfo {
            content_len,
            compressed_len,
            dictionary_id: header.dictionary_id,
        });
        rest = &rest[compressed_len..];
    }
    Ok(out)
}

/// Reverse a resolved codec chain, last to first (§6.1, §8.2).
///
/// The baseline carries no keys, so every `encrypt`-class codec degrades to
/// `missing-key` (matching the Python reader with `keys=None`).
pub fn decode_chain(chain: &[Codec], data: &[u8]) -> Result<Vec<u8>, CodecError> {
    decode_chain_with_decrypt(chain, data, None)
}

/// A caller-supplied encrypt-class transform resolver.
pub type Decryptor<'a> = dyn Fn(&Codec, &[u8]) -> Result<Vec<u8>, CodecError> + 'a;

/// Reverse a resolved codec chain, handing encrypt-class transforms to `decrypt`.
pub fn decode_chain_with_decrypt(
    chain: &[Codec],
    data: &[u8],
    decrypt: Option<&Decryptor<'_>>,
) -> Result<Vec<u8>, CodecError> {
    let mut current = Cow::Borrowed(data);
    for codec in chain.iter().rev() {
        if codec.cls == "encrypt" {
            current = Cow::Owned(match decrypt {
                Some(decrypt) => decrypt(codec, current.as_ref())?,
                None => {
                    return Err(CodecError::Unavailable {
                        reason: "missing-key",
                        detail: format!("no key for encrypt codec '{}'", codec.name),
                    });
                }
            });
        } else {
            current = Cow::Owned(decode_one(codec, current.as_ref())?);
        }
    }
    Ok(current.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dict::DictSeed;

    #[test]
    fn encoded_core_codecs_round_trip() {
        let payload = b"stable payload for writer transform parity".repeat(8);
        for name in ["identity", "gzip", "zstd", "zstd-rsyncable"] {
            let encoded = encode_chain(&[name.to_string()], &payload).expect("encodes");
            let decoded = decode_chain(
                &[Codec::new(
                    name,
                    if name == "identity" {
                        "encode"
                    } else {
                        "compress"
                    },
                )],
                &encoded,
            )
            .expect("decodes");
            assert_eq!(decoded, payload);
        }
    }

    #[test]
    fn gzip_encoding_is_deterministic() {
        let payload = b"stable gzip payload".repeat(16);
        assert_eq!(
            encode_chain(&["gzip".to_string()], &payload).unwrap(),
            encode_chain(&["gzip".to_string()], &payload).unwrap()
        );
    }

    #[test]
    fn zstd_rsyncable_decodes_concatenated_frames() {
        // Build a multi-frame zstd stream that mirrors zstd-rsyncable output.
        let block1 = b"first block of rsyncable data ";
        let block2 = b"second block of rsyncable data";
        let mut encoded = compress_to_vec(&block1[..], CompressionLevel::Uncompressed);
        encoded.extend(compress_to_vec(&block2[..], CompressionLevel::Uncompressed));

        let decoded = decode_one(&Codec::new("zstd-rsyncable", "compress"), &encoded)
            .expect("multi-frame zstd must decode");

        let mut expected = block1.to_vec();
        expected.extend_from_slice(block2);
        assert_eq!(decoded, expected);
    }

    #[test]
    fn zstd_level_is_per_encode_chain() {
        let payload = b"<https://ex/s> <https://ex/p> \"repeat repeat repeat\" .\n".repeat(2048);

        for codec in ["zstd", "zstd-rsyncable"] {
            let fast = encode_chain_with_options(
                &[codec.to_string()],
                &payload,
                EncodeOptions {
                    zstd_level: Some(1),
                    dict: None,
                },
            )
            .expect("fast zstd encodes");
            let high = encode_chain_with_options(
                &[codec.to_string()],
                &payload,
                EncodeOptions {
                    zstd_level: Some(19),
                    dict: None,
                },
            )
            .expect("high zstd encodes");

            assert!(
                high.len() <= fast.len(),
                "{codec}: level 19 should be no larger than level 1"
            );
            let decoded = decode_chain(&[Codec::new(codec, "compress")], &high)
                .expect("levelled zstd decodes");
            assert_eq!(decoded, payload);
        }
    }

    #[test]
    fn zstd_decode_accepts_payloads_over_former_safety_bound() {
        let payload = vec![b'x'; 16 * 1024 * 1024 + 1];
        let encoded = encode_chain(&["zstd".to_string()], &payload).expect("zstd encodes");
        let decoded = decode_chain(&[Codec::new("zstd", "compress")], &encoded)
            .expect("zstd decoder grows past the former fixed output cap");

        assert_eq!(decoded, payload);
    }

    /// A corpus with enough repeated structure to build a dictionary from,
    /// distinct from the payload it primes (but sharing structure with it —
    /// exactly the case a pack dictionary targets).
    fn dict_corpus() -> Vec<Vec<u8>> {
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

    fn dict_payload() -> Vec<u8> {
        (0..64u32)
            .flat_map(|i| {
                format!(
                    "<https://example.org/s{}> <https://example.org/p> \"claim {} about cats\" .\n",
                    i % 37,
                    i + 10_000
                )
                .into_bytes()
            })
            .collect()
    }

    #[test]
    fn zstd_dict_codec_round_trips_for_both_producers() {
        let owned = dict_corpus();
        let corpus: Vec<&[u8]> = owned.iter().map(Vec::as_slice).collect();
        let payload = dict_payload();

        for dict in [
            crate::dict::raw_content_dict(&corpus, 4096).expect("raw content dict builds"),
            crate::dict::trained_dict(&corpus, 4096, DictSeed::FromCorpus).expect("trained dict"),
        ] {
            let encoded = encode_chain_with_options(
                &["zstd".to_string()],
                &payload,
                EncodeOptions {
                    zstd_level: None,
                    dict: Some(&dict),
                },
            )
            .expect("dict-primed zstd encodes");

            let mut codec = Codec::new("zstd", "compress");
            codec.dct = Some(dict.clone());
            let decoded = decode_chain(&[codec], &encoded).expect("dict-primed zstd decodes");
            assert_eq!(decoded, payload);
        }
    }

    #[test]
    fn zstd_dict_codec_rejects_a_mismatched_dictionary() {
        let owned = dict_corpus();
        let corpus: Vec<&[u8]> = owned.iter().map(Vec::as_slice).collect();
        let payload = dict_payload();

        let real_dict = crate::dict::raw_content_dict(&corpus, 4096).expect("dict builds");
        // A dictionary built from unrelated content carries a different dict-id,
        // so the decoder must refuse to substitute it in.
        let other_corpus_owned = [b"an entirely unrelated corpus with different bytes; \
             also not RDF at all, just filler text to reach a usable dictionary size."
            .repeat(64)];
        let other_corpus: Vec<&[u8]> = other_corpus_owned.iter().map(Vec::as_slice).collect();
        let wrong_dict = crate::dict::raw_content_dict(&other_corpus, 4096).expect("dict builds");
        assert_ne!(
            real_dict, wrong_dict,
            "the two dictionaries must actually differ"
        );

        let encoded = encode_chain_with_options(
            &["zstd".to_string()],
            &payload,
            EncodeOptions {
                zstd_level: None,
                dict: Some(&real_dict),
            },
        )
        .expect("dict-primed zstd encodes");

        let mut codec = Codec::new("zstd", "compress");
        codec.dct = Some(wrong_dict);
        let result = decode_chain(&[codec], &encoded);
        assert!(
            result.is_err(),
            "decoding with a mismatched dictionary must fail, not silently succeed"
        );
    }

    /// A payload spanning several rsyncable blocks, built from the same
    /// structure the dictionary corpus carries.
    fn multiblock_payload() -> Vec<u8> {
        (0..4000u32)
            .flat_map(|i| {
                format!(
                    "<https://example.org/s{}> <https://example.org/p> \"claim {} about cats\" .\n",
                    i % 37,
                    i + 10_000
                )
                .into_bytes()
            })
            .collect()
    }

    /// The uncompressed cut points of a `zstd-rsyncable` payload, read from the
    /// frame headers on the wire.
    fn rsyncable_block_sizes(encoded: &[u8]) -> Vec<usize> {
        zstd_block_layout(encoded)
            .expect("an rsyncable payload is an exact sequence of zstd frames")
            .into_iter()
            .map(|block| block.content_len as usize)
            .collect()
    }

    /// (b) A rsyncable+dict frame must cut at exactly the same UNCOMPRESSED
    /// offsets as the same input with no dictionary: priming is pre-block
    /// history, never cross-block state.
    #[test]
    fn zstd_rsyncable_dict_preserves_block_count_and_cut_points() {
        let owned = dict_corpus();
        let corpus: Vec<&[u8]> = owned.iter().map(Vec::as_slice).collect();
        let dict = crate::dict::raw_content_dict(&corpus, 4096).expect("dict builds");
        let payload = multiblock_payload();
        assert!(
            payload.len() > 3 * RSYNCABLE_BLOCK_SIZE,
            "the fixture must span several rsyncable blocks"
        );

        let undicted = encode_chain_with_options(
            &["zstd-rsyncable".to_string()],
            &payload,
            EncodeOptions {
                zstd_level: Some(12),
                dict: None,
            },
        )
        .expect("undicted rsyncable encodes");
        let dicted = encode_chain_with_options(
            &["zstd-rsyncable".to_string()],
            &payload,
            EncodeOptions {
                zstd_level: Some(12),
                dict: Some(&dict),
            },
        )
        .expect("dict-primed rsyncable encodes");

        let expected: Vec<usize> = payload
            .chunks(RSYNCABLE_BLOCK_SIZE)
            .map(<[u8]>::len)
            .collect();
        assert_eq!(
            rsyncable_block_sizes(&undicted),
            expected,
            "the undicted encoding cuts on the fixed block grid"
        );
        assert_eq!(
            rsyncable_block_sizes(&dicted),
            expected,
            "a dictionary must not move a single rsyncable cut point"
        );
    }

    /// (a) rsyncable + dict is lossless, and (h) every primed block carries the
    /// dictionary's own `Dictionary_ID` in its zstd frame header.
    #[test]
    fn zstd_rsyncable_dict_round_trips_and_every_block_names_the_dict_id() {
        let owned = dict_corpus();
        let corpus: Vec<&[u8]> = owned.iter().map(Vec::as_slice).collect();
        let payload = multiblock_payload();

        for dict in [
            crate::dict::raw_content_dict(&corpus, 4096).expect("raw content dict builds"),
            crate::dict::trained_dict(&corpus, 4096, DictSeed::FromCorpus).expect("trained dict"),
        ] {
            let encoded = encode_chain_with_options(
                &["zstd-rsyncable".to_string()],
                &payload,
                EncodeOptions {
                    zstd_level: Some(12),
                    dict: Some(&dict),
                },
            )
            .expect("dict-primed rsyncable encodes");

            let mut codec = Codec::new("zstd-rsyncable", "compress");
            codec.dct = Some(dict.clone());
            let decoded =
                decode_chain(&[codec], &encoded).expect("dict-primed rsyncable round trips");
            assert_eq!(decoded, payload, "rsyncable + dict must be lossless");

            let dict_id = crate::dict::dictionary_id(&dict).expect("finalized dict carries an id");
            assert_ne!(dict_id, 0, "a finalized zstd dictionary has a non-zero id");
            let layout = zstd_block_layout(&encoded).expect("every zstd frame header parses");
            assert!(!layout.is_empty(), "at least one rsyncable block emitted");
            assert!(
                layout
                    .iter()
                    .all(|block| block.dictionary_id == Some(dict_id)),
                "every rsyncable block must name the priming dictionary's id: {layout:?}"
            );
        }
    }

    #[test]
    fn a_dictionary_without_a_zstd_family_transform_is_a_hard_error() {
        let payload = b"payload".repeat(8);
        let dict = vec![0u8; 64];
        let err = encode_chain_with_options(
            &["gzip".to_string()],
            &payload,
            EncodeOptions {
                zstd_level: None,
                dict: Some(&dict),
            },
        )
        .expect_err("a dict with nothing to prime must be a hard error");
        assert!(matches!(err, CodecError::Failed(_)));
    }
}
