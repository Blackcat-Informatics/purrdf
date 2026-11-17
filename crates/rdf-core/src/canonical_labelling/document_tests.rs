// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The canonical document: what each kind of line is spelled as, in the order
//! of the canonical line sequence, and the refusal of bytes that are not a line
//! encoding.

use super::{DocumentError, Line, Term, label, write_document, write_line};

const XSD_INTEGER: &[u8] = b"http://www.w3.org/2001/XMLSchema#integer";
const XSD_STRING: &[u8] = b"http://www.w3.org/2001/XMLSchema#string";
const DIR_LANG: &[u8] = b"http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";

fn iri(local: &str) -> Term {
    Term::iri(format!("http://example.org/{local}").as_bytes())
}

fn blank(name: &str) -> Term {
    Term::blank(name.as_bytes())
}

/// One line of each kind that has a spelling, and the two kinds that have
/// none, over three blank nodes.
fn lines() -> Vec<Line> {
    let language = Term::Literal {
        lexical: (*b"v").into(),
        datatype: DIR_LANG.into(),
        language: Some((*b"en").into()),
        direction: Some(super::Direction::Ltr),
    };
    vec![
        Line::Quad {
            subject: blank("a"),
            predicate: iri("p"),
            object: blank("b"),
            graph: None,
        },
        Line::Quad {
            subject: blank("b"),
            predicate: iri("p"),
            object: Term::typed(b"1", XSD_INTEGER),
            graph: Some(iri("g")),
        },
        Line::Quad {
            subject: iri("s"),
            predicate: iri("p"),
            object: Term::typed(b"plain", XSD_STRING),
            graph: None,
        },
        Line::ReifierBinding {
            reifier: blank("r"),
            triple: Term::triple(blank("a"), iri("p"), blank("b")),
            graph: None,
        },
        Line::Annotation {
            count: 2,
            reifier: blank("r"),
            predicate: iri("q"),
            object: language,
            graph: None,
        },
        Line::EmptyGraph(iri("e")),
        Line::Record {
            payload: (*b"m").into(),
            terms: vec![blank("a")],
        },
    ]
}

/// The document of the canonical line sequence is the N-Quads 1.2 spelling of
/// each statement line in the order of the line encodings: `rdf:reifies` for a
/// binding, one statement for an annotation whatever its count, decimal
/// labels, no `xsd:string` suffix, and nothing for an `empty-graph` or a
/// `record` line.
#[test]
fn the_document_spells_each_kind_in_the_order_of_the_canonical_sequence() {
    let (_, labelled) = label(&lines()).expect("labelled");
    let mut document = String::new();
    write_document(&labelled.lines, &mut document).expect("spelled");
    purrdf_testkit::assert_golden!("canonical-document.nq", &document);
}

/// A truncated encoding, trailing bytes and a `free` leaf are refused; the
/// whole encoding of the same line is spelled.
#[test]
fn bytes_that_are_not_a_line_encoding_are_refused_and_the_whole_encoding_is_not() {
    let (_, labelled) = label(&lines()).expect("labelled");
    let encoding = labelled
        .lines
        .iter()
        .find(|line| line[0] == 0x40)
        .expect("a quad line");
    let mut out = String::new();
    write_line(encoding, &mut out).expect("a whole encoding is spelled");
    assert!(out.ends_with(" .\n"));

    let truncated = &encoding[..encoding.len() - 1];
    assert_eq!(
        write_line(truncated, &mut String::new()),
        Err(DocumentError::Truncated)
    );
    let mut trailing = encoding.to_vec();
    trailing.push(0);
    assert_eq!(
        write_line(&trailing, &mut String::new()),
        Err(DocumentError::Truncated)
    );
    // `quad(free, iri(p), free)` with no graph: the masked line of 3.7.
    let masked = [0x40, 0x04, 0x01, 0, 0, 0, 0, 0, 0, 0, 1, b'p', 0x04, 0x20];
    assert_eq!(
        write_line(&masked, &mut String::new()),
        Err(DocumentError::Tag(0x04))
    );
}

/// N-Quads 1.2 section 3 writes a character outside XML 1.1 `Char` as `UCHAR`,
/// so the document spells U+FFFE and U+FFFF in a literal as escapes while the
/// line encoding, which is what the digest covers, keeps their UTF-8 bytes.
/// The neighbours U+FFFD and the C1 control U+0085 stay raw in the document.
#[test]
fn the_document_escapes_noncharacters_and_the_digest_input_does_not() {
    let lexical = "a\u{fffe}b\u{ffff}c\u{fffd}d\u{85}e";
    let sequence = [Line::Quad {
        subject: iri("s"),
        predicate: iri("p"),
        object: Term::typed(lexical.as_bytes(), XSD_STRING),
        graph: None,
    }];
    let (_, labelled) = label(&sequence).expect("labelled");
    let mut document = String::new();
    write_document(&labelled.lines, &mut document).expect("spelled");
    assert_eq!(
        document,
        "<http://example.org/s> <http://example.org/p> \"a\\uFFFEb\\uFFFFc\u{fffd}d\u{85}e\" .\n"
    );
    let encoding = labelled.lines.iter().next().expect("one line");
    let holds = |needle: &[u8]| encoding.windows(needle.len()).any(|w| w == needle);
    assert!(holds("\u{fffe}".as_bytes()) && holds("\u{ffff}".as_bytes()));
    assert!(!holds(b"\\uFFFE"));
}
