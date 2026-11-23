// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt;
use core::num::NonZeroU64;
use std::collections::TryReserveError;
use std::io::{self, Write};

use purrdf_lex::cbor::head;

use super::{DIGEST_LENGTH, MalformedReason, REGISTRY_VERSION};

pub(super) fn push<T>(output: &mut Vec<T>, value: T) -> Result<(), TryReserveError> {
    output.try_reserve(1)?;
    output.push(value);
    Ok(())
}

pub(super) fn copy_bytes(bytes: &[u8]) -> Result<Vec<u8>, TryReserveError> {
    let mut owned = Vec::new();
    owned.try_reserve_exact(bytes.len())?;
    owned.extend_from_slice(bytes);
    Ok(owned)
}

pub(super) fn write_option<W: Write + ?Sized, T>(
    writer: &mut W,
    value: Option<T>,
    field: impl FnOnce(&mut W, T) -> io::Result<()>,
) -> io::Result<()> {
    match value {
        None => writer.write_all(&[0x81, 1]),
        Some(value) => {
            writer.write_all(&[0x82, 2])?;
            field(writer, value)
        }
    }
}

/// The first wire grammar violation and its offset within the item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WireError {
    /// The violated grammar rule.
    pub reason: MalformedReason,
    /// Zero-based byte offset relative to the start of the item.
    pub offset: usize,
}

impl WireError {
    pub(super) const fn new(reason: MalformedReason, offset: usize) -> Self {
        Self { reason, offset }
    }
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self.reason {
            MalformedReason::NotDeterministic => "a non-deterministic head",
            MalformedReason::WrongType => "the wrong type",
            MalformedReason::WrongLength => "the wrong length",
            MalformedReason::OutOfRange => "an out-of-range value",
            MalformedReason::UnregisteredValue => "an unregistered value",
            MalformedReason::ForbiddenCombination => "a forbidden field combination",
            MalformedReason::CodecConstraint => "a nonconforming codec header",
            MalformedReason::UnorderedOrRepeated => "an out-of-order or repeated element",
            MalformedReason::InconsistentCounts => "inconsistent counts",
        };
        write!(f, "GTS v2 byte {} has {reason}", self.offset)
    }
}

impl std::error::Error for WireError {}

impl purrdf_hash::error::StructuredError for WireError {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        use purrdf_hash::error::ErrorValue;
        visitor.variant("gts-v2-malformed-value");
        visitor.value(Some("reason"), ErrorValue::Code(self.reason.code()));
        visitor.value(Some("offset"), ErrorValue::ByteCount(self.offset as u128));
    }
}

impl MalformedReason {
    pub(super) const fn code(self) -> &'static str {
        match self {
            Self::NotDeterministic => "not_deterministic",
            Self::WrongType => "wrong_type",
            Self::WrongLength => "wrong_length",
            Self::OutOfRange => "out_of_range",
            Self::UnregisteredValue => "unregistered_value",
            Self::ForbiddenCombination => "forbidden_combination",
            Self::CodecConstraint => "codec_constraint",
            Self::UnorderedOrRepeated => "unordered_or_repeated",
            Self::InconsistentCounts => "inconsistent_counts",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Fields<'a> {
    tail: &'a [u8],
    length: usize,
}

pub(super) enum OptionShape {
    Absent,
    Present,
    Unknown {
        code: u64,
        remaining: u64,
        tag_offset: usize,
    },
}

impl<'a> Fields<'a> {
    pub(super) const fn new(bytes: &'a [u8]) -> Self {
        Self::at(bytes, 0)
    }

    /// Read a field at its original offset in a framed item.
    pub(super) const fn at(bytes: &'a [u8], offset: usize) -> Self {
        Self {
            tail: bytes,
            length: bytes.len() + offset,
        }
    }

    pub(super) const fn offset(&self) -> usize {
        self.length - self.tail.len()
    }

    pub(super) const fn remaining(&self) -> &'a [u8] {
        self.tail
    }

    fn argument(&mut self, expected: u8) -> Result<u64, WireError> {
        let offset = self.offset();
        let (major, argument) = self.typed_argument()?;
        if major != expected {
            return Err(WireError::new(MalformedReason::WrongType, offset));
        }
        Ok(argument)
    }

    fn typed_argument(&mut self) -> Result<(u8, u64), WireError> {
        let offset = self.offset();
        let Some(&first) = self.tail.first() else {
            return Err(WireError::new(MalformedReason::WrongLength, offset));
        };
        let major = first >> 5;
        let information = first & 31;
        if information == 31 {
            let reason = match major {
                2..=4 => MalformedReason::NotDeterministic,
                5 => MalformedReason::WrongType,
                _ => MalformedReason::WrongLength,
            };
            return Err(WireError::new(reason, offset));
        }
        let mut rest = self.tail;
        let argument = head::read_argument(&mut rest, major)
            .ok_or_else(|| WireError::new(MalformedReason::WrongLength, offset))?;
        if major == 7 && information == 24 && argument < 32 {
            return Err(WireError::new(MalformedReason::WrongLength, offset));
        }
        if major <= head::ARRAY && self.tail.len() - rest.len() != head::head_len(argument) {
            return Err(WireError::new(MalformedReason::NotDeterministic, offset));
        }
        if major > head::ARRAY {
            return Err(WireError::new(MalformedReason::WrongType, offset));
        }
        self.tail = rest;
        Ok((major, argument))
    }

    pub(super) fn uint(&mut self) -> Result<u64, WireError> {
        self.argument(head::UNSIGNED)
    }

    pub(super) fn int(&mut self) -> Result<i128, WireError> {
        let offset = self.offset();
        let (major, argument) = self.typed_argument()?;
        match major {
            head::UNSIGNED => Ok(i128::from(argument)),
            1 => Ok(-1 - i128::from(argument)),
            _ => Err(WireError::new(MalformedReason::WrongType, offset)),
        }
    }

    pub(super) fn finish(self) -> Result<(), WireError> {
        if self.tail.is_empty() {
            Ok(())
        } else {
            Err(WireError::new(MalformedReason::WrongLength, self.offset()))
        }
    }

    pub(super) fn array(&mut self) -> Result<u64, WireError> {
        self.argument(head::ARRAY)
    }

    pub(super) fn boolean(&mut self) -> Result<bool, WireError> {
        let offset = self.offset();
        if let [value @ (0xf4 | 0xf5), tail @ ..] = self.tail {
            self.tail = tail;
            return Ok(*value == 0xf5);
        }
        // The shortest-head rule precedes the type check for majors 0–4.
        self.typed_argument()?;
        Err(WireError::new(MalformedReason::WrongType, offset))
    }

    pub(super) fn record(&mut self, fields: u64) -> Result<(), WireError> {
        let offset = self.offset();
        if self.array()? != fields {
            return Err(WireError::new(MalformedReason::WrongLength, offset));
        }
        Ok(())
    }

    pub(super) fn fixed_uint(
        &mut self,
        value: u64,
        reason: MalformedReason,
    ) -> Result<(), WireError> {
        let offset = self.offset();
        if self.uint()? != value {
            return Err(WireError::new(reason, offset));
        }
        Ok(())
    }

    pub(super) fn positive(&mut self, reason: MalformedReason) -> Result<NonZeroU64, WireError> {
        let offset = self.offset();
        NonZeroU64::new(self.uint()?).ok_or_else(|| WireError::new(reason, offset))
    }

    pub(super) fn registry_value(
        &mut self,
        registry: NonZeroU64,
        known: u64,
    ) -> Result<NonZeroU64, WireError> {
        let offset = self.offset();
        let value = self.positive(MalformedReason::UnregisteredValue)?;
        if registry.get() <= REGISTRY_VERSION && value.get() != known {
            return Err(WireError::new(MalformedReason::UnregisteredValue, offset));
        }
        Ok(value)
    }

    pub(super) fn option<T>(
        &mut self,
        field: impl FnOnce(&mut Self) -> Result<T, WireError>,
    ) -> Result<Option<T>, WireError> {
        match self.option_shape()? {
            OptionShape::Absent => Ok(None),
            OptionShape::Present => field(self).map(Some),
            OptionShape::Unknown { tag_offset, .. } => Err(WireError::new(
                MalformedReason::UnregisteredValue,
                tag_offset,
            )),
        }
    }

    pub(super) fn option_shape(&mut self) -> Result<OptionShape, WireError> {
        let offset = self.offset();
        let length = self.array()?;
        let wrong_length = WireError::new(MalformedReason::WrongLength, offset);
        if length == 0 {
            return Err(wrong_length);
        }
        let tag_offset = self.offset();
        match self.uint()? {
            1 if length == 1 => Ok(OptionShape::Absent),
            2 if length == 2 => Ok(OptionShape::Present),
            1 | 2 => Err(wrong_length),
            code => Ok(OptionShape::Unknown {
                code,
                remaining: length - 1,
                tag_offset,
            }),
        }
    }

    pub(super) fn bytes(&mut self) -> Result<&'a [u8], WireError> {
        self.string(head::BYTES)
    }

    pub(super) fn count_and_digest(&mut self) -> Result<(u64, &'a [u8; DIGEST_LENGTH]), WireError> {
        self.record(2)?;
        Ok((self.uint()?, self.fixed_bytes()?))
    }

    pub(super) fn fixed_bytes<const LENGTH: usize>(
        &mut self,
    ) -> Result<&'a [u8; LENGTH], WireError> {
        let offset = self.offset();
        self.bytes()?
            .try_into()
            .map_err(|_| WireError::new(MalformedReason::WrongLength, offset))
    }

    pub(super) fn text(&mut self) -> Result<&'a str, WireError> {
        let offset = self.offset();
        checked_text(self.string(head::TEXT)?, offset)
    }

    fn string(&mut self, major: u8) -> Result<&'a [u8], WireError> {
        let offset = self.offset();
        let length = self.argument(major)?;
        let error = WireError::new(MalformedReason::WrongLength, offset);
        let length = usize::try_from(length).map_err(|_| error)?;
        let (bytes, tail) = self.tail.split_at_checked(length).ok_or(error)?;
        self.tail = tail;
        Ok(bytes)
    }

    /// Check one structurally framed value whose position declares no type.
    pub(super) fn closed_value(&mut self) -> Result<(), WireError> {
        self.closed_value_until(usize::MAX)
    }

    pub(super) fn closed_value_until(&mut self, limit: usize) -> Result<(), WireError> {
        // Arrays are the only containers, and all children have the same
        // grammar. Counting remaining values preserves lexical order
        // without allocating a stack. Each array consumes at least one byte
        // and contributes at most u64::MAX entries.
        let mut remaining = 1_u128;
        while remaining != 0 {
            remaining -= 1;
            let offset = self.offset();
            if offset >= limit {
                return Err(WireError::new(MalformedReason::OutOfRange, limit));
            }
            let [first, tail @ ..] = self.tail else {
                return Err(WireError::new(MalformedReason::WrongLength, offset));
            };
            match *first >> 5 {
                major @ (0 | 1) => {
                    self.argument(major)?;
                }
                major @ (head::BYTES | head::TEXT) => {
                    let bytes = self.string(major)?;
                    if major == head::TEXT {
                        checked_text(bytes, offset)?;
                    }
                    if self.offset() > limit {
                        return Err(WireError::new(MalformedReason::OutOfRange, limit));
                    }
                }
                head::ARRAY => {
                    remaining += u128::from(self.array()?);
                }
                _ if matches!(*first, 0xf4 | 0xf5) => self.tail = tail,
                _ => {
                    self.typed_argument()?;
                    return Err(WireError::new(MalformedReason::WrongType, offset));
                }
            }
        }
        if self.offset() > limit {
            return Err(WireError::new(MalformedReason::OutOfRange, limit));
        }
        Ok(())
    }
}

fn checked_text(bytes: &[u8], offset: usize) -> Result<&str, WireError> {
    core::str::from_utf8(bytes).map_err(|_| WireError::new(MalformedReason::WrongType, offset))
}
