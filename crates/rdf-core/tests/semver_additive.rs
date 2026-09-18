// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Compile-time pin of the `purrdf-core` paged-dataset public surface as published at
//! 2.0.2.
//!
//! This file has no assertions that can fail at runtime worth the name; its job is to
//! FAIL TO COMPILE the moment the pinned 2.0.2 shapes below are removed or reshaped —
//! a struct loses/renames/retypes a field, an enum variant is removed or its field
//! set changes, or a pinned method's signature changes. Every check is pure type
//! system: exhaustive struct-literal construction plus exhaustive destructuring for
//! the structs, exhaustive `match` arms bound by name for the enums, and
//! type-annotated calls for the key `PagedDataset` methods.
//!
//! New items and new enum variants are ADDITIVE and expected — they do not, and must
//! not, make this file fail to compile. `PagedFreezeError::SummaryDrift` is one such
//! addition and is deliberately exercised here as "new", not asserted away. Both
//! pinned enums are `#[non_exhaustive]`, so every match below carries a trailing
//! wildcard arm for exactly that reason: it accepts future variants while still
//! pinning every pre-existing one by name and field shape.
//!
//! This file guards against REMOVAL or RESHAPING of the pinned 2.0.2 surface. It is
//! not a substitute for review of genuinely new, additive surface.

use std::sync::Arc;

use purrdf_core::{
    DatasetView, FallibleDatasetView, GlobalDictionary, InMemoryPageProvider, PageFault,
    PageGeneration, PageId, PagePart, PageProvider, PageTranslation, PagedDataset,
    PagedFreezeError, PagedQueryError, PagedQueryEvidence, PagedQueryLimits, PagedQueryView,
    RdfDataset, RdfDatasetBuilder, RdfStoreCapabilities, ViewOperationStatus,
};

/// One frozen single-triple page in the default graph, `example.org`-scoped.
fn page(subject: &str, object: &str) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let subject_iri = format!("http://example.org/{subject}");
    let object_iri = format!("http://example.org/{object}");
    let subject = builder.intern_iri(&subject_iri);
    let predicate = builder.intern_iri("http://example.org/p");
    let object = builder.intern_iri(&object_iri);
    builder.push_quad(subject, predicate, object, None);
    builder.freeze().expect("valid page")
}

/// A two-page sealed `PagedDataset` at `PageGeneration::INITIAL`, for tests that need
/// a real dataset to pull real field values out of.
fn two_page_dataset() -> (PagedDataset, Arc<dyn PageProvider>) {
    let provider: Arc<dyn PageProvider> = Arc::new(InMemoryPageProvider::new(vec![
        page("s0", "o0"),
        page("s1", "o1"),
    ]));
    let dataset =
        PagedDataset::from_provider(Arc::clone(&provider)).expect("seal a disjoint two-page set");
    (dataset, provider)
}

/// Pin `PagePart`'s 2.0.2 field set: `translation`, `capabilities`, `quad_count`,
/// `byte_len`. Adding, removing, or renaming a field breaks this file's compilation.
#[test]
// The field-by-field literal below is deliberately a same-shape rebuild of
// `extracted` — that redundancy against clippy's simplification IS the guard: it
// names every 2.0.2 field explicitly rather than moving the value, so a renamed,
// added, or removed field fails to compile here instead of being silently carried
// through a `..extracted` or a bare move.
#[allow(clippy::unnecessary_struct_initialization)]
fn pagepart_field_shape_is_pinned() {
    let (dataset, _provider) = two_page_dataset();
    let (_dictionary, _generation, mut parts) = dataset.to_parts();
    let extracted = parts.remove(0);

    // The exhaustive struct-literal construction: this line alone fails to compile
    // if `PagePart` gains, loses, or renames a field.
    let part = PagePart {
        translation: extracted.translation,
        capabilities: extracted.capabilities,
        quad_count: extracted.quad_count,
        byte_len: extracted.byte_len,
    };

    // The exhaustive destructure: this line alone fails to compile under the same
    // conditions, from the opposite direction.
    let PagePart {
        translation,
        capabilities,
        quad_count,
        byte_len,
    } = part;

    assert_eq!(translation.term_count(), 3, "s, p, o each intern once");
    assert_eq!(capabilities, RdfStoreCapabilities::plain_rdf());
    assert_eq!(quad_count, 1);
    assert!(
        byte_len > 0,
        "the reference provider charges a positive byte length"
    );
}

/// Pin `PagedQueryEvidence`'s 2.0.2 field set: `generation`, `requested_pages`,
/// `consumed_pages`, `consumed_bytes`.
#[test]
fn pagedqueryevidence_field_shape_is_pinned() {
    let (dataset, _provider) = two_page_dataset();
    let view = dataset.query_view(PagedQueryLimits::UNBOUNDED);
    assert_eq!(view.quads().count(), 2, "both pages admit under no ceiling");
    let evidence = match view.operation_status() {
        ViewOperationStatus::Ready { evidence } => evidence,
        ViewOperationStatus::Failed { error, .. } => {
            panic!("expected a ready operation, got: {error}")
        }
    };

    let rebuilt = PagedQueryEvidence {
        generation: evidence.generation,
        requested_pages: evidence.requested_pages.clone(),
        consumed_pages: evidence.consumed_pages,
        consumed_bytes: evidence.consumed_bytes,
    };

    let PagedQueryEvidence {
        generation,
        requested_pages,
        consumed_pages,
        consumed_bytes,
    } = rebuilt;

    assert_eq!(generation, PageGeneration::INITIAL);
    assert_eq!(requested_pages, vec![PageId(0), PageId(1)]);
    assert_eq!(consumed_pages, 2);
    assert!(consumed_bytes > 0);
}

/// Pin `PagedQueryLimits`'s 2.0.2 field set: `max_pages`, `max_bytes`.
#[test]
fn pagedquerylimits_field_shape_is_pinned() {
    let limits = PagedQueryLimits {
        max_pages: 4,
        max_bytes: 4096,
    };
    let PagedQueryLimits {
        max_pages,
        max_bytes,
    } = limits;
    assert_eq!(max_pages, 4);
    assert_eq!(max_bytes, 4096);
}

/// Pin every 2.0.2 variant (and field shape) of `PagedFreezeError` by name. The
/// trailing wildcard is required by `#[non_exhaustive]` and is exactly how this file
/// accepts additive variants like `SummaryDrift` without pinning their internals.
fn describe_freeze_error(error: &PagedFreezeError) -> String {
    match error {
        PagedFreezeError::Page(fault) => format!("page: {fault}"),
        PagedFreezeError::GenerationMismatch { expected, actual } => {
            format!("generation-mismatch: expected {expected}, actual {actual}")
        }
        PagedFreezeError::PageCountMismatch { metadata, provider } => {
            format!("page-count-mismatch: metadata {metadata}, provider {provider}")
        }
        PagedFreezeError::QuadOverlap(overlap) => format!("quad-overlap: {overlap:?}"),
        // Additive (this branch), not pinned by field shape: intentionally not named
        // away, per the module docs above.
        PagedFreezeError::SummaryDrift { page, message } => {
            format!("summary-drift: page {page:?}, message {message}")
        }
        _ => "some future additive PagedFreezeError variant".to_owned(),
    }
}

#[test]
fn pagedfreezeerror_variant_shapes_are_pinned() {
    let (dataset, provider) = two_page_dataset();
    let (dictionary, generation, mut parts) = dataset.to_parts();
    // Drop one part so the metadata/provider page counts disagree — a real, easy to
    // trigger `PageCountMismatch`.
    parts.pop();
    let error = PagedDataset::from_parts(dictionary, provider, generation, parts)
        .expect_err("mismatched part/provider page counts must refuse");
    assert!(matches!(error, PagedFreezeError::PageCountMismatch { .. }));
    let described = describe_freeze_error(&error);
    assert!(described.starts_with("page-count-mismatch"));
}

/// Pin every 2.0.2 variant (and field shape) of `PagedQueryError` by name, exactly as
/// `describe_freeze_error` above does for `PagedFreezeError`.
fn describe_query_error(error: &PagedQueryError) -> String {
    match error {
        PagedQueryError::Provider { page, message } => {
            format!("provider: page {page:?}, message {message}")
        }
        PagedQueryError::StaleGeneration {
            page,
            expected,
            actual,
        } => format!("stale-generation: page {page:?}, expected {expected}, actual {actual}"),
        PagedQueryError::PageBudgetExceeded {
            page,
            limit,
            consumed,
        } => format!("page-budget-exceeded: page {page:?}, limit {limit}, consumed {consumed}"),
        PagedQueryError::ByteBudgetExceeded {
            page,
            limit,
            consumed,
            page_bytes,
        } => format!(
            "byte-budget-exceeded: page {page:?}, limit {limit}, consumed {consumed}, \
             page_bytes {page_bytes}"
        ),
        PagedQueryError::Stopped {
            page,
            cause,
            message,
        } => format!("stopped: page {page:?}, cause {cause:?}, message {message}"),
        PagedQueryError::InvalidData { page, message } => {
            format!("invalid-data: page {page:?}, message {message}")
        }
        _ => "some future additive PagedQueryError variant".to_owned(),
    }
}

#[test]
fn pagedqueryerror_variant_shapes_are_pinned() {
    let (dataset, _provider) = two_page_dataset();
    // A zero page budget refuses the very first page request — a real, easy to
    // trigger `PageBudgetExceeded`.
    let view = dataset.query_view(PagedQueryLimits::new(0, u64::MAX));
    assert_eq!(view.quads().count(), 0);
    let error = match view.operation_status() {
        ViewOperationStatus::Ready { evidence } => {
            panic!("expected a failed operation, got ready evidence: {evidence:?}")
        }
        ViewOperationStatus::Failed { error, .. } => error,
    };
    assert!(matches!(error, PagedQueryError::PageBudgetExceeded { .. }));
    let described = describe_query_error(&error);
    assert!(described.starts_with("page-budget-exceeded"));
}

/// Pin the key `PagedDataset` public method signatures via type-annotated calls: if
/// any of these change shape (receiver, argument types, or return type), the
/// annotation below stops typechecking.
#[test]
fn pageddataset_key_signatures_are_pinned() {
    let (dataset, provider) = two_page_dataset();

    let page_count: usize = dataset.page_count();
    let dictionary_ref: &GlobalDictionary = dataset.dictionary();
    let generation: PageGeneration = dataset.generation();
    let translation: Option<&PageTranslation> = dataset.translation(PageId(0));
    let _view: PagedQueryView<'_> = dataset.query_view(PagedQueryLimits::UNBOUNDED);

    let compacted: PagedDataset = dataset.compact();
    let dropped: PagedDataset = dataset.drop_page(PageId(0));
    let subset: PagedDataset = dataset.with_pages(&[PageId(1)]);

    let (parts_dictionary, parts_generation, parts): (
        GlobalDictionary,
        PageGeneration,
        Vec<PagePart>,
    ) = dataset.to_parts();
    let rebuilt: Result<PagedDataset, PagedFreezeError> =
        PagedDataset::from_parts(parts_dictionary, provider, parts_generation, parts);

    assert_eq!(page_count, 2);
    assert_eq!(
        dictionary_ref.len(),
        5,
        "4 distinct s/o values plus 1 shared predicate"
    );
    assert_eq!(generation, PageGeneration::INITIAL);
    assert!(translation.is_some());
    assert_eq!(compacted.page_count(), 2);
    assert_eq!(dropped.page_count(), 1);
    assert_eq!(subset.page_count(), 1);
    assert!(rebuilt.is_ok());
}

/// `PageFault` is not one of the four pinned structs/enums above, but it is the
/// payload of `PagedFreezeError::Page` pinned in `describe_freeze_error`; confirm it
/// still resolves as a type here too, so a reader can see the whole chain typechecks.
#[allow(dead_code)]
fn _page_fault_type_still_resolves(_f: PageFault) {}
