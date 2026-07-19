// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public-surface round trips for deterministic generic and Neo4j LPG CSV packages.

use purrdf_rdf::{
    LpgConfig, LpgExecutionLimits, LpgScope, ProjectionLimits, RdfDatasetBuilder, RdfLiteral,
    datasets_isomorphic, lift_lpg, project_lpg_csv, project_neo4j_csv, read_lpg_csv,
    read_neo4j_csv, write_lpg_csv, write_neo4j_csv,
};

const TYPE: &str = "http://example.org/type";

fn artifacts_equal(
    left: &purrdf_rdf::ProjectionPackage,
    right: &purrdf_rdf::ProjectionPackage,
) -> bool {
    left.artifacts().eq(right.artifacts())
}

#[test]
fn public_csv_surfaces_round_trip_without_hidden_vocabulary() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("http://example.org/subject");
    let target = builder.intern_iri("http://example.org/target");
    let rdf_type = builder.intern_iri(TYPE);
    let class = builder.intern_iri("http://example.org/Class");
    let name = builder.intern_iri("http://example.org/name");
    let literal = builder.intern_literal(RdfLiteral::simple("quoted, \"value\""));
    let relates = builder.intern_iri("http://example.org/relates");
    builder.push_quad(subject, rdf_type, class, None);
    builder.push_quad(subject, name, literal, None);
    builder.push_quad(subject, relates, target, None);
    let dataset = builder.freeze().expect("dataset");

    let limits = ProjectionLimits::new(64, 1_000_000, 4_000_000, 6_000_000, 16).expect("limits");
    let config = LpgConfig::new(
        TYPE,
        LpgScope::all(),
        limits,
        LpgExecutionLimits::new(100, 100, 100, 100).expect("execution limits"),
    )
    .expect("config");

    let generic = project_lpg_csv(dataset.as_ref(), &config).expect("generic projection");
    assert!(!generic.loss_ledger.is_empty());
    let generic_graph = read_lpg_csv(&generic.package, &config).expect("generic read");
    assert!(artifacts_equal(
        &generic.package,
        &write_lpg_csv(&generic_graph, &config).expect("generic rewrite")
    ));
    assert!(datasets_isomorphic(
        &dataset,
        &lift_lpg(&generic_graph, &config)
            .expect("generic lift")
            .dataset
    ));

    let neo4j = project_neo4j_csv(dataset.as_ref(), &config).expect("Neo4j projection");
    assert!(!neo4j.loss_ledger.is_empty());
    let neo4j_graph = read_neo4j_csv(&neo4j.package, &config).expect("Neo4j read");
    assert!(artifacts_equal(
        &neo4j.package,
        &write_neo4j_csv(&neo4j_graph, &config).expect("Neo4j rewrite")
    ));
    assert!(datasets_isomorphic(
        &dataset,
        &lift_lpg(&neo4j_graph, &config).expect("Neo4j lift").dataset
    ));
}
