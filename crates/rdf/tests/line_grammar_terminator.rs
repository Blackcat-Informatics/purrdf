// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The N-Triples / N-Quads statement terminator is not optional.
//!
//! > `ntriplesDoc ::= triple? (EOL triple?)* EOL?`
//! >
//! > `triple ::= subject predicate object '.'`
//!
//! — RDF 1.2 N-Triples §2.2. The `'.'` is a literal INSIDE `triple`; what
//! `ntriplesDoc` makes optional is the `triple` and the trailing `EOL`. So a line
//! carrying a subject, a predicate and an object carries a `'.'` as well, and
//! `<s> <p> <o>` with nothing after it is a document this grammar does not define.
//!
//! The parser used to accept it — `expect_dot` matched `Some(Token::Dot) | None`. That is
//! pure over-acceptance: the triple it produced was the triple the author meant, nothing
//! was dropped and nothing was misparsed, so it could only ever surface as a file that
//! PurRDF read and a conforming parser refused.
//!
//! Because a refusal is a claim too, every refusal below is executed next to the valid
//! neighbours the same tightening could plausibly have taken with it.

use purrdf_rdf::{NativeRdfFormat, dataset_from_bytes};

/// Parse `text` in the given line format, returning the quad count or the diagnostic.
fn parse(text: &str, format: NativeRdfFormat) -> Result<usize, String> {
    dataset_from_bytes(text.as_bytes(), format).map(|dataset| dataset.quads().count())
}

/// The refusal itself, in both line formats, with and without a trailing newline — the
/// newline is what made the missing dot look like an end-of-document technicality.
#[test]
fn a_statement_without_its_terminator_is_refused() {
    for (label, text) in [
        ("no newline", "<http://e/s> <http://e/p> <http://e/o>"),
        (
            "trailing newline",
            "<http://e/s> <http://e/p> <http://e/o>\n",
        ),
        (
            "a terminated statement first",
            "<http://e/s> <http://e/p> <http://e/o> .\n<http://e/s> <http://e/p> <http://e/o2>\n",
        ),
    ] {
        let error = parse(text, NativeRdfFormat::NTriples)
            .err()
            .unwrap_or_else(|| panic!("{label}: a statement with no '.' must be refused"));
        assert!(
            error.contains("terminator '.' is missing"),
            "{label}: the refusal must name the missing terminator, got: {error}"
        );
    }
    // A quad with its graph label and no dot is the same defect, not a different one.
    let error = parse(
        "<http://e/s> <http://e/p> <http://e/o> <http://e/g>\n",
        NativeRdfFormat::NQuads,
    )
    .expect_err("an unterminated quad must be refused");
    assert!(error.contains("terminator '.' is missing"), "got: {error}");
}

/// The diagnostic has to point at the line and the column where the terminator was owed,
/// or it is not actionable: the offending thing is an ABSENCE, so its position is the
/// only handle the author has.
#[test]
fn the_refusal_names_the_line_and_column_the_terminator_was_owed() {
    let error = parse(
        "<http://e/s> <http://e/p> <http://e/o> .\n  <http://e/s> <http://e/p> <http://e/o2>\n",
        NativeRdfFormat::NTriples,
    )
    .expect_err("refused");
    assert!(
        error.contains("2:"),
        "the refusal must name line 2, got: {error}"
    );
    // Column 42: the two leading spaces are counted, and the 39 scalars of
    // `<http://e/s> <http://e/p> <http://e/o2>` follow them, so the scalar the terminator
    // was owed at is 2 + 39 + 1.
    assert!(
        error.contains("2:42"),
        "the refusal must point just past the last token, got: {error}"
    );
}

/// THE NEIGHBOURS. Each of these is a lawful document that a tightening aimed at the
/// terminator could have taken with it, and each still parses to the quads it names.
#[test]
fn the_lawful_neighbours_of_the_missing_terminator_still_parse() {
    for (label, text, expected) in [
        (
            "a final statement WITH '.' and no trailing newline",
            "<http://e/s> <http://e/p> <http://e/o> .",
            1,
        ),
        (
            "a blank final line",
            "<http://e/s> <http://e/p> <http://e/o> .\n\n",
            1,
        ),
        (
            "several blank lines, and one in the middle",
            "<http://e/s> <http://e/p> <http://e/o> .\n\n\
             <http://e/s> <http://e/p> <http://e/o2> .\n\n\n",
            2,
        ),
        (
            "a comment-only final line",
            "<http://e/s> <http://e/p> <http://e/o> .\n# trailing note\n",
            1,
        ),
        (
            "a comment-only final line with no trailing newline",
            "<http://e/s> <http://e/p> <http://e/o> .\n# trailing note",
            1,
        ),
        (
            "a trailing comment on the statement's own line",
            "<http://e/s> <http://e/p> <http://e/o> . # note\n",
            1,
        ),
        (
            "a file ending in a bare #xD",
            "<http://e/s> <http://e/p> <http://e/o> .\r",
            1,
        ),
        (
            "CRLF line endings throughout",
            "<http://e/s> <http://e/p> <http://e/o> .\r\n\
             <http://e/s> <http://e/p> <http://e/o2> .\r\n",
            2,
        ),
        (
            "whitespace before the terminator",
            "<http://e/s> <http://e/p> <http://e/o>   .   \n",
            1,
        ),
        ("an empty document", "", 0),
        (
            "a document of nothing but comments and blank lines",
            "# one\n\n#two\n",
            0,
        ),
    ] {
        let count = parse(text, NativeRdfFormat::NTriples)
            .unwrap_or_else(|e| panic!("{label} must still parse, got: {e}"));
        assert_eq!(count, expected, "{label}");
    }
}

/// The N-Quads neighbours, which carry a fourth term before the `'.'` and so run the
/// terminator check from a different cursor position.
#[test]
fn the_lawful_nquads_neighbours_still_parse() {
    for (label, text) in [
        (
            "a graph label before the terminator",
            "<http://e/s> <http://e/p> <http://e/o> <http://e/g> .\n",
        ),
        (
            "a graph label, no trailing newline",
            "<http://e/s> <http://e/p> <http://e/o> <http://e/g> .",
        ),
        (
            "a graph label and a trailing comment",
            "<http://e/s> <http://e/p> <http://e/o> <http://e/g> . # note\n",
        ),
        (
            "a blank node graph label",
            "<http://e/s> <http://e/p> <http://e/o> _:g .\n",
        ),
        (
            "a literal object then a graph label",
            "<http://e/s> <http://e/p> \"lit\"@en <http://e/g> .\n",
        ),
    ] {
        let count = parse(text, NativeRdfFormat::NQuads)
            .unwrap_or_else(|e| panic!("{label} must still parse, got: {e}"));
        assert_eq!(count, 1, "{label}");
    }
}

/// A stray token AFTER the terminator keeps its own, older diagnostic: this change is
/// about the terminator's absence, and must not blur the two.
#[test]
fn a_token_after_the_terminator_still_reports_its_own_defect() {
    let error = parse(
        "<http://e/s> <http://e/p> <http://e/o> . <http://e/g> .\n",
        NativeRdfFormat::NTriples,
    )
    .expect_err("a second statement on one line is refused");
    assert!(
        error.contains("after the statement terminator"),
        "got: {error}"
    );
}
