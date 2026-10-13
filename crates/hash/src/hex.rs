// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Base16 (RFC 4648 §8): the workspace's one hex encoder, decoder and digit
//! reader, and the 32-byte digest value every content identity is.
//!
//! | Operation | Items |
//! |---|---|
//! | render, lowercase | [`Lower`] (`Display`), [`encode`], [`encode_into`], [`encode_to_slice`] |
//! | render, uppercase | [`Upper`] (`Display`), [`encode_upper`], [`encode_upper_into`], [`encode_upper_to_slice`] |
//! | read, either case | [`decode`], [`decode_32`], [`nibble`], [`parse_u32`] |
//! | read, canonical lowercase | [`decode_canonical`], [`decode_32_canonical`], [`nibble_canonical`] |
//! | a 32-byte digest | [`Digest32`] |
//!
//! Each byte renders as two digits, high nibble first, with no separator and
//! no prefix. Lowercase is the rendering of every digest and content address;
//! uppercase is the canonical form some grammars specify (the `xsd:hexBinary`
//! canonical representation, a percent-encoding triplet, a `\u00XX` escape).
//!
//! ```
//! use purrdf_hash::hex::{self, Lower, Upper};
//!
//! assert_eq!(Lower(b"foobar").to_string(), "666f6f626172");
//! assert_eq!(format!("sha256:{}", Lower(&[0x00, 0xff])), "sha256:00ff");
//! assert_eq!(Upper(&[0x0f, 0xa0]).to_string(), "0FA0");
//! assert_eq!(hex::decode("0fA0"), Ok(vec![0x0f, 0xa0]));
//! assert_eq!(hex::nibble(b'e'), Some(14));
//! ```
//!
//! # Reading
//!
//! Two lexical spaces are read, because two are specified:
//!
//! * **either case** ([`decode`], [`decode_32`], [`nibble`]): RFC 4648 §8
//!   calls base16 case-insensitive, and `xsd:hexBinary`'s lexical space
//!   (XML Schema 1.1 Part 2 §3.3.15) is `([0-9a-fA-F]{2})*`;
//! * **canonical lowercase** ([`decode_canonical`], [`decode_32_canonical`],
//!   [`nibble_canonical`]): a content address, a digest label or an escape
//!   that must have exactly one spelling per value. An uppercase digit is
//!   malformed there like any other non-digit byte, so a value never has two
//!   accepted spellings.
//!
//! Every reader is strict: no sign, no `0x` prefix, no whitespace, no
//! separator, and an odd number of digits is refused. A refusal names what is
//! wrong through [`HexError`], with the byte offset of the first bad digit.
//!
//! # Performance
//!
//! Rendering maps each nibble to its digit by comparisons only: `n + b'0'` is
//! the digit for `n <= 9`, and a letter adds a constant selected by a
//! compare mask, with no branch and no table load, so the loop lowers to
//! packed byte compares, adds and an interleave on every vector target, the
//! baseline x86-64 SSE2 included. It is the one body per length class:
//!
//! * up to [`SHORT_MAX`] bytes (every SHA-1, SHA-256 and BLAKE3 digest), the
//!   loop runs inline at the call site with no dispatch: no indirect call
//!   and no selected-path lookup for the rendering of a digest;
//! * longer inputs run [`HexBackend::selected`](crate::backend::HexBackend):
//!   SSSE3 `pshufb` on x86-64, NEON `tbl` on AArch64 and wasm
//!   `i8x16.swizzle` in a `simd128` build; each kernel's tail is the loop.
//!
//! The `hex` group of the `purrdf-hash-conformance` `digests` bench measures
//! each path and each public entry point at 8 B to 4 KiB. Every path writes
//! the same bytes, checked against the frozen table
//! `crates/hash-conformance/tests/vectors/hex_vectors.txt`.
//!
//! Reading classifies and values sixteen digits at a time by comparisons
//! only — `b - b'0' < 10` is a decimal digit, `b - b'a' < 6` a letter (with
//! `b | 0x20` folding `A`-`F` onto `a`-`f` in the either-case space) — each
//! test a `0x00`/`0xFF` byte lane; a non-digit sets its lane of a
//! sixteen-lane rejection accumulator OR-ed across the chunks and folded once
//! after the last, and the offset of the first bad digit is found only on
//! that cold path. A byte at or above `0x80` is neither a digit nor a letter,
//! so non-ASCII input is refused.

use core::fmt::{self, Alignment, Display, Formatter, Write as _};

use std::sync::OnceLock;

use crate::arch::HexEncode;
use crate::backend::HexBackend;
use crate::dispatch::Backend as _;

/// The lowercase base16 alphabet: RFC 4648 §8's, with `a`–`f` for 10–15.
/// The vector kernels' lookup table.
#[cfg(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    all(target_arch = "wasm32", target_feature = "simd128")
))]
pub(crate) const ALPHABET: &[u8; 16] = b"0123456789abcdef";

/// The uppercase base16 alphabet, as RFC 4648 §8 prints it.
#[cfg(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    all(target_arch = "wasm32", target_feature = "simd128")
))]
pub(crate) const UPPER_ALPHABET: &[u8; 16] = b"0123456789ABCDEF";

/// The longest input, in bytes, encoded inline at the call site by the
/// compare-select loop without consulting
/// [`HexBackend::selected`](crate::backend::HexBackend): a 32-byte digest is
/// the longest the workspace renders.
pub const SHORT_MAX: usize = 32;

/// Input bytes rendered per `write_str` by [`Lower`] and [`Upper`].
const CHUNK: usize = 128;

/// Digits classified per step of a decode: one 128-bit vector of bytes.
const LANES: usize = 16;

/// Why a base16 read or a slice write was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HexError {
    /// The text holds an odd number of digits, so its last byte is half a
    /// pair.
    OddLength {
        /// The number of bytes in the text.
        len: usize,
    },
    /// A byte of the text is not a digit of the lexical space being read.
    InvalidDigit {
        /// The byte offset of the first such byte.
        offset: usize,
        /// The byte itself.
        byte: u8,
    },
    /// An output slice cannot hold the rendering.
    OutputTooShort {
        /// The number of bytes the rendering needs.
        needed: usize,
        /// The number of bytes the slice has.
        available: usize,
    },
}

impl Display for HexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Self::OddLength { len } => {
                write!(f, "hex text of {len} bytes has an odd number of digits")
            }
            Self::InvalidDigit { offset, byte } if byte.is_ascii_graphic() => write!(
                f,
                "`{}` at byte {offset} is not a hex digit",
                char::from(byte)
            ),
            Self::InvalidDigit { offset, byte } => {
                write!(f, "byte {byte:#04x} at offset {offset} is not a hex digit")
            }
            Self::OutputTooShort { needed, available } => write!(
                f,
                "a hex rendering of {needed} bytes does not fit in {available}"
            ),
        }
    }
}

impl core::error::Error for HexError {}

// --- Rendering ---------------------------------------------------------------

/// The digit of nibble `n` (`0..16`), as comparisons only.
///
/// `n + b'0'` is the digit for `n <= 9`; the lowercase letters start 39 bytes
/// after `b'9' + 1` and the uppercase ones 7, so a nibble above 9 adds that
/// much more. The comparison yields a `0x00`/`0xFF` byte and the addend is
/// selected by masking, with no branch, so a loop over nibbles lowers to
/// packed byte compares and adds.
#[inline]
const fn digit<const UPPER: bool>(n: u8) -> u8 {
    let letter = ((n > 9) as u8).wrapping_neg();
    let addend = if UPPER {
        b'A' - b'9' - 1
    } else {
        b'a' - b'9' - 1
    };
    n + b'0' + (letter & addend)
}

/// The compare-select loop: `output` holds at least `2 * input.len()` bytes.
#[inline]
fn encode_scalar<const UPPER: bool>(input: &[u8], output: &mut [u8]) {
    let (pairs, _) = output.as_chunks_mut::<2>();
    for (pair, &byte) in pairs.iter_mut().zip(input) {
        *pair = [digit::<UPPER>(byte >> 4), digit::<UPPER>(byte & 0x0f)];
    }
}

/// The portable encoder behind [`HexBackend::Portable`], and the tail of every
/// vector kernel: `output` must be exactly twice as long as `input`.
pub(crate) fn encode_portable(input: &[u8], output: &mut [u8], upper: bool) {
    debug_assert_eq!(output.len(), 2 * input.len());
    if upper {
        encode_scalar::<true>(input, output);
    } else {
        encode_scalar::<false>(input, output);
    }
}

/// The length switch: short inputs inline, longer ones on the selected path.
/// `output` is exactly twice as long as `input`.
#[inline]
pub(crate) fn encode_bytes<const UPPER: bool>(input: &[u8], output: &mut [u8]) {
    debug_assert_eq!(output.len(), 2 * input.len());
    if input.len() <= SHORT_MAX {
        encode_scalar::<UPPER>(input, output);
    } else {
        (selected_encode())(input, output, UPPER);
    }
}

/// The encoder of [`HexBackend::selected`], chosen once.
///
/// Selection walks every path's availability check (a feature-detection read
/// each) and filters the result; the answer cannot change for the life of
/// the process, so it is resolved on first use and read back as one atomic
/// load.
#[inline]
fn selected_encode() -> HexEncode {
    static SELECTED: OnceLock<HexEncode> = OnceLock::new();
    *SELECTED.get_or_init(|| HexBackend::selected().encode_fn())
}

/// A byte string rendered as lowercase base16 by [`Display`].
///
/// Construct it with any byte slice, including a `&[u8; N]` or a digest
/// array, which coerce: `Lower(&digest)`. Formatter width, fill, alignment and
/// precision apply to the rendered characters as they do to a `str`.
/// Rendering never allocates: the digits are produced in a stack buffer and
/// handed to the formatter one `write_str` per 128 input bytes.
#[derive(Clone, Copy, Debug)]
pub struct Lower<'a>(pub &'a [u8]);

/// A byte string rendered as uppercase base16 by [`Display`]: [`Lower`]'s
/// rendering with `A`–`F` for 10–15.
#[derive(Clone, Copy, Debug)]
pub struct Upper<'a>(pub &'a [u8]);

impl Display for Lower<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        render::<false>(f, self.0)
    }
}

impl Display for Upper<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        render::<true>(f, self.0)
    }
}

/// `str` formatting semantics over the rendering of `bytes`: precision caps
/// the characters written, width pads what remains, left-aligned unless the
/// formatter says otherwise.
fn render<const UPPER: bool>(f: &mut Formatter<'_>, bytes: &[u8]) -> fmt::Result {
    if f.width().is_none() && f.precision().is_none() {
        return write_chars::<UPPER>(f, bytes, usize::MAX);
    }
    let total = bytes.len().saturating_mul(2);
    let shown = f
        .precision()
        .map_or(total, |precision| total.min(precision));
    let padding = f.width().map_or(0, |width| width.saturating_sub(shown));
    let (before, after) = match f.align() {
        None | Some(Alignment::Left) => (0, padding),
        Some(Alignment::Right) => (padding, 0),
        Some(Alignment::Center) => (padding / 2, padding - padding / 2),
    };
    let fill = f.fill();
    for _ in 0..before {
        f.write_char(fill)?;
    }
    write_chars::<UPPER>(f, bytes, shown)?;
    for _ in 0..after {
        f.write_char(fill)?;
    }
    Ok(())
}

/// Writes the first `limit` characters of the rendering of `bytes`.
fn write_chars<const UPPER: bool>(
    f: &mut impl fmt::Write,
    bytes: &[u8],
    mut limit: usize,
) -> fmt::Result {
    let mut buffer = [0u8; 2 * CHUNK];
    for chunk in bytes.chunks(CHUNK) {
        if limit == 0 {
            break;
        }
        let text = crate::arch::encode_text::<UPPER>(chunk, &mut buffer[..2 * chunk.len()]);
        // Every character is one ASCII byte, so any prefix is a `str`.
        let take = text.len().min(limit);
        f.write_str(&text[..take])?;
        limit -= take;
    }
    Ok(())
}

/// `bytes` as an owned lowercase base16 string of exactly `2 * bytes.len()`
/// characters; empty for an empty slice. Leading zero bytes keep their two
/// digits.
///
/// ```
/// assert_eq!(purrdf_hash::hex::encode(&[0x00, 0x0f, 0xff]), "000fff");
/// assert_eq!(purrdf_hash::hex::encode(&[]), "");
/// ```
#[must_use]
pub fn encode(bytes: &[u8]) -> String {
    encode_owned::<false>(bytes)
}

/// `bytes` as an owned uppercase base16 string: [`encode`] with `A`–`F`.
///
/// ```
/// assert_eq!(purrdf_hash::hex::encode_upper(&[0x00, 0x0f, 0xff]), "000FFF");
/// ```
#[must_use]
pub fn encode_upper(bytes: &[u8]) -> String {
    encode_owned::<true>(bytes)
}

#[inline]
fn encode_owned<const UPPER: bool>(bytes: &[u8]) -> String {
    crate::arch::encode_string::<UPPER>(bytes)
}

/// Appends the lowercase base16 rendering of `bytes` to `out`, for a caller
/// building one string out of several parts.
///
/// ```
/// let mut label = String::from("sha256:");
/// purrdf_hash::hex::encode_into(&[0xab, 0x01], &mut label);
/// assert_eq!(label, "sha256:ab01");
/// ```
pub fn encode_into(bytes: &[u8], out: &mut String) {
    out.reserve(2 * bytes.len());
    // `String`'s `fmt::Write` never fails.
    let _ = write_chars::<false>(out, bytes, usize::MAX);
}

/// Appends the uppercase base16 rendering of `bytes` to `out`: [`encode_into`]
/// with `A`–`F`.
pub fn encode_upper_into(bytes: &[u8], out: &mut String) {
    out.reserve(2 * bytes.len());
    let _ = write_chars::<true>(out, bytes, usize::MAX);
}

/// Writes the lowercase base16 rendering of `bytes` into the first
/// `2 * bytes.len()` bytes of `out` and returns them as text, for a caller
/// rendering into a fixed buffer or a byte sink without allocating.
///
/// # Errors
///
/// [`HexError::OutputTooShort`], writing nothing, when `out` is shorter than
/// `2 * bytes.len()`.
///
/// ```
/// let mut buffer = [0u8; 8];
/// assert_eq!(purrdf_hash::hex::encode_to_slice(&[0xc3, 0xa9], &mut buffer), Ok("c3a9"));
/// ```
pub fn encode_to_slice<'o>(bytes: &[u8], out: &'o mut [u8]) -> Result<&'o str, HexError> {
    encode_slice::<false>(bytes, out)
}

/// [`encode_to_slice`] with `A`–`F`.
///
/// # Errors
///
/// [`HexError::OutputTooShort`], writing nothing, when `out` is shorter than
/// `2 * bytes.len()`.
///
/// ```
/// let mut buffer = [0u8; 2];
/// assert_eq!(purrdf_hash::hex::encode_upper_to_slice(&[0x7f], &mut buffer), Ok("7F"));
/// ```
pub fn encode_upper_to_slice<'o>(bytes: &[u8], out: &'o mut [u8]) -> Result<&'o str, HexError> {
    encode_slice::<true>(bytes, out)
}

#[inline]
fn encode_slice<'o, const UPPER: bool>(
    bytes: &[u8],
    out: &'o mut [u8],
) -> Result<&'o str, HexError> {
    let needed = bytes.len().saturating_mul(2);
    let available = out.len();
    let Some(window) = out.get_mut(..needed) else {
        return Err(HexError::OutputTooShort { needed, available });
    };
    Ok(crate::arch::encode_text::<UPPER>(bytes, window))
}

// --- Reading -----------------------------------------------------------------

/// Classifies and values `N` digits by comparisons and masks only: each
/// lane's nibble into `values`, and a non-digit's `0xFF` OR-ed into its lane
/// of `rejected` (`0x00` for a digit). `ANY_CASE` accepts `A`–`F` as well as
/// `a`–`f`. The one digit classification every reader runs: sixteen lanes in
/// the decoders, where the loop packs into byte compares, one in [`nibble`].
///
/// Always inlined: a sixteen-lane instance left out of line is four calls per
/// 64-digit read with its lanes spilled through memory between them, and the
/// compares are then no longer in the decoder the assembly gate measures
/// (`hash.hex-decode`).
#[allow(
    clippy::inline_always,
    reason = "the byte-compare lanes must be in the decoder that runs them; an out-of-line \
              instance spills every lane through memory"
)]
#[inline(always)]
fn classify<const ANY_CASE: bool, const N: usize>(
    digits: &[u8; N],
    values: &mut [u8; N],
    rejected: &mut [u8; N],
) {
    for ((value, reject), &b) in values.iter_mut().zip(rejected.iter_mut()).zip(digits) {
        let decimal = b.wrapping_sub(b'0');
        let letter = (if ANY_CASE { b | 0x20 } else { b }).wrapping_sub(b'a');
        let is_decimal = u8::from(decimal < 10).wrapping_neg();
        let is_letter = u8::from(letter < 6).wrapping_neg();
        *value = (decimal & is_decimal) | (letter.wrapping_add(10) & is_letter);
        *reject |= !(is_decimal | is_letter);
    }
}

/// One digit through [`classify`].
#[inline]
fn digit_value<const ANY_CASE: bool>(byte: u8) -> Option<u8> {
    let (mut value, mut rejected) = ([0u8], [0u8]);
    classify::<ANY_CASE, 1>(&[byte], &mut value, &mut rejected);
    (rejected[0] == 0).then_some(value[0])
}

/// The value of one hex digit of either case, or `None` for any other byte.
///
/// Strict: a digit is one of `0`–`9`, `a`–`f`, `A`–`F` and nothing else (no
/// sign, no whitespace, no non-ASCII byte). The digit reader of every grammar
/// that spells a code point or a byte in hex (`\uXXXX`, `%XX`, `&#xX;`).
///
/// ```
/// use purrdf_hash::hex::nibble;
///
/// assert_eq!(nibble(b'7'), Some(7));
/// assert_eq!(nibble(b'b'), Some(11));
/// assert_eq!(nibble(b'B'), Some(11));
/// assert_eq!(nibble(b'g'), None);
/// assert_eq!(nibble(b'+'), None);
/// ```
#[must_use]
#[inline]
pub fn nibble(byte: u8) -> Option<u8> {
    digit_value::<true>(byte)
}

/// The number one or more hex digits of either case spell, most significant
/// first, or `None` when `digits` is empty, holds a byte that is not a digit,
/// or spells a value above [`u32::MAX`].
///
/// Strict like [`nibble`], which reads each digit: no sign, no `0x` prefix, no
/// whitespace, where `u32::from_str_radix` accepts a leading `+`. Leading
/// zeros are value-neutral and any number of them is accepted, as the grammars
/// that spell a code point in variable-length hex allow (`&#x0041;`,
/// ECMAScript's `\u{0041}`, a Unicode Character Database field).
///
/// ```
/// use purrdf_hash::hex::parse_u32;
///
/// assert_eq!(parse_u32(b"1F431"), Some(0x1_F431));
/// assert_eq!(parse_u32(b"000000000041"), Some(0x41));
/// assert_eq!(parse_u32(b"FFFFFFFF"), Some(u32::MAX));
/// assert_eq!(parse_u32(b"100000000"), None);
/// assert_eq!(parse_u32(b"+41"), None);
/// assert_eq!(parse_u32(b""), None);
/// ```
#[must_use]
pub fn parse_u32(digits: &[u8]) -> Option<u32> {
    if digits.is_empty() {
        return None;
    }
    digits.iter().try_fold(0_u32, |value, &byte| {
        let digit = nibble(byte)?;
        value.checked_mul(16)?.checked_add(u32::from(digit))
    })
}

/// The value of one canonical lowercase hex digit (`0`–`9`, `a`–`f`), or
/// `None` for any other byte, uppercase `A`–`F` included: the digit reader of
/// a form whose every value has exactly one spelling.
///
/// ```
/// use purrdf_hash::hex::nibble_canonical;
///
/// assert_eq!(nibble_canonical(b'b'), Some(11));
/// assert_eq!(nibble_canonical(b'B'), None);
/// ```
#[must_use]
#[inline]
pub fn nibble_canonical(byte: u8) -> Option<u8> {
    digit_value::<false>(byte)
}

/// Packs eight nibble pairs into eight bytes.
#[inline]
fn pack(values: &[u8; LANES], bytes: &mut [u8; LANES / 2]) {
    let (pairs, _) = values.as_chunks::<2>();
    for (byte, pair) in bytes.iter_mut().zip(pairs) {
        *byte = (pair[0] << 4) | pair[1];
    }
}

/// Whether no lane of the rejection accumulator is set.
#[inline]
fn clean(rejected: &[u8; LANES]) -> bool {
    rejected.iter().fold(0, |any, &lane| any | lane) == 0
}

/// Decodes `digits` (exactly `2 * out.len()` bytes) into `out`, sixteen
/// digits per step with a vertical rejection accumulator folded once after
/// the last. Returns whether every digit was one; on `false`, `out` holds
/// garbage.
#[inline]
fn decode_into<const ANY_CASE: bool>(digits: &[u8], out: &mut [u8]) -> bool {
    debug_assert_eq!(digits.len(), 2 * out.len());
    let (chunks, tail) = digits.as_chunks::<LANES>();
    let (outs, out_tail) = out.as_chunks_mut::<{ LANES / 2 }>();
    let mut rejected = [0u8; LANES];
    let mut values = [0u8; LANES];
    for (chunk, bytes) in chunks.iter().zip(outs.iter_mut()) {
        classify::<ANY_CASE, LANES>(chunk, &mut values, &mut rejected);
        pack(&values, bytes);
    }
    if !tail.is_empty() {
        let mut padded = [b'0'; LANES];
        padded[..tail.len()].copy_from_slice(tail);
        let mut bytes = [0u8; LANES / 2];
        classify::<ANY_CASE, LANES>(&padded, &mut values, &mut rejected);
        pack(&values, &mut bytes);
        out_tail.copy_from_slice(&bytes[..out_tail.len()]);
    }
    clean(&rejected)
}

/// The first byte of `digits` that is not a digit of the space, as an error.
#[cold]
fn first_invalid<const ANY_CASE: bool>(digits: &[u8]) -> HexError {
    digits
        .iter()
        .position(|&b| digit_value::<ANY_CASE>(b).is_none())
        .map_or(HexError::OddLength { len: digits.len() }, |offset| {
            HexError::InvalidDigit {
                offset,
                byte: digits[offset],
            }
        })
}

/// Validate either-case hex without heap allocation and return its byte length.
pub fn decoded_len(text: &str) -> Result<usize, HexError> {
    validated_len::<true>(text)
}

fn validated_len<const ANY_CASE: bool>(text: &str) -> Result<usize, HexError> {
    let digits = text.as_bytes();
    if !digits.len().is_multiple_of(2) {
        return Err(HexError::OddLength { len: digits.len() });
    }
    let mut output = [0_u8; LANES / 2];
    for chunk in digits.chunks(LANES) {
        if !decode_into::<ANY_CASE>(chunk, &mut output[..chunk.len() / 2]) {
            return Err(first_invalid::<ANY_CASE>(digits));
        }
    }
    Ok(digits.len() / 2)
}

/// Decode into caller-owned storage, without allocating or growing it.
/// Lexical/output errors leave the destination unchanged.
pub fn decode_to_slice(text: &str, output: &mut [u8]) -> Result<usize, HexError> {
    let needed = decoded_len(text)?;
    if output.len() < needed {
        return Err(HexError::OutputTooShort {
            needed,
            available: output.len(),
        });
    }
    if decode_into::<true>(text.as_bytes(), &mut output[..needed]) {
        Ok(needed)
    } else {
        Err(first_invalid::<true>(text.as_bytes()))
    }
}

#[inline]
fn decode_vec<const ANY_CASE: bool>(text: &str) -> Result<Vec<u8>, HexError> {
    let digits = text.as_bytes();
    if !digits.len().is_multiple_of(2) {
        return Err(HexError::OddLength { len: digits.len() });
    }
    let mut out = vec![0u8; digits.len() / 2];
    if decode_into::<ANY_CASE>(digits, &mut out) {
        Ok(out)
    } else {
        Err(first_invalid::<ANY_CASE>(digits))
    }
}

/// Reads base16 text of either case into bytes: the `xsd:hexBinary` lexical
/// space, `([0-9a-fA-F]{2})*`. The empty text is the empty byte string.
///
/// # Errors
///
/// [`HexError::OddLength`] for an odd number of bytes, and otherwise
/// [`HexError::InvalidDigit`] naming the first byte that is not a digit.
///
/// ```
/// use purrdf_hash::hex::{HexError, decode};
///
/// assert_eq!(decode("00fF"), Ok(vec![0x00, 0xff]));
/// assert_eq!(decode(""), Ok(vec![]));
/// assert_eq!(decode("abc"), Err(HexError::OddLength { len: 3 }));
/// assert_eq!(decode("0g"), Err(HexError::InvalidDigit { offset: 1, byte: b'g' }));
/// ```
pub fn decode(text: &str) -> Result<Vec<u8>, HexError> {
    decode_vec::<true>(text)
}

/// Reads canonical lowercase base16 text into bytes: [`decode`] refusing
/// `A`–`F` as it refuses any other non-digit.
///
/// # Errors
///
/// [`HexError::OddLength`] for an odd number of bytes, and otherwise
/// [`HexError::InvalidDigit`] naming the first byte that is not a lowercase
/// digit.
///
/// ```
/// use purrdf_hash::hex::{HexError, decode_canonical};
///
/// assert_eq!(decode_canonical("00ff"), Ok(vec![0x00, 0xff]));
/// assert_eq!(decode_canonical("00fF"), Err(HexError::InvalidDigit { offset: 3, byte: b'F' }));
/// ```
pub fn decode_canonical(text: &str) -> Result<Vec<u8>, HexError> {
    decode_vec::<false>(text)
}

/// The 64-digit reader: every chunk classified first, the accumulator folded
/// once, and only a clean input packed — no data-dependent branch inside
/// either fixed-length pass.
#[inline]
fn decode_fixed<const ANY_CASE: bool>(text: &str) -> Option<[u8; 32]> {
    let digits: &[u8; 64] = text.as_bytes().try_into().ok()?;
    let (chunks, _) = digits.as_chunks::<LANES>();
    let mut nibbles = [[0_u8; LANES]; 64 / LANES];
    let mut rejected = [0_u8; LANES];
    for (values, chunk) in nibbles.iter_mut().zip(chunks) {
        classify::<ANY_CASE, LANES>(chunk, values, &mut rejected);
    }
    if !clean(&rejected) {
        return None;
    }
    let mut out = [0_u8; 32];
    let pairs = nibbles.as_flattened().as_chunks::<2>().0;
    for (byte, pair) in out.iter_mut().zip(pairs) {
        *byte = (pair[0] << 4) | pair[1];
    }
    Some(out)
}

/// Reads exactly 64 digits of either case into 32 bytes, or `None` for any
/// other length or any non-digit.
///
/// ```
/// let text = "00".repeat(31) + "fF";
/// assert_eq!(purrdf_hash::hex::decode_32(&text).map(|d| d[31]), Some(0xff));
/// assert_eq!(purrdf_hash::hex::decode_32("ff"), None);
/// ```
#[must_use]
pub fn decode_32(text: &str) -> Option<[u8; 32]> {
    decode_fixed::<true>(text)
}

/// Reads exactly 64 canonical lowercase digits into 32 bytes, or `None` for
/// any other length, any uppercase digit, or any other non-digit: the reader
/// of a content address, whose every value has exactly one spelling.
///
/// ```
/// let text = "00".repeat(31) + "ff";
/// assert_eq!(purrdf_hash::hex::decode_32_canonical(&text).map(|d| d[31]), Some(0xff));
/// assert_eq!(purrdf_hash::hex::decode_32_canonical(&text.to_uppercase()), None);
/// ```
#[must_use]
pub fn decode_32_canonical(text: &str) -> Option<[u8; 32]> {
    decode_fixed::<false>(text)
}

// --- Digest32 ----------------------------------------------------------------

/// A 32-byte digest: the value of every 256-bit content identity (a BLAKE3 or
/// SHA-256 output), rendered as 64 lowercase digits and read back from
/// exactly that form.
///
/// A domain type wraps it to keep one identity from standing in for another
/// (`struct PlanId(Digest32)`); the storage, the rendering and the canonical
/// reading are this type's.
///
/// ```
/// use purrdf_hash::hex::Digest32;
///
/// let digest = Digest32::new([0xab; 32]);
/// assert_eq!(digest.to_string(), "ab".repeat(32));
/// assert_eq!(Digest32::from_hex(&"ab".repeat(32)), Some(digest));
/// assert_eq!(Digest32::from_hex(&"AB".repeat(32)), None);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Digest32([u8; 32]);

impl Digest32 {
    /// The digest length in bytes.
    pub const LEN: usize = 32;

    /// Wraps 32 digest bytes.
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The 32 digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The 32 digest bytes, by value.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Reads the canonical rendering: exactly 64 lowercase digits
    /// ([`decode_32_canonical`]).
    #[must_use]
    pub fn from_hex(text: &str) -> Option<Self> {
        decode_32_canonical(text).map(Self)
    }

    /// The canonical rendering: 64 lowercase digits.
    #[must_use]
    pub fn to_hex(&self) -> String {
        encode(&self.0)
    }
}

impl From<[u8; 32]> for Digest32 {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<Digest32> for [u8; 32] {
    fn from(digest: Digest32) -> Self {
        digest.0
    }
}

impl AsRef<[u8]> for Digest32 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Display for Digest32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&Lower(&self.0), f)
    }
}

impl fmt::Debug for Digest32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Digest32({})", Lower(&self.0))
    }
}
