// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The same complete boundary artifact on native and actual WASM.

/// Execute all seventeen committed service/schema vectors through their production checker.
fn all_shared_dl_proof_vectors_match_the_current_native_boundary() {
    let cases = purrdf_validate::regime::dl_proof_golden_vectors().unwrap();
    assert_eq!(cases.len(), 17, "the complete service and schema inventory");
    purrdf_validate::regime::check_dl_proof_golden_vectors().unwrap();
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
            producer.prove("consistency", "").unwrap()
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
                )
                .is_err()
        );
    }
}

purrdf_testkit::harness_main!(
    all_shared_dl_proof_vectors_match_the_current_native_boundary,
    imported_closure_proofs_check_against_the_consumers_native_snapshot,
);
