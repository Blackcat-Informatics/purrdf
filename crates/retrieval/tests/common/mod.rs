// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fixtures shared by the integration tests.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder};

/// A dataset with no quads at all.
///
/// Used only where a test is about something other than the data: a plan's
/// identity, an emitted unit's text, a registry mismatch, a producer's own row
/// count. Every one of those questions is answered by relations reached from
/// predicate position, which read no stored data, so an empty graph is the
/// honest input rather than a stand-in for one. A test that is about the data
/// builds its own dataset.
pub(crate) fn empty_dataset() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new()
        .freeze()
        .expect("an empty default graph is structurally valid")
}
