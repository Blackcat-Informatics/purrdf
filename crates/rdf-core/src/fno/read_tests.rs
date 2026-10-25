// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Both graph laws and adjacent strict-profile refusals.

use super::{
    FNO, FNOM, FnoCatalog, FnoReadError, RDF_TYPE, RDFS_LABEL, tests::sample_catalog, to_quads,
};
use crate::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfQuad, RdfTerm, TermValue};
use purrdf_iri::vocab::rdf::{FIRST, REST};
use std::sync::Arc;

fn builder(rows: &[RdfQuad]) -> RdfDatasetBuilder {
    let mut builder = RdfDatasetBuilder::new();
    for row in rows {
        builder.push_owned_quad(row).unwrap();
    }
    builder
}

fn dataset(rows: &[RdfQuad]) -> Arc<RdfDataset> {
    builder(rows).freeze().unwrap()
}

fn roundtrip(catalog: &FnoCatalog) {
    let source = dataset(&to_quads(catalog));
    let read = FnoCatalog::from_dataset(&source, &catalog.as_input().ontology_iri).unwrap();
    assert_eq!(&read, catalog);
    let emitted = dataset(&to_quads(&read));
    assert!(crate::datasets_isomorphic(&source, &emitted).unwrap());
}

#[test]
fn admitted_catalogs_preserve_native_identity_and_both_graph_laws() {
    let base = sample_catalog();
    roundtrip(&base);
    let mut input = base.as_input().clone();
    input.functions[0].expects.reverse();
    input.functions[0]
        .kind_types
        .push(super::OWL_ONTOLOGY.to_owned());
    input.functions[0].label = TermValue::Literal {
        lexical_form: "مرحبا".to_owned(),
        datatype: purrdf_iri::vocab::rdf::DIR_LANG_STRING.to_owned(),
        language: Some("ar".to_owned()),
        direction: Some(crate::RdfTextDirection::Rtl),
    };
    input.functions[0].output.description =
        Some(TermValue::typed_literal("native", "urn:test:Text"));
    input.mappings[0].node.label = "a-b".to_owned();
    input.mappings[0].node.scope = BlankScope(3);
    let mut mapping = input.mappings[0].clone();
    mapping.node.label = "a_b".to_owned();
    mapping.node.scope = BlankScope(4);
    input.mappings.push(mapping);
    let mut unused = input.params[0].clone();
    unused.iri = "urn:test:unused-parameter".to_owned();
    input.params.push(unused);
    let mut unused = input.implementations[0].clone();
    unused.iri = "urn:test:unused-implementation".to_owned();
    input.implementations.push(unused);
    roundtrip(&FnoCatalog::try_new(input).unwrap());

    let mut input = base.as_input().clone();
    input.functions[0].expects.clear();
    input.mappings[0].parameter_mappings.clear();
    let mut other = input.functions[0].clone();
    other.iri = "urn:test:another-function".to_owned();
    input.functions.push(other);
    roundtrip(&FnoCatalog::try_new(input).unwrap());

    let mut input = base.as_input().clone();
    let param = &input.params[0];
    input.functions[0].output.iri.clone_from(&param.iri);
    input.functions[0].output.r#type.clone_from(&param.r#type);
    input.functions[0]
        .output
        .predicate
        .clone_from(&param.predicate);
    input.functions[0].output.label = None;
    input.functions[0].output.description = None;
    input.mappings[0].return_mappings[0]
        .function_output
        .clone_from(&param.iri);
    roundtrip(&FnoCatalog::try_new(input).unwrap());
}

#[test]
fn strict_reader_refuses_unreproducible_rows_and_retains_original_operands() {
    use purrdf_testkit::error_capture::Capture;
    let catalog = sample_catalog();
    let original = to_quads(&catalog);
    for selection in 0..18 {
        let mut rows = original.clone();
        match selection {
            0 => rows.push(RdfQuad::new(
                rows[0].subject.clone(),
                format!("{FNO}unknown"),
                RdfTerm::iri("urn:test:unknown"),
            )),
            1 => {
                let row = rows
                    .iter()
                    .find(|row| row.predicate == format!("{FNO}predicate"))
                    .unwrap()
                    .clone();
                rows.push(RdfQuad::new(
                    row.subject,
                    row.predicate,
                    RdfTerm::iri("urn:test:second-predicate"),
                ));
            }
            2 => {
                let row = rows
                    .iter_mut()
                    .find(|row| row.predicate == RDFS_LABEL)
                    .unwrap();
                row.object = RdfTerm::iri("urn:test:not-literal");
            }
            3 => {
                let row = rows
                    .iter_mut()
                    .find(|row| row.predicate == format!("{FNOM}implementationProperty"))
                    .unwrap();
                row.object = TermValue::lang_literal("binding", "en")
                    .into_rdf_term()
                    .unwrap();
            }
            4 => {
                let row = rows
                    .iter_mut()
                    .find(|row| row.predicate == format!("{FNO}required"))
                    .unwrap();
                row.object = TermValue::typed_literal("1", purrdf_xsd::datatype::XSD_BOOLEAN)
                    .into_rdf_term()
                    .unwrap();
            }
            5 => {
                let at = rows.iter().position(|row| row.predicate == REST).unwrap();
                rows.remove(at);
            }
            6 => {
                let row = rows
                    .iter_mut()
                    .find(|row| row.predicate == format!("{FNO}expects"))
                    .unwrap();
                row.object = RdfTerm::iri("urn:test:not-a-list");
            }
            7 => {
                let head = rows
                    .iter()
                    .find(|row| row.predicate == format!("{FNO}returns"))
                    .unwrap()
                    .object
                    .clone();
                rows.iter_mut()
                    .find(|row| row.predicate == format!("{FNO}expects"))
                    .unwrap()
                    .object = head;
            }
            8 => {
                let head = rows
                    .iter()
                    .find(|row| row.predicate == FIRST)
                    .unwrap()
                    .subject
                    .clone();
                for row in &mut rows {
                    if row.subject == head {
                        row.subject = RdfTerm::iri("urn:test:named-cell");
                    }
                    if row.object == head {
                        row.object = RdfTerm::iri("urn:test:named-cell");
                    }
                }
            }
            9 => {
                let head = rows
                    .iter()
                    .find(|row| row.predicate == FIRST)
                    .unwrap()
                    .subject
                    .clone();
                rows.push(RdfQuad::new(
                    head,
                    RDFS_LABEL,
                    TermValue::simple_literal("extra cell fact")
                        .into_rdf_term()
                        .unwrap(),
                ));
            }
            10 => rows.push(RdfQuad::new(
                RdfTerm::iri("urn:test:orphan"),
                RDF_TYPE,
                RdfTerm::iri(format!("{FNO}Output")),
            )),
            11 => {
                let row = rows.iter_mut().find(|row| row.predicate == FIRST).unwrap();
                row.object = RdfTerm::iri("urn:test:unresolved-parameter");
            }
            12 => rows[0].graph_name = Some(RdfTerm::iri("urn:test:graph")),
            13 => {
                let at = rows
                    .iter()
                    .position(|row| row.predicate == format!("{FNO}type"))
                    .unwrap();
                rows.remove(at);
            }
            14 => {
                let row = rows.iter_mut().find(|row| row.predicate == REST).unwrap();
                row.object = row.subject.clone();
            }
            15 => {
                let row = rows.iter_mut().find(|row| row.predicate == FIRST).unwrap();
                row.object = TermValue::simple_literal("not an IRI member")
                    .into_rdf_term()
                    .unwrap();
            }
            16 => {
                let head = rows
                    .iter()
                    .find(|row| row.predicate == format!("{FNO}expects"))
                    .unwrap()
                    .object
                    .clone();
                rows.iter_mut()
                    .find(|row| row.predicate == format!("{FNO}returns"))
                    .unwrap()
                    .object = head;
            }
            17 => {
                let at = rows
                    .iter()
                    .position(|row| {
                        row.predicate == RDF_TYPE
                            && row.object == RdfTerm::iri(format!("{FNO}Output"))
                    })
                    .unwrap();
                rows.remove(at);
            }
            _ => unreachable!(),
        }
        let error = {
            let source = dataset(&rows);
            FnoCatalog::from_dataset(&source, &catalog.as_input().ontology_iri).unwrap_err()
        };
        let captured = Capture::read(error.as_ref());
        let expected = match selection {
            0 | 9 | 10 => "fno-read-statement",
            1..=4 | 13 | 15 | 17 => "fno-read-field",
            5 | 6 | 14 => "fno-read-list",
            7 | 8 | 12 => "fno-read-statement",
            11 => "fno-read-admission",
            16 => "fno-read-returns",
            _ => unreachable!(),
        };
        assert_eq!(captured.code, expected, "selection {selection}: {error}");
        match error.as_ref() {
            FnoReadError::Field {
                subject,
                predicate,
                objects,
                expected,
            } => {
                if selection == 17 {
                    assert_eq!(
                        *expected,
                        super::FnoFieldKind::Role {
                            class: format!("{FNO}Output")
                        }
                    );
                    assert_eq!(captured.child("expected").code, "fno-field-role");
                }
                let original_objects: std::collections::BTreeSet<_> = rows
                    .iter()
                    .filter(|row| {
                        TermValue::from_rdf_term(&row.subject) == *subject
                            && row.predicate == *predicate
                    })
                    .map(|row| TermValue::from_rdf_term(&row.object))
                    .collect();
                assert_eq!(
                    objects
                        .iter()
                        .cloned()
                        .collect::<std::collections::BTreeSet<_>>(),
                    original_objects
                );
                assert!(
                    rows.iter()
                        .any(|row| TermValue::from_rdf_term(&row.subject) == *subject
                            && (row.predicate == *predicate || matches!(selection, 13 | 17)))
                );
            }
            FnoReadError::List { cause, .. } => {
                assert!(
                    std::error::Error::source(error.as_ref())
                        .unwrap()
                        .downcast_ref::<crate::ListError<TermValue>>()
                        .is_some()
                );
                assert!(
                    rows.iter()
                        .any(|row| TermValue::from_rdf_term(&row.subject) == cause.node)
                        || selection == 6
                );
            }
            FnoReadError::Statement { row, .. } => assert!(rows.iter().any(|original| {
                TermValue::from_rdf_term(&original.subject) == row.s
                    && TermValue::iri(&original.predicate) == row.p
                    && TermValue::from_rdf_term(&original.object) == row.o
            })),
            FnoReadError::Returns {
                subject,
                predicate,
                head,
                members,
            } => {
                assert_eq!(predicate, &format!("{FNO}returns"));
                assert_eq!(members.len(), 2);
                assert!(
                    rows.iter()
                        .any(|row| TermValue::from_rdf_term(&row.subject) == *subject
                            && row.predicate == *predicate
                            && TermValue::from_rdf_term(&row.object) == *head)
                );
            }
            FnoReadError::Admission(cause) => assert!(
                cause.input.functions[0]
                    .expects
                    .iter()
                    .any(|target| target == "urn:test:unresolved-parameter")
            ),
            _ => unreachable!("the selected profile boundary has a native refusal"),
        }
        roundtrip(&catalog);
    }
}

#[test]
fn document_empty_graph_and_foreign_identifier_boundaries_have_healthy_neighbors() {
    let catalog = sample_catalog();
    let rows = to_quads(&catalog);
    let source = dataset(&rows);
    assert!(matches!(
        *FnoCatalog::from_dataset(&source, "urn:test:unselected").unwrap_err(),
        FnoReadError::Document { .. }
    ));
    let mut builder = builder(&rows);
    let graph = builder.intern_iri("urn:test:empty").unwrap();
    builder.declare_named_graph(graph).unwrap();
    let source = builder.freeze().unwrap();
    assert!(matches!(
        *FnoCatalog::from_dataset(&source, &catalog.as_input().ontology_iri).unwrap_err(),
        FnoReadError::Graph { .. }
    ));
    let source = dataset(&rows);
    let datatype = source
        .as_ref()
        .term_id_by_value(&catalog.as_input().doc_label)
        .unwrap();
    let mut foreign = crate::term_fixture::ForeignDatatypeView {
        inner: source,
        datatype,
        foreign: true,
    };
    assert!(matches!(
        *FnoCatalog::from_dataset(&foreign, &catalog.as_input().ontology_iri).unwrap_err(),
        FnoReadError::Term(crate::TermLookupError::ForeignId)
    ));
    foreign.foreign = false;
    assert_eq!(
        FnoCatalog::from_dataset(&foreign, &catalog.as_input().ontology_iri).unwrap(),
        catalog
    );
    roundtrip(&catalog);
}

#[test]
fn source_failures_and_rdf_statement_layers_never_publish_a_partial_catalog() {
    use crate::DatasetView;
    use crate::term_fixture::BudgetedView;
    let catalog = sample_catalog();
    let rows = to_quads(&catalog);
    let source = dataset(&rows);
    let row_count = source.quads().count();
    let document = TermValue::iri(&catalog.as_input().document_iri);
    for cut in 0..=row_count {
        let before = cut == row_count;
        let faulty = BudgetedView::new(Arc::clone(&source), if before { 0 } else { cut });
        if before {
            assert_eq!(faulty.quads().count(), 0);
        }
        let error =
            FnoCatalog::from_dataset(&faulty, &catalog.as_input().ontology_iri).unwrap_err();
        let FnoReadError::Source(cause) = error.as_ref() else {
            panic!("cut {cut}, prior fault {before}: {error}");
        };
        let native = std::error::Error::source(error.as_ref())
            .unwrap()
            .downcast_ref::<crate::term_fixture::ProbeFault>()
            .unwrap();
        assert!(std::ptr::eq(native, cause));
        let cached = faulty.term_id_by_value(&document).unwrap().unwrap();
        assert!(
            faulty
                .with_term(cached, |term| matches!(term, crate::TermRef::Iri(iri)
            if iri == catalog.as_input().document_iri))
                .unwrap()
        );
    }
    let healthy = BudgetedView::new(Arc::clone(&source), usize::MAX);
    assert_eq!(
        FnoCatalog::from_dataset(&healthy, &catalog.as_input().ontology_iri).unwrap(),
        catalog
    );
    for reifier in [false, true] {
        let mut builder = builder(&rows);
        let s = builder.intern_iri("urn:test:reifier").unwrap();
        let p = builder.intern_iri("urn:test:predicate").unwrap();
        let o = builder.intern_iri("urn:test:object").unwrap();
        if reifier {
            let triple = builder.intern_triple(s, p, o).unwrap();
            builder.push_reifier(s, triple).unwrap();
        } else {
            builder.push_annotation(s, p, o).unwrap();
        }
        let source = builder.freeze().unwrap();
        let error =
            FnoCatalog::from_dataset(&source, &catalog.as_input().ontology_iri).unwrap_err();
        let FnoReadError::Statement { row, reason } = error.as_ref() else {
            panic!("{error}");
        };
        assert_eq!(
            *reason,
            if reifier {
                super::FnoStructureKind::Reifier
            } else {
                super::FnoStructureKind::Annotation
            }
        );
        assert_eq!(row.s, TermValue::iri("urn:test:reifier"));
        roundtrip(&catalog);
    }
}
