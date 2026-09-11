// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! XSD/XPath regular-expression dialect support (issue #295).
//!
//! `sh:pattern` (SHACL), SPARQL `REGEX`/`REPLACE`, and ShEx `PATTERN` all
//! specify their pattern facet via `fn:matches`/`fn:replace`
//! ([XPath and XQuery Functions and Operators 3.1 §5.6]), which in turn
//! reuses the `regExp` grammar of
//! [XML Schema Part 2 Appendix G]. That grammar is a distinct dialect from
//! the `regex` crate's own syntax — this module is where the one shared
//! translation between the two will live (tracked by
//! <https://github.com/Blackcat-Informatics/purrdf/issues/295>). This module
//! currently holds only the Unicode block-escape lookup table that
//! translation needs, landed ahead of the translator itself so the table has
//! its own generation/test story before anything consumes it.
//!
//! [XPath and XQuery Functions and Operators 3.1 §5.6]: https://www.w3.org/TR/xpath-functions-31/#regex-syntax
//! [XML Schema Part 2 Appendix G]: https://www.w3.org/TR/xmlschema11-2/#regexs

// `blocks` (the generated `UNICODE_BLOCKS` table + `lookup`) has no non-test
// consumer yet by explicit task ordering (issue #295's plan lands the table
// before the translator that reads it, so the table gets its own generation/
// test story first). `#[cfg(test)]`-gating the declaration, rather than
// reaching for `#[allow(dead_code)]`, keeps the crate's normal build honest:
// there is genuinely no production code path through this module today. The
// translator task removes this `#[cfg(test)]` the moment it adds one.
#[cfg(test)]
mod blocks;
