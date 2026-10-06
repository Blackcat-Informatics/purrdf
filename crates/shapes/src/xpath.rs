// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Dated native XPath execution over immutable SHACL preparations.

mod declarations;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

use std::cell::RefCell;
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};

use purrdf_core::xsd_regex::xpath::{self, Limits, PatternCache, Profile};
use purrdf_core::{FastMap, RdfDataset, RdfDiagnostic};

use crate::ShapesError;
use crate::data::ShaclData;
use crate::engine::{
    FocusExpansion, FocusId, GovernedValidation, PreparedShapes, PreparedValidator,
};
use crate::report::ValidationReport;
use crate::shapes::Shapes;
use crate::term::Term;

/// An operational refusal or an existing SHACL execution error, without a partial report.
#[derive(Debug)]
#[non_exhaustive]
pub enum XPathValidationError {
    /// Existing shape, projection, source or target admission failed.
    Shapes(ShapesError),
    /// A selected dated SHACL bundle refused source or query admission.
    Complete(Box<crate::report::CompleteValidationError>),
    /// The native pattern compiler or matcher refused the request.
    Pattern(xpath::Error),
    /// A SHACL-driven SPARQL query returned its actual diagnostic.
    Query(RdfDiagnostic),
    /// Another existing constraint execution failed.
    Execution(String),
    /// A declared model exceeded the existing preparation traversal's bound.
    Preparation(crate::product::ShapesProductError),
    /// An earlier panic poisoned the preparation's native program cache.
    CachePoisoned,
}

purrdf_lex::variant_from!(XPathValidationError {
    Shapes(ShapesError), Pattern(xpath::Error), Query(RdfDiagnostic), Execution(String),
    Preparation(crate::product::ShapesProductError)
});

impl fmt::Display for XPathValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shapes(error) => error.fmt(f),
            Self::Complete(error) => error.fmt(f),
            Self::Pattern(error) => error.fmt(f),
            Self::Query(error) => error.fmt(f),
            Self::Execution(error) => f.write_str(error),
            Self::Preparation(error) => error.fmt(f),
            Self::CachePoisoned => f.write_str("native SHACL pattern cache is poisoned"),
        }
    }
}

impl std::error::Error for XPathValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Shapes(error) => Some(error),
            Self::Complete(error) => Some(error.as_ref()),
            Self::Pattern(error) => Some(error),
            Self::Query(error) => Some(error),
            Self::Preparation(error) => Some(error),
            Self::Execution(_) | Self::CachePoisoned => None,
        }
    }
}

pub(crate) type Caches = Arc<Mutex<CacheSlots>>;

/// Only cells owned by an immutable declaration are retained as cache keys.
#[derive(Debug, Default)]
pub(crate) struct CacheSlots {
    declared: Option<FastMap<usize, PatternCache>>,
}

impl CacheSlots {
    fn admit_inventory(
        &mut self,
        declarations: &[declarations::Declaration<'_>],
    ) -> Result<&mut FastMap<usize, PatternCache>, Cause> {
        if self.declared.is_none() {
            let mut slots = FastMap::default();
            for declaration in declarations {
                if !slots.contains_key(&declaration.cell) {
                    slots.try_reserve(1).map_err(|_| {
                        Cause::Pattern(xpath::Error::Allocation {
                            resource: xpath::Resource::CompileSlots,
                            units: 1,
                        })
                    })?;
                    slots.insert(declaration.cell, PatternCache::default());
                }
            }
            self.declared = Some(slots);
        }
        Ok(self
            .declared
            .as_mut()
            .expect("successful inventory is retained"))
    }
}

/// Cache addresses are meaningful only while their exact declaration owner lives.
#[derive(Clone, Debug)]
pub(crate) struct Configuration {
    pub(crate) profile: Profile,
    pub(crate) limits: Limits,
    pub(crate) shapes: Arc<Shapes>,
    pub(crate) caches: Caches,
}

impl Configuration {
    /// A selection over `shapes` with caches of its own, for a run that has no
    /// preparation to share them with: a rules run or one node-expression evaluation.
    fn detached(profile: Profile, limits: Limits, shapes: Arc<Shapes>) -> Self {
        Self {
            profile,
            limits,
            shapes,
            caches: Caches::default(),
        }
    }

    fn compiled(
        &self,
        cell: usize,
        pattern: &str,
        flags: &str,
    ) -> Result<Arc<xpath::CompiledPattern>, Cause> {
        let mut caches = self.caches.lock().map_err(|_| Cause::CachePoisoned)?;
        if caches.declared.is_none() {
            caches.admit_inventory(&declarations::collect(&self.shapes)?)?;
        }
        if let Some(cache) = caches
            .declared
            .as_mut()
            .and_then(|slots| slots.get_mut(&cell))
        {
            return cache
                .compiled_shared(self.profile, pattern, flags, self.limits)
                .map_err(Cause::Pattern);
        }
        // A runtime-created expression may carry a temporary pattern cell.
        // It cannot extend this preparation's declaration ownership domain.
        drop(caches);
        xpath::compile(self.profile, pattern, flags, self.limits)
            .map(Arc::new)
            .map_err(Cause::Pattern)
    }

    /// Source well-formedness admission, independent of data targets or findings.
    pub(crate) fn admit_declared_patterns(&self) -> Result<(), XPathValidationError> {
        let operation = || {
            let declarations = declarations::collect(&self.shapes)?;
            let mut caches = self.caches.lock().map_err(|_| Cause::CachePoisoned)?;
            let slots = caches.admit_inventory(&declarations)?;
            for declaration in declarations {
                if let Some(cache) = slots.get_mut(&declaration.cell) {
                    cache
                        .compiled_shared(
                            self.profile,
                            declaration.pattern,
                            declaration.flags,
                            self.limits,
                        )
                        .map_err(Cause::Pattern)?;
                } else {
                    // Public model handles can be filled after a preparation's
                    // first snapshot. Admit the new source without growing the
                    // snapshot's cache or assuming it already has a slot.
                    xpath::compile(
                        self.profile,
                        declaration.pattern,
                        declaration.flags,
                        self.limits,
                    )
                    .map_err(Cause::Pattern)?;
                }
            }
            drop(caches);
            Ok(())
        };
        operation().map_err(Cause::into_public)
    }

    /// Preserve the caller's own outer error separately from the exact native cause.
    pub(crate) fn run<T, E>(
        &self,
        operation: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, RunError<E>> {
        let _scope = enter(self.clone());
        let result = operation();
        match take_cause() {
            Some(cause) => Err(RunError::Cause {
                cause,
                execution: result.err().map(Box::new),
            }),
            None => result.map_err(RunError::Execution),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Cause {
    Pattern(xpath::Error),
    Query(RdfDiagnostic),
    Preparation(crate::product::ShapesProductError),
    CachePoisoned,
}

purrdf_lex::variant_from!(Cause { Preparation(crate::product::ShapesProductError) });

impl Cause {
    pub(crate) fn into_public(self) -> XPathValidationError {
        match self {
            Self::Pattern(error) => XPathValidationError::Pattern(error),
            Self::Query(error) => XPathValidationError::Query(error),
            Self::Preparation(error) => XPathValidationError::Preparation(error),
            Self::CachePoisoned => XPathValidationError::CachePoisoned,
        }
    }
}

#[derive(Debug)]
pub(crate) enum RunError<E> {
    Cause {
        cause: Cause,
        // The paired caller payload is allocated only on this error path.
        execution: Option<Box<E>>,
    },
    Execution(E),
}

impl<E> RunError<E> {
    /// Let the caller resolve its actual typed payload and associated native cause.
    pub(crate) fn into_parts(self) -> (Option<Cause>, Option<E>) {
        match self {
            Self::Cause { cause, execution } => (Some(cause), execution.map(|error| *error)),
            Self::Execution(error) => (None, Some(error)),
        }
    }

    fn into_public(self, map: impl FnOnce(E) -> XPathValidationError) -> XPathValidationError {
        match self.into_parts() {
            (Some(cause), _) => cause.into_public(),
            (None, Some(error)) => map(error),
            (None, None) => unreachable!("a refused run retains at least one actual error"),
        }
    }
}

impl RunError<crate::report::CompleteValidationError> {
    fn into_selected(self, profile: crate::profile::ShaclProfile) -> XPathValidationError {
        if profile == crate::profile::ShaclProfile::LEGACY {
            self.into_public(crate::report::CompleteValidationError::into_xpath)
        } else {
            crate::report::CompleteValidationError::from_native_run(self).into_xpath()
        }
    }
}

#[derive(Debug)]
pub(crate) struct Runtime {
    configuration: Configuration,
    cause: Option<Cause>,
}

thread_local! {
    static CURRENT: RefCell<Option<Runtime>> = const { RefCell::new(None) };
}

#[must_use]
pub(crate) struct Scope {
    previous: Option<Runtime>,
    _not_send: PhantomData<Rc<()>>,
}

impl Drop for Scope {
    fn drop(&mut self) {
        let _ = replace(self.previous.take());
    }
}

pub(crate) fn enter(configuration: Configuration) -> Scope {
    Scope {
        previous: replace(Some(Runtime {
            configuration,
            cause: None,
        })),
        _not_send: PhantomData,
    }
}

pub(crate) fn replace(next: Option<Runtime>) -> Option<Runtime> {
    CURRENT.with(|slot| slot.replace(next))
}

pub(crate) fn current() -> Option<Configuration> {
    CURRENT.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|runtime| runtime.configuration.clone())
    })
}

pub(crate) fn take_cause() -> Option<Cause> {
    CURRENT.with(|slot| {
        slot.borrow_mut()
            .as_mut()
            .and_then(|runtime| runtime.cause.take())
    })
}

pub(crate) fn capture(cause: Cause) {
    CURRENT.with(|slot| {
        if let Some(runtime) = slot.borrow_mut().as_mut()
            && runtime.cause.is_none()
        {
            runtime.cause = Some(cause);
        }
    });
}

pub(crate) fn query_error(error: RdfDiagnostic) -> String {
    let message = format!("query evaluation error: {error}");
    if purrdf_sparql_eval::EvalError::diagnostic_requires_propagation(&error.code) {
        capture(Cause::Query(error));
    }
    message
}

/// An error travels with its root through the indexed chunk reduction.
#[derive(Debug)]
pub(crate) struct RootError {
    message: String,
    cause: Option<Box<Cause>>,
}

pub(crate) fn root_result<T>(result: Result<T, String>) -> Result<T, RootError> {
    let cause = take_cause();
    match (result, cause) {
        (Ok(value), None) => Ok(value),
        (Err(message), cause) => Err(RootError {
            message,
            cause: cause.map(Box::new),
        }),
        (Ok(_), Some(cause)) => Err(RootError {
            message: cause.clone().into_public().to_string(),
            cause: Some(Box::new(cause)),
        }),
    }
}

pub(crate) fn restore_root_error(error: RootError) -> String {
    if let Some(cause) = error.cause {
        capture(*cause);
    }
    error.message
}

pub(crate) type LegacyCell = OnceLock<
    Result<purrdf_core::xsd_regex::CompiledPattern, purrdf_core::xsd_regex::XsdRegexError>,
>;

/// Both pattern laws use the existing per-value result and message emission.
pub(crate) enum Facet<'a> {
    Legacy(
        &'a Result<purrdf_core::xsd_regex::CompiledPattern, purrdf_core::xsd_regex::XsdRegexError>,
    ),
    Native(Arc<xpath::CompiledPattern>, Limits),
    Invalid(String),
}

impl Facet<'_> {
    pub(crate) fn compile_error(&self) -> Option<String> {
        match self {
            Self::Legacy(program) => program.as_ref().err().map(ToString::to_string),
            Self::Invalid(error) => Some(error.clone()),
            Self::Native(_, _) => None,
        }
    }

    pub(crate) fn matches(&self, lexical: Option<&str>) -> Result<bool, String> {
        let Some(lexical) = lexical else {
            return Ok(false);
        };
        match self {
            Self::Legacy(program) => Ok(program
                .as_ref()
                .is_ok_and(|program| program.is_match(lexical))),
            Self::Invalid(_) => Ok(false),
            Self::Native(program, limits) => program.is_match(lexical, *limits).map_err(|error| {
                let message = error.to_string();
                capture(Cause::Pattern(error));
                message
            }),
        }
    }
}

pub(crate) fn facet<'a>(
    cell: &'a LegacyCell,
    pattern: &str,
    flags: Option<&str>,
) -> Result<Facet<'a>, String> {
    let Some(configuration) = current() else {
        return Ok(Facet::Legacy(
            cell.get_or_init(|| crate::constraints::build_regex(pattern, flags)),
        ));
    };
    match configuration.compiled(
        std::ptr::from_ref(cell).addr(),
        pattern,
        flags.unwrap_or_default(),
    ) {
        Ok(program) => Ok(Facet::Native(program, configuration.limits)),
        Err(Cause::Pattern(error)) if !error.is_operational() => {
            Ok(Facet::Invalid(error.to_string()))
        }
        Err(cause) => {
            let message = match &cause {
                Cause::Pattern(error) => error.to_string(),
                Cause::Query(error) => error.to_string(),
                Cause::Preparation(error) => error.to_string(),
                Cause::CachePoisoned => "native SHACL pattern cache is poisoned".to_owned(),
            };
            capture(cause);
            Err(message)
        }
    }
}

/// A reusable SHACL preparation under an explicit dated native pattern law.
#[derive(Clone, Debug)]
pub struct XPathPreparedShapes {
    prepared: PreparedShapes,
    configuration: Configuration,
}

impl XPathPreparedShapes {
    pub(crate) fn new(prepared: PreparedShapes, profile: Profile, limits: Limits) -> Self {
        let configuration = prepared.xpath_configuration(profile, limits);
        Self {
            prepared,
            configuration,
        }
    }

    /// The immutable preparation and its original provenance.
    #[must_use]
    pub const fn preparation(&self) -> &PreparedShapes {
        &self.prepared
    }

    /// Select another dated law and current limits without reusing target results.
    #[must_use]
    pub fn with_xpath_regex(mut self, profile: Profile, limits: Limits) -> Self {
        self.configuration.profile = profile;
        self.configuration.limits = limits;
        self
    }

    /// The selected law and finite request limits.
    #[must_use]
    pub const fn selection(&self) -> (Profile, Limits) {
        (self.configuration.profile, self.configuration.limits)
    }

    /// Admit every declared pattern under this law before considering data targets.
    ///
    /// This explicit source check also checks patterns on shapes with no targets.
    /// Ordinary validation retains its facet-finding channel for malformed patterns.
    /// Successful programs are shared with later binds and validations.
    ///
    /// # Errors
    /// Returns the actual compiler, storage or model-traversal refusal.
    pub fn admit_declared_patterns(&self) -> Result<(), XPathValidationError> {
        self.configuration.admit_declared_patterns()
    }

    /// Bind and validate under one SPARQL budget, including rules and targets.
    ///
    /// Native pattern work also obeys this selection's independent finite limits.
    /// A governor trip returns its typed outcome without a partial report.
    ///
    /// # Errors
    /// Returns actual source, target, pattern or constraint operational failures.
    pub fn validate_with_governors(
        &self,
        data: ShaclData,
        governors: &purrdf_sparql_eval::QueryGovernors,
    ) -> Result<GovernedValidation, XPathValidationError> {
        crate::engine::governed_validation(governors, || self.bind(data)?.validate())
    }

    /// Project, bind and validate a dataset under one shared SPARQL budget.
    ///
    /// Rule entailment, targets and constraints use this selected native law and limits.
    ///
    /// # Errors
    /// Returns actual projection, target, pattern or constraint operational failures.
    pub fn validate_dataset_with_governors(
        &self,
        data: &RdfDataset,
        governors: &purrdf_sparql_eval::QueryGovernors,
    ) -> Result<GovernedValidation, XPathValidationError> {
        crate::engine::governed_validation(governors, || self.bind_dataset(data)?.validate())
    }

    fn bound(
        &self,
        operation: impl FnOnce(
            &PreparedShapes,
        )
            -> Result<PreparedValidator, crate::report::CompleteValidationError>,
    ) -> Result<XPathPreparedValidator, XPathValidationError> {
        let validator = self
            .configuration
            .run(|| operation(&self.prepared))
            .map_err(|error| {
                error.into_selected(self.prepared.shapes().validation_options.shacl_profile)
            })?;
        Ok(XPathPreparedValidator {
            validator,
            configuration: self.configuration.clone(),
        })
    }

    /// Bind a data holder, rerunning active targets under this selection.
    /// # Errors
    /// Returns typed source, target or native operational failures.
    pub fn bind(&self, data: ShaclData) -> Result<XPathPreparedValidator, XPathValidationError> {
        self.bound(|prepared| prepared.bind_request(data))
    }

    /// Project and bind a dataset under this selection.
    /// # Errors
    /// Returns typed projection, target or native operational failures.
    pub fn bind_dataset(
        &self,
        data: &RdfDataset,
    ) -> Result<XPathPreparedValidator, XPathValidationError> {
        self.bound(|prepared| prepared.bind_dataset_request(data))
    }

    /// Bind a shared native dataset without an owned projection.
    /// # Errors
    /// Returns typed source, target or native operational failures.
    pub fn bind_shared_dataset(
        &self,
        data: Arc<RdfDataset>,
    ) -> Result<XPathPreparedValidator, XPathValidationError> {
        self.bound(|prepared| prepared.bind_shared_dataset_request(data))
    }

    /// Bind an immutable SHACL view under this selection.
    /// # Errors
    /// Returns typed target or native operational failures.
    pub fn bind_view(
        &self,
        data: Arc<crate::data_view::ShaclDatasetView>,
    ) -> Result<XPathPreparedValidator, XPathValidationError> {
        self.bound(|prepared| prepared.bind_view_request(data))
    }

    /// Bind a shared source with its shapes graph exposed to SPARQL.
    /// # Errors
    /// Returns typed retention, projection, target or native operational failures.
    pub fn bind_shared_dataset_with_shapes_graph(
        &self,
        data: Arc<RdfDataset>,
        iri: Option<&str>,
        limits: purrdf_rdf::ir::ViewLimits,
    ) -> Result<XPathPreparedValidator, XPathValidationError> {
        self.bound(|prepared| {
            prepared.bind_shared_dataset_with_shapes_graph_request(data, iri, limits)
        })
    }

    /// Bind an immutable mutation snapshot with its shapes graph.
    /// # Errors
    /// Returns typed retention, target or native operational failures.
    pub fn bind_delta_with_shapes_graph(
        &self,
        data: Arc<purrdf_rdf::ir::DeltaDatasetView>,
        iri: Option<&str>,
        limits: purrdf_rdf::ir::ViewLimits,
    ) -> Result<XPathPreparedValidator, XPathValidationError> {
        self.bound(|prepared| prepared.bind_delta_with_shapes_graph_request(data, iri, limits))
    }

    /// Bind an already-projected dataset under this selection.
    /// # Errors
    /// Returns typed target or native operational failures.
    pub fn bind_projected_dataset(
        &self,
        data: Arc<RdfDataset>,
    ) -> Result<XPathPreparedValidator, XPathValidationError> {
        self.bound(|prepared| prepared.bind_projected_dataset_request(data))
    }

    /// Bind an already-projected dataset with its shapes graph.
    /// # Errors
    /// Returns typed graph assembly, target or native operational failures.
    pub fn bind_projected_dataset_with_shapes_graph(
        &self,
        data: Arc<RdfDataset>,
        iri: Option<&str>,
    ) -> Result<XPathPreparedValidator, XPathValidationError> {
        self.bound(|prepared| prepared.bind_projected_dataset_with_shapes_graph_request(data, iri))
    }
}

/// One immutable data binding retaining the exact selection used for its targets.
#[derive(Debug)]
pub struct XPathPreparedValidator {
    validator: PreparedValidator,
    configuration: Configuration,
}

impl XPathPreparedValidator {
    /// The law and limits used to acquire this binding's target set.
    #[must_use]
    pub const fn selection(&self) -> (Profile, Limits) {
        (self.configuration.profile, self.configuration.limits)
    }

    /// The preparation's original source or prepared-product provenance.
    #[must_use]
    pub fn provenance(&self) -> &crate::provenance::ValidatorProvenance {
        self.validator.provenance()
    }

    /// Operational measurements for the binding's retained Core and SPARQL views.
    #[must_use]
    pub fn view_stats(&self) -> [crate::data_view::ShaclViewStats; 2] {
        self.validator.view_stats()
    }

    /// Validate every prepared target under the binding's original selection.
    /// # Errors
    /// Returns typed operational failures, with no partial report.
    pub fn validate(&self) -> Result<ValidationReport, XPathValidationError> {
        self.configuration
            .run(|| {
                self.validator
                    .validate_request(None::<fn(&crate::shapes::Shape, &Term) -> bool>)
            })
            .map_err(|error| error.into_selected(self.validator.profile()))
    }

    /// Validate this binding under one SPARQL budget and current pattern limits.
    ///
    /// Rule entailment and target acquisition completed when this binding was created.
    /// To budget those phases too, use [`XPathPreparedShapes::validate_with_governors`].
    ///
    /// # Errors
    /// Returns typed operational failures, with no partial report.
    pub fn validate_with_governors(
        &self,
        governors: &purrdf_sparql_eval::QueryGovernors,
    ) -> Result<GovernedValidation, XPathValidationError> {
        crate::engine::governed_validation(governors, || self.validate())
    }

    /// Validate supplied candidate nodes under the binding's original selection.
    /// # Errors
    /// Returns typed target, source or native operational failures.
    pub fn validate_focus_nodes(
        &self,
        nodes: &[Term],
    ) -> Result<ValidationReport, XPathValidationError> {
        self.configuration
            .run(|| {
                self.validator
                    .validate_bounded_request(&self.validator.normalize_focus_nodes(nodes))
            })
            .map_err(|error| error.into_selected(self.validator.profile()))
    }

    /// Validate ids minted by this exact binding.
    /// # Errors
    /// Refuses foreign ids, target failures and native operational failures.
    pub fn validate_focus_node_ids(
        &self,
        nodes: &[FocusId],
    ) -> Result<ValidationReport, XPathValidationError> {
        self.configuration
            .run(|| {
                let focus = self.validator.normalize_focus_node_ids(nodes)?;
                self.validator.validate_bounded_request(&focus)
            })
            .map_err(|error| error.into_selected(self.validator.profile()))
    }

    /// Mint an id from this exact immutable data binding.
    #[must_use]
    pub fn term_id(&self, term: &Term) -> Option<FocusId> {
        self.validator.term_id(term)
    }

    /// Expand the binding's immutable change into its affected focus nodes.
    /// # Errors
    /// Refuses invalid change or target admission and native operational failures.
    pub fn affected_focus_node_ids(
        &self,
        delta: &purrdf_rdf::ir::DeltaDatasetView,
    ) -> Result<FocusExpansion, XPathValidationError> {
        self.configuration
            .run(|| self.validator.affected_focus_request(delta))
            .map_err(|error| error.into_selected(self.validator.profile()))
    }

    /// Expand the binding's immutable change and validate exactly the focus nodes it
    /// can move, under the binding's original selection.
    ///
    /// This is [`crate::engine::validate_change`] — the engine's one change loop,
    /// with its unbounded-footprint fallback to a full validation — run inside this
    /// binding's selection, so the expansion and both validation arms use the same
    /// law and limits. The binding must come from
    /// [`XPathPreparedShapes::bind_delta_with_shapes_graph`] over `delta`.
    ///
    /// # Errors
    /// Refuses a binding not made over `delta`, hard validation failures and native
    /// operational failures, with no partial report.
    pub fn validate_change(
        &self,
        delta: &purrdf_rdf::ir::DeltaDatasetView,
    ) -> Result<crate::engine::ChangeValidation, XPathValidationError> {
        self.configuration
            .run(|| crate::engine::validate_change(&self.validator, delta))
            .map_err(|error| error.into_public(XPathValidationError::Execution))
    }

    /// [`Self::validate_change`] under one SPARQL budget, exactly as
    /// [`crate::engine::validate_change_with_governors`] budgets it. Native pattern
    /// work obeys this selection's independent finite limits and spends no fuel.
    ///
    /// # Errors
    /// As [`Self::validate_change`]; a tripped governor is the returned outcome.
    pub fn validate_change_with_governors(
        &self,
        delta: &purrdf_rdf::ir::DeltaDatasetView,
        governors: &purrdf_sparql_eval::QueryGovernors,
    ) -> Result<crate::engine::GovernedChangeValidation, XPathValidationError> {
        self.configuration
            .run(|| {
                crate::engine::validate_change_with_governors(&self.validator, delta, governors)
            })
            .map_err(|error| error.into_public(XPathValidationError::Execution))
    }
}

/// Validate a native dataset under an explicit dated XPath law and finite limits.
///
/// Preparation, target acquisition and validation use the same selected binding
/// path as [`XPathPreparedShapes`]. Pattern syntax retains ordinary facet findings;
/// [`XPathPreparedShapes::admit_declared_patterns`] offers strict source preflight.
///
/// # Errors
/// Returns actual projection, target, native operational or constraint failures.
pub fn validate_dataset(
    data: &RdfDataset,
    shapes: Arc<Shapes>,
    profile: Profile,
    limits: Limits,
) -> Result<ValidationReport, XPathValidationError> {
    PreparedShapes::new(shapes)
        .with_xpath_regex(profile, limits)
        .bind_dataset(data)?
        .validate()
}

/// Selected native validation with one SPARQL budget for targets and constraints.
///
/// # Errors
/// Returns actual projection, target, native operational or constraint failures.
pub fn validate_dataset_with_governors(
    data: &RdfDataset,
    shapes: Arc<Shapes>,
    profile: Profile,
    limits: Limits,
    governors: &purrdf_sparql_eval::QueryGovernors,
) -> Result<GovernedValidation, XPathValidationError> {
    PreparedShapes::new(shapes)
        .with_xpath_regex(profile, limits)
        .validate_dataset_with_governors(data, governors)
}

/// Parse and validate two text graphs under an explicit dated XPath law and finite limits.
///
/// The same parse [`crate::engine::validate_graphs_with_shapes_graph`] runs — the
/// N-Triples data graph, then the shapes graph read over `imports` with the data graph's
/// `sh:shapesGraph` links folded in and `options` set on it — validated through
/// [`validate_dataset`]. Every host's text validation under a selected law reaches here.
///
/// # Errors
/// A parse or import refusal as [`XPathValidationError::Shapes`], exactly as the
/// compatibility entry refuses it; then everything [`validate_dataset`] refuses.
#[allow(
    clippy::too_many_arguments,
    reason = "the compatibility entry's six inputs and the selected law's two"
)]
pub fn validate_graphs_with_shapes_graph(
    data_nt: &str,
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    shapes_graph: Option<&str>,
    options: &crate::engine::ValidationOptions,
    imports: &crate::imports::ShapesImports,
    profile: Profile,
    limits: Limits,
) -> Result<ValidationReport, XPathValidationError> {
    let (data, shapes) = crate::engine::parse_graphs_with_shapes_graph(
        data_nt,
        shapes_ttl,
        shapes_base,
        shapes_graph,
        options,
        imports,
    )?;
    validate_dataset(data.as_ref(), Arc::new(shapes), profile, limits)
}

/// Run a rule source over `data` under an explicit dated XPath law and finite limits.
///
/// This is [`crate::run_rules`], the one rules dispatch, run inside the selection: every
/// `REGEX`/`REPLACE` a `sh:SPARQLRule`, a SHACL-AF function, a node expression or a SPARQL
/// 1.2 RL filter or assignment evaluates, and every `sh:pattern` a rule condition or
/// filter shape decides, compiles and matches under `profile` within `pattern_limits`.
/// A pattern the law does not define keeps the compatibility run's behaviour for an
/// ill-formed pattern (an expression error, a nonconforming value); a resource the law
/// withholds aborts the run with no inference.
///
/// # Errors
/// A refusal of the compatibility run, as [`XPathValidationError::Shapes`] carrying its
/// text; a native pattern refusal as [`XPathValidationError::Pattern`], and a rule query's
/// operational diagnostic as [`XPathValidationError::Query`].
pub fn run_rules(
    source: crate::RuleSource<'_>,
    data: &RdfDataset,
    limits: &crate::RuleLimits,
    knobs: crate::LimitKnobs,
    profile: Profile,
    pattern_limits: Limits,
) -> Result<crate::Inference, XPathValidationError> {
    // The selection's declared-pattern cache is keyed by the cells the shapes own; a
    // clone shares those cells, so the cache serves the rules this run executes. A SPARQL
    // 1.2 RL rule set declares no `sh:pattern`.
    let owner = match source {
        crate::RuleSource::Shapes(shapes) => Arc::new(shapes.clone()),
        crate::RuleSource::Srl(_) => Arc::default(),
    };
    Configuration::detached(profile, pattern_limits, owner)
        .run(|| crate::rules::run_rules(source, data, limits, knobs))
        .map_err(|error| {
            error.into_public(|message| XPathValidationError::Shapes(ShapesError::from(message)))
        })
}

/// Evaluate one node expression under an explicit dated XPath law and finite limits.
///
/// The same parse and evaluation as [`crate::free_expression::evaluate`], inside the
/// selection: a `sh:pattern` a filter shape decides, and a `REGEX`/`REPLACE` a function
/// call or a SPARQL-based expression evaluates, compile and match under `profile` within
/// `limits`.
///
/// # Errors
/// Everything [`crate::free_expression::evaluate`] refuses, as
/// [`XPathValidationError::Shapes`]; a native pattern refusal as
/// [`XPathValidationError::Pattern`], and a query's operational diagnostic as
/// [`XPathValidationError::Query`].
pub fn evaluate_free_expression(
    request: &crate::free_expression::FreeExpression<'_>,
    profile: Profile,
    limits: Limits,
) -> Result<crate::free_expression::NodeExprEvaluation, XPathValidationError> {
    let (shapes, expr) = crate::free_expression::parse(request)?;
    let shapes = Arc::new(shapes);
    Configuration::detached(profile, limits, Arc::clone(&shapes))
        .run(|| crate::free_expression::evaluate_parsed(request, &shapes, &expr))
        .map_err(|error| error.into_public(XPathValidationError::Shapes))
}
