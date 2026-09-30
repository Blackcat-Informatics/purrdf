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

/// A complete role map (a newtype over `BTreeMap<Role, String>` whose `new`
/// validates it) is its JSON object of role → binding, and is revalidated by
/// `new` when read, so the JSON reader admits exactly what the constructor
/// admits.
macro_rules! role_map_json {
    ($($type:ty),+ $(,)?) => {$(
        impl purrdf_lex::json::record::FromJson for $type {
            fn from_json(
                value: &purrdf_lex::json::Value,
            ) -> Result<Self, purrdf_lex::json::record::DecodeError> {
                Ok(Self::new(purrdf_lex::json::record::FromJson::from_json(value)?)?)
            }
        }

        impl purrdf_lex::json::record::ToJson for $type {
            fn to_json(&self) -> purrdf_lex::json::Value {
                purrdf_lex::json::record::ToJson::to_json(&self.0)
            }
        }
    )+};
}

pub(crate) use role_map_json;
