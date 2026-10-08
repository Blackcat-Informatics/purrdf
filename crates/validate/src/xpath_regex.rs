// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The dated XPath regular-expression law a host selects by name, and what every host
//! does with the selection around a SPARQL request.
//!
//! Every host — the command line, the C ABI, WebAssembly and Python — names a law by
//! its stable [`Profile::name`] and hands the name here ([`parse_profile`]). Matching is
//! exact, as [`Profile::from_name`] is: no case folding, abbreviation or undated alias
//! names a law, so an unknown name is refused naming every accepted one
//! ([`UnknownXPathRegex`]) rather than mapped to a guess. No name keeps the compatibility
//! pattern behaviour, unchanged.
//!
//! A selected law runs under the production bounds, [`Limits::new`]: a finite ceiling on
//! every compiler, matcher and replacement resource. Work past one is an operational
//! refusal of the whole request — never an unbound value, a `false` filter, an empty
//! inference or a conforming report.
//!
//! The SHACL entry points that take a selection sit beside their compatibility twins:
//! [`crate::validate_to_sarif_string_with_xpath_regex`],
//! [`crate::validate_changes_to_sarif_string_with_xpath_regex`],
//! [`crate::validate_with_shapes_product_with_xpath_regex`],
//! [`crate::validate_with_rebuilt_shapes_product_with_xpath_regex`],
//! [`crate::entail_to_ntriples_with_xpath_regex`],
//! [`crate::apply_rules_to_ntriples_with_xpath_regex`] and
//! [`crate::eval_node_expr_with_xpath_regex`]. Each takes `Option<Profile>`: `None` is
//! its compatibility twin's exact answer, and `Some` runs the same request through the
//! `purrdf_shapes::xpath` doors.
//!
//! A resource refusal's identity is its resource's stable [`Resource::code`]
//! (`xpath-pattern-bytes`, `xpath-match-steps`, ...), whichever door refused: the SPARQL
//! engine reduces it to a diagnostic under that code, and a SHACL or ShEx run keeps the
//! typed refusal. [`refusal_code`], [`validation_refusal_code`] and
//! [`diagnostic_refusal_code`] read it from each, so every host reports one refusal under
//! one code.

use std::fmt;

use purrdf_core::xsd_regex::xpath::{Error, Resource};
pub use purrdf_core::xsd_regex::xpath::{Limits, Profile};
pub use purrdf_shapes::xpath::XPathValidationError;
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};

/// A name that is not exactly one dated law's [`Profile::name`].
///
/// Its message names the refused value and every accepted name, oldest first; each host
/// prefixes it with its own spelling of the parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownXPathRegex {
    name: String,
}

impl UnknownXPathRegex {
    /// The refused name, verbatim.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for UnknownXPathRegex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} names no dated XPath regex law; the accepted names are ",
            self.name
        )?;
        for (index, profile) in Profile::ALL.into_iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{:?}", profile.name())?;
        }
        Ok(())
    }
}

impl std::error::Error for UnknownXPathRegex {}

/// The dated law `name` selects, or `None` when the host supplied no name.
///
/// # Errors
///
/// [`UnknownXPathRegex`] for a name that is not exactly one [`Profile::name`].
pub fn parse_profile(name: Option<&str>) -> Result<Option<Profile>, UnknownXPathRegex> {
    name.map(|name| {
        Profile::from_name(name).ok_or_else(|| UnknownXPathRegex {
            name: name.to_owned(),
        })
    })
    .transpose()
}

/// The SPARQL engine a host's request runs on: under `profile` with the production
/// bounds when a law was selected, the compatibility engine otherwise.
#[must_use]
pub fn sparql_engine(profile: Option<Profile>) -> NativeSparqlEngine {
    let engine = NativeSparqlEngine::new();
    match profile {
        Some(profile) => engine.with_xpath_regex(profile, Limits::new()),
        None => engine,
    }
}

/// `options` evaluating every pattern built-in under `profile` with the production
/// bounds, or unchanged when no law was selected — the per-request form of
/// [`sparql_engine`], for a host whose engine is shared across requests.
#[must_use]
pub const fn query_options(
    options: QueryOptions<'_>,
    profile: Option<Profile>,
) -> QueryOptions<'_> {
    match profile {
        Some(profile) => options.with_xpath_regex(profile, Limits::new()),
        None => options,
    }
}

/// Every resource a selected law's bounds admit, in [`Limits`] order: the codes a
/// refusal can be reported under.
const RESOURCES: [Resource; 8] = [
    Resource::PatternBytes,
    Resource::CompileSteps,
    Resource::ProgramNodes,
    Resource::CompileSlots,
    Resource::MatchSteps,
    Resource::MatchStates,
    Resource::MatchSlots,
    Resource::OutputBytes,
];

/// The [`Resource::code`] a native XPath operational refusal is identified by: the
/// withheld resource of [`Error::Resource`], or the resource whose storage the host
/// refused in [`Error::Allocation`]. `None` for a pattern, flag or replacement error,
/// which is never a refusal of the request.
#[must_use]
pub const fn refusal_code(error: &Error) -> Option<&'static str> {
    match error {
        Error::Resource(refusal) => Some(refusal.resource.code()),
        Error::Allocation { resource, .. } => Some(resource.code()),
        _ => None,
    }
}

/// The [`Resource::code`] a diagnostic `code` is, when it is one: the SPARQL engine
/// reports a withheld or unallocatable resource under exactly that resource's code. `None`
/// for every other diagnostic code.
#[must_use]
pub fn diagnostic_refusal_code(code: &str) -> Option<&'static str> {
    RESOURCES
        .into_iter()
        .map(Resource::code)
        .find(|resource| *resource == code)
}

/// The [`Resource::code`] a selected-law validation's failure is identified by: a
/// native pattern's operational refusal ([`refusal_code`]), or a SHACL-driven query's
/// diagnostic reported under a resource's code ([`diagnostic_refusal_code`]). `None` for
/// every other failure.
#[must_use]
pub fn validation_refusal_code(error: &XPathValidationError) -> Option<&'static str> {
    match error {
        XPathValidationError::Pattern(error) => refusal_code(error),
        XPathValidationError::Query(diagnostic) => diagnostic_refusal_code(&diagnostic.code),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each resource's refusal, and its allocation refusal, is identified by the
    /// resource's own code, and so is a diagnostic reported under it; a pattern error and
    /// a neighbouring diagnostic code are not refusals.
    #[test]
    fn a_refusal_is_identified_by_its_resource_code() {
        // `Limits` holds one `u64` bound per resource, so a resource added there without
        // its code here changes its size.
        assert_eq!(size_of::<Limits>(), RESOURCES.len() * size_of::<u64>());
        for resource in RESOURCES {
            let refusal = Limits::new()
                .with(resource, 0)
                .admit(resource, 1)
                .expect_err("over the bound");
            let error = Error::from(refusal);
            assert_eq!(refusal_code(&error), Some(resource.code()));
            let allocation = Error::Allocation { resource, units: 1 };
            assert_eq!(refusal_code(&allocation), Some(resource.code()));
            assert_eq!(
                diagnostic_refusal_code(resource.code()),
                Some(resource.code())
            );
            assert_eq!(
                validation_refusal_code(&XPathValidationError::Pattern(error)),
                Some(resource.code())
            );
            assert_eq!(
                validation_refusal_code(&XPathValidationError::Query(
                    purrdf_core::RdfDiagnostic::error(resource.code(), "refused")
                )),
                Some(resource.code())
            );
            // The admitted neighbour at the bound is no refusal at all.
            assert!(Limits::new().with(resource, 1).admit(resource, 1).is_ok());
        }
        let syntax = Error::Syntax {
            offset: 0,
            message: "unbalanced".to_owned(),
        };
        assert_eq!(refusal_code(&syntax), None);
        assert_eq!(refusal_code(&Error::EmptyMatch), None);
        for code in [
            "native-sparql-xpath-operational",
            "xpath-pattern",
            "xpath-pattern-bytes ",
        ] {
            assert_eq!(diagnostic_refusal_code(code), None, "{code:?}");
        }
        assert_eq!(
            validation_refusal_code(&XPathValidationError::Query(
                purrdf_core::RdfDiagnostic::error("native-sparql-query-parse", "refused")
            )),
            None
        );
        assert_eq!(
            validation_refusal_code(&XPathValidationError::CachePoisoned),
            None
        );
    }

    #[test]
    fn each_exact_name_selects_its_law_and_no_name_selects_none() {
        assert_eq!(parse_profile(None), Ok(None));
        for profile in Profile::ALL {
            assert_eq!(parse_profile(Some(profile.name())), Ok(Some(profile)));
        }
    }

    #[test]
    fn a_neighbouring_name_is_refused_naming_every_accepted_one() {
        for name in [
            "xpath-3.1",
            "XPATH-3.1-2017-03-21",
            "",
            " xpath-3.1-2017-03-21",
        ] {
            let error = parse_profile(Some(name)).expect_err(name);
            assert_eq!(error.name(), name);
            assert_eq!(
                error.to_string(),
                format!(
                    "{name:?} names no dated XPath regex law; the accepted names are \
                     \"xpath-2.0-2010-12-14\", \"xpath-3.1-2017-03-21\""
                )
            );
        }
    }

    #[test]
    fn a_selection_reaches_the_request_options() {
        assert_eq!(query_options(QueryOptions::new(), None).xpath_regex(), None);
        for profile in Profile::ALL {
            assert_eq!(
                query_options(QueryOptions::new(), Some(profile)).xpath_regex(),
                Some((profile, Limits::new()))
            );
        }
    }
}
