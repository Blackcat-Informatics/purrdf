// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared test support for the DL search-cost guards (`dl_step_ledger`,
//! `dl_consistency_search_budget`, `dl_work_budget`): one certificate-field reader instead of
//! three copies drifting apart across files that all read the same rendering.

/// The value of the certificate's `field` line, as a number.
///
/// Read out of the RENDERED certificate rather than from a reasoner API, because what these
/// tests are about is the numbers a caller actually sees — across the Python, WASM and C
/// boundaries, that string is all any of them get.
///
/// Matched on the field name plus a space, so a line ADDED to the rendering cannot be read as
/// another one's value (`work` and `work-budget` are two different lines, and a prefix match
/// without the space would let the former swallow the latter) and a reader does not have to
/// re-check the parser every time the certificate grows a measurement.
pub(crate) fn measurement(certificate: &str, field: &str) -> u64 {
    let prefix = format!("{field} ");
    let line = certificate
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("the certificate states no `{field}` line:\n{certificate}"));
    line[prefix.len()..]
        .trim()
        .parse()
        .unwrap_or_else(|error| panic!("`{line}` is not a number: {error}"))
}
