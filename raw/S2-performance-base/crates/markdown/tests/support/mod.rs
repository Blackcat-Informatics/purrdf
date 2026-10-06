// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The slice vocabulary the Markdown suites declare their profiles under.

use purrdf_markdown::Vocabulary;

/// The namespace every slice term of the fixtures lives under.
pub(crate) const SLICE_BASE: &str = "https://example.org/slice/";

/// The vocabulary under [`SLICE_BASE`].
pub(crate) fn v() -> Vocabulary {
    Vocabulary::under(SLICE_BASE).expect("a vocabulary")
}
