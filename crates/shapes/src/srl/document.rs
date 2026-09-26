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
//! they are resolved), and [`infer`] — "Infer is the operation that applies a rule set to
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

use ::purrdf::RdfDataset;
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
        line: u32,
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
    /// The rule set diverged under the default term-generating limit
    /// ([`crate::rules::TermGeneratingLimit::Horizon`]): it kept inferring new terms past
    /// the horizon its input grants. Names the rules that inferred one in the last round.
    Divergence(crate::rules::Divergence),
    /// Evaluation failed: a guard error, a fixed term-generating round limit passed, or
    /// an inferred triple that is not an RDF triple.
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
            Self::Divergence(divergence) => {
                write!(f, "SPARQL 1.2 RL evaluation failed: {divergence}")
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
    line: u32,
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

/// Options for [`infer`].
#[derive(Debug, Clone, Default)]
pub struct InferOptions {
    /// The limit on evaluation rounds that infer a term the evaluation graph did not
    /// hold.
    term_generating_limit: crate::rules::TermGeneratingLimit,
}

impl InferOptions {
    /// Permit exactly `rounds` evaluation rounds that infer a term the evaluation graph
    /// did not hold. A general rule builds no blank node and assigns nothing, but it can
    /// build a triple term from a triple term it matched, every round; SPARQL 1.2 RL §C:
    /// "Applications should take care to limit the amount of computation and memory usage
    /// that can be caused by applying a SPARQL-RL rule set."
    ///
    /// The DEFAULT ([`crate::rules::TermGeneratingLimit::Horizon`]) is a divergence
    /// criterion derived from the input: at most `max(256, 4 × N)` such rounds, `N` the
    /// distinct terms of the base graph and the data blocks, past which the rule set is
    /// refused as [`SrlError::Divergence`]. A rule set bounded by a constant past that
    /// horizon terminates; its caller states the bound here.
    #[must_use]
    pub fn with_max_term_generating_rounds(mut self, rounds: u64) -> Self {
        self.term_generating_limit = crate::rules::TermGeneratingLimit::Fixed(rounds);
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
/// [`SrlError::Stratification`], [`SrlError::Evaluation`].
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
    let rule_options = match options.term_generating_limit {
        crate::rules::TermGeneratingLimit::Fixed(rounds) => {
            RuleOptions::default().with_max_term_generating_rounds(rounds)
        }
        crate::rules::TermGeneratingLimit::Horizon => RuleOptions::default(),
    };
    eval::evaluate(
        &document.rule_set(),
        &data,
        &Shapes::default(),
        &rule_options,
    )
    .map_err(|error| match error {
        crate::rules::RulesError::Diverged(mut divergence) => {
            divergence.rename(|index| document.rules.get(index).map(SrlRule::describe));
            SrlError::Divergence(divergence)
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
        let mut visited: Vec<String> = self.location.iter().cloned().collect();
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
        Ok(merged)
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
