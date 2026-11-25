// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller-registered comparisons of instants at declared civil precision.

use purrdf_core::TermValue;
use purrdf_hash::error::ErrorValue;
pub use purrdf_xsd::temporal::CivilPrecision;
use purrdf_xsd::temporal::{InstantOrder, PrecisionInterval};
use purrdf_xsd::{ErrorCode, XsdDatatype, XsdValue};

use crate::{
    EvalError, NativeDiagnostic, NativeDiagnosticKind, RetainedNativeFailure, WorkspaceCapability,
};

#[derive(Debug)]
struct InvalidCalendarLiteral {
    datatype: XsdDatatype,
    code: Option<ErrorCode>,
}

impl core::fmt::Display for InvalidCalendarLiteral {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "invalid temporal order argument for <{}>",
            self.datatype.iri()
        )?;
        if let Some(code) = self.code {
            write!(f, " ({code})")?;
        }
        Ok(())
    }
}

impl core::error::Error for InvalidCalendarLiteral {}

impl purrdf_hash::error::StructuredError for InvalidCalendarLiteral {
    purrdf_hash::error_visit! {
        @record self { datatype, code } => "temporal-order-invalid-calendar" {
            "datatype" => Iri(self.datatype.iri()),
            "xpath_code" => Value(self.code.map_or(ErrorValue::Absent, |code| ErrorValue::Code(code.qname()))),
        }
    }
}

/// The declared interpretation and arity of one temporal operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalOperand {
    /// One anchored XSD date, year-month, or year.
    Calendar,
    /// One XSD timestamp whose finer fields do not identify the instant.
    Timestamp(CivilPrecision),
    /// Two XSD dates supplied by the caller, with an exclusive end date.
    DateBounds,
}

impl TemporalOperand {
    const fn arity(self) -> usize {
        match self {
            Self::Calendar | Self::Timestamp(_) => 1,
            Self::DateBounds => 2,
        }
    }

    fn interval(
        self,
        input: &[&TermValue],
        workspace: &WorkspaceCapability,
    ) -> Result<PrecisionInterval, EvalError> {
        let args =
            |message| NativeDiagnostic::error(NativeDiagnosticKind::Function, message, workspace);
        let mut owner = crate::workspace::LexicalFrame::new(workspace);
        let mut memory = purrdf_lex::allocation::Memory::new(&mut owner);
        let mut parse = |value: &TermValue| {
            let TermValue::Literal {
                lexical_form,
                datatype,
                ..
            } = value
            else {
                return Err(args("temporal order requires a typed calendar literal"));
            };
            let datatype = XsdDatatype::from_iri(datatype)
                .filter(|dt| {
                    matches!(
                        dt,
                        XsdDatatype::Date
                            | XsdDatatype::DateTime
                            | XsdDatatype::DateTimeStamp
                            | XsdDatatype::GYearMonth
                            | XsdDatatype::GYear
                    )
                })
                .ok_or_else(|| args("temporal order requires an anchored XSD calendar datatype"))?;
            match purrdf_xsd::value::try_parse_with_memory(
                lexical_form,
                datatype,
                false,
                &mut memory,
            )
            .map_err(|error| {
                memory
                    .admission_mut()
                    .storage_error(error, "temporal order input")
            })? {
                purrdf_xsd::value::ParsedValue::Value(value) => Ok(value),
                purrdf_xsd::value::ParsedValue::Invalid(code) => Err(RetainedNativeFailure::error(
                    NativeDiagnosticKind::Function,
                    InvalidCalendarLiteral { datatype, code },
                    workspace,
                )),
            }
        };
        let interval = match self {
            Self::Calendar => PrecisionInterval::from_calendar(&parse(input[0])?),
            Self::Timestamp(precision) => match parse(input[0])? {
                XsdValue::DateTime(value) => {
                    Ok(PrecisionInterval::from_datetime(&value, precision))
                }
                _ => {
                    return Err(args(
                        "explicit timestamp precision requires xsd:dateTime or xsd:dateTimeStamp",
                    ));
                }
            },
            Self::DateBounds => {
                let (XsdValue::Date(start), XsdValue::Date(end)) =
                    (parse(input[0])?, parse(input[1])?)
                else {
                    return Err(args("an explicit period requires two xsd:date endpoints"));
                };
                PrecisionInterval::between_dates(&start, &end)
            }
        };
        interval.map_err(|error| {
            RetainedNativeFailure::error(NativeDiagnosticKind::Function, error, workspace)
        })
    }
}

/// The strict temporal order a caller asks a predicate to prove.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalDirection {
    /// The first represented instant is earlier.
    Before,
    /// The first represented instant is later.
    After,
}

/// A SPARQL predicate's typed operand and order declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemporalOrderPredicate {
    left: TemporalOperand,
    right: TemporalOperand,
    direction: TemporalDirection,
}

impl TemporalOrderPredicate {
    /// Declare how the predicate reads its two successive operands.
    #[must_use]
    pub const fn new(
        left: TemporalOperand,
        right: TemporalOperand,
        direction: TemporalDirection,
    ) -> Self {
        Self {
            left,
            right,
            direction,
        }
    }

    pub(crate) const fn arity(self) -> usize {
        self.left.arity() + self.right.arity()
    }

    /// The registry establishes the flattened arity before entering this body.
    pub(crate) fn evaluate(
        self,
        values: &[&TermValue],
        workspace: &WorkspaceCapability,
    ) -> Result<InstantOrder, EvalError> {
        let (left, right) = values.split_at(self.left.arity());
        let left = self.left.interval(left, workspace)?;
        let right = self.right.interval(right, workspace)?;
        Ok(match self.direction {
            TemporalDirection::Before => left.before(right),
            TemporalDirection::After => left.after(right),
        })
    }
}
