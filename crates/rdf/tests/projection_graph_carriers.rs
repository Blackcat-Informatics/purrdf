// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public-surface round trips for deterministic openCypher and GraphML LPG packages.

use purrdf_rdf::{
    LpgConfig, ProjectionLimits, ProjectionPackage, RdfDatasetBuilder, RdfLiteral,
    datasets_isomorphic, lift_lpg, project_lpg_cypher, project_lpg_graphml, read_lpg_cypher,
    read_lpg_graphml, write_lpg_cypher, write_lpg_graphml,
};

const TYPE: &str = "http://example.org/type";

fn artifacts_equal(left: &ProjectionPackage, right: &ProjectionPackage) -> bool {
    left.artifacts().eq(right.artifacts())
}

#[test]
fn public_graph_carriers_round_trip_without_hidden_vocabulary() {
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("http://example.org/subject");
    let target = builder.intern_iri("http://example.org/target");
    let rdf_type = builder.intern_iri(TYPE);
    let class = builder.intern_iri("http://example.org/Class");
    let name = builder.intern_iri("http://example.org/name");
    let literal = builder.intern_literal(RdfLiteral::simple("quoted ' <&> value"));
    let relates = builder.intern_iri("http://example.org/relates");
    builder.push_quad(subject, rdf_type, class, None);
    builder.push_quad(subject, name, literal, None);
    builder.push_quad(subject, relates, target, None);
    let dataset = builder.freeze().expect("dataset");

    let limits = ProjectionLimits::new(32, 1_000_000, 3_000_000, 5_000_000, 16).expect("limits");
    let config = LpgConfig::new(TYPE, limits, 100).expect("config");

    let cypher = project_lpg_cypher(dataset.as_ref(), &config).expect("Cypher projection");
    assert!(!cypher.loss_ledger.is_empty());
    let cypher_graph = read_lpg_cypher(&cypher.package, &config).expect("Cypher read");
    assert!(artifacts_equal(
        &cypher.package,
        &write_lpg_cypher(&cypher_graph, &config).expect("Cypher rewrite")
    ));
    assert!(datasets_isomorphic(
        &dataset,
        &lift_lpg(&cypher_graph, &config)
            .expect("Cypher lift")
            .dataset
    ));

    let graphml = project_lpg_graphml(dataset.as_ref(), &config).expect("GraphML projection");
    assert!(!graphml.loss_ledger.is_empty());
    let graphml_graph = read_lpg_graphml(&graphml.package, &config).expect("GraphML read");
    assert!(artifacts_equal(
        &graphml.package,
        &write_lpg_graphml(&graphml_graph, &config).expect("GraphML rewrite")
    ));
    assert!(datasets_isomorphic(
        &dataset,
        &lift_lpg(&graphml_graph, &config)
            .expect("GraphML lift")
            .dataset
    ));
}
