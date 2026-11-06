// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original native refusals of reasoning requests and their input documents.

use core::fmt;
use purrdf_core::{RdfDiagnostic, TermValue};
use purrdf_entail::{EntailError, OwnedDirectnessRefusal, Regime, Service};
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError, visit_list};

use super::{MODULE_METHOD_NAMES, PROGRAM_REGIME_NAMES, PROOF_SERVICE_NAMES, REGIME_NAMES};

/// The named part of a reasoning request that has a fixed accepted vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RequestName {
    /// An entailment regime.
    Regime,
    /// A proof-bearing service.
    ProofService,
    /// A locality extraction method.
    ModuleMethod,
}

impl RequestName {
    /// All accepted spellings, in the same order as the human diagnostic.
    #[must_use]
    pub const fn accepted(self) -> &'static [&'static str] {
        match self {
            Self::Regime => &REGIME_NAMES,
            Self::ProofService => &PROOF_SERVICE_NAMES,
            Self::ModuleMethod => &MODULE_METHOD_NAMES,
        }
    }

    const fn description(self) -> &'static str {
        match self {
            Self::Regime => "entailment regime",
            Self::ProofService => "proof service",
            Self::ModuleMethod => "locality-module method",
        }
    }
}

impl StructuredError for RequestName {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        visitor.variant(match self {
            Self::Regime => "reasoning-name-regime",
            Self::ProofService => "reasoning-name-proof-service",
            Self::ModuleMethod => "reasoning-name-module-method",
        });
        visit_list(
            visitor,
            Some("accepted"),
            self.accepted().iter().map(|value| ErrorValue::Code(value)),
        );
    }
}

/// A caller request that does not name an operation this boundary can perform.
#[derive(Debug)]
#[non_exhaustive]
pub enum RequestError {
    /// A native service added upstream has no operation at this boundary.
    UnsupportedService {
        /// Original native service.
        service: Service,
        /// The operation this boundary cannot provide.
        operation: &'static str,
    },
    /// A spelling outside the accepted vocabulary.
    UnknownName {
        /// The requested vocabulary.
        kind: RequestName,
        /// Original caller spelling.
        offered: String,
    },
    /// A regime whose fixed rule table takes no program received one.
    UnexpectedProgram {
        /// Resolved native regime.
        regime: Regime,
        /// Original caller regime spelling.
        spelling: String,
        /// Original supplied program.
        program: String,
    },
    /// A rule document declares an import this in-memory boundary cannot fetch.
    RuleImport {
        /// Original caller regime spelling.
        spelling: String,
        /// Original supplied program.
        program: String,
        /// Resolved native import location.
        location: String,
        /// Resolved native entailment profile, if supplied.
        profile: Option<String>,
    },
    /// A service that takes no argument received a nonempty one.
    UnexpectedArgument {
        /// Original service spelling.
        service: String,
        /// Original supplied argument.
        argument: String,
    },
    /// The module request lacks its required method header.
    ModuleArgument {
        /// Original supplied argument.
        argument: String,
    },
    /// Proof production was requested from a session that records nothing.
    UnrecordedSession {
        /// Resolved requested service.
        service: Service,
        /// Original caller spelling.
        spelling: String,
    },
    /// A variable marker in the supplied pattern has no following name.
    VariableMarker {
        /// Original pattern.
        pattern: String,
        /// Actual marker, either `?` or `$`.
        marker: char,
    },
    /// A projected variable was supplied in a literal's datatype position.
    VariableDatatype {
        /// Original variable name.
        variable: String,
        /// Literal lexical form.
        lexical_form: String,
        /// Actual datatype stand-in IRI.
        datatype: String,
    },
    /// The retained explanation state cannot be accessed after a panic.
    ExplanationLock,
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedService { service, operation } => write!(
                f,
                "service \"{}\" has no {operation} at this boundary",
                service.as_str()
            ),
            Self::UnknownName { kind, offered } => write!(
                f,
                "unknown {} \"{offered}\"; accepted: {}",
                kind.description(),
                kind.accepted().join(", ")
            ),
            Self::UnexpectedProgram { spelling, .. } => write!(
                f,
                "entailment regime \"{spelling}\" takes no rule document, and one was supplied; its rule table is the specification's. Regimes that take one: {}",
                PROGRAM_REGIME_NAMES.join(", ")
            ),
            Self::RuleImport {
                spelling, location, ..
            } => write!(
                f,
                "entailment regime \"{spelling}\": the rule document imports \"{location}\", and resolving an import is I/O this boundary does not perform; resolve it in the caller and use purrdf_entail::materialize"
            ),
            Self::UnexpectedArgument { service, argument } => write!(
                f,
                "service \"{service}\" takes no argument, and \"{argument}\" was supplied; the argument would have been discarded rather than narrowing the question"
            ),
            Self::ModuleArgument { .. } => write!(
                f,
                "an `extract-module` argument opens with a `method <{}>` line",
                MODULE_METHOD_NAMES.join("|")
            ),
            Self::UnrecordedSession { spelling, .. } => write!(
                f,
                "this session records nothing, so \"{spelling}\" has no proof to hand back. Recording is opt-in and costs the traces it keeps: open the session with proofs to ask for one"
            ),
            Self::VariableMarker { marker, .. } => write!(
                f,
                "a variable marker `{marker}` with no name after it; a projected variable is written `{marker}name`"
            ),
            Self::VariableDatatype {
                variable,
                lexical_form,
                ..
            } => write!(
                f,
                "a variable is not a datatype IRI: `?{variable}` stands in the datatype of the literal \"{lexical_form}\", and a basic graph pattern admits a variable only where a term can be bound"
            ),
            Self::ExplanationLock => {
                f.write_str("explain-conclusion: active explanation session lock is poisoned")
            }
        }
    }
}

impl std::error::Error for RequestError {}

impl StructuredError for RequestError {
    purrdf_hash::error_visit! {
        UnsupportedService { service, operation } => "reasoning-unsupported-service" {
            "service" => Code(service.as_str()), "operation" => Code(operation),
        },
        UnknownName { kind, offered } => "reasoning-unknown-name" { "kind" => Record(kind), "offered" => Text(offered) },
        UnexpectedProgram { regime, spelling, program } => "reasoning-unexpected-program" { "regime" => Code(regime.token()), "spelling" => Text(spelling), "program" => Text(program) },
        RuleImport { spelling, program, location, profile } => "reasoning-rule-import" { "spelling" => Text(spelling), "program" => Text(program), "location" => Iri(location), "profile" => Value(profile.as_deref().map_or(ErrorValue::Absent, ErrorValue::Iri)) },
        UnexpectedArgument { service, argument } => "reasoning-unexpected-argument" { "service" => Text(service), "argument" => Text(argument) },
        ModuleArgument { argument } => "reasoning-module-argument" { "argument" => Text(argument) },
        UnrecordedSession { service, spelling } => "reasoning-unrecorded-session" { "service" => Code(service.as_str()), "spelling" => Text(spelling) },
        VariableMarker { pattern, marker } => "reasoning-variable-marker" { "pattern" => Text(pattern), "marker" => Character(*marker) },
        VariableDatatype { variable, lexical_form, datatype } => "reasoning-variable-datatype" { "variable" => Text(variable), "lexical_form" => Text(lexical_form), "datatype" => Iri(datatype) },
        ExplanationLock => "reasoning-explanation-lock" {},
    }
}

/// The input role retained beside an original native RDF diagnostic.
#[derive(Debug)]
#[non_exhaustive]
pub enum InputRole {
    /// The premise document.
    Premise,
    /// A conclusion document.
    Conclusion,
    /// An import document under its caller-supplied key.
    Import(String),
    /// One term read through the existing subject-position scaffold.
    Term,
    /// One N-Quads statement.
    Statement,
    /// A rewritten basic graph pattern.
    Pattern,
    /// The supplied module answer.
    ModuleAnswer,
}

impl StructuredError for InputRole {
    purrdf_hash::error_visit! {
        Premise => "reasoning-input-premise" {},
        Conclusion => "reasoning-input-conclusion" {},
        Import(iri) => "reasoning-input-import" { "iri" => Iri(iri) },
        Term => "reasoning-input-term" {},
        Statement => "reasoning-input-statement" {},
        Pattern => "reasoning-input-pattern" {},
        ModuleAnswer => "reasoning-input-module-answer" {},
    }
}

/// An original parsed input and the exact native reason it cannot serve its role.
#[derive(Debug)]
#[non_exhaustive]
pub enum InputError {
    /// The native RDF parser refused the bytes actually offered to it.
    Parse {
        /// The document or argument's role.
        role: InputRole,
        /// Original caller text.
        offered: String,
        /// Scaffold or rewritten text, if parsing transformed the caller text.
        parsed: Option<String>,
        /// Complete native parser diagnostic.
        cause: RdfDiagnostic,
    },
    /// A term or statement input produced no statement.
    Empty {
        /// The argument's role.
        role: InputRole,
        /// Original caller text.
        offered: String,
        /// Exact scaffold, if one was used.
        parsed: Option<String>,
    },
    /// A term or statement input could not denote exactly one item.
    Multiple {
        /// The argument's role.
        role: InputRole,
        /// Original caller text.
        offered: String,
        /// Exact scaffold, if one was used.
        parsed: Option<String>,
        /// Actual parsed statement count.
        count: usize,
        /// Scaffold integrity when the existing parser path inspected it.
        scaffold_intact: Option<bool>,
    },
    /// A graph name occurred where only a default-graph statement is meaningful.
    Graph {
        /// The argument's role.
        role: InputRole,
        /// Original caller text.
        offered: String,
        /// Original parsed graph term.
        graph: TermValue,
    },
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { role, offered, cause, .. } => match role {
                InputRole::Premise => fmt::Display::fmt(cause, f),
                InputRole::Conclusion => write!(f, "the conclusion is not N-Quads: {cause}"),
                InputRole::Import(iri) => write!(f, "the import document for <{iri}> is not N-Quads: {cause}"),
                InputRole::Term => write!(f, "\"{offered}\" is not one N-Triples term: {cause}"),
                InputRole::Statement => write!(f, "\"{offered}\" is not one N-Quads statement: {cause}"),
                InputRole::Pattern => write!(f, "the basic graph pattern is not N-Triples: {cause}"),
                InputRole::ModuleAnswer => write!(f, "the module answer is not N-Quads: {cause}"),
            },
            Self::Empty { role, offered, .. } => match role {
                InputRole::Term => write!(f, "\"{offered}\" is not one N-Triples term: it is empty"),
                _ => write!(f, "\"{offered}\" is not one N-Quads statement: it is empty"),
            },
            Self::Multiple { role, offered, .. } => match role {
                InputRole::Term => write!(f, "\"{offered}\" is not ONE N-Triples term: it parses as more than one"),
                _ => write!(f, "\"{offered}\" is not ONE N-Quads statement: it parses as more than one"),
            },
            Self::Graph { role: InputRole::Pattern, .. } => f.write_str(
                "a basic graph pattern is matched against the DEFAULT graph, so a pattern that names a graph is refused rather than having that graph silently dropped"),
            Self::Graph { offered, .. } => write!(f,
                "\"{offered}\" names a graph; an axiom is one triple and is not graph-scoped"),
        }
    }
}

impl std::error::Error for InputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse { cause, .. } => Some(cause),
            Self::Empty { .. } | Self::Multiple { .. } | Self::Graph { .. } => None,
        }
    }
}

impl StructuredError for InputError {
    purrdf_hash::error_visit! {
        Parse { role, offered, parsed, cause } => "reasoning-input-parse" {
            "role" => Record(role), "offered" => Text(offered),
            "parsed" => Value(parsed.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
            "cause" => Cause(cause),
        },
        Empty { role, offered, parsed } => "reasoning-input-empty" {
            "role" => Record(role), "offered" => Text(offered),
            "parsed" => Value(parsed.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
        },
        Multiple { role, offered, parsed, count, scaffold_intact } => "reasoning-input-multiple" {
            "role" => Record(role), "offered" => Text(offered),
            "parsed" => Value(parsed.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
            "count" => Unsigned(*count as u128),
            "scaffold_intact" => Value(scaffold_intact.map_or(ErrorValue::Absent, ErrorValue::Boolean)),
        },
        Graph { role, offered, graph } => "reasoning-input-graph" {
            "role" => Record(role), "offered" => Text(offered), "graph" => Term(graph),
        },
    }
}

/// The actual operation whose native entailment failure is being presented.
#[derive(Debug)]
#[non_exhaustive]
pub enum EntailContext {
    /// An unprefixed original native refusal.
    Native,
    /// An operation that prefixes the native error without reclassifying it.
    Prefix(&'static str),
    /// An operation that prefixes its native refusal with its name.
    Operation(String),
    /// A service with actionable advice for an unsatisfiable ontology.
    Service(String),
    /// A rule document supplied to a named regime.
    RuleDocument {
        /// Original caller regime spelling.
        spelling: String,
        /// Original supplied program.
        program: String,
    },
    /// A rule-driven service whose refusal names the actual host's limit controls.
    Regime {
        /// Original caller regime spelling.
        spelling: String,
        /// Host whose controls were supplied.
        host: super::RegimeHost,
        /// Actual requested service.
        service: super::RegimeService,
    },
    /// Import resolution reports its native refusal without an additional prefix.
    Imports,
}

impl StructuredError for super::RegimeHost {
    purrdf_hash::error_visit! {
        Rust => "reasoning-host-rust" {},
        Python => "reasoning-host-python" {},
        PythonText => "reasoning-host-python-text" {},
        Wasm => "reasoning-host-wasm" {},
        CAbi => "reasoning-host-c-abi" {},
        Cli => "reasoning-host-cli" {},
    }
}

impl StructuredError for super::RegimeService {
    purrdf_hash::error_visit! {
        Materialize => "reasoning-service-materialize" {},
        CertainAnswers => "reasoning-service-certain-answers" {},
        GraphEntails => "reasoning-service-graph-entails" {},
        VerifyEntailment => "reasoning-service-verify-entailment" {},
        Query => "reasoning-service-query" {},
    }
}

impl StructuredError for EntailContext {
    purrdf_hash::error_visit! {
        Native => "reasoning-context-native" {},
        Prefix(operation) => "reasoning-context-prefix" { "operation" => Text(operation) },
        Operation(operation) => "reasoning-context-operation" { "operation" => Text(operation) },
        Service(service) => "reasoning-context-service" { "service" => Text(service) },
        RuleDocument { spelling, program } => "reasoning-context-rule-document" { "spelling" => Text(spelling), "program" => Text(program) },
        Regime { spelling, host, service } => "reasoning-context-regime" { "spelling" => Text(spelling), "host" => Record(host), "service" => Record(service) },
        Imports => "reasoning-context-imports" {},
    }
}

/// A reasoning request failure retaining its original native cause and operands.
#[derive(Debug)]
#[non_exhaustive]
pub enum ReasoningError {
    /// Original shared import-array boundary failure.
    ImportArrays(purrdf_core::imports::ImportArrayError),
    /// Original certificate text and typed field-decoding refusal.
    Certificate {
        /// Complete original supplied certificate.
        offered: String,
        /// Original grammar or numeric refusal.
        cause: Box<super::CertificateError>,
    },
    /// Original fixture grammar, output comparison or case execution failure.
    Fixture(Box<super::FixtureError>),
    /// Original proof envelope refusal and document operands.
    ProofDocument(super::ProofDocumentError),
    /// Original native proof decoding or verification refusal.
    Proof {
        /// Actual operation presenting the refusal.
        operation: Option<&'static str>,
        /// Original native cause.
        cause: Box<purrdf_entail::DlProofError>,
    },
    /// Original explanation or justification refusal.
    Explanation {
        /// Actual operation presenting the refusal.
        operation: &'static str,
        /// Original native cause.
        cause: Box<purrdf_entail::ExplainError>,
    },
    /// Invalid request shape or spelling.
    Request(RequestError),
    /// Original RDF input failure, with its offered and parsed text.
    Input(Box<InputError>),
    /// Original import-key refusal.
    ImportKey(purrdf_core::imports::ImportKeyError),
    /// Original premise-IRI refusal.
    PremiseIri(super::PremiseIriError),
    /// Original native entailment failure with its actual operation context.
    Entailment {
        /// Operation that was refused.
        context: EntailContext,
        /// Complete original native error.
        cause: Box<EntailError>,
    },
    /// Original canonical-only operation failure.
    Canonical(crate::canonical::CanonicalServiceError),
    /// Exact direct answers are unavailable under the original native certificate.
    Directness {
        /// Actual service requesting direct answers.
        service: Service,
        /// Original certificate, moved from the refused certified answer.
        cause: Box<OwnedDirectnessRefusal>,
    },
}

impl fmt::Display for ReasoningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ImportArrays(cause) => fmt::Display::fmt(cause, f),
            Self::Certificate { cause, .. } => fmt::Display::fmt(cause, f),
            Self::Fixture(error) => fmt::Display::fmt(error, f),
            Self::ProofDocument(error) => fmt::Display::fmt(error, f),
            Self::Proof {
                operation: Some(operation),
                cause,
            } => write!(f, "{operation}: {cause}"),
            Self::Proof {
                operation: None,
                cause,
            } => fmt::Display::fmt(cause, f),
            Self::Explanation { operation, cause } => write!(f, "{operation}: {cause}"),
            Self::Request(error) => fmt::Display::fmt(error, f),
            Self::Input(error) => fmt::Display::fmt(error, f),
            Self::ImportKey(error) => write!(f, "the import list is unusable: {error}"),
            Self::PremiseIri(error) => fmt::Display::fmt(error, f),
            Self::Canonical(error) => fmt::Display::fmt(error, f),
            Self::Entailment { context, cause } => match context {
                EntailContext::Native => fmt::Display::fmt(cause, f),
                EntailContext::Prefix(operation) => write!(f, "{operation}: {cause}"),
                EntailContext::Operation(operation) => match &**cause {
                    EntailError::Canonicalization(error) => {
                        write!(f, "{operation} was refused canonicalization: {error}")
                    }
                    error => write!(f, "{operation}: {error}"),
                },
                EntailContext::Service(service) => {
                    f.write_str(&super::service_error(service, cause))
                }
                EntailContext::RuleDocument { spelling, .. } => write!(
                    f,
                    "entailment regime \"{spelling}\": rule document: {cause}"
                ),
                EntailContext::Regime {
                    spelling,
                    host,
                    service,
                } => f.write_str(&super::render_entail_error_in(
                    spelling, cause, *host, *service,
                )),
                EntailContext::Imports => fmt::Display::fmt(cause, f),
            },
            Self::Directness { service, cause } => write!(
                f,
                "{cause}\n{}",
                super::render_dl_certificate(
                    super::proof_service_name(*service),
                    cause.certificate()
                )
            ),
        }
    }
}

impl std::error::Error for ReasoningError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::ImportArrays(cause) => cause,
            Self::Certificate { cause, .. } => cause.as_ref(),
            Self::Fixture(cause) => cause.as_ref(),
            Self::ProofDocument(cause) => cause,
            Self::Proof { cause, .. } => cause.as_ref(),
            Self::Explanation { cause, .. } => cause.as_ref(),
            Self::Request(cause) => cause,
            Self::Input(cause) => cause.as_ref(),
            Self::ImportKey(cause) => cause,
            Self::PremiseIri(cause) => cause,
            Self::Entailment { cause, .. } => cause.as_ref(),
            Self::Canonical(cause) => cause,
            Self::Directness { cause, .. } => cause.as_ref(),
        })
    }
}

impl StructuredError for ReasoningError {
    purrdf_hash::error_visit! {
        ImportArrays(cause) => "reasoning-import-arrays" { "cause" => Cause(cause) },
        Certificate { offered, cause } => "reasoning-certificate" { "offered" => Text(offered), "cause" => Cause(cause.as_ref()) },
        Fixture(cause) => "reasoning-fixture" { "cause" => Cause(cause.as_ref()) },
        ProofDocument(cause) => "reasoning-proof-document" { "cause" => Cause(cause) },
        Proof { operation, cause } => "reasoning-proof" {
            "operation" => Value(operation.map_or(ErrorValue::Absent, ErrorValue::Text)), "cause" => Cause(cause.as_ref()),
        },
        Explanation { operation, cause } => "reasoning-explanation" { "operation" => Code(operation), "cause" => Cause(cause.as_ref()) },
        Request(cause) => "reasoning-request" { "cause" => Cause(cause) },
        Input(cause) => "reasoning-input" { "cause" => Cause(cause.as_ref()) },
        ImportKey(cause) => "reasoning-import-key" { "cause" => Cause(cause) },
        PremiseIri(cause) => "reasoning-premise-iri" { "cause" => Cause(cause) },
        Entailment { context, cause } => "reasoning-entailment" { "context" => Record(context), "cause" => Cause(cause.as_ref()) },
        Canonical(cause) => "reasoning-canonical" { "cause" => Cause(cause) },
        Directness { service, cause } => "reasoning-directness" { "service" => Code(service.as_str()), "cause" => Cause(cause.as_ref()) },
    }
}

purrdf_lex::variant_from!(ReasoningError { Fixture(super::FixtureError) via Box::new });
purrdf_lex::variant_from!(ReasoningError { Input(InputError) via Box::new });

purrdf_lex::variant_from!(ReasoningError {
    ImportArrays(purrdf_core::imports::ImportArrayError),
    ProofDocument(super::ProofDocumentError),
    Request(RequestError),
    ImportKey(purrdf_core::imports::ImportKeyError),
    PremiseIri(super::PremiseIriError),
    Canonical(crate::canonical::CanonicalServiceError),
});

impl From<super::FixtureGrammarError> for ReasoningError {
    fn from(cause: super::FixtureGrammarError) -> Self {
        super::FixtureError::Grammar(cause).into()
    }
}

impl ReasoningError {
    pub(super) fn proof(operation: &'static str, cause: purrdf_entail::DlProofError) -> Self {
        Self::Proof {
            operation: Some(operation),
            cause: Box::new(cause),
        }
    }

    pub(super) fn explanation(operation: &'static str, cause: purrdf_entail::ExplainError) -> Self {
        Self::Explanation {
            operation,
            cause: Box::new(cause),
        }
    }

    /// Retain an original native entailment failure under its actual request context.
    pub fn entail(context: EntailContext, cause: EntailError) -> Self {
        Self::Entailment {
            context,
            cause: Box::new(cause),
        }
    }

    /// Retain this original native refusal through the shared diagnostic owner.
    #[must_use]
    pub fn into_diagnostic(self) -> RdfDiagnostic {
        crate::native_error::diagnostic("reasoning-refused", self, None)
    }
}
