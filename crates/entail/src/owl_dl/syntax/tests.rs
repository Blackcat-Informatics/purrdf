// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native syntax capture on tiny authored RDF, never repository corpus production.

use super::{Capture, Kind, Read, SelectionError, Space};
use crate::owl_dl::{
    Kb,
    input_source::Input,
    parser::{CeExtractor, TripleIndex, Vocab, index_insert},
};
use crate::reasoner::schema_tests::Fixture;
use purrdf_iri::vocab::{owl, rdf, rdfs};

#[test]
fn native_syntax_shares_reads_but_preserves_distinct_normalized_spellings() {
    let mut f = Fixture::default();
    let a = f.name("A");
    let b = f.name("B");
    let first = f.blank();
    let second = f.blank();
    let forward = f.list(&[a, b]);
    let reverse = f.list(&[b, a]);
    f.edge(first, owl::INTERSECTION_OF, forward);
    f.edge(second, owl::INTERSECTION_OF, reverse);
    let unrelated = f.name("unrelated");
    f.edge(first, rdfs::SEE_ALSO, unrelated);
    for index in 0..100 {
        f.instance(&format!("i{index}"), first);
    }
    f.instance("different-spelling", second);
    let kb = Kb::from_dataset(&f.freeze()).unwrap();
    let roots: Vec<_> = (0..kb.abox_types.len())
        .map(|index| {
            kb.inputs
                .syntax(Input::Type(index))
                .unwrap()
                .root()
                .unwrap()
        })
        .collect();
    assert_eq!(roots.len(), 101);
    assert!(roots[..100].iter().all(|&root| root == roots[0]));
    assert_ne!(roots[0], roots[100]);
    assert!(
        kb.abox_types
            .iter()
            .all(|&(_, concept)| concept == kb.abox_types[0].1)
    );
    assert!(
        kb.syntax.nodes.len() < 16,
        "shared syntax is not copied once per assertion"
    );
    let chosen = kb.syntax.selected(&[roots[0]], None).unwrap();
    let other = kb.syntax.selected(&[roots[100]], None).unwrap();
    let triples = |ids: &[usize]| {
        ids.iter()
            .flat_map(|&id| {
                kb.syntax.nodes[id]
                    .reads
                    .iter()
                    .filter_map(|read| match read {
                        Read::Triple(s, p, o) => Some((*s, *p, *o)),
                        Read::Child(_) => None,
                    })
            })
            .collect::<Vec<_>>()
    };
    let original = triples(&chosen);
    let alternate = triples(&other);
    assert_ne!(original, alternate);
    assert!(original.iter().all(|&(_, p, _)| kb.interner.value(p)
        != &purrdf_core::TermValue::Iri(rdfs::SEE_ALSO.to_owned())));
    assert!(
        original.iter().any(|&(_, p, _)| kb.interner.value(p)
            == &purrdf_core::TermValue::Iri(rdf::FIRST.to_owned()))
    );
    assert!(original.iter().any(
        |&(_, p, _)| kb.interner.value(p) == &purrdf_core::TermValue::Iri(rdf::REST.to_owned())
    ));
}

#[test]
fn native_syntax_keeps_source_and_query_scaffolding_separate_for_the_same_term() {
    let mut f = Fixture::default();
    let class = f.name("C");
    let source_operand = f.name("SourceOperand");
    f.edge(class, owl::COMPLEMENT_OF, source_operand);
    f.instance("i", class);
    let mut kb = Kb::from_dataset(&f.freeze()).unwrap();
    let class = kb.interner.id_of_iri("https://example.org/C").unwrap();
    let query_operand = kb.interner.intern_iri("https://example.org/QueryOperand");
    let vocab = Vocab::intern(&mut kb.interner);
    let mut index = TripleIndex::new();
    index_insert(&mut index, class, vocab.complement, query_operand);
    let mut ce = CeExtractor::new(&index, &kb.interner, &vocab, &mut kb.data_ranges);
    ce.expr(class).unwrap();
    assert!(ce.is_class_expression(class));
    let query = ce.into_syntax().unwrap();
    let source_root = kb.syntax.root(Kind::Expression(class)).unwrap();
    let query_root = query.root(Kind::Expression(class)).unwrap();
    assert_eq!(kb.syntax.space, Space::Source);
    assert_eq!(query.space, Space::Query);
    let recognition = query.root(Kind::Recognition(class)).unwrap();
    assert!(query.nodes[recognition].complete);
    assert!(query.nodes[recognition].reads.contains(&Read::Triple(
        class,
        vocab.complement,
        query_operand
    )));
    let source = kb.syntax.selected(&[source_root], None).unwrap();
    let queried = query.selected(&[query_root], None).unwrap();
    assert!(
        !source
            .iter()
            .any(|&id| kb.syntax.nodes[id].reads.contains(&Read::Triple(
                class,
                vocab.complement,
                query_operand
            )))
    );
    assert!(
        queried
            .iter()
            .any(|&id| query.nodes[id].reads.contains(&Read::Triple(
                class,
                vocab.complement,
                query_operand
            )))
    );
    assert_eq!(
        query.selected(&[usize::MAX], None),
        Err(SelectionError::Missing(usize::MAX))
    );
}

#[test]
fn malformed_native_list_keeps_its_original_refusal_and_cannot_publish_syntax() {
    use crate::{CollectionPhase, EntailError, interner::Interner, owl_dl::data::DataRangeTable};
    let mut interner = Interner::default();
    let vocab = Vocab::intern(&mut interner);
    let class = interner.intern_iri("https://example.org/BadList");
    let head = interner.intern_iri("https://example.org/Head");
    let member = interner.intern_iri("https://example.org/Member");
    let mut index = TripleIndex::new();
    index_insert(&mut index, class, vocab.intersection, head);
    index_insert(&mut index, head, vocab.first, member);
    let mut ranges = DataRangeTable::default();
    let mut ce = CeExtractor::new(&index, &interner, &vocab, &mut ranges);
    let error = ce.expr(class).unwrap_err();
    let EntailError::Collection(original) = error else {
        panic!("expected the original collection refusal, got {error:?}");
    };
    assert_eq!(original.phase, CollectionPhase::OwlDirect);
    assert_eq!(original.head, *interner.value(head));
    assert_eq!(original.original.node, *interner.value(head));
    assert_eq!(
        original.original.kind,
        purrdf_core::ListErrorKind::MissingRest
    );
    assert_eq!(original.original.members, [interner.value(member).clone()]);
    assert!(
        matches!(ce.into_syntax(), Err(EntailError::OwlSyntax(original))
            if matches!(original.as_ref(), crate::OwlSyntaxError::IncompleteCapture))
    );
}

#[test]
fn native_syntax_distinguishes_completed_empty_body_from_missing_capture() {
    let mut f = Fixture::default();
    let a = f.name("a");
    let b = f.name("b");
    let p = f.name("p");
    f.builder.push_quad(a, p, b, None).unwrap();
    let kb = Kb::from_dataset(&f.freeze()).unwrap();
    assert_eq!(kb.inputs.syntax(Input::Role(0)), Some(Capture::Empty));
    assert_eq!(Kb::empty().inputs.syntax(Input::Role(0)), None);
}

#[test]
fn native_syntax_retains_inverse_cardinality_and_datatype_facet_reads() {
    use purrdf_core::TermValue;
    use purrdf_xsd::datatype::{XSD_INTEGER, XSD_MIN_INCLUSIVE};
    let mut f = Fixture::default();
    let class = f.name("A");
    let filler = f.name("F");
    let property = f.name("p");
    let inverse = f.blank();
    f.edge(inverse, owl::INVERSE_OF, property);
    f.bounds(class, inverse, filler, 2, true, false);
    let range = f.blank();
    let facet = f.blank();
    let integer = f.builder.intern_iri(XSD_INTEGER).unwrap();
    let value = crate::interner::intern_into(
        &mut f.builder,
        &TermValue::Literal {
            lexical_form: "-0002".to_owned(),
            datatype: XSD_INTEGER.to_owned(),
            language: None,
            direction: None,
        },
    )
    .unwrap();
    f.edge(range, owl::ON_DATATYPE, integer);
    f.edge(facet, XSD_MIN_INCLUSIVE, value);
    let list = f.list(&[facet]);
    f.edge(range, owl::WITH_RESTRICTIONS, list);
    let data = f.name("data");
    let datatype_property = f.builder.intern_iri(owl::DATATYPE_PROPERTY).unwrap();
    f.edge(data, rdf::TYPE, datatype_property);
    f.some(class, data, range);
    let kb = Kb::from_dataset(&f.freeze()).unwrap();
    let roots: Vec<_> = (0..kb.tbox.len())
        .filter_map(|index| kb.syntax.tbox_root(index).unwrap().root())
        .collect();
    let selected = kb.syntax.selected(&roots, None).unwrap();
    let reads: Vec<_> = selected
        .iter()
        .flat_map(|&id| kb.syntax.nodes[id].reads.iter())
        .collect();
    for wanted in [
        owl::INVERSE_OF,
        owl::MIN_QUALIFIED_CARDINALITY,
        owl::ON_DATATYPE,
        owl::WITH_RESTRICTIONS,
        XSD_MIN_INCLUSIVE,
    ] {
        assert!(reads.iter().any(|read| matches!(read, Read::Triple(_, p, _) if matches!(kb.interner.value(*p), TermValue::Iri(iri) if iri == wanted))), "missing actual read {wanted}");
    }
    assert!(reads.iter().any(|read| matches!(read, Read::Triple(_, _, object) if matches!(kb.interner.value(*object), TermValue::Literal { lexical_form, .. } if lexical_form == "-0002"))));
}

#[test]
fn native_syntax_selects_finite_expression_cycles_and_preserves_stopping() {
    let mut f = Fixture::default();
    let class = f.name("Cycle");
    f.edge(class, owl::COMPLEMENT_OF, class);
    f.instance("i", class);
    let kb = Kb::from_dataset(&f.freeze()).unwrap();
    let root = kb.inputs.syntax(Input::Type(0)).unwrap().root().unwrap();
    let selected = kb.syntax.selected(&[root], None).unwrap();
    assert_eq!(selected.len(), 1);
    assert!(kb.syntax.nodes[root].reads.contains(&Read::Child(root)));
    #[derive(Debug)]
    struct Stop;
    impl purrdf_datalog::StopSignal for Stop {
        fn stopped(&self) -> bool {
            true
        }
    }
    assert_eq!(
        kb.syntax.selected(&[root], Some(&Stop)),
        Err(SelectionError::Stopped)
    );
}
