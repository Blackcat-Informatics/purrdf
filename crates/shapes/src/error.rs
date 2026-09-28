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
    /// A SHACL-SPARQL or SHACL-AF declaration of the shapes graph, or a node's
    /// `sh:message` values (`message-datatype`), violates a syntax rule, whether or not
    /// any shape reaches it. See [`IllFormedShapesGraph`].
    IllFormed(IllFormedShapesGraph),
    /// A query the engine executes with pre-bound variables violates a pre-binding
    /// restriction. See [`PrebindingViolation`].
    Prebinding(PrebindingViolation),
    /// A shape declares a SHACL-AF custom target (`sh:target`) this engine cannot
    /// compute. See [`UnsupportedTargetRefusal`].
    UnsupportedTarget(UnsupportedTargetRefusal),
    /// A `sh:SPARQLTarget`'s `sh:ask` and its `sh:select` answer two ways for one node a
    /// candidate check asked about. See [`SparqlTargetDisagreement`].
    SparqlTargetDisagreement(SparqlTargetDisagreement),
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
            Self::ShaclJs(_)
            | Self::IllFormed(_)
            | Self::Prebinding(_)
            | Self::UnsupportedTarget(_)
            | Self::SparqlTargetDisagreement(_)
            | Self::Invalid(_) => None,
        }
    }

    /// The SHACL-JS refusal, when this is one.
    #[must_use]
    pub const fn as_shacl_js(&self) -> Option<&ShaclJsRefusal> {
        match self {
            Self::ShaclJs(refusal) => Some(refusal),
            Self::Imports(_)
            | Self::IllFormed(_)
            | Self::Prebinding(_)
            | Self::UnsupportedTarget(_)
            | Self::SparqlTargetDisagreement(_)
            | Self::Invalid(_) => None,
        }
    }

    /// The syntax-rule refusal, when this is one.
    #[must_use]
    pub const fn as_ill_formed(&self) -> Option<&IllFormedShapesGraph> {
        match self {
            Self::IllFormed(refusal) => Some(refusal),
            Self::Imports(_)
            | Self::ShaclJs(_)
            | Self::Prebinding(_)
            | Self::UnsupportedTarget(_)
            | Self::SparqlTargetDisagreement(_)
            | Self::Invalid(_) => None,
        }
    }

    /// The unsupported-custom-target refusal, when this is one.
    #[must_use]
    pub const fn as_unsupported_target(&self) -> Option<&UnsupportedTargetRefusal> {
        match self {
            Self::UnsupportedTarget(refusal) => Some(refusal),
            Self::Imports(_)
            | Self::ShaclJs(_)
            | Self::IllFormed(_)
            | Self::Prebinding(_)
            | Self::SparqlTargetDisagreement(_)
            | Self::Invalid(_) => None,
        }
    }

    /// The `sh:ask` / `sh:select` disagreement refusal, when this is one.
    #[must_use]
    pub const fn as_sparql_target_disagreement(&self) -> Option<&SparqlTargetDisagreement> {
        match self {
            Self::SparqlTargetDisagreement(refusal) => Some(refusal),
            Self::Imports(_)
            | Self::ShaclJs(_)
            | Self::IllFormed(_)
            | Self::Prebinding(_)
            | Self::UnsupportedTarget(_)
            | Self::Invalid(_) => None,
        }
    }

    /// The pre-binding refusal, when this is one.
    #[must_use]
    pub const fn as_prebinding(&self) -> Option<&PrebindingViolation> {
        match self {
            Self::Prebinding(violation) => Some(violation),
            Self::Imports(_)
            | Self::ShaclJs(_)
            | Self::IllFormed(_)
            | Self::UnsupportedTarget(_)
            | Self::SparqlTargetDisagreement(_)
            | Self::Invalid(_) => None,
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
            Self::UnsupportedTarget(refusal) => refusal.fmt(f),
            Self::SparqlTargetDisagreement(refusal) => refusal.fmt(f),
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
            Self::UnsupportedTarget(refusal) => Some(refusal),
            Self::SparqlTargetDisagreement(refusal) => Some(refusal),
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
/// Only a construct a shape REACHES is refused here: SHACL-JS on the shape itself
/// (`sh:js`, a SHACL-JS type), a `sh:JSTarget` or a target whose type is a
/// `sh:JSTargetType`, a `sh:JSRule` among its rules, and a call to a `sh:JSFunction` from
/// a node expression or from reachable SPARQL. A library that only DECLARES such
/// constructs — `sh:JSLibrary`s, `sh:JSFunction`s nothing calls — loads, and those
/// declarations are inert. A `sh:JSValidator` attached to a component is different: as a
/// value of `sh:validator`, `sh:nodeValidator` or `sh:propertyValidator` it violates the
/// attachment's class rule (SHACL 1.2 SPARQL Extensions: "The values of sh:validator must
/// be ASK-based validators", likewise SELECT-based for the scoped attachments), so it is
/// refused as [`ShapesError::IllFormed`] whether or not a shape reaches it.
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

/// A shape refused because it declares a custom target this engine cannot compute.
///
/// SHACL Advanced Features, "Custom Targets": "The behavior of a SHACL engine that is
/// unable to handle a given custom target is left undefined. [...] Engines that are aware
/// of this property and cannot handle a given custom target SHOULD at least report a
/// warning." PurRDF computes a `sh:SPARQLTarget` and an instance of a declared
/// `sh:SPARQLTargetType`; any other value of `sh:target` — a node of another target type,
/// or of none — would validate the shape against focus nodes it never computed. A warning
/// beside a report would still be a report about fewer focus nodes than the shape names,
/// so PurRDF refuses the shapes graph instead, which is stronger than the warning the
/// specification asks for. (A SHACL-JS target is refused as [`ShapesError::ShaclJs`].)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedTargetRefusal {
    shape: String,
    target: String,
    message: String,
}

impl UnsupportedTargetRefusal {
    pub(crate) const fn new(shape: String, target: String, message: String) -> Self {
        Self {
            shape,
            target,
            message,
        }
    }

    /// The shape that declares the target, as the engine renders a term.
    #[must_use]
    pub fn shape(&self) -> &str {
        &self.shape
    }

    /// The `sh:target` value, as the engine renders a term.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    /// The engine's full diagnostic.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for UnsupportedTargetRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for UnsupportedTargetRefusal {}

/// A `sh:SPARQLTarget` whose `sh:ask` and `sh:select` disagree about a node.
///
/// SHACL Advanced Features §3.1 gives a SPARQL-based target two queries. The SELECT
/// defines the target: its `this` bindings are the target nodes. The optional ASK answers
/// the same question for one node: "A SHACL engine can then determine whether a given
/// shape applies to a given node by executing the ASK query with the variable this
/// pre-bound to the node. If the ASK query evaluates to true then the node is in the target
/// of the shape." Both are the author's statement of ONE target, so a candidate check — which
/// decides a given node by the ASK rather than enumerating the SELECT — confirms the ASK's
/// answer against the SELECT's result set for the same data graph. When they disagree the
/// target has no single meaning, and a verdict computed from either answer would differ
/// from a whole-graph validation of the same node; the check is refused instead, naming
/// the shape, the target (by its two queries), the node and both answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparqlTargetDisagreement {
    shape: String,
    ask: String,
    select: String,
    node: String,
    asked: bool,
}

impl SparqlTargetDisagreement {
    pub(crate) const fn new(
        shape: String,
        ask: String,
        select: String,
        node: String,
        asked: bool,
    ) -> Self {
        Self {
            shape,
            ask,
            select,
            node,
            asked,
        }
    }

    /// The shape that declares the target, as the engine renders a term.
    #[must_use]
    pub fn shape(&self) -> &str {
        &self.shape
    }

    /// The target's `sh:ask` query text — with the PREFIX header the target's
    /// `sh:prefixes` supply — which identifies the target among the shape's `sh:target`s.
    #[must_use]
    pub fn ask(&self) -> &str {
        &self.ask
    }

    /// The target's `sh:select` query text, with the same PREFIX header.
    #[must_use]
    pub fn select(&self) -> &str {
        &self.select
    }

    /// The node the two queries disagree about, as the engine renders a term.
    #[must_use]
    pub fn node(&self) -> &str {
        &self.node
    }

    /// The ASK's answer with `$this` pre-bound to [`Self::node`]; the SELECT's is the
    /// opposite.
    #[must_use]
    pub const fn asked(&self) -> bool {
        self.asked
    }
}

impl fmt::Display for SparqlTargetDisagreement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "sparql-target-disagreement: a sh:SPARQLTarget of shape {shape} answers two ways \
             for {node}: its sh:ask, with $this pre-bound to that node, is {asked}, and its \
             sh:select {selects} return it. SHACL Advanced Features section 3.1 defines the \
             target by the SELECT and lets the ASK decide a given node (\"If the ASK query \
             evaluates to true then the node is in the target of the shape\"), so the two \
             state one target and must agree; with them disagreeing, checking the node would \
             not give the verdict a whole-graph validation gives it. Make the sh:ask true \
             exactly for the nodes the sh:select returns.\n  sh:ask: {ask}\n  sh:select: \
             {select}",
            shape = self.shape,
            node = self.node,
            asked = self.asked,
            selects = if self.asked { "does not" } else { "does" },
            ask = self.ask.replace('\n', " "),
            select = self.select.replace('\n', " "),
        )
    }
}

impl std::error::Error for SparqlTargetDisagreement {}

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

/// A shapes graph refused because SHACL-SPARQL or SHACL-AF declarations in it — or the
/// `sh:message` values of a node (SHACL 1.2 Core `message-datatype`) — violate syntax
/// rules, with every violation the graph contains, in a deterministic order.
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
