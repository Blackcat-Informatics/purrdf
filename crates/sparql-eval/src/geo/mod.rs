// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! GeoSPARQL adapters over the shared, evaluator-independent geometry kernel.

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

mod error;
pub mod functions;
pub mod relation;
