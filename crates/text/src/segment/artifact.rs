// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bounded canonical CBOR artifacts over the shared native head codec.

use super::{Dictionary, MAX_ARENA_BYTES, MAX_ARTIFACT_BYTES, MAX_ENTRIES, PROFILE_ID};
use crate::configuration::ConfigurationRefusal;
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
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(ConfigurationRefusal::ArtifactSize { bytes: bytes.len() }.into_error());
        }
        let actual = *blake3::hash(bytes).as_bytes();
        if actual != expected {
            return Err(ConfigurationRefusal::ArtifactPhysical { expected, actual }.into_error());
        }
        let mut rest = bytes;
        let items = read_number(&mut rest, head::ARRAY)?;
        if items != 6 {
            return Err(ConfigurationRefusal::ArtifactArity { items }.into_error());
        }
        let profile = read_text(&mut rest)?;
        if profile != PROFILE_ID {
            return Err(ConfigurationRefusal::ArtifactProfile {
                actual: profile.to_owned(),
            }
            .into_error());
        }
        let version = read_bytes(&mut rest)?;
        if version != <[u8; 3]>::from(unicode::UNICODE_VERSION) {
            return Err(ConfigurationRefusal::ArtifactUnicode {
                actual: version.to_vec(),
            }
            .into_error());
        }
        let arena = read_text(&mut rest)?;
        if arena.len() > MAX_ARENA_BYTES {
            return Err(ConfigurationRefusal::DictionaryArena { bytes: arena.len() }.into_error());
        }
        let offsets = read_array(&mut rest, MAX_ENTRIES + 1)?;
        let costs = read_array(&mut rest, MAX_ENTRIES)?;
        let semantic = read_bytes(&mut rest)?;
        if !rest.is_empty()
            || offsets.len() != costs.len() + 1
            || offsets.first() != Some(&0)
            || offsets.last().copied().map(|n| n as usize) != Some(arena.len())
        {
            return Err(ConfigurationRefusal::ArtifactLayout {
                remaining: rest.len(),
                offsets: offsets.len(),
                costs: costs.len(),
                first: offsets.first().copied(),
                last: offsets.last().copied(),
                arena_bytes: arena.len(),
            }
            .into_error());
        }
        let mut previous = "";
        for range in offsets.windows(2) {
            let word = arena
                .get(range[0] as usize..range[1] as usize)
                .ok_or_else(|| {
                    ConfigurationRefusal::ArtifactOffset {
                        start: range[0],
                        end: range[1],
                        arena_bytes: arena.len(),
                    }
                    .into_error()
                })?;
            if word <= previous {
                return Err(ConfigurationRefusal::ArtifactKeyOrder {
                    previous: previous.to_owned(),
                    word: word.to_owned(),
                }
                .into_error());
            }
            let canonical = Self::canonical_key(word)?;
            if canonical != word {
                return Err(ConfigurationRefusal::ArtifactKeyCanonical {
                    word: word.to_owned(),
                    canonical,
                }
                .into_error());
            }
            previous = word;
        }
        let dictionary = Self::assemble(arena.to_owned(), offsets, costs);
        if semantic != dictionary.fingerprint {
            return Err(ConfigurationRefusal::ArtifactSemantic {
                expected: semantic.to_vec(),
                computed: dictionary.fingerprint,
                canonical: None,
            }
            .into_error());
        }
        if dictionary.artifact_bytes() != bytes {
            return Err(ConfigurationRefusal::ArtifactSemantic {
                expected: semantic.to_vec(),
                computed: dictionary.fingerprint,
                canonical: Some(false),
            }
            .into_error());
        }
        Ok(dictionary)
    }
}
fn read_number(input: &mut &[u8], major: u8) -> Result<u64, TextError> {
    let before = input.len();
    let value = head::read_argument(input, major).ok_or_else(|| {
        ConfigurationRefusal::ArtifactHead {
            major,
            // The shared reader advances only on success; read the refused
            // byte in this cold branch rather than on every decoded head.
            first: input.first().copied(),
            before,
            after: input.len(),
        }
        .into_error()
    })?;
    let bytes = before - input.len();
    let expected = head::head_len(value);
    if bytes != expected {
        return Err(ConfigurationRefusal::ArtifactHeadCanonical {
            major,
            value,
            bytes,
            expected,
        }
        .into_error());
    }
    Ok(value)
}
fn read_text<'a>(input: &mut &'a [u8]) -> Result<&'a str, TextError> {
    let count = read_number(input, head::TEXT)?;
    let length = usize::try_from(count)
        .map_err(|_| ConfigurationRefusal::ArtifactTextSize { length: count }.into_error())?;
    let (value, rest) = input.split_at_checked(length).ok_or_else(|| {
        ConfigurationRefusal::ArtifactTextTruncated {
            requested: length,
            remaining: input.len(),
        }
        .into_error()
    })?;
    *input = rest;
    core::str::from_utf8(value)
        .map_err(|cause| ConfigurationRefusal::ArtifactUtf8 { cause }.into_error())
}
fn read_bytes<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], TextError> {
    let count = read_number(input, head::BYTES)?;
    let length = usize::try_from(count)
        .map_err(|_| ConfigurationRefusal::ArtifactByteSize { length: count }.into_error())?;
    let (value, rest) = input.split_at_checked(length).ok_or_else(|| {
        ConfigurationRefusal::ArtifactBytesTruncated {
            requested: length,
            remaining: input.len(),
        }
        .into_error()
    })?;
    *input = rest;
    Ok(value)
}
fn read_array(input: &mut &[u8], maximum: usize) -> Result<Vec<u32>, TextError> {
    let value = read_number(input, head::ARRAY)?;
    let count = usize::try_from(value)
        .map_err(|_| ConfigurationRefusal::ArtifactArraySize { length: value }.into_error())?;
    if count > maximum || count > input.len() {
        return Err(ConfigurationRefusal::ArtifactArrayLimit {
            count,
            maximum,
            remaining: input.len(),
        }
        .into_error());
    }
    (0..count)
        .map(|_| {
            let value = read_number(input, head::UNSIGNED)?;
            u32::try_from(value)
                .map_err(|_| ConfigurationRefusal::ArtifactInteger { value }.into_error())
        })
        .collect()
}
