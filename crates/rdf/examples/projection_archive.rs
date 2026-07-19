// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Project RDF into a deterministic LPG USTAR archive and lift it back.

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use purrdf_rdf::{
    LiftProfile, LpgConfig, LpgExecutionLimits, LpgScope, ProjectionConfig, ProjectionLimits,
    ProjectionProfile, datasets_isomorphic, lift_archive, parse_dataset, project_archive,
};

fn main() -> Result<(), Box<dyn Error>> {
    let output = PathBuf::from(
        env::args_os()
            .nth(1)
            .ok_or("usage: projection_archive OUTPUT_USTAR")?,
    );
    let dataset = parse_dataset(
        b"@prefix ex: <https://example.org/> . ex:alice ex:knows ex:bob .\n",
        "text/turtle",
        None,
    )?;
    let limits = ProjectionLimits::new(16, 1_000_000, 4_000_000, 5_000_000, 16)?;
    let config = ProjectionConfig::LpgCsv(LpgConfig::new(
        "https://example.org/type",
        LpgScope::all(),
        limits,
        LpgExecutionLimits::new(1_000, 1_000, 1_000, 1_000)?,
    )?);

    let projected = project_archive(dataset.as_ref(), ProjectionProfile::LpgCsv, &config)?;
    fs::write(&output, &projected.archive)?;
    let lifted = lift_archive(&projected.archive, LiftProfile::LpgCsv, &config)?;
    if !datasets_isomorphic(&dataset, &lifted.dataset) {
        return Err("projection round trip changed the RDF dataset".into());
    }

    println!(
        "wrote {} bytes to {} with {} RDF-to-LPG loss record(s)",
        projected.archive.len(),
        output.display(),
        projected.loss_ledger.entries().len()
    );
    Ok(())
}
