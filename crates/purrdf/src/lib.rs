// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Umbrella Rust API for PurRDF.
//!
//! This crate is the user-facing facade and the single dependency a downstream
//! needs: it re-exports the RDF 1.2 implementation surface from [`purrdf_rdf`]
//! at the root, and carries every other published crate under a stable module,
//! so anything a consumer legitimately imports is reachable from `purrdf`
//! alone — never by reaching into a sub-crate.
//!
//! | Module | Sub-crate(s) |
//! |---|---|
//! | (root) | [`purrdf_rdf`] — core types, codecs, GTS/text adapters |
//! | [`gts`] | [`purrdf_gts`] (container engine) + the [`purrdf_rdf`] GTS adapter |
//! | [`sparql`] | [`purrdf_sparql_eval`] + [`purrdf_sparql_algebra`] + [`purrdf_sparql_results`] |
//! | [`shapes`] | [`purrdf_shapes`] (SHACL) |
//! | [`shex`] | [`purrdf_shex`] (ShEx 2.1) |
//! | [`slice`] | [`purrdf_slice`] |
//! | [`xsd`] | [`purrdf_xsd`] |
//! | [`iri`] | [`purrdf_iri`] |
//! | [`events`] | [`purrdf_events`] |
//!
//! Consumer-config types are surfaced at the root ([`SliceVocab`],
//! [`Namespaces`], [`StatementMetadataVocab`]) and unified behind a single
//! [`OntologyProfile`] a downstream builds once (see [`profile`]).

pub use purrdf_rdf::*;

pub mod profile;
pub use profile::{OntologyProfile, ReifierVocab};

// ── consumer-config types, surfaced directly ────────────────────────────────
// A consumer parameterizes an emitter without reaching into a sub-crate.
pub use purrdf_rdf::native_codecs::jsonld::StatementMetadataVocab;
pub use purrdf_shapes::json_schema::Namespaces;
pub use purrdf_slice::SliceVocab;

/// GTS: the container engine ([`purrdf_gts`]) plus the RDF-level GTS adapter
/// from [`purrdf_rdf`] (`read_graph`, `flattened_dataset_from_bytes`, …).
///
/// The two surfaces have disjoint names — the engine exposes modules
/// (`codec`, `model`, `reader`, `writer`, …), the adapter exposes free
/// functions — so both are reachable here without collision.
pub mod gts {
    pub use purrdf_gts::*;
    pub use purrdf_rdf::gts::*;
}

/// SPARQL 1.1/1.2: parser + algebra ([`purrdf_sparql_algebra`]), evaluator
/// ([`purrdf_sparql_eval`]), and results serialization
/// ([`purrdf_sparql_results`]).
pub mod sparql {
    pub use purrdf_sparql_algebra::*;
    pub use purrdf_sparql_eval::*;
    pub use purrdf_sparql_results::*;
    // Both the algebra and eval crates expose an `error` module, so the bare
    // name is ambiguous under the two globs. Bind it to the algebra (parser)
    // errors explicitly; every error *type* (`ParseError`, `EvalError`,
    // `Error`) is still re-exported at this module's root by the globs above.
    pub use purrdf_sparql_algebra::error;
}

/// XSD datatype value spaces and operations.
pub mod xsd {
    pub use purrdf_xsd::*;
}

/// IRI parsing, resolution, and CURIE expansion/contraction.
pub mod iri {
    pub use purrdf_iri::*;
}

/// The zero-dependency streaming RDF event model.
pub mod events {
    pub use purrdf_events::*;
}

/// Native slice catalog and dataset-wrapper support.
pub mod slice {
    pub use purrdf_slice::*;
}

/// SHACL shape support.
pub mod shapes {
    pub use purrdf_shapes::*;
}

/// ShEx 2.1 schema parsing, serialization, and validation.
pub mod shex {
    pub use purrdf_shex::*;
}

/// The common umbrella surface, for `use purrdf::prelude::*;`.
pub mod prelude {
    pub use purrdf_rdf::prelude::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facade_exposes_rdf_slice_shapes_and_shex() {
        let _ = RdfDatasetBuilder::new();
        let _ = slice::rdf_query::DatasetAccumulator::new();
        let _ = shapes::report::ValidationReport {
            conforms: true,
            results: Vec::new(),
        };
        let _ = shex::parse_shexc("PREFIX ex: <https://example.org/>\nex:S { ex:p . }", None)
            .expect("shex facade parses");
    }

    #[test]
    fn facade_exposes_the_completed_umbrella() {
        // gts: the container engine (its `model`) and the rdf-level adapter
        // (`read_graph`) are both reachable under the one `gts` module.
        assert!(!format!("{:?}", gts::model::TermKind::Iri).is_empty());
        let adapter: fn(&[u8], bool) -> _ = gts::read_graph;
        assert!(
            adapter(&[], false).is_err(),
            "empty input is not a GTS graph"
        );

        // sparql: parser (algebra) + engine (eval) + results.
        assert!(!format!("{:?}", sparql::SparqlResultsFormat::Json).is_empty());

        // foundations.
        assert!(iri::parse("https://example.org/x").is_ok());
        assert!(!format!("{:?}", events::TextDirection::Ltr).is_empty());
    }

    #[test]
    fn facade_exposes_unified_consumer_config() {
        let profile = OntologyProfile::for_namespace("https://example.org/vocab/");
        // The three native config types are all reachable from the root, and
        // the profile projects into each.
        let sv: SliceVocab = profile.slice_vocab();
        let ns: Namespaces = profile.namespaces().expect("primary prefix resolves");
        let smv: StatementMetadataVocab<'_> = profile.statement_metadata_vocab();
        assert_eq!(sv.ns(), "https://example.org/vocab/");
        assert_eq!(ns.compact_iri("https://example.org/vocab/Cat"), "vocab:Cat");
        assert!(smv.statement_metadata.ends_with("StatementMetadata"));
    }
}
