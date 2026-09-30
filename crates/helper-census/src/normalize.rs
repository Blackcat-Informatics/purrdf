// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Token normalisation and the fingerprints computed from it.
//!
//! A body is compared as its token stream, never as text: the lexer has already
//! dropped comments and whitespace, so a reformat or a reworded comment cannot
//! separate two copies. Two normal forms are computed.
//!
//! * The **structural form** (clone types 1 and 2, and the near misses of type 3
//!   that differ only in how a value is held) keeps keywords, punctuation,
//!   delimiters, the names of the items a body uses (paths, called functions and
//!   methods, macros, types, variants, primitive types) and the names of the
//!   fields it reads, renames every local name — bindings, parameters,
//!   lifetimes — to its first-appearance position (`$0`, `$1`, …) and replaces
//!   every literal with its kind (`$int`, `$str`, …; a type suffix is part of the
//!   kind's spelling, so `0u64` and `0` agree). It drops the spellings that only
//!   change how a value is held: a borrow (`&x`, `&mut x`), a deref (`*x`), `ref`,
//!   the value adapters (`.as_str()`, `.clone()`, `.to_owned()`, …), a
//!   full-range reindex (`x[..]`), a `let` type annotation and a type-only
//!   turbofish — so `parent[&cursor]` over `usize` and `parent[cursor]` over
//!   `&str` are one breadth-first search. Two bodies whose control flow differs,
//!   that call different functions, match different enums or read different
//!   fields do not collide; nor do two that differ in a const generic argument.
//!   Renaming item names too would make every `match self { Self::A => "a", … }`
//!   of the same arity one "group", which is shape, not shared functionality; for
//!   the same reason a body under [`LITERAL_TIER`] tokens keeps its literals.
//! * The **shim form** renames only the function's own parameters and keeps every
//!   other identifier and literal verbatim, so two thin forwarders collide exactly
//!   when they spell the same target with the same fixed arguments. The target is
//!   compared as written, not resolved: `Self::new(value)` in two types, or
//!   `read_with(text, Limits::DEFAULT)` in two modules, spell the same and forward
//!   to different items.
//!
//! A fingerprint id is `<kind>:<16 lowercase hex digits>`, the first eight bytes of
//! the BLAKE3 digest of the normal form.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

/// The structural form of a body whose normal form has fewer tokens than this is
/// not grouped: at this size a body is a one-line accessor or a required trait
/// method. Measured over the workspace (shipping, tests, benches and examples):
/// lowering the floor from 30 to 20 found 53 new shipping groups, most of them
/// small helpers written twice; below 20 the new groups are dominated by `&`/`&mut`
/// accessor twins and per-type trait impls that share a shape and no job.
pub(crate) const MIN_TOKENS: usize = 20;

/// A single-expression forwarder shorter than this is a thin shim: it is grouped
/// by its shim form, never by its structural form, since two shims that spell
/// different fixed arguments forward to different places.
pub(crate) const SHIM_MAX_TOKENS: usize = 30;

/// A body whose structural form is shorter than this is fingerprinted with its
/// literals verbatim. At this size a body's constants are most of what it says:
/// two twenty-token `match` arms that map the same variants to different strings
/// are two tables, and two that map them to the same strings are one table
/// written twice.
pub(crate) const LITERAL_TIER: usize = 30;

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
                crate::rules::RULES.contains(&id)
                    || crate::layout::RULES.contains(&id)
                    || crate::structure::RULES.contains(&id)
                    || crate::rules::DELEGATED_RULES.contains(&id)
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
    let mut grouped = Vec::new();
    if structural.len() < LITERAL_TIER {
        let level = Level {
            literals: true,
            ..Level::default()
        };
        structural_level(body, level, &mut BTreeMap::new(), &mut grouped);
    }
    let shim = shim_candidate(body).then(|| {
        let mut form = Vec::new();
        shim_form(body, params, &mut BTreeMap::new(), &mut form);
        form
    });
    BodyPrint {
        tokens: structural.len(),
        structural: fingerprint(
            "body",
            if grouped.is_empty() {
                &structural
            } else {
                &grouped
            },
        ),
        shim: shim
            .filter(|form| form.len() >= MIN_SHIM_TOKENS && structural.len() < SHIM_MAX_TOKENS)
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
/// delimiters, item names and field names verbatim, local names (bindings,
/// parameters, lifetimes) renamed to their first-appearance position, literals
/// replaced by their kind, and the value-form spellings
/// [`value_form_erasures`] names dropped.
///
/// A field name is kept because it names a member of a type, as a method name
/// does: two accessors `Self::A { tripped, .. } => Some(*tripped)` and
/// `Self::A { partial, .. } => Some(partial)` read different fields and are two
/// jobs, however alike their shape.
pub(crate) fn structural_form(
    stream: &TokenStream,
    names: &mut BTreeMap<String, usize>,
    out: &mut Vec<String>,
) {
    structural_level(stream, Level::default(), names, out);
}

/// How one delimiter level of a body is normalised.
#[derive(Clone, Copy, Debug, Default)]
struct Level {
    /// The level is the braces of a struct literal or pattern.
    fields: bool,
    /// Literals are kept verbatim rather than abstracted to their kind.
    literals: bool,
}

/// Whether the identifier at `index` names a field: `.field` (not a method
/// call), or, inside a struct literal or pattern (`fields`), a name in field
/// position — first or after a comma, and followed by `:` (not `::`), a comma
/// or the end of the braces.
fn names_a_field(tokens: &[TokenTree], index: usize, fields: bool) -> bool {
    let previous = index.checked_sub(1).and_then(|at| tokens.get(at));
    let before_previous = index.checked_sub(2).and_then(|at| tokens.get(at));
    let next = tokens.get(index + 1);
    let accessed = matches!(previous, Some(TokenTree::Punct(punct)) if punct.as_char() == '.' && punct.spacing() == Spacing::Alone)
        && !is_joint(before_previous, '.')
        && !matches!(next, Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis)
        && !is_joint(next, ':');
    let in_position = fields
        && (index == 0 || is_punct(previous, ','))
        && (next.is_none()
            || is_punct(next, ',')
            || matches!(next, Some(TokenTree::Punct(punct)) if punct.as_char() == ':' && punct.spacing() == Spacing::Alone));
    accessed || in_position
}

/// Whether the braces after `previous` hold a struct literal or pattern: they
/// follow a type or variant name (`Point {`, `Self::A {`).
fn opens_fields(previous: Option<&TokenTree>) -> bool {
    matches!(previous, Some(TokenTree::Ident(ident)) if {
        let text = ident.to_string();
        text == "Self" || text.starts_with(|first: char| first.is_ascii_uppercase())
    })
}

fn structural_level(
    stream: &TokenStream,
    level: Level,
    names: &mut BTreeMap<String, usize>,
    out: &mut Vec<String>,
) {
    let tokens: Vec<TokenTree> = stream.clone().into_iter().collect();
    let erased = value_form_erasures(&tokens);
    for (index, token) in tokens.iter().enumerate() {
        if erased[index] {
            continue;
        }
        let previous = index.checked_sub(1).and_then(|at| tokens.get(at));
        match token {
            TokenTree::Group(group) => {
                let (open, close) = delimiters(group.delimiter());
                out.push(open.to_owned());
                let fields = group.delimiter() == Delimiter::Brace && opens_fields(previous);
                structural_level(&group.stream(), Level { fields, ..level }, names, out);
                out.push(close.to_owned());
            }
            TokenTree::Ident(ident) => {
                let text = ident.to_string();
                if KEYWORDS.contains(&text.as_str())
                    || names_an_item(&tokens, index, &text)
                    || names_a_field(&tokens, index, level.fields)
                {
                    out.push(text);
                } else {
                    let next = names.len();
                    let position = *names.entry(text).or_insert(next);
                    out.push(format!("${position}"));
                }
            }
            TokenTree::Punct(punct) => out.push(punct_text(punct)),
            TokenTree::Literal(literal) => {
                let text = literal.to_string();
                // A tuple field (`pair.0`) is a field name, not a constant.
                if level.literals || (is_punct(previous, '.') && !is_joint(previous, '.')) {
                    out.push(text);
                } else {
                    out.push(literal_kind(&text).to_owned());
                }
            }
        }
    }
}

/// The value-form adapters the structural form drops: a no-argument method that
/// changes how a value is held (borrowed, copied, owned) and not what it is.
/// `next.as_str()` and `next` (over `&str` and over `usize`) are one algorithm.
const VALUE_ADAPTERS: [&str; 11] = [
    "as_str",
    "as_ref",
    "as_slice",
    "as_mut",
    "as_deref",
    "borrow",
    "borrow_mut",
    "clone",
    "cloned",
    "copied",
    "to_owned",
];

fn is_punct(token: Option<&TokenTree>, wanted: char) -> bool {
    matches!(token, Some(TokenTree::Punct(punct)) if punct.as_char() == wanted)
}

fn is_joint(token: Option<&TokenTree>, wanted: char) -> bool {
    matches!(token, Some(TokenTree::Punct(punct)) if punct.as_char() == wanted && punct.spacing() == Spacing::Joint)
}

fn is_ident(token: Option<&TokenTree>, wanted: &str) -> bool {
    matches!(token, Some(TokenTree::Ident(ident)) if ident == wanted)
}

/// Whether `token` ends an operand, so a `&` or `*` after it is the binary
/// operator (`a & b`, `f() * 2`, `x? * y`) rather than a borrow or a deref. A
/// braced block ends a statement (`for … { … } *total += 1;`), not an operand.
fn ends_operand(token: Option<&TokenTree>) -> bool {
    match token {
        None => false,
        Some(TokenTree::Ident(ident)) => {
            let text = ident.to_string();
            matches!(text.as_str(), "self" | "Self" | "true" | "false")
                || !KEYWORDS.contains(&text.as_str())
        }
        Some(TokenTree::Literal(_)) => true,
        Some(TokenTree::Group(group)) => group.delimiter() != Delimiter::Brace,
        Some(TokenTree::Punct(punct)) => punct.as_char() == '?',
    }
}

/// Whether `stream` is exactly `..`: the full-range index of `&x[..]`.
fn is_full_range(stream: &TokenStream) -> bool {
    let tokens: Vec<TokenTree> = stream.clone().into_iter().collect();
    tokens.len() == 2 && is_joint(tokens.first(), '.') && is_punct(tokens.get(1), '.')
}

/// Whether the `>` at `index` is the head of `->` or `=>`, not a closing angle.
fn is_arrow_head(tokens: &[TokenTree], index: usize) -> bool {
    let previous = index.checked_sub(1).and_then(|at| tokens.get(at));
    is_joint(previous, '-') || is_joint(previous, '=')
}

/// The end (exclusive) of the generic argument list whose `<` is at `open`: the
/// matching `>`, counting nested `<`/`>` and ignoring the `>` of `->` and `=>`.
fn angle_end(tokens: &[TokenTree], open: usize) -> Option<usize> {
    let mut depth = 0_usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        let TokenTree::Punct(punct) = token else {
            continue;
        };
        match punct.as_char() {
            '<' => depth += 1,
            '>' if !is_arrow_head(tokens, index) => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index + 1);
                }
            }
            ';' => return None,
            _ => {}
        }
    }
    None
}

/// Whether generic arguments name types only: no literal, `true`, `false` or
/// braced const block, whose value would change what the call does
/// (`Decompose::<true, _>` is compatibility decomposition, `::<false, _>` is not).
fn names_only_types(arguments: &[TokenTree]) -> bool {
    arguments.iter().all(|token| match token {
        TokenTree::Literal(_) => false,
        TokenTree::Ident(ident) => ident != "true" && ident != "false",
        TokenTree::Group(group) => group.delimiter() != Delimiter::Brace,
        TokenTree::Punct(_) => true,
    })
}

/// The span of a `let` binding's type annotation, `: Type` up to the `=` or `;`
/// that closes it, for the `let` at `at`.
fn let_annotation(tokens: &[TokenTree], at: usize) -> Option<(usize, usize)> {
    let mut index = at + 1;
    let colon = loop {
        match tokens.get(index)? {
            TokenTree::Punct(punct) if punct.as_char() == ';' || punct.as_char() == '=' => {
                return None;
            }
            TokenTree::Punct(punct)
                if punct.as_char() == ':'
                    && punct.spacing() == Spacing::Alone
                    && !is_joint(tokens.get(index - 1), ':') =>
            {
                break index;
            }
            _ => index += 1,
        }
    };
    let mut depth = 0_isize;
    for (index, token) in tokens.iter().enumerate().skip(colon + 1) {
        let TokenTree::Punct(punct) = token else {
            continue;
        };
        match punct.as_char() {
            '<' => depth += 1,
            '>' if !is_arrow_head(tokens, index) => depth -= 1,
            ';' => return Some((colon, index)),
            '=' if depth <= 0 && punct.spacing() == Spacing::Alone => {
                return Some((colon, index));
            }
            _ => {}
        }
    }
    Some((colon, tokens.len()))
}

/// Which of `tokens` (one delimiter level) the structural form drops so that
/// spellings of one algorithm that differ only in how a value is held
/// collide: a borrow (`&x`, `&mut x`, `&&x`), a deref (`*x`), a `ref` binding, a
/// value adapter (`.as_str()`, `.clone()`, … — [`VALUE_ADAPTERS`]), a full-range
/// reindex (`x[..]`), a `let` type annotation and a turbofish that names types
/// only (`::<T>`; a const argument is kept, see [`names_only_types`]). Index
/// forms follow: `m[&k]` is `m[k]`. The binary `&`, `&&`, `*` and their compound
/// assignments are kept.
fn value_form_erasures(tokens: &[TokenTree]) -> Vec<bool> {
    let mut erased = vec![false; tokens.len()];
    let mut index = 0;
    while index < tokens.len() {
        let previous = index.checked_sub(1).and_then(|at| tokens.get(at));
        match &tokens[index] {
            TokenTree::Punct(punct) if matches!(punct.as_char(), '&' | '*') => {
                if ends_operand(previous) {
                    // A binary operator; step over `&&`, `&=`, `*=` whole.
                    index += if punct.spacing() == Spacing::Joint {
                        2
                    } else {
                        1
                    };
                    continue;
                }
                erased[index] = true;
                if punct.as_char() == '&' && is_ident(tokens.get(index + 1), "mut") {
                    erased[index + 1] = true;
                    index += 1;
                }
            }
            TokenTree::Punct(punct)
                if punct.as_char() == '.'
                    && punct.spacing() == Spacing::Alone
                    && !is_joint(previous, '.') =>
            {
                let adapter = matches!(
                    tokens.get(index + 1),
                    Some(TokenTree::Ident(ident)) if VALUE_ADAPTERS.contains(&ident.to_string().as_str())
                ) && matches!(
                    tokens.get(index + 2),
                    Some(TokenTree::Group(group))
                        if group.delimiter() == Delimiter::Parenthesis && group.stream().is_empty()
                );
                if adapter {
                    erased[index..index + 3].fill(true);
                    index += 3;
                    continue;
                }
            }
            TokenTree::Punct(punct)
                if punct.as_char() == ':'
                    && punct.spacing() == Spacing::Joint
                    && is_punct(tokens.get(index + 1), ':')
                    && is_punct(tokens.get(index + 2), '<') =>
            {
                if let Some(end) = angle_end(tokens, index + 2)
                    && names_only_types(&tokens[index + 3..end - 1])
                {
                    erased[index..end].fill(true);
                    index = end;
                    continue;
                }
            }
            TokenTree::Group(group)
                if group.delimiter() == Delimiter::Bracket
                    && ends_operand(previous)
                    && is_full_range(&group.stream()) =>
            {
                erased[index] = true;
            }
            TokenTree::Ident(ident) if ident == "ref" => {
                erased[index] = true;
                if is_ident(tokens.get(index + 1), "mut") {
                    erased[index + 1] = true;
                    index += 1;
                }
            }
            TokenTree::Ident(ident) if ident == "let" => {
                if let Some((start, end)) = let_annotation(tokens, index) {
                    erased[start..end].fill(true);
                }
            }
            _ => {}
        }
        index += 1;
    }
    erased
}

/// The shim form of `stream`: parameters renamed, everything else verbatim.
///
/// A path is kept as spelt, so `Self::…` and a module-local callee name whatever
/// the enclosing type or module holds. A constructor of one type therefore shares
/// its form with every other type's constructor of the same spelling: `Self(Vec::new())`,
/// `Self::new(value)`, `Self(iter.into_iter().collect())`, or a field-wise
/// `Self { a: a.into(), b: b.into() }` whose parameters are named after the fields.
/// Such constructors of unrelated types share a shape and no job: each builds its
/// own type.
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

/// Every string literal in `stream` with its 1-based line inside the stream,
/// decoded: a `"…"` or raw string, or a byte string whose bytes are UTF-8.
pub(crate) fn string_literals(stream: &TokenStream, out: &mut Vec<(String, usize)>) {
    for token in stream.clone() {
        match token {
            TokenTree::Group(group) => string_literals(&group.stream(), out),
            TokenTree::Literal(literal) => {
                let line = literal.span().start().line;
                match syn::Lit::new(literal) {
                    syn::Lit::Str(text) => out.push((text.value(), line)),
                    syn::Lit::ByteStr(bytes) => {
                        if let Ok(text) = String::from_utf8(bytes.value()) {
                            out.push((text, line));
                        }
                    }
                    _ => {}
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
pub(crate) fn classify_digits(digits: &str) -> Option<&'static str> {
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

    use super::{
        HEX_LOWER, HEX_UPPER, body_print, hex_tables, integer_value, string_literals,
        structural_form,
    };

    fn tokens(source: &str) -> TokenStream {
        TokenStream::from_str(source).expect("test source lexes")
    }

    fn form(source: &str) -> Vec<String> {
        let mut out = Vec::new();
        structural_form(&tokens(source), &mut BTreeMap::new(), &mut out);
        out
    }

    /// A rule id is one the census runs or one another gate computes; any
    /// other `rule:` id is refused, so a misspelt rule cannot silently match
    /// nothing.
    #[test]
    fn a_rule_id_is_a_census_rule_or_a_delegated_one() {
        assert!(super::is_fingerprint_id("rule:std-default-hasher"));
        assert!(super::is_fingerprint_id("rule:raw-hash-domain"));
        assert!(super::is_fingerprint_id("rule:shared-hash-domain"));
        assert!(super::is_fingerprint_id("rule:vocabulary-literal"));
        assert!(super::is_fingerprint_id("rule:home-literal"));
        assert!(!super::is_fingerprint_id("rule:raw-hash-domains"));
        assert!(!super::is_fingerprint_id("rule:"));
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

    /// The datalog shortest-path pair the thirty-token, verbatim-borrow census
    /// could not see: one over `usize`, one over `&str`.
    #[test]
    fn borrow_deref_clone_and_index_forms_are_one_value_form() {
        for (left, right) in [
            ("cursor = parent[&cursor];", "cursor = parent[cursor];"),
            (
                "for &next in g.get(&node) { s.insert(next); }",
                "for next in g.get(node) { s.insert(next.as_str()); }",
            ),
            ("let x = &mut v; f(*x)", "let x = v; f(x)"),
            (
                "f(a.clone(), b.to_owned(), c.as_ref(), d.cloned())",
                "f(a, b, c, d)",
            ),
            ("g(&text[..])", "g(text)"),
            (
                "if let Some(ref mut x) = y { h(&&x) }",
                "if let Some(x) = y { h(x) }",
            ),
            ("let total = 0u64;", "let total = 7;"),
        ] {
            assert_eq!(form(left), form(right), "{left} / {right}");
        }
    }

    /// A `let` annotation and a type-only turbofish are erased; a const argument
    /// is behaviour, and is kept.
    #[test]
    fn types_are_erased_and_const_arguments_are_kept() {
        let unit = form("let m: BTreeMap<usize, usize> = BTreeMap::new();");
        assert_eq!(unit, form("let m: BTreeMap<&str, &str> = BTreeMap::new();"));
        assert_eq!(unit, form("let m = BTreeMap::new();"));
        assert_eq!(form("let v: Vec<Vec<u8>> = w;"), form("let v = w;"));
        assert_eq!(form("x.parse::<u64>()"), form("x.parse::<u32>()"));
        assert_eq!(form("Vec::<u8>::new()"), form("Vec::new()"));
        assert_ne!(
            form("Decompose::<true, _>::new(n)"),
            form("Decompose::<false, _>::new(n)")
        );
        // A primitive outside an annotation or turbofish is still arithmetic.
        assert_ne!(form("x as u32"), form("x as u64"));
    }

    /// Only the prefix `&` and `*` are borrows and derefs; the binary operators
    /// and their compound assignments stay.
    #[test]
    fn binary_ampersand_and_star_are_kept() {
        assert_ne!(form("a & b"), form("a b"));
        assert_ne!(form("a * b"), form("a b"));
        assert_ne!(form("f(x) * 2"), form("f(x) 2"));
        assert_ne!(form("a &= b"), form("a = b"));
        assert_ne!(form("a *= b"), form("a = b"));
        assert_eq!(form("a && &b"), form("a && b"));
        assert_ne!(form("a && b"), form("a & b"));
        // After a block a `*` opens a statement: it is a deref.
        assert_eq!(
            form("for i in v { f(i); } *t += 1;"),
            form("for i in v { f(i); } t += 1;")
        );
    }

    /// A field names a member of a type, as a method does: accessors of
    /// different fields are different jobs.
    #[test]
    fn field_names_are_kept() {
        assert_ne!(form("self.first + 1"), form("self.second + 1"));
        assert_ne!(form("pair.0"), form("pair.1"));
        assert_ne!(
            form("match self { Self::A { tripped, .. } => Some(*tripped), _ => None }"),
            form("match self { Self::A { partial, .. } => Some(partial), _ => None }")
        );
        assert_eq!(
            form("match self { Self::A { tripped: t, .. } => Some(*t), _ => None }"),
            form("match self { Self::A { tripped: x, .. } => Some(x), _ => None }")
        );
    }

    /// Below [`super::LITERAL_TIER`] a body is fingerprinted with its literals:
    /// two small tables with different strings are two tables.
    #[test]
    fn small_bodies_keep_their_literals() {
        let params = BTreeSet::new();
        let print = |source: &str| body_print(&tokens(source), &params).structural;
        let small = "match self { Self::A => \"a\", Self::B => \"b\", Self::C => \"c\" }";
        let other = "match self { Self::A => \"x\", Self::B => \"y\", Self::C => \"z\" }";
        assert_ne!(print(small), print(other));
        assert_eq!(print(small), print(small));
        assert_eq!(print(LOOP_A), print(LOOP_B));
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
        // The digits are rendered, not spelt, so this file holds no table.
        let lower = purrdf_hash::hex::encode(&[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]);
        let upper = lower.to_ascii_uppercase();
        let near = format!("{}F", &lower[..15]);
        for (source, expected) in [
            (format!("b\"{lower}\""), Some(HEX_LOWER)),
            (format!("\"{upper}\""), Some(HEX_UPPER)),
            (format!("br\"{lower}\""), Some(HEX_LOWER)),
            (
                "[b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f']".to_owned(),
                Some(HEX_LOWER),
            ),
            (
                "['0','1','2','3','4','5','6','7','8','9','A','B','C','D','E','F']".to_owned(),
                Some(HEX_UPPER),
            ),
            (format!("b\"{near}\""), None),
            (format!("b\"{lower}g\""), None),
            ("[b'0', b'1']".to_owned(), None),
        ] {
            let mut found = BTreeSet::new();
            hex_tables(&tokens(&source), &mut found);
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

    #[test]
    fn string_literals_are_decoded_in_every_spelling_with_their_line() {
        let mut found = Vec::new();
        string_literals(
            &tokens("f(\"a\\u{62}\");\nlet x = r#\"q\"#;\n[b\"bytes\", b\"\\xff\"]; 'c'; 7"),
            &mut found,
        );
        assert_eq!(
            found,
            vec![
                ("ab".to_owned(), 1),
                ("q".to_owned(), 2),
                ("bytes".to_owned(), 3)
            ]
        );
    }
}
