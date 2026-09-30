// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed decoding of a JSON [`Value`]: strict records, closed string
//! vocabularies, and the [`FromJson`] / [`ToJson`] conversions every
//! configuration, option and plan document in the workspace is read and
//! written through.
//!
//! A type states its JSON shape once, in a [`FromJson`] and a [`ToJson`]
//! impl — a plain record through [`json_record!`](crate::json_record), a closed vocabulary
//! through [`json_string_enum!`](crate::json_string_enum), anything else by hand; the grammar stays in
//! the reader and writer of [`crate::json`]. The record law is stated here once so that every document
//! a PurRDF component reads refuses the same shapes in the same words.
//!
//! # Contract
//!
//! * **A record is an object, read member by member.** [`Record::new`] refuses
//!   anything but an object — an array is never read positionally as a
//!   record. A declared member that is absent is `missing field`, and one that
//!   is written twice is `duplicate field`, since which occurrence was meant is
//!   not the reader's to guess (RFC 8259 §4 leaves a repeated name's meaning
//!   open). A member the record never reads is refused by
//!   [`Record::deny_unknown`] as `unknown field`, naming the declared set; a
//!   record that does not call it ignores such members, repeated or not.
//! * **Absent and `null` are one thing for an optional member**
//!   ([`Record::optional`]); a defaulted member ([`Record::defaulted`]) takes
//!   its default only when absent, so `null` for a list is refused.
//! * **Closed vocabularies are strings.** [`json_string_enum!`](crate::json_string_enum) spells each
//!   variant of a closed enum once; any other string is `unknown variant`.
//! * **Numbers are exact.** An integer type reads only an integer lexeme
//!   (no fraction, no exponent) that fits it; `f32` and `f64` read any number
//!   and refuse one that denotes no finite value of the type.
//! * **Free-form JSON is a sorted, last-wins map.** A [`Value`] decoded through
//!   [`FromJson`] is put in [`sorted_last_wins`] form, so bytes re-emitted
//!   from it are a function of its content alone.
//! * **Refusals name the problem in stable words, and where.** A
//!   [`DecodeError`] reads `invalid type: …, expected …`, `invalid value: …,
//!   expected …`, `unknown field …`, `missing field …`, `duplicate field …`,
//!   `unknown variant …`, or a constructor's validation message, and carries
//!   the RFC 6901 JSON Pointer of the offending value. A caller whose refusals
//!   are its own error type implements [`Within`] for it and reads through the
//!   `_with` methods, so its construction-law refusals pass through unchanged
//!   while its shape refusals gain the same pointer.
//!
//! ```rust
//! use purrdf_lex::json::{self, Value};
//! use purrdf_lex::json::record::{DecodeError, FromJson, Record};
//!
//! struct Pair {
//!     name: String,
//!     count: Option<u32>,
//! }
//!
//! impl FromJson for Pair {
//!     fn from_json(value: &Value) -> Result<Self, DecodeError> {
//!         let mut fields = Record::new(value, "struct Pair")?;
//!         let pair = Self {
//!             name: fields.required("name")?,
//!             count: fields.optional("count")?,
//!         };
//!         fields.deny_unknown()?;
//!         Ok(pair)
//!     }
//! }
//!
//! let pair = Pair::from_json(&json::read(r#"{"name":"a","count":3}"#).unwrap()).unwrap();
//! assert_eq!((pair.name.as_str(), pair.count), ("a", Some(3)));
//! let error = Pair::from_json(&json::read(r#"{"name":"a","count":-1}"#).unwrap()).err().unwrap();
//! assert_eq!(error.to_string(), "invalid value: integer `-1`, expected u32 at /count");
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::{Error, Limits, Number, Object, Value, read_slice, write_compact};
use crate::json_pointer::push_token;

/// A JSON value that does not have the shape its type requires, and the
/// RFC 6901 JSON Pointer of that value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    pointer: String,
    message: String,
}

impl DecodeError {
    /// A refusal carrying a validation message.
    pub fn custom(message: impl fmt::Display) -> Self {
        Self {
            pointer: String::new(),
            message: message.to_string(),
        }
    }

    /// `found` is not the JSON kind `expected` names.
    pub fn invalid_type(found: &Value, expected: &str) -> Self {
        Self::custom(format_args!(
            "invalid type: {}, expected {expected}",
            Unexpected(found)
        ))
    }

    /// `found` has the right kind but a value `expected` does not admit.
    pub fn invalid_value(found: &Value, expected: &str) -> Self {
        Self::custom(format_args!(
            "invalid value: {}, expected {expected}",
            Unexpected(found)
        ))
    }

    /// A string that names none of a closed set of variants.
    pub fn unknown_variant(variant: &str, expected: &[&str]) -> Self {
        Self::custom(format_args!(
            "unknown variant `{variant}`, {}",
            OneOf(expected, "variants")
        ))
    }

    /// A member no field of the record declares.
    pub fn unknown_field(field: &str, expected: &[&str]) -> Self {
        Self::custom(format_args!(
            "unknown field `{field}`, {}",
            OneOf(expected, "fields")
        ))
    }

    /// A mandatory member is absent.
    pub fn missing_field(field: &str) -> Self {
        Self::custom(format_args!("missing field `{field}`"))
    }

    /// A declared member is written more than once.
    pub fn duplicate_field(field: &str) -> Self {
        Self::custom(format_args!("duplicate field `{field}`"))
    }

    /// The JSON Pointer of the offending value, relative to the value the
    /// outermost reader was given (`""` is that value itself).
    pub fn pointer(&self) -> &str {
        &self.pointer
    }

    /// What is wrong, without the pointer.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for DecodeError {
    /// The message, then ` at ` and the pointer when the value is not the
    /// document itself.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)?;
        if !self.pointer.is_empty() {
            write!(formatter, " at {}", self.pointer)?;
        }
        Ok(())
    }
}

impl std::error::Error for DecodeError {}

impl From<Error> for DecodeError {
    fn from(error: Error) -> Self {
        Self::custom(error)
    }
}

/// A refusal that locates itself by JSON Pointer as it leaves each enclosing
/// value.
///
/// [`DecodeError`] implements it; a reader whose refusals are its own error
/// type implements it so that shape refusals raised through [`Record`] and
/// [`items_with`] carry a pointer, while its other refusals pass unchanged.
pub trait Within: From<DecodeError> {
    /// This refusal, raised inside the member or item `token` names.
    #[must_use]
    fn within(self, token: &str) -> Self;
}

impl Within for DecodeError {
    fn within(mut self, token: &str) -> Self {
        let mut pointer = String::with_capacity(1 + token.len() + self.pointer.len());
        push_token(&mut pointer, token);
        pointer.push_str(&self.pointer);
        self.pointer = pointer;
        self
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
pub trait FromJson: Sized {
    /// Decode `value`, validating it exactly as the type's constructor does.
    ///
    /// # Errors
    ///
    /// Returns a [`DecodeError`] naming the first shape or validation failure.
    fn from_json(value: &Value) -> Result<Self, DecodeError>;
}

/// A type written as a JSON value.
pub trait ToJson {
    /// The value, members in the type's declared order.
    fn to_json(&self) -> Value;
}

/// A type that names a JSON object member: a string, or a closed enum spelled
/// as one.
pub trait JsonKey: Sized {
    /// The member name.
    fn to_key(&self) -> String;

    /// Decode a member name.
    ///
    /// # Errors
    ///
    /// Returns a [`DecodeError`] when `key` names no value of the type.
    fn from_key(key: &str) -> Result<Self, DecodeError>;
}

/// Read one JSON document into `T`.
///
/// The document is read under [`Limits::DEFAULT`] and then decoded; a syntax
/// error and a shape error are both a [`DecodeError`].
///
/// # Errors
///
/// Returns the reader's error as a [`DecodeError`], or `T`'s refusal.
pub fn from_slice<T: FromJson>(bytes: &[u8]) -> Result<T, DecodeError> {
    T::from_json(&read_slice(bytes, Limits::DEFAULT)?)
}

/// Read one free-form JSON document in [`sorted_last_wins`] form.
///
/// # Errors
///
/// Returns the reader's error for a document that is not RFC 8259 JSON, or
/// that nests deeper than [`Limits::DEFAULT`].
pub fn read_document(bytes: &[u8]) -> Result<Value, Error> {
    let mut value = read_slice(bytes, Limits::DEFAULT)?;
    sorted_last_wins(&mut value);
    Ok(value)
}

/// Put every object in the tree into name order, keeping only the LAST member
/// of each repeated name.
///
/// This is the one form a free-form value is carried in (see the module
/// contract). The walk is over a heap work list, so depth costs no stack.
pub fn sorted_last_wins(value: &mut Value) {
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
pub fn to_vec<T: ToJson + ?Sized>(value: &T) -> Vec<u8> {
    write_compact(&value.to_json()).into_bytes()
}

/// A [`Value`] taken apart by kind, by value.
///
/// [`Value`] drops over a heap work list, so it implements `Drop` and a
/// `match` cannot move its payload out; [`into_owned`] moves it out once.
#[derive(Debug)]
pub enum Owned {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number, as its lexeme.
    Number(Number),
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
pub fn into_owned(mut value: Value) -> Owned {
    match &mut value {
        Value::Null => Owned::Null,
        Value::Bool(flag) => Owned::Bool(*flag),
        Value::Number(number) => Owned::Number(std::mem::replace(number, Number::from(0_u8))),
        Value::String(text) => Owned::String(std::mem::take(text)),
        Value::Array(items) => Owned::Array(std::mem::take(items)),
        Value::Object(object) => Owned::Object(std::mem::take(object)),
    }
}

/// A record read member by member.
///
/// Every member the type declares is taken through one of the typed readers;
/// [`Record::deny_unknown`] then refuses whatever is left. A refusal raised
/// while reading a member carries that member's name in its pointer.
#[derive(Debug)]
pub struct Record<'a> {
    object: &'a Object,
    taken: Vec<&'static str>,
}

impl<'a> Record<'a> {
    /// Begin reading `value`, which must be an object; `expecting` names the
    /// record in a refusal (`struct LpgConfig`).
    ///
    /// # Errors
    ///
    /// Returns `invalid type` for anything but an object.
    pub fn new(value: &'a Value, expecting: &str) -> Result<Self, DecodeError> {
        match value {
            Value::Object(object) => Ok(Self {
                object,
                taken: Vec::new(),
            }),
            other => Err(DecodeError::invalid_type(other, expecting)),
        }
    }

    /// The member named `name`, if present, marked as declared.
    ///
    /// # Errors
    ///
    /// Returns `duplicate field`, at the member, when the object writes `name`
    /// twice.
    pub fn raw(&mut self, name: &'static str) -> Result<Option<&'a Value>, DecodeError> {
        self.taken.push(name);
        if self.object.count(name) > 1 {
            return Err(DecodeError::duplicate_field(name).within(name));
        }
        Ok(self.object.get(name))
    }

    /// A mandatory member.
    ///
    /// # Errors
    ///
    /// Returns `missing field`, `duplicate field`, or the member's own error.
    pub fn required<T: FromJson>(&mut self, name: &'static str) -> Result<T, DecodeError> {
        self.required_with(name, T::from_json)
    }

    /// A mandatory member, read by `read`.
    ///
    /// # Errors
    ///
    /// Returns `missing field`, `duplicate field`, or `read`'s error located
    /// within the member.
    pub fn required_with<T, E: Within>(
        &mut self,
        name: &'static str,
        read: impl FnOnce(&'a Value) -> Result<T, E>,
    ) -> Result<T, E> {
        match self.raw(name)? {
            Some(value) => read(value).map_err(|error| error.within(name)),
            None => Err(DecodeError::missing_field(name).into()),
        }
    }

    /// An optional member: absent and `null` are both `None`.
    ///
    /// # Errors
    ///
    /// Returns `duplicate field` or the member's own error.
    pub fn optional<T: FromJson>(&mut self, name: &'static str) -> Result<Option<T>, DecodeError> {
        self.optional_with(name, T::from_json)
    }

    /// An optional member read by `read`: absent and `null` are both `None`.
    ///
    /// # Errors
    ///
    /// Returns `duplicate field` or `read`'s error located within the member.
    pub fn optional_with<T, E: Within>(
        &mut self,
        name: &'static str,
        read: impl FnOnce(&'a Value) -> Result<T, E>,
    ) -> Result<Option<T>, E> {
        match self.raw(name)? {
            None | Some(Value::Null) => Ok(None),
            Some(value) => read(value).map(Some).map_err(|error| error.within(name)),
        }
    }

    /// A defaulted member: absent is `T::default()`, and anything written —
    /// `null` included — is read as `T`.
    ///
    /// # Errors
    ///
    /// Returns `duplicate field` or the member's own error.
    pub fn defaulted<T: FromJson + Default>(
        &mut self,
        name: &'static str,
    ) -> Result<T, DecodeError> {
        match self.raw(name)? {
            None => Ok(T::default()),
            Some(value) => T::from_json(value).map_err(|error| error.within(name)),
        }
    }

    /// The string member `name` naming one of `variants`: an enum's tag.
    ///
    /// # Errors
    ///
    /// Returns `missing field`, `invalid type`, or `unknown variant`.
    pub fn tag(&mut self, name: &'static str, variants: &[&str]) -> Result<&'a str, DecodeError> {
        self.required_with(name, |value| {
            let tag = value
                .as_str()
                .ok_or_else(|| DecodeError::invalid_type(value, "variant identifier"))?;
            if variants.contains(&tag) {
                Ok(tag)
            } else {
                Err(DecodeError::unknown_variant(tag, variants))
            }
        })
    }

    /// Refuse every member no reader took.
    ///
    /// # Errors
    ///
    /// Returns `unknown field` for the first undeclared member, naming the
    /// members the record declared.
    pub fn deny_unknown(self) -> Result<(), DecodeError> {
        match self
            .object
            .keys()
            .find(|name| !self.taken.contains(&name.as_str()))
        {
            Some(name) => Err(DecodeError::unknown_field(name, &self.taken)),
            None => Ok(()),
        }
    }
}

/// The items of the array `value`, each read by `read`; a refusal inside an
/// item is located at the item's index.
///
/// # Errors
///
/// Returns `invalid type` for anything but an array, and `read`'s error
/// otherwise.
pub fn items_with<T, E: Within>(
    value: &Value,
    mut read: impl FnMut(&Value) -> Result<T, E>,
) -> Result<Vec<T>, E> {
    let items = value
        .as_array()
        .ok_or_else(|| DecodeError::invalid_type(value, "a sequence"))?;
    items
        .iter()
        .enumerate()
        .map(|(index, item)| read(item).map_err(|error| error.within(&index.to_string())))
        .collect()
}

/// Spell each variant of a closed, field-less enum as one JSON string.
///
/// The macro gives the enum `JSON_VARIANTS`, `json_str`,
/// [`FromJson`](crate::json::record::FromJson),
/// [`ToJson`](crate::json::record::ToJson) and
/// [`JsonKey`](crate::json::record::JsonKey); each spelling is written exactly
/// once. Any other string, and any value that is not a string, is refused.
///
/// ```rust
/// use purrdf_lex::json::Value;
/// use purrdf_lex::json::record::{FromJson, ToJson};
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// enum Colour {
///     Red,
///     DarkBlue,
/// }
///
/// purrdf_lex::json_string_enum!(Colour { Red => "red", DarkBlue => "dark-blue" });
///
/// assert_eq!(Colour::from_json(&Value::from("dark-blue")), Ok(Colour::DarkBlue));
/// assert_eq!(Colour::Red.to_json(), Value::from("red"));
/// assert!(Colour::from_json(&Value::from("blue")).is_err());
/// ```
#[macro_export]
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

        impl $crate::json::record::FromJson for $type {
            fn from_json(
                value: &$crate::json::Value,
            ) -> Result<Self, $crate::json::record::DecodeError> {
                match value.as_str() {
                    Some(text) => <Self as $crate::json::record::JsonKey>::from_key(text),
                    None => Err($crate::json::record::DecodeError::invalid_type(
                        value,
                        concat!("enum ", stringify!($type)),
                    )),
                }
            }
        }

        impl $crate::json::record::ToJson for $type {
            fn to_json(&self) -> $crate::json::Value {
                $crate::json::Value::from(self.json_str())
            }
        }

        impl $crate::json::record::JsonKey for $type {
            fn to_key(&self) -> String {
                self.json_str().to_owned()
            }

            fn from_key(key: &str) -> Result<Self, $crate::json::record::DecodeError> {
                match key {
                    $($spelling => Ok(Self::$variant),)+
                    other => Err($crate::json::record::DecodeError::unknown_variant(
                        other,
                        Self::JSON_VARIANTS,
                    )),
                }
            }
        }
    };
}

/// Write a record type's [`ToJson`](crate::json::record::ToJson) and
/// [`FromJson`](crate::json::record::FromJson) from one member list.
///
/// A record is an object of named members in a fixed order. This macro is the
/// one body every such impl instantiates, so the record law of this module —
/// members written in declared order; read through [`Record`] with
/// `missing field`, `duplicate field` and `unknown field` refused — is stated
/// once rather than re-spelled per type. Each member is
/// `"json-name" => field: reader` — the name any `&'static str` expression,
/// such as `stringify!(field)` in a macro — where `reader` is a [`Record`] method
/// (`required`, `optional`, `defaulted`), optionally turbofished with the
/// member's type when the constructor alone cannot infer it.
///
/// * `impl ToJson for T { "name" => field, … }` writes the members in order,
///   each through its own [`ToJson`](crate::json::record::ToJson).
/// * `impl FromJson for T as "struct T" { "name" => field: required, … }`
///   reads the members in order, refuses any other member, and builds
///   `Self { field, … }`; a trailing `=> T::new` builds `T::new(field, …)?`
///   instead, so the value is validated by the constructor callers use.
/// * `T as "struct T" { … }` writes both impls from the one list.
///
/// ```rust
/// use purrdf_lex::json::{self, Value};
/// use purrdf_lex::json::record::{DecodeError, FromJson, ToJson};
///
/// #[derive(Debug, PartialEq)]
/// struct Span {
///     start: u32,
///     label: Option<String>,
/// }
///
/// purrdf_lex::json_record!(Span as "struct Span" {
///     "start" => start: required,
///     "label" => label: optional,
/// });
///
/// let span = Span { start: 3, label: None };
/// assert_eq!(json::write_compact(&span.to_json()), r#"{"start":3,"label":null}"#);
/// assert_eq!(Span::from_json(&json::read(r#"{"start":3}"#).unwrap()), Ok(span));
/// assert!(Span::from_json(&json::read(r#"{"start":3,"end":4}"#).unwrap()).is_err());
/// ```
#[macro_export]
macro_rules! json_record {
    (impl ToJson for $type:ty { $($name:expr => $field:ident),* $(,)? }) => {
        impl $crate::json::record::ToJson for $type {
            fn to_json(&self) -> $crate::json::Value {
                $crate::json::Value::Object(
                    $crate::json::Object::new()
                        $(.with($name, $crate::json::record::ToJson::to_json(&self.$field)))*,
                )
            }
        }
    };
    (impl FromJson for $type:ty as $expecting:literal {
        $($name:expr => $field:ident : $reader:ident $(::<$member:ty>)?),* $(,)?
    } $(=> $constructor:path)?) => {
        impl $crate::json::record::FromJson for $type {
            #[allow(
                clippy::needless_question_mark,
                reason = "a constructor's refusal is any error a DecodeError is built from"
            )]
            fn from_json(
                value: &$crate::json::Value,
            ) -> Result<Self, $crate::json::record::DecodeError> {
                let mut record = $crate::json::record::Record::new(value, $expecting)?;
                $(let $field $(: $member)? = record.$reader($name)?;)*
                record.deny_unknown()?;
                $crate::json_record!(@build $($constructor)? { $($field),* })
            }
        }
    };
    (@build { $($field:ident),* }) => {
        Ok(Self { $($field),* })
    };
    (@build $constructor:path { $($field:ident),* }) => {
        Ok($constructor($($field),*)?)
    };
    ($type:ty as $expecting:literal {
        $($name:expr => $field:ident : $reader:ident $(::<$member:ty>)?),* $(,)?
    } $(=> $constructor:path)?) => {
        $crate::json_record!(impl ToJson for $type { $($name => $field),* });
        $crate::json_record!(impl FromJson for $type as $expecting {
            $($name => $field : $reader $(::<$member>)?),*
        } $(=> $constructor)?);
    };
}

// ── Scalars ────────────────────────────────────────────────────────────────

impl FromJson for String {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| DecodeError::invalid_type(value, "a string"))
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

    fn from_key(key: &str) -> Result<Self, DecodeError> {
        Ok(key.to_owned())
    }
}

impl FromJson for char {
    /// A string of exactly one Unicode scalar value.
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let text = value
            .as_str()
            .ok_or_else(|| DecodeError::invalid_type(value, "a character"))?;
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(character), None) => Ok(character),
            _ => Err(DecodeError::invalid_value(value, "a character")),
        }
    }
}

impl ToJson for char {
    fn to_json(&self) -> Value {
        Value::from(self.to_string())
    }
}

impl FromJson for bool {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        value
            .as_bool()
            .ok_or_else(|| DecodeError::invalid_type(value, "a boolean"))
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
            /// An integer lexeme — no fraction, no exponent; `-0` is `0` —
            /// whose value fits the type.
            fn from_json(value: &Value) -> Result<Self, DecodeError> {
                let number = value
                    .as_number()
                    .filter(|number| number.is_integer())
                    .ok_or_else(|| DecodeError::invalid_type(value, $expecting))?;
                number
                    .as_i128()
                    .and_then(|integer| <$type>::try_from(integer).ok())
                    .ok_or_else(|| DecodeError::invalid_value(value, $expecting))
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
    i8 => "i8",
    i16 => "i16",
    i32 => "i32",
    i64 => "i64",
    i128 => "i128",
    isize => "isize",
);

impl FromJson for f64 {
    /// Any number, correctly rounded to binary64 ([`Number::as_f64`]); one
    /// beyond binary64's range is refused, since JSON writes no infinity.
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let number = value
            .as_number()
            .ok_or_else(|| DecodeError::invalid_type(value, "f64"))?;
        let float = number.as_f64();
        if float.is_finite() {
            Ok(float)
        } else {
            Err(DecodeError::invalid_value(value, "a finite f64"))
        }
    }
}

impl ToJson for f64 {
    /// The shortest round-trip lexeme; `null` for NaN and the infinities.
    fn to_json(&self) -> Value {
        Value::from(*self)
    }
}

impl FromJson for f32 {
    /// Any number, rounded to binary64 and then to binary32 — a double rounding
    /// that is exact for every decimal of binary32's precision; one beyond
    /// binary32's range is refused, since JSON writes no infinity.
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let number = value
            .as_number()
            .ok_or_else(|| DecodeError::invalid_type(value, "f32"))?;
        #[allow(
            clippy::cast_possible_truncation,
            reason = "the binary64 to binary32 rounding is the conversion this reads"
        )]
        let float = number.as_f64() as Self;
        if float.is_finite() {
            Ok(float)
        } else {
            Err(DecodeError::invalid_value(value, "a finite f32"))
        }
    }
}

impl ToJson for f32 {
    /// The shortest binary32 round-trip lexeme; `null` for NaN and the
    /// infinities.
    fn to_json(&self) -> Value {
        Value::from(*self)
    }
}

impl FromJson for Value {
    /// A free-form value, in [`sorted_last_wins`] form.
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
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

impl ToJson for Object {
    fn to_json(&self) -> Value {
        Value::Object(self.clone())
    }
}

impl ToJson for Number {
    fn to_json(&self) -> Value {
        Value::Number(self.clone())
    }
}

// ── Containers ─────────────────────────────────────────────────────────────

impl<T: FromJson> FromJson for Option<T> {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
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
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
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

impl<T: FromJson> FromJson for Vec<T> {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        items_with(value, T::from_json)
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

impl<T: ToJson, const N: usize> ToJson for [T; N] {
    fn to_json(&self) -> Value {
        self.as_slice().to_json()
    }
}

impl<T: FromJson + Ord> FromJson for BTreeSet<T> {
    /// A set: a repeated item is one item.
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        items_with(value, T::from_json)
            .map(Vec::into_iter)
            .map(Iterator::collect)
    }
}

impl<T: ToJson> ToJson for BTreeSet<T> {
    fn to_json(&self) -> Value {
        Value::Array(self.iter().map(ToJson::to_json).collect())
    }
}

impl<K: JsonKey + Ord, V: FromJson> FromJson for BTreeMap<K, V> {
    /// A map: a repeated name keeps its last value.
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let object = value
            .as_object()
            .ok_or_else(|| DecodeError::invalid_type(value, "a map"))?;
        let mut map = Self::new();
        for (name, member) in object {
            let key = K::from_key(name).map_err(|error| error.within(name))?;
            let value = V::from_json(member).map_err(|error| error.within(name))?;
            map.insert(key, value);
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
mod tests;
