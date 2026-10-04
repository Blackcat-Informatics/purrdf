// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bounded canonical CBOR artifacts over the shared native head codec.

use super::{Dictionary, MAX_ARENA_BYTES, MAX_ARTIFACT_BYTES, MAX_ENTRIES, PROFILE_ID, invalid};
use crate::{TextError, unicode};
use purrdf_hash::blake3;
use purrdf_lex::cbor::head;

impl Dictionary {
    /// Canonical CBOR `[law, unicode, arena, offsets, costs, semantic_identity]`.
    /// Lookup representation and machine endianness do not affect these bytes.
    #[must_use]
    pub fn artifact_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.arena.len() + self.len() * 8 + 128);
        head::push_head(&mut bytes, head::ARRAY, 6);
        head::write_text(&mut bytes, PROFILE_ID).expect("Vec writes cannot fail");
        head::write_bytes(&mut bytes, &<[u8; 3]>::from(unicode::UNICODE_VERSION))
            .expect("Vec writes cannot fail");
        head::write_text(&mut bytes, &self.arena).expect("Vec writes cannot fail");
        for array in [&self.offsets, &self.costs] {
            head::push_head(&mut bytes, head::ARRAY, array.len() as u64);
            for &value in array {
                head::push_head(&mut bytes, head::UNSIGNED, u64::from(value));
            }
        }
        head::write_bytes(&mut bytes, &self.fingerprint).expect("Vec writes cannot fail");
        bytes
    }
    /// BLAKE3 of the complete physical artifact bytes.
    #[must_use]
    pub fn artifact_fingerprint(&self) -> [u8; 32] {
        *blake3::hash(&self.artifact_bytes()).as_bytes()
    }
    /// Load only the expected physical artifact. Validate canonical CBOR, Unicode
    /// edition, resource limits, UTF-8 boundaries, sorted unique normalized keys,
    /// costs and semantic identity. No source is resolved implicitly.
    ///
    /// # Errors
    /// Any identity mismatch, malformed/noncanonical payload, or resource limit.
    pub fn from_artifact(expected: [u8; 32], bytes: &[u8]) -> Result<Self, TextError> {
        if bytes.len() > MAX_ARTIFACT_BYTES || *blake3::hash(bytes).as_bytes() != expected {
            return Err(invalid(
                "dictionary artifact size or physical identity mismatch",
            ));
        }
        let mut rest = bytes;
        if read_number(&mut rest, head::ARRAY)? != 6
            || read_text(&mut rest)? != PROFILE_ID
            || read_bytes(&mut rest)? != <[u8; 3]>::from(unicode::UNICODE_VERSION)
        {
            return Err(invalid(
                "dictionary artifact format or Unicode edition mismatch",
            ));
        }
        let arena = read_text(&mut rest)?;
        if arena.len() > MAX_ARENA_BYTES {
            return Err(invalid("dictionary arena exceeds its byte bound"));
        }
        let offsets = read_array(&mut rest, MAX_ENTRIES + 1)?;
        let costs = read_array(&mut rest, MAX_ENTRIES)?;
        let semantic = read_bytes(&mut rest)?;
        if !rest.is_empty()
            || offsets.len() != costs.len() + 1
            || offsets.first() != Some(&0)
            || offsets.last().copied().map(|n| n as usize) != Some(arena.len())
        {
            return Err(invalid("dictionary artifact structure mismatch"));
        }
        let mut previous = "";
        for range in offsets.windows(2) {
            let word = arena
                .get(range[0] as usize..range[1] as usize)
                .ok_or_else(|| invalid("dictionary offsets are not UTF-8 boundaries"))?;
            if word <= previous || Self::canonical_key(word)? != word {
                return Err(invalid(
                    "dictionary entries are not sorted canonical unique keys",
                ));
            }
            previous = word;
        }
        let dictionary = Self::assemble(arena.to_owned(), offsets, costs);
        if semantic != dictionary.fingerprint || dictionary.artifact_bytes() != bytes {
            return Err(invalid(
                "dictionary semantic identity or canonical encoding mismatch",
            ));
        }
        Ok(dictionary)
    }
}
fn read_number(input: &mut &[u8], major: u8) -> Result<u64, TextError> {
    let before = input.len();
    let value =
        head::read_argument(input, major).ok_or_else(|| invalid("invalid dictionary CBOR head"))?;
    if before - input.len() != head::head_len(value) {
        return Err(invalid("noncanonical dictionary CBOR head"));
    }
    Ok(value)
}
fn read_text<'a>(input: &mut &'a [u8]) -> Result<&'a str, TextError> {
    let length = usize::try_from(read_number(input, head::TEXT)?)
        .map_err(|_| invalid("dictionary text size overflow"))?;
    let (value, rest) = input
        .split_at_checked(length)
        .ok_or_else(|| invalid("truncated dictionary text"))?;
    *input = rest;
    core::str::from_utf8(value).map_err(|_| invalid("dictionary text is not UTF-8"))
}
fn read_bytes<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], TextError> {
    let length = usize::try_from(read_number(input, head::BYTES)?)
        .map_err(|_| invalid("dictionary byte size overflow"))?;
    let (value, rest) = input
        .split_at_checked(length)
        .ok_or_else(|| invalid("truncated dictionary bytes"))?;
    *input = rest;
    Ok(value)
}
fn read_array(input: &mut &[u8], maximum: usize) -> Result<Vec<u32>, TextError> {
    let count = usize::try_from(read_number(input, head::ARRAY)?)
        .map_err(|_| invalid("dictionary array size overflow"))?;
    if count > maximum || count > input.len() {
        return Err(invalid("dictionary array exceeds its bound"));
    }
    (0..count)
        .map(|_| {
            u32::try_from(read_number(input, head::UNSIGNED)?)
                .map_err(|_| invalid("dictionary integer exceeds u32"))
        })
        .collect()
}
