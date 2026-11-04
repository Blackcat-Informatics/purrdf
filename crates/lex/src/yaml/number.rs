// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fallible numeric spelling preparation under one construction-wide budget.

use super::{
    admission::Allocations,
    read::{ErrorKind, Limits},
    resolve::{NumberText, Plain, Radix, resolve},
};
use crate::allocation::{Admission, Memory, StorageError};
use core::{fmt, num::NonZeroU64};

const GROUP_DIGITS: usize = 19;
// The largest decimal power representable by one unsigned word is positive.
const GROUP_BASE: NonZeroU64 =
    NonZeroU64::new(10_000_000_000_000_000_000).expect("positive decimal group base");

pub(super) struct Budget {
    work: u64,
    bytes: usize,
    limits: Limits,
}
impl Budget {
    pub(super) const fn new(limits: Limits) -> Self {
        Self {
            work: limits.max_numeric_work,
            bytes: limits.max_numeric_bytes,
            limits,
        }
    }
    fn charge(&mut self, units: usize) -> Result<(), ErrorKind> {
        let units = u64::try_from(units).map_err(|_| StorageError::SizeOverflow)?;
        self.work = self.work.checked_sub(units).ok_or(ErrorKind::NumericWork {
            limit: self.limits.max_numeric_work,
        })?;
        Ok(())
    }
    fn output(&mut self, length: usize) -> Result<(), ErrorKind> {
        self.bytes = self
            .bytes
            .checked_sub(length)
            .ok_or(ErrorKind::NumericOutput {
                limit: self.limits.max_numeric_bytes,
            })?;
        // Writing the prepared spelling and validating its JSON number grammar
        // are two linear passes. Charge both before either pass begins.
        self.charge(length.checked_mul(2).ok_or(StorageError::SizeOverflow)?)
    }
    pub(super) fn classify<'a>(&mut self, text: &'a str) -> Result<Plain<'a>, ErrorKind> {
        self.charge(text.len())?;
        Ok(resolve(text))
    }
    pub(super) fn materialize(
        &mut self,
        plan: NumberText<'_>,
        allocations: &mut Allocations,
    ) -> Result<String, ErrorKind> {
        allocations.try_memory(|memory| {
            memory.try_scope(|memory| match plan {
                NumberText::Radix { radix, digits } => self.radix(digits, radix, memory),
                NumberText::Decimal { negative, digits } => {
                    self.parts(&[if negative { "-" } else { "" }, digits], memory)
                }
                NumberText::Float {
                    negative,
                    integer,
                    fraction,
                    exponent,
                } => self.parts(
                    &[
                        if negative { "-" } else { "" },
                        if integer.is_empty() { "0" } else { integer },
                        if fraction.is_some() { "." } else { "" },
                        fraction.map_or("", |part| if part.is_empty() { "0" } else { part }),
                        exponent,
                    ],
                    memory,
                ),
            })
        })
    }
    fn parts<S: Admission + ?Sized>(
        &mut self,
        parts: &[&str],
        memory: &mut Memory<'_, S>,
    ) -> Result<String, ErrorKind> {
        let length = parts.iter().try_fold(0_usize, |length, part| {
            length
                .checked_add(part.len())
                .ok_or(StorageError::SizeOverflow)
        })?;
        self.output(length)?;
        let mut output = String::new();
        memory.reserve_string(&mut output, length)?;
        for part in parts {
            memory.push_str(&mut output, part)?;
        }
        Ok(output)
    }
    fn radix<S: Admission + ?Sized>(
        &mut self,
        digits: &str,
        radix: Radix,
        memory: &mut Memory<'_, S>,
    ) -> Result<String, ErrorKind> {
        self.charge(digits.len())?;
        let digits = digits.trim_start_matches('0');
        if digits.is_empty() {
            return self.parts(&["0"], memory);
        }
        let bits = digits
            .len()
            .checked_mul(radix.bits())
            .ok_or(StorageError::SizeOverflow)?;
        let count = bits.div_ceil(64);
        let mut words = Vec::new();
        memory.reserve(&mut words, count)?;
        words.resize(count, 0_u64);
        self.charge(digits.len())?;
        for (index, byte) in digits.bytes().rev().enumerate() {
            let value = u64::from(
                radix
                    .digit(byte)
                    .ok_or(ErrorKind::Syntax("classified radix digits"))?,
            );
            // The complete bit length was checked before reserving this buffer.
            let bit = index * radix.bits();
            let shift = bit % 64;
            words[bit / 64] |= value << shift;
            if shift + radix.bits() > 64 {
                words[bit / 64 + 1] |= value >> (64 - shift);
            }
        }
        let mut length = count;
        let mut groups = Vec::new();
        while length != 0 {
            self.charge(length)?;
            let (next, remainder) =
                crate::integer_text::div_rem_word(&mut words[..length], GROUP_BASE);
            memory.push(&mut groups, remainder)?;
            length = next;
        }
        let leading = groups.last().copied().ok_or(StorageError::SizeOverflow)?;
        let leading_digits =
            usize::try_from(leading.ilog10()).map_err(|_| StorageError::SizeOverflow)? + 1;
        let bytes = (groups.len() - 1)
            .checked_mul(GROUP_DIGITS)
            .and_then(|bytes| bytes.checked_add(leading_digits))
            .ok_or(StorageError::SizeOverflow)?;
        self.output(bytes)?;
        let output = memory.format(&Groups(&groups))?;
        memory.release_vec(groups)?;
        memory.release_vec(words)?;
        Ok(output)
    }
}

/// Prepared groups only; formatting performs no allocation or radix conversion.
struct Groups<'a>(&'a [u64]);
impl fmt::Display for Groups<'_> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, group) in self.0.iter().rev().enumerate() {
            if index == 0 {
                write!(out, "{group}")?;
            } else {
                write!(out, "{group:019}")?;
            }
        }
        Ok(())
    }
}
