// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The coordinate reference system the geo end-to-end suites express their fixtures in.

use purrdf_geo::geom::Crs;

/// The one coordinate reference system in play: every fixture geometry is expressed in it.
pub(crate) const CRS: &str = "http://example.org/crs/planar";

/// [`CRS`] as a [`Crs`].
pub(crate) fn crs() -> Crs {
    Crs::new(CRS).expect("a non-empty IRI")
}
