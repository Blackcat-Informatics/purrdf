// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! JSON-LD 1.1 API §§7.1–7.2, shared by native flattening and RDF conversion.

use super::{RdfDiagnostic, decode, ordered_object};
use purrdf_core::hash::{hash_table_retained_bound, reserve_table_with_memory, table::HashTable};
use purrdf_hash::fixed::{FixedHasher, hash_one};
use purrdf_lex::allocation::{Ceiling, Memory, StorageError};
use purrdf_lex::json::{Object, Value};
use purrdf_lex::walk::WorkList;
use std::hash::Hasher;

/// Construction allowance for the native node map and flattened output.
/// Expanded input and later RDF materialization have separate authorities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonLdFlattenLimits {
    /// Maximum live requested layouts of newly constructed map, issuer and output storage.
    pub max_allocation_bytes: u64,
}
impl JsonLdFlattenLimits {
    /// The default node-map construction allowance is 32 GiB.
    pub const DEFAULT: Self = Self {
        max_allocation_bytes: 32 * 1024 * 1024 * 1024,
    };
}
impl Default for JsonLdFlattenLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Clone, Copy)]
pub(super) enum Issuance {
    Normative,
    Rdf,
}

pub(super) struct Mapped {
    pub(super) value: Value,
    pub(super) semantic_index: bool,
}
#[derive(Clone, Copy)]
enum Mode {
    Node,
    Detached,
    Atomic,
}
type Members = Vec<(String, Value)>;
type Entries<'a> = Vec<(&'a str, &'a Value)>;
pub(super) struct Node {
    pub(super) id: String,
    pub(super) members: Members,
}
pub(super) struct Graph {
    pub(super) name: Option<String>,
    pub(super) nodes: Vec<usize>,
}
/// The original graph-local map, before flattening consumes descriptions.
pub(super) struct NodeMap {
    pub(super) graphs: Vec<Graph>,
    pub(super) nodes: Vec<Node>,
}
enum Built {
    Flattened(Mapped),
    Graphs(NodeMap),
}
struct Index {
    graph: usize,
    id: String,
    slot: usize,
}
struct GraphIndex {
    name: String,
    slot: usize,
}
struct Issued {
    source: String,
    assigned: String,
}
struct Candidate {
    node: usize,
    member: usize,
    value: usize,
    semantic_hash: u64,
}
struct MemberIndex {
    node: usize,
    slot: usize,
    hash: u64,
}

struct Fields<'a> {
    entries: Entries<'a>,
    at: usize,
    node: usize,
    graph: usize,
    detached: bool,
    prelinked: bool,
    raw: &'a Object,
}
struct Generic<'a> {
    entries: Entries<'a>,
    at: usize,
    members: Members,
    graph: usize,
    mode: Mode,
}
#[derive(Clone, Copy)]
enum Link<'a> {
    Forward { node: usize, member: usize },
    Reverse { node: usize, key: &'a str },
}
enum Task<'a> {
    Visit(&'a Value, usize, Mode),
    Array {
        items: &'a [Value],
        at: usize,
        start: usize,
        graph: usize,
        mode: Mode,
    },
    Fields(Fields<'a>),
    Linked(&'a Value, usize, Link<'a>),
    LinkedItems {
        items: &'a [Value],
        at: usize,
        graph: usize,
        link: Link<'a>,
    },
    FinishLinked(usize, Link<'a>),
    Reverse {
        entries: Entries<'a>,
        at: usize,
        node: usize,
        graph: usize,
    },
    Generic(Generic<'a>),
    GenericAssign(Generic<'a>, String),
    Annotate(Value),
    Discard,
}

pub(super) fn flatten(
    input: &Value,
    limits: JsonLdFlattenLimits,
    issuance: Issuance,
) -> Result<Mapped, RdfDiagnostic> {
    match build(input, limits, issuance, false)? {
        Built::Flattened(mapped) => Ok(mapped),
        Built::Graphs(_) => unreachable!("flatten finish selected"),
    }
}

pub(super) fn build_graphs(
    input: &Value,
    limits: JsonLdFlattenLimits,
) -> Result<NodeMap, RdfDiagnostic> {
    match build(input, limits, Issuance::Normative, true)? {
        Built::Graphs(map) => Ok(map),
        Built::Flattened(_) => unreachable!("graph finish selected"),
    }
}

fn build(
    input: &Value,
    limits: JsonLdFlattenLimits,
    issuance: Issuance,
    graphs: bool,
) -> Result<Built, RdfDiagnostic> {
    let mut ceiling = Ceiling(usize::try_from(limits.max_allocation_bytes).unwrap_or(usize::MAX));
    Memory::new(&mut ceiling).try_scope(|memory| {
        let mut walk = Walk {
            memory,
            graphs: Vec::new(),
            nodes: Vec::new(),
            index: HashTable::new(),
            graph_index: HashTable::new(),
            issued: HashTable::new(),
            candidates: HashTable::new(),
            members: HashTable::new(),
            next: 0,
            semantic_index: false,
            issuance,
            tasks: WorkList::new(),
            results: Vec::new(),
        };
        walk.memory.push(
            &mut walk.graphs,
            Graph {
                name: None,
                nodes: Vec::new(),
            },
        )?;
        if matches!(issuance, Issuance::Rdf) {
            walk.reserve_input_labels(input)?;
        }
        walk.tasks
            .try_push_admitted(Task::Visit(input, 0, Mode::Node), walk.memory)
            .map_err(RdfDiagnostic::from)?;
        while let Some(task) = walk.tasks.pop() {
            walk.step(task)?;
        }
        // Root visits return references; graph records own the actual descriptions.
        let root = walk.results.pop().expect("one node-map root result");
        walk.discard_roots(root)?;
        walk.memory.release_vec(std::mem::take(&mut walk.results))?;
        std::mem::replace(&mut walk.tasks, WorkList::new()).release_admitted(walk.memory)?;
        walk.finish(graphs)
    })
}

struct Walk<'a, 'm, 's> {
    memory: &'m mut Memory<'s, Ceiling>,
    graphs: Vec<Graph>,
    nodes: Vec<Node>,
    index: HashTable<Index>,
    graph_index: HashTable<GraphIndex>,
    issued: HashTable<Issued>,
    candidates: HashTable<Candidate>,
    members: HashTable<MemberIndex>,
    next: u64,
    semantic_index: bool,
    issuance: Issuance,
    tasks: WorkList<Task<'a>, 8>,
    results: Vec<Value>,
}
impl<'a> Walk<'a, '_, '_> {
    fn entries(&mut self, object: &'a Object) -> Result<Entries<'a>, RdfDiagnostic> {
        let mut entries = Vec::new();
        self.memory.reserve(&mut entries, object.len())?;
        entries.extend(object.iter().map(|(key, value)| (key.as_str(), value)));
        entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
        Ok(entries)
    }
    fn issued_insert(&mut self, source: String, assigned: String) -> Result<(), RdfDiagnostic> {
        let required = self
            .issued
            .len()
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?;
        reserve_table_with_memory(
            &mut self.issued,
            required,
            |entry| hash_one(&entry.source),
            self.memory,
        )?;
        self.issued
            .insert_unique(hash_one(&source), Issued { source, assigned }, |entry| {
                hash_one(&entry.source)
            });
        Ok(())
    }
    fn reserve_input_labels(&mut self, input: &Value) -> Result<(), RdfDiagnostic> {
        // Preserve RDF issuance's collision reservation over ALL original strings,
        // including literal text and opaque JSON. Reservation never rewrites them.
        let mut pending: WorkList<_, 8> = WorkList::new();
        pending.try_push_admitted(input, self.memory)?;
        while let Some(value) = pending.pop() {
            match value {
                Value::String(text) if text.starts_with("_:") => {
                    if self
                        .issued
                        .find(hash_one(text), |entry| entry.source == *text)
                        .is_none()
                    {
                        let source = self.memory.string(text)?;
                        self.issued_insert(source, String::new())?;
                    }
                }
                Value::Array(values) => {
                    for value in values {
                        pending.try_push_admitted(value, self.memory)?;
                    }
                }
                Value::Object(object) => {
                    for value in object.values() {
                        pending.try_push_admitted(value, self.memory)?;
                    }
                }
                _ => {}
            }
        }
        pending.release_admitted(self.memory)?;
        Ok(())
    }
    fn identifier(&mut self, source: Option<&str>) -> Result<String, RdfDiagnostic> {
        if let Some(source) = source {
            if !source.starts_with("_:") || matches!(self.issuance, Issuance::Rdf) {
                return Ok(self.memory.string(source)?);
            }
            if let Some(entry) = self
                .issued
                .find(hash_one(source), |entry| entry.source == source)
            {
                return Ok(self.memory.string(&entry.assigned)?);
            }
        }
        let assigned = loop {
            let prefix = if matches!(self.issuance, Issuance::Normative) {
                "b"
            } else {
                "jsonld"
            };
            let candidate = self
                .memory
                .format(&format_args!("_:{prefix}{}", self.next))?;
            self.next = self
                .next
                .checked_add(1)
                .ok_or_else(|| decode("JSON-LD blank identifier sequence exhausted"))?;
            if matches!(self.issuance, Issuance::Normative)
                || self
                    .issued
                    .find(hash_one(&candidate), |entry| entry.source == candidate)
                    .is_none()
            {
                break candidate;
            }
            self.memory.release_string(candidate)?;
        };
        if let Some(source) = source {
            let source = self.memory.string(source)?;
            let copy = self.memory.string(&assigned)?;
            self.issued_insert(source, copy)?;
        }
        Ok(assigned)
    }
    fn identifiers(&mut self, value: &Value) -> Result<Value, RdfDiagnostic> {
        if let Some(text) = value.as_str() {
            return Ok(Value::String(self.identifier(Some(text))?));
        }
        let values = value
            .as_array()
            .ok_or_else(|| decode("expanded identifier must be a string or array"))?;
        let mut output = Vec::new();
        self.memory.reserve(&mut output, values.len())?;
        for value in values {
            let text = value
                .as_str()
                .ok_or_else(|| decode("expanded identifier array must contain strings"))?;
            output.push(Value::String(self.identifier(Some(text))?));
        }
        Ok(Value::Array(output))
    }
    fn reference(&mut self, node: usize) -> Result<Value, RdfDiagnostic> {
        let mut members = Vec::new();
        let key = self.memory.string("@id")?;
        let id = self.memory.string(&self.nodes[node].id)?;
        self.memory.push(&mut members, (key, Value::String(id)))?;
        Ok(Value::Object(Object::from(members)))
    }
    fn new_node(&mut self, graph: usize, id: &str, detached: bool) -> Result<usize, RdfDiagnostic> {
        let hash = hash_one(&(graph, id));
        if !detached
            && let Some(entry) = self
                .index
                .find(hash, |entry| entry.graph == graph && entry.id == id)
        {
            return Ok(entry.slot);
        }
        let slot = self.nodes.len();
        let id = self.memory.string(id)?;
        let mut members = Vec::new();
        let key = self.memory.string("@id")?;
        let value = Value::String(self.memory.string(&id)?);
        self.memory.push(&mut members, (key, value))?;
        self.memory.push(&mut self.nodes, Node { id, members })?;
        if !detached {
            let required = self
                .index
                .len()
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?;
            reserve_table_with_memory(
                &mut self.index,
                required,
                |entry| hash_one(&(entry.graph, entry.id.as_str())),
                self.memory,
            )?;
            let id = self.memory.string(&self.nodes[slot].id)?;
            self.index
                .insert_unique(hash, Index { graph, id, slot }, |entry| {
                    hash_one(&(entry.graph, entry.id.as_str()))
                });
            self.memory.push(&mut self.graphs[graph].nodes, slot)?;
        }
        Ok(slot)
    }
    fn graph(&mut self, name: String) -> Result<usize, RdfDiagnostic> {
        if let Some(entry) = self
            .graph_index
            .find(hash_one(&name), |entry| entry.name == name)
        {
            let index = entry.slot;
            self.memory.release_string(name)?;
            return Ok(index);
        }
        let index = self.graphs.len();
        let required = self
            .graph_index
            .len()
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?;
        reserve_table_with_memory(
            &mut self.graph_index,
            required,
            |entry| hash_one(&entry.name),
            self.memory,
        )?;
        let copy = self.memory.string(&name)?;
        self.graph_index.insert_unique(
            hash_one(&name),
            GraphIndex {
                name: copy,
                slot: index,
            },
            |entry| hash_one(&entry.name),
        );
        self.memory.push(
            &mut self.graphs,
            Graph {
                name: Some(name),
                nodes: Vec::new(),
            },
        )?;
        Ok(index)
    }
    fn find_member(&self, node: usize, key: &str) -> Option<usize> {
        let hash = hash_one(&(node, key));
        self.members
            .find(hash, |entry| {
                entry.hash == hash
                    && entry.node == node
                    && self.nodes[node].members[entry.slot].0 == key
            })
            .map(|entry| entry.slot)
    }
    fn member_slot(&mut self, node: usize, key: &str) -> Result<usize, RdfDiagnostic> {
        if let Some(slot) = self.find_member(node, key) {
            return Ok(slot);
        }
        let required = self
            .members
            .len()
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)?;
        // Rehash uses only retained integers, never already-emitted detached
        // members. Node slots are unique and never reused; exact name lookup
        // short-circuits on the node before accessing its live member storage.
        reserve_table_with_memory(&mut self.members, required, |entry| entry.hash, self.memory)?;
        let slot = self.nodes[node].members.len();
        let name = self.memory.string(key)?;
        self.memory.push(
            &mut self.nodes[node].members,
            (name, Value::Array(Vec::new())),
        )?;
        let hash = hash_one(&(node, key));
        self.members
            .insert_unique(hash, MemberIndex { node, slot, hash }, |entry| entry.hash);
        Ok(slot)
    }
    fn append(&mut self, node: usize, key: &str, mut values: Value) -> Result<(), RdfDiagnostic> {
        let member = self.member_slot(node, key)?;
        if let Value::Array(children) = &mut values {
            for child in children.drain(..) {
                self.append_one(node, member, child)?;
            }
            self.memory.release_vec(std::mem::take(children))?;
        } else {
            self.append_one(node, member, values)?;
        }
        Ok(())
    }
    fn append_one(
        &mut self,
        node: usize,
        member: usize,
        mut value: Value,
    ) -> Result<(), RdfDiagnostic> {
        // The caller admitted and selected this array member.
        let target = self.nodes[node].members[member]
            .1
            .as_array_mut()
            .expect("node-map property array");
        if !value
            .as_object()
            .is_some_and(|object| object.contains_key("@list"))
        {
            let mut hasher = FixedHasher::default();
            value.hash_with_memory(&mut hasher, self.memory)?;
            let semantic_hash = hasher.finish();
            let hash = hash_one(&(node, member, semantic_hash));
            for candidate in self.candidates.iter_hash(hash) {
                if candidate.node == node
                    && candidate.member == member
                    && candidate.semantic_hash == semantic_hash
                    && target[candidate.value].eq_with_memory(&value, self.memory)?
                {
                    value.release_with_memory(self.memory)?;
                    return Ok(());
                }
            }
            let required = self
                .candidates
                .len()
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?;
            reserve_table_with_memory(
                &mut self.candidates,
                required,
                |entry| hash_one(&(entry.node, entry.member, entry.semantic_hash)),
                self.memory,
            )?;
            let index = target.len();
            self.memory.push(target, value)?;
            self.candidates.insert_unique(
                hash,
                Candidate {
                    node,
                    member,
                    semantic_hash,
                    value: index,
                },
                |entry| hash_one(&(entry.node, entry.member, entry.semantic_hash)),
            );
            return Ok(());
        }
        self.memory.push(target, value)?;
        Ok(())
    }
    fn link_value(
        &mut self,
        graph: usize,
        link: Link<'a>,
        mut value: Value,
    ) -> Result<(), RdfDiagnostic> {
        match link {
            Link::Forward { node, member } => self.append_one(node, member, value),
            Link::Reverse { node, key } => {
                let object = value
                    .as_object_mut()
                    .ok_or_else(|| decode("reverse property requires node values"))?;
                let mut id = object
                    .get_mut("@id")
                    .ok_or_else(|| decode("reverse property requires identifiers"))?
                    .take();
                let target = self.new_node(
                    graph,
                    id.as_str()
                        .ok_or_else(|| decode("reverse identifier must be a string"))?,
                    false,
                )?;
                let source = self.memory.string(&self.nodes[node].id)?;
                *object.get_mut("@id").expect("retained reverse @id member") =
                    Value::String(source);
                id.release_with_memory(self.memory)?;
                let property = self.identifier(Some(key))?;
                let member = self.member_slot(target, &property)?;
                self.memory.release_string(property)?;
                self.append_one(target, member, value)
            }
        }
    }
    fn step(&mut self, task: Task<'a>) -> Result<(), RdfDiagnostic> {
        match task {
            Task::Visit(raw, graph, mode) => self.visit(raw, graph, mode, None)?,
            Task::Array {
                items,
                at,
                start,
                graph,
                mode,
            } => {
                if let Some(raw) = items.get(at) {
                    self.tasks
                        .try_push_admitted(
                            Task::Array {
                                items,
                                at: at + 1,
                                start,
                                graph,
                                mode,
                            },
                            self.memory,
                        )
                        .map_err(RdfDiagnostic::from)?;
                    self.tasks
                        .try_push_admitted(Task::Visit(raw, graph, mode), self.memory)
                        .map_err(RdfDiagnostic::from)?;
                } else {
                    let mut values = Vec::new();
                    self.memory
                        .reserve(&mut values, self.results.len() - start)?;
                    values.extend(self.results.drain(start..));
                    self.memory
                        .push(&mut self.results, Value::Array(values))
                        .map_err(RdfDiagnostic::from)?;
                }
            }
            Task::Fields(fields) => self.fields(fields)?,
            Task::Linked(raw, graph, link) => {
                if let Value::Array(items) = raw {
                    self.tasks
                        .try_push_admitted(
                            Task::LinkedItems {
                                items,
                                at: 0,
                                graph,
                                link,
                            },
                            self.memory,
                        )
                        .map_err(RdfDiagnostic::from)?;
                } else {
                    let early = raw.as_object().is_some_and(|object| {
                        !["@value", "@list", "@triple", "@annotation"]
                            .iter()
                            .any(|key| object.contains_key(key))
                    });
                    if !early {
                        self.tasks
                            .try_push_admitted(Task::FinishLinked(graph, link), self.memory)
                            .map_err(RdfDiagnostic::from)?;
                    }
                    self.visit(raw, graph, Mode::Node, early.then_some(link))?;
                }
            }
            Task::LinkedItems {
                items,
                at,
                graph,
                link,
            } => {
                if let Some(raw) = items.get(at) {
                    self.tasks
                        .try_push_admitted(
                            Task::LinkedItems {
                                items,
                                at: at + 1,
                                graph,
                                link,
                            },
                            self.memory,
                        )
                        .map_err(RdfDiagnostic::from)?;
                    self.tasks
                        .try_push_admitted(Task::Linked(raw, graph, link), self.memory)
                        .map_err(RdfDiagnostic::from)?;
                }
            }
            Task::FinishLinked(graph, link) => {
                let value = self.results.pop().expect("normalized linked occurrence");
                self.link_value(graph, link, value)?;
            }
            Task::Reverse {
                entries,
                at,
                node,
                graph,
            } => {
                if let Some(&(key, raw)) = entries.get(at) {
                    let property = self.identifier(Some(key))?;
                    self.memory.release_string(property)?;
                    self.tasks
                        .try_push_admitted(
                            Task::Reverse {
                                entries,
                                at: at + 1,
                                node,
                                graph,
                            },
                            self.memory,
                        )
                        .map_err(RdfDiagnostic::from)?;
                    self.tasks
                        .try_push_admitted(
                            Task::Linked(raw, graph, Link::Reverse { node, key }),
                            self.memory,
                        )
                        .map_err(RdfDiagnostic::from)?;
                } else {
                    self.memory.release_vec(entries)?;
                }
            }
            Task::Generic(plan) => self.generic(plan)?,
            Task::GenericAssign(mut plan, key) => {
                let value = self.results.pop().expect("normalized object member");
                self.memory.push(&mut plan.members, (key, value))?;
                self.tasks
                    .try_push_admitted(Task::Generic(plan), self.memory)
                    .map_err(RdfDiagnostic::from)?;
            }
            Task::Annotate(mut value) => {
                let annotations = self.results.pop().expect("detached annotations");
                let object = value.as_object_mut().expect("normalized annotated object");
                let mut members = std::mem::take(object).into_members();
                let key = self.memory.string("@annotation")?;
                self.memory.push(&mut members, (key, annotations))?;
                self.memory
                    .push(&mut self.results, ordered_object(members))
                    .map_err(RdfDiagnostic::from)?;
            }
            Task::Discard => {
                let value = self.results.pop().expect("discarded graph references");
                self.discard_roots(value)?;
            }
        }
        Ok(())
    }
    fn discard_roots(&mut self, mut value: Value) -> Result<(), RdfDiagnostic> {
        let mut pending: WorkList<_, 8> = WorkList::new();
        pending.try_push_admitted(&value, self.memory)?;
        while let Some(value) = pending.pop() {
            match value {
                Value::Array(values) => {
                    for value in values {
                        pending.try_push_admitted(value, self.memory)?;
                    }
                }
                Value::Object(object)
                    if object.len() == 1 && object.get("@id").and_then(Value::as_str).is_some() => {
                }
                _ => {
                    return Err(decode(
                        "graph roots require plain node references without annotations or native term wrappers",
                    ));
                }
            }
        }
        pending.release_admitted(self.memory)?;
        value.release_with_memory(self.memory)?;
        Ok(())
    }
    fn visit(
        &mut self,
        raw: &'a Value,
        graph: usize,
        mode: Mode,
        link: Option<Link<'a>>,
    ) -> Result<(), RdfDiagnostic> {
        if let Value::Array(items) = raw {
            let task = Task::Array {
                items,
                at: 0,
                start: self.results.len(),
                graph,
                mode,
            };
            return self
                .tasks
                .try_push_admitted(task, self.memory)
                .map_err(RdfDiagnostic::from);
        }
        let Some(object) = raw.as_object() else {
            if matches!(mode, Mode::Atomic) {
                let value = raw.clone_with_memory(self.memory)?;
                return self
                    .memory
                    .push(&mut self.results, value)
                    .map_err(RdfDiagnostic::from);
            }
            return Err(decode("expanded node-map element must be an object"));
        };
        if matches!(mode, Mode::Atomic)
            || object.contains_key("@value")
            || object.contains_key("@list")
            || object.contains_key("@triple")
        {
            let entries = self.entries(object)?;
            return self
                .tasks
                .try_push_admitted(
                    Task::Generic(Generic {
                        entries,
                        at: 0,
                        members: Vec::new(),
                        graph,
                        mode,
                    }),
                    self.memory,
                )
                .map_err(RdfDiagnostic::from);
        }
        // §7.2 remaps type blank identifiers BEFORE issuing the node identifier.
        if let Some(types) = object.get("@type") {
            let mut mapped = self.identifiers(types)?;
            mapped.release_with_memory(self.memory)?;
        }
        let id = self.identifier(object.get("@id").and_then(Value::as_str))?;
        let detached = matches!(mode, Mode::Detached);
        let node = self.new_node(graph, &id, detached)?;
        self.memory.release_string(id)?;
        if let Some(link) = link {
            // §7.2.2 steps 6.3–6.4 link a node BEFORE processing its description.
            let value = self.reference(node)?;
            self.link_value(graph, link, value)?;
        }
        let mut entries = self.entries(object)?;
        entries.sort_unstable_by(|a, b| rank(a.0).cmp(&rank(b.0)).then(a.0.cmp(b.0)));
        self.tasks
            .try_push_admitted(
                Task::Fields(Fields {
                    entries,
                    at: 0,
                    node,
                    graph,
                    detached,
                    prelinked: link.is_some(),
                    raw: object,
                }),
                self.memory,
            )
            .map_err(RdfDiagnostic::from)
    }
    fn fields(&mut self, mut plan: Fields<'a>) -> Result<(), RdfDiagnostic> {
        let Some(&(key, raw)) = plan.entries.get(plan.at) else {
            self.memory.release_vec(plan.entries)?;
            // Only annotation-free linked nodes set this flag; their complete
            // reference was already stored, so they produce no result value.
            if plan.prelinked {
                return Ok(());
            }
            let value = if plan.detached {
                ordered_object(std::mem::take(&mut self.nodes[plan.node].members))
            } else {
                self.reference(plan.node)?
            };
            if let Some(annotations) = plan.raw.get("@annotation") {
                self.tasks
                    .try_push_admitted(Task::Annotate(value), self.memory)
                    .map_err(RdfDiagnostic::from)?;
                self.tasks
                    .try_push_admitted(
                        Task::Visit(annotations, plan.graph, Mode::Detached),
                        self.memory,
                    )
                    .map_err(RdfDiagnostic::from)?;
            } else {
                self.memory
                    .push(&mut self.results, value)
                    .map_err(RdfDiagnostic::from)?;
            }
            return Ok(());
        };
        plan.at += 1;
        let node = plan.node;
        let graph = plan.graph;
        self.tasks
            .try_push_admitted(Task::Fields(plan), self.memory)
            .map_err(RdfDiagnostic::from)?;
        match key {
            "@id" | "@annotation" => {}
            "@type" => {
                let value = self.identifiers(raw)?;
                self.append(node, key, value)?;
            }
            "@index" => {
                self.semantic_index = true;
                if let Some(member) = self.find_member(node, key) {
                    let old = &self.nodes[node].members[member].1;
                    if old.as_str() != raw.as_str() {
                        return Err(decode("conflicting indexes for one JSON-LD node"));
                    }
                } else {
                    let member = self.member_slot(node, key)?;
                    let value = raw.clone_with_memory(self.memory)?;
                    self.nodes[node].members[member].1 = value;
                }
            }
            "@reverse" => {
                let entries = self.entries(
                    raw.as_object()
                        .ok_or_else(|| decode("expanded reverse map must be an object"))?,
                )?;
                self.tasks
                    .try_push_admitted(
                        Task::Reverse {
                            entries,
                            at: 0,
                            node,
                            graph,
                        },
                        self.memory,
                    )
                    .map_err(RdfDiagnostic::from)?;
            }
            "@graph" => {
                let name = self.memory.string(&self.nodes[node].id)?;
                let named = self.graph(name)?;
                self.tasks
                    .try_push_admitted(Task::Discard, self.memory)
                    .map_err(RdfDiagnostic::from)?;
                self.tasks
                    .try_push_admitted(Task::Visit(raw, named, Mode::Node), self.memory)
                    .map_err(RdfDiagnostic::from)?;
            }
            "@included" => {
                self.tasks
                    .try_push_admitted(Task::Discard, self.memory)
                    .map_err(RdfDiagnostic::from)?;
                self.tasks
                    .try_push_admitted(Task::Visit(raw, graph, Mode::Node), self.memory)
                    .map_err(RdfDiagnostic::from)?;
            }
            control if super::context::is_control(control) => {
                return Err(decode(format!(
                    "unsupported expanded node keyword `{control}`"
                )));
            }
            _ => {
                let key = self.identifier(Some(key))?;
                let member = self.member_slot(node, &key)?;
                self.memory.release_string(key)?;
                self.tasks
                    .try_push_admitted(
                        Task::Linked(raw, graph, Link::Forward { node, member }),
                        self.memory,
                    )
                    .map_err(RdfDiagnostic::from)?;
            }
        }
        Ok(())
    }
    fn generic(&mut self, mut plan: Generic<'a>) -> Result<(), RdfDiagnostic> {
        let Some(&(key, raw)) = plan.entries.get(plan.at) else {
            self.memory.release_vec(plan.entries)?;
            return self
                .memory
                .push(&mut self.results, ordered_object(plan.members))
                .map_err(RdfDiagnostic::from);
        };
        plan.at += 1;
        if key == "@index" {
            self.semantic_index = true;
            // §7.2.2 step 5 reconstructs a list with only its @list member.
            // Record semantic metadata for RDF refusal before omitting it; an
            // opaque @json payload is copied at @value and never enters here.
            if plan.entries.iter().any(|(name, _)| *name == "@list") {
                return self
                    .tasks
                    .try_push_admitted(Task::Generic(plan), self.memory)
                    .map_err(RdfDiagnostic::from);
            }
        }
        if key == "@id"
            || key == "@type"
            || (matches!(plan.mode, Mode::Atomic) && key == "@predicate")
        {
            let value = self.identifiers(raw)?;
            let key = self.memory.string(key)?;
            self.memory.push(&mut plan.members, (key, value))?;
            return self
                .tasks
                .try_push_admitted(Task::Generic(plan), self.memory)
                .map_err(RdfDiagnostic::from);
        }
        if key == "@value"
            || (!matches!(plan.mode, Mode::Atomic)
                && !matches!(key, "@list" | "@triple" | "@annotation"))
        {
            let value = raw.clone_with_memory(self.memory)?;
            let key = self.memory.string(key)?;
            self.memory.push(&mut plan.members, (key, value))?;
            return self
                .tasks
                .try_push_admitted(Task::Generic(plan), self.memory)
                .map_err(RdfDiagnostic::from);
        }
        let mode = if key == "@annotation" {
            Mode::Detached
        } else if matches!(key, "@subject" | "@object") {
            Mode::Node
        } else if key == "@triple" || matches!(plan.mode, Mode::Atomic) {
            Mode::Atomic
        } else {
            Mode::Node
        };
        let graph = plan.graph;
        let key = if matches!(plan.mode, Mode::Atomic) {
            self.identifier(Some(key))?
        } else {
            self.memory.string(key)?
        };
        self.tasks
            .try_push_admitted(Task::GenericAssign(plan, key), self.memory)
            .map_err(RdfDiagnostic::from)?;
        self.tasks
            .try_push_admitted(Task::Visit(raw, graph, mode), self.memory)
            .map_err(RdfDiagnostic::from)
    }
    fn graph_values(&mut self, graph: usize) -> Result<Value, RdfDiagnostic> {
        let nodes = &mut self.graphs[graph].nodes;
        nodes.sort_unstable_by(|&a, &b| self.nodes[a].id.cmp(&self.nodes[b].id));
        let mut values = Vec::new();
        self.memory.reserve(&mut values, nodes.len())?;
        for &node in nodes.iter() {
            if self.nodes[node].members.len() > 1 {
                values.push(ordered_object(std::mem::take(
                    &mut self.nodes[node].members,
                )));
            }
        }
        Ok(Value::Array(values))
    }
    fn finish(mut self, retain_graphs: bool) -> Result<Built, RdfDiagnostic> {
        let mut graphs = Vec::new();
        self.memory
            .reserve(&mut graphs, self.graphs.len().saturating_sub(1))?;
        graphs.extend(1..self.graphs.len());
        graphs.sort_unstable_by(|&a, &b| self.graphs[a].name.cmp(&self.graphs[b].name));
        for &graph in &graphs {
            let name = self
                .memory
                .string(self.graphs[graph].name.as_deref().expect("named graph"))?;
            let node = self.new_node(0, &name, false)?;
            self.memory.release_string(name)?;
            if !retain_graphs {
                let value = self.graph_values(graph)?;
                let member = self.member_slot(node, "@graph")?;
                self.nodes[node].members[member].1 = value;
            }
        }
        self.memory.release_vec(graphs)?;
        let retained = retain_graphs.then(|| NodeMap {
            graphs: std::mem::take(&mut self.graphs),
            nodes: std::mem::take(&mut self.nodes),
        });
        let output = if retain_graphs {
            Value::Null
        } else {
            self.graph_values(0)?
        };
        for node in self.nodes.drain(..) {
            self.memory.release_string(node.id)?;
            let mut unused = ordered_object(node.members);
            unused.release_with_memory(self.memory)?;
        }
        self.memory.release_vec(self.nodes)?;
        for graph in self.graphs.drain(..) {
            if let Some(name) = graph.name {
                self.memory.release_string(name)?;
            }
            self.memory.release_vec(graph.nodes)?;
        }
        self.memory.release_vec(self.graphs)?;
        let bytes = hash_table_retained_bound(&self.index).ok_or(StorageError::SizeOverflow)?;
        for entry in self.index.drain() {
            self.memory.release_string(entry.id)?;
        }
        drop(self.index);
        self.memory.release_bytes(bytes)?;
        let bytes =
            hash_table_retained_bound(&self.graph_index).ok_or(StorageError::SizeOverflow)?;
        for entry in self.graph_index.drain() {
            self.memory.release_string(entry.name)?;
        }
        drop(self.graph_index);
        self.memory.release_bytes(bytes)?;
        let bytes = hash_table_retained_bound(&self.issued).ok_or(StorageError::SizeOverflow)?;
        for entry in self.issued.drain() {
            self.memory.release_string(entry.source)?;
            self.memory.release_string(entry.assigned)?;
        }
        drop(self.issued);
        self.memory.release_bytes(bytes)?;
        let bytes =
            hash_table_retained_bound(&self.candidates).ok_or(StorageError::SizeOverflow)?;
        drop(self.candidates);
        self.memory.release_bytes(bytes)?;
        let bytes = hash_table_retained_bound(&self.members).ok_or(StorageError::SizeOverflow)?;
        drop(self.members);
        self.memory.release_bytes(bytes)?;
        if let Some(map) = retained {
            return Ok(Built::Graphs(map));
        }
        Ok(Built::Flattened(Mapped {
            value: output,
            semantic_index: self.semantic_index,
        }))
    }
}

fn rank(key: &str) -> u8 {
    match key {
        "@type" => 0,
        "@index" => 1,
        "@reverse" => 2,
        "@graph" => 3,
        "@included" => 4,
        _ => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_blank_node_sequence_is_not_pointer_width_limited() {
        let mut ceiling = Ceiling(usize::MAX);
        let mut memory = Memory::new(&mut ceiling);
        let mut walk = Walk {
            memory: &mut memory,
            graphs: Vec::new(),
            nodes: Vec::new(),
            index: HashTable::new(),
            graph_index: HashTable::new(),
            issued: HashTable::new(),
            candidates: HashTable::new(),
            members: HashTable::new(),
            next: u64::from(u32::MAX),
            semantic_index: false,
            issuance: Issuance::Rdf,
            tasks: WorkList::new(),
            results: Vec::new(),
        };
        assert_eq!(walk.identifier(None).unwrap(), "_:jsonld4294967295");
        assert_eq!(walk.next, u64::from(u32::MAX) + 1);
    }
}
