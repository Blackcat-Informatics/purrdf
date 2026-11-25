// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Correlated filter results and resident pattern binding work.

use super::tests::run_rows;
use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
use purrdf_xsd::datatype::XSD_INTEGER;

std::thread_local! {
    /// Position unifications counted only while the regression is active.
    static BGP_PATTERN_BINDINGS: std::cell::Cell<Option<u64>> =
        const { std::cell::Cell::new(None) };
}

pub(crate) fn note_pattern_binding() {
    BGP_PATTERN_BINDINGS.with(|counter| {
        if let Some(count) = counter.get() {
            counter.set(Some(count + 1));
        }
    });
}

/// Restrict the parent lookup before evaluating the correlated score
/// filter, rather than unifying the whole parent relation per subject.
#[test]
fn correlated_filter_not_exists_bounds_pattern_bindings() {
    const SUBJECTS: usize = 862;
    let mut builder = RdfDatasetBuilder::new();
    let score = builder.intern_iri("https://example.org/score").unwrap();
    let status = builder.intern_iri("https://example.org/status").unwrap();
    let parent = builder.intern_iri("https://example.org/parent").unwrap();
    let open = builder.intern_iri("https://example.org/open").unwrap();
    let low = builder
        .intern_literal(RdfLiteral::typed("1", XSD_INTEGER))
        .unwrap();
    let high = builder
        .intern_literal(RdfLiteral::typed("2", XSD_INTEGER))
        .unwrap();
    let mut previous = None;
    let mut expected = Vec::new();
    for index in 0..SUBJECTS {
        let iri = format!("https://example.org/subject/{index:04}");
        let subject = builder.intern_iri(&iri).unwrap();
        let high_score = matches!(index % 4, 0 | 3);
        let points = if high_score { high } else { low };
        builder.push_quad(subject, score, points, None).unwrap();
        builder.push_quad(subject, status, open, None).unwrap();
        if index % 2 == 1 {
            let other = previous.expect("the pair has a parent");
            builder.push_quad(subject, parent, other, None).unwrap();
        }
        if index % 4 != 1 {
            expected.push(vec![
                format!("<{iri}>"),
                if high_score { "2" } else { "1" }.to_owned(),
            ]);
        }
        previous = Some(subject);
    }
    let dataset = builder.freeze().expect("valid work fixture");
    let query = "PREFIX ex: <https://example.org/> \
        SELECT ?x ?s WHERE { ?x ex:score ?s ; ex:status ?st . \
        FILTER NOT EXISTS { ?x ex:parent ?p . ?p ex:score ?ps . FILTER(?ps > ?s) } }";
    assert!(
        BGP_PATTERN_BINDINGS
            .with(|counter| counter.replace(Some(0)))
            .is_none()
    );
    let rows = run_rows(&dataset, query, true);
    let bindings = BGP_PATTERN_BINDINGS
        .with(|counter| counter.replace(None))
        .expect("position unifications are counted");
    assert_eq!(rows, expected);
    assert!(
        (1..20_000).contains(&bindings),
        "{bindings} position unifications for {SUBJECTS} subjects"
    );
}
