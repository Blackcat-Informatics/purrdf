// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Egress totality for blank-node labels across the four W3C result-document
//! writers and the CONSTRUCT-graph N-Triples writer.
//!
//! A SPARQL result carries blank-node *labels*, not free text, in every one of
//! its serializations: the TSV/CSV cell is a Turtle `_:` token, the JSON
//! `"bnode"` `value` and the XML `<bnode>` element text are blank-node
//! identifiers. All four therefore share ONE alphabet — the W3C
//! `BLANK_NODE_LABEL` production — and all four escape an out-of-alphabet
//! label into it rather than refusing, so serializing a result is total. The
//! escape is deterministic and injective, so distinct blank nodes stay
//! distinct across the whole document and across formats.

use std::sync::Arc;

use purrdf_core::blank_label::{LabelAlphabet, is_valid_label};
use purrdf_core::{BlankScope, RdfDatasetBuilder, SparqlResult, TermValue};
use purrdf_sparql_results::{ResultProvenance, to_csv, to_json, to_tsv, to_xml};

/// The hostile-label table. Every entry must serialize in every writer; the
/// accept/reject columns are gone because there is no reject.
const HOSTILE_LABELS: &[&str] = &[
    "",
    "bad\u{1f}label",
    "a b",
    "a\u{d7}b",
    "<urn:x>",
    "0abc",
    "a.b",
    "trailing.",
    "-lead",
    "日本",
    "c14n0",
    "purrdfesc_a_000020b",
];

/// A one-variable SELECT whose single binding is a blank node with the given
/// raw label at the DEFAULT scope.
fn select_with_blank(label: &str) -> SparqlResult {
    SparqlResult::Solutions {
        variables: vec!["b".to_string()],
        rows: vec![vec![Some(TermValue::Blank {
            label: label.to_string(),
            scope: BlankScope::DEFAULT,
        })]],
        aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
    }
}

/// A CONSTRUCT-graph result whose one quad has a blank subject with the given
/// raw label at the DEFAULT scope.
fn graph_with_blank(label: &str) -> SparqlResult {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_blank(label, BlankScope::DEFAULT);
    let p = b.intern_iri("https://example.org/p");
    let o = b.intern_iri("https://example.org/o");
    b.push_quad(s, p, o, None);
    SparqlResult::Graph(Arc::clone(&b.freeze().expect("dataset freezes")))
}

fn text_of(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("writers emit utf-8")
}

/// The blank-node label a document wrote, extracted from the surrounding
/// syntax so the assertion checks the LABEL rather than the whole line.
fn label_between<'a>(text: &'a str, open: &str, close: &str) -> &'a str {
    let start = text
        .find(open)
        .unwrap_or_else(|| panic!("no {open:?} in {text}"))
        + open.len();
    let rest = &text[start..];
    let end = rest
        .find(close)
        .unwrap_or_else(|| panic!("no {close:?} after {open:?} in {text}"));
    &rest[..end]
}

/// Every blank-node label a document wrote, in order, extracted between each
/// successive `open`/`close` pair — the multi-row generalization of
/// [`label_between`], used where a document carries more than one binding and
/// the assertion needs each row's label rather than just the first.
fn all_labels_between<'a>(text: &'a str, open: &str, close: &str) -> Vec<&'a str> {
    let mut labels = Vec::new();
    let mut cursor = 0usize;
    while let Some(rel_start) = text[cursor..].find(open) {
        let start = cursor + rel_start + open.len();
        let Some(rel_end) = text[start..].find(close) else {
            break;
        };
        let end = start + rel_end;
        labels.push(&text[start..end]);
        cursor = end + close.len();
    }
    labels
}

#[test]
fn every_writer_emits_a_legal_blank_node_label() {
    let provenance = ResultProvenance::default();
    for label in HOSTILE_LABELS {
        let result = select_with_blank(label);

        let tsv = text_of(
            to_tsv(&result, &provenance)
                .unwrap_or_else(|e| panic!("TSV must serialize {label:?}: {e}"))
                .bytes,
        );
        let csv = text_of(
            to_csv(&result, &provenance)
                .unwrap_or_else(|e| panic!("CSV must serialize {label:?}: {e}"))
                .bytes,
        );
        let json = text_of(
            to_json(&result, &provenance, None)
                .unwrap_or_else(|e| panic!("JSON must serialize {label:?}: {e}"))
                .bytes,
        );
        let xml = text_of(
            to_xml(&result, &provenance, None)
                .unwrap_or_else(|e| panic!("XML must serialize {label:?}: {e}"))
                .bytes,
        );

        let emitted = [
            ("TSV", label_between(&tsv, "_:", "\n")),
            ("CSV", label_between(&csv, "_:", "\r\n")),
            ("JSON", label_between(&json, "\"bnode\",\"value\":\"", "\"")),
            ("XML", label_between(&xml, "<bnode>", "</bnode>")),
        ];
        for (name, written) in emitted {
            assert!(
                is_valid_label(written, LabelAlphabet::BlankNodeLabel),
                "{name} wrote {written:?} for {label:?}, which is not a legal blank-node label"
            );
        }

        // One result, one blank-node identity: the four writers agree on the
        // label, so a consumer joining across formats sees one node.
        let tsv_label = emitted[0].1;
        for (name, written) in emitted {
            assert_eq!(
                written, tsv_label,
                "{name} disagrees with TSV on the blank-node label for {label:?}"
            );
        }
    }
}

#[test]
fn the_hostile_label_reaches_json_and_xml_escaped_not_raw() {
    // The concrete regression: `a×b` used to reach the JSON/XML documents
    // verbatim (`{"type":"bnode","value":"a×b"}`), an identifier no consumer
    // could feed back into a `_:` term, while CSV/TSV hard-failed on it.
    let provenance = ResultProvenance::default();
    let result = select_with_blank("a\u{d7}b");
    let expected = "purrdfesc_a_0000D7b";

    let json = text_of(
        to_json(&result, &provenance, None)
            .expect("JSON serializes")
            .bytes,
    );
    assert!(
        json.contains(&format!("{{\"type\":\"bnode\",\"value\":\"{expected}\"}}")),
        "JSON must carry the escaped id: {json}"
    );
    assert!(
        !json.contains('\u{d7}'),
        "the raw label must not leak: {json}"
    );

    let xml = text_of(
        to_xml(&result, &provenance, None)
            .expect("XML serializes")
            .bytes,
    );
    assert!(
        xml.contains(&format!("<bnode>{expected}</bnode>")),
        "XML must carry the escaped id: {xml}"
    );
    assert!(
        !xml.contains('\u{d7}'),
        "the raw label must not leak: {xml}"
    );

    // …and CSV/TSV, which used to refuse it, now agree with them.
    let csv = text_of(to_csv(&result, &provenance).expect("CSV serializes").bytes);
    let tsv = text_of(to_tsv(&result, &provenance).expect("TSV serializes").bytes);
    assert!(csv.contains(&format!("_:{expected}")), "{csv}");
    assert!(tsv.contains(&format!("_:{expected}")), "{tsv}");
}

#[test]
fn distinct_blank_nodes_stay_distinct_in_every_writer() {
    // The adversarial pair: an illegal label and a legal one equal to its
    // escape image. A non-injective escape would merge the two rows.
    let provenance = ResultProvenance::default();
    let result = SparqlResult::Solutions {
        variables: vec!["b".to_string()],
        rows: vec![
            vec![Some(TermValue::Blank {
                label: "a b".to_string(),
                scope: BlankScope::DEFAULT,
            })],
            vec![Some(TermValue::Blank {
                label: "purrdfesc_a_000020b".to_string(),
                scope: BlankScope::DEFAULT,
            })],
        ],
        aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
    };

    let tsv = text_of(to_tsv(&result, &provenance).expect("TSV").bytes);
    let csv = text_of(to_csv(&result, &provenance).expect("CSV").bytes);
    let json = text_of(to_json(&result, &provenance, None).expect("JSON").bytes);
    let xml = text_of(to_xml(&result, &provenance, None).expect("XML").bytes);

    for (name, labels) in [
        ("TSV", all_labels_between(&tsv, "_:", "\n")),
        ("CSV", all_labels_between(&csv, "_:", "\r\n")),
        (
            "JSON",
            all_labels_between(&json, "\"bnode\",\"value\":\"", "\""),
        ),
        ("XML", all_labels_between(&xml, "<bnode>", "</bnode>")),
    ] {
        assert_eq!(
            labels.len(),
            2,
            "{name} must emit exactly two blank-node labels, one per row: {labels:?}"
        );
        // Extracting the two labels (rather than just counting `purrdfesc_`
        // occurrences) is the load-bearing part of this assertion: a
        // non-injective escape could still print `purrdfesc_` twice while
        // collapsing both rows onto the SAME label.
        assert_ne!(
            labels[0], labels[1],
            "{name} must emit two DISTINCT escaped labels, not collide the illegal \
             label's escape with the already-escaped-looking legal label: {labels:?}"
        );
    }
}

#[test]
fn construct_graph_writer_escapes_every_label() {
    // The CONSTRUCT graph rides inside the JSON document as N-Triples text,
    // through the same kernel writer, so its labels are escaped identically.
    let provenance = ResultProvenance::default();
    for label in HOSTILE_LABELS {
        let result = graph_with_blank(label);
        let bytes = to_json(&result, &provenance, None)
            .unwrap_or_else(|e| panic!("the graph writer must serialize {label:?}: {e}"))
            .bytes;
        let text = text_of(bytes);
        let emitted = label_between(&text, "_:", " ");
        assert!(
            is_valid_label(emitted, LabelAlphabet::BlankNodeLabel),
            "the graph writer wrote {emitted:?} for {label:?}"
        );
    }
}

#[test]
fn writers_are_byte_deterministic_for_escaped_labels() {
    let provenance = ResultProvenance::default();
    for label in HOSTILE_LABELS {
        let result = select_with_blank(label);
        assert_eq!(
            to_tsv(&result, &provenance).expect("TSV").bytes,
            to_tsv(&result, &provenance).expect("TSV").bytes,
            "{label:?}"
        );
        assert_eq!(
            to_json(&result, &provenance, None).expect("JSON").bytes,
            to_json(&result, &provenance, None).expect("JSON").bytes,
            "{label:?}"
        );
        assert_eq!(
            to_xml(&result, &provenance, None).expect("XML").bytes,
            to_xml(&result, &provenance, None).expect("XML").bytes,
            "{label:?}"
        );
    }
}
