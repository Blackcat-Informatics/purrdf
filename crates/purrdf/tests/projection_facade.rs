// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A downstream reaches the complete projection carrier through `purrdf` alone.

use purrdf::{
    LiftProfile, LpgConfig, ProjectionConfig, ProjectionLimits, ProjectionProfile,
    datasets_isomorphic, lift_archive, parse_dataset, project_archive,
};

#[test]
fn umbrella_facade_projects_and_lifts_without_subcrate_imports() {
    let dataset = parse_dataset(
        b"@prefix ex: <https://example.org/> . ex:s ex:p ex:o .\n",
        "text/turtle",
        None,
    )
    .expect("Turtle");
    let limits =
        ProjectionLimits::new(16, 1_000_000, 4_000_000, 5_000_000, 16).expect("portable limits");
    let config = ProjectionConfig::LpgCsv(
        LpgConfig::new("https://example.org/type", limits, 1_000).expect("LPG config"),
    );

    let first =
        project_archive(dataset.as_ref(), ProjectionProfile::LpgCsv, &config).expect("project");
    let second = project_archive(dataset.as_ref(), ProjectionProfile::LpgCsv, &config)
        .expect("project deterministically");
    assert_eq!(first.archive, second.archive);
    assert!(!first.loss_ledger.is_empty());

    let lifted = lift_archive(&first.archive, LiftProfile::LpgCsv, &config).expect("lift");
    assert!(datasets_isomorphic(&dataset, &lifted.dataset));
}
