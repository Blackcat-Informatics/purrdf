// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Token normalisation and the fingerprints computed from it.
//!
//! A body is compared as its token stream, never as text: the lexer has already
//! dropped comments and whitespace, so a reformat or a reworded comment cannot
//! separate two copies. Two normal forms are computed.
//!
//! * The **structural form** (clone types 1 and 2) keeps keywords, punctuation,
//!   delimiters and the names of the items a body uses (paths, called functions and
//!   methods, macros, types, variants, primitive types), renames every local name
//!   — bindings, parameters, fields, lifetimes — to its first-appearance position
//!   (`$0`, `$1`, …) and replaces every literal with its kind (`$int`, `$str`, …).
//!   Two bodies that differ only in the names they chose for their own values and
//!   in the constants they carry collide; two bodies whose control flow differs, or
//!   that call different functions or match different enums, do not. Renaming item
//!   names too would make every `match self { Self::A => "a", … }` of the same
//!   arity one "group", which is shape, not shared functionality.
//! * The **shim form** renames only the function's own parameters and keeps every
//!   other identifier and literal verbatim, so two thin forwarders collide exactly
//!   when they forward to the same target with the same fixed arguments.
//!
//! A fingerprint id is `<kind>:<16 lowercase hex digits>`, the first eight bytes of
//! the BLAKE3 digest of the normal form.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

/// The structural form of a body whose normal form has fewer tokens than this is
/// not grouped: at this size a body is a one-line accessor or a thin shim, and
/// thin shims are allowed. Shims are grouped separately, by their shim form.
pub(crate) const MIN_TOKENS: usize = 30;

/// A shim needs at least this many tokens to be grouped; below it every
/// `{ f(x) }` would be a group.
pub(crate) const MIN_SHIM_TOKENS: usize = 6;

/// The fingerprint kinds a ledger may name.
const FINGERPRINT_KINDS: [&str; 4] = ["body", "shim", "table", "rule"];

/// The hex-digit tables the census recognises, by fingerprint id.
pub(crate) const HEX_LOWER: &str = "table:hex-lower";
/// See [`HEX_LOWER`].
pub(crate) const HEX_UPPER: &str = "table:hex-upper";

/// Rust's keywords, strict and reserved, kept verbatim by both normal forms.
const KEYWORDS: [&str; 51] = [
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
];

/// Whether `id` is a well-formed fingerprint id.
pub(crate) fn is_fingerprint_id(id: &str) -> bool {
    id.split_once(':').is_some_and(|(kind, rest)| {
        FINGERPRINT_KINDS.contains(&kind)
            && if kind == "table" {
                rest == "hex-lower" || rest == "hex-upper"
            } else if kind == "rule" {
                id == crate::rules::STD_DEFAULT_HASHER
            } else {
                rest.len() == 16
                    && rest
                        .bytes()
                        .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
            }
    })
}

/// The fingerprint id of a normal form.
fn fingerprint(kind: &str, form: &[String]) -> String {
    let digest = purrdf_hash::blake3::hash(form.join(" ").as_bytes());
    format!(
        "{kind}:{}",
        purrdf_hash::hex::Lower(&digest.as_bytes()[..8])
    )
}

/// What the census learns from one body.
#[derive(Clone, Debug)]
pub(crate) struct BodyPrint {
    /// Tokens in the structural form.
    pub(crate) tokens: usize,
    /// The structural fingerprint (`body:…`).
    pub(crate) structural: String,
    /// The shim fingerprint (`shim:…`), when the body is a thin forwarder.
    pub(crate) shim: Option<String>,
}

/// Fingerprint a braced body. `params` are the function's parameter names.
pub(crate) fn body_print(body: &TokenStream, params: &BTreeSet<String>) -> BodyPrint {
    let mut structural = Vec::new();
    structural_form(body, &mut BTreeMap::new(), &mut structural);
    let shim = shim_candidate(body).then(|| {
        let mut form = Vec::new();
        shim_form(body, params, &mut BTreeMap::new(), &mut form);
        form
    });
    BodyPrint {
        tokens: structural.len(),
        structural: fingerprint("body", &structural),
        shim: shim
            .filter(|form| form.len() >= MIN_SHIM_TOKENS && structural.len() < MIN_TOKENS)
            .map(|form| fingerprint("shim", &form)),
    }
}

/// Primitive type names, kept verbatim: `as u32` and `as u64` are different
/// arithmetic.
const PRIMITIVES: [&str; 17] = [
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32",
    "f64", "bool", "char", "str",
];

/// Whether the identifier at `index` names an item rather than a local binding:
/// a path segment (`a::b`), a call (`f(…)`, `.m(…)`, `.m::<T>(…)`), a macro
/// (`m!`), a capitalised name (a type, variant or constant) or a primitive type.
/// Item names are kept verbatim: two bodies that call different functions or
/// match different enums are different functionality, however alike their shape.
fn names_an_item(tokens: &[TokenTree], index: usize, text: &str) -> bool {
    let punct_at = |at: usize, wanted: char| matches!(tokens.get(at), Some(TokenTree::Punct(punct)) if punct.as_char() == wanted);
    let path_before = index >= 2 && punct_at(index - 1, ':') && punct_at(index - 2, ':');
    let path_after = punct_at(index + 1, ':') && punct_at(index + 2, ':');
    let call_after = matches!(
        tokens.get(index + 1),
        Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis
    );
    path_before
        || path_after
        || call_after
        || punct_at(index + 1, '!')
        || text.starts_with(|first: char| first.is_ascii_uppercase())
        || PRIMITIVES.contains(&text)
}

/// The structural form of `stream`, appended to `out`: keywords, punctuation,
/// delimiters and item names verbatim, local names (bindings, parameters,
/// fields, lifetimes) renamed to their first-appearance position, literals
/// replaced by their kind.
pub(crate) fn structural_form(
    stream: &TokenStream,
    names: &mut BTreeMap<String, usize>,
    out: &mut Vec<String>,
) {
    let tokens: Vec<TokenTree> = stream.clone().into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Group(group) => {
                let (open, close) = delimiters(group.delimiter());
                out.push(open.to_owned());
                structural_form(&group.stream(), names, out);
                out.push(close.to_owned());
            }
            TokenTree::Ident(ident) => {
                let text = ident.to_string();
                if KEYWORDS.contains(&text.as_str()) || names_an_item(&tokens, index, &text) {
                    out.push(text);
                } else {
                    let next = names.len();
                    let position = *names.entry(text).or_insert(next);
                    out.push(format!("${position}"));
                }
            }
            TokenTree::Punct(punct) => out.push(punct_text(punct)),
            TokenTree::Literal(literal) => out.push(literal_kind(&literal.to_string()).to_owned()),
        }
    }
}

/// The shim form of `stream`: parameters renamed, everything else verbatim.
fn shim_form(
    stream: &TokenStream,
    params: &BTreeSet<String>,
    names: &mut BTreeMap<String, usize>,
    out: &mut Vec<String>,
) {
    for token in stream.clone() {
        match token {
            TokenTree::Group(group) => {
                let (open, close) = delimiters(group.delimiter());
                out.push(open.to_owned());
                shim_form(&group.stream(), params, names, out);
                out.push(close.to_owned());
            }
            TokenTree::Ident(ident) => {
                let text = ident.to_string();
                if params.contains(&text) {
                    let next = names.len();
                    let position = *names.entry(text).or_insert(next);
                    out.push(format!("$p{position}"));
                } else {
                    out.push(text);
                }
            }
            TokenTree::Punct(punct) => out.push(punct_text(&punct)),
            TokenTree::Literal(literal) => out.push(literal.to_string()),
        }
    }
}

/// A thin forwarder: one expression (no `let`, no statement boundary before the
/// tail), at least one call, and no `self` — a delegation on `self` is bound to
/// its own type and cannot collapse with another type's.
fn shim_candidate(body: &TokenStream) -> bool {
    let tokens: Vec<TokenTree> = body.clone().into_iter().collect();
    let mut statements = 0;
    for (index, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Punct(punct) if punct.as_char() == ';' && index + 1 < tokens.len() => {
                statements += 1;
            }
            TokenTree::Ident(ident) if ident == "let" => return false,
            _ => {}
        }
    }
    statements == 0 && has_call(body) && !mentions_self(body)
}

fn has_call(stream: &TokenStream) -> bool {
    let mut previous_is_name = false;
    for token in stream.clone() {
        match token {
            TokenTree::Group(group) => {
                if previous_is_name && group.delimiter() == Delimiter::Parenthesis {
                    return true;
                }
                if has_call(&group.stream()) {
                    return true;
                }
                previous_is_name = false;
            }
            TokenTree::Ident(_) => previous_is_name = true,
            TokenTree::Punct(punct) => previous_is_name = punct.as_char() == '!',
            TokenTree::Literal(_) => previous_is_name = false,
        }
    }
    false
}

fn mentions_self(stream: &TokenStream) -> bool {
    stream.clone().into_iter().any(|token| match token {
        TokenTree::Group(group) => mentions_self(&group.stream()),
        TokenTree::Ident(ident) => ident == "self",
        _ => false,
    })
}

const fn delimiters(delimiter: Delimiter) -> (&'static str, &'static str) {
    match delimiter {
        Delimiter::Parenthesis => ("(", ")"),
        Delimiter::Brace => ("{", "}"),
        Delimiter::Bracket => ("[", "]"),
        Delimiter::None => ("«", "»"),
    }
}

/// A punctuation character, marked when it is joined to the next one so `- -x`
/// and `-= x` stay distinct.
fn punct_text(punct: &proc_macro2::Punct) -> String {
    match punct.spacing() {
        Spacing::Joint => format!("{}~", punct.as_char()),
        Spacing::Alone => punct.as_char().to_string(),
    }
}

/// The kind a literal is abstracted to in the structural form.
fn literal_kind(text: &str) -> &'static str {
    let bytes = text.as_bytes();
    match bytes.first() {
        Some(b'"') => "$str",
        Some(b'\'') => "$char",
        Some(b'b') if bytes.get(1) == Some(&b'\'') => "$byte",
        Some(b'b') => "$bytes",
        Some(b'c') => "$cstr",
        Some(b'r') => "$str",
        Some(b'0'..=b'9') => {
            if integer_value(text).is_some() {
                "$int"
            } else {
                "$float"
            }
        }
        _ => "$lit",
    }
}

/// The value of an integer literal in any Rust spelling — `0xcbf2_9ce4_8422_2325`,
/// `14695981039346656037`, `0o17u8`, `0b1010` — or `None` for anything that is
/// not one (floats included).
pub(crate) fn integer_value(text: &str) -> Option<u128> {
    const SUFFIXES: [&str; 12] = [
        "u128", "i128", "usize", "isize", "u64", "i64", "u32", "i32", "u16", "i16", "u8", "i8",
    ];
    let (radix, digits) = if let Some(rest) = text.strip_prefix("0x") {
        (16, rest)
    } else if let Some(rest) = text.strip_prefix("0o") {
        (8, rest)
    } else if let Some(rest) = text.strip_prefix("0b") {
        (2, rest)
    } else {
        (10, text)
    };
    let digits = SUFFIXES
        .iter()
        .find_map(|suffix| digits.strip_suffix(suffix))
        .unwrap_or(digits);
    let mut value: u128 = 0;
    let mut seen = false;
    for character in digits.chars() {
        if character == '_' {
            continue;
        }
        let digit = character.to_digit(radix)?;
        value = value
            .checked_mul(u128::from(radix))?
            .checked_add(u128::from(digit))?;
        seen = true;
    }
    seen.then_some(value)
}

/// Every integer literal in `stream`, by value.
pub(crate) fn integer_constants(stream: &TokenStream, out: &mut BTreeSet<u128>) {
    for token in stream.clone() {
        match token {
            TokenTree::Group(group) => integer_constants(&group.stream(), out),
            TokenTree::Literal(literal) => {
                if let Some(value) = integer_value(&literal.to_string()) {
                    out.insert(value);
                }
            }
            _ => {}
        }
    }
}

/// The hex-digit tables in `stream`: a string or byte-string literal spelling
/// the sixteen digits in order, or a bracketed array of the sixteen digit
/// characters or bytes in order.
pub(crate) fn hex_tables(stream: &TokenStream, out: &mut BTreeSet<&'static str>) {
    for token in stream.clone() {
        match token {
            TokenTree::Group(group) => {
                if group.delimiter() == Delimiter::Bracket
                    && let Some(table) = digit_array(&group.stream())
                {
                    out.insert(table);
                }
                hex_tables(&group.stream(), out);
            }
            TokenTree::Literal(literal) => {
                if let Some(table) = digit_string(&literal.to_string()) {
                    out.insert(table);
                }
            }
            _ => {}
        }
    }
}

/// Which hex-digit table `digits` spells, if any. The sixteen digits are rendered
/// by the workspace's own base16 encoder rather than retyped, so the census does
/// not carry the table it looks for.
fn classify_digits(digits: &str) -> Option<&'static str> {
    const ASCENDING_NIBBLES: [u8; 8] = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef];
    if digits.len() != 2 * ASCENDING_NIBBLES.len() {
        return None;
    }
    let lower = purrdf_hash::hex::Lower(&ASCENDING_NIBBLES).to_string();
    if digits == lower {
        Some(HEX_LOWER)
    } else if digits == lower.to_ascii_uppercase() {
        Some(HEX_UPPER)
    } else {
        None
    }
}

fn digit_string(literal: &str) -> Option<&'static str> {
    let unprefixed = literal.strip_prefix('b').unwrap_or(literal);
    let unraw = unprefixed
        .strip_prefix('r')
        .map_or(unprefixed, |raw| raw.trim_matches('#'));
    let inner = unraw.strip_prefix('"')?.strip_suffix('"')?;
    classify_digits(inner)
}

fn digit_array(stream: &TokenStream) -> Option<&'static str> {
    let mut digits = String::new();
    for token in stream.clone() {
        match token {
            TokenTree::Literal(literal) => {
                let text = literal.to_string();
                let unprefixed = text.strip_prefix('b').unwrap_or(&text);
                let inner = unprefixed.strip_prefix('\'')?.strip_suffix('\'')?;
                let mut chars = inner.chars();
                let (Some(single), None) = (chars.next(), chars.next()) else {
                    return None;
                };
                digits.push(single);
            }
            TokenTree::Punct(punct) if punct.as_char() == ',' => {}
            _ => return None,
        }
    }
    classify_digits(&digits)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::str::FromStr;

    use proc_macro2::TokenStream;

    use super::{HEX_LOWER, HEX_UPPER, body_print, hex_tables, integer_value, structural_form};

    fn tokens(source: &str) -> TokenStream {
        TokenStream::from_str(source).expect("test source lexes")
    }

    fn form(source: &str) -> Vec<String> {
        let mut out = Vec::new();
        structural_form(&tokens(source), &mut BTreeMap::new(), &mut out);
        out
    }

    const LOOP_A: &str = "let mut total = 0u64; for byte in bytes { total = total.wrapping_mul(31).wrapping_add(u64::from(*byte)); } total";
    const LOOP_B: &str = "let mut acc = 7u64; for b in input { acc = acc.wrapping_mul(131).wrapping_add(u64::from(*b)); } acc";

    #[test]
    fn bodies_that_differ_only_in_names_and_literals_collide() {
        assert_eq!(form(LOOP_A), form(LOOP_B));
        let params = BTreeSet::new();
        assert_eq!(
            body_print(&tokens(LOOP_A), &params).structural,
            body_print(&tokens(LOOP_B), &params).structural
        );
    }

    #[test]
    fn different_control_flow_does_not_collide() {
        let while_loop = "let mut total = 0u64; let mut i = 0; while i < bytes.len() { total = total.wrapping_mul(31).wrapping_add(u64::from(bytes[i])); i += 1; } total";
        assert_ne!(form(LOOP_A), form(while_loop));
        let early_return = "let mut total = 0u64; for byte in bytes { if *byte == 0 { return total; } total = total.wrapping_mul(31).wrapping_add(u64::from(*byte)); } total";
        assert_ne!(form(LOOP_A), form(early_return));
    }

    #[test]
    fn renaming_is_positional_so_a_repeated_name_is_not_two_names() {
        assert_eq!(form("a - b"), form("x - y"));
        assert_ne!(form("a - a"), form("a - b"));
    }

    #[test]
    fn joined_punctuation_is_distinct_from_separate_punctuation() {
        assert_ne!(form("a -= b"), form("a - = b"));
    }

    #[test]
    fn integer_constants_compare_by_value_in_every_spelling() {
        assert_eq!(
            integer_value("0xcbf2_9ce4_8422_2325"),
            Some(14_695_981_039_346_656_037)
        );
        assert_eq!(
            integer_value("14695981039346656037u64"),
            Some(14_695_981_039_346_656_037)
        );
        assert_eq!(integer_value("0o17"), Some(15));
        assert_eq!(integer_value("0b1010_u8"), Some(10));
        assert_eq!(integer_value("1.5"), None);
        assert_eq!(integer_value("1e3"), None);
        assert_eq!(integer_value("_"), None);
    }

    #[test]
    fn hex_tables_are_found_in_every_spelling_and_nothing_else_is() {
        for (source, expected) in [
            ("b\"0123456789abcdef\"", Some(HEX_LOWER)),
            ("\"0123456789ABCDEF\"", Some(HEX_UPPER)),
            ("br\"0123456789abcdef\"", Some(HEX_LOWER)),
            (
                "[b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f']",
                Some(HEX_LOWER),
            ),
            (
                "['0','1','2','3','4','5','6','7','8','9','A','B','C','D','E','F']",
                Some(HEX_UPPER),
            ),
            ("b\"0123456789abcdeF\"", None),
            ("b\"0123456789abcdefg\"", None),
            ("[b'0', b'1']", None),
        ] {
            let mut found = BTreeSet::new();
            hex_tables(&tokens(source), &mut found);
            assert_eq!(found.into_iter().next(), expected, "{source}");
        }
    }

    #[test]
    fn a_thin_forwarder_gets_a_shim_print_and_a_long_body_does_not() {
        let params: BTreeSet<String> = std::iter::once("text".to_owned()).collect();
        let other: BTreeSet<String> = std::iter::once("value".to_owned()).collect();
        let first = body_print(
            &tokens("crate::escape::write(text, Mode::Iri, 16)"),
            &params,
        );
        let second = body_print(
            &tokens("crate::escape::write(value, Mode::Iri, 16)"),
            &other,
        );
        assert!(first.shim.is_some());
        assert_eq!(first.shim, second.shim);
        let different_default =
            body_print(&tokens("crate::escape::write(text, Mode::Iri, 8)"), &params);
        assert_ne!(first.shim, different_default.shim);
        let on_self = body_print(&tokens("self.inner.write(text, Mode::Iri, 16)"), &params);
        assert_eq!(on_self.shim, None);
        assert_eq!(body_print(&tokens(LOOP_A), &params).shim, None);
    }
}
