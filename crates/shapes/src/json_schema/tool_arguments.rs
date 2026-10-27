// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller-owned plain argument representation for the shared SHACL emitter.

pub(super) mod emission;
pub(super) use emission::emitted_json;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::json_model::{Map, Object, Value, json};
use crate::shapes::{Constraint, NodeKindValue, Path, Shape, Shapes, Target};
use crate::term::Term;
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use purrdf_lex::json::record::IntoJsonNative;
use purrdf_xsd::datatype::{XSD_BOOLEAN, XSD_INTEGER, XSD_STRING};

use super::{
    CompiledSchema, Ctx, Namespaces, compile_member_schema_body, compile_object_schema,
    constraint_term, datatype_value_schema, rewrite_openapi_references, term_enum_value,
};

/// Explicit root and FnO-derived field names for plain JSON arguments.
#[derive(Debug, Clone)]
pub struct ToolArgumentProfile {
    /// Named node shape whose object is the argument root.
    pub root_shape_iri: String,
    /// The root's declared target class; no class is inferred from a field name.
    pub root_class_iri: String,
    /// Complete, injective field bindings, scoped to their owning shape.
    pub fields: Vec<ToolFieldBinding>,
}

/// One RDF predicate's plain member name in one shape.
#[derive(Debug, Clone)]
pub struct ToolFieldBinding {
    /// Owning node shape's IRI.
    pub parent_shape_iri: String,
    /// Direct RDF predicate IRI.
    pub predicate_iri: String,
    /// Exact member name supplied by the implementation's FnO mapping.
    pub implementation_property: String,
}

/// Logical ceilings for one plain-schema compilation.
#[derive(Debug, Clone, Copy)]
pub struct ToolSchemaLimits {
    /// Maximum visits to shapes, constraints, bindings and output values.
    pub work: u64,
    /// Conservative logical admission ceiling for working text, collection
    /// entries and serialized artifacts. Cumulative resident charges and native
    /// admitted storage share this ceiling; released resident work is not refunded.
    /// This is not a measurement of allocator overhead or retained process heap.
    pub bytes: u64,
    /// Conservative admission ceiling for generated JSON values, visited source
    /// constraints and working collection entries, including copies.
    pub nodes: u64,
}

/// An actionable refusal of a plain argument projection.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ToolSchemaError {
    /// The named root is absent, deactivated or does not target the class.
    InvalidRoot {
        /// Requested shape.
        shape: String,
    },
    /// A direct predicate has no caller-supplied member name.
    IncompleteBinding {
        /// Owning shape.
        shape: String,
        /// Unmapped predicate.
        predicate: String,
    },
    /// A shape/predicate pair was supplied more than once.
    DuplicateBinding {
        /// Owning shape.
        shape: String,
        /// Repeated predicate.
        predicate: String,
    },
    /// Member names collide, are reserved, or identify an unused binding.
    AmbiguousBinding {
        /// Owning shape.
        shape: String,
        /// Conflicting member or predicate.
        member: String,
    },
    /// The RDF representation cannot be preserved as plain arguments.
    UnsupportedConstruct {
        /// Shape or field being compiled.
        location: String,
        /// Original RDF predicate, when the refusal belongs to a property.
        predicate: Option<String>,
        /// Construct and cause.
        reason: ToolRepresentationError,
    },
    /// The original retained SHACL model could not resolve a source definition.
    Model {
        /// Original source shape identifier.
        location: String,
        /// Native model cause, available without rendering.
        cause: crate::PreparationRefusal,
    },
    /// The shared compiler refused the original shape.
    Compilation {
        /// Original source shape identifier.
        location: String,
        /// Unmodified shared compiler cause.
        cause: super::SchemaCompileError,
    },
    /// Original pattern translation or compilation refusal.
    Pattern {
        /// Original source shape identifier.
        shape: String,
        /// Bound member carrying the original pattern.
        property: String,
        /// Original RDF predicate, distinct from the bound member name.
        predicate: Option<String>,
        /// Native translator cause.
        cause: purrdf_core::xsd_regex::Ecma262Error,
    },
    /// Native allocation failure, distinct from a caller's logical ceiling.
    Storage {
        /// Original allocation cause.
        cause: StorageError,
    },
    /// Recursive compilation reached the native stack safety margin.
    StackMargin {
        /// Source shape or operation at the refusal.
        location: String,
    },
    /// A caller ceiling was reached; no artifact was returned.
    Exhausted {
        /// Exhausted resource.
        resource: &'static str,
        /// Requested usage.
        used: u64,
        /// Caller ceiling.
        limit: u64,
    },
    /// A resource count could not be represented.
    ArithmeticOverflow {
        /// Overflowing resource.
        resource: &'static str,
    },
}

/// Why an RDF representation has no admitted plain JSON encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToolRepresentationError {
    /// Annotation semantics require the original RDF statement carrier.
    AnnotatedMetadata,
    /// Dynamic activation requires the original RDF evaluation context.
    DynamicDeactivation,
    /// Computed values or a non-predicate path cannot name a plain input member.
    PropertyPath,
    /// No datatype, object shape or finite scalar domain selects an encoding.
    MissingDomain,
    /// The requested lexical-length constraint has no admitted value encoding.
    LexicalLength,
    /// A source constraint requires an unsupported RDF carrier.
    Constraint(&'static str),
    /// A class does not select one unique named object shape.
    ClassTarget,
}

impl std::fmt::Display for ToolRepresentationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AnnotatedMetadata => f.write_str("annotated metadata requires its RDF carrier"),
            Self::DynamicDeactivation => {
                f.write_str("dynamic deactivation requires an RDF evaluation context")
            }
            Self::PropertyPath => {
                f.write_str("computed values or a non-predicate path cannot name a plain member")
            }
            Self::MissingDomain => {
                f.write_str("an explicit datatype, object shape or finite value domain is required")
            }
            Self::LexicalLength => {
                f.write_str("lexical length requires an admitted string or boolean encoding")
            }
            Self::Constraint(term) => write!(f, "{term} has no admitted plain representation"),
            Self::ClassTarget => f.write_str("class has no unique named object shape"),
        }
    }
}
impl std::error::Error for ToolRepresentationError {}

impl std::fmt::Display for ToolSchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRoot { shape } => write!(
                f,
                "plain argument root {shape} is absent, inactive or has a different target class"
            ),
            Self::IncompleteBinding { shape, predicate } => write!(
                f,
                "plain argument shape {shape} has no binding for {predicate}"
            ),
            Self::DuplicateBinding { shape, predicate } => write!(
                f,
                "plain argument shape {shape} repeats the binding for {predicate}"
            ),
            Self::AmbiguousBinding { shape, member } => write!(
                f,
                "plain argument shape {shape} has an ambiguous or unused binding {member}"
            ),
            Self::UnsupportedConstruct {
                location,
                predicate,
                reason,
            } => {
                write!(f, "plain arguments at {location}")?;
                if let Some(predicate) = predicate {
                    write!(f, " (predicate {predicate})")?;
                }
                write!(f, ": {reason}")
            }
            Self::Model { location, cause } => {
                write!(f, "plain argument model at {location}: {cause}")
            }
            Self::Compilation { location, cause } => {
                write!(f, "plain argument compilation at {location}: {cause}")
            }
            Self::Pattern {
                shape,
                property,
                predicate,
                cause,
            } => {
                write!(f, "plain argument pattern at {shape} / {property}")?;
                if let Some(predicate) = predicate {
                    write!(f, " (predicate {predicate})")?;
                }
                write!(f, ": {cause}")
            }
            Self::Storage { cause } => write!(f, "plain argument storage: {cause}"),
            Self::StackMargin { location } => write!(
                f,
                "plain argument compilation at {location} reached the stack safety margin"
            ),
            Self::Exhausted {
                resource,
                used,
                limit,
            } => write!(
                f,
                "plain argument {resource} usage {used} exceeds ceiling {limit}"
            ),
            Self::ArithmeticOverflow { resource } => {
                write!(f, "plain argument {resource} count overflowed")
            }
        }
    }
}
impl std::error::Error for ToolSchemaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnsupportedConstruct { reason, .. } => Some(reason),
            Self::Model { cause, .. } => Some(cause),
            Self::Compilation { cause, .. } => Some(cause),
            Self::Pattern { cause, .. } => Some(cause),
            Self::Storage { cause } => Some(cause),
            _ => None,
        }
    }
}

pub(super) struct ToolPolicy {
    predicate: Option<String>,
    bindings: BTreeMap<(String, String), String>,
    used: BTreeSet<(String, String)>,
    classes: BTreeMap<String, Option<Term>>,
    limits: ToolSchemaLimits,
    work: u64,
    input_bytes: u64,
    admitted_nodes: u64,
    native_live: usize,
    native_nodes: u64,
    error: Option<ToolSchemaError>,
}

impl ToolPolicy {
    fn charge(&mut self, amount: u64) -> bool {
        if self.error.is_some() {
            return false;
        }
        match self.work.checked_add(amount) {
            None => self.error = Some(ToolSchemaError::ArithmeticOverflow { resource: "work" }),
            Some(used) if used > self.limits.work => {
                self.error = Some(ToolSchemaError::Exhausted {
                    resource: "work",
                    used,
                    limit: self.limits.work,
                });
            }
            Some(used) => {
                self.work = used;
                return true;
            }
        }
        false
    }
    fn admit_size(&mut self, bytes: usize, nodes: usize) -> bool {
        let Ok(native_bytes) = u64::try_from(self.native_live) else {
            self.error
                .get_or_insert(ToolSchemaError::ArithmeticOverflow { resource: "bytes" });
            return false;
        };
        for (resource, amount, used, native, limit) in [
            (
                "bytes",
                bytes,
                &mut self.input_bytes,
                native_bytes,
                self.limits.bytes,
            ),
            (
                "nodes",
                nodes,
                &mut self.admitted_nodes,
                self.native_nodes,
                self.limits.nodes,
            ),
        ] {
            let next = u64::try_from(amount).ok().and_then(|n| used.checked_add(n));
            let total = next.and_then(|n| n.checked_add(native));
            match (next, total) {
                (_, None) => {
                    self.error
                        .get_or_insert(ToolSchemaError::ArithmeticOverflow { resource });
                    return false;
                }
                (_, Some(total)) if total > limit => {
                    self.error.get_or_insert(ToolSchemaError::Exhausted {
                        resource,
                        used: total,
                        limit,
                    });
                    return false;
                }
                (Some(next), Some(_)) => *used = next,
                _ => unreachable!("a checked total has a checked source count"),
            }
        }
        true
    }
    fn text(&mut self, text: &str) -> bool {
        if !self.admit_size(text.len(), 0) {
            return false;
        }
        match u64::try_from(text.len()) {
            Ok(n) => self.charge(n),
            Err(_) => {
                self.error
                    .get_or_insert(ToolSchemaError::ArithmeticOverflow { resource: "work" });
                false
            }
        }
    }
    fn native<T>(
        &mut self,
        operation: impl FnOnce(&mut Memory<'_, Self>) -> Result<T, StorageError>,
    ) -> Option<T> {
        if self.error.is_some() {
            return None;
        }
        let live = self.native_live;
        let (result, live) = {
            let mut memory = Memory::resume(self, live);
            let result = memory.scope(operation);
            (result, memory.admitted_bytes())
        };
        self.native_live = live;
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                let refusal = ToolSchemaError::Storage { cause: error };
                self.error.get_or_insert(refusal);
                None
            }
        }
    }

    fn native_nodes(&mut self, count: u64) -> bool {
        if self.error.is_some() {
            return false;
        }
        let Some(used) = self.native_nodes.checked_add(count) else {
            self.error = Some(ToolSchemaError::ArithmeticOverflow { resource: "nodes" });
            return false;
        };
        let Some(total) = used.checked_add(self.admitted_nodes) else {
            self.error = Some(ToolSchemaError::ArithmeticOverflow { resource: "nodes" });
            return false;
        };
        if total > self.limits.nodes {
            self.error = Some(ToolSchemaError::Exhausted {
                resource: "nodes",
                used: total,
                limit: self.limits.nodes,
            });
            return false;
        }
        self.native_nodes = used;
        true
    }

    fn refuse(&mut self, location: &str, reason: ToolRepresentationError) {
        if self.error.is_none() {
            self.error = Some(ToolSchemaError::UnsupportedConstruct {
                location: location.to_owned(),
                predicate: self.predicate.clone(),
                reason,
            });
        }
    }
}

/// Keep the original RDF path while nested scalar compilation uses a bound member.
pub(super) fn with_predicate<'ns, T>(
    ctx: &mut Ctx<'ns>,
    predicate: &str,
    operation: impl FnOnce(&mut Ctx<'ns>) -> T,
) -> T {
    if ctx.tool.is_none() || predicate.is_empty() {
        return operation(ctx);
    }
    let predicate = copy_text(ctx, predicate);
    let previous = ctx
        .tool
        .as_mut()
        .and_then(|policy| policy.predicate.replace(predicate));
    let result = operation(ctx);
    if let Some(policy) = &mut ctx.tool {
        policy.predicate = previous;
    }
    result
}

pub(super) fn format_text(ctx: &mut Ctx<'_>, arguments: std::fmt::Arguments<'_>) -> String {
    format_policy(&mut ctx.tool, arguments)
}

pub(super) fn format_policy(
    tool: &mut Option<ToolPolicy>,
    arguments: std::fmt::Arguments<'_>,
) -> String {
    match tool {
        Some(policy) => policy
            .native(|memory| memory.format(&arguments))
            .unwrap_or_default(),
        None => arguments.to_string(),
    }
}

pub(super) fn insert_entry(
    ctx: &mut Ctx<'_>,
    target: &mut Map<String, Value>,
    name: &str,
    value: Value,
) -> Option<Value> {
    if let Some(policy) = &mut ctx.tool
        && (!policy.charge(1) || !policy.admit_size(name.len(), 1))
    {
        return None;
    }
    target.insert(name.to_owned(), value)
}

pub(super) fn resident(ctx: &mut Ctx<'_>, bytes: usize, entries: usize) -> bool {
    ctx.tool
        .as_mut()
        .is_none_or(|policy| policy.charge(entries as u64) && policy.admit_size(bytes, entries))
}

pub(super) fn value_entries(ctx: &mut Ctx<'_>, count: usize) -> bool {
    let Some(bytes) = count.checked_mul(size_of::<Value>()) else {
        if let Some(policy) = &mut ctx.tool {
            policy
                .error
                .get_or_insert(ToolSchemaError::ArithmeticOverflow { resource: "bytes" });
        }
        return false;
    };
    resident(ctx, bytes, count)
}

macro_rules! values {
    ($ctx:expr; $count:expr; $values:expr) => {{
        if $crate::json_schema::tool_arguments::value_entries($ctx, $count) {
            $values.collect::<Vec<_>>()
        } else {
            Vec::new()
        }
    }};
}
pub(super) use values;

pub(super) fn copy_text(ctx: &mut Ctx<'_>, text: &str) -> String {
    match &mut ctx.tool {
        Some(policy) => policy
            .native(|memory| memory.string(text))
            .unwrap_or_default(),
        None => text.to_owned(),
    }
}

pub(super) fn join(ctx: &mut Ctx<'_>, parts: &[String], separator: &str) -> String {
    let Some(policy) = &mut ctx.tool else {
        return parts.join(separator);
    };
    policy
        .native(|memory| {
            let mut text = String::new();
            for (index, part) in parts.iter().enumerate() {
                if index != 0 {
                    memory.push_str(&mut text, separator)?;
                }
                memory.push_str(&mut text, part)?;
            }
            Ok(text)
        })
        .unwrap_or_default()
}

pub(super) fn path_text(ctx: &mut Ctx<'_>, path: &Path) -> String {
    let Some(policy) = &mut ctx.tool else {
        return crate::path::path_to_sparql(path);
    };
    policy
        .native(|memory| {
            let mut pending = purrdf_lex::walk::WorkList::<&Path, 16>::new();
            pending.try_push_admitted(path, memory)?;
            while let Some(path) = pending.pop() {
                // One source admission visit and one rendering visit.
                if !memory.admission_mut().charge(2) {
                    return Err(StorageError::AdmissionFailed);
                }
                match path {
                    Path::Predicate(_) => {}
                    Path::Sequence(parts) | Path::Alternative(parts) => {
                        for part in parts {
                            pending.try_push_admitted(part, memory)?;
                        }
                    }
                    Path::Inverse(inner)
                    | Path::ZeroOrMore(inner)
                    | Path::OneOrMore(inner)
                    | Path::ZeroOrOne(inner) => {
                        pending.try_push_admitted(inner, memory)?;
                    }
                }
            }
            pending.release_admitted(memory)?;
            crate::path::path_to_sparql_with_memory(path, memory)
        })
        .unwrap_or_default()
}

macro_rules! insert {
    ($ctx:expr; $target:expr, $name:expr, $value:expr $(,)?) => {{
        let value = $value;
        let name = $name;
        $crate::json_schema::tool_arguments::insert_entry($ctx, $target, &name, value)
    }};
}
pub(super) use insert;

macro_rules! emission_array {
    ($ctx:expr; $values:expr $(,)?) => {{
        let values = $values;
        $crate::json_schema::tool_arguments::emission::array($ctx, values)
    }};
}
pub(super) use emission_array;

pub(super) fn push_entry<T>(ctx: &mut Ctx<'_>, target: &mut Vec<T>, value: T) {
    if let Some(policy) = &mut ctx.tool
        && (!policy.charge(1) || !policy.admit_size(size_of::<T>(), 1))
    {
        return;
    }
    target.push(value);
}

pub(super) fn extend<T>(
    ctx: &mut Ctx<'_>,
    target: &mut Vec<T>,
    values: impl IntoIterator<Item = T>,
) {
    for value in values {
        push_entry(ctx, target, value);
    }
}

pub(super) fn clone_values(ctx: &mut Ctx<'_>, values: &[Value]) -> Vec<Value> {
    crate::json_model::into_array(emission::value(ctx, values)).unwrap_or_default()
}

macro_rules! push {
    ($ctx:expr; $target:expr, $value:expr $(,)?) => {{
        let value = $value;
        $crate::json_schema::tool_arguments::push_entry($ctx, $target, value)
    }};
}
pub(super) use push;

macro_rules! format {
    ($ctx:expr; $($arguments:tt)*) => {
        $crate::json_schema::tool_arguments::format_text($ctx, format_args!($($arguments)*))
    };
}
pub(super) use format;

macro_rules! record {
    ($ctx:expr; $code:expr, $subject:expr, $note:expr $(,)?) => {{
        let note = $note;
        $crate::json_schema::tool_arguments::record_loss($ctx, $code, $subject, note)
    }};
}
pub(super) use record;

impl Admission for ToolPolicy {
    fn resize(&mut self, live_bytes: usize) -> Result<(), StorageError> {
        // Shrink remains possible after the first refusal, after native scratch
        // has been destroyed. No later failure replaces that original cause.
        if live_bytes <= self.native_live {
            self.native_live = live_bytes;
            return Ok(());
        }
        if self.error.is_some() {
            return Err(StorageError::AdmissionFailed);
        }
        let used = u64::try_from(live_bytes).map_err(|_| {
            self.error = Some(ToolSchemaError::ArithmeticOverflow { resource: "bytes" });
            StorageError::SizeOverflow
        })?;
        let used = used.checked_add(self.input_bytes).ok_or_else(|| {
            self.error = Some(ToolSchemaError::ArithmeticOverflow { resource: "bytes" });
            StorageError::SizeOverflow
        })?;
        if used > self.limits.bytes {
            self.error = Some(ToolSchemaError::Exhausted {
                resource: "bytes",
                used,
                limit: self.limits.bytes,
            });
            return Err(StorageError::AdmissionFailed);
        }
        self.native_live = live_bytes;
        Ok(())
    }
}

// These constructors accept already admitted child values and move them. They
// never adopt a resident compiler result after birth.
pub(super) fn collect<T>(ctx: &mut Ctx<'_>, values: impl IntoIterator<Item = T>) -> Vec<T> {
    let Some(policy) = &mut ctx.tool else {
        return values.into_iter().collect();
    };
    policy
        .native(|memory| {
            let mut output = Vec::new();
            memory.extend(&mut output, values)?;
            Ok(output)
        })
        .unwrap_or_default()
}

pub(super) fn record_loss(ctx: &mut Ctx<'_>, code: &str, subject: &str, note: &str) {
    let Some(policy) = &mut ctx.tool else {
        ctx.ledger.record(super::loss_entry(code, subject, note));
        return;
    };
    if !policy.charge(1) || !policy.native_nodes(1) {
        return;
    }
    policy.native(|memory| {
        let code = memory.string(code)?;
        let note = memory.string(note)?;
        let subject = memory.string(subject)?;
        let location = memory.boxed(super::RdfLocation::default().with_subject(subject))?;
        ctx.ledger.record_with_memory(
            super::LossEntry {
                code: code.into(),
                from: "shacl".into(),
                to: "json-schema".into(),
                note: note.into(),
                location: Some(location),
            },
            memory,
        )
    });
}

pub(super) fn native_value(ctx: &mut Ctx<'_>, source: impl IntoJsonNative) -> Value {
    let policy = ctx
        .tool
        .as_mut()
        .expect("native factory requires plain mode");
    if !policy.native_nodes(1) {
        return Value::Null;
    }
    policy
        .native(|memory| source.into_json_native(memory))
        .unwrap_or(Value::Null)
}

pub(super) fn native_object<'a>(
    ctx: &mut Ctx<'_>,
    members: impl IntoIterator<Item = (&'a str, Value)>,
) -> Value {
    let policy = ctx
        .tool
        .as_mut()
        .expect("native factory requires plain mode");
    if !policy.native_nodes(1) {
        return Value::Null;
    }
    policy
        .native(|memory| {
            let mut object = Object::new();
            for (name, value) in members {
                object = object.with_memory(name, value, memory)?;
            }
            let mut members = object.into_members();
            members.sort_unstable_by(|a, b| a.0.cmp(&b.0));
            Ok(Value::Object(Object::from(members)))
        })
        .unwrap_or(Value::Null)
}

pub(super) fn native_array(ctx: &mut Ctx<'_>, values: impl IntoIterator<Item = Value>) -> Value {
    let policy = ctx
        .tool
        .as_mut()
        .expect("native factory requires plain mode");
    if !policy.native_nodes(1) {
        return Value::Null;
    }
    policy
        .native(|memory| {
            let mut array = Vec::new();
            memory.extend(&mut array, values)?;
            Ok(Value::Array(array))
        })
        .unwrap_or(Value::Null)
}

pub(super) fn native_map(ctx: &mut Ctx<'_>, values: Map<String, Value>) -> Value {
    let policy = ctx.tool.as_mut().expect("native map requires plain mode");
    if !policy.native_nodes(1) {
        return Value::Null;
    }
    policy
        .native(|memory| {
            let mut members = Vec::new();
            memory.extend(&mut members, values)?;
            Ok(Value::Object(Object::from(members)))
        })
        .unwrap_or(Value::Null)
}

pub(super) fn map(ctx: &mut Ctx<'_>, values: Map<String, Value>) -> Value {
    if ctx.tool.is_some() {
        native_map(ctx, values)
    } else {
        crate::json_model::object(values)
    }
}

pub(super) fn clone_map(ctx: &mut Ctx<'_>, values: &Map<String, Value>) -> Value {
    let Some(policy) = &mut ctx.tool else {
        return crate::json_model::object(values.clone());
    };
    if !policy.native_nodes(1)
        || !values
            .values()
            .all(|value| admit_native_clone(value, policy))
    {
        return Value::Null;
    }
    policy
        .native(|memory| {
            let mut object = Object::new();
            for (name, value) in values {
                let value = value.clone_with_memory(memory)?;
                object = object.with_memory(name, value, memory)?;
            }
            Ok(Value::Object(object))
        })
        .unwrap_or(Value::Null)
}

fn native_insert(ctx: &mut Ctx<'_>, target: &mut Value, name: &str, value: Value) {
    let Value::Object(object) = target else {
        return;
    };
    let old = std::mem::take(object);
    let policy = ctx
        .tool
        .as_mut()
        .expect("native member requires plain mode");
    if let Some(mut inserted) = policy.native(|memory| old.with_memory(name, value, memory)) {
        inserted.sort_keys();
        *object = inserted;
    }
}

fn admit_native_clone(value: &Value, policy: &mut ToolPolicy) -> bool {
    if !policy.charge(1) || !policy.native_nodes(1) {
        return false;
    }
    if purrdf_stack::is_low() {
        policy.error.get_or_insert(ToolSchemaError::StackMargin {
            location: "schema value clone".to_owned(),
        });
        return false;
    }
    match value {
        Value::Object(object) => object.iter().all(|(key, child)| {
            policy.charge(key.len() as u64) && admit_native_clone(child, policy)
        }),
        Value::Array(array) => array.iter().all(|child| admit_native_clone(child, policy)),
        Value::String(text) => policy.charge(text.len() as u64),
        Value::Number(number) => policy.charge(number.lexeme().len() as u64),
        _ => true,
    }
}

pub(super) fn native_clone(ctx: &mut Ctx<'_>, value: &Value) -> Value {
    let policy = ctx
        .tool
        .as_mut()
        .expect("native factory requires plain mode");
    if !admit_native_clone(value, policy) {
        return Value::Null;
    }
    policy
        .native(|memory| value.clone_with_memory(memory))
        .unwrap_or(Value::Null)
}

pub(super) fn sort_values(ctx: &mut Ctx<'_>, values: &mut Vec<Value>) {
    let Some(policy) = &mut ctx.tool else {
        crate::term::sort_canonical(values);
        return;
    };
    if !values.iter().all(|value| value_work(value, policy)) {
        return;
    }
    let Some(mut keyed) = policy.native(|memory| {
        let mut keyed = Vec::new();
        for value in values.drain(..) {
            let key = purrdf_lex::json::write_with_memory(
                &value,
                purrdf_lex::json::Format::COMPACT,
                memory,
            )?;
            memory.push(&mut keyed, (key, value))?;
        }
        Ok(keyed)
    }) else {
        return;
    };
    if purrdf_lex::walk::try_sort_unstable_by(&mut keyed, |a, b| {
        if policy.charge(a.0.len().min(b.0.len()) as u64 + 1) {
            Ok(a.0.cmp(&b.0))
        } else {
            Err(())
        }
    })
    .is_err()
    {
        return;
    }
    // The destination retains its original buffer, large enough for every
    // drained value. Keys are scratch and are released after destruction.
    policy.native(|memory| {
        #[expect(
            clippy::iter_with_drain,
            reason = "the emptied allocation must be returned through release_vec"
        )]
        for (key, value) in keyed.drain(..) {
            memory.release_string(key)?;
            values.push(value);
        }
        memory.release_vec(keyed)
    });
}

fn value_work(value: &Value, policy: &mut ToolPolicy) -> bool {
    if !policy.charge(1) {
        return false;
    }
    if purrdf_stack::is_low() {
        policy.error.get_or_insert(ToolSchemaError::StackMargin {
            location: "schema value traversal".to_owned(),
        });
        return false;
    }
    match value {
        Value::Object(object) => object
            .iter()
            .all(|(key, child)| policy.charge(key.len() as u64) && value_work(child, policy)),
        Value::Array(array) => array.iter().all(|child| value_work(child, policy)),
        Value::String(text) => policy.charge(text.len() as u64),
        Value::Number(number) => policy.charge(number.lexeme().len() as u64),
        _ => true,
    }
}

fn native_pretty(value: &Value, policy: &mut ToolPolicy) -> Option<String> {
    if !value_work(value, policy) {
        return None;
    }
    policy.native(|memory| {
        let mut text =
            purrdf_lex::json::write_with_memory(value, purrdf_lex::json::Format::PRETTY, memory)?;
        memory.push_char(&mut text, '\n')?;
        Ok(text)
    })
}

/// Compile plain tool arguments through the existing SHACL schema emitter.
///
/// A property with `sh:maxCount 1` is scalar; every other property is an array.
/// Object values use bound member names and carry no JSON-LD keywords. String,
/// integer and boolean values retain their native JSON forms. Other RDF literal
/// encodings are refused, rather than coerced. Existing unmodeled constraints
/// remain visible in the loss ledger and schema comments. An inactive property
/// of an open object needs no binding; an inactive declared direct path of a
/// closed object still requires a binding for its permitted member name.
/// Each leaf requires
/// an explicit datatype or finite value domain; an object requires a shape or
/// admitted object node kind. A member name alone never selects a value encoding.
///
/// # Errors
/// Returns a typed refusal for invalid roots, incomplete or non-injective field
/// bindings, unsupported RDF encodings, compilation failures and resource limits.
/// No partial artifact is returned.
///
/// ```
/// use purrdf_shapes::json_schema::{compile_tool_arguments, Namespaces,
///     ToolArgumentProfile, ToolFieldBinding, ToolSchemaLimits};
/// let dataset = purrdf_shapes::text_ingest::parse_turtle_to_dataset(r#"
///     @prefix sh: <http://www.w3.org/ns/shacl#> .
///     @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
///     @prefix ex: <https://example.org/> .
///     ex:Arguments a sh:NodeShape; sh:targetClass ex:Call; sh:closed true;
///         sh:property [sh:path ex:customer; sh:datatype xsd:string;
///                      sh:minCount 1; sh:maxCount 1].
/// "#, None).map_err(|errors| errors.join("; "))?;
/// let shapes = purrdf_shapes::shapes::from_dataset(&dataset)?;
/// let namespaces = Namespaces::new("ex", &[("ex".into(), "https://example.org/".into())])?;
/// let profile = ToolArgumentProfile {
///     root_shape_iri: "https://example.org/Arguments".into(),
///     root_class_iri: "https://example.org/Call".into(),
///     fields: vec![ToolFieldBinding {
///         parent_shape_iri: "https://example.org/Arguments".into(),
///         predicate_iri: "https://example.org/customer".into(),
///         implementation_property: "customer_id".into(),
///     }],
/// };
/// let limits = ToolSchemaLimits { work: 100_000, bytes: 64_000_000, nodes: 1_000_000 };
/// let compiled = compile_tool_arguments(&shapes, &namespaces, &profile, &limits)?;
/// assert!(compiled.schema_json.contains("customer_id"));
/// assert!(!compiled.schema_json.contains("@id"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// The resulting object schema contains the following keywords, alongside its
/// draft declaration and empty definition catalog:
///
/// ```json
/// {"type":"object","properties":{"customer_id":{"type":"string"}},
///  "required":["customer_id"],"additionalProperties":false}
/// ```
///
/// It accepts `{"customer_id":"customer-17"}` and rejects a missing customer,
/// an array in place of the string, or an undeclared member.
#[expect(
    clippy::result_large_err,
    reason = "retain the native compiler cause without allocating while reporting refusal"
)]
pub fn compile_tool_arguments(
    shapes: &Shapes,
    namespaces: &Namespaces,
    profile: &ToolArgumentProfile,
    limits: &ToolSchemaLimits,
) -> Result<CompiledSchema, ToolSchemaError> {
    let mut policy = ToolPolicy {
        predicate: None,
        bindings: BTreeMap::new(),
        used: BTreeSet::new(),
        classes: BTreeMap::new(),
        limits: *limits,
        work: 0,
        input_bytes: 0,
        admitted_nodes: 0,
        native_live: 0,
        native_nodes: 0,
        error: None,
    };
    let mut output_names = BTreeSet::new();
    for binding in &profile.fields {
        if !policy.charge(1) {
            break;
        }
        let parent = &binding.parent_shape_iri;
        let predicate = &binding.predicate_iri;
        let member = &binding.implementation_property;
        if !(policy.text(parent) && policy.text(predicate) && policy.text(member)) {
            break;
        }
        // The lookup pair, injectivity set and binding entry each own their
        // named text. Charge those concrete copies before constructing them.
        if ![parent, predicate, parent, member, parent, predicate, member]
            .into_iter()
            .all(|text| policy.text(text))
            || !policy.admit_size(0, 3)
        {
            break;
        }
        if policy
            .bindings
            .contains_key(&(parent.clone(), predicate.clone()))
        {
            return Err(ToolSchemaError::DuplicateBinding {
                shape: parent.clone(),
                predicate: predicate.clone(),
            });
        }
        if member.is_empty()
            || member.starts_with('@')
            || !output_names.insert((parent.clone(), member.clone()))
        {
            return Err(ToolSchemaError::AmbiguousBinding {
                shape: parent.clone(),
                member: member.clone(),
            });
        }
        if policy
            .bindings
            .insert((parent.clone(), predicate.clone()), member.clone())
            .is_some()
        {
            return Err(ToolSchemaError::DuplicateBinding {
                shape: parent.clone(),
                predicate: predicate.clone(),
            });
        }
    }
    let mut root = None;
    if !policy.text(&profile.root_shape_iri) || !policy.text(&profile.root_class_iri) {
        return Err(policy.error.take().expect("root admission failed"));
    }
    for handle in &shapes.node_shapes {
        if !policy.charge(1) {
            break;
        }
        let shape =
            handle
                .resolved_in(Some(&shapes.owner))
                .map_err(|e| ToolSchemaError::Model {
                    location: handle.id.to_string(),
                    cause: e,
                })?;
        if shape.deactivated {
            continue;
        }
        if let Term::NamedNode(id) = &shape.id {
            if !policy.text(id.as_str()) {
                break;
            }
            for target in &shape.targets {
                if !policy.charge(1) {
                    break;
                }
                let class = match target {
                    Target::Class(c) | Target::ImplicitClass(Term::NamedNode(c)) => c.as_str(),
                    _ => continue,
                };
                if !policy.text(class) || !policy.text(id.as_str()) || !policy.admit_size(0, 1) {
                    break;
                }
                if id.as_str() == profile.root_shape_iri && class == profile.root_class_iri {
                    root = Some(shape);
                }
                policy
                    .classes
                    .entry(class.to_owned())
                    .and_modify(|previous| {
                        if previous.as_ref() != Some(&shape.id) {
                            *previous = None;
                        }
                    })
                    .or_insert_with(|| Some(shape.id.clone()));
            }
        }
    }
    if let Some(error) = policy.error.take() {
        return Err(error);
    }
    let root = root.ok_or_else(|| ToolSchemaError::InvalidRoot {
        shape: profile.root_shape_iri.clone(),
    })?;
    let mut ctx = Ctx::new(
        BTreeSet::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        namespaces,
    );
    ctx.model = Some(Arc::clone(&shapes.owner));
    ctx.tool = Some(policy);
    let body = compile_object_schema(root, &mut ctx);
    let mut defs = Map::new();
    ctx.append_catalog_defs(&mut defs)
        .map_err(|e| ToolSchemaError::Compilation {
            location: profile.root_shape_iri.clone(),
            cause: e,
        })?;
    let policy = ctx.tool.as_mut().expect("plain policy retained");
    if let Some(error) = policy.error.take() {
        return Err(error);
    }
    for pair @ (shape, predicate) in policy.bindings.keys() {
        if !policy.used.contains(pair) {
            return Err(ToolSchemaError::AmbiguousBinding {
                shape: shape.clone(),
                member: predicate.clone(),
            });
        }
    }
    // Each retained artifact needs one original tree; clone through its native
    // account once, then move children into their final containers.
    let mut schema = native_clone(&mut ctx, &body);
    let definitions = native_map(&mut ctx, defs);
    let mut openapi_definitions = native_clone(&mut ctx, &definitions);
    let draft = native_value(&mut ctx, "https://json-schema.org/draft/2020-12/schema");
    native_insert(&mut ctx, &mut schema, "$schema", draft);
    native_insert(&mut ctx, &mut schema, "$defs", definitions);
    native_insert(&mut ctx, &mut openapi_definitions, "Arguments", body);
    if let Value::Object(definitions) = &mut openapi_definitions {
        ctx.tool
            .as_mut()
            .expect("plain policy retained")
            .native(|memory| rewrite_openapi_references(definitions.values_mut(), memory));
    }
    let components = native_object(&mut ctx, [("schemas", openapi_definitions)]);
    let title = native_value(&mut ctx, "Tool arguments");
    let version = native_value(&mut ctx, crate::VERSION);
    let info = native_object(&mut ctx, [("title", title), ("version", version)]);
    let paths = native_object(&mut ctx, []);
    let version = native_value(&mut ctx, "3.1.0");
    let openapi = native_object(
        &mut ctx,
        [
            ("openapi", version),
            ("info", info),
            ("paths", paths),
            ("components", components),
        ],
    );
    let mut policy = ctx.tool.take().expect("plain policy retained");
    if let Some(error) = policy.error.take() {
        return Err(error);
    }
    let Some(schema_json) = native_pretty(&schema, &mut policy) else {
        return Err(policy.error.take().expect("native writer refusal"));
    };
    let Some(openapi_json) = native_pretty(&openapi, &mut policy) else {
        return Err(policy.error.take().expect("native writer refusal"));
    };
    Ok(CompiledSchema {
        schema_json,
        openapi_json,
        losses: ctx.ledger,
    })
}

pub(super) fn field_key(ctx: &mut Ctx<'_>, shape: &str, predicate: &str) -> String {
    let shape = shape
        .strip_prefix('<')
        .and_then(|s| s.strip_suffix('>'))
        .unwrap_or(shape);
    let Some(policy) = &mut ctx.tool else {
        return ctx.ns.compact_iri(predicate);
    };
    if !policy.charge(1)
        || !policy.text(shape)
        || !policy.text(predicate)
        || !policy.admit_size(0, 1)
    {
        return String::new();
    }
    let pair = (shape.to_owned(), predicate.to_owned());
    if let Some(length) = policy.bindings.get(&pair).map(String::len)
        && !policy.admit_size(length, 1)
    {
        return String::new();
    }
    if let Some(member) = policy.bindings.get(&pair) {
        policy.used.insert(pair);
        return member.clone();
    }
    policy
        .error
        .get_or_insert_with(|| ToolSchemaError::IncompleteBinding {
            shape: shape.to_owned(),
            predicate: predicate.to_owned(),
        });
    String::new()
}

pub(super) fn admit_object(shape: &Shape, ctx: &mut Ctx<'_>) -> bool {
    let Some(policy) = &mut ctx.tool else {
        return true;
    };
    if !policy.charge(1) || !policy.admit_size(0, 1) {
        return false;
    }
    let Some(location) = policy.native(|memory| memory.format(&shape.id)) else {
        return false;
    };
    if !shape.constraint_annotations.is_empty() {
        policy.refuse(&location, ToolRepresentationError::AnnotatedMetadata);
        return false;
    }
    if !shape.deactivated_when.is_empty() {
        policy.refuse(&location, ToolRepresentationError::DynamicDeactivation);
        return false;
    }
    if purrdf_stack::is_low() {
        policy
            .error
            .get_or_insert(ToolSchemaError::StackMargin { location });
        return false;
    }
    if !admit_expansion(&shape.constraints, policy) {
        return false;
    }
    for c in &shape.constraints {
        if !policy.charge(1) {
            return false;
        }
        if !matches!(
            c,
            Constraint::Reference(_)
                | Constraint::PropertyReference(_)
                | Constraint::Node(_)
                | Constraint::And(_)
                | Constraint::Or(_)
                | Constraint::Xone(_)
                | Constraint::Not(_)
                | Constraint::SomeValue(_)
                | Constraint::Sparql { .. }
                | Constraint::Expression { .. }
                | Constraint::NodeByExpression { .. }
                | Constraint::Class(_)
                | Constraint::RootClass(_)
                | Constraint::Component { .. }
                | Constraint::Equals(_)
                | Constraint::Disjoint(_)
                | Constraint::SubsetOf(_)
                | Constraint::LessThan(_)
                | Constraint::LessThanOrEquals(_)
                | Constraint::UniqueValuesFor { .. }
                | Constraint::SingleLine(_)
                | Constraint::Closed { .. }
        ) {
            policy.refuse(
                &location,
                ToolRepresentationError::Constraint(constraint_term(c)),
            );
            return false;
        }
    }
    for view in shape.properties_in(ctx.model.as_deref()) {
        if !policy.charge(1) || !policy.admit_size(0, 1) {
            return false;
        }
        match view {
            Ok(view) if view.deactivated() => {}

            Ok(view)
                if matches!(view.property.path, Path::Predicate(_))
                    && view.property.values.is_none()
                    && view.property.default_value.is_none()
                    && view.property.property_shapes.is_empty()
                    && view.property.constraint_annotations.is_empty()
                    && view.property.deactivated_when.is_empty()
                    && view
                        .edge
                        .is_none_or(|edge| edge.deactivated_when.is_empty()) =>
            {
                if !policy
                    .charge(u64::try_from(view.property.reifier_shapes.len()).unwrap_or(u64::MAX))
                {
                    return false;
                }
            }
            Ok(view) => {
                let predicate = match &view.property.path {
                    Path::Predicate(predicate) => {
                        let Some(predicate) =
                            policy.native(|memory| memory.string(predicate.as_str()))
                        else {
                            return false;
                        };
                        Some(predicate)
                    }
                    _ => None,
                };
                let previous = std::mem::replace(&mut policy.predicate, predicate);
                policy.refuse(&location, ToolRepresentationError::PropertyPath);
                policy.predicate = previous;
                return false;
            }
            Err(error) => {
                policy.error.get_or_insert(ToolSchemaError::Model {
                    location: location.clone(),
                    cause: error,
                });
                return false;
            }
        }
    }
    true
}

pub(super) fn admit_property_storage(constraints: &[Constraint], ctx: &mut Ctx<'_>) -> bool {
    ctx.tool
        .as_mut()
        .is_none_or(|policy| admit_expansion(constraints, policy))
}

pub(super) fn admit_property(
    constraints: &[&Constraint],
    shape: &str,
    key: &str,
    ctx: &mut Ctx<'_>,
) -> bool {
    let Some(policy) = &mut ctx.tool else {
        return true;
    };
    if !constraints
        .iter()
        .all(|c| admit_expansion(std::slice::from_ref(*c), policy))
    {
        return false;
    }
    let explicit_domain = constraints.iter().any(|constraint| match constraint {
        Constraint::Datatype(_)
        | Constraint::Class(_)
        | Constraint::Node(_)
        | Constraint::Reference(_)
        | Constraint::And(_)
        | Constraint::Or(_)
        | Constraint::Xone(_)
        | Constraint::In(_) => true,
        Constraint::NodeKind(kinds) => kinds.iter().all(|kind| {
            matches!(
                kind,
                NodeKindValue::BlankNode | NodeKindValue::BlankNodeOrIri
            )
        }),
        _ => false,
    });
    if !explicit_domain {
        policy.refuse(
            &std::format!("{shape} / {key}"),
            ToolRepresentationError::MissingDomain,
        );
        return false;
    }
    if constraints.iter().any(|c| matches!(c, Constraint::MinLength(_) | Constraint::MaxLength(_)))
        && !constraints.iter().any(|c| matches!(c, Constraint::Datatype(types) if types.iter().all(|t| matches!(t.as_str(), XSD_STRING | XSD_BOOLEAN)))) {
        policy.refuse(&std::format!("{shape} / {key}"), ToolRepresentationError::LexicalLength);
        return false;
    }
    for c in constraints {
        if !policy.charge(1) {
            return false;
        }
        let accepted = match c {
            Constraint::Datatype(types) => types
                .iter()
                .all(|t| matches!(t.as_str(), XSD_STRING | XSD_INTEGER | XSD_BOOLEAN)),
            Constraint::In(terms) => terms.iter().all(scalar_term),
            Constraint::HasValue(term) => scalar_term(term),
            Constraint::NodeKind(kinds) => kinds.iter().all(|k| {
                matches!(
                    k,
                    NodeKindValue::Literal
                        | NodeKindValue::BlankNode
                        | NodeKindValue::BlankNodeOrIri
                )
            }),
            Constraint::MinInclusive(t)
            | Constraint::MaxInclusive(t)
            | Constraint::MinExclusive(t)
            | Constraint::MaxExclusive(t) => {
                scalar_term(t) && matches!(t, Term::Literal(l) if l.datatype_str() == XSD_INTEGER)
            }
            Constraint::Reference(_)
            | Constraint::Class(_)
            | Constraint::Node(_)
            | Constraint::And(_)
            | Constraint::Or(_)
            | Constraint::Xone(_)
            | Constraint::Not(_)
            | Constraint::SomeValue(_)
            | Constraint::MinCount(_)
            | Constraint::MaxCount(_)
            | Constraint::Pattern { .. }
            | Constraint::MinLength(_)
            | Constraint::MaxLength(_)
            | Constraint::SingleLine(_)
            | Constraint::Sparql { .. }
            | Constraint::Expression { .. }
            | Constraint::NodeByExpression { .. }
            | Constraint::Equals(_)
            | Constraint::Disjoint(_)
            | Constraint::SubsetOf(_)
            | Constraint::LessThan(_)
            | Constraint::LessThanOrEquals(_)
            | Constraint::UniqueValuesFor { .. }
            | Constraint::Component { .. }
            | Constraint::RootClass(_)
            | Constraint::UniqueLang(false) => true,
            _ => false,
        };
        if !accepted {
            policy.refuse(
                &std::format!("{shape} / {key}"),
                ToolRepresentationError::Constraint(constraint_term(c)),
            );
            return false;
        }
    }
    true
}

pub(super) fn scalar_term(term: &Term) -> bool {
    match term {
        Term::Literal(l) if l.datatype_str() == XSD_STRING => true,
        Term::Literal(l) if l.datatype_str() == XSD_INTEGER => {
            l.value().parse::<i128>().is_ok_and(|n| {
                let native_integer = i64::try_from(n).is_ok() || u64::try_from(n).is_ok();
                native_integer && n.to_string() == l.value()
            })
        }
        Term::Literal(l) if l.datatype_str() == XSD_BOOLEAN => {
            matches!(l.value(), "true" | "false")
        }
        _ => false,
    }
}

pub(super) fn datatype_schema(datatype: &str, ctx: &mut Ctx<'_>) -> Value {
    if ctx.tool.is_none() {
        return datatype_value_schema(datatype, ctx.ns);
    }
    let kind = match datatype {
        XSD_STRING => "string",
        XSD_INTEGER => "integer",
        XSD_BOOLEAN => "boolean",
        _ => return native_value(ctx, false),
    };
    let kind = native_value(ctx, kind);
    native_object(ctx, [("type", kind)])
}

pub(super) fn enum_value(term: &Term, ctx: &mut Ctx<'_>) -> Value {
    if ctx.tool.is_none() {
        return term_enum_value(term, ctx.ns);
    }
    match term {
        Term::Literal(literal) if literal.datatype_str() == XSD_STRING => {
            native_value(ctx, literal.value())
        }
        Term::Literal(literal) if literal.datatype_str() == XSD_BOOLEAN => {
            native_value(ctx, literal.value() == "true")
        }
        Term::Literal(literal) if literal.datatype_str() == XSD_INTEGER => native_value(
            ctx,
            literal
                .value()
                .parse::<i128>()
                .expect("admitted integer term"),
        ),
        _ => native_value(ctx, false),
    }
}

pub(super) fn node_kind_schema(kinds: &[NodeKindValue], ctx: &mut Ctx<'_>) -> Value {
    let mut alternatives = Vec::new();
    for kind in kinds {
        let ty = match kind {
            NodeKindValue::Literal => {
                let values = ["string", "integer", "boolean"]
                    .into_iter()
                    .map(|kind| native_value(ctx, kind))
                    .collect::<Vec<_>>();
                native_array(ctx, values)
            }
            _ => native_value(ctx, "object"),
        };
        let alternative = native_object(ctx, [("type", ty)]);
        let policy = ctx.tool.as_mut().expect("plain mode");
        if policy
            .native(|memory| memory.push(&mut alternatives, alternative))
            .is_none()
        {
            return Value::Null;
        }
    }
    let alternatives = native_array(ctx, alternatives);
    native_object(ctx, [("anyOf", alternatives)])
}

pub(super) fn class_schema(class: &str, ctx: &mut Ctx<'_>) -> Value {
    let target = ctx
        .tool
        .as_ref()
        .and_then(|p| p.classes.get(class))
        .cloned()
        .flatten();
    match target {
        Some(target) => ctx.catalog_ref(&target, None),
        None => {
            ctx.tool
                .as_mut()
                .expect("plain mode")
                .refuse(class, ToolRepresentationError::ClassTarget);
            json!({})
        }
    }
}

pub(super) fn member_schema(shape: &Shape, shape_iri: &str, key: &str, ctx: &mut Ctx<'_>) -> Value {
    if shape.deactivated {
        return json!({});
    }
    if !step(ctx) {
        return json!({});
    }
    if purrdf_stack::is_low() {
        ctx.tool
            .as_mut()
            .expect("plain mode")
            .error
            .get_or_insert(ToolSchemaError::StackMargin {
                location: shape_iri.to_owned(),
            });
        return json!({});
    }
    if !shape.constraint_annotations.is_empty() || !shape.deactivated_when.is_empty() {
        ctx.tool.as_mut().expect("plain mode").refuse(
            shape_iri,
            if shape.constraint_annotations.is_empty() {
                ToolRepresentationError::DynamicDeactivation
            } else {
                ToolRepresentationError::AnnotatedMetadata
            },
        );
        return json!({});
    }
    let model = ctx.model.clone();
    let resolved =
        match shape.resolved_in(model.as_deref()) {
            Ok(shape) => shape,
            Err(e) => {
                ctx.tool.as_mut().expect("plain mode").error.get_or_insert(
                    ToolSchemaError::Model {
                        location: shape_iri.to_owned(),
                        cause: e,
                    },
                );
                return json!({});
            }
        };
    if resolved.properties_in(model.as_deref()).next().is_some()
        || resolved
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::Closed { .. }))
    {
        return compile_object_schema(resolved, ctx);
    }
    let source = format_text(ctx, format_args!("{}", resolved.id));
    compile_member_schema_body(resolved, &source, key, ctx)
}

pub(super) fn step(ctx: &mut Ctx<'_>) -> bool {
    ctx.tool.as_mut().is_none_or(|p| p.charge(1))
}

pub(super) fn pattern_input(ctx: &mut Ctx<'_>, regex: &str, flags: &str) -> bool {
    ctx.tool.as_mut().is_none_or(|policy| {
        policy.text(regex)
            && policy.text(flags)
            && policy.admit_size(purrdf_core::xsd_regex::MAX_TRANSLATED_BYTES, 1)
    })
}

pub(super) fn exact_plain_constraint(
    constraint: &Constraint,
    siblings: &[Constraint],
    ctx: &mut Ctx<'_>,
    location: (&str, &str),
) -> Option<bool> {
    ctx.tool.as_ref()?;
    let mut domain = |accepted: fn(&str) -> bool| {
        for sibling in siblings {
            if !step(ctx) {
                return false;
            }
            if let Constraint::Datatype(types) = sibling {
                let mut all = !types.is_empty();
                for ty in types {
                    if !step(ctx) {
                        return false;
                    }
                    if !accepted(ty.as_str()) {
                        all = false;
                        break;
                    }
                }
                if all {
                    return true;
                }
            }
        }
        false
    };
    match constraint {
        Constraint::MinInclusive(_)
        | Constraint::MaxInclusive(_)
        | Constraint::MinExclusive(_)
        | Constraint::MaxExclusive(_) => Some(domain(|ty| ty == XSD_INTEGER)),
        Constraint::MinLength(_) | Constraint::MaxLength(_) => {
            Some(domain(|ty| matches!(ty, XSD_STRING | XSD_BOOLEAN)))
        }
        Constraint::SingleLine(_) | Constraint::UniqueLang(false) => Some(true),
        Constraint::Pattern { regex, flags, .. } => {
            if !domain(|ty| matches!(ty, XSD_STRING | XSD_BOOLEAN))
                || !pattern_input(ctx, regex, flags.as_deref().unwrap_or(""))
            {
                return Some(false);
            }
            Some(
                match purrdf_core::xsd_regex::to_ecma_262(regex, flags.as_deref().unwrap_or("")) {
                    Ok(_) => true,
                    Err(error) => {
                        pattern_resource_refusal(ctx, location.0, location.1, &error);
                        false
                    }
                },
            )
        }
        _ => None,
    }
}

pub(super) fn is_carrier_rejection(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|m| m.get("type").and_then(Value::as_str) == Some("object"))
}

// Charge the source walk independently from emitted values. References remain
// edges and are charged when their catalog definitions are reached.
fn admit_expansion(constraints: &[Constraint], policy: &mut ToolPolicy) -> bool {
    if !policy.admit_size(0, constraints.len()) {
        return false;
    }
    if !policy.charge(u64::try_from(constraints.len()).unwrap_or(u64::MAX)) {
        return false;
    }
    let mut pending = purrdf_lex::walk::WorkList::<&Constraint, 16>::new();
    for constraint in constraints {
        if policy
            .native(|memory| pending.try_push_admitted(constraint, memory))
            .is_none()
        {
            return false;
        }
    }
    while let Some(constraint) = pending.pop() {
        if !policy.charge(1) || !policy.admit_size(0, 1) {
            return false;
        }
        let nested: &[Shape] = match constraint {
            Constraint::And(shapes) | Constraint::Or(shapes) | Constraint::Xone(shapes) => shapes,
            Constraint::Node(shape) | Constraint::Not(shape) | Constraint::SomeValue(shape) => {
                std::slice::from_ref(shape.as_ref())
            }
            Constraint::Closed { ignored, .. } => {
                let nodes = ignored.len();
                if !policy.admit_size(0, nodes) {
                    return false;
                }
                for name in ignored {
                    if !policy.text(name.as_str()) {
                        return false;
                    }
                }
                &[]
            }
            Constraint::Reference(Term::NamedNode(name)) => {
                if !policy.text(name.as_str()) {
                    return false;
                }
                &[]
            }
            Constraint::Class(names) | Constraint::Datatype(names) => {
                let nodes = names.len();
                if !policy.admit_size(0, nodes) {
                    return false;
                }
                for name in names {
                    if !policy.text(name.as_str()) {
                        return false;
                    }
                }
                &[]
            }
            Constraint::NodeKind(kinds) => {
                let nodes = kinds.len();
                if !policy.admit_size(0, nodes) {
                    return false;
                }
                &[]
            }
            Constraint::Pattern { regex, flags, .. } => {
                // The shared translator exposes a hard one-MiB output ceiling.
                // Reserve that one bounded translator result before invoking it;
                // subsequent artifact copies use their actual native builders.
                if !policy.admit_size(purrdf_core::xsd_regex::MAX_TRANSLATED_BYTES, 0) {
                    return false;
                }
                if !policy.text(regex) || !policy.text(flags.as_deref().unwrap_or("")) {
                    return false;
                }
                &[]
            }
            Constraint::In(terms) => {
                let nodes = terms.len();
                if !policy.admit_size(0, nodes) {
                    return false;
                }
                for term in terms {
                    if let Term::Literal(literal) = term
                        && !policy.text(literal.value())
                    {
                        return false;
                    }
                }
                &[]
            }
            Constraint::HasValue(Term::Literal(literal))
            | Constraint::MinInclusive(Term::Literal(literal))
            | Constraint::MaxInclusive(Term::Literal(literal))
            | Constraint::MinExclusive(Term::Literal(literal))
            | Constraint::MaxExclusive(Term::Literal(literal)) => {
                if !policy.text(literal.value()) {
                    return false;
                }
                &[]
            }
            Constraint::Sparql { select, .. } => {
                if !policy.text(select) {
                    return false;
                }
                &[]
            }
            _ => &[],
        };
        for shape in nested {
            if !shape.constraint_annotations.is_empty() || !shape.deactivated_when.is_empty() {
                policy.refuse(
                    "nested argument shape",
                    if shape.constraint_annotations.is_empty() {
                        ToolRepresentationError::DynamicDeactivation
                    } else {
                        ToolRepresentationError::AnnotatedMetadata
                    },
                );
                return false;
            }
            if !policy.admit_size(0, shape.constraints.len()) {
                return false;
            }
            if !policy.charge(u64::try_from(shape.constraints.len()).unwrap_or(u64::MAX)) {
                return false;
            }
            for constraint in &shape.constraints {
                if policy
                    .native(|memory| pending.try_push_admitted(constraint, memory))
                    .is_none()
                {
                    return false;
                }
            }
            for property in &shape.property_shapes {
                if !policy.charge(1) {
                    return false;
                }
                if !policy.admit_size(0, property.constraints.len()) {
                    return false;
                }
                if !policy.charge(u64::try_from(property.constraints.len()).unwrap_or(u64::MAX)) {
                    return false;
                }
                for constraint in &property.constraints {
                    if policy
                        .native(|memory| pending.try_push_admitted(constraint, memory))
                        .is_none()
                    {
                        return false;
                    }
                }
            }
        }
    }
    policy
        .native(|memory| pending.release_admitted(memory))
        .is_some()
}

// Keep the original translator refusal; an unsupported exact translation still
// follows the shared compiler's source-specific loss path.
pub(super) fn pattern_resource_refusal(
    ctx: &mut Ctx<'_>,
    shape: &str,
    property: &str,
    error: &purrdf_core::xsd_regex::Ecma262Error,
) -> bool {
    let Some(policy) = &mut ctx.tool else {
        return false;
    };
    if matches!(error, purrdf_core::xsd_regex::Ecma262Error::Unsupported(_)) {
        return false;
    }
    policy
        .error
        .get_or_insert_with(|| ToolSchemaError::Pattern {
            shape: shape.to_owned(),
            property: property.to_owned(),
            predicate: policy.predicate.clone(),
            cause: error.clone(),
        });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_sort_stops_at_the_original_work_ceiling() {
        let namespaces =
            Namespaces::new("ex", &[("ex".into(), "https://example.org/".into())]).unwrap();
        let mut ctx = Ctx::new(
            BTreeSet::new(),
            BTreeMap::new(),
            BTreeMap::new(),
            &namespaces,
        );
        ctx.tool = Some(ToolPolicy {
            predicate: None,
            bindings: BTreeMap::new(),
            used: BTreeSet::new(),
            classes: BTreeMap::new(),
            limits: ToolSchemaLimits {
                work: 10,
                bytes: 1_000_000,
                nodes: 1_000,
            },
            work: 0,
            input_bytes: 0,
            admitted_nodes: 0,
            native_live: 0,
            native_nodes: 0,
            error: None,
        });
        // Six traversal units and one four-unit comparison fit; the next
        // comparison must return the original exhaustion before comparing.
        let mut values = vec![Value::from("z"), Value::from("y"), Value::from("x")];
        sort_values(&mut ctx, &mut values);
        let policy = ctx.tool.unwrap();
        assert_eq!(policy.work, 10);
        assert!(matches!(
            policy.error,
            Some(ToolSchemaError::Exhausted {
                resource: "work",
                used: 14,
                limit: 10
            })
        ));
    }
}
