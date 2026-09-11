// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The line grammar's `EOL` terminal, through the PUBLIC parse entry points.
//!
//! The unit vectors beside the parser drive its three line paths directly. These drive
//! the doors a user actually knocks on — [`parse_dataset`] over a whole document and
//! [`parse_dataset_from_reader`] over the streaming lane — because the failure this file
//! pins is a SILENT DROP: statements that never reach the dataset, with exit zero and no
//! diagnostic. A dropped statement is invisible from inside the parser and visible only
//! here, where the quads are counted.
//!
//! What is pinned:
//!
//! * The terminator the grammar names is `EOL ::= [#xD#xA]+` (RDF 1.2 N-Triples §2.2,
//!   N-Quads §2.3), so a LONE `#xD` ends a line. `str::lines` — which every line path
//!   used — splits at `#xA` only and absorbs a `#xD` that immediately precedes one, so a
//!   `#xD`-separated pair of statements arrived as ONE line and the second was discarded.
//! * `#xD#xA` is ONE terminator. Counting it as two inserts a phantom blank line between
//!   every pair of statements in a CRLF file and shifts every line number a diagnostic
//!   reports.
//! * A line holds AT MOST ONE statement: tokens after the `'.'` terminator are refused,
//!   not thrown away.
//! * Everything the grammar DOES admit still parses — LF-only files, files with no final
//!   terminator, blank and comment lines, and the escaped spellings of a carriage return
//!   INSIDE a token. Over-refusal is the mirror failure of the silent drop, so every
//!   tightening here ships with its executed neighbour.

use std::fmt::Write as _;
use std::io::{Cursor, Read};

use purrdf_rdf::native_codecs::parse_dataset;
use purrdf_rdf::{RdfDataset, TermValue, parse_dataset_from_reader};

const NTRIPLES: &str = "application/n-triples";
const NQUADS: &str = "application/n-quads";

/// A reader that hands back at most `chunk` bytes per `read`, so a read boundary can be
/// placed between any two bytes — including between a `#xD` and its `#xA`.
struct DribbleReader<'a> {
    data: &'a [u8],
    chunk: usize,
}

impl Read for DribbleReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let take = self.chunk.min(buf.len()).min(self.data.len());
        buf[..take].copy_from_slice(&self.data[..take]);
        self.data = &self.data[take..];
        Ok(take)
    }
}

/// Every subject IRI of `text`, in quad order, as the buffered door reads it.
fn subject_iris(text: &str, media_type: &str) -> Vec<String> {
    let dataset = parse_dataset(text.as_bytes(), media_type, None)
        .unwrap_or_else(|e| panic!("parse {media_type} failed: {e}"));
    subjects_of(&dataset)
}

/// Every subject IRI of `dataset`, in quad order.
fn subjects_of(dataset: &RdfDataset) -> Vec<String> {
    dataset
        .quads()
        .map(|quad| match dataset.term_value(quad.s) {
            TermValue::Iri(iri) => iri,
            other => panic!("subject is not an IRI: {other:?}"),
        })
        .collect()
}

/// `text` read through the streaming door at several read granularities — including one
/// byte, which puts a read boundary between every `#xD` and its `#xA` — asserting all of
/// them agree with the buffered door.
fn streamed_subjects_agree(text: &str, media_type: &str) -> Vec<String> {
    let buffered = parse_dataset(text.as_bytes(), media_type, None)
        .unwrap_or_else(|e| panic!("buffered parse failed: {e}"));
    let expected = subjects_of(&buffered);
    for chunk in [1usize, 2, 3, 7, 64, 4096] {
        let streamed = parse_dataset_from_reader(
            DribbleReader {
                data: text.as_bytes(),
                chunk,
            },
            media_type,
            None,
        )
        .unwrap_or_else(|e| panic!("streaming parse at read granularity {chunk} failed: {e}"));
        assert_eq!(
            subjects_of(&streamed),
            expected,
            "the streaming door must read {text:?} identically at read granularity {chunk}"
        );
    }
    expected
}

/// The defect, at the public door: two statements separated by a lone `#xD` are two
/// statements, and BOTH reach the dataset.
///
/// Before the fix the document was one line, the tokens after the first `'.'` were
/// discarded, and `parse_dataset` returned ONE quad and success.
#[test]
fn a_lone_carriage_return_separates_two_statements_at_the_public_door() {
    let document = concat!(
        "<urn:ex:s1> <urn:ex:p> <urn:ex:o> .\r",
        "<urn:ex:s2> <urn:ex:p> <urn:ex:o> .\r",
    );
    assert_eq!(
        subject_iris(document, NTRIPLES),
        vec!["urn:ex:s1".to_owned(), "urn:ex:s2".to_owned()],
        "a lone #xD is an EOL, so neither statement may be dropped"
    );
    assert_eq!(streamed_subjects_agree(document, NTRIPLES).len(), 2);
}

/// The same, with every terminator shape mixed into one document, and with the last
/// statement carrying no terminator at all.
#[test]
fn mixed_terminators_read_identically_on_both_doors() {
    let document = concat!(
        "# a comment ended by a lone CR\r",
        "<urn:ex:s1> <urn:ex:p> <urn:ex:o> .\r",
        "<urn:ex:s2> <urn:ex:p> <urn:ex:o> .\n",
        "\r",
        "<urn:ex:s3> <urn:ex:p> <urn:ex:o> .\r\n",
        "\r\n",
        "<urn:ex:s4> <urn:ex:p> <urn:ex:o> .",
    );
    let expected = vec![
        "urn:ex:s1".to_owned(),
        "urn:ex:s2".to_owned(),
        "urn:ex:s3".to_owned(),
        "urn:ex:s4".to_owned(),
    ];
    assert_eq!(subject_iris(document, NTRIPLES), expected);
    assert_eq!(streamed_subjects_agree(document, NTRIPLES), expected);
}

/// A CRLF document holds exactly its statements — no phantom blank lines — and its
/// diagnostics carry the line numbers an editor shows.
#[test]
fn a_crlf_document_holds_no_phantom_lines() {
    let document = concat!(
        "<urn:ex:s1> <urn:ex:p> <urn:ex:o> .\r\n",
        "<urn:ex:s2> <urn:ex:p> <urn:ex:o> .\r\n",
        "<urn:ex:s3> <urn:ex:p> <urn:ex:o> .\r\n",
    );
    assert_eq!(streamed_subjects_agree(document, NTRIPLES).len(), 3);

    // A malformed FOURTH line is reported at line 4, on both doors, and identically to
    // the same document written with LF endings.
    let broken = format!("{document}<urn:ex:s4> <urn:ex:p> .\r\n");
    let buffered = parse_dataset(broken.as_bytes(), NTRIPLES, None)
        .expect_err("a two-term statement is not a statement");
    assert_eq!(buffered.location.as_ref().and_then(|l| l.line), Some(4));
    let streamed = parse_dataset_from_reader(Cursor::new(broken.clone()), NTRIPLES, None)
        .expect_err("the streaming door must agree");
    assert_eq!(streamed, buffered, "both doors must give one answer");
    let lf = parse_dataset(broken.replace("\r\n", "\n").as_bytes(), NTRIPLES, None)
        .expect_err("the LF spelling of the same document");
    assert_eq!(
        lf, buffered,
        "CRLF and LF must be the SAME document, diagnostics included"
    );
}

/// Tokens after the statement terminator are refused, naming what was found — they used
/// to be discarded, which is a statement a user wrote vanishing at the public door.
#[test]
fn a_second_statement_on_one_line_is_refused_at_the_public_door() {
    let document = "<urn:ex:s1> <urn:ex:p> <urn:ex:o> . <urn:ex:s2> <urn:ex:p> <urn:ex:o> .\n";
    let buffered = parse_dataset(document.as_bytes(), NTRIPLES, None)
        .expect_err("one line holds at most one statement");
    assert_eq!(buffered.code, "native-codec-parse");
    assert!(
        buffered.message.contains("urn:ex:s2"),
        "the refusal must name the leftover token: {}",
        buffered.message
    );
    // Scalar 37 of the line is the `<` that opens the leftover subject.
    assert_eq!(buffered.location.as_ref().and_then(|l| l.column), Some(37));
    assert_eq!(
        document.chars().nth(36),
        Some('<'),
        "the reported column must be the one the user can point at"
    );
    let streamed = parse_dataset_from_reader(Cursor::new(document), NTRIPLES, None)
        .expect_err("the streaming door must agree");
    assert_eq!(streamed, buffered);

    // The neighbour that matters most: the SAME two statements, one per line, parse.
    assert_eq!(
        subject_iris(
            "<urn:ex:s1> <urn:ex:p> <urn:ex:o> .\n<urn:ex:s2> <urn:ex:p> <urn:ex:o> .\n",
            NTRIPLES,
        ),
        vec!["urn:ex:s1".to_owned(), "urn:ex:s2".to_owned()]
    );
    // And what the grammar DOES allow after a `'.'` — a comment, trailing `WS`, and (in
    // N-Quads) a graph label, which precedes the terminator — all still parse.
    assert_eq!(
        subject_iris("<urn:ex:s1> <urn:ex:p> <urn:ex:o> . # trailing\n", NTRIPLES),
        vec!["urn:ex:s1".to_owned()]
    );
    assert_eq!(
        subject_iris("<urn:ex:s1> <urn:ex:p> <urn:ex:o> . \t\n", NTRIPLES),
        vec!["urn:ex:s1".to_owned()]
    );
    assert_eq!(
        subject_iris(
            "<urn:ex:s1> <urn:ex:p> <urn:ex:o> <urn:ex:g> . # trailing\r",
            NQUADS,
        ),
        vec!["urn:ex:s1".to_owned()]
    );
}

/// The over-refusal side of the whole `EOL` change, executed at the public door: every
/// document shape the grammar admits still parses, and its content still arrives.
#[test]
fn the_valid_neighbours_of_the_eol_split_still_parse() {
    // LF-only, with blank lines, comments, indentation and no final terminator.
    let lf_only = concat!(
        "# a leading comment\n",
        "\n",
        "\t<urn:ex:s1> <urn:ex:p> \"tab\" .\n",
        "    <urn:ex:s2> <urn:ex:p> \"spaces\" . \t\n",
        "\t  # an indented comment\n",
        "<urn:ex:s3> <urn:ex:p> \"last\" .",
    );
    let expected = vec![
        "urn:ex:s1".to_owned(),
        "urn:ex:s2".to_owned(),
        "urn:ex:s3".to_owned(),
    ];
    assert_eq!(subject_iris(lf_only, NTRIPLES), expected);
    assert_eq!(streamed_subjects_agree(lf_only, NTRIPLES), expected);

    // The empty document, and documents of nothing but terminators.
    for text in ["", "\n", "\r", "\r\n", "\n\r\n\r", "# just a comment\r"] {
        assert!(
            parse_dataset(text.as_bytes(), NTRIPLES, None)
                .expect("a document with no statement is still a document")
                .quads()
                .next()
                .is_none(),
            "{text:?} carries no statement and no error"
        );
        assert!(
            parse_dataset_from_reader(Cursor::new(text), NTRIPLES, None)
                .expect("the streaming door must agree")
                .quads()
                .next()
                .is_none()
        );
    }

    // A carriage return written the way the grammar admits it — `ECHAR`'s `\r` inside a
    // literal — is CONTENT, reaches the dataset verbatim, and splits nothing. (A RAW
    // `#xD` there is excluded by `STRING_LITERAL_QUOTE ::= '"' ([^#x22#x5C#xA#xD] |
    // ECHAR | UCHAR)* '"'`, so this escape is the only spelling there is.)
    let escaped = "<urn:ex:s> <urn:ex:p> \"a\\rb\" .\n";
    let dataset = parse_dataset(escaped.as_bytes(), NTRIPLES, None).expect("`\\r` is ECHAR");
    let object = dataset
        .quads()
        .map(|quad| dataset.term_value(quad.o))
        .next()
        .expect("one quad");
    assert_eq!(
        object,
        TermValue::Literal {
            lexical_form: "a\rb".to_owned(),
            datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
            language: None,
            direction: None,
        },
        "the escaped carriage return must survive into the literal"
    );
}

/// A document large enough to cross the chunk-parallel threshold reads the same as the
/// sequential and the streaming lane, at CRLF and at lone-`#xD` endings alike.
///
/// This is where a chunk boundary landing between a `#xD` and its `#xA` would show: the
/// parallel path would see two terminators where the document has one, and every line
/// number after the boundary would drift.
#[test]
fn a_large_crlf_document_reads_identically_on_every_lane() {
    for terminator in ["\r\n", "\r", "\n"] {
        let mut text = String::with_capacity(1 << 21);
        let mut rows = 0;
        while text.len() < (1 << 20) + 4096 {
            write!(
                text,
                "<urn:ex:s{rows}> <urn:ex:p> \"row {rows}\" .{terminator}"
            )
            .expect("write row");
            rows += 1;
        }
        let buffered = parse_dataset(text.as_bytes(), NTRIPLES, None)
            .unwrap_or_else(|e| panic!("buffered parse of {terminator:?} document: {e}"));
        assert_eq!(
            buffered.quads().count(),
            rows,
            "every statement of the {terminator:?} document must arrive"
        );
        let streamed = parse_dataset_from_reader(Cursor::new(text.clone()), NTRIPLES, None)
            .expect("the streaming door must agree");
        assert_eq!(subjects_of(&streamed), subjects_of(&buffered));
    }
}
