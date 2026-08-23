// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared fixture for the pack-input benchmarks: a large pack and its bytes.

use std::io::Write as _;

use purrdf_core::{PackBuilder, RdfDatasetBuilder};
use tempfile::NamedTempFile;

/// The number of instances in the fixture. Each is typed to the bottom class of a
/// shallow `subClassOf` hierarchy, so RDFS re-types every instance up the hierarchy —
/// real, LINEAR closure work (a deep chain would blow the RDFS closure up O(n²) and
/// exhaust the datalog budget). The pack is genuinely large: several thousand triples,
/// well past a memory page.
const INSTANCES: usize = 3_000;

/// The depth of the `subClassOf` hierarchy the instances are typed into.
const HIERARCHY_DEPTH: usize = 3;

const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// Build the fixture pack: a shallow hierarchy `c0 ⊑ c1 ⊑ … ⊑ c{DEPTH}` and
/// [`INSTANCES`] instances of `c0`. Returns the temp file holding the pack bytes (for
/// the on-disk acquisition path) and the same bytes owned (for the owned/verify/view
/// paths).
///
/// # Panics
///
/// Panics on any fixture-construction failure — a bench with a broken fixture has
/// nothing to measure.
#[must_use]
pub(crate) fn large_pack() -> (NamedTempFile, Vec<u8>) {
    let mut builder = RdfDatasetBuilder::new();
    let sub = builder.intern_iri(RDFS_SUBCLASSOF);
    let ty = builder.intern_iri(RDF_TYPE);

    let classes: Vec<_> = (0..=HIERARCHY_DEPTH)
        .map(|i| builder.intern_iri(&format!("http://example.org/c{i}")))
        .collect();
    for window in classes.windows(2) {
        builder.push_quad(window[0], sub, window[1], None);
    }
    for i in 0..INSTANCES {
        let instance = builder.intern_iri(&format!("http://example.org/inst{i}"));
        builder.push_quad(instance, ty, classes[0], None);
    }

    let dataset = builder.freeze().expect("freeze bench dataset");
    let bytes = PackBuilder::build_bytes(&dataset).expect("build pack bytes");

    let mut file = NamedTempFile::new().expect("temp file");
    file.write_all(&bytes).expect("write pack bytes");
    file.flush().expect("flush pack bytes");
    (file, bytes)
}
