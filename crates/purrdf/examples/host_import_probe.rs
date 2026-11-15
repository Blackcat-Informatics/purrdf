// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A module that links the library crates the way a bare WebAssembly embedder does,
//! built so its import section can be read.
//!
//! `make wasm-imports` builds this for `wasm32-unknown-unknown` and refuses the
//! module if it imports anything. The export parses a document and evaluates a
//! query three ways, so the parser, the evaluator and both of the evaluator's host
//! reads are in the linked module: with the query time and entropy seed supplied,
//! with neither (the read a build with no host source refuses), and through an
//! installed source. It is linked, not run; `purrdf-sparql-eval`'s `host_source`
//! test executes the same calls.

use std::time::Duration;

use purrdf::sparql::host_source::{self, HostSource};
use purrdf::sparql::{NativeSparqlEngine, QueryOptions, WallDeadline};
use purrdf_core::SparqlRequest;

/// A source with nothing behind it: an embedder's own clock and entropy go here.
#[derive(Debug)]
struct Fixed;

impl HostSource for Fixed {
    fn unix_millis(&self) -> f64 {
        0.0
    }

    fn entropy_seed(&self) -> u64 {
        0
    }
}

static FIXED: Fixed = Fixed;

const DOCUMENT: &[u8] = b"<http://example.org/s> <http://example.org/p> 1 .";
const QUERY: &str = "SELECT ?s (NOW() AS ?time) (RAND() AS ?random) WHERE { ?s ?p ?o }";

/// One bit per step that answered: the parse, the supplied read, the default read,
/// the installation, the read through the source, and a wall deadline.
#[allow(
    unsafe_code,
    reason = "an unmangled export is what keeps the code in the linked module"
)]
#[unsafe(no_mangle)]
pub extern "C" fn purrdf_host_import_probe() -> u32 {
    let Ok(dataset) = purrdf::parse_dataset(DOCUMENT, "text/turtle", None) else {
        return 0;
    };
    let engine = NativeSparqlEngine::new();
    let request = SparqlRequest {
        query: QUERY,
        base_iri: None,
        substitutions: &[],
    };
    let now = purrdf::xsd::datetime_from_unix_seconds(0);
    let supplied = QueryOptions::new().with_now(&now).with_entropy_seed(7);
    let mut answered = 1;
    for (bit, install, options) in [
        (2, false, supplied),
        (4, false, QueryOptions::EMPTY),
        (16, true, QueryOptions::EMPTY),
    ] {
        if install && host_source::install(&FIXED).is_ok() {
            answered |= 8;
        }
        if engine
            .query_with_options_view(&dataset, request, &options)
            .is_ok()
        {
            answered |= bit;
        }
    }
    if WallDeadline::try_after(Duration::from_millis(1)).is_ok() {
        answered |= 32;
    }
    answered
}
