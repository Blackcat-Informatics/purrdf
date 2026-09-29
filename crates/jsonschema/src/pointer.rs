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
            let high = purrdf_hash::hex::nibble(*bytes.get(index + 1)?)?;
            let low = purrdf_hash::hex::nibble(*bytes.get(index + 2)?)?;
            out.push(high << 4 | low);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Write an escaped pointer as a URI fragment body: every byte outside RFC
/// 3986's `fragment` set (`pchar / "/" / "?"`) is percent-encoded, so the
/// absolute keyword location is a well-formed URI.
pub(crate) fn fragment_encode(pointer: &str) -> String {
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
            purrdf_hash::hex::encode_upper_into(&[byte], &mut out);
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

/// The frozen JSON Pointer vectors, replayed against the JSON Schema pointer module.
#[cfg(test)]
mod json_pointer_frozen_vectors {
    use purrdf_testkit::vectors::{VectorFile, answer_digest, decode_str, encode_str};

    const POINTERS: &str = include_str!("../../lex/tests/vectors/json_pointer_vectors.txt");

    fn escape(token: &str) -> String {
        super::escape_token(token)
    }

    fn tokens(pointer: &str) -> Option<Vec<String>> {
        super::tokens(pointer)
    }

    #[test]
    fn pointers_replay_the_frozen_vectors() {
        let file = VectorFile::parse(POINTERS).expect("json_pointer_vectors.txt");
        let mut replayed = 0;
        for record in file.records() {
            let fields = &record.fields;
            match fields[0] {
                "escape" => {
                    let token = decode_str(fields[1]).expect("a token");
                    assert_eq!(encode_str(&escape(&token)), fields[2], "{token:?}");
                    replayed += 1;
                }
                "plane" => {
                    let plane = u32::from_str_radix(fields[1], 16).expect("hex");
                    let answers = ((plane << 16)..=((plane << 16) | 0xFFFF))
                        .filter_map(char::from_u32)
                        .map(|c| encode_str(&escape(&format!("{c}~{c}/"))));
                    assert_eq!(answer_digest(answers), fields[2], "plane {plane:X}");
                    replayed += 1;
                }
                "parse" => {
                    let pointer = decode_str(fields[1]).expect("a pointer");
                    let expected: Option<Vec<String>> = (fields[2] != "-").then(|| {
                        fields[3..]
                            .iter()
                            .map(|token| decode_str(token).expect("a token"))
                            .collect()
                    });
                    assert_eq!(tokens(&pointer), expected, "{pointer:?}");
                    replayed += 1;
                }
                _ => {}
            }
        }
        assert!(replayed > 100, "{replayed}");
    }
}

/// The frozen percent-encoding vectors, replayed against the keyword-location fragment codec.
#[cfg(test)]
mod percent_frozen_vectors {
    use purrdf_testkit::vectors::{VectorFile, answer_digest, decode_str, encode_str};

    /// Replay one vector file: `run` answers an input (`None` for a refusal),
    /// `encode` says whether answers are encoded text rather than `=`-prefixed
    /// decodings, and `skipped` names inputs this copy is known to answer differently.
    fn replay(
        vectors: &str,
        encode: bool,
        run: impl Fn(&str) -> Option<String>,
        skipped: impl Fn(&str) -> bool,
        plane_skipped: impl Fn(u32) -> bool,
    ) {
        let answer = |input: &str| match run(input) {
            Some(value) if encode => encode_str(&value),
            Some(value) => encode_str(&format!("={value}")),
            None => "-".to_owned(),
        };
        let file = VectorFile::parse(vectors).expect("a percent vector file");
        let mut replayed = 0;
        for record in file.records() {
            if record.fields[0] == "plane" {
                let plane = u32::from_str_radix(record.fields[1], 16).expect("hex");
                if plane_skipped(plane) {
                    continue;
                }
                let answers = ((plane << 16)..=((plane << 16) | 0xFFFF))
                    .filter_map(char::from_u32)
                    .filter(|c| !skipped(&c.to_string()))
                    .map(|c| answer(&c.to_string()));
                assert_eq!(answer_digest(answers), record.fields[2], "plane {plane:X}");
            } else {
                let input = decode_str(record.fields[1]).expect("an encoded input");
                if skipped(&input) {
                    continue;
                }
                assert_eq!(answer(&input), record.fields[2], "{input:?}");
            }
            replayed += 1;
        }
        assert!(replayed > 400, "{replayed}");
    }

    #[test]
    fn fragment_replays_the_frozen_vectors() {
        replay(
            include_str!("../../lex/tests/vectors/percent_fragment_vectors.txt"),
            true,
            |input| Some(super::fragment_encode(input)),
            |input| {
                let _ = input;
                false
            },
            |plane| {
                let _ = plane;
                false
            },
        );
    }

    #[test]
    fn decode_replays_the_frozen_vectors() {
        replay(
            include_str!("../../lex/tests/vectors/percent_decode_vectors.txt"),
            false,
            super::percent_decode,
            |input| {
                let _ = input;
                false
            },
            |plane| {
                let _ = plane;
                false
            },
        );
    }
}
