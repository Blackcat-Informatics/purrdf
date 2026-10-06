// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The JSON spelling of an RDF 1.2 base direction, written once.
//!
//! The token and its parse belong to [`purrdf_core::RdfTextDirection`] (the one
//! base-direction type); this module only adapts that pair to the
//! [`purrdf_lex::json`] record layer, for every JSON artifact in this crate that
//! carries a direction.

use purrdf_core::RdfTextDirection;
use purrdf_lex::json::Value;
use purrdf_lex::json::record::DecodeError;

/// A base direction as its lowercase token, `None` as `null`.
pub(crate) fn direction_to_json(direction: Option<RdfTextDirection>) -> Value {
    direction.map_or(Value::Null, |direction| Value::from(direction.as_str()))
}

/// A base direction read from its lowercase token.
///
/// # Errors
///
/// Returns a decode error when `value` is not a string, or names neither
/// `ltr` nor `rtl`.
pub(crate) fn direction_from_json(value: &Value) -> Result<RdfTextDirection, DecodeError> {
    let token = value
        .as_str()
        .ok_or_else(|| DecodeError::invalid_type(value, "enum RdfTextDirection"))?;
    RdfTextDirection::from_str_token(token).ok_or_else(|| {
        DecodeError::unknown_variant(
            token,
            &[
                RdfTextDirection::Ltr.as_str(),
                RdfTextDirection::Rtl.as_str(),
            ],
        )
    })
}
