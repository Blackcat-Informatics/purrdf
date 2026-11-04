// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Offline rank-BPE for canonical ASCII references only. No Unicode tokenizer.

use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

use purrdf_hash::{blake3, fixed::FixedState, hex};

type Failure = Box<dyn Error>;

pub(super) struct Bpe {
    ranks: HashMap<Vec<u8>, u32, FixedState>,
}

impl Bpe {
    pub(super) fn read(path: &Path, expected: &str) -> Result<Self, Failure> {
        let bytes = std::fs::read(path)?;
        if hex::encode(blake3::hash(&bytes).as_bytes()) != expected {
            return Err(format!("pinned vocabulary digest mismatch: {}", path.display()).into());
        }
        let mut ranks = HashMap::with_hasher(FixedState::new());
        for line in std::str::from_utf8(&bytes)?.lines() {
            let (token, rank) = line.split_once(' ').ok_or("invalid rank record")?;
            if ranks
                .insert(purrdf_xsd::parse_base64(token)?, rank.parse()?)
                .is_some()
            {
                return Err("duplicate pinned rank token".into());
            }
        }
        Ok(Self { ranks })
    }

    pub(super) fn piece(&self, bytes: &[u8]) -> Result<Vec<u32>, Failure> {
        if let Some(rank) = self.ranks.get(bytes) {
            return Ok(vec![*rank]);
        }
        let mut boundaries: Vec<usize> = (0..=bytes.len()).collect();
        loop {
            // Upstream merges the lowest rank, with the earliest boundary
            // winning ties, then repeats. Dictionary pieces are at most ten
            // bytes; independently published source probes may be longer.
            let next = boundaries
                .windows(3)
                .enumerate()
                .filter_map(|(index, span)| {
                    self.ranks
                        .get(&bytes[span[0]..span[2]])
                        .map(|rank| (*rank, index))
                })
                .min();
            let Some((_, index)) = next else { break };
            boundaries.remove(index + 1);
        }
        boundaries
            .windows(2)
            .map(|span| {
                self.ranks
                    .get(&bytes[span[0]..span[1]])
                    .copied()
                    .ok_or_else(|| "missing byte token".into())
            })
            .collect()
    }

    pub(super) fn reference(&self, text: &str) -> Result<Vec<u32>, Failure> {
        if !(text.starts_with("b3w1: ") || text.starts_with("b3p1: ")) {
            return Err("analyzer requires a canonical word-reference header".into());
        }
        let mut ids = Vec::new();
        // These are the exact matches of both pinned pretokenizer regexes on
        // this language. Spaces join the following lowercase-letter piece.
        for byte in &text.as_bytes()[..5] {
            ids.extend(self.piece(&[*byte])?);
        }
        for word in text[6..].split(' ') {
            if word.is_empty() || !word.bytes().all(|byte| byte.is_ascii_lowercase()) {
                return Err("analyzer only accepts single-space lowercase ASCII words".into());
            }
            ids.extend(self.piece(format!(" {word}").as_bytes())?);
        }
        Ok(ids)
    }

    pub(super) fn source_vectors(&self, name: &str) -> Result<(), Failure> {
        // Independent expected ids: OpenAI tiktoken 0.12.0's public test fixes
        // cl100k hello/world; the o200k ids and header bytes are direct complete
        // token records in its immutable vocabulary, without any BPE merging.
        let hello = if name == "cl100k" { 15339 } else { 24912 };
        let world = if name == "cl100k" { 1917 } else { 2375 };
        if self.piece(b"hello")? != [hello] || self.piece(b" world")? != [world] {
            return Err("independent hello/world token vector mismatch".into());
        }
        // Independently published OpenAI Cookbook integer vectors for a single
        // lowercase-letter regex piece. These exercise actual ranked merges,
        // rather than a whole-token dictionary hit.
        let expected: &[u32] = if name == "cl100k" {
            &[519, 85342, 34500, 479, 8997, 2191]
        } else {
            &[493, 129_901, 376, 160_388, 21203, 2367]
        };
        if self.piece(b"antidisestablishmentarianism")? != expected {
            return Err("independent published multi-piece BPE vector mismatch".into());
        }
        for tag in ["b3w1: hello world", "b3p1: hello world"] {
            let ids = self.reference(tag)?;
            let mut direct = Vec::new();
            for byte in &tag.as_bytes()[..5] {
                direct.push(*self.ranks.get(&[*byte][..]).ok_or("header byte absent")?);
            }
            for piece in [b" hello".as_slice(), b" world"] {
                direct.push(
                    *self
                        .ranks
                        .get(piece)
                        .ok_or("independent whole token absent")?,
                );
            }
            if ids != direct {
                return Err("canonical header source vector mismatch".into());
            }
        }
        Ok(())
    }
}
