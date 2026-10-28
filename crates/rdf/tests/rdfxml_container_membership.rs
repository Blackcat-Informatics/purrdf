// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RDF/XML §7.4 list expansion preserves ordered and repeated container members.

use purrdf_iri::vocab::rdf;
use purrdf_rdf::{NativeRdfFormat, dataset_from_bytes, datasets_isomorphic};

fn assert_expansion(xml: &str, expected: &str) {
    let xml = format!(
        r#"<rdf:RDF xmlns:rdf="{}" xmlns:ex="http://example.org/" rdf:version="1.2"
            xml:base="http://example.org/">{xml}</rdf:RDF>"#,
        rdf::NS,
    );
    let actual = dataset_from_bytes(xml.as_bytes(), NativeRdfFormat::RdfXml)
        .expect("RDF/XML membership document");
    let expected = format!(
        "@prefix rdf: <{}> . @prefix ex: <http://example.org/> . {expected}",
        rdf::NS,
    );
    let expected = dataset_from_bytes(expected.as_bytes(), NativeRdfFormat::Turtle)
        .expect("expanded membership graph");
    assert!(
        datasets_isomorphic(&actual, &expected).expect("graph comparison"),
        "membership expansion differs for {xml}"
    );
}

#[test]
fn li_preserves_order_and_duplicates_under_every_node_kind() {
    for (node, rdf_type) in [
        ("rdf:Bag", "rdf:Bag"),
        ("rdf:Seq", "rdf:Seq"),
        ("rdf:Alt", "rdf:Alt"),
        ("ex:Ordinary", "ex:Ordinary"),
        ("rdf:Description", ""),
    ] {
        let xml = format!(
            r#"<{node} rdf:about="http://example.org/b">
                <rdf:li>coast</rdf:li>
                <rdf:_9>explicit</rdf:_9><ex:li>unrelated</ex:li>
                <rdf:li>lighthouse</rdf:li><rdf:li>coast</rdf:li>
            </{node}>"#,
        );
        let typed = if rdf_type.is_empty() {
            String::new()
        } else {
            format!("ex:b a {rdf_type} .")
        };
        assert_expansion(
            &xml,
            &format!(
                r#"{typed} ex:b rdf:_1 "coast" ; rdf:_2 "lighthouse" ; rdf:_3 "coast" ;
                    rdf:_9 "explicit" ; ex:li "unrelated" ."#,
            ),
        );
    }
}

#[test]
fn li_counter_is_local_to_each_explicit_or_implied_node() {
    assert_expansion(
        r#"<rdf:Description rdf:about="http://example.org/outer">
            <rdf:li>before</rdf:li>
            <rdf:li><rdf:Description rdf:about="http://example.org/inner">
                <rdf:li>inner first</rdf:li><rdf:li>inner second</rdf:li>
            </rdf:Description></rdf:li>
            <rdf:li rdf:parseType="Resource">
                <rdf:li>resource first</rdf:li>
                <ex:nested rdf:parseType="Resource"><rdf:li>nested first</rdf:li></ex:nested>
                <rdf:li>resource second</rdf:li>
            </rdf:li>
            <rdf:li rdf:resource="http://example.org/after"/>
        </rdf:Description>
        <rdf:Description rdf:about="http://example.org/outer"><rdf:li>reset</rdf:li></rdf:Description>"#,
        r#"ex:outer rdf:_1 "before", "reset" ; rdf:_2 ex:inner ;
            rdf:_3 [ rdf:_1 "resource first" ; ex:nested [ rdf:_1 "nested first" ] ;
                rdf:_2 "resource second" ] ; rdf:_4 ex:after .
        ex:inner rdf:_1 "inner first" ; rdf:_2 "inner second" ."#,
    );
}

#[test]
fn reification_and_triple_terms_use_the_expanded_predicate() {
    assert_expansion(
        r##"<rdf:Description rdf:about="http://example.org/b">
            <rdf:li rdf:ID="first">coast</rdf:li>
            <rdf:li rdf:annotation="#second" rdf:nodeID="member"/>
            <ex:quoted rdf:parseType="Triple">
                <rdf:Description rdf:about="http://example.org/quoted">
                    <rdf:li>quoted first</rdf:li>
                </rdf:Description>
            </ex:quoted>
        </rdf:Description>"##,
        r#"ex:b rdf:_1 "coast" ; rdf:_2 _:member ;
            ex:quoted <<( ex:quoted rdf:_1 "quoted first" )>> .
        <http://example.org/#first> a rdf:Statement ; rdf:subject ex:b ;
            rdf:predicate rdf:_1 ; rdf:object "coast" .
        <http://example.org/#second> rdf:reifies <<( ex:b rdf:_2 _:member )>> ."#,
    );
}
