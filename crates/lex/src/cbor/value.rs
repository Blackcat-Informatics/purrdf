// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The CBOR data model (RFC 8949 §2), whose every whole-tree walk runs over a
//! heap work list.

use core::fmt::{self, Write as _};
use core::mem;

use crate::json_escape::{JsonEscapes, push_string};

/// An integer in the range CBOR major types 0 and 1 carry: `-2^64 ..= 2^64 - 1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Integer(i128);

/// An integer outside `-2^64 ..= 2^64 - 1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntegerRangeError;

impl fmt::Display for IntegerRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a CBOR integer is between -2^64 and 2^64 - 1")
    }
}

impl std::error::Error for IntegerRangeError {}

impl Integer {
    /// `-2^64`, the most negative integer major type 1 carries.
    pub const MIN: Self = Self(-(1_i128 << 64));
    /// `2^64 - 1`, the largest integer major type 0 carries.
    pub const MAX: Self = Self((1_i128 << 64) - 1);

    /// The value.
    pub const fn get(self) -> i128 {
        self.0
    }

    /// The major type (0 or 1) and argument that spell this integer.
    pub(crate) const fn head(self) -> (u8, u64) {
        if self.0 >= 0 {
            (0, self.0 as u64)
        } else {
            (1, (-1 - self.0) as u64)
        }
    }
}

macro_rules! integer_from {
    ($($t:ty),* $(,)?) => {$(
        impl From<$t> for Integer {
            fn from(value: $t) -> Self {
                // Every machine integer of 64 bits or fewer is in range.
                Self(value as i128)
            }
        }

        impl TryFrom<Integer> for $t {
            type Error = core::num::TryFromIntError;

            fn try_from(value: Integer) -> Result<Self, Self::Error> {
                Self::try_from(value.0)
            }
        }

        impl From<$t> for Value {
            fn from(value: $t) -> Self {
                Self::Integer(Integer::from(value))
            }
        }
    )*};
}

integer_from!(usize, isize);

macro_rules! integer_from_lossless {
    ($($t:ty),* $(,)?) => {$(
        impl From<$t> for Integer {
            fn from(value: $t) -> Self {
                Self(i128::from(value))
            }
        }

        impl TryFrom<Integer> for $t {
            type Error = core::num::TryFromIntError;

            fn try_from(value: Integer) -> Result<Self, Self::Error> {
                Self::try_from(value.0)
            }
        }

        impl From<$t> for Value {
            fn from(value: $t) -> Self {
                Self::Integer(Integer::from(value))
            }
        }
    )*};
}

integer_from_lossless!(u8, u16, u32, u64, i8, i16, i32, i64);

impl TryFrom<i128> for Integer {
    type Error = IntegerRangeError;

    fn try_from(value: i128) -> Result<Self, IntegerRangeError> {
        if (Self::MIN.0..=Self::MAX.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(IntegerRangeError)
        }
    }
}

impl TryFrom<u128> for Integer {
    type Error = IntegerRangeError;

    fn try_from(value: u128) -> Result<Self, IntegerRangeError> {
        u64::try_from(value)
            .map(Self::from)
            .map_err(|_| IntegerRangeError)
    }
}

impl From<Integer> for i128 {
    fn from(value: Integer) -> Self {
        value.0
    }
}

impl TryFrom<Integer> for u128 {
    type Error = core::num::TryFromIntError;

    fn try_from(value: Integer) -> Result<Self, Self::Error> {
        Self::try_from(value.0)
    }
}

impl fmt::Display for Integer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// A CBOR data item.
///
/// A value nests as deep as its producer wrote it, so `Drop`, `Clone`,
/// `PartialEq` and `Debug` are loops over heap work lists rather than the
/// compiler's recursive glue. `Debug` prints RFC 8949 §8 diagnostic notation.
/// Equality is structural, with floats compared as `f64` values (so a NaN is
/// unequal to itself, as in IEEE 754).
pub enum Value {
    /// Major types 0 and 1.
    Integer(Integer),
    /// Major type 2.
    Bytes(Vec<u8>),
    /// A binary16, binary32 or binary64 float, widened.
    Float(f64),
    /// Major type 3.
    Text(String),
    /// Simple values 20 and 21.
    Bool(bool),
    /// Simple value 22.
    Null,
    /// Any other simple value: 0–19, 23 (`undefined`) or 32–255. The
    /// encoders write `Simple(n)` as major type 7 with argument `n`, so a
    /// caller holds only those values: 20–22 are [`Value::Bool`] and
    /// [`Value::Null`], and 24–31 have no well-formed encoding (RFC 8949 §3.3).
    Simple(u8),
    /// Major type 6: a tag number and the tagged item.
    Tag(u64, Box<Self>),
    /// Major type 4.
    Array(Vec<Self>),
    /// Major type 5: key/value pairs in order, repeats retained.
    Map(Vec<(Self, Self)>),
}

impl Value {
    /// The integer.
    pub const fn as_integer(&self) -> Option<Integer> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// The byte string.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// The text string.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The float.
    pub const fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(value) => Some(*value),
            _ => None,
        }
    }

    /// The boolean.
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// Whether this is `null`.
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// The tag number and tagged item.
    pub fn as_tag(&self) -> Option<(u64, &Self)> {
        match self {
            Self::Tag(tag, item) => Some((*tag, item)),
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

    /// The map's entries.
    pub const fn as_map(&self) -> Option<&Vec<(Self, Self)>> {
        match self {
            Self::Map(entries) => Some(entries),
            _ => None,
        }
    }

    /// The map's entries, mutably.
    pub const fn as_map_mut(&mut self) -> Option<&mut Vec<(Self, Self)>> {
        match self {
            Self::Map(entries) => Some(entries),
            _ => None,
        }
    }

    /// The byte string, by value; the value itself otherwise.
    ///
    /// # Errors
    ///
    /// The value, when it is not a byte string.
    pub fn into_bytes(mut self) -> Result<Vec<u8>, Self> {
        if let Self::Bytes(inner) = &mut self {
            return Ok(mem::take(inner));
        }
        Err(self)
    }

    /// The text string, by value; the value itself otherwise.
    ///
    /// # Errors
    ///
    /// The value, when it is not a text string.
    pub fn into_text(mut self) -> Result<String, Self> {
        if let Self::Text(inner) = &mut self {
            return Ok(mem::take(inner));
        }
        Err(self)
    }

    /// The array's items, by value; the value itself otherwise.
    ///
    /// # Errors
    ///
    /// The value, when it is not an array.
    pub fn into_array(mut self) -> Result<Vec<Self>, Self> {
        if let Self::Array(inner) = &mut self {
            return Ok(mem::take(inner));
        }
        Err(self)
    }

    /// The map's entries, by value; the value itself otherwise.
    ///
    /// # Errors
    ///
    /// The value, when it is not a map.
    pub fn into_map(mut self) -> Result<Vec<(Self, Self)>, Self> {
        if let Self::Map(inner) = &mut self {
            return Ok(mem::take(inner));
        }
        Err(self)
    }

    /// Take this value, leaving `null`.
    #[must_use]
    pub fn take(&mut self) -> Self {
        mem::replace(self, Self::Null)
    }

    /// The value of the first entry whose key is the text `key`, when this is
    /// a map.
    pub fn get(&self, key: &str) -> Option<&Self> {
        map_get(self.as_map()?, key)
    }

    /// Order the entries of every map in the tree by the bytewise order of
    /// their keys' deterministic encodings (RFC 8949 §4.2.1), repeats kept in
    /// their relative order, over a heap work list. [`super::encode`] of the
    /// result is [`super::canonical`] of the original.
    pub fn canonicalize(&mut self) {
        let mut work: Vec<&mut Self> = vec![self];
        while let Some(value) = work.pop() {
            match value {
                Self::Tag(_, item) => work.push(item),
                Self::Array(items) => work.extend(items.iter_mut()),
                Self::Map(entries) => {
                    let mut keyed: Vec<(Vec<u8>, (Self, Self))> = entries
                        .drain(..)
                        .map(|entry| (super::canonical(&entry.0), entry))
                        .collect();
                    keyed.sort_by(|a, b| a.0.cmp(&b.0));
                    entries.extend(keyed.into_iter().map(|(_, entry)| entry));
                    for (key, value) in entries.iter_mut() {
                        work.push(key);
                        work.push(value);
                    }
                }
                _ => {}
            }
        }
    }

    /// How many values this value owns directly (a map entry counts twice).
    fn child_count(&self) -> usize {
        match self {
            Self::Tag(..) => 1,
            Self::Array(items) => items.len(),
            Self::Map(entries) => entries.len() * 2,
            _ => 0,
        }
    }
}

/// The value of the first entry whose key is the text `key`.
pub fn map_get<'a>(entries: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    entries
        .iter()
        .find(|(candidate, _)| candidate.as_text() == Some(key))
        .map(|(_, value)| value)
}

impl Drop for Value {
    fn drop(&mut self) {
        fn take_children(value: &mut Value, work: &mut Vec<Value>) {
            match value {
                Value::Tag(_, item) if item.child_count() > 0 => work.push(item.take()),
                Value::Array(items) => work.extend(items.drain(..).filter(|v| v.child_count() > 0)),
                Value::Map(entries) => {
                    for (key, value) in entries.drain(..) {
                        if key.child_count() > 0 {
                            work.push(key);
                        }
                        if value.child_count() > 0 {
                            work.push(value);
                        }
                    }
                }
                _ => {}
            }
        }
        if self.child_count() == 0 {
            return;
        }
        let mut work = Vec::new();
        take_children(self, &mut work);
        while let Some(mut value) = work.pop() {
            take_children(&mut value, &mut work);
        }
    }
}

/// A copy of a value that owns no value; a container here is empty.
fn clone_leaf(value: &Value) -> Value {
    match value {
        Value::Integer(integer) => Value::Integer(*integer),
        Value::Bytes(bytes) => Value::Bytes(bytes.clone()),
        Value::Float(float) => Value::Float(*float),
        Value::Text(text) => Value::Text(text.clone()),
        Value::Bool(flag) => Value::Bool(*flag),
        Value::Null => Value::Null,
        Value::Simple(simple) => Value::Simple(*simple),
        Value::Tag(tag, _) => Value::Tag(*tag, Box::new(Value::Null)),
        Value::Array(_) => Value::Array(Vec::new()),
        Value::Map(_) => Value::Map(Vec::new()),
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        enum Step<'a> {
            Enter(&'a Value),
            Exit(&'a Value),
        }
        if self.child_count() == 0 {
            return clone_leaf(self);
        }
        let mut steps = vec![Step::Enter(self)];
        let mut copies: Vec<Self> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                Step::Enter(node) if node.child_count() > 0 => {
                    steps.push(Step::Exit(node));
                    match node {
                        Self::Tag(_, item) => steps.push(Step::Enter(item)),
                        Self::Array(items) => steps.extend(items.iter().rev().map(Step::Enter)),
                        Self::Map(entries) => {
                            for (key, value) in entries.iter().rev() {
                                steps.push(Step::Enter(value));
                                steps.push(Step::Enter(key));
                            }
                        }
                        _ => unreachable!("only a container has children"),
                    }
                }
                Step::Enter(leaf) => copies.push(clone_leaf(leaf)),
                Step::Exit(node) => {
                    let first = copies.len() - node.child_count();
                    let mut children = copies.drain(first..);
                    let copy = match node {
                        Self::Tag(tag, _) => {
                            Self::Tag(*tag, Box::new(children.next().expect("a tag has one item")))
                        }
                        Self::Array(_) => Self::Array(children.by_ref().collect()),
                        Self::Map(entries) => {
                            let mut pairs = Vec::with_capacity(entries.len());
                            while let Some(key) = children.next() {
                                let value = children.next().expect("a map entry has a value");
                                pairs.push((key, value));
                            }
                            Self::Map(pairs)
                        }
                        _ => unreachable!("only a container is exited"),
                    };
                    drop(children);
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
                (Self::Integer(a), Self::Integer(b)) if a == b => {}
                (Self::Bytes(a), Self::Bytes(b)) if a == b => {}
                (Self::Float(a), Self::Float(b)) if a == b => {}
                (Self::Text(a), Self::Text(b)) if a == b => {}
                (Self::Bool(a), Self::Bool(b)) if a == b => {}
                (Self::Null, Self::Null) => {}
                (Self::Simple(a), Self::Simple(b)) if a == b => {}
                (Self::Tag(a, x), Self::Tag(b, y)) if a == b => work.push((x, y)),
                (Self::Array(a), Self::Array(b)) if a.len() == b.len() => {
                    work.extend(a.iter().zip(b));
                }
                (Self::Map(a), Self::Map(b)) if a.len() == b.len() => {
                    for ((ka, va), (kb, vb)) in a.iter().zip(b) {
                        work.push((ka, kb));
                        work.push((va, vb));
                    }
                }
                _ => return false,
            }
        }
        true
    }
}

impl fmt::Debug for Value {
    /// RFC 8949 §8 diagnostic notation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        enum Job<'a> {
            Value(&'a Value),
            Text(&'static str),
        }
        let mut out = String::new();
        let mut jobs = vec![Job::Value(self)];
        while let Some(job) = jobs.pop() {
            match job {
                Job::Text(text) => out.push_str(text),
                Job::Value(value) => match value {
                    Self::Integer(integer) => {
                        let _ = write!(out, "{integer}");
                    }
                    Self::Bytes(bytes) => {
                        out.push_str("h'");
                        let _ = write!(out, "{}", purrdf_hash::hex::Lower(bytes));
                        out.push('\'');
                    }
                    Self::Float(float) => {
                        if float.is_nan() {
                            out.push_str("NaN");
                        } else if float.is_infinite() {
                            out.push_str(if *float > 0.0 {
                                "Infinity"
                            } else {
                                "-Infinity"
                            });
                        } else {
                            let _ = write!(out, "{float:?}");
                        }
                    }
                    Self::Text(text) => push_string(&mut out, text, JsonEscapes::ShortForms),
                    Self::Bool(true) => out.push_str("true"),
                    Self::Bool(false) => out.push_str("false"),
                    Self::Null => out.push_str("null"),
                    Self::Simple(23) => out.push_str("undefined"),
                    Self::Simple(simple) => {
                        let _ = write!(out, "simple({simple})");
                    }
                    Self::Tag(tag, item) => {
                        let _ = write!(out, "{tag}(");
                        jobs.push(Job::Text(")"));
                        jobs.push(Job::Value(item));
                    }
                    Self::Array(items) => {
                        out.push('[');
                        jobs.push(Job::Text("]"));
                        for (index, item) in items.iter().enumerate().rev() {
                            jobs.push(Job::Value(item));
                            if index > 0 {
                                jobs.push(Job::Text(", "));
                            }
                        }
                    }
                    Self::Map(entries) => {
                        out.push('{');
                        jobs.push(Job::Text("}"));
                        for (index, (key, value)) in entries.iter().enumerate().rev() {
                            jobs.push(Job::Value(value));
                            jobs.push(Job::Text(": "));
                            jobs.push(Job::Value(key));
                            if index > 0 {
                                jobs.push(Job::Text(", "));
                            }
                        }
                    }
                },
            }
        }
        f.write_str(&out)
    }
}

impl From<Integer> for Value {
    fn from(value: Integer) -> Self {
        Self::Integer(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Self::Float(f64::from(value))
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(value)
    }
}

impl From<&[u8]> for Value {
    fn from(value: &[u8]) -> Self {
        Self::Bytes(value.to_vec())
    }
}

impl From<Vec<Self>> for Value {
    fn from(value: Vec<Self>) -> Self {
        Self::Array(value)
    }
}

impl From<Vec<(Self, Self)>> for Value {
    fn from(value: Vec<(Self, Self)>) -> Self {
        Self::Map(value)
    }
}
