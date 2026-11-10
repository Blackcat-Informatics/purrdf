// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Strict mapping, chunk independence, original refusals and live workspace.

use core::convert::Infallible;
use purrdf_lex::control::Unmetered;
use purrdf_lex::gb18030::{Direction, ErrorKind, Extent, Limits, Profile, StreamError, Transcoder};

const LIMITS: Limits = Limits {
    input_bytes: u64::MAX,
    output_bytes: u64::MAX,
};
const PROFILES: [Profile; 2] = [Profile::Gb18030_2005, Profile::Gb18030_2022Amendment1];

fn one(profile: Profile, direction: Direction, input: &[u8]) -> ([u8; 4], usize) {
    let mut bytes = [0; 4];
    let mut width = 0;
    Transcoder::new(profile, direction, LIMITS)
        .push(input, true, &mut Unmetered, |emission| {
            assert_eq!(
                emission.source,
                Extent {
                    start: 0,
                    end: input.len() as u64
                }
            );
            assert_eq!(
                emission.output,
                Extent {
                    start: 0,
                    end: emission.bytes.len() as u64
                }
            );
            bytes[..emission.bytes.len()].copy_from_slice(emission.bytes);
            width = emission.bytes.len();
            Ok::<_, Infallible>(())
        })
        .expect("representable scalar");
    (bytes, width)
}

#[test]
fn every_unicode_scalar_roundtrips_bijectively_in_each_profile_without_allocation() {
    for profile in PROFILES {
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let mut count = 0;
        for scalar in (0..=0x10_ffff).filter_map(char::from_u32) {
            let mut utf8 = [0; 4];
            let original = scalar.encode_utf8(&mut utf8).as_bytes();
            let (encoded, width) = one(profile, Direction::Encode, original);
            let (decoded, decoded_width) = one(profile, Direction::Decode, &encoded[..width]);
            assert_eq!(
                &decoded[..decoded_width],
                original,
                "{profile:?} U+{:X}",
                u32::from(scalar)
            );
            count += 1;
        }
        assert_eq!(count, 1_112_064);
        assert_eq!(window.close().allocations, 0);
    }
}

#[test]
fn legacy_mapping_matches_the_independently_pinned_reference() {
    let reference = include_str!("../data/gb18030/gb18030-2005.ucm");
    let mut checked = 0;
    for line in reference.lines().filter(|line| line.starts_with("<U")) {
        let mut fields = line.split_whitespace();
        let scalar = fields.next().expect("scalar field");
        let scalar = purrdf_hash::hex::parse_u32(&scalar.as_bytes()[2..scalar.len() - 1])
            .expect("hex scalar");
        let scalar = char::from_u32(scalar).expect("reference scalar");
        let mut encoded = [0; 4];
        let mut width = 0;
        for byte in fields.next().expect("bytes").split("\\x").skip(1) {
            width += purrdf_hash::hex::decode_to_slice(byte, &mut encoded[width..])
                .expect("reference byte");
        }
        assert_eq!(fields.next(), Some("|0"));
        let mut utf8 = [0; 4];
        let utf8 = scalar.encode_utf8(&mut utf8).as_bytes();
        let (output, n) = one(Profile::Gb18030_2005, Direction::Decode, &encoded[..width]);
        assert_eq!(&output[..n], utf8);
        let (output, n) = one(Profile::Gb18030_2005, Direction::Encode, utf8);
        assert_eq!(&output[..n], &encoded[..width]);
        checked += 1;
    }
    assert!(checked > 30000);
}

#[test]
fn reciprocal_editions_amendment_and_non_web_mapping_boundaries_are_exact() {
    for (encoded, legacy, current) in [
        (&[0xa8, 0xbc][..], '\u{1e3f}', '\u{1e3f}'),
        (&[0x81, 0x35, 0xf4, 0x37], '\u{e7c7}', '\u{e7c7}'),
        (&[0xfe, 0x59], '\u{e81e}', '\u{9fb4}'),
        (&[0x82, 0x35, 0x90, 0x37], '\u{9fb4}', '\u{e81e}'),
        (&[0xa6, 0xd9], '\u{e78d}', '\u{fe10}'),
        (&[0x84, 0x31, 0x82, 0x36], '\u{fe10}', '\u{e78d}'),
        (&[0xa3, 0xa0], '\u{e5e5}', '\u{e5e5}'),
        (&[0xa2, 0xe3], '€', '€'),
        (&[0xfe, 0x51], '\u{e816}', '\u{e816}'),
        (&[0xfd, 0x9c], '\u{f92c}', '\u{f92c}'),
        (&[0x81, 0x39, 0xa3, 0x32], '\u{2ffc}', '\u{2ffc}'),
        (&[0x81, 0x39, 0xa3, 0x35], '\u{2fff}', '\u{2fff}'),
        (&[0x81, 0x39, 0xbc, 0x33], '\u{31ef}', '\u{31ef}'),
        (&[0x90, 0x30, 0x81, 0x30], '\u{10000}', '\u{10000}'),
        (&[0xe3, 0x32, 0x9a, 0x35], '\u{10ffff}', '\u{10ffff}'),
    ] {
        for (profile, scalar) in PROFILES.into_iter().zip([legacy, current]) {
            let (output, width) = one(profile, Direction::Decode, encoded);
            let mut utf8 = [0; 4];
            assert_eq!(&output[..width], scalar.encode_utf8(&mut utf8).as_bytes());
            let (output, width) = one(
                profile,
                Direction::Encode,
                scalar.encode_utf8(&mut utf8).as_bytes(),
            );
            assert_eq!(&output[..width], encoded);
        }
    }
}

#[test]
fn all_scalar_splits_preserve_complete_source_contributions_in_both_directions() {
    let text = "A\0€中\u{9fb4}\u{e81e}\u{2ffc}\u{20087}\u{10ffff}\r\n";
    for profile in PROFILES {
        let mut encoded = Vec::new();
        Transcoder::new(profile, Direction::Encode, LIMITS)
            .push(text.as_bytes(), true, &mut Unmetered, |item| {
                encoded.extend_from_slice(item.bytes);
                Ok::<_, Infallible>(())
            })
            .expect("encoding");
        for (direction, input) in [
            (Direction::Decode, encoded.as_slice()),
            (Direction::Encode, text.as_bytes()),
        ] {
            let convert = |chunks: &[&[u8]]| {
                let mut codec = Transcoder::new(profile, direction, LIMITS);
                let mut result = Vec::new();
                for (index, chunk) in chunks.iter().enumerate() {
                    codec
                        .push(chunk, index + 1 == chunks.len(), &mut Unmetered, |item| {
                            result.push((
                                item.scalar,
                                item.source,
                                item.output,
                                item.bytes.to_vec(),
                            ));
                            Ok::<_, Infallible>(())
                        })
                        .expect("split scalar");
                }
                result
            };
            let expected = convert(&[input]);
            for a in 0..=input.len() {
                for b in a..=input.len() {
                    assert_eq!(convert(&[&input[..a], &input[a..b], &input[b..]]), expected);
                }
            }
            assert_eq!(convert(&input.chunks(1).collect::<Vec<_>>()), expected);
        }
    }
}

#[test]
fn malformed_incomplete_and_reserved_neighbors_keep_source_and_poison_the_stream() {
    for profile in PROFILES {
        for (direction, input, kind) in [
            (Direction::Decode, &[0x80][..], ErrorKind::Malformed),
            (Direction::Decode, &[0x81, 0x7f], ErrorKind::Malformed),
            (Direction::Decode, &[0x81, 0x30, 0x80], ErrorKind::Malformed),
            (
                Direction::Decode,
                &[0x81, 0x30, 0x81, 0x40],
                ErrorKind::Malformed,
            ),
            (
                Direction::Decode,
                &[0x84, 0x31, 0xa4, 0x3a],
                ErrorKind::Malformed,
            ),
            (
                Direction::Decode,
                &[0x84, 0x31, 0xa5, 0x30],
                ErrorKind::Unassigned,
            ),
            (Direction::Decode, &[0x81], ErrorKind::Incomplete),
            (Direction::Decode, &[0x81, 0x30], ErrorKind::Incomplete),
            (
                Direction::Decode,
                &[0x90, 0x30, 0x81],
                ErrorKind::Incomplete,
            ),
            (
                Direction::Decode,
                &[0xe3, 0x32, 0x9a, 0x36],
                ErrorKind::Unassigned,
            ),
            (Direction::Encode, &[0xed, 0xa0], ErrorKind::Malformed),
            (Direction::Encode, &[0xf4, 0x90], ErrorKind::Malformed),
            (Direction::Encode, &[0xe4, 0xb8], ErrorKind::Incomplete),
        ] {
            for split in 0..=input.len() {
                let mut codec = Transcoder::new(profile, direction, LIMITS);
                codec
                    .push(b"A", false, &mut Unmetered, |_| Ok::<_, Infallible>(()))
                    .expect("valid prefix");
                let first = codec.push(&input[..split], false, &mut Unmetered, |_| {
                    Ok::<_, Infallible>(())
                });
                let result = first.and_then(|_| {
                    codec.push(&input[split..], true, &mut Unmetered, |_| {
                        Ok::<_, Infallible>(())
                    })
                });
                let StreamError::Native(error) = result.expect_err("strict refusal");
                assert_eq!(error.profile, profile);
                assert_eq!(error.direction, direction);
                assert_eq!(error.kind, kind);
                assert_eq!(error.source.start, 1);
                assert_eq!(
                    &error.bytes[..usize::from(error.length)],
                    &input[..usize::from(error.length)]
                );
                assert_eq!(error.source.end, 1 + u64::from(error.length));
                if direction == Direction::Encode {
                    assert!(error.utf8.is_some());
                }
                assert!(
                    matches!(codec.push(&[],true,&mut Unmetered, |_| Ok::<_,Infallible>(())),Err(StreamError::Native(error)) if error.kind==ErrorKind::Closed)
                );
            }
        }
        for valid in [
            &[0x81, 0x80][..],
            &[0x81, 0x30, 0x81, 0x30],
            &[0x84, 0x31, 0xa4, 0x39],
            &[0x90, 0x30, 0x81, 0x30],
            &[0xe3, 0x32, 0x9a, 0x35],
        ] {
            one(profile, Direction::Decode, valid);
        }
    }
}

#[test]
fn output_work_and_original_admission_refusals_never_become_successful_prefixes() {
    let profile = Profile::Gb18030_2022Amendment1;
    let mut codec = Transcoder::new(
        profile,
        Direction::Decode,
        Limits {
            input_bytes: 2,
            output_bytes: 2,
        },
    );
    let error = codec
        .push(&[0xd6, 0xd0], true, &mut Unmetered, |_| {
            Ok::<_, Infallible>(())
        })
        .expect_err("three UTF8 bytes");
    assert!(
        matches!(error,StreamError::Native(error) if error.kind==ErrorKind::OutputLimit {limit:2})
    );
    let mut codec = Transcoder::new(
        profile,
        Direction::Decode,
        Limits {
            input_bytes: 1,
            output_bytes: 3,
        },
    );
    let error = codec
        .push(&[0xd6, 0xd0], true, &mut Unmetered, |_| {
            Ok::<_, Infallible>(())
        })
        .expect_err("source ceiling");
    assert!(
        matches!(error,StreamError::Native(error) if error.kind==ErrorKind::InputLimit {limit:1})
    );
    let mut codec = Transcoder::new(
        profile,
        Direction::Decode,
        Limits {
            input_bytes: 2,
            output_bytes: 3,
        },
    );
    codec
        .push(&[0xd6, 0xd0], true, &mut Unmetered, |_| {
            Ok::<_, Infallible>(())
        })
        .expect("exact limits");
    let mut codec = Transcoder::new(profile, Direction::Decode, LIMITS);
    let token = Box::new(42);
    let pointer = &raw const *token;
    let mut token = Some(token);
    let error = codec
        .push(
            b"A",
            true,
            &mut |_: u64| Err(token.take().expect("one cancellation poll")),
            |_| Ok::<_, Infallible>(()),
        )
        .expect_err("cancelled");
    let StreamError::Control(original) = error else {
        panic!("control owner");
    };
    assert_eq!(&raw const *original, pointer);
    let mut budget = purrdf_lex::allocation::Ceiling(0);
    let mut memory = purrdf_lex::allocation::Memory::new(&mut budget);
    let mut output = String::new();
    let mut codec = Transcoder::new(profile, Direction::Decode, LIMITS);
    let error = codec
        .push(b"A", true, &mut Unmetered, |item| {
            memory.reserve_string(&mut output, item.bytes.len())
        })
        .expect_err("original allocation admission");
    assert!(matches!(
        error,
        StreamError::Output(purrdf_lex::allocation::StorageError::AdmissionFailed)
    ));
    assert_eq!(output, "");
    assert!(
        codec
            .push(&[], true, &mut Unmetered, |_| Ok::<_, Infallible>(()))
            .is_err()
    );
}
