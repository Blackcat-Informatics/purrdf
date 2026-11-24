// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::ops::Range;

use purrdf_lex::cbor::head;

use super::frame::{FrameHead, Strings};
use super::input::{Input, ItemInput, ScanStrings, argument_width};
use super::{RangeReason, Stop};
use crate::v2::fields::{Fields, push};
use crate::v2::parity::SPAN_BYTES;
use crate::v2::{FrameType, Item, PayloadCapacity};

const LOCATOR_BYTES: u64 = 11;

pub(super) fn locate(
    input: &mut Input<impl FnMut(u64, &mut [u8]) -> std::io::Result<()>>,
    file_end: u64,
    segment_end: u64,
) -> Result<(u64, u64), Stop> {
    let stop = Stop::Unknown(RangeReason::Locator);
    let offset = segment_end
        .checked_sub(LOCATOR_BYTES)
        .filter(|_| segment_end <= file_end)
        .ok_or(stop)?;
    let mut bytes = [0; LOCATOR_BYTES as usize];
    ItemInput::new(offset..segment_end).read(input, offset, &mut bytes, RangeReason::Locator)?;
    let item = Item::first(&bytes)
        .ok()
        .flatten()
        .filter(|item| item.bytes().len() == bytes.len())
        .ok_or(Stop::Unknown(RangeReason::Locator))?;
    let distance = item
        .locator_distance()
        .ok()
        .filter(|&distance| distance != 0 && distance <= offset)
        .ok_or(Stop::Unknown(RangeReason::Locator))?;
    Ok((offset - distance, offset))
}

pub(super) struct Layout {
    pub(super) items: Vec<Range<u64>>,
    pub(super) index: usize,
}

pub(super) struct IndexArea {
    pub(super) layout: Layout,
    pub(super) index: ItemInput,
    pub(super) head: FrameHead,
}

pub(super) fn index_area(
    input: &mut Input<impl FnMut(u64, &mut [u8]) -> std::io::Result<()>>,
    at: u64,
    locator: u64,
    items: u64,
    frame_limits: PayloadCapacity,
) -> Result<IndexArea, Stop> {
    let invalid = RangeReason::IndexItem;
    let mut index = ItemInput::new(at..locator);
    let head = FrameHead::read(
        input,
        &mut index,
        Some(FrameType::Index),
        Strings::Full {
            stored_limit: frame_limits.stored_bytes_per_frame,
        },
        invalid,
    )?;
    head.admit_payload(input, &mut index, frame_limits, invalid)?;
    let payload = &head.payload;
    index.bytes(input, payload.start, payload.end - payload.start, invalid)?;
    let index_end = head.walk_tail(
        input,
        &mut index,
        Some(frame_limits.stored_bytes_per_frame),
        invalid,
    )?;
    index.end_at(index_end);

    let mut suffix = Vec::new();
    push(&mut suffix, at..index_end)?;
    let mut at = index_end;
    while at < locator {
        let mut part = ItemInput::new(at..locator);
        let first = head_at(input, &mut part, at, invalid)?;
        let kind = if first.major == head::ARRAY && first.argument != Some(0) {
            Some(head_at(input, &mut part, first.end, invalid)?)
        } else {
            None
        };
        let is_frame = first.first == 0x8b && kind.as_ref().is_some_and(|head| head.first == 2);
        let through = if is_frame {
            let frame = FrameHead::read(input, &mut part, None, Strings::Skip, invalid)?;
            frame.walk_tail(input, &mut part, None, invalid)?
        } else {
            part.scan(input, at, locator, ScanStrings::Skip, invalid)?
                .end()
        };
        if through == locator {
            if kind.is_none_or(|head| head.first != 4) {
                return Err(Stop::Unknown(invalid));
            }
            let bytes = part.bytes(input, at, through - at, invalid)?;
            let layout = layout(&bytes, at, &suffix, items)?;
            return Ok(IndexArea {
                layout,
                index,
                head,
            });
        }
        push(&mut suffix, at..through)?;
        at = through;
    }
    Err(Stop::Unknown(invalid))
}

/// The suffix was structurally walked from the index to the offsets item.
/// Every walked item's original extent must agree with the storage claim.
pub(super) fn layout(
    offsets: &[u8],
    offsets_start: u64,
    suffix: &[Range<u64>],
    item_limit: u64,
) -> Result<Layout, Stop> {
    let decode = || {
        let mut fields = Fields::new(offsets);
        fields.record(2)?;
        fields.fixed_uint(
            super::super::item::OFFSETS_KIND,
            crate::v2::MalformedReason::WrongType,
        )?;
        let count = fields.array()?;
        Ok::<_, crate::v2::WireError>((fields, count))
    };
    let (lengths, count) = decode().map_err(|_| Stop::Unknown(RangeReason::Offsets))?;
    let mut fields = lengths;
    let mut total = 0_u128;
    for _ in 0..count {
        let length = fields
            .uint()
            .ok()
            .filter(|&length| length != 0)
            .ok_or(Stop::Unknown(RangeReason::Offsets))?;
        total += u128::from(length);
    }
    fields
        .finish()
        .map_err(|_| Stop::Unknown(RangeReason::Offsets))?;
    if count >= item_limit {
        return Err(Stop::Unknown(RangeReason::Capacity));
    }
    let start = u128::from(offsets_start)
        .checked_sub(total)
        .ok_or(Stop::Unknown(RangeReason::Offsets))?;
    let mut start = start as u64;
    let mut items = Vec::new();
    let count = usize::try_from(count).map_err(|_| Stop::AddressSpace)?;
    items.try_reserve_exact(count)?;
    let mut lengths = lengths;
    for _ in 0..count {
        let length = lengths
            .uint()
            .expect("all encoded item lengths were checked");
        // The total length was checked against the start of the offsets.
        let end = start + length;
        items.push(start..end);
        start = end;
    }
    let index = items
        .len()
        .checked_sub(suffix.len())
        .filter(|&index| index != 0 && !suffix.is_empty() && items[index..] == *suffix)
        .ok_or(Stop::Unknown(RangeReason::Offsets))?;
    Ok(Layout { items, index })
}

impl Layout {
    /// Place the ordinary covered frames and skip parity according to the
    /// covered prefix's encoded lengths. No byte is acquired at this step.
    pub(super) fn covered(&self, count: u64) -> Result<Vec<usize>, Stop> {
        let prefix = &self.items[1..self.index];
        let count = usize::try_from(count)
            .ok()
            .filter(|&count| count <= prefix.len())
            .ok_or(Stop::Unknown(RangeReason::Placement))?;
        let mut covered = Vec::new();
        covered.try_reserve_exact(count)?;
        if prefix.len() == count {
            covered.extend(1..self.index);
            return Ok(covered);
        }
        let span = SPAN_BYTES as u128;
        let mut ordinary_bytes = u128::from(self.items[0].end - self.items[0].start);
        let mut span_end = span;
        for (i, range) in prefix.iter().enumerate() {
            if ordinary_bytes >= span_end {
                span_end += span;
            } else {
                if covered.len() == count {
                    return Err(Stop::Unknown(RangeReason::Placement));
                }
                covered.push(i + 1);
                ordinary_bytes += u128::from(range.end - range.start);
            }
        }
        if covered.len() != count {
            return Err(Stop::Unknown(RangeReason::Placement));
        }
        Ok(covered)
    }
}

pub(super) fn head_at(
    input: &mut Input<impl FnMut(u64, &mut [u8]) -> std::io::Result<()>>,
    item: &mut ItemInput,
    offset: u64,
    outside: RangeReason,
) -> Result<CborHead, Stop> {
    let mut bytes = [0; 9];
    item.read(input, offset, &mut bytes[..1], outside)?;
    let first = bytes[0];
    let major = first >> 5;
    let width = argument_width(first);
    if width != 0 {
        item.read(input, offset + 1, &mut bytes[1..=width], outside)?;
    }
    let encoded = &mut &bytes[..=width];
    Ok(CborHead {
        first,
        end: offset + 1 + width as u64,
        major,
        argument: head::read_argument(encoded, major),
    })
}

pub(super) struct CborHead {
    pub(super) first: u8,
    pub(super) end: u64,
    pub(super) major: u8,
    pub(super) argument: Option<u64>,
}

impl CborHead {
    pub(super) fn uint(&self, start: u64) -> Option<u64> {
        let code = self.argument.filter(|_| self.major == head::UNSIGNED)?;
        head::with_encoded_head(head::UNSIGNED, code, |encoded| {
            (encoded.len() as u64 == self.end - start).then_some(code)
        })
    }
}
