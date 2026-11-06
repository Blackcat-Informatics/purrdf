// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Well-formedness conditions and their original variable operands.

use crate::term::Term;
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Condition {
    Head(String),
    Filter(String),
    NegatedAssignment(String),
    AssignmentRead { variable: String, read: String },
    Rebound(String),
    NestedNegation,
    Exists,
}

/// The first violated rule condition, with original variable and rule operands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WellFormednessError(Box<Failure>);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Failure {
    condition: Condition,
    rule: Option<Term>,
}

impl WellFormednessError {
    pub(super) fn new(condition: Condition) -> Self {
        Self(Box::new(Failure {
            condition,
            rule: None,
        }))
    }

    pub(crate) fn in_rule(mut self, rule: Term) -> Self {
        self.0.rule = Some(rule);
        self
    }
}

impl std::fmt::Display for WellFormednessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(rule) = &self.0.rule {
            write!(f, "rule {rule} is not well formed: ")?;
        }
        match &self.0.condition {
            Condition::Head(variable) => write!(
                f,
                "head variable ?{variable} is not bound by the rule body (SPARQL 1.2 \
                RL: \"each variable in a triple template of the rule head is an \
                element of Vall\")"
            ),
            Condition::Filter(variable) => write!(
                f,
                "filter reads ?{variable} before any earlier element binds it \
                (SPARQL 1.2 RL: \"every variable mentioned in a filter element is \
                an element of Vi-1\")"
            ),
            Condition::NegatedAssignment(variable) => write!(
                f,
                "an assignment to ?{variable} inside a negation element; a negation \
                element body is \"comprised of a sequence of triple pattern elements \
                and filter elements\""
            ),
            Condition::AssignmentRead { variable, read } => write!(
                f,
                "assignment to ?{variable} reads ?{read} before any earlier element \
                binds it"
            ),
            Condition::Rebound(variable) => write!(
                f,
                "assignment to ?{variable}, which an earlier element already binds \
                (SPARQL 1.2 RL: \"the assignment variable is an not element of \
                Vi-1\")"
            ),
            Condition::NestedNegation => f.write_str(
                "a negation element nested inside a negation element; its body holds \
                triple patterns and filters only",
            ),
            Condition::Exists => f.write_str(
                "an EXISTS pattern inside a rule-element expression; SPARQL 1.2 RL expresses \
                absence with its negation element (NOT { … })",
            ),
        }
    }
}

impl std::error::Error for WellFormednessError {}

impl StructuredError for WellFormednessError {
    fn visit<'e>(&'e self, visitor: &mut dyn ErrorVisitor<'e>) {
        let (code, variable, read) = match &self.0.condition {
            Condition::Head(variable) => ("srl-head-unbound", Some(variable), None),
            Condition::Filter(variable) => ("srl-filter-unbound", Some(variable), None),
            Condition::NegatedAssignment(variable) => {
                ("srl-negated-assignment", Some(variable), None)
            }
            Condition::AssignmentRead { variable, read } => {
                ("srl-assignment-unbound", Some(variable), Some(read))
            }
            Condition::Rebound(variable) => ("srl-assignment-rebound", Some(variable), None),
            Condition::NestedNegation => ("srl-nested-negation", None, None),
            Condition::Exists => ("srl-expression-exists", None, None),
        };
        visitor.variant(code);
        visitor.value(
            Some("variable"),
            variable.map_or(ErrorValue::Absent, |value| ErrorValue::Text(value)),
        );
        visitor.value(
            Some("read"),
            read.map_or(ErrorValue::Absent, |value| ErrorValue::Text(value)),
        );
        visitor.value(
            Some("rule"),
            self.0
                .rule
                .as_ref()
                .map_or(ErrorValue::Absent, |rule| ErrorValue::Term(rule)),
        );
    }
}
