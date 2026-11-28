// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Triple-term rows written column by column, with no value column, as the
//! reifier tests store them.

use purrdf_gts::model::{Term, TermKind};

/// A self-describing triple term (wire `"tt"`) naming `(s, p, o)` directly.
#[allow(
    dead_code,
    reason = "each test file includes this module and uses its own subset"
)]
pub(super) const fn triple_term(s: usize, p: usize, o: usize) -> Term {
    Term {
        kind: TermKind::Triple,
        value: None,
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: Some((s, p, o)),
    }
}

/// A triple term carried only by its reifier reference (wire `"rf"`).
#[allow(
    dead_code,
    reason = "each test file includes this module and uses its own subset"
)]
pub(super) const fn indirect_triple_term(reifier: usize) -> Term {
    Term {
        kind: TermKind::Triple,
        value: None,
        datatype: None,
        lang: None,
        direction: None,
        reifier: Some(reifier),
        triple: None,
    }
}
