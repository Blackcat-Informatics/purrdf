// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The line grammar's whitespace terminal, through the PUBLIC parse entry points.
//!
//! The unit vectors beside the parser drive its three line paths directly. These drive
//! the doors a user actually knocks on — [`parse_dataset`] over a whole document, and
//! [`parse_dataset_from_reader`] over the streaming lane — because a silently wrong IRI
//! written into a data file outlives a wrong query answer, and the entry point is where
//! a caller would discover either.
//!
//! What is pinned:
//!
//! * The separator the grammar names is `WS ::= #x20 | #x9 | #xD | #xA` (Turtle 1.2
//!   §6.5, SPARQL 1.2 §19.8) — four code points, not the twenty-six of
//!   `char::is_whitespace`. A scanner's character class decides token BOUNDARIES, so a
//!   wider class does not merely accept more documents, it re-reads documents both
//!   classes accept.
//! * Everything the grammar DOES admit around whitespace still parses. Over-refusal is
//!   the mirror failure of the silent re-read and it hides better, because a refusal
//!   looks like strictness, so every tightening here ships with its executed neighbour.
//! * A U+00A0 or a U+FEFF inside a quoted literal or an `IRIREF` body is CONTENT, is
//!   lawful (`ucschar` covers both), and must reach the frozen dataset verbatim. This is
//!   a rule about the line grammar, not about what a token may hold.

use purrdf_rdf::native_codecs::parse_dataset;
use purrdf_rdf::{TermValue, parse_dataset_from_reader};

const TURTLE: &str = "text/turtle";
const NTRIPLES: &str = "application/n-triples";

/// Every subject IRI of `text`, in quad order.
fn subject_iris(text: &str, media_type: &str) -> Vec<String> {
    let dataset = parse_dataset(text.as_bytes(), media_type, None)
        .unwrap_or_else(|e| panic!("parse {media_type} failed: {e}"));
    dataset
        .quads()
        .map(|quad| match dataset.term_value(quad.s) {
            TermValue::Iri(iri) => iri,
            other => panic!("subject is not an IRI: {other:?}"),
        })
        .collect()
}

/// Every object term of `text`, in quad order.
fn object_values(text: &str, media_type: &str) -> Vec<TermValue> {
    let dataset = parse_dataset(text.as_bytes(), media_type, None)
        .unwrap_or_else(|e| panic!("parse {media_type} failed: {e}"));
    dataset
        .quads()
        .map(|quad| dataset.term_value(quad.o))
        .collect()
}

/// A whole Turtle document through the PUBLIC [`parse_dataset`] door, carrying every
/// lawful whitespace shape at once: `WS` indentation in both members, comments at the
/// line start and indented, blank lines, CRLF line endings, no trailing newline, and a
/// U+00A0 inside both a quoted literal and an `IRIREF` body.
///
/// This is the vector that matters: it is the one a user's file looks like, and it is
/// the one that would have written a silently different IRI into a data file.
#[test]
fn an_end_to_end_turtle_document_with_every_lawful_whitespace_shape_parses() {
    // Deliberately assembled with real CRLF and no trailing newline.
    let document = concat!(
        "# a leading comment\r\n",
        "@prefix ex: <https://example.org/> .\r\n",
        "\r\n",
        "\tex:indented-with-a-tab ex:p \"tab\" .\r\n",
        "    ex:indented-with-spaces ex:p \"spaces\" .\n",
        "\t  # an indented comment\n",
        "\n",
        // U+00A0 inside a quoted literal and inside an IRIREF body: content, lawful,
        // and untouched by a rule about the LINE grammar.
        "<urn:ex:a\u{a0}b> ex:p \"no\u{a0}break\" .\n",
        "ex:no-trailing-newline ex:p \"last\" ."
    );

    let subjects = subject_iris(document, TURTLE);
    assert_eq!(
        subjects,
        vec![
            "https://example.org/indented-with-a-tab".to_owned(),
            "https://example.org/indented-with-spaces".to_owned(),
            "urn:ex:a\u{a0}b".to_owned(),
            "https://example.org/no-trailing-newline".to_owned(),
        ],
        "the NO-BREAK SPACE in the IRIREF must reach the frozen dataset verbatim"
    );

    let objects = object_values(document, TURTLE);
    assert!(
        objects.contains(&TermValue::Literal {
            lexical_form: "no\u{a0}break".to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
            language: None,
            direction: None,
        }),
        "the NO-BREAK SPACE in the literal must survive: {objects:?}"
    );
}

/// The same shapes through the N-Triples line family, where the trim lives.
#[test]
fn an_end_to_end_ntriples_document_with_every_lawful_whitespace_shape_parses() {
    let document = concat!(
        "# a leading comment\r\n",
        "\r\n",
        "\t<urn:ex:tab> <urn:ex:p> \"tab\" .\r\n",
        "    <urn:ex:spaces> <urn:ex:p> \"spaces\" . \t\n",
        "\t  # an indented comment\n",
        "<urn:ex:a\u{a0}b> <urn:ex:p> \"no\u{a0}break\" .\n",
        "<urn:ex:last> <urn:ex:p> \"last\" ."
    );
    assert_eq!(
        subject_iris(document, NTRIPLES),
        vec![
            "urn:ex:tab".to_owned(),
            "urn:ex:spaces".to_owned(),
            "urn:ex:a\u{a0}b".to_owned(),
            "urn:ex:last".to_owned(),
        ]
    );
    // And the streaming door reads the identical document identically.
    let streamed = parse_dataset_from_reader(document.as_bytes(), NTRIPLES, None)
        .expect("the streaming lane must accept the same document");
    assert_eq!(streamed.quads().count(), 4);
}

/// A line of nothing but U+00A0 is not blank, through the public door, on both the
/// buffered and the streaming lane — and the neighbouring SPACE-only line still is.
#[test]
fn a_no_break_space_only_line_is_refused_through_the_public_entry_points() {
    let offending = "<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n\u{a0}\n";
    let buffered = parse_dataset(offending.as_bytes(), NTRIPLES, None)
        .expect_err("a U+00A0-only line is neither blank nor a comment");
    assert_eq!(buffered.location.as_ref().and_then(|l| l.line), Some(2));
    assert!(
        buffered.message.contains("U+00A0 NO-BREAK SPACE"),
        "the refusal must name the scalar: {}",
        buffered.message
    );
    let streamed = parse_dataset_from_reader(offending.as_bytes(), NTRIPLES, None)
        .expect_err("the streaming lane must agree");
    assert_eq!(streamed, buffered, "both doors must give one answer");

    // The neighbour: a real blank line, and a line of real `WS`, are still blank.
    assert_eq!(
        subject_iris(
            "<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n\n \t \n<urn:ex:s2> <urn:ex:p> <urn:ex:o> .\n",
            NTRIPLES,
        ),
        vec!["urn:ex:s".to_owned(), "urn:ex:s2".to_owned()]
    );
}

/// A U+00A0 before `#` does not open a comment, so the line is no longer silently
/// discarded — and a `#` after real `WS` still opens one.
#[test]
fn a_no_break_space_before_a_hash_is_refused_through_the_public_entry_points() {
    let offending = "\u{a0}# this does not open a comment\n";
    let buffered = parse_dataset(offending.as_bytes(), NTRIPLES, None)
        .expect_err("a U+00A0 before `#` does not open a comment");
    assert_eq!(buffered.location.as_ref().and_then(|l| l.line), Some(1));
    let streamed = parse_dataset_from_reader(offending.as_bytes(), NTRIPLES, None)
        .expect_err("the streaming lane must agree");
    assert_eq!(streamed, buffered);

    // The neighbour: the same comment opened after real `WS`, and at the line start.
    for comment in ["# a comment\n", " \t# a comment\n"] {
        let dataset = parse_dataset(comment.as_bytes(), NTRIPLES, None)
            .expect("a comment line carries no statement and no error");
        assert_eq!(dataset.quads().count(), 0);
    }
}

/// A leading U+FEFF is refused BY NAME at the public door; a second consecutive one and
/// a mid-line one are refused too; and a U+FEFF inside a token is content and parses.
#[test]
fn a_byte_order_mark_is_refused_by_name_through_the_public_entry_points() {
    let marked = "\u{feff}<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n";
    let buffered = parse_dataset(marked.as_bytes(), NTRIPLES, None)
        .expect_err("the line grammar names no byte order mark");
    assert!(
        buffered
            .message
            .contains("U+FEFF ZERO WIDTH NO-BREAK SPACE"),
        "the refusal must name the character: {}",
        buffered.message
    );
    let streamed = parse_dataset_from_reader(marked.as_bytes(), NTRIPLES, None)
        .expect_err("the streaming lane must agree");
    assert_eq!(streamed, buffered);

    // A SECOND consecutive mark is a leading mark in its own right.
    let doubled = "\u{feff}\u{feff}<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n";
    let doubled_err = parse_dataset(doubled.as_bytes(), NTRIPLES, None)
        .expect_err("a second mark is refused too");
    assert!(
        doubled_err
            .message
            .contains("U+FEFF ZERO WIDTH NO-BREAK SPACE")
    );

    // A MID-LINE mark outside a token is refused as the stray token it lexes into,
    // and is NOT called a byte order mark.
    let mid_line = "<urn:ex:s> \u{feff}<urn:ex:p> <urn:ex:o> .\n";
    let mid_line_err = parse_dataset(mid_line.as_bytes(), NTRIPLES, None)
        .expect_err("a stray U+FEFF opens no terminal");
    assert!(!mid_line_err.message.contains("byte order mark"));

    // The neighbours: the identical document without the mark, and a U+FEFF INSIDE a
    // token — `ucschar` in an IRIREF, content in a literal — both still parse.
    assert_eq!(
        subject_iris("<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n", NTRIPLES),
        vec!["urn:ex:s".to_owned()]
    );
    assert_eq!(
        subject_iris(
            "<urn:ex:a\u{feff}b> <urn:ex:p> \"x\u{feff}y\" .\n",
            NTRIPLES
        ),
        vec!["urn:ex:a\u{feff}b".to_owned()],
        "U+FEFF inside an IRIREF body is lawful and must survive verbatim"
    );
}

/// U+FEFF is a LAWFUL name character — `PN_CHARS_BASE` includes `[#xFDF0-#xFFFD]` and
/// `#xFEFF` is in it — so every name position that admits it must keep admitting it,
/// through the public door, in both the line family and Turtle.
///
/// This is the neighbour that keeps the leading-mark refusal from becoming a blanket ban
/// on the character, which would be over-refusal. U+00A0, which belongs to no name class
/// at all, is the contrast that shows the two characters are not interchangeable.
#[test]
fn a_byte_order_mark_in_a_name_position_still_parses_through_the_public_entry_point() {
    // N-Triples: a blank node label.
    let dataset = parse_dataset(
        "_:a\u{feff}b <urn:ex:p> <urn:ex:o> .\n".as_bytes(),
        NTRIPLES,
        None,
    )
    .expect("U+FEFF is a lawful BLANK_NODE_LABEL scalar");
    assert_eq!(dataset.quads().count(), 1);

    // Turtle: a prefixed name's local part.
    assert_eq!(
        subject_iris(
            "@prefix ex: <https://example.org/> .\nex:a\u{feff}b ex:p ex:o .\n",
            TURTLE,
        ),
        vec!["https://example.org/a\u{feff}b".to_owned()],
        "U+FEFF is a lawful PN_LOCAL scalar and must resolve verbatim"
    );

    // The contrast: U+00A0 is in no name class, so the same two shapes are refused.
    for (text, media_type) in [
        ("_:a\u{a0}b <urn:ex:p> <urn:ex:o> .\n", NTRIPLES),
        (
            "@prefix ex: <https://example.org/> .\nex:a\u{a0}b ex:p ex:o .\n",
            TURTLE,
        ),
    ] {
        let error = parse_dataset(text.as_bytes(), media_type, None)
            .expect_err("U+00A0 is not a name character");
        assert!(
            error.message.contains("U+00A0 NO-BREAK SPACE"),
            "the refusal must name the scalar: {}",
            error.message
        );
    }
}
