// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SPARQL 1.2 RL documents: parse, check, resolve imports, stratify, infer.
//!
//! The four preparation stages of SPARQL 1.2 RL are four calls, each failing with its own
//! [`SrlError`] variant, so a caller — and the W3C harness — can tell a document the
//! grammar refuses from a rule set that is not well formed, from one that cannot be
//! stratified:
//!
//! 1. [`parse`] — §7 "SPARQL-RL Grammar": [`SrlError::Syntax`].
//! 2. [`RuleSetDocument::resolve_imports`] — §4.5 "Processing Imports":
//!    [`SrlError::Import`].
//! 3. [`RuleSetDocument::check_well_formed`] — §4.2 "Well-formedness Conditions":
//!    [`SrlError::WellFormedness`].
//! 4. [`RuleSetDocument::stratify`] — §4.4 "Stratification": [`SrlError::Stratification`].
//!
//! [`parse_and_check`] runs 1, 3 and 4 (a document with imports is not stratified until
//! they are resolved); [`check`] runs all four from an import table, well-formedness again
//! over the imported rules, and evaluates nothing — the check-only entry point every host
//! exposes; and [`infer`] — "Infer is the operation that applies a rule set to
//! a given base graph and produces an inference graph containing inferred triples" —
//! runs 3 and 4 before §6 "Rule Set Evaluation".
//!
//! # Imports
//!
//! §4.5: "Reading documents from the web has security implications. Support for
//! importing rule sets is optional for SRL processors. Further, implementations MAY
//! provide partial support, such as supporting imports of certain rules sets and not
//! others, and possibly from a verified copy." PurRDF performs no I/O of its own (the
//! library is the same on every target, `wasm32` included), so a document's imports are
//! resolved through a caller-supplied [`ImportResolver`] — which may fetch, read a
//! verified copy, or decline an import it does not support — following the §A "Example
//! Imports Algorithm": each import URL is read once ("Processors MUST only import a rule
//! set once to avoid infinite loops when processing IMPORTS statements"), recursively,
//! and merged ("MR.rules = RS1.rules ∪ RS2.rules; MR.data = rdf_merge(RS1.data,
//! RS2.data)"). The §4.5 error conditions are each an [`SrlError::Import`]: "An
//! implementation that provides partial support MUST signal an error if it encounters an
//! import that it does not support" (the resolver declines), "An implementation MUST
//! signal an error if it can not resolve the import document, or if the document is not
//! syntactically valid". A document whose imports are not resolved is refused by
//! [`RuleSetDocument::stratify`] and [`infer`] with the same error, since its rule set is
//! not the one it declares.

use std::fmt;
use std::sync::Arc;

use ::purrdf_rdf::RdfDataset;
use purrdf_datalog::seminaive::EvalError;
use purrdf_iri::LineIndex;

use super::depend;
use super::eval::{self, Inference};
use super::ir::{DeclaredSchedule, ElementRule, IrRule, IrRuleBody, RuleSet, Scheduling};
use super::syntax;
use crate::data::ShaclData;
use crate::rules::RuleOptions;
use crate::shapes::Shapes;
use crate::term::{NamedNode, Term, Triple};

/// Why a SPARQL 1.2 RL document was refused, by the stage that refused it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SrlError {
    /// The document is not a SPARQL-RL Document: §7 "A conforming SRL document is an RDF
    /// string that conforms to the grammar starting with the RuleSet production".
    Syntax {
        /// The document the error is in: `None` for the document parsed, the import IRI
        /// for an imported one.
        document: Option<String>,
        /// 1-based line.
        line: u64,
        /// 1-based column.
        column: u32,
        /// What the grammar refused.
        message: String,
    },
    /// An import could not be resolved (§4.5).
    Import {
        /// The import IRI.
        iri: String,
        /// Why.
        message: String,
    },
    /// An import table ([`RuleSetDocument::resolve_import_table`]) supplies rule sets the
    /// import closure never names: each would be read and never used.
    UnreachedImports {
        /// The unreached table keys, in table order.
        iris: Vec<String>,
    },
    /// A rule is not well formed (§4.2).
    WellFormedness {
        /// The rule, as [`SrlRule::describe`] names it.
        rule: String,
        /// The violated condition.
        message: String,
    },
    /// The rule set violates the stratification condition (§4.4.1): "there is no
    /// recursive dependency involving a closed dependency in the dependency graph".
    Stratification {
        /// The rule whose closed dependency lies in a cycle.
        rule: String,
        /// The rule it depends on through that closed edge.
        depends_on: String,
        /// The cycle, `rule -> depends_on -> … -> rule`, each rule named.
        cycle: Vec<String>,
    },
    /// The rule set passed a rule-evaluation limit ([`InferOptions`]): the
    /// term-generating round limit, the generated-term budget, the stored-fact limit or
    /// the join-step limit. Names the limit, the numbers, the knob that raises it and, for
    /// the two term limits, the rules that inferred a new term in the last round.
    LimitExceeded(crate::rules::RuleLimitExceeded),
    /// Evaluation failed: a guard error, the fixed term-arena ceiling passed, or an
    /// inferred triple that is not an RDF triple.
    Evaluation {
        /// Why.
        message: String,
    },
}

impl fmt::Display for SrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax {
                document,
                line,
                column,
                message,
            } => match document {
                None => write!(
                    f,
                    "SPARQL 1.2 RL syntax error at line {line} column {column}: {message}"
                ),
                Some(iri) => write!(
                    f,
                    "SPARQL 1.2 RL syntax error in <{iri}> at line {line} column {column}: {message}"
                ),
            },
            Self::Import { iri, message } => {
                write!(f, "SPARQL 1.2 RL import <{iri}> failed: {message}")
            }
            Self::UnreachedImports { iris } => write!(
                f,
                "the SPARQL 1.2 RL rule set's import closure never reaches {}, so the import \
                 table's rule {} would be read and never used; remove {}",
                iris.iter()
                    .map(|iri| format!("<{iri}>"))
                    .collect::<Vec<_>>()
                    .join(", "),
                if iris.len() == 1 { "set" } else { "sets" },
                if iris.len() == 1 { "it" } else { "them" },
            ),
            Self::WellFormedness { rule, message } => {
                write!(f, "SPARQL 1.2 RL {rule} is not well formed: {message}")
            }
            Self::Stratification {
                rule,
                depends_on,
                cycle,
            } => write!(
                f,
                "the SPARQL 1.2 RL rule set is not stratifiable: {rule} depends on {depends_on} \
                 through a negation or as a run-once rule, inside the cycle {} -> {rule} \
                 (SPARQL 1.2 RL §4.4.1: \"there is no recursive dependency involving a closed \
                 dependency in the dependency graph\")",
                cycle.join(" -> ")
            ),
            Self::LimitExceeded(limit) => {
                write!(f, "SPARQL 1.2 RL evaluation failed: {limit}")
            }
            Self::Evaluation { message } => write!(f, "SPARQL 1.2 RL evaluation failed: {message}"),
        }
    }
}

impl std::error::Error for SrlError {}

/// One rule of a document.
#[derive(Debug, Clone)]
pub struct SrlRule {
    /// `'RULE' iri?`.
    iri: Option<NamedNode>,
    /// The rule.
    rule: ElementRule,
    /// The document it was written in: `None` for the parsed document, else the import.
    document: Option<String>,
    /// 1-based line of its `RULE` keyword.
    line: u64,
    /// 1-based column of its `RULE` keyword.
    column: u32,
}

impl SrlRule {
    /// The rule's IRI, when written: "A rule can be given a URI to help identify it."
    #[must_use]
    pub fn iri(&self) -> Option<&NamedNode> {
        self.iri.as_ref()
    }

    /// The rule, in the rule-set IR.
    #[must_use]
    pub fn rule(&self) -> &ElementRule {
        &self.rule
    }

    /// The rule named for a person: its IRI, else where it is written.
    #[must_use]
    pub fn describe(&self) -> String {
        let place = match &self.document {
            None => format!("line {} column {}", self.line, self.column),
            Some(iri) => format!("line {} column {} of <{iri}>", self.line, self.column),
        };
        match &self.iri {
            Some(iri) => format!("rule <{}> ({place})", iri.as_str()),
            None => format!("the rule at {place}"),
        }
    }
}

/// A parsed SPARQL 1.2 RL rule set.
#[derive(Debug, Clone)]
pub struct RuleSetDocument {
    /// The rules, in document order (imported rules after the importer's).
    rules: Vec<SrlRule>,
    /// "ruleset.data — The RDF graph formed by union of the data blocks in the rule set."
    data: Vec<[Term; 3]>,
    /// "ruleset.imports — The set of imports of a rule set", not yet resolved.
    imports: Vec<NamedNode>,
    /// The `VERSION` labels, in document order.
    versions: Vec<String>,
    /// The base IRI the document was parsed under: its location, for §A's `V = {
    /// location of RS }`.
    location: Option<String>,
}

/// Reads an imported rule set: given an import IRI, the document's text, or why it
/// cannot be read or is not supported.
pub trait ImportResolver {
    /// The SPARQL-RL document at `iri`.
    ///
    /// # Errors
    ///
    /// The import is not supported, or its document cannot be read.
    fn resolve(&mut self, iri: &str) -> Result<String, String>;
}

impl<F: FnMut(&str) -> Result<String, String>> ImportResolver for F {
    fn resolve(&mut self, iri: &str) -> Result<String, String> {
        self(iri)
    }
}

/// One stratum of a stratification: "A stratification layer SL, is a pair of disjoint
/// sets of rules (SL.once, SL.general)".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stratum {
    /// The run-once rules, by index into [`RuleSetDocument::rules`], in document order.
    pub once: Vec<usize>,
    /// The general rules, by index, in document order.
    pub general: Vec<usize>,
}

/// Parse a SPARQL 1.2 RL document — the grammar only; see [`parse_and_check`].
///
/// `base` is the document's base IRI from outside it (SPARQL 1.2 RL §7.4: "Base URI from
/// the Encapsulating Entity", "Base URI from the Retrieval URI"); `None` leaves only its
/// own `BASE` directives, so a relative IRI before any is an error.
///
/// # Errors
///
/// [`SrlError::Syntax`].
pub fn parse(text: &str, base: Option<&str>) -> Result<RuleSetDocument, SrlError> {
    parse_document(text, base, None)
}

/// Parse a SPARQL 1.2 RL document, check its rules are well formed, and — when it has no
/// imports — that it can be stratified.
///
/// # Errors
///
/// [`SrlError::Syntax`], [`SrlError::WellFormedness`], [`SrlError::Stratification`].
pub fn parse_and_check(text: &str, base: Option<&str>) -> Result<RuleSetDocument, SrlError> {
    let document = parse(text, base)?;
    document.check_well_formed()?;
    if document.imports.is_empty() {
        document.stratify()?;
    }
    Ok(document)
}

/// How far [`check`] takes a rule set: each level is one of SPARQL 1.2 RL's conformance
/// questions, and each includes the ones before it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CheckLevel {
    /// §7: "A conforming SRL document is an RDF string that conforms to the grammar
    /// starting with the RuleSet production" — the document and every document its
    /// `IMPORTS` closure reads. The question a W3C positive syntax test asks, "regardless
    /// of well-formedness and stratification".
    Syntax,
    /// [`Self::Syntax`], and §4.2: "A rule set is a well-formed rule set if and only if all
    /// rules of the rule set are well-formed rules" — the imported rules included.
    WellFormed,
    /// [`Self::WellFormed`], and §4.4: the combined rule set can be stratified. Every static
    /// check [`infer`] applies before it evaluates, so a rule set accepted at this level is
    /// one [`infer`] will not refuse before reading the base graph. The default.
    #[default]
    Stratified,
}

impl CheckLevel {
    /// Every level, in order.
    pub const ALL: [Self; 3] = [Self::Syntax, Self::WellFormed, Self::Stratified];

    /// The level's name, as every host spells it: `syntax`, `well-formed`, `stratified`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Syntax => "syntax",
            Self::WellFormed => "well-formed",
            Self::Stratified => "stratified",
        }
    }

    /// The level [`Self::name`] spells, or `None`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.name() == name)
    }

    /// What an accepted rule set is at this level, for [`CheckedRuleSet::summary`].
    const fn verdict(self) -> &'static str {
        match self {
            Self::Syntax => "is syntactically valid",
            Self::WellFormed => "is well formed",
            Self::Stratified => "is well formed and stratified",
        }
    }
}

impl fmt::Display for CheckLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The static checks SPARQL 1.2 RL applies to a rule set before §6 "Rule Set Evaluation",
/// up to `level`, and nothing else: the §7 grammar, the §4.5 `IMPORTS` closure resolved
/// from `imports` (each imported document held to the grammar), then — from
/// [`CheckLevel::WellFormed`] — §4.2 well-formedness of every rule, imported ones
/// included, then — at [`CheckLevel::Stratified`] — §4.4 stratification of the combined
/// rule set. No base graph is read and no rule is evaluated.
///
/// This is the check-only entry point every PurRDF host exposes (`purrdf rules --srl FILE
/// --check[=LEVEL]`, Python `check_rules`, WebAssembly `shaclCheckRules`, C
/// `purrdf_shacl_check_rules`), and every host's rules run passes through it at
/// [`CheckLevel::Stratified`] before it evaluates: a rule set accepted there is exactly one
/// [`infer`] will not refuse statically, and one refused there is refused by every rules
/// run with the same error.
///
/// `base` is the rule set's base IRI from outside it, as [`parse`] takes it. `imports` is
/// the import table [`RuleSetDocument::resolve_import_table`] resolves the `IMPORTS`
/// closure from — the empty table still refuses a rule set that imports anything, and an
/// entry the closure never reaches is refused as unused.
///
/// # Errors
///
/// [`SrlError::Syntax`], [`SrlError::Import`], [`SrlError::UnreachedImports`],
/// [`SrlError::WellFormedness`], [`SrlError::Stratification`] — each naming the stage that
/// refused, so a caller can tell a document the grammar refuses from a rule set that is
/// ill formed from one that cannot be stratified.
pub fn check(
    text: &str,
    base: Option<&str>,
    imports: &[(&str, &str)],
    level: CheckLevel,
) -> Result<CheckedRuleSet, SrlError> {
    let document = parse(text, base)?;
    if level >= CheckLevel::WellFormed {
        // The importer's own rules first, so an ill-formed rule is named before an import
        // that cannot be resolved — the order a rules run has always refused in.
        document.check_well_formed()?;
    }
    let (document, imported) = document.resolve_import_table_listing(imports)?;
    if level >= CheckLevel::WellFormed && !imported.is_empty() {
        // The combined rule set: an imported rule is a rule of the set an evaluation runs,
        // so it is held to §4.2 exactly as the importer's own are (its error names its
        // document).
        document.check_well_formed()?;
    }
    let strata = if level >= CheckLevel::Stratified {
        Some(document.stratify()?)
    } else {
        None
    };
    Ok(CheckedRuleSet {
        level,
        document,
        imported,
        strata,
    })
}

/// A SPARQL 1.2 RL rule set [`check`] accepted at its [`CheckLevel`]: parsed with its
/// imports resolved and combined, and — as the level asks — well formed and stratified.
#[derive(Debug, Clone)]
pub struct CheckedRuleSet {
    /// The level it was checked to.
    level: CheckLevel,
    /// The combined rule set, imports resolved.
    document: RuleSetDocument,
    /// The imported rule sets' IRIs, in the order §A read them.
    imported: Vec<String>,
    /// The stratification of the combined rule set, when the level asked for it.
    strata: Option<Vec<Stratum>>,
}

impl CheckedRuleSet {
    /// The level the rule set was checked to.
    #[must_use]
    pub const fn level(&self) -> CheckLevel {
        self.level
    }

    /// The combined rule set — the importer's rules, then every imported rule — which
    /// [`infer`] takes when the level was [`CheckLevel::Stratified`].
    #[must_use]
    pub fn document(&self) -> &RuleSetDocument {
        &self.document
    }

    /// The combined rule set, owned.
    #[must_use]
    pub fn into_document(self) -> RuleSetDocument {
        self.document
    }

    /// The IRIs of the imported rule sets, each once, in the order they were read.
    #[must_use]
    pub fn imported(&self) -> &[String] {
        &self.imported
    }

    /// The stratification layers of the combined rule set (§4.4.2), or `None` below
    /// [`CheckLevel::Stratified`], which does not stratify.
    #[must_use]
    pub fn strata(&self) -> Option<&[Stratum]> {
        self.strata.as_deref()
    }

    /// The one-line summary every host reports for an accepted rule set: the level it
    /// passed, the counts of rules, data-block triples, imported rule sets and (when
    /// stratified) strata, and the `VERSION` labels.
    #[must_use]
    pub fn summary(&self) -> String {
        fn plural(count: usize, one: &str, many: &str) -> String {
            format!("{count} {}", if count == 1 { one } else { many })
        }
        let versions = if self.document.versions.is_empty() {
            "no VERSION".to_owned()
        } else {
            format!(
                "VERSION {}",
                self.document
                    .versions
                    .iter()
                    .map(|label| format!("\"{label}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let mut counts = vec![
            plural(self.document.rules.len(), "rule", "rules"),
            plural(self.document.data.len(), "data triple", "data triples"),
            plural(
                self.imported.len(),
                "imported rule set",
                "imported rule sets",
            ),
        ];
        if let Some(strata) = &self.strata {
            counts.push(plural(strata.len(), "stratum", "strata"));
        }
        counts.push(versions);
        format!(
            "SPARQL 1.2 RL rule set {} (level {}): {}",
            self.level.verdict(),
            self.level.name(),
            counts.join(", ")
        )
    }
}

/// Options for [`infer`]: the four rule-evaluation limits, and how the host names their
/// knobs.
#[derive(Debug, Clone)]
pub struct InferOptions {
    /// The limits, as the rules engine takes them.
    rules: RuleOptions,
}

impl Default for InferOptions {
    fn default() -> Self {
        Self {
            rules: RuleOptions::default().with_limit_knobs(crate::rules::LimitKnobs::new(
                "InferOptions::with_max_term_generating_rounds",
                "InferOptions::with_max_generated_terms",
                "InferOptions::with_max_stored_facts",
                "InferOptions::with_max_join_steps",
            )),
        }
    }
}

impl InferOptions {
    /// Permit exactly `rounds` evaluation rounds that infer a term the evaluation graph
    /// did not hold. A general rule builds no blank node and assigns nothing, but it can
    /// build a triple term from a triple term it matched, every round; SPARQL 1.2 RL §C:
    /// "Applications should take care to limit the amount of computation and memory usage
    /// that can be caused by applying a SPARQL-RL rule set."
    ///
    /// The default and its reasoning are
    /// [`RuleOptions::with_max_term_generating_rounds`]'s; a run past the limit is
    /// refused as [`SrlError::LimitExceeded`].
    #[must_use]
    pub fn with_max_term_generating_rounds(mut self, rounds: u64) -> Self {
        self.rules = self.rules.with_max_term_generating_rounds(rounds);
        self
    }

    /// Permit exactly `terms` terms inferred beyond the input's; the default and its
    /// reasoning are [`RuleOptions::with_max_generated_terms`]'s, and a run past the
    /// budget is refused as [`SrlError::LimitExceeded`].
    #[must_use]
    pub fn with_max_generated_terms(mut self, terms: u64) -> Self {
        self.rules = self.rules.with_max_generated_terms(terms);
        self
    }

    /// Permit an evaluation store of exactly `facts` facts — the base graph, the data
    /// blocks and every inferred triple; the default and its reasoning are
    /// [`RuleOptions::with_max_stored_facts`]'s, and a run past the limit is refused as
    /// [`SrlError::LimitExceeded`].
    #[must_use]
    pub fn with_max_stored_facts(mut self, facts: u64) -> Self {
        self.rules = self.rules.with_max_stored_facts(facts);
        self
    }

    /// Permit exactly `steps` candidate solutions enumerated by the rule bodies; the
    /// default and its reasoning are [`RuleOptions::with_max_join_steps`]'s, and a run past
    /// the limit is refused as [`SrlError::LimitExceeded`].
    #[must_use]
    pub fn with_max_join_steps(mut self, steps: u64) -> Self {
        self.rules = self.rules.with_max_join_steps(steps);
        self
    }

    /// Name the limits' knobs as the host exposes them (see
    /// [`RuleOptions::with_limit_knobs`]).
    #[must_use]
    pub fn with_limit_knobs(mut self, knobs: crate::rules::LimitKnobs) -> Self {
        self.rules = self.rules.with_limit_knobs(knobs);
        self
    }
}

/// Apply a rule set to a base graph — the default graph of `base` — and return the
/// inference graph: SPARQL 1.2 RL §6, "Inputs: data graph G, called the base graph, and a
/// rule set RS. Output: an RDF graph GI of inferred triples".
///
/// # Errors
///
/// [`SrlError::Import`] for unresolved imports, [`SrlError::WellFormedness`],
/// [`SrlError::Stratification`], [`SrlError::LimitExceeded`], [`SrlError::Evaluation`].
pub fn infer(
    document: &RuleSetDocument,
    base: &RdfDataset,
    options: &InferOptions,
) -> Result<Inference, SrlError> {
    document.check_well_formed()?;
    document.stratify()?;
    let projected =
        crate::engine::project_dataset(base).map_err(|message| SrlError::Evaluation { message })?;
    let data = ShaclData::new(Arc::clone(&projected), projected, None);
    eval::evaluate(
        &document.rule_set(),
        &data,
        &Shapes::default(),
        &options.rules,
    )
    .map_err(|error| match error {
        crate::rules::RulesError::LimitExceeded(mut limit) => {
            limit.rename(|index| document.rules.get(index).map(SrlRule::describe));
            SrlError::LimitExceeded(limit)
        }
        crate::rules::RulesError::Failed(message) => SrlError::Evaluation { message },
    })
}

impl RuleSetDocument {
    /// The rules, in document order.
    #[must_use]
    pub fn rules(&self) -> &[SrlRule] {
        &self.rules
    }

    /// The triples of the data blocks.
    #[must_use]
    pub fn data(&self) -> &[[Term; 3]] {
        &self.data
    }

    /// The imports not yet resolved.
    #[must_use]
    pub fn imports(&self) -> &[NamedNode] {
        &self.imports
    }

    /// The `VERSION` labels, in document order.
    #[must_use]
    pub fn versions(&self) -> &[String] {
        &self.versions
    }

    /// The rule set in the rule-set IR, scheduled by stratification.
    #[must_use]
    pub fn rule_set(&self) -> RuleSet<'static> {
        RuleSet {
            rules: self
                .rules
                .iter()
                .enumerate()
                .map(|(index, rule)| IrRule {
                    id: rule
                        .iri
                        .clone()
                        .map_or_else(|| Term::blank(format!("srl-rule-{index}")), Term::NamedNode),
                    body: IrRuleBody::Elements(rule.rule.clone()),
                    schedule: DeclaredSchedule::default(),
                    expected_predicates: Vec::new(),
                })
                .collect(),
            data: self.data.clone(),
            scheduling: Scheduling::Stratified,
        }
    }

    /// SPARQL 1.2 RL §4.2: "A rule set is a well-formed rule set if and only if all rules
    /// of the rule set are well-formed rules."
    ///
    /// # Errors
    ///
    /// [`SrlError::WellFormedness`] naming the first ill-formed rule.
    pub fn check_well_formed(&self) -> Result<(), SrlError> {
        for rule in &self.rules {
            rule.rule
                .check_well_formed()
                .map_err(|message| SrlError::WellFormedness {
                    rule: rule.describe(),
                    message,
                })?;
        }
        Ok(())
    }

    /// The stratification of the (resolved) rule set: §4.4.2's algorithm over §4.3's
    /// dependency graph.
    ///
    /// # Errors
    ///
    /// [`SrlError::Import`] when imports are unresolved, [`SrlError::Stratification`]
    /// naming a closed dependency in a cycle and the cycle.
    pub fn stratify(&self) -> Result<Vec<Stratum>, SrlError> {
        if let Some(iri) = self.imports.first() {
            return Err(SrlError::Import {
                iri: iri.as_str().to_owned(),
                message: "the document's imports are not resolved; resolve them with \
                          RuleSetDocument::resolve_imports (SPARQL 1.2 RL §6.2: \"Evaluation \
                          of a rule set involves collecting all imported rule sets, building \
                          a single, combined rule set\")"
                    .to_owned(),
            });
        }
        let schedule = depend::stratify(&self.rule_set()).map_err(|error| match error {
            EvalError::NonStratifiableRules {
                rule,
                depends_on,
                cycle,
            } => SrlError::Stratification {
                rule: self.rules[rule].describe(),
                depends_on: self.rules[depends_on].describe(),
                cycle: cycle
                    .iter()
                    .map(|&index| self.rules[index].describe())
                    .collect(),
            },
            other => SrlError::Evaluation {
                message: other.to_string(),
            },
        })?;
        Ok(schedule
            .layers()
            .iter()
            .map(|layer| Stratum {
                once: layer.once().iter().flatten().copied().collect(),
                general: layer.iterating().iter().flatten().copied().collect(),
            })
            .collect())
    }

    /// Resolve the document's imports through `resolver`, recursively, each import read
    /// once — SPARQL 1.2 RL §A. An imported document is parsed with its import IRI as its
    /// base (§7.4, "the URL from which a particular SPARQL-RL document was retrieved"),
    /// its rules follow the importer's, and its data-block blank nodes are standardized
    /// apart ("rdf_merge").
    ///
    /// # Errors
    ///
    /// [`SrlError::Import`]: the resolver declines or fails, or an imported document is
    /// not a SPARQL-RL Document.
    pub fn resolve_imports(&self, resolver: &mut dyn ImportResolver) -> Result<Self, SrlError> {
        self.resolve_imports_listing(resolver)
            .map(|(merged, _)| merged)
    }

    /// [`Self::resolve_imports`], also answering the IRIs of the rule sets it read, in
    /// the order it read them.
    fn resolve_imports_listing(
        &self,
        resolver: &mut dyn ImportResolver,
    ) -> Result<(Self, Vec<String>), SrlError> {
        let mut visited: Vec<String> = self.location.iter().cloned().collect();
        let preloaded = visited.len();
        let mut merged = Self {
            rules: self.rules.clone(),
            data: self.data.clone(),
            imports: Vec::new(),
            versions: self.versions.clone(),
            location: self.location.clone(),
        };
        let mut imported = 0usize;
        resolve_into(
            &self.imports,
            &mut visited,
            resolver,
            &mut merged,
            &mut imported,
        )?;
        visited.drain(..preloaded);
        Ok((merged, visited))
    }
}

/// Why an import no table entry names is unresolved — the text of the
/// [`SrlError::Import`] [`RuleSetDocument::resolve_import_table`] refuses it with.
pub const UNRESOLVED_IMPORT_MESSAGE: &str = "no import-table entry supplies the rule set it \
     names, and PurRDF fetches nothing it was not handed; supply that rule set's text under \
     this IRI";

impl RuleSetDocument {
    /// Resolve the document's imports from a caller-supplied TABLE of `(import IRI, rule
    /// set text)` pairs — the one route every PurRDF host resolves `IMPORTS` through.
    ///
    /// PurRDF fetches nothing (see [`ImportResolver`]), so the table is the whole of
    /// what an import can resolve to, followed transitively by [`Self::resolve_imports`]:
    /// an imported rule set's own `IMPORTS` are looked up in the same table, each IRI read
    /// once. A table entry must also be USED — an entry the closure never names would be
    /// read and never used, so it is refused rather than ignored.
    ///
    /// # Errors
    ///
    /// [`SrlError::Import`] for a table key that is not an absolute IRI or that the table
    /// names twice, for an import no entry names ([`UNRESOLVED_IMPORT_MESSAGE`]), and for
    /// an entry that is not a SPARQL-RL document; [`SrlError::UnreachedImports`] for
    /// entries the closure never names.
    pub fn resolve_import_table(&self, table: &[(&str, &str)]) -> Result<Self, SrlError> {
        self.resolve_import_table_listing(table)
            .map(|(merged, _)| merged)
    }

    /// [`Self::resolve_import_table`], also answering the IRIs of the rule sets it read.
    fn resolve_import_table_listing(
        &self,
        table: &[(&str, &str)],
    ) -> Result<(Self, Vec<String>), SrlError> {
        for (index, (iri, _)) in table.iter().enumerate() {
            if !purrdf_iri::is_absolute(iri).unwrap_or(false) {
                return Err(SrlError::Import {
                    iri: (*iri).to_owned(),
                    message: "an import-table key must be the absolute IRI an IMPORTS names; \
                              this one could never match one"
                        .to_owned(),
                });
            }
            if table[..index].iter().any(|(earlier, _)| earlier == iri) {
                return Err(SrlError::Import {
                    iri: (*iri).to_owned(),
                    message: "the import table names this IRI twice, and one IRI names one \
                              rule set; keeping either would be a choice made for the caller"
                        .to_owned(),
                });
            }
        }
        let mut used: Vec<&str> = Vec::new();
        let mut resolver = |iri: &str| -> Result<String, String> {
            let (key, text) = table
                .iter()
                .find(|(key, _)| *key == iri)
                .ok_or_else(|| UNRESOLVED_IMPORT_MESSAGE.to_owned())?;
            used.push(key);
            Ok((*text).to_owned())
        };
        let resolved = self.resolve_imports_listing(&mut resolver)?;
        let unreached: Vec<String> = table
            .iter()
            .filter(|(key, _)| !used.contains(key))
            .map(|(key, _)| (*key).to_owned())
            .collect();
        if !unreached.is_empty() {
            return Err(SrlError::UnreachedImports { iris: unreached });
        }
        Ok(resolved)
    }
}

/// §A `imports(RS, V)`, merging into `merged`.
fn resolve_into(
    imports: &[NamedNode],
    visited: &mut Vec<String>,
    resolver: &mut dyn ImportResolver,
    merged: &mut RuleSetDocument,
    imported: &mut usize,
) -> Result<(), SrlError> {
    for import in imports {
        let iri = import.as_str();
        if visited.iter().any(|seen| seen == iri) {
            continue;
        }
        visited.push(iri.to_owned());
        let text = resolver.resolve(iri).map_err(|message| SrlError::Import {
            iri: iri.to_owned(),
            message,
        })?;
        let document =
            parse_document(&text, Some(iri), Some(iri)).map_err(|error| SrlError::Import {
                iri: iri.to_owned(),
                message: error.to_string(),
            })?;
        *imported += 1;
        // rdf_merge: the imported data's blank nodes are not the merged data's.
        let prefix = {
            let mut prefix = format!("import{imported}-");
            while merged
                .data
                .iter()
                .flatten()
                .any(|term| mentions_label_prefix(term, &prefix))
            {
                prefix.insert(0, 'x');
            }
            prefix
        };
        merged.rules.extend(document.rules);
        merged.data.extend(
            document
                .data
                .iter()
                .map(|triple| triple.each_ref().map(|term| prefix_blanks(term, &prefix))),
        );
        merged.versions.extend(document.versions);
        resolve_into(&document.imports, visited, resolver, merged, imported)?;
    }
    Ok(())
}

/// Whether a blank-node label in `term` starts with `prefix`.
fn mentions_label_prefix(term: &Term, prefix: &str) -> bool {
    match term {
        Term::BlankNode(label) => label.starts_with(prefix),
        Term::Triple(inner) => {
            mentions_label_prefix(&inner.subject, prefix)
                || mentions_label_prefix(&inner.object, prefix)
        }
        Term::NamedNode(_) | Term::Literal(_) => false,
    }
}

/// `term` with every blank-node label prefixed.
fn prefix_blanks(term: &Term, prefix: &str) -> Term {
    match term {
        Term::BlankNode(label) => Term::blank(format!("{prefix}{label}")),
        Term::Triple(inner) => Term::Triple(Box::new(Triple {
            subject: prefix_blanks(&inner.subject, prefix),
            predicate: inner.predicate.clone(),
            object: prefix_blanks(&inner.object, prefix),
        })),
        Term::NamedNode(_) | Term::Literal(_) => term.clone(),
    }
}

/// Parse `text` under `base`, naming `document` in errors and rules.
fn parse_document(
    text: &str,
    base: Option<&str>,
    document: Option<&str>,
) -> Result<RuleSetDocument, SrlError> {
    let lines = LineIndex::new(text);
    let parsed = syntax::parse(text, base).map_err(|error| {
        let position = lines.locate(text, error.offset);
        SrlError::Syntax {
            document: document.map(ToOwned::to_owned),
            line: position.line,
            column: position.column,
            message: error.message,
        }
    })?;
    Ok(RuleSetDocument {
        rules: parsed
            .rules
            .into_iter()
            .map(|rule| {
                let position = lines.locate(text, rule.offset);
                SrlRule {
                    iri: rule.iri,
                    rule: rule.rule,
                    document: document.map(ToOwned::to_owned),
                    line: position.line,
                    column: position.column,
                }
            })
            .collect(),
        data: parsed.data,
        imports: parsed.imports,
        versions: parsed.versions,
        location: base.map(ToOwned::to_owned),
    })
}
