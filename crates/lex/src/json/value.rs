// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The JSON value tree, whose every whole-tree walk runs over a heap work list.

use core::fmt;
use core::mem;
use core::ops::{Index, IndexMut};
use std::borrow::Cow;

use super::{Format, Kind, Number, write_into};
use crate::json_pointer;

/// A JSON value.
///
/// A value nests as deep as its document writes it, so `Drop`, `Clone`,
/// `PartialEq`, `Debug` and `Display` are loops over heap work lists rather
/// than the compiler's recursive glue: no walk spends a stack frame per level.
/// `Debug` and `Display` print the value as compact JSON (`{:#}` pretty).
#[derive(Default)]
pub enum Value {
    /// `null`.
    #[default]
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number, as its lexeme.
    Number(Number),
    /// A string, escapes decoded.
    String(String),
    /// An array, in order.
    Array(Vec<Self>),
    /// An object: its members in order, repeated names retained.
    Object(Object),
}

/// A JSON object: its members in order, repeated names retained.
///
/// [`Object::get`] answers the FIRST member with a name; [`Object::insert`]
/// replaces the first member with a name in place or appends; [`Object::push`]
/// always appends, so a repeat is kept as the reader keeps it.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Object {
    members: Vec<(String, Value)>,
}

static NULL: Value = Value::Null;

impl Object {
    /// An empty object.
    pub const fn new() -> Self {
        Self {
            members: Vec::new(),
        }
    }

    /// An empty object with room for `capacity` members.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            members: Vec::with_capacity(capacity),
        }
    }

    /// How many members, repeats included.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Whether there are no members.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// The value of the first member named `name`.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.members
            .iter()
            .find(|(member, _)| member == name)
            .map(|(_, value)| value)
    }

    /// The value of the first member named `name`, mutably.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.members
            .iter_mut()
            .find(|(member, _)| member == name)
            .map(|(_, value)| value)
    }

    /// Whether a member is named `name`.
    pub fn contains_key(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// How many members are named `name`.
    pub fn count(&self, name: &str) -> usize {
        self.members
            .iter()
            .filter(|(member, _)| member == name)
            .count()
    }

    /// The first name that a later member repeats, if any.
    pub fn first_duplicate(&self) -> Option<&str> {
        let mut seen: std::collections::HashSet<&str, purrdf_hash::fixed::FixedState> =
            std::collections::HashSet::with_capacity_and_hasher(
                self.members.len(),
                purrdf_hash::fixed::FixedState::new(),
            );
        self.members
            .iter()
            .map(|(name, _)| name.as_str())
            .find(|name| !seen.insert(*name))
    }

    /// Set the first member named `name` to `value` in place, returning the
    /// value it replaces, or append the member when there is none.
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<Value>) -> Option<Value> {
        let name = name.into();
        let value = value.into();
        match self.get_mut(&name) {
            Some(slot) => Some(mem::replace(slot, value)),
            None => {
                self.members.push((name, value));
                None
            }
        }
    }

    /// Append a member, even when its name repeats one already present.
    pub fn push(&mut self, name: impl Into<String>, value: impl Into<Value>) {
        self.members.push((name.into(), value.into()));
    }

    /// [`Object::insert`], by value: a builder step.
    ///
    /// ```rust
    /// use purrdf_lex::json::{Object, write_compact};
    ///
    /// let object = Object::new().with("b", 1_u8).with("a", true).with("b", 2_u8);
    /// assert_eq!(write_compact(&object.into()), r#"{"b":2,"a":true}"#);
    /// ```
    #[must_use]
    pub fn with(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.insert(name, value);
        self
    }

    /// Remove the first member named `name`, keeping the others' order.
    pub fn remove(&mut self, name: &str) -> Option<Value> {
        let index = self.members.iter().position(|(member, _)| member == name)?;
        Some(self.members.remove(index).1)
    }

    /// The members, in order.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (&String, &Value)> + ExactSizeIterator {
        self.members.iter().map(|(name, value)| (name, value))
    }

    /// The members, in order, values mutable.
    pub fn iter_mut(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = (&String, &mut Value)> + ExactSizeIterator {
        self.members.iter_mut().map(|(name, value)| (&*name, value))
    }

    /// The member names, in order.
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &String> + ExactSizeIterator {
        self.members.iter().map(|(name, _)| name)
    }

    /// The member values, in order.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &Value> + ExactSizeIterator {
        self.members.iter().map(|(_, value)| value)
    }

    /// The member values, in order, mutable.
    pub fn values_mut(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = &mut Value> + ExactSizeIterator {
        self.members.iter_mut().map(|(_, value)| value)
    }

    /// The members as a slice.
    pub fn members(&self) -> &[(String, Value)] {
        &self.members
    }

    /// The members, by value.
    pub fn into_members(self) -> Vec<(String, Value)> {
        self.members
    }

    /// Order this object's members by name (bytewise, which is `str` order),
    /// keeping repeats in their relative order. Nested objects are untouched;
    /// [`Value::sort_keys`] sorts a whole tree.
    pub fn sort_keys(&mut self) {
        self.members.sort_by(|(a, _), (b, _)| a.cmp(b));
    }
}

impl From<Vec<(String, Value)>> for Object {
    fn from(members: Vec<(String, Value)>) -> Self {
        Self { members }
    }
}

impl<K: Into<String>, V: Into<Value>> FromIterator<(K, V)> for Object {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(members: I) -> Self {
        Self {
            members: members
                .into_iter()
                .map(|(name, value)| (name.into(), value.into()))
                .collect(),
        }
    }
}

impl<K: Into<String>, V: Into<Value>> Extend<(K, V)> for Object {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, members: I) {
        self.members.extend(
            members
                .into_iter()
                .map(|(name, value)| (name.into(), value.into())),
        );
    }
}

impl IntoIterator for Object {
    type Item = (String, Value);
    type IntoIter = std::vec::IntoIter<(String, Value)>;

    fn into_iter(self) -> Self::IntoIter {
        self.members.into_iter()
    }
}

impl<'a> IntoIterator for &'a Object {
    type Item = &'a (String, Value);
    type IntoIter = core::slice::Iter<'a, (String, Value)>;

    fn into_iter(self) -> Self::IntoIter {
        self.members.iter()
    }
}

impl Index<&str> for Object {
    type Output = Value;

    /// The first member named `name`, or `null` when there is none.
    fn index(&self, name: &str) -> &Value {
        self.get(name).unwrap_or(&NULL)
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Compact JSON: one level of members, each value written over its own
        // work list.
        let mut out = String::from("{");
        for (index, (name, value)) in self.members.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            crate::json_escape::push_string(&mut out, name, Format::COMPACT.escapes);
            out.push(':');
            write_into(&mut out, value, Format::COMPACT);
        }
        out.push('}');
        f.write_str(&out)
    }
}

impl Value {
    /// The value's kind.
    pub const fn kind(&self) -> Kind {
        match self {
            Self::Null => Kind::Null,
            Self::Bool(true) => Kind::True,
            Self::Bool(false) => Kind::False,
            Self::Number(_) => Kind::Number,
            Self::String(_) => Kind::String,
            Self::Array(_) => Kind::Array,
            Self::Object(_) => Kind::Object,
        }
    }

    /// An array of `items`.
    pub fn array<T: Into<Self>>(items: impl IntoIterator<Item = T>) -> Self {
        Self::Array(items.into_iter().map(Into::into).collect())
    }

    /// An object of `members`, in order, repeats retained.
    pub fn object<K: Into<String>, V: Into<Self>>(
        members: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        Self::Object(members.into_iter().collect())
    }

    /// Whether this is `null`.
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// The boolean.
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// The number.
    pub const fn as_number(&self) -> Option<&Number> {
        match self {
            Self::Number(number) => Some(number),
            _ => None,
        }
    }

    /// An integer number's value, when it fits `u64` ([`Number::as_u64`]).
    pub fn as_u64(&self) -> Option<u64> {
        self.as_number().and_then(Number::as_u64)
    }

    /// An integer number's value, when it fits `i64` ([`Number::as_i64`]).
    pub fn as_i64(&self) -> Option<i64> {
        self.as_number().and_then(Number::as_i64)
    }

    /// A number's nearest binary64 ([`Number::as_f64`]).
    pub fn as_f64(&self) -> Option<f64> {
        self.as_number().map(Number::as_f64)
    }

    /// The string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    /// The array's items.
    pub const fn as_array(&self) -> Option<&Vec<Self>> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// The array's items, mutably.
    pub const fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// The object.
    pub const fn as_object(&self) -> Option<&Object> {
        match self {
            Self::Object(object) => Some(object),
            _ => None,
        }
    }

    /// The object, mutably.
    pub const fn as_object_mut(&mut self) -> Option<&mut Object> {
        match self {
            Self::Object(object) => Some(object),
            _ => None,
        }
    }

    /// The first member named `name`, when this is an object.
    pub fn get(&self, name: &str) -> Option<&Self> {
        self.as_object().and_then(|object| object.get(name))
    }

    /// The first member named `name`, mutably, when this is an object.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Self> {
        self.as_object_mut().and_then(|object| object.get_mut(name))
    }

    /// The item at `index`, when this is an array.
    pub fn item(&self, index: usize) -> Option<&Self> {
        self.as_array().and_then(|items| items.get(index))
    }

    /// The value an RFC 6901 JSON Pointer names: `""` is this value, and each
    /// `/`-prefixed token steps into an object member (the first with that
    /// name) or an array index.
    ///
    /// ```rust
    /// use purrdf_lex::json;
    ///
    /// let value = json::read(r#"{"a/b": [0, {"~": 1}]}"#).unwrap();
    /// assert_eq!(value.pointer("/a~1b/1/~0").unwrap().as_u64(), Some(1));
    /// assert!(value.pointer("/a~1b/01").is_none());
    /// ```
    pub fn pointer(&self, pointer: &str) -> Option<&Self> {
        let mut at = self;
        for token in json_pointer::tokens(pointer)? {
            at = match at {
                Self::Object(object) => object.get(&token)?,
                Self::Array(items) => items.get(json_pointer::array_index(&token)?)?,
                _ => return None,
            };
        }
        Some(at)
    }

    /// [`Value::pointer`], mutably.
    pub fn pointer_mut(&mut self, pointer: &str) -> Option<&mut Self> {
        let mut at = self;
        for token in json_pointer::tokens(pointer)? {
            at = match at {
                Self::Object(object) => object.get_mut(&token)?,
                Self::Array(items) => items.get_mut(json_pointer::array_index(&token)?)?,
                _ => return None,
            };
        }
        Some(at)
    }

    /// Take this value, leaving `null`.
    #[must_use]
    pub fn take(&mut self) -> Self {
        mem::take(self)
    }

    /// Order the members of every object in the tree by name (bytewise),
    /// keeping repeats in their relative order, over a heap work list.
    pub fn sort_keys(&mut self) {
        let mut work: Vec<&mut Self> = vec![self];
        while let Some(value) = work.pop() {
            match value {
                Self::Array(items) => work.extend(items.iter_mut()),
                Self::Object(object) => {
                    object.sort_keys();
                    work.extend(object.values_mut());
                }
                _ => {}
            }
        }
    }

    /// Whether this value owns other values.
    fn has_children(&self) -> bool {
        match self {
            Self::Array(items) => !items.is_empty(),
            Self::Object(object) => !object.is_empty(),
            _ => false,
        }
    }
}

// ── Drop, Clone, PartialEq over work lists ─────────────────────────────────

impl Drop for Value {
    fn drop(&mut self) {
        if !self.has_children() {
            return;
        }
        let mut work: Vec<Self> = Vec::new();
        let take = |value: &mut Self, work: &mut Vec<Self>| match value {
            Self::Array(items) => work.extend(items.drain(..).filter(Self::has_children)),
            Self::Object(object) => work.extend(
                object
                    .members
                    .drain(..)
                    .map(|(_, member)| member)
                    .filter(Self::has_children),
            ),
            _ => {}
        };
        take(self, &mut work);
        while let Some(mut value) = work.pop() {
            take(&mut value, &mut work);
            // `value` now owns no value, so its own drop returns at once.
        }
    }
}

/// A copy of a value that owns no value; an array or object here is empty.
fn clone_leaf(value: &Value) -> Value {
    match value {
        Value::Null => Value::Null,
        Value::Bool(flag) => Value::Bool(*flag),
        Value::Number(number) => Value::Number(number.clone()),
        Value::String(text) => Value::String(text.clone()),
        Value::Array(_) => Value::Array(Vec::new()),
        Value::Object(_) => Value::Object(Object::new()),
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        enum Step<'a> {
            Enter(&'a Value),
            Exit(&'a Value),
        }
        if !self.has_children() {
            return clone_leaf(self);
        }
        let mut steps = vec![Step::Enter(self)];
        let mut copies: Vec<Self> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                Step::Enter(node) if node.has_children() => {
                    steps.push(Step::Exit(node));
                    match node {
                        Self::Array(items) => steps.extend(items.iter().rev().map(Step::Enter)),
                        Self::Object(object) => {
                            steps.extend(object.values().rev().map(Step::Enter));
                        }
                        _ => unreachable!("only a container has children"),
                    }
                }
                Step::Enter(leaf) => copies.push(clone_leaf(leaf)),
                Step::Exit(node) => {
                    let copy = match node {
                        Self::Array(items) => {
                            let first = copies.len() - items.len();
                            Self::Array(copies.drain(first..).collect())
                        }
                        Self::Object(object) => {
                            let first = copies.len() - object.len();
                            Self::Object(Object {
                                members: object
                                    .keys()
                                    .cloned()
                                    .zip(copies.drain(first..))
                                    .collect(),
                            })
                        }
                        _ => unreachable!("only a container is exited"),
                    };
                    copies.push(copy);
                }
            }
        }
        copies.pop().expect("the root's copy is assembled last")
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        let mut work: Vec<(&Self, &Self)> = vec![(self, other)];
        while let Some(pair) = work.pop() {
            match pair {
                (Self::Null, Self::Null) => {}
                (Self::Bool(a), Self::Bool(b)) if a == b => {}
                (Self::Number(a), Self::Number(b)) if a == b => {}
                (Self::String(a), Self::String(b)) if a == b => {}
                (Self::Array(a), Self::Array(b)) if a.len() == b.len() => {
                    work.extend(a.iter().zip(b));
                }
                (Self::Object(a), Self::Object(b)) if a.len() == b.len() => {
                    for ((name_a, value_a), (name_b, value_b)) in a.members.iter().zip(&b.members) {
                        if name_a != name_b {
                            return false;
                        }
                        work.push((value_a, value_b));
                    }
                }
                _ => return false,
            }
        }
        true
    }
}

impl Eq for Value {}

impl fmt::Display for Value {
    /// Compact JSON; `{:#}` writes it pretty.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        write_into(
            &mut out,
            self,
            if f.alternate() {
                Format::PRETTY
            } else {
                Format::COMPACT
            },
        );
        f.write_str(&out)
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

// ── Indexing ───────────────────────────────────────────────────────────────

impl Index<&str> for Value {
    type Output = Self;

    /// The first member named `name`, or `null` when this is not an object or
    /// has no such member.
    fn index(&self, name: &str) -> &Self {
        self.get(name).unwrap_or(&NULL)
    }
}

impl Index<usize> for Value {
    type Output = Self;

    /// The item at `index`, or `null` when this is not an array or is shorter.
    fn index(&self, index: usize) -> &Self {
        self.item(index).unwrap_or(&NULL)
    }
}

impl IndexMut<&str> for Value {
    /// The first member named `name`, appended as `null` when absent; a `null`
    /// becomes an empty object first.
    ///
    /// # Panics
    ///
    /// When this is neither `null` nor an object.
    fn index_mut(&mut self, name: &str) -> &mut Self {
        if self.is_null() {
            *self = Self::Object(Object::new());
        }
        let Self::Object(object) = self else {
            panic!(
                "a JSON {} has no members to index by name",
                self.kind().name()
            );
        };
        let index = match object.members.iter().position(|(member, _)| member == name) {
            Some(index) => index,
            None => {
                object.members.push((name.to_owned(), Self::Null));
                object.members.len() - 1
            }
        };
        &mut object.members[index].1
    }
}

impl IndexMut<usize> for Value {
    /// # Panics
    ///
    /// When this is not an array or `index` is out of bounds.
    fn index_mut(&mut self, index: usize) -> &mut Self {
        match self {
            Self::Array(items) => &mut items[index],
            other => panic!("a JSON {} has no items to index", other.kind().name()),
        }
    }
}

// ── Construction ───────────────────────────────────────────────────────────

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Number> for Value {
    fn from(value: Number) -> Self {
        Self::Number(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<&String> for Value {
    fn from(value: &String) -> Self {
        Self::String(value.clone())
    }
}

impl From<Cow<'_, str>> for Value {
    fn from(value: Cow<'_, str>) -> Self {
        Self::String(value.into_owned())
    }
}

impl From<Object> for Value {
    fn from(value: Object) -> Self {
        Self::Object(value)
    }
}

impl From<()> for Value {
    fn from((): ()) -> Self {
        Self::Null
    }
}

impl From<f64> for Value {
    /// The shortest round-trip lexeme ([`Number::from_f64`]); `null` for NaN
    /// and the infinities, which JSON cannot spell.
    fn from(value: f64) -> Self {
        Number::from_f64(value).map_or(Self::Null, Self::Number)
    }
}

impl From<f32> for Value {
    /// [`Number::from_f32`]; `null` for NaN and the infinities.
    fn from(value: f32) -> Self {
        Number::from_f32(value).map_or(Self::Null, Self::Number)
    }
}

impl<T: Into<Self>> From<Vec<T>> for Value {
    fn from(items: Vec<T>) -> Self {
        Self::Array(items.into_iter().map(Into::into).collect())
    }
}

impl<T: Clone + Into<Self>> From<&[T]> for Value {
    fn from(items: &[T]) -> Self {
        Self::Array(items.iter().cloned().map(Into::into).collect())
    }
}

impl<T: Into<Self>> From<Option<T>> for Value {
    /// `null` for `None`.
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, Into::into)
    }
}

impl<T: Into<Self>> FromIterator<T> for Value {
    /// An array.
    fn from_iter<I: IntoIterator<Item = T>>(items: I) -> Self {
        Self::array(items)
    }
}

macro_rules! value_from_integer {
    ($($t:ty),* $(,)?) => {$(
        impl From<$t> for Value {
            fn from(value: $t) -> Self {
                Self::Number(Number::from(value))
            }
        }

        impl PartialEq<$t> for Value {
            fn eq(&self, other: &$t) -> bool {
                self.as_number()
                    .and_then(Number::as_i128)
                    .is_some_and(|value| i128::try_from(*other).is_ok_and(|other| other == value))
            }
        }
    )*};
}

value_from_integer!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

impl From<u128> for Value {
    fn from(value: u128) -> Self {
        Self::Number(Number::from(value))
    }
}

impl From<i128> for Value {
    fn from(value: i128) -> Self {
        Self::Number(Number::from(value))
    }
}

impl PartialEq<str> for Value {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == Some(other)
    }
}

impl PartialEq<&str> for Value {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == Some(*other)
    }
}

impl PartialEq<String> for Value {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == Some(other.as_str())
    }
}

impl PartialEq<bool> for Value {
    fn eq(&self, other: &bool) -> bool {
        self.as_bool() == Some(*other)
    }
}
