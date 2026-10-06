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

use std::fmt;

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

#[cfg(test)]
mod tests {
    use super::*;

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
