// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one error type every decoder in the crate reports.

use std::fmt;

/// Which Huffman alphabet a code-construction error is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alphabet {
    /// The code-length code of a dynamic block header (RFC 1951 §3.2.7).
    CodeLength,
    /// The literal/length alphabet (symbols 0–287).
    LiteralLength,
    /// The distance alphabet (symbols 0–31).
    Distance,
}

impl fmt::Display for Alphabet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::CodeLength => "code-length",
            Self::LiteralLength => "literal/length",
            Self::Distance => "distance",
        })
    }
}

/// A refused DEFLATE or gzip stream.
///
/// Every variant is a property of the input bytes (or of the caller's limit),
/// never of how the input was split into chunks. A decoder that has returned
/// an error keeps returning it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// A block header carries `BTYPE = 11`, which RFC 1951 §3.2.3 reserves.
    InvalidBlockType,
    /// A stored block's `NLEN` is not the one's complement of its `LEN`
    /// (RFC 1951 §3.2.4).
    StoredLengthMismatch {
        /// The `LEN` field.
        len: u16,
        /// The `NLEN` field.
        nlen: u16,
    },
    /// `HLIT` names more than the 286 literal/length codes RFC 1951 §3.2.7
    /// allows.
    TooManyLengthCodes {
        /// The number of literal/length code lengths the header declared.
        count: u16,
    },
    /// Code-length symbol 16 ("copy the previous length") appeared before any
    /// length had been read.
    RepeatWithoutPrevious,
    /// A code-length run extends past the `HLIT + HDIST` lengths the header
    /// declared.
    CodeLengthsOverrun,
    /// The code lengths over-subscribe the code space: no prefix code has them.
    OversubscribedCode {
        /// The alphabet whose lengths are impossible.
        alphabet: Alphabet,
    },
    /// The literal/length code gives the end-of-block symbol (256) no code, so
    /// the block could never end.
    MissingEndOfBlock,
    /// The data reached a bit pattern the block's (incomplete) code does not
    /// assign.
    InvalidCode {
        /// The alphabet the pattern was read in.
        alphabet: Alphabet,
    },
    /// Literal/length symbol 286 or 287 occurred in the data (RFC 1951
    /// §3.2.5: they "will never actually occur").
    InvalidLengthSymbol {
        /// The symbol read.
        symbol: u16,
    },
    /// Distance symbol 30 or 31 occurred in the data.
    InvalidDistanceSymbol {
        /// The symbol read.
        symbol: u16,
    },
    /// A back-reference reaches before the first byte of the stream.
    DistanceTooFar {
        /// The distance the match names.
        distance: u32,
        /// How many bytes the stream had produced when it was read.
        available: u64,
    },
    /// The input ended inside a stream, a block, a gzip header or a trailer.
    Truncated,
    /// Decoding would produce more than the caller's output limit.
    LimitExceeded {
        /// The limit, in decompressed bytes.
        limit: u64,
    },
    /// Bytes follow the end of a raw DEFLATE stream decoded one-shot.
    TrailingData,
    /// The input does not begin with the gzip identification bytes `1f 8b`.
    NotGzip,
    /// A gzip member names a compression method other than 8 (deflate).
    UnsupportedMethod {
        /// The `CM` byte.
        method: u8,
    },
    /// A gzip member sets one of the reserved `FLG` bits 5–7, which RFC 1952
    /// §2.3.1.2 requires a decoder to refuse.
    ReservedFlags {
        /// The `FLG` byte.
        flags: u8,
    },
    /// The header CRC16 (`FHCRC`) does not match the header bytes.
    HeaderCrcMismatch {
        /// The CRC16 the header carries.
        stored: u16,
        /// The low 16 bits of the CRC-32 of the header bytes.
        computed: u16,
    },
    /// A member's trailer CRC-32 does not match its decompressed bytes.
    CrcMismatch {
        /// The zero-based member index.
        member: u64,
        /// The trailer's CRC-32.
        stored: u32,
        /// The CRC-32 of the bytes decoded.
        computed: u32,
    },
    /// A member's trailer `ISIZE` does not equal its decompressed size modulo
    /// 2^32.
    SizeMismatch {
        /// The zero-based member index.
        member: u64,
        /// The trailer's `ISIZE`.
        stored: u32,
        /// The decoded size modulo 2^32.
        computed: u32,
    },
    /// Bytes after the last complete gzip member do not begin another member.
    TrailingGarbage {
        /// The offset of the first such byte within the whole input.
        offset: u64,
    },
    /// The input held no gzip member at all.
    Empty,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBlockType => f.write_str("reserved block type 11"),
            Self::StoredLengthMismatch { len, nlen } => write!(
                f,
                "stored block LEN {len:#06x} and NLEN {nlen:#06x} are not complements"
            ),
            Self::TooManyLengthCodes { count } => {
                write!(
                    f,
                    "{count} literal/length codes declared; at most 286 exist"
                )
            }
            Self::RepeatWithoutPrevious => {
                f.write_str("code-length repeat (16) with no previous length")
            }
            Self::CodeLengthsOverrun => {
                f.write_str("code-length run exceeds the declared number of lengths")
            }
            Self::OversubscribedCode { alphabet } => {
                write!(f, "over-subscribed {alphabet} code lengths")
            }
            Self::MissingEndOfBlock => f.write_str("end-of-block symbol has no code"),
            Self::InvalidCode { alphabet } => write!(f, "unassigned {alphabet} code in data"),
            Self::InvalidLengthSymbol { symbol } => {
                write!(
                    f,
                    "literal/length symbol {symbol} does not occur in valid data"
                )
            }
            Self::InvalidDistanceSymbol { symbol } => {
                write!(f, "distance symbol {symbol} does not occur in valid data")
            }
            Self::DistanceTooFar {
                distance,
                available,
            } => write!(
                f,
                "distance {distance} reaches before the stream start ({available} bytes produced)"
            ),
            Self::Truncated => f.write_str("input ended inside the stream"),
            Self::LimitExceeded { limit } => {
                write!(f, "decompressed output exceeds the {limit}-byte limit")
            }
            Self::TrailingData => f.write_str("bytes follow the end of the deflate stream"),
            Self::NotGzip => f.write_str("not a gzip member (missing 1f 8b)"),
            Self::UnsupportedMethod { method } => {
                write!(f, "gzip compression method {method} is not deflate (8)")
            }
            Self::ReservedFlags { flags } => {
                write!(f, "gzip FLG {flags:#04x} sets reserved bits")
            }
            Self::HeaderCrcMismatch { stored, computed } => write!(
                f,
                "gzip header CRC16 {stored:#06x} does not match {computed:#06x}"
            ),
            Self::CrcMismatch {
                member,
                stored,
                computed,
            } => write!(
                f,
                "gzip member {member}: CRC-32 {stored:#010x} does not match {computed:#010x}"
            ),
            Self::SizeMismatch {
                member,
                stored,
                computed,
            } => write!(
                f,
                "gzip member {member}: ISIZE {stored} does not match {computed} (mod 2^32)"
            ),
            Self::TrailingGarbage { offset } => write!(
                f,
                "bytes at offset {offset} after the last gzip member do not begin another member"
            ),
            Self::Empty => f.write_str("no gzip member in the input"),
        }
    }
}

impl std::error::Error for Error {}

impl From<Error> for std::io::Error {
    /// Truncation maps to `UnexpectedEof`, everything else to `InvalidData`; the
    /// typed error stays reachable through `get_ref` / `into_inner`.
    fn from(error: Error) -> Self {
        let kind = if error == Error::Truncated {
            std::io::ErrorKind::UnexpectedEof
        } else {
            std::io::ErrorKind::InvalidData
        };
        Self::new(kind, error)
    }
}
