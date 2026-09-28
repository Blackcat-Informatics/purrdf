// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 6901 JSON Pointer: the reference-token escape and the pointer split.
//!
//! ```text
//! json-pointer    = *( "/" reference-token )
//! reference-token = *( unescaped / escaped )
//! unescaped       = %x00-2E / %x30-7D / %x7F-10FFFF   ; %x2F ('/') and %x7E ('~') are excluded
//! escaped         = "~" ( "0" / "1" )                  ; representing '~' and '/', respectively
//! ```
//!
//! A pointer is a sequence of reference tokens each introduced by `/`; the
//! empty string is the whole document. Inside a token `~` is written `~0`
//! and `/` is written `~1`, and RFC 6901 §4 fixes the order the two are
//! undone in: `~1` first, then `~0`, so that `~01` decodes to `~1` and not
//! to `/`. The escaper here writes `~0` first for the same reason.
//!
//! These functions live in this leaf because a JSON Pointer is the one
//! addressing scheme every JSON-facing crate — JSON Schema, JSON-LD, the
//! ordered-JSON codec — shares, and because a pointer travels inside an IRI
//! fragment, where [`percent`](crate::percent) is its companion.
//!
//! ```rust
//! use purrdf_iri::json_pointer::{escape_token, tokens, unescape_token};
//!
//! assert_eq!(escape_token("a/b~c"), "a~1b~0c");
//! assert_eq!(unescape_token("a~1b~0c").unwrap(), "a/b~c");
//! let parts = tokens("/properties/a~1b/0").unwrap();
//! assert_eq!(parts, ["properties", "a/b", "0"]);
//! assert!(tokens("").unwrap().is_empty());
//! assert!(tokens("properties").is_err());
//! ```

use core::fmt;
use std::borrow::Cow;

/// Why a string is not a JSON Pointer, or a token not a reference token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum PointerError {
    /// A non-empty pointer does not start with `/` (RFC 6901 §3:
    /// `json-pointer = *( "/" reference-token )`).
    MissingLeadingSlash,
    /// A `~` is followed by something other than `0` or `1`, or ends the
    /// token (RFC 6901 §3: `escaped = "~" ( "0" / "1" )`). Carries the byte
    /// offset of the `~` — into the token for [`unescape_token`], into the
    /// whole pointer for [`tokens`].
    BadEscape {
        /// Byte offset of the offending `~`.
        at: usize,
    },
}

impl PointerError {
    /// The human-readable reason, as a `&'static str`; [`fmt::Display`]
    /// renders this (and the offset, where there is one).
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::MissingLeadingSlash => "a non-empty JSON Pointer must start with `/`",
            Self::BadEscape { .. } => "`~` in a JSON Pointer must be followed by `0` or `1`",
        }
    }

    /// The same error with its offset moved by `by` bytes — how a token's
    /// offset becomes a pointer's.
    const fn rebase(self, by: usize) -> Self {
        match self {
            Self::MissingLeadingSlash => self,
            Self::BadEscape { at } => Self::BadEscape { at: at + by },
        }
    }
}

impl fmt::Display for PointerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::MissingLeadingSlash => f.write_str(self.message()),
            Self::BadEscape { at } => write!(f, "{} at byte {at}", self.message()),
        }
    }
}

impl core::error::Error for PointerError {}

/// Append `token` to `out` as a reference token: `~` as `~0`, then `/` as
/// `~1` (RFC 6901 §3). No `/` separator is written; the caller places the
/// token with `out.push('/')` first.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::json_pointer::push_escaped;
///
/// let mut pointer = String::new();
/// for token in ["properties", "a/b", "m~n"] {
///     pointer.push('/');
///     push_escaped(&mut pointer, token);
/// }
/// assert_eq!(pointer, "/properties/a~1b/m~0n");
/// ```
pub fn push_escaped(out: &mut String, token: &str) {
    out.reserve(token.len());
    for ch in token.chars() {
        match ch {
            '~' => out.push_str("~0"),
            '/' => out.push_str("~1"),
            other => out.push(other),
        }
    }
}

/// `token` as a reference token — see [`push_escaped`] — borrowing it
/// unchanged when it holds neither `~` nor `/`.
///
/// # Examples
///
/// ```rust
/// use std::borrow::Cow;
///
/// use purrdf_iri::json_pointer::escape_token;
///
/// assert_eq!(escape_token("a/b"), "a~1b");
/// assert_eq!(escape_token("m~n"), "m~0n");
/// // `~` first, so a `~1` in the input does not read back as `/`.
/// assert_eq!(escape_token("~1"), "~01");
/// assert!(matches!(escape_token("plain"), Cow::Borrowed(_)));
/// ```
#[must_use]
pub fn escape_token(token: &str) -> Cow<'_, str> {
    let Some(first) = token.find(['~', '/']) else {
        return Cow::Borrowed(token);
    };
    let mut out = String::with_capacity(token.len() + 2);
    out.push_str(&token[..first]);
    push_escaped(&mut out, &token[first..]);
    Cow::Owned(out)
}

/// The text a reference token names: `~1` as `/` and then `~0` as `~`
/// (RFC 6901 §4, in that order), borrowing `token` unchanged when it holds
/// no `~`.
///
/// A `~` followed by anything but `0` or `1`, or ending the token, is a
/// [`PointerError::BadEscape`] at the `~`'s byte offset in `token` — never a
/// `~` passed through.
///
/// # Examples
///
/// ```rust
/// use std::borrow::Cow;
///
/// use purrdf_iri::json_pointer::{PointerError, unescape_token};
///
/// assert_eq!(unescape_token("a~1b").unwrap(), "a/b");
/// assert_eq!(unescape_token("m~0n").unwrap(), "m~n");
/// // Order matters: `~01` is `~1`, not `/`.
/// assert_eq!(unescape_token("~01").unwrap(), "~1");
/// assert!(matches!(unescape_token("plain").unwrap(), Cow::Borrowed(_)));
/// assert_eq!(unescape_token("a~2").unwrap_err(), PointerError::BadEscape { at: 1 });
/// assert_eq!(unescape_token("a~").unwrap_err(), PointerError::BadEscape { at: 1 });
/// ```
pub fn unescape_token(token: &str) -> Result<Cow<'_, str>, PointerError> {
    let Some(first) = token.find('~') else {
        return Ok(Cow::Borrowed(token));
    };
    let bytes = token.as_bytes();
    let mut out = String::with_capacity(token.len());
    out.push_str(&token[..first]);
    let mut at = first;
    while at < bytes.len() {
        if bytes[at] == b'~' {
            match bytes.get(at + 1) {
                Some(b'0') => out.push('~'),
                Some(b'1') => out.push('/'),
                _ => return Err(PointerError::BadEscape { at }),
            }
            at += 2;
        } else {
            // Copy the run up to the next `~` whole: `~` is ASCII, so the run
            // ends on a char boundary.
            let end = token[at..].find('~').map_or(bytes.len(), |off| at + off);
            out.push_str(&token[at..end]);
            at = end;
        }
    }
    Ok(Cow::Owned(out))
}

/// The reference tokens of `pointer`, each unescaped (RFC 6901 §3–§4).
///
/// The empty pointer names the whole document and yields no tokens; any
/// other pointer must start with `/`, and every `/` after it introduces one
/// token, so `/` alone is one empty token and `//` is two. A bad escape is
/// reported at its byte offset in `pointer`.
///
/// Tokens are returned as text; whether a token is an array index (RFC 6901
/// §4's `0` / `%x31-39 *DIGIT`, no leading zero) or a member name is decided
/// by the value it is applied to, so that reading is the caller's.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::json_pointer::{PointerError, tokens};
///
/// assert!(tokens("").unwrap().is_empty());
/// assert_eq!(tokens("/").unwrap(), [""]);
/// assert_eq!(tokens("/a~1b/c~0d/0").unwrap(), ["a/b", "c~d", "0"]);
/// assert_eq!(tokens("a/b").unwrap_err(), PointerError::MissingLeadingSlash);
/// assert_eq!(tokens("/a/b~x").unwrap_err(), PointerError::BadEscape { at: 4 });
/// ```
pub fn tokens(pointer: &str) -> Result<Vec<Cow<'_, str>>, PointerError> {
    if pointer.is_empty() {
        return Ok(Vec::new());
    }
    let Some(rest) = pointer.strip_prefix('/') else {
        return Err(PointerError::MissingLeadingSlash);
    };
    let mut out = Vec::with_capacity(rest.bytes().filter(|&b| b == b'/').count() + 1);
    let mut offset = 1;
    for raw in rest.split('/') {
        out.push(unescape_token(raw).map_err(|error| error.rebase(offset))?);
        offset += raw.len() + 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{PointerError, escape_token, push_escaped, tokens, unescape_token};

    #[test]
    fn escape_pins_the_two_escapes_in_rfc_order() {
        assert_eq!(escape_token(""), "");
        assert_eq!(escape_token("a"), "a");
        assert_eq!(escape_token("/"), "~1");
        assert_eq!(escape_token("~"), "~0");
        assert_eq!(escape_token("~/"), "~0~1");
        assert_eq!(escape_token("/~"), "~1~0");
        assert_eq!(escape_token("~1"), "~01");
        assert_eq!(escape_token("~0"), "~00");
        assert_eq!(escape_token("a/b~c/d"), "a~1b~0c~1d");
        assert_eq!(escape_token("é/ü"), "é~1ü");
        // Nothing else is touched: a `%`, a space, a quote pass through.
        assert_eq!(escape_token("a %b\"c"), "a %b\"c");
    }

    #[test]
    fn escape_borrows_only_when_nothing_is_escaped() {
        assert!(matches!(escape_token("plain"), Cow::Borrowed("plain")));
        assert!(matches!(escape_token(""), Cow::Borrowed("")));
        assert!(matches!(escape_token("a/b"), Cow::Owned(_)));
        assert!(matches!(escape_token("a~b"), Cow::Owned(_)));
    }

    #[test]
    fn push_escaped_appends_the_same_bytes_escape_token_returns() {
        for token in ["", "a", "a/b", "m~n", "~1", "é/ü", "//~~"] {
            let mut out = String::from("/x/");
            push_escaped(&mut out, token);
            assert_eq!(out, format!("/x/{}", escape_token(token)), "{token:?}");
        }
    }

    #[test]
    fn unescape_pins_rfc_6901_section_4_order() {
        assert_eq!(unescape_token("").unwrap(), "");
        assert_eq!(unescape_token("~1").unwrap(), "/");
        assert_eq!(unescape_token("~0").unwrap(), "~");
        assert_eq!(unescape_token("~01").unwrap(), "~1");
        assert_eq!(unescape_token("~00").unwrap(), "~0");
        assert_eq!(unescape_token("~10").unwrap(), "/0");
        assert_eq!(unescape_token("a~1b~0c~1d").unwrap(), "a/b~c/d");
        assert_eq!(unescape_token("é~1ü").unwrap(), "é/ü");
        assert_eq!(unescape_token("~0~0~1~1").unwrap(), "~~//");
    }

    #[test]
    fn unescape_borrows_only_when_there_is_no_tilde() {
        assert!(matches!(
            unescape_token("a/b").unwrap(),
            Cow::Borrowed("a/b")
        ));
        assert!(matches!(unescape_token("").unwrap(), Cow::Borrowed("")));
        assert!(matches!(unescape_token("~0").unwrap(), Cow::Owned(_)));
    }

    #[test]
    fn unescape_refusals_each_have_an_accepted_neighbour() {
        for (refused, at, accepted) in [
            ("~", 0, "~0"),
            ("a~", 1, "a~1"),
            ("~2", 0, "~1"),
            ("~~", 0, "~0~0"),
            ("~/", 0, "~1"),
            ("a~b", 1, "a~0b"),
            ("~0~", 2, "~0~0"),
            ("é~x", 2, "é~0"),
        ] {
            assert_eq!(
                unescape_token(refused).unwrap_err(),
                PointerError::BadEscape { at },
                "{refused:?}"
            );
            assert!(unescape_token(accepted).is_ok(), "{accepted:?}");
        }
    }

    #[test]
    fn escape_then_unescape_is_the_identity() {
        for token in [
            "", "a", "/", "~", "~1", "~0", "a/b~c", "//", "~~", "é/~", "0", "-",
        ] {
            assert_eq!(
                unescape_token(&escape_token(token)).unwrap(),
                token,
                "{token:?}"
            );
        }
    }

    #[test]
    fn tokens_pins_the_pointer_grammar() {
        assert_eq!(tokens("").unwrap(), Vec::<Cow<'_, str>>::new());
        assert_eq!(tokens("/").unwrap(), [""]);
        assert_eq!(tokens("//").unwrap(), ["", ""]);
        assert_eq!(tokens("/a").unwrap(), ["a"]);
        assert_eq!(tokens("/a/").unwrap(), ["a", ""]);
        assert_eq!(tokens("/a/b/c").unwrap(), ["a", "b", "c"]);
        assert_eq!(tokens("/a~1b/c~0d").unwrap(), ["a/b", "c~d"]);
        assert_eq!(tokens("/0/-/01").unwrap(), ["0", "-", "01"]);
        assert_eq!(tokens("/ /%20/\"").unwrap(), [" ", "%20", "\""]);
        // The RFC 6901 §5 example document's pointers.
        assert_eq!(tokens("/foo/0").unwrap(), ["foo", "0"]);
        assert_eq!(tokens("/a~1b").unwrap(), ["a/b"]);
        assert_eq!(tokens("/c%d").unwrap(), ["c%d"]);
        assert_eq!(tokens("/e^f").unwrap(), ["e^f"]);
        assert_eq!(tokens("/g|h").unwrap(), ["g|h"]);
        assert_eq!(tokens("/i\\j").unwrap(), ["i\\j"]);
        assert_eq!(tokens("/k\"l").unwrap(), ["k\"l"]);
        assert_eq!(tokens("/ ").unwrap(), [" "]);
        assert_eq!(tokens("/m~0n").unwrap(), ["m~n"]);
    }

    #[test]
    fn tokens_borrow_when_unescaped() {
        let parts = tokens("/a/b~1c").unwrap();
        assert!(matches!(parts[0], Cow::Borrowed("a")));
        assert!(matches!(parts[1], Cow::Owned(_)));
    }

    #[test]
    fn tokens_refusals_each_have_an_accepted_neighbour() {
        for (refused, error, accepted) in [
            ("a", PointerError::MissingLeadingSlash, "/a"),
            ("a/b", PointerError::MissingLeadingSlash, "/a/b"),
            ("#/a", PointerError::MissingLeadingSlash, "/a"),
            (" /a", PointerError::MissingLeadingSlash, "/ /a"),
            ("/~", PointerError::BadEscape { at: 1 }, "/~0"),
            ("/a/b~", PointerError::BadEscape { at: 4 }, "/a/b~1"),
            ("/a/b~x/c", PointerError::BadEscape { at: 4 }, "/a/b~1x/c"),
            ("/~0/~2", PointerError::BadEscape { at: 4 }, "/~0/~1"),
            ("/é/~", PointerError::BadEscape { at: 4 }, "/é/~0"),
        ] {
            assert_eq!(tokens(refused).unwrap_err(), error, "{refused:?}");
            assert!(tokens(accepted).is_ok(), "{accepted:?}");
        }
        assert_eq!(
            PointerError::MissingLeadingSlash.to_string(),
            "a non-empty JSON Pointer must start with `/`"
        );
        assert_eq!(
            PointerError::BadEscape { at: 4 }.to_string(),
            "`~` in a JSON Pointer must be followed by `0` or `1` at byte 4"
        );
    }

    #[test]
    fn tokens_round_trip_through_push_escaped() {
        let parts = ["properties", "a/b", "m~n", "", "0", "~1"];
        let mut pointer = String::new();
        for part in parts {
            pointer.push('/');
            push_escaped(&mut pointer, part);
        }
        assert_eq!(pointer, "/properties/a~1b/m~0n//0/~01");
        assert_eq!(tokens(&pointer).unwrap(), parts);
    }
}
