// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::ops::Range;
use std::io::{self, Read, Seek, SeekFrom};

use super::{RangeReason, Stop};
use purrdf_lex::cbor::{DecodeErrorKind, Limits, StructuralRange};

pub(super) struct Input<R> {
    reader: R,
    remaining: u64,
}

impl<R> Input<R>
where
    R: FnMut(u64, &mut [u8]) -> io::Result<()>,
{
    pub(super) const fn new(reader: R, decoded_bytes: u64) -> Self {
        Self {
            reader,
            remaining: decoded_bytes,
        }
    }

    pub(super) fn charge(&mut self, bytes: u64) -> Result<(), Stop> {
        self.remaining = self
            .remaining
            .checked_sub(bytes)
            .ok_or(Stop::Unknown(RangeReason::Capacity))?;
        Ok(())
    }

    fn fetch(&mut self, offset: u64, output: &mut [u8]) -> Result<(), Stop> {
        (self.reader)(offset, output).map_err(|_| Stop::Unknown(RangeReason::Read))
    }
}

struct Kept {
    offset: u64,
    bytes: Vec<u8>,
}

#[derive(Clone, Copy)]
pub(super) enum ScanStrings {
    Skip,
    Keep,
    Limited(u64),
}

impl ScanStrings {
    fn admit(self, end: u64) -> Result<(), Stop> {
        if matches!(self, Self::Limited(limit) if end > limit) {
            return Err(Stop::Unknown(RangeReason::Capacity));
        }
        Ok(())
    }
}

impl Kept {
    fn end(&self) -> u64 {
        self.offset + self.bytes.len() as u64
    }
}

/// Keep the ranges read within one item. After placing the items from the
/// offsets list, every item has disjoint extents. Earlier reads of the index
/// and its suffix are attached to the placed items at their original offsets.
pub(super) struct ItemInput {
    extent: Range<u64>,
    kept: Vec<Kept>,
}

impl ItemInput {
    pub(super) const fn new(extent: Range<u64>) -> Self {
        Self {
            extent,
            kept: Vec::new(),
        }
    }

    pub(super) const fn extent(&self) -> &Range<u64> {
        &self.extent
    }

    pub(super) fn end_at(&mut self, end: u64) {
        self.extent.end = end;
    }

    pub(super) fn read<R>(
        &mut self,
        input: &mut Input<R>,
        offset: u64,
        output: &mut [u8],
        outside: RangeReason,
    ) -> Result<(), Stop>
    where
        R: FnMut(u64, &mut [u8]) -> io::Result<()>,
    {
        let length = output.len() as u64;
        let index = self.admit(input, offset, length, outside)?;
        self.fill(input, index, offset, output)
    }

    pub(super) fn bytes<R>(
        &mut self,
        input: &mut Input<R>,
        offset: u64,
        length: u64,
        outside: RangeReason,
    ) -> Result<Vec<u8>, Stop>
    where
        R: FnMut(u64, &mut [u8]) -> io::Result<()>,
    {
        let index = self.admit(input, offset, length, outside)?;
        let length = usize::try_from(length).map_err(|_| Stop::AddressSpace)?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length)?;
        bytes.resize(length, 0);
        self.fill(input, index, offset, &mut bytes)?;
        Ok(bytes)
    }

    fn admit<R>(
        &self,
        input: &mut Input<R>,
        offset: u64,
        length: u64,
        outside: RangeReason,
    ) -> Result<usize, Stop>
    where
        R: FnMut(u64, &mut [u8]) -> io::Result<()>,
    {
        let end = offset
            .checked_add(length)
            .filter(|&end| self.extent().start <= offset && end <= self.extent().end)
            .ok_or(Stop::Unknown(outside))?;
        let index = self.kept.partition_point(|part| part.end() <= offset);
        let cached = self.kept[index..]
            .iter()
            .take_while(|part| part.offset < end)
            .map(|part| part.end().min(end) - part.offset.max(offset))
            .sum::<u64>();
        input.charge(length - cached)?;
        Ok(index)
    }

    fn fill<R>(
        &mut self,
        input: &mut Input<R>,
        mut index: usize,
        offset: u64,
        mut output: &mut [u8],
    ) -> Result<(), Stop>
    where
        R: FnMut(u64, &mut [u8]) -> io::Result<()>,
    {
        // Admission has already validated the original request.
        let end = offset + output.len() as u64;
        let mut cursor = offset;
        while cursor < end {
            let following = self.kept.get(index);
            if let Some(part) = following.filter(|part| part.offset <= cursor) {
                // Both bounds are inside this retained byte vector.
                let start = (cursor - part.offset) as usize;
                let len = (part.end().min(end) - cursor) as usize;
                output[..len].copy_from_slice(&part.bytes[start..start + len]);
                output = &mut output[len..];
                cursor += len as u64;
                index += 1;
            } else {
                let through = following.map_or(end, |part| part.offset.min(end));
                // The unread range is inside the caller's output buffer.
                let len = (through - cursor) as usize;
                index += usize::from(self.extend(input, index, cursor, &mut output[..len])?);
                output = &mut output[len..];
                cursor = through;
            }
        }
        Ok(())
    }

    fn extend<R>(
        &mut self,
        input: &mut Input<R>,
        index: usize,
        offset: u64,
        output: &mut [u8],
    ) -> Result<bool, Stop>
    where
        R: FnMut(u64, &mut [u8]) -> io::Result<()>,
    {
        let len = output.len();
        if let Some(previous) = index
            .checked_sub(1)
            .and_then(|previous| self.kept.get_mut(previous))
            .filter(|part| part.end() == offset)
        {
            previous.bytes.try_reserve(len)?;
            input.fetch(offset, output)?;
            previous.bytes.extend_from_slice(output);
            Ok(false)
        } else {
            let mut bytes = Vec::new();
            self.kept.try_reserve(1)?;
            bytes.try_reserve_exact(len)?;
            input.fetch(offset, output)?;
            bytes.extend_from_slice(output);
            self.kept.insert(index, Kept { offset, bytes });
            Ok(true)
        }
    }

    pub(super) fn scan<R>(
        &mut self,
        input: &mut Input<R>,
        at: u64,
        through: u64,
        strings: ScanStrings,
        invalid: RangeReason,
    ) -> Result<StructuralRange, Stop>
    where
        R: FnMut(u64, &mut [u8]) -> io::Result<()>,
    {
        let mut walker = Walker {
            input,
            item: self,
            offset: at,
            through,
            arguments: [0; 8],
            next_argument: 0,
            argument_end: 0,
            strings,
            invalid,
            stopped: None,
        };
        let result = StructuralRange::scan(
            &mut walker,
            at,
            through,
            Limits {
                max_depth: usize::try_from(through.saturating_sub(at)).unwrap_or(usize::MAX),
            },
        );
        if let Some(stop) = walker.stopped {
            return Err(stop);
        }
        match result {
            Ok(Some(range)) => Ok(range),
            Err(error) if matches!(error.kind(), DecodeErrorKind::TooLong) => {
                Err(Stop::Structure(error))
            }
            _ => Err(Stop::Unknown(invalid)),
        }
    }
}

struct Walker<'a, R> {
    input: &'a mut Input<R>,
    item: &'a mut ItemInput,
    offset: u64,
    through: u64,
    arguments: [u8; 8],
    next_argument: usize,
    argument_end: usize,
    strings: ScanStrings,
    invalid: RangeReason,
    stopped: Option<Stop>,
}

impl<R> Walker<'_, R>
where
    R: FnMut(u64, &mut [u8]) -> io::Result<()>,
{
    fn failed(&mut self, stop: Stop) -> io::Error {
        self.stopped = Some(stop);
        io::Error::other("GTS range walk stopped")
    }

    fn byte(&mut self) -> Result<u8, Stop> {
        // StructuralRange reads the first byte of each head and consumes its
        // argument one byte at a time. The GTS source reads the argument as
        // one range, then serves those bytes to the structural walk.
        if self.next_argument < self.argument_end {
            if self.next_argument == 0 {
                let argument = &mut self.arguments[..self.argument_end];
                let end = self
                    .offset
                    .checked_add(argument.len() as u64)
                    .filter(|&end| end <= self.through)
                    .ok_or(Stop::Unknown(self.invalid))?;
                self.strings.admit(end)?;
                self.item
                    .read(self.input, self.offset, argument, self.invalid)?;
            }
            let byte = self.arguments[self.next_argument];
            self.next_argument += 1;
            return Ok(byte);
        }
        let mut head = [0];
        self.strings.admit(self.offset + 1)?;
        self.item
            .read(self.input, self.offset, &mut head, self.invalid)?;
        self.next_argument = 0;
        self.argument_end = argument_width(head[0]);
        Ok(head[0])
    }
}

pub(super) const fn argument_width(first: u8) -> usize {
    match first & 31 {
        code @ 24..=27 => 1 << (code - 24),
        _ => 0,
    }
}

impl<R> Read for Walker<'_, R>
where
    R: FnMut(u64, &mut [u8]) -> io::Result<()>,
{
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let Some(first) = output.first_mut().filter(|_| self.offset < self.through) else {
            return Ok(0);
        };
        *first = self.byte().map_err(|stop| self.failed(stop))?;
        self.offset += 1;
        Ok(1)
    }
}

impl<R> Seek for Walker<'_, R>
where
    R: FnMut(u64, &mut [u8]) -> io::Result<()>,
{
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let SeekFrom::Start(offset) = position else {
            return Err(io::Error::other(
                "structural source requires an absolute byte offset",
            ));
        };
        if !matches!(self.strings, ScanStrings::Skip) && offset > self.offset {
            self.strings
                .admit(offset)
                .map_err(|stop| self.failed(stop))?;
            self.item
                .bytes(self.input, self.offset, offset - self.offset, self.invalid)
                .map_err(|stop| self.failed(stop))?;
        }
        self.offset = offset;
        Ok(offset)
    }
}
