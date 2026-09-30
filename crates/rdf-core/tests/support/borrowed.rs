// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Observing a resolved term's borrowed content without copying it, for the
//! zero-allocation test and the IR layout bench.

use purrdf_core::TermRef;

/// Touch a borrowed term's `&str` content without copying it — returns a length so the
/// borrow is genuinely observed by the optimizer. Triple-term components are ids (no
/// borrowed string content), so they contribute nothing here.
pub(crate) fn term_ref_len(t: TermRef<'_>) -> usize {
    match t {
        TermRef::Iri(s) => s.len(),
        TermRef::Blank { label, .. } => label.len(),
        TermRef::Literal {
            lexical, language, ..
        } => lexical.len() + language.map_or(0, str::len),
        TermRef::Triple { .. } => 0,
    }
}
