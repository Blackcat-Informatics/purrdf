// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The community SHACL corpus, every applicable dated execution, graded exactly.

use std::path::{Path, PathBuf};

use purrdf_conformance_kit::outcome::OutcomeKind;
use purrdf_lex::json::Value;
use purrdf_shapes::ShaclProfile;
use purrdf_shapes::profile::AdmissionReason;
use purrdf_sparql_conformance::community::{self, Corpus, Selection, Totals};

fn corpus_root() -> PathBuf {
    purrdf_testkit::paths::workspace_root().join("corpora/community")
}

/// Copy the corpus so one artifact can be altered without touching the source.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn every_applicable_dated_execution_passes_exactly() {
    let mut control =
        purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None);

    let corpus = Corpus::acquire(&corpus_root()).unwrap();
    let (totals, executions) = corpus.run(&mut control).unwrap();
    print!("{}", totals.scoreboard());
    for execution in &executions {
        for record in execution.records.iter().filter(|record| !record.passed) {
            println!(
                "FAIL {} {} {}: expected {:?}/{:?}, observed {:?}/{:?}: {}",
                record.case,
                record.profile,
                record.surface,
                record.expected.kind,
                record.expected.reason,
                record.observed.kind,
                record.observed.reason,
                record.observed.message
            );
        }
    }
    assert_eq!(totals.cases, 79);
    assert_eq!(totals.executions, 130);
    assert_eq!(totals.unsupported, 0, "both corpus profiles are built in");
    assert_eq!(totals.failed, 0);
    assert_eq!(totals.passed, 130);
    assert_eq!((totals.query_executions, totals.passed_queries), (15, 15));
    assert_eq!(totals.observations, 610);
    assert_eq!(totals.passed_observations, totals.observations);
    assert_eq!(
        totals.profiles,
        [
            ("shacl-20170720".to_owned(), [57, 57, 0, 0]),
            ("shacl12-20260918".to_owned(), [58, 58, 0, 0]),
            ("sparql11-query-over-rdf12".to_owned(), [15, 15, 0, 0]),
        ]
        .into()
    );
    // Every validation execution ran all five public routes; rules run once.
    for execution in &executions {
        let routes: Vec<_> = execution
            .records
            .iter()
            .map(|record| record.surface.as_str())
            .collect();
        assert!(
            routes
                == [
                    "native/query",
                    "native/prepared/0",
                    "native/algebra/0",
                    "native/prepared/1",
                    "native/algebra/1"
                ]
                || routes == ["native/rules"]
                || routes
                    == [
                        "native/fresh",
                        "native/prepared/0",
                        "native/prepared/1",
                        "native/product-admitted",
                        "native/product-rebuilt",
                    ]
                || routes == ["native/parse"],
            "{} {}: {routes:?}",
            execution.case,
            execution.profile
        );
    }
}

#[test]
fn nested_suite_executes_the_same_reviewed_inputs() {
    let mut control =
        purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None);

    let scratch = purrdf_testkit::TempDir::new_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let copy = scratch.path().join("community");
    let nested = copy.join("nested");
    copy_tree(&corpus_root(), &nested);
    let catalog = std::fs::read_to_string(nested.join("catalog.json")).unwrap();
    let catalog = catalog
        .replace("\"directory\": \".\"", "\"directory\": \"nested\"")
        .replace("\"manifest.ttl\"", "\"nested/manifest.ttl\"")
        .replace(
            "\"shacl/inventory.json\"",
            "\"nested/shacl/inventory.json\"",
        )
        .replace("\"shacl/manifest.ttl\"", "\"nested/shacl/manifest.ttl\"")
        .replace("\"sparql/", "\"nested/sparql/")
        .replace("\"reviews/index.json\"", "\"nested/reviews/index.json\"");
    std::fs::write(copy.join("catalog.json"), catalog).unwrap();
    let index_path = nested.join("reviews/index.json");
    let index = std::fs::read_to_string(&index_path).unwrap();
    let index = index
        .replace("\"path\": \"", "\"path\": \"nested/")
        .replace("\"derivation\": \"", "\"derivation\": \"nested/");
    std::fs::write(index_path, index).unwrap();
    for suite in ["interactions", "service"] {
        let path = nested.join(format!("sparql/{suite}/inventory.json"));
        let inventory = std::fs::read_to_string(&path).unwrap().replace(
            "http://example.org/community/sparql/",
            "http://example.org/community/nested/sparql/",
        );
        std::fs::write(path, inventory).unwrap();
    }

    let corpus = Corpus::acquire(&copy).expect("relocated reviewed bytes are admitted");
    let (totals, executions) = corpus.run(&mut control).unwrap();
    assert_eq!(totals.cases, 79);
    assert_eq!(totals.executions, 130);
    assert_eq!(totals.unsupported, 0);
    assert_eq!(totals.failed, 0, "{executions:#?}");
    assert_eq!(totals.passed_observations, 610);
}

#[test]
fn an_unimplemented_profile_is_reported_apart_from_failures() {
    let mut control =
        purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None);

    let mut profiles = Value::object::<&str, Value>([]);
    profiles["shacl12-20991231"] = Value::object([(
        "specification",
        Value::String("https://www.w3.org/TR/2099/WD-shacl12-sparql-20991231/".to_owned()),
    )]);
    profiles["shacl-20170720"] = Value::object([(
        "specification",
        Value::String("https://www.w3.org/TR/2017/REC-shacl-20170720/".to_owned()),
    )]);
    let unsupported = community::select(&profiles, "shacl12-20991231").unwrap();
    assert!(matches!(unsupported, Selection::Unsupported(_)));
    // The neighbouring dated profile still resolves to its built-in law.
    assert_eq!(
        community::select(&profiles, "shacl-20170720").unwrap(),
        Selection::Supported(ShaclProfile::REC_20170720)
    );
    // A profile the catalog does not declare is a corpus error, not a status.
    assert!(community::select(&profiles, "shacl-20170721").is_err());

    let corpus = Corpus::acquire(&corpus_root()).unwrap();
    let cases = corpus.cases().unwrap();
    let case = cases
        .iter()
        .find(|case| case.case.id == "anchored-optional-iri-violating")
        .unwrap();
    let probe = corpus
        .execute_selected(case, "shacl12-20991231", &unsupported, &mut control)
        .unwrap();
    assert!(probe.unsupported());
    assert!(!probe.passed());
    assert_eq!(probe.records[0].observed.kind, OutcomeKind::Unsupported);
    let neighbour = corpus
        .execute(case, "shacl12-20260918", &mut control)
        .unwrap();
    assert!(neighbour.passed());
    let query = cases
        .iter()
        .find(|case| case.case.id == "mixed-predicate-inverse-sequence")
        .unwrap();
    let query_probe = corpus
        .execute_selected(query, "shacl12-20991231", &unsupported, &mut control)
        .unwrap();
    assert!(query_probe.unsupported());
    let totals = Totals::of(2, &[probe, neighbour, query_probe]);
    assert_eq!(
        (totals.passed, totals.failed, totals.unsupported),
        (1, 0, 2)
    );
    assert_eq!((totals.query_executions, totals.passed_queries), (1, 0));
    assert!(
        totals
            .scoreboard()
            .contains("COMMUNITY-SHACL: passed 1 total 2")
    );
    assert!(
        totals
            .scoreboard()
            .contains("COMMUNITY-QUERY: passed 0 total 1")
    );
}

#[test]
fn a_report_graded_against_another_expectation_is_a_mismatch() {
    let mut control =
        purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None);

    let corpus = Corpus::acquire(&corpus_root()).unwrap();
    let cases = corpus.cases().unwrap();
    let by_id = |id: &str| *cases.iter().find(|case| case.case.id == id).unwrap();
    // The draft admits a local VALUES that the Recommendation refuses outright;
    // running the draft case under the Recommendation must not pass.
    let draft = by_id("admission-local-values-draft");
    let crossed = corpus
        .execute_selected(
            &draft,
            "shacl-20170720",
            &Selection::Supported(ShaclProfile::REC_20170720),
            &mut control,
        )
        .unwrap();
    assert!(!crossed.passed());
    assert_eq!(
        crossed.records[0].observed.reason.as_deref(),
        Some("explicit-values")
    );
    assert!(
        corpus
            .execute(&draft, "shacl12-20260918", &mut control)
            .unwrap()
            .passed()
    );
}

#[test]
fn reason_tokens_follow_the_dated_law() {
    let rec = ShaclProfile::REC_20170720;
    let wd = ShaclProfile::WD_20260918;
    assert_eq!(
        community::admission_reason(rec, AdmissionReason::Values, None),
        "explicit-values"
    );
    assert_eq!(
        community::admission_reason(wd, AdmissionReason::Values, Some("this")),
        "prebound-values"
    );
    assert_eq!(
        community::admission_reason(rec, AdmissionReason::Service, None),
        "service"
    );
    assert_eq!(
        community::admission_reason(wd, AdmissionReason::Service, None),
        "service-policy"
    );
}

#[test]
fn changed_reviewed_bytes_are_refused_and_the_unchanged_copy_is_admitted() {
    let scratch = purrdf_testkit::TempDir::new_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let copy = scratch.path().join("community");
    copy_tree(&corpus_root(), &copy);
    // The untouched copy is admitted, at a different location, with every case.
    let admitted = Corpus::acquire(&copy).unwrap();
    assert_eq!(admitted.cases().unwrap().len(), 79);

    let shapes = copy.join("shacl/anchored-optional-iri-violating/shapes.ttl");
    let mut bytes = std::fs::read(&shapes).unwrap();
    bytes.extend_from_slice(b"\n# altered after review\n");
    std::fs::write(&shapes, bytes).unwrap();
    let error = Corpus::acquire(&copy).unwrap_err();
    assert!(error.contains("changed"), "{error}");
}

fn digest(bytes: &[u8]) -> String {
    purrdf_hash::hex::Digest32::new(*purrdf_hash::blake3::hash(bytes).as_bytes()).to_hex()
}

/// Rebind one deliberately changed boundary-test artifact by exact path, so
/// byte-identical files under other paths retain their original admission.
fn replace_reviewed_bytes(root: &Path, relative: &str, bytes: &[u8]) {
    std::fs::write(root.join(relative), bytes).unwrap();
    let path = root.join("reviews/index.json");
    let mut index = purrdf_lex::json::read_slice(
        &std::fs::read(&path).unwrap(),
        purrdf_lex::json::Limits::DEFAULT,
    )
    .unwrap();
    let Value::Array(cases) = &mut index["cases"] else {
        panic!("registry cases")
    };
    let mut changed = 0;
    for case in cases {
        let Value::Array(payloads) = &mut case["payloads"] else {
            panic!("review payloads")
        };
        for payload in payloads {
            if payload["path"].as_str() == Some(relative) {
                payload["blake3"] = Value::String(digest(bytes));
                changed += 1;
            }
        }
    }
    assert!(changed > 0, "the test artifact must already be reviewed");
    std::fs::write(path, purrdf_lex::json::write_pretty(&index)).unwrap();
}

/// Corrupt one reviewed expected report in a copy, re-bind its registry digest
/// so acquisition admits the corrupted bytes, and require that no route passes.
fn corrupted_expectation_fails(
    case_id: &str,
    from: &str,
    to: &str,
    control: &mut purrdf_core::CanonicalizationControl<'_>,
) {
    let scratch = purrdf_testkit::TempDir::new_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let copy = scratch.path().join("community");
    copy_tree(&corpus_root(), &copy);
    let manifest = copy.join(format!("shacl/{case_id}/manifest.ttl"));
    let original = std::fs::read_to_string(&manifest).unwrap();
    assert_eq!(original.matches(from).count(), 1, "{case_id}: {from}");
    let corrupted = original.replace(from, to);
    std::fs::write(&manifest, &corrupted).unwrap();
    let index = copy.join("reviews/index.json");
    let registry = std::fs::read_to_string(&index).unwrap();
    let (old, new) = (digest(original.as_bytes()), digest(corrupted.as_bytes()));
    assert!(registry.contains(&old));
    std::fs::write(&index, registry.replace(&old, &new)).unwrap();

    let corpus = Corpus::acquire(&copy).unwrap();
    let cases = corpus.cases().unwrap();
    let case = cases.iter().find(|case| case.case.id == case_id).unwrap();
    for profile in case.case.selected_profiles().unwrap() {
        let execution = corpus.execute(case, profile, control).unwrap();
        assert!(
            !execution.passed(),
            "{case_id} {profile} passed a corrupted oracle"
        );
        for record in &execution.records {
            assert!(
                matches!(
                    record.observed.kind,
                    OutcomeKind::Mismatch | OutcomeKind::Malformed
                ),
                "{case_id} {profile} {}: {:?} {}",
                record.surface,
                record.observed.kind,
                record.observed.message
            );
        }
    }
    // The uncorrupted original still passes under every selected profile.
    let original = Corpus::acquire(&corpus_root()).unwrap();
    let cases = original.cases().unwrap();
    let case = cases.iter().find(|case| case.case.id == case_id).unwrap();
    for profile in case.case.selected_profiles().unwrap() {
        assert!(original.execute(case, profile, control).unwrap().passed());
    }
}

#[test]
fn an_omitted_source_constraint_is_not_excused() {
    let mut control =
        purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None);

    corrupted_expectation_fails(
        "anchored-optional-blank-violating",
        "_:result0 sh:sourceConstraint ex:Constraint .\n",
        "",
        &mut control,
    );
}

#[test]
fn a_wrong_source_blank_is_not_repaired_by_report_isomorphism() {
    let mut control =
        purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None);

    // Moving the first result onto the conforming blank keeps the report graph
    // isomorphic to the original; only the source correspondence can tell.
    corrupted_expectation_fails(
        "anchored-optional-blank-violating",
        "sh:focusNode _:b ; sh:resultSeverity sh:Violation ; sh:sourceShape ex:Shape ; \
         sh:sourceConstraintComponent sh:SPARQLConstraintComponent .\n_:result0 sh:value _:b .",
        "sh:focusNode _:a ; sh:resultSeverity sh:Violation ; sh:sourceShape ex:Shape ; \
         sh:sourceConstraintComponent sh:SPARQLConstraintComponent .\n_:result0 sh:value _:a .",
        &mut control,
    );
}

#[test]
fn a_changed_path_topology_is_a_mismatch() {
    let mut control =
        purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None);

    corrupted_expectation_fails(
        "complex-sequence-inverse-path",
        "_:inverse sh:inversePath ex:q .",
        "_:inverse sh:inversePath ex:p .",
        &mut control,
    );
}

#[test]
fn a_missing_row_message_is_a_mismatch() {
    let mut control =
        purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None);

    corrupted_expectation_fails(
        "solution-message-priority",
        "_:result0 sh:resultMessage \"row message\" .",
        "_:result0 sh:resultMessage \"declared message\" .",
        &mut control,
    );
}

#[test]
fn a_suite_kind_without_a_runner_is_refused() {
    let scratch = purrdf_testkit::TempDir::new_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let copy = scratch.path().join("community");
    copy_tree(&corpus_root(), &copy);
    let catalog = copy.join("catalog.json");
    let text = std::fs::read_to_string(&catalog).unwrap();
    assert!(text.contains("\"kind\": \"shacl\""));
    std::fs::write(
        &catalog,
        text.replace("\"kind\": \"shacl\"", "\"kind\": \"shex\""),
    )
    .unwrap();
    let error = Corpus::acquire(&copy).unwrap_err();
    assert!(error.contains("no community native runner"), "{error}");
}

#[test]
fn query_ingress_preserves_artifact_bases_and_admitted_sources() {
    let scratch = purrdf_testkit::TempDir::new_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let relative = scratch.path().join("relative-community");
    copy_tree(&corpus_root(), &relative);
    let query_path = "sparql/service/service-bnode-distinct.rq";
    let text = "SELECT ?localSelf ?remoteSelf WHERE { ?localSelf <urn:p> <relative> . SERVICE <http://example.org/service/a> { ?remoteSelf <urn:p> <relative> } }";
    replace_reviewed_bytes(&relative, query_path, text.as_bytes());
    for path in ["sparql/service/local.ttl", "sparql/service/endpoint-a.ttl"] {
        replace_reviewed_bytes(&relative, path, b"<> <urn:p> <relative> .");
    }
    replace_reviewed_bytes(&relative, "sparql/service/service-bnode-distinct.srj", br#"{"head":{"vars":["localSelf","remoteSelf"]},"results":{"bindings":[{"localSelf":{"type":"uri","value":"http://example.org/community/sparql/service/local.ttl"},"remoteSelf":{"type":"uri","value":"http://example.org/community/sparql/service/endpoint-a.ttl"}}]}}"#);
    let mut control =
        purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None);
    let corpus = Corpus::acquire(&relative).unwrap();
    let cases = corpus.cases().unwrap();
    let case = cases
        .iter()
        .find(|case| case.case.id == "service-bnode-distinct")
        .unwrap();
    let execution = corpus
        .execute(case, "sparql11-query-over-rdf12", &mut control)
        .unwrap();
    assert!(execution.passed(), "{execution:#?}");
    // The same valid query with an implicit external source is refused before
    // the legacy workspace file loader can inspect that unreviewed path.
    let from = text.replace(
        " WHERE",
        " FROM <http://purrdf.test/suite/unreviewed.ttl> WHERE",
    );
    replace_reviewed_bytes(&relative, query_path, from.as_bytes());
    let corpus = Corpus::acquire(&relative).unwrap();
    let cases = corpus.cases().unwrap();
    let case = cases
        .iter()
        .find(|case| case.case.id == "service-bnode-distinct")
        .unwrap();
    let error = corpus
        .execute(case, "sparql11-query-over-rdf12", &mut control)
        .unwrap_err();
    assert!(
        error.contains("dataset clauses are not admitted"),
        "{error}"
    );

    let copy = scratch.path().join("community");
    copy_tree(&corpus_root(), &copy);
    assert!(Corpus::acquire(&copy).unwrap().cases().is_ok());
    let original = "sparql/interactions/mixed-predicate-inverse-sequence.srj";
    let replacement = "sparql/interactions/mixed-equivalent-union.srj";
    assert_eq!(
        std::fs::read(copy.join(original)).unwrap(),
        std::fs::read(copy.join(replacement)).unwrap()
    );
    let path = copy.join("sparql/interactions/inventory.json");
    let mut inventory = purrdf_lex::json::read_slice(
        &std::fs::read(&path).unwrap(),
        purrdf_lex::json::Limits::DEFAULT,
    )
    .unwrap();
    assert_eq!(
        inventory["cases"][0]["id"].as_str(),
        Some("mixed-predicate-inverse-sequence")
    );
    inventory["cases"][0]["expected"] = Value::String("mixed-equivalent-union.srj".to_owned());
    std::fs::write(path, purrdf_lex::json::write_pretty(&inventory)).unwrap();
    let path = copy.join("reviews/index.json");
    let mut index = purrdf_lex::json::read_slice(
        &std::fs::read(&path).unwrap(),
        purrdf_lex::json::Limits::DEFAULT,
    )
    .unwrap();
    let Value::Array(reviews) = &mut index["cases"] else {
        panic!("review registry cases")
    };
    let review = reviews
        .iter_mut()
        .find(|review| review["id"].as_str() == Some("mixed-predicate-inverse-sequence"))
        .unwrap();
    let Value::Array(payloads) = &mut review["payloads"] else {
        panic!("reviewed artifacts")
    };
    let payload = payloads
        .iter_mut()
        .find(|payload| payload["path"].as_str() == Some(original))
        .unwrap();
    payload["path"] = Value::String(replacement.to_owned());
    std::fs::write(path, purrdf_lex::json::write_pretty(&index)).unwrap();
    // The complete bytes and review hashes remain valid. Manifest association
    // independently refuses the substituted artifact before any query runs.
    let corpus = Corpus::acquire(&copy).unwrap();
    let error = corpus.cases().unwrap_err();
    assert!(error.contains("result artifact disagree"), "{error}");
}

/// The community EARL names one subject per (profile, surface) route, each
/// described, and no subject is asserted about one test twice.
#[test]
fn the_community_earl_asserts_each_subject_and_test_once() {
    let mut control =
        purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None);

    use purrdf_conformance_kit::outcome::{AssertorDescription, DescriptionTerm, to_earl};
    use purrdf_iri::vocab::{earl, rdf, rdfs};
    use std::collections::{BTreeMap, BTreeSet};

    let corpus = Corpus::acquire(&corpus_root()).unwrap();
    let (_, executions) = corpus.run(&mut control).unwrap();
    let records: Vec<_> = executions
        .iter()
        .flat_map(|execution| execution.records.iter().cloned())
        .collect();
    let base = "http://example.org/community/subject/purrdf-native";
    let report = community::earl_report(
        &records,
        base,
        AssertorDescription::new(
            "http://example.org/community/runner/community-conformance",
            vec![earl::ASSERTOR.to_owned()],
            Vec::new(),
        ),
        |profile, surface| {
            vec![(
                rdfs::LABEL.to_owned(),
                DescriptionTerm::Text(format!("{profile} {surface}")),
            )]
        },
    )
    .unwrap();
    let routes: BTreeSet<_> = records
        .iter()
        .map(|record| (record.profile.as_str(), record.surface.as_str()))
        .collect();
    assert!(routes.len() > 1, "the corpus has several routes");
    assert_eq!(report.subjects().len(), routes.len());
    assert_eq!(report.records().len(), records.len());
    let dataset = to_earl(&report).unwrap();
    let text = String::from_utf8(
        purrdf::serialize_dataset(
            dataset.as_ref(),
            "application/n-triples",
            purrdf::SerializeGraph::Dataset,
        )
        .unwrap(),
    )
    .unwrap();
    // Each assertion node's `earl:subject` and `earl:test`, read back from the
    // serialized graph.
    let object_of = |predicate: &str| -> BTreeMap<String, String> {
        let infix = format!(" <{predicate}> ");
        text.lines()
            .filter_map(|line| {
                let (node, rest) = line.split_once(&infix)?;
                Some((node.to_owned(), rest.strip_suffix(" .")?.to_owned()))
            })
            .collect()
    };
    let (subjects, tests) = (object_of(earl::SUBJECT), object_of(earl::TEST));
    assert_eq!(subjects.len(), records.len());
    let mut pairs: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for (node, subject) in &subjects {
        *pairs.entry((subject, &tests[node])).or_default() += 1;
    }
    assert_eq!(pairs.len(), records.len());
    assert!(pairs.values().all(|&count| count == 1), "{pairs:?}");
    for subject in report.subjects() {
        let iri = subject.iri();
        assert!(iri.starts_with(&format!("{base}#profile=")));
        assert!(
            text.lines()
                .any(|line| line.starts_with(&format!("<{iri}> <{}> ", rdfs::LABEL))),
            "{iri} is described"
        );
        assert!(text.contains(&format!(
            "<{iri}> <{}> <{}> .",
            rdf::TYPE,
            earl::TEST_SUBJECT
        )));
    }
}

/// A profile or surface value holding the field separators cannot make two
/// different routes share a subject IRI.
#[test]
fn route_subject_iris_stay_distinct_when_values_hold_separators() {
    use purrdf_conformance_kit::outcome::{AssertorDescription, Observation, Record};
    let observation = Observation {
        kind: OutcomeKind::Compared,
        reason: None,
        message: String::new(),
    };
    let record = |profile: &str, surface: &str| Record {
        case: "http://example.org/t".to_owned(),
        profile: profile.to_owned(),
        surface: surface.to_owned(),
        expected: observation.clone(),
        observed: observation.clone(),
        passed: true,
        artifacts: Vec::new(),
    };
    // Unencoded, both routes would read `#profile=a&surface=b&surface=c`.
    let records = [record("a&surface=b", "c"), record("a", "b&surface=c")];
    let report = community::earl_report(
        &records,
        "http://example.org/s",
        AssertorDescription::new("http://example.org/a", Vec::new(), Vec::new()),
        |_, _| Vec::new(),
    )
    .unwrap();
    let iris: Vec<_> = report
        .subjects()
        .iter()
        .map(purrdf_conformance_kit::outcome::Described::iri)
        .collect();
    assert_eq!(iris.len(), 2);
    assert_ne!(iris[0], iris[1]);
    for iri in iris {
        let fragment = iri.split_once('#').unwrap().1;
        assert_eq!(fragment.matches('&').count(), 1, "{iri}");
        assert_eq!(fragment.matches('=').count(), 2, "{iri}");
    }
}
