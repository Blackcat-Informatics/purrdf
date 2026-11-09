// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One physical framing and effective metadata engine for slice and seek callers.

use super::{
    Error, Member, Meter, Model, NameSpans, Operation, Pax, Problem, Record, Span, storage,
};
use crate::files::FileEntryKind;
use purrdf_lex::allocation::{Admission, Memory};
use std::{
    borrow::Cow,
    io::{Read, Seek, SeekFrom},
};

pub(super) trait Source {
    fn len(&self) -> u64;
    fn read<'a, A: Admission + ?Sized>(
        &'a mut self,
        span: Span,
        operation: Operation,
        meter: &Meter,
        memory: &mut Memory<'_, A>,
    ) -> Result<Cow<'a, [u8]>, Error>;
}
pub(super) struct Slice<'a>(pub(super) &'a [u8]);
impl Source for Slice<'_> {
    fn len(&self) -> u64 {
        self.0.len() as u64
    }
    fn read<'a, A: Admission + ?Sized>(
        &'a mut self,
        span: Span,
        _operation: Operation,
        meter: &Meter,
        _memory: &mut Memory<'_, A>,
    ) -> Result<Cow<'a, [u8]>, Error> {
        meter.work(span.end - span.start)?;
        span.slice(self.0).map(Cow::Borrowed)
    }
}
pub(super) struct Seeking<'a, R> {
    pub(super) reader: &'a mut R,
    pub(super) length: u64,
    pub(super) buffer: Vec<u8>,
    pub(super) cached: Option<Span>,
}
impl<R: Read + Seek> Source for Seeking<'_, R> {
    fn len(&self) -> u64 {
        self.length
    }
    fn read<'a, A: Admission + ?Sized>(
        &'a mut self,
        span: Span,
        operation: Operation,
        meter: &Meter,
        memory: &mut Memory<'_, A>,
    ) -> Result<Cow<'a, [u8]>, Error> {
        meter.work(span.end - span.start)?;
        if let Some(cached) = self.cached
            && span.start >= cached.start
            && span.end <= cached.end
        {
            let relative = Span {
                start: span.start - cached.start,
                end: span.end - cached.start,
            };
            return relative.slice(&self.buffer).map(Cow::Borrowed);
        }
        let count = usize::try_from(span.end - span.start).map_err(|_| Error::Truncated {
            required: span,
            available: self.length,
        })?;
        memory
            .reserve(&mut self.buffer, count)
            .map_err(|cause| storage(meter, cause))?;
        self.buffer.resize(count, 0);
        self.cached = None;
        self.reader
            .seek(SeekFrom::Start(span.start))
            .map_err(|cause| Error::Io {
                operation: Operation::Seek,
                offset: span.start,
                cause: cause.into(),
            })?;
        self.reader
            .read_exact(&mut self.buffer)
            .map_err(|cause| Error::Io {
                operation,
                offset: span.start,
                cause: cause.into(),
            })?;
        self.cached = Some(span);
        Ok(Cow::Borrowed(&self.buffer))
    }
}

struct Header {
    record: Record,
    size: u64,
    name: NameSpans,
    link: Option<Span>,
    user: Option<Span>,
    group: Option<Span>,
    mode: u32,
    uid: u64,
    gid: u64,
    mtime: u64,
    dev_major: u64,
    dev_minor: u64,
}
#[derive(Default)]
struct State {
    global: Vec<usize>,
    local: Vec<usize>,
    long_name: Option<Span>,
    long_link: Option<Span>,
    pending: bool,
}

pub(super) fn parse<A: Admission + ?Sized>(
    source: &mut impl Source,
    meter: &Meter,
    memory: &mut Memory<'_, A>,
) -> Result<Model, Error> {
    let mut model = Model::default();
    let mut state = State::default();
    let mut offset = 0;
    loop {
        meter.offset.set(offset);
        let header_span = bounded(offset, 512, source.len())?;
        meter.work(2048)?;
        let bytes = source.read(header_span, Operation::Header, meter, memory)?;
        if bytes.iter().all(|byte| *byte == 0) {
            drop(bytes);
            finish(source, header_span.end, &state, &model, meter, memory)?;
            return Ok(model);
        }
        meter.entry()?;
        let mut header = header(&bytes, header_span)?;
        drop(bytes);
        let metadata = matches!(header.record.kind, b'x' | b'g' | b'L' | b'K');
        let effective = if metadata {
            Vec::new()
        } else {
            effective(&state, &model, meter, memory)?
        };
        let mut size = header.size;
        if !metadata {
            for index in &effective {
                validate_override(source, &model.pax[*index], meter, memory)?;
            }
            if let Some(field) = selected(&model, &effective, "size", meter)? {
                let bytes = source.read(field.value, Operation::Field, meter, memory)?;
                size = decimal(&bytes, field.value)?;
            }
        }
        header.record.body = bounded(header_span.end, size, source.len())?;
        // The effective size is established before both body slicing and the
        // next-header offset; extension headers retain their own physical size.
        let pad = (512 - (header.record.body.end - header.record.body.start) % 512) % 512;
        let padding = bounded(header.record.body.end, pad, source.len())?;
        meter.work(pad)?;
        if source
            .read(padding, Operation::Padding, meter, memory)?
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(Error::Invalid {
                span: padding,
                problem: Problem::Padding,
            });
        }
        offset = padding.end;
        memory
            .push(&mut model.records, header.record)
            .map_err(|cause| storage(meter, cause))?;
        if metadata {
            extension(source, &mut model, &mut state, &header, meter, memory)?;
        } else {
            let member = member(
                source, &model, &mut state, &header, effective, meter, memory,
            )?;
            memory
                .push(&mut model.members, member)
                .map_err(|cause| storage(meter, cause))?;
        }
    }
}

fn finish<A: Admission + ?Sized>(
    source: &mut impl Source,
    offset: u64,
    state: &State,
    model: &Model,
    meter: &Meter,
    memory: &mut Memory<'_, A>,
) -> Result<(), Error> {
    let second = bounded(offset, 512, source.len())?;
    meter.work(512)?;
    if source
        .read(second, Operation::Terminator, meter, memory)?
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(Error::Invalid {
            span: second,
            problem: Problem::Trailing,
        });
    }
    let mut offset = second.end;
    while offset < source.len() {
        let span = bounded(offset, (source.len() - offset).min(4096), source.len())?;
        meter.work(span.end - span.start)?;
        if source
            .read(span, Operation::Terminator, meter, memory)?
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(Error::Invalid {
                span,
                problem: Problem::Trailing,
            });
        }
        offset = span.end;
    }
    if state.pending {
        return Err(Error::Invalid {
            span: second,
            problem: Problem::PendingMetadata,
        });
    }
    if model.members.is_empty() {
        return Err(Error::Invalid {
            span: second,
            problem: Problem::Empty,
        });
    }
    Ok(())
}

fn header(bytes: &[u8], span: Span) -> Result<Header, Error> {
    let number = |range: std::ops::Range<usize>| {
        let field = Span {
            start: span.start + range.start as u64,
            end: span.start + range.end as u64,
        };
        super::super::parse_octal(&bytes[range]).ok_or(Error::Invalid {
            span: field,
            problem: Problem::Octal,
        })
    };
    let expected = number(148..156)?;
    let actual: u64 = bytes
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            u64::from(if (148..156).contains(&index) {
                b' '
            } else {
                *byte
            })
        })
        .sum();
    if actual != expected {
        return Err(Error::Checksum {
            offset: span.start,
            actual,
            expected,
        });
    }
    let ustar = &bytes[257..263] == b"ustar\0" && &bytes[263..265] == b"00";
    let gnu = &bytes[257..263] == b"ustar " && matches!(&bytes[263..265], b" \0" | b"00");
    let v7 = bytes[257..265].iter().all(|byte| *byte == 0);
    if !(ustar || gnu || v7) {
        return Err(Error::Invalid {
            span: Span {
                start: span.start + 257,
                end: span.start + 265,
            },
            problem: Problem::Format,
        });
    }
    let field = |range: std::ops::Range<usize>| {
        let start = range.start;
        let length = range.end - start;
        let used = bytes[range]
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(length);
        Span {
            start: span.start + start as u64,
            end: span.start + (start + used) as u64,
        }
    };
    let optional = |range: std::ops::Range<usize>| {
        let value = field(range);
        (value.end > value.start).then_some(value)
    };
    let name = NameSpans {
        prefix: if ustar { optional(345..500) } else { None },
        value: field(0..100),
    };
    let size = number(124..136)?;
    let body = Span {
        start: span.end,
        end: span.end,
    };
    let mode = u32::try_from(number(100..108)?).map_err(|_| Error::Invalid {
        span,
        problem: Problem::Octal,
    })?;
    Ok(Header {
        record: Record {
            header: span,
            body,
            kind: bytes[156],
        },
        size,
        name,
        link: optional(157..257),
        user: optional(265..297),
        group: optional(297..329),
        mode,
        uid: number(108..116)?,
        gid: number(116..124)?,
        mtime: number(136..148)?,
        dev_major: number(329..337)?,
        dev_minor: number(337..345)?,
    })
}

fn bounded(start: u64, length: u64, available: u64) -> Result<Span, Error> {
    let end = start.checked_add(length).ok_or(Error::Limit {
        kind: super::Limit::Source,
        attempted: u128::from(start) + u128::from(length),
        maximum: u128::from(u64::MAX),
        offset: start,
    })?;
    let span = Span { start, end };
    if end > available {
        Err(Error::Truncated {
            required: span,
            available,
        })
    } else {
        Ok(span)
    }
}

fn extension<A: Admission + ?Sized>(
    source: &mut impl Source,
    model: &mut Model,
    state: &mut State,
    header: &Header,
    meter: &Meter,
    memory: &mut Memory<'_, A>,
) -> Result<(), Error> {
    let body = header.record.body;
    meter.metadata(body.end - body.start)?;
    meter.work(body.end - body.start)?;
    let bytes = source.read(body, Operation::Metadata, meter, memory)?;
    match header.record.kind {
        b'x' | b'g' => {
            let fields = pax_fields(&bytes, body, header.record.header, meter, memory)?;
            for field in fields {
                let index = model.pax.len();
                memory
                    .push(&mut model.pax, field)
                    .map_err(|cause| storage(meter, cause))?;
                let global = header.record.kind == b'g';
                update(
                    if global {
                        &mut state.global
                    } else {
                        &mut state.local
                    },
                    index,
                    &model.pax,
                    global,
                    meter,
                    memory,
                )?;
            }
        }
        b'L' | b'K' => {
            let count = bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(bytes.len());
            if bytes[count..].iter().any(|byte| *byte != 0) {
                return Err(Error::Invalid {
                    span: body,
                    problem: Problem::Format,
                });
            }
            let value = Span {
                start: body.start,
                end: body.start + count as u64,
            };
            if header.record.kind == b'L' {
                state.long_name = Some(value);
            } else {
                state.long_link = Some(value);
            }
        }
        actual => {
            return Err(Error::Kind {
                offset: header.record.header.start,
                actual,
            });
        }
    }
    state.pending = true;
    Ok(())
}

pub(super) fn pax_fields<A: Admission + ?Sized>(
    bytes: &[u8],
    body: Span,
    header: Span,
    meter: &Meter,
    memory: &mut Memory<'_, A>,
) -> Result<Vec<Pax>, Error> {
    let mut fields = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        meter.work((bytes.len() - offset) as u64)?;
        let rest = Span {
            start: body.start + offset as u64,
            end: body.end,
        };
        let delimiter = bytes[offset..]
            .iter()
            .position(|byte| *byte == b' ')
            .ok_or(Error::Pax {
                record: body.start + offset as u64,
                span: rest,
                body,
                length: 0,
                problem: Problem::PaxLength,
            })?
            + offset;
        let length_span = Span {
            start: rest.start,
            end: body.start + delimiter as u64,
        };
        let text =
            std::str::from_utf8(&bytes[offset..delimiter]).map_err(|cause| Error::PaxUtf8 {
                record: body.start + offset as u64,
                span: length_span,
                body,
                length: true,
                cause,
            })?;
        let length: usize = text.parse().map_err(|cause| Error::PaxInteger {
            record: body.start + offset as u64,
            span: length_span,
            body,
            cause,
        })?;
        if !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(Error::Pax {
                record: body.start + offset as u64,
                span: length_span,
                body,
                length,
                problem: Problem::Decimal,
            });
        }
        let end = offset
            .checked_add(length)
            .filter(|end| *end <= bytes.len() && *end > delimiter)
            .ok_or(Error::Pax {
                record: body.start + offset as u64,
                span: rest,
                body,
                length,
                problem: Problem::PaxOverrun,
            })?;
        let value_span = Span {
            start: body.start + delimiter as u64 + 1,
            end: body.start + end as u64,
        };
        let record =
            std::str::from_utf8(&bytes[delimiter + 1..end]).map_err(|cause| Error::PaxUtf8 {
                record: body.start + offset as u64,
                span: value_span,
                body,
                length: false,
                cause,
            })?;
        let record = record.strip_suffix('\n').ok_or(Error::Pax {
            record: body.start + offset as u64,
            span: value_span,
            body,
            length,
            problem: Problem::PaxNewline,
        })?;
        let (key, value) = record
            .split_once('=')
            .filter(|(key, _)| !key.is_empty())
            .ok_or(Error::Pax {
                record: body.start + offset as u64,
                span: Span {
                    start: value_span.start,
                    end: value_span.end - 1,
                },
                body,
                length,
                problem: Problem::PaxEquals,
            })?;
        let key_span = Span {
            start: value_span.start,
            end: value_span.start + key.len() as u64,
        };
        let value_span = Span {
            start: key_span.end + 1,
            end: key_span.end + 1 + value.len() as u64,
        };
        let key = memory.string(key).map_err(|cause| storage(meter, cause))?;
        memory
            .push(
                &mut fields,
                Pax {
                    key,
                    key_span,
                    value: value_span,
                    header,
                },
            )
            .map_err(|cause| storage(meter, cause))?;
        offset = end;
    }
    Ok(fields)
}

fn key_position(
    fields: &[Pax],
    indexes: &[usize],
    key: &str,
    meter: &Meter,
) -> Result<Option<usize>, Error> {
    for (position, index) in indexes.iter().enumerate() {
        meter.work(
            (fields[*index].key.len() as u64)
                .saturating_add(key.len() as u64)
                .saturating_add(1),
        )?;
        if fields[*index].key == key {
            return Ok(Some(position));
        }
    }
    Ok(None)
}
fn update<A: Admission + ?Sized>(
    indexes: &mut Vec<usize>,
    index: usize,
    fields: &[Pax],
    delete_empty: bool,
    meter: &Meter,
    memory: &mut Memory<'_, A>,
) -> Result<(), Error> {
    let old = key_position(fields, indexes, &fields[index].key, meter)?;
    let delete = delete_empty && fields[index].value.start == fields[index].value.end;
    match (old, delete) {
        (Some(position), true) => {
            indexes.remove(position);
        }
        (Some(position), false) => indexes[position] = index,
        (None, false) => memory
            .push(indexes, index)
            .map_err(|cause| storage(meter, cause))?,
        (None, true) => {}
    }
    Ok(())
}
fn effective<A: Admission + ?Sized>(
    state: &State,
    model: &Model,
    meter: &Meter,
    memory: &mut Memory<'_, A>,
) -> Result<Vec<usize>, Error> {
    let mut indexes = Vec::new();
    memory
        .extend(&mut indexes, state.global.iter().copied())
        .map_err(|cause| storage(meter, cause))?;
    for index in &state.local {
        update(&mut indexes, *index, &model.pax, true, meter, memory)?;
    }
    Ok(indexes)
}
fn selected<'a>(
    model: &'a Model,
    indexes: &[usize],
    key: &str,
    meter: &Meter,
) -> Result<Option<&'a Pax>, Error> {
    key_position(&model.pax, indexes, key, meter)
        .map(|position| position.map(|position| &model.pax[indexes[position]]))
}

fn validate_override<A: Admission + ?Sized>(
    source: &mut impl Source,
    field: &Pax,
    meter: &Meter,
    memory: &mut Memory<'_, A>,
) -> Result<(), Error> {
    meter.work(field.key.len() as u64 + field.value.end - field.value.start + 1)?;
    if field.key.starts_with("GNU.sparse.")
        || matches!(field.key.as_str(), "SCHILY.realsize" | "SCHILY.filetype")
    {
        return Err(Error::Invalid {
            span: field.key_span,
            problem: Problem::Sparse,
        });
    }
    if matches!(
        field.key.as_str(),
        "size" | "uid" | "gid" | "devmajor" | "devminor"
    ) {
        decimal(
            &source.read(field.value, Operation::Field, meter, memory)?,
            field.value,
        )?;
    } else if field.key == "mtime" {
        timestamp(
            &source.read(field.value, Operation::Field, meter, memory)?,
            field.value,
        )?;
    }
    Ok(())
}
fn decimal(bytes: &[u8], span: Span) -> Result<u64, Error> {
    let text = std::str::from_utf8(bytes).map_err(|cause| Error::Utf8 { span, cause })?;
    let value = text
        .parse()
        .map_err(|cause| Error::Integer { span, cause })?;
    if !bytes.iter().all(u8::is_ascii_digit) {
        return Err(Error::Invalid {
            span,
            problem: Problem::Decimal,
        });
    }
    Ok(value)
}
fn timestamp(bytes: &[u8], span: Span) -> Result<(), Error> {
    let text = std::str::from_utf8(bytes).map_err(|cause| Error::Utf8 { span, cause })?;
    let (whole, fraction) = text
        .split_once('.')
        .map_or((text, None), |(whole, fraction)| (whole, Some(fraction)));
    let digits = whole
        .strip_prefix('+')
        .or_else(|| whole.strip_prefix('-'))
        .unwrap_or(whole);
    if digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.is_some_and(|fraction| {
            fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return Err(Error::Invalid {
            span,
            problem: Problem::Timestamp,
        });
    }
    Ok(())
}

fn member<A: Admission + ?Sized>(
    source: &mut impl Source,
    model: &Model,
    state: &mut State,
    header: &Header,
    effective: Vec<usize>,
    meter: &Meter,
    memory: &mut Memory<'_, A>,
) -> Result<Member, Error> {
    let kind = match header.record.kind {
        0 | b'0' => FileEntryKind::File,
        b'5' => FileEntryKind::Directory,
        b'2' => FileEntryKind::Symlink,
        b'1' => FileEntryKind::Hardlink,
        b'6' => FileEntryKind::Fifo,
        b'3' => FileEntryKind::CharDev,
        b'4' => FileEntryKind::BlockDev,
        actual => {
            return Err(Error::Kind {
                offset: header.record.header.start,
                actual,
            });
        }
    };
    // Both pending GNU values belong to this real member even when PAX wins.
    let long_name = state.long_name.take();
    let long_link = state.long_link.take();
    state.local.clear();
    state.pending = false;
    let path = selected(model, &effective, "path", meter)?.map_or_else(
        || {
            long_name.map_or(header.name, |value| NameSpans {
                prefix: None,
                value,
            })
        },
        |field| NameSpans {
            prefix: None,
            value: field.value,
        },
    );
    let link = selected(model, &effective, "linkpath", meter)?
        .map_or_else(|| long_link.or(header.link), |field| Some(field.value));
    let mut number = |key, default| -> Result<u64, Error> {
        match selected(model, &effective, key, meter)? {
            None => Ok(default),
            Some(field) => decimal(
                &source.read(field.value, Operation::Field, meter, memory)?,
                field.value,
            ),
        }
    };
    let uid = number("uid", header.uid)?;
    let gid = number("gid", header.gid)?;
    let dev_major = number("devmajor", header.dev_major)?;
    let dev_minor = number("devminor", header.dev_minor)?;
    let user =
        selected(model, &effective, "uname", meter)?.map_or(header.user, |field| Some(field.value));
    let group = selected(model, &effective, "gname", meter)?
        .map_or(header.group, |field| Some(field.value));
    Ok(Member {
        record: header.record,
        path,
        link,
        kind,
        mode: header.mode,
        uid,
        gid,
        header_mtime: header.mtime,
        user,
        group,
        dev_major,
        dev_minor,
        effective,
    })
}
