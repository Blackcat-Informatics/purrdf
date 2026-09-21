// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

#![cfg(not(target_arch = "wasm32"))]

//! Streaming blob limits remain effective when a content key is supplied.

use purrdf_gts::reader::{
    BlobPayload, ReadOptions, StreamingReadResult, StreamingSink, read_to_sink_from_reader,
    read_to_sink_with_options,
};
use purrdf_gts::writer::{Encrypt0Options, FrameOptions, Writer};

fn fresh_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).expect("test encryption randomness");
    bytes
}

fn wrong_key(mut key: [u8; 32]) -> [u8; 32] {
    key[0] ^= 1;
    key
}

#[derive(Default)]
struct Sink {
    decoded_limit: Option<usize>,
    encoded_limit: Option<usize>,
    payloads: Vec<Vec<u8>>,
}

impl StreamingSink for Sink {
    fn blob_decode_limit(&self) -> Option<usize> {
        self.decoded_limit
    }

    fn blob_encoded_limit(&self) -> Option<usize> {
        self.encoded_limit
    }

    fn blob_payload(&mut self, payload: BlobPayload<'_>) {
        assert!(
            payload.codecs.is_empty(),
            "eager decoding must complete first"
        );
        self.payloads
            .push(payload.bytes.expect("decoded payload").to_vec());
    }
}

fn blob(data: &[u8], transforms: &[&str], key: Option<[u8; 32]>) -> Vec<u8> {
    let mut writer = Writer::new("generic");
    writer
        .add_frame_with_options(
            "blob",
            FrameOptions {
                raw: Some(data.to_vec()),
                transform: transforms.iter().map(|name| (*name).into()).collect(),
                encrypt: key.map(|key| Encrypt0Options {
                    kid: "recipient".into(),
                    key,
                    iv: fresh_bytes(),
                }),
                ..FrameOptions::default()
            },
        )
        .expect("tiny authored frame");
    writer.into_bytes()
}

fn read(bytes: &[u8], sink: &mut Sink, key: Option<[u8; 32]>) -> StreamingReadResult {
    let resolve = |kid: &str| if kid == "recipient" { key } else { None };
    read_to_sink_with_options(
        bytes,
        ReadOptions::new(true, None).with_content_key(&resolve),
        sink,
    )
}

fn assert_limit(result: &StreamingReadResult, sink: &Sink, detail: &str) {
    assert!(
        sink.payloads.is_empty(),
        "oversized output cannot reach the sink"
    );
    assert!(
        result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == purrdf_gts::reader::BLOB_BUDGET_DIAGNOSTIC
                && diagnostic.detail.contains(detail)
        }),
        "expected {detail}: {:?}",
        result.diagnostics
    );
}

#[test]
fn encrypted_plaintext_accepts_exact_and_empty_limits() {
    for data in [b"".as_slice(), b"bounded".as_slice()] {
        let mut sink = Sink {
            decoded_limit: Some(data.len()),
            ..Sink::default()
        };
        let key = fresh_bytes();
        let result = read(&blob(data, &[], Some(key)), &mut sink, Some(key));
        assert!(
            !result.diagnostics.iter().any(|diagnostic| {
                matches!(diagnostic.code.as_str(), "DamagedFrame" | "MissingKey")
            }),
            "{:?}",
            result.diagnostics
        );
        assert_eq!(sink.payloads, vec![data.to_vec()]);
    }
}

#[test]
fn encrypted_plaintext_limit_is_checked_before_authentication_or_emission() {
    let key = fresh_bytes();
    let bytes = blob(b"bounded", &[], Some(key));
    for key in [key, wrong_key(key)] {
        let mut sink = Sink {
            decoded_limit: Some(6),
            ..Sink::default()
        };
        let result = read(&bytes, &mut sink, Some(key));
        // The wrong key would fail authentication if decryption ran first.
        assert_limit(&result, &sink, "decoded transform output exceeds 6 bytes");
    }
}

#[test]
fn supplied_key_does_not_unbound_compression_with_or_without_encryption() {
    let data = [b'x'; 4096];
    for codec in ["gzip", "zstd"] {
        for encrypted in [false, true] {
            let key = fresh_bytes();
            let bytes = blob(&data, &[codec], encrypted.then_some(key));
            let mut sink = Sink {
                decoded_limit: Some(4095),
                ..Sink::default()
            };
            let result = read(&bytes, &mut sink, Some(key));
            assert_limit(
                &result,
                &sink,
                "decoded transform output exceeds 4095 bytes",
            );
            let mut exact = Sink {
                decoded_limit: Some(4096),
                ..Sink::default()
            };
            read(&bytes, &mut exact, Some(key));
            assert_eq!(exact.payloads, vec![data.to_vec()]);
        }
    }
}

#[test]
fn encrypted_intermediate_limit_applies_even_when_final_plaintext_fits() {
    let key = fresh_bytes();
    let bytes = blob(b"x", &["gzip"], Some(key));
    let mut sink = Sink {
        decoded_limit: Some(1),
        ..Sink::default()
    };
    let result = read(&bytes, &mut sink, Some(key));
    assert_limit(&result, &sink, "decoded transform output exceeds 1 bytes");
    let mut unbounded = Sink::default();
    read(&bytes, &mut unbounded, Some(key));
    assert_eq!(unbounded.payloads, vec![b"x".to_vec()]);
}

#[test]
fn keyed_blob_still_enforces_original_encoded_limit() {
    let key = fresh_bytes();
    let bytes = blob(b"bounded", &[], Some(key));
    let mut sink = Sink {
        decoded_limit: Some(7),
        encoded_limit: Some(7),
        ..Sink::default()
    };
    let result = read(&bytes, &mut sink, Some(key));
    assert_limit(&result, &sink, "encoded blob exceeds 7 bytes");
}

#[test]
fn bounded_decryption_preserves_missing_and_wrong_key_failures() {
    let key = fresh_bytes();
    let bytes = blob(b"bounded", &[], Some(key));
    for key in [None, Some(wrong_key(key))] {
        let mut sink = Sink {
            decoded_limit: Some(7),
            ..Sink::default()
        };
        let result = read(&bytes, &mut sink, key);
        assert_eq!(sink.payloads, Vec::<Vec<u8>>::new());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == "MissingKey" }),
            "{:?}",
            result.diagnostics
        );
    }
}

struct ShortReads<'a> {
    remaining: &'a [u8],
    step: usize,
}

impl std::io::Read for ShortReads<'_> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        const SIZES: [usize; 7] = [1, 17, 2, 4093, 3, 257, 5];
        let length = output
            .len()
            .min(self.remaining.len())
            .min(SIZES[self.step % SIZES.len()]);
        self.step += 1;
        output[..length].copy_from_slice(&self.remaining[..length]);
        self.remaining = &self.remaining[length..];
        Ok(length)
    }
}

fn encoded_payload_len(container: &[u8]) -> usize {
    let (items, torn) = purrdf_gts::wire::iter_items(container);
    assert!(torn.is_none());
    items
        .iter()
        .find_map(|(_, value)| {
            value
                .as_map()
                .and_then(|map| purrdf_gts::wire::map_get(map, "d"))
                .and_then(ciborium::Value::as_bytes)
                .map(Vec::len)
        })
        .expect("authored blob must contain encoded bytes")
}

#[test]
fn short_reads_preserve_exact_encoded_and_decoded_limit_boundaries() {
    let key = fresh_bytes();
    for length in [0, 1, 15, 16, 17, 4095, 4096, 4097, 65536] {
        let plaintext: Vec<u8> = (0..=255).cycle().take(length).collect();
        let container = blob(&plaintext, &[], Some(key));
        let encoded = encoded_payload_len(&container);
        for (encoded_limit, decoded_limit, accepted) in [
            (encoded, length, true),
            (encoded + 1, length + 1, true),
            (encoded - 1, length, false),
            (encoded, length.saturating_sub(1), length == 0),
        ] {
            let mut sink = Sink {
                encoded_limit: Some(encoded_limit),
                decoded_limit: Some(decoded_limit),
                ..Sink::default()
            };
            let resolve = |kid: &str| (kid == "recipient").then_some(key);
            let result = read_to_sink_from_reader(
                ShortReads {
                    remaining: &container,
                    step: 0,
                },
                ReadOptions::new(true, None).with_content_key(&resolve),
                &mut sink,
            );
            if accepted {
                assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
                assert_eq!(sink.payloads, vec![plaintext.clone()]);
            } else {
                assert_limit(&result, &sink, "exceeds");
            }
        }
    }
}

#[test]
fn short_read_authentication_failure_never_emits_plaintext() {
    let key = fresh_bytes();
    let container = blob(&vec![0; 65536], &[], Some(key));
    let resolve = |_: &str| Some(wrong_key(key));
    let mut sink = Sink::default();
    let result = read_to_sink_from_reader(
        ShortReads {
            remaining: &container,
            step: 0,
        },
        ReadOptions::new(true, None).with_content_key(&resolve),
        &mut sink,
    );
    assert_eq!(sink.payloads, Vec::<Vec<u8>>::new());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "MissingKey")
    );
}
