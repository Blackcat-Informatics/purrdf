// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! JSON-LD 1.1 API §5 document expansion, before node-map generation.

use super::context::is_control;
use super::{
    CompiledJsonLdContext, JsonLdContainer, JsonLdNullable, JsonLdTermDefinition,
    JsonLdTypeMapping, RdfDiagnostic, decode,
};
use purrdf_lex::allocation::{Ceiling, Memory};
use purrdf_lex::json::{Object, Value};
use purrdf_lex::walk::WorkList;
use std::rc::Rc;

/// Native construction allowance for structural expansion, separate from input
/// decoding, context compilation/resolution and subsequent RDF materialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonLdExpansionLimits {
    /// Maximum live requested layouts of newly constructed output and walk scratch.
    pub max_allocation_bytes: u64,
}
impl JsonLdExpansionLimits {
    /// A new 32 GiB structural construction policy; this is not the serializer's account.
    pub const DEFAULT: Self = Self {
        max_allocation_bytes: 32 * 1024 * 1024 * 1024,
    };
}
impl Default for JsonLdExpansionLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub(super) struct Expanded {
    pub(super) value: Value,
    pub(super) document_base: Option<String>,
}

type Context = Rc<CompiledJsonLdContext>;
type Members = Vec<(String, Value)>;

struct Member<'a> {
    original: &'a str,
    name: String,
    raw: &'a Value,
    child: bool,
    reverse: bool,
}
struct Plan<'a> {
    members: Vec<Member<'a>>,
    context: Context,
    type_context: Context,
    property: Option<&'a str>,
    nest: bool,
    input_json: bool,
}
#[derive(Clone, Copy)]
struct Request<'a> {
    raw: &'a Value,
    property: Option<&'a str>,
    from_map: bool,
    root: bool,
    nest: bool,
}
struct Injection<'a> {
    term: &'a str,
    key: &'a str,
    index: Option<String>,
    id: bool,
    rdf_type: bool,
    graph: bool,
    context: Context,
}
enum Task<'a> {
    Expand(Request<'a>, Context, Option<Context>),
    Elements {
        remaining: std::slice::Iter<'a, Value>,
        request: Request<'a>,
        context: Context,
        nest_type: Option<Context>,
        output: Value,
        list: bool,
        waiting: bool,
    },
    Property(&'a Value, &'a str, Context),
    Array {
        start: usize,
        list: bool,
    },
    Object(Plan<'a>, usize),
    Wrap {
        list: bool,
        graph: bool,
    },
    Inject(Injection<'a>),
}

pub(super) fn expand(
    input: &Value,
    context: &CompiledJsonLdContext,
    limits: JsonLdExpansionLimits,
) -> Result<Expanded, RdfDiagnostic> {
    let mut ceiling = Ceiling(usize::try_from(limits.max_allocation_bytes).unwrap_or(usize::MAX));
    let mut memory = Memory::new(&mut ceiling);
    memory.try_scope(|memory| {
        let mut walk = Walk {
            tasks: WorkList::new(),
            results: Vec::new(),
            // The retrieval URL stays fixed when a local context changes @base.
            // Native contexts without an original URL use their entry base once.
            document_url: context
                .original_document_iri()
                .or_else(|| context.base_iri())
                .map(|url| memory.string(url))
                .transpose()?,
            document_base: context
                .base_iri()
                .map(|base| memory.string(base))
                .transpose()?,
            memory,
        };
        walk.push(Task::Expand(
            Request {
                raw: input,
                property: None,
                from_map: false,
                root: true,
                nest: false,
            },
            Rc::new(context.clone()),
            None,
        ))?;
        // Every expansion/continuation produces one result. A parent consumes
        // exactly the children it scheduled; successful completion leaves one root.
        while let Some(task) = walk.tasks.pop() {
            walk.step(task)?;
        }
        let value = walk.results.pop().expect("one document result");
        walk.memory.release_vec(std::mem::take(&mut walk.results))?;
        std::mem::replace(&mut walk.tasks, WorkList::new()).release_admitted(walk.memory)?;
        if let Some(url) = walk.document_url.take() {
            walk.memory.release_string(url)?;
        }
        let mut value = value;
        // API expand normalization applies to a map before array normalization;
        // an input array containing one graph object retains that graph object.
        if let Value::Object(object) = &mut value
            && object.len() == 1
            && object.contains_key("@graph")
        {
            // The guard established one object containing exactly @graph.
            let graph = object.get_mut("@graph").unwrap().take();
            value.release_with_memory(walk.memory)?;
            value = graph;
        }
        let value = walk.array(value)?;
        Ok(Expanded {
            value,
            document_base: walk.document_base.take(),
        })
    })
}

struct Walk<'a, 'm, 's> {
    tasks: WorkList<Task<'a>, 8>,
    results: Vec<Value>,
    document_url: Option<String>,
    document_base: Option<String>,
    memory: &'m mut Memory<'s, Ceiling>,
}
impl<'a> Walk<'a, '_, '_> {
    fn ordered_members<'b>(
        &mut self,
        object: &'b Object,
    ) -> Result<Vec<(&'b str, &'b Value)>, RdfDiagnostic> {
        let mut members = Vec::new();
        self.memory.reserve(&mut members, object.len())?;
        for (key, value) in object.iter() {
            members.push((key.as_str(), value));
        }
        members.sort_unstable_by(|a, b| a.0.cmp(b.0));
        Ok(members)
    }
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
    fn string(&mut self, text: &str) -> Result<Value, RdfDiagnostic> {
        Ok(Value::String(self.memory.string(text)?))
    }
    fn object(&self, mut members: Members) -> Value {
        members.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        Value::Object(Object::from(members))
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
                return Err(decode(format!("colliding JSON-LD keyword `{key}`")));
            }
            self.append(old, value)?;
        } else {
            let key = self.memory.string(key)?;
            self.memory.push(members, (key, value))?;
        }
        Ok(())
    }
    fn append(&mut self, target: &mut Value, mut value: Value) -> Result<(), RdfDiagnostic> {
        // Only array() results and explicitly initialized arrays reach append.
        let target = target.as_array_mut().expect("expanded property array");
        match &mut value {
            Value::Null => {}
            Value::Array(values) => {
                self.memory.extend(target, values.drain(..))?;
                self.memory.release_vec(std::mem::take(values))?;
            }
            _ => self.memory.push(target, value)?,
        }
        Ok(())
    }
    fn array(&mut self, value: Value) -> Result<Value, RdfDiagnostic> {
        if matches!(value, Value::Array(_)) {
            return Ok(value);
        }
        let mut values = Vec::new();
        if !value.is_null() {
            self.memory.push(&mut values, value)?;
        }
        Ok(Value::Array(values))
    }
    fn iri(
        &mut self,
        context: &CompiledJsonLdContext,
        text: &str,
        vocab: bool,
        relative: bool,
    ) -> Result<Value, RdfDiagnostic> {
        // The resolver/compiler owns its native scratch. The new output copy is
        // admitted BEFORE copying; no resident result is adopted into this account.
        match context.expand_iri(text, vocab, relative)? {
            Some(text) => self.string(&text),
            None => Ok(Value::Null),
        }
    }
    fn field(&mut self, key: &str, value: Value) -> Result<Value, RdfDiagnostic> {
        let mut members = Vec::new();
        self.put(&mut members, key, value, false)?;
        Ok(self.object(members))
    }
    fn step(&mut self, task: Task<'a>) -> Result<(), RdfDiagnostic> {
        match task {
            Task::Expand(request, context, nest_type) => self.element(request, context, nest_type),
            Task::Elements {
                mut remaining,
                request,
                context,
                nest_type,
                mut output,
                list,
                waiting,
            } => {
                if waiting {
                    let mut value = self.results.pop().expect("array child");
                    if list && value.as_array().is_some() {
                        value = self.field("@list", value)?;
                    }
                    self.append(&mut output, value)?;
                }
                if let Some(raw) = remaining.next() {
                    self.push(Task::Elements {
                        remaining,
                        request,
                        context: Rc::clone(&context),
                        nest_type: nest_type.clone(),
                        output,
                        list,
                        waiting: true,
                    })?;
                    self.push(Task::Expand(Request { raw, ..request }, context, nest_type))
                } else {
                    self.result(output)
                }
            }
            Task::Property(raw, key, context) => self.property(raw, key, context),
            Task::Array { start, list } => {
                let mut result = Value::Array(Vec::new());
                for index in start..self.results.len() {
                    let mut value = self.results[index].take();
                    if list && value.as_array().is_some() {
                        value = self.field("@list", value)?;
                    }
                    self.append(&mut result, value)?;
                }
                self.results.truncate(start);
                self.result(result)
            }
            Task::Object(plan, start) => self.finish_object(plan, start),
            Task::Wrap { list, graph } => {
                let value = self.results.pop().expect("wrapped child");
                if value.is_null() {
                    return self.result(value);
                }
                let value = if list {
                    if value.get("@list").is_some() {
                        self.array(value)?
                    } else {
                        let values = self.array(value)?;
                        let list = self.field("@list", values)?;
                        self.array(list)?
                    }
                } else if graph {
                    let mut output = Value::Array(Vec::new());
                    let mut values = self.array(value)?;
                    // array() guarantees this mutable array view.
                    for value in values.as_array_mut().unwrap().drain(..) {
                        let graph_values = self.array(value)?;
                        let graph = self.field("@graph", graph_values)?;
                        self.append(&mut output, graph)?;
                    }
                    values.release_with_memory(self.memory)?;
                    output
                } else {
                    self.array(value)?
                };
                self.result(value)
            }
            Task::Inject(injection) => self.inject(injection),
        }
    }
    fn element(
        &mut self,
        request: Request<'a>,
        context: Context,
        nest_type: Option<Context>,
    ) -> Result<(), RdfDiagnostic> {
        match request.raw {
            Value::Null => self.result(Value::Null),
            Value::Array(values) => {
                let list = request
                    .property
                    .and_then(|key| context.term(key))
                    .is_some_and(|definition| {
                        definition.containers().contains(&JsonLdContainer::List)
                    });
                self.push(Task::Elements {
                    remaining: values.iter(),
                    request,
                    context,
                    nest_type,
                    output: Value::Array(Vec::new()),
                    list,
                    waiting: false,
                })
            }
            Value::Object(object) => self.start_object(request, object, &context, nest_type),
            _ if request.property.is_none() || request.property == Some("@graph") => {
                self.result(Value::Null)
            }
            _ => {
                let scoped = request
                    .property
                    .and_then(|key| context.term(key))
                    .filter(|definition| definition.scoped_context().is_some())
                    .map(|definition| context.apply_property_scope(definition))
                    .transpose()?;
                let active = scoped.as_ref().unwrap_or(&context);
                let value = self.scalar(
                    request.raw,
                    active,
                    request.property.and_then(|key| active.term(key)),
                )?;
                self.result(value)
            }
        }
    }
    fn start_object(
        &mut self,
        request: Request<'a>,
        object: &'a Object,
        context: &Context,
        nest_type: Option<Context>,
    ) -> Result<(), RdfDiagnostic> {
        let property_scope = request
            .property
            .and_then(|key| context.term(key))
            .filter(|definition| definition.scoped_context().is_some());
        let mut active = Rc::clone(context);
        if !request.nest {
            if !request.from_map && context.has_previous_context() {
                let mut value = false;
                let mut id_only = object.len() == 1;
                for key in object.keys() {
                    let expanded = context.expand_iri(key, true, false)?;
                    value |= expanded.as_deref() == Some("@value");
                    id_only &= expanded.as_deref() == Some("@id");
                }
                if !value && !id_only {
                    // The immutable context passed has_previous_context above.
                    active = Rc::new(context.previous_context().expect("previous context"));
                }
            }
            if let Some(definition) = property_scope {
                active = Rc::new(active.apply_property_scope(definition)?);
            }
            if let Some(local) = object.get("@context") {
                active =
                    Rc::new(active.apply_document_context(local, self.document_url.as_deref())?);
                if request.root {
                    if let Some(old) = self.document_base.take() {
                        self.memory.release_string(old)?;
                    }
                    self.document_base = active
                        .base_iri()
                        .map(|base| self.memory.string(base))
                        .transpose()?;
                }
            }
        }
        let type_context = nest_type.unwrap_or_else(|| Rc::clone(&active));
        let mut types = Vec::new();
        let mut input_json = false;
        let mut first_type_key: Option<&str> = None;
        for (key, raw) in object.iter() {
            if active.expand_iri(key, true, false)?.as_deref() == Some("@type") {
                let values = raw
                    .as_array()
                    .map_or_else(|| std::slice::from_ref(raw), Vec::as_slice);
                for value in values {
                    if let Some(term) = value.as_str() {
                        self.memory.push(&mut types, (key.as_str(), term))?;
                    }
                }
                if first_type_key.is_none_or(|first| key.as_str() < first) {
                    first_type_key = Some(key);
                    input_json = values
                        .last()
                        .and_then(Value::as_str)
                        .map(|value| type_context.expand_iri(value, true, true))
                        .transpose()?
                        .flatten()
                        .as_deref()
                        == Some("@json");
                }
            }
        }
        if !request.nest {
            types.sort_unstable();
            for (_, term) in &types {
                if let Some(definition) = type_context
                    .term(term)
                    .filter(|definition| definition.scoped_context().is_some())
                {
                    active = Rc::new(active.apply_type_scope(definition, false)?);
                }
            }
        }
        self.memory.release_vec(types)?;
        let mut members = Vec::new();
        for (key, raw) in object.iter() {
            if key == "@context" {
                continue;
            }
            let Some(name) = active.expand_iri(key, true, false)? else {
                continue;
            };
            if !is_control(&name) && !name.contains(':') {
                continue;
            }
            if request.nest && name == "@value" {
                return Err(decode("@nest cannot contain a value object"));
            }
            if request.property == Some("@reverse") && is_control(&name) {
                return Err(decode("keyword inside @reverse map"));
            }
            if name == "@reverse" && raw.as_object().is_none() {
                return Err(decode("@reverse must be an object"));
            }
            let child = matches!(
                name.as_str(),
                "@graph"
                    | "@included"
                    | "@reverse"
                    | "@list"
                    | "@set"
                    | "@nest"
                    | "@annotation"
                    | "@triple"
                    | "@subject"
                    | "@object"
            ) || !is_control(&name);
            let reverse = active
                .term(key)
                .is_some_and(JsonLdTermDefinition::is_reverse_property);
            let name = self.memory.string(&name)?;
            self.memory.push(
                &mut members,
                Member {
                    original: key,
                    name,
                    raw,
                    child,
                    reverse,
                },
            )?;
        }
        // Raw input order is not semantic; §5.1 ordered traversal uses lexical keys.
        members.sort_unstable_by(|a, b| a.original.cmp(b.original));
        let start = self.results.len();
        for member in members.iter().rev().filter(|member| member.child) {
            let key = member.original;
            if !is_control(&member.name) {
                self.push(Task::Property(member.raw, key, Rc::clone(&active)))?;
            } else {
                let property = match member.name.as_str() {
                    "@included" => None,
                    "@graph" => Some("@graph"),
                    "@reverse" => Some("@reverse"),
                    "@annotation" => Some("@annotation"),
                    "@triple" => Some("@triple"),
                    "@subject" => Some("@subject"),
                    "@object" => Some("@object"),
                    _ => request.property,
                };
                self.push(Task::Expand(
                    Request {
                        raw: member.raw,
                        property,
                        from_map: false,
                        root: false,
                        nest: member.name == "@nest",
                    },
                    Rc::clone(&active),
                    (member.name == "@nest").then(|| Rc::clone(&type_context)),
                ))?;
            }
        }
        // Put the continuation below its children, retaining their original order.
        let children = members.iter().filter(|member| member.child).count();
        self.push(Task::Object(
            Plan {
                members,
                context: active,
                type_context,
                property: request.property,
                nest: request.nest,
                input_json,
            },
            start,
        ))?;
        self.tasks.reverse_top(children + 1);
        // reverse_top above yields continuation first followed by lexical children.
        // Reverse just the children to preserve the stack's execution order.
        self.tasks.reverse_top(children);
        Ok(())
    }
    fn scalar(
        &mut self,
        raw: &Value,
        context: &CompiledJsonLdContext,
        definition: Option<&JsonLdTermDefinition>,
    ) -> Result<Value, RdfDiagnostic> {
        if let Some(mapping @ (JsonLdTypeMapping::Id | JsonLdTypeMapping::Vocab)) =
            definition.and_then(JsonLdTermDefinition::type_mapping)
            && let Some(text) = raw.as_str()
        {
            let value = self.iri(
                context,
                text,
                matches!(mapping, JsonLdTypeMapping::Vocab),
                true,
            )?;
            return self.field("@id", value);
        }
        let mut members = Vec::new();
        let value = raw.clone_with_memory(self.memory)?;
        self.put(&mut members, "@value", value, false)?;
        if let Some(datatype) = definition
            .and_then(JsonLdTermDefinition::type_mapping)
            .and_then(|mapping| match mapping {
                JsonLdTypeMapping::Datatype(datatype) => Some(datatype.as_str()),
                JsonLdTypeMapping::Json => Some("@json"),
                _ => None,
            })
        {
            let datatype = self.string(datatype)?;
            self.put(&mut members, "@type", datatype, false)?;
        } else if raw.as_str().is_some() {
            let language = match definition.and_then(JsonLdTermDefinition::language_mapping) {
                Some(JsonLdNullable::Null) => None,
                Some(JsonLdNullable::Value(value)) => Some(value),
                None => context.default_language(),
            };
            if let Some(language) = language {
                let value = self.string(language)?;
                self.put(&mut members, "@language", value, false)?;
            }
            let direction = match definition.and_then(JsonLdTermDefinition::direction_mapping) {
                Some(JsonLdNullable::Null) => None,
                Some(JsonLdNullable::Value(value)) => Some(value),
                None => context.default_direction(),
            };
            if let Some(direction) = direction {
                let value = self.string(direction.as_str())?;
                self.put(&mut members, "@direction", value, false)?;
            }
        }
        Ok(self.object(members))
    }
    fn property(
        &mut self,
        raw: &'a Value,
        key: &'a str,
        context: Context,
    ) -> Result<(), RdfDiagnostic> {
        let definition = context.term(key);
        if definition.and_then(JsonLdTermDefinition::type_mapping) == Some(&JsonLdTypeMapping::Json)
        {
            let mut members = Vec::new();
            let value = raw.clone_with_memory(self.memory)?;
            self.put(&mut members, "@value", value, false)?;
            let json = self.string("@json")?;
            self.put(&mut members, "@type", json, false)?;
            let object = self.object(members);
            let value = self.array(object)?;
            return self.result(value);
        }
        let contains = |container| {
            definition.is_some_and(|definition| definition.containers().contains(&container))
        };
        if contains(JsonLdContainer::Language)
            && let Some(languages) = raw.as_object()
        {
            let mut output = Value::Array(Vec::new());
            let languages = self.ordered_members(languages)?;
            for &(language, values) in &languages {
                let values = values
                    .as_array()
                    .map_or_else(|| std::slice::from_ref(values), Vec::as_slice);
                for raw in values {
                    if raw.is_null() {
                        continue;
                    }
                    if raw.as_str().is_none() {
                        return Err(decode("language map values must be strings"));
                    }
                    let mut members = Vec::new();
                    let value = raw.clone_with_memory(self.memory)?;
                    self.put(&mut members, "@value", value, false)?;
                    if context.expand_iri(language, true, false)?.as_deref() != Some("@none") {
                        let folded = purrdf_iri::langtag::identity_fold(language);
                        let value = self.string(&folded)?;
                        self.put(&mut members, "@language", value, false)?;
                    }
                    let direction =
                        match definition.and_then(JsonLdTermDefinition::direction_mapping) {
                            Some(JsonLdNullable::Null) => None,
                            Some(JsonLdNullable::Value(value)) => Some(value),
                            None => context.default_direction(),
                        };
                    if let Some(direction) = direction {
                        let value = self.string(direction.as_str())?;
                        self.put(&mut members, "@direction", value, false)?;
                    }
                    let object = self.object(members);
                    self.append(&mut output, object)?;
                }
            }
            self.memory.release_vec(languages)?;
            return self.result(output);
        }
        let id = contains(JsonLdContainer::Id);
        let rdf_type = contains(JsonLdContainer::Type);
        let index = contains(JsonLdContainer::Index);
        let graph = contains(JsonLdContainer::Graph);
        if (id || rdf_type || index)
            && let Some(entries) = raw.as_object()
        {
            self.push(Task::Array {
                start: self.results.len(),
                list: false,
            })?;
            let entries = self.ordered_members(entries)?;
            for &(index_key, values) in entries.iter().rev() {
                let mut map_context = if (id || rdf_type) && context.has_previous_context() {
                    // The same immutable context passed the presence guard.
                    Rc::new(context.previous_context().unwrap())
                } else {
                    Rc::clone(&context)
                };
                if rdf_type
                    && let Some(definition) = map_context
                        .term(index_key)
                        .filter(|definition| definition.scoped_context().is_some())
                {
                    map_context = Rc::new(map_context.apply_type_scope(definition, true)?);
                }
                let values = values
                    .as_array()
                    .map_or_else(|| std::slice::from_ref(values), Vec::as_slice);
                for raw in values.iter().rev() {
                    let index = if index {
                        Some(
                            self.memory.string(
                                definition
                                    .and_then(JsonLdTermDefinition::index_mapping)
                                    .unwrap_or("@index"),
                            )?,
                        )
                    } else {
                        None
                    };
                    self.push(Task::Inject(Injection {
                        term: key,
                        key: index_key,
                        index,
                        id,
                        rdf_type,
                        graph,
                        context: Rc::clone(&context),
                    }))?;
                    self.push(Task::Expand(
                        Request {
                            raw,
                            property: Some(key),
                            from_map: true,
                            root: false,
                            nest: false,
                        },
                        Rc::clone(&map_context),
                        None,
                    ))?;
                }
            }
            self.memory.release_vec(entries)?;
            return Ok(());
        }
        self.push(Task::Wrap {
            list: contains(JsonLdContainer::List),
            graph,
        })?;
        self.push(Task::Expand(
            Request {
                raw,
                property: Some(key),
                from_map: false,
                root: false,
                nest: false,
            },
            context,
            None,
        ))
    }
    fn inject(&mut self, injection: Injection<'_>) -> Result<(), RdfDiagnostic> {
        let Injection {
            term,
            key,
            index,
            id,
            rdf_type,
            graph,
            context,
        } = injection;
        let context = &*context;
        let child = self.results.pop().expect("container child");
        let mut values = self.array(child)?;
        let mut output = Value::Array(Vec::new());
        let none = context.expand_iri(key, true, false)?.as_deref() == Some("@none");
        // Container children were normalized by array() immediately above.
        for mut value in values.as_array_mut().unwrap().drain(..) {
            if graph && value.get("@graph").is_none() {
                let child = self.array(value)?;
                value = self.field("@graph", child)?;
            }
            let Some(object) = value.as_object_mut() else {
                return Err(decode("container expansion requires objects"));
            };
            let mut members = std::mem::take(object).into_members();
            if !none {
                if let Some(index) = &index {
                    if index == "@index" {
                        if purrdf_lex::assoc::get(&members, index).is_none() {
                            let value = self.string(key)?;
                            self.put(&mut members, index, value, false)?;
                        }
                    } else {
                        if purrdf_lex::assoc::get(&members, "@value").is_some() {
                            return Err(decode(
                                "custom index cannot add a property to a value object",
                            ));
                        }
                        let mut raw = self.string(key)?;
                        let item = self.scalar(
                            &raw,
                            context,
                            context
                                .term(term)
                                .and_then(JsonLdTermDefinition::index_term)
                                .and_then(|term| context.term(term)),
                        )?;
                        raw.release_with_memory(self.memory)?;
                        let mut array = self.array(item)?;
                        if let Some(old) = purrdf_lex::assoc::get_mut(&mut members, index) {
                            self.append(&mut array, old.take())?;
                            *old = array;
                        } else {
                            self.put(&mut members, index, array, false)?;
                        }
                    }
                } else if id && purrdf_lex::assoc::get(&members, "@id").is_none() {
                    let id = self.iri(context, key, false, true)?;
                    self.put(&mut members, "@id", id, false)?;
                } else if rdf_type {
                    let rdf_type = self.iri(context, key, true, false)?;
                    let mut types = self.array(rdf_type)?;
                    if let Some(old) = purrdf_lex::assoc::get_mut(&mut members, "@type") {
                        self.append(&mut types, old.take())?;
                        *old = types;
                    } else {
                        self.put(&mut members, "@type", types, false)?;
                    }
                }
            }
            value = self.object(members);
            self.append(&mut output, value)?;
        }
        values.release_with_memory(self.memory)?;
        if let Some(index) = index {
            self.memory.release_string(index)?;
        }
        self.result(output)
    }
    #[expect(
        clippy::iter_with_drain,
        reason = "Original admitted buffers remain alive until release_vec destroys and refunds them through the same account."
    )]
    fn merge(&mut self, output: &mut Members, mut object: Value) -> Result<(), RdfDiagnostic> {
        let object = object
            .as_object_mut()
            .ok_or_else(|| decode("nested expansion must be an object"))?;
        let mut members = std::mem::take(object).into_members();
        for (key, value) in members.drain(..) {
            let append = !is_control(&key) || matches!(key.as_str(), "@type" | "@included");
            if key == "@reverse" {
                let mut value = value;
                let mut reverse = std::mem::take(
                    value
                        .as_object_mut()
                        .ok_or_else(|| decode("nested reverse map"))?,
                )
                .into_members();
                for (property, values) in reverse.drain(..) {
                    self.reverse(output, &property, values)?;
                    self.memory.release_string(property)?;
                }
                self.memory.release_vec(reverse)?;
            } else {
                self.put(output, &key, value, append)?;
            }
            self.memory.release_string(key)?;
        }
        self.memory.release_vec(members)?;
        Ok(())
    }
    #[expect(
        clippy::iter_with_drain,
        reason = "Original admitted buffers remain alive until release_vec destroys and refunds them through the same account."
    )]
    fn finish_object(&mut self, mut plan: Plan<'a>, start: usize) -> Result<(), RdfDiagnostic> {
        let mut output: Members = Vec::new();
        let mut nests = Vec::new();
        let mut next = start;
        for member in plan.members.drain(..) {
            let mut value = if member.child {
                let value = self.results[next].take();
                next += 1;
                value
            } else {
                Value::Null
            };
            let keyword = member.name.as_str();
            match keyword {
                "@id" => {
                    value = self.iri(
                        &plan.context,
                        member
                            .raw
                            .as_str()
                            .ok_or_else(|| decode("@id must be a string"))?,
                        false,
                        true,
                    )?;
                }
                "@type" => {
                    let mut values = Vec::new();
                    for raw in member
                        .raw
                        .as_array()
                        .map_or_else(|| std::slice::from_ref(member.raw), Vec::as_slice)
                    {
                        let term = raw
                            .as_str()
                            .ok_or_else(|| decode("@type must contain IRI strings"))?;
                        let value = self.iri(&plan.type_context, term, true, true)?;
                        self.memory.push(&mut values, value)?;
                    }
                    value = if member.raw.as_array().is_some() {
                        Value::Array(values)
                    } else {
                        // A non-array type input contributed exactly one entry.
                        let value = values.pop().expect("one type");
                        self.memory.release_vec(values)?;
                        value
                    };
                    if purrdf_lex::assoc::get(&output, "@type").is_some() {
                        value = self.array(value)?;
                        // The presence guard above precedes any map mutation.
                        let old = purrdf_lex::assoc::get_mut(&mut output, "@type").unwrap();
                        let mut previous = self.array(old.take())?;
                        self.append(&mut previous, value)?;
                        *old = previous;
                        self.memory.release_string(member.name)?;
                        continue;
                    }
                }
                "@value" => {
                    if !plan.input_json
                        && !matches!(
                            member.raw,
                            Value::Null | Value::String(_) | Value::Number(_) | Value::Bool(_)
                        )
                    {
                        return Err(decode("@value requires a scalar unless @type is @json"));
                    }
                    value = member.raw.clone_with_memory(self.memory)?;
                }
                "@language" => {
                    let language = member
                        .raw
                        .as_str()
                        .ok_or_else(|| decode("@language must be a string"))?;
                    value = self.string(&purrdf_iri::langtag::identity_fold(language))?;
                }
                "@direction" => {
                    let direction = member
                        .raw
                        .as_str()
                        .filter(|value| matches!(*value, "ltr" | "rtl"))
                        .ok_or_else(|| decode("@direction must be ltr or rtl"))?;
                    value = self.string(direction)?;
                }
                "@index" => {
                    value = self.string(
                        member
                            .raw
                            .as_str()
                            .ok_or_else(|| decode("@index must be a string"))?,
                    )?;
                }
                "@predicate" if plan.property == Some("@triple") => {
                    value = self.iri(
                        &plan.context,
                        member
                            .raw
                            .as_str()
                            .ok_or_else(|| decode("@predicate must be an IRI string"))?,
                        true,
                        false,
                    )?;
                }
                "@subject" | "@object" if plan.property == Some("@triple") => {}
                "@triple" => {
                    if value.as_object().is_none() {
                        return Err(decode("@triple must be an object"));
                    }
                }
                "@annotation" => {
                    value = self.array(value)?;
                }
                "@list" if plan.property.is_none() || plan.property == Some("@graph") => {
                    value.release_with_memory(self.memory)?;
                    self.memory.release_string(member.name)?;
                    continue;
                }
                "@graph" | "@included" | "@list" => {
                    value = self.array(value)?;
                    // array() normalized graph/included/list child results.
                    if keyword == "@included"
                        && value.as_array().unwrap().iter().any(|value| {
                            value.get("@value").is_some() || value.get("@list").is_some()
                        })
                    {
                        return Err(decode("@included requires node objects"));
                    }
                }
                "@nest" => {
                    self.memory.push(&mut nests, value)?;
                    self.memory.release_string(member.name)?;
                    continue;
                }
                "@reverse" => {
                    if value.is_null() {
                        self.memory.release_string(member.name)?;
                        continue;
                    }
                    let reverse = value
                        .as_object_mut()
                        .ok_or_else(|| decode("@reverse requires an object"))?;
                    if let Some(double) = reverse.get_mut("@reverse") {
                        let double = double.take();
                        self.merge(&mut output, double)?;
                    }
                    let mut reverse_members = std::mem::take(reverse).into_members();
                    for (key, values) in reverse_members.drain(..) {
                        if key != "@reverse" {
                            if values.as_array().is_none_or(|values| {
                                values.iter().any(|value| {
                                    value.get("@value").is_some() || value.get("@list").is_some()
                                })
                            }) {
                                return Err(decode("reverse properties require node values"));
                            }
                            self.reverse(&mut output, &key, values)?;
                        }
                        self.memory.release_string(key)?;
                    }
                    self.memory.release_vec(reverse_members)?;
                    self.memory.release_string(member.name)?;
                    continue;
                }
                "@set" => {}
                keyword if is_control(keyword) => {
                    return Err(decode(format!("unsupported expanded keyword `{keyword}`")));
                }
                _ => {
                    if value.is_null() {
                        self.memory.release_string(member.name)?;
                        continue;
                    }
                    value = self.array(value)?;
                    if member.reverse {
                        // The property value was normalized by array() above.
                        if value.as_array().unwrap().iter().any(|value| {
                            value.get("@value").is_some() || value.get("@list").is_some()
                        }) {
                            return Err(decode("reverse properties require node values"));
                        }
                        self.reverse(&mut output, keyword, value)?;
                        self.memory.release_string(member.name)?;
                        continue;
                    }
                }
            }
            let append = !is_control(keyword) || keyword == "@included";
            self.put(&mut output, keyword, value, append)?;
            self.memory.release_string(member.name)?;
        }
        self.memory.release_vec(plan.members)?;
        self.results.truncate(start);
        for mut nest in nests.drain(..) {
            if let Value::Array(values) = &mut nest {
                for value in values.drain(..) {
                    self.merge(&mut output, value)?;
                }
                nest.release_with_memory(self.memory)?;
            } else {
                self.merge(&mut output, nest)?;
            }
        }
        self.memory.release_vec(nests)?;
        let mut value = self.object(output);
        if plan.property == Some("@triple") {
            // object() constructed this result from admitted unique members.
            let object = value.as_object().unwrap();
            if object.len() != 3
                || !["@subject", "@predicate", "@object"]
                    .iter()
                    .all(|key| object.contains_key(key))
            {
                return Err(decode(
                    "@triple requires exactly @subject, @predicate and @object",
                ));
            }
        } else if let Some(object) = value.as_object_mut() {
            // Guarded members below remain present until the entire value is
            // released or the explicitly guarded @set member is taken.
            if object.contains_key("@value") {
                if object.keys().any(|key| {
                    !matches!(
                        key.as_str(),
                        "@value" | "@type" | "@language" | "@direction" | "@index" | "@annotation"
                    )
                }) {
                    return Err(decode("unexpected member in value object"));
                }
                if object.contains_key("@type")
                    && (object.contains_key("@language") || object.contains_key("@direction"))
                {
                    return Err(decode("typed value cannot carry language or direction"));
                }
                if let Some(datatype) = object.get("@type")
                    && datatype.as_str().is_none_or(|text| {
                        text != "@json" && purrdf_iri::BaseIri::parse(text).is_err()
                    })
                {
                    return Err(decode("value datatype must be an IRI"));
                }
                if object.get("@value").unwrap().is_null()
                    && object.get("@type").and_then(Value::as_str) != Some("@json")
                {
                    if object.contains_key("@annotation") {
                        return Err(decode("null @value object cannot carry @annotation"));
                    }
                    value.release_with_memory(self.memory)?;
                } else if object.contains_key("@language")
                    && object.get("@value").unwrap().as_str().is_none()
                {
                    return Err(decode("language-tagged value must be a string"));
                }
            } else if object.contains_key("@set") || object.contains_key("@list") {
                let set = object.contains_key("@set");
                if object
                    .keys()
                    .any(|key| !matches!(key.as_str(), "@set" | "@list" | "@index" | "@annotation"))
                    || (set && object.contains_key("@list"))
                {
                    return Err(decode("unexpected member in set/list object"));
                }
                if set {
                    if object.contains_key("@annotation") {
                        return Err(decode("@set cannot carry @annotation"));
                    }
                    // set was derived from this unchanged map's contains_key.
                    let child = object.get_mut("@set").unwrap().take();
                    value.release_with_memory(self.memory)?;
                    value = child;
                }
            } else if let Some(types) = object.get_mut("@type")
                && types.as_array().is_none()
            {
                *types = self.array(types.take())?;
            }
        }
        if value
            .as_object()
            .is_some_and(|object| object.len() == 1 && object.contains_key("@language"))
        {
            value.release_with_memory(self.memory)?;
        }
        if !plan.nest && (plan.property.is_none() || plan.property == Some("@graph")) {
            let prune = value.as_object().is_some_and(|object| {
                object.is_empty()
                    || object.contains_key("@value")
                    || object.contains_key("@list")
                    || (object.len() == 1 && object.contains_key("@id"))
            });
            if prune {
                value.release_with_memory(self.memory)?;
            }
        }
        self.result(value)
    }
    fn reverse(
        &mut self,
        output: &mut Members,
        key: &str,
        value: Value,
    ) -> Result<(), RdfDiagnostic> {
        if purrdf_lex::assoc::get(output, "@reverse").is_none() {
            self.put(output, "@reverse", Value::Object(Object::new()), false)?;
        }
        // The absent branch above inserts an object; existing entries are only
        // created by this same helper, so both member and object views hold.
        let reverse = purrdf_lex::assoc::get_mut(output, "@reverse")
            .unwrap()
            .as_object_mut()
            .unwrap();
        let mut members = std::mem::take(reverse).into_members();
        self.put(&mut members, key, value, true)?;
        members.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        *reverse = Object::from(members);
        Ok(())
    }
}
