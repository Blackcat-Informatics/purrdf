// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exercise the binding's actual interpreter-free load ingress. No extension-module
//! symbols or alternate engine are linked into this native boundary control.

#[path = "../src/py_store/load.rs"]
mod load;

use purrdf_core::ir::MutableDataset;
use purrdf_core::{
    BlankScope, DatasetMut, RdfDatasetBuilder, RdfQuad, RdfTerm, RdfTriple, TermValue,
};
use purrdf_lex::allocation::{CapacityError, CapacitySpace, StorageError};

fn empty() -> MutableDataset {
    MutableDataset::new(RdfDatasetBuilder::new().freeze().unwrap())
}

#[test]
fn load_retains_original_lexical_and_declaration_error_boundaries() {
    let mut dataset = empty();
    let mut next = 1;
    let invalid = RdfQuad::new(
        RdfTerm::iri("relative"),
        "http://example.org/p",
        RdfTerm::iri("http://example.org/o"),
    );
    assert!(matches!(
        load::insert_loaded_document(&mut dataset, &[invalid], vec![], &mut next),
        Err(load::LoadError::Insert(purrdf::iri::IriReadError::Lexical(
            _
        )))
    ));
    let error = load::insert_loaded_document(
        &mut dataset,
        &[],
        vec![TermValue::iri("relative")],
        &mut next,
    )
    .unwrap_err();
    let load::LoadError::Declaration(diagnostic) = error else {
        panic!("expected the original declaration diagnostic");
    };
    assert!(diagnostic.storage_error().is_none());
    assert_eq!(next, 1);
}

#[test]
fn last_scope_and_repeated_refusal_preserve_every_blank_ingress() {
    let plain = RdfQuad::new(
        RdfTerm::iri("http://example.org/s"),
        "http://example.org/p",
        RdfTerm::iri("http://example.org/o"),
    );
    let direct = RdfQuad::new(
        RdfTerm::blank_node("same"),
        "http://example.org/p",
        RdfTerm::blank_node("same"),
    );
    let nested = RdfQuad::new(
        RdfTerm::iri("http://example.org/s"),
        "http://example.org/p",
        RdfTerm::triple(RdfTriple::new(
            RdfTerm::blank_node("same"),
            "http://example.org/p",
            RdfTerm::blank_node("same"),
        )),
    );
    let composite = purrdf::parse_dataset(
        br#"<http://example.org/s> <http://example.org/p>
            "[_:same, <<( _:same <http://example.org/p> _:other )>>]"^^
            <http://w3id.org/awslabs/neptune/SPARQL-CDTs/List> ."#,
        "text/turtle",
        None,
    )
    .unwrap();
    let composite = purrdf::flat_rdf_quads_from_dataset(&composite);
    for (quads, declarations) in [
        (vec![direct], vec![]),
        (vec![nested], vec![]),
        (composite, vec![]),
        (
            vec![plain.clone().in_graph(RdfTerm::blank_node("graph"))],
            vec![],
        ),
        (vec![], vec![TermValue::blank("empty-graph")]),
    ] {
        let mut dataset = empty();
        let mut next = u64::from(u32::MAX);
        load::insert_loaded_document(&mut dataset, &quads, declarations.clone(), &mut next)
            .unwrap();
        assert_eq!(next, u64::from(u32::MAX) + 1);
        for quad in &quads {
            assert!(
                dataset.contains(&load::rdf_quad_to_values_scoped(quad, BlankScope(u32::MAX),))
            );
        }
        let before = dataset.freeze().unwrap();
        let mut blanks = 0;
        for id in before
            .quads()
            .flat_map(|row| [Some(row.s), Some(row.o), row.g].into_iter().flatten())
            .chain(before.named_graphs())
        {
            let _ = before.term_value(id).visit_blank_identities(|_, scope| {
                assert_eq!(scope, BlankScope(u32::MAX));
                blanks += 1;
                std::ops::ControlFlow::<()>::Continue(())
            });
        }
        assert!(
            blanks > 0,
            "the stored document must retain its scoped blanks"
        );
        let expected = StorageError::Capacity(CapacityError::new(
            CapacitySpace::ImportBlankScopes,
            u128::from(u32::MAX) + 1,
            u128::from(u32::MAX),
        ));
        // A new blank-free prefix must not publish before a later blank term or
        // declaration discovers the exhausted scope. Exercise every shape above,
        // including a declaration-only blank after this ordinary quad.
        let prefix = RdfQuad::new(
            RdfTerm::iri("http://example.org/unpublished-prefix"),
            "http://example.org/p",
            RdfTerm::iri("http://example.org/o"),
        );
        let mut refused_quads = vec![prefix.clone()];
        refused_quads.extend_from_slice(&quads);
        for _ in 0..3 {
            let error = load::insert_loaded_document(
                &mut dataset,
                &refused_quads,
                declarations.clone(),
                &mut next,
            )
            .unwrap_err();
            let load::LoadError::Insert(purrdf::iri::IriReadError::Storage(cause)) = error else {
                panic!("expected the original mutation storage cause");
            };
            assert_eq!(cause, expected);
            assert!(!dataset.contains(&load::rdf_quad_to_values_scoped(
                &prefix,
                BlankScope::DEFAULT,
            )));
            let diagnostic = purrdf_core::RdfDiagnostic::from(cause);
            assert_eq!(diagnostic.code, "rdf-capacity-exhausted");
            assert_eq!(next, u64::from(u32::MAX) + 1);
            let after = dataset.freeze().unwrap();
            assert_eq!(
                after.quads().collect::<Vec<_>>(),
                before.quads().collect::<Vec<_>>()
            );
            assert_eq!(
                after.named_graphs().collect::<Vec<_>>(),
                before.named_graphs().collect::<Vec<_>>()
            );
        }
        load::insert_loaded_document(&mut dataset, &[], vec![], &mut next).unwrap();
        load::insert_loaded_document(
            &mut dataset,
            std::slice::from_ref(&plain),
            vec![TermValue::iri("http://example.org/empty")],
            &mut next,
        )
        .unwrap();
        assert_eq!(next, u64::from(u32::MAX) + 1);
        assert!(dataset.contains(&load::rdf_quad_to_values_scoped(
            &plain,
            BlankScope::DEFAULT
        )));
        // A neighboring fresh owner can still load the same blank-bearing document.
        let mut healthy = empty();
        let mut healthy_next = 1;
        load::insert_loaded_document(&mut healthy, &quads, declarations, &mut healthy_next)
            .unwrap();
        assert_eq!(healthy_next, 2);
    }
}
