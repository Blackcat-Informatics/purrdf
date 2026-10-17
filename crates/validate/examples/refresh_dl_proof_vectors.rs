// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicitly refresh only the proof/check bodies of the shared first-party vectors.
//! Ordinary tests compare the committed artifact; they never invoke this producer.

use purrdf_validate::regime::{
    DL_PROOF_GOLDEN_VECTORS, PROOF_SERVICE_NAMES, check_dl_proof, dl_proof_golden_vectors,
    prove_to_string,
};
use std::collections::BTreeMap;
use std::io;

/// Verify every original answer, then refresh only the artifact's proof/check bodies.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dl-proof.vectors");
    let source = std::fs::read_to_string(&path)?;
    if source != DL_PROOF_GOLDEN_VECTORS {
        return Err(io::Error::other("proof vector source changed after compilation").into());
    }
    let cases = dl_proof_golden_vectors().map_err(io::Error::other)?;
    for service in PROOF_SERVICE_NAMES {
        if !cases.iter().any(|case| case.service() == service) {
            return Err(io::Error::other(format!("missing proof service {service}")).into());
        }
    }
    let mut produced = BTreeMap::new();
    for case in &cases {
        let proof = prove_to_string(
            case.input(),
            case.service(),
            case.argument().trim_end(),
            0,
            0,
        )?;
        if proof.answer() != case.answer() {
            return Err(
                io::Error::other(format!("{}: expected answer changed", case.name())).into(),
            );
        }
        let check = check_dl_proof(
            case.input(),
            case.service(),
            case.argument().trim_end(),
            proof.answer(),
            proof.certificate(),
            proof.proof_document(),
        )?;
        if produced
            .insert(case.name(), (proof.proof_document().to_owned(), check))
            .is_some()
        {
            return Err(io::Error::other(format!("duplicate case {}", case.name())).into());
        }
    }
    let mut output = String::new();
    let mut current = None;
    let mut replacing = false;
    for line in source.lines() {
        if let Some(name) = line.strip_prefix("@case ") {
            current = Some(
                produced
                    .get(name)
                    .ok_or_else(|| io::Error::other("unknown case"))?,
            );
        }
        if line.starts_with('@') {
            replacing = false;
            output.push_str(line);
            output.push('\n');
            if matches!(line, "@proof" | "@check") {
                let (proof, check) =
                    current.ok_or_else(|| io::Error::other("body outside a case"))?;
                output.push_str(if line == "@proof" { proof } else { check });
                replacing = true;
            }
        } else if !replacing {
            output.push_str(line);
            output.push('\n');
        }
    }
    if output != source {
        std::fs::write(path, output)?;
    }
    println!(
        "refreshed proof/check bodies for {} original cases",
        cases.len()
    );
    Ok(())
}
