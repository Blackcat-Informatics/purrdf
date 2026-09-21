// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A minimal length-framed writer/reader for the plan's canonical encoding.
//!
//! The encoding is deliberately its own format rather than a serde format: a
//! plan's identity must be a pure function of the plan's fields, independent of
//! any serializer's version, formatting choices, or map iteration order. Every
//! variable-length field is framed by an eight-byte little-endian length, so the
//! encoding is self-delimiting and map entries can be sorted without ambiguity.
//! All integers are little-endian, so the bytes are identical on every target.

use crate::error::PlanError;

/// Append-only canonical byte writer.
pub(crate) struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    /// A writer with no bytes.
    pub(crate) fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Consume the writer and return its bytes.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    /// Write one byte.
    pub(crate) fn u8(&mut self, value: u8) {
        self.buf.push(value);
    }

    /// Write a little-endian `u16`.
    pub(crate) fn u16(&mut self, value: u16) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a little-endian `u32`.
    pub(crate) fn u32(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a little-endian `u64`.
    pub(crate) fn u64(&mut self, value: u64) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a little-endian `i128`.
    pub(crate) fn i128(&mut self, value: i128) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Write an `f32` as its exact bit pattern.
    pub(crate) fn f32_bits(&mut self, value: f32) {
        self.u32(value.to_bits());
    }

    /// Write a length-framed byte string.
    pub(crate) fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.buf.extend_from_slice(value);
    }

    /// Write a length-framed UTF-8 string.
    pub(crate) fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    /// Write a present/absent discriminant and, when present, a framed string.
    pub(crate) fn option_string(&mut self, value: Option<&str>) {
        match value {
            None => self.u8(0),
            Some(value) => {
                self.u8(1);
                self.string(value);
            }
        }
    }

    /// Write a present/absent discriminant and, when present, a little-endian
    /// `u64`.
    ///
    /// Absence is a tag rather than a sentinel value because every `u64` is a
    /// legal measurement: zero is "the provider measured none", and a sentinel
    /// would make one measurement unrepresentable in order to spell the absence
    /// of all of them. The discriminant is the same one
    /// [`Self::option_string`] writes, so the encoding spells "present" one way
    /// throughout.
    pub(crate) fn option_u64(&mut self, value: Option<u64>) {
        match value {
            None => self.u8(0),
            Some(value) => {
                self.u8(1);
                self.u64(value);
            }
        }
    }

    /// Write a present/absent discriminant and, when present, a little-endian
    /// `i128`.
    ///
    /// The discriminant is the one [`Self::option_string`] and
    /// [`Self::option_u64`] write, so the encoding spells "present" one way
    /// throughout — which is a property of this module only for as long as every
    /// optional field is written through it.
    pub(crate) fn option_i128(&mut self, value: Option<i128>) {
        match value {
            None => self.u8(0),
            Some(value) => {
                self.u8(1);
                self.i128(value);
            }
        }
    }

    /// Write a length-framed run of little-endian `u32`s.
    pub(crate) fn u32_slice(&mut self, values: &[u32]) {
        self.u64(values.len() as u64);
        for value in values {
            self.u32(*value);
        }
    }
}

/// A cursor over canonical bytes.
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    /// A reader over `bytes`.
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    /// The number of bytes not yet consumed.
    pub(crate) fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    /// Take exactly `len` bytes.
    fn take(&mut self, len: usize) -> Result<&'a [u8], PlanError> {
        let end = self.offset.checked_add(len).ok_or(PlanError::Truncated {
            offset: self.offset,
        })?;
        if end > self.bytes.len() {
            return Err(PlanError::Truncated {
                offset: self.offset,
            });
        }
        let slice = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(slice)
    }

    /// Read one byte.
    pub(crate) fn u8(&mut self) -> Result<u8, PlanError> {
        Ok(self.take(1)?[0])
    }

    /// Read a little-endian `u16`.
    pub(crate) fn u16(&mut self) -> Result<u16, PlanError> {
        let mut raw = [0u8; 2];
        raw.copy_from_slice(self.take(2)?);
        Ok(u16::from_le_bytes(raw))
    }

    /// Read a little-endian `u32`.
    pub(crate) fn u32(&mut self) -> Result<u32, PlanError> {
        let mut raw = [0u8; 4];
        raw.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(raw))
    }

    /// Read a little-endian `u64`.
    pub(crate) fn u64(&mut self) -> Result<u64, PlanError> {
        let mut raw = [0u8; 8];
        raw.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(raw))
    }

    /// Read a little-endian `i128`.
    pub(crate) fn i128(&mut self) -> Result<i128, PlanError> {
        let mut raw = [0u8; 16];
        raw.copy_from_slice(self.take(16)?);
        Ok(i128::from_le_bytes(raw))
    }

    /// Read an `f32` from its exact bit pattern.
    pub(crate) fn f32_bits(&mut self) -> Result<f32, PlanError> {
        Ok(f32::from_bits(self.u32()?))
    }

    /// Read a length-framed byte string.
    pub(crate) fn bytes(&mut self) -> Result<&'a [u8], PlanError> {
        let len = self.u64()?;
        let len = usize::try_from(len).map_err(|_| PlanError::Truncated {
            offset: self.offset,
        })?;
        self.take(len)
    }

    /// Read a length-framed UTF-8 string.
    pub(crate) fn string(&mut self, what: &'static str) -> Result<String, PlanError> {
        let bytes = self.bytes()?;
        core::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| PlanError::InvalidUtf8 { what })
    }

    /// Read a present/absent discriminant and, when present, a framed string.
    pub(crate) fn option_string(
        &mut self,
        what: &'static str,
    ) -> Result<Option<String>, PlanError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.string(what)?)),
            tag => Err(PlanError::InvalidTag { what, tag }),
        }
    }

    /// Read a present/absent discriminant and, when present, a `u64`.
    ///
    /// An absent value stays absent. "The provider measured nothing" and "the
    /// provider measured zero" are different facts, and a decoder that read the
    /// first as the second would turn silence into a claim.
    pub(crate) fn option_u64(&mut self, what: &'static str) -> Result<Option<u64>, PlanError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.u64()?)),
            tag => Err(PlanError::InvalidTag { what, tag }),
        }
    }

    /// Read a present/absent discriminant and, when present, an `i128`.
    ///
    /// A discriminant that is neither is a typed refusal rather than a
    /// treated-as-absent field: the optional values written this way are the
    /// endpoints of a constrained request term, and reading a constrained one as
    /// unconstrained would widen the query the plan describes.
    pub(crate) fn option_i128(&mut self, what: &'static str) -> Result<Option<i128>, PlanError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.i128()?)),
            tag => Err(PlanError::InvalidTag { what, tag }),
        }
    }

    /// Read a length-framed run of little-endian `u32`s.
    ///
    /// The declared count is capped before it is used to reserve, so a hostile
    /// encoding cannot make a short document allocate a long vector; the loop
    /// still reads every element the count names and runs out of bytes by name
    /// if the document lied.
    pub(crate) fn u32_slice(&mut self) -> Result<Vec<u32>, PlanError> {
        let count = self.count()?;
        let mut values = Vec::with_capacity(count.min(1024));
        for _ in 0..count {
            values.push(self.u32()?);
        }
        Ok(values)
    }

    /// Read a length prefix and return it as a `usize`.
    pub(crate) fn count(&mut self) -> Result<usize, PlanError> {
        let len = self.u64()?;
        usize::try_from(len).map_err(|_| PlanError::Truncated {
            offset: self.offset,
        })
    }

    /// Refuse an encoding that was not fully consumed.
    pub(crate) fn finish(&self) -> Result<(), PlanError> {
        if self.remaining() == 0 {
            Ok(())
        } else {
            Err(PlanError::TrailingBytes {
                extra: self.remaining(),
            })
        }
    }
}
