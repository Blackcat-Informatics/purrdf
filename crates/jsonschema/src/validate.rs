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
//! evaluation path has entered; `$dynamicRef` (2020-12) and `$recursiveRef`
//! (2019-09) search it outermost first.
//! A `$ref` or `$dynamicRef` that re-enters a subschema it is already
//! evaluating at the same instance location would never terminate, and fails
//! with an error naming the cycle instead.

use serde_json::Value;

use crate::equal;
use crate::number::Decimal;
use crate::output::{Output, OutputUnit};
use crate::pointer;
use crate::schema::{Body, JsonType, Keyword, Kind, Node, NodeId, Schema};

impl Schema {
    /// Whether `instance` is valid — the `flag` output, computed with every
    /// short cut the verdict allows.
    pub fn is_valid(&self, instance: &Value) -> bool {
        Evaluator::new(self, false)
            .node(self.root, instance, false)
            .valid
    }

    /// Evaluate `instance` and record the full output unit tree, from which
    /// [`Output::to_json`] writes any of the standard formats.
    pub fn evaluate(&self, instance: &Value) -> Output {
        let outcome = Evaluator::new(self, true).node(self.root, instance, false);
        let root = outcome.unit.unwrap_or_else(|| OutputUnit {
            valid: outcome.valid,
            keyword_location: String::new(),
            absolute_keyword_location: self.nodes[self.root].location.clone(),
            instance_location: String::new(),
            error: None,
            annotation: None,
            children: Vec::new(),
        });
        Output { root }
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
    unit: Option<OutputUnit>,
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

struct Evaluator<'s> {
    schema: &'s Schema,
    output: bool,
    scope: Vec<usize>,
    /// `(subschema, instance address)` of every reference being followed.
    following: Vec<(NodeId, usize)>,
    keyword_path: String,
    instance_path: String,
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
    const fn new(schema: &'s Schema, output: bool) -> Self {
        Self {
            schema,
            output,
            scope: Vec::new(),
            following: Vec::new(),
            keyword_path: String::new(),
            instance_path: String::new(),
        }
    }

    fn node(&mut self, id: NodeId, instance: &Value, collect: bool) -> Outcome {
        let schema = self.schema;
        let node = &schema.nodes[id];
        let entered = self.scope.last() != Some(&node.resource);
        if entered {
            self.scope.push(node.resource);
        }
        let outcome = match &node.body {
            Body::Bool(true) => Outcome {
                valid: true,
                properties: None,
                items: None,
                unit: self.output.then(|| self.unit(node, true, None, Vec::new())),
            },
            Body::Bool(false) => Outcome {
                valid: false,
                properties: None,
                items: None,
                unit: self.output.then(|| {
                    self.unit(
                        node,
                        false,
                        Some("the false schema accepts nothing".to_owned()),
                        Vec::new(),
                    )
                }),
            },
            Body::Keywords { keywords, tracks } => {
                self.keywords(node, keywords, instance, collect || *tracks)
            }
        };
        if entered {
            self.scope.pop();
        }
        outcome
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

    fn keywords(
        &mut self,
        node: &Node,
        keywords: &[Keyword],
        instance: &Value,
        track: bool,
    ) -> Outcome {
        let mut state = State {
            properties: match instance {
                Value::Object(map) if track => Some(Bits::new(map.len())),
                _ => None,
            },
            items: match instance {
                Value::Array(items) if track => Some(Bits::new(items.len())),
                _ => None,
            },
            units: Vec::new(),
        };
        let mut valid = true;
        for keyword in keywords {
            if !self.keyword(node, keyword, instance, track, &mut state) {
                valid = false;
                if !self.output {
                    break;
                }
            }
        }
        let unit = self
            .output
            .then(|| self.unit(node, valid, None, std::mem::take(&mut state.units)));
        Outcome {
            valid,
            properties: state.properties,
            items: state.items,
            unit,
        }
    }

    /// Evaluate one keyword, recording its unit; returns whether it passed.
    fn keyword(
        &mut self,
        node: &Node,
        keyword: &Keyword,
        instance: &Value,
        track: bool,
        state: &mut State,
    ) -> bool {
        if let Kind::If {
            condition,
            then,
            otherwise,
        } = &keyword.kind
        {
            return self.conditional(node, *condition, *then, *otherwise, instance, track, state);
        }
        let saved = self.keyword_path.len();
        if self.output {
            self.keyword_path.push('/');
            self.keyword_path
                .push_str(&pointer::escape_token(&keyword.name));
        }
        let mut children = Vec::new();
        let verdict = self.verdict(&keyword.kind, instance, track, state, &mut children);
        let valid = verdict.is_ok();
        if self.output {
            state
                .units
                .push(self.keyword_unit(node, &keyword.name, verdict, children));
        }
        self.keyword_path.truncate(saved);
        valid
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
                pointer::fragment_encode(&pointer::escape_token(name))
            ),
            instance_location: self.instance_path.clone(),
            error,
            annotation,
            children,
        }
    }

    /// `if`/`then`/`else` (Core §10.2.2): `if` never fails;
    /// `then` applies when it passed, `else` when it failed. Each is its own
    /// keyword unit.
    #[allow(clippy::too_many_arguments)]
    fn conditional(
        &mut self,
        node: &Node,
        condition: NodeId,
        then: Option<NodeId>,
        otherwise: Option<NodeId>,
        instance: &Value,
        track: bool,
        state: &mut State,
    ) -> bool {
        let saved = self.keyword_path.len();
        if self.output {
            self.keyword_path.push_str("/if");
        }
        let outcome = self.node(condition, instance, track);
        state.absorb(&outcome);
        if self.output {
            let children = outcome.unit.into_iter().collect();
            state
                .units
                .push(self.keyword_unit(node, "if", Ok(None), children));
        }
        self.keyword_path.truncate(saved);
        let (branch, name) = if outcome.valid {
            (then, "then")
        } else {
            (otherwise, "else")
        };
        let Some(branch) = branch else {
            return true;
        };
        if self.output {
            self.keyword_path.push('/');
            self.keyword_path.push_str(name);
        }
        let taken = self.node(branch, instance, track);
        state.absorb(&taken);
        let valid = taken.valid;
        if self.output {
            let verdict = if valid {
                Ok(None)
            } else {
                Err(format!("the `{name}` subschema failed"))
            };
            let children = taken.unit.into_iter().collect();
            state
                .units
                .push(self.keyword_unit(node, name, verdict, children));
        }
        self.keyword_path.truncate(saved);
        valid
    }

    /// An array index as a location token — only when output is on, since
    /// nothing reads a location otherwise and the hot path must not allocate
    /// one per item.
    fn token(&self, index: usize) -> String {
        if self.output {
            index.to_string()
        } else {
            String::new()
        }
    }

    /// Apply subschema `id` to `instance`, which sits at instance token
    /// `instance_token` below the current instance (or at it, for `None`),
    /// with `keyword_tokens` appended to the keyword path.
    fn apply(
        &mut self,
        id: NodeId,
        instance: &Value,
        collect: bool,
        keyword_tokens: &[&str],
        instance_token: Option<&str>,
        children: &mut Vec<OutputUnit>,
    ) -> Outcome {
        let (keyword_saved, instance_saved) = (self.keyword_path.len(), self.instance_path.len());
        if self.output {
            for token in keyword_tokens {
                self.keyword_path.push('/');
                self.keyword_path.push_str(&pointer::escape_token(token));
            }
            if let Some(token) = instance_token {
                self.instance_path.push('/');
                self.instance_path.push_str(&pointer::escape_token(token));
            }
        }
        let mut outcome = self.node(id, instance, collect);
        if let Some(unit) = outcome.unit.take() {
            children.push(unit);
        }
        self.keyword_path.truncate(keyword_saved);
        self.instance_path.truncate(instance_saved);
        outcome
    }

    /// Follow a reference to `target`, refusing to re-enter a subschema at an
    /// instance location it is already evaluating.
    fn follow(
        &mut self,
        target: NodeId,
        instance: &Value,
        track: bool,
        state: &mut State,
        children: &mut Vec<OutputUnit>,
    ) -> Verdict {
        let key = (target, address(instance));
        if self.following.contains(&key) {
            return Err(format!(
                "reference cycle: {} is re-entered at the same instance location without \
                 consuming any of it",
                self.schema.nodes[target].location
            ));
        }
        self.following.push(key);
        let outcome = self.apply(target, instance, track, &[], None, children);
        self.following.pop();
        state.absorb(&outcome);
        if outcome.valid {
            Ok(None)
        } else {
            Err(format!(
                "the referenced schema {} failed",
                self.schema.nodes[target].location
            ))
        }
    }

    #[allow(clippy::too_many_lines)]
    fn verdict(
        &mut self,
        kind: &Kind,
        instance: &Value,
        track: bool,
        state: &mut State,
        children: &mut Vec<OutputUnit>,
    ) -> Verdict {
        let output = self.output;
        let annotate = |value: Value| Ok(output.then_some(value));
        match kind {
            Kind::Ref(target) => self.follow(*target, instance, track, state, children),
            Kind::DynamicRef { target, anchor } => {
                let target = anchor
                    .as_ref()
                    .and_then(|name| {
                        self.scope.iter().find_map(|&resource| {
                            self.schema.resources[resource]
                                .dynamic_anchors
                                .get(name)
                                .copied()
                        })
                    })
                    .unwrap_or(*target);
                self.follow(target, instance, track, state, children)
            }
            Kind::RecursiveRef { target, dynamic } => {
                // 2019-09 Core §8.2.4.2.2: when the target is a
                // `$recursiveAnchor`, the outermost resource in the dynamic
                // scope that is one too is evaluated instead.
                let target = if *dynamic {
                    self.scope
                        .iter()
                        .find_map(|&resource| self.schema.resources[resource].recursive_root)
                        .unwrap_or(*target)
                } else {
                    *target
                };
                self.follow(target, instance, track, state, children)
            }
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
                    .is_multiple_of(*divisor)
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
                Value::String(text) if !pattern.regex.is_match(text) => Err(format!(
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
                (Value::String(text), Some(format)) if !format.check(text) => {
                    Err(format!("the string is not a valid {name}"))
                }
                _ => annotate(Value::String(name.clone())),
            },
            Kind::ContentEncoding(encoding) => match instance {
                Value::String(text) if encoding.decode(text).is_none() => {
                    Err(format!("the string is not valid {encoding:?} content"))
                }
                _ => Ok(None),
            },
            Kind::ContentMediaType { media, encoding } => match instance {
                Value::String(text) => {
                    let decoded = match encoding {
                        // Undecodable content is `contentEncoding`'s failure.
                        Some(encoding) => match encoding.decode(text) {
                            Some(bytes) => bytes,
                            None => return Ok(None),
                        },
                        None => text.as_bytes().to_vec(),
                    };
                    if media.check(&decoded) {
                        Ok(None)
                    } else {
                        Err(format!("the content is not a {media:?} document"))
                    }
                }
                _ => Ok(None),
            },
            Kind::AllOf(schemas) => {
                let mut failed = Vec::new();
                for (index, &schema) in schemas.iter().enumerate() {
                    let outcome = self.apply(
                        schema,
                        instance,
                        track,
                        &[&self.token(index)],
                        None,
                        children,
                    );
                    state.absorb(&outcome);
                    if !outcome.valid {
                        failed.push(index);
                        if !self.output {
                            break;
                        }
                    }
                }
                if failed.is_empty() {
                    Ok(None)
                } else {
                    Err(format!("subschemas {failed:?} failed"))
                }
            }
            Kind::AnyOf(schemas) => {
                let mut any = false;
                for (index, &schema) in schemas.iter().enumerate() {
                    let outcome = self.apply(
                        schema,
                        instance,
                        track,
                        &[&self.token(index)],
                        None,
                        children,
                    );
                    state.absorb(&outcome);
                    any |= outcome.valid;
                    if any && !self.output && !track {
                        break;
                    }
                }
                if any {
                    Ok(None)
                } else {
                    Err("no subschema passed".to_owned())
                }
            }
            Kind::OneOf(schemas) => {
                let mut passed = Vec::new();
                let mut outcomes = Vec::new();
                for (index, &schema) in schemas.iter().enumerate() {
                    let outcome = self.apply(
                        schema,
                        instance,
                        track,
                        &[&self.token(index)],
                        None,
                        children,
                    );
                    if outcome.valid {
                        passed.push(index);
                        outcomes.push(outcome);
                        if passed.len() > 1 && !self.output {
                            break;
                        }
                    }
                }
                match passed.len() {
                    1 => {
                        state.absorb(&outcomes[0]);
                        Ok(None)
                    }
                    0 => Err("no subschema passed".to_owned()),
                    _ => Err(format!("more than one subschema passed: {passed:?}")),
                }
            }
            Kind::Not(schema) => {
                let outcome = self.apply(*schema, instance, false, &[], None, children);
                if outcome.valid {
                    Err("the value must not be valid against the subschema".to_owned())
                } else {
                    Ok(None)
                }
            }
            Kind::If { .. } => Ok(None),
            Kind::DependentSchemas(dependencies) => {
                let Value::Object(map) = instance else {
                    return Ok(None);
                };
                let mut failed = Vec::new();
                for (property, schema) in dependencies {
                    if !map.contains_key(property) {
                        continue;
                    }
                    let outcome = self.apply(*schema, instance, track, &[property], None, children);
                    state.absorb(&outcome);
                    if !outcome.valid {
                        failed.push(property.as_str());
                        if !self.output {
                            break;
                        }
                    }
                }
                if failed.is_empty() {
                    Ok(None)
                } else {
                    Err(format!("the dependent schemas of {failed:?} failed"))
                }
            }
            Kind::PrefixItems(schemas) => {
                let Value::Array(items) = instance else {
                    return Ok(None);
                };
                let applied = items.len().min(schemas.len());
                let mut failed = Vec::new();
                for index in 0..applied {
                    let token = self.token(index);
                    let outcome = self.apply(
                        schemas[index],
                        &items[index],
                        false,
                        &[&token],
                        Some(&token),
                        children,
                    );
                    if let Some(evaluated) = &mut state.items {
                        evaluated.set(index);
                    }
                    if !outcome.valid {
                        failed.push(index);
                        if !self.output {
                            break;
                        }
                    }
                }
                if !failed.is_empty() {
                    return Err(format!("items {failed:?} failed their prefix schemas"));
                }
                if applied == 0 {
                    return Ok(None);
                }
                annotate(if applied == items.len() {
                    Value::Bool(true)
                } else {
                    Value::from(applied - 1)
                })
            }
            Kind::Items { schema, skip } => {
                let Value::Array(items) = instance else {
                    return Ok(None);
                };
                let mut failed = Vec::new();
                for (index, item) in items.iter().enumerate().skip(*skip) {
                    let token = self.token(index);
                    let outcome = self.apply(*schema, item, false, &[], Some(&token), children);
                    if let Some(evaluated) = &mut state.items {
                        evaluated.set(index);
                    }
                    if !outcome.valid {
                        failed.push(index);
                        if !self.output {
                            break;
                        }
                    }
                }
                if !failed.is_empty() {
                    return Err(format!("items {failed:?} failed the items schema"));
                }
                if items.len() > *skip {
                    annotate(Value::Bool(true))
                } else {
                    Ok(None)
                }
            }
            Kind::Contains {
                schema,
                min,
                max,
                annotates,
            } => {
                let Value::Array(items) = instance else {
                    return Ok(None);
                };
                let mut matched = Vec::new();
                let decided_early = !self.output && !track && max.is_none();
                for (index, item) in items.iter().enumerate() {
                    if decided_early && matched.len() as u64 >= *min {
                        break;
                    }
                    let token = self.token(index);
                    let outcome = self.apply(*schema, item, false, &[], Some(&token), children);
                    if outcome.valid {
                        matched.push(index);
                        if *annotates && let Some(evaluated) = &mut state.items {
                            evaluated.set(index);
                        }
                    }
                }
                let found = matched.len() as u64;
                if found < *min {
                    return Err(format!(
                        "{found} items match the contains schema; at least {min} must"
                    ));
                }
                if let Some(max) = max
                    && found > *max
                {
                    return Err(format!(
                        "{found} items match the contains schema; at most {max} may"
                    ));
                }
                if !annotates {
                    return Ok(None);
                }
                annotate(if matched.len() == items.len() {
                    Value::Bool(true)
                } else {
                    Value::from(matched)
                })
            }
            Kind::Properties(schemas) => {
                let Value::Object(map) = instance else {
                    return Ok(None);
                };
                let mut applied = Vec::new();
                let mut failed = Vec::new();
                for (position, (name, value)) in map.iter().enumerate() {
                    let Some(&schema) = schemas.get(name) else {
                        continue;
                    };
                    let outcome = self.apply(schema, value, false, &[name], Some(name), children);
                    if let Some(evaluated) = &mut state.properties {
                        evaluated.set(position);
                    }
                    if output {
                        applied.push(Value::String(name.clone()));
                    }
                    if !outcome.valid {
                        failed.push(name.as_str());
                        if !self.output {
                            break;
                        }
                    }
                }
                if failed.is_empty() {
                    annotate(Value::Array(applied))
                } else {
                    Err(format!("properties {failed:?} failed their schemas"))
                }
            }
            Kind::PatternProperties(patterns) => {
                let Value::Object(map) = instance else {
                    return Ok(None);
                };
                let mut applied = Vec::new();
                let mut failed = Vec::new();
                'properties: for (position, (name, value)) in map.iter().enumerate() {
                    let mut matched = false;
                    for (pattern, schema) in patterns {
                        if !pattern.regex.is_match(name) {
                            continue;
                        }
                        matched = true;
                        let outcome = self.apply(
                            *schema,
                            value,
                            false,
                            &[&pattern.source],
                            Some(name),
                            children,
                        );
                        if !outcome.valid {
                            failed.push(name.as_str());
                            if !self.output {
                                break 'properties;
                            }
                        }
                    }
                    if matched {
                        if let Some(evaluated) = &mut state.properties {
                            evaluated.set(position);
                        }
                        if output {
                            applied.push(Value::String(name.clone()));
                        }
                    }
                }
                if failed.is_empty() {
                    annotate(Value::Array(applied))
                } else {
                    Err(format!(
                        "properties {failed:?} failed their pattern schemas"
                    ))
                }
            }
            Kind::AdditionalProperties {
                schema,
                properties,
                patterns,
            } => {
                let Value::Object(map) = instance else {
                    return Ok(None);
                };
                let mut applied = Vec::new();
                let mut failed = Vec::new();
                for (position, (name, value)) in map.iter().enumerate() {
                    if properties.iter().any(|known| known == name)
                        || patterns.iter().any(|pattern| pattern.is_match(name))
                    {
                        continue;
                    }
                    let outcome = self.apply(*schema, value, false, &[], Some(name), children);
                    if let Some(evaluated) = &mut state.properties {
                        evaluated.set(position);
                    }
                    if output {
                        applied.push(Value::String(name.clone()));
                    }
                    if !outcome.valid {
                        failed.push(name.as_str());
                        if !self.output {
                            break;
                        }
                    }
                }
                if failed.is_empty() {
                    annotate(Value::Array(applied))
                } else {
                    Err(format!("additional properties {failed:?} are not allowed"))
                }
            }
            Kind::PropertyNames(schema) => {
                let Value::Object(map) = instance else {
                    return Ok(None);
                };
                let mut failed = Vec::new();
                for name in map.keys() {
                    let key = Value::String(name.clone());
                    let outcome = self.apply(*schema, &key, false, &[], Some(name), children);
                    if !outcome.valid {
                        failed.push(name.as_str());
                        if !self.output {
                            break;
                        }
                    }
                }
                if failed.is_empty() {
                    Ok(None)
                } else {
                    Err(format!("property names {failed:?} are not valid"))
                }
            }
            Kind::UnevaluatedItems(schema) => {
                let Value::Array(items) = instance else {
                    return Ok(None);
                };
                let mut failed = Vec::new();
                let mut any = false;
                for (index, item) in items.iter().enumerate() {
                    if state
                        .items
                        .as_ref()
                        .is_some_and(|evaluated| evaluated.get(index))
                    {
                        continue;
                    }
                    any = true;
                    let token = self.token(index);
                    let outcome = self.apply(*schema, item, false, &[], Some(&token), children);
                    if !outcome.valid {
                        failed.push(index);
                        if !self.output {
                            break;
                        }
                    }
                }
                if let Some(evaluated) = &mut state.items {
                    evaluated.fill(items.len());
                }
                if !failed.is_empty() {
                    return Err(format!("unevaluated items {failed:?} are not allowed"));
                }
                if any {
                    annotate(Value::Bool(true))
                } else {
                    Ok(None)
                }
            }
            Kind::UnevaluatedProperties(schema) => {
                let Value::Object(map) = instance else {
                    return Ok(None);
                };
                let mut applied = Vec::new();
                let mut failed = Vec::new();
                for (position, (name, value)) in map.iter().enumerate() {
                    if state
                        .properties
                        .as_ref()
                        .is_some_and(|evaluated| evaluated.get(position))
                    {
                        continue;
                    }
                    let outcome = self.apply(*schema, value, false, &[], Some(name), children);
                    if output {
                        applied.push(Value::String(name.clone()));
                    }
                    if !outcome.valid {
                        failed.push(name.as_str());
                        if !self.output {
                            break;
                        }
                    }
                }
                if let Some(evaluated) = &mut state.properties {
                    evaluated.fill(map.len());
                }
                if failed.is_empty() {
                    annotate(Value::Array(applied))
                } else {
                    Err(format!("unevaluated properties {failed:?} are not allowed"))
                }
            }
            Kind::Annotation(value) => annotate(value.clone()),
        }
    }
}

fn number_check(instance: &Value, check: impl FnOnce(Decimal) -> Result<(), String>) -> Verdict {
    match instance {
        Value::Number(number) => check(Decimal::from_number(number)).map(|()| None),
        _ => Ok(None),
    }
}
