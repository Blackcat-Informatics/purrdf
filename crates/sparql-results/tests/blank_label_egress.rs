// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Egress enforcement for blank-node labels across the four W3C result-document
//! writers and the CONSTRUCT-graph N-Triples writer.
//!
//! The TSV/CSV writers emit `_:label` tokens that must re-lex as Turtle terms,
//! so they enforce the exact W3C `BLANK_NODE_LABEL` alphabet; the JSON/XML
//! writers escape the label as an opaque string, so any NON-EMPTY label is
//! accepted (only emptiness is refused). The CONSTRUCT-graph writer emits
//! N-Triples lines and enforces `BLANK_NODE_LABEL` like the tabular writers.
//! A rejection is always a hard [`Error`] — never a silent remap.

use std::sync::Arc;

use purrdf_core::{BlankScope, RdfDatasetBuilder, SparqlResult, TermValue};
use purrdf_sparql_results::{ResultProvenance, to_csv, to_json, to_tsv, to_xml};

/// The hostile-label table: `(label, legal as BLANK_NODE_LABEL)`. Restated
/// independently of `purrdf_core::blank_label` so the matrix cross-checks the
/// writer stack rather than echoing the validator. Every label is non-empty, so
/// the JSON/XML (unconstrained) verdict is always ACCEPT.
const HOSTILE_LABELS: &[(&str, bool)] = &[
    ("bad\u{1f}label", false),
    ("a b", false),
    ("<urn:x>", false),
    ("0abc", true),
    ("a.b", true),
    ("trailing.", false),
    ("-lead", false),
    ("\u{d7}y", false),
    ("日本", true),
    ("c14n0", true),
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

#[test]
fn tabular_writers_enforce_the_blank_node_label_alphabet() {
    let provenance = ResultProvenance::default();
    for &(label, bnl_ok) in HOSTILE_LABELS {
        let result = select_with_blank(label);
        for (name, outcome) in [
            ("TSV", to_tsv(&result, &provenance)),
            ("CSV", to_csv(&result, &provenance)),
        ] {
            if bnl_ok {
                let out = outcome
                    .unwrap_or_else(|e| panic!("{name} must accept blank label {label:?}: {e}"));
                assert!(
                    String::from_utf8(out.bytes).expect("utf-8").contains("_:"),
                    "{name} emits the blank token for {label:?}"
                );
            } else {
                let err =
                    outcome.expect_err(&format!("{name} must reject blank label {label:?} loudly"));
                assert!(
                    err.to_string().contains("blank-node label"),
                    "{name} error names the failure for {label:?}: {err}"
                );
            }
        }
    }
}

#[test]
fn json_and_xml_writers_accept_any_non_empty_label() {
    let provenance = ResultProvenance::default();
    for &(label, _) in HOSTILE_LABELS {
        let result = select_with_blank(label);
        to_json(&result, &provenance)
            .unwrap_or_else(|e| panic!("JSON must accept blank label {label:?}: {e}"));
        // The XML LABEL alphabet is likewise unconstrained, but the XML 1.0
        // DOCUMENT layer independently refuses characters XML cannot represent
        // at all (C0 controls other than tab/LF/CR are illegal even as
        // character references) — an orthogonal, pre-existing representability
        // hard error, not a label-alphabet rejection.
        let xml_representable = !label
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\t' | '\n' | '\r'));
        let outcome = to_xml(&result, &provenance);
        if xml_representable {
            outcome.unwrap_or_else(|e| panic!("XML must accept blank label {label:?}: {e}"));
        } else {
            let err = outcome.expect_err(&format!(
                "XML must reject the XML-unrepresentable label {label:?} loudly"
            ));
            assert!(
                err.to_string().contains("cannot represent"),
                "the rejection is the XML character constraint, not a label-alphabet check: {err}"
            );
        }
    }
}

#[test]
fn every_writer_rejects_the_empty_label() {
    let provenance = ResultProvenance::default();
    let result = select_with_blank("");
    to_tsv(&result, &provenance).expect_err("TSV rejects the empty label");
    to_csv(&result, &provenance).expect_err("CSV rejects the empty label");
    to_json(&result, &provenance).expect_err("JSON rejects the empty label");
    to_xml(&result, &provenance).expect_err("XML rejects the empty label");
}

#[test]
fn construct_graph_writer_enforces_the_blank_node_label_alphabet() {
    // The CONSTRUCT graph rides inside the JSON document as N-Triples text, so
    // its labels must satisfy BLANK_NODE_LABEL even though the JSON *bindings*
    // alphabet is unconstrained.
    let provenance = ResultProvenance::default();
    for &(label, bnl_ok) in HOSTILE_LABELS {
        let result = graph_with_blank(label);
        let outcome = to_json(&result, &provenance);
        if bnl_ok {
            outcome
                .unwrap_or_else(|e| panic!("graph writer must accept blank label {label:?}: {e}"));
        } else {
            let err = outcome.expect_err(&format!(
                "graph writer must reject blank label {label:?} loudly"
            ));
            assert!(
                err.to_string().contains("blank-node label"),
                "graph-writer error names the failure for {label:?}: {err}"
            );
        }
    }
}
