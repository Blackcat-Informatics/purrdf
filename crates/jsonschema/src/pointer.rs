// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 6901 JSON Pointers, resolved against a `serde_json` document.
//!
//! A pointer is kept in its escaped string form (`""` or `/a/b~1c`) wherever it
//! is a location, so appending a token is string concatenation and two
//! locations compare as strings. Tokens are escaped exactly once, on the way in,
//! by [`purrdf_iri::json_pointer`]; a location travels in a URI fragment
//! percent-encoded by [`purrdf_iri::percent`] with its `FRAGMENT` set.

use purrdf_iri::json_pointer;
use serde_json::Value;

pub(crate) use json_pointer::{escape_token, tokens};

/// Append one unescaped token to an escaped pointer.
pub(crate) fn push_token(pointer: &str, token: &str) -> String {
    let mut out = String::with_capacity(pointer.len() + 1 + token.len());
    out.push_str(pointer);
    json_pointer::push_token(&mut out, token);
    out
}

/// Resolve unescaped tokens against `root` (RFC 6901 §4). An array index is
/// `0` or a digit string without a leading zero, inside the array's bounds.
pub(crate) fn lookup<'v, T: AsRef<str>>(root: &'v Value, tokens: &[T]) -> Option<&'v Value> {
    let mut current = root;
    for token in tokens {
        let token = token.as_ref();
        current = match current {
            Value::Object(map) => map.get(token)?,
            Value::Array(items) => items.get(json_pointer::array_index(token)?)?,
            _ => return None,
        };
    }
    Some(current)
}

/// Resolve an escaped pointer against `root`.
pub(crate) fn lookup_str<'v>(root: &'v Value, pointer: &str) -> Option<&'v Value> {
    lookup(root, &tokens(pointer)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tokens_round_trip_through_escaping() {
        let pointer = push_token(&push_token("", "a/b"), "m~n");
        assert_eq!(pointer, "/a~1b/m~0n");
        assert_eq!(tokens(&pointer), Some(vec!["a/b".into(), "m~n".into()]));
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
    fn a_keyword_location_fragment_is_percent_encoded() {
        let encode = |pointer| purrdf_iri::percent::encode(pointer, purrdf_iri::percent::FRAGMENT);
        assert_eq!(encode("/a b/%/é"), "/a%20b/%25/%C3%A9");
        assert_eq!(encode("/properties/~0a~1b"), "/properties/~0a~1b");
    }
}
