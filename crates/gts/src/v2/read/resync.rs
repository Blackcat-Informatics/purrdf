// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::collections::TryReserveError;

use purrdf_hash::blake3::Hasher;

use crate::FastMap;
use crate::v2::{
    fields::push,
    header::ResyncDeclaration,
    rdf::{UNIT_CONTEXT, unit_digest},
};

/// How the reader reached a resync opener.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResyncReach {
    /// The framing walk delimited the item in sequence.
    Walked,
    /// A scan accepted the item after losing framing.
    Scanned,
}

/// Result of checking the declared digest of the bytes before a resync opener.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResyncPast {
    /// The opener declares no past digest.
    Absent,
    /// The unit digest of the admitted range equals the declaration.
    Matches,
    /// The unit digest of the admitted range differs from the declaration.
    Differs,
    /// The range, algorithm or independent byte budget prevents the check.
    NotChecked,
}

/// A decoded resync opener and the reader's past check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResyncOpener {
    /// File-relative segment ordinal.
    pub segment: u64,
    /// Byte offset where the reader found the opener.
    pub offset: usize,
    /// Byte offset recorded in the opener.
    pub declared_offset: u64,
    /// Whether the opener was walked or scanned.
    pub reach: ResyncReach,
    /// Result of checking the past digest without changing the payload budget.
    pub past: ResyncPast,
}

struct PastCheck<'a> {
    opener: usize,
    start: usize,
    end: usize,
    digest: &'a [u8],
}

pub(super) struct PastChecks<'a> {
    ends: FastMap<usize, usize>,
    pending: Vec<PastCheck<'a>>,
    remaining: Option<u64>,
}

impl<'a> PastChecks<'a> {
    pub(super) fn new(limit: u64) -> Self {
        Self {
            ends: FastMap::default(),
            pending: Vec::new(),
            remaining: Some(limit),
        }
    }

    pub(super) fn admit(
        &mut self,
        opener: usize,
        at: usize,
        declaration: ResyncDeclaration<'a>,
        hash_available: bool,
    ) -> Result<ResyncPast, TryReserveError> {
        let Some(digest) = declaration.past else {
            return Ok(ResyncPast::Absent);
        };
        let Some(remaining) = self.remaining else {
            return Ok(ResyncPast::NotChecked);
        };
        let Some(start) = usize::try_from(declaration.offset)
            .ok()
            .and_then(|length| at.checked_sub(length))
            .filter(|_| hash_available)
        else {
            return Ok(ResyncPast::NotChecked);
        };
        if start == at {
            return Ok(past_outcome(digest, &unit_digest(&[])));
        }
        // The framing walk gives each checked range with the same start
        // an increasing end. Charge the new bytes in file order.
        let end = self.ends.get(&start).copied();
        self.remaining = remaining.checked_sub((at - end.unwrap_or(start)) as u64);
        if self.remaining.is_none() {
            return Ok(ResyncPast::NotChecked);
        }
        if end.is_none() {
            self.ends.try_reserve(1)?;
        }
        push(
            &mut self.pending,
            PastCheck {
                opener,
                start,
                end: at,
                digest,
            },
        )?;
        self.ends.insert(start, at);
        Ok(ResyncPast::NotChecked)
    }

    pub(super) fn finish(self, bytes: &[u8], openers: &mut [ResyncOpener]) {
        let Self { mut pending, .. } = self;
        // Past checks do not affect the fold. Group the admitted ranges so
        // one live hash suffices while sharing work for equal starts.
        pending.sort_unstable_by_key(|check| (check.start, check.end));
        for group in pending.chunk_by(|a, b| a.start == b.start) {
            if let [check] = group {
                openers[check.opener].past =
                    past_outcome(check.digest, &unit_digest(&bytes[check.start..check.end]));
                continue;
            }
            let mut hash = Hasher::new_derive_key(UNIT_CONTEXT);
            let mut end = group[0].start;
            for check in group {
                hash.update(&bytes[end..check.end]);
                end = check.end;
                openers[check.opener].past = past_outcome(check.digest, hash.finalize().as_bytes());
            }
        }
    }
}

fn past_outcome(expected: &[u8], actual: &[u8; 32]) -> ResyncPast {
    if expected == actual {
        ResyncPast::Matches
    } else {
        ResyncPast::Differs
    }
}
