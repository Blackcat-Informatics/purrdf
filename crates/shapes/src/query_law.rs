// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Execution-qualified SHACL query admission, with exact invocation causes.
//!
//! REC declaration admission remains in the parser. A runtime audit observes
//! the original query and its actual source role before substitution; AF/WD
//! bodies that are never invoked do not acquire an execution refusal.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use purrdf_sparql_algebra::Query;
use purrdf_sparql_eval::{UserFunction, UserFunctionAdmission, UserFunctionRefusal};

use crate::profile::{AdmissionRefusal, QueryPurpose, ShaclProfile};

/// The source role and all declared parameters supplied by the actual caller.
/// Supplied values do not narrow this set when optional arguments are absent.
#[derive(Clone, Copy)]
pub(crate) struct Invocation<'a> {
    pub(crate) purpose: QueryPurpose,
    parameters: Parameters<'a>,
}

#[derive(Clone, Copy)]
enum Parameters<'a> {
    Names(&'a [&'a str]),
    Declarations(&'a [String]),
    Bindings(&'a [(String, crate::term::Term)]),
}

impl<'a> Parameters<'a> {
    fn names(self) -> impl ExactSizeIterator<Item = &'a str> {
        let length = match self {
            Self::Names(names) => names.len(),
            Self::Declarations(names) => names.len(),
            Self::Bindings(bindings) => bindings.len(),
        };
        (0..length).map(move |index| match self {
            Self::Names(names) => names[index],
            Self::Declarations(names) => names[index].as_str(),
            Self::Bindings(bindings) => bindings[index].0.as_str(),
        })
    }
}

impl<'a> Invocation<'a> {
    pub(crate) const fn without_parameters(purpose: QueryPurpose) -> Self {
        Self {
            purpose,
            parameters: Parameters::Names(&[]),
        }
    }

    pub(crate) const fn with_names(purpose: QueryPurpose, parameters: &'a [&'a str]) -> Self {
        Self {
            purpose,
            parameters: Parameters::Names(parameters),
        }
    }

    pub(crate) const fn with_declarations(purpose: QueryPurpose, parameters: &'a [String]) -> Self {
        Self {
            purpose,
            parameters: Parameters::Declarations(parameters),
        }
    }

    pub(crate) const fn with_bindings(
        purpose: QueryPurpose,
        parameters: &'a [(String, crate::term::Term)],
    ) -> Self {
        Self {
            purpose,
            parameters: Parameters::Bindings(parameters),
        }
    }
}

/// One operation or root-focus traversal. Workers at distinct roots receive
/// distinct cause slots; an internal evaluator carries a refusal in its error
/// until canonical worker reduction selects the authoritative cause.
#[derive(Debug)]
pub(crate) struct Runtime {
    profile: ShaclProfile,
    sources: Option<Arc<crate::shapes::ConstraintSources>>,
    failure: Mutex<Option<crate::report::ReportFailure>>,
}

impl Runtime {
    /// The dated law this runtime applies.
    pub(crate) const fn profile(&self) -> ShaclProfile {
        self.profile
    }

    pub(crate) fn admit(&self, invocation: Invocation<'_>, query: &Query) -> Result<(), String> {
        crate::prebinding::admit(
            self.profile,
            invocation.purpose,
            query,
            invocation.parameters.names(),
        )
        .map_err(|refusal| {
            let message = refusal.to_string();
            self.record(refusal);
            message
        })
    }

    fn record(&self, refusal: AdmissionRefusal) {
        self.record_failure(crate::report::ReportFailure::Admission(refusal));
    }

    fn record_failure(&self, refusal: crate::report::ReportFailure) {
        self.failure
            .lock()
            .expect("query admission failure lock")
            .get_or_insert(refusal);
    }

    pub(crate) fn take_failure(&self) -> Option<crate::report::ReportFailure> {
        self.failure
            .lock()
            .expect("query admission failure lock")
            .take()
    }

    pub(crate) fn function_admission(self: &Arc<Self>) -> Arc<dyn UserFunctionAdmission> {
        Arc::clone(self) as Arc<dyn UserFunctionAdmission>
    }

    fn refuse_source(
        &self,
        purpose: Option<QueryPurpose>,
        source: &crate::term::Term,
        index: Option<usize>,
    ) -> String {
        let refusal =
            crate::report::QuerySourceRefusal::new(self.profile, purpose, source.clone(), index);
        let message = refusal.to_string();
        self.record_failure(crate::report::ReportFailure::QuerySource(refusal));
        message
    }

    /// Look up one actual target ordinal, then validate its complete public
    /// model against that occurrence. Equal query text is never a lookup key.
    pub(crate) fn target_invocation(
        &self,
        shape: &crate::term::Term,
        index: usize,
        target: &crate::shapes::Target,
    ) -> Result<Invocation<'_>, String> {
        let occurrence = self
            .sources
            .as_ref()
            .and_then(|sources| sources.targets.get(shape)?.get(&index))
            .ok_or_else(|| self.refuse_source(None, shape, Some(index)))?;
        let matches = matches!(target,
            crate::shapes::Target::Sparql { select, ask, substitutions }
            if occurrence.select == *select
                && occurrence.ask == *ask
                && occurrence.substitutions == *substitutions
        );
        if !matches {
            return Err(self.refuse_source(
                Some(occurrence.purpose),
                &occurrence.source_target,
                Some(index),
            ));
        }
        Ok(Invocation::with_declarations(
            occurrence.purpose,
            &occurrence.parameters,
        ))
    }

    pub(crate) fn component_invocation(
        &self,
        site: crate::report::ConstraintSite<'_>,
        constraint: &crate::plan::PlannedConstraint<'_>,
        purpose: QueryPurpose,
    ) -> Result<Invocation<'_>, String> {
        let occurrence =
            crate::report::admit_component_occurrence(self.sources.as_deref(), site, constraint)
                .ok_or_else(|| self.refuse_source(Some(purpose), site.shape, site.index))?;
        Ok(Invocation::with_declarations(
            purpose,
            &occurrence.parameters,
        ))
    }

    /// A linked shape occurrence and a global occurrence of the same rule node
    /// have distinct source keys and potentially distinct initial bindings.
    pub(crate) fn rule_invocation(
        &self,
        rule: &crate::rules::Rule,
        shape: Option<&crate::term::Term>,
    ) -> Result<Invocation<'_>, String> {
        let purpose = if shape.is_some() {
            QueryPurpose::ConstructRule
        } else {
            QueryPurpose::GlobalConstructRule
        };
        let occurrence = self
            .sources
            .as_ref()
            .and_then(|sources| {
                let uses = sources.rules.get(&rule.id)?;
                match shape {
                    Some(shape) => uses.linked.get(shape),
                    None => uses.global.as_ref(),
                }
            })
            .filter(|occurrence| {
                let crate::rules::RuleBody::Sparql {
                    construct,
                    parameters,
                } = &rule.body
                else {
                    return false;
                };
                occurrence.purpose == purpose
                    && occurrence.construct == *construct
                    && occurrence.substitutions == *parameters
            })
            .ok_or_else(|| self.refuse_source(Some(purpose), &rule.id, None))?;
        Ok(Invocation::with_declarations(
            purpose,
            &occurrence.parameters,
        ))
    }
}

impl UserFunctionAdmission for Runtime {
    fn admit(&self, function: &UserFunction, query: &Query) -> Result<(), UserFunctionRefusal> {
        crate::prebinding::admit(
            self.profile,
            QueryPurpose::Function,
            query,
            function
                .params
                .iter()
                .map(|parameter| parameter.var.as_str()),
        )
        .map_err(UserFunctionRefusal::new)
    }

    fn observe_refusal(&self, refusal: &UserFunctionRefusal) {
        let cause = refusal
            .cause()
            .downcast_ref::<AdmissionRefusal>()
            .expect("a SHACL query law retains its exact admission refusal");
        self.record(cause.clone());
    }
}

std::thread_local! {
    static CURRENT: RefCell<Option<Arc<Runtime>>> = const { RefCell::new(None) };
}

pub(crate) fn current() -> Option<Arc<Runtime>> {
    CURRENT.with(|slot| slot.borrow().clone())
}

pub(crate) fn replace(next: Option<Arc<Runtime>>) -> Option<Arc<Runtime>> {
    CURRENT.with(|slot| slot.replace(next))
}

/// A dated scope owns one exact cause channel. Compatibility has no runtime
/// owner and allocates nothing; nested scopes restore the prior request law.
pub(crate) struct Scope {
    pub(crate) runtime: Option<Arc<Runtime>>,
    previous: Option<Arc<Runtime>>,
    _not_send: PhantomData<Rc<()>>,
}

pub(crate) fn enter(
    profile: ShaclProfile,
    sources: Option<&Arc<crate::shapes::ConstraintSources>>,
) -> Scope {
    let runtime = (profile != ShaclProfile::LEGACY).then(|| {
        Arc::new(Runtime {
            profile,
            sources: sources.cloned(),
            failure: Mutex::new(None),
        })
    });
    let previous = replace(runtime.clone());
    Scope {
        runtime,
        previous,
        _not_send: PhantomData,
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        let _ = replace(self.previous.take());
    }
}
