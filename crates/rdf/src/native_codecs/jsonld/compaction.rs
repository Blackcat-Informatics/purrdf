// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! JSON-LD 1.1 §§6.1–6.3, one iterative document/value compaction body.

use super::{
    CompiledJsonLdContext, JsonLdContainer as C, JsonLdDirection, JsonLdNullable,
    JsonLdTermDefinition, JsonLdTypeMapping, RdfDiagnostic, decode,
};
use purrdf_lex::allocation::{Ceiling, Memory};
use purrdf_lex::json::{Object, Value};
use purrdf_lex::walk::WorkList;
use std::ops::Deref;
use std::rc::Rc;

/// Output-shape and document-relative IRI selection, independent of contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonLdCompactionPolicy {
    /// Collapse eligible singleton collections; list and grammar positions stay intact.
    pub compact_arrays: bool,
    /// Allow document-position references relative to the compiled output base.
    pub compact_to_relative: bool,
}
impl JsonLdCompactionPolicy {
    /// Both JSON-LD API defaults enabled.
    pub const DEFAULT: Self = Self {
        compact_arrays: true,
        compact_to_relative: true,
    };
}
impl Default for JsonLdCompactionPolicy {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Fresh document-construction authority, separate from decoding and expansion.
///
/// Admits output, retained context copies, frames and compactor scratch. Borrowed
/// input and initial contexts are not recharged. Context compilation, scope state
/// and IRI/inverse-selection scratch retain their existing separate semantic limits;
/// this is neither a total-operation memory cap nor context-state admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonLdCompactionLimits {
    /// Maximum live requested layouts owned by this construction operation.
    pub max_allocation_bytes: u64,
}
impl JsonLdCompactionLimits {
    /// Explicit 32 GiB document-construction allowance.
    pub const DEFAULT: Self = Self {
        max_allocation_bytes: 32 * 1024 * 1024 * 1024,
    };
}
impl Default for JsonLdCompactionLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Clone)]
pub(super) enum Context<'a> {
    Borrowed(&'a CompiledJsonLdContext),
    Scoped(Rc<CompiledJsonLdContext>),
}
impl Deref for Context<'_> {
    type Target = CompiledJsonLdContext;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(value) => value,
            Self::Scoped(value) => value,
        }
    }
}
type Members = Vec<(String, Value)>;
#[derive(Clone, Copy)]
enum Shape {
    Policy,
    Array,
}
struct Request<'a> {
    raw: &'a Value,
    property: Option<String>,
    shape: Shape,
    component: bool,
    node_only: bool,
    keep_map_metadata: bool,
}
struct Edge<'a> {
    raw: &'a Value,
    property: String,
    nest: Option<String>,
    action: Action,
    force: bool,
}
#[derive(Clone, Copy)]
enum Action {
    Add,
    Direct,
    Nodes,
    Reverse,
    List,
    ListObject,
    Graph,
    Map(C),
    Json,
    Explicit,
}
struct Plan<'a> {
    context: Context<'a>,
    members: Members,
    edges: Vec<Edge<'a>>,
}
enum Task<'a> {
    Visit(Request<'a>, Context<'a>),
    Array { count: usize, collapse: bool },
    Object(Plan<'a>),
}

/// Compact an expanded lexical JSON value with an immutable output context.
///
/// The compact operation's root finishing is applied exactly once. Flattened
/// expanded arrays compose directly with this function, without another envelope.
/// # Errors
/// Returns malformed expanded-shape, context/IRI or construction refusals.
pub fn compact_expanded_with_context(
    input: &Value,
    context: &CompiledJsonLdContext,
    policy: &JsonLdCompactionPolicy,
    limits: &JsonLdCompactionLimits,
) -> Result<Value, RdfDiagnostic> {
    run(input, context, *policy, *limits, true)
}

pub(super) fn compact_node(
    input: &Value,
    context: &CompiledJsonLdContext,
    policy: JsonLdCompactionPolicy,
) -> Result<Value, RdfDiagnostic> {
    run(
        input,
        context,
        policy,
        JsonLdCompactionLimits::DEFAULT,
        false,
    )
}
fn run(
    input: &Value,
    context: &CompiledJsonLdContext,
    policy: JsonLdCompactionPolicy,
    limits: JsonLdCompactionLimits,
    root: bool,
) -> Result<Value, RdfDiagnostic> {
    let mut ceiling = Ceiling(usize::try_from(limits.max_allocation_bytes).unwrap_or(usize::MAX));
    Memory::new(&mut ceiling).try_scope(|memory| {
        let mut walk = Walk {
            memory,
            policy,
            tasks: WorkList::new(),
            results: Vec::new(),
        };
        walk.push(Task::Visit(
            Request {
                raw: input,
                property: None,
                shape: Shape::Policy,
                component: false,
                node_only: root,
                keep_map_metadata: false,
            },
            Context::Borrowed(context),
        ))?;
        while let Some(task) = walk.tasks.pop() {
            walk.step(task)?;
        }
        // Every scheduled visit produces one result, consumed by its continuation.
        let mut output = walk.results.pop().expect("one compacted root");
        walk.memory.release_vec(std::mem::take(&mut walk.results))?;
        std::mem::replace(&mut walk.tasks, WorkList::new()).release_admitted(walk.memory)?;
        if root {
            output = match &mut output {
                Value::Null => Value::Object(Object::new()),
                Value::Array(values) if values.is_empty() => {
                    walk.memory.release_vec(std::mem::take(values))?;
                    Value::Object(Object::new())
                }
                Value::Array(values) => {
                    let mut members = Vec::new();
                    let key = context.compact_iri("@graph", true)?;
                    walk.put(
                        &mut members,
                        &key,
                        Value::Array(std::mem::take(values)),
                        false,
                    )?;
                    walk.object(members)
                }
                Value::Object(_) => output,
                _ => {
                    return Err(decode(
                        "expanded document root must contain nodes, not a scalar",
                    ));
                }
            };
            if !context.canonical_context().is_null() {
                let retained = context.canonical_context().clone_with_memory(walk.memory)?;
                let object = output
                    .as_object_mut()
                    .expect("root finish produces an object");
                let mut members = std::mem::take(object).into_members();
                walk.put(&mut members, "@context", retained, false)?;
                output = walk.object(members);
            }
        }
        Ok(output)
    })
}
struct Walk<'a, 'm, 's> {
    memory: &'m mut Memory<'s, Ceiling>,
    policy: JsonLdCompactionPolicy,
    tasks: WorkList<Task<'a>, 8>,
    results: Vec<Value>,
}
impl<'a> Walk<'a, '_, '_> {
    fn push(&mut self, task: Task<'a>) -> Result<(), RdfDiagnostic> {
        self.tasks
            .try_push_admitted(task, self.memory)
            .map_err(Into::into)
    }
    fn result(&mut self, value: Value) -> Result<(), RdfDiagnostic> {
        self.memory
            .push(&mut self.results, value)
            .map_err(Into::into)
    }
    fn object(&self, mut members: Members) -> Value {
        members.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        Value::Object(Object::from(members))
    }
    fn copy(&mut self, value: &Value) -> Result<Value, RdfDiagnostic> {
        value.clone_with_memory(self.memory).map_err(Into::into)
    }
    fn string(&mut self, value: &str) -> Result<Value, RdfDiagnostic> {
        Ok(Value::String(self.memory.string(value)?))
    }
    fn iri(
        &mut self,
        context: &CompiledJsonLdContext,
        text: &str,
        vocab: bool,
    ) -> Result<Value, RdfDiagnostic> {
        let selected =
            context.compact_document_iri(text, vocab, self.policy.compact_to_relative)?;
        self.string(&selected)
    }
    fn discard(&mut self, mut value: Value) -> Result<(), RdfDiagnostic> {
        value.release_with_memory(self.memory).map_err(Into::into)
    }
    fn property_done(&mut self, property: Option<String>) -> Result<(), RdfDiagnostic> {
        if let Some(property) = property {
            self.memory.release_string(property)?;
        }
        Ok(())
    }
    fn put(
        &mut self,
        members: &mut Members,
        key: &str,
        value: Value,
        append: bool,
    ) -> Result<(), RdfDiagnostic> {
        if let Some(old) = purrdf_lex::assoc::get_mut(members, key) {
            if !append {
                return Err(decode(format!(
                    "compaction produced colliding member `{key}`"
                )));
            }
            self.add(old, value)?;
        } else {
            let key = self.memory.string(key)?;
            self.memory.push(members, (key, value))?;
        }
        Ok(())
    }
    fn array(&mut self, value: Value) -> Result<Value, RdfDiagnostic> {
        if value.as_array().is_some() {
            return Ok(value);
        }
        let mut values = Vec::new();
        self.memory.push(&mut values, value)?;
        Ok(Value::Array(values))
    }
    fn add(&mut self, target: &mut Value, mut value: Value) -> Result<(), RdfDiagnostic> {
        if target.as_array().is_none() {
            *target = self.array(target.take())?;
        }
        let values = target.as_array_mut().expect("array normalized above");
        if let Value::Array(items) = &mut value {
            self.memory.extend(values, items.drain(..))?;
            self.memory.release_vec(std::mem::take(items))?;
        } else {
            self.memory.push(values, value)?;
        }
        Ok(())
    }
    fn collapse(&mut self, mut value: Value, collapse: bool) -> Result<Value, RdfDiagnostic> {
        if collapse
            && let Value::Array(values) = &mut value
            && values.len() == 1
        {
            let only = values.pop().expect("singleton length checked");
            self.memory.release_vec(std::mem::take(values))?;
            return Ok(only);
        }
        Ok(value)
    }
    fn step(&mut self, task: Task<'a>) -> Result<(), RdfDiagnostic> {
        match task {
            Task::Visit(request, context) => self.visit(request, context),
            Task::Array { count, collapse } => {
                let mut values = Vec::new();
                self.memory.reserve(&mut values, count)?;
                for _ in 0..count {
                    let value = self.results.pop().expect("one result per array child");
                    if !value.is_null() {
                        values.push(value);
                    }
                }
                let value = self.collapse(Value::Array(values), collapse)?;
                self.result(value)
            }
            Task::Object(mut plan) => {
                for edge in plan.edges.drain(..) {
                    let value = self.results.pop().expect("one result per object edge");
                    self.finish_edge(&plan.context, &mut plan.members, &edge, value)?;
                    self.memory.release_string(edge.property)?;
                    if let Some(nest) = edge.nest {
                        self.memory.release_string(nest)?;
                    }
                }
                self.memory.release_vec(plan.edges)?;
                let value = self.object(plan.members);
                self.result(value)
            }
        }
    }
    fn visit(
        &mut self,
        request: Request<'a>,
        mut context: Context<'a>,
    ) -> Result<(), RdfDiagnostic> {
        match request.raw {
            Value::Array(items) => {
                if items.iter().any(|item| item.as_array().is_some()) {
                    return Err(decode("expanded arrays cannot contain raw arrays"));
                }
                let definition = request
                    .property
                    .as_deref()
                    .and_then(|property| context.term(property));
                let collapse = match request.shape {
                    Shape::Array => false,
                    Shape::Policy => {
                        self.policy.compact_arrays
                            && request
                                .property
                                .as_deref()
                                .is_none_or(|property| property != "@graph" && property != "@set")
                            && definition.is_none_or(|definition| {
                                !definition.containers().contains(&C::List)
                                    && !definition.containers().contains(&C::Set)
                            })
                    }
                };
                self.push(Task::Array {
                    count: items.len(),
                    collapse,
                })?;
                // Forward scheduling makes the result stack pop in source order.
                for raw in items {
                    let property = request
                        .property
                        .as_deref()
                        .map(|property| self.memory.string(property))
                        .transpose()?;
                    self.push(Task::Visit(
                        Request {
                            raw,
                            property,
                            shape: Shape::Policy,
                            component: request.component,
                            node_only: request.node_only,
                            keep_map_metadata: request.keep_map_metadata,
                        },
                        context.clone(),
                    ))?;
                }
                self.property_done(request.property)
            }
            Value::Object(object) => {
                validate(object, request.property.as_deref(), request.component)?;
                if request.node_only
                    && ["@value", "@list", "@set", "@triple"]
                        .iter()
                        .any(|key| object.contains_key(key))
                {
                    return Err(decode(
                        "expanded document/graph entries must be node objects",
                    ));
                }
                let type_context = context.clone();
                if !(object.contains_key("@value")
                    || object.len() == 1 && object.contains_key("@id"))
                    && let Some(previous) = context.previous_context()
                {
                    context = Context::Scoped(Rc::new(previous));
                }
                if let Some(definition) = request
                    .property
                    .as_deref()
                    .and_then(|property| context.term(property))
                    .filter(|definition| definition.scoped_context().is_some())
                {
                    context = Context::Scoped(Rc::new(context.apply_property_scope(definition)?));
                }
                if !request.component
                    && !request.keep_map_metadata
                    && let Some(scalar) =
                        self.scalar(object, &context, request.property.as_deref())?
                {
                    self.property_done(request.property)?;
                    return self.result(scalar);
                }
                if let Some(list) = object.get("@list")
                    && request
                        .property
                        .as_deref()
                        .and_then(|property| context.term(property))
                        .is_some_and(|definition| definition.containers().contains(&C::List))
                    && !object.contains_key("@annotation")
                    && !object.contains_key("@index")
                {
                    self.push(Task::Visit(
                        Request {
                            raw: list,
                            property: request.property,
                            shape: Shape::Array,
                            component: false,
                            node_only: false,
                            keep_map_metadata: request.keep_map_metadata,
                        },
                        context,
                    ))?;
                    return Ok(());
                }
                if let Some(set) = object.get("@set") {
                    self.push(Task::Visit(
                        Request {
                            raw: set,
                            property: request.property,
                            shape: Shape::Policy,
                            component: false,
                            node_only: false,
                            keep_map_metadata: request.keep_map_metadata,
                        },
                        context,
                    ))?;
                    return Ok(());
                }
                if !object.contains_key("@value")
                    && let Some(types) = object.get("@type").and_then(Value::as_array)
                {
                    let mut terms = Vec::new();
                    self.memory.reserve(&mut terms, types.len())?;
                    for datatype in types {
                        let iri = datatype.as_str().expect("validated node type");
                        let term = type_context.compact_iri(iri, true)?;
                        let term = self.memory.string(&term)?;
                        self.memory.push(&mut terms, term)?;
                    }
                    terms.sort_unstable();
                    terms.reverse();
                    while let Some(term) = terms.pop() {
                        if let Some(definition) = type_context
                            .term(&term)
                            .filter(|definition| definition.scoped_context().is_some())
                        {
                            context = Context::Scoped(Rc::new(
                                context.apply_type_scope(definition, false)?,
                            ));
                        }
                        self.memory.release_string(term)?;
                    }
                    self.memory.release_vec(terms)?;
                }
                let mut plan = Plan {
                    context,
                    members: Vec::new(),
                    edges: Vec::new(),
                };
                let mut entries = Vec::new();
                self.memory.reserve(&mut entries, object.len())?;
                entries.extend(object.iter());
                entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
                for (key, raw) in entries.iter().copied() {
                    match key.as_str() {
                        "@id" | "@predicate" => {
                            let value = self.iri(
                                &plan.context,
                                raw.as_str().expect("validated identifier"),
                                key == "@predicate",
                            )?;
                            let alias = plan.context.compact_iri(key, true)?;
                            self.put(&mut plan.members, &alias, value, false)?;
                        }
                        "@type" => {
                            let mut value = if let Some(iri) = raw.as_str() {
                                self.iri(&type_context, iri, true)?
                            } else {
                                let mut types = Vec::new();
                                for datatype in raw.as_array().expect("validated node types") {
                                    let value = self.iri(
                                        &type_context,
                                        datatype.as_str().expect("validated type"),
                                        true,
                                    )?;
                                    self.memory.push(&mut types, value)?;
                                }
                                Value::Array(types)
                            };
                            let alias = plan.context.compact_iri(key, true)?;
                            let force = !self.policy.compact_arrays
                                || plan.context.term(&alias).is_some_and(|definition| {
                                    definition.containers().contains(&C::Set)
                                });
                            if raw.as_array().is_some() {
                                value = self.collapse(value, !force)?;
                            }
                            self.put(&mut plan.members, &alias, value, false)?;
                        }
                        "@index"
                            if !request.keep_map_metadata
                                && request
                                    .property
                                    .as_deref()
                                    .and_then(|property| plan.context.term(property))
                                    .is_some_and(|definition| {
                                        definition.containers().contains(&C::Index)
                                            && definition.index_mapping().is_none()
                                    }) => {}
                        "@value" | "@index" | "@language" | "@direction" => {
                            let alias = plan.context.compact_iri(key, true)?;
                            let value = self.copy(raw)?;
                            self.put(&mut plan.members, &alias, value, false)?;
                        }
                        "@reverse" | "@triple" | "@subject" | "@object" | "@annotation"
                        | "@graph" | "@included" | "@list" => {
                            let alias = plan.context.compact_iri(key, true)?;
                            let action = if key == "@reverse" {
                                Action::Reverse
                            } else if matches!(key.as_str(), "@graph" | "@included" | "@annotation")
                            {
                                Action::Nodes
                            } else {
                                Action::Direct
                            };
                            self.edge(
                                &mut plan,
                                raw,
                                &alias,
                                None,
                                action,
                                key == "@graph" || key == "@list",
                            )?;
                        }
                        _ => {
                            let items = raw.as_array().expect("validated expanded property array");
                            if items.is_empty() {
                                let alias = plan.context.compact_property_iri(
                                    key,
                                    raw,
                                    request.property.as_deref() == Some("@reverse"),
                                    false,
                                    &[],
                                )?;
                                let nest = plan
                                    .context
                                    .term(&alias)
                                    .and_then(|definition| definition.nest());
                                self.insert_property(
                                    &mut plan.members,
                                    &alias,
                                    nest,
                                    Value::Array(Vec::new()),
                                    true,
                                )?;
                            }
                            let json_eligible = self.policy.compact_arrays
                                && items.len() == 1
                                && items[0].get("@type").and_then(Value::as_str) == Some("@json")
                                && items[0].get("@index").is_none()
                                && !items[0]
                                    .as_object()
                                    .is_some_and(|item| item.contains_key("@annotation"));
                            let mut used_lists = Vec::new();
                            let mut explicit_terms = Vec::new();
                            let first_edge = plan.edges.len();
                            for item in items {
                                let alias = plan.context.compact_property_iri(
                                    key,
                                    item,
                                    request.property.as_deref() == Some("@reverse"),
                                    json_eligible,
                                    &used_lists,
                                )?;
                                let selection_context = plan.context.clone();
                                let definition = selection_context.term(&alias);
                                if item.get("@list").is_some()
                                    && definition.is_some_and(|definition| {
                                        definition.containers().contains(&C::List)
                                    })
                                {
                                    self.group_key(&mut used_lists, &alias)?;
                                }
                                let nest = definition.and_then(|definition| definition.nest());
                                let explicit = selection_context
                                    .property_uses_explicit_values(&alias, item)?;
                                if explicit {
                                    self.group_key(&mut explicit_terms, &alias)?;
                                    for edge in &mut plan.edges[first_edge..] {
                                        if edge.property == alias {
                                            edge.action = Action::Explicit;
                                            edge.force = true;
                                        }
                                    }
                                }
                                let explicit = explicit_terms.binary_search(&alias).is_ok();
                                let force = !self.policy.compact_arrays
                                    || definition.is_some_and(|definition| {
                                        definition.containers().contains(&C::Set)
                                    });
                                let action = if explicit {
                                    Action::Explicit
                                } else if item.get("@graph").is_some() {
                                    Action::Graph
                                } else if item.get("@list").is_some()
                                    && definition.is_some_and(|definition| {
                                        definition.containers().contains(&C::List)
                                    })
                                    && (item.get("@index").is_some()
                                        || item.get("@annotation").is_some())
                                {
                                    Action::ListObject
                                } else if item.get("@annotation").is_some() {
                                    Action::Add
                                } else if item.get("@list").is_some() {
                                    Action::List
                                } else if definition.is_some_and(|definition| {
                                    definition.type_mapping() == Some(&JsonLdTypeMapping::Json)
                                }) && json_eligible
                                {
                                    Action::Json
                                } else if let Some(container) = definition.and_then(|definition| {
                                    [C::Language, C::Index, C::Id, C::Type].into_iter().find(
                                        |container| definition.containers().contains(container),
                                    )
                                }) {
                                    Action::Map(container)
                                } else {
                                    Action::Add
                                };
                                self.edge(&mut plan, item, &alias, nest, action, force)?;
                            }
                            while let Some(term) = used_lists.pop() {
                                self.memory.release_string(term)?;
                            }
                            self.memory.release_vec(used_lists)?;
                            while let Some(term) = explicit_terms.pop() {
                                self.memory.release_string(term)?;
                            }
                            self.memory.release_vec(explicit_terms)?;
                        }
                    }
                }
                self.memory.release_vec(entries)?;
                self.property_done(request.property)?;
                // Each edge stays in its parent's admitted frame until its child returns.
                for edge in &plan.edges {
                    let (raw, shape) = match edge.action {
                        Action::List => (edge.raw.get("@list").expect("list edge"), Shape::Array),
                        Action::Graph => (edge.raw, Shape::Policy),
                        _ => (
                            edge.raw,
                            if edge.force && matches!(edge.action, Action::Direct | Action::Nodes) {
                                Shape::Array
                            } else {
                                Shape::Policy
                            },
                        ),
                    };
                    let property = Some(self.memory.string(
                        if matches!(edge.action, Action::Reverse) {
                            "@reverse"
                        } else {
                            &edge.property
                        },
                    )?);
                    let component = edge.property == "@subject"
                        || edge.property == "@object"
                        || matches!(edge.action, Action::Map(C::Id));
                    self.push(Task::Visit(
                        Request {
                            raw,
                            property,
                            shape,
                            component,
                            node_only: matches!(edge.action, Action::Nodes),
                            keep_map_metadata: matches!(edge.action, Action::Explicit)
                                || plan
                                    .context
                                    .property_keeps_index(&edge.property, edge.raw)?,
                        },
                        plan.context.clone(),
                    ))?;
                }
                // The finish must run after its children, despite being planned last.
                // WorkList is LIFO, so insert the parent beneath the just scheduled frames.
                let count = plan.edges.len();
                self.push_parent(plan, count)
            }
            Value::Null => {
                self.property_done(request.property)?;
                self.result(Value::Null)
            }
            _ => {
                self.property_done(request.property)?;
                Err(decode(
                    "expanded values must be objects, not unwrapped scalars",
                ))
            }
        }
    }
    fn push_parent(&mut self, plan: Plan<'a>, count: usize) -> Result<(), RdfDiagnostic> {
        // A bounded admitted staging vector rotates only the newly scheduled children.
        let mut children = Vec::new();
        self.memory.reserve(&mut children, count)?;
        for _ in 0..count {
            children.push(self.tasks.pop().expect("scheduled child count"));
        }
        self.push(Task::Object(plan))?;
        while let Some(child) = children.pop() {
            self.push(child)?;
        }
        self.memory.release_vec(children)?;
        Ok(())
    }
    fn group_key(&mut self, keys: &mut Vec<String>, key: &str) -> Result<(), RdfDiagnostic> {
        let Err(at) = keys.binary_search_by(|old| old.as_str().cmp(key)) else {
            return Ok(());
        };
        let owned = self.memory.string(key)?;
        self.memory.reserve_for_push(keys)?;
        keys.insert(at, owned);
        Ok(())
    }
    fn edge(
        &mut self,
        plan: &mut Plan<'a>,
        raw: &'a Value,
        property: &str,
        nest: Option<&str>,
        action: Action,
        force: bool,
    ) -> Result<(), RdfDiagnostic> {
        let property = self.memory.string(property)?;
        let nest = nest.map(|nest| self.memory.string(nest)).transpose()?;
        self.memory.push(
            &mut plan.edges,
            Edge {
                raw,
                property,
                nest,
                action,
                force,
            },
        )?;
        Ok(())
    }
    fn scalar(
        &mut self,
        object: &Object,
        context: &CompiledJsonLdContext,
        property: Option<&str>,
    ) -> Result<Option<Value>, RdfDiagnostic> {
        if object.contains_key("@annotation") {
            return Ok(None);
        }
        let definition = property.and_then(|property| context.term(property));
        let indexed = object.contains_key("@index");
        if indexed
            && definition.is_none_or(|definition| {
                !definition.containers().contains(&C::Index) || definition.index_mapping().is_some()
            })
        {
            return Ok(None);
        }
        if let Some(id) = object.get("@id").and_then(Value::as_str)
            && object.len() == 1 + usize::from(indexed)
        {
            return self.scalar_id(id, context, property);
        }
        let Some(value) = object.get("@value") else {
            return Ok(None);
        };
        let datatype = object.get("@type").and_then(Value::as_str);
        let mapping = definition.and_then(|definition| definition.type_mapping());
        if let Some(datatype) = datatype {
            if matches!(
                (datatype, mapping),
                ("@json", Some(JsonLdTypeMapping::Json))
            ) || matches!(mapping,Some(JsonLdTypeMapping::Datatype(iri)) if iri==datatype)
            {
                return self.copy(value).map(Some);
            }
            return Ok(None);
        }
        if mapping.is_some() {
            return Ok(None);
        }
        if value.as_str().is_none() {
            return self.copy(value).map(Some);
        }
        let language = match definition.and_then(JsonLdTermDefinition::language_mapping) {
            Some(JsonLdNullable::Null) => None,
            Some(JsonLdNullable::Value(value)) => Some(value),
            None => context.default_language(),
        };
        let direction = match definition.and_then(JsonLdTermDefinition::direction_mapping) {
            Some(JsonLdNullable::Null) => None,
            Some(JsonLdNullable::Value(value)) => Some(value.as_str()),
            None => context.default_direction().map(JsonLdDirection::as_str),
        };
        let actual = object.get("@language").and_then(Value::as_str);
        if language.map(purrdf_iri::langtag::identity_fold)
            == actual.map(purrdf_iri::langtag::identity_fold)
            && direction == object.get("@direction").and_then(Value::as_str)
        {
            return self.copy(value).map(Some);
        }
        Ok(None)
    }
    fn scalar_id(
        &mut self,
        id: &str,
        context: &CompiledJsonLdContext,
        property: Option<&str>,
    ) -> Result<Option<Value>, RdfDiagnostic> {
        let mapping = property
            .and_then(|property| context.term(property))
            .and_then(|definition| definition.type_mapping());
        let vocab = match mapping {
            Some(JsonLdTypeMapping::Id) => false,
            Some(JsonLdTypeMapping::Vocab) => true,
            _ => return Ok(None),
        };
        self.iri(context, id, vocab).map(Some)
    }
    fn insert_property(
        &mut self,
        members: &mut Members,
        property: &str,
        nest: Option<&str>,
        value: Value,
        force: bool,
    ) -> Result<(), RdfDiagnostic> {
        if let Some(nest) = nest {
            if purrdf_lex::assoc::get(members, nest).is_none() {
                self.put(members, nest, Value::Object(Object::new()), false)?;
            }
            let target = purrdf_lex::assoc::get_mut(members, nest).expect("nest inserted");
            let object = target
                .as_object_mut()
                .ok_or_else(|| decode("compacted @nest must be an object"))?;
            let mut nested = std::mem::take(object).into_members();
            self.insert_property(&mut nested, property, None, value, force)?;
            *target = self.object(nested);
            return Ok(());
        }
        let value = if force { self.array(value)? } else { value };
        self.put(members, property, value, true)
    }
    fn take_field(
        &mut self,
        value: &mut Value,
        key: &str,
        first: bool,
        context: &CompiledJsonLdContext,
    ) -> Result<Option<Value>, RdfDiagnostic> {
        let Some(object) = value.as_object_mut() else {
            return Ok(None);
        };
        let Some(at) = object.members().iter().position(|(name, _)| name == key) else {
            return Ok(None);
        };
        let mut members = std::mem::take(object).into_members();
        let (name, mut field) = members.remove(at);
        if first && field.as_array().is_some_and(Vec::is_empty) {
            self.memory.release_string(name)?;
            self.discard(field)?;
            *value = self.object(members);
            return Ok(None);
        }
        let result = if first
            && let Value::Array(items) = &mut field
            && !items.is_empty()
        {
            let first = items.remove(0);
            if !items.is_empty() {
                let collapse = self.policy.compact_arrays
                    && context
                        .term(key)
                        .is_none_or(|definition| !definition.containers().contains(&C::Set));
                let field = self.collapse(field, collapse)?;
                members.insert(at, (name, field));
            } else {
                self.memory.release_string(name)?;
                self.discard(field)?;
            }
            first
        } else {
            self.memory.release_string(name)?;
            field
        };
        *value = self.object(members);
        Ok(Some(result))
    }
    fn extract_map_key(
        &mut self,
        value: &mut Value,
        key: &str,
        first: bool,
        context: &CompiledJsonLdContext,
    ) -> Result<Option<Value>, RdfDiagnostic> {
        let field = value.get(key).map(|field| {
            if first {
                field
                    .as_array()
                    .and_then(|items| items.first())
                    .unwrap_or(field)
            } else {
                field
            }
        });
        if field.is_some_and(|field| field.as_str().is_none()) {
            return Ok(None);
        }
        if let Some(field) = field
            && let Some(text) = field.as_str()
            && context.expand_iri(text, true, false)?.as_deref() == Some("@none")
        {
            // Sentinel injection adds nothing. Keep the real field in the child,
            // preserving its value even when its spelling is a keyword alias.
            return self.copy(field).map(Some);
        }
        self.take_field(value, key, first, context)
    }
    fn map_entry(
        &mut self,
        members: &mut Members,
        edge: &Edge<'a>,
        key: &str,
        value: Value,
    ) -> Result<(), RdfDiagnostic> {
        self.map_entry_at(members, edge, key, value, edge.nest.as_deref())
    }
    fn map_entry_at(
        &mut self,
        members: &mut Members,
        edge: &Edge<'a>,
        key: &str,
        value: Value,
        nest: Option<&str>,
    ) -> Result<(), RdfDiagnostic> {
        if let Some(nest) = nest {
            if purrdf_lex::assoc::get(members, nest).is_none() {
                self.put(members, nest, Value::Object(Object::new()), false)?;
            }
            let target = purrdf_lex::assoc::get_mut(members, nest).expect("nest inserted");
            let object = target
                .as_object_mut()
                .ok_or_else(|| decode("container @nest must be an object"))?;
            let mut nested = std::mem::take(object).into_members();
            self.map_entry_at(&mut nested, edge, key, value, None)?;
            *target = self.object(nested);
            return Ok(());
        }
        let mut map = if let Some(old) = purrdf_lex::assoc::get_mut(members, &edge.property) {
            old.take()
        } else {
            Value::Object(Object::new())
        };
        let object = map
            .as_object_mut()
            .ok_or_else(|| decode("container map collided with an ordinary property"))?;
        let mut entries = std::mem::take(object).into_members();
        let value = if edge.force {
            self.array(value)?
        } else {
            value
        };
        self.put(&mut entries, key, value, true)?;
        let map = self.object(entries);
        if let Some(old) = purrdf_lex::assoc::get_mut(members, &edge.property) {
            *old = map;
        } else {
            self.put(members, &edge.property, map, false)?;
        }
        Ok(())
    }
    fn finish_edge(
        &mut self,
        context: &CompiledJsonLdContext,
        members: &mut Members,
        edge: &Edge<'a>,
        mut value: Value,
    ) -> Result<(), RdfDiagnostic> {
        match edge.action {
            Action::Direct | Action::Nodes => self.put(members, &edge.property, value, false),
            Action::Explicit => {
                self.insert_property(members, &edge.property, edge.nest.as_deref(), value, true)
            }
            Action::Json => {
                self.insert_property(members, &edge.property, edge.nest.as_deref(), value, false)
            }
            Action::Reverse => {
                let object = value
                    .as_object_mut()
                    .ok_or_else(|| decode("compacted reverse map must remain an object"))?;
                let mut remaining = Vec::new();
                let mut reversed = std::mem::take(object).into_members();
                reversed.reverse();
                while let Some((key, child)) = reversed.pop() {
                    if context
                        .term(&key)
                        .is_some_and(JsonLdTermDefinition::is_reverse_property)
                    {
                        self.insert_property(members, &key, None, child, false)?;
                        self.memory.release_string(key)?;
                    } else {
                        self.memory.push(&mut remaining, (key, child))?;
                    }
                }
                self.memory.release_vec(reversed)?;
                if !remaining.is_empty() {
                    let value = self.object(remaining);
                    self.put(members, &edge.property, value, false)
                } else {
                    self.memory.release_vec(remaining)?;
                    Ok(())
                }
            }
            Action::ListObject => {
                self.insert_property(members, &edge.property, edge.nest.as_deref(), value, false)
            }
            Action::List => {
                value = self.array(value)?;
                let definition = context.term(&edge.property);
                if definition.is_some_and(|definition| definition.containers().contains(&C::List))
                    && edge.raw.get("@annotation").is_none()
                {
                    return self.insert_property(
                        members,
                        &edge.property,
                        edge.nest.as_deref(),
                        value,
                        false,
                    );
                }
                let mut list = Vec::new();
                let alias = context.compact_iri("@list", true)?;
                self.put(&mut list, &alias, value, false)?;
                if let Some(index) = edge.raw.get("@index") {
                    let alias = context.compact_iri("@index", true)?;
                    let index = self.copy(index)?;
                    self.put(&mut list, &alias, index, false)?;
                }
                // Annotation-bearing list objects are compacted as complete objects instead.
                let value = self.object(list);
                self.insert_property(
                    members,
                    &edge.property,
                    edge.nest.as_deref(),
                    value,
                    edge.force,
                )
            }
            Action::Graph => self.graph(context, members, edge, value),
            Action::Map(container) => {
                let definition = context
                    .term(&edge.property)
                    .expect("selected map definition");
                let mut key = match container {
                    C::Language => {
                        if let Some(raw) = edge.raw.get("@value") {
                            self.discard(value)?;
                            value = self.copy(raw)?;
                        }
                        edge.raw
                            .get("@language")
                            .map(|raw| self.copy(raw))
                            .transpose()?
                    }
                    C::Index if definition.index_mapping().is_none() => edge
                        .raw
                        .get("@index")
                        .map(|raw| self.copy(raw))
                        .transpose()?,
                    C::Index => {
                        let key = context
                            .compact_iri(definition.index_mapping().expect("custom index"), true)?;
                        self.extract_map_key(&mut value, &key, true, context)?
                    }
                    C::Id => {
                        let key = context.compact_iri("@id", true)?;
                        self.extract_map_key(&mut value, &key, false, context)?
                    }
                    C::Type => {
                        let key = context.compact_iri("@type", true)?;
                        self.extract_map_key(&mut value, &key, true, context)?
                    }
                    _ => unreachable!("only map containers schedule map actions"),
                };
                if container == C::Type {
                    let alias = context.compact_iri("@id", true)?;
                    if value
                        .as_object()
                        .is_some_and(|object| object.len() == 1 && object.contains_key(&alias))
                        && let Some(id) = edge.raw.get("@id").and_then(Value::as_str)
                        && let Some(scalar) = self.scalar_id(id, context, Some(&edge.property))?
                    {
                        self.discard(value)?;
                        value = scalar;
                    }
                }
                let key = match key {
                    Some(Value::String(ref mut key)) => std::mem::take(key),
                    Some(other) => {
                        self.discard(other)?;
                        return Err(decode("container map key must compact to a string"));
                    }
                    None => {
                        let key = context.compact_iri("@none", true)?;
                        self.memory.string(&key)?
                    }
                };
                self.map_entry(members, edge, &key, value)?;
                self.memory.release_string(key)?;
                Ok(())
            }
            Action::Add => self.insert_property(
                members,
                &edge.property,
                edge.nest.as_deref(),
                value,
                edge.force,
            ),
        }
    }
    fn graph(
        &mut self,
        context: &CompiledJsonLdContext,
        members: &mut Members,
        edge: &Edge<'a>,
        mut value: Value,
    ) -> Result<(), RdfDiagnostic> {
        let definition = context.term(&edge.property);
        let has = |container| {
            definition.is_some_and(|definition| definition.containers().contains(&container))
        };
        let graph_alias = context.compact_iri("@graph", true)?;
        let custom = definition.and_then(|definition| definition.index_mapping());
        // A custom graph index is represented by its actual expanded property.
        // Consume only its first selected value; preserve residual metadata/IDs.
        let custom_key = if has(C::Graph)
            && has(C::Index)
            && let Some(index) = custom
        {
            let raw = edge
                .raw
                .get(index)
                .and_then(Value::as_array)
                .and_then(|items| items.first());
            if let Some(raw) = raw {
                let alias = context.compact_property_iri(index, raw, false, false, &[])?;
                self.extract_map_key(&mut value, &alias, true, context)?
            } else {
                None
            }
        } else {
            None
        };
        let simple = edge
            .raw
            .as_object()
            .expect("graph edge")
            .keys()
            .all(|key| matches!(key.as_str(), "@graph" | "@index"));
        if has(C::Graph) && has(C::Id) {
            let id_alias = context.compact_iri("@id", true)?;
            let key = self.extract_map_key(&mut value, &id_alias, false, context)?;
            let key = match key {
                Some(key) => key,
                None => {
                    let key = context.compact_iri("@none", true)?;
                    self.string(&key)?
                }
            };
            let mut body = self
                .take_field(&mut value, &graph_alias, false, context)?
                .expect("projected graph body");
            if value.as_object().is_some_and(Object::is_empty) {
                self.discard(value)?;
                body = self.collapse(body, self.policy.compact_arrays)?;
            } else {
                let object = value.as_object_mut().expect("compacted graph");
                let mut fields = std::mem::take(object).into_members();
                self.put(&mut fields, &graph_alias, body, false)?;
                body = self.object(fields);
            }
            return self.map_key(members, edge, key, body);
        }
        if has(C::Graph) && has(C::Index) {
            let key = if custom.is_some() {
                custom_key
            } else {
                edge.raw
                    .get("@index")
                    .map(|raw| self.copy(raw))
                    .transpose()?
            };
            let key = match key {
                Some(key) => key,
                None => {
                    let key = context.compact_iri("@none", true)?;
                    self.string(&key)?
                }
            };
            let fields = value.as_object().expect("compacted graph");
            if fields.len() == 1 && fields.contains_key(&graph_alias) {
                let mut body = self
                    .take_field(&mut value, &graph_alias, false, context)?
                    .expect("graph field checked");
                self.discard(value)?;
                body = self.collapse(body, true)?;
                // Multiple nodes must remain one anonymous graph at injection.
                if body.as_array().is_some_and(|items| items.len() > 1) {
                    let mut fields = Vec::new();
                    self.put(&mut fields, &graph_alias, body, false)?;
                    body = self.object(fields);
                }
                value = body;
            }
            return self.map_key(members, edge, key, value);
        }
        if has(C::Graph) && simple {
            let mut body = self
                .take_field(&mut value, &graph_alias, false, context)?
                .expect("graph object");
            self.discard(value)?;
            body = self.collapse(body, true)?;
            if body.as_array().is_some_and(|items| items.len() > 1) {
                let alias = context.compact_iri("@included", true)?;
                let mut fields = Vec::new();
                self.put(&mut fields, &alias, body, false)?;
                body = self.object(fields);
            }
            return self.insert_property(
                members,
                &edge.property,
                edge.nest.as_deref(),
                body,
                edge.force,
            );
        }
        self.insert_property(
            members,
            &edge.property,
            edge.nest.as_deref(),
            value,
            edge.force,
        )
    }
    fn map_key(
        &mut self,
        members: &mut Members,
        edge: &Edge<'a>,
        mut key: Value,
        value: Value,
    ) -> Result<(), RdfDiagnostic> {
        let Value::String(text) = &mut key else {
            return Err(decode("container map key must compact to a string"));
        };
        let text = std::mem::take(text);
        self.map_entry(members, edge, &text, value)?;
        self.memory.release_string(text)?;
        Ok(())
    }
}

fn validate(object: &Object, property: Option<&str>, component: bool) -> Result<(), RdfDiagnostic> {
    if property == Some("@subject") && object.contains_key("@value") {
        return Err(decode("triple subject cannot be a literal"));
    }
    if property == Some("@reverse")
        && object.iter().any(|(key, value)| {
            key.starts_with('@')
                || value.as_array().is_none_or(|items| {
                    items.iter().any(|item| {
                        item.get("@value").is_some()
                            || item.get("@list").is_some()
                            || item.get("@set").is_some()
                    })
                })
        })
    {
        return Err(decode("expanded @reverse requires node property arrays"));
    }
    if !object.contains_key("@value")
        && (object.contains_key("@language") || object.contains_key("@direction"))
    {
        return Err(decode("language and direction require @value"));
    }
    if object.contains_key("@triple")
        && object
            .keys()
            .any(|key| !matches!(key.as_str(), "@triple" | "@annotation"))
    {
        return Err(decode("unexpected member on a triple term"));
    }
    if let Some(duplicate) = object.first_duplicate() {
        return Err(decode(format!("duplicate expanded member `{duplicate}`")));
    }
    if property == Some("@triple")
        && (object.len() != 3
            || !["@subject", "@predicate", "@object"]
                .iter()
                .all(|key| object.contains_key(key)))
    {
        return Err(decode(
            "@triple requires exactly @subject, @predicate and @object",
        ));
    }
    if let Some(id) = object.get("@id")
        && id.as_str().is_none()
    {
        return Err(decode("expanded @id must be an IRI string"));
    }
    if let Some(index) = object.get("@index")
        && index.as_str().is_none()
    {
        return Err(decode("expanded @index must be a string"));
    }
    if let Some(language) = object.get("@language")
        && language.as_str().is_none()
    {
        return Err(decode("expanded @language must be a string"));
    }
    if let Some(direction) = object.get("@direction")
        && !matches!(direction.as_str(), Some("ltr" | "rtl"))
    {
        return Err(decode("expanded @direction must be ltr or rtl"));
    }
    if object.contains_key("@value") {
        if object.keys().any(|key| {
            !matches!(
                key.as_str(),
                "@value" | "@type" | "@language" | "@direction" | "@index" | "@annotation"
            )
        }) {
            return Err(decode("unexpected member in expanded value object"));
        }
        let datatype = object.get("@type").and_then(Value::as_str);
        if object.get("@type").is_some() && datatype.is_none() {
            return Err(decode(
                "expanded value @type must be an IRI string or @json",
            ));
        }
        if datatype.is_some()
            && (object.contains_key("@language") || object.contains_key("@direction"))
        {
            return Err(decode("typed value cannot carry language or direction"));
        }
        let value = object.get("@value").expect("value presence checked");
        if datatype != Some("@json") && (value.as_array().is_some() || value.as_object().is_some())
        {
            return Err(decode("structured @value requires @type @json"));
        }
        if (object.contains_key("@language") || object.contains_key("@direction"))
            && value.as_str().is_none()
        {
            return Err(decode("language/direction value must be a string"));
        }
        if value.is_null() && datatype != Some("@json") && object.contains_key("@annotation") {
            return Err(decode("null @value cannot carry @annotation"));
        }
    } else if object.contains_key("@list") || object.contains_key("@set") {
        if object
            .keys()
            .any(|key| !matches!(key.as_str(), "@list" | "@set" | "@index" | "@annotation"))
            || object.contains_key("@list") && object.contains_key("@set")
        {
            return Err(decode("unexpected member in expanded list/set object"));
        }
        if object.contains_key("@set") && object.contains_key("@annotation") {
            return Err(decode("@set cannot carry @annotation"));
        }
    } else if let Some(types) = object.get("@type")
        && types
            .as_array()
            .is_none_or(|types| types.iter().any(|value| value.as_str().is_none()))
    {
        return Err(decode(
            "expanded node @type must be an array of IRI strings",
        ));
    }
    for (key, value) in object.iter() {
        match key.as_str() {
            "@value" | "@id" | "@index" | "@language" | "@direction" | "@type" => {}
            "@predicate" if property == Some("@triple") => {
                if value.as_str().is_none() {
                    return Err(decode("@predicate must be an IRI string"));
                }
            }
            "@subject" | "@object" if property == Some("@triple") => {
                if value.as_object().is_none() {
                    return Err(decode("triple components must remain objects"));
                }
            }
            "@triple" | "@reverse" => {
                if value.as_object().is_none() {
                    return Err(decode(format!("expanded {key} must be an object")));
                }
            }
            "@graph" | "@included" | "@list" | "@set" | "@annotation" => {
                if value.as_array().is_none() {
                    return Err(decode(format!("expanded {key} must be an array")));
                }
            }
            _ if key.starts_with('@') => {
                return Err(decode(format!("unexpected expanded keyword `{key}`")));
            }
            _ => {
                if value
                    .as_array()
                    .is_none_or(|values| values.iter().any(|value| value.as_object().is_none()))
                {
                    return Err(decode(format!(
                        "expanded property `{key}` must be an array of objects"
                    )));
                }
            }
        }
    }
    if component && object.contains_key("@list") {
        return Err(decode("triple component cannot be a list"));
    }
    Ok(())
}
