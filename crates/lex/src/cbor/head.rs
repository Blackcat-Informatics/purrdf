// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The CBOR data-item head (RFC 8949 §3): a major type and its argument.
//!
//! The initial byte carries the major type in its top three bits and, in its
//! low five, either the argument itself (below 24) or how many bytes of
//! big-endian argument follow (24, 25, 26, 27 for one, two, four, eight). The
//! writers here always pick the shortest form (§4.2.1); the borrowed readers
//! accept every definite form, shortest or not, since a well-formed item may
//! spell its argument long (§3).

use std::io::{self, Write};

/// Major type 0: an unsigned integer.
pub const UNSIGNED: u8 = 0;
/// Major type 1: a negative integer, `-1 - argument`.
pub const NEGATIVE: u8 = 1;
/// Major type 2: a byte string of `argument` bytes.
pub const BYTES: u8 = 2;
/// Major type 3: a UTF-8 text string of `argument` bytes.
pub const TEXT: u8 = 3;
/// Major type 4: an array of `argument` items.
pub const ARRAY: u8 = 4;
/// Major type 5: a map of `argument` key/value pairs.
pub const MAP: u8 = 5;
/// Major type 6: tag number `argument`, then the tagged item.
pub const TAG: u8 = 6;
/// Major type 7: simple values and floats.
pub const SIMPLE: u8 = 7;

/// The head of `major` with `argument`, shortest form, and its length.
pub(crate) const fn encode(major: u8, argument: u64) -> ([u8; 9], usize) {
    let prefix = major << 5;
    let mut bytes = [0_u8; 9];
    let be = argument.to_be_bytes();
    if argument < 24 {
        bytes[0] = prefix | argument as u8;
        (bytes, 1)
    } else if argument <= 0xff {
        bytes[0] = prefix | 0x18;
        bytes[1] = argument as u8;
        (bytes, 2)
    } else if argument <= 0xffff {
        bytes[0] = prefix | 0x19;
        bytes[1] = be[6];
        bytes[2] = be[7];
        (bytes, 3)
    } else if argument <= 0xffff_ffff {
        bytes[0] = prefix | 0x1a;
        bytes[1] = be[4];
        bytes[2] = be[5];
        bytes[3] = be[6];
        bytes[4] = be[7];
        (bytes, 5)
    } else {
        bytes[0] = prefix | 0x1b;
        let mut k = 0;
        while k < 8 {
            bytes[1 + k] = be[k];
            k += 1;
        }
        (bytes, 9)
    }
}

/// How many bytes the shortest head of `argument` takes: 1, 2, 3, 5 or 9.
pub const fn head_len(argument: u64) -> usize {
    encode(0, argument).1
}

/// Append the shortest head of `major` with `argument` to `out`.
///
/// ```rust
/// use purrdf_lex::cbor::head::{MAP, TEXT, push_head};
///
/// let mut out = Vec::new();
/// push_head(&mut out, MAP, 2);
/// push_head(&mut out, TEXT, 24);
/// push_head(&mut out, TEXT, 256);
/// assert_eq!(out, [0xa2, 0x78, 24, 0x79, 0x01, 0x00]);
/// ```
pub fn push_head(out: &mut Vec<u8>, major: u8, argument: u64) {
    let (bytes, len) = encode(major, argument);
    out.extend_from_slice(&bytes[..len]);
}

/// Write the shortest head of `major` with `argument` to `writer`.
///
/// # Errors
///
/// The writer's error.
pub fn write_head<W: Write + ?Sized>(writer: &mut W, major: u8, argument: u64) -> io::Result<()> {
    let (bytes, len) = encode(major, argument);
    writer.write_all(&bytes[..len])
}

/// Write a definite text string: its head, then its UTF-8 bytes.
///
/// # Errors
///
/// The writer's error.
pub fn write_text<W: Write + ?Sized>(writer: &mut W, text: &str) -> io::Result<()> {
    write_head(writer, TEXT, text.len() as u64)?;
    writer.write_all(text.as_bytes())
}

/// Write a definite byte string: its head, then its bytes.
///
/// # Errors
///
/// The writer's error.
pub fn write_bytes<W: Write + ?Sized>(writer: &mut W, bytes: &[u8]) -> io::Result<()> {
    write_head(writer, BYTES, bytes.len() as u64)?;
    writer.write_all(bytes)
}

/// Read one definite head of major type `major` from the front of `input`,
/// advancing past it: its argument.
///
/// `None`, with `input` unchanged, when the next item has another major type, is indefinite (additional information 31), uses a
/// reserved additional information (28–30), or is truncated. A long argument
/// (`0x18 0x05` for 5) is accepted: it is well-formed (RFC 8949 §3), merely
/// not deterministic.
///
/// ```rust
/// use purrdf_lex::cbor::head::{ARRAY, read_argument};
///
/// let mut input: &[u8] = &[0x83, 0x98, 0x03, 0x01];
/// assert_eq!(read_argument(&mut input, ARRAY), Some(3));
/// assert_eq!(read_argument(&mut input, ARRAY), Some(3));
/// assert_eq!(input, [0x01]);
/// assert_eq!(read_argument(&mut &[0x9f][..], ARRAY), None);
/// ```
pub fn read_argument(input: &mut &[u8], major: u8) -> Option<u64> {
    let (&initial, tail) = input.split_first()?;
    if initial >> 5 != major {
        return None;
    }
    let width = match initial & 31 {
        info @ 0..=23 => {
            *input = tail;
            return Some(u64::from(info));
        }
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        _ => return None,
    };
    let (bytes, rest) = tail.split_at_checked(width)?;
    *input = rest;
    Some(
        bytes
            .iter()
            .fold(0, |value, &byte| (value << 8) | u64::from(byte)),
    )
}

/// Read one definite byte string from the front of `input`, advancing past
/// it, and borrow its bytes. The whole string is bounds-checked before it is
/// borrowed.
pub fn read_bytes<'a>(input: &mut &'a [u8]) -> Option<&'a [u8]> {
    let mut rest = *input;
    let length = usize::try_from(read_argument(&mut rest, BYTES)?).ok()?;
    let (bytes, tail) = rest.split_at_checked(length)?;
    *input = tail;
    Some(bytes)
}

/// Read one definite text string from the front of `input`, advancing past
/// it, and borrow its text; `None` when its bytes are not UTF-8.
pub fn read_text<'a>(input: &mut &'a [u8]) -> Option<&'a str> {
    let mut rest = *input;
    let length = usize::try_from(read_argument(&mut rest, TEXT)?).ok()?;
    let (bytes, tail) = rest.split_at_checked(length)?;
    let text = core::str::from_utf8(bytes).ok()?;
    *input = tail;
    Some(text)
}
