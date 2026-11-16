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

/// Append the original reference-token spelling after admitting its exact
/// escaped extent. The shared escape body then writes within that capacity.
///
/// # Errors
///
/// Returns checked length, allocator or original admission refusal.
pub fn push_token_with_memory<S: crate::allocation::Admission + ?Sized>(
    pointer: &mut String,
    token: &str,
    memory: &mut crate::allocation::Memory<'_, S>,
) -> Result<(), crate::allocation::StorageError> {
    let escaped = token
        .bytes()
        .filter(|byte| matches!(byte, b'~' | b'/'))
        .count();
    let required = pointer
        .len()
        .checked_add(1)
        .and_then(|length| length.checked_add(token.len()))
        .and_then(|length| length.checked_add(escaped))
        .ok_or(crate::allocation::StorageError::SizeOverflow)?;
    memory.reserve_string(pointer, required)?;
    push_token(pointer, token);
    Ok(())
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

/// A JSON Pointer held as its reference tokens, unescaped.
///
/// A token is kept exactly as the member name or array index it addresses;
/// the RFC 6901 spelling is produced only by [`Display`](core::fmt::Display),
/// through [`escape_token`]. A pointer built here therefore never needs to be
/// parsed to recover its tokens, and a member name holding `/` or `~` stays
/// one token.
///
/// ```rust
/// use purrdf_lex::json_pointer::JsonPointer;
///
/// let pointer = JsonPointer::root().child("$defs").child("a/b").child_index(0);
/// assert_eq!(pointer.tokens(), ["$defs", "a/b", "0"]);
/// assert_eq!(pointer.to_string(), "/$defs/a~1b/0");
/// assert_eq!(JsonPointer::parse("/$defs/a~1b/0"), Some(pointer));
/// assert_eq!(JsonPointer::root().to_string(), "");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct JsonPointer {
    tokens: Vec<String>,
}

impl JsonPointer {
    /// The pointer to the whole document: no tokens.
    #[must_use]
    pub const fn root() -> Self {
        Self { tokens: Vec::new() }
    }

    /// The pointer `pointer` spells, or `None` when it is not a JSON Pointer
    /// (see [`tokens`]).
    #[must_use]
    pub fn parse(pointer: &str) -> Option<Self> {
        Some(Self {
            tokens: tokens(pointer)?.into_iter().map(Cow::into_owned).collect(),
        })
    }

    /// This pointer extended by the member name `token`, unescaped.
    #[must_use]
    pub fn child(&self, token: impl Into<String>) -> Self {
        let mut tokens = Vec::with_capacity(self.tokens.len() + 1);
        tokens.extend_from_slice(&self.tokens);
        tokens.push(token.into());
        Self { tokens }
    }

    /// This pointer extended by the array index `index`.
    #[must_use]
    pub fn child_index(&self, index: usize) -> Self {
        self.child(index.to_string())
    }

    /// The reference tokens, unescaped, from the document root.
    #[must_use]
    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }
}

impl JsonPointer {
    /// The bytes of the RFC 6901 spelling, escaped as they are read, so a
    /// comparison allocates nothing.
    fn spelling(&self) -> impl Iterator<Item = u8> + '_ {
        self.tokens.iter().flat_map(|token| {
            core::iter::once(b'/').chain(token.bytes().flat_map(|byte| {
                let (first, second) = match byte {
                    b'~' => (b'~', Some(b'0')),
                    b'/' => (b'~', Some(b'1')),
                    other => (other, None),
                };
                core::iter::once(first).chain(second)
            }))
        })
    }
}

/// Pointers order by their RFC 6901 spelling, the order a pointer list
/// written as text has always had.
impl Ord for JsonPointer {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.spelling().cmp(other.spelling())
    }
}

impl PartialOrd for JsonPointer {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl core::fmt::Display for JsonPointer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for token in &self.tokens {
            f.write_str("/")?;
            f.write_str(&escape_token(token))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{JsonPointer, array_index, escape_token, tokens, unescape_token};

    /// A member name holding `/` or `~` stays one token: the spelling escapes
    /// it, parsing that spelling returns the same tokens, and the pointer
    /// addresses that member and not a nested path.
    #[test]
    fn a_member_name_with_slash_or_tilde_is_one_token() {
        let document = crate::json::read(r#"{"a/b": {"c~d": 1}, "a": {"b": {"c~d": 2}}}"#)
            .expect("a JSON object");
        let pointer = JsonPointer::root().child("a/b").child("c~d");
        assert_eq!(pointer.to_string(), "/a~1b/c~0d");
        assert_eq!(
            JsonPointer::parse(&pointer.to_string()),
            Some(pointer.clone())
        );
        assert_eq!(
            document
                .pointer(&pointer.to_string())
                .and_then(crate::json::Value::as_u64),
            Some(1)
        );
        let plain = JsonPointer::root()
            .child("$defs")
            .child("Person")
            .child_index(3);
        assert_eq!(plain.to_string(), "/$defs/Person/3");
    }

    /// Pointers order exactly as their spellings do, which is the order every
    /// text-keyed map of locations had. Token order would differ: `["a", "b"]`
    /// spells `/a/b` and sorts after `["a-"]`, spelled `/a-`, because `-`
    /// sorts before `/`; an escaped `/` and `~` keep their spelled order too.
    #[test]
    fn pointers_order_by_their_spelling_not_by_token() {
        let mut pointers = [
            JsonPointer::root().child("a").child("b"),
            JsonPointer::root().child("a-"),
            JsonPointer::root().child("a/b"),
            JsonPointer::root().child("a~b"),
            JsonPointer::root().child("a"),
            JsonPointer::root(),
        ];
        pointers.sort();
        let spelled: Vec<String> = pointers.iter().map(ToString::to_string).collect();
        let mut expected = spelled.clone();
        expected.sort();
        assert_eq!(spelled, expected);
        assert_eq!(spelled, ["", "/a", "/a-", "/a/b", "/a~0b", "/a~1b"]);
    }

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
