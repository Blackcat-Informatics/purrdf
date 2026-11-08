// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Admitted visual-hashing 1.0.0 presentations of authoritative content identities.
//!
//! Emoji and spoken labels share MSB-first six-bit symbols of BLAKE3 XOF output.
//! Randomart walks the original input bytes, without another hash. The content
//! entry point always supplies the exact 32 bytes of [`Digest32`], never its hex,
//! alias or future word spelling. Resolve a scoped reference before calling it.
//! A short fingerprint can collide: compare full identities and verify content
//! cryptographically. Visual similarity says nothing about content similarity.
//!
//! Rendering law and alphabet derive from visual-hashing 1.0.0, revision
//! `519c5bbdb5e0ef4c6c70aa0b5cf42a8877f6e776`; the upstream three-license grant
//! is preserved. See `docs/provenance/visual-hashing.md`. The implementation uses
//! only the shared native BLAKE3 reader and constant stack workspace. An admitted
//! counting pass determines exact output sizes before any allocation; rendering
//! repeats the admitted symbol stream and publishes all outputs atomically.

mod alphabet;
use crate::allocation::{Admission, Memory, OwnedAdmission, Resident, StorageError};
pub use alphabet::{ALPHABET_SIZE, EMOJI, LABELS};
use core::{fmt, mem, num::NonZeroUsize};
use purrdf_hash::{
    blake3,
    hex::{self, Digest32},
};

/// Frozen algorithm and alphabet identity, independent of output length.
pub const SCHEME: &str = "visual-hashing-1.0.0";
const WIDTH: usize = 17;
const HEIGHT: usize = 9;
const RAMP: &[u8] = b" .o+=*BOX@%&#/^";
const BORDER: &str = "+----------------+";

/// An explicitly selected positive symbol count and original randomart label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request<'a> {
    symbols: NonZeroUsize,
    label: &'a str,
}
impl<'a> Request<'a> {
    /// Select the frozen scheme's length and header. Zero and impossible output
    /// or XOF lengths refuse before scanning or allocation.
    pub fn new(symbols: usize, label: &'a str) -> Result<Self, Error> {
        let symbols = NonZeroUsize::new(symbols).ok_or(Error::Length { symbols })?;
        // Every frozen emoji is four UTF-8 bytes, followed by one separator
        // except the last. This is the exact emoji size, never a label estimate.
        symbols
            .get()
            .checked_mul(5)
            .and_then(|n| n.checked_sub(1))
            .ok_or_else(|| Error::Length {
                symbols: symbols.get(),
            })?;
        xof_bytes(symbols.get())?;
        Ok(Self { symbols, label })
    }
    /// Requested number of emoji and corresponding spoken labels.
    pub const fn symbols(self) -> usize {
        self.symbols.get()
    }
}

/// Native refusal, retaining any original allocation or admission cause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Zero or unrepresentable requested symbol count.
    Length {
        /// Original requested count.
        symbols: usize,
    },
    /// A full digest needs exactly 64 hexadecimal bytes.
    DigestLength {
        /// Original spelling byte count.
        bytes: usize,
    },
    /// Original shared hexadecimal reader refusal.
    Hex(hex::HexError),
    /// Original work/storage refusal and any subsequent refused cleanup.
    Storage {
        /// Unchanged first native cause.
        cause: StorageError,
        /// Later cleanup refusal, without replacing cause.
        cleanup_refusal: Option<StorageError>,
    },
}
impl From<StorageError> for Error {
    fn from(cause: StorageError) -> Self {
        Self::Storage {
            cause,
            cleanup_refusal: None,
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length { symbols } => write!(
                f,
                "visual fingerprint symbol count {symbols} is zero or unrepresentable"
            ),
            Self::DigestLength { bytes } => write!(
                f,
                "full digest requires 64 hexadecimal bytes, received {bytes}"
            ),
            Self::Hex(cause) => cause.fmt(f),
            Self::Storage {
                cause,
                cleanup_refusal,
            } => {
                cause.fmt(f)?;
                if let Some(cleanup) = cleanup_refusal {
                    write!(f, "; cleanup also refused: {cleanup}")?;
                }
                Ok(())
            }
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Hex(cause) => Some(cause),
            Self::Storage { cause, .. } => Some(cause),
            _ => None,
        }
    }
}
impl purrdf_hash::error::StructuredError for Error {
    purrdf_hash::error_visit! {
        Length { symbols } => "visual-fingerprint-length" { "symbols" => Unsigned(*symbols as u128) },
        DigestLength { bytes } => "visual-fingerprint-digest-length" { "bytes" => ByteCount(*bytes as u128) },
        Hex(cause) => "visual-fingerprint-hex" { "cause" => Cause(cause) },
        Storage { cause, cleanup_refusal } => "visual-fingerprint-storage" {
            "cause" => Cause(cause),
            "cleanup_refusal" => Value(cleanup_refusal.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |error| purrdf_hash::error::ErrorValue::Cause(error))),
        },
    }
}

/// Decode either hexadecimal case through the shared reader, without changing
/// `Digest32`'s canonical-lowercase parser. No spelling is hashed.
pub fn parse_digest<S: Admission + ?Sized>(
    text: &str,
    admission: &mut S,
) -> Result<Digest32, Error> {
    if text.len() != Digest32::LEN * 2 {
        return Err(Error::DigestLength { bytes: text.len() });
    }
    admission.checkpoint(text.len() as u64)?;
    let mut bytes = [0; Digest32::LEN];
    hex::decode_to_slice(text, &mut bytes).map_err(Error::Hex)?;
    Ok(Digest32::new(bytes))
}

#[derive(Default)]
struct Payload {
    emoji: String,
    spoken: String,
    art: String,
}

/// Complete raw-byte rendering retained under its original admission owner.
/// This lower entry is for existing public-key renderings, not content spelling.
pub struct Rendering<S: Admission> {
    symbols: usize,
    payload: Payload,
    // Payloads above die before the original account's final cleanup.
    admission: OwnedAdmission<S>,
}
purrdf_hash::debug_non_exhaustive!([S: Admission] Rendering<S> {});
impl<S: Admission> Rendering<S> {
    /// Frozen algorithm and alphabet identity.
    pub const fn scheme(&self) -> &'static str {
        SCHEME
    }
    /// Original requested symbol count.
    pub const fn symbols(&self) -> usize {
        self.symbols
    }
    /// Space-separated emoji.
    pub fn emoji(&self) -> &str {
        &self.payload.emoji
    }
    /// Space-separated stable names for the exact same symbol indices.
    pub fn spoken(&self) -> &str {
        &self.payload.spoken
    }
    /// Original bishop-walk ASCII art, including upstream header and borders.
    pub fn randomart(&self) -> &str {
        &self.payload.art
    }
    /// Destroy all output before refunding its account, reporting a refused
    /// cleanup. Drop also attempts cleanup without claiming a successful refund.
    pub fn release(mut self) -> Result<(), StorageError> {
        drop(mem::take(&mut self.payload));
        self.admission.resize(0)
    }
}
impl Rendering<Resident> {
    /// Move resident payload into an existing host result. Resident carries no
    /// borrowed external grant; this operation is unavailable to quota owners.
    pub fn into_strings(mut self) -> (String, String, String) {
        let Payload { emoji, spoken, art } = mem::take(&mut self.payload);
        (emoji, spoken, art)
    }
}

/// Full authoritative identity beside its human comparison aids.
pub struct Presentation<S: Admission> {
    digest: Digest32,
    rendering: Rendering<S>,
}
purrdf_hash::debug_non_exhaustive!([S: Admission] Presentation<S> {});
impl<S: Admission> Presentation<S> {
    /// Original full identity, unchanged by its presentation.
    pub const fn digest(&self) -> Digest32 {
        self.digest
    }
    /// Immutable rendering with its retained original account.
    pub const fn rendering(&self) -> &Rendering<S> {
        &self.rendering
    }
    /// Destroy output and explicitly report any refused cleanup.
    pub fn release(self) -> Result<(), StorageError> {
        self.rendering.release()
    }
}
/// Present the exact canonical digest bytes, never a formatted representation.
pub fn present<S: Admission>(
    digest: Digest32,
    request: Request<'_>,
    admission: S,
) -> Result<Presentation<S>, Error> {
    render_bytes(digest.as_bytes(), request, admission)
        .map(|rendering| Presentation { digest, rendering })
}

fn xof_bytes(symbols: usize) -> Result<u64, Error> {
    u64::try_from((symbols as u128 * 6).div_ceil(8)).map_err(|_| Error::Length { symbols })
}

struct Symbols {
    reader: blake3::OutputReader,
    bytes: [u8; 64],
    at: usize,
    count: usize,
    acc: u16,
    bits: u8,
    remaining: usize,
    xof_left: u64,
}
impl Symbols {
    fn new(reader: blake3::OutputReader, symbols: usize, bytes: u64) -> Self {
        Self {
            reader,
            bytes: [0; 64],
            at: 0,
            count: 0,
            acc: 0,
            bits: 0,
            remaining: symbols,
            xof_left: bytes,
        }
    }
    fn next<S: Admission + ?Sized>(
        &mut self,
        account: &mut S,
    ) -> Result<Option<usize>, StorageError> {
        if self.remaining == 0 {
            return Ok(None);
        }
        account.checkpoint(1)?;
        while self.bits < 6 {
            if self.at == self.count {
                self.count = usize::try_from(self.xof_left.min(64))
                    .map_err(|_| StorageError::SizeOverflow)?;
                account.checkpoint(self.count as u64)?;
                self.reader.fill(&mut self.bytes[..self.count]);
                self.xof_left -= self.count as u64;
                self.at = 0;
            }
            self.acc = (self.acc << 8) | u16::from(self.bytes[self.at]);
            self.at += 1;
            self.bits += 8;
        }
        self.bits -= 6;
        let index = usize::from((self.acc >> self.bits) & 63);
        self.acc &= (1 << self.bits) - 1;
        self.remaining -= 1;
        Ok(Some(index))
    }
}

enum Art<'a> {
    Work(u64),
    Text(&'a str),
    Char(char),
}
fn art(
    data: &[u8],
    label: &str,
    mut emit: impl FnMut(Art<'_>) -> Result<(), StorageError>,
) -> Result<usize, StorageError> {
    let mut grid = [[0_u8; WIDTH]; HEIGHT];
    let (mut x, mut y) = (WIDTH / 2, HEIGHT / 2);
    for &byte in data {
        emit(Art::Work(4))?;
        for shift in [0, 2, 4, 6] {
            let step = (byte >> shift) & 3;
            x = if step & 1 == 0 {
                x.saturating_sub(1)
            } else {
                (x + 1).min(WIDTH - 1)
            };
            y = if step & 2 == 0 {
                y.saturating_sub(1)
            } else {
                (y + 1).min(HEIGHT - 1)
            };
            // Only the ramp index is observable; saturation avoids overflowing
            // counters while preserving every upstream valid-input rendering.
            grid[y][x] = grid[y][x].saturating_add(1).min((RAMP.len() - 1) as u8);
        }
    }
    let mut chars = 0usize;
    if label.is_empty() {
        emit(Art::Text(BORDER))?;
    } else {
        emit(Art::Text("+--["))?;
        emit(Art::Text(label))?;
        for _ in label.chars() {
            emit(Art::Work(1))?;
            chars += 1;
        }
        for _ in chars..14 {
            emit(Art::Char(' '))?;
        }
        emit(Art::Text("]+"))?;
    }
    for row in 0..HEIGHT {
        emit(Art::Text("\n|"))?;
        for col in 0..WIDTH {
            emit(Art::Work(1))?;
            let byte = if (col, row) == (WIDTH / 2, HEIGHT / 2) {
                b'S'
            } else if (col, row) == (x, y) {
                b'E'
            } else {
                RAMP[usize::from(grid[row][col])]
            };
            emit(Art::Char(char::from(byte)))?;
        }
        emit(Art::Char('|'))?;
    }
    emit(Art::Char('\n'))?;
    emit(Art::Text(BORDER))?;
    Ok(chars)
}

/// Render original bytes through the one native implementation. Content
/// callers use [`present`]; legacy key callers explicitly retain their raw bytes.
/// Every scan and native copy is admitted, and refusal publishes no partial model.
pub fn render_bytes<S: Admission>(
    data: &[u8],
    request: Request<'_>,
    admission: S,
) -> Result<Rendering<S>, Error> {
    let bytes = xof_bytes(request.symbols())?;
    // No multiplication may wrap before work control can stop a huge request.
    let input_work = u64::try_from(data.len()).map_err(|_| StorageError::SizeOverflow)?;
    input_work
        .checked_mul(9)
        .ok_or(StorageError::SizeOverflow)?;
    bytes
        .checked_mul(2)
        .and_then(|n| {
            u64::try_from(request.symbols())
                .ok()?
                .checked_mul(2)?
                .checked_add(n)
        })
        .ok_or(StorageError::SizeOverflow)?;
    let mut owner = OwnedAdmission::new(admission);
    let mut payload = Payload::default();
    let result = (|| -> Result<(), StorageError> {
        let mut memory = Memory::new(&mut owner);
        memory.admission_mut().checkpoint(1)?;
        let mut hash = blake3::Hasher::new();
        for chunk in data.chunks(256) {
            memory.admission_mut().checkpoint(chunk.len() as u64)?;
            hash.update(chunk);
        }
        let reader = hash.finalize_xof();
        let mut symbols = Symbols::new(reader.clone(), request.symbols(), bytes);
        let mut emoji_size = 0usize;
        let mut spoken_size = 0usize;
        let mut first = true;
        while let Some(index) = symbols.next(memory.admission_mut())? {
            let separator = usize::from(!first);
            emoji_size = emoji_size
                .checked_add(EMOJI[index].len() + separator)
                .ok_or(StorageError::SizeOverflow)?;
            spoken_size = spoken_size
                .checked_add(LABELS[index].len() + separator)
                .ok_or(StorageError::SizeOverflow)?;
            memory
                .admission_mut()
                .checkpoint((EMOJI[index].len() + LABELS[index].len() + separator * 2) as u64)?;
            first = false;
        }
        let mut art_size = 0usize;
        let label_chars = art(data, request.label, |event| {
            let bytes = match event {
                Art::Work(units) => return memory.admission_mut().checkpoint(units),
                Art::Text(text) => text.len(),
                Art::Char(c) => c.len_utf8(),
            };
            memory.admission_mut().checkpoint(bytes as u64)?;
            art_size = art_size
                .checked_add(bytes)
                .ok_or(StorageError::SizeOverflow)?;
            Ok(())
        })?;
        let output_size = emoji_size
            .checked_add(spoken_size)
            .and_then(|n| n.checked_add(art_size))
            .ok_or(StorageError::SizeOverflow)?;
        // Exact whole-operation units: hash input once; both symbol/XOF scans,
        // both count/copy passes and both raw-input bishop walks. Compute in
        // u128 before storage so no admitted counter can be asked to wrap.
        let total_work = u128::from(input_work) * 9
            + request.symbols() as u128 * 2
            + u128::from(bytes) * 2
            + output_size as u128 * 2
            + label_chars as u128 * 2
            + (WIDTH * HEIGHT * 2) as u128
            + 1;
        u64::try_from(total_work).map_err(|_| StorageError::SizeOverflow)?;
        memory.reserve_string(&mut payload.emoji, emoji_size)?;
        memory.reserve_string(&mut payload.spoken, spoken_size)?;
        memory.reserve_string(&mut payload.art, art_size)?;
        let mut symbols = Symbols::new(reader, request.symbols(), bytes);
        let mut first = true;
        while let Some(index) = symbols.next(memory.admission_mut())? {
            if !first {
                memory.push_str(&mut payload.emoji, " ")?;
                memory.push_str(&mut payload.spoken, " ")?;
            }
            memory.push_str(&mut payload.emoji, EMOJI[index])?;
            memory.push_str(&mut payload.spoken, LABELS[index])?;
            first = false;
        }
        art(data, request.label, |event| match event {
            Art::Work(units) => memory.admission_mut().checkpoint(units),
            Art::Text(text) => memory.push_str(&mut payload.art, text),
            Art::Char(c) => memory.push_char(&mut payload.art, c),
        })?;
        Ok(())
    })();
    if let Err(cause) = result {
        drop(payload);
        let cleanup_refusal = owner.resize(0).err();
        return Err(Error::Storage {
            cause,
            cleanup_refusal,
        });
    }
    Ok(Rendering {
        symbols: request.symbols(),
        payload,
        admission: owner,
    })
}
