// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Reader for the W3C `rs:` (`http://www.w3.org/2001/sw/DataAccess/tests/result-set#`)
//! Turtle result-set encoding.
//!
//! A handful of `mf:QueryEvaluationTest` cases (e.g. `bindings/manifest#graph`)
//! ship their expected SELECT result as a Turtle graph *describing* an
//! `rs:ResultSet` — `rs:resultVariable` literals for the header and
//! `rs:solution`/`rs:binding`/`rs:variable`/`rs:value` for each row — rather
//! than SPARQL Results XML/JSON. A `.ttl` result file is therefore not always a
//! CONSTRUCT graph, and the distinction is load-bearing: reading one that
//! describes an `rs:ResultSet` as a graph leaves the SELECT case uncomparable,
//! which is a gap in the ledger rather than a case that ran.
//!
//! This module decodes that Turtle encoding into the same [`ParsedSolutions`]
//! shape [`crate::compare`] already compares SRX/SRJ results against, by
//! parsing the file with the native Turtle codec and querying its structure
//! with the native SPARQL engine — the same dog-fooding [`crate::manifest`]
//! uses to read the `mf:`/`ut:` test-manifest vocabulary itself.

use std::collections::BTreeMap;

use purrdf_core::TermValue;
use purrdf_sparql_results::ParsedSolutions;

use crate::manifest::query_rows;

/// The `rs:` vocabulary namespace.
const RS_NS: &str = "http://www.w3.org/2001/sw/DataAccess/tests/result-set#";

/// Parse an `rs:ResultSet` Turtle document into [`ParsedSolutions`].
///
/// `base` is the declaring manifest's own sentinel base
/// ([`crate::manifest::SparqlTestCase::base`]), so a relative IRI inside the
/// result document resolves to exactly the IRI the same reference in the case's
/// `qt:data` fixture produced. Using a base shared by every manifest would make
/// two different groups' relative references collide on one IRI.
///
/// # Errors
///
/// Returns a message if the bytes do not parse as Turtle, or if the graph does
/// not carry a well-formed `rs:ResultSet` shape (a binding missing its
/// `rs:variable`/`rs:value`, etc).
pub fn parse(base: &str, bytes: &[u8]) -> Result<ParsedSolutions, String> {
    let dataset = purrdf::parse_dataset(bytes, "text/turtle", Some(base))
        .map_err(|e| format!("parse rs:ResultSet turtle: {e}"))?;

    // The result variables (`rs:resultVariable` literals). Order does not
    // matter for solution-multiset equality (`compare::compare_solutions`
    // keys every cell by variable name), so a stable alphabetical order is
    // fine and keeps this deterministic.
    let var_query = format!(
        "PREFIX rs: <{RS_NS}>\n\
         SELECT ?var WHERE {{ ?rs a rs:ResultSet ; rs:resultVariable ?var }}"
    );
    let var_rows = query_rows(&dataset, &var_query)?;
    let variables: Vec<String> = var_rows
        .iter()
        .filter_map(|row| lexical(row.get("var")))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    // Every (solution, variable-name, value) triple. `?sol` is a solution's
    // blank node; grouped below into one row per distinct solution.
    let binding_query = format!(
        "PREFIX rs: <{RS_NS}>\n\
         SELECT ?sol ?varName ?val WHERE {{\n\
         ?rs a rs:ResultSet ; rs:solution ?sol .\n\
         ?sol rs:binding ?b .\n\
         ?b rs:variable ?varName ; rs:value ?val .\n\
         }}"
    );
    let binding_rows = query_rows(&dataset, &binding_query)?;

    // Group bindings by solution. The grouping key is a `Debug`-formatted
    // discriminator over the solution's (opaque, per-parse) blank node — it is
    // used ONLY to bucket rows together here and is never compared or emitted,
    // so any stable, distinct-per-solution string is correct.
    let mut groups: BTreeMap<String, BTreeMap<String, TermValue>> = BTreeMap::new();
    for row in &binding_rows {
        let sol = row
            .get("sol")
            .ok_or("rs:ResultSet solution row missing ?sol")?;
        let var_name =
            lexical(row.get("varName")).ok_or("rs:binding has no literal rs:variable")?;
        let val = row.get("val").ok_or("rs:binding has no rs:value")?.clone();
        groups
            .entry(format!("{sol:?}"))
            .or_default()
            .insert(var_name, val);
    }

    let rows: Vec<Vec<Option<TermValue>>> = groups
        .into_values()
        .map(|mut binding| variables.iter().map(|v| binding.remove(v)).collect())
        .collect();

    Ok(ParsedSolutions { variables, rows })
}

/// The lexical form of a bound literal term, if any.
fn lexical(term: Option<&TermValue>) -> Option<String> {
    match term {
        Some(TermValue::Literal { lexical_form, .. }) => Some(lexical_form.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRAPH_TTL: &str = r#"@prefix rs: <http://www.w3.org/2001/sw/DataAccess/tests/result-set#> .

[]  a rs:ResultSet ;
    rs:resultVariable
                "g" , "t" ;
    rs:solution [ rs:binding  [ rs:value    <empty.ttl> ;
                                rs:variable "g"
                              ] ;
                  rs:binding  [ rs:value    "foo";
                                rs:variable "t"
                              ]
                ] ;
    rs:solution [ rs:binding  [ rs:value    <empty.ttl> ;
                                rs:variable "g"
                              ] ;
                  rs:binding  [ rs:value    "bar";
                                rs:variable "t"
                              ]
                ] ;
    rs:solution [ rs:binding  [ rs:value    <data02.ttl> ;
                                rs:variable "g"
                              ] ;
                  rs:binding  [ rs:value    "foo";
                                rs:variable "t"
                              ]
                ] .
"#;

    #[test]
    fn parses_the_bindings_graph_fixture_shape() {
        let parsed =
            parse(crate::manifest::BASE_ROOT, GRAPH_TTL.as_bytes()).expect("parse rs:ResultSet");
        assert_eq!(parsed.variables, vec!["g".to_owned(), "t".to_owned()]);
        assert_eq!(parsed.rows.len(), 3);
        for row in &parsed.rows {
            assert!(
                matches!(row[0], Some(TermValue::Iri(_))),
                "g is a graph IRI"
            );
            assert!(
                matches!(&row[1], Some(TermValue::Literal { lexical_form, .. }) if lexical_form == "foo" || lexical_form == "bar"),
                "t is foo/bar"
            );
        }
    }

    #[test]
    fn rejects_non_turtle_bytes() {
        assert!(parse(crate::manifest::BASE_ROOT, b"not turtle { }").is_err());
    }
}
