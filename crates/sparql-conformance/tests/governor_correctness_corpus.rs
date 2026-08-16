// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The D0 guarantee's second half, over the **whole** W3C SPARQL conformance corpus.
//!
//! D0 has two halves. The first is that an ungoverned query is unchanged, which every
//! existing conformance run already demonstrates. The second is the one this file exists
//! for: a query run through the **governed** entry point under
//! [`QueryGovernors::UNBOUNDED`] — which declines every ceiling and every counter — must
//! return the identical answer, byte for byte, to the ordinary ungoverned path.
//!
//! # Why the corpus, and all of it
//!
//! The governed path is not a wrapper around the ungoverned one. It runs the evaluator on
//! a different result channel (`Evaluated`, which every operator has to thread) and
//! materializes through a different function. That is exactly the shape of change that
//! goes wrong on the operator nobody thought to test, so the check is run over every
//! query-evaluation case the suite has — property paths, `GRAPH` scopes, aggregates,
//! federated `SERVICE`, entailment regimes, and the RDF 1.2 triple-term and reifier
//! cases — rather than over a sample chosen by whoever wrote the test.
//!
//! # Why it lives here and not beside its siblings
//!
//! The other properties of this harness are in
//! `crates/sparql-eval/tests/governor_correctness.rs`. This one cannot be: the corpus and
//! its manifest loader belong to this crate, and this crate **depends on**
//! `purrdf-sparql-eval`. Reaching the corpus from there would mean a new dependency edge,
//! which the repository forbids. So the walker is reused where it lives — the same
//! `manifest::load` / `run::load_dataset` / `service::build` sequence
//! `cost_planner_corpus.rs` uses, with the governed run substituted for one of its two
//! planner variants.
//!
//! [`QueryGovernors::UNBOUNDED`]: purrdf_sparql_eval::QueryGovernors::UNBOUNDED

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use purrdf_core::{RdfDataset, SparqlRequest, SparqlResult};
use purrdf_sparql_conformance::manifest::{ExpectedResult, SparqlTestCase, TestKind};
use purrdf_sparql_eval::{
    AggregateRegistry, EvalOptions, GovernedOutcome, NativeSparqlEngine, ParserOptions,
    QueryGovernors, QueryOptions, StandpointPredicates,
};

/// Build the per-case statistical-aggregate registry `case.aggregate_namespace`
/// requests (see `crate::manifest::SparqlTestCase` and `crate::run::run`, whose
/// per-case registration this mirrors), or `None` when the case declares none. An
/// AGG(...) evaluation case must be compared under both paths with an engine that
/// can actually resolve its aggregate, or it would fail identically for a reason
/// unrelated to D0.
fn case_aggregates(case: &SparqlTestCase) -> Option<AggregateRegistry> {
    case.aggregate_namespace.as_ref().map(|namespace| {
        let mut registry = AggregateRegistry::new();
        registry.register_statistical_aggregates(namespace);
        registry
    })
}

/// The sentinel base the manifest loader resolves case IRIs against.
const BASE: &str = "http://purrdf.test/manifest/";

/// The extension-function namespace the conformance harness configures.
const EXT_NS: &str = "https://example.org/ext/";

/// An engine configured exactly as the conformance harness configures it, so a case that
/// passes there is evaluable here.
fn harness_engine() -> NativeSparqlEngine {
    NativeSparqlEngine::new()
        .with_parser_options(ParserOptions {
            extension_fn_namespaces: vec![EXT_NS.to_owned()],
            property_fn_namespaces: vec![purrdf_sparql_conformance::run::REL_NS.to_owned()],
            property_fn_iris: Vec::new(),
        })
        .with_standpoint_predicates(StandpointPredicates::new(
            format!("{EXT_NS}accordingTo"),
            format!("{EXT_NS}sharpens"),
        ))
        .with_eval_options(EvalOptions {
            exists_memo: true,
            force_structural_bgp_order: false,
            force_sequential: false,
        })
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: Some(BASE),
        substitutions: &[],
    }
}

/// A total, deterministic rendering of the whole egress model, for byte comparison.
///
/// Not a SPARQL Results serializer: those have a support matrix (XML rejects graphs,
/// CSV/TSV reject booleans), so routing through one would silently exclude whole classes
/// of case from the comparison — which is the failure this file exists to catch. This
/// renders every arm and every field the type carries, including the `aux` dataset that
/// value-constructing builtins mint into a solutions result, so "identical" means
/// identical rather than "identical in the parts a writer happened to emit".
///
/// Graphs go through the RDFC-1.0 canonicalizer, which is what makes the comparison
/// meaningful for blank nodes: two runs may legitimately label a fresh blank differently,
/// and canonical N-Quads is the byte form in which they must nevertheless agree — the
/// same equality the conformance harness's own `CONSTRUCT` comparison uses.
fn render(result: &SparqlResult) -> String {
    match result {
        SparqlResult::Boolean(value) => format!("boolean\n{value}\n"),
        SparqlResult::Graph(graph) => {
            format!("graph\n{}", purrdf_core::canonicalize(graph).nquads)
        }
        SparqlResult::Solutions {
            variables,
            rows,
            aux,
        } => {
            let mut out = format!("solutions\nvariables {variables:?}\n");
            for row in rows {
                writeln!(out, "row {row:?}").expect("writing to a String cannot fail");
            }
            write!(out, "aux\n{}", purrdf_core::canonicalize(aux).nquads)
                .expect("writing to a String cannot fail");
            out
        }
    }
}

/// Recursively list every `manifest.ttl` under `suite/`.
fn discover_manifests(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(discover_manifests(&path));
            } else if path.file_name().and_then(|n| n.to_str()) == Some("manifest.ttl") {
                out.push(path);
            }
        }
    }
    out
}

/// The ungoverned answer for one case, or the diagnostic that prevented one.
fn ungoverned(
    engine: &NativeSparqlEngine,
    dataset: &Arc<RdfDataset>,
    query: &str,
    remote: Option<&purrdf_sparql_eval::LocalRemoteQuerySource>,
    aggregates: Option<&AggregateRegistry>,
) -> Result<SparqlResult, String> {
    let empty_aggregates = AggregateRegistry::EMPTY;
    let options = QueryOptions {
        property_functions: purrdf_sparql_conformance::run::harness_relations(),
        aggregates: aggregates.unwrap_or(&empty_aggregates),
        ..QueryOptions::EMPTY
    };
    match remote {
        Some(source) => engine.query_with_source(dataset, request(query), source, options),
        None => engine.query_with_options_view(&**dataset, request(query), options),
    }
    .map_err(|error| error.to_string())
}

#[test]
fn d0_governed_unbounded_is_byte_identical_to_ungoverned() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("suite");
    let manifests = discover_manifests(&root);
    assert!(
        manifests.len() >= 10,
        "suite inventory shrank: found only {} manifests",
        manifests.len()
    );

    // One engine per path, each with its own plan cache, so the comparison is between two
    // full parse-plan-evaluate pipelines rather than between two calls that shared a plan.
    let plain = harness_engine();
    let governed_engine = harness_engine();

    let mut cases = 0_usize;
    let mut compared = 0_usize;
    let mut skipped = 0_usize;
    let mut agreed_errors = 0_usize;
    // Cases whose manifest expects the run to be REFUSED (a `.err` `mf:result`): the
    // seam's hard errors. They cannot produce two comparable answers, but they are
    // not excused either — D0 still owes that both paths refuse, and refuse
    // identically, which the `(Err, Err)` arm below checks. Counted so the identity
    // asserted at the end stays exact rather than being loosened to an inequality.
    let mut expected_failures = 0_usize;
    let mut mismatches: Vec<(String, String)> = Vec::new();

    for manifest in &manifests {
        let loaded = purrdf_sparql_conformance::manifest::load(manifest)
            .unwrap_or_else(|error| panic!("load {}: {error}", manifest.display()));
        for case in loaded {
            if !matches!(case.kind, TestKind::QueryEval) {
                continue;
            }
            cases += 1;
            if matches!(case.expected, ExpectedResult::EvalError(_)) {
                expected_failures += 1;
            }
            let query = std::fs::read_to_string(&case.query)
                .unwrap_or_else(|error| panic!("read query {}: {error}", case.query.display()));

            let Ok(dataset) = purrdf_sparql_conformance::run::load_dataset(&case) else {
                // A case whose fixtures do not load has no answer for either path to
                // return, so it says nothing about the governed one.
                skipped += 1;
                continue;
            };
            let Ok(remote) = purrdf_sparql_conformance::service::build(&case) else {
                skipped += 1;
                continue;
            };

            let case_aggregates = case_aggregates(&case);
            let empty_aggregates = AggregateRegistry::EMPTY;
            let aggregates_ref = case_aggregates.as_ref().unwrap_or(&empty_aggregates);

            let expected = ungoverned(
                &plain,
                &dataset,
                &query,
                remote.as_ref(),
                case_aggregates.as_ref(),
            );

            // The governed path, with every ceiling and every counter declined. A trip is
            // not merely unexpected here, it is unrepresentable: `UNBOUNDED` engages
            // nothing, so no charge site can refuse anything.
            let actual = match remote.as_ref() {
                Some(source) => governed_engine.query_governed_with_source(
                    &dataset,
                    request(&query),
                    source,
                    QueryOptions {
                        aggregates: aggregates_ref,
                        ..QueryOptions::EMPTY
                    },
                    &QueryGovernors::UNBOUNDED,
                ),
                // The relation table travels on the governed path too, in the same
                // options every governed entry takes: a first-party relation case must
                // be COMPARED here, and a governed run whose calls resolved to nothing
                // would be comparing a different query against the oracle. The
                // per-case aggregate registry (`case_aggregates`) travels alongside it
                // for the identical reason — see `case_aggregates`'s doc comment.
                None => governed_engine.query_governed(
                    &dataset,
                    request(&query),
                    QueryOptions {
                        property_functions: purrdf_sparql_conformance::run::harness_relations(),
                        aggregates: aggregates_ref,
                        ..QueryOptions::EMPTY
                    },
                    &QueryGovernors::UNBOUNDED,
                ),
            }
            .map_err(|error| error.to_string());

            match (expected, actual) {
                (
                    Ok(expected),
                    Ok(GovernedOutcome::Complete {
                        result, evidence, ..
                    }),
                ) => {
                    compared += 1;
                    if !evidence.is_complete() {
                        mismatches.push((
                            case.iri.clone(),
                            format!("UNBOUNDED reported a trip: {evidence:?}"),
                        ));
                        continue;
                    }
                    let (left, right) = (render(&expected), render(&result));
                    if left != right {
                        mismatches.push((
                            case.iri.clone(),
                            format!(
                                "governed result is not byte-identical\n\
                                 ungoverned:\n{left}\ngoverned:\n{right}"
                            ),
                        ));
                    }
                }
                (Ok(_), Ok(GovernedOutcome::BudgetExhausted(exhausted))) => {
                    mismatches.push((
                        case.iri.clone(),
                        format!(
                            "UNBOUNDED engages no ceiling, so nothing can trip — yet: {:?}",
                            exhausted.tripped
                        ),
                    ));
                }
                (Ok(_), Err(error)) => {
                    mismatches.push((
                        case.iri.clone(),
                        format!("governed path failed where the ungoverned one succeeded: {error}"),
                    ));
                }
                (Err(expected), Ok(_)) => {
                    mismatches.push((
                        case.iri.clone(),
                        format!(
                            "governed path succeeded where the ungoverned one failed: {expected}"
                        ),
                    ));
                }
                (Err(expected), Err(actual)) => {
                    // Both refuse the case — an unsupported feature, or a query the suite
                    // expects to fail. D0 is about agreement, so agreeing to fail counts,
                    // but only if they fail the SAME way: a governed path that turned one
                    // diagnostic into another would be a behaviour change hiding inside an
                    // error path.
                    if expected == actual {
                        agreed_errors += 1;
                    } else {
                        mismatches.push((
                            case.iri.clone(),
                            format!(
                                "both paths failed, with different diagnostics\n\
                                 ungoverned: {expected}\ngoverned:   {actual}"
                            ),
                        ));
                    }
                }
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} of {cases} governed runs diverged from the ungoverned oracle:\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .map(|(iri, why)| format!("  {iri}: {why}"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    // Inventory tripwires, in the same spirit as the suite's own: a corpus walk that
    // quietly stopped finding cases would otherwise report success for doing nothing.
    assert!(
        cases >= 300,
        "the query-evaluation corpus shrank: only {cases} cases were enumerated"
    );
    // Every case in the corpus is genuinely exercised: nothing is skipped for unloadable
    // fixtures, and the only cases excused from producing two comparable answers are the
    // ones whose manifest EXPECTS a refusal — where agreeing to fail, identically, is the
    // whole of what D0 can owe. A floor on `compared` alone would let the walk quietly
    // stop reaching cases as long as enough of them still worked, so the counts are
    // pinned as an identity instead.
    assert_eq!(
        (compared + agreed_errors, skipped, agreed_errors),
        (cases, 0, expected_failures),
        "the byte-identity claim rests on the comparisons, not on the enumeration: \
         {compared} of {cases} cases produced two comparable answers, {skipped} were \
         skipped, and {agreed_errors} agreed to fail against {expected_failures} case(s) \
         whose manifest expects a refusal"
    );
}
