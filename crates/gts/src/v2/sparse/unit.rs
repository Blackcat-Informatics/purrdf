// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;

use crate::v2::fields::{copy_bytes, push};
use crate::v2::read::blob::{Member, Pack, UnitDeclaration};
use crate::v2::{DIGEST_LENGTH, FrameType, MalformedReason, ValueName};

use super::frame::LocalFrame;
use super::{RangeReason, Stop};

type Digest = [u8; DIGEST_LENGTH];

/// A verified unit of a sparse answer.
#[derive(Debug)]
#[non_exhaustive]
pub struct RangeUnit {
    /// Item number placed by the segment's offsets list.
    pub position: u64,
    /// Position in the directory of a pack, when applicable.
    pub member: Option<u64>,
    /// The locally verified unit bytes.
    pub bytes: Vec<u8>,
}

pub(super) fn loose_unit(
    frame: &LocalFrame,
    kind: FrameType,
    target: &Digest,
    position: usize,
) -> Result<RangeUnit, Stop> {
    let declaration = UnitDeclaration::read(&mut frame.content(), kind == FrameType::Blob)?;
    let bytes = frame.payload();
    if declaration.digest != *target {
        return Err(Stop::Unknown(RangeReason::Contradiction));
    }
    if ValueName::of(bytes).digest != *target {
        return Err(Stop::Unknown(RangeReason::Payload));
    }
    Ok(RangeUnit {
        position: position as u64,
        member: None,
        bytes: copy_bytes(bytes)?,
    })
}

/// All claims for this digest and this pack are presented in member order.
/// Verify the claim in both directions before returning units from the pack.
pub(super) fn pack_units(
    frame: &LocalFrame,
    registry: NonZeroU64,
    target: &Digest,
    position: usize,
    mut claimed: impl Iterator<Item = u64>,
    output: &mut Vec<RangeUnit>,
) -> Result<(), Stop> {
    let invalid = Stop::Unknown(RangeReason::Payload);
    let mut content = frame.content();
    content.record(2)?;
    let count = content.positive(MalformedReason::OutOfRange)?;
    let reported_units = content.uint()?;
    let pack = Pack::read(frame.payload(), count.get(), registry).map_err(Stop::content)?;
    if pack.unit_length != u128::from(reported_units) {
        return Err(invalid);
    }
    let members = pack.members().map_err(Stop::content)?;
    let mut next = claimed.next();
    for (index, member) in members.iter().enumerate() {
        let index = index as u64;
        let unit = match member {
            Member::Unit(declaration, bytes) if declaration.digest == *target => Some(*bytes),
            _ => None,
        };
        if next == Some(index) {
            let bytes = unit.ok_or(Stop::Unknown(RangeReason::Contradiction))?;
            if ValueName::of(bytes).digest != *target {
                return Err(Stop::Unknown(RangeReason::Payload));
            }
            push(
                output,
                RangeUnit {
                    position: position as u64,
                    member: Some(index),
                    bytes: copy_bytes(bytes)?,
                },
            )?;
            next = claimed.next();
        } else if unit.is_some() {
            return Err(Stop::Unknown(RangeReason::Contradiction));
        }
    }
    if next.is_some() {
        return Err(Stop::Unknown(RangeReason::Contradiction));
    }
    Ok(())
}
