// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Paths and result wrappers shared by the `purrdf-sparql-conformance` integration
//! tests: each is written once here rather than copied into every file that needs it.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::path::{Path, PathBuf};

use purrdf_core::{RdfDatasetBuilder, SparqlResult};

/// The vendored conformance suites' root, `suite/` under this package.
pub fn suite_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("suite")
}

/// Every `manifest.ttl` under [`suite_root`], in path order.
pub fn suite_manifests() -> Vec<PathBuf> {
    purrdf_sparql_conformance::paths::suite_manifests(&suite_root())
        .unwrap_or_else(|error| panic!("discover the suite manifests: {error}"))
        .into_iter()
        .map(|manifest| manifest.path)
        .collect()
}

/// Wrap decoded `SELECT` solutions as the model-level [`SparqlResult`] the writers take
/// and the comparator compares. The `aux` dataset is always empty: SRJ/SRX carry no
/// auxiliary graph.
pub fn as_solutions(parsed: purrdf_sparql_results::ParsedSolutions) -> SparqlResult {
    SparqlResult::Solutions {
        variables: parsed.variables,
        rows: parsed.rows,
        aux: RdfDatasetBuilder::new()
            .freeze()
            .expect("an empty dataset always freezes"),
    }
}
