// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The property names and values an ECMA-262 `\p{…}` / `\P{…}` escape may
//! spell, and the `regex` property each one denotes.
//!
//! ECMA-262 (§22.2.2.9, `UnicodeMatchProperty` and `UnicodeMatchPropertyValue`)
//! accepts a name only when it is spelled *exactly* as one of the aliases its
//! tables list — case, underscores and all. `regex` matches property names
//! loosely (`\p{letter}`, `\p{Is_Latin}`), so handing it the escape verbatim
//! would accept patterns every ECMA-262 engine rejects. Every alias is
//! therefore checked against the tables first, and only the canonical name is
//! ever handed on.
//!
//! The tables are generated from the vendored Unicode Character Database by
//! the workspace's one Unicode table generator,
//! `crates/lex/examples/gen_unicode_tables.rs` (see `property_tables.rs`):
//!
//! * General_Category values: every `gc` alias of UCD
//!   `PropertyValueAliases.txt`, mapped to its short alias.
//! * Script and Script_Extensions values: every `sc` alias of the same file,
//!   mapped to its long name.
//! * Binary properties: the closed list of ECMA-262 Table 66 "Binary Unicode
//!   property aliases and their canonical property names", every spelling it
//!   lists, each confirmed against UCD `PropertyAliases.txt`.

pub(super) use super::property_tables::{BINARY, GENERAL_CATEGORY, SCRIPT};

/// The canonical name behind an exact alias in `table`.
pub(super) fn lookup(table: &[(&str, &'static str)], alias: &str) -> Option<&'static str> {
    table
        .iter()
        .find_map(|&(candidate, canonical)| (candidate == alias).then_some(canonical))
}
