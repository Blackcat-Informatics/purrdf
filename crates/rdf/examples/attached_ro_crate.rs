// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Project one RDF dataset and an explicit payload carrier into an attached RO-Crate 1.3.

use purrdf_rdf::{
    ProjectionConfig, ProjectionProfile, RoCrateAssets, parse_dataset, project_archive_with_assets,
};

const SOURCE: &str = include_str!("../tests/fixtures/research-objects/carrier/shared.ttl");
const CONFIG: &str = include_str!("../tests/fixtures/research-objects/carrier/ro-crate-1.3.json");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = SOURCE.replace("files/train.csv", "data/train.csv").replace(
        "\"42\"^^<https://example.org/rdf/role-50>",
        "\"3\"^^<https://example.org/rdf/role-50>",
    );
    let config_json = CONFIG.replace("\"metadata-only\"", "\"attached\"");
    let config = ProjectionConfig::from_json(config_json.as_bytes())?;
    let dataset = parse_dataset(source.as_bytes(), "text/turtle", None)?;
    let assets =
        RoCrateAssets::from_artifacts(config.limits(), [("data/train.csv", b"cat".as_slice())])?;

    let crate_archive = project_archive_with_assets(
        dataset.as_ref(),
        ProjectionProfile::RoCrate13,
        &config,
        &assets,
    )?;
    println!("{} bytes", crate_archive.archive.len());
    Ok(())
}
