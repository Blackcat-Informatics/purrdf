// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Project and lift one RDF dataset through every research-object carrier.

use purrdf_rdf::{
    LiftProfile, ProjectionConfig, ProjectionProfile, lift_archive, parse_dataset, project_archive,
};

const SOURCE: &[u8] = include_bytes!("../tests/fixtures/research-objects/carrier/shared.ttl");
const CASES: [(ProjectionProfile, LiftProfile, &[u8]); 5] = [
    (
        ProjectionProfile::Croissant11,
        LiftProfile::Croissant11,
        include_bytes!("../tests/fixtures/research-objects/carrier/croissant-1.1.json"),
    ),
    (
        ProjectionProfile::RoCrate13,
        LiftProfile::RoCrate13,
        include_bytes!("../tests/fixtures/research-objects/carrier/ro-crate-1.3.json"),
    ),
    (
        ProjectionProfile::DataCite46,
        LiftProfile::DataCite46,
        include_bytes!("../tests/fixtures/research-objects/carrier/datacite-4.6.json"),
    ),
    (
        ProjectionProfile::Dcat3,
        LiftProfile::Dcat3,
        include_bytes!("../tests/fixtures/research-objects/carrier/dcat-3.json"),
    ),
    (
        ProjectionProfile::FrictionlessDataPackage1,
        LiftProfile::FrictionlessDataPackage1,
        include_bytes!(
            "../tests/fixtures/research-objects/carrier/frictionless-data-package-1.json"
        ),
    ),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dataset = parse_dataset(SOURCE, "text/turtle", None)?;

    for (project_profile, lift_profile, config_bytes) in CASES {
        let config = ProjectionConfig::from_json(config_bytes)?;
        let first = project_archive(dataset.as_ref(), project_profile, &config)?;
        let second = project_archive(dataset.as_ref(), project_profile, &config)?;
        assert_eq!(first.archive, second.archive);

        let lifted = lift_archive(&first.archive, lift_profile, &config)?;
        let rewritten = project_archive(lifted.dataset.as_ref(), project_profile, &config)?;
        assert_eq!(first.archive, rewritten.archive);

        println!(
            "{}: {} archive bytes, {} project losses, {} lift losses",
            project_profile,
            first.archive.len(),
            first.loss_ledger.entries().len(),
            lifted.loss_ledger.entries().len()
        );
    }

    Ok(())
}
