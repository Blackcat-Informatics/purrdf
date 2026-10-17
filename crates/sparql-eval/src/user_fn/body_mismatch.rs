// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original body results retained across their evaluation context's destruction.

use purrdf_core::{DatasetView, ViewTermId};
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError, visit_list};

use super::{AdmittedVec, EvalCtx, EvalError, Outcome, UserFnBody, WorkspaceTerm};

#[derive(Debug)]
struct BodyMismatch<I: ViewTermId> {
    iri: purrdf_lex::allocation::SharedText,
    expected: UserFnBody,
    // Keep the actual original outcome and all of its owners for presentation.
    // Inspection exposes its semantic result, not scratch/source-local IDs or
    // the physical schema, row, dictionary and admission control structures.
    outcome: Outcome<I>,
    rows: AdmittedVec<AdmittedVec<Option<WorkspaceTerm>>>,
}

pub(super) fn retain<D: DatasetView + Sync>(
    iri: &str,
    expected: UserFnBody,
    outcome: Outcome<D::Id>,
    child: &EvalCtx<'_, D>,
) -> Result<crate::RetainedNativeFailure, EvalError> {
    let iri = child.growth.authored_text(iri)?;
    let mut rows = AdmittedVec::new(&child.growth);
    if let Outcome::Solutions(solutions) = &outcome {
        for row in &solutions.rows {
            let mut values = AdmittedVec::new(&child.growth);
            for cell in row {
                values.push(
                    cell.map(|value| {
                        child.scratch.try_owned_value_of(
                            child.dataset,
                            value,
                            &child.growth,
                            |error| child.workspace.source_error(error),
                        )
                    })
                    .transpose()?,
                )?;
            }
            rows.push(values)?;
        }
    }
    crate::RetainedNativeFailure::new(
        crate::NativeDiagnosticKind::Internal,
        BodyMismatch {
            iri,
            expected,
            outcome,
            rows,
        },
        &child.growth,
    )
}

impl<I: ViewTermId> StructuredError for BodyMismatch<I> {
    fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
        let Self {
            iri,
            expected,
            outcome,
            rows,
        } = self;
        visitor.variant("shacl-function-body-mismatch");
        visitor.value(Some("iri"), ErrorValue::Iri(iri.as_str()));
        visitor.value(Some("expected"), ErrorValue::Record(expected));
        visitor.value(Some("actual"), ErrorValue::Code(outcome.form_label()));
        match outcome {
            Outcome::Boolean(value) => visitor.value(Some("outcome"), ErrorValue::Boolean(*value)),
            Outcome::Graph(graph) => visitor.value(Some("outcome"), ErrorValue::Record(&**graph)),
            Outcome::Solutions(solutions) => {
                visitor.begin_record(Some("outcome"), "shacl-function-body-solutions");
                visit_list(
                    visitor,
                    Some("variables"),
                    solutions
                        .schema
                        .vars()
                        .iter()
                        .map(|variable| ErrorValue::Text(variable.as_str())),
                );
                visitor.begin_list(Some("rows"), rows.len());
                for row in &**rows {
                    visit_list(
                        visitor,
                        None,
                        row.iter().map(|value| {
                            value
                                .as_ref()
                                .map_or(ErrorValue::Absent, |value| ErrorValue::Term(&**value))
                        }),
                    );
                }
                visitor.end_list();
                visitor.end_record();
            }
        }
    }
}

impl<I: ViewTermId> core::fmt::Display for BodyMismatch<I> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "SHACL-AF function <{}> body kind {:?} produced {:?}",
            self.iri, self.expected, self.outcome
        )
    }
}

impl<I: ViewTermId> std::error::Error for BodyMismatch<I> {}
