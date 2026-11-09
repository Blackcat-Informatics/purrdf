// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit v4 projection over the shared framing and effective metadata engine.

use super::{Account, Error, Limits, Member, Meter, Model, Operation, Problem, Span, core};
use crate::{
    files::{FileEntryKind, FilePaxRecord},
    tar::{CORE_PAX_KEYS, RawTarEntry, SeekTarEntry, TarError},
};
use core::Source;
use purrdf_lex::allocation::{Admission, Memory, Resident};
#[cfg(test)]
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};

fn limits(source_bytes: u64) -> Limits {
    // Existing entry points carry no caller-selected budget. This selects their
    // explicit resident policy before parsing; strict refusal never retries it.
    Limits {
        source_bytes,
        entries: usize::MAX,
        metadata_bytes: u64::MAX,
        storage_bytes: usize::MAX,
        work: u64::MAX,
    }
}

pub(crate) fn buffered(data: &[u8]) -> Result<Vec<RawTarEntry<'_>>, TarError> {
    let mut source = core::Slice(data);
    let meter =
        Meter::new(limits(data.len() as u64), data.len() as u64).map_err(TarError::Inventory)?;
    let mut resident = Resident;
    let mut account = Account {
        original: &mut resident,
        meter: &meter,
    };
    let mut memory = Memory::new(&mut account);
    let model = core::parse(&mut source, &meter, &mut memory)
        .map_err(|error| project_error(error, &mut source, &meter, &mut memory))?;
    let mut output = Vec::new();
    for member in &model.members {
        let entry = project(&mut source, member, &model, &meter, &mut memory)?;
        let data = if entry.kind == FileEntryKind::File {
            member
                .record
                .body
                .slice(data)
                .map_err(TarError::Inventory)?
        } else {
            &[]
        };
        output.push(RawTarEntry {
            path: entry.path,
            kind: entry.kind,
            mode: entry.mode,
            modified_seconds: entry.modified_seconds,
            uid: entry.uid,
            gid: entry.gid,
            user_name: entry.user_name,
            group_name: entry.group_name,
            link_target: entry.link_target,
            dev_major: entry.dev_major,
            dev_minor: entry.dev_minor,
            pax_records: entry.pax_records,
            data,
        });
    }
    Ok(output)
}

pub(crate) fn seekable<R: Read + Seek>(reader: &mut R) -> Result<Vec<SeekTarEntry>, TarError> {
    let length = reader
        .seek(SeekFrom::End(0))
        .map_err(|cause| TarError::io("read tar length", None, cause))?;
    let mut source = core::Seeking {
        reader,
        length,
        buffer: Vec::new(),
        cached: None,
    };
    let meter = Meter::new(limits(length), length).map_err(TarError::Inventory)?;
    let mut resident = Resident;
    let mut account = Account {
        original: &mut resident,
        meter: &meter,
    };
    let mut memory = Memory::new(&mut account);
    let model = core::parse(&mut source, &meter, &mut memory)
        .map_err(|error| project_error(error, &mut source, &meter, &mut memory))?;
    model
        .members
        .iter()
        .map(|member| project(&mut source, member, &model, &meter, &mut memory))
        .collect()
}

fn text<A: Admission + ?Sized>(
    source: &mut impl Source,
    span: Span,
    meter: &Meter,
    memory: &mut Memory<'_, A>,
) -> Result<String, TarError> {
    let bytes = source
        .read(span, Operation::Field, meter, memory)
        .map_err(TarError::Inventory)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
fn optional<A: Admission + ?Sized>(
    source: &mut impl Source,
    span: Option<Span>,
    meter: &Meter,
    memory: &mut Memory<'_, A>,
) -> Result<Option<String>, TarError> {
    span.map(|span| text(source, span, meter, memory))
        .transpose()
}
fn project<A: Admission + ?Sized>(
    source: &mut impl Source,
    member: &Member,
    model: &Model,
    meter: &Meter,
    memory: &mut Memory<'_, A>,
) -> Result<SeekTarEntry, TarError> {
    let mut path = String::new();
    if let Some(prefix) = member.path.prefix {
        path.push_str(&text(source, prefix, meter, memory)?);
        path.push('/');
    }
    path.push_str(&text(source, member.path.value, meter, memory)?);
    if path.is_empty() {
        return Err(TarError::EmptyPath);
    }
    let path = path
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_owned();
    let mut pax_records = Vec::new();
    let mut modified_seconds = member.header_mtime;
    for index in &member.effective {
        let field = &model.pax[*index];
        let value = text(source, field.value, meter, memory)?;
        if field.key == "mtime" {
            // The core already validated the entire signed decimal spelling.
            // v4 GTS import deliberately retains its unsigned-whole projection:
            // positive fractions floor; a valid negative value uses the original
            // header value. The strict inventory retains the effective raw value.
            modified_seconds = value
                .split('.')
                .next()
                .and_then(|whole| whole.parse::<u64>().ok())
                .unwrap_or(member.header_mtime);
        }
        if !CORE_PAX_KEYS.contains(&field.key.as_str()) {
            pax_records.push(FilePaxRecord {
                key: field.key.clone(),
                value,
            });
        }
    }
    pax_records.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(SeekTarEntry {
        path,
        kind: member.kind,
        mode: Some(member.mode),
        modified_seconds: Some(modified_seconds),
        uid: Some(member.uid),
        gid: Some(member.gid),
        user_name: optional(source, member.user, meter, memory)?,
        group_name: optional(source, member.group, meter, memory)?,
        link_target: optional(source, member.link, meter, memory)?,
        dev_major: Some(member.dev_major),
        dev_minor: Some(member.dev_minor),
        pax_records,
        data_offset: member.record.body.start,
        data_size: if member.kind == FileEntryKind::File {
            member.record.body.end - member.record.body.start
        } else {
            0
        },
    })
}

fn original<A: Admission + ?Sized>(
    source: &mut impl Source,
    span: Span,
    meter: &Meter,
    memory: &mut Memory<'_, A>,
) -> Result<Vec<u8>, TarError> {
    source
        .read(span, Operation::Field, meter, memory)
        .map(std::borrow::Cow::into_owned)
        .map_err(TarError::Inventory)
}
fn project_error<A: Admission + ?Sized>(
    error: Error,
    source: &mut impl Source,
    meter: &Meter,
    memory: &mut Memory<'_, A>,
) -> TarError {
    match project_failure(error, source, meter, memory) {
        Ok(error) | Err(error) => error,
    }
}
fn project_failure<A: Admission + ?Sized>(
    error: Error,
    source: &mut impl Source,
    meter: &Meter,
    memory: &mut Memory<'_, A>,
) -> Result<TarError, TarError> {
    Ok(match error {
        Error::Io {
            operation, cause, ..
        } => TarError::Io {
            operation: operation.name(),
            path: None,
            cause,
        },
        Error::Checksum {
            offset,
            actual,
            expected,
        } => TarError::Checksum {
            offset,
            actual,
            expected,
        },
        Error::Invalid {
            span,
            problem: Problem::Octal,
        } if span.start % 512 == 148 => TarError::InvalidChecksum {
            offset: span.start - 148,
            actual: original(source, span, meter, memory)?,
        },
        Error::Invalid {
            span,
            problem: Problem::Octal,
        } if span.start % 512 == 124 => TarError::InvalidSize {
            offset: span.start - 124,
            actual: original(source, span, meter, memory)?,
        },
        Error::Invalid {
            problem: Problem::Empty,
            ..
        } => TarError::EmptyArchive,
        Error::Truncated {
            required,
            available,
        } => TarError::BodyOverrun {
            offset: usize::try_from(required.start).map_err(|_| TarError::BodyTooLarge {
                size: required.start,
            })?,
            size: required.end - required.start,
            available: usize::try_from(available)
                .map_err(|_| TarError::BodyTooLarge { size: available })?,
        },
        Error::Kind { offset, actual } => TarError::EntryType {
            actual,
            block: u128::from(offset / 512),
        },
        Error::PaxUtf8 {
            record,
            span,
            body,
            length,
            cause,
        } => {
            let actual = original(source, span, meter, memory)?;
            let offset =
                usize::try_from(record - body.start).map_err(|_| TarError::BodyTooLarge {
                    size: span.start - body.start,
                })?;
            if length {
                TarError::PaxLengthUtf8 {
                    offset,
                    actual,
                    cause,
                }
            } else {
                TarError::PaxRecordUtf8 {
                    offset,
                    actual,
                    cause,
                }
            }
        }
        Error::PaxInteger { span, cause, .. } => TarError::PaxLengthInteger {
            actual: text(source, span, meter, memory)?,
            cause,
        },
        Error::Pax {
            record,
            span,
            body,
            length,
            problem,
        } => {
            let offset =
                usize::try_from(record - body.start).map_err(|_| TarError::BodyTooLarge {
                    size: span.start - body.start,
                })?;
            match problem {
                Problem::PaxLength => TarError::PaxLengthMissing {
                    offset,
                    remaining: usize::try_from(span.end - span.start).map_err(|_| {
                        TarError::BodyTooLarge {
                            size: span.end - span.start,
                        }
                    })?,
                },
                Problem::PaxOverrun => TarError::PaxOverrun {
                    offset,
                    length,
                    available: usize::try_from(body.end - body.start).map_err(|_| {
                        TarError::BodyTooLarge {
                            size: body.end - body.start,
                        }
                    })?,
                },
                Problem::PaxNewline => TarError::PaxNewline {
                    offset,
                    actual: text(source, span, meter, memory)?,
                },
                Problem::PaxEquals => TarError::PaxEquals {
                    offset,
                    actual: text(source, span, meter, memory)?,
                },
                problem => TarError::Inventory(Error::Pax {
                    record,
                    span,
                    body,
                    length,
                    problem,
                }),
            }
        }
        error => TarError::Inventory(error),
    })
}

#[cfg(test)]
pub(crate) fn pax(bytes: &[u8]) -> Result<BTreeMap<String, String>, TarError> {
    let mut source = core::Slice(bytes);
    let meter =
        Meter::new(limits(bytes.len() as u64), bytes.len() as u64).map_err(TarError::Inventory)?;
    let mut resident = Resident;
    let mut account = Account {
        original: &mut resident,
        meter: &meter,
    };
    let mut memory = Memory::new(&mut account);
    let body = Span {
        start: 0,
        end: bytes.len() as u64,
    };
    let fields = core::pax_fields(bytes, body, body, &meter, &mut memory)
        .map_err(|error| project_error(error, &mut source, &meter, &mut memory))?;
    fields
        .into_iter()
        .map(|field| {
            Ok((
                field.key,
                text(&mut source, field.value, &meter, &mut memory)?,
            ))
        })
        .collect()
}
