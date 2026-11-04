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
    }
    fn io(&self) -> (&str, &str) {
        match self {
            Self::Encode { input, output, .. }
            | Self::Decode { input, output, .. }
            | Self::Inspect { input, output, .. }
            | Self::Batch { input, output, .. } => (input, output),
        }
    }
}

/// A conversion refusal retaining the original native codec or I/O cause.
#[derive(Debug)]
pub enum Failure {
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
            let len = Reference::Full(digest).encode(buffer)?;
            output.write_all(&buffer[..len])?;
            output.write_all(b"\n")?;
        }
    }
    Ok(())
}

struct PreparedInput {
    reader: io::BufReader<Box<dyn io::Read>>,
    single: Option<Digest32>,
}

fn prepare(command: &Command) -> Result<PreparedInput, Failure> {
    let (input_path, output_path) = command.io();
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
    // Single-record conversion finishes validation before opening an output file.
    let single = if matches!(command, Command::Batch { .. }) {
        None
    } else {
        let format = match command {
            Command::Encode { from, .. } => (*from).into(),
            Command::Inspect { from, .. } => *from,
            _ => Format::Words,
        };
        let mut input = Vec::new();
        input.try_reserve_exact(word_id::MAX_ENCODED_LEN + 2)?;
        reader
            .by_ref()
            .take((word_id::MAX_ENCODED_LEN + 2) as u64)
            .read_to_end(&mut input)?;
        if !matches!(format, Format::Raw) && input.last() == Some(&b'\n') {
            input.pop();
        }
        Some(digest(&input, format)?)
    };
    Ok(PreparedInput { reader, single })
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
    let PreparedInput { mut reader, single } = prepare(command).map_err(CliError::from)?;
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
) -> Result<(), Failure> {
    match command {
        Command::Batch { from, to, .. } => {
            batch(reader, writer, *from, *to, buffer, refusals)?;
        }
        Command::Inspect { .. } => {
            let digest = single.ok_or(Failure::Invariant("missing validated input"))?;
            writeln!(
                writer,
                "digest {digest}\npayload_words 22\ncheck_words 1\ndisplayed_words 23"
            )?;
            let len = Reference::Full(digest).encode(buffer)?;
            let check = buffer[..len]
                .iter()
                .rposition(|byte| *byte == b' ')
                .ok_or(Failure::Invariant("missing encoded separator"))?;
            writer.write_all(b"check ")?;
            writer.write_all(&buffer[check + 1..len])?;
            writer.write_all(b"\ncanonical ")?;
            writer.write_all(&buffer[..len])?;
            writer.write_all(b"\n")?;
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
) -> Result<(), Failure> {
    let mut input = Vec::new();
    input.try_reserve_exact(word_id::MAX_ENCODED_LEN + 1)?;
    let mut record = 0;
    let mut stream_offset = 0_u64;
    loop {
        input.clear();
        let start = stream_offset;
        let mut saw_input = false;
        loop {
            let chunk = reader.fill_buf()?;
            if chunk.is_empty() {
                break;
            }
            saw_input = true;
            let newline = chunk.iter().position(|byte| *byte == b'\n');
            let consume = newline.map_or(chunk.len(), |index| index + 1);
            let keep = consume.min((word_id::MAX_ENCODED_LEN + 1).saturating_sub(input.len()));
            input.extend_from_slice(&chunk[..keep]);
            stream_offset += consume as u64;
            reader.consume(consume);
            if newline.is_some() {
                break;
            }
        }
        if !saw_input {
            break;
        }
        record += 1;
        let result = if stream_offset - start > (word_id::MAX_ENCODED_LEN + 1) as u64 {
            Err(Failure::RecordTooLong {
                record,
                offset: start + word_id::MAX_ENCODED_LEN as u64,
            })
        } else {
            if input.last() == Some(&b'\n') {
                input.pop();
            }
            digest(
                &input,
                match from {
                    Lines::HexLines => Format::Hex,
                    Lines::WordLines => Format::Words,
                },
            )
        };
        match result {
            Ok(digest) => write_digest(
                writer,
                digest,
                match to {
                    Lines::HexLines => Format::Hex,
                    Lines::WordLines => Format::Words,
                },
                buffer,
            )?,
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
