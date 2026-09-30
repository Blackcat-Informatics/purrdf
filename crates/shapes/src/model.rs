// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Namespace IRI constants for SHACL, RDF, RDFS, and XSD, plus the
//! caller-supplied graph-box role vocabulary.
//!
//! The W3C terms are not defined here: [`sh`], [`shnex`], [`sparql_ns`],
//! [`rdf`] and [`rdfs`] re-export the namespace modules of
//! [`purrdf_iri::vocab`], and [`xsd`] re-exports the datatype constants of
//! [`purrdf_xsd::datatype`] under this crate's established names.
//!
//! All constants are plain `&'static str` IRIs. Native query patterns key on the
//! IRI string directly; constructors like [`crate::term::NamedNode::from`] wrap one
//! into a term when a value is needed.

/// SHACL namespace constants (`http://www.w3.org/ns/shacl#`).
pub use purrdf_iri::vocab::sh;

/// SHACL 1.2 node-expression namespace constants
/// (`http://www.w3.org/ns/shacl-node-expr#`).
///
/// Where a kind here has an older SHACL Advanced Features spelling in the [`sh`]
/// namespace (`sh:union`, `sh:if`, `sh:count`, …), BOTH spellings parse to the
/// SAME `crate::expression::NodeExpr` arm and run through the SAME evaluator —
/// two spec-defined surfaces over one intermediate representation, exactly as two
/// RDF syntaxes parse to one graph model.
pub use purrdf_iri::vocab::shnex;

/// The SPARQL 1.2 term vocabulary (`http://www.w3.org/ns/sparql#`).
///
/// SHACL 1.2 Node Expressions §5 makes the IRIs of this vocabulary callable from
/// a node expression: "A blank node that uses a SPARQL function URI
/// `sparql:<NAME>` as its predicate with an `rdf:List` of arguments as its
/// object is called a SHACL SPARQL function expression with the corresponding
/// SPARQL function name."
///
/// Which local names are callable, and the SPARQL surface form each lowers to,
/// is decided by [`crate::expression::sparql_ns_lowering`] — one uniform table
/// over the vocabulary, not a per-name branch.
pub use purrdf_iri::vocab::sparql as sparql_ns;

/// The CALLER-SUPPLIED graph-box role vocabulary for the OPTIONAL box-role
/// annotation feature.
///
/// PurRDF is not an ontology and mints no vocabulary IRIs of its own, so there
/// is deliberately NO default (mirroring the `LanguageVocab` /
/// `StatementMetadataVocab` pattern elsewhere in the workspace): a shapes
/// parse / validation run without a configured vocab leaves the box-role
/// feature INACTIVE — every box-role list on shapes and validation results
/// stays empty.
///
/// Configure it through
/// [`from_dataset_with_config`](crate::shapes::from_dataset_with_config) /
/// [`parse_shapes_with_config`](crate::engine::parse_shapes_with_config); the
/// parsed [`Shapes`](crate::shapes::Shapes) carries it into validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoxRoleVocab {
    /// The predicate annotating a shape node or data-graph predicate with its
    /// graph-box role.
    pub graph_box_role: String,
    /// The ABox role individual.
    pub box_abox: String,
    /// The TBox role individual.
    pub box_tbox: String,
    /// The RBox role individual.
    pub box_rbox: String,
    /// The CBox role individual (stamped onto reifier-shape results).
    pub box_cbox: String,
    /// The ConfigBox role individual.
    pub box_config_box: String,
}

impl BoxRoleVocab {
    /// Derive the six term IRIs by concatenation from a namespace whose local
    /// names are `graphBoxRole` / `boxABox` / `boxTBox` / `boxRBox` / `boxCBox`
    /// / `boxConfigBox`.
    #[must_use]
    pub fn for_namespace(ns: &str) -> Self {
        Self {
            graph_box_role: format!("{ns}graphBoxRole"),
            box_abox: format!("{ns}boxABox"),
            box_tbox: format!("{ns}boxTBox"),
            box_rbox: format!("{ns}boxRBox"),
            box_cbox: format!("{ns}boxCBox"),
            box_config_box: format!("{ns}boxConfigBox"),
        }
    }
}

/// RDF namespace constants (`http://www.w3.org/1999/02/22-rdf-syntax-ns#`).
pub use purrdf_iri::vocab::rdf;

/// RDFS namespace constants (`http://www.w3.org/2000/01/rdf-schema#`).
///
/// Every term of [`purrdf_iri::vocab::rdfs`], plus `BASE`, this crate's
/// established name for the namespace IRI.
pub mod rdfs {
    pub use purrdf_iri::vocab::rdfs::NS as BASE;
    pub use purrdf_iri::vocab::rdfs::*;
}

/// XSD namespace and datatype constants (`http://www.w3.org/2001/XMLSchema#`),
/// re-exported from [`purrdf_xsd::datatype`] under this crate's established names.
pub mod xsd {
    pub use purrdf_xsd::datatype::{
        XSD_BOOLEAN as BOOLEAN, XSD_DECIMAL as DECIMAL, XSD_DOUBLE as DOUBLE,
        XSD_INTEGER as INTEGER, XSD_NS as BASE, XSD_STRING as STRING,
    };
}
