// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Offline reproducible lookup candidates. These own setup data for generator
//! and benchmark comparison; shipping conversion uses immutable generated data.

use purrdf_hash::{fnv::fnv1a64, mix::splitmix64_finalize};

/// Identical word bytes with a two-letter bucket index and a displaced static
/// perfect-hash index. Setup is excluded from conversion measurements.
#[derive(Debug)]
pub struct Lookup {
    blob: String,
    offsets: Vec<u16>,
    buckets: [u16; 677],
    displacements: [i32; 1024],
    slots: [u16; 4096],
}

impl Lookup {
    /// Construct both candidates from exactly 4,096 sorted ASCII words.
    pub fn new(words: &[&str]) -> Result<Self, &'static str> {
        if words.len() != 4096
            || words.windows(2).any(|pair| pair[0] >= pair[1])
            || words
                .iter()
                .any(|word| word.len() < 3 || !word.bytes().all(|byte| byte.is_ascii_lowercase()))
        {
            return Err("lookup candidates require 4,096 ordered ASCII words");
        }
        let mut blob = String::new();
        let mut offsets = Vec::new();
        for word in words {
            offsets
                .push(u16::try_from(blob.len()).map_err(|_| "word blob exceeds metadata width")?);
            blob.push_str(word);
        }
        offsets.push(u16::try_from(blob.len()).map_err(|_| "word blob exceeds metadata width")?);
        let buckets = std::array::from_fn(|key| {
            words.partition_point(|word| {
                let bytes = word.as_bytes();
                usize::from(bytes[0] - b'a') * 26 + usize::from(bytes[1] - b'a') < key
            }) as u16
        });
        let hashes: Vec<_> = words.iter().map(|word| fnv1a64(word.as_bytes())).collect();
        let mut groups = vec![Vec::new(); 1024];
        for (index, hash) in hashes.iter().enumerate() {
            groups[*hash as usize & 1023].push(index);
        }
        let mut order: Vec<_> = (0..1024).collect();
        order.sort_unstable_by_key(|index| (core::cmp::Reverse(groups[*index].len()), *index));
        let mut slots = [u16::MAX; 4096];
        let mut displacements = [0_i32; 1024];
        for group in order {
            if groups[group].len() <= 1 {
                continue;
            }
            let mut chosen = Vec::new();
            let seed = (1..=i32::MAX)
                .find(|seed| {
                    chosen.clear();
                    for index in &groups[group] {
                        let slot =
                            splitmix64_finalize(hashes[*index] ^ *seed as u64) as usize & 4095;
                        if slots[slot] != u16::MAX || chosen.contains(&slot) {
                            return false;
                        }
                        chosen.push(slot);
                    }
                    true
                })
                .ok_or("perfect-hash displacement search exhausted")?;
            displacements[group] = seed;
            for (index, slot) in groups[group].iter().zip(chosen) {
                slots[slot] = *index as u16;
            }
        }
        let mut free = 0;
        for (group, members) in groups.iter().enumerate() {
            if members.len() != 1 {
                continue;
            }
            while slots[free] != u16::MAX {
                free += 1;
            }
            slots[free] = members[0] as u16;
            displacements[group] = -(free as i32) - 1;
        }
        let result = Self {
            blob,
            offsets,
            buckets,
            displacements,
            slots,
        };
        for (index, word) in words.iter().enumerate() {
            if result.perfect(word.as_bytes()) != Some(index as u16)
                || result.bucket(word.as_bytes()) != Some(index as u16)
            {
                return Err("generated lookup does not reproduce its alphabet");
            }
        }
        Ok(result)
    }

    fn word(&self, index: u16) -> &[u8] {
        let index = usize::from(index);
        &self.blob.as_bytes()
            [usize::from(self.offsets[index])..usize::from(self.offsets[index + 1])]
    }

    /// Probe the displaced hash and validate the complete candidate bytes.
    pub fn perfect(&self, word: &[u8]) -> Option<u16> {
        let hash = fnv1a64(word);
        let displacement = self.displacements[hash as usize & 1023];
        if displacement == 0 {
            return None;
        }
        let slot = if displacement < 0 {
            (-displacement - 1) as usize
        } else {
            splitmix64_finalize(hash ^ displacement as u64) as usize & 4095
        };
        let index = self.slots[slot];
        (self.word(index) == word).then_some(index)
    }

    /// Probe the compact first-two-letter bucket with lexical binary search.
    pub fn bucket(&self, word: &[u8]) -> Option<u16> {
        if word.len() < 3 || !word[..2].iter().all(u8::is_ascii_lowercase) {
            return None;
        }
        let key = usize::from(word[0] - b'a') * 26 + usize::from(word[1] - b'a');
        let mut range = usize::from(self.buckets[key])..usize::from(self.buckets[key + 1]);
        while range.start < range.end {
            let middle = range.start + range.len() / 2;
            match self.word(middle as u16).cmp(word) {
                core::cmp::Ordering::Less => range.start = middle + 1,
                core::cmp::Ordering::Greater => range.end = middle,
                core::cmp::Ordering::Equal => return Some(middle as u16),
            }
        }
        None
    }

    /// Borrow generated displacement and slot arrays for immutable source output.
    pub const fn perfect_tables(&self) -> (&[i32; 1024], &[u16; 4096]) {
        (&self.displacements, &self.slots)
    }
}
