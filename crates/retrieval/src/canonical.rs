// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A minimal length-framed writer/reader for the plan's canonical encoding.
//!
//! The encoding is deliberately its own format rather than a document format: a
//! plan's identity must be a pure function of the plan's fields, independent of
//! any serializer's version, formatting choices, or map iteration order. Every
//! variable-length field is framed by an eight-byte little-endian length, so the
//! encoding is self-delimiting and map entries can be sorted without ambiguity.
//! All integers are little-endian, so the bytes are identical on every target.

use purrdf_hash::frame::{frame_le, frame_le_into};

use crate::error::PlanError;

/// Append-only canonical byte writer.
pub(crate) struct Writer<B = Vec<u8>> {
    buf: B,
}

/// Destination of the one canonical encoder: retained bytes or their digest.
pub(crate) trait CanonicalSink {
    fn append(&mut self, bytes: &[u8]);
    fn frame(&mut self, bytes: &[u8]);
}
impl CanonicalSink for Vec<u8> {
    fn append(&mut self, bytes: &[u8]) {
        self.extend_from_slice(bytes);
    }
    fn frame(&mut self, bytes: &[u8]) {
        frame_le(self, bytes);
    }
}
impl CanonicalSink for &mut purrdf_hash::blake3::RecordHasher {
    fn append(&mut self, bytes: &[u8]) {
        self.update(bytes);
    }
    fn frame(&mut self, bytes: &[u8]) {
        frame_le_into(*self, bytes);
    }
}

/// Native export storage latches its first refusal before further writes.
pub(crate) struct AdmittedBytes {
    bytes: purrdf_sparql_eval::AdmittedVec<u8>,
    workspace: purrdf_sparql_eval::WorkspaceCapability,
    failure: Option<purrdf_sparql_eval::EvalError>,
}
impl AdmittedBytes {
    fn append_checked(&mut self, bytes: &[u8]) -> Result<(), purrdf_sparql_eval::EvalError> {
        for byte in bytes {
            self.bytes.push(*byte)?;
        }
        Ok(())
    }
}
impl CanonicalSink for AdmittedBytes {
    fn append(&mut self, bytes: &[u8]) {
        if self.failure.is_none() {
            self.failure = self.append_checked(bytes).err();
        }
    }
    fn frame(&mut self, bytes: &[u8]) {
        if self.failure.is_none() {
            self.failure =
                purrdf_hash::frame::try_frame_le_with(bytes, |part| self.append_checked(part))
                    .err();
        }
    }
}

impl Writer<AdmittedBytes> {
    /// Preserve a prior write refusal before a caller admits nested output.
    pub(crate) fn ensure_live(&self) -> Result<(), purrdf_sparql_eval::EvalError> {
        match &self.buf.failure {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    pub(crate) fn admitted(workspace: &purrdf_sparql_eval::WorkspaceCapability) -> Self {
        Self {
            buf: AdmittedBytes {
                bytes: purrdf_sparql_eval::AdmittedVec::new(workspace),
                workspace: workspace.clone(),
                failure: None,
            },
        }
    }
    pub(crate) fn into_admitted(
        self,
    ) -> Result<
        purrdf_sparql_eval::SharedWorkspace<purrdf_sparql_eval::AdmittedVec<u8>>,
        purrdf_sparql_eval::EvalError,
    > {
        if let Some(error) = self.buf.failure {
            return Err(error);
        }
        purrdf_sparql_eval::SharedWorkspace::new_admitted(self.buf.bytes, &self.buf.workspace)
    }
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
}

impl<'a> Writer<&'a mut purrdf_hash::blake3::RecordHasher> {
    /// Write the original canonical spelling straight into the original digest.
    pub(crate) fn digest(hasher: &'a mut purrdf_hash::blake3::RecordHasher) -> Self {
        Self { buf: hasher }
    }
}

impl<B: CanonicalSink> Writer<B> {
    /// Write one byte.
    pub(crate) fn u8(&mut self, value: u8) {
        self.buf.append(&[value]);
    }

    /// Write a little-endian `u16`.
    pub(crate) fn u16(&mut self, value: u16) {
        self.buf.append(&value.to_le_bytes());
    }

    /// Write a little-endian `u32`.
    pub(crate) fn u32(&mut self, value: u32) {
        self.buf.append(&value.to_le_bytes());
    }

    /// Write a little-endian `u64`.
    pub(crate) fn u64(&mut self, value: u64) {
        self.buf.append(&value.to_le_bytes());
    }

    /// Write a little-endian `i128`.
    pub(crate) fn i128(&mut self, value: i128) {
        self.buf.append(&value.to_le_bytes());
    }

    /// Write an unsigned score count in little-endian order.
    pub(crate) fn u128(&mut self, value: u128) {
        self.buf.append(&value.to_le_bytes());
    }

    /// Write an `f32` as its exact bit pattern.
    pub(crate) fn f32_bits(&mut self, value: f32) {
        self.u32(value.to_bits());
    }

    /// Write a length-framed byte string.
    pub(crate) fn bytes(&mut self, value: &[u8]) {
        self.buf.frame(value);
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

    /// Take the next `N` bytes as an array.
    fn array<const N: usize>(&mut self) -> Result<[u8; N], PlanError> {
        let array = self
            .bytes
            .get(self.offset..)
            .and_then(<[u8]>::first_chunk)
            .copied()
            .ok_or(PlanError::Truncated {
                offset: self.offset,
            })?;
        self.offset += N;
        Ok(array)
    }

    /// Read one byte.
    pub(crate) fn u8(&mut self) -> Result<u8, PlanError> {
        Ok(self.take(1)?[0])
    }

    /// Read a little-endian `u16`.
    pub(crate) fn u16(&mut self) -> Result<u16, PlanError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    /// Read a little-endian `u32`.
    pub(crate) fn u32(&mut self) -> Result<u32, PlanError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    /// Read a little-endian `u64`.
    pub(crate) fn u64(&mut self) -> Result<u64, PlanError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    /// Read a little-endian `i128`.
    pub(crate) fn i128(&mut self) -> Result<i128, PlanError> {
        Ok(i128::from_le_bytes(self.array()?))
    }

    /// Read an unsigned score count.
    pub(crate) fn u128(&mut self) -> Result<u128, PlanError> {
        Ok(u128::from_le_bytes(self.array()?))
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
            .map_err(|source| PlanError::InvalidUtf8 { what, source })
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
