// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The composite-term constructors the CDT suites build their fixtures with.

use purrdf_cdt::{CdtTerm, CdtValue};

/// A composite element, refused by the constructor only when it would break one of the
/// crate's two bounds — which no fixture does.
pub(crate) fn composite(value: CdtValue) -> CdtTerm {
    CdtTerm::composite(value).expect("the fixture is within every bound")
}

/// A triple-term element, under the same standing as [`composite`].
pub(crate) fn triple(subject: CdtTerm, predicate: CdtTerm, object: CdtTerm) -> CdtTerm {
    CdtTerm::triple(subject, predicate, object).expect("the fixture is within every bound")
}
