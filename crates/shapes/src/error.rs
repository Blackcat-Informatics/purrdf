// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The error every shapes-graph entry point returns.
//!
//! Most of what can go wrong while turning a shapes document into a verdict is a message for
//! a human — a syntax error, an unsupported construct, a SPARQL body that does not parse —
//! and is carried as one ([`ShapesError::Invalid`]). An incomplete `owl:imports` closure is
//! different: it is the one refusal a caller can ACT on programmatically, by supplying the
//! documents it names, and it must read the same on every host. So it is its own variant,
//! carrying the typed [`ShapesImportError`], and a caller branches on the variant rather
//! than on message text. A shapes graph that uses the SHACL JavaScript Extensions is the
//! other: SHACL-JS is a 2017 Working Group Note, not SHACL 1.2, and this engine has no
//! JavaScript engine, so a shape that reaches SHACL-JS is refused typed
//! ([`ShapesError::ShaclJs`]), naming the extension, the node and the term, rather than
//! reading like a malformed graph. SHACL-JS that no shape reaches is declared vocabulary
//! and loads.
//!
//! Two refusals of SHACL-SPARQL and SHACL-AF declarations are typed as well, because they
//! answer to two different sentences of the specifications and a caller must be able to
//! tell them apart:
//!
//! * [`ShapesError::IllFormed`] — a declaration violates a SYNTAX RULE. SHACL 1.2 Core,
//!   "Handling of Ill-formed Shapes Graphs": "If the shapes graph contains ill-formed
//!   nodes, then the result of the validation process is undefined. A SHACL processor
//!   SHOULD produce a failure in this case." The sentence has no reachability qualifier,
//!   so the load fails whether or not any shape reaches the declaration, and the
//!   refusal lists EVERY such violation in the graph, each naming its declaration and
//!   syntax rule ([`IllFormedShapesGraph`]).
//! * [`ShapesError::Prebinding`] — a query that is EXECUTED with pre-bound variables
//!   violates a pre-binding restriction. SHACL 1.2 SPARQL Extensions, Appendix A:
//!   "SHACL-SPARQL processors MUST report a failure when it is operating on a shapes
//!   graph that contains SHACL-SPARQL queries (via sh:ask, sh:construct and sh:select)
//!   that are executed with pre-bound variables and violate any of these MUST
//!   restrictions." The failure is scoped to queries that execute, so it is raised where
//!   the engine determines that a query executes ([`PrebindingViolation`]); the same
//!   query nowhere executed is reported by `lint` instead (its `unexecuted` section).

use std::fmt;

use crate::imports::ShapesImportError;

/// Why a shapes-graph entry point refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapesError {
    /// The shapes graph's `owl:imports` closure is not in hand, or the import table the
    /// caller supplied cannot be used. See [`crate::imports`].
    Imports(ShapesImportError),
    /// A shape of the shapes graph reaches a construct of the SHACL JavaScript
    /// Extensions (SHACL-JS). See [`ShaclJsRefusal`].
    ShaclJs(ShaclJsRefusal),
    /// A SHACL-SPARQL or SHACL-AF declaration of the shapes graph violates a syntax
    /// rule, whether or not any shape reaches it. See [`IllFormedShapesGraph`].
    IllFormed(IllFormedShapesGraph),
    /// A query the engine executes with pre-bound variables violates a pre-binding
    /// restriction. See [`PrebindingViolation`].
    Prebinding(PrebindingViolation),
    /// Anything else: a document that does not parse, an unsupported or malformed SHACL
    /// construct, a failure during evaluation. The engine's own diagnostic.
    Invalid(String),
}

impl ShapesError {
    /// The import refusal, when this is one.
    #[must_use]
    pub const fn as_imports(&self) -> Option<&ShapesImportError> {
        match self {
            Self::Imports(error) => Some(error),
            Self::ShaclJs(_) | Self::IllFormed(_) | Self::Prebinding(_) | Self::Invalid(_) => None,
        }
    }

    /// The SHACL-JS refusal, when this is one.
    #[must_use]
    pub const fn as_shacl_js(&self) -> Option<&ShaclJsRefusal> {
        match self {
            Self::ShaclJs(refusal) => Some(refusal),
            Self::Imports(_) | Self::IllFormed(_) | Self::Prebinding(_) | Self::Invalid(_) => None,
        }
    }

    /// The syntax-rule refusal, when this is one.
    #[must_use]
    pub const fn as_ill_formed(&self) -> Option<&IllFormedShapesGraph> {
        match self {
            Self::IllFormed(refusal) => Some(refusal),
            Self::Imports(_) | Self::ShaclJs(_) | Self::Prebinding(_) | Self::Invalid(_) => None,
        }
    }

    /// The pre-binding refusal, when this is one.
    #[must_use]
    pub const fn as_prebinding(&self) -> Option<&PrebindingViolation> {
        match self {
            Self::Prebinding(violation) => Some(violation),
            Self::Imports(_) | Self::ShaclJs(_) | Self::IllFormed(_) | Self::Invalid(_) => None,
        }
    }
}

impl fmt::Display for ShapesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Imports(error) => error.fmt(f),
            Self::ShaclJs(refusal) => refusal.fmt(f),
            Self::IllFormed(refusal) => refusal.fmt(f),
            Self::Prebinding(violation) => violation.fmt(f),
            Self::Invalid(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ShapesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Imports(error) => Some(error),
            Self::ShaclJs(refusal) => Some(refusal),
            Self::IllFormed(refusal) => Some(refusal),
            Self::Prebinding(violation) => Some(violation),
            Self::Invalid(_) => None,
        }
    }
}

/// A shapes graph refused because it uses the SHACL JavaScript Extensions.
///
/// SHACL-JS (`sh:JSConstraint`, `sh:js`, `sh:JSValidator`, `sh:JSRule`, …) is the 2017
/// Working Group Note "SHACL JavaScript Extensions", not part of SHACL 1.2, and this
/// engine has no JavaScript engine to evaluate it. A constraint it cannot evaluate is
/// refused rather than validated as if it were absent. The equivalent SHACL-SPARQL
/// constraint (`sh:sparql` with a `sh:select` query) loads.
///
/// Only a construct a shape REACHES is refused: SHACL-JS on the shape itself (`sh:js`, a
/// SHACL-JS type), a `sh:JSTarget` or a target whose type is a `sh:JSTargetType`, a
/// `sh:JSRule` among its rules, a use of a constraint component whose selected
/// validator is a `sh:JSValidator` with no SPARQL validator beside it, and a call to a
/// `sh:JSFunction` from a node expression or from reachable SPARQL. A library that only
/// DECLARES SHACL-JS — `sh:JSLibrary`s, `sh:JSFunction`s, `sh:JSValidator`s of
/// components no shape uses, SHACL-JS alternatives on built-in components — loads, and
/// those declarations are inert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaclJsRefusal {
    node: String,
    term: String,
    message: String,
}

impl ShaclJsRefusal {
    pub(crate) const fn new(node: String, term: String, message: String) -> Self {
        Self {
            node,
            term,
            message,
        }
    }

    /// The node that uses the term, as the engine renders a term.
    #[must_use]
    pub fn node(&self) -> &str {
        &self.node
    }

    /// The SHACL-JS term's IRI.
    #[must_use]
    pub fn term(&self) -> &str {
        &self.term
    }

    /// The engine's full diagnostic, which names the SHACL JavaScript Extensions.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ShaclJsRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ShaclJsRefusal {}

/// A syntax-rule violation as a declaration's parser reports it, before the declaration
/// is named: the id of the rule, when a numbered rule states it, and the diagnostic.
pub(crate) type RuleViolation = (Option<&'static str>, String);

/// One violation of a syntax rule by one SHACL-SPARQL or SHACL-AF declaration.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IllFormedDeclaration {
    declaration: String,
    rule: Option<&'static str>,
    message: String,
}

impl IllFormedDeclaration {
    pub(crate) fn new(
        declaration: impl Into<String>,
        rule: Option<&'static str>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            declaration: declaration.into(),
            rule,
            message: message.into(),
        }
    }

    /// The declaration, as the engine renders it: e.g. `validator _:b0 of the
    /// constraint component <http://example.org/ns#C>, via
    /// <http://www.w3.org/ns/shacl#nodeValidator>`, or `the sh:SPARQLFunction
    /// <http://example.org/ns#f>`.
    #[must_use]
    pub fn declaration(&self) -> &str {
        &self.declaration
    }

    /// The id of the syntax rule the declaration violates, as the specification's
    /// "Summary of Syntax Rules" spells it — e.g. `nodeValidator-class` (SHACL 1.2
    /// SPARQL Extensions), `parameter-name-not-in`, `SPARQLFunction-query` (SHACL
    /// Advanced Features). `None` for a refusal no numbered rule states: a term the
    /// engine would otherwise silently ignore, or a declaration that would be
    /// evaluated ambiguously.
    #[must_use]
    pub const fn rule(&self) -> Option<&'static str> {
        self.rule
    }

    /// The engine's diagnostic for this one violation.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for IllFormedDeclaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.declaration, self.message)?;
        if let Some(rule) = self.rule {
            write!(f, " [syntax rule {rule}]")?;
        }
        Ok(())
    }
}

/// A shapes graph refused because SHACL-SPARQL or SHACL-AF declarations in it violate
/// syntax rules, with every violation the graph contains, in a deterministic order.
///
/// SHACL 1.2 Core, "Handling of Ill-formed Shapes Graphs": "If the shapes graph contains
/// ill-formed nodes, then the result of the validation process is undefined. A SHACL
/// processor SHOULD produce a failure in this case." PurRDF treats that SHOULD as a MUST,
/// and the sentence has no reachability qualifier: a declaration no shape reaches is
/// refused exactly like one a shape uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IllFormedShapesGraph {
    violations: Vec<IllFormedDeclaration>,
}

impl IllFormedShapesGraph {
    /// `violations`, sorted and de-duplicated; `None` when there are none.
    pub(crate) fn from_violations(mut violations: Vec<IllFormedDeclaration>) -> Option<Self> {
        violations.sort();
        violations.dedup();
        (!violations.is_empty()).then_some(Self { violations })
    }

    /// Every violation, sorted by declaration, rule and message.
    #[must_use]
    pub fn violations(&self) -> &[IllFormedDeclaration] {
        &self.violations
    }
}

impl fmt::Display for IllFormedShapesGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the shapes graph is ill-formed: {} declaration{} violate{} a syntax rule, and SHACL \
             1.2 Core says \"A SHACL processor SHOULD produce a failure in this case\"",
            self.violations.len(),
            if self.violations.len() == 1 { "" } else { "s" },
            if self.violations.len() == 1 { "s" } else { "" },
        )?;
        for violation in &self.violations {
            write!(f, "\n  {violation}")?;
        }
        Ok(())
    }
}

impl std::error::Error for IllFormedShapesGraph {}

/// A query executed with pre-bound variables that violates a pre-binding restriction of
/// SHACL 1.2 SPARQL Extensions, Appendix A — a `MINUS`, a `VALUES`, an `AS ?var` for a
/// potentially pre-bound variable, among the restrictions PurRDF enforces.
///
/// As [`ShapesError::Prebinding`], the refusal of a query the engine executes: the
/// selected validator of a custom constraint component a shape uses, or a
/// `sh:SPARQLFunction` a node expression or reachable SPARQL text calls. The same value
/// describes a violating query nothing executes in the `unexecuted` section of
/// [`crate::lint`], where it is a finding rather than a refusal.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrebindingViolation {
    declaration: String,
    message: String,
}

impl PrebindingViolation {
    pub(crate) fn new(declaration: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            declaration: declaration.into(),
            message: message.into(),
        }
    }

    /// The declaration whose query violates the restriction, as the engine renders it.
    #[must_use]
    pub fn declaration(&self) -> &str {
        &self.declaration
    }

    /// The restriction violated, naming the offending construct.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for PrebindingViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: its query violates a pre-binding restriction [syntax rule \
             pre-binding-limitations]: {}",
            self.declaration, self.message
        )
    }
}

impl std::error::Error for PrebindingViolation {}

impl From<ShapesImportError> for ShapesError {
    fn from(error: ShapesImportError) -> Self {
        Self::Imports(error)
    }
}

impl From<String> for ShapesError {
    fn from(message: String) -> Self {
        Self::Invalid(message)
    }
}

impl From<&str> for ShapesError {
    fn from(message: &str) -> Self {
        Self::Invalid(message.to_owned())
    }
}

/// The rendered message, for a caller whose own error type is a string.
impl From<ShapesError> for String {
    fn from(error: ShapesError) -> Self {
        error.to_string()
    }
}
