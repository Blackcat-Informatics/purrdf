// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Freeze native positive-envelope bytes for the exact packaged host probe.

use purrdf_core::{CanonicalizationControl, RDFC_V2_CORPUS_POLICY};
use purrdf_iri::vocab::rdf;
use purrdf_lex::json::{Object, Value, write_pretty};
use purrdf_validate::regime::{PositiveReasonerSession, positive_limits_from_json};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let destination = std::env::args_os().nth(1).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "supply the positive host vector destination",
        )
    })?;
    let limits = r#"{"historySteps":"16","sourceTerms":"512","sourceRows":"512","questionBytes":"8192","metadataBytes":"4194304","metadataWork":"4194304"}"#;
    let initial = format!("<urn:a> <{}> <urn:C> .\n", rdf::TYPE);
    let axiom = format!("<urn:b> <{}> <urn:C> .", rdf::TYPE);
    let latest = format!("{initial}{axiom}\n");
    let producer = PositiveReasonerSession::open_with_proofs(
        &initial,
        positive_limits_from_json(limits)?,
        0,
        0,
        None,
        &mut CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None),
    )?;
    let (producer, _) = producer.class_satisfiability("<urn:query>")?;
    let (producer, _) = producer.advance(
        &latest,
        &mut CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None),
    )?;
    let (_, answer) = producer.entails(&axiom)?;
    if !answer.answer().starts_with("entails true\n") {
        return Err(std::io::Error::other("the positive vector's original answer changed").into());
    }
    let proof = answer
        .proof()
        .ok_or_else(|| std::io::Error::other("the native proof is absent"))?;
    let consumer = PositiveReasonerSession::open_with_proofs(
        &latest,
        positive_limits_from_json(limits)?,
        0,
        0,
        None,
        &mut CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None),
    )?;
    let report = consumer.check_positive_proof(
        "entails",
        &axiom,
        answer.answer(),
        answer.certificate(),
        proof,
        &mut CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None),
    )?;
    let value = Value::from(
        Object::new()
            .with("initial", initial)
            .with("latest", latest)
            .with("axiom", axiom)
            .with("limits", limits)
            .with("answer", answer.answer())
            .with("certificate", answer.certificate())
            .with("proof", proof)
            .with("report", report),
    );
    std::fs::write(destination, format!("{}\n", write_pretty(&value)))?;
    Ok(())
}
