// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The published JSON Schema meta-schemas, as test data.
//!
//! `purrdf-jsonschema` carries no meta-schema: its callers register the ones
//! they need. The workspace's tests and examples take them from here — the
//! verbatim upstream documents vendored under
//! `crates/jsonschema/tests/metaschemas/` (provenance and licence recorded
//! there), which never reach a published crate. Each entry is
//! `(retrieval URI, JSON text)`; the URI is the document's own `$id` without
//! an empty fragment.

/// A meta-schema document: its URI and its JSON text.
pub type Document = (&'static str, &'static str);

macro_rules! vendored {
    ($path:literal) => {
        include_str!(concat!("../../jsonschema/tests/metaschemas/", $path))
    };
}

/// The draft 2020-12 meta-schema and its eight vocabulary meta-schemas.
pub const DRAFT_2020_12: &[Document] = &[
    (
        "https://json-schema.org/draft/2020-12/schema",
        vendored!("draft2020-12/schema.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/core",
        vendored!("draft2020-12/meta/core.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/applicator",
        vendored!("draft2020-12/meta/applicator.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/unevaluated",
        vendored!("draft2020-12/meta/unevaluated.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/validation",
        vendored!("draft2020-12/meta/validation.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/meta-data",
        vendored!("draft2020-12/meta/meta-data.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/format-annotation",
        vendored!("draft2020-12/meta/format-annotation.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/format-assertion",
        vendored!("draft2020-12/meta/format-assertion.json"),
    ),
    (
        "https://json-schema.org/draft/2020-12/meta/content",
        vendored!("draft2020-12/meta/content.json"),
    ),
];

/// The draft 2019-09 meta-schema and its six vocabulary meta-schemas.
pub const DRAFT_2019_09: &[Document] = &[
    (
        "https://json-schema.org/draft/2019-09/schema",
        vendored!("draft2019-09/schema.json"),
    ),
    (
        "https://json-schema.org/draft/2019-09/meta/core",
        vendored!("draft2019-09/meta/core.json"),
    ),
    (
        "https://json-schema.org/draft/2019-09/meta/applicator",
        vendored!("draft2019-09/meta/applicator.json"),
    ),
    (
        "https://json-schema.org/draft/2019-09/meta/validation",
        vendored!("draft2019-09/meta/validation.json"),
    ),
    (
        "https://json-schema.org/draft/2019-09/meta/meta-data",
        vendored!("draft2019-09/meta/meta-data.json"),
    ),
    (
        "https://json-schema.org/draft/2019-09/meta/format",
        vendored!("draft2019-09/meta/format.json"),
    ),
    (
        "https://json-schema.org/draft/2019-09/meta/content",
        vendored!("draft2019-09/meta/content.json"),
    ),
];

/// The draft-07 meta-schema.
pub const DRAFT_07: &[Document] = &[(
    "http://json-schema.org/draft-07/schema",
    vendored!("draft-07/schema.json"),
)];

/// Every vendored meta-schema, all three drafts.
pub fn all() -> impl Iterator<Item = Document> {
    DRAFT_2020_12
        .iter()
        .chain(DRAFT_2019_09)
        .chain(DRAFT_07)
        .copied()
}
