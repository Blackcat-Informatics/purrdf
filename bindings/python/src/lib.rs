// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Unified Python extension module for PurRDF.
//!
//! The Rust crates remain PyO3-free. This crate owns the CPython extension
//! boundary and delegates into the public Rust crate APIs.
//!
//! # GIL release on heavy operations
//!
//! Every heavy compute entry point (parsing, serialization, canonicalization,
//! GTS fold/emit, SPARQL query/update evaluation, SHACL/ShEx validation,
//! entailment-regime materialization, slice discovery/analysis, SSSOM
//! parse/validate, the ranked-retrieval ladder) releases the GIL while the
//! engine runs, via
//! [`pyo3::Python::detach`]: Python-side arguments are converted to plain Rust
//! data first, the engine call runs without the GIL, and Python result objects
//! are built after the GIL is reacquired. Other Python threads therefore make
//! progress during long-running native calls.
//!
//! Entailment is the newest member of that list and the clearest case for it: an
//! RDFS or OWL-RL chase over a real ontology is the longest-running call this
//! extension offers, so `entail.materialize` converts its arguments to an owned
//! `Arc<RdfDataset>` and a native `Regime` first, runs the chase AND the report
//! rendering detached, and only then builds the returned pair. A rule-inventory
//! lookup (`entail.rules`) is a `&'static` table read, not a compute path, and
//! deliberately does not release the GIL.

pub use purrdf::*;

/// `FromPyObject` for a `Copy` `#[pyclass]` declared `skip_from_py_object`: extract
/// the class guard and copy the value out of it.
///
/// `#[pyclass(from_py_object)]` generates this impl with `Clone::clone(&*guard)` as
/// the body. For a `Copy` class that clone is a copy wearing a `.clone()` -- a real
/// `clippy::clone_on_copy`, and one no `#[allow]` on the type can reach, because
/// pyo3 emits the impl as a SIBLING item outside the type's attribute scope. So a
/// `Copy` class opts out of the generated impl and gets this one, which
/// dereferences through `Copy` instead. It is a transcription of the pyo3 0.29
/// expansion, not a redesign: same `Error` type, same `PyClassGuard` extraction,
/// same error path, so the Python-visible behaviour is the generated impl's. The
/// `INPUT_TYPE` associated const the pyo3 macro can also emit is gated on pyo3's
/// `experimental-inspect` feature, which is off here, so there is nothing else to
/// carry over.
macro_rules! copy_pyclass_from_py_object {
    ($ty:ty) => {
        impl<'a, 'py> ::pyo3::FromPyObject<'a, 'py> for $ty {
            type Error = ::pyo3::pyclass::PyClassGuardError<'a, 'py>;

            fn extract(
                obj: ::pyo3::Borrowed<'a, 'py, ::pyo3::PyAny>,
            ) -> Result<Self, <Self as ::pyo3::FromPyObject<'a, 'py>>::Error> {
                Ok(*obj.extract::<::pyo3::PyClassGuard<'_, Self>>()?)
            }
        }
    };
}

mod attestation;
mod py_entail;
mod py_geo;
mod py_gts;
mod py_gts_dataset;
mod py_gts_view;
mod py_jsonld;
mod py_projection;
mod py_retrieval;
mod py_shex;
mod py_slice;
mod py_sssom;
mod py_store;
mod rdf;
mod shacl;

use pyo3::prelude::*;

#[pymodule]
fn purrdf_native(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    let geo_module = PyModule::new(py, "geo")?;
    py_geo::register(&geo_module)?;
    m.add_submodule(&geo_module)?;
    let rdf_module = PyModule::new(py, "rdf")?;
    rdf::register(&rdf_module)?;
    m.add_submodule(&rdf_module)?;

    let shacl_module = PyModule::new(py, "shacl")?;
    shacl::register(&shacl_module)?;
    m.add_submodule(&shacl_module)?;

    let entail_module = PyModule::new(py, "entail")?;
    py_entail::register(&entail_module)?;
    m.add_submodule(&entail_module)?;

    let shex_module = PyModule::new(py, "shex")?;
    py_shex::register(&shex_module)?;
    m.add_submodule(&shex_module)?;

    let retrieval_module = PyModule::new(py, "retrieval")?;
    py_retrieval::register(&retrieval_module)?;
    m.add_submodule(&retrieval_module)?;

    let slice_module = PyModule::new(py, "slice")?;
    py_slice::register(&slice_module)?;
    m.add_submodule(&slice_module)?;

    Ok(())
}
