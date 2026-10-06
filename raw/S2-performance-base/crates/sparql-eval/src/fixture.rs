// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Relations the property-function tests of this crate and of the crates above it
//! register.
//!
//! This is test support, not API: it is hidden from the documentation and carries no
//! stability promise. It lives in the library because the evaluator's own tests and
//! the SHACL engine's tests pin the same relation behaviour at two layers, and a
//! test target can share code with another crate's tests only through a library
//! both reach.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    BindingPattern, EvalError, IndexGeneration, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction,
    ServiceLevel, Volatility,
};

/// A stable relation of arity one-to-one over ONE fixed row, in the all-free mode,
/// counting how many times the engine opened it; its cursor declares `generation`
/// and `service_level`.
#[derive(Debug)]
pub struct OneRowRelation {
    modes: [BindingPattern; 1],
    row: PfRow,
    generation: &'static str,
    service_level: ServiceLevel,
    opens: Arc<AtomicU64>,
}

impl OneRowRelation {
    /// The relation answering `row`, its cursor declaring `generation`, each open
    /// counted into `opens`. Its service level is undeclared.
    #[must_use]
    pub fn new(row: PfRow, generation: &'static str, opens: Arc<AtomicU64>) -> Self {
        Self {
            modes: [BindingPattern::from_code("ff")],
            row,
            generation,
            service_level: ServiceLevel::Undeclared,
            opens,
        }
    }

    /// The same relation, its cursor declaring `service_level`.
    #[must_use]
    pub fn with_service_level(self, service_level: ServiceLevel) -> Self {
        Self {
            service_level,
            ..self
        }
    }
}

/// The cursor an [`OneRowRelation`] opens.
#[derive(Debug)]
struct OneRowCursor {
    rows: std::vec::IntoIter<PfRow>,
    generation: &'static str,
    service_level: ServiceLevel,
}

impl PfCursor for OneRowCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.rows.next())
    }

    fn generation(&self) -> IndexGeneration {
        IndexGeneration::declared(self.generation)
    }

    fn service_level(&self) -> ServiceLevel {
        self.service_level.clone()
    }
}

impl PropertyFunction for OneRowRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        1
    }

    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        self.opens.fetch_add(1, Ordering::Relaxed);
        Ok(Box::new(OneRowCursor {
            rows: vec![self.row.clone()].into_iter(),
            generation: self.generation,
            service_level: self.service_level.clone(),
        }))
    }
}
