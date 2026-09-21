// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A **prepared, parameterized execution**: prepare once, bind and run many times.
//!
//! [`PlanCache`](crate::engine::PlanCache)'s own documentation already tells callers
//! to *"pass changing data as substitutions to a prepared plan instead of splicing it
//! into query text"* — because splicing a value into text mints a new query, and a
//! new query misses the plan cache, is re-parsed, and is re-admitted. Until now there
//! was no object to follow that advice with: every entry point took the query as a
//! `&str`, so a caller running one query per row paid a cache probe per row to be
//! handed back the same plan each time.
//!
//! A [`PreparedExecution`] is that object. It holds the admitted plan, so the text is
//! parsed and admitted once; it holds its parameters as already-interned
//! [`Variable`]s, so a name is never rebuilt from a borrow; and it holds their
//! current values in a vector it overwrites, so binding a new row writes cells rather
//! than building a list.
//!
//! # Why `&mut self` on the run, and why that is a guarantee rather than a limitation
//!
//! [`NativeSparqlEngine::execute`](crate::NativeSparqlEngine::execute) takes the
//! execution by unique reference. That is deliberate and load-bearing: it makes an
//! execution that is already in flight **impossible to reach again**, as a
//! borrow-check error rather than as a runtime hazard.
//!
//! The hazard is real and not hypothetical. A SHACL-AF function body can be a query
//! whose evaluation re-enters validation and reaches the same evaluator — the shapes
//! crate keeps a call-depth counter precisely because "a recursion can LEAVE this
//! evaluator and come back". A handle reachable from an ambient cache would hand that
//! inner call the very tree the outer call is mid-evaluation over. `&mut` means the
//! compiler refuses that program.
//!
//! The consequence for callers is that a handle belongs to one worker at a time. That
//! is the natural shape anyway: the engine it was prepared against is itself `!Sync`.

use std::sync::Arc;

use purrdf_core::{RdfDiagnostic, TermValue};
use purrdf_sparql_algebra::Variable;

use crate::engine::PreparedQuery;

/// A plan prepared once, with named parameters bound and re-bound per execution.
///
/// Build one with
/// [`NativeSparqlEngine::prepare_execution`](crate::NativeSparqlEngine::prepare_execution)
/// and run it with
/// [`NativeSparqlEngine::execute`](crate::NativeSparqlEngine::execute).
#[derive(Debug)]
pub struct PreparedExecution {
    /// The admitted plan. Held by `Arc`, so it stays valid for this execution's
    /// lifetime even if the engine's cache evicts it.
    pub(crate) prepared: Arc<PreparedQuery>,
    /// The declared parameters, interned once, in declaration order.
    pub(crate) parameters: Box<[Variable]>,
    /// The current value of each parameter, positionally. `None` until bound;
    /// running with any parameter still `None` is refused rather than defaulted.
    pub(crate) values: Vec<Option<TermValue>>,
}

impl PreparedExecution {
    /// The declared parameters, in declaration order.
    #[must_use]
    pub fn parameters(&self) -> &[Variable] {
        &self.parameters
    }

    /// The plan this execution runs.
    #[must_use]
    pub fn plan(&self) -> &PreparedQuery {
        &self.prepared
    }

    /// The slot of the parameter named `name`, if it was declared.
    #[must_use]
    pub fn slot(&self, name: &str) -> Option<usize> {
        self.parameters
            .iter()
            .position(|parameter| parameter.as_str() == name)
    }

    /// Bind the parameter in `slot` to `value`.
    ///
    /// # Errors
    ///
    /// [`RdfDiagnostic`] if `slot` is not a declared parameter. An out-of-range slot
    /// is a caller mistake about the query's own shape, not a value this execution
    /// could reasonably answer for, so it is refused rather than ignored — the same
    /// rule the pre-binding rewrite applies to a value it cannot ground.
    pub fn bind(&mut self, slot: usize, value: TermValue) -> Result<(), RdfDiagnostic> {
        let Some(cell) = self.values.get_mut(slot) else {
            return Err(RdfDiagnostic::error(
                "native-sparql-execution-parameter",
                format!(
                    "no parameter in slot {slot}: this execution declares {}",
                    self.parameter_list()
                ),
            ));
        };
        *cell = Some(value);
        Ok(())
    }

    /// Bind the parameter called `name` to `value`.
    ///
    /// # Errors
    ///
    /// [`RdfDiagnostic`] if no parameter of that name was declared. Silently dropping
    /// an unknown name would leave the parameter it was meant for at its previous
    /// value and answer a query nobody asked, which is the shape of a silent wrong
    /// answer rather than of a lenient API.
    pub fn bind_named(&mut self, name: &str, value: TermValue) -> Result<(), RdfDiagnostic> {
        let Some(slot) = self.slot(name) else {
            return Err(RdfDiagnostic::error(
                "native-sparql-execution-parameter",
                format!(
                    "no parameter named {name:?}: this execution declares {}",
                    self.parameter_list()
                ),
            ));
        };
        self.values[slot] = Some(value);
        Ok(())
    }

    /// The declared parameters, for a diagnostic.
    fn parameter_list(&self) -> String {
        if self.parameters.is_empty() {
            return "none".to_owned();
        }
        self.parameters
            .iter()
            .map(|parameter| format!("?{}", parameter.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The names of every parameter still unbound.
    pub(crate) fn unbound(&self) -> Vec<&str> {
        self.parameters
            .iter()
            .zip(&self.values)
            .filter(|(_, value)| value.is_none())
            .map(|(parameter, _)| parameter.as_str())
            .collect()
    }

    /// The current bindings, as the rewrite consumes them: parallel slices, so
    /// running costs no list to build.
    ///
    /// Only valid once every parameter is bound, which
    /// [`NativeSparqlEngine::execute`](crate::NativeSparqlEngine::execute) checks
    /// before it calls this.
    pub(crate) fn prebindings(&self) -> crate::substitute::Prebindings<'_> {
        crate::substitute::Prebindings::Paired(&self.parameters, &self.values)
    }
}
