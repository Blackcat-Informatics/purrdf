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

/// Move an already owned JSON payload, or construct a primitive spelling,
/// through an original native memory account. Owned payloads must be admitted
/// by that account before birth, or be explicitly provided by their caller.
pub trait IntoJsonNative {
    /// The original value, without an alternate record or numeric codec.
    /// # Errors
    /// Returns checked layout, admission or physical allocation refusal.
    fn into_json_native<S: crate::allocation::Admission + ?Sized>(
        self,
        memory: &mut crate::allocation::Memory<'_, S>,
    ) -> Result<Value, crate::allocation::StorageError>;
}

macro_rules! native_owned_value {
    ($($source:ty),+ $(,)?) => { $(
        impl IntoJsonNative for $source {
            fn into_json_native<S: crate::allocation::Admission + ?Sized>(
                self,
                _memory: &mut crate::allocation::Memory<'_, S>,
            ) -> Result<Value, crate::allocation::StorageError> {
                Ok(self.into())
            }
        }
    )+ };
}
native_owned_value!(Value, Object, Vec<Value>, String, bool, ());

impl IntoJsonNative for &str {
    fn into_json_native<S: crate::allocation::Admission + ?Sized>(
        self,
        memory: &mut crate::allocation::Memory<'_, S>,
    ) -> Result<Value, crate::allocation::StorageError> {
        memory.string(self).map(Value::String)
    }
}

impl<T: IntoJsonNative> IntoJsonNative for Option<T> {
    fn into_json_native<S: crate::allocation::Admission + ?Sized>(
        self,
        memory: &mut crate::allocation::Memory<'_, S>,
    ) -> Result<Value, crate::allocation::StorageError> {
        self.map_or(Ok(Value::Null), |value| value.into_json_native(memory))
    }
}

macro_rules! native_integer_value {
    ($($source:ty),+ $(,)?) => { $(
        impl IntoJsonNative for $source {
            fn into_json_native<S: crate::allocation::Admission + ?Sized>(
                self,
                memory: &mut crate::allocation::Memory<'_, S>,
            ) -> Result<Value, crate::allocation::StorageError> {
                memory.format(&self).map(Number::from_valid).map(Value::Number)
            }
        }
    )+ };
}
native_integer_value!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

/// Storage used by strict record metadata and diagnostics. Implementations
/// borrow the original native account; these methods create no alternate codec.
pub trait RecordAllocation {
    /// Retain native diagnostic fields and presentation under the original account.
    fn diagnostic(
        &self,
        reason: DecodeReason<'_>,
    ) -> Result<DecodeError, crate::allocation::StorageError>;
    /// Destroy a retained diagnostic before refunding its original owned buffers.
    fn release_diagnostic(&self, error: DecodeError)
    -> Result<(), crate::allocation::StorageError>;
    /// Admit and append one declared field name.
    fn take(
        &self,
        fields: &mut Vec<&'static str>,
        name: &'static str,
    ) -> Result<(), crate::allocation::StorageError>;
    /// Destroy the original field buffer before refunding its layout.
    fn release(&self, fields: Vec<&'static str>) -> Result<(), crate::allocation::StorageError>;
    /// Copy typed UTF-8 through the original fallible string home.
    fn text(&self, text: &str) -> Result<String, crate::allocation::StorageError>;
    /// Destroy transient text before refunding its original capacity.
    fn release_text(&self, text: String) -> Result<(), crate::allocation::StorageError>;
    /// Render the original borrowed diagnostic into admitted text.
    fn message(
        &self,
        message: &dyn fmt::Display,
    ) -> Result<String, crate::allocation::StorageError>;
    /// Prepend a pointer token while retaining old and new strings through growth.
    fn pointer(
        &self,
        token: &str,
        pointer: String,
    ) -> Result<String, crate::allocation::StorageError>;
}

/// A stack borrow of one original native memory for strict record decoding.
/// Calls borrow it only while growing a buffer; nested field readers can use
/// the same account without keeping a mutable borrow across their callbacks.
pub struct RecordMemory<'m, 'storage, S: crate::allocation::Admission + ?Sized> {
    memory: &'m std::cell::RefCell<crate::allocation::Memory<'storage, S>>,
}
purrdf_hash::debug_non_exhaustive!([S: crate::allocation::Admission + ?Sized] RecordMemory<'_, '_, S> {});

impl<'m, 'storage, S: crate::allocation::Admission + ?Sized> RecordMemory<'m, 'storage, S> {
    /// Borrow an account established before any record is constructed.
    #[must_use]
    pub const fn new(
        memory: &'m std::cell::RefCell<crate::allocation::Memory<'storage, S>>,
    ) -> Self {
        Self { memory }
    }
}
impl<S: crate::allocation::Admission + ?Sized> RecordAllocation for RecordMemory<'_, '_, S> {
    fn diagnostic(
        &self,
        reason: DecodeReason<'_>,
    ) -> Result<DecodeError, crate::allocation::StorageError> {
        DecodeError::capture_with_memory(reason, &mut self.memory.borrow_mut())
    }
    fn release_diagnostic(
        &self,
        error: DecodeError,
    ) -> Result<(), crate::allocation::StorageError> {
        error.release_with_memory(&mut self.memory.borrow_mut())
    }
    fn take(
        &self,
        fields: &mut Vec<&'static str>,
        name: &'static str,
    ) -> Result<(), crate::allocation::StorageError> {
        self.memory.borrow_mut().push(fields, name)
    }
    fn release(&self, fields: Vec<&'static str>) -> Result<(), crate::allocation::StorageError> {
        self.memory.borrow_mut().release_vec(fields)
    }
    fn text(&self, text: &str) -> Result<String, crate::allocation::StorageError> {
        self.memory.borrow_mut().string(text)
    }
    fn release_text(&self, text: String) -> Result<(), crate::allocation::StorageError> {
        self.memory.borrow_mut().release_string(text)
    }
    fn message(
        &self,
        message: &dyn fmt::Display,
    ) -> Result<String, crate::allocation::StorageError> {
        self.memory.borrow_mut().format(message)
    }
    fn pointer(
        &self,
        token: &str,
        pointer: String,
    ) -> Result<String, crate::allocation::StorageError> {
        let mut memory = self.memory.borrow_mut();
        let result = memory.scope(|memory| {
            let mut joined = String::new();
            crate::json_pointer::push_token_with_memory(&mut joined, token, memory)?;
            memory.push_str(&mut joined, &pointer)?;
            Ok(joined)
        });
        match (result, memory.release_string(pointer)) {
            (Ok(joined), Ok(())) => Ok(joined),
            (Err(original), _) => Err(original),
            (Ok(joined), Err(original)) => {
                let _ = memory.release_string(joined);
                Err(original)
            }
        }
    }
}

/// A JSON value that does not have the shape its type requires, and the
/// RFC 6901 JSON Pointer of that value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    pointer: String,
    message: String,
    failure: error::Failure,
}

mod error;
pub use error::DecodeReason;

impl DecodeError {
    /// A refusal carrying a validation message.
    pub fn custom(message: impl fmt::Display) -> Self {
        Self::custom_native(message, None)
    }

    /// Render the same diagnostic under an original record account.
    #[must_use]
    pub fn custom_native(
        message: impl fmt::Display,
        allocation: Option<&dyn RecordAllocation>,
    ) -> Self {
        Self::from_reason_native(DecodeReason::Message(&message), allocation)
    }

    /// A typed physical refusal carrying no dynamically allocated diagnostic.
    #[must_use]
    pub const fn from_storage(storage: crate::allocation::StorageError) -> Self {
        Self {
            pointer: String::new(),
            message: String::new(),
            failure: error::Failure::Storage(storage),
        }
    }

    /// The original physical cause, distinct from malformed input.
    #[must_use]
    pub const fn storage_error(&self) -> Option<crate::allocation::StorageError> {
        self.failure.storage_error()
    }

    /// `found` is not the JSON kind `expected` names.
    pub fn invalid_type(found: &Value, expected: &str) -> Self {
        Self::invalid_type_native(found, expected, None)
    }

    /// The same stable record refusal under original native storage.
    #[must_use]
    pub fn invalid_type_native(
        found: &Value,
        expected: &str,
        allocation: Option<&dyn RecordAllocation>,
    ) -> Self {
        Self::from_reason_native(DecodeReason::InvalidType { found, expected }, allocation)
    }

    /// `found` has the right kind but a value `expected` does not admit.
    pub fn invalid_value(found: &Value, expected: &str) -> Self {
        Self::invalid_value_native(found, expected, None)
    }

    /// The same stable record refusal under original native storage.
    #[must_use]
    pub fn invalid_value_native(
        found: &Value,
        expected: &str,
        allocation: Option<&dyn RecordAllocation>,
    ) -> Self {
        Self::from_reason_native(DecodeReason::InvalidValue { found, expected }, allocation)
    }

    /// A string that names none of a closed set of variants.
    pub fn unknown_variant(variant: &str, expected: &[&str]) -> Self {
        Self::unknown_variant_native(variant, expected, None)
    }

    /// The same stable record refusal under original native storage.
    #[must_use]
    pub fn unknown_variant_native(
        variant: &str,
        expected: &[&str],
        allocation: Option<&dyn RecordAllocation>,
    ) -> Self {
        Self::from_reason_native(
            DecodeReason::UnknownVariant { variant, expected },
            allocation,
        )
    }

    /// A member no field of the record declares.
    pub fn unknown_field(field: &str, expected: &[&str]) -> Self {
        Self::unknown_field_native(field, expected, None)
    }

    /// The same stable record refusal under original native storage.
    #[must_use]
    pub fn unknown_field_native(
        field: &str,
        expected: &[&str],
        allocation: Option<&dyn RecordAllocation>,
    ) -> Self {
        Self::from_reason_native(DecodeReason::UnknownField { field, expected }, allocation)
    }

    /// A mandatory member is absent.
    pub fn missing_field(field: &str) -> Self {
        Self::missing_field_native(field, None)
    }

    /// The same stable record refusal under original native storage.
    #[must_use]
    pub fn missing_field_native(field: &str, allocation: Option<&dyn RecordAllocation>) -> Self {
        Self::from_reason_native(DecodeReason::MissingField(field), allocation)
    }

    /// A declared member is written more than once.
    pub fn duplicate_field(field: &str) -> Self {
        Self::duplicate_field_native(field, None)
    }

    /// The same stable record refusal under original native storage.
    #[must_use]
    pub fn duplicate_field_native(field: &str, allocation: Option<&dyn RecordAllocation>) -> Self {
        Self::from_reason_native(DecodeReason::DuplicateField(field), allocation)
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
        if let Some(error) = self.storage_error() {
            return error.fmt(formatter);
        }
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
        Self::from_reason_native(DecodeReason::Json(error), None)
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

    /// Prefix the same diagnostic through the original native record account.
    /// Custom engine errors can preserve their own already retained owner.
    #[must_use]
    fn within_native(self, token: &str, allocation: Option<&dyn RecordAllocation>) -> Self {
        let _ = allocation;
        self.within(token)
    }

    /// Locate an array element. A custom engine cause can keep its existing
    /// owner and avoid allocating a pointer it does not expose.
    #[must_use]
    fn within_index_native(self, index: usize, allocation: Option<&dyn RecordAllocation>) -> Self {
        let _ = allocation;
        self.within(&index.to_string())
    }
}

impl Within for DecodeError {
    fn within(self, token: &str) -> Self {
        self.within_native(token, None)
    }

    fn within_native(mut self, token: &str, allocation: Option<&dyn RecordAllocation>) -> Self {
        if self.storage_error().is_some() {
            return self;
        }
        match allocation {
            Some(allocation) => {
                match allocation.pointer(token, std::mem::take(&mut self.pointer)) {
                    Ok(pointer) => self.pointer = pointer,
                    Err(error) => {
                        let _ = allocation.release_diagnostic(self);
                        return Self::from_storage(error);
                    }
                }
            }
            None => {
                let mut pointer = String::with_capacity(1 + token.len() + self.pointer.len());
                push_token(&mut pointer, token);
                pointer.push_str(&self.pointer);
                self.pointer = pointer;
            }
        }
        self
    }

    fn within_index_native(self, index: usize, allocation: Option<&dyn RecordAllocation>) -> Self {
        if self.storage_error().is_some() {
            return self;
        }
        let Some(allocation) = allocation else {
            return self.within(&index.to_string());
        };
        let token = match allocation.message(&index) {
            Ok(token) => token,
            Err(error) => {
                let _ = allocation.release_diagnostic(self);
                return Self::from_storage(error);
            }
        };
        let result = self.within_native(&token, Some(allocation));
        match allocation.release_text(token) {
            Ok(()) => result,
            Err(error) if result.storage_error().is_none() => {
                let _ = allocation.release_diagnostic(result);
                Self::from_storage(error)
            }
            Err(_) => result,
        }
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

/// A typed value whose construction uses the original strict-record account.
/// Primitive implementations share their original conversion and refusal law.
pub trait FromJsonNative: Sized {
    /// Decode without a resident allocation on a selected native account.
    ///
    /// # Errors
    ///
    /// Returns the original shape, lexical or physical refusal.
    fn from_json_native(
        value: &Value,
        allocation: Option<&dyn RecordAllocation>,
    ) -> Result<Self, DecodeError>;
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
pub struct Record<'a> {
    object: &'a Object,
    taken: Vec<&'static str>,
    allocation: Option<&'a dyn RecordAllocation>,
}
purrdf_hash::debug_non_exhaustive!(Record<'_> { taken });

impl Drop for Record<'_> {
    fn drop(&mut self) {
        if let Some(allocation) = self.allocation {
            // A shrink refusal is retained by the account; this destructor has
            // already destroyed the exact original metadata buffer.
            let _ = allocation.release(std::mem::take(&mut self.taken));
        }
    }
}

impl<'a> Record<'a> {
    /// Begin reading `value`, which must be an object; `expecting` names the
    /// record in a refusal (`struct LpgConfig`).
    ///
    /// # Errors
    ///
    /// Returns `invalid type` for anything but an object.
    pub fn new(value: &'a Value, expecting: &str) -> Result<Self, DecodeError> {
        Self::new_native(value, expecting, None)
    }

    /// Begin the same strict record with original metadata/diagnostic storage.
    ///
    /// # Errors
    ///
    /// Returns the original wrong-kind or physical refusal.
    pub fn new_native(
        value: &'a Value,
        expecting: &str,
        allocation: Option<&'a dyn RecordAllocation>,
    ) -> Result<Self, DecodeError> {
        match value {
            Value::Object(object) => Ok(Self {
                object,
                taken: Vec::new(),
                allocation,
            }),
            other => Err(DecodeError::invalid_type_native(
                other, expecting, allocation,
            )),
        }
    }

    /// Read a native primitive through the original record law.
    ///
    /// # Errors
    ///
    /// Returns missing/duplicate fields, lexical or physical refusal.
    pub fn required_native<T: FromJsonNative>(
        &mut self,
        name: &'static str,
    ) -> Result<T, DecodeError> {
        let allocation = self.allocation;
        self.required_with(name, |value| T::from_json_native(value, allocation))
    }

    /// Read an optional native primitive; absent and null remain equivalent.
    ///
    /// # Errors
    ///
    /// Returns the original field, lexical or physical refusal.
    pub fn optional_native<T: FromJsonNative>(
        &mut self,
        name: &'static str,
    ) -> Result<Option<T>, DecodeError> {
        let allocation = self.allocation;
        self.optional_with(name, |value| T::from_json_native(value, allocation))
    }

    /// The member named `name`, if present, marked as declared.
    ///
    /// # Errors
    ///
    /// Returns `duplicate field`, at the member, when the object writes `name`
    /// twice.
    pub fn raw(&mut self, name: &'static str) -> Result<Option<&'a Value>, DecodeError> {
        match self.allocation {
            Some(allocation) => allocation
                .take(&mut self.taken, name)
                .map_err(DecodeError::from_storage)?,
            None => self.taken.push(name),
        }
        if self.object.count(name) > 1 {
            return Err(DecodeError::duplicate_field_native(name, self.allocation)
                .within_native(name, self.allocation));
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
            Some(value) => read(value).map_err(|error| error.within_native(name, self.allocation)),
            None => Err(DecodeError::missing_field_native(name, self.allocation).into()),
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
            Some(value) => read(value)
                .map(Some)
                .map_err(|error| error.within_native(name, self.allocation)),
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
        let allocation = self.allocation;
        self.required_with(name, |value| {
            let tag = value.as_str().ok_or_else(|| {
                DecodeError::invalid_type_native(value, "variant identifier", allocation)
            })?;
            if variants.contains(&tag) {
                Ok(tag)
            } else {
                Err(DecodeError::unknown_variant_native(
                    tag, variants, allocation,
                ))
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
        match self.first_unknown() {
            Some((name, _)) => Err(DecodeError::unknown_field_native(
                name,
                &self.taken,
                self.allocation,
            )),
            None => Ok(()),
        }
    }

    /// The first undeclared member in original object order, with its native value.
    /// This borrows the original input without rendering or decoding it again.
    pub fn first_unknown(&self) -> Option<(&'a str, &'a Value)> {
        self.object
            .iter()
            .find(|(name, _)| !self.taken.contains(&name.as_str()))
            .map(|(name, value)| (name.as_str(), value))
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
    read: impl FnMut(&Value) -> Result<T, E>,
) -> Result<Vec<T>, E> {
    let mut resident = crate::allocation::Resident;
    let memory = std::cell::RefCell::new(crate::allocation::Memory::new(&mut resident));
    items_with_memory(value, &memory, None, read)
}

/// Decode the same ordered array with original admission before its vector
/// or element factory grows. Nested readers borrow the same account separately.
///
/// # Errors
///
/// Returns the original shape/element refusal, with its array-index pointer,
/// or an original physical refusal. Partial elements die on every error.
pub fn items_with_memory<T, E: Within, S: crate::allocation::Admission + ?Sized>(
    value: &Value,
    memory: &std::cell::RefCell<crate::allocation::Memory<'_, S>>,
    allocation: Option<&dyn RecordAllocation>,
    mut read: impl FnMut(&Value) -> Result<T, E>,
) -> Result<Vec<T>, E> {
    let items = value
        .as_array()
        .ok_or_else(|| DecodeError::invalid_type_native(value, "a sequence", allocation))?;
    let mut output = Vec::new();
    memory
        .borrow_mut()
        .reserve(&mut output, items.len())
        .map_err(DecodeError::from_storage)?;
    for (index, item) in items.iter().enumerate() {
        output.push(read(item).map_err(|error| error.within_index_native(index, allocation))?);
    }
    Ok(output)
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

impl FromJsonNative for String {
    fn from_json_native(
        value: &Value,
        allocation: Option<&dyn RecordAllocation>,
    ) -> Result<Self, DecodeError> {
        let text = value
            .as_str()
            .ok_or_else(|| DecodeError::invalid_type_native(value, "a string", allocation))?;
        match allocation {
            Some(allocation) => allocation.text(text).map_err(DecodeError::from_storage),
            None => Ok(text.to_owned()),
        }
    }
}

impl FromJsonNative for bool {
    fn from_json_native(
        value: &Value,
        allocation: Option<&dyn RecordAllocation>,
    ) -> Result<Self, DecodeError> {
        value
            .as_bool()
            .ok_or_else(|| DecodeError::invalid_type_native(value, "a boolean", allocation))
    }
}

// ── Scalars ────────────────────────────────────────────────────────────────

impl FromJson for String {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        Self::from_json_native(value, None)
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
        Self::from_json_native(value, None)
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
            fn from_json(value: &Value) -> Result<Self, DecodeError> {
                Self::from_json_native(value, None)
            }
        }
        impl FromJsonNative for $type {
            fn from_json_native(value: &Value, allocation: Option<&dyn RecordAllocation>) -> Result<Self, DecodeError> {
                let number = value
                    .as_number()
                    .filter(|number| number.is_integer())
                    .ok_or_else(|| DecodeError::invalid_type_native(value, $expecting, allocation))?;
                number
                    .as_i128()
                    .and_then(|integer| <$type>::try_from(integer).ok())
                    .ok_or_else(|| DecodeError::invalid_value_native(value, $expecting, allocation))
            }
        }
        impl ToJson for $type {
            fn to_json(&self) -> Value { Value::from(*self) }
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
