// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The dated native XPath regex law a SPARQL or SHACL entry point evaluates under.
//!
//! Every entry point that can evaluate SPARQL `REGEX`/`REPLACE`, or a SHACL
//! `sh:pattern`, takes an optional `xpathRegex`: the stable name of a dated law in
//! [`purrdf_core::xsd_regex::xpath`], `"xpath-2.0-2010-12-14"` or
//! `"xpath-3.1-2017-03-21"`. The name is matched exactly, by
//! [`Profile::from_name`]: no case folding, abbreviation or undated alias names a
//! law, so an unknown name is refused, naming the accepted ones, rather than mapped
//! to a guess or ignored. Absent, the entry keeps its compatibility regex behaviour.
//!
//! A selected law runs under the production [`Limits::new`] bounds. A pattern the
//! law's grammar refuses keeps the host's ordinary expression-error behaviour; a
//! resource or allocation refusal is an operational failure of the whole request,
//! never an empty, unbound or `false` answer.

use purrdf_core::xsd_regex::xpath::Profile;
use purrdf_sparql_eval::QueryOptions;

/// The option's name, as every JavaScript surface spells it.
pub(crate) const OPTION: &str = "xpathRegex";

/// The dated law `name` selects: `None` when no name was supplied.
///
/// # Errors
///
/// A name that is not exactly one [`Profile::name`], with the accepted names listed
/// ([`purrdf_validate::xpath_regex::parse_profile`], every host's one reading).
pub(crate) fn parse(name: Option<&str>) -> Result<Option<Profile>, String> {
    purrdf_validate::xpath_regex::parse_profile(name).map_err(|error| format!("{OPTION} {error}"))
}

/// `options` evaluating `REGEX`/`REPLACE` under `profile` with the production bounds, or
/// unchanged when no law was selected ([`purrdf_validate::xpath_regex::query_options`]).
pub(crate) const fn select(
    options: QueryOptions<'_>,
    profile: Option<Profile>,
) -> QueryOptions<'_> {
    purrdf_validate::xpath_regex::query_options(options, profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each stable name selects its own law, and nothing else names one: a neighbour
    /// differing only in its date, its case or its emptiness is refused with every
    /// accepted name listed.
    #[test]
    fn only_the_exact_stable_names_select_a_law() {
        assert_eq!(parse(None), Ok(None));
        for profile in Profile::ALL {
            assert_eq!(parse(Some(profile.name())), Ok(Some(profile)));
        }
        assert_eq!(
            parse(Some("xpath-3.1-2017-03-21")),
            Ok(Some(Profile::Xpath31))
        );
        assert_eq!(
            parse(Some("xpath-2.0-2010-12-14")),
            Ok(Some(Profile::Xpath20))
        );
        for refused in [
            "xpath-3.1",
            "XPATH-3.1-2017-03-21",
            "",
            " xpath-3.1-2017-03-21",
        ] {
            let message = parse(Some(refused)).expect_err(refused);
            assert!(message.starts_with("xpathRegex "), "{message}");
            assert!(
                message.contains("\"xpath-2.0-2010-12-14\", \"xpath-3.1-2017-03-21\""),
                "{message}"
            );
        }
    }

    /// No selection leaves the request on the compatibility law; a selection carries
    /// its law and the production bounds.
    #[test]
    fn a_selection_reaches_the_request_options() {
        assert_eq!(select(QueryOptions::new(), None).xpath_regex(), None);
        assert_eq!(
            select(QueryOptions::new(), Some(Profile::Xpath20)).xpath_regex(),
            Some((
                Profile::Xpath20,
                purrdf_core::xsd_regex::xpath::Limits::new()
            ))
        );
    }
}

#[cfg(test)]
mod exports;
