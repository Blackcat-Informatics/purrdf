// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! JSON Pointer (RFC 6901): the reference-token escape, pointer parsing and
//! the array-index token.
//!
//! A pointer is `""` or a sequence of `/`-prefixed reference tokens, and a
//! token spells `~` as `~0` and `/` as `~1` (§3). Escaping replaces `~` before
//! `/` — the other order would turn `/` into `~1` and then into `~01` — and
//! unescaping reads `~0` and `~1` left to right, so `~01` is the token `~1`
//! (§4). Every other `~` sequence is not a pointer and is refused.
//!
//! ```rust
//! use purrdf_lex::json_pointer::{push_token, tokens};
//!
//! let mut pointer = String::new();
//! push_token(&mut pointer, "a/b");
//! push_token(&mut pointer, "m~n");
//! assert_eq!(pointer, "/a~1b/m~0n");
//! assert_eq!(tokens(&pointer).unwrap(), ["a/b", "m~n"]);
//! ```

use std::borrow::Cow;

/// `token` escaped as an RFC 6901 reference token: `~` as `~0`, then `/` as
/// `~1`; borrowed when it holds neither.
///
/// ```rust
/// use purrdf_lex::json_pointer::escape_token;
///
/// assert_eq!(escape_token("a/b~c"), "a~1b~0c");
/// assert_eq!(escape_token("~1"), "~01");
/// assert!(matches!(escape_token("plain"), std::borrow::Cow::Borrowed(_)));
/// ```
#[must_use]
pub fn escape_token(token: &str) -> Cow<'_, str> {
    if !token.contains(['~', '/']) {
        return Cow::Borrowed(token);
    }
    let mut out = String::with_capacity(token.len() + 2);
    push_escaped(&mut out, token);
    Cow::Owned(out)
}

/// Append `/` and `token`, escaped, to the pointer `pointer`.
///
/// ```rust
/// use purrdf_lex::json_pointer::push_token;
///
/// let mut pointer = String::from("/$defs");
/// push_token(&mut pointer, "a/b");
/// assert_eq!(pointer, "/$defs/a~1b");
/// ```
pub fn push_token(pointer: &mut String, token: &str) {
    pointer.push('/');
    push_escaped(pointer, token);
}

/// Append `token`, escaped, to `out`.
fn push_escaped(out: &mut String, token: &str) {
    let mut rest = token;
    while let Some(at) = rest.find(['~', '/']) {
        out.push_str(&rest[..at]);
        out.push_str(if rest.as_bytes()[at] == b'~' {
            "~0"
        } else {
            "~1"
        });
        rest = &rest[at + 1..];
    }
    out.push_str(rest);
}

/// The reference token `token` spells, or `None` when a `~` in it is not
/// followed by `0` or `1`; borrowed when it holds no `~`.
///
/// ```rust
/// use purrdf_lex::json_pointer::unescape_token;
///
/// assert_eq!(unescape_token("a~1b~0c").as_deref(), Some("a/b~c"));
/// assert_eq!(unescape_token("~01").as_deref(), Some("~1"));
/// assert_eq!(unescape_token("~2"), None);
/// assert_eq!(unescape_token("a~"), None);
/// ```
#[must_use]
pub fn unescape_token(token: &str) -> Option<Cow<'_, str>> {
    let Some(first) = token.find('~') else {
        return Some(Cow::Borrowed(token));
    };
    let bytes = token.as_bytes();
    let mut out = String::with_capacity(token.len());
    out.push_str(&token[..first]);
    let mut at = first;
    loop {
        out.push(match bytes.get(at + 1) {
            Some(b'0') => '~',
            Some(b'1') => '/',
            _ => return None,
        });
        at += 2;
        let run = token[at..].find('~').map_or(token.len(), |next| at + next);
        out.push_str(&token[at..run]);
        if run == token.len() {
            return Some(Cow::Owned(out));
        }
        at = run;
    }
}

/// The unescaped reference tokens of `pointer`, or `None` when it is not a
/// JSON Pointer: it must be empty (no tokens) or begin with `/`, and every
/// token must [unescape](unescape_token).
///
/// ```rust
/// use purrdf_lex::json_pointer::tokens;
///
/// assert_eq!(tokens("").unwrap(), Vec::<&str>::new());
/// assert_eq!(tokens("/").unwrap(), [""]);
/// assert_eq!(tokens("/a~1b/0").unwrap(), ["a/b", "0"]);
/// assert_eq!(tokens("a"), None);
/// assert_eq!(tokens("/~2"), None);
/// ```
#[must_use]
pub fn tokens(pointer: &str) -> Option<Vec<Cow<'_, str>>> {
    if pointer.is_empty() {
        return Some(Vec::new());
    }
    pointer
        .strip_prefix('/')?
        .split('/')
        .map(unescape_token)
        .collect()
}

/// The array index a reference token names (RFC 6901 §4): `0`, or a digit
/// string without a leading zero that fits a `usize`; `None` for anything else
/// (`-`, `01`, `+1`, the empty token).
///
/// ```rust
/// use purrdf_lex::json_pointer::array_index;
///
/// assert_eq!(array_index("0"), Some(0));
/// assert_eq!(array_index("12"), Some(12));
/// assert_eq!(array_index("01"), None);
/// assert_eq!(array_index("-"), None);
/// ```
#[must_use]
pub fn array_index(token: &str) -> Option<usize> {
    match token.as_bytes() {
        [b'0'] => Some(0),
        [b'1'..=b'9', rest @ ..] if rest.iter().all(u8::is_ascii_digit) => token.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{array_index, escape_token, tokens, unescape_token};

    #[test]
    fn a_tilde_not_followed_by_zero_or_one_is_refused() {
        for token in ["~", "~2", "a~", "~a", "~~0", "~/"] {
            assert_eq!(unescape_token(token), None, "{token:?}");
        }
        assert_eq!(tokens("/a/~2"), None);
        assert_eq!(tokens("no-slash"), None);
    }

    #[test]
    fn tilde_zero_and_tilde_one_still_unescape_left_to_right() {
        assert_eq!(unescape_token("~0").as_deref(), Some("~"));
        assert_eq!(unescape_token("~1").as_deref(), Some("/"));
        assert_eq!(unescape_token("~01").as_deref(), Some("~1"));
        assert_eq!(unescape_token("~10").as_deref(), Some("/0"));
        assert_eq!(escape_token("~/"), "~0~1");
        assert_eq!(tokens("//").unwrap(), ["", ""]);
    }

    #[test]
    fn an_array_index_is_a_canonical_decimal() {
        for (token, index) in [
            ("0", Some(0)),
            ("10", Some(10)),
            ("01", None),
            ("", None),
            ("-", None),
            ("+1", None),
            ("1a", None),
        ] {
            assert_eq!(array_index(token), index, "{token:?}");
        }
    }
}
