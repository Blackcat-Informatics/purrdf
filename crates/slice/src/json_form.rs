// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Field readers for the `from_json` decoders of this crate's public records.
//!
//! Every record decoder reads its members through these, so a missing member
//! and a mistyped one are refused with one wording, naming the member.

use purrdf_lex::json::{Object, Value};

use crate::error::SliceError;

/// `value` as an object, or a refusal naming `what` it should have been.
pub(crate) fn object<'a>(value: &'a Value, what: &str) -> Result<&'a Object, SliceError> {
    value
        .as_object()
        .ok_or_else(|| SliceError::Json(format!("{what} must be a JSON object")))
}

/// The member `name`, which must be present.
pub(crate) fn field<'a>(object: &'a Object, name: &str) -> Result<&'a Value, SliceError> {
    object
        .get(name)
        .ok_or_else(|| SliceError::Json(format!("missing member `{name}`")))
}

/// The string member `name`.
pub(crate) fn string(object: &Object, name: &str) -> Result<String, SliceError> {
    field(object, name)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| SliceError::Json(format!("member `{name}` must be a string")))
}

/// The member `name`, a string or `null`; absent reads as `null`.
pub(crate) fn optional_string(object: &Object, name: &str) -> Result<Option<String>, SliceError> {
    match object.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(SliceError::Json(format!(
            "member `{name}` must be a string or null"
        ))),
    }
}

/// The array-of-strings member `name`.
pub(crate) fn strings(object: &Object, name: &str) -> Result<Vec<String>, SliceError> {
    field(object, name)?
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .ok_or_else(|| SliceError::Json(format!("member `{name}` must be an array of strings")))
}

/// The array-of-bytes member `name`: every item an integer in `0..=255`.
pub(crate) fn bytes(object: &Object, name: &str) -> Result<Vec<u8>, SliceError> {
    field(object, name)?
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_u64().and_then(|byte| u8::try_from(byte).ok()))
                .collect()
        })
        .ok_or_else(|| SliceError::Json(format!("member `{name}` must be an array of bytes")))
}

/// The string payload of a one-member `{variant: <string>}` object.
pub(crate) fn newtype_variant(value: &Value, variant: &str) -> Result<String, SliceError> {
    value
        .as_object()
        .filter(|object| object.len() == 1)
        .and_then(|object| object.get(variant))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            SliceError::Json(format!(
                "expected a variant name or a one-member `{variant}` object"
            ))
        })
}
