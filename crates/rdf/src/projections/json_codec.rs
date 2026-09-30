// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The projection side of [`purrdf_lex::json::record`].
//!
//! Every projection configuration, sideband model and report crosses JSON
//! through `purrdf_lex::json::record`: its [`FromJson`] and [`ToJson`] impls
//! are written against that module's strict [`Record`] reader, and its
//! refusals are that module's [`DecodeError`]. What is projection-specific is
//! only that a projection constructor's own refusal, a [`ProjectionError`],
//! is a decode refusal carrying the constructor's message, so a `FromJson`
//! impl validates through the same constructor callers use and never admits
//! a value the constructor refuses.
//!
//! The re-exports below are the names the LPG and OBO Graphs projections and
//! the DataCite writer read these items by.

use purrdf_lex::json::record::DecodeError;

use super::ProjectionError;

#[cfg(test)]
pub(crate) use purrdf_lex::json::record::to_vec;
pub(crate) use purrdf_lex::json::record::{
    DecodeError as JsonError, FromJson, Record as Fields, ToJson, from_slice,
};
pub(crate) use purrdf_lex::json_string_enum;

impl From<ProjectionError> for DecodeError {
    fn from(error: ProjectionError) -> Self {
        Self::custom(error)
    }
}
