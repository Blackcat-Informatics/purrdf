// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The JSON value model every schema emitter in this crate builds on:
//! [`purrdf_lex::json::Value`], plus the [`json!`] literal builder and the
//! [`ToJson`] conversion it applies to interpolated Rust values.
//!
//! # Member order
//!
//! Every emitter here writes each object with its members in name order, so
//! the output bytes are a function of the schema alone and never of the order
//! a compilation step happened to visit it in. The builders keep that
//! invariant by construction: [`json!`] sorts an object literal's members,
//! [`object`] turns a name-ordered [`Map`] into an object, and
//! [`ToJson`] for a [`BTreeMap`] yields its members in key order. A literal
//! that repeats a name keeps the last value, as a map insert does. A value
//! read from outside (a document handed in by a caller) is brought to the
//! same order with [`Value::sort_keys`] at the reader.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(crate) use purrdf_lex::json::{Number, Object, Value, write_compact, write_pretty};

/// A name-ordered map of object members under construction.
pub(crate) type Map<K, V> = BTreeMap<K, V>;

/// The object whose members are `map`'s entries, in name order.
pub(crate) fn object(map: Map<String, Value>) -> Value {
    Value::Object(map.into_iter().collect())
}

/// The members of `value` when it is an object, or `value` itself back.
pub(crate) fn into_object(mut value: Value) -> Result<Object, Value> {
    match &mut value {
        Value::Object(object) => Ok(std::mem::take(object)),
        _ => Err(value),
    }
}

/// The items of `value` when it is an array, or `value` itself back.
pub(crate) fn into_array(mut value: Value) -> Result<Vec<Value>, Value> {
    match &mut value {
        Value::Array(items) => Ok(std::mem::take(items)),
        _ => Err(value),
    }
}

/// Insert `name` into `object`, keeping its members in name order: replace the
/// member of that name in place, or insert it at its sorted position.
///
/// Returns the replaced value.
pub(crate) fn insert_sorted(
    object: &mut Object,
    name: impl Into<String>,
    value: impl Into<Value>,
) -> Option<Value> {
    let name = name.into();
    let value = value.into();
    if let Some(slot) = object.get_mut(&name) {
        return Some(std::mem::replace(slot, value));
    }
    let mut members = std::mem::take(object).into_members();
    let at = members.partition_point(|(member, _)| member.as_str() < name.as_str());
    members.insert(at, (name, value));
    *object = Object::from(members);
    None
}

/// `text` as a JSON string literal, quotes included, in the one spelling
/// every writer here uses ([`purrdf_lex::json::Format::COMPACT`]'s).
pub(crate) fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    purrdf_lex::json_escape::push_string(&mut out, text, purrdf_lex::json::Format::COMPACT.escapes);
    out
}

/// Kind predicates over a [`Value`], each a `matches!` on its variant.
pub(crate) trait ValueKind {
    /// Whether this is a string.
    fn is_string(&self) -> bool;
    /// Whether this is `true` or `false`.
    fn is_boolean(&self) -> bool;
    /// Whether this is a number.
    fn is_number(&self) -> bool;
    /// Whether this is an array.
    fn is_array(&self) -> bool;
    /// Whether this is an object.
    fn is_object(&self) -> bool;
    /// Whether this is a number whose lexeme is an integer fitting `u64`.
    fn is_u64(&self) -> bool;
    /// Whether this is a number whose lexeme is an integer fitting `i64`.
    fn is_i64(&self) -> bool;
    /// This number's nearest binary64, when that is finite: a lexeme beyond
    /// the binary64 range has no binary64 to compare or print.
    fn as_finite_f64(&self) -> Option<f64>;
}

impl ValueKind for Value {
    fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    fn is_boolean(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }

    fn is_u64(&self) -> bool {
        self.as_u64().is_some()
    }

    fn is_i64(&self) -> bool {
        self.as_i64().is_some()
    }

    fn as_finite_f64(&self) -> Option<f64> {
        self.as_number().and_then(NumberKind::as_finite_f64)
    }
}

/// Integer predicates over a [`Number`]'s lexeme.
pub(crate) trait NumberKind {
    /// Whether the lexeme is an integer fitting `u64`.
    fn is_u64(&self) -> bool;
    /// Whether the lexeme is an integer fitting `i64`.
    fn is_i64(&self) -> bool;
    /// The lexeme's nearest binary64, when that is finite.
    fn as_finite_f64(&self) -> Option<f64>;
}

impl NumberKind for Number {
    fn is_u64(&self) -> bool {
        self.as_u64().is_some()
    }

    fn is_i64(&self) -> bool {
        self.as_i64().is_some()
    }

    fn as_finite_f64(&self) -> Option<f64> {
        Some(self.as_f64()).filter(|value| value.is_finite())
    }
}

/// Set the member `name` of the object `value` in name order
/// ([`insert_sorted`]); a `null` becomes an empty object first, as indexing
/// assignment does.
///
/// # Panics
///
/// When `value` is neither `null` nor an object.
pub(crate) fn set_member(value: &mut Value, name: &str, member: impl Into<Value>) {
    if value.is_null() {
        *value = Value::Object(Object::new());
    }
    let object = value
        .as_object_mut()
        .expect("a member is set only on an object");
    insert_sorted(object, name, member);
}

/// A Rust value's JSON form: the conversion [`json!`] applies to every
/// interpolated expression.
pub(crate) trait ToJson {
    /// This value as JSON.
    fn to_json(&self) -> Value;
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

impl ToJson for str {
    fn to_json(&self) -> Value {
        Value::from(self)
    }
}

impl ToJson for String {
    fn to_json(&self) -> Value {
        Value::from(self.as_str())
    }
}

impl ToJson for Cow<'_, str> {
    fn to_json(&self) -> Value {
        Value::from(self.as_ref())
    }
}

impl ToJson for char {
    fn to_json(&self) -> Value {
        Value::from(self.to_string())
    }
}

impl ToJson for bool {
    fn to_json(&self) -> Value {
        Value::Bool(*self)
    }
}

impl ToJson for f64 {
    fn to_json(&self) -> Value {
        Value::from(*self)
    }
}

impl ToJson for f32 {
    fn to_json(&self) -> Value {
        Value::from(*self)
    }
}

macro_rules! integer_to_json {
    ($($t:ty),* $(,)?) => {$(
        impl ToJson for $t {
            fn to_json(&self) -> Value {
                Value::from(*self)
            }
        }
    )*};
}

integer_to_json!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

impl ToJson for () {
    fn to_json(&self) -> Value {
        Value::Null
    }
}

impl<T: ToJson + ?Sized> ToJson for &T {
    fn to_json(&self) -> Value {
        (**self).to_json()
    }
}

impl<T: ToJson + ?Sized> ToJson for Box<T> {
    fn to_json(&self) -> Value {
        (**self).to_json()
    }
}

impl<T: ToJson + ?Sized> ToJson for Arc<T> {
    fn to_json(&self) -> Value {
        (**self).to_json()
    }
}

impl<T: ToJson> ToJson for Option<T> {
    fn to_json(&self) -> Value {
        self.as_ref().map_or(Value::Null, ToJson::to_json)
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

impl<T: ToJson> ToJson for Vec<T> {
    fn to_json(&self) -> Value {
        self.as_slice().to_json()
    }
}

impl<T: ToJson> ToJson for BTreeSet<T> {
    fn to_json(&self) -> Value {
        Value::Array(self.iter().map(ToJson::to_json).collect())
    }
}

impl<K: AsRef<str>, V: ToJson> ToJson for BTreeMap<K, V> {
    fn to_json(&self) -> Value {
        Value::Object(
            self.iter()
                .map(|(name, value)| (name.as_ref().to_owned(), value.to_json()))
                .collect(),
        )
    }
}

/// Implement [`ToJson`] for types whose public inherent `to_json` is the JSON
/// form, so they nest inside [`json!`] literals and generic containers.
macro_rules! to_json_by_inherent {
    ($($t:ty),* $(,)?) => {$(
        impl $crate::json_model::ToJson for $t {
            fn to_json(&self) -> $crate::json_model::Value {
                <$t>::to_json(self)
            }
        }
    )*};
}

pub(crate) use to_json_by_inherent;

/// Build a [`Value`] from a JSON literal, `crate::json_model::json!`'s syntax:
/// `null`, `true`, `false`, arrays and objects nest as written, and any other
/// expression is converted with [`ToJson`]. An object's members come out in
/// name order (the module's member-order contract).
macro_rules! json {
    // ── Array elements, accumulated as `expr,` ──────────────────────────
    (@array [$($elems:expr,)*]) => {
        ::std::vec![$($elems,)*]
    };
    (@array [$($elems:expr),*]) => {
        ::std::vec![$($elems),*]
    };
    (@array [$($elems:expr,)*] null $($rest:tt)*) => {
        $crate::json_model::json!(@array [$($elems,)* $crate::json_model::json!(null)] $($rest)*)
    };
    (@array [$($elems:expr,)*] true $($rest:tt)*) => {
        $crate::json_model::json!(@array [$($elems,)* $crate::json_model::json!(true)] $($rest)*)
    };
    (@array [$($elems:expr,)*] false $($rest:tt)*) => {
        $crate::json_model::json!(@array [$($elems,)* $crate::json_model::json!(false)] $($rest)*)
    };
    (@array [$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
        $crate::json_model::json!(@array [$($elems,)* $crate::json_model::json!([$($array)*])] $($rest)*)
    };
    (@array [$($elems:expr,)*] {$($map:tt)*} $($rest:tt)*) => {
        $crate::json_model::json!(@array [$($elems,)* $crate::json_model::json!({$($map)*})] $($rest)*)
    };
    (@array [$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
        $crate::json_model::json!(@array [$($elems,)* $crate::json_model::json!($next),] $($rest)*)
    };
    (@array [$($elems:expr,)*] $last:expr) => {
        $crate::json_model::json!(@array [$($elems,)* $crate::json_model::json!($last)])
    };
    (@array [$($elems:expr),*] , $($rest:tt)*) => {
        $crate::json_model::json!(@array [$($elems,)*] $($rest)*)
    };

    // ── Object members, inserted into the map `$object` ─────────────────
    (@object $object:ident () () ()) => {};
    (@object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        let _ = $object.insert(::std::string::String::from($($key)+), $value);
        $crate::json_model::json!(@object $object () ($($rest)*) ($($rest)*));
    };
    (@object $object:ident [$($key:tt)+] ($value:expr)) => {
        let _ = $object.insert(::std::string::String::from($($key)+), $value);
    };
    (@object $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::json_model::json!(@object $object [$($key)+] ($crate::json_model::json!(null)) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: true $($rest:tt)*) $copy:tt) => {
        $crate::json_model::json!(@object $object [$($key)+] ($crate::json_model::json!(true)) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: false $($rest:tt)*) $copy:tt) => {
        $crate::json_model::json!(@object $object [$($key)+] ($crate::json_model::json!(false)) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::json_model::json!(@object $object [$($key)+] ($crate::json_model::json!([$($array)*])) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::json_model::json!(@object $object [$($key)+] ($crate::json_model::json!({$($map)*})) $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::json_model::json!(@object $object [$($key)+] ($crate::json_model::json!($value)) , $($rest)*);
    };
    (@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::json_model::json!(@object $object [$($key)+] ($crate::json_model::json!($value)));
    };
    (@object $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        $crate::json_model::json!(@object $object ($key) (: $($rest)*) (: $($rest)*));
    };
    (@object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        $crate::json_model::json!(@object $object ($($key)* $tt) ($($rest)*) ($($rest)*));
    };

    // ── Entry points ────────────────────────────────────────────────────
    (null) => {
        $crate::json_model::Value::Null
    };
    (true) => {
        $crate::json_model::Value::Bool(true)
    };
    (false) => {
        $crate::json_model::Value::Bool(false)
    };
    ([]) => {
        $crate::json_model::Value::Array(::std::vec::Vec::new())
    };
    ([ $($tt:tt)+ ]) => {
        $crate::json_model::Value::Array($crate::json_model::json!(@array [] $($tt)+))
    };
    ({}) => {
        $crate::json_model::Value::Object($crate::json_model::Object::new())
    };
    ({ $($tt:tt)+ }) => {
        $crate::json_model::object({
            let mut object = $crate::json_model::Map::<::std::string::String, $crate::json_model::Value>::new();
            $crate::json_model::json!(@object object () ($($tt)+) ($($tt)+));
            object
        })
    };
    ($other:expr) => {
        $crate::json_model::ToJson::to_json(&$other)
    };
}

pub(crate) use json;

#[cfg(test)]
#[allow(
    unused_qualifications,
    reason = "json! spells full paths so that it expands anywhere in the crate"
)]
mod tests {
    use super::*;

    #[test]
    fn object_literals_come_out_in_name_order_with_the_last_repeat() {
        let name = "b";
        let value =
            json!({ "c": [1, null, {"z": true, "y": false}], (name): 2.5, "a": "x", "c": 3 });
        assert_eq!(
            purrdf_lex::json::write_compact(&value),
            r#"{"a":"x","b":2.5,"c":3}"#
        );
    }

    #[test]
    fn interpolated_values_convert_by_reference() {
        let names = vec!["p".to_owned(), "q".to_owned()];
        let count = 3_usize;
        let absent: Option<&str> = None;
        let value = json!({ "names": names, "count": count, "absent": absent, "nested": &names });
        assert_eq!(
            purrdf_lex::json::write_compact(&value),
            r#"{"absent":null,"count":3,"names":["p","q"],"nested":["p","q"]}"#
        );
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn insert_sorted_places_a_new_member_and_replaces_an_existing_one() {
        let mut object = Object::new().with("a", 1).with("c", 3);
        assert_eq!(insert_sorted(&mut object, "b", 2), None);
        assert_eq!(insert_sorted(&mut object, "c", 4), Some(Value::from(3)));
        assert_eq!(
            purrdf_lex::json::write_compact(&Value::Object(object)),
            r#"{"a":1,"b":2,"c":4}"#
        );
    }
}
