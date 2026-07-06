// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf-sparql-algebra` — the native **SPARQL 1.1/1.2 front-end** for the RDF
//! 1.2 query stack.
//!
//! A pure-Rust, wasm-clean leaf crate that parses SPARQL query text into a
//! purrdf-owned, **RDF 1.2-native** query algebra ([`Query`]/[`GraphPattern`]).
//! It is the drop-in replacement for the oxigraph-family SPARQL parser (purrdf S5,
//! ) and the front-end the downstream evaluator S6 (`sparql-eval`,
//! ) consumes. It builds only on the two CLOSED foundation leaves
//! [`purrdf_iri`] and [`purrdf_xsd`], and deliberately does **not**
//! depend on `purrdf-core`.
//!
//! # Scope (purrdf S5)
//!
//! Parse + algebra **only** — no evaluation. The in-scope SPARQL surface is
//! corpus-driven (the project's `queries/**/*.rq` plus the DSL-generated
//! projections) and covers both halves of SPARQL 1.1/1.2:
//!
//! - **Query**: the four query forms, basic graph patterns, `OPTIONAL`,
//!   `UNION`, `MINUS`, `GRAPH`, `FILTER`/`BIND`/`VALUES`, property paths,
//!   `GROUP BY`/aggregates, `EXISTS`/`NOT EXISTS`, solution modifiers, and the
//!   RDF 1.2 quoted triple terms (`<<( s p o )>>`) used by the codec round-trips.
//! - **Update** ([`Update`]/[`GraphUpdateOperation`]): `INSERT DATA` /
//!   `DELETE DATA`, the `DELETE`/`INSERT … WHERE` family (`WITH`/`USING`,
//!   `DELETE WHERE`), `LOAD`, and the graph-management operations
//!   `CLEAR`/`DROP`/`CREATE`/`ADD`/`MOVE`/`COPY`.
//!
//! Anything outside this surface is a hard [`ParseError::Unsupported`], never a
//! silently-degraded parse.
//!
//! # Hard-fail
//!
//! Per the repo `no-optionality` doctrine, every malformed or out-of-scope query
//! is a typed [`ParseError`]; there is no lenient mode and no partial algebra.
//!
//! # S6 reasoner-delegation seam
//!
//! The algebra is a faithful, standard, evaluable IR. Exploiting the native
//! OWL/EL-DL reasoner (entailment-regime-aware matching, path→subsumption-closure
//! delegation) is an *evaluation* concern that belongs in S6; this crate keeps
//! its own enums so S6 can grow annotations/variants without a breaking re-clone.
//! See the [`algebra`] module docs.
//!
//! # Examples
//!
//! Parse a query string into the algebra and inspect its root:
//!
//! ```
//! use purrdf_sparql_algebra::{GraphPattern, Query, SparqlParser};
//!
//! let query = SparqlParser::new()
//!     .parse_query(
//!         "SELECT ?name WHERE { <http://example.org/alice> <http://example.org/name> ?name }",
//!     )
//!     .expect("a well-formed query parses");
//!
//! let Query::Select { pattern, .. } = query else {
//!     panic!("a SELECT query parses to `Query::Select`");
//! };
//! let GraphPattern::Project { variables, inner } = pattern else {
//!     panic!("the projection wraps the root pattern");
//! };
//! assert_eq!(variables.len(), 1);
//! assert_eq!(variables[0].as_str(), "name");
//! assert!(matches!(*inner, GraphPattern::Bgp { .. }));
//! ```
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![forbid(unsafe_code)]

pub mod algebra;
pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod serialize;
pub mod substitute;

pub use algebra::{
    AggregateExpression, AggregateFunction, Expression, Function, GraphPattern, GraphTarget,
    GraphUpdateOperation, NegatedPathElement, OrderExpression, PropertyPathExpression, PurrdfCall,
    PurrdfFn, Query, QueryDataset, Update, UsingClause,
};
pub use ast::{
    BaseDirection, BlankNode, GroundTerm, GroundTriple, Literal, NamedNode, NamedNodePattern,
    QuadPattern, TermPattern, TriplePattern, Variable,
};
pub use error::{ParseError, Result};
pub use parser::{ParserOptions, SparqlParser};
pub use serialize::pattern_to_select_query;
