// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! [`JsonValue`]'s `Drop`, `Clone`, `PartialEq` and `Debug`, each over an explicit
//! heap work list rather than recursion.
//!
//! A value owns other values through [`JsonValue::Array`] and [`JsonValue::Object`],
//! and a `geoJSONLiteral` is untrusted input, so a document nests as deep as its
//! author writes it and the reader has no depth at which to refuse it. The glue
//! `#[derive]` writes for a recursive type recurses once per level, and in Rust a
//! stack overflow is an `abort` no caller can catch, so every walk over a whole value
//! is a loop over a heap stack, on the pattern of `geom::tree`:
//!
//! * **`Drop`** moves a container's values onto a work list and dismantles the list
//!   in a loop, taking every popped value's own values before it goes.
//! * **`Clone`** builds the copy bottom-up: a two-phase walk enters each value and,
//!   on the way back out, assembles a container's copy from its members' finished
//!   copies. A value that owns no value is copied outright.
//! * **`PartialEq`** compares two values pair by pair off one work list.
//! * **`Debug`** prints the *script* `#[derive(Debug)]` prints, through
//!   [`crate::debug_script`], so the bytes are the derive's exactly in both forms.

use core::fmt;
use core::mem;

use super::JsonValue;
use crate::debug_script::{self, Tok};

/// Whether `value` owns other values.
fn has_children(value: &JsonValue) -> bool {
    match value {
        JsonValue::Array(items) => !items.is_empty(),
        JsonValue::Object(members) => !members.is_empty(),
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) | JsonValue::String(_) => false,
    }
}

// ── Drop ─────────────────────────────────────────────────────────────────────────

impl Drop for JsonValue {
    fn drop(&mut self) {
        let mut work: Vec<Self> = match self {
            Self::Array(items) if !items.is_empty() => mem::take(items),
            Self::Object(members) if !members.is_empty() => mem::take(members)
                .into_iter()
                .map(|(_, value)| value)
                .collect(),
            _ => return,
        };
        while let Some(mut value) = work.pop() {
            match &mut value {
                Self::Array(items) => work.append(items),
                Self::Object(members) => work.extend(members.drain(..).map(|(_, member)| member)),
                Self::Null | Self::Bool(_) | Self::Number(_) | Self::String(_) => {}
            }
            // `value` drops here owning no value, so the drop it runs is this one's
            // early return.
        }
    }
}

// ── Clone ────────────────────────────────────────────────────────────────────────

/// One step of the bottom-up copy.
enum Step<'a> {
    /// Visit a value: copy one that owns no value outright, or schedule a
    /// container's values before it.
    Enter(&'a JsonValue),
    /// Every value of the container has been copied; assemble the container's copy.
    Exit(&'a JsonValue),
}

/// A copy of a value that owns no value; an array or object reaching here is empty.
fn clone_leaf(value: &JsonValue) -> JsonValue {
    match value {
        JsonValue::Null => JsonValue::Null,
        JsonValue::Bool(flag) => JsonValue::Bool(*flag),
        JsonValue::Number(lexeme) => JsonValue::Number(lexeme.clone()),
        JsonValue::String(text) => JsonValue::String(text.clone()),
        JsonValue::Array(_) => JsonValue::Array(Vec::new()),
        JsonValue::Object(_) => JsonValue::Object(Vec::new()),
    }
}

/// A copy of the tree under `root`, built bottom-up over a work list.
fn clone_tree(root: &JsonValue) -> JsonValue {
    let mut steps: Vec<Step<'_>> = vec![Step::Enter(root)];
    let mut copies: Vec<JsonValue> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Enter(node) => match node {
                JsonValue::Array(items) if !items.is_empty() => {
                    // Values are pushed last-first so they pop, and so their copies
                    // land, in document order.
                    steps.push(Step::Exit(node));
                    steps.extend(items.iter().rev().map(Step::Enter));
                }
                JsonValue::Object(members) if !members.is_empty() => {
                    steps.push(Step::Exit(node));
                    steps.extend(members.iter().rev().map(|(_, value)| Step::Enter(value)));
                }
                leaf => copies.push(clone_leaf(leaf)),
            },
            Step::Exit(node) => {
                let copy = match node {
                    JsonValue::Array(items) => {
                        let first = copies.len() - items.len();
                        JsonValue::Array(copies.drain(first..).collect())
                    }
                    JsonValue::Object(members) => {
                        let first = copies.len() - members.len();
                        JsonValue::Object(
                            members
                                .iter()
                                .map(|(name, _)| name.clone())
                                .zip(copies.drain(first..))
                                .collect(),
                        )
                    }
                    _ => unreachable!("only an array or an object is exited"),
                };
                copies.push(copy);
            }
        }
    }
    copies
        .pop()
        .expect("the root's copy is the last one assembled")
}

impl Clone for JsonValue {
    fn clone(&self) -> Self {
        if has_children(self) {
            clone_tree(self)
        } else {
            clone_leaf(self)
        }
    }
}

// ── PartialEq ────────────────────────────────────────────────────────────────────

impl PartialEq for JsonValue {
    fn eq(&self, other: &Self) -> bool {
        let mut work: Vec<(&Self, &Self)> = vec![(self, other)];
        while let Some((a, b)) = work.pop() {
            match (a, b) {
                (Self::Null, Self::Null) => {}
                (Self::Bool(x), Self::Bool(y)) => {
                    if x != y {
                        return false;
                    }
                }
                (Self::Number(x), Self::Number(y)) | (Self::String(x), Self::String(y)) => {
                    if x != y {
                        return false;
                    }
                }
                (Self::Array(x), Self::Array(y)) => {
                    if x.len() != y.len() {
                        return false;
                    }
                    work.extend(x.iter().zip(y.iter()));
                }
                (Self::Object(x), Self::Object(y)) => {
                    if x.len() != y.len() {
                        return false;
                    }
                    for ((name_x, value_x), (name_y, value_y)) in x.iter().zip(y.iter()) {
                        if name_x != name_y {
                            return false;
                        }
                        work.push((value_x, value_y));
                    }
                }
                _ => return false,
            }
        }
        true
    }
}

// ── Debug ────────────────────────────────────────────────────────────────────────

/// Append the script `#[derive(Debug)]` prints for `node` to `out`, each nested value
/// as a [`Tok::Node`]. An object member is the plain tuple `(name, value)`, which the
/// standard library prints as a tuple with the empty name.
fn script<'a>(node: &'a JsonValue, out: &mut Vec<Tok<'a, &'a JsonValue>>) {
    match node {
        JsonValue::Null => out.extend([Tok::Tuple("Null"), Tok::EndTuple]),
        JsonValue::Bool(flag) => out.extend([Tok::Tuple("Bool"), Tok::Leaf(flag), Tok::EndTuple]),
        JsonValue::Number(lexeme) => {
            out.extend([Tok::Tuple("Number"), Tok::Leaf(lexeme), Tok::EndTuple]);
        }
        JsonValue::String(text) => {
            out.extend([Tok::Tuple("String"), Tok::Leaf(text), Tok::EndTuple]);
        }
        JsonValue::Array(items) => {
            out.extend([Tok::Tuple("Array"), Tok::List]);
            out.extend(items.iter().map(Tok::Node));
            out.extend([Tok::EndList, Tok::EndTuple]);
        }
        JsonValue::Object(members) => {
            out.extend([Tok::Tuple("Object"), Tok::List]);
            for (name, value) in members {
                out.extend([
                    Tok::Tuple(""),
                    Tok::Leaf(name),
                    Tok::Node(value),
                    Tok::EndTuple,
                ]);
            }
            out.extend([Tok::EndList, Tok::EndTuple]);
        }
    }
}

impl fmt::Debug for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_script::write(f, self, script)
    }
}

#[cfg(test)]
mod tests {
    use super::super::arbitrary::value;
    use super::super::{JsonValue, parse, write};
    use crate::geom::arbitrary::{self, Lcg};

    /// A type-for-type twin with the compiler's own `Clone`, `PartialEq` and `Debug`,
    /// so the derive itself is the oracle for what the iterative impls must do.
    #[allow(
        dead_code,
        reason = "the twin's fields exist to be printed by its derived `Debug`, which dead-code \
                  analysis does not count as a read"
    )]
    mod derived {
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub(super) enum JsonValue {
            Null,
            Bool(bool),
            Number(String),
            String(String),
            Array(Vec<Self>),
            Object(Vec<(String, Self)>),
        }
    }

    /// The twin of `value`; recursive, on shallow fixtures only.
    fn twin(value: &JsonValue) -> derived::JsonValue {
        match value {
            JsonValue::Null => derived::JsonValue::Null,
            JsonValue::Bool(flag) => derived::JsonValue::Bool(*flag),
            JsonValue::Number(lexeme) => derived::JsonValue::Number(lexeme.clone()),
            JsonValue::String(text) => derived::JsonValue::String(text.clone()),
            JsonValue::Array(items) => derived::JsonValue::Array(items.iter().map(twin).collect()),
            JsonValue::Object(members) => derived::JsonValue::Object(
                members
                    .iter()
                    .map(|(name, member)| (name.clone(), twin(member)))
                    .collect(),
            ),
        }
    }

    fn fixtures() -> Vec<JsonValue> {
        let mut rng = Lcg::new(0x5eed_1001);
        let mut all: Vec<JsonValue> = [
            "null",
            "true",
            "1",
            "\"a\"",
            "[]",
            "{}",
            "[[]]",
            "{\"\":{}}",
            r#"{"type":"Point","coordinates":[1,2]}"#,
            r#"[null,true,false,0,-0,1e10,"",[],{}]"#,
            r#"{"a":{"b":{"c":[1,[2,[3]]]}},"a":1}"#,
        ]
        .into_iter()
        .map(|text| parse(text).expect("the fixture is well-formed"))
        .collect();
        all.extend((0..120).map(|_| value(&mut rng, 4)));
        all
    }

    /// The iterative `Debug` writes byte for byte what the derive writes, in both
    /// forms, and equality and cloning agree with the derive's, over fixed and
    /// generated values of every shape.
    #[test]
    fn debug_equality_and_clone_are_the_derives_own() {
        let all = fixtures();
        let twins: Vec<derived::JsonValue> = all.iter().map(twin).collect();
        for (index, value) in all.iter().enumerate() {
            assert_eq!(format!("{value:?}"), format!("{:?}", twins[index]));
            assert_eq!(format!("{value:#?}"), format!("{:#?}", twins[index]));
            let copy = value.clone();
            assert_eq!(&copy, value, "a copy equals its original");
            assert_eq!(
                twin(&copy),
                twins[index].clone(),
                "and its twin is the twin's copy"
            );
            assert_eq!(write(&copy), write(value), "and writes the same bytes");
            for (other_index, other) in all.iter().enumerate() {
                assert_eq!(
                    value == other,
                    twins[index] == twins[other_index],
                    "equality agrees with the derive: {value:?} against {other:?}"
                );
            }
        }
    }

    /// A hundred thousand nested arrays, and a hundred thousand nested objects, are
    /// cloned, compared, printed and dropped on a 128 KiB stack.
    ///
    /// The `Debug` lengths are formulas pinned against the derive's own spelling at
    /// depths one and two: `Array([])` is nine bytes and each further level wraps the
    /// one below in `Array([` and `])`, nine bytes; `Object([])` is ten bytes and each
    /// further level wraps the one below in `Object([("a", ` and `)])`, seventeen.
    #[test]
    fn a_hundred_thousand_deep_value_clones_compares_prints_and_drops() {
        const ONE_ARRAY: &str = "Array([])";
        const TWO_ARRAYS: &str = "Array([Array([])])";
        const ONE_OBJECT: &str = "Object([])";
        const TWO_OBJECTS: &str = "Object([(\"a\", Object([]))])";
        let depth = arbitrary::DEEP;
        let empty_array = || JsonValue::Array(Vec::new());
        let empty_object = || JsonValue::Object(Vec::new());
        let in_array = |inner: JsonValue| JsonValue::Array(vec![inner]);
        let in_object = |inner: JsonValue| JsonValue::Object(vec![("a".to_owned(), inner)]);
        assert_eq!(format!("{:?}", empty_array()), ONE_ARRAY);
        assert_eq!(format!("{:?}", in_array(empty_array())), TWO_ARRAYS);
        assert_eq!(format!("{:?}", empty_object()), ONE_OBJECT);
        assert_eq!(format!("{:?}", in_object(empty_object())), TWO_OBJECTS);
        let per_array = TWO_ARRAYS.len() - ONE_ARRAY.len();
        let per_object = TWO_OBJECTS.len() - ONE_OBJECT.len();
        assert_eq!((ONE_ARRAY.len(), per_array), (9, 9));
        assert_eq!((ONE_OBJECT.len(), per_object), (10, 17));
        let expected_array_len = ONE_ARRAY.len() + per_array * (depth - 1);
        let expected_object_len = ONE_OBJECT.len() + per_object * (depth - 1);

        arbitrary::on_small_stack(move || {
            let mut arrays = empty_array();
            let mut objects = empty_object();
            for _ in 1..depth {
                arrays = in_array(arrays);
                objects = in_object(objects);
            }
            let array_copy = arrays.clone();
            let object_copy = objects.clone();
            assert_eq!(array_copy, arrays);
            assert_eq!(object_copy, objects);
            assert_ne!(arrays, objects);
            assert_eq!(format!("{arrays:?}").len(), expected_array_len);
            assert_eq!(format!("{objects:?}").len(), expected_object_len);
            // One byte differing at the bottom is found through every level.
            let mut deep_true = JsonValue::Bool(true);
            let mut deep_false = JsonValue::Bool(false);
            for _ in 1..depth {
                deep_true = in_array(deep_true);
                deep_false = in_array(deep_false);
            }
            assert_ne!(deep_true, deep_false);
            assert_eq!(deep_true, deep_true.clone());
            drop(deep_true);
            drop(deep_false);
            drop(array_copy);
            drop(object_copy);
            drop(arrays);
            drop(objects);
        });
    }
}
