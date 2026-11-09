// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Strict supplied-byte tar inventory, without transport decoding or path policy.
//!
//! The same framing and effective-metadata engine serves native tar import.
//! Paths retain their original raw bytes; UTF-8 conversion is explicit and never
//! lossy. USTAR prefix/name fragments retain separate source locations. Payloads
//! borrow the original uncompressed archive. No extraction or GTS serialization
//! occurs here. Caller limits cover source bytes, all physical headers, extension
//! bytes, high-water admitted layouts and work before growth or scanning.

mod core;
mod error;
pub(super) mod legacy;
#[cfg(test)]
mod tests;
pub use error::{Error, Limit, Operation, Problem};

use crate::files::FileEntryKind;
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use std::{
    cell::{Cell, RefCell},
    ops::Range,
};

/// An original half-open archive byte range, never a synthetic joined path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub(super) start: u64,
    pub(super) end: u64,
}
impl Span {
    /// Original starting byte offset.
    #[must_use]
    pub const fn start(self) -> u64 {
        self.start
    }
    /// Original exclusive end offset.
    #[must_use]
    pub const fn end(self) -> u64 {
        self.end
    }
    /// Original byte range.
    #[must_use]
    pub fn range(self) -> Range<u64> {
        self.start..self.end
    }
    fn slice(self, data: &[u8]) -> Result<&[u8], Error> {
        let start = usize::try_from(self.start).map_err(|_| Error::Truncated {
            required: self,
            available: data.len() as u64,
        })?;
        let end = usize::try_from(self.end).map_err(|_| Error::Truncated {
            required: self,
            available: data.len() as u64,
        })?;
        data.get(start..end).ok_or(Error::Truncated {
            required: self,
            available: data.len() as u64,
        })
    }
}
impl purrdf_hash::error::StructuredError for Span {
    purrdf_hash::error_visit! {
        @record self { start, end } => "gts-tar-span" {
            "start" => ByteCount(u128::from(self.start)), "end" => ByteCount(u128::from(self.end)),
        }
    }
}

/// Explicit allowances for one complete strict inventory computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Most supplied uncompressed bytes.
    pub source_bytes: u64,
    /// Most physical nonzero headers, including extension records.
    pub entries: usize,
    /// Most cumulative GNU/PAX extension body bytes.
    pub metadata_bytes: u64,
    /// Most current admitted layout total, including unreleased construction scratch.
    pub storage_bytes: usize,
    /// Most scanned bytes, metadata comparisons and native storage work.
    pub work: u64,
}

/// Actual logical and admitted resource counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Usage {
    /// Original supplied source length.
    pub source_bytes: u64,
    /// Physical nonzero headers processed, including extensions.
    pub entries: usize,
    /// GNU/PAX extension body bytes processed.
    pub metadata_bytes: u64,
    /// High-water admitted layout total, including unreleased construction scratch.
    /// This is not the physical allocator peak.
    pub storage_bytes: usize,
    /// Actual logical scan/comparison/native-storage work units.
    pub work: u64,
}

/// One exact original field and its original byte location.
#[derive(Debug, Clone, Copy)]
pub struct Field<'a> {
    bytes: &'a [u8],
    span: Span,
}
impl<'a> Field<'a> {
    /// Exact bytes, without replacement or path normalization.
    #[must_use]
    pub const fn bytes(self) -> &'a [u8] {
        self.bytes
    }
    /// Original field location.
    #[must_use]
    pub const fn span(self) -> Span {
        self.span
    }
    /// Decode the original field as UTF-8.
    ///
    /// # Errors
    /// Retains the original UTF-8 cause and field location on refusal.
    pub fn utf8(self) -> Result<&'a str, Error> {
        std::str::from_utf8(self.bytes).map_err(|cause| Error::Utf8 {
            span: self.span,
            cause,
        })
    }
}

/// Effective member name, with a separate USTAR prefix when one applies.
#[derive(Debug, Clone, Copy)]
pub struct Name<'a> {
    prefix: Option<Field<'a>>,
    value: Field<'a>,
}
impl<'a> Name<'a> {
    /// An original USTAR prefix. Its effective separator is one `/`.
    #[must_use]
    pub const fn prefix(self) -> Option<Field<'a>> {
        self.prefix
    }
    /// Original name, or the effective PAX/GNU replacement of the whole name.
    #[must_use]
    pub const fn value(self) -> Field<'a> {
        self.value
    }
    /// Validate the exact text fragments without allocating a joined path.
    ///
    /// # Errors
    /// Retains the original cause and separate offending fragment location.
    pub fn utf8_parts(self) -> Result<(Option<&'a str>, &'a str), Error> {
        Ok((
            self.prefix.map(Field::utf8).transpose()?,
            self.value.utf8()?,
        ))
    }
}

/// A selected PAX value with its original key/value/header evidence.
#[derive(Debug, Clone, Copy)]
pub struct Property<'a> {
    /// Exact original keyword.
    pub key: Field<'a>,
    /// Exact original value.
    pub value: Field<'a>,
    /// Original extension header governing this record.
    pub header: Span,
}

/// A physical header and its effective body range, including extension records.
#[derive(Debug, Clone, Copy)]
pub struct Record {
    /// Original complete header.
    pub header: Span,
    /// Effective body range; member sizes include selected PAX size.
    pub body: Span,
    /// Original header type flag.
    pub kind: u8,
}

#[derive(Debug)]
struct Pax {
    key: String,
    key_span: Span,
    value: Span,
    header: Span,
}
#[derive(Debug, Clone, Copy)]
struct NameSpans {
    prefix: Option<Span>,
    value: Span,
}
#[derive(Debug)]
struct Member {
    record: Record,
    path: NameSpans,
    link: Option<Span>,
    kind: FileEntryKind,
    mode: u32,
    uid: u64,
    gid: u64,
    header_mtime: u64,
    user: Option<Span>,
    group: Option<Span>,
    dev_major: u64,
    dev_minor: u64,
    effective: Vec<usize>,
}
#[derive(Debug, Default)]
struct Model {
    members: Vec<Member>,
    pax: Vec<Pax>,
    records: Vec<Record>,
}

/// Complete immutable archive inventory retaining the original source borrow.
#[derive(Debug)]
pub struct Inventory<'a> {
    data: &'a [u8],
    model: Model,
    usage: Usage,
}
impl<'a> Inventory<'a> {
    /// Every real member in original archive order; extensions remain in `records`.
    pub fn entries(&self) -> impl ExactSizeIterator<Item = Entry<'_, 'a>> {
        self.model.members.iter().map(|member| Entry {
            inventory: self,
            member,
        })
    }
    /// Every physical nonzero header, including all GNU/PAX metadata.
    #[must_use]
    pub fn records(&self) -> &[Record] {
        &self.model.records
    }
    /// Original complete uncompressed source bytes.
    #[must_use]
    pub const fn source(&self) -> &'a [u8] {
        self.data
    }
    /// Actual counters for this admitted computation.
    #[must_use]
    pub const fn usage(&self) -> Usage {
        self.usage
    }
    fn field(&self, span: Span) -> Result<Field<'a>, Error> {
        Ok(Field {
            bytes: span.slice(self.data)?,
            span,
        })
    }
}

/// A borrowed real-member view; it owns no payload copy or synthetic source span.
#[derive(Debug, Clone, Copy)]
pub struct Entry<'i, 'a> {
    inventory: &'i Inventory<'a>,
    member: &'i Member,
}
impl<'a> Entry<'_, 'a> {
    /// Original complete header and effective payload ranges.
    #[must_use]
    pub const fn record(self) -> Record {
        self.member.record
    }
    /// Native member kind, including links and supported special entries.
    #[must_use]
    pub const fn kind(self) -> FileEntryKind {
        self.member.kind
    }
    /// Effective numeric user identity.
    #[must_use]
    pub const fn uid(self) -> u64 {
        self.member.uid
    }
    /// Effective numeric group identity.
    #[must_use]
    pub const fn gid(self) -> u64 {
        self.member.gid
    }
    /// Original admitted numeric permission mode.
    #[must_use]
    pub const fn mode(self) -> u32 {
        self.member.mode
    }
    /// Effective device major/minor identities.
    #[must_use]
    pub const fn device(self) -> (u64, u64) {
        (self.member.dev_major, self.member.dev_minor)
    }
    /// Effective path fragments, without `./` or trailing-separator normalization.
    ///
    /// # Errors
    /// Refuses an inconsistent source range rather than fabricating bytes.
    pub fn path(self) -> Result<Name<'a>, Error> {
        Ok(Name {
            prefix: self
                .member
                .path
                .prefix
                .map(|span| self.inventory.field(span))
                .transpose()?,
            value: self.inventory.field(self.member.path.value)?,
        })
    }
    /// Effective exact link target, when present.
    ///
    /// # Errors
    /// Refuses an inconsistent original range.
    pub fn link_target(self) -> Result<Option<Field<'a>>, Error> {
        self.member
            .link
            .map(|span| self.inventory.field(span))
            .transpose()
    }
    /// Exact effective payload for every member kind, borrowed from source.
    ///
    /// # Errors
    /// Refuses an inconsistent original range.
    pub fn payload(self) -> Result<&'a [u8], Error> {
        self.member.record.body.slice(self.inventory.data)
    }
    /// Effective PAX values, including core keywords, with exact original evidence.
    pub fn properties(self) -> impl ExactSizeIterator<Item = Result<Property<'a>, Error>> {
        self.member.effective.iter().map(|index| {
            let field = &self.inventory.model.pax[*index];
            Ok(Property {
                key: self.inventory.field(field.key_span)?,
                value: self.inventory.field(field.value)?,
                header: field.header,
            })
        })
    }
}

/// Inventory supplied uncompressed tar bytes using explicit original admission.
///
/// The result never normalizes paths or silently skips unsupported constructs.
/// Selected numeric overrides are validated before use; selected PAX size controls
/// framing. Global and next-member metadata retain separate state. There is no
/// transport sniff, decompression, extraction or legacy retry.
///
/// # Errors
/// Returns typed original framing/metadata/resource failures before partial output
/// escapes. Invalid path UTF-8 remains exact bytes until explicit `Field::utf8`.
pub fn read<'a, A: Admission + ?Sized>(
    data: &'a [u8],
    limits: Limits,
    admission: &mut A,
) -> Result<Inventory<'a>, Error> {
    let meter = Meter::new(limits, data.len() as u64)?;
    let mut account = Account {
        original: admission,
        meter: &meter,
    };
    let mut memory = Memory::new(&mut account);
    let model = core::parse(
        &mut core::Slice(data),
        core::FramingPolicy::Strict,
        &meter,
        &mut memory,
    );
    match model {
        Ok(model) => Ok(Inventory {
            data,
            model,
            usage: meter.usage(),
        }),
        Err(error) => Err(meter.failure.borrow_mut().take().unwrap_or(error)),
    }
}

struct Meter {
    limits: Limits,
    source: u64,
    entries: Cell<usize>,
    metadata: Cell<u64>,
    storage: Cell<usize>,
    work: Cell<u64>,
    offset: Cell<u64>,
    failure: RefCell<Option<Error>>,
}
impl Meter {
    fn new(limits: Limits, source: u64) -> Result<Self, Error> {
        check(
            Limit::Source,
            u128::from(source),
            u128::from(limits.source_bytes),
            0,
        )?;
        Ok(Self {
            limits,
            source,
            entries: Cell::new(0),
            metadata: Cell::new(0),
            storage: Cell::new(0),
            work: Cell::new(0),
            offset: Cell::new(0),
            failure: RefCell::new(None),
        })
    }
    fn work(&self, units: u64) -> Result<(), Error> {
        let attempted = u128::from(self.work.get()) + u128::from(units);
        check(
            Limit::Work,
            attempted,
            u128::from(self.limits.work),
            self.offset.get(),
        )?;
        self.work.set(self.work.get() + units);
        Ok(())
    }
    fn entry(&self) -> Result<(), Error> {
        let attempted = self.entries.get() as u128 + 1;
        check(
            Limit::Entries,
            attempted,
            self.limits.entries as u128,
            self.offset.get(),
        )?;
        self.entries.set(self.entries.get() + 1);
        Ok(())
    }
    fn metadata(&self, bytes: u64) -> Result<(), Error> {
        let attempted = u128::from(self.metadata.get()) + u128::from(bytes);
        check(
            Limit::Metadata,
            attempted,
            u128::from(self.limits.metadata_bytes),
            self.offset.get(),
        )?;
        self.metadata.set(self.metadata.get() + bytes);
        Ok(())
    }
    fn usage(&self) -> Usage {
        Usage {
            source_bytes: self.source,
            entries: self.entries.get(),
            metadata_bytes: self.metadata.get(),
            storage_bytes: self.storage.get(),
            work: self.work.get(),
        }
    }
}
fn check(kind: Limit, attempted: u128, maximum: u128, offset: u64) -> Result<(), Error> {
    if attempted > maximum {
        Err(Error::Limit {
            kind,
            attempted,
            maximum,
            offset,
        })
    } else {
        Ok(())
    }
}
struct Account<'a, A: Admission + ?Sized> {
    original: &'a mut A,
    meter: &'a Meter,
}
impl<A: Admission + ?Sized> Admission for Account<'_, A> {
    fn progress_enabled(&self) -> bool {
        true
    }
    fn checkpoint(&mut self, units: u64) -> Result<(), StorageError> {
        if let Err(error) = self.meter.work(units) {
            *self.meter.failure.borrow_mut() = Some(error);
            return Err(StorageError::AdmissionFailed);
        }
        self.original.checkpoint(units)
    }
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if let Err(error) = check(
            Limit::Storage,
            bytes as u128,
            self.meter.limits.storage_bytes as u128,
            self.meter.offset.get(),
        ) {
            *self.meter.failure.borrow_mut() = Some(error);
            return Err(StorageError::AdmissionFailed);
        }
        self.original.resize(bytes)?;
        self.meter.storage.set(self.meter.storage.get().max(bytes));
        Ok(())
    }
}
fn storage(meter: &Meter, cause: StorageError) -> Error {
    meter.failure.borrow_mut().take().unwrap_or(Error::Storage {
        offset: meter.offset.get(),
        cause,
    })
}
