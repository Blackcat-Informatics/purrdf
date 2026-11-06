// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original rule-set selection and processor operands.

use crate::term::{NamedNode, Term};
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};

/// A declared rule processor or requested rule set the host cannot supply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleAdmissionError(Box<Failure>);

#[derive(Clone, Debug, PartialEq, Eq)]
enum Failure {
    Processor {
        subject: Term,
        processor: Term,
        rule_set: bool,
    },
    MissingSet(NamedNode),
}

impl RuleAdmissionError {
    pub(super) fn processor(subject: Term, processor: Term, rule_set: bool) -> Self {
        Self(Box::new(Failure::Processor {
            subject,
            processor,
            rule_set,
        }))
    }
    pub(super) fn missing_set(iri: NamedNode) -> Self {
        Self(Box::new(Failure::MissingSet(iri)))
    }
}

impl std::fmt::Display for RuleAdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.as_ref() {
            Failure::Processor {
                subject,
                processor,
                rule_set,
            } => {
                let kind = if *rule_set { "rule set" } else { "rule" };
                write!(
                    f,
                    "{kind} {subject} declares sh:ruleProcessor {processor}, which this rules engine \
                    has not been given a processor for; SHACL 1.2 Inference Rules: \"A rules \
                    engine that encounters rules or rule sets with a value for \
                    sh:ruleProcessor that they are unable to handle MUST report a failure\""
                )
            }
            Failure::MissingSet(iri) => write!(
                f,
                "rule set {} is not declared in the shapes graph (no sh:RuleSet of that IRI)",
                Term::NamedNode(iri.clone())
            ),
        }
    }
}

impl std::error::Error for RuleAdmissionError {}

impl StructuredError for RuleAdmissionError {
    fn visit<'e>(&'e self, visitor: &mut dyn ErrorVisitor<'e>) {
        match self.0.as_ref() {
            Failure::Processor {
                subject,
                processor,
                rule_set,
            } => {
                visitor.variant(if *rule_set {
                    "shacl-rule-set-processor"
                } else {
                    "shacl-rule-processor"
                });
                visitor.value(Some("subject"), ErrorValue::Term(subject));
                visitor.value(Some("processor"), ErrorValue::Term(processor));
            }
            Failure::MissingSet(iri) => {
                visitor.variant("shacl-rule-set-missing");
                visitor.value(Some("iri"), ErrorValue::Iri(iri.as_str()));
            }
        }
    }
}
