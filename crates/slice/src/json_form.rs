// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Field readers for the `from_json` decoders of this crate's public records.
//!
//! Every record decoder reads its members through these, so a missing member
//! and a mistyped one are refused with one wording, naming the member.

use purrdf_lex::json::{Object, Value};

use crate::error::{JsonMemberKind, JsonRefusal, SliceError};

/// `value` as an object, or a refusal naming `what` it should have been.
pub(crate) fn object<'a>(value: &'a Value, what: &str) -> Result<&'a Object, SliceError> {
    value.as_object().ok_or_else(|| {
        SliceError::Json(JsonRefusal::Object {
            what: what.to_owned(),
            value: value.clone(),
        })
    })
}

/// The member `name`, which must be present.
pub(crate) fn field<'a>(object: &'a Object, name: &str) -> Result<&'a Value, SliceError> {
    object.get(name).ok_or_else(|| {
        SliceError::Json(JsonRefusal::Missing {
            name: name.to_owned(),
        })
    })
}

/// The string member `name`.
pub(crate) fn string(object: &Object, name: &str) -> Result<String, SliceError> {
    let value = field(object, name)?;
    value.as_str().map(str::to_owned).ok_or_else(|| {
        SliceError::Json(JsonRefusal::Member {
            name: name.to_owned(),
            expected: JsonMemberKind::String,
            value: value.clone(),
        })
    })
}

/// The member `name`, a string or `null`; absent reads as `null`.
pub(crate) fn optional_string(object: &Object, name: &str) -> Result<Option<String>, SliceError> {
    match object.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(value) => Err(SliceError::Json(JsonRefusal::Member {
            name: name.to_owned(),
            expected: JsonMemberKind::OptionalString,
            value: value.clone(),
        })),
    }
}

/// The array-of-strings member `name`.
pub(crate) fn strings(object: &Object, name: &str) -> Result<Vec<String>, SliceError> {
    let value = field(object, name)?;
    value
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .ok_or_else(|| {
            SliceError::Json(JsonRefusal::Member {
                name: name.to_owned(),
                expected: JsonMemberKind::Strings,
                value: value.clone(),
            })
        })
}

/// The array-of-bytes member `name`: every item an integer in `0..=255`.
pub(crate) fn bytes(object: &Object, name: &str) -> Result<Vec<u8>, SliceError> {
    let value = field(object, name)?;
    value
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_u64().and_then(|byte| u8::try_from(byte).ok()))
                .collect()
        })
        .ok_or_else(|| {
            SliceError::Json(JsonRefusal::Member {
                name: name.to_owned(),
                expected: JsonMemberKind::Bytes,
                value: value.clone(),
            })
        })
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
            SliceError::Json(JsonRefusal::Variant {
                variant: variant.to_owned(),
                value: value.clone(),
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::Capture;

    #[test]
    fn rejected_byte_array_retains_exact_values_after_record_death() {
        let original = purrdf_lex::json::read(r#"{"bytes":[true,9007199254740993]}"#).unwrap();
        let error = bytes(original.as_object().unwrap(), "bytes").unwrap_err();
        drop(original);
        let fields = Capture::read(&error);
        let reason = fields.child("reason");
        assert_eq!(reason.code, "slice-json-member");
        assert!(matches!(reason.field("name"), ErrorValue::Text("bytes")));
        assert_eq!(reason.child("expected").code, "slice-json-bytes");
        let value = reason.child("value");
        let entries = value.records("values");
        assert_eq!(entries.len(), 2);
        assert!(matches!(
            entries[0].field("value"),
            ErrorValue::Boolean(true)
        ));
        assert!(matches!(
            entries[1].field("lexeme"),
            ErrorValue::Text("9007199254740993")
        ));
    }
}
