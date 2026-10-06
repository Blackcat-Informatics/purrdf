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
    let corpus = Corpus::acquire(&corpus_root()).unwrap();
    let (totals, executions) = corpus.run().unwrap();
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
    assert_eq!(totals.cases, 64);
    assert_eq!(totals.executions, 115);
    assert_eq!(totals.unsupported, 0, "both corpus profiles are built in");
    assert_eq!(totals.failed, 0);
    assert_eq!(totals.passed, 115);
    assert_eq!(totals.passed_observations, totals.observations);
    assert_eq!(
        totals.profiles,
        [
            ("shacl-20170720".to_owned(), [57, 57, 0, 0]),
            ("shacl12-20260918".to_owned(), [58, 58, 0, 0]),
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
            routes == ["native/rules"]
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
fn an_unimplemented_profile_is_reported_apart_from_failures() {
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
        .execute_selected(case, "shacl12-20991231", &unsupported)
        .unwrap();
    assert!(probe.unsupported());
    assert!(!probe.passed());
    assert_eq!(probe.records[0].observed.kind, OutcomeKind::Unsupported);
    let neighbour = corpus.execute(case, "shacl12-20260918").unwrap();
    assert!(neighbour.passed());
    let totals = Totals::of(1, &[probe, neighbour]);
    assert_eq!(
        (totals.passed, totals.failed, totals.unsupported),
        (1, 0, 1)
    );
}

#[test]
fn a_report_graded_against_another_expectation_is_a_mismatch() {
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
        )
        .unwrap();
    assert!(!crossed.passed());
    assert_eq!(
        crossed.records[0].observed.reason.as_deref(),
        Some("explicit-values")
    );
    assert!(corpus.execute(&draft, "shacl12-20260918").unwrap().passed());
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
    assert_eq!(admitted.cases().unwrap().len(), 64);

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

/// Corrupt one reviewed expected report in a copy, re-bind its registry digest
/// so acquisition admits the corrupted bytes, and require that no route passes.
fn corrupted_expectation_fails(case_id: &str, from: &str, to: &str) {
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
        let execution = corpus.execute(case, profile).unwrap();
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
        assert!(original.execute(case, profile).unwrap().passed());
    }
}

#[test]
fn an_omitted_source_constraint_is_not_excused() {
    corrupted_expectation_fails(
        "anchored-optional-blank-violating",
        "_:result0 sh:sourceConstraint ex:Constraint .\n",
        "",
    );
}

#[test]
fn a_wrong_source_blank_is_not_repaired_by_report_isomorphism() {
    // Moving the first result onto the conforming blank keeps the report graph
    // isomorphic to the original; only the source correspondence can tell.
    corrupted_expectation_fails(
        "anchored-optional-blank-violating",
        "sh:focusNode _:b ; sh:resultSeverity sh:Violation ; sh:sourceShape ex:Shape ; \
         sh:sourceConstraintComponent sh:SPARQLConstraintComponent .\n_:result0 sh:value _:b .",
        "sh:focusNode _:a ; sh:resultSeverity sh:Violation ; sh:sourceShape ex:Shape ; \
         sh:sourceConstraintComponent sh:SPARQLConstraintComponent .\n_:result0 sh:value _:a .",
    );
}

#[test]
fn a_changed_path_topology_is_a_mismatch() {
    corrupted_expectation_fails(
        "complex-sequence-inverse-path",
        "_:inverse sh:inversePath ex:q .",
        "_:inverse sh:inversePath ex:p .",
    );
}

#[test]
fn a_missing_row_message_is_a_mismatch() {
    corrupted_expectation_fails(
        "solution-message-priority",
        "_:result0 sh:resultMessage \"row message\" .",
        "_:result0 sh:resultMessage \"declared message\" .",
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
    assert!(error.contains("no community SHACL runner"), "{error}");
}
