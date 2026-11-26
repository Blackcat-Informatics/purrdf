// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native integration contracts for preparing and repeatedly executing queries.

#[path = "prepared_cases/operations.rs"]
mod prepared_operations;
#[path = "prepared_cases/parameter_mentions.rs"]
mod prepared_parameter_mentions;
#[path = "prepared_cases/parameters.rs"]
mod prepared_parameters;
#[path = "prepared_cases/reuse.rs"]
mod prepared_reuse;
mod support;
