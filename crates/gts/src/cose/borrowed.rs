// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrow the definite COSE envelope produced by the writer. Other CBOR forms
//! go through the full decoder, including every field this view cannot inspect.

use purrdf_lex::cbor::head::{ARRAY, MAP, TAG, UNSIGNED, read_argument, read_bytes};

use super::{Encrypt0Parts, TAG_ENCRYPT0};

pub(super) fn parse(mut input: &[u8]) -> Option<Encrypt0Parts<'_>> {
    if input.first()? >> 5 == TAG && read_argument(&mut input, TAG)? != TAG_ENCRYPT0 {
        return None;
    }
    if read_argument(&mut input, ARRAY)? != 3 {
        return None;
    }
    let protected = read_bytes(&mut input)?;
    let count = read_argument(&mut input, MAP)?;
    let mut kid = None;
    let mut iv = None;
    // Every entry must have a direct unsigned label and definite bytes. If an
    // extension uses another shape, the full decoder validates that field and
    // the rest of the envelope; unknown values are never blindly skipped.
    for _ in 0..count {
        let label = read_argument(&mut input, UNSIGNED)?;
        let value = read_bytes(&mut input)?;
        match label {
            4 if kid.is_none() => kid = std::str::from_utf8(value).ok(),
            5 if iv.is_none() => iv = Some(value),
            _ => {}
        }
    }
    let ciphertext = read_bytes(&mut input)?;
    // Like `cbor::decode_prefix`, this parses one item and permits trailing
    // bytes. Protected bytes remain opaque: authentication binds their spelling.
    Some(Encrypt0Parts {
        kid: kid?.into(),
        protected: protected.into(),
        iv: iv?.into(),
        ciphertext: ciphertext.into(),
    })
}
