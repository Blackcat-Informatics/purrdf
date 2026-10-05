// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable SHACL law identities and purpose-aware SPARQL query admission.
//!
//! A dated identity describes a bundle of Core, SPARQL and XPath specifications.
//! [`ShaclProfile::admit_query`] checks the bundle's pre-binding restrictions on
//! parsed algebra. Executing a validation under that bundle additionally requires
//! its XPath law for Core patterns and SPARQL `REGEX`/`REPLACE`.

use std::fmt;

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
        crate::prebinding::admit(self, purpose, query, parameters)
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
            Self::ConstructRule => matches!(query, Query::Construct { .. }),
            Self::Function => matches!(query, Query::Select { .. } | Query::Ask { .. }),
        }
    }

    pub(crate) const fn binds_this(self) -> bool {
        !matches!(self, Self::Function | Self::TargetType | Self::SelectTarget)
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
