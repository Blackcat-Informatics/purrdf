// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compatibility names for the workspace’s one floating-environment checker.

pub(crate) use purrdf_xsd::ieee::environment::check;
#[cfg(test)]
pub(crate) use purrdf_xsd::ieee::environment::{Departure, PROBES, probe};
pub use purrdf_xsd::ieee::environment::{FloatEnvironmentError, FloatEnvironmentEvidence};
