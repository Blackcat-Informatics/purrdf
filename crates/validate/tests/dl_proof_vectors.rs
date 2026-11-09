// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The same complete boundary artifact on native and actual WASM.

/// Execute all seventeen committed service/schema vectors through their production checker.
fn all_shared_dl_proof_vectors_match_the_current_native_boundary() {
    let cases = purrdf_validate::regime::dl_proof_golden_vectors().unwrap();
    assert_eq!(cases.len(), 17, "the complete service and schema inventory");
    purrdf_validate::regime::check_dl_proof_golden_vectors(
        &mut purrdf_core::CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None),
    )
    .unwrap();
}

fn imported_closure_proofs_check_against_the_consumers_native_snapshot() {
    use purrdf_iri::vocab::{owl, rdf, rdfs};
    use purrdf_validate::regime::ReasonerSession;

    for (blank, derived) in [(false, false), (true, false), (false, true)] {
        let subject = if blank {
            "_:tom"
        } else {
            "<http://example.org/tom>"
        };
        let extra = if derived {
            format!("{subject} <http://example.org/p> <http://example.org/jerry> .\n")
        } else {
            format!("{subject} <{}> <http://example.org/Dog> .\n", rdf::TYPE)
        };
        let premise = format!(
            "<http://example.org/o> <{}> <{}> .\n<http://example.org/o> <{}> <http://example.org/schema> .\n{subject} <{}> <http://example.org/Cat> .\n{extra}",
            rdf::TYPE,
            owl::ONTOLOGY,
            owl::IMPORTS,
            rdf::TYPE,
        );
        let domain = if derived {
            format!(
                "<http://example.org/p> <{}> <http://example.org/Dog> .\n",
                rdfs::DOMAIN
            )
        } else {
            String::new()
        };
        // Equal blank labels in different source documents must not alias.
        let unrelated = if blank {
            format!("_:tom <{}> <http://example.org/Noise> .\n", rdf::TYPE)
        } else {
            String::new()
        };
        let schema = format!(
            "<http://example.org/Cat> <{}> <http://example.org/Dog> .\n{domain}{unrelated}",
            owl::DISJOINT_WITH,
        );
        let imports = [("http://example.org/schema", schema.as_str())];
        let produced = {
            let mut producer =
                ReasonerSession::open_premise(&premise, &imports, &[], 0, 0, true).unwrap();
            producer
                .prove(
                    "consistency",
                    "",
                    &mut purrdf_core::CanonicalizationControl::new(
                        purrdf_core::RDFC_V2_CORPUS_POLICY,
                        None,
                    ),
                )
                .unwrap()
        };
        assert_eq!(produced.answer(), "consistency false\n");
        // This is an independent consumer source load, with no recording or
        // previous search. It shares neither a context nor a store with the producer.
        let consumer = ReasonerSession::open_premise(&premise, &imports, &[], 0, 0, false).unwrap();
        assert!(!consumer.records_proofs());
        let checked = consumer
            .check_proof(
                "consistency",
                "",
                produced.answer(),
                produced.certificate(),
                produced.proof_document(),
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None,
                ),
            )
            .unwrap();
        assert!(checked.contains("\nanswer checked 0\n"));
        assert!(checked.contains("\nunattested 0\n"));
        assert!(
            consumer
                .check_proof(
                    "consistency",
                    "",
                    "consistency true\n",
                    produced.certificate(),
                    produced.proof_document(),
                    &mut purrdf_core::CanonicalizationControl::new(
                        purrdf_core::RDFC_V2_CORPUS_POLICY,
                        None
                    ),
                )
                .is_err()
        );
        let forged = produced.proof_document().replacen("input ", "input 0", 1);
        assert_ne!(forged, produced.proof_document());
        assert!(
            consumer
                .check_proof(
                    "consistency",
                    "",
                    produced.answer(),
                    produced.certificate(),
                    &forged,
                    &mut purrdf_core::CanonicalizationControl::new(
                        purrdf_core::RDFC_V2_CORPUS_POLICY,
                        None
                    ),
                )
                .is_err()
        );
        let other_schema = schema.replace(owl::DISJOINT_WITH, rdfs::SUB_CLASS_OF);
        let other_imports = [("http://example.org/schema", other_schema.as_str())];
        let other =
            ReasonerSession::open_premise(&premise, &other_imports, &[], 0, 0, false).unwrap();
        assert!(
            other
                .check_proof(
                    "consistency",
                    "",
                    produced.answer(),
                    produced.certificate(),
                    produced.proof_document(),
                    &mut purrdf_core::CanonicalizationControl::new(
                        purrdf_core::RDFC_V2_CORPUS_POLICY,
                        None
                    ),
                )
                .is_err()
        );
    }
}

purrdf_testkit::harness_main!(
    all_shared_dl_proof_vectors_match_the_current_native_boundary,
    imported_closure_proofs_check_against_the_consumers_native_snapshot,
    positive_limits_preserve_strict_fields_and_original_integer_failure,
    positive_transport_checks_complete_operands_and_retains_revision_caps,
);

fn positive_limits_preserve_strict_fields_and_original_integer_failure() {
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::Capture;
    use purrdf_validate::regime::{ReasoningError, positive_limits_from_json};

    let text = r#"{"historySteps":"16","sourceTerms":"512","sourceRows":"512","questionBytes":"8192","metadataBytes":"4194304","metadataWork":"4194304"}"#;
    positive_limits_from_json(text).unwrap();
    for bad in [
        text.replace("\"16\"", "16"),
        text.replace("\"16\"", "\"016\""),
        text.replace("\"16\"", "\"+16\""),
        text.replace("\"historySteps\":\"16\",", ""),
        text.replace("\"historySteps\"", "\"unknown\""),
        text.replace(
            "\"historySteps\":\"16\"",
            "\"historySteps\":\"16\",\"historySteps\":\"16\"",
        ),
    ] {
        assert!(positive_limits_from_json(&bad).is_err(), "{bad}");
    }
    let offered = "184467440737095516160";
    let bad = text.replace("\"16\"", &format!("\"{offered}\""));
    let ReasoningError::PositiveLimits { cause, .. } = positive_limits_from_json(&bad).unwrap_err()
    else {
        panic!("expected original positive limit refusal");
    };
    assert_eq!(cause.pointer(), "/historySteps");
    let captured = Capture::read(cause.native_cause().unwrap());
    assert_eq!(captured.code, "positive-limit-integer");
    assert!(matches!(captured.field("offered"), ErrorValue::Text(value) if value == offered));
    assert!(matches!(
        captured.field("target"),
        ErrorValue::Code("usize")
    ));
    assert_eq!(captured.child("cause").code, "std-integer-parse");
    let original = std::error::Error::source(cause.native_cause().unwrap()).unwrap();
    assert!(original.is::<core::num::ParseIntError>());
}

fn positive_transport_checks_complete_operands_and_retains_revision_caps() {
    use purrdf_core::{CanonicalizationControl, RDFC_V2_CORPUS_POLICY};
    use purrdf_lex::json::{self, Limits};
    use purrdf_validate::regime::{
        PositiveReasonerSession, decode_dl_proof, positive_limits_from_json,
    };

    let vector = json::read_slice(
        include_bytes!("fixtures/positive-host.json"),
        Limits::DEFAULT,
    )
    .unwrap();
    let field = |name| vector.get(name).unwrap().as_str().unwrap();
    let control = || CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None);
    let open = |source, steps, work| {
        PositiveReasonerSession::open_with_proofs(
            source,
            positive_limits_from_json(field("limits")).unwrap(),
            steps,
            work,
            None,
            &mut control(),
        )
        .unwrap()
    };
    let producer = open(field("initial"), 7, 11);
    let before = producer.consistency().unwrap();
    let (producer, _) = producer.advance(field("latest"), &mut control()).unwrap();
    let after = producer.consistency().unwrap();
    for certificate in [before.certificate(), after.certificate()] {
        assert!(certificate.contains("\nbudget 7\n"), "{certificate}");
        assert!(certificate.contains("\nwork-budget 11\n"), "{certificate}");
    }
    let producer = open(field("initial"), 0, 0);
    let (producer, _) = producer.class_satisfiability("<urn:query>").unwrap();
    let (producer, _) = producer.advance(field("latest"), &mut control()).unwrap();
    let (_, produced) = producer.entails(field("axiom")).unwrap();
    assert_eq!(produced.answer(), field("answer"));
    assert_eq!(produced.certificate(), field("certificate"));
    assert_eq!(produced.proof_document(), field("proof"));
    assert!(decode_dl_proof(field("proof")).is_err());
    let consumer = open(field("latest"), 0, 0);
    let check = |answer, certificate, proof| {
        consumer.check_positive_proof(
            "entails",
            field("axiom"),
            answer,
            certificate,
            proof,
            &mut control(),
        )
    };
    assert!(check("entails false\n", field("certificate"), field("proof")).is_err());
    assert!(check(field("answer"), "invalid certificate", field("proof")).is_err());
    let forged = field("proof").replacen("input ", "input 0", 1);
    assert_ne!(forged, field("proof"));
    assert!(check(field("answer"), field("certificate"), &forged).is_err());
    assert_eq!(
        check(field("answer"), field("certificate"), field("proof")).unwrap(),
        field("report")
    );
    consumer.consistency().unwrap();
}
