// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! GeoSPARQL compatibility facade over the shared geometry kernel and evaluator adapters.
//!
//! Geometry values and computational modules retain their established paths.
//! Scalar and property-function registrations use the native SPARQL evaluator.

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

pub use purrdf_geo_kernel::*;
pub use purrdf_sparql_eval::geo::{functions, relation};
