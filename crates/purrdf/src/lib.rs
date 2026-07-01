// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Umbrella Rust API for PurRDF.
//!
//! This crate is the user-facing facade. It re-exports the RDF 1.2 implementation
//! surface from [`purrdf_rdf`] at the root, and carries the first-class slice and
//! SHACL shape crates under stable modules.

pub use purrdf_rdf::*;

/// Native slice catalog and dataset-wrapper support.
pub mod slice {
    pub use purrdf_slice::*;
}

/// SHACL shape support.
pub mod shapes {
    pub use purrdf_shapes::*;
}

/// The common umbrella surface, for `use purrdf::prelude::*;`.
pub mod prelude {
    pub use purrdf_rdf::prelude::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facade_exposes_rdf_slice_and_shapes() {
        let _ = RdfDatasetBuilder::new();
        let _ = slice::rdf_query::DatasetAccumulator::new();
        let _ = shapes::report::ValidationReport {
            conforms: true,
            results: Vec::new(),
        };
    }
}
