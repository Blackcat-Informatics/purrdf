// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Strict records read from the JSON option documents a JS caller passes as text.
//!
//! An options document (the visualization options, a SERVICE profile) is one
//! JSON object whose members are a closed set of camelCase names. [`Record`]
//! reads it member by member over [`purrdf_lex::json`], the workspace's one JSON
//! reader, and states the record law once for every such document:
//!
//! * the document is an object, or it is refused;
//! * a member named twice is refused (`duplicate field`), since which of the two
//!   was meant is not the reader's to guess;
//! * a member the record does not declare is refused (`unknown field`), naming
//!   the member and the declared set, unless the record is [`Record::open`];
//! * an optional member that is absent or `null` is unset; a defaulted member
//!   that is absent takes its default, and one written as `null` is refused,
//!   because `null` is not a list or an object;
//! * a member of the wrong JSON kind is refused, naming the member and the kind
//!   it must be.

use purrdf_lex::json::{self, Object, Value};

/// One options object, checked against its declared member names.
pub(crate) struct Record<'a> {
    /// The object's members.
    object: &'a Object,
    /// What the document is, for error messages (e.g. `service profile`).
    what: &'a str,
}

impl<'a> Record<'a> {
    /// `value` as a record whose members are exactly `fields`: an unknown or a
    /// repeated member is refused.
    pub(crate) fn closed(value: &'a Value, what: &'a str, fields: &[&str]) -> Result<Self, String> {
        let record = Self::open(value, what)?;
        if let Some((name, _)) = record
            .object
            .iter()
            .find(|(name, _)| !fields.contains(&name.as_str()))
        {
            let expected = fields
                .iter()
                .map(|field| format!("`{field}`"))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!(
                "{what}: unknown field `{name}`, expected one of {expected}"
            ));
        }
        Ok(record)
    }

    /// `value` as a record that ignores members it does not read; a repeated
    /// member is still refused.
    pub(crate) fn open(value: &'a Value, what: &'a str) -> Result<Self, String> {
        let object = value
            .as_object()
            .ok_or_else(|| format!("{what}: invalid type: {}, expected an object", kind(value)))?;
        if let Some(name) = object.first_duplicate() {
            return Err(format!("{what}: duplicate field `{name}`"));
        }
        Ok(Self { object, what })
    }

    /// The member `name`, unset when absent or `null`, else read by `read`.
    pub(crate) fn optional<T>(
        &self,
        name: &str,
        expected: &str,
        read: impl FnOnce(&'a Value) -> Option<T>,
    ) -> Result<Option<T>, String> {
        match self.object.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => read(value)
                .map(Some)
                .ok_or_else(|| self.mismatch(name, value, expected)),
        }
    }

    /// The member `name`, `T::default()` when absent; `null` and any other kind
    /// than `read` accepts are refused.
    pub(crate) fn defaulted<T: Default>(
        &self,
        name: &str,
        expected: &str,
        read: impl FnOnce(&'a Value) -> Result<T, String>,
    ) -> Result<T, String> {
        self.object.get(name).map_or_else(
            || Ok(T::default()),
            |value| self.present(name, value, expected, read),
        )
    }

    /// The member `name`, which must be present and not `null`.
    pub(crate) fn required<T>(
        &self,
        name: &str,
        expected: &str,
        read: impl FnOnce(&'a Value) -> Result<T, String>,
    ) -> Result<T, String> {
        let value = self
            .object
            .get(name)
            .ok_or_else(|| format!("{}: missing field `{name}`", self.what))?;
        self.present(name, value, expected, read)
    }

    /// A present member through `read`, `null` refused.
    fn present<T>(
        &self,
        name: &str,
        value: &'a Value,
        expected: &str,
        read: impl FnOnce(&'a Value) -> Result<T, String>,
    ) -> Result<T, String> {
        if value.is_null() {
            return Err(self.mismatch(name, value, expected));
        }
        read(value).map_err(|error| {
            if error.is_empty() {
                self.mismatch(name, value, expected)
            } else {
                format!("{}: field `{name}`: {error}", self.what)
            }
        })
    }

    /// The refusal of member `name` holding `value` where `expected` belongs.
    fn mismatch(&self, name: &str, value: &Value, expected: &str) -> String {
        format!(
            "{}: field `{name}`: invalid type: {}, expected {expected}",
            self.what,
            kind(value)
        )
    }
}

/// The JSON kind of `value`, as a refusal names it.
fn kind(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(flag) => format!("boolean `{flag}`"),
        Value::Number(number) => format!("number `{}`", number.lexeme()),
        Value::String(text) => format!(
            "string {}",
            json::write_compact(&Value::from(text.as_str()))
        ),
        Value::Array(_) => "an array".to_owned(),
        Value::Object(_) => "an object".to_owned(),
    }
}

/// A string.
pub(crate) fn string(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

/// An integer lexeme that fits `T`.
pub(crate) fn integer<T: TryFrom<i128>>(value: &Value) -> Option<T> {
    value
        .as_number()
        .and_then(json::Number::as_i128)
        .and_then(|number| T::try_from(number).ok())
}

/// An array of strings; the empty message asks the record to name the member.
pub(crate) fn strings(value: &Value) -> Result<Vec<String>, String> {
    value
        .as_array()
        .and_then(|items| items.iter().map(string).collect())
        .ok_or_else(String::new)
}

/// An array of records, each read by `read`.
pub(crate) fn records<T>(
    value: &Value,
    read: impl Fn(&Value) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    value
        .as_array()
        .ok_or_else(String::new)?
        .iter()
        .map(read)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> Value {
        json::read(text).expect("test JSON")
    }

    #[test]
    fn an_unknown_member_of_a_closed_record_is_refused_and_a_declared_one_is_read() {
        let refused = doc(r#"{"a":1,"b":2}"#);
        let error = Record::closed(&refused, "options", &["a"])
            .err()
            .expect("unknown member");
        assert!(error.contains("unknown field `b`"), "{error}");
        let accepted = doc(r#"{"a":1}"#);
        let record = Record::closed(&accepted, "options", &["a"]).expect("declared member");
        assert_eq!(
            record.optional("a", "an integer", integer::<u32>),
            Ok(Some(1))
        );
    }

    #[test]
    fn an_open_record_ignores_an_unknown_member_but_refuses_a_repeat() {
        let open = doc(r#"{"a":1,"b":2}"#);
        assert!(Record::open(&open, "options").is_ok());
        let repeated = doc(r#"{"a":1,"a":2}"#);
        let error = Record::open(&repeated, "options").err().expect("repeat");
        assert!(error.contains("duplicate field `a`"), "{error}");
    }

    #[test]
    fn a_document_that_is_not_an_object_is_refused() {
        let error = Record::open(&doc("[1]"), "options").err().expect("array");
        assert!(error.contains("expected an object"), "{error}");
        assert!(Record::open(&doc("{}"), "options").is_ok());
    }

    #[test]
    fn null_unsets_an_optional_member_but_is_refused_for_a_defaulted_one() {
        let value = doc(r#"{"a":null,"b":null}"#);
        let record = Record::closed(&value, "options", &["a", "b"]).expect("record");
        assert_eq!(record.optional("a", "a string", string), Ok(None));
        let error = record
            .defaulted("b", "an array of strings", strings)
            .expect_err("null list");
        assert!(error.contains("field `b`"), "{error}");
        let absent = doc("{}");
        let record = Record::closed(&absent, "options", &["b"]).expect("record");
        assert_eq!(
            record.defaulted("b", "an array of strings", strings),
            Ok(vec![])
        );
    }

    #[test]
    fn a_member_of_the_wrong_kind_or_range_is_refused_and_named() {
        let value = doc(r#"{"n":-1,"f":1.0,"ok":7}"#);
        let record = Record::closed(&value, "options", &["n", "f", "ok"]).expect("record");
        for name in ["n", "f"] {
            let error = record
                .optional(name, "a non-negative integer", integer::<u32>)
                .expect_err("not a u32");
            assert!(error.contains(&format!("field `{name}`")), "{error}");
        }
        assert_eq!(
            record.optional("ok", "an integer", integer::<u32>),
            Ok(Some(7))
        );
    }

    #[test]
    fn a_missing_required_member_is_refused_and_a_present_one_read() {
        let absent = doc("{}");
        let record = Record::closed(&absent, "profile", &["c"]).expect("record");
        let error = record
            .required("c", "an array", strings)
            .expect_err("missing");
        assert!(error.contains("missing field `c`"), "{error}");
        let present = doc(r#"{"c":["x"]}"#);
        let record = Record::closed(&present, "profile", &["c"]).expect("record");
        assert_eq!(
            record.required("c", "an array", strings),
            Ok(vec!["x".to_owned()])
        );
    }
}
