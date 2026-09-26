// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 6901 JSON Pointers and the RFC 3986 fragment encoding they travel in.
//!
//! A pointer is kept in its escaped string form (`""` or `/a/b~1c`) wherever it
//! is a location, so appending a token is string concatenation and two
//! locations compare as strings. Tokens are escaped exactly once, on the way in.

use serde_json::Value;

/// Escape one reference token: `~` as `~0`, then `/` as `~1` (RFC 6901 §3).
pub(crate) fn escape_token(token: &str) -> String {
    if !token.contains(['~', '/']) {
        return token.to_owned();
    }
    let mut out = String::with_capacity(token.len() + 2);
    for ch in token.chars() {
        match ch {
            '~' => out.push_str("~0"),
            '/' => out.push_str("~1"),
            other => out.push(other),
        }
    }
    out
}

/// Append one unescaped token to an escaped pointer.
pub(crate) fn push_token(pointer: &str, token: &str) -> String {
    let escaped = escape_token(token);
    let mut out = String::with_capacity(pointer.len() + 1 + escaped.len());
    out.push_str(pointer);
    out.push('/');
    out.push_str(&escaped);
    out
}

/// Split an escaped pointer into its unescaped tokens. `None` when the text is
/// not a JSON Pointer: it must be empty or start with `/`, and every `~` must
/// be followed by `0` or `1` (RFC 6901 §3, §4).
pub(crate) fn tokens(pointer: &str) -> Option<Vec<String>> {
    if pointer.is_empty() {
        return Some(Vec::new());
    }
    let rest = pointer.strip_prefix('/')?;
    rest.split('/').map(unescape_token).collect()
}

fn unescape_token(token: &str) -> Option<String> {
    if !token.contains('~') {
        return Some(token.to_owned());
    }
    let mut out = String::with_capacity(token.len());
    let mut chars = token.chars();
    while let Some(ch) = chars.next() {
        if ch == '~' {
            match chars.next() {
                Some('0') => out.push('~'),
                Some('1') => out.push('/'),
                _ => return None,
            }
        } else {
            out.push(ch);
        }
    }
    Some(out)
}

/// Resolve unescaped tokens against `root` (RFC 6901 §4). An array index is
/// `0` or a digit string without a leading zero, inside the array's bounds.
pub(crate) fn lookup<'v>(root: &'v Value, tokens: &[String]) -> Option<&'v Value> {
    let mut current = root;
    for token in tokens {
        current = match current {
            Value::Object(map) => map.get(token)?,
            Value::Array(items) => items.get(array_index(token)?)?,
            _ => return None,
        };
    }
    Some(current)
}

/// Resolve an escaped pointer against `root`.
pub(crate) fn lookup_str<'v>(root: &'v Value, pointer: &str) -> Option<&'v Value> {
    lookup(root, &tokens(pointer)?)
}

fn array_index(token: &str) -> Option<usize> {
    let bytes = token.as_bytes();
    let well_formed = match bytes {
        [b'0'] => true,
        [b'1'..=b'9', rest @ ..] => rest.iter().all(u8::is_ascii_digit),
        _ => false,
    };
    if well_formed {
        token.parse().ok()
    } else {
        None
    }
}

/// Decode the `%HH` escapes of a URI fragment into UTF-8 text. `None` when an
/// escape is malformed or the decoded bytes are not UTF-8.
pub(crate) fn percent_decode(text: &str) -> Option<String> {
    if !text.contains('%') {
        return Some(text.to_owned());
    }
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex_value(*bytes.get(index + 1)?)?;
            let low = hex_value(*bytes.get(index + 2)?)?;
            out.push(high << 4 | low);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Write an escaped pointer as a URI fragment body: every byte outside RFC
/// 3986's `fragment` set (`pchar / "/" / "?"`) is percent-encoded, so the
/// absolute keyword location is a well-formed URI.
pub(crate) fn fragment_encode(pointer: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(pointer.len());
    for &byte in pointer.as_bytes() {
        let allowed = byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'.'
                    | b'_'
                    | b'~'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
                    | b':'
                    | b'@'
                    | b'/'
                    | b'?'
            );
        if allowed {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tokens_round_trip_through_escaping() {
        let pointer = push_token(&push_token("", "a/b"), "m~n");
        assert_eq!(pointer, "/a~1b/m~0n");
        assert_eq!(
            tokens(&pointer),
            Some(vec!["a/b".to_owned(), "m~n".to_owned()])
        );
        assert_eq!(tokens("a"), None);
        assert_eq!(tokens("/~2"), None);
        assert_eq!(tokens(""), Some(Vec::new()));
    }

    #[test]
    fn lookup_refuses_leading_zero_indices_and_accepts_their_neighbour() {
        let doc = json!({"a": [10, 20]});
        assert_eq!(lookup_str(&doc, "/a/01"), None);
        assert_eq!(lookup_str(&doc, "/a/1"), Some(&json!(20)));
        assert_eq!(lookup_str(&doc, "/a/2"), None);
    }

    #[test]
    fn percent_escapes_decode_and_encode() {
        assert_eq!(percent_decode("a%25b%C3%A9"), Some("a%bé".to_owned()));
        assert_eq!(percent_decode("a%2"), None);
        assert_eq!(fragment_encode("/a b/%/é"), "/a%20b/%25/%C3%A9");
        assert_eq!(fragment_encode("/properties/~0a~1b"), "/properties/~0a~1b");
    }
}
