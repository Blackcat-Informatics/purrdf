// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native Store / SPARQL / parse / canonicalize surface for the `purrdf` Python
//! extension — the in-repo replacement for the external `pyoxigraph` package
//! (#667). Backed entirely by the oxigraph-free `purrdf-core` IR + the native
//! SPARQL engine (EPIC #906): no `oxigraph` types cross this surface.
//!
//! # Why this exists
//!
//! `pyoxigraph` is *literally the Python binding to oxigraph*, the same engine
//! every purrdf-* crate already links (`oxigraph 0.5`, `rdf-12`). Depending on it
//! is depending on an externally-versioned copy of an engine we own. This module
//! exposes the Store + SPARQL (SELECT / ASK / CONSTRUCT, variable substitution) +
//! `parse` / `serialize` + RDFC-1.0 canonicalization surface our Python layer
//! needs, so `make check` / CI / the build run with **no external RDF runtime**
//! (CONSTITUTION Principle 18).
//!
//! # Kernel-clean separation
//!
//! This module lives in the dedicated Python binding crate. The RDF kernel stays
//! PyO3-free.
//!
//! # Single-responsibility layout (#835)
//!
//! This module is the thin facade over five focused submodules, split along the
//! P2 backend-trait seams so the trait extraction (#836) is a clean lift:
//!
//! * [`term`] — the term object model (`NamedNode` … `Quad`, `Variable`) and the
//!   Python ⇄ oxigraph term converters/extractors (`TermFactory` seam).
//! * [`io`] — `parse` / `serialize` + the pure-Rust `parse_quads` /
//!   `serialize_triples` cores (`RdfParserBackend` / `RdfSerializer` seams).
//! * [`query`] — the materialized SPARQL result model (`SparqlEngine` seam).
//! * [`store`] — the mutable `Store` / `Dataset` / `QuadIter` (`MutableStore` /
//!   `Dataset` seams).
//! * [`canon`] — `CanonicalizationAlgorithm` + the `canonicalize_quads` core.
//!
//! # Design
//!
//! * **Eager materialization** — `Store.query` freezes a snapshot and collects the
//!   native engine's results into owned `Vec`s before returning, so a borrow of the
//!   store never escapes into a `'static` `#[pyclass]`.
//! * **Pure-Rust cores** — [`parse_quads`] and [`canonicalize_quads`] hold the
//!   load-bearing logic and are unit-tested without a Python interpreter; the
//!   `#[pymethods]` are thin wrappers over them.
//! * **Faithful object model** — the term/result classes mirror the slice of the
//!   `pyoxigraph` API the codebase relies on, so the Python migration is a
//!   mechanical import swap rather than a rewrite of ~150 call sites.

mod canon;
mod io;
mod mutable;
mod query;
mod results;
mod store;
mod term;
mod xsd;

pub(crate) use io::{parse_quads, PyRdfFormat};

use pyo3::prelude::*;

/// Register the native Store / term / SPARQL surface on the `purrdf` module.
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRdfFormat>()?;
    m.add_class::<canon::PyCanonicalizationAlgorithm>()?;
    m.add_class::<term::PyNamedNode>()?;
    m.add_class::<term::PyBlankNode>()?;
    m.add_class::<term::PyLiteral>()?;
    m.add_class::<term::PyTriple>()?;
    m.add_class::<term::PyQuad>()?;
    m.add_class::<term::PyDefaultGraph>()?;
    m.add_class::<term::PyVariable>()?;
    m.add_class::<query::PyQuerySolutions>()?;
    m.add_class::<query::PyQuerySolution>()?;
    m.add_class::<query::PyQueryTriples>()?;
    m.add_class::<query::PyQueryBoolean>()?;
    m.add_class::<store::PyStore>()?;
    m.add_class::<store::PyDataset>()?;
    m.add_class::<mutable::PyMutableDataset>()?;
    m.add_class::<store::PyQuadIter>()?;
    m.add_function(wrap_pyfunction!(io::parse, m)?)?;
    m.add_function(wrap_pyfunction!(io::serialize, m)?)?;
    m.add_function(wrap_pyfunction!(results::serialize_sparql_solutions, m)?)?;
    m.add_function(wrap_pyfunction!(results::serialize_sparql_boolean, m)?)?;
    m.add_function(wrap_pyfunction!(results::parse_sparql_results, m)?)?;
    m.add_function(wrap_pyfunction!(xsd::xsd_value_compare, m)?)?;
    m.add_function(wrap_pyfunction!(xsd::xsd_canonical_lexical, m)?)?;
    m.add_function(wrap_pyfunction!(xsd::xsd_decode_binary, m)?)?;
    m.add_function(wrap_pyfunction!(xsd::xsd_normalize_whitespace, m)?)?;
    Ok(())
}
