// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Evaluating an instance against a compiled [`Schema`].
//!
//! One evaluator serves both entry points. [`Schema::is_valid`] runs it with
//! output off: a subschema stops at its first failing keyword, and `anyOf`,
//! `oneOf` and `contains` stop as soon as their verdict is decided. The only
//! exception is a subschema carrying `unevaluatedItems` or
//! `unevaluatedProperties`: it and every subschema applied in place beneath
//! it record which items and properties they evaluated (Core §11), and a
//! passing `anyOf` there evaluates every branch, because each passing branch
//! contributes. [`Schema::evaluate`] runs it with output on and evaluates
//! everything, recording one [`OutputUnit`] per subschema and keyword.
//!
//! The dynamic scope (Core §7.1) is the stack of schema resources the
//! evaluation path has entered; `$dynamicRef` searches it outermost first.
//! A `$ref` or `$dynamicRef` that re-enters a subschema it is already
//! evaluating at the same instance location would never terminate, and fails
//! with an error naming the cycle instead. A chain of more than
//! [`MAX_REF_CHAIN`] distinct references followed in a row at one instance
//! location stops the evaluation with [`EvaluationCause::ReferenceChain`].
//!
//! The evaluator keeps its subschemas in progress on a heap-allocated work
//! stack, never on the machine stack: an instance nested thousands of levels
//! deep costs memory proportional to its depth, not a thread stack that a
//! wasm32 host or a small worker thread cannot grow.

use std::collections::BTreeMap;
use std::rc::Rc;

use serde_json::Value;
use serde_json::map::{Iter as MapIter, Keys as MapKeys};

use crate::ecma::{self, CompiledPattern, MatchLimits, PatternError};
use crate::equal;
use crate::error::{EvaluationCause, EvaluationError};
use crate::format::Format;
use crate::number::Decimal;
use crate::output::{Output, OutputUnit};
use crate::pointer;
use crate::schema::{Body, JsonType, Keyword, Kind, Node, NodeId, Pattern, Schema};
use purrdf_iri::percent;

/// The longest chain of `$ref`, `$dynamicRef` and `$recursiveRef`
/// resolutions an evaluation follows in a row at one instance location.
///
/// Every link of such a chain re-applies a schema to the same value without
/// consuming any of it, so the chain's length is bounded by the schema, not
/// by the instance; a longer chain stops the evaluation with
/// [`EvaluationCause::ReferenceChain`] rather than spending unbounded work on
/// one value. Descending into an item or a property starts a new chain, so
/// the bound never limits how deep an instance may be.
pub const MAX_REF_CHAIN: usize = 250;

impl Schema {
    /// Whether `instance` is valid — the `flag` output, computed with every
    /// short cut the verdict allows.
    pub fn is_valid(&self, instance: &Value) -> Result<bool, EvaluationError> {
        self.is_valid_with_limits(instance, MatchLimits::default())
    }

    /// Validate with a caller-selected shared regular-expression budget.
    pub fn is_valid_with_limits(
        &self,
        instance: &Value,
        limits: MatchLimits,
    ) -> Result<bool, EvaluationError> {
        let mut evaluator = Evaluator::new(self, false, limits);
        let valid = evaluator.run(self.root, instance).valid;
        evaluator.error.map_or(Ok(valid), Err)
    }

    /// Evaluate `instance` and record the full output unit tree, from which
    /// [`Output::to_json`] writes any of the standard formats.
    pub fn evaluate(&self, instance: &Value) -> Result<Output, EvaluationError> {
        self.evaluate_with_limits(instance, MatchLimits::default())
    }

    /// Produce output with a caller-selected shared regular-expression budget.
    pub fn evaluate_with_limits(
        &self,
        instance: &Value,
        limits: MatchLimits,
    ) -> Result<Output, EvaluationError> {
        let mut evaluator = Evaluator::new(self, true, limits);
        let outcome = evaluator.run(self.root, instance);
        if let Some(error) = evaluator.error {
            return Err(error);
        }
        let root = outcome.unit.map_or_else(
            || OutputUnit {
                valid: outcome.valid,
                keyword_location: String::new(),
                absolute_keyword_location: self.nodes[self.root].location.clone(),
                instance_location: String::new(),
                error: None,
                annotation: None,
                children: Vec::new(),
            },
            |unit| *unit,
        );
        Ok(Output { root })
    }
}

/// A set of item indices or property positions.
#[derive(Debug, Clone, Default)]
struct Bits(Vec<u64>);

impl Bits {
    fn new(len: usize) -> Self {
        Self(vec![0; len.div_ceil(64)])
    }

    fn set(&mut self, index: usize) {
        self.0[index / 64] |= 1 << (index % 64);
    }

    fn get(&self, index: usize) -> bool {
        self.0[index / 64] & (1 << (index % 64)) != 0
    }

    fn union(&mut self, other: &Self) {
        for (word, other) in self.0.iter_mut().zip(&other.0) {
            *word |= other;
        }
    }

    fn fill(&mut self, len: usize) {
        for index in 0..len {
            self.set(index);
        }
    }
}

/// The result of evaluating one subschema.
struct Outcome {
    valid: bool,
    /// Evaluated property positions, when tracking.
    properties: Option<Bits>,
    /// Evaluated item indices, when tracking.
    items: Option<Bits>,
    /// Its output unit, when recording output; boxed so an outcome stays
    /// small as it passes between subschemas.
    unit: Option<Box<OutputUnit>>,
}

/// What a subschema's keywords accumulate.
struct State {
    properties: Option<Bits>,
    items: Option<Bits>,
    units: Vec<OutputUnit>,
}

impl State {
    fn absorb(&mut self, outcome: &Outcome) {
        if !outcome.valid {
            return;
        }
        if let (Some(mine), Some(theirs)) = (&mut self.properties, &outcome.properties) {
            mine.union(theirs);
        }
        if let (Some(mine), Some(theirs)) = (&mut self.items, &outcome.items) {
            mine.union(theirs);
        }
    }
}

/// A keyword's verdict: its annotation when it passed, its error when not.
type Verdict = Result<Option<Value>, String>;

/// The value a subschema is applied to.
#[derive(Clone)]
enum Instance<'v> {
    /// A value inside the caller's instance.
    Borrowed(&'v Value),
    /// A property name, which `propertyNames` evaluates as a string.
    Name(Rc<Value>),
}

impl<'v> Instance<'v> {
    fn value(&self) -> &Value {
        match self {
            Self::Borrowed(value) => value,
            Self::Name(name) => name,
        }
    }

    /// The array, when the value is one. A property name is a string, so
    /// only a value inside the caller's instance can be an array.
    fn array(&self) -> Option<&'v [Value]> {
        match self {
            Self::Borrowed(Value::Array(items)) => Some(items),
            _ => None,
        }
    }

    /// The object, when the value is one.
    fn object(&self) -> Option<&'v serde_json::Map<String, Value>> {
        match self {
            Self::Borrowed(Value::Object(map)) => Some(map),
            _ => None,
        }
    }
}

/// One location token an application appends; written out only when output
/// is on, so the hot path never allocates one.
#[derive(Clone, Copy)]
enum Token<'s, 'v> {
    None,
    Index(usize),
    Schema(&'s str),
    Instance(&'v str),
}

impl Token<'_, '_> {
    fn push_to(self, path: &mut String) {
        let escaped = match self {
            Self::None => return,
            Self::Index(index) => std::borrow::Cow::Owned(index.to_string()),
            Self::Schema(token) => pointer::escape_token(token),
            Self::Instance(token) => pointer::escape_token(token),
        };
        path.push('/');
        path.push_str(&escaped);
    }
}

/// A keyword asking for subschema `id` to be applied to `instance`.
struct Request<'s, 'v> {
    id: NodeId,
    instance: Instance<'v>,
    collect: bool,
    keyword_token: Token<'s, 'v>,
    instance_token: Token<'s, 'v>,
}

impl<'s, 'v> Request<'s, 'v> {
    /// Apply `id` to the keyword's own instance.
    fn here(id: NodeId, instance: &Instance<'v>, collect: bool, token: Token<'s, 'v>) -> Self {
        Self {
            id,
            instance: instance.clone(),
            collect,
            keyword_token: token,
            instance_token: Token::None,
        }
    }

    /// Apply `id` to a value one level below the keyword's instance.
    const fn below(
        id: NodeId,
        value: &'v Value,
        keyword_token: Token<'s, 'v>,
        instance_token: Token<'s, 'v>,
    ) -> Self {
        Self {
            id,
            instance: Instance::Borrowed(value),
            collect: false,
            keyword_token,
            instance_token,
        }
    }
}

/// What an applicator keyword does next.
enum Resume<'s, 'v> {
    /// Apply a subschema, then resume with its outcome.
    Apply(Request<'s, 'v>),
    /// The keyword is decided.
    Done(Verdict),
}

/// An applicator keyword's progress: where its loop stands and what it has
/// gathered, so it can resume when the subschema it asked for completes.
enum Cursor<'s, 'v> {
    Follow(NodeId),
    Conditional {
        then: Option<NodeId>,
        otherwise: Option<NodeId>,
        /// The branch taken once `if` is decided.
        branch: Option<&'static str>,
        saved: usize,
    },
    AllOf {
        schemas: &'s [NodeId],
        index: usize,
        failed: Vec<usize>,
    },
    AnyOf {
        schemas: &'s [NodeId],
        index: usize,
        any: bool,
    },
    OneOf {
        schemas: &'s [NodeId],
        index: usize,
        passed: Vec<usize>,
        first: Option<Outcome>,
    },
    Not(NodeId),
    DependentSchemas {
        dependencies: &'s [(String, NodeId)],
        map: &'v serde_json::Map<String, Value>,
        index: usize,
        failed: Vec<&'s str>,
    },
    PrefixItems {
        schemas: &'s [NodeId],
        items: &'v [Value],
        applied: usize,
        index: usize,
        failed: Vec<usize>,
    },
    Items {
        schema: NodeId,
        items: &'v [Value],
        index: usize,
        skip: usize,
        failed: Vec<usize>,
    },
    Contains {
        schema: NodeId,
        min: u64,
        max: Option<u64>,
        evaluates: bool,
        items: &'v [Value],
        index: usize,
        matched: Vec<usize>,
    },
    Properties {
        schemas: &'s BTreeMap<String, NodeId>,
        entries: std::iter::Enumerate<MapIter<'v>>,
        current: Option<(usize, &'v String)>,
        applied: Vec<Value>,
        failed: Vec<&'v str>,
    },
    PatternProperties {
        patterns: &'s [(Pattern, NodeId)],
        entries: std::iter::Enumerate<MapIter<'v>>,
        current: Option<(usize, &'v String, &'v Value)>,
        pattern: usize,
        matched: bool,
        applied: Vec<Value>,
        failed: Vec<&'v str>,
    },
    AdditionalProperties {
        schema: NodeId,
        properties: &'s [String],
        patterns: &'s [CompiledPattern],
        entries: std::iter::Enumerate<MapIter<'v>>,
        current: Option<(usize, &'v String)>,
        applied: Vec<Value>,
        failed: Vec<&'v str>,
    },
    PropertyNames {
        schema: NodeId,
        names: MapKeys<'v>,
        current: Option<&'v str>,
        failed: Vec<&'v str>,
    },
    UnevaluatedItems {
        schema: NodeId,
        items: &'v [Value],
        index: usize,
        any: bool,
        failed: Vec<usize>,
    },
    UnevaluatedProperties {
        schema: NodeId,
        map: &'v serde_json::Map<String, Value>,
        entries: std::iter::Enumerate<MapIter<'v>>,
        current: Option<&'v String>,
        applied: Vec<Value>,
        failed: Vec<&'v str>,
    },
}

/// The keyword being evaluated, which an [`EvaluationError`] names; spelled
/// out only when an error is actually recorded.
type Active<'s> = Option<(&'s Node, &'s Keyword)>;

/// The keyword path length and active keyword to restore when a keyword is
/// decided.
type Restore<'s> = (usize, Active<'s>);

/// An applicator keyword waiting for a subschema's outcome.
struct Pending<'s, 'v> {
    keyword: &'s Keyword,
    /// The keyword path length and active keyword to restore when the
    /// keyword is decided; `None` for `if`, which records its own units.
    restore: Option<Restore<'s>>,
    /// The units of the subschemas it applied.
    children: Vec<OutputUnit>,
    cursor: Cursor<'s, 'v>,
}

/// A subschema being evaluated: its keywords run in order, and an applicator
/// among them suspends it until the subschema it applies completes.
struct Frame<'s, 'v> {
    node: &'s Node,
    keywords: &'s [Keyword],
    instance: Instance<'v>,
    /// Whether this subschema entered a new resource of the dynamic scope.
    entered: bool,
    track: bool,
    state: State,
    valid: bool,
    next: usize,
    pending: Option<Pending<'s, 'v>>,
    /// The path lengths to restore once the subschema completes.
    keyword_saved: usize,
    instance_saved: usize,
}

/// What a frame does next.
enum Step<'s, 'v> {
    Apply(Request<'s, 'v>),
    Done(Outcome),
}

/// A reference being followed: its target, the instance it is applied to,
/// and how many references in a row have reached that instance.
struct Link {
    target: NodeId,
    address: usize,
    chain: usize,
}

struct Evaluator<'s> {
    schema: &'s Schema,
    output: bool,
    scope: Vec<usize>,
    /// Every reference being followed, outermost first.
    following: Vec<Link>,
    keyword_path: String,
    instance_path: String,
    active_keyword: Active<'s>,
    limits: MatchLimits,
    error: Option<EvaluationError>,
}

fn address(value: &Value) -> usize {
    std::ptr::from_ref(value) as usize
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(number) => {
            if Decimal::from_number(number).is_integer() {
                "integer"
            } else {
                "number"
            }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn has_type(value: &Value, wanted: JsonType) -> bool {
    match (wanted, value) {
        (JsonType::Null, Value::Null)
        | (JsonType::Boolean, Value::Bool(_))
        | (JsonType::Object, Value::Object(_))
        | (JsonType::Array, Value::Array(_))
        | (JsonType::Number, Value::Number(_))
        | (JsonType::String, Value::String(_)) => true,
        (JsonType::Integer, Value::Number(number)) => Decimal::from_number(number).is_integer(),
        _ => false,
    }
}

impl<'s> Evaluator<'s> {
    const fn new(schema: &'s Schema, output: bool, limits: MatchLimits) -> Self {
        Self {
            schema,
            output,
            scope: Vec::new(),
            following: Vec::new(),
            keyword_path: String::new(),
            instance_path: String::new(),
            active_keyword: None,
            limits,
            error: None,
        }
    }

    /// Record the first reason the evaluation cannot reach a verdict.
    fn stop(&mut self, cause: EvaluationCause) {
        let active = self.active_keyword;
        self.error.get_or_insert_with(|| EvaluationError {
            keyword_location: active.map_or_else(String::new, |(node, keyword)| {
                format!("{}/{}", node.location, pointer::escape_token(&keyword.name))
            }),
            instance_location: self.instance_path.clone(),
            cause,
        });
    }

    fn matches(&mut self, pattern: &CompiledPattern, text: &str) -> bool {
        match pattern.is_match(text, &mut self.limits) {
            Ok(verdict) => verdict,
            Err(cause) => {
                self.stop(EvaluationCause::Pattern(cause));
                false
            }
        }
    }

    fn format_matches(&mut self, format: Format, text: &str) -> bool {
        if format != Format::Regex {
            return format.check(text);
        }
        match ecma::parse(text) {
            Ok(_) => true,
            Err(error @ PatternError::Resource { .. }) => {
                self.stop(EvaluationCause::Pattern(error));
                false
            }
            Err(_) => false,
        }
    }

    fn annotate(&self, value: Value) -> Verdict {
        Ok(self.output.then_some(value))
    }

    /// Evaluate the root subschema over `instance` on the work stack.
    fn run<'v>(&mut self, root: NodeId, instance: &'v Value) -> Outcome {
        let mut stack: Vec<Frame<'s, 'v>> = Vec::with_capacity(16);
        if let Some(outcome) =
            self.enter(&mut stack, root, Instance::Borrowed(instance), false, 0, 0)
        {
            return outcome;
        }
        let mut delivered = None;
        while let Some(frame) = stack.last_mut() {
            match self.step(frame, delivered.take()) {
                Step::Apply(request) => {
                    let keyword_saved = self.keyword_path.len();
                    let instance_saved = self.instance_path.len();
                    if self.output {
                        request.keyword_token.push_to(&mut self.keyword_path);
                        request.instance_token.push_to(&mut self.instance_path);
                    }
                    if let Some(outcome) = self.enter(
                        &mut stack,
                        request.id,
                        request.instance,
                        request.collect,
                        keyword_saved,
                        instance_saved,
                    ) {
                        self.keyword_path.truncate(keyword_saved);
                        self.instance_path.truncate(instance_saved);
                        delivered = Some(outcome);
                    }
                }
                Step::Done(outcome) => {
                    let (keyword_saved, instance_saved) =
                        (frame.keyword_saved, frame.instance_saved);
                    if frame.entered {
                        self.scope.pop();
                    }
                    stack.pop();
                    self.keyword_path.truncate(keyword_saved);
                    self.instance_path.truncate(instance_saved);
                    if stack.is_empty() {
                        return outcome;
                    }
                    delivered = Some(outcome);
                }
            }
        }
        unreachable!("the root subschema returns its outcome when it completes")
    }

    /// Begin evaluating subschema `id` over `instance`: a boolean subschema
    /// is decided at once and its outcome returned, one with keywords is
    /// pushed onto `stack` as a frame.
    fn enter<'v>(
        &mut self,
        stack: &mut Vec<Frame<'s, 'v>>,
        id: NodeId,
        instance: Instance<'v>,
        collect: bool,
        keyword_saved: usize,
        instance_saved: usize,
    ) -> Option<Outcome> {
        let schema = self.schema;
        let node = &schema.nodes[id];
        match &node.body {
            Body::Bool(true) => Some(Outcome {
                valid: true,
                properties: None,
                items: None,
                unit: self
                    .output
                    .then(|| Box::new(self.unit(node, true, None, Vec::new()))),
            }),
            Body::Bool(false) => Some(Outcome {
                valid: false,
                properties: None,
                items: None,
                unit: self.output.then(|| {
                    Box::new(self.unit(
                        node,
                        false,
                        Some("the false schema accepts nothing".to_owned()),
                        Vec::new(),
                    ))
                }),
            }),
            Body::Keywords { keywords, tracks } => {
                let entered = self.scope.last() != Some(&node.resource);
                if entered {
                    self.scope.push(node.resource);
                }
                let track = collect || *tracks;
                let value = instance.value();
                let state = State {
                    properties: match value {
                        Value::Object(map) if track => Some(Bits::new(map.len())),
                        _ => None,
                    },
                    items: match value {
                        Value::Array(items) if track => Some(Bits::new(items.len())),
                        _ => None,
                    },
                    units: Vec::new(),
                };
                stack.push(Frame {
                    node,
                    keywords,
                    instance,
                    entered,
                    track,
                    state,
                    valid: true,
                    next: 0,
                    pending: None,
                    keyword_saved,
                    instance_saved,
                });
                None
            }
        }
    }

    /// Advance `frame`: resume its suspended keyword with `delivered`, then
    /// run keywords until one applies a subschema or none remain.
    fn step<'v>(&mut self, frame: &mut Frame<'s, 'v>, delivered: Option<Outcome>) -> Step<'s, 'v> {
        if let Some(mut outcome) = delivered {
            let Some(pending) = frame.pending.as_mut() else {
                unreachable!("an outcome is delivered only to the keyword that asked for it")
            };
            if let Some(unit) = outcome.unit.take() {
                pending.children.push(*unit);
            }
            let resumed = self.resume(frame, Some(outcome));
            if let Some(request) = self.settle(frame, resumed) {
                return Step::Apply(request);
            }
        }
        loop {
            if !frame.valid && !self.output {
                break;
            }
            let Some(keyword) = frame.keywords.get(frame.next) else {
                break;
            };
            frame.next += 1;
            if let Some(request) = self.begin(frame, keyword) {
                return Step::Apply(request);
            }
        }
        let unit = self.output.then(|| {
            Box::new(self.unit(
                frame.node,
                frame.valid,
                None,
                std::mem::take(&mut frame.state.units),
            ))
        });
        Step::Done(Outcome {
            valid: frame.valid,
            properties: frame.state.properties.take(),
            items: frame.state.items.take(),
            unit,
        })
    }

    /// Start `keyword`; returns the subschema it applies first, if any.
    fn begin<'v>(
        &mut self,
        frame: &mut Frame<'s, 'v>,
        keyword: &'s Keyword,
    ) -> Option<Request<'s, 'v>> {
        if let Kind::If {
            condition,
            then,
            otherwise,
        } = &keyword.kind
        {
            // `if`/`then`/`else` (Core §10.2.2): `if` never fails; `then`
            // applies when it passed, `else` when it failed. Each is its own
            // keyword unit.
            let saved = self.keyword_path.len();
            if self.output {
                self.keyword_path.push_str("/if");
            }
            frame.pending = Some(Pending {
                keyword,
                restore: None,
                children: Vec::new(),
                cursor: Cursor::Conditional {
                    then: *then,
                    otherwise: *otherwise,
                    branch: None,
                    saved,
                },
            });
            return Some(Request::here(
                *condition,
                &frame.instance,
                frame.track,
                Token::None,
            ));
        }
        let saved = self.keyword_path.len();
        let saved_active = self.active_keyword.replace((frame.node, keyword));
        if self.output {
            self.keyword_path.push('/');
            self.keyword_path
                .push_str(&pointer::escape_token(&keyword.name));
        }
        let restore = Some((saved, saved_active));
        let Some(cursor) = self.cursor(&keyword.kind, &frame.instance) else {
            let verdict = self.leaf(&keyword.kind, frame.instance.value());
            self.decide(frame, keyword, restore, Vec::new(), verdict);
            return None;
        };
        frame.pending = Some(Pending {
            keyword,
            restore,
            children: Vec::new(),
            cursor,
        });
        let resumed = self.resume(frame, None);
        self.settle(frame, resumed)
    }

    /// Pass on a keyword's request, or record its verdict when it is decided.
    fn settle<'v>(
        &mut self,
        frame: &mut Frame<'s, 'v>,
        resumed: Resume<'s, 'v>,
    ) -> Option<Request<'s, 'v>> {
        match resumed {
            Resume::Apply(request) => Some(request),
            Resume::Done(verdict) => {
                let Some(pending) = frame.pending.take() else {
                    unreachable!("a keyword is decided only while it is pending")
                };
                self.decide(
                    frame,
                    pending.keyword,
                    pending.restore,
                    pending.children,
                    verdict,
                );
                None
            }
        }
    }

    /// Record a decided keyword's unit and restore the paths it extended.
    fn decide(
        &mut self,
        frame: &mut Frame<'s, '_>,
        keyword: &'s Keyword,
        restore: Option<Restore<'s>>,
        children: Vec<OutputUnit>,
        verdict: Verdict,
    ) {
        if verdict.is_err() {
            frame.valid = false;
        }
        let Some((saved, saved_active)) = restore else {
            return;
        };
        if self.output {
            let unit = self.keyword_unit(frame.node, &keyword.name, verdict, children);
            frame.state.units.push(unit);
        }
        self.keyword_path.truncate(saved);
        self.active_keyword = saved_active;
    }

    fn unit(
        &self,
        node: &Node,
        valid: bool,
        error: Option<String>,
        children: Vec<OutputUnit>,
    ) -> OutputUnit {
        OutputUnit {
            valid,
            keyword_location: self.keyword_path.clone(),
            absolute_keyword_location: node.location.clone(),
            instance_location: self.instance_path.clone(),
            error,
            annotation: None,
            children,
        }
    }

    fn keyword_unit(
        &self,
        node: &Node,
        name: &str,
        verdict: Verdict,
        children: Vec<OutputUnit>,
    ) -> OutputUnit {
        let (valid, error, annotation) = match verdict {
            Ok(annotation) => (true, None, annotation),
            Err(error) => (false, Some(error), None),
        };
        OutputUnit {
            valid,
            keyword_location: self.keyword_path.clone(),
            absolute_keyword_location: format!(
                "{}/{}",
                node.location,
                percent::encode(&pointer::escape_token(name), percent::FRAGMENT)
            ),
            instance_location: self.instance_path.clone(),
            error,
            annotation,
            children,
        }
    }

    /// The reference target `$dynamicRef` or `$recursiveRef` reaches from
    /// the current dynamic scope.
    fn reference_target(&self, kind: &Kind) -> Option<NodeId> {
        match kind {
            Kind::Ref(target) => Some(*target),
            Kind::DynamicRef { target, anchor } => Some(
                anchor
                    .as_ref()
                    .and_then(|name| {
                        self.scope.iter().find_map(|&resource| {
                            self.schema.resources[resource]
                                .dynamic_anchors
                                .get(name)
                                .copied()
                        })
                    })
                    .unwrap_or(*target),
            ),
            Kind::RecursiveRef(target) => {
                // 2019-09 Core §8.2.4.2.2: when the target's resource has
                // `$recursiveAnchor: true`, the outermost resource in the
                // dynamic scope that also has one is the target instead.
                let resource = self.schema.nodes[*target].resource;
                Some(
                    if self.schema.resources[resource].recursive_root.is_some() {
                        self.scope
                            .iter()
                            .find_map(|&resource| self.schema.resources[resource].recursive_root)
                            .unwrap_or(*target)
                    } else {
                        *target
                    },
                )
            }
            _ => None,
        }
    }

    /// The cursor of an applicator keyword that has subschemas to apply;
    /// `None` for any other keyword, including an applicator whose instance
    /// has a type it does not apply to, which [`Self::leaf`] decides at once.
    fn cursor<'v>(&self, kind: &'s Kind, instance: &Instance<'v>) -> Option<Cursor<'s, 'v>> {
        Some(match kind {
            Kind::Ref(_) | Kind::DynamicRef { .. } | Kind::RecursiveRef(_) => {
                Cursor::Follow(self.reference_target(kind)?)
            }
            Kind::AllOf(schemas) => Cursor::AllOf {
                schemas,
                index: 0,
                failed: Vec::new(),
            },
            Kind::AnyOf(schemas) => Cursor::AnyOf {
                schemas,
                index: 0,
                any: false,
            },
            Kind::OneOf(schemas) => Cursor::OneOf {
                schemas,
                index: 0,
                passed: Vec::new(),
                first: None,
            },
            Kind::Not(schema) => Cursor::Not(*schema),
            Kind::DependentSchemas(dependencies) => Cursor::DependentSchemas {
                dependencies,
                map: instance.object()?,
                index: 0,
                failed: Vec::new(),
            },
            Kind::PrefixItems(schemas) => {
                let items = instance.array()?;
                Cursor::PrefixItems {
                    schemas,
                    items,
                    applied: items.len().min(schemas.len()),
                    index: 0,
                    failed: Vec::new(),
                }
            }
            Kind::Items { schema, skip } => Cursor::Items {
                schema: *schema,
                items: instance.array()?,
                index: *skip,
                skip: *skip,
                failed: Vec::new(),
            },
            Kind::Contains {
                schema,
                min,
                max,
                evaluates,
            } => Cursor::Contains {
                schema: *schema,
                min: *min,
                max: *max,
                evaluates: *evaluates,
                items: instance.array()?,
                index: 0,
                matched: Vec::new(),
            },
            Kind::Properties(schemas) => Cursor::Properties {
                schemas,
                entries: instance.object()?.iter().enumerate(),
                current: None,
                applied: Vec::new(),
                failed: Vec::new(),
            },
            Kind::PatternProperties(patterns) => Cursor::PatternProperties {
                patterns,
                entries: instance.object()?.iter().enumerate(),
                current: None,
                pattern: 0,
                matched: false,
                applied: Vec::new(),
                failed: Vec::new(),
            },
            Kind::AdditionalProperties {
                schema,
                properties,
                patterns,
            } => Cursor::AdditionalProperties {
                schema: *schema,
                properties,
                patterns,
                entries: instance.object()?.iter().enumerate(),
                current: None,
                applied: Vec::new(),
                failed: Vec::new(),
            },
            Kind::PropertyNames(schema) => Cursor::PropertyNames {
                schema: *schema,
                names: instance.object()?.keys(),
                current: None,
                failed: Vec::new(),
            },
            Kind::UnevaluatedItems(schema) => Cursor::UnevaluatedItems {
                schema: *schema,
                items: instance.array()?,
                index: 0,
                any: false,
                failed: Vec::new(),
            },
            Kind::UnevaluatedProperties(schema) => {
                let map = instance.object()?;
                Cursor::UnevaluatedProperties {
                    schema: *schema,
                    map,
                    entries: map.iter().enumerate(),
                    current: None,
                    applied: Vec::new(),
                    failed: Vec::new(),
                }
            }
            _ => return None,
        })
    }

    /// Resume the frame's pending keyword with the outcome of the subschema
    /// it applied last (`None` when it starts), and say what it does next.
    #[allow(clippy::too_many_lines)] // one arm per applicator keyword
    fn resume<'v>(
        &mut self,
        frame: &mut Frame<'s, 'v>,
        delivered: Option<Outcome>,
    ) -> Resume<'s, 'v> {
        let output = self.output;
        let Frame {
            node,
            instance,
            track,
            state,
            pending,
            ..
        } = frame;
        let (node, track) = (*node, *track);
        let Some(pending) = pending.as_mut() else {
            unreachable!("only a pending keyword resumes")
        };
        match &mut pending.cursor {
            Cursor::Follow(target) => {
                let target = *target;
                let Some(outcome) = delivered else {
                    return self.follow(target, instance, track);
                };
                self.following.pop();
                state.absorb(&outcome);
                Resume::Done(if outcome.valid {
                    Ok(None)
                } else {
                    Err(format!(
                        "the referenced schema {} failed",
                        self.schema.nodes[target].location
                    ))
                })
            }
            Cursor::Conditional {
                then,
                otherwise,
                branch,
                saved,
            } => {
                let Some(outcome) = delivered else {
                    unreachable!("`if` is started by `begin`")
                };
                state.absorb(&outcome);
                let children = std::mem::take(&mut pending.children);
                match *branch {
                    None => {
                        if output {
                            let unit = self.keyword_unit(node, "if", Ok(None), children);
                            state.units.push(unit);
                        }
                        self.keyword_path.truncate(*saved);
                        let (next, name) = if outcome.valid {
                            (*then, "then")
                        } else {
                            (*otherwise, "else")
                        };
                        let Some(next) = next else {
                            return Resume::Done(Ok(None));
                        };
                        *branch = Some(name);
                        if output {
                            self.keyword_path.push('/');
                            self.keyword_path.push_str(name);
                        }
                        Resume::Apply(Request::here(next, instance, track, Token::None))
                    }
                    Some(name) => {
                        let verdict = if outcome.valid {
                            Ok(None)
                        } else {
                            Err(format!("the `{name}` subschema failed"))
                        };
                        if output {
                            let unit = self.keyword_unit(node, name, verdict.clone(), children);
                            state.units.push(unit);
                        }
                        self.keyword_path.truncate(*saved);
                        Resume::Done(verdict)
                    }
                }
            }
            Cursor::AllOf {
                schemas,
                index,
                failed,
            } => {
                if let Some(outcome) = delivered {
                    state.absorb(&outcome);
                    if !outcome.valid {
                        failed.push(*index);
                        if !output {
                            return Resume::Done(Err(format!("subschemas {failed:?} failed")));
                        }
                    }
                    *index += 1;
                }
                if let Some(&schema) = schemas.get(*index) {
                    return Resume::Apply(Request::here(
                        schema,
                        instance,
                        track,
                        Token::Index(*index),
                    ));
                }
                Resume::Done(if failed.is_empty() {
                    Ok(None)
                } else {
                    Err(format!("subschemas {failed:?} failed"))
                })
            }
            Cursor::AnyOf {
                schemas,
                index,
                any,
            } => {
                if let Some(outcome) = delivered {
                    state.absorb(&outcome);
                    *any |= outcome.valid;
                    *index += 1;
                }
                let decided = *any && !output && !track;
                if !decided && let Some(&schema) = schemas.get(*index) {
                    return Resume::Apply(Request::here(
                        schema,
                        instance,
                        track,
                        Token::Index(*index),
                    ));
                }
                Resume::Done(if *any {
                    Ok(None)
                } else {
                    Err("no subschema passed".to_owned())
                })
            }
            Cursor::OneOf {
                schemas,
                index,
                passed,
                first,
            } => {
                if let Some(outcome) = delivered {
                    if outcome.valid {
                        passed.push(*index);
                        if first.is_none() {
                            *first = Some(outcome);
                        }
                    }
                    *index += 1;
                }
                let decided = passed.len() > 1 && !output;
                if !decided && let Some(&schema) = schemas.get(*index) {
                    return Resume::Apply(Request::here(
                        schema,
                        instance,
                        track,
                        Token::Index(*index),
                    ));
                }
                Resume::Done(match (passed.len(), first) {
                    (1, Some(outcome)) => {
                        state.absorb(outcome);
                        Ok(None)
                    }
                    (0, _) => Err("no subschema passed".to_owned()),
                    _ => Err(format!("more than one subschema passed: {passed:?}")),
                })
            }
            Cursor::Not(schema) => match delivered {
                None => Resume::Apply(Request::here(*schema, instance, false, Token::None)),
                Some(outcome) => Resume::Done(if outcome.valid {
                    Err("the value must not be valid against the subschema".to_owned())
                } else {
                    Ok(None)
                }),
            },
            Cursor::DependentSchemas {
                dependencies,
                map,
                index,
                failed,
            } => {
                if let Some(outcome) = delivered {
                    state.absorb(&outcome);
                    if !outcome.valid {
                        failed.push(dependencies[*index].0.as_str());
                        if !output {
                            return Resume::Done(Err(format!(
                                "the dependent schemas of {failed:?} failed"
                            )));
                        }
                    }
                    *index += 1;
                }
                while let Some((property, schema)) = dependencies.get(*index) {
                    if map.contains_key(property) {
                        return Resume::Apply(Request::here(
                            *schema,
                            instance,
                            track,
                            Token::Schema(property),
                        ));
                    }
                    *index += 1;
                }
                Resume::Done(if failed.is_empty() {
                    Ok(None)
                } else {
                    Err(format!("the dependent schemas of {failed:?} failed"))
                })
            }
            Cursor::PrefixItems {
                schemas,
                items,
                applied,
                index,
                failed,
            } => {
                if let Some(outcome) = delivered {
                    if let Some(evaluated) = &mut state.items {
                        evaluated.set(*index);
                    }
                    if !outcome.valid {
                        failed.push(*index);
                        if !output {
                            return Resume::Done(Err(format!(
                                "items {failed:?} failed their prefix schemas"
                            )));
                        }
                    }
                    *index += 1;
                }
                if *index < *applied {
                    return Resume::Apply(Request::below(
                        schemas[*index],
                        &items[*index],
                        Token::Index(*index),
                        Token::Index(*index),
                    ));
                }
                if !failed.is_empty() {
                    return Resume::Done(Err(format!(
                        "items {failed:?} failed their prefix schemas"
                    )));
                }
                if *applied == 0 {
                    return Resume::Done(Ok(None));
                }
                Resume::Done(self.annotate(if *applied == items.len() {
                    Value::Bool(true)
                } else {
                    Value::from(*applied - 1)
                }))
            }
            Cursor::Items {
                schema,
                items,
                index,
                skip,
                failed,
            } => {
                if let Some(outcome) = delivered {
                    if let Some(evaluated) = &mut state.items {
                        evaluated.set(*index);
                    }
                    if !outcome.valid {
                        failed.push(*index);
                        if !output {
                            return Resume::Done(Err(format!(
                                "items {failed:?} failed the items schema"
                            )));
                        }
                    }
                    *index += 1;
                }
                if let Some(item) = items.get(*index) {
                    return Resume::Apply(Request::below(
                        *schema,
                        item,
                        Token::None,
                        Token::Index(*index),
                    ));
                }
                if !failed.is_empty() {
                    return Resume::Done(Err(format!("items {failed:?} failed the items schema")));
                }
                Resume::Done(if items.len() > *skip {
                    self.annotate(Value::Bool(true))
                } else {
                    Ok(None)
                })
            }
            Cursor::Contains {
                schema,
                min,
                max,
                evaluates,
                items,
                index,
                matched,
            } => {
                if let Some(outcome) = delivered {
                    if outcome.valid {
                        matched.push(*index);
                        if *evaluates && let Some(evaluated) = &mut state.items {
                            evaluated.set(*index);
                        }
                    }
                    *index += 1;
                }
                let decided_early =
                    !output && !track && max.is_none() && matched.len() as u64 >= *min;
                if !decided_early && let Some(item) = items.get(*index) {
                    return Resume::Apply(Request::below(
                        *schema,
                        item,
                        Token::None,
                        Token::Index(*index),
                    ));
                }
                let found = matched.len() as u64;
                if found < *min {
                    return Resume::Done(Err(format!(
                        "{found} items match the contains schema; at least {min} must"
                    )));
                }
                if let Some(max) = max
                    && found > *max
                {
                    return Resume::Done(Err(format!(
                        "{found} items match the contains schema; at most {max} may"
                    )));
                }
                Resume::Done(self.annotate(if matched.len() == items.len() {
                    Value::Bool(true)
                } else {
                    Value::from(std::mem::take(matched))
                }))
            }
            Cursor::Properties {
                schemas,
                entries,
                current,
                applied,
                failed,
            } => {
                if let Some(outcome) = delivered {
                    let Some((position, name)) = current.take() else {
                        unreachable!("an outcome answers the property that asked for it")
                    };
                    if let Some(evaluated) = &mut state.properties {
                        evaluated.set(position);
                    }
                    if output {
                        applied.push(Value::String(name.clone()));
                    }
                    if !outcome.valid {
                        failed.push(name.as_str());
                        if !output {
                            return Resume::Done(Err(format!(
                                "properties {failed:?} failed their schemas"
                            )));
                        }
                    }
                }
                for (position, (name, value)) in entries.by_ref() {
                    let Some(&schema) = schemas.get(name) else {
                        continue;
                    };
                    *current = Some((position, name));
                    return Resume::Apply(Request::below(
                        schema,
                        value,
                        Token::Instance(name),
                        Token::Instance(name),
                    ));
                }
                Resume::Done(if failed.is_empty() {
                    self.annotate(Value::Array(std::mem::take(applied)))
                } else {
                    Err(format!("properties {failed:?} failed their schemas"))
                })
            }
            Cursor::PatternProperties {
                patterns,
                entries,
                current,
                pattern,
                matched,
                applied,
                failed,
            } => {
                if let Some(outcome) = delivered {
                    let Some((_, name, _)) = *current else {
                        unreachable!("an outcome answers the property that asked for it")
                    };
                    if !outcome.valid {
                        failed.push(name.as_str());
                        if !output {
                            return Resume::Done(Err(format!(
                                "properties {failed:?} failed their pattern schemas"
                            )));
                        }
                    }
                    *pattern += 1;
                }
                loop {
                    let (position, name, value) = match *current {
                        Some(entry) => entry,
                        None => {
                            let Some((position, (name, value))) = entries.next() else {
                                break;
                            };
                            *current = Some((position, name, value));
                            *pattern = 0;
                            *matched = false;
                            (position, name, value)
                        }
                    };
                    while let Some((candidate, schema)) = patterns.get(*pattern) {
                        if self.matches(&candidate.regex, name) {
                            *matched = true;
                            return Resume::Apply(Request::below(
                                *schema,
                                value,
                                Token::Schema(&candidate.source),
                                Token::Instance(name),
                            ));
                        }
                        *pattern += 1;
                    }
                    if *matched {
                        if let Some(evaluated) = &mut state.properties {
                            evaluated.set(position);
                        }
                        if output {
                            applied.push(Value::String(name.clone()));
                        }
                    }
                    *current = None;
                }
                Resume::Done(if failed.is_empty() {
                    self.annotate(Value::Array(std::mem::take(applied)))
                } else {
                    Err(format!(
                        "properties {failed:?} failed their pattern schemas"
                    ))
                })
            }
            Cursor::AdditionalProperties {
                schema,
                properties,
                patterns,
                entries,
                current,
                applied,
                failed,
            } => {
                if let Some(outcome) = delivered {
                    let Some((position, name)) = current.take() else {
                        unreachable!("an outcome answers the property that asked for it")
                    };
                    if let Some(evaluated) = &mut state.properties {
                        evaluated.set(position);
                    }
                    if output {
                        applied.push(Value::String(name.clone()));
                    }
                    if !outcome.valid {
                        failed.push(name.as_str());
                        if !output {
                            return Resume::Done(Err(format!(
                                "additional properties {failed:?} are not allowed"
                            )));
                        }
                    }
                }
                for (position, (name, value)) in entries.by_ref() {
                    if properties.iter().any(|known| known == name)
                        || patterns.iter().any(|pattern| self.matches(pattern, name))
                    {
                        continue;
                    }
                    *current = Some((position, name));
                    return Resume::Apply(Request::below(
                        *schema,
                        value,
                        Token::None,
                        Token::Instance(name),
                    ));
                }
                Resume::Done(if failed.is_empty() {
                    self.annotate(Value::Array(std::mem::take(applied)))
                } else {
                    Err(format!("additional properties {failed:?} are not allowed"))
                })
            }
            Cursor::PropertyNames {
                schema,
                names,
                current,
                failed,
            } => {
                if let Some(outcome) = delivered {
                    let Some(name) = current.take() else {
                        unreachable!("an outcome answers the property name that asked for it")
                    };
                    if !outcome.valid {
                        failed.push(name);
                        if !output {
                            return Resume::Done(Err(format!(
                                "property names {failed:?} are not valid"
                            )));
                        }
                    }
                }
                if let Some(name) = names.next() {
                    *current = Some(name);
                    return Resume::Apply(Request {
                        id: *schema,
                        instance: Instance::Name(Rc::new(Value::String(name.clone()))),
                        collect: false,
                        keyword_token: Token::None,
                        instance_token: Token::Instance(name),
                    });
                }
                Resume::Done(if failed.is_empty() {
                    Ok(None)
                } else {
                    Err(format!("property names {failed:?} are not valid"))
                })
            }
            Cursor::UnevaluatedItems {
                schema,
                items,
                index,
                any,
                failed,
            } => {
                let mut stopped = false;
                if let Some(outcome) = delivered {
                    if !outcome.valid {
                        failed.push(*index);
                        stopped = !output;
                    }
                    *index += 1;
                }
                if !stopped {
                    while let Some(item) = items.get(*index) {
                        if state
                            .items
                            .as_ref()
                            .is_some_and(|evaluated| evaluated.get(*index))
                        {
                            *index += 1;
                            continue;
                        }
                        *any = true;
                        return Resume::Apply(Request::below(
                            *schema,
                            item,
                            Token::None,
                            Token::Index(*index),
                        ));
                    }
                }
                if let Some(evaluated) = &mut state.items {
                    evaluated.fill(items.len());
                }
                if !failed.is_empty() {
                    return Resume::Done(Err(format!(
                        "unevaluated items {failed:?} are not allowed"
                    )));
                }
                Resume::Done(if *any {
                    self.annotate(Value::Bool(true))
                } else {
                    Ok(None)
                })
            }
            Cursor::UnevaluatedProperties {
                schema,
                map,
                entries,
                current,
                applied,
                failed,
            } => {
                let mut stopped = false;
                if let Some(outcome) = delivered {
                    let Some(name) = current.take() else {
                        unreachable!("an outcome answers the property that asked for it")
                    };
                    if output {
                        applied.push(Value::String(name.clone()));
                    }
                    if !outcome.valid {
                        failed.push(name.as_str());
                        stopped = !output;
                    }
                }
                if !stopped {
                    for (position, (name, value)) in entries.by_ref() {
                        if state
                            .properties
                            .as_ref()
                            .is_some_and(|evaluated| evaluated.get(position))
                        {
                            continue;
                        }
                        *current = Some(name);
                        return Resume::Apply(Request::below(
                            *schema,
                            value,
                            Token::None,
                            Token::Instance(name),
                        ));
                    }
                }
                if let Some(evaluated) = &mut state.properties {
                    evaluated.fill(map.len());
                }
                Resume::Done(if failed.is_empty() {
                    self.annotate(Value::Array(std::mem::take(applied)))
                } else {
                    Err(format!("unevaluated properties {failed:?} are not allowed"))
                })
            }
        }
    }

    /// Follow a reference to `target`, refusing to re-enter a subschema at an
    /// instance location it is already evaluating, and stopping a chain of
    /// more than [`MAX_REF_CHAIN`] references at one instance location.
    fn follow<'v>(
        &mut self,
        target: NodeId,
        instance: &Instance<'v>,
        track: bool,
    ) -> Resume<'s, 'v> {
        let address = address(instance.value());
        if self
            .following
            .iter()
            .any(|link| link.target == target && link.address == address)
        {
            return Resume::Done(Err(format!(
                "reference cycle: {} is re-entered at the same instance location without \
                 consuming any of it",
                self.schema.nodes[target].location
            )));
        }
        // The references being followed reach ever deeper values, so the
        // ones that reached this very value are the innermost run of them.
        let chain = match self.following.last() {
            Some(link) if link.address == address => link.chain + 1,
            _ => 1,
        };
        if chain > MAX_REF_CHAIN {
            self.stop(EvaluationCause::ReferenceChain);
            return Resume::Done(Err(format!(
                "more than {MAX_REF_CHAIN} references are followed in a row at one \
                 instance location"
            )));
        }
        self.following.push(Link {
            target,
            address,
            chain,
        });
        Resume::Apply(Request::here(target, instance, track, Token::None))
    }

    /// Decide a keyword that applies no subschema.
    fn leaf(&mut self, kind: &Kind, instance: &Value) -> Verdict {
        match kind {
            Kind::Content(content) => match instance {
                Value::String(text) => content.check(text).map(|()| None),
                _ => Ok(None),
            },
            Kind::Type(types) => {
                if types.iter().any(|&wanted| has_type(instance, wanted)) {
                    Ok(None)
                } else {
                    let names: Vec<&str> = types.iter().map(|wanted| wanted.name()).collect();
                    Err(format!(
                        "expected {}, found {}",
                        names.join(" or "),
                        type_name(instance)
                    ))
                }
            }
            Kind::Enum(values) => {
                if values.iter().any(|value| equal::equal(value, instance)) {
                    Ok(None)
                } else {
                    Err("the value is not one of the enumerated values".to_owned())
                }
            }
            Kind::Const(value) => {
                if equal::equal(value, instance) {
                    Ok(None)
                } else {
                    Err(format!("the value must be {value}"))
                }
            }
            Kind::MultipleOf(divisor, spelled) => number_check(instance, |value| {
                value
                    .is_multiple_of(divisor)
                    .then_some(())
                    .ok_or_else(|| format!("the number is not a multiple of {spelled}"))
            }),
            Kind::Maximum(bound, spelled) => number_check(instance, |value| {
                (value <= *bound)
                    .then_some(())
                    .ok_or_else(|| format!("the number exceeds the maximum {spelled}"))
            }),
            Kind::ExclusiveMaximum(bound, spelled) => number_check(instance, |value| {
                (value < *bound).then_some(()).ok_or_else(|| {
                    format!("the number is not below the exclusive maximum {spelled}")
                })
            }),
            Kind::Minimum(bound, spelled) => number_check(instance, |value| {
                (value >= *bound)
                    .then_some(())
                    .ok_or_else(|| format!("the number is below the minimum {spelled}"))
            }),
            Kind::ExclusiveMinimum(bound, spelled) => number_check(instance, |value| {
                (value > *bound).then_some(()).ok_or_else(|| {
                    format!("the number is not above the exclusive minimum {spelled}")
                })
            }),
            Kind::MaxLength(limit) => match instance {
                Value::String(text) if text.chars().count() as u64 > *limit => {
                    Err(format!("the string is longer than {limit} characters"))
                }
                _ => Ok(None),
            },
            Kind::MinLength(limit) => match instance {
                Value::String(text) if (text.chars().count() as u64) < *limit => {
                    Err(format!("the string is shorter than {limit} characters"))
                }
                _ => Ok(None),
            },
            Kind::Pattern(pattern) => match instance {
                Value::String(text) if !self.matches(&pattern.regex, text) => Err(format!(
                    "the string does not match the pattern {:?}",
                    pattern.source
                )),
                _ => Ok(None),
            },
            Kind::MaxItems(limit) => match instance {
                Value::Array(items) if items.len() as u64 > *limit => {
                    Err(format!("the array has more than {limit} items"))
                }
                _ => Ok(None),
            },
            Kind::MinItems(limit) => match instance {
                Value::Array(items) if (items.len() as u64) < *limit => {
                    Err(format!("the array has fewer than {limit} items"))
                }
                _ => Ok(None),
            },
            Kind::UniqueItems => match instance {
                Value::Array(items) => match equal::first_duplicate(items) {
                    Some((first, second)) => Err(format!("items {first} and {second} are equal")),
                    None => Ok(None),
                },
                _ => Ok(None),
            },
            Kind::MaxProperties(limit) => match instance {
                Value::Object(map) if map.len() as u64 > *limit => {
                    Err(format!("the object has more than {limit} properties"))
                }
                _ => Ok(None),
            },
            Kind::MinProperties(limit) => match instance {
                Value::Object(map) if (map.len() as u64) < *limit => {
                    Err(format!("the object has fewer than {limit} properties"))
                }
                _ => Ok(None),
            },
            Kind::Required(names) => match instance {
                Value::Object(map) => {
                    let missing: Vec<&str> = names
                        .iter()
                        .filter(|name| !map.contains_key(name.as_str()))
                        .map(String::as_str)
                        .collect();
                    if missing.is_empty() {
                        Ok(None)
                    } else {
                        Err(format!("missing required properties {missing:?}"))
                    }
                }
                _ => Ok(None),
            },
            Kind::DependentRequired(dependencies) => match instance {
                Value::Object(map) => {
                    for (property, required) in dependencies {
                        if map.contains_key(property)
                            && let Some(missing) = required
                                .iter()
                                .find(|name| !map.contains_key(name.as_str()))
                        {
                            return Err(format!(
                                "property {property:?} requires property {missing:?}"
                            ));
                        }
                    }
                    Ok(None)
                }
                _ => Ok(None),
            },
            Kind::Format(name, check) => match (instance, check) {
                (Value::String(text), Some(format)) if !self.format_matches(*format, text) => {
                    Err(format!("the string is not a valid {name}"))
                }
                _ => self.annotate(Value::String(name.clone())),
            },
            Kind::Annotation(value) => self.annotate(value.clone()),
            // An applicator whose instance has a type it does not apply to.
            Kind::Ref(_)
            | Kind::DynamicRef { .. }
            | Kind::RecursiveRef(_)
            | Kind::AllOf(_)
            | Kind::AnyOf(_)
            | Kind::OneOf(_)
            | Kind::Not(_)
            | Kind::If { .. }
            | Kind::DependentSchemas(_)
            | Kind::PrefixItems(_)
            | Kind::Items { .. }
            | Kind::Contains { .. }
            | Kind::Properties(_)
            | Kind::PatternProperties(_)
            | Kind::AdditionalProperties { .. }
            | Kind::PropertyNames(_)
            | Kind::UnevaluatedItems(_)
            | Kind::UnevaluatedProperties(_) => Ok(None),
        }
    }
}

fn number_check(instance: &Value, check: impl FnOnce(Decimal) -> Result<(), String>) -> Verdict {
    match instance {
        Value::Number(number) => check(Decimal::from_number(number)).map(|()| None),
        _ => Ok(None),
    }
}
