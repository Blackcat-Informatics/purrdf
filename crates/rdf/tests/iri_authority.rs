// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public RDF IRIREF admission follows the central authority grammar.

use purrdf_rdf::{TermValue, parse_dataset};

#[test]
fn rdf_irirefs_accept_generic_ports_and_preserve_ip_literal_spelling() {
    for iri in [
        "http://[2001:DB8::1]:99999/",
        "http://[Vf.address]:000000000000000000000000001/",
        "http://h:99999999999999999999999999999/",
    ] {
        let source = format!("<{iri}> <https://example.org/p> <https://example.org/o> .");
        for media_type in ["text/turtle", "application/n-triples"] {
            let dataset = parse_dataset(source.as_bytes(), media_type, None)
                .expect("generic authority syntax");
            assert!(
                dataset.term_id_by_value(&TermValue::iri(iri)).is_some(),
                "lexical identity changed for {iri}"
            );
        }
    }
}

#[test]
fn rdf_irirefs_and_base_directives_reject_invalid_ip_literals() {
    for iri in [
        "http://[not-ip]/",
        "http://[1::2::3]/",
        "http://[vG.a]/",
        "http://[::ffff:256.0.0.1]/",
    ] {
        let source = format!("<{iri}> <https://example.org/p> <https://example.org/o> .");
        for media_type in ["text/turtle", "application/n-triples"] {
            assert!(
                parse_dataset(source.as_bytes(), media_type, None).is_err(),
                "{media_type} admitted {iri}"
            );
        }
        let source = format!("@base <{iri}> . <s> <p> <o> .");
        assert!(
            parse_dataset(source.as_bytes(), "text/turtle", None).is_err(),
            "base directive admitted {iri}"
        );
    }
}
