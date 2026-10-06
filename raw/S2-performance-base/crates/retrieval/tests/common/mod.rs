// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixtures shared by the integration tests.

use std::sync::{Arc, OnceLock};

use purrdf_core::{RdfDataset, RdfDatasetBuilder};

/// A dataset with no quads at all.
///
/// Used only where a test is about something other than the data: a plan's
/// identity, an emitted unit's text, a registry mismatch, a producer's own row
/// count. Every one of those questions is answered by relations reached from
/// predicate position, which read no stored data, so an empty graph is the
/// honest input rather than a stand-in for one. A test that is about the data
/// builds its own dataset.
/// Shared rather than built per call, and handed out by reference: a stream an
/// execution returns borrows the dataset it was read from, because an exclusion
/// lookup is a question asked of that dataset while the fusion is merging. A
/// fixture that built a fresh dataset per call would therefore hand every test a
/// stream borrowing a temporary. One frozen, empty dataset for the whole binary
/// is the same value in every case — it holds no quads, so no test can observe
/// which one it got.
pub(crate) fn empty_dataset() -> &'static RdfDataset {
    static EMPTY: OnceLock<Arc<RdfDataset>> = OnceLock::new();
    EMPTY.get_or_init(|| {
        RdfDatasetBuilder::new()
            .freeze()
            .expect("an empty default graph is structurally valid")
    })
}
