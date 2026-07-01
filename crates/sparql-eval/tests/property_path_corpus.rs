// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Integration test: a real corpus-shaped property-path query evaluates end-to-end
//! through the public API (S8 #914).
//!
//! This proves the #914 gap is closed on the public path — a `rdfs:subClassOf*`
//! query (the most common corpus shape, e.g. `queries/competency/agents.rq`) is
//! parsed by [`SparqlParser`] and evaluated by [`evaluate_query`], returning
//! solutions rather than the old `EvalError::Unsupported("property path …")`.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermRef};
use purrdf_sparql_algebra::SparqlParser;
use purrdf_sparql_eval::{evaluate_query, EvalCtx, Outcome, SolutionTerm};

const RDFS_SUBCLASS: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const EX: &str = "https://example.org/";

/// A small class taxonomy: Dog ⊑ Mammal ⊑ Animal ⊑ Agent.
fn taxonomy() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let sc = b.intern_iri(RDFS_SUBCLASS.to_owned());
    let edge = |s: &str, o: &str, b: &mut RdfDatasetBuilder| {
        let s = b.intern_iri(format!("{EX}{s}"));
        let o = b.intern_iri(format!("{EX}{o}"));
        b.push_quad(s, sc, o, None);
    };
    edge("Dog", "Mammal", &mut b);
    edge("Mammal", "Animal", &mut b);
    edge("Animal", "Agent", &mut b);
    b.freeze().expect("freeze")
}

#[test]
fn subclassof_star_query_evaluates_end_to_end() {
    let ds = taxonomy();

    // The corpus shape: `?k rdfs:subClassOf* ex:Agent`.
    let query = "\
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>\n\
        PREFIX ex: <https://example.org/>\n\
        SELECT ?k WHERE { ?k rdfs:subClassOf* ex:Agent }";

    let parsed = SparqlParser::new()
        .parse_query(query)
        .expect("property-path query parses");

    let mut ctx = EvalCtx::new(&ds);
    let outcome = evaluate_query(&parsed, &mut ctx).expect("property path no longer Unsupported");

    let Outcome::Solutions(seq) = outcome else {
        panic!("SELECT must yield Solutions");
    };

    // ?k = the transitive subclasses of Agent, plus Agent itself (zero-length `*`).
    let mut got: Vec<String> = seq
        .rows
        .iter()
        .filter_map(|row| match row[0] {
            Some(SolutionTerm::Existing(id)) => match ds.resolve(id) {
                TermRef::Iri(s) => Some(s.strip_prefix(EX).unwrap_or(s).to_owned()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    got.sort();

    assert_eq!(got, vec!["Agent", "Animal", "Dog", "Mammal"]);
}
