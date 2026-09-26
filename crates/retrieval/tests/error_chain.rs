// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The error types expose their cause through `Error::source`, and only where
//! there is one.
//!
//! A caller walking the chain to report or classify a refusal depends on the
//! link being present exactly where a cause is carried. Each assertion that a
//! link exists sits beside a control that must have none, so a `source` that
//! answered `None` everywhere, or `Some` everywhere, fails here.

use std::error::Error as _;

use purrdf_retrieval::{Iri, PlanError, SearchError};

fn invalid_iri() -> PlanError {
    Iri::parse("not an iri").expect_err("a relative reference with spaces is not an IRI")
}

#[test]
fn an_invalid_iri_names_the_kernel_parser_refusal_as_its_source() {
    let err = invalid_iri();
    let source = err
        .source()
        .expect("InvalidIri carries the parser's refusal");
    assert!(
        source.downcast_ref::<purrdf_core::IriError>().is_some(),
        "the source is the kernel's IriError, not a rendering of it"
    );
    assert!(err.to_string().ends_with(&source.to_string()));

    let control = PlanError::TrailingBytes { extra: 3 };
    assert!(
        control.source().is_none(),
        "a variant with no cause has no source"
    );
}

#[test]
fn a_search_error_names_the_stage_error_it_wraps_as_its_source() {
    let err = SearchError::from(invalid_iri());
    assert!(matches!(err, SearchError::PlanError(_)));
    let stage = err.source().expect("the wrapped stage error is the source");
    let plan = stage
        .downcast_ref::<PlanError>()
        .expect("the source is the PlanError itself");
    assert!(matches!(plan, PlanError::InvalidIri { .. }));
    assert!(
        stage.source().is_some(),
        "the chain continues through the stage error to the parser's refusal"
    );

    let control = SearchError::from(PlanError::TrailingBytes { extra: 3 });
    let stage = control
        .source()
        .expect("the wrapped stage error is the source");
    assert!(
        stage.source().is_none(),
        "the chain ends where the stage error has no cause"
    );
}
