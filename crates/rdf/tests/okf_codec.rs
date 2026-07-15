// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Production-surface tests for the bidirectional native OKF codec.

use purrdf_rdf::{
    BlankScope, DatasetSink, OkfBundle, OkfConfig, OkfWriter, RdfDataset, RdfDatasetBuilder,
    RdfLiteral, assert_ledger_complete, assert_ledger_sound, datasets_isomorphic, lift_okf_bundle,
    write_okf_bundle,
};

fn config() -> OkfConfig {
    OkfConfig::new(
        "https://example.org/okf#",
        "https://example.org/doc/",
        ["type", "title", "resource", "tags", "active", "producer"],
    )
    .expect("valid caller profile")
}

fn lift(bundle: &OkfBundle, config: &OkfConfig) -> std::sync::Arc<RdfDataset> {
    let mut sink = DatasetSink::new();
    let outcome = lift_okf_bundle(bundle, config, &mut sink).expect("lift bundle");
    assert!(outcome.losses.is_empty());
    sink.into_dataset().expect("finished sink")
}

#[test]
fn public_visitor_and_sink_surfaces_are_bidirectional_and_stable() {
    let config = config();
    let bundle = OkfBundle::from_documents([
        (
            "schema.md",
            "---\ntype: Schema\ntitle: Events\n---\nColumns.\n",
        ),
        (
            "table.md",
            "---\ntype: Table\nresource: https://example.org/data/events\ntags: [stable, analytics]\nactive: true\nproducer:\n  name: fixture\n---\nSee [schema](schema.md).\n",
        ),
    ])
    .expect("valid OKF bundle");
    let dataset = lift(&bundle, &config);

    let convenience = write_okf_bundle(&dataset, &config).expect("write bundle");
    let mut visitor = OkfWriter::new(&config);
    dataset.emit(&mut visitor);
    let direct = visitor.finish().expect("finish visitor");
    assert_eq!(direct, convenience);
    assert!(direct.losses.is_empty());

    let reparsed = lift(&direct.bundle, &config);
    assert!(datasets_isomorphic(&dataset, &reparsed));
    let stable = write_okf_bundle(&reparsed, &config).expect("rewrite bundle");
    assert_eq!(direct.bundle, stable.bundle);
}

#[test]
fn public_writer_reports_the_closed_loss_profile() {
    let config = config();
    let mut builder = RdfDatasetBuilder::new();
    let subject = builder.intern_iri("https://example.org/doc/concept.md");
    let path_predicate = builder.intern_iri(config.path_predicate());
    let body_predicate = builder.intern_iri(config.body_predicate());
    let type_predicate = builder.intern_iri(config.predicate_iri("type").expect("type IRI"));
    let path = builder.intern_literal(RdfLiteral::simple("concept.md"));
    let body = builder.intern_literal(RdfLiteral::simple("Body.\n"));
    let kind = builder.intern_literal(RdfLiteral::simple("Concept"));
    builder.push_quad(subject, path_predicate, path, None);
    builder.push_quad(subject, body_predicate, body, None);
    builder.push_quad(subject, type_predicate, kind, None);

    let owl = builder.intern_iri("http://www.w3.org/2002/07/owl#equivalentClass");
    let other = builder.intern_iri("https://example.org/Other");
    let graph = builder.intern_iri("https://example.org/graph");
    builder.push_quad(subject, owl, other, None);
    builder.push_quad(subject, owl, other, Some(graph));
    let triple = builder.intern_triple(subject, owl, other);
    let reifier = builder.intern_blank("unrelated", BlankScope::DEFAULT);
    builder.push_reifier(reifier, triple);
    let note_predicate = builder.intern_iri("https://example.org/note");
    let note = builder.intern_literal(RdfLiteral::simple("metadata"));
    builder.push_annotation(reifier, note_predicate, note);

    let dataset = builder.freeze().expect("valid RDF dataset");
    let outcome = write_okf_bundle(&dataset, &config).expect("lossy projection");
    assert_ledger_complete(
        &outcome.losses,
        &[
            "named-graph-dropped",
            "okf-annotation-dropped",
            "okf-non-profile-quad-dropped",
            "okf-reifier-dropped",
        ],
    );
    assert_ledger_sound(&outcome.losses, "rdf-1.2-dataset", "okf");
}
