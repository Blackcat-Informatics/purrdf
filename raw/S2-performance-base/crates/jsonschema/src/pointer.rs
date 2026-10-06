// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 6901 JSON Pointers as schema and instance locations.
//!
//! A pointer is kept in its escaped string form (`""` or `/a/b~1c`) wherever it
//! is a location, so appending a token is string concatenation and two
//! locations compare as strings. Tokens are escaped exactly once, on the way in,
//! by [`purrdf_iri::json_pointer`]; a location travels in a URI fragment
//! percent-encoded by [`purrdf_iri::percent`] with its `FRAGMENT` set, and
//! resolved against a document by [`purrdf_lex::json::Value::pointer`].

pub(crate) use json_pointer::{escape_token, tokens};
use purrdf_iri::json_pointer;

/// Append one unescaped token to an escaped pointer.
pub(crate) fn push_token(pointer: &str, token: &str) -> String {
    let mut out = String::with_capacity(pointer.len() + 1 + token.len());
    out.push_str(pointer);
    json_pointer::push_token(&mut out, token);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::json::{self, Value};

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
        let doc = json::read(r#"{"a": [10, 20]}"#).expect("JSON");
        assert_eq!(doc.pointer("/a/01"), None);
        assert_eq!(doc.pointer("/a/1"), Some(&Value::from(20_u8)));
        assert_eq!(doc.pointer("/a/2"), None);
    }

    #[test]
    fn a_keyword_location_fragment_is_percent_encoded() {
        let encode = |pointer| purrdf_iri::percent::encode(pointer, purrdf_iri::percent::FRAGMENT);
        assert_eq!(encode("/a b/%/é"), "/a%20b/%25/%C3%A9");
        assert_eq!(encode("/properties/~0a~1b"), "/properties/~0a~1b");
    }
}
