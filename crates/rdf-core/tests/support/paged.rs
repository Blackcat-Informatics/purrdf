// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The page and operation-status fixtures the paged-dataset suites share.

// Included by `#[path]` into several integration-test binaries, and no single binary uses
// every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::sync::Arc;

use purrdf_core::{
    PagedQueryError, PagedQueryEvidence, RdfDataset, RdfDatasetBuilder, ViewOperationStatus,
};

/// One frozen single-triple page in the default graph, `example.org`-scoped:
/// `ex:{subject} ex:p ex:{object}`.
pub fn page(subject: &str, object: &str) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let subject_iri = format!("http://example.org/{subject}");
    let object_iri = format!("http://example.org/{object}");
    let subject = builder.intern_iri(&subject_iri);
    let predicate = builder.intern_iri("http://example.org/p");
    let object = builder.intern_iri(&object_iri);
    builder.push_quad(subject, predicate, object, None);
    builder.freeze().expect("valid page")
}

/// The evidence of an operation that must have completed.
pub fn ready_evidence(
    status: ViewOperationStatus<PagedQueryError, PagedQueryEvidence>,
) -> PagedQueryEvidence {
    match status {
        ViewOperationStatus::Ready { evidence } => evidence,
        ViewOperationStatus::Failed { error, .. } => {
            panic!("expected a ready operation, got: {error}")
        }
    }
}
