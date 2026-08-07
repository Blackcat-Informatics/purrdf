// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact label alphabets for the syntax this workspace *emits* -- blank-node
//! labels, XML `NCName`s, XML character data -- plus the deterministic
//! `(label, scope)` <-> token codec ([`encode_blank_label`] /
//! [`decode_blank_label`]) every serializer and every text parser goes through,
//! so an out-of-alphabet label never becomes an unreadable document and two
//! distinct blank nodes never become one.
//!
//! # Ingress-liberal, egress-exact
//!
//! The workspace's first-party SPARQL/Turtle lexer
//! (`crates/sparql-algebra/src/lexer.rs`, `is_pn_chars`/`is_pn_chars_base`)
//! deliberately over-accepts on *parse*: every scalar above `0x7F` passes,
//! rather than the exact ranges the grammar names for `PN_CHARS_BASE`. That
//! approximation is sound on ingress -- a superset lexer can only accept
//! documents a conforming parser would also accept, never reject one, and it
//! keeps the lexer's hot path a single branch. It is unsound on *egress*: a
//! label this workspace writes must be re-readable by every external
//! conforming parser, which implements the grammar's exact Unicode ranges,
//! not this workspace's liberal approximation. This module is therefore the
//! exact egress contract for label syntax -- the ranges below are transcribed
//! verbatim from the W3C Turtle/SPARQL `PN_CHARS_BASE`/`PN_CHARS` productions
//! and the XML 1.0 `NameStartChar`/`NameChar`/`Char` productions, not
//! approximated.
//!
//! # Two rules, and nothing else
//!
//! A serialized blank node has ONE string slot, but the IR identifies a blank
//! node by a `(label, scope)` PAIR (C0.2), and the label can be any string at
//! all -- including one the target syntax cannot spell. [`encode_blank_label`]
//! is the total, injective, deterministic function from that pair to a token:
//!
//! 1. **Foreign data is untouched.** A label at [`BlankScope::DEFAULT`] that is
//!    legal under the target alphabet and does NOT begin with the reserved
//!    [`ESCAPE_MARKER`] is written VERBATIM, byte for byte, borrowed. Every
//!    label in every document this workspace did not write falls here, so a
//!    parse/serialize pass over foreign data is a pure pass-through in both
//!    directions and no external label can be rewritten, merged or churned.
//! 2. **Everything else is enveloped.** A non-default scope, an
//!    out-of-alphabet label, or a label inside the reserved marker namespace is
//!    written as ONE self-describing envelope that carries both the scope and
//!    the label:
//!
//!    ```text
//!    envelope ::= 'purrdfesc' scope? '_' body
//!    scope    ::= [1-9][0-9]*          -- canonical decimal, omitted for scope 0
//!    body     ::= ( [A-Za-z0-9] | '_' HEX HEX HEX HEX HEX HEX )*
//!    ```
//!
//!    The body encodes the label scalar by scalar: an ASCII letter or digit
//!    passes through, every other scalar becomes `_` plus its code point as
//!    exactly six UPPERCASE hex digits (`_0000D7` for `×`). The envelope is
//!    therefore `[A-Za-z0-9_]+` starting with a letter, which is simultaneously
//!    a legal `BLANK_NODE_LABEL`, a legal `NCName` and legal XML character
//!    data -- one spelling satisfies every alphabet at once.
//!
//! The properties that follow, which callers actually rely on:
//!
//! - **Injective over `(label, scope)`** -- the verbatim and envelope images
//!   are disjoint (an envelope always begins with the reserved marker, a
//!   verbatim label never does), the scope digits are terminated by the first
//!   `_`, and the body's `_` always introduces exactly six hex digits. Two
//!   distinct pairs therefore never share a token, in any alphabet.
//! - **Deterministic** -- a pure function of `(label, scope, alphabet)`: no
//!   clock, randomness, hash-iteration order or document-level state, so the
//!   same dataset always serializes to the same bytes, and the row-at-a-time
//!   SPARQL-results writers can use the same codec as the whole-dataset RDF
//!   serializers with no shared collision map.
//! - **Total** -- serialization never refuses a label. A blank-node label is
//!   not part of a graph's meaning (RDF identifies blank nodes only up to
//!   renaming), so rewriting one preserves the graph exactly, while emitting an
//!   out-of-alphabet label would produce a document no conforming parser could
//!   read back.
//!
//! # Ingress: pass-through, or an image-checked envelope
//!
//! [`decode_blank_label`] is the single text-ingress inverse, and it accepts an
//! envelope ONLY when re-encoding what it decoded reproduces the token BYTE
//! EXACTLY (the image test):
//!
//! - a token that does not begin with [`ESCAPE_MARKER`] is `(token, DEFAULT)`,
//!   with no transformation whatsoever -- so `_:a.b`, `_:a..b` and `_:a...b`
//!   are three distinct nodes, exactly as the document spells them;
//! - a marker-prefixed token that decodes to `(label, scope)` is accepted iff
//!   `encode_blank_label(label, scope, alphabet)` is the token again; otherwise
//!   it is `(token, DEFAULT)` verbatim, because no serializer could have
//!   written it.
//!
//! `decode(t) == (l, s)` with the image test passing therefore holds **iff**
//! `encode(l, s, alphabet) == t`, which is what makes the round trip restore
//! blank-node label IDENTITY rather than mere isomorphism, and makes
//! `serialize(parse(serialize(D)))` byte-identical to `serialize(D)`.
//!
//! The one thing a caller must know about the reserved namespace: a foreign
//! token like `_:purrdfesc_abc` is NOT in the encoder's image (the encoder
//! would have written `_:abc` for the label `abc`), so it fails the image test
//! and is kept verbatim as the label `purrdfesc_abc` -- distinct from the label
//! `abc`, which is the whole point. On the way back out it is marker-prefixed,
//! so it is enveloped, and THAT envelope is in the image: the bytes move once,
//! on the first write, and are a fixed point from then on.
//!
//! Callers that want a *chosen* relabeling rather than a mechanical envelope
//! have explicit recourse operations -- `canonical_relabel`, `skolemize` and
//! `deskolemize` in [`crate::ir`] -- which rewrite the dataset before egress
//! so the labels in the document are the caller's, not the codec's.
//! Canonicalization's `c14n{n}` labels are legal in every alphabet and outside
//! the reserved namespace, so the encoding is the identity on them and their
//! bytes never move.
//!
//! # Relabeling vs. canonicalization: consistent doctrines, different roles
//!
//! RDFC-1.0 canonicalization refuses to relabel for a different reason: its
//! output bytes are not a rendering choice, they mint the dataset's
//! content-addressed identity (two isomorphic graphs must canonicalize to
//! byte-identical output, and two non-isomorphic graphs must not collide).
//! Relabeling a canonical label to dodge an alphabet constraint would be
//! indistinguishable, downstream, from silently changing which graph the
//! identity was computed over. It never needs to: canonicalization mints
//! `c14n0`, `c14n1`, … labels (see [`crate::ir::canon`]) whose ASCII letters
//! and digits are a subset of every alphabet this module defines, so a
//! canonical label is legal everywhere and [`encode_blank_label`] is the
//! identity on it.

use core::cmp::Ordering;
use std::borrow::Cow;

use crate::BlankScope;

/// Which label grammar [`is_valid_label`] should check against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelAlphabet {
    /// The W3C Turtle/SPARQL `BLANK_NODE_LABEL` production (the part after
    /// `_:`); see [`is_valid_blank_node_label`]. The alphabet of the Turtle
    /// family (Turtle / TriG / N-Triples / N-Quads) and of every syntax whose
    /// specification types a blank-node identifier as a `BLANK_NODE_LABEL`
    /// rather than as free text (HexTuples, JSON-LD, YAML-LD, the SPARQL
    /// result formats).
    BlankNodeLabel,
    /// The XML 1.0 `NCName` production; see [`is_valid_ncname`]. The alphabet
    /// of the RDF/XML `rdf:nodeID` attribute.
    NcName,
    /// Non-empty text that XML 1.0 character data carries through unchanged;
    /// see [`is_valid_xml_text`]. The alphabet of the TriX `<id>` element.
    XmlText,
    /// No syntactic constraint at all: EVERY string is legal, including the
    /// empty one. This is the alphabet of the OWNED term model
    /// ([`RdfTerm::BlankNode`](crate::RdfTerm::BlankNode)), whose blank-node
    /// slot is a plain `String` rather than a token in some document syntax —
    /// the surface [`BlankScope::qualify_label`] and
    /// [`BlankScope::unqualify_label`] encode for.
    ///
    /// The reserved marker namespace still applies here (it is what keeps the
    /// encoding injective), so a scoped or marker-prefixed label is enveloped
    /// on this surface exactly as it would be in a document; only the
    /// *alphabet* constraint is lifted, so an owned label may hold any scalar.
    Unconstrained,
}

/// The reserved marker that opens every envelope [`encode_blank_label`]
/// writes: `purrdfesc` followed by the scope's canonical decimal digits (none
/// at [`BlankScope::DEFAULT`]), an `_`, and the encoded body.
///
/// It names a RESERVED label namespace. A label that begins with it is always
/// enveloped — even when it is otherwise legal and unscoped — which is exactly
/// what keeps the encoding injective without any document-level state: the
/// verbatim and envelope images can then never overlap.
pub const ESCAPE_MARKER: &str = "purrdfesc";

/// Whether `label` is legal under `alphabet`.
///
/// Dispatches to [`is_valid_blank_node_label`], [`is_valid_ncname`], or
/// [`is_valid_xml_text`], per [`LabelAlphabet`];
/// [`Unconstrained`](LabelAlphabet::Unconstrained) admits every string.
#[must_use]
pub fn is_valid_label(label: &str, alphabet: LabelAlphabet) -> bool {
    match alphabet {
        LabelAlphabet::BlankNodeLabel => is_valid_blank_node_label(label),
        LabelAlphabet::NcName => is_valid_ncname(label),
        LabelAlphabet::XmlText => is_valid_xml_text(label),
        LabelAlphabet::Unconstrained => true,
    }
}

/// Encode a `(label, scope)` pair into `alphabet`'s single-slot token,
/// returning the label BORROWED and byte-identical when it can be written
/// verbatim.
///
/// The one egress function every serializer goes through. Pure, deterministic,
/// stateless, and **injective** for a fixed `alphabet`: distinct pairs always
/// produce distinct tokens, so blank-node co-reference survives serialization
/// exactly and two distinct blank nodes can never merge.
///
/// # The two rules
///
/// - `scope` is [`BlankScope::DEFAULT`], `label` is legal under `alphabet`, and
///   `label` does not begin with [`ESCAPE_MARKER`] → the label itself,
///   borrowed, byte for byte. This is where all foreign data lands.
/// - otherwise → the envelope `purrdfesc{scope}_{body}` (the scope digits are
///   omitted at [`BlankScope::DEFAULT`]), where `body` encodes the label scalar
///   by scalar: an ASCII letter or digit passes through as itself, every other
///   scalar becomes `_` plus its code point as exactly six uppercase hex digits
///   (`_0000D7` for `×`).
///
/// The envelope is always `[A-Za-z0-9_]+` beginning with a letter, so it is
/// simultaneously a legal `BLANK_NODE_LABEL` (including the rule that a label
/// may not end in `.`), a legal `NCName`, and legal XML character data — one
/// spelling satisfies every alphabet at once.
///
/// Decoding an envelope is unambiguous: the scope digits are the maximal digit
/// run after the marker and are always terminated by the `_` that opens the
/// body, and inside the body a `_` always introduces exactly six hex digits
/// while no pass-through character is `_`. The envelope image cannot collide
/// with a verbatim label either, because a label that already begins with
/// [`ESCAPE_MARKER`] is enveloped as well, even when it is legal and unscoped —
/// the one case where a default-scope legal label does not pass through.
#[must_use]
pub fn encode_blank_label(label: &str, scope: BlankScope, alphabet: LabelAlphabet) -> Cow<'_, str> {
    if scope == BlankScope::DEFAULT
        && is_valid_label(label, alphabet)
        && !label.starts_with(ESCAPE_MARKER)
    {
        return Cow::Borrowed(label);
    }
    let mut encoded = String::with_capacity(ESCAPE_MARKER.len() + label.len() + 16);
    encoded.push_str(ESCAPE_MARKER);
    if scope != BlankScope::DEFAULT {
        use std::fmt::Write as _;
        // Canonical decimal, never zero-padded: the decode below accepts only
        // this spelling, so the encoding stays injective over scopes.
        let _ = write!(encoded, "{}", scope.ordinal());
    }
    encoded.push('_');
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() {
            encoded.push(ch);
        } else {
            encoded.push('_');
            push_hex6(ch as u32, &mut encoded);
        }
    }
    debug_assert!(
        is_valid_label(&encoded, alphabet),
        "encode_blank_label must always produce a token legal under the target alphabet"
    );
    debug_assert!(
        decode_envelope(&encoded).is_some_and(|(l, s)| l == label && s == scope),
        "every envelope must decode back to the pair it encodes"
    );
    Cow::Owned(encoded)
}

/// Encode `label` at [`BlankScope::DEFAULT`] into `alphabet` — the unscoped
/// shorthand for [`encode_blank_label`].
///
/// This is what a caller holding a RAW label and no scope (an owned-model
/// rendering, a diagnostic, a kernel emitter) applies at egress. See
/// [`encode_blank_label`] for the encoding and its properties.
#[must_use]
pub fn escape_label(label: &str, alphabet: LabelAlphabet) -> Cow<'_, str> {
    encode_blank_label(label, BlankScope::DEFAULT, alphabet)
}

/// Decode a parsed blank-node token into the `(label, scope)` pair it denotes —
/// the single text-ingress inverse of [`encode_blank_label`].
///
/// # The two rules, and the image test
///
/// - A token that does NOT begin with [`ESCAPE_MARKER`] is `(token, DEFAULT)`,
///   borrowed and byte for byte, with no transformation whatsoever. Every token
///   in every document this workspace did not write falls here, so distinct
///   foreign labels always stay distinct nodes.
/// - A marker-prefixed token is decoded as an envelope, and the result is
///   ACCEPTED only if re-encoding it reproduces the token BYTE EXACTLY:
///   `encode_blank_label(label, scope, alphabet) == token`. A token that fails
///   the test — a malformed body, a zero-padded or zero scope, or an envelope
///   the encoder would never have written for that pair (`purrdfesc_abc`, since
///   the label `abc` is written verbatim) — is kept verbatim as
///   `(token, DEFAULT)`.
///
/// So `decode_blank_label(t, α) == (l, s)` by the envelope branch **iff**
/// `encode_blank_label(l, s, α) == t`. That equivalence is what makes a
/// parse/serialize cycle byte-stable and label-identity-preserving, and it is
/// asserted in debug builds on every accepted envelope.
#[must_use]
pub fn decode_blank_label(token: &str, alphabet: LabelAlphabet) -> (Cow<'_, str>, BlankScope) {
    // The hot path for real data: one prefix compare, no allocation, no scan.
    if !token.starts_with(ESCAPE_MARKER) {
        return (Cow::Borrowed(token), BlankScope::DEFAULT);
    }
    let Some((label, scope)) = decode_envelope(token) else {
        return (Cow::Borrowed(token), BlankScope::DEFAULT);
    };
    if encode_blank_label(&label, scope, alphabet).as_ref() != token {
        // Not in the encoder's image under this alphabet, so no serializer
        // could have written it: it denotes itself.
        return (Cow::Borrowed(token), BlankScope::DEFAULT);
    }
    (Cow::Owned(label), scope)
}

/// Re-target an OWNED-model blank label — the
/// [`Unconstrained`](LabelAlphabet::Unconstrained) spelling
/// [`BlankScope::qualify_label`] writes into
/// [`RdfTerm::BlankNode`](crate::RdfTerm::BlankNode)'s single string slot —
/// into `alphabet`'s token space.
///
/// # Why a re-target, and not a second escape
///
/// The owned model already carries an ENCODED label, so applying the egress
/// encoding to it again would envelope an envelope: the document would spell a
/// scoped or marker-prefixed node one layer deeper than ingress unwraps, and
/// the round trip would stop restoring label identity. Decoding the owned
/// spelling first and re-encoding the `(label, scope)` pair it denotes makes
/// the composition EXACT —
/// `retarget_owned_label(qualify_label(l, s), α) == encode_blank_label(l, s, α)`
/// for every pair and every alphabet — so an owned-model detour costs nothing.
///
/// A label a caller MINTED by hand rather than read out of the owned rendering
/// is not an envelope, so it decodes to itself at [`BlankScope::DEFAULT`] and
/// is simply escaped into `alphabet`: egress stays total either way.
#[must_use]
pub fn retarget_owned_label(owned: &str, alphabet: LabelAlphabet) -> Cow<'_, str> {
    match decode_blank_label(owned, LabelAlphabet::Unconstrained) {
        (Cow::Borrowed(label), scope) => encode_blank_label(label, scope, alphabet),
        (Cow::Owned(label), scope) => {
            Cow::Owned(encode_blank_label(&label, scope, alphabet).into_owned())
        }
    }
}

/// Parse `token` as an envelope: [`ESCAPE_MARKER`], the scope's canonical
/// decimal digits (absent at [`BlankScope::DEFAULT`]), `_`, and the encoded
/// body. `None` when any part is malformed.
///
/// This is the syntactic half of the decode; [`decode_blank_label`] adds the
/// image test that rejects a well-formed envelope the encoder would not have
/// written.
fn decode_envelope(token: &str) -> Option<(String, BlankScope)> {
    let rest = token.strip_prefix(ESCAPE_MARKER)?;
    // The scope digits are the maximal digit run, always terminated by the `_`
    // that opens the body — so the split is unambiguous even when the body
    // itself starts with digits.
    let digits_len = rest.bytes().take_while(u8::is_ascii_digit).count();
    let (digits, body) = rest.split_at(digits_len);
    let body = body.strip_prefix('_')?;
    let scope = if digits.is_empty() {
        BlankScope::DEFAULT
    } else {
        // `encode_blank_label` writes a canonical, non-zero decimal, so a
        // zero-padded or zero ordinal is outside its image and must not decode
        // (the image test below would reject it anyway; refusing here keeps the
        // grammar's statement exact).
        if digits.starts_with('0') {
            return None;
        }
        let ordinal = digits.parse::<u32>().ok()?;
        if ordinal == 0 {
            return None;
        }
        BlankScope(ordinal)
    };
    let mut decoded = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(ch) = chars.next() {
        if ch.is_ascii_alphanumeric() {
            decoded.push(ch);
            continue;
        }
        if ch != '_' {
            return None;
        }
        let mut cp: u32 = 0;
        for _ in 0..6 {
            cp = cp * 16 + chars.next().and_then(hex6_digit)?;
        }
        decoded.push(char::from_u32(cp)?);
    }
    Some((decoded, scope))
}

/// One digit of a fixed-width escape group: `0-9` or UPPERCASE `A-F`, matching
/// exactly what [`push_hex6`] writes. Lowercase is deliberately refused so the
/// decode accepts only the escape's own image.
fn hex6_digit(c: char) -> Option<u32> {
    match c {
        '0'..='9' => Some(c as u32 - '0' as u32),
        'A'..='F' => Some(c as u32 - 'A' as u32 + 10),
        _ => None,
    }
}

/// Append `cp` as exactly six uppercase hex digits (24 bits covers the whole
/// `0..=0x10FFFF` scalar range), the fixed-width escape body [`escape_label`]
/// writes after each `_`.
fn push_hex6(cp: u32, out: &mut String) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for nibble in (0..6).rev() {
        let index = ((cp >> (nibble * 4)) & 0xF) as usize;
        out.push(char::from(HEX[index]));
    }
}

/// Whether `label` is legal as a serialized blank-node label (`_:{label}`).
///
/// Implements the exact W3C Turtle/SPARQL production
/// `BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9]) ((PN_CHARS | '.')* PN_CHARS)?`,
/// validating the part after `_:`. A label that fails this check cannot be
/// emitted by any codec without producing a document that no conforming
/// parser (including PurRDF's own) can read back.
#[must_use]
pub fn is_valid_blank_node_label(label: &str) -> bool {
    let mut chars = label.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(is_pn_chars_u(first) || first.is_ascii_digit()) {
        return false;
    }
    // The grammar is `(PN_CHARS | '.')* PN_CHARS` after the first character:
    // any run of PN_CHARS/'.' is legal mid-label, but the *final* character
    // must be PN_CHARS (never '.'). Track whether the most recently accepted
    // character was a '.' and reject at the end if so.
    let mut trailing_dot = false;
    for ch in chars {
        if ch == '.' {
            trailing_dot = true;
        } else if is_pn_chars(ch) {
            trailing_dot = false;
        } else {
            return false;
        }
    }
    !trailing_dot
}

/// Whether `prefix` is legal as a mint-time PREFIX for [`is_valid_blank_node_label`]:
/// a string a caller can safely splice in front of every label an evaluator mints,
/// so `{prefix}{stem}{n}` (`stem` one of this workspace's fixed mint stems --
/// `c`, `bnode`, `lc` -- and `n` a decimal counter) is always a legal
/// `BLANK_NODE_LABEL`.
///
/// # Why this differs from [`is_valid_blank_node_label`]
///
/// A PREFIX never occupies the FINAL position of the label it seeds -- the mint
/// stem's first letter and the counter's digits always follow it -- so the
/// `BLANK_NODE_LABEL` grammar's "the last character is never `.`" rule does not
/// apply to a prefix's own last character; it applies to the *mint stem's* last
/// character instead, which is always a decimal digit and therefore always
/// legal. Every OTHER position in the prefix is exactly as constrained as it
/// would be inside a full label: the first character must be a legal
/// `BLANK_NODE_LABEL` lead (`PN_CHARS_U` or a digit) and every character after
/// it must be `PN_CHARS` or `.`.
///
/// The empty prefix is legal: it is exactly "no prefix", and every mint stem
/// already starts with a legal lead character on its own.
#[must_use]
pub fn is_valid_blank_node_label_prefix(prefix: &str) -> bool {
    let mut chars = prefix.chars();
    let Some(first) = chars.next() else {
        return true;
    };
    if !(is_pn_chars_u(first) || first.is_ascii_digit()) {
        return false;
    }
    chars.all(|ch| is_pn_chars(ch) || ch == '.')
}

/// Whether `label` is a legal XML 1.0 `NCName`.
///
/// Implements the exact production `NCName ::= NCNameStartChar NCNameChar*`,
/// where `NCNameStartChar = NameStartChar - ':'` and `NCNameChar` is
/// `NCNameStartChar` plus `'-' | '.' | [0-9] | #xB7 | [#x0300-#x036F] |
/// [#x203F-#x2040]`. `NCNameStartChar` is character-for-character identical
/// to the Turtle/SPARQL `PN_CHARS_U` alphabet, so this reuses the same
/// range tables as [`is_valid_blank_node_label`]. Unlike a blank-node label,
/// an `NCName` MAY end in `.`: the grammar places no restriction on the
/// final character.
#[must_use]
pub fn is_valid_ncname(label: &str) -> bool {
    let mut chars = label.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_pn_chars_u(first) {
        return false;
    }
    chars.all(|ch| is_pn_chars(ch) || ch == '.')
}

/// Whether `label` survives XML 1.0 character data unchanged.
///
/// Requires a non-empty string in which every scalar is a legal XML 1.0 `Char`
/// (`#x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] | [#x10000-#x10FFFF]`,
/// so C0 controls and `U+FFFE`/`U+FFFF` are excluded; surrogates are not
/// representable in a `str` at all) and no scalar is whitespace. XML has no
/// representation whatsoever for a C0 control -- not even a character
/// reference -- so such a label cannot be carried by an XML document at any
/// escaping level. Whitespace is excluded for a second reason: XML normalizes
/// line endings and collapses whitespace in attribute values, and this
/// workspace's TriX reader trims `<id>` element text, so a whitespace-bearing
/// label cannot carry identity through an XML round trip even though XML can
/// represent the characters.
#[must_use]
pub fn is_valid_xml_text(label: &str) -> bool {
    !label.is_empty() && label.chars().all(is_xml_text_char)
}

/// One scalar of [`is_valid_xml_text`]: a legal XML 1.0 `Char` that is not
/// whitespace. `#x9`/`#xA`/`#xD` are legal `Char`s but are whitespace, so the
/// whitespace test alone removes them from the `[#x20-…]` gap below.
fn is_xml_text_char(c: char) -> bool {
    !c.is_whitespace()
        && matches!(c as u32, 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x0010_FFFF)
}

/// Inclusive Unicode scalar-value range `[lo, hi]`.
type CharRange = (u32, u32);

/// `PN_CHARS_BASE` from the W3C Turtle/SPARQL grammar, which is also
/// character-for-character the XML 1.0 `NameStartChar` production minus
/// `':'` and `'_'` (`'_'` is folded into `PN_CHARS_U` instead, see
/// [`is_pn_chars_u`]). Ranges are sorted and non-overlapping, which
/// [`in_ranges`] relies on for binary search.
const PN_CHARS_BASE_RANGES: &[CharRange] = &[
    (0x0041, 0x005A),   // [A-Z]
    (0x0061, 0x007A),   // [a-z]
    (0x00C0, 0x00D6),   // [#xC0-#xD6]
    (0x00D8, 0x00F6),   // [#xD8-#xF6]
    (0x00F8, 0x02FF),   // [#xF8-#x2FF]
    (0x0370, 0x037D),   // [#x370-#x37D]
    (0x037F, 0x1FFF),   // [#x37F-#x1FFF]
    (0x200C, 0x200D),   // [#x200C-#x200D]
    (0x2070, 0x218F),   // [#x2070-#x218F]
    (0x2C00, 0x2FEF),   // [#x2C00-#x2FEF]
    (0x3001, 0xD7FF),   // [#x3001-#xD7FF]
    (0xF900, 0xFDCF),   // [#xF900-#xFDCF]
    (0xFDF0, 0xFFFD),   // [#xFDF0-#xFFFD]
    (0x10000, 0xEFFFF), // [#x10000-#xEFFFF]
];

/// The extra ranges `PN_CHARS`/`NCNameChar` fold in beyond `PN_CHARS_U`
/// (beyond `'-'` and `[0-9]`, which are cheap ASCII checks handled inline).
/// Sorted and non-overlapping for [`in_ranges`].
const PN_CHARS_EXTRA_RANGES: &[CharRange] = &[
    (0x00B7, 0x00B7), // #xB7
    (0x0300, 0x036F), // [#x300-#x36F]
    (0x203F, 0x2040), // [#x203F-#x2040]
];

/// Binary-search `cp` against a sorted, non-overlapping table of inclusive
/// ranges.
fn in_ranges(cp: u32, ranges: &[CharRange]) -> bool {
    ranges
        .binary_search_by(|&(lo, hi)| {
            if cp < lo {
                Ordering::Greater
            } else if cp > hi {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}

/// `PN_CHARS_BASE` (== XML `NameStartChar - ':' - '_'`).
fn is_pn_chars_base(c: char) -> bool {
    in_ranges(c as u32, PN_CHARS_BASE_RANGES)
}

/// `PN_CHARS_U ::= PN_CHARS_BASE | '_'` (== `NCNameStartChar`).
fn is_pn_chars_u(c: char) -> bool {
    c == '_' || is_pn_chars_base(c)
}

/// `PN_CHARS ::= PN_CHARS_U | '-' | [0-9] | #xB7 | [#x300-#x036F] |
/// [#x203F-#x2040]`.
fn is_pn_chars(c: char) -> bool {
    is_pn_chars_u(c) || c == '-' || c.is_ascii_digit() || in_ranges(c as u32, PN_CHARS_EXTRA_RANGES)
}

#[cfg(test)]
mod tests {
    use super::{
        ESCAPE_MARKER, LabelAlphabet, decode_blank_label, encode_blank_label, escape_label,
        is_pn_chars, is_pn_chars_u, is_valid_blank_node_label, is_valid_blank_node_label_prefix,
        is_valid_label, is_valid_ncname, is_valid_xml_text, retarget_owned_label,
    };
    use crate::BlankScope;
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    /// Every alphabet the codec targets, for sweeps that must hold on all of
    /// them — the three document syntaxes plus the owned model's unconstrained
    /// surface.
    const ALL_ALPHABETS: &[LabelAlphabet] = &[
        LabelAlphabet::BlankNodeLabel,
        LabelAlphabet::NcName,
        LabelAlphabet::XmlText,
        LabelAlphabet::Unconstrained,
    ];

    /// The scopes every sweep crosses its labels with, including the `u32`
    /// boundary the envelope's scope decimal must survive.
    const ALL_SCOPES: &[BlankScope] = &[
        BlankScope::DEFAULT,
        BlankScope(1),
        BlankScope(2),
        BlankScope(12),
        BlankScope(u32::MAX),
    ];

    /// Adversarial labels: control characters, whitespace, delimiters, the
    /// alphabet boundary gaps, non-ASCII letters, the empty label, the dotted
    /// family the old dot-doubling folded together, and labels inside the
    /// reserved marker namespace.
    const HOSTILE_LABELS: &[&str] = &[
        "",
        "a",
        "0abc",
        "a.b",
        "a..b",
        "a...b",
        "a.s1",
        "trailing.",
        "-lead",
        ".lead",
        "a b",
        "a\tb",
        "a\nb",
        "bad\u{1f}label",
        "\u{7f}",
        "<urn:x>",
        "a:b",
        "a/b",
        "\u{d7}y",
        "日本",
        "c14n0",
        "purrdfesc",
        "purrdfesc_a",
        "purrdfesc_",
        "purrdfesc1_a",
        "purrdfesc_a_000020b",
    ];

    /// Independent re-transcription of `PN_CHARS_BASE` via `matches!` range
    /// patterns, kept deliberately separate from the production binary-search
    /// table so the exhaustive sweep below cross-checks the table rather than
    /// restating it.
    fn expected_pn_chars_base(cp: u32) -> bool {
        matches!(
            cp,
            0x0041..=0x005A
                | 0x0061..=0x007A
                | 0x00C0..=0x00D6
                | 0x00D8..=0x00F6
                | 0x00F8..=0x02FF
                | 0x0370..=0x037D
                | 0x037F..=0x1FFF
                | 0x200C..=0x200D
                | 0x2070..=0x218F
                | 0x2C00..=0x2FEF
                | 0x3001..=0xD7FF
                | 0xF900..=0xFDCF
                | 0xFDF0..=0xFFFD
                | 0x10000..=0xEFFFF
        )
    }

    fn expected_pn_chars_u(cp: u32) -> bool {
        cp == 0x5F || expected_pn_chars_base(cp)
    }

    fn expected_pn_chars(cp: u32) -> bool {
        expected_pn_chars_u(cp)
            || cp == 0x2D
            || (0x30..=0x39).contains(&cp)
            || cp == 0xB7
            || (0x0300..=0x036F).contains(&cp)
            || (0x203F..=0x2040).contains(&cp)
    }

    #[test]
    fn internal_tables_match_independent_transcription() {
        for cp in 0u32..=0x2FFF {
            let Some(c) = char::from_u32(cp) else {
                continue;
            };
            assert_eq!(
                is_pn_chars_u(c),
                expected_pn_chars_u(cp),
                "pn_chars_u mismatch at {cp:#06x}"
            );
            assert_eq!(
                is_pn_chars(c),
                expected_pn_chars(cp),
                "pn_chars mismatch at {cp:#06x}"
            );
        }
    }

    /// Exhaustive sweep of all 256 single-byte scalar values, in both the
    /// leading position (a one-character label) and an inner/final position
    /// (`"a" + c`), against the [`LabelAlphabet::BlankNodeLabel`] alphabet.
    #[test]
    fn ascii_sweep_blank_node_label() {
        for byte in 0u32..=255 {
            let c = char::from_u32(byte).expect("0..=255 are always valid Unicode scalars");
            let lead = c.to_string();
            let expected_lead = expected_pn_chars_u(byte) || c.is_ascii_digit();
            assert_eq!(
                is_valid_blank_node_label(&lead),
                expected_lead,
                "lead byte {byte:#04x} ({c:?})"
            );

            // As the final character of "a{c}", the grammar requires PN_CHARS
            // (never '.') -- so the expected verdict is `expected_pn_chars`,
            // not `expected_pn_chars(byte) || c == '.'`.
            let inner = format!("a{c}");
            let expected_inner = expected_pn_chars(byte);
            assert_eq!(
                is_valid_blank_node_label(&inner),
                expected_inner,
                "inner byte {byte:#04x} ({c:?})"
            );
        }
    }

    /// Exhaustive sweep of all 256 single-byte scalar values against the
    /// [`LabelAlphabet::NcName`] alphabet.
    #[test]
    fn ascii_sweep_ncname() {
        for byte in 0u32..=255 {
            let c = char::from_u32(byte).expect("0..=255 are always valid Unicode scalars");
            let lead = c.to_string();
            let expected_lead = expected_pn_chars_u(byte);
            assert_eq!(
                is_valid_ncname(&lead),
                expected_lead,
                "lead byte {byte:#04x} ({c:?})"
            );

            // Unlike BlankNodeLabel, NCNameChar (unconditionally) permits '.',
            // including as the final character.
            let inner = format!("a{c}");
            let expected_inner = expected_pn_chars(byte) || c == '.';
            assert_eq!(
                is_valid_ncname(&inner),
                expected_inner,
                "inner byte {byte:#04x} ({c:?})"
            );
        }
    }

    #[test]
    fn unicode_boundary_accepted_blank_node_label() {
        for label in [
            "ü", "日本", "\u{200C}", // ZWNJ, PN_CHARS_BASE
            "a\u{B7}",  // middle dot, PN_CHARS-only, inner position
            "a\u{36F}", // combining mark, PN_CHARS-only, inner position
        ] {
            assert!(is_valid_blank_node_label(label), "{label:?}");
        }
    }

    #[test]
    fn unicode_boundary_rejected_blank_node_label() {
        for label in [
            "\u{D7}",    // × -- the gap just past [#xC0-#xD6]
            "\u{F7}",    // ÷ -- the gap between [#xD8-#xF6] and [#xF8-#x2FF]
            "\u{37E}",   // Greek question mark -- the gap in [#x370-#x37D]/[#x37F-#x1FFF]
            "a\u{2002}", // en-space, not in any PN_CHARS range
            "\u{2028}",  // line separator, not in any PN_CHARS_BASE range
        ] {
            assert!(!is_valid_blank_node_label(label), "{label:?}");
        }
    }

    #[test]
    fn structural_blank_node_label() {
        for label in [
            "0b",
            "b0",
            "_x",
            "a-b.c",
            "a_b",
            "c1.s5",
            "f-3c_c1",
            "ünïcode",
            "日本",
        ] {
            assert!(is_valid_blank_node_label(label), "{label:?}");
        }
        for label in [
            "",
            "\u{1f}",
            "bad\u{1f}label",
            "<urn:x>",
            "a:b",
            "a b",
            "a/b",
            "trailing.", // trailing '.' is disallowed for BLANK_NODE_LABEL specifically
            "-lead",
            ".lead",
            "a\nb",
        ] {
            assert!(!is_valid_blank_node_label(label), "{label:?}");
        }
    }

    #[test]
    fn structural_blank_node_label_prefix() {
        // Legal prefixes, including ones a full-label check would reject:
        // empty (no prefix), and a trailing '.' (illegal as a FULL label's
        // last character, legal here because the mint stem always follows).
        for prefix in [
            "",
            "f",
            "fTag_",
            "f-3c1a2d_",
            "trailing.",
            "0abc",
            "日本",
            "_x",
        ] {
            assert!(is_valid_blank_node_label_prefix(prefix), "{prefix:?}");
        }
        // Illegal prefixes: a bad lead character, or a body character outside
        // PN_CHARS/'.'.
        for prefix in ["-lead", ".lead", "a b", "a\tb", "a:b", "<urn:x>", "a/b"] {
            assert!(!is_valid_blank_node_label_prefix(prefix), "{prefix:?}");
        }
    }

    /// Every legal prefix, concatenated in front of every one of this
    /// workspace's fixed mint stems plus a decimal counter, must produce a
    /// full legal `BLANK_NODE_LABEL` -- the property the prefix check exists
    /// to guarantee.
    #[test]
    fn legal_prefix_always_yields_a_legal_minted_label() {
        const STEMS: &[&str] = &["c", "bnode", "lc"];
        for prefix in ["", "f", "fTag_", "f-3c1a2d_", "trailing.", "0abc", "日本"] {
            assert!(is_valid_blank_node_label_prefix(prefix), "{prefix:?}");
            for stem in STEMS {
                for n in [0u64, 1, 42] {
                    let minted = format!("{prefix}{stem}{n}");
                    assert!(
                        is_valid_blank_node_label(&minted),
                        "prefix {prefix:?} + stem {stem:?} + {n} = {minted:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn structural_ncname() {
        for label in [
            "b0",
            "_x",
            "a-b.c",
            "a_b",
            "c1.s5",
            "f-3c_c1",
            "ünïcode",
            "日本",
        ] {
            assert!(is_valid_ncname(label), "{label:?}");
        }
        for label in [
            "",
            "\u{1f}",
            "bad\u{1f}label",
            "<urn:x>",
            "a:b",
            "a b",
            "a/b",
            "-lead",
            ".lead",
            "a\nb",
            "0b", // NCNameStartChar excludes digits, unlike PN_CHARS_U | [0-9]
        ] {
            assert!(!is_valid_ncname(label), "{label:?}");
        }
        // Unlike BLANK_NODE_LABEL, NCName places no constraint on the final
        // character: a trailing '.' is syntactically legal.
        assert!(is_valid_ncname("trailing."));
    }

    #[test]
    fn dispatch_via_is_valid_label() {
        assert!(is_valid_label("a-b.c", LabelAlphabet::BlankNodeLabel));
        assert!(!is_valid_label("0b", LabelAlphabet::NcName));
        assert!(is_valid_label("b0", LabelAlphabet::NcName));
        assert!(is_valid_label("<urn:x>", LabelAlphabet::XmlText));
    }

    #[test]
    fn xml_text_admits_representable_non_whitespace_and_refuses_the_rest() {
        for label in ["a", "<urn:x>", "a.b", "trailing.", "日本", "\u{d7}y", "&"] {
            assert!(is_valid_xml_text(label), "{label:?}");
        }
        for label in [
            "",
            "a b",
            "a\tb",
            "a\nb",
            "a\rb",
            "bad\u{1f}label",
            "\u{fffe}",
            "\u{ffff}",
            "\u{85}", // NEL: whitespace
            "\u{a0}", // NBSP: whitespace
        ] {
            assert!(!is_valid_xml_text(label), "{label:?}");
        }
        // U+007F is a legal XML 1.0 Char (the exclusion is C0, not DEL).
        assert!(is_valid_xml_text("\u{7f}"));
    }

    // ── encode_blank_label ──────────────────────────────────────────────────

    #[test]
    fn unscoped_legal_labels_are_borrowed_byte_identically() {
        // The one legal label that does NOT pass through is a label inside the
        // reserved marker namespace, which must be enveloped away from it.
        let legal: &[(&str, LabelAlphabet)] = &[
            ("alpha", LabelAlphabet::BlankNodeLabel),
            ("beta.s2", LabelAlphabet::BlankNodeLabel),
            ("a.b", LabelAlphabet::BlankNodeLabel),
            ("a..b", LabelAlphabet::BlankNodeLabel),
            ("a...b", LabelAlphabet::BlankNodeLabel),
            ("0abc", LabelAlphabet::BlankNodeLabel),
            ("日本", LabelAlphabet::BlankNodeLabel),
            ("c14n0", LabelAlphabet::BlankNodeLabel),
            ("alpha", LabelAlphabet::NcName),
            ("trailing.", LabelAlphabet::NcName),
            ("<urn:x>", LabelAlphabet::XmlText),
            ("a b", LabelAlphabet::Unconstrained),
            ("", LabelAlphabet::Unconstrained),
        ];
        for &(label, alphabet) in legal {
            let encoded = encode_blank_label(label, BlankScope::DEFAULT, alphabet);
            assert!(
                matches!(encoded, Cow::Borrowed(_)),
                "{label:?} under {alphabet:?} must pass through borrowed"
            );
            assert_eq!(encoded, label);
        }
    }

    /// The adversary's probe, at the unit level: five distinct legal labels that
    /// an encoder mapping the alphabet onto a proper subset of itself would fold
    /// together. Each must reach the wire as ITSELF, and each must decode back.
    #[test]
    fn the_dotted_and_marker_family_stays_five_distinct_labels() {
        const PROBE: &[&str] = &["a.b", "a..b", "a...b", "purrdfesc_abc", "abc"];
        let mut tokens: BTreeMap<String, &str> = BTreeMap::new();
        for label in PROBE {
            let token =
                encode_blank_label(label, BlankScope::DEFAULT, LabelAlphabet::BlankNodeLabel)
                    .into_owned();
            if let Some(previous) = tokens.insert(token.clone(), label) {
                panic!("{previous:?} and {label:?} both encode to {token:?}");
            }
            let (decoded, scope) = decode_blank_label(&token, LabelAlphabet::BlankNodeLabel);
            assert_eq!(
                (decoded.as_ref(), scope),
                (*label, BlankScope::DEFAULT),
                "{label:?} did not survive its token {token:?}"
            );
        }
        // The four labels outside the reserved namespace reach the wire verbatim;
        // only the marker-prefixed one is enveloped.
        assert!(tokens.contains_key("a.b"));
        assert!(tokens.contains_key("a..b"));
        assert!(tokens.contains_key("a...b"));
        assert!(tokens.contains_key("abc"));
        assert!(tokens.contains_key("purrdfesc_purrdfesc_00005Fabc"));
    }

    #[test]
    fn encode_output_is_always_legal_under_the_target_alphabet() {
        for &alphabet in ALL_ALPHABETS {
            for label in HOSTILE_LABELS {
                for &scope in ALL_SCOPES {
                    let encoded = encode_blank_label(label, scope, alphabet);
                    assert!(
                        is_valid_label(&encoded, alphabet),
                        "encoding of {label:?} @ {scope:?} under {alphabet:?} is illegal: \
                         {encoded:?}"
                    );
                }
            }
        }
    }

    /// An ENVELOPE is legal in every alphabet at once, whichever one it was
    /// written for — the property that lets one spelling serve every syntax.
    #[test]
    fn every_envelope_is_legal_in_every_alphabet() {
        for label in HOSTILE_LABELS {
            for &scope in ALL_SCOPES {
                let Cow::Owned(envelope) =
                    encode_blank_label(label, scope, LabelAlphabet::BlankNodeLabel)
                else {
                    continue;
                };
                for &alphabet in ALL_ALPHABETS {
                    assert!(
                        is_valid_label(&envelope, alphabet),
                        "{envelope:?} (from {label:?} @ {scope:?}) is illegal under {alphabet:?}"
                    );
                }
            }
        }
    }

    /// Property sweep: EVERY single-scalar label over a broad code-point range,
    /// plus that scalar in an inner position, encodes to a legal token under
    /// every alphabet.
    #[test]
    fn encode_output_is_legal_for_every_scalar_position() {
        for cp in (0u32..=0x2FFF).chain([
            0xD7FF,
            0xE000,
            0xFFFD,
            0xFFFE,
            0xFFFF,
            0x0001_0000,
            0x0010_FFFF,
        ]) {
            let Some(c) = char::from_u32(cp) else {
                continue;
            };
            for label in [c.to_string(), format!("a{c}"), format!("{c}z")] {
                for &alphabet in ALL_ALPHABETS {
                    let encoded = escape_label(&label, alphabet);
                    assert!(
                        is_valid_label(&encoded, alphabet),
                        "encoding of {label:?} ({cp:#06x}) under {alphabet:?} is illegal: \
                         {encoded:?}"
                    );
                }
            }
        }
    }

    /// Injectivity over `(label, scope)` PAIRS, which is what blank-node
    /// identity actually rests on: no two pairs may share a token.
    #[test]
    fn encode_is_injective_over_label_scope_pairs() {
        for &alphabet in ALL_ALPHABETS {
            let mut seen: BTreeMap<String, (&str, BlankScope)> = BTreeMap::new();
            for label in HOSTILE_LABELS {
                for &scope in ALL_SCOPES {
                    let encoded = encode_blank_label(label, scope, alphabet).into_owned();
                    if let Some(previous) = seen.insert(encoded.clone(), (label, scope)) {
                        panic!(
                            "{alphabet:?} maps {previous:?} and {:?} both to {encoded:?}",
                            (label, scope)
                        );
                    }
                }
            }
        }
    }

    /// The marker-collision case stated explicitly: a LEGAL label that happens
    /// to equal the envelope of an illegal one must itself be enveloped, so the
    /// two never conflate.
    #[test]
    fn a_legal_label_equal_to_an_envelope_is_encoded_away_from_it() {
        let illegal = "a b";
        let image = escape_label(illegal, LabelAlphabet::BlankNodeLabel).into_owned();
        assert_eq!(image, "purrdfesc_a_000020b");
        assert!(
            is_valid_blank_node_label(&image),
            "the envelope is itself a legal label"
        );
        let twin = escape_label(&image, LabelAlphabet::BlankNodeLabel).into_owned();
        assert_ne!(
            twin, image,
            "a legal label matching an envelope must be encoded away from it"
        );
        assert!(twin.starts_with(ESCAPE_MARKER));
    }

    #[test]
    fn encode_is_deterministic_across_runs() {
        for &alphabet in ALL_ALPHABETS {
            for label in HOSTILE_LABELS {
                for &scope in ALL_SCOPES {
                    assert_eq!(
                        encode_blank_label(label, scope, alphabet),
                        encode_blank_label(label, scope, alphabet),
                        "{label:?} @ {scope:?} under {alphabet:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_envelope_is_the_documented_grammar() {
        // Default scope: no scope digits, the marker's `_` opens the body.
        assert_eq!(
            escape_label("bad\u{1f}label", LabelAlphabet::BlankNodeLabel),
            "purrdfesc_bad_00001Flabel"
        );
        assert_eq!(
            escape_label("0abc", LabelAlphabet::NcName),
            "purrdfesc_0abc"
        );
        assert_eq!(escape_label("", LabelAlphabet::NcName), "purrdfesc_");
        assert_eq!(
            escape_label("\u{10FFFF}", LabelAlphabet::BlankNodeLabel),
            "purrdfesc__10FFFF"
        );
        // Non-default scope: canonical decimal digits between the marker and the
        // body separator, and the SAME envelope in every alphabet.
        for &alphabet in ALL_ALPHABETS {
            assert_eq!(
                encode_blank_label("x", BlankScope(2), alphabet),
                "purrdfesc2_x"
            );
            assert_eq!(
                encode_blank_label("a.b", BlankScope(12), alphabet),
                "purrdfesc12_a_00002Eb"
            );
            assert_eq!(
                encode_blank_label("x", BlankScope(u32::MAX), alphabet),
                "purrdfesc4294967295_x"
            );
        }
    }

    // ── decode_blank_label ──────────────────────────────────────────────────

    /// The load-bearing equivalence, over the hostile table crossed with every
    /// scope and every alphabet:
    /// `decode(t, α) == (l, s)` **iff** `encode(l, s, α) == t`.
    ///
    /// The forward half (every encoded token decodes back to its pair) is what
    /// makes a round trip identity-preserving; the reverse half (a token the
    /// encoder would not have written is never decoded) is what stops two
    /// distinct document labels merging into one node.
    #[test]
    fn decode_inverts_encode_and_accepts_nothing_else() {
        for &alphabet in ALL_ALPHABETS {
            for label in HOSTILE_LABELS {
                for &scope in ALL_SCOPES {
                    let token = encode_blank_label(label, scope, alphabet);
                    let (decoded, decoded_scope) = decode_blank_label(&token, alphabet);
                    assert_eq!(
                        (decoded.as_ref(), decoded_scope),
                        (*label, scope),
                        "{label:?} @ {scope:?} under {alphabet:?} encoded as {token:?}"
                    );
                }
                // The reverse direction, over the tokens a conforming document
                // can actually carry (legal under the alphabet) and outside the
                // reserved namespace — i.e. every token in every foreign
                // document: whatever the token decodes to must re-encode to
                // exactly that token, which is what makes the decode injective
                // there. (A marker-prefixed token the encoder would not have
                // written is the documented reserved-namespace exception: it
                // denotes itself and is enveloped on the way out.)
                if !is_valid_label(label, alphabet) || label.starts_with(ESCAPE_MARKER) {
                    continue;
                }
                let (decoded, decoded_scope) = decode_blank_label(label, alphabet);
                assert_eq!(
                    (decoded.as_ref(), decoded_scope),
                    (*label, BlankScope::DEFAULT),
                    "{label:?} under {alphabet:?} must decode verbatim"
                );
                assert_eq!(
                    encode_blank_label(&decoded, decoded_scope, alphabet).as_ref(),
                    *label,
                    "decode of {label:?} under {alphabet:?} is not in the encoder's image"
                );
            }
        }
    }

    /// The same inverse over an exhaustive scalar sweep, in every character
    /// position the encoding distinguishes.
    #[test]
    fn decode_inverts_encode_for_every_scalar_position() {
        for cp in (0u32..=0x2FFF).chain([
            0xD7FF,
            0xE000,
            0xFFFD,
            0xFFFE,
            0xFFFF,
            0x0001_0000,
            0x0010_FFFF,
        ]) {
            let Some(c) = char::from_u32(cp) else {
                continue;
            };
            for label in [c.to_string(), format!("a{c}"), format!("{c}z")] {
                for &alphabet in ALL_ALPHABETS {
                    let token = escape_label(&label, alphabet);
                    let (decoded, scope) = decode_blank_label(&token, alphabet);
                    assert_eq!(
                        (decoded.as_ref(), scope),
                        (label.as_str(), BlankScope::DEFAULT),
                        "{label:?} ({cp:#06x}) under {alphabet:?} encoded as {token:?}"
                    );
                }
            }
        }
    }

    /// A token outside the reserved marker namespace is interned VERBATIM,
    /// without allocating and without any transformation at all — the rule that
    /// makes every foreign document a pure pass-through.
    #[test]
    fn a_token_outside_the_reserved_namespace_is_verbatim() {
        for token in [
            "", "a", "a.b", "a..b", "a...b", "a.s1", "x.s01", "c1.s5", "purrdfes", "日本",
        ] {
            for &alphabet in ALL_ALPHABETS {
                let (decoded, scope) = decode_blank_label(token, alphabet);
                assert_eq!(decoded.as_ref(), token, "{token:?} under {alphabet:?}");
                assert_eq!(scope, BlankScope::DEFAULT, "{token:?}");
                assert!(
                    matches!(decoded, Cow::Borrowed(_)),
                    "{token:?} must pass through without allocating"
                );
            }
        }
    }

    /// A marker-prefixed token that the encoder could not have written fails the
    /// image test and stands for ITSELF, so it can never merge with the label it
    /// superficially names.
    #[test]
    fn a_marker_token_outside_the_image_is_verbatim() {
        for token in [
            "purrdfesc",
            "purrdfesc1",
            "purrdfesc_abc",         // the label `abc` is written verbatim
            "purrdfesc01_a",         // zero-padded scope digits
            "purrdfesc0_a",          // scope 0 never spells its ordinal
            "purrdfesc4294967296_a", // out of `u32` range
            "purrdfesc__12",         // a hex group shorter than six digits
            "purrdfesc__00002e",     // lowercase hex is not what the encoder writes
            "purrdfesc_a.b",         // '.' is not a body pass-through character
            "purrdfesc__00D800",     // a surrogate code point is not a scalar
            "purrdfesc__110000",     // beyond the last Unicode scalar
            "purrdfesc_日本",        // a non-ASCII pass-through never survives encoding
        ] {
            let (decoded, scope) = decode_blank_label(token, LabelAlphabet::BlankNodeLabel);
            assert_eq!(decoded.as_ref(), token, "{token:?}");
            assert_eq!(scope, BlankScope::DEFAULT, "{token:?}");
        }
        // …and the label it stands for is enveloped on the way back out, so the
        // token space stabilizes after ONE write.
        let label = "purrdfesc_abc";
        let token = escape_label(label, LabelAlphabet::BlankNodeLabel).into_owned();
        assert_eq!(token, "purrdfesc_purrdfesc_00005Fabc");
        assert_eq!(
            decode_blank_label(&token, LabelAlphabet::BlankNodeLabel),
            (Cow::Owned(label.to_owned()), BlankScope::DEFAULT)
        );
    }

    /// The `abc` / `purrdfesc_abc` pair the image test exists for: two tokens in
    /// ONE document must stay two labels.
    #[test]
    fn a_marker_token_never_merges_with_the_label_it_names() {
        for &alphabet in ALL_ALPHABETS {
            let plain = decode_blank_label("abc", alphabet);
            let marked = decode_blank_label("purrdfesc_abc", alphabet);
            assert_ne!(plain, marked, "under {alphabet:?}");
        }
    }

    // ── retarget_owned_label ────────────────────────────────────────────────

    /// The owned-model detour is EXACT: re-targeting the owned rendering of a
    /// pair into an alphabet is the same as encoding that pair for the alphabet
    /// directly, so a term that travels through [`crate::RdfTerm`] reaches the
    /// wire spelled exactly as the IR would have spelled it.
    #[test]
    fn retarget_owned_label_equals_a_direct_encode() {
        for label in HOSTILE_LABELS {
            for &scope in ALL_SCOPES {
                let owned = scope.qualify_label(label);
                for &alphabet in ALL_ALPHABETS {
                    assert_eq!(
                        retarget_owned_label(&owned, alphabet),
                        encode_blank_label(label, scope, alphabet),
                        "{label:?} @ {scope:?} through the owned rendering {owned:?} \
                         under {alphabet:?}"
                    );
                }
            }
        }
    }

    /// A REWRITTEN label's body is pure `[A-Za-z0-9_]`, which is why one
    /// encoding satisfies every alphabet at once (a pass-through keeps the
    /// caller's bytes and is exempt by construction).
    #[test]
    fn rewritten_labels_are_ascii_word_characters_only() {
        for &alphabet in ALL_ALPHABETS {
            for label in HOSTILE_LABELS {
                for &scope in ALL_SCOPES {
                    let Cow::Owned(encoded) = encode_blank_label(label, scope, alphabet) else {
                        continue;
                    };
                    assert!(
                        encoded
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_'),
                        "{encoded:?}"
                    );
                }
            }
        }
    }
}
