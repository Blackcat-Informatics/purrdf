// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The extension environment a query from the Python surface is interpreted in.
//!
//! A Python caller supplies its relation and aggregate registries per call, as
//! optional arguments. The evaluator wants them as one value — the environment that
//! decides whether a predicate IRI in the query text is a data edge or a call to a
//! registered relation — so this is where the two optionals become that value, once,
//! in one place both the read and the mutable store seams use.
//!
//! An absent registry and the canonical empty one are the same value here, for the
//! reason every other seam in this workspace treats them the same: an empty registry
//! resolves no IRI, so no query's behaviour can depend on which empty registry it was
//! interpreted against. There is one spelling of "nothing registered", not two.

use purrdf_sparql_eval::{AggregateRegistry, ExtensionEnv, PropertyFunctionRegistry};
use pyo3::PyResult;
use pyo3::exceptions::PyValueError;

/// The environment for a call carrying `relations` and `aggregates`.
///
/// # Errors
///
/// A `ValueError` if a registered relation's or aggregate's declaration methods
/// panic — deriving the environment reads every declaration, because that is how the
/// parser learns which predicate IRIs are calls.
pub(crate) fn extension_env(
    relations: Option<&PropertyFunctionRegistry>,
    aggregates: Option<&AggregateRegistry>,
) -> PyResult<ExtensionEnv> {
    ExtensionEnv::over(
        relations
            .cloned()
            .unwrap_or(PropertyFunctionRegistry::EMPTY),
        aggregates.cloned().unwrap_or(AggregateRegistry::EMPTY),
    )
    .map_err(|e| PyValueError::new_err(format!("extension environment: {e}")))
}
