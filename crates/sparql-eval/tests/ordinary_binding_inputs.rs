// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Physical relation seeds preserve bags and the interner's complete term identity.

mod support;

use purrdf_core::{
    RdfDatasetBuilder, RdfLiteral, ResourceDimension, SparqlEngine, SparqlRequest, SparqlResult,
};
use purrdf_sparql_eval::{
    EvalOptions, NativeSparqlEngine, PartialAnswers, QueryGovernors, QueryOptions,
};
use support::{EX, render_cell, row, sorted_rows};

/// A driver bag above the shared chunk cutoff exercises the actual ungoverned
/// fork. Duplicate/nullable drivers, carried computed terms and blank projection
/// retain the serial bag and its complete source order across four workers.
#[test]
fn ungoverned_optional_driver_chunks_preserve_bags_and_order() {
    const BLOCKS: usize = 512;
    let mut builder = RdfDatasetBuilder::new();
    let [a, b, p, q, v1, v2, v3, z1, z2, z3] =
        ["a", "b", "p", "q", "v1", "v2", "v3", "z1", "z2", "z3"]
            .map(|local| builder.intern_iri(&format!("{EX}{local}")));
    for (s, predicate, object) in [
        (a, p, v1),
        (a, p, v2),
        (b, p, v3),
        (v1, q, z1),
        (v2, q, z2),
        (v3, q, z3),
    ] {
        builder.push_quad(s, predicate, object, None);
    }
    let dataset = builder.freeze().expect("driver chunk fixture");
    let drivers = "ex:a ex:a ex:missing UNDEF ".repeat(BLOCKS);
    let query = format!(
        "PREFIX ex: <{EX}> SELECT ?x ?z ?carry WHERE {{ VALUES ?x {{ {drivers} }} \
         BIND(\"fresh\" AS ?carry) OPTIONAL {{ ?x ex:p _:middle . _:middle ex:q ?z }} }}"
    );
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("four-worker fork proof");
    let run = |force_sequential| {
        pool.install(|| {
            NativeSparqlEngine::new()
                .with_eval_options(EvalOptions {
                    force_sequential,
                    ..EvalOptions::default()
                })
                .query(
                    &dataset,
                    SparqlRequest {
                        query: &query,
                        base_iri: None,
                        substitutions: &[],
                    },
                )
                .expect("complete ungoverned OPTIONAL")
        })
    };
    let parallel = run(false);
    let sequential = run(true);
    let mut expected = Vec::new();
    for (x, z, copies) in [
        ("a", "z1", BLOCKS * 3),
        ("a", "z2", BLOCKS * 3),
        ("b", "z3", BLOCKS),
        ("missing", "UNBOUND", BLOCKS),
    ] {
        let x = format!("<{EX}{x}>");
        let z = if z == "UNBOUND" {
            z.into()
        } else {
            format!("<{EX}{z}>")
        };
        expected.extend((0..copies).map(|_| row(&[("x", &x), ("z", &z), ("carry", "fresh")])));
    }
    expected.sort();
    assert_eq!(sorted_rows(&parallel, render_cell), expected);
    assert_eq!(sorted_rows(&sequential, render_cell), expected);
    let (
        SparqlResult::Solutions {
            variables: parallel_variables,
            rows: parallel_rows,
            ..
        },
        SparqlResult::Solutions {
            variables: sequential_variables,
            rows: sequential_rows,
            ..
        },
    ) = (parallel, sequential)
    else {
        panic!("the fixture returns solutions");
    };
    assert_eq!(parallel_variables, sequential_variables);
    assert_eq!(parallel_rows, sequential_rows);
}

#[test]
fn duplicate_nullable_and_absent_drivers_keep_their_exact_join_bags() {
    let mut builder = RdfDatasetBuilder::new();
    let [a, b, p] = ["a", "b", "p"].map(|local| builder.intern_iri(&format!("{EX}{local}")));
    for (subject, value) in [(a, "1"), (a, "2"), (b, "3")] {
        let object =
            builder.intern_literal(RdfLiteral::typed(value, purrdf_xsd::datatype::XSD_INTEGER));
        builder.push_quad(subject, p, object, None);
    }
    let dataset = builder.freeze().expect("nullable bag fixture");
    let a = format!("<{EX}a>");
    let b = format!("<{EX}b>");
    let missing = format!("<{EX}missing>");
    for force_sequential in [false, true] {
        let engine = NativeSparqlEngine::new().with_eval_options(EvalOptions {
            force_sequential,
            ..EvalOptions::default()
        });
        for optional in [false, true] {
            let body = if optional {
                "OPTIONAL { ?x ex:p ?v }"
            } else {
                "?x ex:p ?v"
            };
            let query = format!(
                "PREFIX ex: <{EX}> SELECT ?x ?v ?mark WHERE {{ \
                 VALUES ?x {{ ex:a ex:a ex:missing UNDEF }} BIND(\"fresh\" AS ?mark) {body} }}"
            );
            let outcome = engine
                .query_governed(
                    &dataset,
                    SparqlRequest {
                        query: &query,
                        base_iri: None,
                        substitutions: &[],
                    },
                    QueryOptions::EMPTY,
                    &QueryGovernors::METERED.with_max_intermediate_cells(128),
                )
                .expect("native bag outcome");
            let result = outcome.into_complete().expect("bounded bag completes");
            let mut expected = Vec::new();
            for _ in 0..3 {
                for value in ["1", "2"] {
                    expected.push(row(&[("x", &a), ("v", value), ("mark", "fresh")]));
                }
            }
            expected.push(row(&[("x", &b), ("v", "3"), ("mark", "fresh")]));
            if optional {
                expected.push(row(&[("x", &missing), ("v", "UNBOUND"), ("mark", "fresh")]));
            }
            expected.sort();
            assert_eq!(
                sorted_rows(&result, render_cell),
                expected,
                "optional={optional}"
            );
        }
    }
}

#[test]
fn computed_existing_missing_and_quoted_slots_use_dataset_identity_before_scanning() {
    let mut builder = RdfDatasetBuilder::new();
    let [a, p, q, o, record, has, noise] = ["a", "p", "q", "o", "record", "has", "noise"]
        .map(|local| builder.intern_iri(&format!("{EX}{local}")));
    let value = builder.intern_literal(RdfLiteral::typed("1", purrdf_xsd::datatype::XSD_INTEGER));
    builder.push_quad(a, p, value, None);
    let quote = builder.intern_triple(a, q, o);
    builder.push_quad(record, has, quote, None);
    for index in 0..5_000 {
        let subject = builder.intern_iri(&format!("{EX}noise{index}"));
        builder.push_quad(subject, noise, o, None);
    }
    let dataset = builder.freeze().expect("term identity fixture");
    let cases = [
        (
            "?x ?v",
            "VALUES ?name { \"a\" \"missing\" } \
             BIND(IRI(CONCAT(\"http://example.org/\", ?name)) AS ?x) \
             BIND(\"fresh\" AS ?mark) ?x ex:p ?v",
            row(&[("x", &format!("<{EX}a>")), ("v", "1")]),
        ),
        (
            "?record",
            "VALUES ?x { <<( ex:a ex:q ex:o )>> <<( ex:missing ex:q ex:o )>> } \
             BIND(\"fresh\" AS ?mark) ?record ex:has ?x",
            row(&[("record", &format!("<{EX}record>"))]),
        ),
        (
            "?record",
            "VALUES ?s { ex:a ex:missing } BIND(\"fresh\" AS ?mark) \
             ?record ex:has <<( ?s ex:q ex:o )>>",
            row(&[("record", &format!("<{EX}record>"))]),
        ),
    ];
    for (projection, body, expected) in cases {
        let query = format!("PREFIX ex: <{EX}> SELECT {projection} WHERE {{ {body} }}");
        let outcome = NativeSparqlEngine::new()
            .query_governed(
                &dataset,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_max_intermediate_cells(128),
            )
            .expect("term identity outcome");
        assert!(
            outcome.evidence().consumed_in(ResourceDimension::Fuel) < 128,
            "{query}: {outcome:?}"
        );
        let result = outcome
            .into_complete()
            .expect("bounded term identity relation");
        assert_eq!(sorted_rows(&result, render_cell), vec![expected], "{query}");
    }
}

/// Repeated BGP occurrences share a layout while masks vary by driver. Every
/// fuel, cell and answer cut must keep a true subbag, including empty padding.
#[test]
fn retained_optional_preparation_preserves_masks_and_every_governor_cut() {
    let mut builder = RdfDatasetBuilder::new();
    let [a, b, p, q, v1, v2, v3, z1, z2, z3, record, has, o] = [
        "a", "b", "p", "q", "v1", "v2", "v3", "z1", "z2", "z3", "record", "has", "o",
    ]
    .map(|local| builder.intern_iri(&format!("{EX}{local}")));
    for (s, predicate, object) in [
        (a, p, v1),
        (a, p, v2),
        (b, p, v3),
        (v1, q, z1),
        (v2, q, z2),
        (v3, q, z3),
    ] {
        builder.push_quad(s, predicate, object, None);
    }
    let quote = builder.intern_triple(a, q, o);
    builder.push_quad(record, has, quote, None);
    let dataset = builder.freeze().expect("retained BGP fixture");
    let iri = |local| format!("<{EX}{local}>");
    let matches = |include_v| {
        let mut expected = Vec::new();
        for (x, v, z, copies) in [
            ("a", "v1", "z1", 3),
            ("a", "v2", "z2", 3),
            ("b", "v3", "z3", 2),
        ] {
            for _ in 0..copies {
                let mut fields = vec![("x", iri(x)), ("z", iri(z)), ("mark", "fresh".into())];
                if include_v {
                    fields.push(("v", iri(v)));
                }
                expected.push(row(&fields
                    .iter()
                    .map(|(k, v)| (*k, v.as_str()))
                    .collect::<Vec<_>>()));
            }
        }
        let mut fields = vec![
            ("x", iri("missing")),
            ("z", "UNBOUND".into()),
            ("mark", "fresh".into()),
        ];
        if include_v {
            fields.push(("v", "UNBOUND".into()));
        }
        expected.push(row(&fields
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect::<Vec<_>>()));
        expected
    };
    let drivers = "VALUES ?x { ex:a UNDEF ex:b ex:a ex:missing } BIND(\"fresh\" AS ?mark)";
    let missing_ground = ["a", "UNBOUND", "b", "a", "missing"]
        .map(|x| {
            row(&[
                ("x", &if x == "UNBOUND" { x.into() } else { iri(x) }),
                ("v", "UNBOUND"),
                ("z", "UNBOUND"),
                ("mark", "fresh"),
            ])
        })
        .to_vec();
    let cases = [
        (
            "?x ?v ?z ?mark",
            format!("{drivers} OPTIONAL {{ ?x ex:p ?v . ?v ex:q ?z }}"),
            matches(true),
        ),
        (
            "?x ?z ?mark",
            format!("{drivers} OPTIONAL {{ ?x ex:p _:value . _:value ex:q ?z }}"),
            matches(false),
        ),
        (
            "?x ?v ?z ?mark",
            format!("{drivers} OPTIONAL {{ ?x ex:absent ?v . ?v ex:q ?z }}"),
            missing_ground,
        ),
        (
            "?x ?record ?mark",
            "VALUES ?x { ex:a ex:missing UNDEF } BIND(\"fresh\" AS ?mark) \
             OPTIONAL { ?record ex:has <<( ?x ex:q ex:o )>> }"
                .into(),
            vec![
                row(&[
                    ("x", &iri("a")),
                    ("record", &iri("record")),
                    ("mark", "fresh"),
                ]),
                row(&[
                    ("x", &iri("a")),
                    ("record", &iri("record")),
                    ("mark", "fresh"),
                ]),
                row(&[
                    ("x", &iri("missing")),
                    ("record", "UNBOUND"),
                    ("mark", "fresh"),
                ]),
            ],
        ),
    ];
    for force_sequential in [false, true] {
        let engine = NativeSparqlEngine::new().with_eval_options(EvalOptions {
            force_sequential,
            ..EvalOptions::default()
        });
        for (projection, body, expected) in &cases {
            let query = format!("PREFIX ex: <{EX}> SELECT {projection} WHERE {{ {body} }}");
            let request = SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            };
            let measured = engine
                .query_governed(
                    &dataset,
                    request,
                    QueryOptions::EMPTY,
                    &QueryGovernors::METERED,
                )
                .expect("measure retained OPTIONAL");
            let fuel = measured.evidence().consumed_in(ResourceDimension::Fuel);
            let cells = measured
                .evidence()
                .consumed_in(ResourceDimension::IntermediateCells);
            let mut expected = expected.clone();
            expected.sort();
            assert_eq!(
                sorted_rows(
                    &measured
                        .into_complete()
                        .expect("complete retained OPTIONAL"),
                    render_cell
                ),
                expected,
                "{query}",
            );
            let allowances = (0..=fuel)
                .map(|cap| QueryGovernors::METERED.with_fuel(cap))
                .chain(
                    (0..=cells).map(|cap| QueryGovernors::METERED.with_max_intermediate_cells(cap)),
                )
                .chain(
                    (0..=expected.len() as u64)
                        .map(|cap| QueryGovernors::METERED.with_max_answers(cap)),
                );
            for governors in allowances {
                let outcome = engine
                    .query_governed(&dataset, request, QueryOptions::EMPTY, &governors)
                    .expect("each cut returns a typed outcome");
                let actual = if let Some(exhausted) = outcome.exhausted() {
                    let PartialAnswers::Certain(partial) = &exhausted.partial else {
                        panic!("{query}: monotone OPTIONAL cut {governors:?}: {exhausted:?}");
                    };
                    sorted_rows(partial.result(), render_cell)
                } else {
                    sorted_rows(&outcome.into_complete().expect("uncut result"), render_cell)
                };
                for group in actual.chunk_by(|a, b| a == b) {
                    let before = expected.partition_point(|candidate| candidate < &group[0]);
                    let through = expected.partition_point(|candidate| candidate <= &group[0]);
                    assert!(
                        group.len() <= through - before,
                        "{query}: cut {governors:?} fabricated multiplicity {group:?}"
                    );
                }
            }
        }
    }
}
