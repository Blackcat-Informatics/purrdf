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

use structured_zstd::decoding::{errors::FrameDecoderError, FrameDecoder};
use structured_zstd::encoding::{compress_to_vec, CompressionLevel};

/// A catalog entry (§5, §8.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Codec {
    pub name: String,
    /// `"encode"` | `"compress"` | `"encrypt"`.
    pub cls: String,
}

/// Why a transform chain could not be reversed.
#[derive(Debug)]
pub enum CodecError {
    /// A missing capability: `reason` is `"unknown-codec"` or `"missing-key"`
    /// — the frame degrades to an opaque node with that reason (§8.3).
    Unavailable {
        reason: &'static str,
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
pub struct EncodeOptions {
    /// Optional per-frame zstd compression level used by `zstd` and by each
    /// independent `zstd-rsyncable` block. `None` preserves the previous Rust
    /// writer default, roughly zstd level 1.
    pub zstd_level: Option<i32>,
}

fn zstd_level(level: Option<i32>) -> CompressionLevel {
    level
        .map(CompressionLevel::Level)
        .unwrap_or(DEFAULT_ZSTD_LEVEL)
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
                        continue;
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

fn encode_zstd_rsyncable(data: &[u8], level: Option<i32>) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for block in data.chunks(RSYNCABLE_BLOCK_SIZE) {
        out.extend(encode_zstd(block, level));
    }
    out
}

fn encode_one(name: &str, data: &[u8], options: EncodeOptions) -> Result<Vec<u8>, CodecError> {
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
        "zstd" => Ok(encode_zstd(data, options.zstd_level)),
        "zstd-rsyncable" => Ok(encode_zstd_rsyncable(data, options.zstd_level)),
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
    options: EncodeOptions,
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
                    })
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

    #[test]
    fn encoded_core_codecs_round_trip() {
        let payload = b"stable payload for writer transform parity".repeat(8);
        for name in ["identity", "gzip", "zstd", "zstd-rsyncable"] {
            let encoded = encode_chain(&[name.to_string()], &payload).expect("encodes");
            let decoded = decode_chain(
                &[Codec {
                    name: name.to_string(),
                    cls: if name == "identity" {
                        "encode".into()
                    } else {
                        "compress".into()
                    },
                }],
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

        let decoded = decode_one(
            &Codec {
                name: "zstd-rsyncable".into(),
                cls: "compress".into(),
            },
            &encoded,
        )
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
                },
            )
            .expect("fast zstd encodes");
            let high = encode_chain_with_options(
                &[codec.to_string()],
                &payload,
                EncodeOptions {
                    zstd_level: Some(19),
                },
            )
            .expect("high zstd encodes");

            assert!(
                high.len() <= fast.len(),
                "{codec}: level 19 should be no larger than level 1"
            );
            let decoded = decode_chain(
                &[Codec {
                    name: codec.to_string(),
                    cls: "compress".into(),
                }],
                &high,
            )
            .expect("levelled zstd decodes");
            assert_eq!(decoded, payload);
        }
    }

    #[test]
    fn zstd_decode_accepts_payloads_over_former_safety_bound() {
        let payload = vec![b'x'; 16 * 1024 * 1024 + 1];
        let encoded = encode_chain(&["zstd".to_string()], &payload).expect("zstd encodes");
        let decoded = decode_chain(
            &[Codec {
                name: "zstd".into(),
                cls: "compress".into(),
            }],
            &encoded,
        )
        .expect("zstd decoder grows past the former fixed output cap");

        assert_eq!(decoded, payload);
    }
}
