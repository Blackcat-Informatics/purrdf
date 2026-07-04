// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RIF-Core rule model consumed by the forward-chaining evaluator.
//!
//! Every RIF construct these conformance cases use is a *frame* `o[p->v]`, which
//! maps to the RDF triple `(o, p, v)`. So the whole model is triple-shaped: a
//! [`Fact`] is a ground triple, an [`Atom`] is a triple pattern (with variables),
//! a [`Rule`] is a definite Horn clause `head :- body`, and a [`RuleSet`] is a bag
//! of ground facts plus rules. The evaluator forward-chains this to a fixpoint.
//!
//! The model is deliberately monotonic definite Horn: no built-ins, no negation,
//! no class membership (`#`/`##`), no `External`. Those are absent from the RIF
//! SPARQL-entailment conformance cases and would be a separate, larger effort.

use purrdf_core::TermValue;

/// A ground RDF triple derived from a RIF frame `o[p->v]`.
pub type Fact = (TermValue, TermValue, TermValue);

/// One position of a triple pattern: a named variable or a ground term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RifTerm {
    /// A universally-quantified rule variable, by name (e.g. `x`).
    Var(String),
    /// A ground term (an IRI or a typed/string literal).
    Const(TermValue),
}

/// A triple pattern `s p o` (a single-slot RIF frame), possibly with variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atom {
    /// The frame object (triple subject).
    pub s: RifTerm,
    /// The slot predicate (triple predicate) — always a ground IRI in these cases.
    pub p: RifTerm,
    /// The slot value (triple object).
    pub o: RifTerm,
}

/// A definite Horn rule `head :- body`: if every body atom matches (under one
/// consistent variable binding), every head atom is derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// The rule body (`if`): a conjunction of triple patterns.
    pub body: Vec<Atom>,
    /// The rule head (`then`): the triple patterns to derive.
    pub head: Vec<Atom>,
}

/// A parsed RIF document: ground facts plus Horn rules.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuleSet {
    /// Ground frame facts (and any imported RDF triples added by the loader).
    pub facts: Vec<Fact>,
    /// Forward-chaining Horn rules.
    pub rules: Vec<Rule>,
}

impl RuleSet {
    /// An empty rule set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a ground fact.
    pub fn push_fact(&mut self, fact: Fact) {
        self.facts.push(fact);
    }

    /// Append a rule.
    pub fn push_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// Merge another rule set's facts and rules into this one.
    pub fn extend(&mut self, other: Self) {
        self.facts.extend(other.facts);
        self.rules.extend(other.rules);
    }
}
