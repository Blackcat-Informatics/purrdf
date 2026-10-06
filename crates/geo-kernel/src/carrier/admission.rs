// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Integer-only carrier admission before geometry and rational allocations.

use crate::{ExecutionPolicy, GeoError, MetricWorkObserver, Rat};

/// Actual cumulative parser work and conservative retained source allowance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseReceipt {
    work: u64,
    workspace: u64,
}
impl ParseReceipt {
    /// Logical admitted parse work, including exact decimal construction.
    #[must_use]
    pub const fn work_items(self) -> u64 {
        self.work
    }
    /// Conservative complete retained source/scratch allowance.
    #[must_use]
    pub const fn workspace_bytes(self) -> u64 {
        self.workspace
    }
}

/// Integer-only admission for original source bytes and exact decimal construction.
/// It can serve a strict record reader before any typed coordinates are built.
pub struct ParseAdmission<'a> {
    policy: ExecutionPolicy,
    observer: Option<&'a mut dyn MetricWorkObserver>,
    work: u64,
    workspace: u64,
    observed: usize,
    remainder: usize,
    coordinates: u64,
    error: Option<GeoError>,
}
purrdf_hash::debug_non_exhaustive!(ParseAdmission<'_> { policy, work, workspace, coordinates });

impl<'a> ParseAdmission<'a> {
    /// Logical byte work and storage for borrowed source text before interpreting it.
    /// `retained_bytes` includes actual owned capacity. Sixty-three additional
    /// bytes per text byte retain the original carrier's conservative parser,
    /// interpretation and error-copy envelope, without counting spare capacity
    /// as byte work. The caller admits this allowance; no work is charged here.
    ///
    /// # Errors
    /// Refuses checked source-storage overflow.
    pub fn text_allowance(bytes: u64, retained_bytes: u64) -> Result<(u64, u64), GeoError> {
        let storage = bytes
            .checked_mul(63)
            .and_then(|copies| copies.checked_add(retained_bytes))
            .ok_or(GeoError::ArithmeticOverflow(
                "carrier source text allowance",
            ))?;
        Ok((bytes.div_ceil(16), storage))
    }

    /// Reserve complete source storage before parsing and charge the entry chunk.
    /// # Errors
    /// Refuses checked work/storage exhaustion or the observer's cancellation.
    pub fn new(
        bytes: usize,
        policy: ExecutionPolicy,
        observer: Option<&'a mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        let bytes = u64::try_from(bytes)
            .map_err(|_| GeoError::ArithmeticOverflow("carrier source admission"))?;
        let workspace = Self::text_allowance(bytes, bytes)?
            .1
            .checked_add(1024)
            .ok_or(GeoError::ArithmeticOverflow("carrier source admission"))?;
        if workspace > policy.limits().max_workspace_bytes {
            return Err(GeoError::MemoryExhausted {
                limit: policy.limits().max_workspace_bytes,
            });
        }
        let mut admission = Self {
            policy,
            observer,
            work: 0,
            workspace,
            observed: 0,
            remainder: 0,
            coordinates: 0,
            error: None,
        };
        admission.charge(1, workspace)?;
        Ok(admission)
    }

    /// Actual already charged input and decimal-construction receipt.
    #[must_use]
    pub fn receipt(&self) -> ParseReceipt {
        ParseReceipt {
            work: self.work,
            workspace: self.workspace,
        }
    }
    /// Take the original latched operational cause after a reader was interrupted.
    pub fn take_error(&mut self) -> Option<GeoError> {
        self.error.take()
    }

    fn charge(&mut self, work: u64, growth: u64) -> Result<(), GeoError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        let result = (|| {
            let limit = self.policy.limits().max_work_items;
            let total = self
                .work
                .checked_add(work)
                .ok_or(GeoError::WorkExhausted { limit })?;
            if total > limit {
                return Err(GeoError::WorkExhausted { limit });
            }
            self.work = total;
            if let Some(observer) = self.observer.as_deref_mut() {
                observer.charge_chunk(work, growth)?;
            }
            Ok(())
        })();
        if let Err(error) = &result {
            self.error = Some(error.clone());
        }
        result
    }

    pub(crate) fn advance_to(&mut self, offset: usize) -> bool {
        let bytes = offset.saturating_sub(self.observed);
        self.observed = self.observed.max(offset);
        self.advance(bytes)
    }
    pub(crate) fn advance(&mut self, bytes: usize) -> bool {
        let Some(total) = self.remainder.checked_add(bytes) else {
            self.error = Some(GeoError::ArithmeticOverflow("carrier byte work"));
            return false;
        };
        self.remainder = total % 16;
        self.charge((total / 16) as u64, 0).is_ok()
    }

    pub(crate) fn coordinate(&mut self) -> Result<(), GeoError> {
        let limit = self.policy.limits().max_output_elements;
        self.coordinates = self
            .coordinates
            .checked_add(1)
            .ok_or(GeoError::OutputExhausted { limit })?;
        if self.coordinates > limit {
            let error = GeoError::OutputExhausted { limit };
            self.error = Some(error.clone());
            return Err(error);
        }
        self.charge(1, 0)
    }

    /// Admit a bounded exact source or record walk before processing its items.
    /// # Errors
    /// Refuses exhausted work or the original observer's cancellation.
    pub fn admit_items(&mut self, items: u64) -> Result<(), GeoError> {
        self.charge(items, 0)
    }

    /// Admit additional live source ownership before copying or inspecting it.
    /// The existing byte parser allowance can already cover part of the source.
    pub(crate) fn retain_source(&mut self, bytes: u64) -> Result<(), GeoError> {
        let limit = self.policy.limits().max_workspace_bytes;
        let workspace = self
            .workspace
            .checked_add(bytes)
            .filter(|workspace| *workspace <= limit)
            .ok_or(GeoError::MemoryExhausted { limit })?;
        self.charge(0, bytes)?;
        self.workspace = workspace;
        Ok(())
    }

    /// Admit an original exact decimal spelling without constructing its value.
    /// Non-decimal strings do not perform numerical construction.
    /// # Errors
    /// Refuses work/storage exhaustion before integer or power allocation.
    pub fn admit_decimal_lexeme(&mut self, text: &str) -> Result<(), GeoError> {
        if let Some((digits, power)) = Rat::decimal_construction(text) {
            self.admit_construction(digits, power)?;
        }
        Ok(())
    }

    fn admit_construction(&mut self, digits: usize, power: u64) -> Result<(), GeoError> {
        let cost = purrdf_xsd::integer::ExactArithmeticCost::decimal(digits, power).ok_or(
            GeoError::ArithmeticOverflow("decimal construction admission"),
        )?;
        let work = cost.work_items;
        let growth = cost.workspace_bytes;
        let workspace =
            self.workspace
                .checked_add(growth)
                .ok_or_else(|| GeoError::MemoryExhausted {
                    limit: self.policy.limits().max_workspace_bytes,
                })?;
        if workspace > self.policy.limits().max_workspace_bytes {
            let error = GeoError::MemoryExhausted {
                limit: self.policy.limits().max_workspace_bytes,
            };
            self.error = Some(error.clone());
            return Err(error);
        }
        self.workspace = workspace;
        self.charge(work, growth)
    }

    pub(crate) fn decimal(&mut self, text: &str) -> Result<Option<Rat>, GeoError> {
        Rat::parse_decimal_admitted(text, &mut |digits, power| {
            self.admit_construction(digits, power)
        })
    }
}

impl purrdf_lex::json::ReadObserver for ParseAdmission<'_> {
    fn advance(&mut self, bytes: usize) -> bool {
        self.advance(bytes)
    }
}

pub(crate) struct JsonProgress<'a, 'observer>(
    pub(crate) &'a std::cell::RefCell<ParseAdmission<'observer>>,
);
impl purrdf_lex::json::ReadObserver for JsonProgress<'_, '_> {
    fn advance(&mut self, bytes: usize) -> bool {
        self.0.borrow_mut().advance(bytes)
    }
}

pub(crate) fn whitespace_only(
    lexical: &str,
    admission: Option<&std::rc::Rc<std::cell::RefCell<ParseAdmission<'_>>>>,
) -> Result<bool, GeoError> {
    for (index, chunk) in lexical.as_bytes().chunks(256).enumerate() {
        if let Some(admission) = admission {
            let mut admission = admission.borrow_mut();
            if !admission.advance_to((index * 256 + chunk.len()).min(lexical.len())) {
                return Err(admission
                    .take_error()
                    .expect("refused carrier input has a cause"));
            }
        }
        if !chunk.iter().copied().all(purrdf_iri::terminals::is_ws) {
            return Ok(false);
        }
    }
    Ok(true)
}
