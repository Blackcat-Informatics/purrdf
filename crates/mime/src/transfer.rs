// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact RFC2045 transfer decoding; malformed input never yields partial bytes.

use crate::TransferEncoding;

pub(crate) struct DecodeError {
    pub(crate) at: usize,
    pub(crate) reason: &'static str,
}

pub(crate) fn validate_identity(source: &[u8], seven_bit: bool) -> Result<(), DecodeError> {
    let mut at = 0;
    let mut line_bytes = 0;
    while at < source.len() {
        let byte = source[at];
        let reason = if byte == 0 {
            Some("NUL in line-oriented transfer body")
        } else if seven_bit && byte >= 128 {
            Some("non-7bit byte under 7bit declaration")
        } else if byte == b'\n' {
            Some("bare LF in line-oriented transfer body")
        } else if byte == b'\r' {
            if source.get(at + 1) != Some(&b'\n') {
                Some("bare CR in line-oriented transfer body")
            } else {
                line_bytes = 0;
                at += 2;
                continue;
            }
        } else {
            line_bytes += 1;
            (line_bytes > 998).then_some("line exceeds RFC2045's 998-octet transfer syntax")
        };
        if let Some(reason) = reason {
            return Err(DecodeError { at, reason });
        }
        at += 1;
    }
    Ok(())
}

pub(crate) fn decode(source: &[u8], encoding: &TransferEncoding) -> Result<Vec<u8>, DecodeError> {
    match encoding {
        TransferEncoding::SevenBit | TransferEncoding::EightBit => {
            validate_identity(source, *encoding == TransferEncoding::SevenBit)?;
            Ok(source.to_vec())
        }
        TransferEncoding::Binary => Ok(source.to_vec()),
        TransferEncoding::Base64 => base64(source),
        TransferEncoding::QuotedPrintable => quoted_printable(source, false),
        TransferEncoding::Unknown(_) => Err(DecodeError {
            at: 0,
            reason: "unsupported declared transfer encoding",
        }),
        TransferEncoding::Ambiguous => Err(DecodeError {
            at: 0,
            reason: "conflicting transfer encoding declarations",
        }),
    }
}

fn base64(source: &[u8]) -> Result<Vec<u8>, DecodeError> {
    // RFC2045 §6.8 tells decoders to ignore characters outside the alphabet.
    // This lexical policy is MIME's; the actual bit/padding decoder remains the
    // one XSD binary home. Padding '=' is retained so malformed padding refuses.
    let lexical: String = source
        .iter()
        .copied()
        .filter(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
        .map(char::from)
        .collect();
    purrdf_xsd::parse_base64(&lexical).map_err(|_| DecodeError {
        at: 0,
        reason: "malformed base64 groups or padding",
    })
}

pub(crate) fn quoted_printable(source: &[u8], encoded_word: bool) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < source.len() {
        let byte = source[at];
        if byte == b'=' {
            if !encoded_word && source.get(at + 1..at + 3) == Some(b"\r\n") {
                at += 3;
                continue;
            }
            let Some(pair) = source.get(at + 1..at + 3) else {
                return Err(DecodeError {
                    at,
                    reason: "truncated quoted-printable escape",
                });
            };
            // Base16 decoding is the existing native home, not a second nibble table.
            let lexical = core::str::from_utf8(pair).map_err(|_| DecodeError {
                at,
                reason: "non-ASCII quoted-printable escape",
            })?;
            let decoded = purrdf_xsd::parse_hex(lexical).map_err(|_| DecodeError {
                at,
                reason: "invalid quoted-printable hexadecimal escape",
            })?;
            out.extend_from_slice(&decoded);
            at += 3;
        } else if encoded_word && byte == b'_' {
            out.push(b' ');
            at += 1;
        } else if matches!(byte, b'\t' | b' ') && !encoded_word {
            let start = at;
            while source
                .get(at)
                .is_some_and(|value| matches!(value, b'\t' | b' '))
            {
                at += 1;
            }
            // RFC2045 §6.7: unencoded transport padding at a physical line end
            // is not part of decoded content; encoded =20/=09 remains payload.
            if at < source.len() && source.get(at..at + 2) != Some(b"\r\n") {
                out.extend_from_slice(&source[start..at]);
            }
        } else if !encoded_word && source.get(at..at + 2) == Some(b"\r\n") {
            out.extend_from_slice(b"\r\n");
            at += 2;
        } else if (33..=126).contains(&byte) && byte != b'=' {
            out.push(byte);
            at += 1;
        } else {
            return Err(DecodeError {
                at,
                reason: "illegal unencoded quoted-printable byte or bare line ending",
            });
        }
    }
    Ok(out)
}

pub(crate) fn encoded_words_valid(source: &[u8], name: Option<&[u8]>) -> Result<bool, ()> {
    let Some(name) = name else { return Ok(false) };
    // RFC2047 §5: words occupy unstructured text, phrases or comments. Received
    // has no encoded-word positions. Quoted strings, addr-specs and MIME
    // parameters remain literal ASCII; recognizing a substring there would
    // invent a charset judgment about original bytes.
    if name.eq_ignore_ascii_case(b"received") {
        return Ok(false);
    }
    let address = [
        b"from".as_slice(),
        b"to",
        b"cc",
        b"bcc",
        b"sender",
        b"reply-to",
        b"resent-from",
        b"resent-to",
        b"resent-cc",
        b"resent-bcc",
        b"resent-sender",
    ]
    .iter()
    .any(|field| name.eq_ignore_ascii_case(field));
    let structured = address
        || [
            b"content-type".as_slice(),
            b"content-disposition",
            b"content-transfer-encoding",
            b"mime-version",
            b"date",
            b"resent-date",
            b"message-id",
            b"resent-message-id",
            b"in-reply-to",
            b"references",
            b"return-path",
        ]
        .iter()
        .any(|field| name.eq_ignore_ascii_case(field));
    let mut at = 0;
    let mut comment = 0usize;
    let mut quoted = false;
    let mut angle = false;
    let mut found = false;
    while at < source.len() {
        let byte = source[at];
        if structured {
            if byte == b'\\' && (comment > 0 || quoted) {
                at = (at + 2).min(source.len());
                continue;
            }
            match byte {
                b'"' if comment == 0 => quoted = !quoted,
                b'(' if !quoted => comment += 1,
                b')' if !quoted && comment > 0 => comment -= 1,
                b'<' if !quoted && comment == 0 => angle = true,
                b'>' if !quoted && comment == 0 => angle = false,
                _ => {}
            }
        }
        let position = !structured || comment > 0 || (address && !quoted && !angle);
        let boundary = at == 0
            || matches!(source[at - 1], b' ' | b'\t')
            || (comment > 0 && source[at - 1] == b'(')
            || (address && matches!(source[at - 1], b',' | b';' | b':'));
        if !position || !boundary || !source[at..].starts_with(b"=?") {
            at += 1;
            continue;
        }
        let Some(end) =
            purrdf_lex::scan::find_byte_pair(&source[at + 2..], [b'?'; 2], [b'='; 2], 1)
                .map(|offset| at + 2 + offset + 2)
        else {
            // A lone literal "=?" is not an encoded word. A word-shaped
            // declaration with both interior separators is a broken one.
            let token = source[at..]
                .split(|byte| matches!(byte, b' ' | b'\t'))
                .next()
                .unwrap_or_default();
            let mut remaining = token;
            let mut separators = 0;
            for _ in 0..3 {
                let Some(offset) = purrdf_lex::scan::find_byte(remaining, b'?') else {
                    break;
                };
                separators += 1;
                remaining = &remaining[offset + 1..];
            }
            if separators == 3 {
                return Err(());
            }
            at += 2;
            continue;
        };
        if address && comment == 0 && source.get(end) == Some(&b'@') {
            at = end;
            continue;
        }
        let preceding_separated = at == 0
            || matches!(source[at - 1], b' ' | b'\t')
            || (comment > 0 && source[at - 1] == b'(');
        let separated = end == source.len()
            || matches!(source[end], b' ' | b'\t')
            || (comment > 0 && source[end] == b')');
        if !preceding_separated
            || !separated
            || !encoded_word_valid(&source[at..end], address && comment == 0, comment > 0)
        {
            return Err(());
        }
        found = true;
        at = end;
    }
    Ok(found)
}

fn encoded_word_valid(word: &[u8], phrase: bool, comment: bool) -> bool {
    if word.len() > 75 {
        return false;
    }
    let inner = &word[2..word.len() - 2];
    let Some(charset_end) = purrdf_lex::scan::find_byte(inner, b'?') else {
        return false;
    };
    let charset = &inner[..charset_end];
    // RFC2047 §2 token, whose especials include '.' and '=' unlike a MIME
    // parameter token. No Unicode property or guessed charset is substituted.
    if charset.is_empty()
        || charset.iter().any(|byte| {
            !byte.is_ascii_graphic()
                || matches!(
                    byte,
                    b'(' | b')'
                        | b'<'
                        | b'>'
                        | b'@'
                        | b','
                        | b';'
                        | b':'
                        | b'\\'
                        | b'"'
                        | b'/'
                        | b'['
                        | b']'
                        | b'?'
                        | b'.'
                        | b'='
                )
        })
    {
        return false;
    }
    let rest = &inner[charset_end + 1..];
    let Some(encoding) = rest.first() else {
        return false;
    };
    if rest.get(1) != Some(&b'?') {
        return false;
    }
    let encoded = &rest[2..];
    if encoded.is_empty()
        || encoded
            .iter()
            .any(|byte| !byte.is_ascii_graphic() || *byte == b'?')
    {
        return false;
    }
    match encoding.to_ascii_lowercase() {
        b'b' => {
            core::str::from_utf8(encoded).is_ok_and(|text| purrdf_xsd::parse_base64(text).is_ok())
        }
        b'q' => {
            (!phrase
                || encoded.iter().all(|byte| {
                    byte.is_ascii_alphanumeric()
                        || matches!(byte, b'!' | b'*' | b'+' | b'-' | b'/' | b'=' | b'_')
                }))
                && (!comment
                    || !encoded
                        .iter()
                        .any(|byte| matches!(byte, b'(' | b')' | b'\\')))
                && quoted_printable(encoded, true).is_ok()
        }
        _ => false,
    }
}
