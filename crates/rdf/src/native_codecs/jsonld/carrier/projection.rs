// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Typed RDF preparation projected to the shared expanded-value compactor.

use super::super::{
    JsonLdContainer, JsonLdTermSelection, JsonLdTermSelectionKind, JsonLdTypeMapping,
    RdfDiagnostic, ordered_object,
};
use super::{CompactionScope, NamedGraph, Node, Term, Value};
use crate::native_codecs::jsonld::compaction::Context;
use purrdf_lex::allocation::{Ceiling, Memory};
use purrdf_lex::json::{self, Object, Value as JsonValue};
use purrdf_lex::walk::WorkList;
use std::collections::BTreeSet;
use std::rc::Rc;

type Members = Vec<(String, JsonValue)>;
struct Fields {
    members: Members,
    keys: Vec<String>,
}
enum Task<'a> {
    Node(&'a Node, Option<&'a str>, Context<'a>),
    Value(&'a Value, Context<'a>),
    Term(&'a Term, Context<'a>),
    Values(&'a [Value], Context<'a>),
    Graph(
        &'a NamedGraph,
        bool,
        Option<(&'a str, &'a str)>,
        Context<'a>,
    ),
    Fields(Fields),
    Group(Fields, Vec<Self>),
    GroupArray(Vec<Self>),
    Array(usize),
    Annotated(usize),
}
struct Projection<'a, 'm, 's> {
    scope: &'a CompactionScope<'a>,
    embedded: &'m mut BTreeSet<String>,
    tasks: WorkList<Task<'a>, 8>,
    results: Vec<JsonValue>,
    memory: &'m mut Memory<'s, Ceiling>,
}

pub(super) fn node<'a>(
    scope: &'a CompactionScope<'a>,
    node: &'a Node,
    graph: Option<&'a str>,
    embedded: &mut BTreeSet<String>,
) -> Result<JsonValue, RdfDiagnostic> {
    // Projection is newly owned input to compaction, with its own live authority.
    // The subsequent compactor borrows it and never refunds it into its fresh account.
    let mut ceiling = Ceiling(
        usize::try_from(super::super::JsonLdCompactionLimits::DEFAULT.max_allocation_bytes)
            .unwrap_or(usize::MAX),
    );
    Memory::new(&mut ceiling).try_scope(|memory| {
        let mut projection = Projection {
            scope,
            embedded,
            tasks: WorkList::new(),
            results: Vec::new(),
            memory,
        };
        projection
            .tasks
            .try_push_admitted(
                Task::Node(node, graph, Context::Borrowed(scope.context)),
                projection.memory,
            )
            .map_err(RdfDiagnostic::from)?;
        while let Some(task) = projection.tasks.pop() {
            projection.step(task)?;
        }
        let value = projection.results.pop().expect("one projected root");
        projection
            .memory
            .release_vec(std::mem::take(&mut projection.results))?;
        std::mem::replace(&mut projection.tasks, WorkList::new())
            .release_admitted(projection.memory)?;
        Ok(value)
    })
}
impl<'a> Projection<'a, '_, '_> {
    fn string(&mut self, text: &str) -> Result<JsonValue, RdfDiagnostic> {
        Ok(JsonValue::String(self.memory.string(text)?))
    }
    fn field(
        &mut self,
        members: &mut Members,
        key: &str,
        value: JsonValue,
    ) -> Result<(), RdfDiagnostic> {
        let key = self.memory.string(key)?;
        self.memory.push(members, (key, value))?;
        Ok(())
    }
    fn key(&mut self, keys: &mut Vec<String>, key: &str) -> Result<(), RdfDiagnostic> {
        let key = self.memory.string(key)?;
        self.memory.push(keys, key)?;
        Ok(())
    }
    fn schedule(&mut self, fields: Fields, tasks: Vec<Task<'a>>) -> Result<(), RdfDiagnostic> {
        self.tasks
            .try_push_admitted(Task::Fields(fields), self.memory)
            .map_err(RdfDiagnostic::from)?;
        let mut tasks = tasks;
        tasks.reverse();
        while let Some(task) = tasks.pop() {
            self.tasks
                .try_push_admitted(task, self.memory)
                .map_err(RdfDiagnostic::from)?;
        }
        self.memory.release_vec(tasks)?;
        Ok(())
    }
    fn step(&mut self, task: Task<'a>) -> Result<(), RdfDiagnostic> {
        match task {
            Task::Group(fields, tasks) => self.schedule(fields, tasks),
            Task::GroupArray(mut tasks) => {
                self.tasks
                    .try_push_admitted(Task::Array(tasks.len()), self.memory)
                    .map_err(RdfDiagnostic::from)?;
                tasks.reverse();
                while let Some(task) = tasks.pop() {
                    self.tasks
                        .try_push_admitted(task, self.memory)
                        .map_err(RdfDiagnostic::from)?;
                }
                self.memory.release_vec(tasks)?;
                Ok(())
            }
            Task::Fields(mut fields) => {
                for key in fields.keys.drain(..) {
                    let value = self.results.pop().expect("one projection per field");
                    self.memory.push(&mut fields.members, (key, value))?;
                }
                self.memory.release_vec(fields.keys)?;
                let value = ordered_object(fields.members);
                self.memory
                    .push(&mut self.results, value)
                    .map_err(RdfDiagnostic::from)
            }
            Task::Array(count) => {
                let mut values = Vec::new();
                self.memory.reserve(&mut values, count)?;
                for _ in 0..count {
                    values.push(self.results.pop().expect("one projection per array item"));
                }
                self.memory
                    .push(&mut self.results, JsonValue::Array(values))
                    .map_err(RdfDiagnostic::from)
            }
            Task::Annotated(count) => {
                let mut nodes = Vec::new();
                self.memory.reserve(&mut nodes, count)?;
                for _ in 0..count {
                    nodes.push(self.results.pop().expect("one projected annotation"));
                }
                let mut term = self.results.pop().expect("annotated term projection");
                let object = term
                    .as_object_mut()
                    .expect("all carrier terms project objects");
                let mut fields = std::mem::take(object).into_members();
                self.field(&mut fields, "@annotation", JsonValue::Array(nodes))?;
                let value = ordered_object(fields);
                self.memory
                    .push(&mut self.results, value)
                    .map_err(RdfDiagnostic::from)
            }
            Task::Value(value, context) => {
                if value.annotations.is_empty() {
                    return self
                        .tasks
                        .try_push_admitted(Task::Term(&value.term, context), self.memory)
                        .map_err(RdfDiagnostic::from);
                }
                self.tasks
                    .try_push_admitted(Task::Annotated(value.annotations.len()), self.memory)
                    .map_err(RdfDiagnostic::from)?;
                for annotation in &value.annotations {
                    self.tasks
                        .try_push_admitted(
                            Task::Node(annotation, None, context.clone()),
                            self.memory,
                        )
                        .map_err(RdfDiagnostic::from)?;
                }
                self.tasks
                    .try_push_admitted(Task::Term(&value.term, context), self.memory)
                    .map_err(RdfDiagnostic::from)
            }
            Task::Values(values, context) => {
                self.tasks
                    .try_push_admitted(Task::Array(values.len()), self.memory)
                    .map_err(RdfDiagnostic::from)?;
                for value in values {
                    self.tasks
                        .try_push_admitted(Task::Value(value, context.clone()), self.memory)
                        .map_err(RdfDiagnostic::from)?;
                }
                Ok(())
            }
            Task::Term(term, context) => {
                let mut members = Vec::new();
                match term {
                    Term::Id(id) => {
                        let value = self.string(id)?;
                        self.field(&mut members, "@id", value)?;
                    }
                    Term::Literal(literal) => {
                        let value = self.string(&literal.lexical)?;
                        self.field(&mut members, "@value", value)?;
                        for (key, text) in [
                            ("@language", literal.language.as_deref()),
                            ("@direction", literal.direction.as_deref()),
                            ("@type", literal.datatype.as_deref()),
                        ] {
                            if let Some(text) = text {
                                let value = self.string(text)?;
                                self.field(&mut members, key, value)?;
                            }
                        }
                    }
                    Term::List(values) => {
                        let mut keys = Vec::new();
                        self.key(&mut keys, "@list")?;
                        self.tasks
                            .try_push_admitted(Task::Fields(Fields { members, keys }), self.memory)
                            .map_err(RdfDiagnostic::from)?;
                        return self
                            .tasks
                            .try_push_admitted(Task::Values(values, context), self.memory)
                            .map_err(RdfDiagnostic::from);
                    }
                    Term::Triple(triple) => {
                        let mut keys = Vec::new();
                        self.key(&mut keys, "@triple")?;
                        self.tasks
                            .try_push_admitted(Task::Fields(Fields { members, keys }), self.memory)
                            .map_err(RdfDiagnostic::from)?;
                        let mut members = Vec::new();
                        let predicate = self.string(&triple.predicate)?;
                        self.field(&mut members, "@predicate", predicate)?;
                        let mut keys = Vec::new();
                        self.key(&mut keys, "@subject")?;
                        self.key(&mut keys, "@object")?;
                        self.tasks
                            .try_push_admitted(Task::Fields(Fields { members, keys }), self.memory)
                            .map_err(RdfDiagnostic::from)?;
                        self.tasks
                            .try_push_admitted(
                                Task::Term(&triple.subject, context.clone()),
                                self.memory,
                            )
                            .map_err(RdfDiagnostic::from)?;
                        return self
                            .tasks
                            .try_push_admitted(Task::Term(&triple.object, context), self.memory)
                            .map_err(RdfDiagnostic::from);
                    }
                }
                let value = ordered_object(members);
                self.memory
                    .push(&mut self.results, value)
                    .map_err(RdfDiagnostic::from)
            }
            Task::Graph(graph, keep_id, index, context) => {
                let mut members = Vec::new();
                if keep_id {
                    let id = self.string(&graph.id)?;
                    self.field(&mut members, "@id", id)?;
                }
                if let Some((mapping, key)) = index {
                    let mut lexical = Vec::new();
                    let value = self.string(key)?;
                    self.field(&mut lexical, "@value", value)?;
                    let value = ordered_object(lexical);
                    let mut values = Vec::new();
                    self.memory.push(&mut values, value)?;
                    self.field(&mut members, mapping, JsonValue::Array(values))?;
                }
                let mut keys = Vec::new();
                self.key(&mut keys, "@graph")?;
                self.tasks
                    .try_push_admitted(Task::Fields(Fields { members, keys }), self.memory)
                    .map_err(RdfDiagnostic::from)?;
                self.tasks
                    .try_push_admitted(Task::Array(graph.nodes.len()), self.memory)
                    .map_err(RdfDiagnostic::from)?;
                for node in &graph.nodes {
                    self.tasks
                        .try_push_admitted(
                            Task::Node(node, Some(graph.id.as_str()), context.clone()),
                            self.memory,
                        )
                        .map_err(RdfDiagnostic::from)?;
                }
                Ok(())
            }
            Task::Node(node, graph, context) => {
                let mut fields = Fields {
                    members: Vec::new(),
                    keys: Vec::new(),
                };
                let id = self.string(&node.id)?;
                self.field(&mut fields.members, "@id", id)?;
                if !node.types.is_empty() {
                    let mut types = Vec::new();
                    self.memory.reserve(&mut types, node.types.len())?;
                    for datatype in &node.types {
                        types.push(self.string(datatype)?);
                    }
                    self.field(&mut fields.members, "@type", JsonValue::Array(types))?;
                }
                let mut tasks = Vec::new();
                for (property, values) in &node.properties {
                    if let Some(graph_tasks) =
                        self.graph_tasks(node, graph, property, values, &context)?
                    {
                        self.key(&mut fields.keys, property)?;
                        self.memory
                            .push(&mut tasks, Task::GroupArray(graph_tasks))?;
                        continue;
                    }
                    if self.scope.policy.compact_arrays
                        && let [value] = values.as_slice()
                        && value.annotations.is_empty()
                        && let Term::Literal(literal) = &value.term
                        && literal.datatype.as_deref() == Some(super::super::RDF_JSON)
                        && let Some(parsed) = canonical(&literal.lexical, self.memory)?
                    {
                        let mut candidate = Vec::new();
                        let datatype = self.string("@json")?;
                        self.field(&mut candidate, "@type", datatype)?;
                        self.field(&mut candidate, "@value", parsed)?;
                        let mut candidate = ordered_object(candidate);
                        let term =
                            context.compact_property_iri(property, &candidate, false, true, &[])?;
                        if context.term(&term).is_some_and(|definition| {
                            definition.type_mapping() == Some(&JsonLdTypeMapping::Json)
                                && !definition.containers().contains(&JsonLdContainer::Set)
                        }) {
                            let mut singleton = Vec::new();
                            self.memory.push(&mut singleton, candidate)?;
                            self.field(&mut fields.members, property, JsonValue::Array(singleton))?;
                            continue;
                        }
                        candidate.release_with_memory(self.memory)?;
                    }
                    self.key(&mut fields.keys, property)?;
                    self.memory
                        .push(&mut tasks, Task::Values(values, context.clone()))?;
                }
                if !node.reverse_properties.is_empty() {
                    self.key(&mut fields.keys, "@reverse")?;
                    // A reverse map has the same expanded property arrays, without a node ID.
                    let mut reverse = Fields {
                        members: Vec::new(),
                        keys: Vec::new(),
                    };
                    let mut reverse_tasks = Vec::new();
                    for (property, values) in &node.reverse_properties {
                        self.key(&mut reverse.keys, property)?;
                        self.memory
                            .push(&mut reverse_tasks, Task::Values(values, context.clone()))?;
                    }
                    // The same grouping task represents both node fields and reverse fields.
                    self.memory
                        .push(&mut tasks, Task::Group(reverse, reverse_tasks))?;
                }
                self.schedule(fields, tasks)
            }
        }
    }
    fn graph_tasks(
        &mut self,
        node: &Node,
        graph: Option<&str>,
        property: &str,
        values: &'a [Value],
        context: &Context<'a>,
    ) -> Result<Option<Vec<Task<'a>>>, RdfDiagnostic> {
        if values.is_empty() || values.iter().any(|value|!value.annotations.is_empty() || !matches!(&value.term,Term::Id(id) if self.scope.named_by_id.contains_key(id.as_str()))) {return Ok(None);}
        let selection = JsonLdTermSelection::new(
            [
                vec![
                    JsonLdContainer::Graph,
                    JsonLdContainer::Index,
                    JsonLdContainer::Set,
                ],
                vec![JsonLdContainer::Graph, JsonLdContainer::Index],
                vec![
                    JsonLdContainer::Graph,
                    JsonLdContainer::Id,
                    JsonLdContainer::Set,
                ],
                vec![JsonLdContainer::Graph, JsonLdContainer::Id],
                vec![JsonLdContainer::Graph, JsonLdContainer::Set],
                vec![JsonLdContainer::Graph],
            ],
            JsonLdTermSelectionKind::Type,
            ["@none", "@any"],
        );
        let term = context.compact_iri_with_selection(property, true, Some(&selection))?;
        let Some(definition) = context
            .term(&term)
            .filter(|definition| definition.containers().contains(&JsonLdContainer::Graph))
        else {
            return Ok(None);
        };
        let id_map = definition.containers().contains(&JsonLdContainer::Id);
        let index_map = definition.containers().contains(&JsonLdContainer::Index);
        let lookup = (
            graph.map(|graph| self.memory.string(graph)).transpose()?,
            self.memory.string(&node.id)?,
            self.memory.string(property)?,
        );
        let plan = self.scope.graph_index_plans.get(&lookup);
        if let Some(graph) = lookup.0 {
            self.memory.release_string(graph)?;
        }
        self.memory.release_string(lookup.1)?;
        self.memory.release_string(lookup.2)?;
        if index_map && plan.is_none() {
            return Ok(None);
        }
        for value in values {
            let Term::Id(id) = &value.term else {
                unreachable!("graph reference eligibility checked");
            };
            if self.embedded.contains(id)
                || !id_map
                    && (!id.starts_with("_:") || self.scope.reference_counts.get(id) != Some(&1))
            {
                return Ok(None);
            }
        }
        let value_context = context.scoped_context(&term)?.map_or_else(
            || context.clone(),
            |context| Context::Scoped(Rc::new(context)),
        );
        let mut tasks = Vec::new();
        self.memory.reserve(&mut tasks, values.len())?;
        for value in values {
            let Term::Id(id) = &value.term else {
                unreachable!("graph reference eligibility checked");
            };
            self.embedded.insert(id.clone());
            let named = self
                .scope
                .named_by_id
                .get(id.as_str())
                .expect("named graph eligibility checked");
            let index = plan.and_then(|plan| {
                plan.keys_by_graph
                    .get(id)
                    .expect("graph-index plan covers reference")
                    .as_deref()
                    .map(|key| (plan.index_mapping.as_str(), key))
            });
            tasks.push(Task::Graph(named, id_map, index, value_context.clone()));
        }
        Ok(Some(tasks))
    }
}

/// Existing lexical eligibility, constructed through the projection account.
/// Number spelling inspection stays in the codec's shared numeric home.
pub(super) fn canonical(
    lexical: &str,
    memory: &mut Memory<'_, Ceiling>,
) -> Result<Option<JsonValue>, RdfDiagnostic> {
    let mut reader = json::Reader::new(
        lexical,
        json::Limits {
            max_depth: 128,
            max_values: u64::MAX,
            max_string_bytes: usize::MAX,
            unique_members: true,
        },
    );
    let mut value = match reader.read_value_with_memory(memory) {
        Ok(value) => value,
        Err(error) => {
            return match error.kind() {
                json::ErrorKind::Storage(storage)
                | json::ErrorKind::Escape(purrdf_lex::json_escape::JsonEscapeErrorKind::Storage(
                    storage,
                )) => Err(storage.into()),
                _ => Ok(None),
            };
        }
    };
    if let Err(error) = reader.finish_with_memory(memory) {
        value.release_with_memory(memory)?;
        return match error.kind() {
            json::ErrorKind::Storage(storage)
            | json::ErrorKind::Escape(purrdf_lex::json_escape::JsonEscapeErrorKind::Storage(
                storage,
            )) => Err(storage.into()),
            _ => Ok(None),
        };
    }
    let mut canonical_numbers = true;
    let mut work = WorkList::<&mut JsonValue, 8>::new();
    work.try_push_admitted(&mut value, memory)?;
    while let Some(at) = work.pop() {
        match at {
            JsonValue::Number(number) => {
                let text = crate::json_number::binary64_lexeme_with_memory(number, memory)?;
                if let Some(text) = text {
                    canonical_numbers &= text == number.lexeme();
                    memory.release_string(text)?;
                } else {
                    canonical_numbers = false;
                }
            }
            JsonValue::Object(object) => {
                let mut members = std::mem::take(object).into_members();
                members.sort_unstable_by(|a, b| a.0.cmp(&b.0));
                *object = Object::from(members);
                for child in object.values_mut() {
                    work.try_push_admitted(child, memory)?;
                }
            }
            JsonValue::Array(items) => {
                for child in items {
                    work.try_push_admitted(child, memory)?;
                }
            }
            _ => {}
        }
    }
    work.release_admitted(memory)?;
    let mut text = String::new();
    json::write_into_with_memory(&mut text, &value, json::Format::COMPACT, memory)?;
    let eligible = canonical_numbers && text == lexical;
    memory.release_string(text)?;
    if eligible {
        Ok(Some(value))
    } else {
        value.release_with_memory(memory)?;
        Ok(None)
    }
}
