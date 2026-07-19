// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Project caller-classified RDF resources into a deterministic OKF v0.1 bundle.

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use purrdf_rdf::{ProjectionConfig, parse_dataset, project_okf_terms};

const CONFIG: &[u8] = include_bytes!("../tests/fixtures/okf-terms.json");

fn main() -> Result<(), Box<dyn Error>> {
    let output = PathBuf::from(
        env::args_os()
            .nth(1)
            .ok_or("usage: okf_terms OUTPUT_USTAR")?,
    );
    let dataset = parse_dataset(
        br#"@prefix ex: <https://example.org/> .
ex:Agent ex:type ex:Class ;
    ex:label "Agent" ;
    ex:description "An acting resource." ;
    ex:tag "core" ;
    ex:related ex:knows .
ex:knows ex:type ex:Property ; ex:label "knows" .
"#,
        "text/turtle",
        None,
    )?;
    let ProjectionConfig::OkfTerms(config) = ProjectionConfig::from_json(CONFIG)? else {
        return Err("embedded configuration is not tagged okf-terms".into());
    };
    let projection = project_okf_terms(dataset.as_ref(), &config)?;
    if !projection.loss_ledger.is_empty() {
        return Err("example configuration failed to represent every source statement".into());
    }
    let archive = projection.package.to_ustar()?;
    fs::write(&output, &archive)?;
    println!(
        "wrote {} concepts and {} categories as {} deterministic bytes to {}",
        projection.report.concepts,
        projection.report.categories,
        archive.len(),
        output.display()
    );
    Ok(())
}
