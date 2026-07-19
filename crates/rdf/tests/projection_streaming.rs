// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public transactional streaming parity and progress checks for every LPG carrier.

use std::collections::BTreeMap;

use purrdf_rdf::{
    LpgConfig, LpgExecutionLimits, LpgProgress, LpgProgressPhase, LpgScope, ProjectionArtifactSink,
    ProjectionError, ProjectionLimits, ProjectionPackage, RdfDatasetBuilder, RdfLiteral,
    project_lpg_csv, project_lpg_csv_to_sink, project_lpg_cypher, project_lpg_cypher_to_sink,
    project_lpg_graphml, project_lpg_graphml_to_sink, project_neo4j_csv, project_neo4j_csv_to_sink,
};

const TYPE: &str = "https://example.org/type";

#[derive(Debug, Default)]
struct RecordingSink {
    artifacts: BTreeMap<String, Vec<u8>>,
    current: Option<(String, Vec<u8>)>,
    path_order: Vec<String>,
    chunks: usize,
    aborts: usize,
    active: bool,
    committed: bool,
    fail_after_chunks: Option<usize>,
}

impl RecordingSink {
    fn failing_after(chunks: usize) -> Self {
        Self {
            fail_after_chunks: Some(chunks),
            ..Self::default()
        }
    }
}

impl ProjectionArtifactSink for RecordingSink {
    fn begin_package(&mut self) -> Result<(), ProjectionError> {
        self.artifacts.clear();
        self.current = None;
        self.path_order.clear();
        self.chunks = 0;
        self.active = true;
        self.committed = false;
        Ok(())
    }

    fn begin_artifact(&mut self, path: &str) -> Result<(), ProjectionError> {
        self.current = Some((path.to_owned(), Vec::new()));
        self.path_order.push(path.to_owned());
        Ok(())
    }

    fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), ProjectionError> {
        if self
            .fail_after_chunks
            .is_some_and(|maximum| self.chunks >= maximum)
        {
            return Err(ProjectionError::integrity("injected sink failure"));
        }
        self.chunks += 1;
        self.current
            .as_mut()
            .expect("writer begins an artifact before chunks")
            .1
            .extend_from_slice(chunk);
        Ok(())
    }

    fn finish_artifact(&mut self) -> Result<(), ProjectionError> {
        let (path, bytes) = self
            .current
            .take()
            .expect("writer finishes an active artifact");
        assert!(self.artifacts.insert(path, bytes).is_none());
        Ok(())
    }

    fn commit_package(&mut self) -> Result<(), ProjectionError> {
        self.active = false;
        self.committed = true;
        Ok(())
    }

    fn abort_package(&mut self) {
        self.artifacts.clear();
        self.current = None;
        self.active = false;
        self.committed = false;
        self.aborts += 1;
    }
}

fn config() -> LpgConfig {
    LpgConfig::new(
        TYPE,
        LpgScope::all(),
        ProjectionLimits::new(64, 4_000_000, 12_000_000, 16_000_000, 16).expect("limits"),
        LpgExecutionLimits::new(10_000, 10_000, 10_000, 10_000).expect("execution limits"),
    )
    .expect("config")
}

fn dataset() -> std::sync::Arc<purrdf_rdf::RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let graph = builder.intern_iri("https://example.org/graph");
    let rdf_type = builder.intern_iri(TYPE);
    let class = builder.intern_iri("https://example.org/Person");
    let name = builder.intern_iri("https://example.org/name");
    let knows = builder.intern_iri("https://example.org/knows");
    let confidence = builder.intern_iri("https://example.org/confidence");
    let alice = builder.intern_iri("https://example.org/alice");
    let bob = builder.intern_iri("https://example.org/bob");
    let literal = builder.intern_literal(RdfLiteral::simple("Alice, \"A\""));
    let biography = builder.intern_iri("https://example.org/biography");
    let long_literal = builder.intern_literal(RdfLiteral::simple("x".repeat(100_000)));
    builder.push_quad(alice, rdf_type, class, Some(graph));
    builder.push_quad(alice, name, literal, Some(graph));
    builder.push_quad(alice, biography, long_literal, Some(graph));
    builder.push_quad(alice, knows, bob, Some(graph));
    let quoted = builder.intern_triple(alice, knows, bob);
    let reifier = builder.intern_iri("https://example.org/reifier");
    let high = builder.intern_iri("https://example.org/high");
    builder.push_reifier_in_graph(reifier, quoted, Some(graph));
    builder.push_annotation_in_graph(reifier, confidence, high, Some(graph));
    builder.freeze().expect("dataset")
}

fn assert_artifacts_equal(sink: &RecordingSink, package: &ProjectionPackage) {
    assert!(sink.committed);
    assert_eq!(
        sink.artifacts
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
            .collect::<Vec<_>>(),
        package.artifacts().collect::<Vec<_>>()
    );
    assert!(sink.chunks > sink.artifacts.len());
    assert!(sink.path_order.windows(2).all(|pair| pair[0] < pair[1]));
}

fn assert_progress(progress: &[LpgProgress]) {
    assert!(
        progress
            .iter()
            .any(|row| row.phase == LpgProgressPhase::Scanning)
    );
    assert!(
        progress
            .iter()
            .any(|row| row.phase == LpgProgressPhase::Building)
    );
    assert!(
        progress
            .iter()
            .any(|row| row.phase == LpgProgressPhase::Writing)
    );
    assert_eq!(
        progress.last().map(|row| row.phase),
        Some(LpgProgressPhase::Complete)
    );
    assert!(progress.windows(2).all(|pair| {
        pair[0].report.input_records <= pair[1].report.input_records
            && pair[0].report.model_records <= pair[1].report.model_records
            && pair[0].report.nodes <= pair[1].report.nodes
            && pair[0].report.edges <= pair[1].report.edges
            && pair[0].artifacts <= pair[1].artifacts
            && pair[0].bytes <= pair[1].bytes
    }));
}

#[test]
fn every_lpg_sink_is_chunked_transactional_and_byte_identical() {
    let dataset = dataset();
    let config = config();

    let mut generic_sink = RecordingSink::default();
    let mut generic_progress = Vec::new();
    let generic = project_lpg_csv_to_sink(
        dataset.as_ref(),
        &config,
        &mut generic_sink,
        &mut |row: &LpgProgress| {
            generic_progress.push(row.clone());
            Ok(())
        },
    )
    .expect("generic stream");
    let generic_package = project_lpg_csv(dataset.as_ref(), &config).expect("generic package");
    assert_eq!(generic.report, generic_package.report);
    assert_artifacts_equal(&generic_sink, &generic_package.package);
    assert_progress(&generic_progress);

    let mut neo4j_sink = RecordingSink::default();
    let mut neo4j_progress = Vec::new();
    project_neo4j_csv_to_sink(
        dataset.as_ref(),
        &config,
        &mut neo4j_sink,
        &mut |row: &LpgProgress| {
            neo4j_progress.push(row.clone());
            Ok(())
        },
    )
    .expect("Neo4j stream");
    let neo4j_package = project_neo4j_csv(dataset.as_ref(), &config).expect("Neo4j package");
    assert_artifacts_equal(&neo4j_sink, &neo4j_package.package);
    assert_progress(&neo4j_progress);

    let mut cypher_sink = RecordingSink::default();
    let mut cypher_progress = Vec::new();
    project_lpg_cypher_to_sink(
        dataset.as_ref(),
        &config,
        &mut cypher_sink,
        &mut |row: &LpgProgress| {
            cypher_progress.push(row.clone());
            Ok(())
        },
    )
    .expect("Cypher stream");
    let cypher_package = project_lpg_cypher(dataset.as_ref(), &config).expect("Cypher package");
    assert_artifacts_equal(&cypher_sink, &cypher_package.package);
    assert_progress(&cypher_progress);

    let mut graphml_sink = RecordingSink::default();
    let mut graphml_progress = Vec::new();
    project_lpg_graphml_to_sink(
        dataset.as_ref(),
        &config,
        &mut graphml_sink,
        &mut |row: &LpgProgress| {
            graphml_progress.push(row.clone());
            Ok(())
        },
    )
    .expect("GraphML stream");
    let graphml_package = project_lpg_graphml(dataset.as_ref(), &config).expect("GraphML package");
    assert_artifacts_equal(&graphml_sink, &graphml_package.package);
    assert_progress(&graphml_progress);
}

#[test]
fn sink_and_observer_failures_abort_without_a_success_event() {
    let dataset = dataset();
    let config = config();

    let mut failing_sink = RecordingSink::failing_after(2);
    let mut progress = Vec::new();
    let error = project_lpg_graphml_to_sink(
        dataset.as_ref(),
        &config,
        &mut failing_sink,
        &mut |row: &LpgProgress| {
            progress.push(row.clone());
            Ok(())
        },
    )
    .expect_err("injected sink failure");
    assert!(error.message().contains("injected sink failure"));
    assert_eq!(failing_sink.aborts, 1);
    assert!(failing_sink.artifacts.is_empty());
    assert!(!failing_sink.committed);
    assert_ne!(
        progress.last().map(|row| row.phase),
        Some(LpgProgressPhase::Complete)
    );

    let mut observer_sink = RecordingSink::default();
    let mut observed = Vec::new();
    project_lpg_csv_to_sink(
        dataset.as_ref(),
        &config,
        &mut observer_sink,
        &mut |row: &LpgProgress| {
            observed.push(row.phase);
            if row.phase == LpgProgressPhase::Writing {
                Err(ProjectionError::integrity("injected observer failure"))
            } else {
                Ok(())
            }
        },
    )
    .expect_err("injected observer failure");
    assert_eq!(observer_sink.aborts, 1);
    assert!(observer_sink.artifacts.is_empty());
    assert!(!observer_sink.committed);
    assert!(!observed.contains(&LpgProgressPhase::Complete));
}
