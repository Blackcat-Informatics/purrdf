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

/// The original base64 grammar refusal, before content presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Base64Refusal {
    /// The non-linebreak symbol count is not a multiple of four.
    Length {
        /// Original symbol count.
        symbols: usize,
    },
    /// Padding appears in a nonfinal group or exceeds two symbols.
    Padding {
        /// Zero-based group index.
        group: usize,
        /// Padding count.
        padding: usize,
        /// Whether this was the final group.
        last: bool,
    },
    /// A symbol is absent from the base64 alphabet.
    Symbol {
        /// Offset among non-linebreak symbols.
        offset: usize,
        /// Original rejected byte.
        byte: u8,
    },
}

impl purrdf_hash::error::StructuredError for Base64Refusal {
    purrdf_hash::error_visit! {
        Length { symbols } => "schema-base64-length" { "symbols" => Unsigned(*symbols as u128) },
        Padding { group, padding, last } => "schema-base64-padding" { "group" => Unsigned(*group as u128), "padding" => Unsigned(*padding as u128), "last" => Boolean(*last) },
        Symbol { offset, byte } => "schema-base64-symbol" { "offset" => Unsigned(*offset as u128), "byte" => Unsigned(u128::from(*byte)) },
    }
}

impl std::fmt::Display for Base64Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the string is not valid base64")
    }
}
impl std::error::Error for Base64Refusal {}

/// A draft-07 content assertion's original native decoding refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentRefusal {
    /// Original source text and base64 grammar cause.
    Base64 {
        /// Original encoded text.
        text: String,
        /// Original decoding cause.
        cause: Base64Refusal,
    },
    /// Original decoded content and JSON grammar cause.
    Json {
        /// Original content bytes, decoded if base64 was declared.
        bytes: Vec<u8>,
        /// Original grammar refusal and offset.
        cause: purrdf_lex::json::Error,
    },
}

impl purrdf_hash::error::StructuredError for ContentRefusal {
    purrdf_hash::error_visit! {
        Base64 { text, cause } => "schema-content-base64" { "text" => Text(text), "cause" => Cause(cause) },
        Json { bytes, cause } => "schema-content-json" { "bytes" => Bytes(bytes), "cause" => Cause(cause) },
    }
}

impl std::fmt::Display for ContentRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Base64 { cause, .. } => cause.fmt(f),
            Self::Json { .. } => f.write_str("the content is not a JSON document"),
        }
    }
}
purrdf_lex::error_source!(ContentRefusal => {
    Self::Base64 { cause, .. } => cause,
    Self::Json { cause, .. } => cause
});

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
    pub(crate) fn check(self, text: &str) -> Result<(), ContentRefusal> {
        let decoded;
        let bytes = if self.base64 {
            decoded = decode_base64(text).map_err(|cause| ContentRefusal::Base64 {
                text: text.to_owned(),
                cause,
            })?;
            decoded.as_slice()
        } else {
            text.as_bytes()
        };
        if self.json {
            let check = || {
                let mut reader = Reader::from_slice(bytes, Limits::with_depth(usize::MAX))?;
                reader.skip_value()?;
                reader.finish()
            };
            check().map_err(|cause| ContentRefusal::Json {
                bytes: bytes.to_vec(),
                cause,
            })?;
        }
        Ok(())
    }
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
fn decode_base64(text: &str) -> Result<Vec<u8>, Base64Refusal> {
    let symbols: Vec<u8> = text
        .bytes()
        .filter(|&byte| byte != b'\r' && byte != b'\n')
        .collect();
    if !symbols.len().is_multiple_of(4) {
        return Err(Base64Refusal::Length {
            symbols: symbols.len(),
        });
    }
    let mut out = Vec::with_capacity(symbols.len() / 4 * 3);
    let groups = symbols.len() / 4;
    for (index, group) in symbols.as_chunks::<4>().0.iter().enumerate() {
        let last = index + 1 == groups;
        let padding = group.iter().rev().take_while(|&&byte| byte == b'=').count();
        if padding > 2 || (padding > 0 && !last) {
            return Err(Base64Refusal::Padding {
                group: index,
                padding,
                last,
            });
        }
        let mut word: u32 = 0;
        for (position, &symbol) in group[..4 - padding].iter().enumerate() {
            let value = sextet(symbol).ok_or(Base64Refusal::Symbol {
                offset: index * 4 + position,
                byte: symbol,
            })?;
            word = (word << 6) | u32::from(value);
        }
        word <<= 6 * padding;
        let [_, high, middle, low] = word.to_be_bytes();
        out.extend_from_slice(&[high, middle, low][..3 - padding]);
    }
    Ok(out)
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
        assert_eq!(decode_base64("Zm9v").as_deref(), Ok(b"foo".as_slice()));
        assert_eq!(decode_base64("Zm8=").as_deref(), Ok(b"fo".as_slice()));
        assert_eq!(decode_base64("Zg==").as_deref(), Ok(b"f".as_slice()));
        assert_eq!(
            decode_base64("Zm9v\r\nYmFy").as_deref(),
            Ok(b"foobar".as_slice())
        );
        assert_eq!(decode_base64(""), Ok(Vec::new()));
        assert_eq!(
            decode_base64("Zm9%"),
            Err(Base64Refusal::Symbol {
                offset: 3,
                byte: b'%'
            })
        );
        assert_eq!(
            decode_base64("Zm9"),
            Err(Base64Refusal::Length { symbols: 3 })
        );
        assert_eq!(
            decode_base64("Zg==Zg=="),
            Err(Base64Refusal::Padding {
                group: 0,
                padding: 2,
                last: false
            })
        );
        assert_eq!(
            decode_base64("Z==="),
            Err(Base64Refusal::Padding {
                group: 0,
                padding: 3,
                last: true
            })
        );
    }

    #[test]
    fn content_retains_original_native_decoder_causes_after_input_death() {
        use purrdf_hash::error::ErrorValue;
        use purrdf_testkit::error_capture::Capture;

        let text = String::from("Zm9%");
        let error = Content {
            base64: true,
            json: true,
        }
        .check(&text)
        .unwrap_err();
        drop(text);
        let capture = Capture::read(&error);
        assert!(matches!(capture.field("text"), ErrorValue::Text("Zm9%")));
        let cause = capture.child("cause");
        assert_eq!(cause.code, "schema-base64-symbol");
        assert!(matches!(cause.field("offset"), ErrorValue::Unsigned(3)));
        assert!(matches!(cause.field("byte"), ErrorValue::Unsigned(37)));

        let text = String::from("[1,]");
        let error = Content {
            base64: false,
            json: true,
        }
        .check(&text)
        .unwrap_err();
        drop(text);
        let expected = Reader::from_slice(b"[1,]", Limits::with_depth(usize::MAX))
            .unwrap()
            .skip_value()
            .unwrap_err();
        let capture = Capture::read(&error);
        assert!(matches!(capture.field("bytes"), ErrorValue::Bytes(b"[1,]")));
        let cause = capture.child("cause");
        assert_eq!(cause.code, Capture::read(&expected).code);
        assert!(
            matches!(cause.field("offset"), ErrorValue::ByteCount(value) if value == expected.offset() as u128)
        );
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
