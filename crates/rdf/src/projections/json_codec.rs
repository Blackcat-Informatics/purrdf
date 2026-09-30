// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one conversion layer between projection types and [`purrdf_lex::json`].
//!
//! Every projection configuration, sideband model and report crosses JSON here.
//! A type states its JSON shape once, by hand, in a [`FromJson`] and a
//! [`ToJson`] impl written against [`Value`]; the reading, writing, number and
//! string grammar all stay in `purrdf_lex::json`, which every JSON reader and
//! writer in the workspace shares.
//!
//! # Contract
//!
//! * **Objects are strict records.** [`Fields`] reads an object member by
//!   member: a declared member that is absent is `missing field`, one that is
//!   written twice is `duplicate field`, and [`Fields::deny_unknown`] refuses
//!   every member the type did not declare. A record is only ever an object;
//!   an array is never read positionally as a record.
//! * **Closed vocabularies are strings.** [`json_string_enum!`] spells each
//!   variant of a closed enum once; any other string is `unknown variant`.
//! * **Free-form JSON is a sorted, last-wins map.** A [`Value`] a projection
//!   carries through (a JSON-LD context, a CSVW annotation, a research-object
//!   document) is held in [`sorted_last_wins`] form: every object's members in
//!   name order, a repeated name keeping its last value. Carrying the value in
//!   one form makes its bytes a function of its content, so an emitted artifact
//!   never depends on how a caller happened to order or repeat members.
//! * **Records keep declaration order.** A [`ToJson`] impl writes members in the
//!   order it names them, so an artifact's layout is the type's declaration.
//! * **Refusals name the problem in stable words.** [`JsonError`] reads
//!   `invalid type: …, expected …`, `unknown field …`, `missing field …`,
//!   `duplicate field …`, `unknown variant …`, or the validation message of the
//!   constructor that refused the decoded parts.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use purrdf_lex::json::{self, Limits, Object, Value};

use super::ProjectionError;

/// A JSON document or member that does not have the shape its type requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JsonError(String);

impl JsonError {
    /// A refusal carrying a validation message.
    pub(crate) fn custom(message: impl fmt::Display) -> Self {
        Self(message.to_string())
    }

    /// `found` is not the JSON kind `expected` names.
    pub(crate) fn invalid_type(found: &Value, expected: &str) -> Self {
        Self(format!(
            "invalid type: {}, expected {expected}",
            Unexpected(found)
        ))
    }

    /// `found` has the right kind but a value `expected` does not admit.
    pub(crate) fn invalid_value(found: &Value, expected: &str) -> Self {
        Self(format!(
            "invalid value: {}, expected {expected}",
            Unexpected(found)
        ))
    }

    /// A string that names none of a closed set of variants.
    pub(crate) fn unknown_variant(variant: &str, expected: &[&str]) -> Self {
        Self(format!(
            "unknown variant `{variant}`, {}",
            OneOf(expected, "variants")
        ))
    }

    /// A member no field of the record declares.
    pub(crate) fn unknown_field(field: &str, expected: &[&str]) -> Self {
        Self(format!(
            "unknown field `{field}`, {}",
            OneOf(expected, "fields")
        ))
    }

    /// A mandatory member is absent.
    pub(crate) fn missing_field(field: &str) -> Self {
        Self(format!("missing field `{field}`"))
    }

    /// A declared member is written more than once.
    pub(crate) fn duplicate_field(field: &str) -> Self {
        Self(format!("duplicate field `{field}`"))
    }
}

impl fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for JsonError {}

impl From<ProjectionError> for JsonError {
    fn from(error: ProjectionError) -> Self {
        Self::custom(error)
    }
}

impl From<json::Error> for JsonError {
    fn from(error: json::Error) -> Self {
        Self::custom(error)
    }
}

/// A value, described the way a refusal names it.
struct Unexpected<'a>(&'a Value);

impl fmt::Display for Unexpected<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Value::Null => formatter.write_str("null"),
            Value::Bool(value) => write!(formatter, "boolean `{value}`"),
            Value::Number(number) if number.is_integer() => {
                write!(formatter, "integer `{}`", number.lexeme())
            }
            Value::Number(number) => write!(formatter, "floating point `{}`", number.lexeme()),
            Value::String(value) => write!(formatter, "string {value:?}"),
            Value::Array(_) => formatter.write_str("sequence"),
            Value::Object(_) => formatter.write_str("map"),
        }
    }
}

/// The alternatives a refusal offers.
struct OneOf<'a>(&'a [&'a str], &'static str);

impl fmt::Display for OneOf<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            [] => write!(formatter, "there are no {}", self.1),
            [only] => write!(formatter, "expected `{only}`"),
            [first, second] => write!(formatter, "expected `{first}` or `{second}`"),
            names => {
                formatter.write_str("expected one of ")?;
                for (index, name) in names.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(formatter, "`{name}`")?;
                }
                Ok(())
            }
        }
    }
}

/// A type read from a JSON value.
pub(crate) trait FromJson: Sized {
    /// Decode `value`, validating it exactly as the type's constructor does.
    ///
    /// # Errors
    ///
    /// Returns a [`JsonError`] naming the first shape or validation failure.
    fn from_json(value: &Value) -> Result<Self, JsonError>;
}

/// A type written as a JSON value.
pub(crate) trait ToJson {
    /// The value, members in the type's declared order.
    fn to_json(&self) -> Value;
}

/// A type that names a JSON object member: a string, or a closed enum spelled
/// as one.
pub(crate) trait JsonKey: Sized {
    /// The member name.
    fn to_key(&self) -> String;

    /// Decode a member name.
    ///
    /// # Errors
    ///
    /// Returns a [`JsonError`] when `key` names no value of the type.
    fn from_key(key: &str) -> Result<Self, JsonError>;
}

/// Read one JSON document into `T`.
///
/// The document is read under [`Limits::DEFAULT`] and then decoded; a syntax
/// error and a shape error are both a [`JsonError`].
pub(crate) fn from_slice<T: FromJson>(bytes: &[u8]) -> Result<T, JsonError> {
    T::from_json(&json::read_slice(bytes, Limits::DEFAULT)?)
}

/// Read one free-form JSON document in [`sorted_last_wins`] form.
///
/// # Errors
///
/// Returns the reader's error for a document that is not RFC 8259 JSON, or
/// that nests deeper than [`Limits::DEFAULT`].
pub(crate) fn read_document(bytes: &[u8]) -> Result<Value, json::Error> {
    let mut value = json::read_slice(bytes, Limits::DEFAULT)?;
    sorted_last_wins(&mut value);
    Ok(value)
}

/// Put every object in the tree into name order, keeping only the LAST member
/// of each repeated name.
///
/// This is the one form a free-form value is carried in (see the module
/// contract). The walk is over a heap work list, so depth costs no stack.
pub(crate) fn sorted_last_wins(value: &mut Value) {
    let mut work: Vec<&mut Value> = vec![value];
    while let Some(value) = work.pop() {
        match value {
            Value::Array(items) => work.extend(items.iter_mut()),
            Value::Object(object) => {
                object.sort_keys();
                if object.first_duplicate().is_some() {
                    let members: Vec<(String, Value)> = std::mem::take(object).into_members();
                    // Stable sort kept repeats in document order; keep the last
                    // of each run of equal names.
                    let mut kept: Vec<(String, Value)> = Vec::with_capacity(members.len());
                    for member in members {
                        match kept.last_mut() {
                            Some(last) if last.0 == member.0 => *last = member,
                            _ => kept.push(member),
                        }
                    }
                    *object = Object::from(kept);
                }
                work.extend(object.values_mut());
            }
            _ => {}
        }
    }
}

/// `value` as compact JSON bytes.
pub(crate) fn to_vec<T: ToJson + ?Sized>(value: &T) -> Vec<u8> {
    json::write_compact(&value.to_json()).into_bytes()
}

/// A [`Value`] taken apart by kind, by value.
///
/// [`Value`] drops over a heap work list, so it implements `Drop` and a
/// `match` cannot move its payload out; [`into_owned`] moves it out once.
#[derive(Debug)]
pub(crate) enum Owned {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number, as its lexeme.
    Number(json::Number),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Value>),
    /// An object.
    Object(Object),
}

impl From<Owned> for Value {
    fn from(owned: Owned) -> Self {
        match owned {
            Owned::Null => Self::Null,
            Owned::Bool(flag) => Self::Bool(flag),
            Owned::Number(number) => Self::Number(number),
            Owned::String(text) => Self::String(text),
            Owned::Array(items) => Self::Array(items),
            Owned::Object(object) => Self::Object(object),
        }
    }
}

/// Move `value`'s payload out, by kind.
pub(crate) fn into_owned(mut value: Value) -> Owned {
    match &mut value {
        Value::Null => Owned::Null,
        Value::Bool(flag) => Owned::Bool(*flag),
        Value::Number(number) => Owned::Number(std::mem::replace(number, json::Number::from(0_u8))),
        Value::String(text) => Owned::String(std::mem::take(text)),
        Value::Array(items) => Owned::Array(std::mem::take(items)),
        Value::Object(object) => Owned::Object(std::mem::take(object)),
    }
}

/// A record read member by member.
///
/// Every member the type declares is taken through one of the typed readers;
/// [`Fields::deny_unknown`] then refuses whatever is left.
pub(crate) struct Fields<'a> {
    object: &'a Object,
    taken: Vec<&'static str>,
}

impl<'a> Fields<'a> {
    /// Begin reading `value`, which must be an object; `expecting` names the
    /// record in a refusal (`struct LpgConfig`).
    ///
    /// # Errors
    ///
    /// Returns `invalid type` for anything but an object.
    pub(crate) fn new(value: &'a Value, expecting: &str) -> Result<Self, JsonError> {
        match value {
            Value::Object(object) => Ok(Self {
                object,
                taken: Vec::new(),
            }),
            other => Err(JsonError::invalid_type(other, expecting)),
        }
    }

    /// The member named `name`, if present, marked as declared.
    ///
    /// # Errors
    ///
    /// Returns `duplicate field` when the object writes `name` twice.
    pub(crate) fn raw(&mut self, name: &'static str) -> Result<Option<&'a Value>, JsonError> {
        self.taken.push(name);
        if self.object.count(name) > 1 {
            return Err(JsonError::duplicate_field(name));
        }
        Ok(self.object.get(name))
    }

    /// A mandatory member.
    ///
    /// # Errors
    ///
    /// Returns `missing field`, `duplicate field`, or the member's own error.
    pub(crate) fn required<T: FromJson>(&mut self, name: &'static str) -> Result<T, JsonError> {
        match self.raw(name)? {
            Some(value) => T::from_json(value),
            None => Err(JsonError::missing_field(name)),
        }
    }

    /// An optional member: absent and `null` are both `None`.
    ///
    /// # Errors
    ///
    /// Returns `duplicate field` or the member's own error.
    pub(crate) fn optional<T: FromJson>(
        &mut self,
        name: &'static str,
    ) -> Result<Option<T>, JsonError> {
        match self.raw(name)? {
            None | Some(Value::Null) => Ok(None),
            Some(value) => T::from_json(value).map(Some),
        }
    }

    /// The string member `name` naming one of `variants`: an enum's tag.
    ///
    /// # Errors
    ///
    /// Returns `missing field`, `invalid type`, or `unknown variant`.
    pub(crate) fn tag(
        &mut self,
        name: &'static str,
        variants: &[&str],
    ) -> Result<&'a str, JsonError> {
        let value = self
            .raw(name)?
            .ok_or_else(|| JsonError::missing_field(name))?;
        let tag = value
            .as_str()
            .ok_or_else(|| JsonError::invalid_type(value, "variant identifier"))?;
        if variants.contains(&tag) {
            Ok(tag)
        } else {
            Err(JsonError::unknown_variant(tag, variants))
        }
    }

    /// Refuse every member no reader took.
    ///
    /// # Errors
    ///
    /// Returns `unknown field` for the first undeclared member.
    pub(crate) fn deny_unknown(self) -> Result<(), JsonError> {
        match self
            .object
            .keys()
            .find(|name| !self.taken.contains(&name.as_str()))
        {
            Some(name) => Err(JsonError::unknown_field(name, &self.taken)),
            None => Ok(()),
        }
    }
}

/// Spell each variant of a closed, field-less enum as one JSON string.
///
/// The macro gives the enum `json_str`, [`FromJson`], [`ToJson`] and
/// [`JsonKey`]; each spelling is written exactly once.
macro_rules! json_string_enum {
    ($type:ty { $($variant:ident => $spelling:literal),+ $(,)? }) => {
        impl $type {
            /// Every JSON spelling, in declaration order.
            #[allow(dead_code)]
            pub(crate) const JSON_VARIANTS: &'static [&'static str] = &[$($spelling),+];

            /// The variant's JSON spelling.
            #[allow(dead_code)]
            pub(crate) const fn json_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $spelling,)+
                }
            }
        }

        impl $crate::projections::json_codec::FromJson for $type {
            fn from_json(
                value: &::purrdf_lex::json::Value,
            ) -> Result<Self, $crate::projections::json_codec::JsonError> {
                match value.as_str() {
                    Some(text) => <Self as $crate::projections::json_codec::JsonKey>::from_key(text),
                    None => Err($crate::projections::json_codec::JsonError::invalid_type(
                        value,
                        concat!("enum ", stringify!($type)),
                    )),
                }
            }
        }

        impl $crate::projections::json_codec::ToJson for $type {
            fn to_json(&self) -> ::purrdf_lex::json::Value {
                ::purrdf_lex::json::Value::from(self.json_str())
            }
        }

        impl $crate::projections::json_codec::JsonKey for $type {
            fn to_key(&self) -> String {
                self.json_str().to_owned()
            }

            fn from_key(
                key: &str,
            ) -> Result<Self, $crate::projections::json_codec::JsonError> {
                match key {
                    $($spelling => Ok(Self::$variant),)+
                    other => Err($crate::projections::json_codec::JsonError::unknown_variant(
                        other,
                        Self::JSON_VARIANTS,
                    )),
                }
            }
        }
    };
}

pub(crate) use json_string_enum;

// ── Scalars ────────────────────────────────────────────────────────────────

impl FromJson for String {
    fn from_json(value: &Value) -> Result<Self, JsonError> {
        value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| JsonError::invalid_type(value, "a string"))
    }
}

impl ToJson for String {
    fn to_json(&self) -> Value {
        Value::from(self.as_str())
    }
}

impl ToJson for str {
    fn to_json(&self) -> Value {
        Value::from(self)
    }
}

impl JsonKey for String {
    fn to_key(&self) -> String {
        self.clone()
    }

    fn from_key(key: &str) -> Result<Self, JsonError> {
        Ok(key.to_owned())
    }
}

impl FromJson for char {
    /// A string of exactly one Unicode scalar value.
    fn from_json(value: &Value) -> Result<Self, JsonError> {
        let text = value
            .as_str()
            .ok_or_else(|| JsonError::invalid_type(value, "a character"))?;
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(character), None) => Ok(character),
            _ => Err(JsonError::invalid_value(value, "a character")),
        }
    }
}

impl ToJson for char {
    fn to_json(&self) -> Value {
        Value::from(self.to_string())
    }
}

impl FromJson for bool {
    fn from_json(value: &Value) -> Result<Self, JsonError> {
        value
            .as_bool()
            .ok_or_else(|| JsonError::invalid_type(value, "a boolean"))
    }
}

impl ToJson for bool {
    fn to_json(&self) -> Value {
        Value::Bool(*self)
    }
}

macro_rules! json_integer {
    ($($type:ty => $expecting:literal),+ $(,)?) => {$(
        impl FromJson for $type {
            fn from_json(value: &Value) -> Result<Self, JsonError> {
                let number = value
                    .as_number()
                    .filter(|number| number.is_integer())
                    .ok_or_else(|| JsonError::invalid_type(value, $expecting))?;
                number
                    .as_i128()
                    .and_then(|integer| <$type>::try_from(integer).ok())
                    .ok_or_else(|| JsonError::invalid_value(value, $expecting))
            }
        }

        impl ToJson for $type {
            fn to_json(&self) -> Value {
                Value::from(*self)
            }
        }
    )+};
}

json_integer!(
    u8 => "u8",
    u16 => "u16",
    u32 => "u32",
    u64 => "u64",
    usize => "usize",
    i64 => "i64",
);

impl FromJson for Value {
    /// A free-form value, in [`sorted_last_wins`] form.
    fn from_json(value: &Value) -> Result<Self, JsonError> {
        let mut value = value.clone();
        sorted_last_wins(&mut value);
        Ok(value)
    }
}

impl ToJson for Value {
    fn to_json(&self) -> Value {
        self.clone()
    }
}

// ── Containers ─────────────────────────────────────────────────────────────

impl<T: FromJson> FromJson for Option<T> {
    fn from_json(value: &Value) -> Result<Self, JsonError> {
        match value {
            Value::Null => Ok(None),
            value => T::from_json(value).map(Some),
        }
    }
}

impl<T: ToJson> ToJson for Option<T> {
    fn to_json(&self) -> Value {
        self.as_ref().map_or(Value::Null, ToJson::to_json)
    }
}

impl<T: FromJson> FromJson for Box<T> {
    fn from_json(value: &Value) -> Result<Self, JsonError> {
        T::from_json(value).map(Self::new)
    }
}

impl<T: ToJson + ?Sized> ToJson for Box<T> {
    fn to_json(&self) -> Value {
        (**self).to_json()
    }
}

impl<T: ToJson + ?Sized> ToJson for &T {
    fn to_json(&self) -> Value {
        (**self).to_json()
    }
}

/// The items of a JSON array.
fn items(value: &Value) -> Result<&[Value], JsonError> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| JsonError::invalid_type(value, "a sequence"))
}

impl<T: FromJson> FromJson for Vec<T> {
    fn from_json(value: &Value) -> Result<Self, JsonError> {
        items(value)?.iter().map(T::from_json).collect()
    }
}

impl<T: ToJson> ToJson for Vec<T> {
    fn to_json(&self) -> Value {
        self.as_slice().to_json()
    }
}

impl<T: ToJson> ToJson for [T] {
    fn to_json(&self) -> Value {
        Value::Array(self.iter().map(ToJson::to_json).collect())
    }
}

impl<T: FromJson + Ord> FromJson for BTreeSet<T> {
    /// A set: a repeated item is one item.
    fn from_json(value: &Value) -> Result<Self, JsonError> {
        items(value)?.iter().map(T::from_json).collect()
    }
}

impl<T: ToJson> ToJson for BTreeSet<T> {
    fn to_json(&self) -> Value {
        Value::Array(self.iter().map(ToJson::to_json).collect())
    }
}

impl<K: JsonKey + Ord, V: FromJson> FromJson for BTreeMap<K, V> {
    /// A map: a repeated name keeps its last value.
    fn from_json(value: &Value) -> Result<Self, JsonError> {
        let object = value
            .as_object()
            .ok_or_else(|| JsonError::invalid_type(value, "a map"))?;
        let mut map = Self::new();
        for (name, member) in object {
            map.insert(K::from_key(name)?, V::from_json(member)?);
        }
        Ok(map)
    }
}

impl<K: JsonKey, V: ToJson> ToJson for BTreeMap<K, V> {
    /// Members in the map's key order.
    fn to_json(&self) -> Value {
        Value::Object(
            self.iter()
                .map(|(key, value)| (key.to_key(), value.to_json()))
                .collect(),
        )
    }
}

macro_rules! json_tuple {
    ($(($($name:ident),+)),+ $(,)?) => {$(
        impl<$($name: ToJson),+> ToJson for ($($name,)+) {
            /// A tuple is an array of its parts, in order.
            #[allow(non_snake_case)]
            fn to_json(&self) -> Value {
                let ($($name,)+) = self;
                Value::Array(vec![$($name.to_json()),+])
            }
        }
    )+};
}

json_tuple!((A, B), (A, B, C), (A, B, C, D));

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct Pair {
        name: String,
        count: Option<u32>,
    }

    impl FromJson for Pair {
        fn from_json(value: &Value) -> Result<Self, JsonError> {
            let mut fields = Fields::new(value, "struct Pair")?;
            let pair = Self {
                name: fields.required("name")?,
                count: fields.optional("count")?,
            };
            fields.deny_unknown()?;
            Ok(pair)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Colour {
        Red,
        DarkBlue,
    }

    json_string_enum!(Colour { Red => "red", DarkBlue => "dark-blue" });

    fn pair(text: &str) -> Result<Pair, JsonError> {
        from_slice(text.as_bytes())
    }

    #[test]
    fn a_record_reads_declared_members_and_treats_null_as_absent() {
        assert_eq!(
            pair(r#"{"count":3,"name":"a"}"#).expect("record"),
            Pair {
                name: "a".to_owned(),
                count: Some(3)
            }
        );
        assert_eq!(
            pair(r#"{"name":"a","count":null}"#).expect("null optional"),
            pair(r#"{"name":"a"}"#).expect("absent optional")
        );
    }

    #[test]
    fn a_record_refuses_unknown_missing_and_repeated_members() {
        assert_eq!(
            pair(r#"{"name":"a","extra":1}"#)
                .expect_err("unknown")
                .to_string(),
            "unknown field `extra`, expected `name` or `count`"
        );
        assert_eq!(
            pair(r#"{"count":1}"#).expect_err("missing").to_string(),
            "missing field `name`"
        );
        assert_eq!(
            pair(r#"{"name":"a","name":"b"}"#)
                .expect_err("duplicate")
                .to_string(),
            "duplicate field `name`"
        );
    }

    /// A record is an object: the positional array spelling of a record is
    /// refused, and the same members as an object are read.
    #[test]
    fn a_record_is_never_read_from_an_array() {
        assert_eq!(
            pair(r#"["a",1]"#).expect_err("array").to_string(),
            "invalid type: sequence, expected struct Pair"
        );
        assert!(pair(r#"{"name":"a","count":1}"#).is_ok());
    }

    #[test]
    fn integers_refuse_fractions_exponents_and_out_of_range_values() {
        assert_eq!(u32::from_json(&json::read("7").expect("json")), Ok(7));
        assert_eq!(
            u32::from_json(&json::read("7.0").expect("json"))
                .expect_err("fraction")
                .to_string(),
            "invalid type: floating point `7.0`, expected u32"
        );
        assert!(u32::from_json(&json::read("1e2").expect("json")).is_err());
        assert_eq!(
            u8::from_json(&json::read("256").expect("json"))
                .expect_err("range")
                .to_string(),
            "invalid value: integer `256`, expected u8"
        );
        assert_eq!(u8::from_json(&json::read("255").expect("json")), Ok(255));
        assert!(u64::from_json(&json::read("-1").expect("json")).is_err());
        assert_eq!(i64::from_json(&json::read("-1").expect("json")), Ok(-1));
    }

    /// A closed enum is its string spelling: a map spelling of a unit variant
    /// is refused, and the string spelling beside it is read.
    #[test]
    fn a_closed_enum_is_exactly_its_spellings() {
        assert_eq!(
            Colour::from_json(&Value::from("dark-blue")),
            Ok(Colour::DarkBlue)
        );
        assert_eq!(Colour::DarkBlue.to_json(), Value::from("dark-blue"));
        assert_eq!(
            Colour::from_json(&Value::from("blue"))
                .expect_err("unknown")
                .to_string(),
            "unknown variant `blue`, expected `red` or `dark-blue`"
        );
        assert!(Colour::from_json(&json::read(r#"{"red":null}"#).expect("json")).is_err());
        assert!(Colour::from_json(&json::read(r#""red""#).expect("json")).is_ok());
        let map: BTreeMap<Colour, u8> =
            from_slice(br#"{"dark-blue":2,"red":1}"#).expect("keyed map");
        assert_eq!(
            json::write_compact(&map.to_json()),
            r#"{"red":1,"dark-blue":2}"#
        );
    }

    #[test]
    fn free_form_values_are_sorted_and_a_repeat_keeps_its_last_value() {
        let value =
            read_document(br#"{"b":{"y":1,"x":[{"k":1,"k":2}]},"a":0,"b":3}"#).expect("document");
        assert_eq!(json::write_compact(&value), r#"{"a":0,"b":3}"#);
        let value =
            read_document(br#"{"b":{"y":1,"x":[{"k":1,"j":2,"k":3}]},"a":0}"#).expect("document");
        assert_eq!(
            json::write_compact(&value),
            r#"{"a":0,"b":{"x":[{"j":2,"k":3}],"y":1}}"#
        );
    }

    #[test]
    fn maps_keep_the_last_repeat_and_sets_collapse_repeats() {
        let map: BTreeMap<String, u8> = from_slice(br#"{"a":1,"a":2}"#).expect("map");
        assert_eq!(map.get("a"), Some(&2));
        let set: BTreeSet<String> = from_slice(br#"["b","a","b"]"#).expect("set");
        assert_eq!(set.len(), 2);
    }
}
