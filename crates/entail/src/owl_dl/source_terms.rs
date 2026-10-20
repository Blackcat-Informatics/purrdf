// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source membership across interleaved assertion and query preparation.

use purrdf_core::ir::pack::bits::BitVec;
use purrdf_lex::allocation::{Memory, Resident, StorageError};

/// The immutable cold prefix costs no bitmap. Later source terms occupy bits
/// relative to that prefix; intervening query symbols remain clear.
#[derive(Debug, Default)]
pub(crate) struct SourceTerms {
    prefix: u32,
    later: BitVec,
    live: usize,
}

impl SourceTerms {
    pub(crate) fn from_prefix(prefix: u32) -> Self {
        Self {
            prefix,
            ..Self::default()
        }
    }

    pub(crate) fn contains(&self, id: u32) -> bool {
        id < self.prefix || self.later.get((id - self.prefix) as usize) == Some(true)
    }

    pub(crate) fn record(&mut self, id: u32) -> Result<(), StorageError> {
        if id < self.prefix {
            return Ok(());
        }
        if id == self.prefix && self.later.is_empty() {
            self.prefix = self
                .prefix
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?;
            return Ok(());
        }
        let mut storage = Resident;
        let mut memory = Memory::resume(&mut storage, self.live);
        let result = self
            .later
            .set_with_memory((id - self.prefix) as usize, true, &mut memory);
        self.live = memory.admitted_bytes();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_membership_keeps_query_holes_and_promotes_original_ids() {
        let mut terms = SourceTerms::from_prefix(10);
        assert_eq!(terms.live, 0);
        for id in 0..10 {
            assert!(terms.contains(id));
        }
        terms.record(12).unwrap();
        terms.record(80).unwrap();
        assert!(!terms.contains(10));
        assert!(!terms.contains(11));
        assert!(terms.contains(12));
        assert!(!terms.contains(79));
        assert!(terms.contains(80));
        assert!(!terms.contains(81));
        terms.record(10).unwrap();
        assert!(terms.contains(10));
        assert!(!terms.contains(11));
        assert!(terms.contains(80));
    }

    #[test]
    fn cold_source_ingress_keeps_the_prefix_without_allocating_a_bitmap() {
        let mut terms = SourceTerms::from_prefix(10);
        for id in 0..1000 {
            terms.record(id).unwrap();
        }
        assert_eq!(terms.prefix, 1000);
        assert!(terms.later.is_empty());
        assert_eq!(terms.live, 0);
        assert!(terms.contains(999));
        assert!(!terms.contains(1000));
    }
}
