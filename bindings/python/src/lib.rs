// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Unified Python extension module for PurRDF.
//!
//! The Rust crates remain PyO3-free. This crate owns the CPython extension
//! boundary and delegates into the public Rust crate APIs.
//!
//! # GIL release on heavy operations
//!
//! Every heavy compute entry point (parsing, serialization, canonicalization,
//! GTS fold/emit, SPARQL query/update evaluation, SHACL/ShEx validation, slice
//! discovery/analysis, SSSOM parse/validate) releases the GIL while the engine
//! runs, via [`pyo3::Python::detach`]: Python-side arguments are converted to
//! plain Rust data first, the engine call runs without the GIL, and Python
//! result objects are built after the GIL is reacquired. Other Python threads
//! therefore make progress during long-running native calls.

pub use purrdf::*;

mod py_gts;
mod py_gts_dataset;
mod py_gts_view;
mod py_jsonld;
mod py_projection;
mod py_shex;
mod py_slice;
mod py_sssom;
mod py_store;
mod rdf;
mod shacl;

use pyo3::prelude::*;

#[pymodule]
fn purrdf_native(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let rdf_module = PyModule::new(py, "rdf")?;
    rdf::register(&rdf_module)?;
    m.add_submodule(&rdf_module)?;

    let shacl_module = PyModule::new(py, "shacl")?;
    shacl::register(&shacl_module)?;
    m.add_submodule(&shacl_module)?;

    let shex_module = PyModule::new(py, "shex")?;
    py_shex::register(&shex_module)?;
    m.add_submodule(&shex_module)?;

    let slice_module = PyModule::new(py, "slice")?;
    py_slice::register(&slice_module)?;
    m.add_submodule(&slice_module)?;

    Ok(())
}
