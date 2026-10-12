// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The evaluator's owned transient frame for native numeric scalar computations.

use crate::{EvalError, WorkspaceAllocation, WorkspaceCapability};
use purrdf_xsd::bigint::LimbScratchError;
use purrdf_xsd::exact::cost::NumericOperationLayout;

/// Keeps every newly allocated native intermediate admitted until scalar completion.
/// Parsed input carriers independently keep their borrowed payloads admitted.
pub(crate) struct NumericFrame<'a> {
    workspace: &'a WorkspaceCapability,
    allocation: Option<WorkspaceAllocation>,
    failure: Option<EvalError>,
    admitted_bytes: usize,
}

pub(crate) struct NumericText {
    // Group payload dies before its original transient admission.
    text: purrdf_xsd::numeric::PreparedNumericText,
    _allocation: Option<WorkspaceAllocation>,
}

pub(crate) struct CalendarText<'a> {
    text: purrdf_xsd::temporal::PreparedCalendarText<'a>,
    _allocation: Option<WorkspaceAllocation>,
}

pub(crate) struct DurationText<'a> {
    text: purrdf_xsd::temporal::PreparedDurationText<'a>,
    _allocation: Option<WorkspaceAllocation>,
}

impl core::fmt::Display for DurationText<'_> {
    fn fmt(&self, output: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.text, output)
    }
}

impl core::fmt::Display for CalendarText<'_> {
    fn fmt(&self, output: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.text, output)
    }
}

impl NumericText {
    pub(crate) fn len(&self) -> usize {
        self.text.len()
    }
    pub(crate) fn write_to(&self, output: &mut String) -> Result<(), EvalError> {
        self.text
            .write_to(output)
            .map_err(|_| EvalError::WorkspaceBoundOverflow)
    }
}

impl<'a> NumericFrame<'a> {
    pub(crate) const fn new(workspace: &'a WorkspaceCapability) -> Self {
        Self {
            workspace,
            allocation: None,
            failure: None,
            admitted_bytes: 0,
        }
    }

    pub(crate) fn admit(&mut self, layout: NumericOperationLayout) -> Result<(), LimbScratchError> {
        // Returning the stored operational refusal must neither render an error
        // nor resize an already refused reservation.
        if self.failure.is_some() {
            return Err(LimbScratchError::AllocationFailed);
        }
        let Ok(bytes) = u64::try_from(layout.required_bytes()) else {
            return Err(LimbScratchError::SizeOverflow);
        };
        let admitted = if let Some(allocation) = &mut self.allocation {
            allocation.resize(bytes)
        } else if bytes == 0 {
            Ok(())
        } else {
            self.workspace.charge(bytes).map(|allocation| {
                self.allocation = Some(allocation);
            })
        };
        if let Err(error) = admitted {
            self.failure = Some(error);
            return Err(LimbScratchError::AllocationFailed);
        }
        self.admitted_bytes = layout.required_bytes();
        Ok(())
    }

    /// Only a Copy scalar may leave after this transient frame is released.
    /// An owned magnitude or text result needs its separate retained carrier.
    pub(crate) fn finish_scalar<T: Copy>(
        mut self,
        result: Result<T, LimbScratchError>,
    ) -> Result<T, EvalError> {
        if let Some(error) = self.failure.take() {
            return Err(error);
        }
        result.map_err(|error| match error {
            LimbScratchError::SizeOverflow => EvalError::WorkspaceBoundOverflow,
            _ => EvalError::AllocationFailed {
                construct: "native numeric destination",
            },
        })
    }
    /// Transfer a computed native payload into the same immutable owner used by
    /// parsed operands/cache entries. No magnitude leaves with a dropped grant.
    pub(crate) fn finish_value(
        mut self,
        result: Result<purrdf_xsd::XsdValue, purrdf_xsd::numeric::NumericOperationError>,
    ) -> Result<Result<crate::parsed_value::ParsedValue, Option<purrdf_xsd::ErrorCode>>, EvalError>
    {
        if self.failure.is_some() {
            // A native result/diagnostic, if any, dies before its transient owner.
            drop(result);
            return Err(self.failure.take().expect("checked stored failure"));
        }
        match result {
            Ok(value) => crate::parsed_value::ParsedValue::from_admitted(
                value,
                self.allocation.take(),
                self.admitted_bytes,
            )
            .map(Ok),
            Err(purrdf_xsd::numeric::NumericOperationError::Value(error)) => Ok(Err(error.code())),
            Err(purrdf_xsd::numeric::NumericOperationError::Storage(error)) => Err(match error {
                LimbScratchError::SizeOverflow => EvalError::WorkspaceBoundOverflow,
                _ => EvalError::AllocationFailed {
                    construct: "native numeric destination",
                },
            }),
        }
    }

    /// Prepared native groups are owned until their final lexical write.
    pub(crate) fn finish_text(
        self,
        result: Result<Option<purrdf_xsd::numeric::PreparedNumericText>, LimbScratchError>,
    ) -> Result<Option<NumericText>, EvalError> {
        self.finish_prepared(
            result,
            purrdf_xsd::numeric::PreparedNumericText::temporary_bytes,
        )
        .map(|result| {
            result.map(|(text, allocation)| NumericText {
                text,
                _allocation: allocation,
            })
        })
    }

    pub(crate) fn finish_calendar_text<'v>(
        self,
        result: Result<Option<purrdf_xsd::temporal::PreparedCalendarText<'v>>, LimbScratchError>,
    ) -> Result<Option<CalendarText<'v>>, EvalError> {
        self.finish_prepared(
            result,
            purrdf_xsd::temporal::PreparedCalendarText::temporary_bytes,
        )
        .map(|result| {
            result.map(|(text, allocation)| CalendarText {
                text,
                _allocation: allocation,
            })
        })
    }

    pub(crate) fn finish_duration_text<'v>(
        self,
        result: Result<purrdf_xsd::temporal::PreparedDurationText<'v>, LimbScratchError>,
    ) -> Result<DurationText<'v>, EvalError> {
        self.finish_prepared(
            result.map(Some),
            purrdf_xsd::temporal::PreparedDurationText::temporary_bytes,
        )
        .map(|result| {
            let (text, allocation) = result.expect("duration preparation returns a value");
            DurationText {
                text,
                _allocation: allocation,
            }
        })
    }

    fn finish_prepared<T>(
        mut self,
        result: Result<Option<T>, LimbScratchError>,
        temporary_bytes: impl Fn(&T) -> usize,
    ) -> Result<Option<(T, Option<WorkspaceAllocation>)>, EvalError> {
        if self.failure.is_some() {
            drop(result);
            return Err(self.failure.take().expect("checked stored failure"));
        }
        match result {
            Ok(Some(text)) => {
                if temporary_bytes(&text) > self.admitted_bytes {
                    drop(text);
                    return Err(EvalError::WorkspaceBoundOverflow);
                }
                Ok(Some((text, self.allocation.take())))
            }
            Ok(None) => Ok(None),
            Err(LimbScratchError::SizeOverflow) => Err(EvalError::WorkspaceBoundOverflow),
            Err(_) => Err(EvalError::AllocationFailed {
                construct: "native exact render groups",
            }),
        }
    }

    pub(crate) fn finish_cast(
        self,
        result: Result<Option<purrdf_xsd::XsdValue>, LimbScratchError>,
    ) -> Result<Option<crate::parsed_value::ParsedValue>, EvalError> {
        match result {
            Ok(None) => self.finish_scalar(Ok(())).map(|()| None),
            Ok(Some(value)) => self.finish_value(Ok(value)).map(Result::ok),
            Err(error) => self.finish_scalar::<()>(Err(error)).map(|()| None),
        }
    }
}
