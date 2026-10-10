// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Safe storage specialized to the little-endian binary integer magnitude.
//!
//! Every inline slot is initialized. The biased length leaves zero available as
//! an enum niche without a union, an uninitialized slot, or unsafe code.

use core::{
    fmt,
    hash::{Hash, Hasher},
    num::NonZeroU8,
    ops::{Deref, DerefMut},
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) enum Mag {
    Inline { words: [u64; 3], length: NonZeroU8 },
    Heap(Vec<u64>),
    Shared(Arc<Vec<u64>>),
    Pooled(super::scratch::Pooled),
}

impl Mag {
    pub(super) const fn new() -> Self {
        Self::Inline {
            words: [0; 3],
            length: NonZeroU8::new(1).expect("one is nonzero"),
        }
    }

    pub(super) fn with_capacity(capacity: usize) -> Self {
        if capacity <= 4 {
            Self::new()
        } else {
            Self::Heap(Vec::with_capacity(capacity))
        }
    }

    pub(super) fn from_slice(words: &[u64]) -> Self {
        if words.len() > 3 {
            return Self::Heap(words.to_vec());
        }
        let mut inline = [0; 3];
        inline[..words.len()].copy_from_slice(words);
        Self::Inline {
            words: inline,
            length: NonZeroU8::new(words.len() as u8 + 1).expect("biased length is nonzero"),
        }
    }

    pub(super) fn zeroed(length: usize) -> Self {
        if length <= 3 {
            Self::Inline {
                words: [0; 3],
                length: NonZeroU8::new(length as u8 + 1).expect("biased length is nonzero"),
            }
        } else {
            Self::Heap(vec![0; length])
        }
    }

    /// A fully sized native arithmetic destination, refusing physical allocation.
    pub(crate) fn try_destination(
        capacity: usize,
        length: usize,
    ) -> Result<Self, super::LimbScratchError> {
        if length > capacity {
            return Err(super::LimbScratchError::SizeOverflow);
        }
        if capacity <= 3 {
            return Ok(Self::zeroed(length));
        }
        let mut words = Vec::new();
        words
            .try_reserve_exact(capacity)
            .map_err(|_| super::LimbScratchError::SizeOverflow)?;
        words.resize(length, 0);
        Ok(Self::Heap(words))
    }

    pub(super) fn push(&mut self, word: u64) {
        match self {
            Self::Inline { words, length } if length.get() <= 3 => {
                words[usize::from(length.get() - 1)] = word;
                *length = NonZeroU8::new(length.get() + 1).expect("biased length is nonzero");
            }
            Self::Inline { words, .. } => {
                let mut heap = Vec::with_capacity(4);
                heap.extend_from_slice(words);
                heap.push(word);
                *self = Self::Heap(heap);
            }
            Self::Heap(words) => words.push(word),
            Self::Shared(words) => Arc::get_mut(words)
                .expect("a shared canonical magnitude is never an arithmetic destination")
                .push(word),
            Self::Pooled(words) => {
                let unique = words.words_mut();
                assert!(
                    unique.len() < unique.capacity(),
                    "admitted integer destination capacity"
                );
                unique.push(word);
            }
        }
    }

    pub(super) fn pop(&mut self) -> Option<u64> {
        let result = match self {
            Self::Inline { words, length } => {
                if length.get() == 1 {
                    return None;
                }
                *length = NonZeroU8::new(length.get() - 1).expect("biased length stays nonzero");
                let slot = &mut words[usize::from(length.get() - 1)];
                Some(core::mem::replace(slot, 0))
            }
            Self::Heap(words) => words.pop(),
            Self::Shared(words) => Arc::get_mut(words)
                .expect("a shared canonical magnitude is never an arithmetic destination")
                .pop(),
            Self::Pooled(words) => words.words_mut().pop(),
        };
        if !matches!(self, Self::Inline { .. }) && self.len() <= 3 {
            *self = Self::from_slice(self);
        }
        result
    }

    pub(super) fn resize(&mut self, length: usize, value: u64) {
        while self.len() > length {
            self.pop();
        }
        while self.len() < length {
            self.push(value);
        }
    }

    pub(super) fn allocated_bytes(&self) -> usize {
        match self {
            Self::Inline { .. } => 0,
            Self::Heap(words) => words.capacity().saturating_mul(size_of::<u64>()),
            Self::Shared(words) => {
                words.capacity().saturating_mul(size_of::<u64>())
                    + size_of::<Vec<u64>>()
                    + 2 * size_of::<usize>()
            }
            Self::Pooled(words) => words.allocated_bytes(),
        }
    }
}

impl Deref for Mag {
    type Target = [u64];
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Inline { words, length } => &words[..usize::from(length.get() - 1)],
            Self::Heap(words) => words,
            Self::Shared(words) => words,
            Self::Pooled(words) => words.words(),
        }
    }
}

impl DerefMut for Mag {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Inline { words, length } => &mut words[..usize::from(length.get() - 1)],
            Self::Heap(words) => words,
            Self::Shared(words) => Arc::get_mut(words)
                .expect("a shared canonical magnitude is never an arithmetic destination"),
            Self::Pooled(words) => words.words_mut(),
        }
    }
}

impl<'a> IntoIterator for &'a mut Mag {
    type Item = &'a mut u64;
    type IntoIter = core::slice::IterMut<'a, u64>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl PartialEq for Mag {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl Eq for Mag {}
impl Hash for Mag {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}
impl fmt::Debug for Mag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

purrdf_hash::default_from_new!(Mag);
