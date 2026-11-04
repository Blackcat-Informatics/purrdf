// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Public bounded conversion independent of a document schema or host grammar.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_lex::integer_text::{RadixError, parse_u128};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[test]
fn unsigned_boundaries_and_leading_zeros_preserve_the_numeric_value() {
    for (radix, maximum, overflow) in [
        (2, "1".repeat(128), format!("1{}", "0".repeat(128))),
        (
            8,
            format!("3{}", "7".repeat(42)),
            format!("4{}", "0".repeat(42)),
        ),
        (
            10,
            u128::MAX.to_string(),
            "340282366920938463463374607431768211456".to_owned(),
        ),
        (16, "f".repeat(32), format!("1{}", "0".repeat(32))),
    ] {
        assert_eq!(parse_u128(b"0", radix), Ok(0));
        assert_eq!(parse_u128(b"0000", radix), Ok(0));
        assert_eq!(parse_u128(maximum.as_bytes(), radix), Ok(u128::MAX));
        assert_eq!(
            parse_u128(overflow.as_bytes(), radix),
            Err(RadixError::Overflow)
        );
        let padded = format!("{}{maximum}", "0".repeat(4096));
        assert_eq!(parse_u128(padded.as_bytes(), radix), Ok(u128::MAX));
    }
    assert_eq!(parse_u128(b"11111111", 2), Ok(255));
    assert_eq!(parse_u128(b"377", 8), Ok(255));
    assert_eq!(parse_u128(b"aBcDeF", 16), Ok(0xAB_CDEF));
    for radix in 2..=16 {
        assert_eq!(parse_u128(b"10", radix), Ok(u128::from(radix)));
    }
}

#[test]
fn invalid_digits_take_precedence_over_overflow_and_host_syntax_is_refused() {
    for radix in [0, 1, 17, u8::MAX] {
        assert_eq!(
            parse_u128(b"", radix),
            Err(RadixError::InvalidRadix { radix })
        );
    }
    assert_eq!(parse_u128(b"", 2), Err(RadixError::Empty));
    for (digits, radix, offset, byte) in [
        (&b"102"[..], 2, 2, b'2'),
        (&b"78"[..], 8, 1, b'8'),
        (&b"fg"[..], 16, 1, b'g'),
        (&b"0b1"[..], 2, 1, b'b'),
        (&b"1_0"[..], 8, 1, b'_'),
        (&b"1u8"[..], 8, 1, b'u'),
        (&b"+1"[..], 8, 0, b'+'),
        (&b"-1"[..], 8, 0, b'-'),
        (&b" 1"[..], 8, 0, b' '),
        (&b"1\xff"[..], 8, 1, 0xff),
    ] {
        assert_eq!(
            parse_u128(digits, radix),
            Err(RadixError::InvalidDigit { offset, byte })
        );
    }
    for (radix, valid, invalid) in [(2, b'1', b'2'), (8, b'7', b'8')] {
        assert_eq!(parse_u128(&[valid], radix), Ok(u128::from(valid - b'0')));
        let mut digits = vec![valid; 4096];
        assert_eq!(parse_u128(&digits, radix), Err(RadixError::Overflow));
        digits.extend_from_slice(&[invalid, b'?']);
        assert_eq!(
            parse_u128(&digits, radix),
            Err(RadixError::InvalidDigit {
                offset: 4096,
                byte: invalid
            })
        );
    }
}

#[test]
fn successful_and_refused_unsigned_reads_allocate_nothing() {
    let overflowing = [b'7'; 4096];
    let mut invalid = overflowing;
    invalid[4095] = b'8';
    let window = CurrentThreadWindow::open();
    let results = [
        parse_u128(std::hint::black_box(b"377"), 8),
        parse_u128(std::hint::black_box(&overflowing), 8),
        parse_u128(std::hint::black_box(&invalid), 8),
        parse_u128(std::hint::black_box(b""), 8),
        parse_u128(std::hint::black_box(b"0"), 1),
    ];
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.requested_bytes, 0);
    assert_eq!(
        results,
        [
            Ok(255),
            Err(RadixError::Overflow),
            Err(RadixError::InvalidDigit {
                offset: 4095,
                byte: b'8'
            }),
            Err(RadixError::Empty),
            Err(RadixError::InvalidRadix { radix: 1 })
        ]
    );
}
