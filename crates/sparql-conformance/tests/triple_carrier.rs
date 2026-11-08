// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The official default-graph query corpus through the physical triple carrier.
//! Named input/request refusals are enumerated by exact case identity, never silently
//! skipped. Supported results use the original comparator and expected-failure ledger.

use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

use purrdf_core::{SparqlRequest, TripleDataset, TripleReadError};
use purrdf_sparql_conformance::{Summary, discover, manifest, run, service};
use purrdf_sparql_eval::{FallibleSparqlError, InProcessServiceResolver};

/// Classify and execute one query through the actual fallible native entry point.
fn run_triple(
    case: &manifest::SparqlTestCase,
    remote: &InProcessServiceResolver,
) -> Result<Option<run::RunOutcome>, String> {
    let text = std::fs::read_to_string(&case.query)
        .map_err(|error| format!("read query {}: {error}", case.query.display()))?;
    let dataset = run::query_eval_dataset(case, &text)?;
    let owned = Arc::try_unwrap(dataset).map_err(|_| {
        format!(
            "{}: corpus loader retained a second dataset owner",
            case.iri
        )
    })?;
    let compact = match TripleDataset::try_from(owned) {
        Ok(compact) => compact,
        Err(TripleReadError::NamedGraphsUnsupported) => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let engine = run::query_eval_engine();
    let env = run::query_eval_env(case)?;
    let options = run::query_eval_options(case, &env).with_remote(Some(remote));
    let read = compact.read().map_err(|error| error.to_string())?;
    let result = engine.query_fallible_view(
        &read,
        SparqlRequest {
            query: &text,
            base_iri: Some(&case.base),
            substitutions: &[],
        },
        &options,
    );
    match result {
        Ok(complete) => Ok(Some(run::RunOutcome::RetainedEval {
            result: complete.result,
            ordered: run::query_eval_is_ordered(&text),
        })),
        Err(FallibleSparqlError::Operational {
            error: TripleReadError::NamedGraphsUnsupported,
            ..
        }) => Ok(None),
        Err(error) => Err(format!("evaluate {}: {error}", case.iri)),
    }
}

#[test]
fn official_default_graph_queries_use_compact_storage_and_original_grading() {
    let mut control =
        purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None);

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("suite");
    let mut inventory = String::new();
    let mut identities = std::collections::BTreeSet::new();
    let mut summary = Summary::default();
    let mut queries = 0;
    let mut named = 0;
    for family in ["w3c-sparql10", "w3c-sparql11", "w3c-sparql12"] {
        for group in discover(&root.join(family))
            .expect("original manifest discovery")
            .groups
        {
            for case in manifest::load(&group.path).expect("original manifest loader") {
                assert!(
                    identities.insert(case.iri.clone()),
                    "duplicate corpus case {}",
                    case.iri
                );
                let category = if matches!(case.kind, manifest::TestKind::QueryEval) {
                    queries += 1;
                    // Preserve the original source-setup verdict and order: it is
                    // neither an expected query refusal nor a named-graph exclusion.
                    let remote = match service::build(&case) {
                        Ok(remote) => remote,
                        Err(message) => {
                            summary
                                .record_setup_failure(&case, message)
                                .expect("unambiguous original xfail ledger");
                            writeln!(
                                inventory,
                                "{family}/{}\t{}\tsetup-failure",
                                group.relative, case.iri
                            )
                            .expect("write inventory");
                            continue;
                        }
                    };
                    match run_triple(&case, &remote) {
                        Ok(None) => {
                            named += 1;
                            "named-graph-refusal"
                        }
                        outcome => {
                            summary
                                .record_outcome(
                                    &case,
                                    outcome.map(|result| {
                                        result.expect(
                                            "a named refusal was handled before result grading",
                                        )
                                    }),
                                    &mut control,
                                )
                                .expect("unambiguous original xfail ledger");
                            "default-query"
                        }
                    }
                } else {
                    "other-test-kind"
                };
                writeln!(
                    inventory,
                    "{family}/{}\t{}\t{category}",
                    group.relative, case.iri
                )
                .expect("write inventory");
            }
        }
    }
    eprintln!(
        "triple corpus: {queries} queries, {named} named-graph refusals; {}",
        summary.tally_line()
    );
    assert!(summary.is_ok(), "{}", summary.failure_report());
    assert!(summary.passed > 0 && named > 0);
    purrdf_testkit::assert_golden!("triple-query-inventory.tsv", &inventory);
}
