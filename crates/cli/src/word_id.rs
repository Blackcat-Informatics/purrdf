// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit full-ID conversion and bounded buffered record framing.

use std::fmt;
use std::io::{self, Read as _, Write as _};
use std::path::PathBuf;

use crate::{error::CliError, sink::OutTarget};
use clap::{Subcommand, ValueEnum};
use purrdf_hash::{
    hex::{self, Digest32},
    word_id::{self, Reference},
};
mod scoped;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum Format {
    Hex,
    Raw,
    Words,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum DigestFormat {
    Hex,
    Raw,
}
impl From<DigestFormat> for Format {
    fn from(format: DigestFormat) -> Self {
        match format {
            DigestFormat::Hex => Self::Hex,
            DigestFormat::Raw => Self::Raw,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum Lines {
    HexLines,
    WordLines,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Build a `PURRAL01` snapshot from canonical hex lines; report exact lengths
    /// and the retained revision on stderr. The scope belongs to the caller.
    Plan {
        #[arg(long)]
        scope: String,
        #[arg(long, default_value = "-")]
        input: String,
        #[arg(long)]
        output: String,
    },
    /// Shorten a member of the exact retained snapshot.
    Shorten {
        #[command(flatten)]
        snapshot: scoped::Snapshot,
        #[arg(long, value_enum)]
        from: Format,
        #[arg(long, value_enum)]
        length: scoped::LengthChoice,
        #[arg(long, default_value = "-")]
        input: String,
        #[arg(long, default_value = "-")]
        output: String,
    },
    /// Expand a checked reference in the required original snapshot revision.
    Expand {
        #[command(flatten)]
        snapshot: scoped::Snapshot,
        #[arg(long, value_enum)]
        to: DigestFormat,
        #[arg(long, default_value = "-")]
        input: String,
        #[arg(long, default_value = "-")]
        output: String,
    },
    /// Inspect a checked reference and its uniquely resolved complete identity.
    InspectScoped {
        #[command(flatten)]
        snapshot: scoped::Snapshot,
        #[arg(long, default_value = "-")]
        input: String,
        #[arg(long, default_value = "-")]
        output: String,
    },
    /// Convert explicit LF-framed formats using one retained snapshot index.
    BatchScoped {
        #[command(flatten)]
        snapshot: scoped::Snapshot,
        #[arg(long, value_enum)]
        from: Lines,
        #[arg(long, value_enum)]
        to: Lines,
        #[arg(long, value_enum)]
        length: scoped::LengthChoice,
        #[arg(long, default_value = "-")]
        input: String,
        #[arg(long, default_value = "-")]
        output: String,
    },
    /// Encode one 64-digit canonical hex or exact 32-byte input record.
    Encode {
        #[arg(long, value_enum)]
        from: DigestFormat,
        #[arg(long, default_value = "-")]
        input: String,
        #[arg(long, default_value = "-")]
        output: String,
    },
    /// Validate a full word ID and emit canonical hex or exact digest bytes.
    Decode {
        #[arg(long, value_enum)]
        to: DigestFormat,
        #[arg(long, default_value = "-")]
        input: String,
        #[arg(long, default_value = "-")]
        output: String,
    },
    /// Show the full identity, payload/check separation and canonical spelling.
    Inspect {
        #[arg(long, value_enum)]
        from: Format,
        #[arg(long, default_value = "-")]
        input: String,
        #[arg(long, default_value = "-")]
        output: String,
    },
    /// Convert LF-framed records, reusing buffers. Invalid records produce no
    /// output record, report their original input record/byte on stderr, and
    /// make the final exit code one. Successful records keep input order.
    Batch {
        #[arg(long, value_enum)]
        from: Lines,
        #[arg(long, value_enum)]
        to: Lines,
        #[arg(long, default_value = "-")]
        input: String,
        #[arg(long, default_value = "-")]
        output: String,
    },
}

impl Command {
    pub(crate) fn paths(&self, paths: &mut Vec<PathBuf>) {
        for path in <[&str; 2]>::from(self.io()) {
            if path != "-" {
                paths.push(PathBuf::from(path));
            }
        }
        if let Some(snapshot) = self.snapshot() {
            paths.push(PathBuf::from(&snapshot.manifest));
        }
    }
    fn snapshot(&self) -> Option<&scoped::Snapshot> {
        match self {
            Self::Shorten { snapshot, .. }
            | Self::Expand { snapshot, .. }
            | Self::InspectScoped { snapshot, .. }
            | Self::BatchScoped { snapshot, .. } => Some(snapshot),
            _ => None,
        }
    }
    fn io(&self) -> (&str, &str) {
        match self {
            Self::Plan { input, output, .. }
            | Self::Shorten { input, output, .. }
            | Self::Expand { input, output, .. }
            | Self::InspectScoped { input, output, .. }
            | Self::BatchScoped { input, output, .. }
            | Self::Encode { input, output, .. }
            | Self::Decode { input, output, .. }
            | Self::Inspect { input, output, .. }
            | Self::Batch { input, output, .. } => (input, output),
        }
    }
}

/// A conversion refusal retaining the original native codec or I/O cause.
#[derive(Debug)]
pub enum Failure {
    /// Original native snapshot owner failure.
    Alias(purrdf_core::content_alias::Error),
    /// Original native prefix planning or lookup failure.
    Prefix(purrdf_core::content_prefix::Error),
    /// Original admitted storage failure.
    Storage(purrdf_lex::allocation::StorageError),
    /// Source record context with the original typed failure.
    Record {
        /// One-based original source record.
        record: usize,
        /// Absolute source byte offset of the record start.
        offset: u64,
        /// Original unmodified refusal.
        cause: Box<Self>,
    },
    /// Original strict word codec refusal.
    Codec(word_id::Error),
    /// Original canonical base16 refusal.
    Hex(hex::HexError),
    /// Original file or stream failure.
    Io(io::Error),
    /// The selected digest input format has an exact byte width.
    Length {
        /// Input bytes.
        actual: usize,
        /// Required bytes.
        expected: usize,
    },
    /// A bounded LF-framed input record exceeded the canonical maximum.
    RecordTooLong {
        /// One-based record ordinal.
        record: usize,
        /// Absolute stream byte offset of the first excess byte.
        offset: u64,
    },
    /// Ordered batch output completed with explicitly reported invalid records.
    Batch {
        /// Number of refused records.
        refused: usize,
        /// Original first refusal, retained after per-record reporting.
        first: Box<Self>,
    },
    /// A format or path combination has no defined conversion.
    Usage(&'static str),
    /// An internal preparation or native serialization invariant was broken.
    Invariant(&'static str),
    /// Original input-buffer allocation refusal.
    Allocation(std::collections::TryReserveError),
}
impl fmt::Display for Failure {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Alias(error) => error.fmt(out),
            Self::Prefix(error) => error.fmt(out),
            Self::Storage(error) => error.fmt(out),
            Self::Record {
                record,
                offset,
                cause,
            } => write!(out, "record {record}, stream byte {offset}: {cause}"),
            Self::Codec(error) => error.fmt(out),
            Self::Hex(error) => error.fmt(out),
            Self::Io(error) => error.fmt(out),
            Self::Length { actual, expected } => write!(
                out,
                "digest input has {actual} bytes; expected exactly {expected}"
            ),
            Self::RecordTooLong { record, offset } => write!(
                out,
                "record {record} exceeds canonical word ID length at stream byte {offset}"
            ),
            Self::Batch { refused, .. } => write!(
                out,
                "word ID batch refused {refused} records; successful output remains in input order"
            ),
            Self::Usage(message) | Self::Invariant(message) => out.write_str(message),
            Self::Allocation(error) => error.fmt(out),
        }
    }
}
impl std::error::Error for Failure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Alias(error) => Some(error),
            Self::Prefix(error) => Some(error),
            Self::Storage(error) => Some(error),
            Self::Record { cause, .. } => Some(&**cause),
            Self::Codec(error) => Some(error),
            Self::Hex(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Allocation(error) => Some(error),
            Self::Batch { first, .. } => Some(&**first),
            _ => None,
        }
    }
}
purrdf_lex::variant_from!(Failure { Codec(word_id::Error), Hex(hex::HexError), Io(io::Error), Allocation(std::collections::TryReserveError) });
purrdf_lex::variant_from!(Failure { Alias(purrdf_core::content_alias::Error), Prefix(purrdf_core::content_prefix::Error), Storage(purrdf_lex::allocation::StorageError) });

fn digest(input: &[u8], format: Format) -> Result<Digest32, Failure> {
    match format {
        Format::Words => {
            let Reference::Full(digest) = word_id::decode(input)? else {
                return Err(word_id::Error::ExpectedFull.into());
            };
            Ok(digest)
        }
        Format::Raw => {
            let bytes: [u8; 32] = input.try_into().map_err(|_| Failure::Length {
                actual: input.len(),
                expected: 32,
            })?;
            Ok(Digest32::new(bytes))
        }
        Format::Hex => {
            if input.len() != 64 {
                return Err(Failure::Length {
                    actual: input.len(),
                    expected: 64,
                });
            }
            let mut bytes = [0; 32];
            // Validate through the native digit reader before UTF-8 borrowing,
            // so a later malformed UTF-8 byte cannot hide an earlier bad digit.
            if let Some((offset, byte)) = input
                .iter()
                .enumerate()
                .find(|(_, byte)| hex::nibble_canonical(**byte).is_none())
            {
                return Err(hex::HexError::InvalidDigit {
                    offset,
                    byte: *byte,
                }
                .into());
            }
            let text = std::str::from_utf8(input).map_err(|error| hex::HexError::InvalidDigit {
                offset: error.valid_up_to(),
                byte: input[error.valid_up_to()],
            })?;
            hex::decode_canonical_to_slice(text, &mut bytes)?;
            Ok(Digest32::new(bytes))
        }
    }
}

fn write_digest(
    output: &mut dyn io::Write,
    digest: Digest32,
    format: Format,
    buffer: &mut [u8; word_id::MAX_ENCODED_LEN],
) -> Result<(), Failure> {
    match format {
        Format::Raw => output.write_all(digest.as_bytes())?,
        Format::Hex => {
            hex::encode_to_slice(digest.as_bytes(), &mut buffer[..64])?;
            output.write_all(&buffer[..64])?;
            output.write_all(b"\n")?;
        }
        Format::Words => {
            write_reference(output, Reference::Full(digest), buffer)?;
        }
    }
    Ok(())
}

fn write_reference(
    output: &mut dyn io::Write,
    reference: Reference,
    buffer: &mut [u8; word_id::MAX_ENCODED_LEN],
) -> Result<(), Failure> {
    let len = reference.encode(buffer)?;
    output.write_all(&buffer[..len])?;
    output.write_all(b"\n")?;
    Ok(())
}

fn inspect_reference(
    output: &mut dyn io::Write,
    reference: Reference,
    buffer: &mut [u8; word_id::MAX_ENCODED_LEN],
) -> Result<(), Failure> {
    writeln!(
        output,
        "payload_words {}\ncheck_words 1\ndisplayed_words {}",
        reference.payload_words(),
        reference.payload_words() + 1
    )?;
    let len = reference.encode(buffer)?;
    let check = buffer[..len]
        .iter()
        .rposition(|byte| *byte == b' ')
        .ok_or(Failure::Invariant("missing encoded separator"))?;
    output.write_all(b"check ")?;
    output.write_all(&buffer[check + 1..len])?;
    output.write_all(b"\ncanonical ")?;
    output.write_all(&buffer[..len])?;
    output.write_all(b"\n")?;
    Ok(())
}

fn read_single(reader: &mut dyn io::Read, format: Format) -> Result<Vec<u8>, Failure> {
    let mut input = Vec::new();
    input.try_reserve_exact(word_id::MAX_ENCODED_LEN + 2)?;
    reader
        .take((word_id::MAX_ENCODED_LEN + 2) as u64)
        .read_to_end(&mut input)?;
    if !matches!(format, Format::Raw) && input.last() == Some(&b'\n') {
        input.pop();
    }
    Ok(input)
}

struct PreparedInput {
    reader: io::BufReader<Box<dyn io::Read>>,
    single: Option<Digest32>,
    scoped: Option<scoped::State>,
}

fn prepare(command: &Command) -> Result<PreparedInput, Failure> {
    let (input_path, output_path) = command.io();
    if let Some(snapshot) = command.snapshot() {
        if snapshot.manifest == "-" {
            return Err(Failure::Usage("snapshot manifest must name a file"));
        }
        if output_path != "-"
            && (snapshot.manifest == output_path
                || crate::canonical::same_file(
                    std::path::Path::new(&snapshot.manifest),
                    std::path::Path::new(output_path),
                )?)
        {
            return Err(Failure::Usage(
                "output aliases the bound snapshot manifest; choose a distinct output",
            ));
        }
    }
    if input_path != "-" && input_path == output_path {
        return Err(Failure::Usage("input and output must have different paths"));
    }
    if input_path != "-"
        && output_path != "-"
        && crate::canonical::same_file(
            std::path::Path::new(input_path),
            std::path::Path::new(output_path),
        )?
    {
        return Err(Failure::Usage(
            "input and output alias the same file; choose a distinct output",
        ));
    }
    let input: Box<dyn io::Read> = if input_path == "-" {
        Box::new(io::stdin().lock())
    } else {
        Box::new(std::fs::File::open(input_path)?)
    };
    let mut reader = io::BufReader::new(input);
    if command.snapshot().is_some() || matches!(command, Command::Plan { .. }) {
        let (single, scoped) = scoped::prepare(command, &mut reader)?;
        return Ok(PreparedInput {
            reader,
            single,
            scoped: Some(scoped),
        });
    }
    // Single-record conversion finishes validation before opening an output file.
    let single = if matches!(command, Command::Batch { .. }) {
        None
    } else {
        let format = match command {
            Command::Encode { from, .. } => (*from).into(),
            Command::Inspect { from, .. } => *from,
            _ => Format::Words,
        };
        let input = read_single(&mut reader, format)?;
        Some(digest(&input, format)?)
    };
    Ok(PreparedInput {
        reader,
        single,
        scoped: None,
    })
}

#[derive(Default)]
struct Refusals {
    count: usize,
    first: Option<Box<Failure>>,
}
impl Refusals {
    fn finish(self) -> Option<Failure> {
        self.first.map(|first| Failure::Batch {
            refused: self.count,
            first,
        })
    }
}

pub(crate) fn run(command: &Command) -> Result<(), CliError> {
    let PreparedInput {
        mut reader,
        single,
        mut scoped,
    } = prepare(command).map_err(CliError::from)?;
    let mut target = OutTarget::open(command.io().1)?;
    let mut writer = io::BufWriter::new(target.writer());
    let mut buffer = [0; word_id::MAX_ENCODED_LEN];
    let mut refusals = Refusals::default();
    let operation = convert(
        command,
        &mut reader,
        single,
        &mut writer,
        &mut buffer,
        &mut refusals,
        scoped.as_mut(),
    );
    let result = match (refusals.finish(), operation) {
        (None, result) => result.map_err(CliError::from),
        (Some(original), Ok(())) => Err(CliError::from(original)),
        (Some(original), Err(failure)) => Err(CliError::OutputPublication {
            original: Box::new(CliError::from(original)),
            failure: Box::new(CliError::from(failure)),
        }),
    };
    let result = match result {
        Err(error) if !matches!(&error, CliError::WordId(failure) if matches!(&**failure, Failure::Batch { .. })) =>
        {
            drop(writer.into_parts());
            // Only the original stdout owner can classify an observed hangup.
            return Err(target.abandon(error));
        }
        result => result,
    };
    let flush = writer.flush().map_err(CliError::OutputIo);
    // Disarm without another write attempt, including after a failed flush.
    drop(writer.into_parts());
    match (result, flush) {
        (Ok(()), Ok(())) => target.finish(),
        (Ok(()), Err(error)) => Err(target.abandon(error)),
        (Err(original), Err(failure)) => Err(target.abandon(CliError::OutputPublication {
            original: Box::new(original),
            failure: Box::new(failure),
        })),
        (Err(original), Ok(())) => match target.finish() {
            Ok(()) => Err(original),
            Err(failure) => Err(CliError::OutputPublication {
                original: Box::new(original),
                failure: Box::new(failure),
            }),
        },
    }
}

fn convert(
    command: &Command,
    reader: &mut dyn io::BufRead,
    single: Option<Digest32>,
    writer: &mut dyn io::Write,
    buffer: &mut [u8; word_id::MAX_ENCODED_LEN],
    refusals: &mut Refusals,
    scoped: Option<&mut scoped::State>,
) -> Result<(), Failure> {
    if let Some(state) = scoped {
        return state.convert(command, reader, single, writer, buffer, refusals);
    }
    match command {
        Command::Batch { from, to, .. } => {
            batch(reader, writer, *from, *to, buffer, refusals, None)?;
        }
        Command::Inspect { .. } => {
            let digest = single.ok_or(Failure::Invariant("missing validated input"))?;
            writeln!(writer, "digest {digest}")?;
            inspect_reference(writer, Reference::Full(digest), buffer)?;
        }
        Command::Encode { .. } => write_digest(
            writer,
            single.ok_or(Failure::Invariant("missing validated input"))?,
            Format::Words,
            buffer,
        )?,
        Command::Decode { to, .. } => write_digest(
            writer,
            single.ok_or(Failure::Invariant("missing validated input"))?,
            (*to).into(),
            buffer,
        )?,
        _ => return Err(Failure::Invariant("missing scoped preparation")),
    }
    Ok(())
}

fn batch(
    reader: &mut dyn io::BufRead,
    writer: &mut dyn io::Write,
    from: Lines,
    to: Lines,
    buffer: &mut [u8; word_id::MAX_ENCODED_LEN],
    refusals: &mut Refusals,
    mut scoped: Option<&mut scoped::State>,
) -> Result<(), Failure> {
    let mut input = Vec::new();
    input.try_reserve_exact(word_id::MAX_ENCODED_LEN + 1)?;
    let mut record = 0;
    let mut stream_offset = 0_u64;
    loop {
        let start = stream_offset;
        let result = match read_record(reader, &mut input, &mut record, &mut stream_offset) {
            Ok(false) => break,
            Err(error @ Failure::RecordTooLong { .. }) => Err(error),
            Err(error) => return Err(error),
            Ok(true) => {
                let format = match from {
                    Lines::HexLines => Format::Hex,
                    Lines::WordLines => Format::Words,
                };
                if let Some(state) = scoped.as_deref_mut() {
                    state.read(&input, format)
                } else {
                    digest(&input, format)
                }
            }
        };
        match result {
            Ok(digest) => {
                if let Some(state) = scoped.as_deref_mut() {
                    state.write(writer, digest, to, buffer)?;
                } else {
                    write_digest(
                        writer,
                        digest,
                        match to {
                            Lines::HexLines => Format::Hex,
                            Lines::WordLines => Format::Words,
                        },
                        buffer,
                    )?;
                }
            }
            Err(error) => {
                let reported = writeln!(
                    io::stderr().lock(),
                    "record {record}, stream byte {start}: {error}"
                );
                refusals.count += 1;
                if refusals.first.is_none() {
                    refusals.first = Some(Box::new(error));
                }
                reported?;
            }
        }
    }
    Ok(())
}

fn read_record(
    reader: &mut dyn io::BufRead,
    input: &mut Vec<u8>,
    record: &mut usize,
    stream_offset: &mut u64,
) -> Result<bool, Failure> {
    input.clear();
    let start = *stream_offset;
    loop {
        let chunk = match reader.fill_buf() {
            Ok(chunk) => chunk,
            Err(error) => {
                *record += 1;
                return Err(error.into());
            }
        };
        if chunk.is_empty() {
            break;
        }
        let newline = chunk.iter().position(|byte| *byte == b'\n');
        let consume = newline.map_or(chunk.len(), |index| index + 1);
        let keep = consume.min((word_id::MAX_ENCODED_LEN + 1).saturating_sub(input.len()));
        input.extend_from_slice(&chunk[..keep]);
        *stream_offset += consume as u64;
        reader.consume(consume);
        if newline.is_some() {
            break;
        }
    }
    if *stream_offset == start {
        return Ok(false);
    }
    *record += 1;
    if *stream_offset - start > (word_id::MAX_ENCODED_LEN + 1) as u64 {
        return Err(Failure::RecordTooLong {
            record: *record,
            offset: start + word_id::MAX_ENCODED_LEN as u64,
        });
    }
    if input.last() == Some(&b'\n') {
        input.pop();
    }
    Ok(true)
}
