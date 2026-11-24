// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;
use core::ops::Range;
use std::borrow::Cow;
use std::io;

use purrdf_lex::cbor::head;

use super::input::{Input, ItemInput, ScanStrings};
use super::opening::head_at;
use super::{RangeReason, Stop};
use crate::v2::fields::Fields;
use crate::v2::link::permissive_identifier;
use crate::v2::payload::{STORED_CHECK_LENGTH, ZSTD};
use crate::v2::{
    DIGEST_LENGTH, FRAME_FIELDS, FrameEnvelope, FrameField, FrameType, Item, LinkState,
    PayloadBudget, PayloadCapacity, PayloadState,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Strings {
    Skip,
    Full { stored_limit: u64 },
    Content,
    Predecessor,
}

pub(super) struct FrameHead {
    pub(super) kind: Option<FrameType>,
    pub(super) payload: Range<u64>,
    anchor: Option<Range<u64>>,
    fields: [u64; 9],
}

impl FrameHead {
    pub(super) fn read(
        input: &mut Input<impl FnMut(u64, &mut [u8]) -> io::Result<()>>,
        item: &mut ItemInput,
        expected: Option<FrameType>,
        strings: Strings,
        invalid: RangeReason,
    ) -> Result<Self, Stop> {
        let mut offset = item.extent().start;
        let through = item.extent().end;
        let array = head_at(input, item, offset, invalid)?;
        if array.first != 0x8b {
            return Err(Stop::Unknown(invalid));
        }
        let generation = head_at(input, item, array.end, invalid)?;
        if generation.first != 2 {
            return Err(Stop::Unknown(invalid));
        }
        let type_head = head_at(input, item, generation.end, invalid)?;
        let code = type_head.uint(generation.end);
        if code.is_none() && expected.is_some() {
            return Err(Stop::Unknown(invalid));
        }
        let kind = code.and_then(FrameType::from_code);
        if expected.is_some() && kind.is_none() && invalid == RangeReason::Placement {
            return Err(Stop::Unknown(RangeReason::FrameType));
        }
        if expected.is_some_and(|expected| kind != Some(expected)) {
            return Err(Stop::Unknown(invalid));
        }

        let invalid = if invalid == RangeReason::Placement {
            RangeReason::Payload
        } else {
            invalid
        };
        let mut fields = [offset; 9];
        let mut anchor = None;
        fields[1] = generation.end;
        offset = if code.is_some() {
            type_head.end
        } else {
            item.scan(input, generation.end, through, ScanStrings::Skip, invalid)?
                .end()
        };
        fields[2] = offset;
        for (field, end) in fields.iter_mut().enumerate().skip(3) {
            let element = field - 1;
            let from = offset;
            offset = if element == FrameField::Content as usize
                && kind == Some(FrameType::Parity)
                && strings == Strings::Predecessor
            {
                let (end, extent) = parity_anchor_head(input, item, offset, through, invalid)?;
                anchor = extent;
                end
            } else {
                let keep = matches!(strings, Strings::Full { .. })
                    || (strings == Strings::Content && element == FrameField::Content as usize);
                let scan = if keep {
                    ScanStrings::Keep
                } else {
                    ScanStrings::Skip
                };
                item.scan(input, offset, through, scan, invalid)?.end()
            };
            if let Strings::Full { stored_limit } = strings
                && element == FrameField::Content as usize
                && offset - from > stored_limit
            {
                return Err(Stop::Unknown(RangeReason::Capacity));
            }
            *end = offset;
        }
        let payload = head_at(input, item, offset, invalid)?;
        if payload.major != head::BYTES {
            return Err(Stop::Unknown(RangeReason::Payload));
        }
        let payload_end = payload
            .argument
            .and_then(|length| payload.end.checked_add(length))
            .filter(|&end| end <= through)
            .ok_or(Stop::Unknown(invalid))?;
        Ok(Self {
            kind,
            fields,
            anchor,
            payload: payload.end..payload_end,
        })
    }

    pub(super) fn field(&self, field: FrameField) -> Option<Range<u64>> {
        let i = field as usize;
        self.fields.get(i + 1).map(|&end| self.fields[i]..end)
    }

    pub(super) fn admit_payload(
        &self,
        input: &mut Input<impl FnMut(u64, &mut [u8]) -> io::Result<()>>,
        item: &mut ItemInput,
        capacity: PayloadCapacity,
        invalid: RangeReason,
    ) -> Result<(), Stop> {
        let length = self
            .field(FrameField::DecodedLength)
            .expect("the frame head stores the decoded length extent");
        let claimed = item.bytes(input, length.start, length.end - length.start, invalid)?;
        let declared = Fields::new(&claimed).uint().ok();
        if self.payload.end - self.payload.start > capacity.stored_bytes_per_frame
            || declared.is_some_and(|length| length > capacity.decoded_bytes_per_frame)
        {
            return Err(Stop::Unknown(RangeReason::Capacity));
        }
        Ok(())
    }

    pub(super) fn check_full(
        &self,
        input: &mut Input<impl FnMut(u64, &mut [u8]) -> io::Result<()>>,
        item: &mut ItemInput,
        registry: NonZeroU64,
    ) -> Result<(), Stop> {
        let start = item.extent().start;
        let end = self.fields[8];
        let bytes = item.bytes(input, start, end - start, RangeReason::Payload)?;
        // This envelope supports only field checks through element 7. The
        // empty extents beyond it stand for fields not admitted yet.
        let mut ends = [bytes.len(); FRAME_FIELDS];
        for (local, absolute) in ends.iter_mut().zip(&self.fields[1..]) {
            *local = (*absolute - start) as usize;
        }
        FrameEnvelope::new(&bytes, ends).check_range_head(registry)?;
        Ok(())
    }

    pub(super) fn check_unit_head(
        &self,
        input: &mut Input<impl FnMut(u64, &mut [u8]) -> io::Result<()>>,
        item: &mut ItemInput,
        kind: FrameType,
    ) -> Result<(), Stop> {
        let invalid = RangeReason::Payload;
        let mut read = |field| {
            let range = self.field(field).expect("the unit head locates this field");
            item.bytes(input, range.start, range.end - range.start, invalid)
        };
        let critical = Fields::new(&read(FrameField::Critical)?).boolean()?;
        let transparent = Fields::new(&read(FrameField::Transparent)?).boolean()?;
        if critical != kind.critical() || transparent != kind.transparent() {
            return Err(Stop::Unknown(invalid));
        }
        let length = Fields::new(&read(FrameField::DecodedLength)?).uint()?;
        crate::v2::frame::check_type_length(kind, length, 0)?;
        let start = self
            .field(FrameField::PayloadDigest)
            .expect("the unit head locates its digest")
            .start;
        let digest = head_at(input, item, start, invalid)?;
        if digest.first != 0x58 || digest.argument != Some(DIGEST_LENGTH as u64) {
            return Err(Stop::Unknown(invalid));
        }
        self.check_unit_storage(input, item, length)
    }

    fn check_unit_storage(
        &self,
        input: &mut Input<impl FnMut(u64, &mut [u8]) -> io::Result<()>>,
        item: &mut ItemInput,
        length: u64,
    ) -> Result<(), Stop> {
        let invalid = RangeReason::Payload;
        let storage = self
            .field(FrameField::Storage)
            .expect("the unit head locates storage");
        let record = head_at(input, item, storage.start, invalid)?;
        if !matches!(record.first, 0x81 | 0x83) {
            return Err(Stop::Unknown(invalid));
        }
        let tag = head_at(input, item, record.end, invalid)?;
        match (record.first, tag.first) {
            (0x81, 1) => Ok(()),
            (0x83, 2) if length != 0 => {
                let codec = head_at(input, item, tag.end, invalid)?;
                if codec.uint(tag.end) != Some(ZSTD) {
                    return Err(Stop::Unknown(invalid));
                }
                let check = head_at(input, item, codec.end, invalid)?;
                if check.first == 0x40 + STORED_CHECK_LENGTH as u8 {
                    return Ok(());
                }
                Err(Stop::Unknown(invalid))
            }
            _ => Err(Stop::Unknown(invalid)),
        }
    }

    pub(super) fn acquire(
        &self,
        input: &mut Input<impl FnMut(u64, &mut [u8]) -> io::Result<()>>,
        item: &mut ItemInput,
        capacity: PayloadCapacity,
    ) -> Result<(), Stop> {
        let invalid = RangeReason::Payload;
        item.bytes(
            input,
            self.payload.start,
            self.payload.end - self.payload.start,
            invalid,
        )?;
        let tail = item.extent().end - self.payload.end;
        const IDENTIFIER_BYTES: u64 = DIGEST_LENGTH as u64 + 2;
        if tail.saturating_sub(IDENTIFIER_BYTES) > capacity.stored_bytes_per_frame {
            return Err(Stop::Unknown(RangeReason::Capacity));
        }
        item.bytes(input, self.payload.end, tail, invalid)?;
        Ok(())
    }

    pub(super) fn walk_tail(
        &self,
        input: &mut Input<impl FnMut(u64, &mut [u8]) -> io::Result<()>>,
        item: &mut ItemInput,
        signature_limit: Option<u64>,
        invalid: RangeReason,
    ) -> Result<u64, Stop> {
        let through = item.extent().end;
        let identifier_strings = if signature_limit.is_some() {
            ScanStrings::Keep
        } else {
            ScanStrings::Skip
        };
        let identifier = item.scan(
            input,
            self.payload.end,
            through,
            identifier_strings,
            invalid,
        )?;
        let signatures = signature_limit.map_or(ScanStrings::Skip, |limit| {
            ScanStrings::Limited(identifier.end().saturating_add(limit))
        });
        let signatures = item.scan(input, identifier.end(), through, signatures, invalid)?;
        Ok(signatures.end())
    }
}

pub(super) fn predecessor(
    input: &mut Input<impl FnMut(u64, &mut [u8]) -> io::Result<()>>,
    item: &mut ItemInput,
) -> Result<[u8; DIGEST_LENGTH], Stop> {
    let invalid = RangeReason::Identifier;
    let frame = FrameHead::read(input, item, None, Strings::Predecessor, invalid)?;
    let through = item.extent().end;
    let tail = item.bytes(
        input,
        frame.payload.end,
        through - frame.payload.end,
        invalid,
    )?;
    let identifier = Item::first(&tail)
        .map_err(|_| Stop::Unknown(invalid))?
        .ok_or(Stop::Unknown(invalid))?;
    let signatures = &tail[identifier.bytes().len()..];
    let signature = Item::first(signatures)
        .map_err(|_| Stop::Unknown(invalid))?
        .ok_or(Stop::Unknown(invalid))?;
    if signature.bytes().len() != signatures.len() {
        return Err(Stop::Unknown(invalid));
    }
    let transparent = frame
        .field(FrameField::Transparent)
        .expect("frame heads store the transparent flag");
    let transparent = transparent.end - transparent.start == 1
        && item.bytes(input, transparent.start, 1, invalid)? == [0xf5];
    let mut predecessor = identifier.bytes();
    let anchor;
    match (frame.kind, transparent) {
        (Some(FrameType::Parity), true) => {
            let extent = frame.anchor.ok_or(Stop::Unknown(invalid))?;
            anchor = item.bytes(input, extent.start, extent.end - extent.start, invalid)?;
            predecessor = &anchor;
        }
        (_, true) => return Err(Stop::Unknown(invalid)),
        _ => {}
    }
    permissive_identifier(predecessor).ok_or(Stop::Unknown(invalid))
}

#[cfg(test)]
mod tests {
    use core::ops::Range;

    use crate::v2::SegmentWriter;

    use super::{
        DIGEST_LENGTH, FrameField, Input, Item, ItemInput, RangeReason, Stop, predecessor,
    };

    #[test]
    fn predecessor_checks_transparency_without_reading_an_opaque_string_body() {
        let mut writer = SegmentWriter::new(Vec::new()).unwrap();
        writer.add_blob(b"one unit", None).unwrap();
        let bytes = writer.into_inner();
        let opener_length = Item::first(&bytes).unwrap().unwrap().bytes().len();
        let bytes = &bytes[opener_length..];
        let frame = Item::first(bytes)
            .unwrap()
            .unwrap()
            .frame()
            .unwrap()
            .unwrap();
        let expected = frame
            .read_field(FrameField::Identifier)
            .fixed_bytes()
            .unwrap();
        let transparent = frame.start(FrameField::Transparent);
        let mut invalid = bytes.to_vec();
        invalid.splice(
            transparent..transparent + 1,
            [0x58, 32].into_iter().chain([0; 32]),
        );
        let skipped = transparent as u64 + 2..transparent as u64 + 34;
        assert_eq!(&bounded_predecessor(&invalid, skipped).unwrap(), expected);
    }

    #[test]
    fn a_non_bytes_parity_anchor_does_not_request_its_nested_string_body() {
        let fixture = include_bytes!("../../../tests/fixtures/v2/P5/parity-base.bin");
        let frame = Item::first(&fixture[481..])
            .unwrap()
            .unwrap()
            .frame()
            .unwrap()
            .unwrap();
        let mut record = frame.read_field(FrameField::Content);
        record.record(5).unwrap();
        for _ in 0..3 {
            record.uint().unwrap();
        }
        let at = record.offset();
        let anchor = record.fixed_bytes().unwrap();
        assert_eq!(&bounded_predecessor(frame.bytes(), 0..0).unwrap(), anchor);

        let mut invalid = frame.bytes().to_vec();
        invalid.insert(at, 0x81);
        let skipped = at as u64 + 3..at as u64 + 35;
        assert!(matches!(
            bounded_predecessor(&invalid, skipped),
            Err(Stop::Unknown(RangeReason::Identifier))
        ));
    }

    fn bounded_predecessor(bytes: &[u8], skipped: Range<u64>) -> Result<[u8; DIGEST_LENGTH], Stop> {
        let mut input = Input::new(
            |at, destination: &mut [u8]| {
                let end = at + destination.len() as u64;
                assert!(end <= skipped.start || at >= skipped.end);
                destination.copy_from_slice(&bytes[at as usize..end as usize]);
                Ok(())
            },
            u64::MAX,
        );
        let mut item = ItemInput::new(0..bytes.len() as u64);
        predecessor(&mut input, &mut item)
    }
}

enum Decoded {
    Plain(Range<usize>),
    Compressed(Vec<u8>),
}

pub(super) struct LocalFrame {
    bytes: Vec<u8>,
    content: Range<usize>,
    decoded: Decoded,
}

impl LocalFrame {
    pub(super) fn content(&self) -> Fields<'_> {
        Fields::new(&self.bytes[self.content.clone()])
    }

    pub(super) fn payload(&self) -> &[u8] {
        match &self.decoded {
            Decoded::Plain(extent) => &self.bytes[extent.clone()],
            Decoded::Compressed(bytes) => bytes,
        }
    }
}

pub(super) fn verify(
    input: &mut Input<impl FnMut(u64, &mut [u8]) -> io::Result<()>>,
    item: &mut ItemInput,
    head: &FrameHead,
    previous: &[u8; DIGEST_LENGTH],
    capacity: PayloadCapacity,
    registry: NonZeroU64,
) -> Result<LocalFrame, Stop> {
    let invalid = RangeReason::Payload;
    let extent = item.extent().clone();
    let bytes = item.bytes(input, extent.start, extent.end - extent.start, invalid)?;
    let frame = Item::first(&bytes)
        .map_err(|_| Stop::Unknown(invalid))?
        .filter(|item| item.bytes().len() == bytes.len())
        .ok_or(Stop::Unknown(invalid))?
        .frame()
        .map_err(|_| Stop::Unknown(invalid))?
        .ok_or(Stop::Unknown(invalid))?;
    let wire = frame.check_wire(previous, None, registry);
    if wire.malformed.is_some() {
        return Err(Stop::Unknown(invalid));
    }
    if wire.link.state != LinkState::Linked {
        return Err(Stop::Unknown(RangeReason::Identifier));
    }
    let payload = frame.payload_wire(registry);
    if let Some(length) = payload.compressed_length() {
        input.charge(length)?;
    }
    let mut budget = PayloadBudget::new(PayloadCapacity {
        decoded_bytes_per_read: u64::MAX,
        ..capacity
    });
    let checked = frame
        .check_payload(payload, &mut budget)
        .map_err(Stop::DecodeResource)?;
    if checked.malformed.is_some() {
        return Err(Stop::Unknown(invalid));
    }
    let PayloadState::Verified(payload) = checked.state else {
        return Err(Stop::Unknown(invalid));
    };
    let decoded = match payload {
        Cow::Borrowed(_) => {
            let local =
                |offset| usize::try_from(offset - extent.start).map_err(|_| Stop::AddressSpace);
            Decoded::Plain(local(head.payload.start)?..local(head.payload.end)?)
        }
        Cow::Owned(bytes) => Decoded::Compressed(bytes),
    };
    let start = frame.start(FrameField::Content);
    let content = start..start + frame.field(FrameField::Content).len();
    Ok(LocalFrame {
        bytes,
        content,
        decoded,
    })
}

fn parity_anchor_head(
    input: &mut Input<impl FnMut(u64, &mut [u8]) -> io::Result<()>>,
    item: &mut ItemInput,
    offset: u64,
    through: u64,
    invalid: RangeReason,
) -> Result<(u64, Option<Range<u64>>), Stop> {
    let head = head_at(input, item, offset, invalid)?;
    if head.first != 0x85 {
        return item
            .scan(input, offset, through, ScanStrings::Skip, invalid)
            .map(|range| (range.end(), None));
    }
    let mut offset = head.end;
    let mut anchor = None;
    for element in 0..5 {
        let is_anchor = element == 3 && head_at(input, item, offset, invalid)?.major == head::BYTES;
        let scan = if is_anchor {
            ScanStrings::Keep
        } else {
            ScanStrings::Skip
        };
        let end = item.scan(input, offset, through, scan, invalid)?.end();
        if is_anchor {
            anchor = Some(offset..end);
        }
        offset = end;
    }
    Ok((offset, anchor))
}
