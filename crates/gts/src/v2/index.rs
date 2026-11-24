// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;
use std::collections::TryReserveError;

use super::fields::{Fields, OptionShape, push};
use super::terms::read_enum;
use super::{
    ContentError, DIGEST_LENGTH, FrameCheck, FrameEnvelope, FrameField, FrameType, LinkState,
    MalformedReason, PayloadState, REGISTRY_VERSION, Range, WireError,
};

type Digest = [u8; DIGEST_LENGTH];
const LAST_FRAME_TYPE: u64 = FrameType::Parity as u64;

mod content;

pub(super) use content::IndexContent;
use content::{check_graph_directory, check_pack_members};

/// Comparisons of an index's claims with the observed ordinary prefix.
///
/// Each result is independent. Statistics and the content digest require
/// labelling the segment's fold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct IndexPrefixCheck {
    /// The frame count and head agree with the observed prefix.
    pub head_matches: bool,
    /// The range root agrees with the observed prefix.
    pub root_matches: bool,
    /// The declared runs match the maximal runs of frame types.
    pub type_runs_match: bool,
    /// The graph directory matches the written rows of the covered frames.
    /// Absent when the covered row payloads were unavailable.
    pub graph_directory_matches: Option<bool>,
    /// The pack-member locations match the covered pack directories.
    /// Absent when the covered pack payloads were unavailable.
    pub pack_members_match: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
struct IndexStatistics {
    counts: [u64; 3],
    predicates_match: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct IndexFoldClaims {
    statistics: IndexStatistics,
    content_digest: Option<Digest>,
}

impl IndexFoldClaims {
    pub(super) fn compare(self, counts: [u64; 3], digest: Option<Digest>) -> (bool, bool) {
        (
            self.statistics.counts == counts && self.statistics.predicates_match == Some(true),
            self.content_digest == digest,
        )
    }
}

#[derive(Debug, Default)]
pub(super) struct TypeRuns(Vec<(Option<u64>, u64)>);

impl TypeRuns {
    pub(super) fn reserve_next(&mut self, frame_type: Option<u64>) -> Result<(), TryReserveError> {
        if self.0.last().is_none_or(|(code, _)| *code != frame_type) {
            self.0.try_reserve(1)?;
        }
        Ok(())
    }

    pub(super) fn record(&mut self, frame_type: Option<u64>) {
        if let Some((code, count)) = self.0.last_mut()
            && *code == frame_type
        {
            // The range has already admitted this ordinary frame.
            *count += 1;
        } else {
            self.0.push((frame_type, 1));
        }
    }

    pub(super) fn inspect(
        &self,
        frame: &FrameEnvelope<'_>,
        checked: &FrameCheck<'_>,
        registry: NonZeroU64,
        head: &Digest,
        range: &Range,
        content: Option<&IndexContent>,
    ) -> Option<Result<(IndexPrefixCheck, IndexFoldClaims), ContentError>> {
        if checked.malformed.is_some() || checked.link.state == LinkState::Broken {
            return None;
        }
        let PayloadState::Verified(payload) = &checked.payload.state else {
            return None;
        };
        Some(self.check(frame, payload, registry, head, range, content))
    }

    fn check(
        &self,
        frame: &FrameEnvelope<'_>,
        payload: &[u8],
        registry: NonZeroU64,
        head: &Digest,
        range: &Range,
        content: Option<&IndexContent>,
    ) -> Result<(IndexPrefixCheck, IndexFoldClaims), ContentError> {
        let (count, claimed_head) = frame.read_field(FrameField::Content).count_and_digest()?;
        let mut fields = Fields::new(payload);
        fields.record(6)?;
        let type_runs_match = self.compare(&mut fields, registry)?;
        let graph_directory_matches = check_graph_directory(&mut fields, registry, content)?;
        let statistics = check_statistics(&mut fields, content.and_then(IndexContent::predicates))?;
        let root = fields.fixed_bytes::<DIGEST_LENGTH>()?;
        let content_digest = read_content_digest(&mut fields, registry)?;
        let pack_members_match = check_pack_members(&mut fields, content)?;
        fields.finish()?;
        let prefix = IndexPrefixCheck {
            head_matches: count == range.count().get() - 1 && claimed_head == head,
            root_matches: *root == range.root(),
            type_runs_match,
            graph_directory_matches,
            pack_members_match,
        };
        Ok((
            prefix,
            IndexFoldClaims {
                statistics,
                content_digest,
            },
        ))
    }

    fn compare(&self, fields: &mut Fields<'_>, registry: NonZeroU64) -> Result<bool, ContentError> {
        let count = fields.array()?;
        let mut matches = count == self.0.len() as u64;
        let mut expected = self.0.iter().copied();
        for _ in 0..count {
            fields.record(2)?;
            // A newer registry's type runs can describe unknown ordinary
            // frames by the code at their envelope's frame type field.
            let code = if registry.get() > REGISTRY_VERSION {
                fields.positive(MalformedReason::UnregisteredValue)?.get()
            } else {
                read_enum(fields, LAST_FRAME_TYPE, registry)?
            };
            let length = fields.uint()?;
            matches &= expected.next() == Some((Some(code), length));
        }
        Ok(matches)
    }
}

fn check_statistics(
    fields: &mut Fields<'_>,
    predicates: Option<&crate::FastMap<u64, u128>>,
) -> Result<IndexStatistics, WireError> {
    fields.record(4)?;
    let counts = [fields.uint()?, fields.uint()?, fields.uint()?];
    let entries = fields.array()?;
    let mut matches = predicates.is_some_and(|values| values.len() as u64 == entries);
    let mut last = None;
    for _ in 0..entries {
        fields.record(2)?;
        let predicate = fields.uint()?;
        let rows = u128::from(fields.uint()?);
        matches &= last.is_none_or(|last| last < predicate)
            && predicates
                .and_then(|values| values.get(&predicate))
                .copied()
                == Some(rows);
        last = Some(predicate);
    }
    Ok(IndexStatistics {
        counts,
        predicates_match: predicates.map(|_| matches),
    })
}

fn read_content_digest(
    fields: &mut Fields<'_>,
    registry: NonZeroU64,
) -> Result<Option<Digest>, ContentError> {
    Ok(match fields.option_shape()? {
        OptionShape::Absent => None,
        OptionShape::Present => Some(*fields.fixed_bytes::<DIGEST_LENGTH>()?),
        OptionShape::Unknown {
            code, tag_offset, ..
        } if code != 0 && registry.get() > REGISTRY_VERSION => {
            return Err(ContentError::UnknownEnumeration {
                code,
                offset: tag_offset,
            });
        }
        OptionShape::Unknown { tag_offset, .. } => {
            return Err(WireError::new(MalformedReason::UnregisteredValue, tag_offset).into());
        }
    })
}

/// An accelerator item outside the frame chain and parity stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum IndexStorageItem {
    /// The list of encoded lengths of earlier items.
    Offsets,
    /// The distance back to the closing index.
    Locator,
}

/// A storage accelerator's claim or placement failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum IndexStorageProblem {
    /// The claim differs from the observed item lengths or index location.
    ClaimMismatch,
    /// The item is out of order or repeated within its segment.
    Misplaced,
    /// A placed item breaks its wire grammar.
    Malformed(WireError),
}

/// A diagnostic for an offsets item or locator read in segment order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct IndexStorageFinding {
    /// Physical offset of the accelerator item.
    pub offset: usize,
    /// The accelerator that produced this diagnostic.
    pub item: IndexStorageItem,
    /// The claim or placement failure.
    pub problem: IndexStorageProblem,
}

struct StorageClaim {
    offset: usize,
    position: usize,
    item: IndexStorageItem,
    claim: Result<bool, WireError>,
}

#[derive(Default)]
pub(super) struct StorageIndex {
    last_index: Option<usize>,
    claims: Vec<StorageClaim>,
}

impl StorageIndex {
    pub(super) const fn closing_index(&mut self, offset: usize) {
        self.last_index = Some(offset);
    }

    pub(super) fn locator_matches(&self, offset: usize, distance: u64) -> bool {
        self.last_index
            .is_some_and(|start| (offset - start) as u64 == distance)
    }

    pub(super) fn observe(
        &mut self,
        offset: usize,
        position: usize,
        item: IndexStorageItem,
        claim: Result<bool, WireError>,
    ) -> Result<(), TryReserveError> {
        push(
            &mut self.claims,
            StorageClaim {
                offset,
                position,
                item,
                claim,
            },
        )
    }

    pub(super) fn finish(
        &self,
        item_count: usize,
        findings: &mut Vec<IndexStorageFinding>,
    ) -> Result<(), TryReserveError> {
        let last_position = item_count - 1;
        let last_item_is_locator = self.claims.last().is_some_and(|claim| {
            claim.position == last_position && claim.item == IndexStorageItem::Locator
        });
        let mut locator_seen = false;
        for (index, claim) in self.claims.iter().enumerate() {
            let last = claim.position == last_position;
            let placed = match claim.item {
                IndexStorageItem::Offsets => {
                    index == 0
                        && (last
                            || (last_item_is_locator
                                && last_position.checked_sub(1) == Some(claim.position)))
                }
                IndexStorageItem::Locator => {
                    let placed = last && !locator_seen;
                    locator_seen = true;
                    placed
                }
            };
            let problem = if !placed {
                IndexStorageProblem::Misplaced
            } else {
                match claim.claim {
                    Ok(true) => continue,
                    Ok(false) => IndexStorageProblem::ClaimMismatch,
                    Err(error) => IndexStorageProblem::Malformed(error),
                }
            };
            push(
                findings,
                IndexStorageFinding {
                    offset: claim.offset,
                    item: claim.item,
                    problem,
                },
            )?;
        }
        Ok(())
    }
}
