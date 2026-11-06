// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original native failures of standalone SHACL SPARQL operations.

use purrdf_core::RdfDiagnostic;
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};
use purrdf_sparql_eval::EvalError;

use crate::error::{FailureClass, ShapesError};

/// A native SPARQL failure together with the SHACL operation that encountered it.
/// The original cause remains available through standard error sources and
/// borrowed structured inspection, including after the input owners are dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationError {
    operation: Operation,
    cause: Cause,
    class: FailureClass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Operation {
    Aggregate(String),
    Environment,
    Binding,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Cause {
    Evaluation(EvalError),
    Binding(RdfDiagnostic),
}

impl OperationError {
    pub(super) fn aggregate(name: &str, cause: EvalError) -> ShapesError {
        Self::evaluation(Operation::Aggregate(name.to_owned()), cause)
    }

    pub(super) fn environment(cause: EvalError) -> ShapesError {
        Self::evaluation(Operation::Environment, cause)
    }

    fn evaluation(operation: Operation, cause: EvalError) -> ShapesError {
        let class = if cause.is_native_storage_failure()
            || cause
                .code()
                .is_some_and(EvalError::diagnostic_requires_propagation)
        {
            FailureClass::Processor
        } else {
            FailureClass::Evaluation
        };
        ShapesError::SparqlOperation(Box::new(Self {
            operation,
            cause: Cause::Evaluation(cause),
            class,
        }))
    }

    pub(super) fn binding(cause: RdfDiagnostic) -> ShapesError {
        let class = if cause.storage_error().is_some()
            || EvalError::diagnostic_requires_propagation(&cause.code)
        {
            FailureClass::Processor
        } else {
            FailureClass::Evaluation
        };
        ShapesError::SparqlOperation(Box::new(Self {
            operation: Operation::Binding,
            cause: Cause::Binding(cause),
            class,
        }))
    }

    pub(crate) const fn failure_class(&self) -> FailureClass {
        self.class
    }
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.operation {
            Operation::Aggregate(name) => write!(f, "aggregate {name} evaluation error: ")?,
            Operation::Environment => f.write_str("extension environment: ")?,
            Operation::Binding => {}
        }
        match &self.cause {
            Cause::Evaluation(cause) => cause.fmt(f),
            Cause::Binding(cause) => cause.fmt(f),
        }
    }
}

impl std::error::Error for OperationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match &self.cause {
            Cause::Evaluation(cause) => cause,
            Cause::Binding(cause) => cause,
        })
    }
}

impl StructuredError for OperationError {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        match &self.operation {
            Operation::Aggregate(name) => {
                visitor.variant("shapes-sparql-aggregate");
                visitor.value(Some("aggregate"), ErrorValue::Text(name));
            }
            Operation::Environment => visitor.variant("shapes-sparql-environment"),
            Operation::Binding => visitor.variant("shapes-sparql-function-binding"),
        }
        visitor.value(
            Some("cause"),
            ErrorValue::Cause(match &self.cause {
                Cause::Evaluation(cause) => cause,
                Cause::Binding(cause) => cause,
            }),
        );
    }
}
