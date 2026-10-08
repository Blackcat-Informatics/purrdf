// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable SHACL law identities and purpose-aware SPARQL query admission.
//!
//! A dated identity describes a bundle of Core, SPARQL and XPath specifications.
//! [`ShaclProfile::admit_query`] checks the bundle's pre-binding restrictions on
//! parsed algebra. Executing a validation under that bundle additionally requires
//! its XPath law for Core patterns and SPARQL `REGEX`/`REPLACE`.

use std::fmt;

use purrdf_core::xsd_regex::xpath::{Limits, Profile};
use purrdf_sparql_algebra::Query;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum Identity {
    #[default]
    Legacy,
    Recommendation,
    WorkingDraft,
}

/// An immutable, explicitly dated SHACL semantic law.
///
/// The default is [`Self::LEGACY`], which preserves the compatibility law.
/// Dated handles identify their exact Core, SPARQL and required XPath editions;
/// no unknown date is mapped to a nearby edition or to the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ShaclProfile(Identity);

impl ShaclProfile {
    /// The existing compatibility law, including its frozen query restrictions.
    pub const LEGACY: Self = Self(Identity::Legacy);

    /// SHACL REC 2017-07-20, with the incorporated-errata XPath 2.0 law.
    ///
    /// SPARQL 1.1 references XPath 2.0's First Edition. The Second Edition named
    /// by [`Self::xpath_specification`] incorporates that edition's errata.
    pub const REC_20170720: Self = Self(Identity::Recommendation);

    /// SHACL Core WD 2026-09-17 and SPARQL WD 2026-09-18, with XPath 3.1.
    pub const WD_20260918: Self = Self(Identity::WorkingDraft);

    /// The stable identifier accepted by [`Self::from_id`].
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self.0 {
            Identity::Legacy => "legacy",
            Identity::Recommendation => "shacl-rec-2017-07-20",
            Identity::WorkingDraft => "shacl-wd-2026-09-18",
        }
    }

    /// Decode one exact supported identifier.
    ///
    /// # Errors
    ///
    /// Returns [`UnsupportedProfile`] for every other spelling or date.
    pub fn from_id(id: &str) -> Result<Self, UnsupportedProfile> {
        match id {
            "legacy" => Ok(Self::LEGACY),
            "shacl-rec-2017-07-20" => Ok(Self::REC_20170720),
            "shacl-wd-2026-09-18" => Ok(Self::WD_20260918),
            _ => Err(UnsupportedProfile { id: id.to_owned() }),
        }
    }

    /// The dated Core specification; the compatibility law has no dated claim.
    #[must_use]
    pub const fn core_specification(self) -> Option<&'static str> {
        match self.0 {
            Identity::Legacy => None,
            Identity::Recommendation => Some("https://www.w3.org/TR/2017/REC-shacl-20170720/"),
            Identity::WorkingDraft => Some("https://www.w3.org/TR/2026/WD-shacl12-core-20260917/"),
        }
    }

    /// The dated SPARQL extension specification.
    #[must_use]
    pub const fn sparql_specification(self) -> Option<&'static str> {
        match self.0 {
            Identity::Legacy => None,
            Identity::Recommendation => Some("https://www.w3.org/TR/2017/REC-shacl-20170720/"),
            Identity::WorkingDraft => {
                Some("https://www.w3.org/TR/2026/WD-shacl12-sparql-20260918/")
            }
        }
    }

    /// The required native XPath law's exact specification.
    ///
    /// REC20170720 uses XPath 2.0 Second Edition's incorporated errata. The draft
    /// uses the XPath 3.1 Recommendation referenced by SPARQL 1.2 WD20260820.
    /// The compatibility law leaves regex selection to the caller.
    #[must_use]
    pub const fn xpath_specification(self) -> Option<&'static str> {
        match self.0 {
            Identity::Legacy => None,
            Identity::Recommendation => {
                Some("https://www.w3.org/TR/2010/REC-xpath-functions-20101214/")
            }
            Identity::WorkingDraft => {
                Some("https://www.w3.org/TR/2017/REC-xpath-functions-31-20170321/")
            }
        }
    }

    /// The native XPath law required by this exact SHACL bundle.
    ///
    /// The compatibility law has no implicit native selection. The returned type
    /// is the compiler's own law identity, rather than a separate SHACL regex
    /// profile that could drift from it.
    #[must_use]
    pub const fn xpath_profile(self) -> Option<Profile> {
        match self.0 {
            Identity::Legacy => None,
            Identity::Recommendation => Some(Profile::Xpath20),
            Identity::WorkingDraft => Some(Profile::Xpath31),
        }
    }

    /// Resolve the bundle's native XPath request and admit a caller override.
    ///
    /// A dated bundle selects its required law with [`Limits::default`] unless
    /// the caller supplies finite limits for that same law. Every supplied limit,
    /// including zero, is preserved. The compatibility law passes the explicit
    /// request through unchanged and selects no native law when it is absent.
    ///
    /// This resolves configuration only. The native compiler and every reused
    /// program still admit the current source, program and storage requirements,
    /// and each execution receives fresh runtime budgets.
    ///
    /// # Errors
    ///
    /// [`XPathProfileConflict`] when an override would replace the XPath law
    /// required by the selected dated bundle.
    pub fn resolve_xpath(
        self,
        selection: Option<(Profile, Limits)>,
    ) -> Result<Option<(Profile, Limits)>, XPathProfileConflict> {
        let Some(required) = self.xpath_profile() else {
            return Ok(selection);
        };
        match selection {
            Some((requested, _)) if requested != required => Err(XPathProfileConflict {
                profile: self,
                required,
                requested,
            }),
            Some(selection) => Ok(Some(selection)),
            None => Ok(Some((required, Limits::default()))),
        }
    }

    /// Audit a parsed query for its actual SHACL purpose.
    ///
    /// `parameters` contains the names, without `?`/`$`, of declared pre-bound
    /// parameters. Constraint/validator roles additionally supply their standard
    /// bindings; function/target-type roles supply only their parameters. A SELECT
    /// target produces `$this`, while its ASK inverse pre-binds `$this`.
    ///
    /// The audit visits nested patterns and expressions, including aggregate
    /// operands and EXISTS. It does not evaluate the query or admit a regex
    /// program. Successful query admission alone is no validation verdict.
    ///
    /// # Errors
    ///
    /// Returns a typed [`AdmissionRefusal`] for the first prohibited construct or
    /// a query form incompatible with `purpose`. `SERVICE` is refused under every
    /// law because PurRDF's SHACL execution is local and deterministic.
    pub fn admit_query(
        self,
        purpose: QueryPurpose,
        query: &Query,
        parameters: &[&str],
    ) -> Result<(), AdmissionRefusal> {
        crate::prebinding::admit(self, purpose, query, parameters.iter().copied())
    }
}

impl fmt::Display for ShaclProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// An identifier for which this engine has no named SHACL law.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnsupportedProfile {
    id: String,
}

impl UnsupportedProfile {
    /// The exact unsupported input, retained without normalization.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl fmt::Display for UnsupportedProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported SHACL profile {:?}", self.id)
    }
}

impl std::error::Error for UnsupportedProfile {}

/// An explicit regex override incompatible with the selected dated SHACL law.
///
/// Changing finite limits within the required law is compatible. This error
/// identifies a semantic-law conflict before parsing or execution can substitute
/// another law or fall back to compatibility behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XPathProfileConflict {
    profile: ShaclProfile,
    required: Profile,
    requested: Profile,
}

impl XPathProfileConflict {
    /// The dated SHACL bundle requiring this regex law.
    #[must_use]
    pub const fn profile(self) -> ShaclProfile {
        self.profile
    }

    /// The native XPath law required by that bundle.
    #[must_use]
    pub const fn required(self) -> Profile {
        self.required
    }

    /// The incompatible native XPath law supplied by the caller.
    #[must_use]
    pub const fn requested(self) -> Profile {
        self.requested
    }
}

impl fmt::Display for XPathProfileConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} requires {}, but the regex override requests {}",
            self.profile,
            self.required.name(),
            self.requested.name()
        )
    }
}

impl std::error::Error for XPathProfileConflict {}

/// The admitted law, source occurrences and existing native execution home of
/// one immutable preparation/binding. No request is allocated for Legacy's
/// unselected path.
#[derive(Clone, Debug)]
pub(crate) struct Request {
    pub(crate) profile: ShaclProfile,
    native: Option<crate::xpath::Configuration>,
    sources: Option<std::sync::Arc<crate::shapes::ConstraintSources>>,
}

impl Request {
    pub(crate) const fn sources(
        &self,
    ) -> Option<&std::sync::Arc<crate::shapes::ConstraintSources>> {
        self.sources.as_ref()
    }

    pub(crate) fn admit(
        prepared: &crate::engine::PreparedShapes,
    ) -> Result<Option<Self>, crate::report::CompleteValidationError> {
        let options = &prepared.shapes().validation_options;
        let requested = options
            .shacl_profile
            .resolve_xpath(options.xpath_regex)
            .map_err(crate::report::CompleteValidationError::XPathProfile)?;
        let selection = if requested.is_some()
            && let Some(native) = crate::xpath::current()
        {
            options
                .shacl_profile
                .resolve_xpath(Some((native.profile, native.limits)))
                .map_err(crate::report::CompleteValidationError::XPathProfile)?
        } else {
            requested
        };
        if options.shacl_profile == ShaclProfile::LEGACY && selection.is_none() {
            return Ok(None);
        }
        let sources = if options.shacl_profile == ShaclProfile::LEGACY {
            None
        } else {
            Some(
                prepared
                    .shapes()
                    .report_sources_with_profile(options.shacl_profile)?,
            )
        };
        let native =
            selection.map(|(profile, limits)| prepared.xpath_configuration(profile, limits));
        if options.shacl_profile != ShaclProfile::LEGACY
            && let Some(configuration) = &native
        {
            configuration
                .admit_declared_patterns()
                .map_err(|error| crate::report::CompleteValidationError::XPath(Box::new(error)))?;
        }
        Ok(Some(Self {
            profile: options.shacl_profile,
            native,
            sources,
        }))
    }

    /// Carry this law through the existing query/native scopes, and defer error
    /// projection until the actual governor and typed source/report result agree.
    pub(crate) fn run<T>(
        &self,
        operation: impl FnOnce() -> Result<T, crate::report::CompleteValidationError>,
    ) -> Result<T, crate::report::CompleteValidationError> {
        let execute = || {
            let scope = crate::query_law::enter(self.profile, self.sources.as_ref());
            let outcome = operation();
            if let Some(state) = crate::sparql::current_governors()
                && let Some(error) = crate::report::CompleteValidationError::resource(&state)
            {
                return Err(error);
            }
            let failure = scope
                .runtime
                .as_ref()
                .and_then(|runtime| runtime.take_failure());
            match outcome {
                Ok(value) => failure.map_or(Ok(value), |failure| Err(failure.into_public())),
                Err(error) => Err(match error {
                    crate::report::CompleteValidationError::Execution(_)
                    | crate::report::CompleteValidationError::Shapes(
                        crate::error::ShapesError::Invalid(_),
                    ) => failure.map_or(error, crate::report::ReportFailure::into_public),
                    error => error,
                }),
            }
        };
        match &self.native {
            Some(configuration) => configuration
                .run(execute)
                .map_err(crate::report::CompleteValidationError::from_native_run),
            None => execute(),
        }
    }
}

/// The role determining a query's pre-bound variables and permitted query form.
///
/// AF rules, functions and targets, and draft node expressions retain their own
/// role bindings. Choosing one of these roles audits pre-binding; declarations
/// and extension applicability are checked by their respective shape parsers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum QueryPurpose {
    /// An `sh:sparql` SELECT constraint, with `$this` pre-bound.
    SelectConstraint,
    /// A SELECT component validator, with `$this` and component parameters.
    SelectValidator,
    /// An ASK component validator, with `$this`, `$value` and parameters.
    AskValidator,
    /// An AF CONSTRUCT rule, with the focus node bound to `$this`.
    ConstructRule,
    /// A global CONSTRUCT rule, with only its template parameters pre-bound.
    ///
    /// SHACL 1.2 Inference Rules executes a global SPARQL rule without the
    /// initial focus-node binding used by a shape rule.
    GlobalConstructRule,
    /// An AF SELECT or ASK function body, with its parameters pre-bound.
    Function,
    /// An AF SELECT target type, with its parameters pre-bound.
    TargetType,
    /// An AF SELECT target that produces focus nodes without pre-binding `$this`.
    SelectTarget,
    /// An AF ASK target inverse, with the candidate `$this` pre-bound.
    AskTarget,
    /// A SELECT node expression, with the current `$this` pre-bound.
    SelectExpression,
    /// A scalar expression's generated SELECT, with `$this` pre-bound.
    ScalarExpression,
}

impl QueryPurpose {
    pub(crate) fn accepts(self, query: &Query) -> bool {
        match self {
            Self::SelectConstraint
            | Self::SelectValidator
            | Self::TargetType
            | Self::SelectTarget
            | Self::SelectExpression
            | Self::ScalarExpression => matches!(query, Query::Select { .. }),
            Self::AskValidator | Self::AskTarget => matches!(query, Query::Ask { .. }),
            Self::ConstructRule | Self::GlobalConstructRule => {
                matches!(query, Query::Construct { .. })
            }
            Self::Function => matches!(query, Query::Select { .. } | Query::Ask { .. }),
        }
    }

    pub(crate) const fn binds_this(self) -> bool {
        !matches!(
            self,
            Self::Function | Self::TargetType | Self::SelectTarget | Self::GlobalConstructRule
        )
    }

    pub(crate) const fn binds_shape_context(self) -> bool {
        matches!(
            self,
            Self::SelectConstraint | Self::SelectValidator | Self::AskValidator
        )
    }
}

/// Why a query cannot be admitted under the selected pre-binding law.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AdmissionReason {
    /// Contextual application algebra requires its dedicated preparation route.
    ContextualApplication,
    /// A MINUS pattern forbidden for this query role.
    Minus,
    /// A federated SERVICE pattern; remote execution is never admitted.
    Service,
    /// A VALUES clause forbidden wholly or because it mentions a pre-bound name.
    Values,
    /// Assignment to a potentially pre-bound variable.
    Assignment,
    /// A REC subquery omits a required potentially pre-bound variable.
    SubqueryProjection,
    /// The algebra has the wrong query form for its declared purpose.
    QueryForm,
}

/// A structured query-admission failure, separate from a data violation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmissionRefusal {
    profile: ShaclProfile,
    purpose: QueryPurpose,
    reason: AdmissionReason,
    variable: Option<String>,
}

impl AdmissionRefusal {
    pub(crate) fn new(
        profile: ShaclProfile,
        purpose: QueryPurpose,
        reason: AdmissionReason,
        variable: Option<String>,
    ) -> Self {
        Self {
            profile,
            purpose,
            reason,
            variable,
        }
    }

    /// The law used for this audit.
    #[must_use]
    pub const fn profile(&self) -> ShaclProfile {
        self.profile
    }

    /// The role used to determine potential pre-bindings.
    #[must_use]
    pub const fn purpose(&self) -> QueryPurpose {
        self.purpose
    }

    /// The stable refusal category.
    #[must_use]
    pub const fn reason(&self) -> AdmissionReason {
        self.reason
    }

    /// The implicated variable, without its sigil, when the restriction names one.
    #[must_use]
    pub fn variable(&self) -> Option<&str> {
        self.variable.as_deref()
    }
}

impl fmt::Display for AdmissionRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = match self.reason {
            AdmissionReason::ContextualApplication => {
                "contextual application algebra requires its typed preparation route"
            }
            AdmissionReason::Minus => "MINUS is forbidden",
            AdmissionReason::Service => "SERVICE is forbidden",
            AdmissionReason::Values => "VALUES is forbidden",
            AdmissionReason::Assignment => {
                "assignment to a potentially pre-bound variable is forbidden"
            }
            AdmissionReason::SubqueryProjection => {
                "subquery omits a required potentially pre-bound variable"
            }
            AdmissionReason::QueryForm => "query form does not match its SHACL purpose",
        };
        write!(
            f,
            "{description} under {} for {:?}",
            self.profile, self.purpose
        )?;
        if let Some(variable) = &self.variable {
            write!(f, ": ?{variable}")?;
        }
        Ok(())
    }
}

impl std::error::Error for AdmissionRefusal {}
