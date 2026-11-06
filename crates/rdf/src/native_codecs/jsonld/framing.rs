// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! JSON-LD 1.1 Framing REC 2020-07-16 §§4–5.3, on the shared native node map.

use super::{
    CompiledJsonLdContext, JsonLdCompactionLimits, JsonLdCompactionPolicy, JsonLdExpansionLimits,
    JsonLdFlattenLimits, RdfDiagnostic, compaction, decode,
    node_map::{self, NodeMap},
    structural,
};
use purrdf_lex::allocation::{Ceiling, Memory, StorageError};
use purrdf_lex::json::{Number, Object, Value};
use purrdf_lex::walk::WorkList;

/// The three embedding policies admitted by the pinned Framing Recommendation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JsonLdEmbed {
    /// Embed a node at most once in a graph, retaining later references.
    #[default]
    Once,
    /// Embed at every occurrence, stopping at ancestor cycles.
    Always,
    /// Keep embedded nodes as identifier references.
    Never,
}

/// Deterministic framing policy. Frame-local controls override these defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "These are the independent boolean options specified by the JSON-LD Framing API."
)]
pub struct JsonLdFramingPolicy {
    /// Default embedding behavior.
    pub embed: JsonLdEmbed,
    /// Include only properties named by the frame.
    pub explicit: bool,
    /// Require every property constraint to match.
    pub require_all: bool,
    /// Omit absent properties instead of supplying defaults.
    pub omit_default: bool,
    /// Do not force a graph envelope beyond ordinary compaction root finishing.
    pub omit_graph: bool,
    /// Select the default graph instead of the merged graph.
    pub frame_default: bool,
    /// Shared compaction controls; both default to true.
    pub compaction: JsonLdCompactionPolicy,
}
impl JsonLdFramingPolicy {
    /// JSON-LD 1.1 framing defaults, with deterministic lexical traversal.
    pub const DEFAULT: Self = Self {
        embed: JsonLdEmbed::Once,
        explicit: false,
        require_all: false,
        omit_default: false,
        omit_graph: true,
        frame_default: false,
        compaction: JsonLdCompactionPolicy::DEFAULT,
    };
}
impl Default for JsonLdFramingPolicy {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Separate phase authorities; none is an aggregate operation-memory bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsonLdFramingLimits {
    /// Independent allowance for each input/frame structural expansion.
    pub expansion: JsonLdExpansionLimits,
    /// Original node-map construction authority.
    pub node_map: JsonLdFlattenLimits,
    /// Live new framing views, output, markers, work frames and scratch.
    pub max_allocation_bytes: u64,
    /// Deterministic subject, property, matching and embedding work units.
    pub max_work: u64,
    /// Maximum emitted subject occurrences, including identifier-only references.
    pub max_emitted_nodes: u64,
    /// Subsequent shared compactor construction authority.
    pub compaction: JsonLdCompactionLimits,
}
impl JsonLdFramingLimits {
    /// 32 GiB framing construction, 2^30 work units and 2^24 emitted nodes.
    pub const DEFAULT: Self = Self {
        expansion: JsonLdExpansionLimits::DEFAULT,
        node_map: JsonLdFlattenLimits::DEFAULT,
        max_allocation_bytes: 32 * 1024 * 1024 * 1024,
        max_work: 1 << 30,
        max_emitted_nodes: 1 << 24,
        compaction: JsonLdCompactionLimits::DEFAULT,
    };
}
impl Default for JsonLdFramingLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Output compaction base, independent of the frame retrieval URL.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JsonLdFramingBase<'a> {
    /// Use the expanded input document base, when relative compaction is enabled.
    #[default]
    Input,
    /// Do not compact document-position IRIs relative to a base.
    None,
    /// Use this explicit absolute output base.
    Iri(&'a str),
}

/// Frame a native document value without lowering through RDF.
///
/// `input_context` interprets the input; `frame_context` is the initial pattern
/// context. `frame_retrieval_base` resolves the frame and its offline contexts.
/// Only the frame's own `@context` is compiled for output: missing means an
/// empty context and explicit null remains null. Context/compiler/scope work has
/// its existing separate semantic authority, outside `max_work`.
/// # Errors
/// Returns invalid frames, unsupported native-term patterns, deterministic work
/// or emission refusals, and original native storage/context diagnostics.
#[expect(
    clippy::too_many_arguments,
    reason = "The input, pattern, retrieval and output context roles are independent API authorities."
)]
pub fn frame_value_with_context(
    input: &Value,
    frame: &Value,
    input_context: &CompiledJsonLdContext,
    frame_context: &CompiledJsonLdContext,
    frame_retrieval_base: Option<&str>,
    output_base: JsonLdFramingBase<'_>,
    policy: &JsonLdFramingPolicy,
    limits: &JsonLdFramingLimits,
) -> Result<Value, RdfDiagnostic> {
    let expanded = structural::expand(input, input_context, limits.expansion)?;
    let interpretation = frame_context.with_frame_base_fallback(frame_retrieval_base)?;
    let pattern = structural::expand_frame(
        frame,
        &interpretation,
        limits.expansion,
        frame_retrieval_base,
    )?
    .value;
    let empty = Value::Object(Object::new());
    let own_context = singleton(frame).get("@context").unwrap_or(&empty);
    let base = match output_base {
        JsonLdFramingBase::Input if policy.compaction.compact_to_relative => {
            expanded.document_base.as_deref()
        }
        JsonLdFramingBase::Input | JsonLdFramingBase::None => None,
        JsonLdFramingBase::Iri(base) => Some(base),
    };
    let output = CompiledJsonLdContext::compile_with_bases(
        own_context,
        frame_retrieval_base,
        base,
        frame_context.registry(),
    )?;
    let map = node_map::build_graphs(&expanded.value, limits.node_map)?;
    let mut ceiling = Ceiling(usize::try_from(limits.max_allocation_bytes).unwrap_or(usize::MAX));
    Memory::new(&mut ceiling).try_scope(|memory| {
        let mut walk = Walk {
            memory,
            map: &map,
            views: Vec::new(),
            defaults: Vec::new(),
            tasks: WorkList::new(),
            results: Vec::new(),
            ancestors: Vec::new(),
            embedded: Vec::new(),
            work: limits.max_work,
            emissions: limits.max_emitted_nodes,
            base_flags: Flags::from(*policy),
        };
        let frame = frame_object(&pattern)?;
        walk.validate(frame)?;
        walk.views()?;
        let graph = if policy.frame_default || frame.contains_key("@graph") {
            0
        } else {
            walk.views.len() - 1
        };
        walk.tasks
            .try_push_admitted(
                Task::Frame(Traversal {
                    graph,
                    selection: Selection::All,
                    at: 0,
                    frame: Some(frame),
                    flags: Flags::from(*policy),
                    embedded: false,
                    values: Vec::new(),
                    waiting: false,
                }),
                walk.memory,
            )
            .map_err(RdfDiagnostic::from)?;
        while let Some(task) = walk.tasks.pop() {
            walk.step(task)?;
        }
        // Every scheduled frame/continuation leaves exactly one value.
        let mut framed = walk.results.pop().expect("one framed document");
        walk.prune(&mut framed)?;
        let mut compacted = compaction::compact_framed(
            &framed,
            &output,
            policy.compaction,
            limits.compaction,
            &walk.defaults,
        )?;
        // The framing operation always emits its own context, including null.
        // This finishing storage belongs to framing, not the completed compactor.
        if compacted.get("@context").is_none() {
            let context = output.canonical_context().clone_with_memory(walk.memory)?;
            let object = compacted
                .as_object_mut()
                .expect("compactor document finishing");
            let mut foreign = std::mem::take(object).into_members();
            let mut members = Vec::new();
            walk.memory
                .reserve(&mut members, foreign.len().saturating_add(1))?;
            members.append(&mut foreign);
            // This capacity was admitted by compaction, not framing.
            drop(foreign);
            walk.put(&mut members, "@context", context)?;
            *object = Object::from(members);
        }
        if !policy.omit_graph {
            let graph_key = output.compact_iri("@graph", true)?;
            let already = compacted
                .get(&graph_key)
                .is_some_and(|value| value.as_array().is_some());
            if !already {
                let object = compacted.as_object_mut().expect("compactor root object");
                let old = std::mem::take(object).into_members();
                let mut context = Value::Null;
                let mut body = Vec::new();
                for (key, value) in old {
                    if key == "@context" {
                        context = value;
                    } else {
                        walk.memory.push(&mut body, (key, value))?;
                    }
                }
                // Compactor-owned capacity is not refunded to framing.
                let mut values = Vec::new();
                walk.memory
                    .push(&mut values, Value::Object(Object::from(body)))?;
                let mut members = Vec::new();
                walk.put(&mut members, "@context", context)?;
                walk.put(&mut members, &graph_key, Value::Array(values))?;
                compacted = Value::Object(Object::from(members));
            }
        }
        Ok(compacted)
    })
}

/// Typed defaults are outside the JSON value algebra. The private numeric
/// preservation grammar is only produced here and only consumed by the private
/// compactor entry; a user string or JSON null can never identify a marker.
pub(super) enum DefaultValue<'a> {
    Null,
    Value(&'a Value),
}

#[derive(Clone, Copy)]
struct Flags {
    embed: JsonLdEmbed,
    explicit: bool,
    require_all: bool,
    omit_default: bool,
}
impl From<JsonLdFramingPolicy> for Flags {
    fn from(policy: JsonLdFramingPolicy) -> Self {
        Self {
            embed: policy.embed,
            explicit: policy.explicit,
            require_all: policy.require_all,
            omit_default: policy.omit_default,
        }
    }
}
impl Flags {
    fn apply(mut self, frame: Option<&Object>) -> Result<Self, RdfDiagnostic> {
        if let Some(frame) = frame {
            if let Some(value) = frame.get("@embed") {
                self.embed = embed(value)?;
            }
            for (key, target) in [
                ("@explicit", &mut self.explicit),
                ("@requireAll", &mut self.require_all),
                ("@omitDefault", &mut self.omit_default),
            ] {
                if let Some(value) = frame.get(key) {
                    *target = singleton(value)
                        .as_bool()
                        .ok_or_else(|| decode(format!("framing {key} must be boolean")))?;
                }
            }
        }
        Ok(self)
    }
}
fn embed(value: &Value) -> Result<JsonLdEmbed, RdfDiagnostic> {
    match singleton(value) {
        Value::Bool(true) => Ok(JsonLdEmbed::Once),
        Value::Bool(false) => Ok(JsonLdEmbed::Never),
        Value::String(value) => match value.as_str() {
            "@once" => Ok(JsonLdEmbed::Once),
            "@always" => Ok(JsonLdEmbed::Always),
            "@never" => Ok(JsonLdEmbed::Never),
            _ => Err(decode(
                "framing @embed must be @once, @always, @never or a boolean",
            )),
        },
        _ => Err(decode(
            "framing @embed must be @once, @always, @never or a boolean",
        )),
    }
}
fn singleton(value: &Value) -> &Value {
    value
        .as_array()
        .filter(|values| values.len() == 1)
        .map_or(value, |values| &values[0])
}
fn frame_object(value: &Value) -> Result<&Object, RdfDiagnostic> {
    singleton(value)
        .as_object()
        .ok_or_else(|| decode("a frame must be one object or an array containing one object"))
}
fn values(value: &Value) -> &[Value] {
    value
        .as_array()
        .map_or_else(|| std::slice::from_ref(value), Vec::as_slice)
}
pub(super) fn control(key: &str) -> bool {
    matches!(
        key,
        "@embed" | "@explicit" | "@requireAll" | "@omitDefault" | "@default"
    )
}
fn wildcard(frame: &Object) -> bool {
    frame.keys().all(|key| control(key))
}
fn atomic(value: &Value) -> bool {
    value.get("@triple").is_some() || value.get("@annotation").is_some()
}

struct Property<'a> {
    key: &'a str,
    values: Vec<&'a Value>,
}
struct Subject<'a> {
    id: &'a str,
    properties: Vec<Property<'a>>,
}
struct View<'a> {
    name: Option<&'a str>,
    nodes: Vec<Subject<'a>>,
}
#[derive(Clone, Copy)]
enum Selection {
    All,
    One(usize),
}
struct Traversal<'a> {
    graph: usize,
    selection: Selection,
    at: usize,
    frame: Option<&'a Object>,
    flags: Flags,
    embedded: bool,
    values: Vec<Value>,
    waiting: bool,
}
struct Node<'a> {
    graph: usize,
    subject: usize,
    frame: Option<&'a Object>,
    flags: Flags,
    members: Vec<(String, Value)>,
    edges: Vec<Edge<'a>>,
    at: usize,
    waiting: bool,
}
struct Edge<'a> {
    key: &'a str,
    operation: Operation<'a>,
}
#[derive(Clone, Copy)]
enum Operation<'a> {
    Property(usize, Option<&'a Object>),
    Graph(usize, Option<&'a Object>),
    Included(Option<&'a Object>),
    Reverse(&'a Object),
}
#[derive(Clone, Copy)]
enum Items<'a> {
    Property { subject: usize, property: usize },
    List(&'a [Value]),
}
struct Collection<'a> {
    graph: usize,
    items: Items<'a>,
    at: usize,
    frame: Option<&'a Object>,
    flags: Flags,
    output: Vec<Value>,
    waiting: bool,
}
enum Task<'a> {
    Frame(Traversal<'a>),
    Node(Node<'a>),
    Collection(Collection<'a>),
    List {
        original: &'a Object,
    },
    Reverse {
        graph: usize,
        subject: usize,
        frame: &'a Object,
        at: usize,
        flags: Flags,
        members: Vec<(String, Value)>,
        waiting: Option<&'a str>,
    },
}
struct Walk<'a, 'm, 's> {
    memory: &'m mut Memory<'s, Ceiling>,
    map: &'a NodeMap,
    views: Vec<View<'a>>,
    defaults: Vec<DefaultValue<'a>>,
    tasks: WorkList<Task<'a>, 8>,
    results: Vec<Value>,
    ancestors: Vec<(usize, usize)>,
    embedded: Vec<(usize, usize)>,
    work: u64,
    emissions: u64,
    base_flags: Flags,
}

enum Match<'a> {
    Node {
        graph: usize,
        subject: usize,
        frame: Option<&'a Object>,
        flags: Flags,
    },
    Item {
        graph: usize,
        value: &'a Value,
        frame: &'a Object,
    },
    Combine {
        count: usize,
        all: bool,
    },
}

impl<'a> Walk<'a, '_, '_> {
    fn emit(&mut self) -> Result<(), RdfDiagnostic> {
        charge(&mut self.work, 1)?;
        self.emissions = self.emissions.checked_sub(1).ok_or_else(|| {
            RdfDiagnostic::error(
                "jsonld-framing-emission-limit",
                "framing exhausted its emitted-node allowance",
            )
        })?;
        Ok(())
    }
    fn admit_copy(&mut self, value: &Value) -> Result<(), RdfDiagnostic> {
        let mut scan: WorkList<_, 8> = WorkList::new();
        scan.try_push_admitted(value, self.memory)?;
        while let Some(value) = scan.pop() {
            charge(&mut self.work, 1)?;
            match value {
                Value::Array(values) => {
                    for value in values {
                        scan.try_push_admitted(value, self.memory)?;
                    }
                }
                Value::Object(object) => {
                    let node = object.contains_key("@id")
                        || ![
                            "@value",
                            "@list",
                            "@triple",
                            "@subject",
                            "@predicate",
                            "@object",
                        ]
                        .iter()
                        .any(|key| object.contains_key(key));
                    if node {
                        self.emit()?;
                    }
                    let json = object.get("@type").and_then(Value::as_str) == Some("@json");
                    for (key, value) in object.iter() {
                        if json && key == "@value" {
                            continue;
                        }
                        scan.try_push_admitted(value, self.memory)?;
                    }
                }
                _ => {}
            }
        }
        scan.release_admitted(self.memory)?;
        Ok(())
    }
    fn put(
        &mut self,
        members: &mut Vec<(String, Value)>,
        key: &str,
        value: Value,
    ) -> Result<(), RdfDiagnostic> {
        let key = self.memory.string(key)?;
        self.memory.push(members, (key, value))?;
        members.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        Ok(())
    }
    fn append(&mut self, target: &mut Vec<Value>, mut value: Value) -> Result<(), RdfDiagnostic> {
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
    fn validate(&mut self, frame: &'a Object) -> Result<(), RdfDiagnostic> {
        let mut tasks: WorkList<_, 8> = WorkList::new();
        tasks.try_push_admitted(frame, self.memory)?;
        while let Some(frame) = tasks.pop() {
            charge(&mut self.work, 1)?;
            Flags::from(JsonLdFramingPolicy::DEFAULT).apply(Some(frame))?;
            for (key, value) in frame.iter() {
                charge(&mut self.work, 1)?;
                if matches!(
                    key.as_str(),
                    "@triple" | "@subject" | "@predicate" | "@object" | "@annotation"
                ) {
                    return Err(decode(
                        "framing patterns cannot inspect native @triple components or @annotation internals; use an ordinary property wildcard or exclusion",
                    ));
                }
                if control(key) {
                    continue;
                }
                if matches!(key.as_str(), "@id" | "@type") {
                    let items = values(value);
                    let wildcard =
                        items.len() == 1 && items[0].as_object().is_some_and(Object::is_empty);
                    let default_type =
                        key == "@type" && items.len() == 1 && items[0].get("@default").is_some();
                    let json_value = key == "@type"
                        && frame.contains_key("@value")
                        && items.len() == 1
                        && items[0].as_str() == Some("@json");
                    if !wildcard
                        && !default_type
                        && !json_value
                        && items.iter().any(|value| {
                            value
                                .as_str()
                                .is_none_or(|iri| purrdf_iri::BaseIri::parse(iri).is_err())
                        })
                    {
                        return Err(decode(format!(
                            "framing {key} must contain absolute IRIs, an empty wildcard, or a type default"
                        )));
                    }
                    continue;
                }
                if matches!(
                    key.as_str(),
                    "@value" | "@language" | "@direction" | "@index"
                ) {
                    continue;
                }
                if key == "@reverse" {
                    let reverse = value
                        .as_object()
                        .ok_or_else(|| decode("frame @reverse must be an object"))?;
                    for (_, value) in reverse.iter() {
                        tasks.try_push_admitted(frame_object(value)?, self.memory)?;
                    }
                    continue;
                }
                if key.starts_with('@') && !matches!(key.as_str(), "@graph" | "@included" | "@list")
                {
                    return Err(decode(format!("unsupported framing keyword {key}")));
                }
                let items = values(value);
                if items.len() > 1 {
                    return Err(decode(
                        "a property frame must be empty or contain exactly one frame object",
                    ));
                }
                if let Some(item) = items.first() {
                    tasks.try_push_admitted(
                        item.as_object()
                            .ok_or_else(|| decode("a property frame must be an object"))?,
                        self.memory,
                    )?;
                }
            }
        }
        tasks.release_admitted(self.memory)?;
        Ok(())
    }
    fn add_to_view(&mut self, view: &mut View<'a>, slot: usize) -> Result<(), RdfDiagnostic> {
        charge(&mut self.work, 1)?;
        let node = &self.map.nodes[slot];
        let at = match view
            .nodes
            .binary_search_by(|subject| subject.id.cmp(&node.id))
        {
            Ok(at) => at,
            Err(at) => {
                self.memory.reserve_for_push(&mut view.nodes)?;
                view.nodes.insert(
                    at,
                    Subject {
                        id: &node.id,
                        properties: Vec::new(),
                    },
                );
                at
            }
        };
        for (key, raw) in &node.members {
            charge(&mut self.work, 1)?;
            if key == "@id" {
                continue;
            }
            let properties = &mut view.nodes[at].properties;
            let property = match properties.binary_search_by(|property| property.key.cmp(key)) {
                Ok(at) => at,
                Err(at) => {
                    self.memory.reserve_for_push(properties)?;
                    properties.insert(
                        at,
                        Property {
                            key,
                            values: Vec::new(),
                        },
                    );
                    at
                }
            };
            for value in values(raw) {
                charge(
                    &mut self.work,
                    properties[property].values.len().saturating_add(1),
                )?;
                let target = &mut properties[property].values;
                let mut duplicate = false;
                for old in target.iter() {
                    if old.eq_with_memory(value, self.memory)? {
                        duplicate = true;
                        break;
                    }
                }
                if !duplicate {
                    self.memory.push(target, value)?;
                }
            }
        }
        Ok(())
    }
    fn views(&mut self) -> Result<(), RdfDiagnostic> {
        let mut merged = View {
            name: Some("@merged"),
            nodes: Vec::new(),
        };
        for graph in &self.map.graphs {
            let mut view = View {
                name: graph.name.as_deref(),
                nodes: Vec::new(),
            };
            for &node in &graph.nodes {
                self.add_to_view(&mut view, node)?;
                self.add_to_view(&mut merged, node)?;
            }
            self.memory.push(&mut self.views, view)?;
        }
        self.memory.push(&mut self.views, merged)?;
        Ok(())
    }
    fn find(&mut self, graph: usize, id: &str) -> Result<Option<usize>, RdfDiagnostic> {
        // Charge the search before its comparisons, including unsuccessful lookup.
        charge(
            &mut self.work,
            self.views[graph]
                .nodes
                .len()
                .checked_ilog2()
                .unwrap_or(0)
                .saturating_add(1) as usize,
        )?;
        Ok(self.views[graph]
            .nodes
            .binary_search_by(|subject| subject.id.cmp(id))
            .ok())
    }
    fn reference(&mut self, graph: usize, subject: usize) -> Result<Value, RdfDiagnostic> {
        let id = self.memory.string(self.views[graph].nodes[subject].id)?;
        let mut members = Vec::new();
        self.put(&mut members, "@id", Value::String(id))?;
        Ok(Value::Object(Object::from(members)))
    }
    fn step(&mut self, task: Task<'a>) -> Result<(), RdfDiagnostic> {
        match task {
            Task::Frame(frame) => self.frame(frame),
            Task::Node(node) => self.node(node),
            Task::Collection(collection) => self.collection(collection),
            Task::List { original } => {
                // A list continuation follows exactly one collection result.
                let list = self.results.pop().expect("list collection result");
                let mut members = Vec::new();
                self.put(&mut members, "@list", list)?;
                for (key, value) in original.iter().filter(|(key, _)| key.as_str() != "@list") {
                    self.admit_copy(value)?;
                    let value = value.clone_with_memory(self.memory)?;
                    self.put(&mut members, key, value)?;
                }
                self.memory
                    .push(&mut self.results, Value::Object(Object::from(members)))
                    .map_err(RdfDiagnostic::from)
            }
            Task::Reverse {
                graph,
                subject,
                frame,
                at,
                flags,
                members,
                waiting,
            } => self.reverse(graph, subject, frame, at, flags, members, waiting),
        }
    }
    fn frame(&mut self, mut frame: Traversal<'a>) -> Result<(), RdfDiagnostic> {
        if frame.waiting {
            // A suspended frame has exactly one completed subject result.
            let value = self.results.pop().expect("framed subject result");
            self.append(&mut frame.values, value)?;
            frame.waiting = false;
        }
        let count = match frame.selection {
            Selection::All => self.views[frame.graph].nodes.len(),
            Selection::One(_) => 1,
        };
        while frame.at < count {
            charge(&mut self.work, 1)?;
            let subject = match frame.selection {
                Selection::All => frame.at,
                Selection::One(subject) => subject,
            };
            frame.at += 1;
            let flags = frame.flags.apply(frame.frame)?;
            if !self.matches(frame.graph, subject, frame.frame, flags)? {
                continue;
            }
            charge(&mut self.work, self.embedded.len())?;
            let already = self.embedded.contains(&(frame.graph, subject));
            if !frame.embedded && already {
                continue;
            }
            self.emit()?;
            charge(&mut self.work, self.ancestors.len())?;
            if frame.embedded
                && (flags.embed == JsonLdEmbed::Never
                    || self.ancestors.contains(&(frame.graph, subject))
                    || flags.embed == JsonLdEmbed::Once && already)
            {
                let reference = self.reference(frame.graph, subject)?;
                self.memory.push(&mut frame.values, reference)?;
                continue;
            }
            if !already {
                self.memory
                    .push(&mut self.embedded, (frame.graph, subject))?;
            }
            self.memory
                .push(&mut self.ancestors, (frame.graph, subject))?;
            let node = self.plan(frame.graph, subject, frame.frame, flags)?;
            frame.waiting = true;
            self.tasks
                .try_push_admitted(Task::Frame(frame), self.memory)
                .map_err(RdfDiagnostic::from)?;
            return self
                .tasks
                .try_push_admitted(Task::Node(node), self.memory)
                .map_err(RdfDiagnostic::from);
        }
        self.memory
            .push(&mut self.results, Value::Array(frame.values))
            .map_err(RdfDiagnostic::from)
    }
    fn plan(
        &mut self,
        graph: usize,
        subject: usize,
        frame: Option<&'a Object>,
        flags: Flags,
    ) -> Result<Node<'a>, RdfDiagnostic> {
        let mut reference = self.reference(graph, subject)?;
        // reference always constructs one admitted object.
        let members =
            std::mem::take(reference.as_object_mut().expect("reference object")).into_members();
        let mut node = Node {
            graph,
            subject,
            frame,
            flags,
            members,
            edges: Vec::new(),
            at: 0,
            waiting: false,
        };
        let id = self.views[graph].nodes[subject].id;
        charge(&mut self.work, self.views.len())?;
        let named = self.views.iter().position(|view| view.name == Some(id));
        if let Some(named) = named {
            let graph_pattern = frame.and_then(|frame| frame.get("@graph"));
            let subframe = graph_pattern
                .and_then(|pattern| values(pattern).first())
                .map(frame_object)
                .transpose()?;
            if graph_pattern.is_some() || self.views[graph].name != Some("@merged") {
                self.memory.push(
                    &mut node.edges,
                    Edge {
                        key: "@graph",
                        operation: Operation::Graph(named, subframe),
                    },
                )?;
            }
        }
        if let Some(included) = frame.and_then(|frame| frame.get("@included")) {
            let included = frame_object(included)?;
            self.memory.push(
                &mut node.edges,
                Edge {
                    key: "@included",
                    operation: Operation::Included(Some(included)),
                },
            )?;
        }
        let count = self.views[graph].nodes[subject].properties.len();
        for property in 0..count {
            charge(&mut self.work, 1)?;
            let key = self.views[graph].nodes[subject].properties[property].key;
            if key.starts_with('@') {
                let raw = &self.views[graph].nodes[subject].properties[property].values;
                let value = if key == "@index" {
                    raw[0].clone_with_memory(self.memory)?
                } else {
                    let mut values = Vec::new();
                    for raw in raw {
                        let value = raw.clone_with_memory(self.memory)?;
                        self.memory.push(&mut values, value)?;
                    }
                    Value::Array(values)
                };
                self.put(&mut node.members, key, value)?;
            } else {
                let raw = frame.and_then(|frame| frame.get(key));
                if flags.explicit && raw.is_none() {
                    continue;
                }
                let subframe = raw
                    .filter(|value| !values(value).is_empty())
                    .map(frame_object)
                    .transpose()?;
                if subframe.is_none() {
                    charge(&mut self.work, 1)?;
                } // synthesized inherited frame
                self.memory.push(
                    &mut node.edges,
                    Edge {
                        key,
                        operation: Operation::Property(property, subframe),
                    },
                )?;
            }
        }
        if let Some(reverse) = frame
            .and_then(|frame| frame.get("@reverse"))
            .and_then(Value::as_object)
        {
            self.memory.push(
                &mut node.edges,
                Edge {
                    key: "@reverse",
                    operation: Operation::Reverse(reverse),
                },
            )?;
        }
        Ok(node)
    }
    fn node(&mut self, mut node: Node<'a>) -> Result<(), RdfDiagnostic> {
        if node.waiting {
            // Each edge schedules one child operation.
            let value = self.results.pop().expect("framing edge result");
            let key = node.edges[node.at - 1].key;
            if key.starts_with('@') || !value.as_array().is_some_and(Vec::is_empty) {
                self.put(&mut node.members, key, value)?;
            } else {
                let mut unused = value;
                unused.release_with_memory(self.memory)?;
            }
            node.waiting = false;
        }
        if let Some(edge) = node.edges.get(node.at) {
            let operation = edge.operation;
            node.at += 1;
            node.waiting = true;
            let graph = node.graph;
            let subject = node.subject;
            let flags = if node.frame.is_some_and(|frame| frame.contains_key(edge.key)) {
                self.base_flags
            } else {
                node.flags
            };
            self.tasks
                .try_push_admitted(Task::Node(node), self.memory)
                .map_err(RdfDiagnostic::from)?;
            return match operation {
                Operation::Property(property, frame) => self
                    .tasks
                    .try_push_admitted(
                        Task::Collection(Collection {
                            graph,
                            items: Items::Property { subject, property },
                            at: 0,
                            frame,
                            flags,
                            output: Vec::new(),
                            waiting: false,
                        }),
                        self.memory,
                    )
                    .map_err(RdfDiagnostic::from),
                Operation::Graph(graph, frame) => self
                    .tasks
                    .try_push_admitted(
                        Task::Frame(Traversal {
                            graph,
                            selection: Selection::All,
                            at: 0,
                            frame,
                            flags,
                            embedded: false,
                            values: Vec::new(),
                            waiting: false,
                        }),
                        self.memory,
                    )
                    .map_err(RdfDiagnostic::from),
                Operation::Included(frame) => self
                    .tasks
                    .try_push_admitted(
                        Task::Frame(Traversal {
                            graph,
                            selection: Selection::All,
                            at: 0,
                            frame,
                            flags,
                            embedded: false,
                            values: Vec::new(),
                            waiting: false,
                        }),
                        self.memory,
                    )
                    .map_err(RdfDiagnostic::from),
                Operation::Reverse(frame) => self
                    .tasks
                    .try_push_admitted(
                        Task::Reverse {
                            graph,
                            subject,
                            frame,
                            at: 0,
                            flags,
                            members: Vec::new(),
                            waiting: None,
                        },
                        self.memory,
                    )
                    .map_err(RdfDiagnostic::from),
            };
        }
        if let Some(frame) = node.frame {
            for (key, value) in frame.iter() {
                charge(&mut self.work, 1)?;
                if (key.starts_with('@') && key != "@type")
                    || node.members.iter().any(|(name, _)| name == key)
                {
                    continue;
                }
                let subframe = values(value).first().and_then(Value::as_object);
                let flags = if subframe.is_some() {
                    self.base_flags
                } else {
                    node.flags
                }
                .apply(subframe)?;
                if flags.omit_default {
                    continue;
                }
                charge(&mut self.work, 1)?;
                let default = subframe.and_then(|frame| frame.get("@default"));
                if key == "@type" {
                    // A type default is already an admitted array of expanded
                    // IRIs, not a nullable property value. Keep the compactor's
                    // existing typed-position validation and IRI selection body.
                    if let Some(default) = default {
                        let value = default.clone_with_memory(self.memory)?;
                        self.put(&mut node.members, key, value)?;
                    }
                    continue;
                }
                let default = match default {
                    None => DefaultValue::Null,
                    Some(value) if singleton(value).as_str() == Some("@null") => DefaultValue::Null,
                    Some(value) => {
                        self.admit_copy(value)?;
                        DefaultValue::Value(value)
                    }
                };
                let index = self.defaults.len();
                self.memory.push(&mut self.defaults, default)?;
                let token = self.memory.format(&index)?;
                // format of usize consists solely of decimal digits.
                let token =
                    Number::from_lexeme(token).expect("formatted marker index is a JSON integer");
                let mut marker = Vec::new();
                self.put(&mut marker, "@preserve", Value::Number(token))?;
                let mut marker_values = Vec::new();
                self.memory
                    .push(&mut marker_values, Value::Object(Object::from(marker)))?;
                self.put(&mut node.members, key, Value::Array(marker_values))?;
            }
        }
        self.memory.release_vec(node.edges)?;
        self.ancestors.pop();
        self.memory
            .push(&mut self.results, Value::Object(Object::from(node.members)))
            .map_err(RdfDiagnostic::from)
    }
    fn item(&self, graph: usize, items: Items<'a>, at: usize) -> Option<&'a Value> {
        match items {
            Items::Property { subject, property } => self.views[graph].nodes[subject].properties
                [property]
                .values
                .get(at)
                .copied(),
            Items::List(items) => items.get(at),
        }
    }
    fn collection(&mut self, mut collection: Collection<'a>) -> Result<(), RdfDiagnostic> {
        if collection.waiting {
            // Each suspended collection has one completed item result.
            let value = self.results.pop().expect("framing item result");
            self.append(&mut collection.output, value)?;
            collection.waiting = false;
        }
        while let Some(item) = self.item(collection.graph, collection.items, collection.at) {
            charge(&mut self.work, 1)?;
            collection.at += 1;
            if let Some(list) = item.get("@list").and_then(Value::as_array)
                && !atomic(item)
            {
                if let Some(frame) = collection.frame
                    && !self.matches_item(collection.graph, item, frame)?
                {
                    continue;
                }
                let frame = collection
                    .frame
                    .and_then(|frame| frame.get("@list"))
                    .filter(|value| !values(value).is_empty())
                    .map(frame_object)
                    .transpose()?;
                let graph = collection.graph;
                let flags = if frame.is_some() {
                    self.base_flags
                } else {
                    collection.flags.apply(collection.frame)?
                };
                collection.waiting = true;
                self.tasks
                    .try_push_admitted(Task::Collection(collection), self.memory)
                    .map_err(RdfDiagnostic::from)?;
                // The presence of @list above establishes the object view.
                self.tasks
                    .try_push_admitted(
                        Task::List {
                            original: item.as_object().expect("list object"),
                        },
                        self.memory,
                    )
                    .map_err(RdfDiagnostic::from)?;
                return self
                    .tasks
                    .try_push_admitted(
                        Task::Collection(Collection {
                            graph,
                            items: Items::List(list),
                            at: 0,
                            frame,
                            flags,
                            output: Vec::new(),
                            waiting: false,
                        }),
                        self.memory,
                    )
                    .map_err(RdfDiagnostic::from);
            }
            if let Some(id) = item.get("@id").and_then(Value::as_str)
                && !atomic(item)
                && let Some(subject) = self.find(collection.graph, id)?
            {
                let graph = collection.graph;
                let frame = collection.frame;
                let flags = collection.flags;
                collection.waiting = true;
                self.tasks
                    .try_push_admitted(Task::Collection(collection), self.memory)
                    .map_err(RdfDiagnostic::from)?;
                return self
                    .tasks
                    .try_push_admitted(
                        Task::Frame(Traversal {
                            graph,
                            selection: Selection::One(subject),
                            at: 0,
                            frame,
                            flags,
                            embedded: true,
                            values: Vec::new(),
                            waiting: false,
                        }),
                        self.memory,
                    )
                    .map_err(RdfDiagnostic::from);
            }
            if let Some(frame) = collection.frame
                && !self.matches_item(collection.graph, item, frame)?
            {
                continue;
            }
            self.admit_copy(item)?;
            let copy = item.clone_with_memory(self.memory)?;
            self.memory.push(&mut collection.output, copy)?;
        }
        self.memory
            .push(&mut self.results, Value::Array(collection.output))
            .map_err(RdfDiagnostic::from)
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "The reverse continuation stores one bounded iterative state, without recursion."
    )]
    fn reverse(
        &mut self,
        graph: usize,
        subject: usize,
        frame: &'a Object,
        mut at: usize,
        flags: Flags,
        mut members: Vec<(String, Value)>,
        waiting: Option<&'a str>,
    ) -> Result<(), RdfDiagnostic> {
        if let Some(key) = waiting {
            // One reverse candidate scheduled one frame result.
            let value = self.results.pop().expect("reverse candidate result");
            if let Some(old) = purrdf_lex::assoc::get_mut(&mut members, key) {
                let mut target = old.take();
                self.append(
                    target.as_array_mut().expect("reverse property array"),
                    value,
                )?;
                *old = target;
            } else {
                self.put(&mut members, key, value)?;
            }
        }
        let count = self.views[graph].nodes.len();
        let total = count
            .checked_mul(frame.len())
            .ok_or(StorageError::SizeOverflow)?;
        while at < total {
            charge(&mut self.work, frame.len().saturating_add(1))?;
            let property = at / count;
            let candidate = at % count;
            at += 1;
            // property is bounded by frame.len() through total above.
            let (key, raw) = frame.iter().nth(property).expect("reverse property cursor");
            let id = self.views[graph].nodes[subject].id;
            charge(
                &mut self.work,
                self.views[graph].nodes[candidate].properties.len(),
            )?;
            let values = self.views[graph].nodes[candidate]
                .properties
                .iter()
                .find(|property| property.key == key)
                .map(|property| &property.values);
            let linked = if let Some(values) = values {
                charge(&mut self.work, values.len())?;
                values
                    .iter()
                    .any(|value| value.get("@id").and_then(Value::as_str) == Some(id))
            } else {
                false
            };
            if !linked {
                continue;
            }
            let subframe = frame_object(raw)?;
            self.tasks
                .try_push_admitted(
                    Task::Reverse {
                        graph,
                        subject,
                        frame,
                        at,
                        flags,
                        members,
                        waiting: Some(key),
                    },
                    self.memory,
                )
                .map_err(RdfDiagnostic::from)?;
            return self
                .tasks
                .try_push_admitted(
                    Task::Frame(Traversal {
                        graph,
                        selection: Selection::One(candidate),
                        at: 0,
                        frame: Some(subframe),
                        flags,
                        embedded: true,
                        values: Vec::new(),
                        waiting: false,
                    }),
                    self.memory,
                )
                .map_err(RdfDiagnostic::from);
        }
        self.memory
            .push(&mut self.results, Value::Object(Object::from(members)))
            .map_err(RdfDiagnostic::from)
    }
    fn matches(
        &mut self,
        graph: usize,
        subject: usize,
        frame: Option<&'a Object>,
        flags: Flags,
    ) -> Result<bool, RdfDiagnostic> {
        self.matching(Match::Node {
            graph,
            subject,
            frame,
            flags,
        })
    }
    fn matches_item(
        &mut self,
        graph: usize,
        value: &'a Value,
        frame: &'a Object,
    ) -> Result<bool, RdfDiagnostic> {
        self.matching(Match::Item {
            graph,
            value,
            frame,
        })
    }
    fn matching(&mut self, first: Match<'a>) -> Result<bool, RdfDiagnostic> {
        let mut tasks: WorkList<_, 8> = WorkList::new();
        let mut results = Vec::new();
        tasks.try_push_admitted(first, self.memory)?;
        while let Some(task) = tasks.pop() {
            charge(&mut self.work, 1)?;
            match task {
                Match::Combine { count, all } => {
                    let start = results.len() - count;
                    let matched = if all {
                        results[start..].iter().all(|value| *value)
                    } else {
                        results[start..].iter().any(|value| *value)
                    };
                    charge(&mut self.work, count)?;
                    results.truncate(start);
                    self.memory.push(&mut results, matched)?;
                }
                Match::Item {
                    graph,
                    value,
                    frame,
                } => {
                    if wildcard(frame) {
                        self.memory.push(&mut results, true)?;
                    } else if frame.contains_key("@value") {
                        let matched = self.value_matches(value, frame)?;
                        self.memory.push(&mut results, matched)?;
                    } else if let Some(pattern) = frame.get("@list") {
                        let Some(items) = value.get("@list").and_then(Value::as_array) else {
                            self.memory.push(&mut results, false)?;
                            continue;
                        };
                        let pattern = values(pattern);
                        if pattern.is_empty() {
                            self.memory.push(&mut results, items.is_empty())?;
                        } else {
                            let pattern = frame_object(&pattern[0])?;
                            charge(&mut self.work, items.len())?;
                            tasks.try_push_admitted(
                                Match::Combine {
                                    count: items.len(),
                                    all: false,
                                },
                                self.memory,
                            )?;
                            for item in items {
                                tasks.try_push_admitted(
                                    Match::Item {
                                        graph,
                                        value: item,
                                        frame: pattern,
                                    },
                                    self.memory,
                                )?;
                            }
                        }
                    } else if let Some(id) = value.get("@id").and_then(Value::as_str) {
                        if let Some(subject) = self.find(graph, id)? {
                            tasks.try_push_admitted(
                                Match::Node {
                                    graph,
                                    subject,
                                    frame: Some(frame),
                                    flags: self.base_flags,
                                },
                                self.memory,
                            )?;
                        } else {
                            self.memory.push(&mut results, false)?;
                        }
                    } else {
                        self.memory.push(&mut results, false)?;
                    }
                }
                Match::Node {
                    graph,
                    subject,
                    frame,
                    flags,
                } => {
                    let Some(frame) = frame else {
                        self.memory.push(&mut results, true)?;
                        continue;
                    };
                    let flags = flags.apply(Some(frame))?;
                    charge(&mut self.work, frame.len())?;
                    let properties = &self.views[graph].nodes[subject].properties;
                    let id = self.views[graph].nodes[subject].id;
                    let mut hard_failure = false;
                    let mut constraints = 0;
                    for (key, pattern) in frame.iter() {
                        if key.starts_with('@') && !matches!(key.as_str(), "@id" | "@type") {
                            continue;
                        }
                        constraints += 1;
                        charge(&mut self.work, properties.len())?;
                        let actual = properties
                            .iter()
                            .find(|property| property.key == key)
                            .map(|property| &property.values);
                        if key == "@id" {
                            charge(&mut self.work, values(pattern).len())?;
                            hard_failure |= !identifier_matches(Some(id), values(pattern));
                        } else if key == "@type" {
                            let patterns = values(pattern);
                            let comparisons = patterns
                                .len()
                                .checked_mul(actual.map_or(0, Vec::len))
                                .and_then(|count| count.checked_add(patterns.len()))
                                .ok_or(StorageError::SizeOverflow)?;
                            charge(&mut self.work, comparisons)?;
                            let default =
                                patterns.len() == 1 && patterns[0].get("@default").is_some();
                            hard_failure |= !default
                                && match actual.filter(|values| !values.is_empty()) {
                                    None => !patterns.is_empty(),
                                    Some(actual) => !actual
                                        .iter()
                                        .any(|value| identifier_matches(value.as_str(), patterns)),
                                };
                        } else if values(pattern).is_empty()
                            && actual.is_some_and(|values| !values.is_empty())
                        {
                            hard_failure = true;
                        }
                    }
                    if hard_failure {
                        self.memory.push(&mut results, false)?;
                        continue;
                    }
                    if constraints == 0 {
                        self.memory.push(&mut results, true)?;
                        continue;
                    }
                    tasks.try_push_admitted(
                        Match::Combine {
                            count: constraints,
                            all: flags.require_all,
                        },
                        self.memory,
                    )?;
                    for (key, pattern) in frame.iter() {
                        if key.starts_with('@') && !matches!(key.as_str(), "@id" | "@type") {
                            continue;
                        }
                        if matches!(key.as_str(), "@id" | "@type") {
                            self.memory.push(&mut results, true)?;
                            continue;
                        }
                        let properties = &self.views[graph].nodes[subject].properties;
                        charge(&mut self.work, properties.len())?;
                        let actual = properties
                            .iter()
                            .find(|property| property.key == key)
                            .map(|property| &property.values);
                        if values(pattern).is_empty() {
                            self.memory.push(&mut results, true)?;
                            continue;
                        }
                        let pattern = frame_object(pattern)?;
                        if actual.is_none_or(Vec::is_empty) {
                            let has_default = pattern.contains_key("@default");
                            self.memory
                                .push(&mut results, has_default && flags.require_all)?;
                        } else {
                            // The nonempty guard above established this property.
                            let actual = actual.expect("nonempty property");
                            charge(&mut self.work, actual.len())?;
                            tasks.try_push_admitted(
                                Match::Combine {
                                    count: actual.len(),
                                    all: false,
                                },
                                self.memory,
                            )?;
                            for value in actual {
                                tasks.try_push_admitted(
                                    Match::Item {
                                        graph,
                                        value,
                                        frame: pattern,
                                    },
                                    self.memory,
                                )?;
                            }
                        }
                    }
                }
            }
        }
        // Every match task produces one boolean; combiners consume their inputs.
        let matched = results.pop().expect("one matching result");
        self.memory.release_vec(results)?;
        tasks.release_admitted(self.memory)?;
        Ok(matched)
    }
    fn value_matches(&mut self, value: &Value, pattern: &Object) -> Result<bool, RdfDiagnostic> {
        charge(&mut self.work, 4)?;
        charge(
            &mut self.work,
            pattern
                .get("@language")
                .map_or(0, |value| values(value).len()),
        )?;
        let Some(actual) = value.get("@value") else {
            return Ok(false);
        };
        // Opaque @json is compared as one lexical value, never as JSON-LD.
        let json = value.get("@type").and_then(Value::as_str) == Some("@json");
        let matched_value = if let Some(expected) = pattern.get("@value") {
            if json {
                expected.eq_with_memory(actual, self.memory)?
            } else {
                self.field_matches(Some(actual), Some(expected), false)?
            }
        } else {
            false
        };
        Ok(matched_value
            && self.field_matches(value.get("@type"), pattern.get("@type"), true)?
            && language_matches(value.get("@language"), pattern.get("@language"))
            && self.field_matches(value.get("@direction"), pattern.get("@direction"), true)?)
    }
    fn field_matches(
        &mut self,
        actual: Option<&Value>,
        pattern: Option<&Value>,
        absent_any: bool,
    ) -> Result<bool, RdfDiagnostic> {
        let Some(pattern) = pattern else {
            return Ok(absent_any || actual.is_none());
        };
        let values = values(pattern);
        charge(&mut self.work, values.len())?;
        if values.is_empty() {
            return Ok(actual.is_none());
        }
        if values.len() == 1 && values[0].as_object().is_some_and(Object::is_empty) {
            return Ok(actual.is_some());
        }
        if let Some(actual) = actual {
            for expected in values {
                if expected.eq_with_memory(actual, self.memory)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    #[expect(
        clippy::iter_with_drain,
        reason = "The original admitted count buffer remains alive until release_vec physically destroys and refunds it."
    )]
    fn prune(&mut self, value: &mut Value) -> Result<(), RdfDiagnostic> {
        let mut counts: Vec<(String, usize)> = Vec::new();
        let mut scan: WorkList<_, 8> = WorkList::new();
        scan.try_push_admitted(&*value, self.memory)?;
        while let Some(value) = scan.pop() {
            charge(&mut self.work, 1)?;
            match value {
                Value::Array(values) => {
                    for value in values {
                        scan.try_push_admitted(value, self.memory)?;
                    }
                }
                Value::Object(object) => {
                    let json = object.get("@type").and_then(Value::as_str) == Some("@json");
                    for (key, value) in object.iter() {
                        charge(&mut self.work, 1)?;
                        if json && key == "@value" {
                            continue;
                        }
                        if matches!(key.as_str(), "@id" | "@type" | "@predicate") {
                            for value in values(value) {
                                if let Some(id) = value.as_str().filter(|id| id.starts_with("_:")) {
                                    charge(&mut self.work, counts.len().saturating_add(1))?;
                                    if let Some((_, count)) =
                                        counts.iter_mut().find(|(name, _)| name == id)
                                    {
                                        *count = count.saturating_add(1);
                                    } else {
                                        let id = self.memory.string(id)?;
                                        self.memory.push(&mut counts, (id, 1))?;
                                    }
                                }
                            }
                        }
                        scan.try_push_admitted(value, self.memory)?;
                    }
                }
                _ => {}
            }
        }
        scan.release_admitted(self.memory)?;
        let mut cleanup: WorkList<_, 8> = WorkList::new();
        cleanup.try_push_admitted(value, self.memory)?;
        while let Some(value) = cleanup.pop() {
            charge(&mut self.work, 1)?;
            match value {
                Value::Array(values) => {
                    for value in values {
                        cleanup.try_push_admitted(value, self.memory)?;
                    }
                }
                Value::Object(object) => {
                    // Native triple/annotation values are atomic for cleanup,
                    // while the preceding identity scan counted their references.
                    if object.contains_key("@triple") || object.contains_key("@annotation") {
                        continue;
                    }
                    let json = object.get("@type").and_then(Value::as_str) == Some("@json");
                    let prune = if let Some(id) = object
                        .get("@id")
                        .and_then(Value::as_str)
                        .filter(|id| id.starts_with("_:"))
                    {
                        charge(&mut self.work, counts.len())?;
                        counts.iter().any(|(name, count)| name == id && *count == 1)
                    } else {
                        false
                    };
                    if prune {
                        let mut members = std::mem::take(object).into_members();
                        // The prune predicate established the identifier member.
                        let at = members
                            .iter()
                            .position(|(name, _)| name == "@id")
                            .expect("pruned identifier exists");
                        let (key, mut removed) = members.remove(at);
                        self.memory.release_string(key)?;
                        removed.release_with_memory(self.memory)?;
                        *object = Object::from(members);
                    }
                    for (key, value) in object.iter_mut() {
                        if json && key == "@value" {
                            continue;
                        }
                        // Defaults remain a typed private compaction seam.
                        if key == "@preserve" {
                            continue;
                        }
                        cleanup.try_push_admitted(value, self.memory)?;
                    }
                }
                _ => {}
            }
        }
        cleanup.release_admitted(self.memory)?;
        for (name, _) in counts.drain(..) {
            self.memory.release_string(name)?;
        }
        self.memory.release_vec(counts)?;
        Ok(())
    }
}

fn charge(work: &mut u64, count: usize) -> Result<(), RdfDiagnostic> {
    let count = u64::try_from(count).unwrap_or(u64::MAX);
    *work = work.checked_sub(count).ok_or_else(|| {
        RdfDiagnostic::error(
            "jsonld-framing-work-limit",
            "framing exhausted its deterministic work allowance",
        )
    })?;
    Ok(())
}

fn identifier_matches(actual: Option<&str>, pattern: &[Value]) -> bool {
    if pattern.len() == 1 && pattern[0].as_object().is_some_and(Object::is_empty) {
        return actual.is_some();
    }
    pattern.iter().any(|value| value.as_str() == actual)
}
fn language_matches(actual: Option<&Value>, pattern: Option<&Value>) -> bool {
    match (actual.and_then(Value::as_str), pattern) {
        (_, None) => true,
        (Some(actual), Some(pattern)) => values(pattern).iter().any(|value| {
            value.as_object().is_some_and(Object::is_empty)
                || value
                    .as_str()
                    .is_some_and(|expected| actual.eq_ignore_ascii_case(expected))
        }),
        (None, Some(pattern)) => values(pattern).is_empty(),
    }
}
