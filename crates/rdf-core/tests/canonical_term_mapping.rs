// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact native identity through relabeling, independent of serialization.

use purrdf_core::{
    BlankScope, CanonError, CanonicalRelabeling, ContentIdScheme, QuadHandle, QuadIds, RdfDataset,
    RdfDatasetBuilder, RdfLiteral, RdfLocation, RdfTextDirection, TermId, TermRef,
    canonical_relabel, canonical_relabel_with_mapping,
};

const P: &str = "https://example.org/p";
const LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";

fn mapped(result: &CanonicalRelabeling, source: TermId) -> TermId {
    result.map_term(source).expect("retained source term")
}

/// Compare native IDs, labels and every record directly. Recanonicalizing both
/// would hide an incorrect label, record-order or term-allocation change.
fn assert_original_api_output(source: &RdfDataset, result: &CanonicalRelabeling) {
    let original = canonical_relabel(source).expect("admissible original API");
    let output = result.dataset();
    assert_eq!(original.term_count(), output.term_count());
    for index in 0..original.term_count() {
        let id = TermId::from_index(u32::try_from(index).expect("fixture index"));
        assert_eq!(original.resolve(id), output.resolve(id));
    }
    assert_eq!(
        original.quads().collect::<Vec<_>>(),
        output.quads().collect::<Vec<_>>()
    );
    assert_eq!(
        original.reifiers_with_graph().collect::<Vec<_>>(),
        output.reifiers_with_graph().collect::<Vec<_>>()
    );
    assert_eq!(
        original.annotations_with_graph().collect::<Vec<_>>(),
        output.annotations_with_graph().collect::<Vec<_>>()
    );
    assert_eq!(
        original.named_graphs().collect::<Vec<_>>(),
        output.named_graphs().collect::<Vec<_>>()
    );
}

fn assert_mapped_records(source: &RdfDataset, result: &CanonicalRelabeling) {
    let target = result.dataset();
    let m = |id| mapped(result, id);
    for (index, q) in source.quads().enumerate() {
        let expected = QuadIds {
            s: m(q.s),
            p: m(q.p),
            o: m(q.o),
            g: q.g.map(m),
        };
        let output_index = target
            .quads()
            .position(|q| q == expected)
            .expect("mapped quad");
        assert_eq!(
            source.location_of(QuadHandle::from_index(
                u32::try_from(index).expect("fixture index")
            )),
            target.location_of(QuadHandle::from_index(
                u32::try_from(output_index).expect("fixture index")
            ))
        );
    }
    for (r, t, g) in source.reifiers_with_graph() {
        assert!(
            target
                .reifiers_with_graph()
                .any(|row| row == (m(r), m(t), g.map(m)))
        );
    }
    for (r, p, o, g) in source.annotations_with_graph() {
        assert!(
            target
                .annotations_with_graph()
                .any(|row| row == (m(r), m(p), m(o), g.map(m)))
        );
    }
    for g in source.named_graphs() {
        assert!(target.named_graphs().any(|target| target == m(g)));
    }
}

fn assert_mapped_term(source: &RdfDataset, result: &CanonicalRelabeling, id: TermId) {
    let output = result.dataset().resolve(mapped(result, id));
    match source.resolve(id) {
        TermRef::Iri(iri) => assert_eq!(output, TermRef::Iri(iri)),
        TermRef::Blank { .. } => {
            let TermRef::Blank { label, scope } = output else {
                panic!("blank lost its kind")
            };
            assert!(label.starts_with("c14n"));
            assert_eq!(scope, BlankScope::DEFAULT);
        }
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => assert_eq!(
            output,
            TermRef::Literal {
                lexical,
                datatype: mapped(result, datatype),
                language,
                direction
            }
        ),
        TermRef::Triple { s, p, o } => assert_eq!(
            output,
            TermRef::Triple {
                s: mapped(result, s),
                p: mapped(result, p),
                o: mapped(result, o)
            }
        ),
    }
}

#[test]
fn mapped_records_preserve_native_identity_and_source_locations() {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(P);
    let a = builder.intern_blank("shared", BlankScope(2));
    let b = builder.intern_blank("shared", BlankScope(3));
    let graph = builder.intern_blank("graph", BlankScope(2));
    let empty = builder.intern_blank("empty", BlankScope(3));
    builder.declare_named_graph(empty);
    let typed = builder.intern_literal(RdfLiteral::typed("01", "https://example.org/datatype"));
    let language = builder.intern_literal(RdfLiteral::language_tagged("bonjour", "FR"));
    let directional = builder.intern_literal(RdfLiteral {
        lexical_form: "مرحبا".into(),
        datatype: None,
        language: Some("ar".into()),
        direction: Some(RdfTextDirection::Rtl),
    });
    let inner = builder.intern_triple(b, p, typed);
    let nested = builder.intern_triple(a, p, inner);
    let handle = builder.push_quad_with_handle(a, p, nested, Some(graph));
    builder.attach_location(
        handle,
        RdfLocation::file("source.ttl").with_line(7).with_column(3),
    );
    builder.push_quad(b, p, language, None);
    builder.push_reifier_in_graph(a, nested, Some(graph));
    builder.push_annotation_in_graph(a, p, directional, Some(graph));
    let source = builder.freeze().expect("valid RDF 1.2 fixture");
    let result = canonical_relabel_with_mapping(&source).expect("admissible");

    assert_ne!(
        mapped(&result, a),
        mapped(&result, b),
        "independent scopes must not conflate"
    );
    assert_mapped_records(&source, &result);
    for index in 0..source.term_count() {
        assert_mapped_term(
            &source,
            &result,
            TermId::from_index(u32::try_from(index).expect("fixture index")),
        );
    }
    assert_original_api_output(&source, &result);
    let count = result.dataset().term_count();
    assert_eq!(result.into_dataset().term_count(), count);
}

#[test]
fn composite_only_blanks_and_interned_iris_are_mapped() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri("https://example.org/s");
    let p = builder.intern_iri(P);
    let embedded = builder.intern_iri("https://example.org/embedded");
    let unused = builder.intern_iri("https://example.org/unused");
    let literal = builder.intern_literal(RdfLiteral::typed(
        "[_:hidden, [<https://example.org/embedded>, _:hidden], <https://example.org/lexical-only>]", LIST
    ));
    builder.push_quad(s, p, literal, None);
    let source = builder.freeze().expect("valid composite fixture");
    assert!(
        source
            .term_id_by_iri("https://example.org/lexical-only")
            .is_none()
    );
    let hidden = source
        .term_id_by_blank("hidden", BlankScope::DEFAULT)
        .expect("embedded blank");
    let result = canonical_relabel_with_mapping(&source).expect("admissible");
    let output = result.dataset();
    assert_eq!(
        output.resolve(mapped(&result, embedded)),
        TermRef::Iri("https://example.org/embedded")
    );
    assert_eq!(result.map_term(unused), None);
    let TermRef::Blank { label, scope } = output.resolve(mapped(&result, hidden)) else {
        panic!("embedded blank lost its kind")
    };
    assert_eq!(scope, BlankScope::DEFAULT);
    let TermRef::Literal {
        lexical, datatype, ..
    } = output.resolve(mapped(&result, literal))
    else {
        panic!("composite lost its kind")
    };
    assert_eq!(lexical.matches(&format!("_:{label}")).count(), 2);
    assert!(!lexical.contains("_:hidden"));
    assert_eq!(
        Some(datatype),
        result.map_term(source.term_id_by_iri(LIST).expect("source datatype"))
    );
    assert_original_api_output(&source, &result);
}

#[test]
fn unused_dictionary_terms_are_not_fabricated_by_mapping() {
    let mut builder = RdfDatasetBuilder::new();
    let unused_iri = builder.intern_iri("https://example.org/unused");
    let unused_blank = builder.intern_blank("unused", BlankScope::DEFAULT);
    let unused_literal = builder.intern_literal(RdfLiteral::typed(
        "absent",
        "https://example.org/unused-type",
    ));
    let declared = builder.intern_blank("only declaration", BlankScope(11));
    builder.declare_named_graph(declared);
    let source = builder.freeze().expect("valid declaration-only fixture");
    let result = canonical_relabel_with_mapping(&source).expect("admissible");
    for id in [
        unused_iri,
        unused_blank,
        unused_literal,
        source
            .term_id_by_iri("https://example.org/unused-type")
            .expect("unused datatype"),
    ] {
        assert_eq!(result.map_term(id), None);
    }
    assert_eq!(
        result.map_term(TermId::from_index(
            u32::try_from(source.term_count()).expect("fixture index")
        )),
        None
    );
    assert_eq!(
        result.dataset().resolve(mapped(&result, declared)),
        TermRef::Blank {
            label: "c14n0",
            scope: BlankScope::DEFAULT
        }
    );
    assert_mapped_records(&source, &result);
    assert_original_api_output(&source, &result);
}

#[test]
fn mapped_content_ids_and_predecessors_keep_the_configured_scheme() {
    let scheme = ContentIdScheme::new("blake3:").expect("valid scheme");
    let mut builder = RdfDatasetBuilder::with_content_addressing(
        scheme.clone(),
        Some("https://example.org/derivedFrom".into()),
    );
    let successor = builder
        .intern_iri("blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let predecessor = builder
        .intern_iri("blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let derived = builder.intern_iri("https://example.org/derivedFrom");
    let p = builder.intern_iri(P);
    let blank = builder.intern_blank("changed", BlankScope(2));
    builder.push_quad(successor, p, blank, None);
    builder.push_annotation(successor, derived, predecessor);
    let source = builder.freeze().expect("valid configured fixture");
    let result = canonical_relabel_with_mapping(&source).expect("admissible");
    assert_eq!(result.dataset().content_id_scheme(), Some(&scheme));
    for id in [successor, predecessor] {
        assert!(source.content_id(id).is_some());
        assert_eq!(
            source.content_id(id),
            result.dataset().content_id(mapped(&result, id))
        );
    }
    assert_eq!(
        result.dataset().predecessors(mapped(&result, successor)),
        &[mapped(&result, predecessor)]
    );
    assert_mapped_records(&source, &result);
    assert_original_api_output(&source, &result);
}

#[test]
fn mapping_does_not_change_admission_refusals() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri("https://example.org/s");
    let p = builder.intern_iri("urn:purrdf:rdfc:reifies");
    builder.push_quad(s, p, s, None);
    let source = builder
        .freeze()
        .expect("valid RDF with reserved vocabulary");
    let original = canonical_relabel(&source).expect_err("reserved vocabulary");
    let mapped = canonical_relabel_with_mapping(&source).expect_err("same refusal");
    assert!(matches!(mapped, CanonError::ReservedVocabulary(_)));
    assert_eq!(original.to_string(), mapped.to_string());
}
