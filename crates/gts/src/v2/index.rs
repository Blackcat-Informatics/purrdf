// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;
use std::collections::TryReserveError;

use super::fields::{Fields, OptionShape};
use super::rows::graph_name;
use super::terms::read_enum;
use super::{
    ContentError, DIGEST_LENGTH, FrameCheck, FrameEnvelope, FrameField, FrameType, LinkState,
    MalformedReason, PayloadState, REGISTRY_VERSION, Range, WireError,
};

type Digest = [u8; DIGEST_LENGTH];
const LAST_FRAME_TYPE: u64 = FrameType::Parity as u64;

/// Comparisons of an index's claims with the observed ordinary prefix.
///
/// Each result is independent. The directory, statistics, content digest
/// and pack member claims require additional fold evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct IndexPrefixCheck {
    /// The frame count and head agree with the observed prefix.
    pub head_matches: bool,
    /// The range root agrees with the observed prefix.
    pub root_matches: bool,
    /// The declared runs match the maximal runs of frame types.
    pub type_runs_match: bool,
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
    ) -> Option<Result<IndexPrefixCheck, ContentError>> {
        if checked.malformed.is_some() || checked.link.state == LinkState::Broken {
            return None;
        }
        let PayloadState::Verified(payload) = &checked.payload.state else {
            return None;
        };
        Some(self.check(frame, payload, registry, head, range))
    }

    fn check(
        &self,
        frame: &FrameEnvelope<'_>,
        payload: &[u8],
        registry: NonZeroU64,
        head: &Digest,
        range: &Range,
    ) -> Result<IndexPrefixCheck, ContentError> {
        let (count, claimed_head) = frame.read_field(FrameField::Content).count_and_digest()?;
        let mut fields = Fields::new(payload);
        fields.record(6)?;
        let type_runs_match = self.compare(&mut fields, registry)?;
        check_graph_directory(&mut fields, registry)?;
        check_statistics(&mut fields)?;
        let root = fields.fixed_bytes::<DIGEST_LENGTH>()?;
        check_content_digest(&mut fields, registry)?;
        check_pack_members(&mut fields)?;
        fields.finish()?;
        Ok(IndexPrefixCheck {
            head_matches: count == range.count().get() - 1 && claimed_head == head,
            root_matches: *root == range.root(),
            type_runs_match,
        })
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

fn check_graph_directory(
    fields: &mut Fields<'_>,
    registry: NonZeroU64,
) -> Result<(), ContentError> {
    for _ in 0..fields.array()? {
        fields.record(2)?;
        graph_name(fields, registry)?;
        for _ in 0..fields.array()? {
            fields.record(4)?;
            read_enum(fields, LAST_FRAME_TYPE, registry)?;
            fields.uint()?;
            fields.uint()?;
            fields.uint()?;
        }
    }
    Ok(())
}

fn check_statistics(fields: &mut Fields<'_>) -> Result<(), WireError> {
    fields.record(4)?;
    fields.uint()?;
    fields.uint()?;
    fields.uint()?;
    for _ in 0..fields.array()? {
        fields.record(2)?;
        fields.uint()?;
        fields.uint()?;
    }
    Ok(())
}

fn check_content_digest(fields: &mut Fields<'_>, registry: NonZeroU64) -> Result<(), ContentError> {
    match fields.option_shape()? {
        OptionShape::Absent => {}
        OptionShape::Present => {
            fields.fixed_bytes::<DIGEST_LENGTH>()?;
        }
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
    }
    Ok(())
}

fn check_pack_members(fields: &mut Fields<'_>) -> Result<(), WireError> {
    for _ in 0..fields.array()? {
        fields.record(3)?;
        fields.fixed_bytes::<DIGEST_LENGTH>()?;
        fields.uint()?;
        fields.uint()?;
    }
    Ok(())
}

/// The prefix check of an index encountered during a graph read.
#[derive(Debug)]
#[non_exhaustive]
pub struct GraphIndexCheck {
    /// Physical offset of the index frame.
    pub offset: usize,
    /// Verified-prefix comparisons or a grammar failure in the index.
    pub prefix: Result<IndexPrefixCheck, ContentError>,
}
