// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Base16 (`purrdf_hash::hex`): the frozen encoding and digit tables replayed
//! through every rendering and reading entry point and every encoding path
//! this host runs, the RFC 4648 §10 vectors, the formatter options, the
//! refusals of each reader beside the valid neighbour each still accepts, and
//! [`Digest32`].
//!
//! The frozen tables are `tests/vectors/hex_vectors.txt` (every length 0..=64
//! from every first byte, both renderings) and `tests/vectors/hex_digit_vectors.txt`
//! (every byte as a digit of each lexical space). They were recorded from the
//! workspace's hex encoders and decoders before those became this module; a
//! disagreement is a defect in `purrdf_hash::hex`, never a reason to edit a
//! vector.
//!
//! The target is `harness = false` on `purrdf_testkit`'s runner, so the same
//! cases run natively under `cargo test` and on `wasm32-unknown-unknown` in
//! Node (`make wasm-test`), baseline and `+simd128`.

use purrdf_hash::Backend as _;
use purrdf_hash::backend::HexBackend;
use purrdf_hash::dispatch::{assert_required_available, host_advertises};
use purrdf_hash::hex::{
    Digest32, HexError, Lower, SHORT_MAX, Upper, decode, decode_32, decode_32_canonical,
    decode_canonical, encode, encode_into, encode_to_slice, encode_upper, encode_upper_into,
    encode_upper_to_slice, nibble, nibble_canonical,
};
use purrdf_testkit::rng::xoshiro256_bytes;
use purrdf_testkit::vectors::{VectorFile, decode_str};

/// The frozen encoding table: every length 0..=64 from every first byte.
const HEX_VECTORS: &str = include_str!("vectors/hex_vectors.txt");

/// The frozen digit table: every byte as a digit of each lexical space.
const DIGIT_VECTORS: &str = include_str!("vectors/hex_digit_vectors.txt");

/// One frozen encoding record.
struct Encoding {
    input: Vec<u8>,
    lower: String,
    upper: String,
}

/// Each frozen encoding record, its input rebuilt from its length and first
/// byte as the file's header states.
fn frozen_encodings() -> Vec<Encoding> {
    let file =
        VectorFile::parse(HEX_VECTORS).unwrap_or_else(|error| panic!("hex_vectors.txt: {error}"));
    file.records()
        .iter()
        .map(|record| {
            let len: usize = record.fields[0].parse().expect("a decimal length");
            let first: u8 = record.fields[1].parse().expect("a decimal first byte");
            Encoding {
                input: (0..len).map(|i| first.wrapping_add(i as u8)).collect(),
                lower: decode_str(record.fields[2]).expect("encoded lowercase digits"),
                upper: decode_str(record.fields[3]).expect("encoded uppercase digits"),
            }
        })
        .collect()
}

/// One frozen digit record: the byte's value in the either-case, canonical
/// lowercase and canonical uppercase spaces.
struct DigitRecord {
    byte: u8,
    any: Option<u8>,
    lower: Option<u8>,
    upper: Option<u8>,
}

fn frozen_digits() -> Vec<DigitRecord> {
    let file = VectorFile::parse(DIGIT_VECTORS)
        .unwrap_or_else(|error| panic!("hex_digit_vectors.txt: {error}"));
    let value = |field: &str| (field != "-").then(|| field.parse().expect("a decimal value"));
    let records: Vec<DigitRecord> = file
        .records()
        .iter()
        .map(|record| DigitRecord {
            byte: record.fields[0].parse().expect("a decimal byte"),
            any: value(record.fields[1]),
            lower: value(record.fields[2]),
            upper: value(record.fields[3]),
        })
        .collect();
    assert_eq!(records.len(), 256);
    records
}

/// Every frozen record renders as its lowercase and uppercase fields through
/// every rendering entry point and on every encoding path this host runs.
fn frozen_encodings_render_through_every_entry_point() {
    let records = frozen_encodings();
    assert_eq!(records.len(), 65 * 256);
    let mut buffer = [0u8; 2 * 64 + 3];
    for Encoding {
        input,
        lower,
        upper,
    } in &records
    {
        assert_eq!(&Lower(input).to_string(), lower, "{input:?}");
        assert_eq!(&Upper(input).to_string(), upper, "{input:?}");
        assert_eq!(&encode(input), lower, "{input:?}");
        assert_eq!(&encode_upper(input), upper, "{input:?}");
        let mut appended = String::from("x:");
        encode_into(input, &mut appended);
        assert_eq!(appended, format!("x:{lower}"));
        let mut appended = String::from("x:");
        encode_upper_into(input, &mut appended);
        assert_eq!(appended, format!("x:{upper}"));
        assert_eq!(encode_to_slice(input, &mut buffer), Ok(lower.as_str()));
        assert_eq!(
            encode_upper_to_slice(input, &mut buffer),
            Ok(upper.as_str())
        );
        for backend in HexBackend::all_available() {
            let mut out = vec![0u8; 2 * input.len()];
            backend.encode(input, &mut out).expect("an available path");
            assert_eq!(out, lower.as_bytes(), "{input:?} on {}", backend.name());
            backend
                .encode_upper(input, &mut out)
                .expect("an available path");
            assert_eq!(out, upper.as_bytes(), "{input:?} on {}", backend.name());
        }
    }
}

/// Every frozen record reads back through every reader whose lexical space
/// holds its rendering, and the canonical readers refuse the uppercase
/// rendering at its first letter.
fn frozen_encodings_read_back_through_every_reader() {
    for Encoding {
        input,
        lower,
        upper,
    } in &frozen_encodings()
    {
        assert_eq!(decode(lower).as_ref(), Ok(input), "{lower}");
        assert_eq!(decode(upper).as_ref(), Ok(input), "{upper}");
        // Mixed case: each digit takes the case of its position's parity.
        let mixed: String = lower
            .chars()
            .zip(upper.chars())
            .enumerate()
            .map(|(at, (l, u))| if at % 2 == 0 { l } else { u })
            .collect();
        assert_eq!(decode(&mixed).as_ref(), Ok(input), "{mixed}");
        assert_eq!(decode_canonical(lower).as_ref(), Ok(input), "{lower}");
        match upper.bytes().position(|b| b.is_ascii_uppercase()) {
            None => assert_eq!(decode_canonical(upper).as_ref(), Ok(input)),
            Some(offset) => assert_eq!(
                decode_canonical(upper),
                Err(HexError::InvalidDigit {
                    offset,
                    byte: upper.as_bytes()[offset],
                }),
                "{upper}"
            ),
        }
        if input.len() == 32 {
            let digest: [u8; 32] = input.as_slice().try_into().expect("32 bytes");
            assert_eq!(decode_32(lower), Some(digest));
            assert_eq!(decode_32(upper), Some(digest));
            assert_eq!(decode_32(&mixed), Some(digest));
            assert_eq!(decode_32_canonical(lower), Some(digest));
            let has_letter = upper.bytes().any(|b| b.is_ascii_uppercase());
            assert_eq!(decode_32_canonical(upper), (!has_letter).then_some(digest));
            assert_eq!(Digest32::from_hex(lower), Some(Digest32::new(digest)));
            assert_eq!(&Digest32::new(digest).to_string(), lower);
            assert_eq!(&Digest32::new(digest).to_hex(), lower);
        } else {
            assert_eq!(decode_32(lower), None, "{lower}");
            assert_eq!(decode_32_canonical(lower), None, "{lower}");
        }
    }
}

/// Every byte reads as the frozen digit table says, alone and as either digit
/// of a pair.
fn frozen_digits_read_through_every_reader() {
    for DigitRecord {
        byte,
        any,
        lower,
        upper,
    } in frozen_digits()
    {
        assert_eq!(nibble(byte), any, "{byte:#04x}");
        assert_eq!(nibble_canonical(byte), lower, "{byte:#04x}");
        // The uppercase-canonical space is the either-case one minus `a`-`f`.
        assert_eq!(any.filter(|_| !byte.is_ascii_lowercase()), upper);
        for (pair, offset) in [([byte, b'0'], 0), ([b'0', byte], 1)] {
            let Ok(text) = core::str::from_utf8(&pair) else {
                // A lone byte at or above 0x80 is not `&str`; the non-ASCII
                // case below covers it inside whole characters.
                assert!(byte >= 0x80);
                continue;
            };
            let expected = |value: Option<u8>| {
                value.map_or(Err(HexError::InvalidDigit { offset, byte }), |v| {
                    Ok(vec![if offset == 0 { v << 4 } else { v }])
                })
            };
            assert_eq!(decode(text), expected(any), "{text:?}");
            assert_eq!(decode_canonical(text), expected(lower), "{text:?}");
        }
    }
}

/// Every ASCII byte at every one of the 64 positions of an otherwise valid
/// digest: each invalid class (controls, the bytes below `0`, between `9` and
/// `A`, `G`-`Z`, between `Z` and `a`, `g`-`z`, above `z`, DELETE) and the
/// uppercase `A`-`F` only the canonical reader refuses.
fn decode_32_judges_every_ascii_byte_at_every_position() {
    let digits = frozen_digits();
    let mixed = "fedcbaFEDCBA9876543210".repeat(3)[..64].to_owned();
    let lower = "fedcba9876543210".repeat(4);
    assert!(decode_32(&mixed).is_some() && decode_32_canonical(&mixed).is_none());
    assert!(decode_32(&lower).is_some() && decode_32_canonical(&lower).is_some());
    for base in [&mixed, &lower] {
        for position in 0..64 {
            for record in digits.iter().filter(|record| record.byte < 0x80) {
                let mut bytes = base.clone().into_bytes();
                bytes[position] = record.byte;
                let text = String::from_utf8(bytes).expect("ASCII is UTF-8");
                let any_ok = record.any.is_some();
                let canonical_ok = record.lower.is_some() && decode_canonical(base).is_ok();
                assert_eq!(decode_32(&text).is_some(), any_ok, "{text:?}");
                assert_eq!(
                    decode_32_canonical(&text).is_some(),
                    canonical_ok,
                    "{text:?}"
                );
                assert_eq!(decode(&text).is_ok(), any_ok, "{text:?}");
                if !any_ok {
                    assert_eq!(
                        decode(&text),
                        Err(HexError::InvalidDigit {
                            offset: position,
                            byte: record.byte
                        })
                    );
                }
            }
        }
    }
}

/// Every non-ASCII byte, as the lead or a continuation byte of a UTF-8
/// character standing in for digits at every position it fits, is refused at
/// the character's first byte; and every length other than 64 is refused by
/// the fixed readers.
fn readers_refuse_non_ascii_and_wrong_lengths() {
    let lower = "fedcba9876543210".repeat(4);
    let mut chars: Vec<char> = Vec::new();
    chars.extend(('\u{80}'..='\u{7ff}').step_by(7));
    chars.extend(('\u{800}'..='\u{ffff}').step_by(509));
    chars.extend(('\u{10000}'..='\u{10ffff}').step_by(40_009));
    let mut seen = [false; 256];
    for c in chars {
        let width = c.len_utf8();
        let mut encoded = [0_u8; 4];
        c.encode_utf8(&mut encoded);
        for &b in &encoded[..width] {
            seen[usize::from(b)] = true;
        }
        for position in 0..=64 - width {
            let text = format!("{}{c}{}", &lower[..position], &lower[position + width..]);
            assert_eq!(text.len(), 64);
            assert_eq!(decode_32(&text), None, "{text:?}");
            assert_eq!(decode_32_canonical(&text), None, "{text:?}");
            assert_eq!(
                decode(&text),
                Err(HexError::InvalidDigit {
                    offset: position,
                    byte: encoded[0]
                })
            );
        }
    }
    assert!(
        (0x80..=0xBF).chain(0xC2..=0xF4).all(|b| seen[b]),
        "every continuation and lead byte is exercised"
    );
    for len in [0, 1, 2, 31, 32, 62, 63, 65, 66, 128] {
        let text = "a".repeat(len);
        assert_eq!(decode_32(&text), None, "length {len}");
        assert_eq!(decode_32_canonical(&text), None, "length {len}");
    }
    // The valid neighbour of every wrong length.
    assert!(decode_32(&"a".repeat(64)).is_some());
    assert!(decode_32_canonical(&"a".repeat(64)).is_some());
}

/// An odd number of digits is refused before any digit is judged; the even
/// neighbour one digit shorter is read.
fn decode_refuses_an_odd_length_beside_its_even_neighbour() {
    for text in ["a", "abc", "0g0", "zzz"] {
        assert_eq!(decode(text), Err(HexError::OddLength { len: text.len() }));
        assert_eq!(
            decode_canonical(text),
            Err(HexError::OddLength { len: text.len() })
        );
    }
    assert_eq!(decode("ab"), Ok(vec![0xab]));
    assert_eq!(decode(""), Ok(vec![]));
    assert_eq!(decode_canonical(""), Ok(vec![]));
}

/// No sign, prefix, whitespace or separator is part of either lexical space;
/// the bare digits beside each are.
fn decode_refuses_signs_prefixes_and_whitespace() {
    for (text, offset, byte) in [
        ("+1", 0, b'+'),
        ("-1", 0, b'-'),
        ("0x1f", 1, b'x'),
        (" 1f ", 0, b' '),
        ("1f\n", 2, b'\n'),
        ("1f 2e", 2, b' '),
        ("1f:2e", 2, b':'),
    ] {
        let expected = if text.len() % 2 == 0 {
            Err(HexError::InvalidDigit { offset, byte })
        } else {
            Err(HexError::OddLength { len: text.len() })
        };
        assert_eq!(decode(text), expected, "{text:?}");
    }
    assert_eq!(decode("1f2e"), Ok(vec![0x1f, 0x2e]));
    assert_eq!(nibble(b'+'), None);
    assert_eq!(nibble(b'1'), Some(1));
}

/// A leading zero byte keeps its two digits (the "render as one integer"
/// bug), an empty input renders empty, and every one of the 256 byte values
/// renders as exactly two characters.
fn rendering_keeps_leading_zeros_and_two_digits_per_byte() {
    assert_eq!(encode(&[0x00, 0x00, 0x01]), "000001");
    assert_eq!(encode(&[]), "");
    assert_eq!(Upper(&[]).to_string(), "");
    let all: Vec<u8> = (0..=u8::MAX).collect();
    let rendered = encode(&all);
    assert_eq!(rendered.len(), 512);
    assert_eq!(decode(&rendered), Ok(all.clone()));
    assert_eq!(encode_upper(&all), rendered.to_ascii_uppercase());
    // Array and vector references coerce into the field.
    let digest: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];
    assert_eq!(Lower(&digest).to_string(), "deadbeef");
    assert_eq!(Upper(&digest).to_string(), "DEADBEEF");
}

/// Digest-shaped edge values: all zeros, all ones, a zero leading byte, a
/// byte string mixing a zero, a letter pair and a high-bit-clear byte, and a
/// 64-byte run of every seventh byte value.
fn digest_edge_values_render_whole() {
    assert_eq!(encode(&[0u8; 32]), "0".repeat(64));
    assert_eq!(encode(&[0xffu8; 32]), "f".repeat(64));
    assert_eq!(encode_upper(&[0xffu8; 32]), "F".repeat(64));
    let mut leading_zero = [0x5au8; 32];
    leading_zero[0] = 0;
    let rendered = encode(&leading_zero);
    assert_eq!(rendered.len(), 64);
    assert!(rendered.starts_with("005a"));
    assert_eq!(encode(&[0x00, 0xab, 0x7f]), "00ab7f");
    let sevenths: Vec<u8> = (0..64u16).map(|i| (i * 7 % 256) as u8).collect();
    let rendered = encode(&sevenths);
    assert_eq!(&rendered[..8], "00070e15");
    assert_eq!(decode(&rendered), Ok(sevenths));
}

/// RFC 4648 §10's BASE16 vectors. The RFC prints them in upper case; §8 calls
/// base16 case-insensitive.
fn rfc4648_base16_vectors() {
    let vectors: [(&str, &str); 7] = [
        ("", ""),
        ("f", "66"),
        ("fo", "666F"),
        ("foo", "666F6F"),
        ("foob", "666F6F62"),
        ("fooba", "666F6F6261"),
        ("foobar", "666F6F626172"),
    ];
    for (input, expected) in vectors {
        assert_eq!(Upper(input.as_bytes()).to_string(), expected, "{input:?}");
        assert_eq!(
            Lower(input.as_bytes()).to_string(),
            expected.to_ascii_lowercase(),
            "{input:?}"
        );
        assert_eq!(decode(expected), Ok(input.as_bytes().to_vec()));
        for backend in HexBackend::all_available() {
            let mut out = vec![0u8; 2 * input.len()];
            backend
                .encode_upper(input.as_bytes(), &mut out)
                .expect("an available path");
            assert_eq!(out, expected.as_bytes(), "{input:?} on {}", backend.name());
        }
    }
}

/// Inputs at the length switch and across the 128-byte rendering chunk: no
/// character is lost or repeated at either seam, in either case.
fn inputs_across_the_length_switch_and_chunk_seams_render_whole() {
    for len in [
        SHORT_MAX - 1,
        SHORT_MAX,
        SHORT_MAX + 1,
        127,
        128,
        129,
        255,
        256,
        257,
        1000,
        4096 + 7,
    ] {
        let data = xoshiro256_bytes(len, len as u64);
        let lower = portable(&data, false);
        let upper = portable(&data, true);
        assert_eq!(Lower(&data).to_string(), lower, "length {len}");
        assert_eq!(Upper(&data).to_string(), upper, "length {len}");
        assert_eq!(encode(&data), lower, "length {len}");
        assert_eq!(encode_upper(&data), upper, "length {len}");
        assert_eq!(decode(&lower), Ok(data.clone()), "length {len}");
        assert_eq!(decode(&upper), Ok(data.clone()), "length {len}");
        assert_eq!(decode_canonical(&lower), Ok(data), "length {len}");
    }
}

/// Width, fill, alignment and precision act on the characters as they do on
/// the same text as a `str`, including across chunk seams, in both cases.
fn formatter_options_match_str() {
    for len in [0, 1, 3, 32, 200] {
        let data = xoshiro256_bytes(len, 7);
        for (upper, text) in [
            (false, portable(&data, false)),
            (true, portable(&data, true)),
        ] {
            let rendered: &dyn core::fmt::Display =
                if upper { &Upper(&data) } else { &Lower(&data) };
            assert_eq!(format!("{rendered:>70}"), format!("{text:>70}"));
            assert_eq!(format!("{rendered:<70}"), format!("{text:<70}"));
            assert_eq!(format!("{rendered:*^71}"), format!("{text:*^71}"));
            assert_eq!(format!("{rendered:70}"), format!("{text:70}"));
            assert_eq!(format!("{rendered:.5}"), format!("{text:.5}"));
            assert_eq!(format!("{rendered:.300}"), format!("{text:.300}"));
            assert_eq!(format!("{rendered:-^9.3}"), format!("{text:-^9.3}"));
            assert_eq!(format!("{rendered:.0}"), "");
        }
    }
}

/// A slice too short for the rendering is refused, writing nothing; the
/// exactly-sized neighbour and a longer buffer are written.
fn encode_to_slice_refuses_a_short_output_beside_the_exact_one() {
    let data = xoshiro256_bytes(40, 3);
    for wrong in [0, 1, 79] {
        let mut out = vec![0xAAu8; wrong];
        assert_eq!(
            encode_to_slice(&data, &mut out),
            Err(HexError::OutputTooShort {
                needed: 80,
                available: wrong
            })
        );
        assert_eq!(
            encode_upper_to_slice(&data, &mut out),
            Err(HexError::OutputTooShort {
                needed: 80,
                available: wrong
            })
        );
        assert!(out.iter().all(|&b| b == 0xAA), "nothing written");
    }
    let mut exact = vec![0u8; 80];
    assert_eq!(
        encode_to_slice(&data, &mut exact),
        Ok(portable(&data, false).as_str())
    );
    let mut longer = vec![0xAAu8; 83];
    assert_eq!(
        encode_upper_to_slice(&data, &mut longer).map(str::len),
        Ok(80)
    );
    assert_eq!(&longer[80..], &[0xAA; 3], "nothing past the rendering");
}

/// A backend refuses an output that is not exactly twice the input, writing
/// nothing, beside the exactly-sized neighbour it accepts.
fn backend_encode_refuses_a_missized_output() {
    let data = xoshiro256_bytes(40, 3);
    for backend in HexBackend::all_available() {
        let mut exact = vec![0u8; 80];
        assert_eq!(backend.encode(&data, &mut exact), Some(()));
        assert_eq!(exact, portable(&data, false).as_bytes());
        for wrong in [79, 81, 0] {
            let mut out = vec![0xAAu8; wrong];
            assert_eq!(backend.encode(&data, &mut out), None, "{wrong} bytes");
            assert_eq!(backend.encode_upper(&data, &mut out), None, "{wrong} bytes");
            assert!(out.iter().all(|&b| b == 0xAA), "nothing written");
        }
    }
}

/// Every available path against the portable one, at every length 0..=256,
/// every input and output offset 0..16 into their buffers, in both cases.
fn every_path_matches_portable() {
    let source = xoshiro256_bytes(256 + 16, 0x5eed);
    let mut out_buffer = vec![0u8; 2 * 256 + 16];
    let mut ran = Vec::new();
    for backend in HexBackend::all_available() {
        for upper in [false, true] {
            for len in 0..=256 {
                for in_offset in 0..16 {
                    let data = &source[in_offset..in_offset + len];
                    let expected = portable(data, upper);
                    for out_offset in [in_offset, (in_offset + 5) % 16] {
                        out_buffer.fill(0);
                        let out = &mut out_buffer[out_offset..out_offset + 2 * len];
                        let encode = if upper {
                            HexBackend::encode_upper
                        } else {
                            HexBackend::encode
                        };
                        encode(backend, data, out).expect("an available path");
                        assert_eq!(
                            out,
                            expected.as_bytes(),
                            "{} at length {len}, input offset {in_offset}, output offset {out_offset}",
                            backend.name()
                        );
                        assert!(out_buffer[..out_offset].iter().all(|&b| b == 0));
                        assert!(out_buffer[out_offset + 2 * len..].iter().all(|&b| b == 0));
                    }
                }
            }
        }
        ran.push(backend.name());
    }
    purrdf_testkit::harness::print_line(&format!(
        "hex: {} checked; {} selected",
        ran.join(", "),
        HexBackend::selected().name()
    ));
    assert!(ran.contains(&"portable"));
    // A vector path the build or processor provides is the one selected,
    // except SSSE3 in a build whose target has AVX-512BW.
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    assert_eq!(HexBackend::selected(), HexBackend::Wasm32Simd128);
    #[cfg(all(target_arch = "wasm32", not(target_feature = "simd128")))]
    assert_eq!(HexBackend::selected(), HexBackend::Portable);
    #[cfg(target_arch = "aarch64")]
    assert_eq!(HexBackend::selected(), HexBackend::Aarch64Neon);
    #[cfg(all(target_arch = "x86_64", target_feature = "avx512bw"))]
    assert_eq!(HexBackend::selected(), HexBackend::Portable);
    #[cfg(all(target_arch = "x86_64", not(target_feature = "avx512bw")))]
    assert_eq!(
        HexBackend::selected(),
        if HexBackend::X86Ssse3.is_available() {
            HexBackend::X86Ssse3
        } else {
            HexBackend::Portable
        }
    );
}

/// Whether this host is expected to run a base16 path: its architecture and
/// build.
fn expected_here(backend: HexBackend) -> bool {
    match backend {
        HexBackend::Portable => true,
        HexBackend::X86Ssse3 => cfg!(target_arch = "x86_64") && host_advertises(&["ssse3"]),
        HexBackend::Aarch64Neon => cfg!(target_arch = "aarch64"),
        HexBackend::Wasm32Simd128 => cfg!(all(target_arch = "wasm32", target_feature = "simd128")),
    }
}

/// Every base16 path `PURRDF_REQUIRE_SIMD_PATHS` requires is available, and a
/// required vector path is the one inputs longer than [`SHORT_MAX`] render
/// through (SSSE3 only below AVX-512BW, where it is the measured choice).
fn required_paths_are_available_and_selected() {
    for backend in assert_required_available("hex", expected_here) {
        let deselected = cfg!(target_feature = "avx512bw") && backend == HexBackend::X86Ssse3;
        if backend != HexBackend::Portable && !deselected {
            assert_eq!(
                HexBackend::selected(),
                backend,
                "hex:{} is required but not selected",
                backend.name()
            );
        }
    }
}

/// [`Digest32`]: the canonical rendering, its reading (lowercase only, beside
/// the uppercase spelling it refuses), the conversions and `Debug`.
fn digest32_renders_and_reads_its_canonical_form() {
    let bytes: [u8; 32] = core::array::from_fn(|i| (i * 37) as u8);
    let digest = Digest32::new(bytes);
    let text = encode(&bytes);
    assert_eq!(digest.to_string(), text);
    assert_eq!(digest.to_hex(), text);
    assert_eq!(format!("{digest:?}"), format!("Digest32({text})"));
    assert_eq!(Digest32::from_hex(&text), Some(digest));
    assert_eq!(Digest32::from_hex(&text.to_ascii_uppercase()), None);
    assert_eq!(Digest32::from_hex(&text[..62]), None);
    assert_eq!(digest.as_bytes(), &bytes);
    assert_eq!(digest.into_bytes(), bytes);
    assert_eq!(<[u8; 32]>::from(digest), bytes);
    assert_eq!(Digest32::from(bytes), digest);
    assert_eq!(digest.as_ref(), bytes.as_slice());
    assert_eq!(Digest32::LEN, 32);
    assert_eq!(Digest32::default().to_string(), "0".repeat(64));
    assert!(Digest32::new([0; 32]) < Digest32::new([1; 32]));
}

/// Each refusal names what is wrong in its message.
fn errors_name_what_is_wrong() {
    assert_eq!(
        HexError::OddLength { len: 3 }.to_string(),
        "hex text of 3 bytes has an odd number of digits"
    );
    assert_eq!(
        HexError::InvalidDigit {
            offset: 4,
            byte: b'g'
        }
        .to_string(),
        "`g` at byte 4 is not a hex digit"
    );
    assert_eq!(
        HexError::InvalidDigit {
            offset: 0,
            byte: b'\n'
        }
        .to_string(),
        "byte 0x0a at offset 0 is not a hex digit"
    );
    assert_eq!(
        HexError::OutputTooShort {
            needed: 4,
            available: 2
        }
        .to_string(),
        "a hex rendering of 4 bytes does not fit in 2"
    );
    let _: &dyn core::error::Error = &HexError::OddLength { len: 1 };
}

/// The reference rendering: the portable path, itself pinned by the frozen
/// table above.
fn portable(bytes: &[u8], upper: bool) -> String {
    let mut out = vec![0u8; 2 * bytes.len()];
    let encode = if upper {
        HexBackend::encode_upper
    } else {
        HexBackend::encode
    };
    encode(HexBackend::Portable, bytes, &mut out).expect("the portable path is always available");
    String::from_utf8(out).expect("digits are ASCII")
}

purrdf_testkit::harness_main!(
    frozen_encodings_render_through_every_entry_point,
    frozen_encodings_read_back_through_every_reader,
    frozen_digits_read_through_every_reader,
    decode_32_judges_every_ascii_byte_at_every_position,
    readers_refuse_non_ascii_and_wrong_lengths,
    decode_refuses_an_odd_length_beside_its_even_neighbour,
    decode_refuses_signs_prefixes_and_whitespace,
    rendering_keeps_leading_zeros_and_two_digits_per_byte,
    digest_edge_values_render_whole,
    rfc4648_base16_vectors,
    inputs_across_the_length_switch_and_chunk_seams_render_whole,
    formatter_options_match_str,
    encode_to_slice_refuses_a_short_output_beside_the_exact_one,
    backend_encode_refuses_a_missized_output,
    every_path_matches_portable,
    required_paths_are_available_and_selected,
    digest32_renders_and_reads_its_canonical_form,
    errors_name_what_is_wrong,
);
