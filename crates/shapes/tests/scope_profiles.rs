// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! General SPARQL legality and SHACL constraint admission are separate evidence.

use purrdf_shapes::engine::parse_shapes;
use purrdf_sparql_algebra::SparqlParser;

fn constraint(query: &str) -> String {
    format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         <http://example.org/Shape> a sh:NodeShape ;\n\
         sh:targetNode <http://example.org/focus> ;\n\
         sh:sparql [ sh:select \"\"\"{query}\"\"\" ] ."
    )
}

#[test]
fn legal_sparql_constructs_have_explicit_shacl_rejection_controls() {
    for (query, reason) in [
        (
            "SELECT $this WHERE { $this <http://example.org/p> ?v MINUS { $this <http://example.org/q> ?v } }",
            "MINUS",
        ),
        (
            "SELECT $this WHERE { SERVICE <http://example.org/remote> { $this <http://example.org/p> ?v } }",
            "SERVICE",
        ),
        (
            "SELECT $this WHERE { VALUES $this { <http://example.org/focus> } }",
            "VALUES",
        ),
        (
            "SELECT $this WHERE { { SELECT ?v WHERE { $this <http://example.org/p> ?v } } }",
            "subquery",
        ),
        (
            "SELECT $this WHERE { BIND(<http://example.org/focus> AS $this) }",
            "pre-bound",
        ),
    ] {
        SparqlParser::new()
            .parse_query(query)
            .expect("the general SPARQL control parses");
        let error = parse_shapes(&constraint(query), None)
            .expect_err("the constraint profile refuses the construct")
            .to_string();
        assert!(error.contains(reason), "{query}: {error}");
    }
}

#[test]
fn optional_exists_paths_and_projected_subqueries_remain_legal_constraints() {
    for query in [
        "SELECT $this WHERE { OPTIONAL { $this <http://example.org/p> ?v } FILTER(!BOUND(?v)) }",
        "SELECT $this WHERE { FILTER NOT EXISTS { $this <http://example.org/p> ?v } }",
        "SELECT $this WHERE { FILTER EXISTS { $this (<http://example.org/p>|<http://example.org/q>)/<http://example.org/r> ?v } }",
        "SELECT $this WHERE { { SELECT $this ?v WHERE { $this <http://example.org/p> ?v } } }",
        "SELECT $this WHERE { $this <http://example.org/p> ?v BIND(?v AS ?local) FILTER(?local = ?v) }",
    ] {
        parse_shapes(&constraint(query), None)
            .unwrap_or_else(|error| panic!("legal constraint {query}: {error}"));
    }
}
