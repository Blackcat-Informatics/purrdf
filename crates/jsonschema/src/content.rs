// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Draft-07 `contentEncoding` and `contentMediaType` as assertions
//! (draft-07 Validation §8).
//!
//! Draft-07 lets an implementation check that a string is in the encoding it
//! declares and that the decoded bytes are a document of the media type it
//! declares. This crate checks the one encoding and the one media type it can
//! decode completely: `base64` (RFC 2045 §6.8 over the RFC 4648 §4 alphabet)
//! and `application/json` (RFC 8259, checked by [`purrdf_lex::json::Reader`]). Any other
//! encoding or media type stays an annotation — never guessed at. In 2019-09
//! and 2020-12 these keywords are annotations only, and this module is not
//! used.

use purrdf_lex::json::{Limits, Object, Reader, Value};

/// A content check a schema object asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Content {
    /// The string is base64 and must decode.
    base64: bool,
    /// The (decoded) bytes must be a JSON text.
    json: bool,
}

impl Content {
    /// The check the object's `contentEncoding` and `contentMediaType`
    /// describe; `None` when there is nothing this crate can check (an
    /// unknown encoding makes the bytes, and so the media type, unknowable).
    pub(crate) fn from_keywords(map: &Object) -> Option<Self> {
        let base64 = match map.get("contentEncoding") {
            None => false,
            Some(encoding) => {
                // RFC 2045 §6.1: encoding names are case-insensitive.
                if !encoding
                    .as_str()
                    .is_some_and(|name| name.eq_ignore_ascii_case("base64"))
                {
                    return None;
                }
                true
            }
        };
        let json = map
            .get("contentMediaType")
            .and_then(Value::as_str)
            .is_some_and(is_json_media_type);
        (base64 || json).then_some(Self { base64, json })
    }

    /// Why `text` fails the check, if it does.
    pub(crate) fn check(self, text: &str) -> Result<(), String> {
        let decoded;
        let bytes = if self.base64 {
            decoded = decode_base64(text).ok_or("the string is not valid base64")?;
            decoded.as_slice()
        } else {
            text.as_bytes()
        };
        if self.json && !is_json_text(bytes) {
            return Err("the content is not a JSON document".to_owned());
        }
        Ok(())
    }
}

/// Whether `bytes` are one UTF-8 JSON text. The check is syntax only, and the
/// reader's container stack lives on the heap, so no nesting depth is refused
/// that the grammar admits.
fn is_json_text(bytes: &[u8]) -> bool {
    Reader::from_slice(bytes, Limits::with_depth(usize::MAX))
        .is_ok_and(|mut reader| reader.skip_value().is_ok() && reader.finish().is_ok())
}

/// `application/json`, in any case, with or without parameters (RFC 2045
/// §5.1: type and subtype are case-insensitive).
fn is_json_media_type(media_type: &str) -> bool {
    let essence = media_type.split(';').next().unwrap_or_default().trim();
    essence.eq_ignore_ascii_case("application/json")
}

/// Decode base64: the RFC 4648 §4 alphabet in groups of four, `=` padding
/// only at the end, line breaks (RFC 2045 §6.8 limits encoded lines) ignored.
/// Any other character is refused.
fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let symbols: Vec<u8> = text
        .bytes()
        .filter(|&byte| byte != b'\r' && byte != b'\n')
        .collect();
    if !symbols.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::with_capacity(symbols.len() / 4 * 3);
    let groups = symbols.len() / 4;
    for (index, group) in symbols.as_chunks::<4>().0.iter().enumerate() {
        let last = index + 1 == groups;
        let padding = group.iter().rev().take_while(|&&byte| byte == b'=').count();
        if padding > 2 || (padding > 0 && !last) {
            return None;
        }
        let mut word: u32 = 0;
        for &symbol in &group[..4 - padding] {
            word = (word << 6) | u32::from(sextet(symbol)?);
        }
        word <<= 6 * padding;
        let [_, high, middle, low] = word.to_be_bytes();
        out.extend_from_slice(&[high, middle, low][..3 - padding]);
    }
    Some(out)
}

const fn sextet(symbol: u8) -> Option<u8> {
    Some(match symbol {
        b'A'..=b'Z' => symbol - b'A',
        b'a'..=b'z' => symbol - b'a' + 26,
        b'0'..=b'9' => symbol - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_refuses_foreign_symbols_and_accepts_its_neighbours() {
        assert_eq!(decode_base64("Zm9v").as_deref(), Some(b"foo".as_slice()));
        assert_eq!(decode_base64("Zm8=").as_deref(), Some(b"fo".as_slice()));
        assert_eq!(decode_base64("Zg==").as_deref(), Some(b"f".as_slice()));
        assert_eq!(
            decode_base64("Zm9v\r\nYmFy").as_deref(),
            Some(b"foobar".as_slice())
        );
        assert_eq!(decode_base64(""), Some(Vec::new()));
        assert_eq!(decode_base64("Zm9%"), None);
        assert_eq!(decode_base64("Zm9"), None);
        assert_eq!(decode_base64("Zg==Zg=="), None);
        assert_eq!(decode_base64("Z==="), None);
    }

    #[test]
    fn only_json_media_types_are_parsed() {
        assert!(is_json_media_type("application/json"));
        assert!(is_json_media_type("Application/JSON; charset=utf-8"));
        assert!(!is_json_media_type("text/plain"));
        let mut map = Object::new();
        map.insert("contentMediaType".to_owned(), Value::from("text/plain"));
        assert_eq!(Content::from_keywords(&map), None);
        map.insert(
            "contentEncoding".to_owned(),
            Value::from("quoted-printable"),
        );
        assert_eq!(Content::from_keywords(&map), None);
        map.insert("contentEncoding".to_owned(), Value::from("BASE64"));
        let content = Content::from_keywords(&map).expect("base64 is checked");
        assert!(content.check("Zm9v").is_ok());
        assert!(content.check("{}").is_err());
    }

    #[test]
    fn json_content_is_checked_whole_and_at_any_depth() {
        let content = Content {
            base64: false,
            json: true,
        };
        assert!(content.check("[1, {\"a\": null}]").is_ok());
        assert!(content.check("[1] 2").is_err());
        assert!(content.check("[1,]").is_err());
        let deep = format!("{}{}", "[".repeat(10_000), "]".repeat(10_000));
        assert!(content.check(&deep).is_ok());
        assert!(content.check(&deep[1..]).is_err());
    }
}
