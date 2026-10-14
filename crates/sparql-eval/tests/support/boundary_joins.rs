// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Scaled indexed-join fixtures shared by integration tests and planner benchmarks.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral};
use purrdf_iri::vocab::rdf::TYPE;
use purrdf_xsd::datatype::{XSD_BOOLEAN, XSD_DOUBLE, XSD_INTEGER};

use super::EX;

/// The intermediate-cell allowance shared by every equivalent query form.
pub const CELLS: u64 = 8_388_608;

/// Three boundary-sensitive queries paired with equivalent selective forms.
pub const QUERIES: [(&str, &str, &str, usize); 3] = [
    (
        "after_optional",
        "SELECT ?item WHERE { ?item a ex:Item . ?p1 ex:of ?item . ?p2 ex:of ?item . \
         OPTIONAL { ?p3 ex:of ?item . ?p3 ex:num 3 . ?p3 ex:val ?v3 } \
         ?p1 ex:num 1 . ?p1 ex:val ?v1 . ?p2 ex:num 2 . ?p2 ex:val ?v2 . \
         FILTER (?v1 < 0.01 || ?v2 != 0) }",
        "SELECT ?item WHERE { ?item a ex:Item . ?p1 ex:of ?item . ?p2 ex:of ?item . \
         ?p1 ex:num 1 . ?p1 ex:val ?v1 . ?p2 ex:num 2 . ?p2 ex:val ?v2 . \
         OPTIONAL { ?p3 ex:of ?item . ?p3 ex:num 3 . ?p3 ex:val ?v3 } \
         FILTER (?v1 < 0.01 || ?v2 != 0) }",
        3_510,
    ),
    (
        "after_bind",
        "SELECT DISTINCT ?item ?copy WHERE { ?item a ex:Item . ?p1 ex:of ?item . \
         ?p2 ex:of ?item . BIND (?item AS ?copy) ?p1 ex:num 1 . ?p2 ex:num 2 . }",
        "SELECT DISTINCT ?item ?copy WHERE { ?item a ex:Item . ?p1 ex:of ?item . \
         ?p2 ex:of ?item . ?p1 ex:num 1 . ?p2 ex:num 2 . BIND (?item AS ?copy) }",
        4_000,
    ),
    (
        "optional_body",
        "SELECT ?a ?flag WHERE { ?a a ex:Anchor . \
         OPTIONAL { ?a ex:at ?n . ?m ex:at ?n . ?m ex:of ?dev . ?dev ex:flag ?flag } }",
        "SELECT ?a ?flag WHERE { ?a a ex:Anchor . \
         OPTIONAL { ?a a ex:Anchor . ?a ex:at ?n . ?m ex:at ?n . \
         ?m ex:of ?dev . ?dev ex:flag ?flag } }",
        1_000,
    ),
];

/// A query under the common example namespace.
pub fn query(body: &str) -> String {
    format!("PREFIX ex: <{EX}> {body}")
}

/// 4,000 items with two numbered parts and a third for every fourth item,
/// plus 1,000 nodes with 100 members each and ten selective anchors.
pub fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let rdf_type = builder.intern_iri(TYPE).unwrap();
    let item_type = builder.intern_iri(&format!("{EX}Item")).unwrap();
    let part_type = builder.intern_iri(&format!("{EX}Part")).unwrap();
    let anchor_type = builder.intern_iri(&format!("{EX}Anchor")).unwrap();
    let of = builder.intern_iri(&format!("{EX}of")).unwrap();
    let num = builder.intern_iri(&format!("{EX}num")).unwrap();
    let val = builder.intern_iri(&format!("{EX}val")).unwrap();
    let at = builder.intern_iri(&format!("{EX}at")).unwrap();
    let flag = builder.intern_iri(&format!("{EX}flag")).unwrap();
    let numbers = [1, 2, 3].map(|number| {
        builder
            .intern_literal(RdfLiteral::typed(number.to_string(), XSD_INTEGER))
            .unwrap()
    });
    let values = ["0", "1", "0.001", "0.1"].map(|value| {
        builder
            .intern_literal(RdfLiteral::typed(value.to_owned(), XSD_DOUBLE))
            .unwrap()
    });
    let flags = ["false", "true"].map(|value| {
        builder
            .intern_literal(RdfLiteral::typed(value.to_owned(), XSD_BOOLEAN))
            .unwrap()
    });
    for index in 0..4_000 {
        let item = builder.intern_iri(&format!("{EX}item{index}")).unwrap();
        builder.push_quad(item, rdf_type, item_type, None).unwrap();
        let count = if index % 4 == 0 { 3 } else { 2 };
        for (part_index, &number) in numbers.iter().enumerate().take(count) {
            let part = builder
                .intern_iri(&format!("{EX}part{index}_{part_index}"))
                .unwrap();
            let value = match part_index {
                0 => values[if index < 80 { 2 } else { 3 }],
                1 => values[usize::from(index % 8 != 0)],
                _ => values[1],
            };
            builder.push_quad(part, rdf_type, part_type, None).unwrap();
            builder.push_quad(part, of, item, None).unwrap();
            builder.push_quad(part, num, number, None).unwrap();
            builder.push_quad(part, val, value, None).unwrap();
        }
    }
    for index in 0..1_000 {
        let node = builder.intern_iri(&format!("{EX}node{index}")).unwrap();
        if index < 10 {
            let anchor = builder.intern_iri(&format!("{EX}anchor{index}")).unwrap();
            builder
                .push_quad(anchor, rdf_type, anchor_type, None)
                .unwrap();
            builder.push_quad(anchor, at, node, None).unwrap();
        }
        for member_index in 0..100 {
            let member = builder
                .intern_iri(&format!("{EX}member{index}_{member_index}"))
                .unwrap();
            let device = builder
                .intern_iri(&format!("{EX}device{index}_{member_index}"))
                .unwrap();
            builder.push_quad(member, at, node, None).unwrap();
            builder.push_quad(member, of, device, None).unwrap();
            builder
                .push_quad(device, flag, flags[member_index % 2], None)
                .unwrap();
        }
    }
    builder
        .freeze()
        .expect("scaled boundary-join fixture freezes")
}
