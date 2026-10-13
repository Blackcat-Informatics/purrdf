// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The dated regular-expression law a C caller selects by name.
//!
//! Every `*_xpath_regex` entry point takes a nullable `regex_profile`: NULL keeps
//! the compatibility regular-expression behaviour the entry point without the
//! suffix has always had, and a name selects one dated native XPath law, exactly
//! as [`Profile::from_name`] spells it. Matching is exact, so an undated, case-folded
//! or abbreviated name is refused with [`PurrdfStatus::InvalidArgument`] rather than
//! mapped to a guess. The selected law runs under the production [`Limits::new`].
//!
//! A selected law refuses work it cannot admit within those limits. That refusal is
//! [`PurrdfStatus::RegexResourceError`], never an empty result, a `false` answer or a
//! conforming report.

use std::os::raw::c_char;

use purrdf_core::RdfDiagnostic;
pub(crate) use purrdf_core::xsd_regex::xpath::Profile;
use purrdf_rs::shapes::xpath::XPathValidationError;
use purrdf_validate::ShapesError;

use crate::error::PurrdfError;
use crate::opt_cstr_to_str;
use crate::status::PurrdfStatus;

/// The diagnostic code the SPARQL engine gives a native XPath operational failure
/// that names no single resource.
const XPATH_OPERATIONAL_CODE: &str = "native-sparql-xpath-operational";

/// Decode a nullable `regex_profile` argument: NULL is `None` (compatibility), and a
/// name is the dated law it spells exactly.
///
/// # Errors
///
/// [`PurrdfStatus::InvalidUtf8`] for a name that is not UTF-8, and
/// [`PurrdfStatus::InvalidArgument`] for any name [`Profile::from_name`] does not
/// accept — the message lists every accepted name.
///
/// # Safety
/// Same contract as [`opt_cstr_to_str`].
pub(crate) unsafe fn decode_regex_profile(
    regex_profile: *const c_char,
) -> Result<Option<Profile>, PurrdfError> {
    let name = unsafe { opt_cstr_to_str(regex_profile)? };
    purrdf_validate::xpath_regex::parse_profile(name).map_err(|error| {
        PurrdfError::new(
            PurrdfStatus::InvalidArgument,
            format!("regex_profile {error}, or NULL for the compatibility regular expressions"),
        )
    })
}

/// Whether a SPARQL diagnostic code is a native XPath law's operational refusal.
///
/// The engine reduces a withheld resource to that resource's own code
/// (`xpath-pattern-bytes`, `xpath-match-steps`, ...) and any other native operational
/// failure to [`XPATH_OPERATIONAL_CODE`].
fn is_regex_resource_code(code: &str) -> bool {
    code.starts_with("xpath-") || code == XPATH_OPERATIONAL_CODE
}

/// Map a SPARQL request's diagnostic onto the C error channel: a native XPath
/// operational refusal as [`PurrdfStatus::RegexResourceError`], everything else as
/// [`PurrdfStatus::QueryError`]. Both keep the diagnostic's code and presentation.
pub(crate) fn query_error(diagnostic: &RdfDiagnostic) -> PurrdfError {
    let status = if is_regex_resource_code(&diagnostic.code) {
        PurrdfStatus::RegexResourceError
    } else {
        PurrdfStatus::QueryError
    };
    PurrdfError::from_diagnostic(status, diagnostic)
}

/// Map a selected-law SHACL validation's error onto the C error channel.
///
/// A shapes, data or import refusal goes through `shapes`, the mapping the entry
/// point's compatibility path gives the same refusal; a withheld native pattern
/// resource, or a poisoned native program cache, is
/// [`PurrdfStatus::RegexResourceError`]; a SHACL-SPARQL query's diagnostic goes
/// through [`query_error`]; any other execution failure is a
/// [`PurrdfStatus::ParseError`], as the compatibility path reports it.
pub(crate) fn validation_error(
    error: XPathValidationError,
    shapes: impl FnOnce(ShapesError) -> PurrdfError,
) -> PurrdfError {
    match error {
        XPathValidationError::Shapes(error) => shapes(error),
        XPathValidationError::Pattern(error) if error.is_operational() => {
            PurrdfError::new(PurrdfStatus::RegexResourceError, error.to_string())
        }
        XPathValidationError::CachePoisoned => PurrdfError::new(
            PurrdfStatus::RegexResourceError,
            XPathValidationError::CachePoisoned.to_string(),
        ),
        XPathValidationError::Query(diagnostic) => query_error(&diagnostic),
        other => PurrdfError::new(PurrdfStatus::ParseError, other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::*;

    fn decoded(name: &str) -> Result<Option<Profile>, PurrdfError> {
        let name = CString::new(name).expect("no NUL");
        unsafe { decode_regex_profile(name.as_ptr()) }
    }

    #[test]
    fn null_is_compatibility_and_each_exact_name_is_its_law() {
        assert_eq!(
            unsafe { decode_regex_profile(std::ptr::null()) }.expect("NULL decodes"),
            None
        );
        for profile in Profile::ALL {
            assert_eq!(decoded(profile.name()).expect("exact name"), Some(profile));
        }
    }

    #[test]
    fn a_neighbouring_name_is_refused_listing_the_accepted_names() {
        for name in [
            "xpath-3.1",
            "XPATH-3.1-2017-03-21",
            "",
            " xpath-3.1-2017-03-21",
        ] {
            let error = decoded(name).expect_err(name);
            assert_eq!(error.code, PurrdfStatus::InvalidArgument, "{name:?}");
            let message = error.message.to_str().expect("UTF-8");
            assert!(
                message.contains("\"xpath-2.0-2010-12-14\"")
                    && message.contains("\"xpath-3.1-2017-03-21\""),
                "{message}"
            );
        }
    }

    #[test]
    fn only_native_xpath_codes_are_regex_resource_refusals() {
        for code in [
            "xpath-pattern-bytes",
            "xpath-match-steps",
            XPATH_OPERATIONAL_CODE,
        ] {
            assert!(is_regex_resource_code(code), "{code}");
        }
        for code in ["native-sparql-query-parse", "native-sparql-query-eval"] {
            assert!(!is_regex_resource_code(code), "{code}");
        }
    }
}
