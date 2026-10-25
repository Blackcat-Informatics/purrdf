// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RDF-authored, source-backed ordered plans and original membership provenance.
use super::{
    Account, Admission, CompiledGlobSet, LANGUAGE_VERSION, Mode, Resident, SetCompileError,
    SetMatchError, StorageError,
};
use purrdf_core::collections::{ListFault, RdfListWalk, SoleObject};
use purrdf_core::{
    DatasetHandle, DatasetView, NativeBuildError, RdfDataset, RdfDatasetBuilder, TermId, TermRef,
};
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};
use purrdf_lex::allocation::Memory;
use std::cell::Cell;

/// The portable RDF schema; it stores source, never executable bytecode.
pub const DESCRIPTION_VERSION: &str = "purrdf-glob-rdf-v1";
const SURFACES: [&str; 7] = [
    "literal",
    "iri",
    "iri-scheme",
    "iri-authority",
    "iri-path",
    "iri-query",
    "iri-fragment",
];
/// Every role is explicit. Values are absolute IRIs supplied by the caller.
pub const ROLES: [&str; 13] = [
    "description_type",
    "pattern_type",
    "version",
    "language",
    "mode",
    "surface",
    "patterns",
    "source",
    "evaluation_type",
    "plan",
    "input",
    "matched",
    "work",
];
const TYPE: &str = purrdf_iri::vocab::rdf::TYPE;
const FIRST: &str = purrdf_iri::vocab::rdf::FIRST;
const REST: &str = purrdf_iri::vocab::rdf::REST;
const NIL: &str = purrdf_iri::vocab::rdf::NIL;

/// Borrowed explicit vocabulary, with no namespace or identity fallback.
#[derive(Debug, Clone, Copy)]
pub struct Vocabulary<'a> {
    terms: [&'a str; 13],
}
/// An original invalid vocabulary argument, borrowed from its caller.
#[derive(Debug)]
pub struct VocabularyError<'a> {
    /// The refused role or IRI spelling.
    pub value: &'a str,
    /// Closed configuration reason.
    pub reason: &'static str,
}
impl std::fmt::Display for VocabularyError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "glob RDF vocabulary {}: {}", self.reason, self.value)
    }
}
impl std::error::Error for VocabularyError<'_> {}
impl StructuredError for VocabularyError<'_> {
    fn visit<'e>(&'e self, visitor: &mut dyn ErrorVisitor<'e>) {
        visitor.variant("glob-rdf-vocabulary");
        visitor.value(Some("reason"), ErrorValue::Code(self.reason));
        visitor.value(Some("value"), ErrorValue::Text(self.value));
    }
}
impl<'a> Vocabulary<'a> {
    /// Validate all roles, including duplicate names and colliding IRIs.
    pub fn new(entries: &[(&'a str, &'a str)]) -> Result<Self, VocabularyError<'a>> {
        let mut terms = [""; 13];
        for &(role, iri) in entries {
            let Some(index) = ROLES.iter().position(|name| *name == role) else {
                return Err(VocabularyError {
                    value: role,
                    reason: "unknown-role",
                });
            };
            if !terms[index].is_empty() {
                return Err(VocabularyError {
                    value: role,
                    reason: "duplicate-role",
                });
            }
            if purrdf_iri::absolute_verdict(iri) != Some(true) {
                return Err(VocabularyError {
                    value: iri,
                    reason: "invalid-absolute-iri",
                });
            }
            if terms.contains(&iri) || [TYPE, FIRST, REST, NIL].contains(&iri) {
                return Err(VocabularyError {
                    value: iri,
                    reason: "colliding-iri",
                });
            }
            terms[index] = iri;
        }
        for (index, value) in terms.iter().enumerate() {
            if value.is_empty() {
                return Err(VocabularyError {
                    value: ROLES[index],
                    reason: "missing-role",
                });
            }
        }
        Ok(Self { terms })
    }
}

/// Original refusal, retaining the exact frozen source rather than copied terms.
#[derive(Debug)]
pub struct Error {
    source: DatasetHandle,
    plan: TermId,
    input: Option<(DatasetHandle, TermId)>,
    surface: Option<usize>,
    reason: Reason,
}
#[derive(Debug)]
enum Reason {
    InvalidId {
        value: TermId,
        available: usize,
        input: bool,
    },
    EvaluationCollision(TermId),
    Schema {
        node: TermId,
        role: usize,
        code: &'static str,
    },
    Collection(ListFault<TermId>),
    Compile {
        cause: SetCompileError,
        node: TermId,
    },
    Match(SetMatchError),
    Storage(StorageError),
    Build(NativeBuildError),
    Input(TermId),
    Identity,
}
impl Error {
    /// The original plan snapshot. Local IDs never escape without this owner.
    pub fn source_dataset(&self) -> &RdfDataset {
        &self.source
    }
    /// Original input snapshot, when the refusal happened during execution.
    pub fn input_dataset(&self) -> Option<&RdfDataset> {
        self.input.as_ref().map(|(data, _)| &**data)
    }
    /// Original requested plan ID, interpreted only in the retained source.
    pub fn plan_id(&self) -> TermId {
        self.plan
    }
    /// Original input ID, interpreted only in the retained input snapshot.
    pub fn input_id(&self) -> Option<TermId> {
        self.input.as_ref().map(|(_, id)| *id)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.reason {
            Reason::InvalidId {
                value, available, ..
            } => write!(
                f,
                "glob RDF term ID {} is outside its snapshot of {available} terms",
                value.index()
            ),
            Reason::EvaluationCollision(_) => f.write_str(
                "glob RDF evaluation identity collides with a plan, pattern or prior evaluation",
            ),
            Reason::Schema { role, code, .. } => write!(f, "glob RDF {}: {code}", ROLES[*role]),
            Reason::Collection(fault) => write!(f, "glob RDF collection: {:?}", fault.kind),
            Reason::Compile { cause, .. } => cause.fmt(f),
            Reason::Match(cause) => cause.fmt(f),
            Reason::Storage(cause) => cause.fmt(f),
            Reason::Build(cause) => cause.fmt(f),
            Reason::Input(_) => {
                f.write_str("glob RDF input does not have the selected IRI/literal surface")
            }
            Reason::Identity => f.write_str("glob RDF evaluation identity must be an absolute IRI"),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.reason {
            Reason::Compile { cause, .. } => Some(cause),
            Reason::Match(cause) => Some(cause),
            Reason::Storage(cause) => Some(cause),
            Reason::Build(cause) => Some(cause),
            _ => None,
        }
    }
}
impl StructuredError for Error {
    fn visit<'e>(&'e self, visitor: &mut dyn ErrorVisitor<'e>) {
        visitor.variant("glob-rdf-refusal");
        visitor.value(
            Some("plan_id"),
            ErrorValue::Unsigned(self.plan.index() as u128),
        );
        if self.plan.index() < self.source.term_count() {
            self.source.inspect_term(self.plan, Some("plan"), visitor);
        }
        if let Some((input, id)) = &self.input {
            visitor.value(Some("input_id"), ErrorValue::Unsigned(id.index() as u128));
            if id.index() < input.term_count() {
                input.inspect_term(*id, Some("input"), visitor);
            }
        }
        if let Some(surface) = self.surface {
            visitor.value(Some("surface"), ErrorValue::Code(SURFACES[surface]));
        }
        match &self.reason {
            Reason::InvalidId {
                value,
                available,
                input,
            } => {
                visitor.value(Some("reason"), ErrorValue::Code("term-id-out-of-range"));
                visitor.value(Some("value"), ErrorValue::Unsigned(value.index() as u128));
                visitor.value(Some("available"), ErrorValue::Unsigned(*available as u128));
                visitor.value(Some("input_owner"), ErrorValue::Boolean(*input));
            }
            Reason::EvaluationCollision(node) => {
                visitor.value(
                    Some("reason"),
                    ErrorValue::Code("evaluation-identity-collision"),
                );
                self.source.inspect_term(*node, Some("node"), visitor);
            }
            Reason::Schema { node, role, code } => {
                visitor.value(Some("role"), ErrorValue::Code(ROLES[*role]));
                visitor.value(Some("reason"), ErrorValue::Code(code));
                self.source.inspect_term(*node, Some("node"), visitor);
            }
            Reason::Collection(fault) => {
                visitor.value(Some("cause"), ErrorValue::Record(&fault.kind));
                visitor.value(Some("members"), ErrorValue::Unsigned(fault.members as u128));
                self.source.inspect_term(fault.node, Some("node"), visitor);
            }
            Reason::Compile { cause, node } => {
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
                self.source.inspect_term(*node, Some("node"), visitor);
            }
            Reason::Match(cause) => visitor.value(Some("cause"), ErrorValue::Cause(cause)),
            Reason::Storage(cause) => visitor.value(Some("cause"), ErrorValue::Cause(cause)),
            Reason::Build(cause) => visitor.value(Some("cause"), ErrorValue::Cause(cause)),
            Reason::Input(node) => {
                if let Some((input, _)) = &self.input {
                    input.inspect_term(*node, Some("node"), visitor);
                }
            }
            Reason::Identity => visitor.value(
                Some("reason"),
                ErrorValue::Code("invalid-evaluation-identity"),
            ),
        }
    }
}

// Only subject/predicate SPOG probes, with graph filtering after the probe.
// No lazy permutation index is initialized by a schema read.
fn objects(
    data: &RdfDataset,
    subject: TermId,
    predicate: TermId,
) -> impl Iterator<Item = TermId> + '_ {
    DatasetView::quads_for_pattern(
        data,
        Some(subject),
        Some(predicate),
        None,
        purrdf_core::GraphMatch::Any,
    )
    .filter(|quad| quad.g.is_none())
    .map(|quad| quad.o)
}
fn sole(data: &RdfDataset, subject: TermId, predicate: Option<TermId>) -> SoleObject<TermId> {
    predicate.map_or(SoleObject::None, |p| {
        SoleObject::of(objects(data, subject, p))
    })
}
fn required(
    data: &RdfDataset,
    node: TermId,
    predicate: Option<TermId>,
    role: usize,
) -> Result<TermId, Reason> {
    match sole(data, node, predicate) {
        SoleObject::One(value) => Ok(value),
        _ => Err(Reason::Schema {
            node,
            role,
            code: "required-single-value",
        }),
    }
}
fn text(data: &RdfDataset, node: TermId, role: usize) -> Result<&str, Reason> {
    match data.resolve(node) {
        TermRef::Literal {
            lexical,
            datatype,
            language: None,
            direction: None,
        } if matches!(data.resolve(datatype), TermRef::Iri(iri) if iri == purrdf_core::datatype::XSD_STRING) => {
            Ok(lexical)
        }
        _ => Err(Reason::Schema {
            node,
            role,
            code: "expected-string",
        }),
    }
}
#[derive(Clone, Copy, Debug)]
struct Schema {
    root: TermId,
    head: TermId,
    first: Option<TermId>,
    rest: Option<TermId>,
    nil: Option<TermId>,
    source: Option<TermId>,
    count: usize,
}
fn walk(
    data: &RdfDataset,
    schema: Schema,
) -> impl Iterator<Item = Result<TermId, ListFault<TermId>>> + Clone + '_ {
    RdfListWalk::new(
        schema.head,
        schema.nil,
        move |cell| sole(data, cell, schema.first),
        move |cell| sole(data, cell, schema.rest),
    )
}
fn check_fields(data: &RdfDataset, node: TermId, allowed: &[Option<TermId>]) -> Result<(), Reason> {
    for quad in
        DatasetView::quads_for_pattern(data, Some(node), None, None, purrdf_core::GraphMatch::Any)
    {
        if quad.g.is_some() || !allowed.contains(&Some(quad.p)) {
            return Err(Reason::Schema {
                node: quad.p,
                role: 0,
                code: "unknown-or-named-field",
            });
        }
    }
    Ok(())
}
fn schema(
    data: &RdfDataset,
    root: TermId,
    vocab: Vocabulary<'_>,
) -> Result<(Schema, Mode, usize), Reason> {
    let ids = vocab.terms.map(|iri| data.term_id_by_iri(iri));
    let ty = data.term_id_by_iri(TYPE);
    check_fields(data, root, &[ty, ids[2], ids[3], ids[4], ids[5], ids[6]])?;
    if required(data, root, ty, 0)?
        != ids[0].ok_or(Reason::Schema {
            node: root,
            role: 0,
            code: "missing-description-type",
        })?
    {
        return Err(Reason::Schema {
            node: root,
            role: 0,
            code: "wrong-description-type",
        });
    }
    for (role, expected) in [(2, DESCRIPTION_VERSION), (3, LANGUAGE_VERSION)] {
        let value = required(data, root, ids[role], role)?;
        if text(data, value, role)? != expected {
            return Err(Reason::Schema {
                node: value,
                role,
                code: "incompatible-version",
            });
        }
    }
    let mode_node = required(data, root, ids[4], 4)?;
    let mode = match text(data, mode_node, 4)? {
        "text" => Mode::Text,
        "path" => Mode::Path,
        _ => {
            return Err(Reason::Schema {
                node: mode_node,
                role: 4,
                code: "unknown-mode",
            });
        }
    };
    let surface_node = required(data, root, ids[5], 5)?;
    let surface_text = text(data, surface_node, 5)?;
    let surface = SURFACES
        .iter()
        .position(|name| *name == surface_text)
        .ok_or(Reason::Schema {
            node: surface_node,
            role: 5,
            code: "unknown-surface",
        })?;
    let mut schema = Schema {
        root,
        head: required(data, root, ids[6], 6)?,
        first: data.term_id_by_iri(FIRST),
        rest: data.term_id_by_iri(REST),
        nil: data.term_id_by_iri(NIL),
        source: ids[7],
        count: 0,
    };
    for member in walk(data, schema) {
        let member = member.map_err(Reason::Collection)?;
        match data.resolve(member) {
            TermRef::Iri(_) | TermRef::Blank { .. } => (),
            _ => {
                return Err(Reason::Schema {
                    node: member,
                    role: 1,
                    code: "expected-pattern-identity",
                });
            }
        }
        check_fields(data, member, &[ty, ids[7]])?;
        if required(data, member, ty, 1)?
            != ids[1].ok_or(Reason::Schema {
                node: member,
                role: 1,
                code: "missing-pattern-type",
            })?
        {
            return Err(Reason::Schema {
                node: member,
                role: 1,
                code: "wrong-pattern-type",
            });
        }
        text(data, required(data, member, ids[7], 7)?, 7)?;
        schema.count = schema
            .count
            .checked_add(1)
            .ok_or(Reason::Storage(StorageError::SizeOverflow))?;
    }
    Ok((schema, mode, surface))
}

/// A validated RDF description and its real compiled ordered programs.
#[derive(Debug)]
pub struct PreparedPlan<'v, S: Admission = Resident> {
    source: DatasetHandle,
    schema: Schema,
    vocab: Vocabulary<'v>,
    surface: usize,
    compiled: CompiledGlobSet<S>,
}
impl<'v, S: Admission> PreparedPlan<'v, S> {
    /// Consume a frozen snapshot; failure retains it with original local context.
    #[expect(
        clippy::result_large_err,
        reason = "Refusal retains original RDF snapshots inline without allocating on the failure path"
    )]
    pub fn compile(
        source: DatasetHandle,
        plan: TermId,
        vocab: &Vocabulary<'v>,
        storage: S,
    ) -> Result<Self, Error> {
        let result = (|| {
            if plan.index() >= source.term_count() {
                return Err(Reason::InvalidId {
                    value: plan,
                    available: source.term_count(),
                    input: false,
                });
            }
            let (schema, mode, surface) = schema(&source, plan, *vocab)?;
            let data = &*source;
            let mut members = walk(data, schema);
            let sources = (0..schema.count).map(move |_| {
                let member = members
                    .next()
                    .expect("validated collection length")
                    .expect("immutable validated collection");
                let source_id =
                    required(data, member, schema.source, 7).expect("validated source field");
                text(data, source_id, 7).expect("validated source string")
            });
            let compiled =
                super::compile::compile_set_iter(sources, mode, storage).map_err(|cause| {
                    let node = match cause {
                        SetCompileError::Pattern { index, .. } => walk(data, schema)
                            .nth(index)
                            .expect("validated index")
                            .expect("validated list"),
                        SetCompileError::Storage(_) => schema.root,
                    };
                    Reason::Compile { cause, node }
                })?;
            Ok((schema, surface, compiled))
        })();
        match result {
            Ok((schema, surface, compiled)) => Ok(Self {
                source,
                schema,
                vocab: *vocab,
                surface,
                compiled,
            }),
            Err(reason) => Err(Error {
                source,
                plan,
                input: None,
                surface: None,
                reason,
            }),
        }
    }
    /// Original caller-provided plan identity, in its retained snapshot.
    pub fn identity(&self) -> TermId {
        self.schema.root
    }
    /// Original source snapshot, borrowed without releasing its admission.
    pub fn dataset(&self) -> &RdfDataset {
        &self.source
    }
    /// Execute against an original RDF term. Result storage is admitted before
    /// matching; no input or result is formatted to cross this boundary.
    #[expect(
        clippy::result_large_err,
        reason = "Refusal retains original RDF snapshots inline without allocating on the failure path"
    )]
    pub fn execute<W: Admission>(
        self,
        input: DatasetHandle,
        input_term: TermId,
        max_work: u64,
        storage: W,
    ) -> Result<Execution<'v, S, W>, Error> {
        let mut account = Account(storage);
        let mut indices = Vec::new();
        let result = (|| {
            if input_term.index() >= input.term_count() {
                return Err(Reason::InvalidId {
                    value: input_term,
                    available: input.term_count(),
                    input: true,
                });
            }
            Memory::new(&mut account.0)
                .reserve(&mut indices, self.schema.count)
                .map_err(Reason::Storage)?;
            indices.resize(self.schema.count, 0);
            let surface = match (self.surface, input.resolve(input_term)) {
                (0, TermRef::Literal { lexical, .. }) => Some(lexical),
                (selection, TermRef::Iri(iri)) if selection != 0 => {
                    let parts = purrdf_iri::components(iri).ok_or(Reason::Input(input_term))?;
                    match selection {
                        1 => Some(iri),
                        2 => parts.scheme,
                        3 => parts.authority,
                        4 => Some(parts.path),
                        5 => parts.query,
                        6 => parts.fragment,
                        _ => unreachable!("validated surface"),
                    }
                }
                _ => return Err(Reason::Input(input_term)),
            };
            let report = self
                .compiled
                .matches(surface, max_work, &mut indices)
                .map_err(Reason::Match)?;
            Ok((report.indices.len(), report.work))
        })();
        match result {
            Ok((len, work)) => {
                indices.truncate(len);
                Ok(Execution {
                    plan: self,
                    input,
                    input_term,
                    indices,
                    work,
                    _account: account,
                })
            }
            Err(reason) => Err(Error {
                source: self.source,
                plan: self.schema.root,
                input: Some((input, input_term)),
                surface: Some(self.surface),
                reason,
            }),
        }
    }
}

/// A complete native result with all original RDF identity owners.
#[derive(Debug)]
pub struct Execution<'v, S: Admission = Resident, W: Admission = Resident> {
    plan: PreparedPlan<'v, S>,
    input: DatasetHandle,
    input_term: TermId,
    indices: Vec<usize>,
    work: u64,
    _account: Account<W>,
}

/// An export refusal retains its original output admission until the native
/// cause and all of its diagnostic payload have been destroyed.
pub struct ExportError<O: Admission> {
    error: Error,
    _account: Option<Account<O>>,
}
impl<O: Admission> std::fmt::Debug for ExportError<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExportError")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}
impl<O: Admission> std::fmt::Display for ExportError<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}
impl<O: Admission + 'static> std::error::Error for ExportError<O> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
impl<O: Admission> StructuredError for ExportError<O> {
    fn visit<'e>(&'e self, visitor: &mut dyn ErrorVisitor<'e>) {
        self.error.visit(visitor);
    }
}
impl<O: Admission> ExportError<O> {
    /// Borrow the original refusal and retained RDF identities.
    pub fn error(&self) -> &Error {
        &self.error
    }
}
impl<'v, S: Admission, W: Admission> Execution<'v, S, W> {
    /// Every original source occurrence that matched, including duplicates.
    pub fn indices(&self) -> &[usize] {
        &self.indices
    }
    /// Aggregate deterministic matching work.
    pub fn work(&self) -> u64 {
        self.work
    }
    /// Recover the same compiled plan for another execution.
    pub fn into_plan(self) -> PreparedPlan<'v, S> {
        self.plan
    }
    /// Stage a self-contained RDF description and RDF 1.2 membership provenance.
    /// The caller supplies the evaluation IRI and an original output account.
    #[expect(
        clippy::result_large_err,
        reason = "Refusal retains original RDF snapshots inline without allocating on the failure path"
    )]
    pub fn export<O: Admission + Send + Sync + 'static>(
        self,
        evaluation_iri: &str,
        storage: O,
    ) -> Result<DatasetHandle, ExportError<O>> {
        let collision = self
            .plan
            .source
            .term_id_by_iri(evaluation_iri)
            .filter(|id| {
                *id == self.plan.schema.root
                    || walk(&self.plan.source, self.plan.schema).any(|member| member == Ok(*id))
                    || self.plan.source.term_id_by_iri(TYPE).is_some_and(|ty| {
                        objects(&self.plan.source, *id, ty).any(|class| {
                            [0, 1, 8].into_iter().any(|role| {
                                Some(class)
                                    == self.plan.source.term_id_by_iri(self.plan.vocab.terms[role])
                            })
                        })
                    })
            });
        let result = if purrdf_iri::absolute_verdict(evaluation_iri) != Some(true) {
            Err((Reason::Identity, Some(Account(storage))))
        } else if let Some(node) = collision {
            Err((Reason::EvaluationCollision(node), Some(Account(storage))))
        } else {
            self.export_native(evaluation_iri, storage)
        };
        result.map_err(|(reason, account)| ExportError {
            error: Error {
                source: self.plan.source,
                plan: self.plan.schema.root,
                input: Some((self.input, self.input_term)),
                surface: Some(self.plan.surface),
                reason,
            },
            _account: account,
        })
    }
    fn export_native<O: Admission + Send + Sync + 'static>(
        &self,
        evaluation: &str,
        storage: O,
    ) -> Result<DatasetHandle, (Reason, Option<Account<O>>)> {
        let mut account = Account(storage);
        let result: Result<RdfDataset, Reason> = (|| {
            let mut builder = RdfDatasetBuilder::new();
            let mut memory = Memory::new(&mut account.0);
            let map = builder
                .push_dataset_with_mapping_with_memory(&self.plan.source, &mut memory)
                .map_err(Reason::Storage)?;
            let result: Result<(), StorageError> = (|| {
                let evaluation = builder.intern_iri_with_memory(evaluation, &mut memory)?;
                let ty = builder.intern_iri_with_memory(TYPE, &mut memory)?;
                let terms = self.plan.vocab.terms;
                let class = builder.intern_iri_with_memory(terms[8], &mut memory)?;
                builder.push_quad_with_memory(evaluation, ty, class, None, &mut memory)?;
                let plan_predicate = builder.intern_iri_with_memory(terms[9], &mut memory)?;
                builder.push_quad_with_memory(
                    evaluation,
                    plan_predicate,
                    map[self.plan.schema.root.index()].expect("description root imported"),
                    None,
                    &mut memory,
                )?;
                let input_predicate = builder.intern_iri_with_memory(terms[10], &mut memory)?;
                let input = builder.import_native_term_with_memory(
                    &self.input,
                    self.input_term,
                    &mut memory,
                )?;
                builder.push_quad_with_memory(
                    evaluation,
                    input_predicate,
                    input,
                    None,
                    &mut memory,
                )?;
                let matched = builder.intern_iri_with_memory(terms[11], &mut memory)?;
                let first = builder.intern_iri_with_memory(FIRST, &mut memory)?;
                let current = Cell::new(self.plan.schema.head);
                let schema = self.plan.schema;
                let list = RdfListWalk::new(
                    schema.head,
                    schema.nil,
                    |cell| {
                        current.set(cell);
                        sole(&self.plan.source, cell, schema.first)
                    },
                    |cell| sole(&self.plan.source, cell, schema.rest),
                );
                let mut wanted = self.indices.iter().copied().peekable();
                for (index, member) in list.enumerate() {
                    let member = member.expect("validated immutable membership");
                    if wanted.peek() == Some(&index) {
                        let cell = map[current.get().index()].expect("membership cell imported");
                        let pattern = map[member.index()].expect("pattern imported");
                        let quoted =
                            builder.intern_triple_with_memory(cell, first, pattern, &mut memory)?;
                        builder.push_quad_with_memory(
                            evaluation,
                            matched,
                            quoted,
                            None,
                            &mut memory,
                        )?;
                        wanted.next();
                    }
                }
                let work_predicate = builder.intern_iri_with_memory(terms[12], &mut memory)?;
                let number = memory.format(&format_args!("{}", self.work))?;
                let work = builder.intern_literal_parts_with_memory(
                    &number,
                    Some(purrdf_core::datatype::XSD_INTEGER),
                    None,
                    None,
                    &mut memory,
                )?;
                memory.release_string(number)?;
                builder.push_quad_with_memory(
                    evaluation,
                    work_predicate,
                    work,
                    None,
                    &mut memory,
                )?;
                Ok(())
            })();
            memory
                .release_vec_after(map, result)
                .map_err(Reason::Storage)?;
            let dataset = builder
                .freeze_with_memory(&mut memory)
                .map_err(Reason::Build)?;
            memory
                .add_bytes(DatasetHandle::allocation_layout::<Account<O>>().size())
                .map_err(Reason::Storage)?;
            Ok(dataset)
        })();
        match result {
            Ok(dataset) => DatasetHandle::try_from_admitted(dataset, account)
                .map_err(|_| (Reason::Storage(StorageError::AllocationFailed), None)),
            Err(reason) => Err((reason, Some(account))),
        }
    }
}
