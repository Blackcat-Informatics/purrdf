// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RDF/XML writer's predicate name class, proved by ROUND TRIP.
//!
//! A serializer's character class is not a membership test, it is a claim about what the
//! reader on the other side will do. So every case below writes RDF/XML with the
//! production writer and reads it back with the production reader, and asserts the graph
//! came back identical. A unit test over the predicate alone could not have caught what
//! this file catches: the class and the split point it feeds are one decision, and the
//! old code got a `ns:ªx` element name past every unit assertion while emitting XML no
//! conforming parser will read.
//!
//! The productions at stake:
//!
//! > `NameStartChar ::= ':' | [A-Z] | '_' | [a-z] | [#xC0-#xD6] | [#xD8-#xF6] |`
//! > `[#xF8-#x2FF] | [#x370-#x37D] | [#x37F-#x1FFF] | [#x200C-#x200D] |`
//! > `[#x2070-#x218F] | [#x2C00-#x2FEF] | [#x3001-#xD7FF] | [#xF900-#xFDCF] |`
//! > `[#xFDF0-#xFFFD] | [#x10000-#xEFFFF]`
//! >
//! > `NameChar ::= NameStartChar | '-' | '.' | [0-9] | #xB7 | [#x300-#x36F] |`
//! > `[#x203F-#x2040]`
//!
//! — XML 1.0 Fifth Edition §2.3 `[4]` / `[4a]`, and
//!
//! > `NCName ::= NCNameStartChar NCNameChar*`
//! >
//! > `NCNameChar ::= NameChar - ':'`
//! >
//! > `NCNameStartChar ::= NameStartChar - ':'`
//!
//! — Namespaces in XML 1.0 (Third Edition) §3, which is the class an element's local
//! part actually has to satisfy, because an element name is a `QName ::= PrefixedName |`
//! `UnprefixedName` and a `PrefixedName ::= Prefix ':' LocalPart`.

use purrdf_rdf::{
    NativeRdfFormat, SerializeGraph, SerializeOptions, StatementLayer, dataset_from_bytes,
    serialize_dataset_to_format, serialize_dataset_with,
};

/// Write `nt` as RDF/XML with the production writer, read the result back with the
/// production reader, and return the re-read graph as sorted N-Triples.
///
/// The assertion every caller then makes is `== nt`: writing and reading must compose to
/// the identity on the graph. Returning the text rather than asserting inside lets a
/// failing case print what actually came back.
fn roundtrip_through_rdfxml(nt: &str) -> String {
    let original = dataset_from_bytes(nt.as_bytes(), NativeRdfFormat::NTriples)
        .unwrap_or_else(|e| panic!("the fixture must parse as N-Triples: {e}"));
    let xml = serialize_dataset_to_format(&*original, NativeRdfFormat::RdfXml, None)
        .unwrap_or_else(|e| panic!("the fixture must serialize as RDF/XML: {e}"))
        .bytes;
    let reread = dataset_from_bytes(&xml, NativeRdfFormat::RdfXml).unwrap_or_else(|e| {
        panic!(
            "the emitted RDF/XML must parse: {e}\n--- emitted ---\n{}",
            String::from_utf8_lossy(&xml)
        )
    });
    let back = serialize_dataset_to_format(&*reread, NativeRdfFormat::NTriples, None)
        .expect("N-Triples egress")
        .bytes;
    let mut lines: Vec<String> = String::from_utf8(back)
        .expect("N-Triples is UTF-8")
        .lines()
        .map(str::to_owned)
        .collect();
    lines.sort();
    lines.push(String::new());
    lines.join("\n")
}

/// One triple whose predicate carries `local`, in sorted-N-Triples form.
fn fixture(local: &str) -> String {
    format!("<http://example.org/s> <http://example.org/ns#{local}> <http://example.org/o> .\n")
}

/// The case that motivated the whole change: U+00AA FEMININE ORDINAL INDICATOR is
/// `char::is_alphabetic` and is **not** a `NameStartChar` — `[4]` goes `[A-Za-z_:]`
/// straight to `[#xC0-#xD6]`, and U+00AA sits below that first non-ASCII range. The old
/// class admitted it, so the writer emitted the element `<ns0:ªx>`, which no conforming
/// XML parser will read back.
///
/// The fix is not to refuse the IRI: the split point moves instead, so the local part
/// becomes the `NCName` suffix `x` and U+00AA joins the namespace. The predicate that
/// comes back is the predicate that went in.
#[test]
fn a_feminine_ordinal_indicator_does_not_open_an_element_name() {
    let nt = fixture("\u{AA}x");
    assert_eq!(roundtrip_through_rdfxml(&nt), nt);
}

/// The over-refusal half, and the larger one. `NameChar` adds U+00B7, the 112 combining
/// marks `[#x300-#x36F]` and `[#x203F-#x2040]`; none of those is `char::is_numeric` or
/// `char::is_alphabetic`, so the old class refused EVERY NFD-decomposed local name and
/// routed it down the lossy fallback. `café` spelled `cafe` + U+0301 is canonically
/// equivalent to the composed spelling and is what many exporters emit.
#[test]
fn an_nfd_decomposed_local_name_round_trips() {
    let nt = fixture("caf\u{65}\u{301}");
    assert_eq!(roundtrip_through_rdfxml(&nt), nt);
}

/// U+00B7 MIDDLE DOT and the two ties `[#x203F-#x2040]` are `NameChar` and not
/// `NameStartChar`, so they must continue a local name and never open one.
#[test]
fn middle_dot_and_undertie_continue_a_local_name() {
    for local in ["a\u{B7}b", "a\u{203F}b", "a\u{2040}b"] {
        let nt = fixture(local);
        assert_eq!(roundtrip_through_rdfxml(&nt), nt, "local part {local:?}");
    }
}

/// The neighbour that must not regress: an ordinary ASCII local name still writes as a
/// prefixed name against the namespace an author would expect.
#[test]
fn an_ordinary_ascii_local_name_still_round_trips() {
    for local in ["p", "hasName", "a-b", "a.b", "a0", "_p"] {
        let nt = fixture(local);
        assert_eq!(roundtrip_through_rdfxml(&nt), nt, "local part {local:?}");
    }
}

/// A `':'` IS a `NameStartChar`, and is exactly what `NCName` subtracts — an element
/// name may hold only one colon, the prefix separator. So a colon in the IRI's tail
/// cannot be part of the local part; it ends up in the namespace and the triple still
/// round-trips.
#[test]
fn a_colon_in_the_tail_belongs_to_the_namespace_not_the_local_part() {
    for nt in [
        fixture("a:b"),
        "<http://example.org/s> <urn:example:pred> <http://example.org/o> .\n".to_owned(),
    ] {
        assert_eq!(roundtrip_through_rdfxml(&nt), nt, "fixture {nt:?}");
    }
}

/// The split point is the LONGEST `NCName` suffix, so an IRI whose tail begins with a
/// character that may only CONTINUE a name still round-trips — the namespace simply
/// absorbs the leading scalar rather than the predicate being rewritten.
#[test]
fn a_tail_that_cannot_open_a_name_moves_the_split_rather_than_the_predicate() {
    for local in ["1abc", "-abc", ".abc", "\u{301}abc"] {
        let nt = fixture(local);
        assert_eq!(roundtrip_through_rdfxml(&nt), nt, "local part {local:?}");
    }
}

/// An IRI with no `NCName` suffix at all cannot be an RDF/XML element name, and RDF/XML
/// offers no other spelling for a predicate. The writer therefore FAILS, naming the IRI,
/// rather than emitting a different predicate — which is what it used to do.
#[test]
fn an_iri_with_no_ncname_suffix_is_refused_rather_than_rewritten() {
    let nt = "<http://example.org/s> <http://example.org/123> <http://example.org/o> .\n";
    let dataset = dataset_from_bytes(nt.as_bytes(), NativeRdfFormat::NTriples).expect("N-Triples");
    let error = serialize_dataset_to_format(&*dataset, NativeRdfFormat::RdfXml, None)
        .expect_err("an unwritable predicate must be refused, not silently rewritten");
    assert!(
        error.message.contains("http://example.org/123"),
        "the refusal must name the offending IRI, got: {}",
        error.message
    );
    // The valid neighbour, executed: one scalar later the tail IS an `NCName` and the
    // same shape of IRI writes and reads back unchanged. The refusal above is exactness,
    // not a narrowed alphabet.
    let ok = "<http://example.org/s> <http://example.org/a123> <http://example.org/o> .\n";
    assert_eq!(roundtrip_through_rdfxml(ok), ok);
}

/// The statement layer takes the same path. A reifier binding renders as
/// `rdf:parseType="Triple"`, and the quoted triple's own predicate is qualified by
/// `write_property` INSIDE that element — a predicate the namespace pre-pass never saw,
/// because `serializer_namespaces` walks only top-level properties. It must still be
/// declared, or the document does not parse at all.
///
/// RDF/XML is star-INcapable under the transcode loss contract, so the statement layer
/// only reaches the writer when a caller asks for it explicitly with
/// [`StatementLayer::Emit`]; that is the configuration under test here.
#[test]
fn a_quoted_triples_inner_predicate_is_declared_too() {
    let nt = concat!(
        "<http://example.org/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
        "<<( <http://example.org/s> <http://inner.example/vocab#p> <http://example.org/o> )>> .\n",
    );
    let dataset = dataset_from_bytes(nt.as_bytes(), NativeRdfFormat::NTriples).expect("N-Triples");
    let xml = serialize_dataset_with(
        &*dataset,
        NativeRdfFormat::RdfXml,
        None,
        &SerializeOptions {
            selection: SerializeGraph::Dataset,
            statement_layer: StatementLayer::Emit,
            jsonld_options: None,
        },
    )
    .expect("RDF/XML egress")
    .bytes;
    let text = String::from_utf8(xml.clone()).expect("RDF/XML is UTF-8");
    assert!(
        text.contains("http://inner.example/vocab#"),
        "the inner predicate's namespace must be DECLARED, not left to an undeclared \
         prefix; got:\n{text}"
    );
    let reread = dataset_from_bytes(&xml, NativeRdfFormat::RdfXml)
        .unwrap_or_else(|e| panic!("the emitted RDF/XML must parse: {e}\n{text}"));
    let back = serialize_dataset_with(
        &*reread,
        NativeRdfFormat::NTriples,
        None,
        &SerializeOptions {
            selection: SerializeGraph::Dataset,
            statement_layer: StatementLayer::Emit,
            jsonld_options: None,
        },
    )
    .expect("N-Triples egress")
    .bytes;
    assert_eq!(String::from_utf8(back).expect("UTF-8"), nt);
}
