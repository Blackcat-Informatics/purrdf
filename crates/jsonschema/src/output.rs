// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The 2020-12 output formats (Core §12): `flag`, `basic` and `detailed`.
//!
//! An evaluation with output records one [`OutputUnit`] per subschema and
//! per keyword it evaluated, nested as the schema is: a subschema's unit
//! holds its keywords' units, and an applicator keyword's unit holds the
//! units of the subschemas it applied. The formats are projections of that
//! tree:
//!
//! * **flag** — `{"valid": …}` alone.
//! * **basic** — one flat list: every failing keyword under a failing path
//!   (`errors`), or every annotation under a passing one (`annotations`).
//! * **detailed** — the tree, keeping only the failing (or, when valid, the
//!   annotating) branches, with every node that has one child replaced by
//!   that child.
//!
//! Annotations from a subschema that failed are never reported (Core §7.7.1.2):
//! both projections descend only into units whose validity matches the
//! result they explain.

use std::fmt;

use purrdf_lex::json::{Object, Value};

/// Which output format [`Output::to_json`] writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// `{"valid": bool}`.
    Flag,
    /// A flat list of output units.
    Basic,
    /// A condensed hierarchy following the schema.
    Detailed,
}

/// One node of an evaluation's output.
///
/// Units nest once per subschema and keyword applied, so an instance
/// thousands of levels deep yields a unit tree as deep. Every whole-tree
/// operation — [`Clone`], [`PartialEq`], [`Debug`](fmt::Debug) and drop — walks
/// it over a heap work list rather than one stack frame per level.
pub struct OutputUnit {
    /// Whether the keyword or subschema passed.
    pub valid: bool,
    /// The JSON Pointer of the keyword or subschema along the evaluation path,
    /// through every `$ref` crossed (`/properties/a/$ref/type`).
    pub keyword_location: String,
    /// The keyword's or subschema's absolute location: its resource's URI and
    /// a JSON Pointer fragment within that resource.
    pub absolute_keyword_location: String,
    /// The JSON Pointer of the instance value evaluated.
    pub instance_location: String,
    /// Why the keyword failed; `None` for a passing unit or one whose failure
    /// is explained entirely by its children.
    pub error: Option<String>,
    /// The annotation the keyword produced, if it passed and produces one.
    pub annotation: Option<Value>,
    /// The units of the keywords or subschemas beneath this one.
    pub children: Vec<Self>,
}

impl Drop for OutputUnit {
    /// Units nest once per subschema and keyword applied, so an instance
    /// thousands of levels deep yields a unit tree as deep; it is taken apart
    /// over a heap work list rather than one stack frame per level.
    fn drop(&mut self) {
        let mut work = std::mem::take(&mut self.children);
        while let Some(mut unit) = work.pop() {
            work.append(&mut unit.children);
            // `unit` now holds no children, so its own drop returns at once.
        }
    }
}

impl OutputUnit {
    /// This unit's own fields, with no children.
    fn shallow_clone(&self) -> Self {
        Self {
            valid: self.valid,
            keyword_location: self.keyword_location.clone(),
            absolute_keyword_location: self.absolute_keyword_location.clone(),
            instance_location: self.instance_location.clone(),
            error: self.error.clone(),
            annotation: self.annotation.clone(),
            children: Vec::with_capacity(self.children.len()),
        }
    }

    /// Whether this unit's own fields, and its number of children, equal
    /// `other`'s.
    fn shallow_eq(&self, other: &Self) -> bool {
        self.valid == other.valid
            && self.keyword_location == other.keyword_location
            && self.absolute_keyword_location == other.absolute_keyword_location
            && self.instance_location == other.instance_location
            && self.error == other.error
            && self.annotation == other.annotation
            && self.children.len() == other.children.len()
    }
}

impl Clone for OutputUnit {
    /// Copies the tree depth first: `building` holds the copies of the units
    /// on the path from the root, each gaining its children as they finish.
    fn clone(&self) -> Self {
        let mut building = vec![self.shallow_clone()];
        let mut path: Vec<(&Self, usize)> = vec![(self, 0)];
        while let Some((source, next)) = path.last_mut() {
            if let Some(child) = source.children.get(*next) {
                *next += 1;
                building.push(child.shallow_clone());
                path.push((child, 0));
                continue;
            }
            path.pop();
            let done = building.pop().expect("one copy per unit on the path");
            match building.last_mut() {
                Some(parent) => parent.children.push(done),
                None => return done,
            }
        }
        unreachable!("the root's copy is returned when its path frame pops")
    }
}

impl PartialEq for OutputUnit {
    fn eq(&self, other: &Self) -> bool {
        let mut pairs = vec![(self, other)];
        while let Some((left, right)) = pairs.pop() {
            if !left.shallow_eq(right) {
                return false;
            }
            pairs.extend(left.children.iter().zip(&right.children));
        }
        true
    }
}

impl fmt::Debug for OutputUnit {
    /// The tree in pre-order, one entry per unit carrying its depth below this
    /// one and its number of children.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Entry<'u>(usize, &'u OutputUnit);
        impl fmt::Debug for Entry<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Self(depth, unit) = self;
                f.debug_struct("OutputUnit")
                    .field("depth", depth)
                    .field("valid", &unit.valid)
                    .field("keyword_location", &unit.keyword_location)
                    .field("absolute_keyword_location", &unit.absolute_keyword_location)
                    .field("instance_location", &unit.instance_location)
                    .field("error", &unit.error)
                    .field("annotation", &unit.annotation)
                    .field("children", &unit.children.len())
                    .finish()
            }
        }
        let mut list = f.debug_list();
        let mut work = vec![(0_usize, self)];
        while let Some((depth, unit)) = work.pop() {
            list.entry(&Entry(depth, unit));
            work.extend(unit.children.iter().rev().map(|child| (depth + 1, child)));
        }
        list.finish()
    }
}

/// The result of evaluating an instance with output: the verdict and the
/// unit tree of the root schema.
#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub(crate) root: OutputUnit,
}

impl Output {
    /// Whether the instance is valid.
    pub fn is_valid(&self) -> bool {
        self.root.valid
    }

    /// The root schema's unit.
    pub fn root(&self) -> &OutputUnit {
        &self.root
    }

    /// The failing keywords of an invalid result, depth first (empty when
    /// valid) — the `basic` format's `errors`.
    pub fn errors(&self) -> impl Iterator<Item = &OutputUnit> {
        let mut found = Vec::new();
        if !self.root.valid {
            collect(&self.root, false, &mut found);
        }
        found.into_iter()
    }

    /// The annotations of a valid result, depth first (empty when invalid) —
    /// the `basic` format's `annotations`.
    pub fn annotations(&self) -> impl Iterator<Item = &OutputUnit> {
        let mut found = Vec::new();
        if self.root.valid {
            collect(&self.root, true, &mut found);
        }
        found.into_iter()
    }

    /// The output in `format`, as JSON, every object's members in name order
    /// so the output's serialization is one fixed spelling.
    pub fn to_json(&self, format: OutputFormat) -> Value {
        let mut json = match format {
            OutputFormat::Flag => {
                let mut object = Object::new();
                object.insert("valid".to_owned(), Value::Bool(self.root.valid));
                Value::Object(object)
            }
            OutputFormat::Basic => {
                let mut object = header(&self.root);
                let (key, units): (&str, Vec<Value>) = if self.root.valid {
                    ("annotations", self.annotations().map(leaf).collect())
                } else {
                    ("errors", self.errors().map(leaf).collect())
                };
                if !units.is_empty() {
                    object.insert(key.to_owned(), Value::Array(units));
                }
                Value::Object(object)
            }
            OutputFormat::Detailed => detailed(&self.root, self.root.valid)
                .unwrap_or_else(|| Value::Object(header(&self.root))),
        };
        json.sort_keys();
        json
    }
}

/// Every unit beneath `unit` (inclusive) on paths whose validity is `valid`
/// that carries an annotation (`valid`) or an error (`!valid`), depth first,
/// over a heap work list.
fn collect<'u>(unit: &'u OutputUnit, valid: bool, found: &mut Vec<&'u OutputUnit>) {
    let mut work = vec![unit];
    while let Some(unit) = work.pop() {
        if unit.valid != valid {
            continue;
        }
        let carries = if valid {
            unit.annotation.is_some()
        } else {
            unit.error.is_some()
        };
        if carries {
            found.push(unit);
        }
        work.extend(unit.children.iter().rev());
    }
}

fn header(unit: &OutputUnit) -> Object {
    let mut object = Object::new();
    object.insert("valid".to_owned(), Value::Bool(unit.valid));
    object.insert(
        "keywordLocation".to_owned(),
        Value::String(unit.keyword_location.clone()),
    );
    object.insert(
        "absoluteKeywordLocation".to_owned(),
        Value::String(unit.absolute_keyword_location.clone()),
    );
    object.insert(
        "instanceLocation".to_owned(),
        Value::String(unit.instance_location.clone()),
    );
    object
}

fn leaf(unit: &OutputUnit) -> Value {
    let mut object = header(unit);
    if let Some(error) = &unit.error {
        object.insert("error".to_owned(), Value::String(error.clone()));
    }
    if let Some(annotation) = &unit.annotation {
        object.insert("annotation".to_owned(), annotation.clone());
    }
    Value::Object(object)
}

/// The condensed hierarchy beneath `unit` along paths whose validity is
/// `valid`; `None` when nothing there reports anything. Built bottom up over
/// heap work lists, so the unit tree's depth costs no stack.
fn detailed(unit: &OutputUnit, valid: bool) -> Option<Value> {
    enum Step<'u> {
        Enter(&'u OutputUnit),
        Exit(&'u OutputUnit),
    }
    let mut steps = vec![Step::Enter(unit)];
    // One entry per unit entered, in order: its condensed hierarchy.
    let mut built: Vec<Option<Value>> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Enter(unit) if unit.valid != valid => built.push(None),
            Step::Enter(unit) => {
                steps.push(Step::Exit(unit));
                steps.extend(unit.children.iter().rev().map(Step::Enter));
            }
            Step::Exit(unit) => {
                let first = built.len() - unit.children.len();
                let children: Vec<Value> = built.drain(first..).flatten().collect();
                built.push(condense(unit, valid, children));
            }
        }
    }
    built.pop().flatten()
}

/// One unit of the detailed format, given its children's condensed
/// hierarchies: `None` when it and they report nothing, its one child when it
/// reports nothing itself, and otherwise its own unit over them.
fn condense(unit: &OutputUnit, valid: bool, mut children: Vec<Value>) -> Option<Value> {
    let own = if valid {
        unit.annotation.is_some()
    } else {
        unit.error.is_some()
    };
    if !own {
        match children.len() {
            0 => return None,
            1 => return children.pop(),
            _ => {}
        }
    }
    let mut object = header(unit);
    if let Some(error) = unit.error.as_ref().filter(|_| !valid) {
        object.insert("error".to_owned(), Value::String(error.clone()));
    }
    if let Some(annotation) = unit.annotation.as_ref().filter(|_| valid) {
        object.insert("annotation".to_owned(), annotation.clone());
    }
    if !children.is_empty() {
        let key = if valid { "annotations" } else { "errors" };
        object.insert(key.to_owned(), Value::Array(children));
    }
    Some(Value::Object(object))
}
