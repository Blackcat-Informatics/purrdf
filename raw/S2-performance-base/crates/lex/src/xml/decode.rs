// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! From document bytes to the text the reader reads: XML 1.0 Appendix F's
//! encoding detection, for the encodings the reader supports.

use std::borrow::Cow;

use super::error::{XmlError, XmlErrorKind};

/// How a document's bytes are laid out, before any declaration is read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Layout {
    /// A UTF-8 byte order mark opens the document.
    Utf8Bom,
    /// A UTF-16 byte order mark opens the document.
    Utf16Bom { big_endian: bool },
    /// No byte order mark, but the first four bytes are `<?` in UTF-16.
    Utf16Bare { big_endian: bool },
    /// An ASCII-compatible encoding: UTF-8, US-ASCII, an ISO 8859 or a
    /// national encoding; the declaration says which.
    AsciiCompatible,
}

/// Decode a document's bytes to the text [`Reader`](super::Reader) and
/// [`Document`](super::Document) read, as XML 1.0 Appendix F detects the
/// encoding.
///
/// The supported encodings are **UTF-8**, **UTF-16** (either byte order,
/// byte order mark required unless the document opens `<?` in UTF-16 and
/// declares `UTF-16`), **US-ASCII** and **ISO-8859-1**. A document the
/// declaration or the byte pattern places in any other encoding (UCS-4,
/// EBCDIC, Shift_JIS, EUC-JP, ISO-2022-JP, and the rest) is refused with
/// [`XmlErrorKind::UnsupportedEncoding`] rather than read as though it were
/// UTF-8: the reader hard-fails and never mis-parses. A document whose bytes
/// contradict its declaration (UTF-16 bytes declared `UTF-8`, UTF-8 bytes
/// declared `UTF-16`, a byte above 0x7F under `US-ASCII`, ill-formed UTF-8 or
/// UTF-16) is refused with [`XmlErrorKind::Encoding`].
///
/// UTF-8 input without a declaration or with a UTF-8 declaration is borrowed,
/// not copied.
///
/// # Errors
///
/// [`XmlError`] as above; the offset is the byte where the encoding went
/// wrong (0 when the whole document is at issue).
pub fn decode(bytes: &[u8]) -> Result<Cow<'_, str>, XmlError> {
    let unsupported =
        |name: &str| XmlError::new(XmlErrorKind::UnsupportedEncoding(name.to_owned()), 0);
    let layout = match bytes {
        [0x00, 0x00, 0xFE, 0xFF, ..] | [0xFF, 0xFE, 0x00, 0x00, ..] => {
            return Err(unsupported("UTF-32"));
        }
        [0x00, 0x00, 0x00, 0x3C, ..]
        | [0x3C, 0x00, 0x00, 0x00, ..]
        | [0x00, 0x00, 0x3C, 0x00, ..]
        | [0x00, 0x3C, 0x00, 0x00, ..] => return Err(unsupported("UCS-4")),
        [0x4C, 0x6F, 0xA7, 0x94, ..] => return Err(unsupported("EBCDIC")),
        [0xEF, 0xBB, 0xBF, ..] => Layout::Utf8Bom,
        [0xFE, 0xFF, ..] => Layout::Utf16Bom { big_endian: true },
        [0xFF, 0xFE, ..] => Layout::Utf16Bom { big_endian: false },
        [0x00, 0x3C, 0x00, 0x3F, ..] => Layout::Utf16Bare { big_endian: true },
        [0x3C, 0x00, 0x3F, 0x00, ..] => Layout::Utf16Bare { big_endian: false },
        _ => Layout::AsciiCompatible,
    };
    match layout {
        Layout::Utf16Bom { big_endian } | Layout::Utf16Bare { big_endian } => {
            let bom = matches!(layout, Layout::Utf16Bom { .. });
            let text = utf16(bytes, big_endian)?;
            let declared = declared_encoding(text.as_bytes());
            match declared {
                Some(name) if !is_utf16(name) => Err(XmlError::new(
                    XmlErrorKind::Encoding(
                        "UTF-16 bytes with a declaration naming another encoding",
                    ),
                    0,
                )),
                None if !bom => Err(XmlError::new(
                    XmlErrorKind::Encoding(
                        "UTF-16 without a byte order mark or an encoding declaration",
                    ),
                    0,
                )),
                _ => Ok(Cow::Owned(text)),
            }
        }
        Layout::Utf8Bom | Layout::AsciiCompatible => {
            let body = if layout == Layout::Utf8Bom {
                &bytes[3..]
            } else {
                bytes
            };
            let declared = declared_encoding(body);
            let name = declared.unwrap_or("UTF-8");
            if is_utf16(name) {
                return Err(XmlError::new(
                    XmlErrorKind::Encoding("a UTF-16 declaration on a document that is not UTF-16"),
                    0,
                ));
            }
            if name.eq_ignore_ascii_case("UTF-8") {
                return std::str::from_utf8(bytes).map(Cow::Borrowed).map_err(|e| {
                    XmlError::new(XmlErrorKind::Encoding("ill-formed UTF-8"), e.valid_up_to())
                });
            }
            if layout == Layout::Utf8Bom {
                return Err(XmlError::new(
                    XmlErrorKind::Encoding(
                        "a UTF-8 byte order mark with a declaration naming another encoding",
                    ),
                    0,
                ));
            }
            if name.eq_ignore_ascii_case("US-ASCII") {
                return match bytes.iter().position(|b| !b.is_ascii()) {
                    Some(at) => Err(XmlError::new(
                        XmlErrorKind::Encoding("a byte above 0x7F in a US-ASCII document"),
                        at,
                    )),
                    // ASCII is UTF-8.
                    None => Ok(Cow::Borrowed(
                        std::str::from_utf8(bytes).expect("ASCII is UTF-8"),
                    )),
                };
            }
            if name.eq_ignore_ascii_case("ISO-8859-1") {
                return Ok(Cow::Owned(bytes.iter().map(|&b| char::from(b)).collect()));
            }
            Err(unsupported(name))
        }
    }
}

/// Whether `name` is a spelling of UTF-16 the reader accepts a UTF-16
/// document under.
fn is_utf16(name: &str) -> bool {
    name.eq_ignore_ascii_case("UTF-16")
}

/// UTF-16 `bytes` as text: a leading byte order mark is kept (the reader
/// skips U+FEFF), an odd byte count or an unpaired surrogate is refused.
fn utf16(bytes: &[u8], big_endian: bool) -> Result<String, XmlError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(XmlError::new(
            XmlErrorKind::Encoding("UTF-16 with an odd number of bytes"),
            bytes.len() - 1,
        ));
    }
    let units = bytes.as_chunks::<2>().0.iter().map(|&pair| {
        if big_endian {
            u16::from_be_bytes(pair)
        } else {
            u16::from_le_bytes(pair)
        }
    });
    let mut text = String::with_capacity(bytes.len() / 2);
    for (index, scalar) in char::decode_utf16(units).enumerate() {
        match scalar {
            Ok(c) => text.push(c),
            Err(_) => {
                return Err(XmlError::new(
                    XmlErrorKind::Encoding("an unpaired surrogate in UTF-16"),
                    index * 2,
                ));
            }
        }
    }
    Ok(text)
}

/// The `EncName` of the XML declaration that opens `bytes` (a byte order
/// mark already removed), when there is one. The declaration is ASCII in
/// every encoding the reader supports, so it is read from the bytes; a
/// malformed declaration reads as none here and is refused by the reader.
fn declared_encoding(bytes: &[u8]) -> Option<&str> {
    let rest = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let rest = rest.strip_prefix("\u{FEFF}".as_bytes()).unwrap_or(rest);
    if !rest.starts_with(b"<?xml") || !rest.get(5).is_some_and(|b| b" \t\r\n".contains(b)) {
        return None;
    }
    let close = rest.windows(2).position(|w| w == b"?>")?;
    let decl = &rest[5..close];
    let name_at = decl.windows(8).position(|w| w == b"encoding")?;
    // `version` may not contain "encoding"; a match preceded by a name
    // character is not the pseudo-attribute.
    if name_at > 0 && !b" \t\r\n\"'".contains(&decl[name_at - 1]) {
        return None;
    }
    let mut at = name_at + 8;
    let skip = |mut at: usize| {
        while decl.get(at).is_some_and(|b| b" \t\r\n".contains(b)) {
            at += 1;
        }
        at
    };
    at = skip(at);
    if decl.get(at) != Some(&b'=') {
        return None;
    }
    at = skip(at + 1);
    let quote = *decl.get(at)?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let end = decl[at + 1..].iter().position(|&b| b == quote)? + at + 1;
    std::str::from_utf8(&decl[at + 1..end]).ok()
}
