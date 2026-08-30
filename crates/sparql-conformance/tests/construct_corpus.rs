// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The first-party **CONSTRUCT** conformance corpus
//! (`crates/sparql-conformance/corpus/construct/`).
//!
//! The corpus covers both `CONSTRUCT` forms, case for case: the SPARQL 1.1
//! §16.2 triple-producing form (expected results in Turtle, so every statement
//! must land in the DEFAULT graph) and the quad-producing
//! `CONSTRUCT GRAPH <iri>` form (expected results in N-Quads, carrying the
//! target graph on every line). A downstream consumer asking "is this query
//! form covered by conformance evidence" gets a scoreboard row, not a promise.
//!
//! # Why its own target, beside `suite/`
//!
//! `sparql_conformance.rs`'s `datatest_stable::harness!` is rooted at `suite/`
//! and folds every manifest it finds into ONE conformance-matrix row. This
//! corpus lives under `corpus/` instead so it reports its own row (and carries
//! its own ratchet budget in `scripts/conformance-baseline.json`) rather than
//! disappearing into the full-corpus tally, where a regression in it would move
//! a four-digit number by one.
//!
//! The scoreboard line `CONSTRUCT-CORPUS: passed N total M` is what
//! `scripts/conformance-matrix.py` scrapes.

use std::path::PathBuf;

use purrdf_sparql_conformance::manifest::TestKind;

/// The corpus manifest.
fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("corpus")
        .join("construct")
        .join("manifest.ttl")
}

/// Every case in the corpus must pass — there is no xfail ledger here, because a
/// first-party corpus that ledgers its own cases is a corpus that grades itself
/// against what the engine happens to do.
#[test]
fn construct_corpus_is_green() {
    let manifest = manifest();
    assert!(manifest.is_file(), "corpus manifest missing: {manifest:?}");

    let total = purrdf_sparql_conformance::manifest::load(&manifest)
        .unwrap_or_else(|e| panic!("load the CONSTRUCT corpus manifest: {e}"))
        .len();
    let summary = purrdf_sparql_conformance::run_manifest(&manifest)
        .unwrap_or_else(|e| panic!("run the CONSTRUCT corpus: {e}"));

    // The scoreboard line the conformance matrix scrapes. Printed BEFORE the
    // assertions so a red run still reports its tally.
    println!("CONSTRUCT-CORPUS: passed {} total {total}", summary.passed);
    assert!(
        summary.is_ok(),
        "CONSTRUCT corpus failed:\n{}",
        summary.failure_report()
    );
    assert_eq!(
        summary.xfail, 0,
        "the first-party CONSTRUCT corpus carries no xfail ledger"
    );
    assert_eq!(
        summary.passed, total,
        "every declared CONSTRUCT case must run and pass"
    );
}

/// Count-and-kind tripwire: the corpus is the deliverable, so its shape is
/// pinned rather than merely reported. A case silently dropped from
/// `mf:entries` — or a quad-form case quietly converted into a triple-form one —
/// changes these numbers.
#[test]
fn construct_corpus_case_count_and_kinds() {
    let cases = purrdf_sparql_conformance::manifest::load(&manifest())
        .unwrap_or_else(|e| panic!("load the CONSTRUCT corpus manifest: {e}"));

    assert_eq!(cases.len(), 19, "the corpus declares 19 cases");
    assert_eq!(
        cases
            .iter()
            .filter(|c| c.kind == TestKind::QueryEval)
            .count(),
        17
    );
    assert_eq!(
        cases
            .iter()
            .filter(|c| c.kind == TestKind::PositiveSyntax)
            .count(),
        1
    );
    assert_eq!(
        cases
            .iter()
            .filter(|c| c.kind == TestKind::NegativeSyntax)
            .count(),
        1
    );
    assert_eq!(
        cases.iter().filter(|c| c.kind == TestKind::Unknown).count(),
        0,
        "an unmodeled case would be a silent skip"
    );

    // Both halves must be present. The quad-form evaluation cases are exactly
    // those whose template names a graph; if that half ever emptied out, the
    // corpus would still be "green" while measuring only the pre-existing form.
    let (quad_form, triple_form): (Vec<_>, Vec<_>) = cases
        .iter()
        .filter(|c| c.kind == TestKind::QueryEval)
        .partition(|c| {
            !template_graphs(&std::fs::read_to_string(&c.query).unwrap_or_default()).is_empty()
        });
    assert_eq!(
        quad_form.len(),
        9,
        "nine evaluation cases must exercise the quad-producing form"
    );
    assert_eq!(
        triple_form.len(),
        8,
        "eight evaluation cases must exercise the triple-producing form"
    );

    // The pairing is the corpus's central claim, so it is checked rather than
    // asserted in prose: every triple-form case must have a `graph`-prefixed
    // counterpart. (The reverse does not hold: `graphReifierScope` needs TWO
    // graphs to say anything at all, so the triple form cannot host it.)
    for case in &triple_form {
        let local = case.iri.rsplit('#').next().expect("a fragment-local name");
        let mut chars = local.chars();
        let counterpart = format!(
            "graph{}{}",
            chars.next().expect("a non-empty name").to_uppercase(),
            chars.as_str()
        );
        assert!(
            quad_form
                .iter()
                .any(|c| c.iri.ends_with(&format!("#{counterpart}"))),
            "{local} has no quad-form counterpart `{counterpart}`: the paired-corpus \
             claim would be false"
        );
    }
}

/// The graph IRIs a `CONSTRUCT` **template** names, in order — both the
/// whole-template `CONSTRUCT GRAPH <iri>` shorthand and the `GRAPH <iri> { … }`
/// blocks of the quad-template grammar.
///
/// Scoped to the text before `WHERE`, so a `GRAPH ?g` in the pattern (which says
/// nothing about where the result lands) is never mistaken for a target graph.
fn template_graphs(query: &str) -> Vec<String> {
    let template = query.split("WHERE").next().unwrap_or(query);
    let mut graphs = Vec::new();
    let mut rest = template;
    while let Some(idx) = rest.find("GRAPH") {
        rest = &rest[idx + "GRAPH".len()..];
        if let Some(iri) = rest.trim_start().strip_prefix('<')
            && let Some(end) = iri.find('>')
        {
            graphs.push(format!("<{}>", &iri[..end]));
        }
    }
    graphs
}

/// The quad-form evaluation cases must expect **N-Quads** results carrying the
/// target graph, and the triple-form ones must expect graph results that carry
/// none. This is the corpus's own anti-tautology check: an expected file that
/// silently lost its graph term would let a triple-emitting regression pass.
#[test]
fn quad_form_expectations_actually_name_a_graph() {
    let cases = purrdf_sparql_conformance::manifest::load(&manifest())
        .unwrap_or_else(|e| panic!("load the CONSTRUCT corpus manifest: {e}"));

    const TARGET_GRAPH: &str = "<http://example.org/out>";
    let mut graph_bearing_lines = 0_usize;
    for case in &cases {
        if case.kind != TestKind::QueryEval {
            continue;
        }
        let query = std::fs::read_to_string(&case.query).expect("read the case query");
        let purrdf_sparql_conformance::manifest::ExpectedResult::Graph(result) = &case.expected
        else {
            panic!("{} must expect a graph result", case.iri);
        };
        let expected = std::fs::read_to_string(result).expect("read the expected result");
        let statements: Vec<&str> = expected
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("@prefix"))
            .collect();

        // The expectation's own SHAPE has to agree with the query's form: a
        // quad-form case must expect N-Quads and a triple-form one must not, or
        // the file-extension contract this corpus is built on stops holding.
        let graphs = template_graphs(&query);
        assert_eq!(
            !graphs.is_empty(),
            result.extension().is_some_and(|e| e == "nq"),
            "{}: a template that names a graph must expect N-Quads, and one that \
             names none must not",
            case.iri
        );

        if graphs.is_empty() {
            for line in &statements {
                assert!(
                    !line.contains(TARGET_GRAPH),
                    "{}: a triple-form expectation must carry no graph term, got `{line}`",
                    case.iri
                );
            }
        } else {
            for line in &statements {
                assert!(
                    graphs.iter().any(|g| line.contains(g.as_str())),
                    "{}: a quad-form expectation must carry a graph its template names, \
                     got `{line}`",
                    case.iri
                );
                graph_bearing_lines += 1;
            }
        }
    }
    assert!(
        graph_bearing_lines >= 18,
        "the quad-form expectations must pin real statements, saw {graph_bearing_lines}"
    );
}

/// The RDF 1.2 statement layer is a separate emission path — reifier
/// declarations and annotations do not travel through `push_quad` — so a
/// regression that left them in the default graph beside the target graph's
/// quads would be invisible to every case above. It is therefore measured under
/// BOTH forms, and the per-graph keying of the layer is pinned outright.
#[test]
fn the_corpus_measures_the_rdf_12_statement_layer() {
    let cases = purrdf_sparql_conformance::manifest::load(&manifest())
        .unwrap_or_else(|e| panic!("load the CONSTRUCT corpus manifest: {e}"));

    const REIFIES: &str = "rdf-syntax-ns#reifies";
    let expectation = |local: &str| -> String {
        let case = cases
            .iter()
            .find(|c| c.iri.ends_with(&format!("#{local}")))
            .unwrap_or_else(|| panic!("{local} must be a declared case"));
        let purrdf_sparql_conformance::manifest::ExpectedResult::Graph(result) = &case.expected
        else {
            panic!("{local} must expect a graph result");
        };
        std::fs::read_to_string(result).expect("read the expected result")
    };

    // The triple form: a reifier and its annotation, all in the default graph.
    let triple = expectation("reified");
    assert!(
        triple.contains("~_:") && triple.contains("{|"),
        "the triple-form statement-layer expectation must declare a reifier and \
         annotate it"
    );
    assert!(
        !triple.contains("http://example.org/out"),
        "the triple-form statement layer must carry no graph term"
    );

    // The quad form: the SAME template with a target graph, and every line of the
    // statement layer — the reifier declarations and the annotations alike —
    // must carry that graph, not fall back to the default one.
    let quad = expectation("graphReified");
    assert!(
        quad.lines().filter(|l| l.contains(REIFIES)).count() >= 2,
        "the quad-form statement-layer expectation must pin real reifier declarations"
    );
    for line in quad.lines().map(str::trim).filter(|l| !l.is_empty()) {
        assert!(
            line.contains("<http://example.org/out> ."),
            "every quad-form statement-layer line must end in the target graph, got `{line}`"
        );
    }
    assert!(
        quad.contains("<http://example.org/source> <http://example.org/ledger>"),
        "the quad-form annotation must survive into the target graph"
    );

    // The per-graph keying: ONE reifier id, the same annotation predicate, two
    // graphs. In the graph that declares the reifier the statement is an
    // annotation; in the graph that does not, the identical shape is an ordinary
    // quad. The two canonicalize differently, so this case fails the moment the
    // keying degrades to "any reifier declared anywhere in the output".
    let scope = expectation("graphReifierScope");
    assert!(
        scope.contains(REIFIES) && scope.contains("<http://example.org/out> ."),
        "the keying case must declare its reifier in the target graph"
    );
    assert!(
        scope.contains("<http://example.org/other> ."),
        "the keying case must place a same-shaped statement in a SECOND graph, or it \
         pins nothing about per-graph keying"
    );
    let scope_query = {
        let case = cases
            .iter()
            .find(|c| c.iri.ends_with("#graphReifierScope"))
            .expect("the keying case must be declared");
        std::fs::read_to_string(&case.query).expect("read the keying query")
    };
    let graphs = template_graphs(&scope_query);
    assert_eq!(
        graphs.len(),
        2,
        "the keying case's template must write into exactly two graphs, saw {graphs:?}"
    );
    assert_ne!(graphs[0], graphs[1], "the two graphs must differ");
}
