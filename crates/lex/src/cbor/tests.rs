// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::encode::bottom_up;
use super::head::{self, ARRAY, BYTES, MAP, TEXT};
use super::{
    DecodeErrorKind, Integer, IntegerRangeError, Limits, Value, canonical, decode,
    decode_deterministic, decode_prefix, decode_sequence, encode, encoded_len, read_from,
    write_canonical_map,
};

fn hex(text: &str) -> Vec<u8> {
    let text: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    (0..text.len())
        .step_by(2)
        .map(|at| {
            let digit = |at: usize| purrdf_hash::hex::nibble(text.as_bytes()[at]).expect("hex");
            (digit(at) << 4) | digit(at + 1)
        })
        .collect()
}

fn int(value: i128) -> Value {
    Value::Integer(Integer::try_from(value).expect("in range"))
}

fn text(value: &str) -> Value {
    Value::from(value)
}

fn array(items: Vec<Value>) -> Value {
    Value::Array(items)
}

fn map(entries: Vec<(Value, Value)>) -> Value {
    Value::Map(entries)
}

fn kind(bytes: &[u8]) -> DecodeErrorKind {
    decode(bytes, Limits::DEFAULT).expect_err("refused").kind()
}

fn deterministic_kind(bytes: &[u8]) -> DecodeErrorKind {
    decode_deterministic(bytes, Limits::DEFAULT)
        .expect_err("refused")
        .kind()
}

/// Every RFC 8949 Appendix A example that has a definite, preferred encoding:
/// the value and its bytes.
fn appendix_a() -> Vec<(Value, &'static str)> {
    let one_to = |n: i128| array((1..=n).map(int).collect());
    vec![
        (int(0), "00"),
        (int(1), "01"),
        (int(10), "0a"),
        (int(23), "17"),
        (int(24), "1818"),
        (int(25), "1819"),
        (int(100), "1864"),
        (int(1000), "1903e8"),
        (int(1_000_000), "1a000f4240"),
        (int(1_000_000_000_000), "1b000000e8d4a51000"),
        (int(18_446_744_073_709_551_615), "1bffffffffffffffff"),
        (int(-18_446_744_073_709_551_616), "3bffffffffffffffff"),
        (int(-1), "20"),
        (int(-10), "29"),
        (int(-100), "3863"),
        (int(-1000), "3903e7"),
        (Value::Float(0.0), "f90000"),
        (Value::Float(-0.0), "f98000"),
        (Value::Float(1.0), "f93c00"),
        (Value::Float(1.1), "fb3ff199999999999a"),
        (Value::Float(1.5), "f93e00"),
        (Value::Float(65504.0), "f97bff"),
        (Value::Float(100_000.0), "fa47c35000"),
        (Value::Float(3.402_823_466_385_288_6e38), "fa7f7fffff"),
        (Value::Float(1.0e300), "fb7e37e43c8800759c"),
        (Value::Float(5.960_464_477_539_063e-8), "f90001"),
        (Value::Float(0.000_061_035_156_25), "f90400"),
        (Value::Float(-4.0), "f9c400"),
        (Value::Float(-4.1), "fbc010666666666666"),
        (Value::Float(f64::INFINITY), "f97c00"),
        (Value::Float(f64::NAN), "f97e00"),
        (Value::Float(f64::NEG_INFINITY), "f9fc00"),
        (Value::Bool(false), "f4"),
        (Value::Bool(true), "f5"),
        (Value::Null, "f6"),
        (Value::Simple(23), "f7"),
        (Value::Simple(16), "f0"),
        (Value::Simple(255), "f8ff"),
        (
            Value::Tag(0, Box::new(text("2013-03-21T20:04:00Z"))),
            "c074323031332d30332d32315432303a30343a30305a",
        ),
        (Value::Tag(1, Box::new(int(1_363_896_240))), "c11a514b67b0"),
        (
            Value::Tag(1, Box::new(Value::Float(1_363_896_240.5))),
            "c1fb41d452d9ec200000",
        ),
        (
            Value::Tag(23, Box::new(Value::Bytes(vec![1, 2, 3, 4]))),
            "d74401020304",
        ),
        (
            Value::Tag(24, Box::new(Value::Bytes(b"dIETF".to_vec()))),
            "d818456449455446",
        ),
        (
            Value::Tag(32, Box::new(text("http://www.example.com"))),
            "d82076687474703a2f2f7777772e6578616d706c652e636f6d",
        ),
        (
            Value::Tag(2, Box::new(Value::Bytes(hex("010000000000000000")))),
            "c249010000000000000000",
        ),
        (Value::Bytes(Vec::new()), "40"),
        (Value::Bytes(vec![1, 2, 3, 4]), "4401020304"),
        (text(""), "60"),
        (text("a"), "6161"),
        (text("IETF"), "6449455446"),
        (text("\"\\"), "62225c"),
        (text("\u{fc}"), "62c3bc"),
        (text("\u{6c34}"), "63e6b0b4"),
        (text("\u{10151}"), "64f0908591"),
        (array(vec![]), "80"),
        (array(vec![int(1), int(2), int(3)]), "83010203"),
        (
            array(vec![
                int(1),
                array(vec![int(2), int(3)]),
                array(vec![int(4), int(5)]),
            ]),
            "8301820203820405",
        ),
        (
            one_to(25),
            "98190102030405060708090a0b0c0d0e0f101112131415161718181819",
        ),
        (map(vec![]), "a0"),
        (map(vec![(int(1), int(2)), (int(3), int(4))]), "a201020304"),
        (
            map(vec![
                (text("a"), int(1)),
                (text("b"), array(vec![int(2), int(3)])),
            ]),
            "a26161016162820203",
        ),
        (
            array(vec![text("a"), map(vec![(text("b"), text("c"))])]),
            "826161a161626163",
        ),
        (
            map(["a", "b", "c", "d", "e"]
                .into_iter()
                .map(|k| (text(k), text(&k.to_uppercase())))
                .collect()),
            "a56161614161626142616361436164614461656145",
        ),
    ]
}

#[test]
fn the_encoder_writes_the_rfc_8949_appendix_a_bytes() {
    for (value, expected) in appendix_a() {
        let expected = hex(expected);
        assert_eq!(encode(&value), expected, "{value:?}");
        // Every map in the appendix is already in deterministic order.
        assert_eq!(canonical(&value), expected, "{value:?}");
        assert_eq!(encoded_len(&value), expected.len(), "{value:?}");
    }
}

#[test]
fn the_decoder_reads_every_appendix_a_example_back() {
    for (value, bytes) in appendix_a() {
        let bytes = hex(bytes);
        let decoded = decode(&bytes, Limits::DEFAULT).unwrap();
        // NaN is unequal to itself; compare it by encoding.
        assert_eq!(encode(&decoded), bytes, "{value:?}");
        if !matches!(value, Value::Float(f) if f.is_nan()) {
            assert_eq!(decoded, value);
        }
        assert_eq!(
            decode_deterministic(&bytes, Limits::DEFAULT).map(|v| encode(&v)),
            Ok(bytes)
        );
    }
}

#[test]
fn indefinite_and_non_preferred_appendix_a_forms_decode_to_their_preferred_value() {
    let one_to_25 = || array((1..=25).map(int).collect());
    let nested = || {
        array(vec![
            int(1),
            array(vec![int(2), int(3)]),
            array(vec![int(4), int(5)]),
        ])
    };
    for (bytes, value) in [
        ("5f42010243030405ff", Value::Bytes(vec![1, 2, 3, 4, 5])),
        ("7f657374726561646d696e67ff", text("streaming")),
        ("9fff", array(vec![])),
        ("9f018202039f0405ffff", nested()),
        ("9f01820203820405ff", nested()),
        ("83018202039f0405ff", nested()),
        ("83019f0203ff820405", nested()),
        (
            "9f0102030405060708090a0b0c0d0e0f101112131415161718181819ff",
            one_to_25(),
        ),
        (
            "bf61610161629f0203ffff",
            map(vec![
                (text("a"), int(1)),
                (text("b"), array(vec![int(2), int(3)])),
            ]),
        ),
        (
            "826161bf61626163ff",
            array(vec![text("a"), map(vec![(text("b"), text("c"))])]),
        ),
        (
            "bf6346756ef563416d7421ff",
            map(vec![
                (text("Fun"), Value::Bool(true)),
                (text("Amt"), int(-2)),
            ]),
        ),
        ("fa7f800000", Value::Float(f64::INFINITY)),
        ("fb7ff0000000000000", Value::Float(f64::INFINITY)),
        ("faff800000", Value::Float(f64::NEG_INFINITY)),
    ] {
        let bytes = hex(bytes);
        assert_eq!(
            decode(&bytes, Limits::DEFAULT).unwrap(),
            value,
            "{bytes:02x?}"
        );
        assert!(matches!(
            deterministic_kind(&bytes),
            DecodeErrorKind::NotDeterministic(_)
        ));
    }
    for bytes in ["fa7fc00000", "fb7ff8000000000000"] {
        let value = decode(&hex(bytes), Limits::DEFAULT).unwrap();
        assert_eq!(encode(&value), hex("f97e00"));
    }
}

// ---- the deterministic decoder: refusals and their neighbours --------------

#[test]
fn a_non_shortest_head_is_refused_deterministically_and_the_shortest_is_accepted() {
    for (long, short) in [
        ("1805", "05"),
        ("190017", "17"),
        ("1900ff", "18ff"),
        ("1a0000ffff", "19ffff"),
        ("1b00000000ffffffff", "1affffffff"),
        ("3817", "37"),
        ("5801ff", "41ff"),
        ("780161", "6161"),
        ("9800", "80"),
        ("b800", "a0"),
        ("d80000", "c000"),
    ] {
        assert_eq!(
            deterministic_kind(&hex(long)),
            DecodeErrorKind::NotDeterministic("an argument must use its shortest form"),
            "{long}"
        );
        let decoded = decode(&hex(long), Limits::DEFAULT).expect("well-formed");
        assert_eq!(encode(&decoded), hex(short));
        assert!(
            decode_deterministic(&hex(short), Limits::DEFAULT).is_ok(),
            "{short}"
        );
    }
    // The boundary values themselves are shortest.
    for shortest in ["1818", "1900ff", "1a00010000", "1b0000000100000000"] {
        let bytes = hex(shortest);
        if bytes[0] == 0x19 {
            assert!(decode_deterministic(&bytes, Limits::DEFAULT).is_err());
        } else {
            assert!(
                decode_deterministic(&bytes, Limits::DEFAULT).is_ok(),
                "{shortest}"
            );
        }
    }
    assert!(decode_deterministic(&hex("190100"), Limits::DEFAULT).is_ok());
}

#[test]
fn an_indefinite_length_is_refused_deterministically_and_accepted_otherwise() {
    for bytes in ["9f01ff", "bf6161f5ff", "5f4101ff", "7f6161ff"] {
        let bytes = hex(bytes);
        assert_eq!(
            deterministic_kind(&bytes),
            DecodeErrorKind::NotDeterministic("a length must be definite")
        );
        assert!(decode(&bytes, Limits::DEFAULT).is_ok());
    }
}

#[test]
fn unsorted_or_repeated_map_keys_are_refused_deterministically() {
    // {"id": 1, "x": 2}: RFC 8949 bytewise order puts "x" (61 78) before "id"
    // (62 69 64), where RFC 7049 length-first order would not.
    let unsorted = hex("a2626964016178 02");
    assert_eq!(
        deterministic_kind(&unsorted),
        DecodeErrorKind::NotDeterministic("map keys must be in strictly ascending bytewise order")
    );
    let value = decode(&unsorted, Limits::DEFAULT).unwrap();
    let sorted = canonical(&value);
    assert_eq!(sorted, hex("a2617802626964 01"));
    assert_eq!(decode_deterministic(&sorted, Limits::DEFAULT).unwrap(), {
        let mut value = value;
        value.canonicalize();
        value
    });
    let repeated = hex("a2616101616102");
    assert!(decode_deterministic(&repeated, Limits::DEFAULT).is_err());
    assert_eq!(
        decode(&repeated, Limits::DEFAULT).unwrap(),
        map(vec![(text("a"), int(1)), (text("a"), int(2))])
    );
    // Container keys are ordered by their whole encodings too.
    let nested = map(vec![
        (array(vec![int(2)]), int(0)),
        (array(vec![int(1)]), int(1)),
    ]);
    assert!(decode_deterministic(&encode(&nested), Limits::DEFAULT).is_err());
    assert!(decode_deterministic(&canonical(&nested), Limits::DEFAULT).is_ok());
}

#[test]
fn a_float_a_shorter_float_holds_is_refused_deterministically() {
    for (long, short) in [
        ("fa3fc00000", "f93e00"),
        ("fb3ff8000000000000", "f93e00"),
        ("fb3fb999999999999a", "fb3fb999999999999a"),
    ] {
        if long == short {
            assert!(decode_deterministic(&hex(long), Limits::DEFAULT).is_ok());
            continue;
        }
        assert_eq!(
            deterministic_kind(&hex(long)),
            DecodeErrorKind::NotDeterministic("a float must use its shortest exact form")
        );
        assert!(decode_deterministic(&hex(short), Limits::DEFAULT).is_ok());
    }
}

// ---- well-formedness ---------------------------------------------------------

#[test]
fn malformed_items_are_refused_and_their_well_formed_neighbours_accepted() {
    for (bad, good) in [
        ("1c", "18ff"),
        ("ff", "f6"),
        ("f810", "f820"),
        ("5f6161ff", "5f4161ff"),
        ("5f5f4100ffff", "5f4100ff"),
        ("7f5f4100ffff", "7f6100ff"),
        ("1f", "1b0000000000000000"),
        ("3f", "20"),
        ("df00", "c000"),
        ("ff00", "00"),
        ("bf6161ff", "bf6161f6ff"),
        ("81ff", "9fff"),
    ] {
        assert!(
            matches!(kind(&hex(bad)), DecodeErrorKind::Malformed(_)),
            "{bad}: {:?}",
            kind(&hex(bad))
        );
        assert!(decode(&hex(good), Limits::DEFAULT).is_ok(), "{good}");
    }
}

#[test]
fn text_must_be_utf8_chunk_by_chunk() {
    assert_eq!(kind(&hex("62c328")), DecodeErrorKind::InvalidUtf8);
    assert_eq!(kind(&hex("7f61c361bcff")), DecodeErrorKind::InvalidUtf8);
    assert_eq!(
        decode(&hex("7f62c3bcff"), Limits::DEFAULT).unwrap(),
        text("\u{fc}")
    );
}

#[test]
fn truncated_input_is_refused_without_allocating_its_declared_length() {
    for bytes in [
        "19",
        "1901",
        "4401",
        "6261",
        "8201",
        "a101",
        "c0",
        "5bffffffffffffffff",
        "9bffffffffffffffff00",
    ] {
        assert!(
            matches!(
                kind(&hex(bytes)),
                DecodeErrorKind::Truncated | DecodeErrorKind::TooLong
            ),
            "{bytes}"
        );
    }
    assert_eq!(kind(&hex("0000")), DecodeErrorKind::Trailing);
    assert_eq!(
        decode_prefix(&hex("0000"), Limits::DEFAULT).unwrap(),
        (int(0), 1)
    );
}

#[test]
fn the_depth_cap_accepts_n_open_items_and_refuses_n_plus_one() {
    for cap in [0_usize, 1, 5, 256] {
        let limits = Limits { max_depth: cap };
        let nested = |depth: usize, byte: u8| {
            let mut bytes = vec![byte; depth];
            bytes.push(0x00);
            bytes
        };
        for opener in [0x81_u8, 0xc1] {
            assert!(decode(&nested(cap, opener), limits).is_ok(), "{cap}");
            assert_eq!(
                decode(&nested(cap + 1, opener), limits).unwrap_err().kind(),
                DecodeErrorKind::Depth { limit: cap }
            );
        }
    }
    // An empty array or map counts as open, as it does in JSON.
    assert!(decode(&hex("8180"), Limits { max_depth: 1 }).is_err());
    assert!(decode(&hex("8180"), Limits { max_depth: 2 }).is_ok());
}

// ---- integers ---------------------------------------------------------------

#[test]
fn integers_outside_the_major_type_range_are_refused_and_its_ends_accepted() {
    assert_eq!(Integer::try_from(1_i128 << 64), Err(IntegerRangeError));
    assert_eq!(
        Integer::try_from(-(1_i128 << 64) - 1),
        Err(IntegerRangeError)
    );
    assert_eq!(Integer::try_from((1_i128 << 64) - 1), Ok(Integer::MAX));
    assert_eq!(Integer::try_from(-(1_i128 << 64)), Ok(Integer::MIN));
    assert_eq!(Integer::try_from(u128::from(u64::MAX)), Ok(Integer::MAX));
    assert!(Integer::try_from(u128::from(u64::MAX) + 1).is_err());
    assert_eq!(u64::try_from(Integer::MAX), Ok(u64::MAX));
    assert!(i64::try_from(Integer::MAX).is_err());
    assert_eq!(i128::from(Integer::from(-7_i8)), -7);
}

// ---- floats -----------------------------------------------------------------

#[test]
fn every_binary16_pattern_widens_and_narrows_back_to_itself() {
    for half in 0..=u16::MAX {
        let mut bytes = vec![0xf9];
        bytes.extend_from_slice(&half.to_be_bytes());
        let value = decode(&bytes, Limits::DEFAULT).unwrap();
        assert_eq!(encode(&value), bytes, "{half:#06x}");
    }
}

#[test]
fn a_float_is_written_in_the_narrowest_exact_width() {
    for (value, width) in [
        (65504.0, 3),
        (65505.0, 5),
        (f64::from(f32::MAX), 5),
        (f64::from(f32::MIN_POSITIVE), 5),
        (f64::from(f32::from_bits(1)), 5),
        (f64::from(f32::from_bits(1)) / 2.0, 9),
        (2.0_f64.powi(-24), 3),
        (2.0_f64.powi(-25), 5),
        (3.0 * 2.0_f64.powi(-24), 3),
        (0.1, 9),
        (f64::from_bits(0x7ff8_0000_0000_0001), 9),
        (f64::from_bits(0x7ff8_0000_2000_0000), 5),
    ] {
        let bytes = encode(&Value::Float(value));
        assert_eq!(bytes.len(), width, "{value:e}");
        let back = decode(&bytes, Limits::DEFAULT).unwrap().as_float().unwrap();
        assert_eq!(back.to_bits(), value.to_bits(), "{value:e}");
    }
}

// ---- the canonical encoders ---------------------------------------------------

/// A fixed-seed generator (SplitMix64), so every run draws the same values.
struct SplitMix(u64);

impl SplitMix {
    const fn next(&mut self) -> u64 {
        purrdf_testkit::rng::splitmix64_next(&mut self.0)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn arbitrary(rng: &mut SplitMix, depth: usize) -> Value {
    let leaf = depth == 0 || rng.below(3) == 0;
    match rng.below(if leaf { 7 } else { 10 }) {
        0 => int(i128::from(rng.next() as i64) >> rng.below(64)),
        1 => Value::Bytes((0..rng.below(4)).map(|_| rng.next() as u8).collect()),
        2 => Value::Float(f64::from(rng.below(1000) as u32) / 8.0),
        3 => text(["", "a", "id", "x", "sig", "é"][rng.below(6) as usize]),
        4 => Value::Bool(rng.below(2) == 0),
        5 => Value::Null,
        6 => Value::Simple([0, 19, 23, 32, 255][rng.below(5) as usize]),
        7 => Value::Tag(rng.below(300), Box::new(arbitrary(rng, depth - 1))),
        8 => array(
            (0..rng.below(4))
                .map(|_| arbitrary(rng, depth - 1))
                .collect(),
        ),
        _ => map((0..rng.below(4))
            .map(|_| (arbitrary(rng, depth - 1), arbitrary(rng, depth - 1)))
            .collect()),
    }
}

/// The streaming canonical encoder agrees with the bottom-up one, reads back
/// deterministically, and equals encoding the canonicalized value; decoding
/// what `encode` wrote gives the value back.
#[test]
fn streaming_and_bottom_up_canonical_encoders_agree() {
    let mut rng = SplitMix(0xC0B0_2026_0000_0001);
    for _ in 0..2000 {
        let value = arbitrary(&mut rng, 4);
        let bytes = canonical(&value);
        assert_eq!(bytes, bottom_up(&value), "{value:?}");
        let mut sorted = value.clone();
        sorted.canonicalize();
        assert_eq!(encode(&sorted), bytes);
        assert_eq!(decode(&encode(&value), Limits::DEFAULT).unwrap(), value);
        assert_eq!(encoded_len(&value), bytes.len());
        // Repeated keys make a map non-deterministic, and are the only reason
        // canonical bytes are refused.
        if decode_deterministic(&bytes, Limits::DEFAULT).is_err() {
            assert!(format!("{value:?}").contains('{'));
        }
    }
}

#[test]
fn the_excluding_map_encoder_equals_canonical_of_the_reduced_map() {
    let entries = vec![
        (text("sig"), Value::Bytes(vec![9; 4])),
        (
            text("d"),
            map(vec![(text("z"), int(1)), (int(-1), text("n"))]),
        ),
        (text("id"), Value::Bytes(vec![8; 4])),
        (text("t"), text("snapshot")),
        (int(7), Value::Null),
    ];
    for excluded in [&[][..], &["id"][..], &["id", "sig"][..], &["absent"][..]] {
        let reduced: Vec<_> = entries
            .iter()
            .filter(|(key, _)| !key.as_text().is_some_and(|t| excluded.contains(&t)))
            .cloned()
            .collect();
        let mut streamed = Vec::new();
        write_canonical_map(&entries, excluded, &mut streamed).unwrap();
        assert_eq!(streamed, canonical(&map(reduced)), "{excluded:?}");
    }
}

// ---- heads ------------------------------------------------------------------

#[test]
fn heads_are_written_shortest_and_read_in_any_definite_width() {
    for (argument, bytes) in [
        (0_u64, "40"),
        (23, "57"),
        (24, "5818"),
        (255, "58ff"),
        (256, "590100"),
        (65_535, "59ffff"),
        (65_536, "5a00010000"),
        (u64::from(u32::MAX), "5affffffff"),
        (u64::from(u32::MAX) + 1, "5b0000000100000000"),
    ] {
        let mut out = Vec::new();
        head::push_head(&mut out, BYTES, argument);
        assert_eq!(out, hex(bytes));
        assert_eq!(head::head_len(argument), out.len());
        let mut written = Vec::new();
        head::write_head(&mut written, BYTES, argument).unwrap();
        assert_eq!(written, out);
        assert_eq!(
            head::read_argument(&mut out.as_slice(), BYTES),
            Some(argument)
        );
    }
    let mut long: &[u8] = &hex("1b0000000000000005ff");
    assert_eq!(head::read_argument(&mut long, 0), Some(5));
    assert_eq!(long, [0xff]);
    for (bytes, major) in [("9f", ARRAY), ("1c", 0), ("5b00", BYTES), ("a1", ARRAY)] {
        let bytes = hex(bytes);
        let mut input = bytes.as_slice();
        assert_eq!(head::read_argument(&mut input, major), None, "{bytes:02x?}");
        assert_eq!(input, bytes.as_slice());
    }
    let mut input: &[u8] = &hex("4201026161ff");
    assert_eq!(head::read_bytes(&mut input), Some(&[1_u8, 2][..]));
    assert_eq!(head::read_text(&mut input), Some("a"));
    assert_eq!(input, [0xff]);
    assert_eq!(head::read_bytes(&mut &hex("4301")[..]), None);
    assert_eq!(head::read_text(&mut &hex("61ff")[..]), None);
    let mut out = Vec::new();
    head::write_text(&mut out, "ab").unwrap();
    head::write_bytes(&mut out, &[7]).unwrap();
    head::write_head(&mut out, MAP, 0).unwrap();
    head::write_head(&mut out, TEXT, 0).unwrap();
    assert_eq!(out, hex("626162410 7a060"));
}

// ---- streams and sequences ------------------------------------------------------

#[test]
fn a_reader_decode_takes_exactly_one_item_and_reports_a_clean_end() {
    let bytes = hex("a1616101 9f01ff 6162");
    let bytes: Vec<u8> = bytes;
    let mut reader = std::io::Cursor::new(bytes);
    assert_eq!(
        read_from(&mut reader, Limits::DEFAULT).unwrap(),
        Some(map(vec![(text("a"), int(1))]))
    );
    assert_eq!(reader.position(), 4);
    assert_eq!(
        read_from(&mut reader, Limits::DEFAULT).unwrap(),
        Some(array(vec![int(1)]))
    );
    assert_eq!(
        read_from(&mut reader, Limits::DEFAULT).unwrap(),
        Some(text("b"))
    );
    assert_eq!(read_from(&mut reader, Limits::DEFAULT).unwrap(), None);
    let mut torn = std::io::Cursor::new(hex("8201"));
    assert_eq!(
        read_from(&mut torn, Limits::DEFAULT).unwrap_err().kind(),
        DecodeErrorKind::Truncated
    );
    let mut huge = std::io::Cursor::new(hex("5bffffffffffffffff01"));
    assert_eq!(
        read_from(&mut huge, Limits::DEFAULT).unwrap_err().kind(),
        DecodeErrorKind::Truncated
    );
}

#[test]
fn a_sequence_keeps_its_intact_prefix_and_marks_a_torn_tail() {
    let whole = hex("0161618201 02");
    assert_eq!(
        decode_sequence(&whole, Limits::DEFAULT),
        (
            vec![
                (0, int(1)),
                (1, text("a")),
                (3, array(vec![int(1), int(2)]))
            ],
            None
        )
    );
    let torn = &whole[..whole.len() - 1];
    assert_eq!(
        decode_sequence(torn, Limits::DEFAULT),
        (vec![(0, int(1)), (1, text("a"))], Some(3))
    );
    assert_eq!(decode_sequence(&[], Limits::DEFAULT), (vec![], None));
}

// ---- no machine-stack recursion -------------------------------------------------

#[test]
fn a_hundred_thousand_deep_item_decodes_encodes_clones_compares_prints_and_drops() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            const DEPTH: usize = 100_000;
            let unbounded = Limits {
                max_depth: usize::MAX,
            };
            for opener in [&[0x81_u8][..], &[0xc1], &[0xa1, 0x00]] {
                let mut bytes = opener.repeat(DEPTH);
                bytes.push(0x00);
                let value = decode(&bytes, unbounded).unwrap();
                assert_eq!(encode(&value), bytes);
                assert_eq!(canonical(&value), bytes);
                assert_eq!(decode_deterministic(&bytes, unbounded).unwrap(), value);
                let copy = value.clone();
                assert_eq!(copy, value);
                assert!(format!("{copy:?}").len() > DEPTH);
                let mut sorted = copy.clone();
                sorted.canonicalize();
                drop((copy, sorted, value));
            }
            // A map nested in a map KEY, a hundred thousand deep, is sorted by
            // the bottom-up encoder.
            let mut bytes = [0xa1_u8].repeat(DEPTH);
            bytes.push(0x00);
            bytes.extend(std::iter::repeat_n(0x00, DEPTH));
            let value = decode(&bytes, unbounded).unwrap();
            assert_eq!(canonical(&value), bytes);
            drop(value);
        })
        .unwrap()
        .join()
        .unwrap();
}
