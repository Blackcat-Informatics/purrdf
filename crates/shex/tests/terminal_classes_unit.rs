// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The ShEx and ShapeMap scanners decide token boundaries with their grammars'
//! terminals, not with Unicode properties.
//!
//! Both scanners used to answer `@pass` / `PASSED TOKENS` with
//! `char::is_whitespace` and `PN_CHARS` with `char::is_alphanumeric`. Neither
//! substitution is the harmless liberality it looks like, because a scanner's
//! classes decide where a token STOPS: too wide and a document that both
//! spellings accept is re-tokenized into a different parse, with no diagnostic.
//!
//! **The risk direction of the fix is the mirror bug — over-refusal.** Every
//! refusal pinned below is therefore paired with a neighbouring document that is
//! valid and must still be accepted, and the properties asserted are *which
//! tokens result* or *whether the public entry point accepts*, never a specific
//! error arm.

use purrdf_core::TermValue;
use purrdf_iri::terminals;
use purrdf_rdf::parse_dataset;
use purrdf_shex::lexer::{Token, tokenize};
use purrdf_shex::{
    NodeSelector, ShapeSelector, ValidationOptions, parse_shape_map, parse_shexc,
    validate_shape_map,
};

/// The four scalars `@pass` and `PASSED TOKENS` name: `#x20 #x9 #xD #xA`.
const WS: [char; 4] = [' ', '\t', '\r', '\n'];

/// Every Unicode scalar value, in order.
fn all_scalars() -> impl Iterator<Item = char> {
    (0..=0x0010_FFFF_u32).filter_map(char::from_u32)
}

/// Every scalar with the Unicode `White_Space` property that `WS` does NOT name.
///
/// Derived from the property rather than hand-listed on purpose: a hand-picked
/// set is exactly how U+205F and U+2029 escape a review.
fn whitespace_beyond_the_grammar() -> impl Iterator<Item = char> {
    all_scalars().filter(|c| c.is_whitespace() && !WS.contains(c))
}

/// A ShExC schema with `sep` sitting where a separator would go.
fn shexc_with_separator(sep: char) -> String {
    format!("<http://example.org/S>{sep}{{ <http://example.org/p> LITERAL }}")
}

/// A shape map with `sep` sitting where a separator would go.
fn shape_map_with_separator(sep: char) -> String {
    format!("<http://a.example/s>{sep}@START")
}

/// The single shape label a one-shape ShExC schema declares.
fn sole_label(src: &str) -> String {
    let schema = parse_shexc(src, None).unwrap_or_else(|e| panic!("parse {src:?}: {e}"));
    assert_eq!(schema.shapes.len(), 1, "one shape declared by {src:?}");
    schema.shapes[0].id.clone()
}

// ── ShExC: `@pass ::= [ \t\r\n]+ | "#" [^\r\n]*` ──────────────────────────────

/// The four scalars the production names still separate two terminals.
///
/// The valid neighbour for every refusal below: narrowing the class must not
/// cost the grammar its own whitespace.
#[test]
fn shexc_still_separates_terminals_with_the_four_ws_scalars() {
    for ws in WS {
        let src = shexc_with_separator(ws);
        assert_eq!(
            sole_label(&src),
            "http://example.org/S",
            "{ws:?} is `WS` and must separate"
        );
    }
}

/// No other Unicode whitespace scalar separates ShExC terminals.
///
/// Asserted over the whole `White_Space` property rather than over U+00A0 and
/// U+3000 alone, so a scalar nobody thought to list cannot slip through.
#[test]
fn shexc_refuses_every_unicode_whitespace_the_production_does_not_name() {
    for sep in whitespace_beyond_the_grammar() {
        let src = shexc_with_separator(sep);
        assert!(
            parse_shexc(&src, None).is_err(),
            "{sep:?} (U+{:04X}) is not `WS` and must not act as a separator",
            u32::from(sep)
        );
    }
}

/// U+000C FORM FEED is refused — the `is_ascii_whitespace` trap, pinned.
///
/// `u8::is_ascii_whitespace` implements the WhatWG Infra definition and admits
/// U+000C, which `@pass` does not name; swapping one wrong predicate for another
/// would have looked like a fix and passed every other test here.
#[test]
fn shexc_refuses_form_feed_which_is_ascii_whitespace_but_not_ws() {
    assert!(b'\x0c'.is_ascii_whitespace());
    assert!(parse_shexc(&shexc_with_separator('\u{C}'), None).is_err());
    // The neighbour that shares the trap's definition: TAB is in both sets and
    // must still work, so this is exactness rather than blanket strictness.
    assert!(b'\t'.is_ascii_whitespace());
    assert_eq!(
        sole_label(&shexc_with_separator('\t')),
        "http://example.org/S"
    );
}

// ── ShExC: `[167s] PN_CHARS` in a name ────────────────────────────────────────

/// U+00B7 MIDDLE DOT is `PN_CHARS` but not `PN_CHARS_BASE`, and stays in a label.
#[test]
fn shexc_accepts_middle_dot_inside_a_shape_label() {
    assert!(terminals::is_pn_chars('\u{B7}'));
    assert!(!terminals::is_pn_chars_base('\u{B7}'));
    assert_eq!(
        sole_label("PREFIX ex: <http://example.org/>\nex:a\u{B7}b { ex:p LITERAL }"),
        "http://example.org/a\u{B7}b"
    );
}

/// An NFD-decomposed `é` survives in a shape label, exactly as the NFC one does.
///
/// `[#x300-#x36F]` is 113 code points of `PN_CHARS`, and NFD is what macOS
/// filesystems and many exports emit. A class that refused combining marks would
/// parse `ex:café` in NFC and fail it in NFD — the same author, the same schema,
/// a different byte encoding of one character.
#[test]
fn shexc_accepts_a_combining_mark_in_a_shape_label_in_both_normal_forms() {
    assert!(terminals::is_pn_chars('\u{301}'));
    assert_eq!(
        sole_label("PREFIX ex: <http://example.org/>\nex:cafe\u{301} { ex:p LITERAL }"),
        "http://example.org/cafe\u{301}",
        "NFD: `e` + U+0301 COMBINING ACUTE"
    );
    assert_eq!(
        sole_label("PREFIX ex: <http://example.org/>\nex:caf\u{E9} { ex:p LITERAL }"),
        "http://example.org/caf\u{E9}",
        "NFC: U+00E9"
    );
}

/// U+1680 OGHAM SPACE MARK is a NAME character in ShExC, exactly as in SPARQL.
///
/// It is the one scalar that is simultaneously Unicode `White_Space` AND inside
/// `PN_CHARS_BASE` (via `[#x37F-#x1FFF]`), so the grammar makes it lawfully part
/// of a name and a label containing it is ONE token. Skipping it as whitespace
/// would have split that label in two with no error at all — the silent
/// re-tokenization, not a refusal. Do not "fix" this: the next reader who deletes
/// it manufactures the over-refusal this file exists to prevent.
#[test]
fn shexc_keeps_ogham_space_mark_inside_one_name_token() {
    assert!('\u{1680}'.is_whitespace());
    assert!(terminals::is_pn_chars_base('\u{1680}'));

    let tokens: Vec<Token> = tokenize("ex:a\u{1680}b")
        .expect("a name containing U+1680 lexes")
        .into_iter()
        .map(|s| s.token)
        .collect();
    assert_eq!(
        tokens,
        vec![Token::PName("ex".into(), "a\u{1680}b".into())],
        "U+1680 is a name character, so this is ONE token"
    );

    assert_eq!(
        sole_label("PREFIX ex: <http://example.org/>\nex:a\u{1680}b { ex:p LITERAL }"),
        "http://example.org/a\u{1680}b"
    );

    // The case where skipping it was visibly destructive: at a token boundary,
    // U+1680 STARTS a name (it is `PN_CHARS_BASE`), it does not vanish. Treating
    // it as whitespace produced `Word("abc")` here — a token one scalar shorter
    // than the one the author wrote, with no error to say so.
    let tokens: Vec<Token> = tokenize("\u{1680}abc")
        .expect("a name beginning with U+1680 lexes")
        .into_iter()
        .map(|s| s.token)
        .collect();
    assert_eq!(tokens, vec![Token::Word("\u{1680}abc".into())]);
}

// ── ShapeMap: `PASSED TOKENS ::= [ \t\r\n]+ | "#" [^\r\n]*` ───────────────────

/// The four scalars ShapeMap's own production names still separate its terminals.
#[test]
fn shape_map_still_separates_terminals_with_the_four_ws_scalars() {
    for ws in WS {
        let src = shape_map_with_separator(ws);
        let map = parse_shape_map(&src, None).unwrap_or_else(|e| panic!("parse {src:?}: {e}"));
        assert_eq!(map.0.len(), 1, "{ws:?} is `WS` and must separate");
        assert_eq!(map.0[0].shape, ShapeSelector::Start);
    }
}

/// No other Unicode whitespace scalar separates ShapeMap terminals.
#[test]
fn shape_map_refuses_every_unicode_whitespace_the_production_does_not_name() {
    for sep in whitespace_beyond_the_grammar() {
        let src = shape_map_with_separator(sep);
        assert!(
            parse_shape_map(&src, None).is_err(),
            "{sep:?} (U+{:04X}) is not `WS` and must not act as a separator",
            u32::from(sep)
        );
    }
}

/// U+000C FORM FEED is refused in a shape map too, and TAB still is not.
#[test]
fn shape_map_refuses_form_feed_but_not_tab() {
    assert!(parse_shape_map(&shape_map_with_separator('\u{C}'), None).is_err());
    assert!(parse_shape_map(&shape_map_with_separator('\t'), None).is_ok());
}

// ── ShapeMap: the keyword follow boundary ─────────────────────────────────────
//
// `'a'`, `"FOCUS"` and `"START"` are literal strings in productions [4], [6] and
// [7]. Their right edge asks "could a longer terminal have started here?", and
// the only name-shaped terminals in the family are [168s] PN_PREFIX / [169s]
// PN_LOCAL, both runs of [167s] PN_CHARS. Every OTHER follow character in the
// grammar is punctuation or end of input, and refusing those is the mirror bug.

/// A keyword abutting the punctuation the grammar puts after it still matches.
///
/// These are the documents a "keyword must be followed by whitespace" rule would
/// quietly reject: `,` ends a `shapeAssociation` ([1]), `}` closes a
/// `triplePattern` ([6]), `<` opens the `IRIREF` that `iri` is ([136s]), and
/// end-of-input ends the map.
#[test]
fn shape_map_keywords_abut_their_grammatical_follow_characters() {
    for (src, expected) in [
        // `START` at end of input.
        ("<http://a.example/s>@START", 1),
        // `START` immediately before `,`.
        ("<http://a.example/s>@START,<http://a.example/t>@START", 2),
        // `FOCUS` immediately before `<`.
        ("{FOCUS<http://a.example/p> _}@START", 1),
        // `FOCUS` immediately before `}`.
        ("{_ <http://a.example/p> FOCUS}@START", 1),
        // `a` immediately before `<`.
        ("{FOCUS a<http://a.example/C>}@START", 1),
    ] {
        let map = parse_shape_map(src, None).unwrap_or_else(|e| panic!("parse {src:?}: {e}"));
        assert_eq!(map.0.len(), expected, "{src:?}");
    }
}

/// A keyword may not be carved out of a longer run of `PN_CHARS`.
///
/// `-` and a combining mark are the interesting members: both are `PN_CHARS`,
/// neither is `char::is_alphanumeric`, so the property the code used to ask
/// answered "boundary" for spellings the grammar reads as one name.
#[test]
fn shape_map_refuses_a_keyword_spliced_out_of_a_name() {
    for src in [
        "<http://a.example/s>@STARTS",
        "<http://a.example/s>@START-x",
        "<http://a.example/s>@START\u{301}",
        "{FOCUSED <http://a.example/p> _}@START",
    ] {
        assert!(parse_shape_map(src, None).is_err(), "{src:?}");
    }
}

// ── ShapeMap: `[142s] BLANK_NODE_LABEL` ───────────────────────────────────────

/// A combining mark belongs to a blank-node label; U+2460 does not.
#[test]
fn shape_map_bounds_a_blank_node_label_with_pn_chars() {
    let map = parse_shape_map("_:cafe\u{301}@START", None).expect("NFD blank label parses");
    assert_eq!(
        map.0[0].node,
        NodeSelector::Node(TermValue::blank("cafe\u{301}")),
        "U+0301 is `PN_CHARS`, so it is part of the label"
    );

    // U+2460 CIRCLED DIGIT ONE is `char::is_alphanumeric` and is NOT `PN_CHARS`,
    // so it used to be absorbed into a label the grammar says stops before it.
    assert!('\u{2460}'.is_alphanumeric());
    assert!(!terminals::is_pn_chars('\u{2460}'));
    assert!(parse_shape_map("_:a\u{2460}@START", None).is_err());
    // The neighbour: an ordinary ASCII-digit label is untouched.
    let map = parse_shape_map("_:a1@START", None).expect("`_:a1` parses");
    assert_eq!(map.0[0].node, NodeSelector::Node(TermValue::blank("a1")));
}

// ── ShapeMap: `[145s] LANGTAG` ────────────────────────────────────────────────

/// `LANGTAG` enumerates ASCII letters and digits, and is position-dependent.
///
/// This is the one terminal here that is NOT a `PN_CHARS` run: the production is
/// `([a-zA-Z])+ ("-" ([a-zA-Z0-9])+)*`, so applying the name class uniformly
/// would have been its own defect.
#[test]
fn shape_map_language_tags_follow_the_langtag_production() {
    for tag in [
        "en",
        "en-UK",
        "zh-Hans",
        "de-CH-1901",
        "x-private",
        "i-klingon",
    ] {
        let src = format!("\"x\"@{tag}@START");
        let map = parse_shape_map(&src, None).unwrap_or_else(|e| panic!("parse {src:?}: {e}"));
        assert_eq!(
            map.0[0].node,
            NodeSelector::Node(TermValue::lang_literal("x", tag)),
            "{tag} is a well-formed LANGTAG"
        );
    }
    for tag in ["\u{65E5}\u{672C}\u{8A9E}", "en-", "1ab", "-en"] {
        let src = format!("\"x\"@{tag}@START");
        assert!(parse_shape_map(&src, None).is_err(), "{src:?}");
    }
}

// ── ShapeMap: the prefixed-name diagnostic ────────────────────────────────────

/// The refusal quotes the name the author actually wrote, combining marks included.
///
/// This run is detection-only — nothing is consumed and every caller has already
/// established that no conforming term starts here — so the class decides what the
/// message SAYS, not what parses. With a letter property it said `ex:cafe`, naming
/// a different IRI than the one being refused.
#[test]
fn shape_map_quotes_a_decomposed_prefixed_name_verbatim() {
    let err = parse_shape_map("ex:cafe\u{301}@<http://a.example/S>", None)
        .expect_err("a prefixed name is refused");
    let message = err.to_string();
    assert!(
        message.contains("ex:cafe\u{301}"),
        "the diagnostic names the input as written: {message}"
    );
}

// ── End to end: the defect's whole character is that it reaches the answer ────

/// A NO-BREAK SPACE separator does not survive to a validation result.
///
/// Driven through [`validate_shape_map`], the public one-call entry point, because
/// a boundary defect that only a lexer unit test can see is not the defect: the
/// one that shipped produced a clean, plausible ANSWER.
#[test]
fn a_no_break_space_separator_never_reaches_a_validation_result() {
    let schema = parse_shexc(
        "<http://example.org/UserShape> { <http://example.org/name> LITERAL }",
        None,
    )
    .expect("a well-formed schema parses");
    let data = parse_dataset(
        b"<http://example.org/alice> <http://example.org/name> \"Alice\" .",
        "text/turtle",
        None,
    )
    .expect("a well-formed graph parses");

    let conforming = "<http://example.org/alice> @<http://example.org/UserShape>";
    let result = validate_shape_map(
        &schema,
        &data,
        conforming,
        None,
        &ValidationOptions::default(),
    )
    .expect("the neighbouring valid map validates");
    assert!(result.all_conformant());

    let with_nbsp = conforming.replace(' ', "\u{A0}");
    assert!(
        validate_shape_map(
            &schema,
            &data,
            &with_nbsp,
            None,
            &ValidationOptions::default(),
        )
        .is_err(),
        "U+00A0 is not `WS`, so this map has no reading and must not produce a result"
    );
}
