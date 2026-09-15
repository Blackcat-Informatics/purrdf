// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Borrow the definite COSE envelope produced by the writer. Other CBOR forms
//! go through the full decoder, including every field this view cannot inspect.

use super::{Encrypt0Parts, TAG_ENCRYPT0};

/// Read one definite CBOR argument without accepting another major type.
fn argument(input: &mut &[u8], major: u8) -> Option<u64> {
    let (&head, tail) = input.split_first()?;
    if head >> 5 != major {
        return None;
    }
    *input = tail;
    let extra = head & 31;
    let width = match extra {
        0..=23 => return Some(u64::from(extra)),
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        _ => return None,
    };
    let (bytes, tail) = input.split_at_checked(width)?;
    *input = tail;
    Some(
        bytes
            .iter()
            .fold(0, |value, byte| (value << 8) | u64::from(*byte)),
    )
}

/// Validate the entire byte string before borrowing it or allocating output.
fn bytes<'a>(input: &mut &'a [u8]) -> Option<&'a [u8]> {
    let length = usize::try_from(argument(input, 2)?).ok()?;
    let (bytes, tail) = input.split_at_checked(length)?;
    *input = tail;
    Some(bytes)
}

pub(super) fn parse(mut input: &[u8]) -> Option<Encrypt0Parts<'_>> {
    if input.first()? >> 5 == 6 && argument(&mut input, 6)? != TAG_ENCRYPT0 {
        return None;
    }
    if argument(&mut input, 4)? != 3 {
        return None;
    }
    let protected = bytes(&mut input)?;
    let count = argument(&mut input, 5)?;
    let mut kid = None;
    let mut iv = None;
    // Every entry must have a direct unsigned label and definite bytes. If an
    // extension uses another shape, the full decoder validates that field and
    // the rest of the envelope; unknown values are never blindly skipped.
    for _ in 0..count {
        let label = argument(&mut input, 0)?;
        let value = bytes(&mut input)?;
        match label {
            4 if kid.is_none() => kid = std::str::from_utf8(value).ok(),
            5 if iv.is_none() => iv = Some(value),
            _ => {}
        }
    }
    let ciphertext = bytes(&mut input)?;
    // Like ciborium::from_reader, this parses one item and permits trailing
    // bytes. Protected bytes remain opaque: authentication binds their spelling.
    Some(Encrypt0Parts {
        kid: kid?.into(),
        protected: protected.into(),
        iv: iv?.into(),
        ciphertext: ciphertext.into(),
    })
}
