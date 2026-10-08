// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reader for the W3C `rs:` (`http://www.w3.org/2001/sw/DataAccess/tests/result-set#`)
//! RDF result-set encoding (Turtle or RDF/XML).
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
//! This module decodes the RDF encoding into the same [`ParsedSolutions`]
//! shape [`crate::compare`] already compares SRX/SRJ results against, by
//! parsing the file with the native Turtle codec and querying its structure
//! with the native SPARQL engine — the same dog-fooding [`crate::manifest`]
//! uses to read the `mf:`/`ut:` test-manifest vocabulary itself.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::TermValue;
use purrdf_sparql_results::ParsedSolutions;

use crate::manifest::query_rows;

/// The `rs:` vocabulary namespace.
const RS_NS: &str = purrdf_iri::vocab::rs::NS;

/// The two result forms the W3C DAWG RDF vocabulary carries.
#[derive(Debug)]
pub enum RdfResult {
    /// An ASK result.
    Boolean(bool),
    /// SELECT rows, ordered only when the document explicitly supplies indices.
    Solutions {
        /// The header and rows shared with the XML/JSON readers.
        solutions: ParsedSolutions,
        /// Every solution has a unique, consecutive `rs:index`.
        ordered: bool,
    },
}

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
    match parse_result(base, "text/turtle", bytes)? {
        RdfResult::Solutions { solutions, .. } => Ok(solutions),
        RdfResult::Boolean(_) => Err("an ASK result is not a SELECT solution sequence".to_owned()),
    }
}

/// Decode a Turtle or RDF/XML DAWG result set, including ASK and indexed rows.
///
/// The W3C test-case rules make `rs:index`, rather than RDF document order,
/// authoritative: <https://www.w3.org/2009/sparql/docs/tests/README.html>.
///
/// # Errors
/// Refuses malformed result roots, boolean values, bindings or row indices.
pub fn parse_result(base: &str, media: &str, bytes: &[u8]) -> Result<RdfResult, String> {
    let dataset = purrdf::parse_dataset(bytes, media, Some(base))
        .map_err(|e| format!("parse rs:ResultSet: {e}"))?;
    let roots = query_rows(
        &dataset,
        &format!("PREFIX rs: <{RS_NS}> SELECT ?rs WHERE {{ ?rs a rs:ResultSet }}"),
    )?;
    if roots.len() != 1 {
        return Err("the document must declare exactly one rs:ResultSet".to_owned());
    }
    let booleans = query_rows(
        &dataset,
        &format!(
            "PREFIX rs: <{RS_NS}> SELECT ?value WHERE {{ ?rs a rs:ResultSet ; rs:boolean ?value }}"
        ),
    )?;
    if !booleans.is_empty() {
        if booleans.len() != 1 {
            return Err("an ASK result must declare exactly one rs:boolean".to_owned());
        }
        let Some(TermValue::Literal {
            lexical_form,
            datatype,
            ..
        }) = booleans[0].get("value")
        else {
            return Err("rs:boolean must be an xsd:boolean literal".to_owned());
        };
        if datatype != purrdf_core::datatype::XSD_BOOLEAN {
            return Err("rs:boolean must be an xsd:boolean literal".to_owned());
        }
        let value = purrdf::xsd::simple::parse_boolean(lexical_form)
            .map_err(|error| format!("rs:boolean: {error}"))?;
        let select_members = query_rows(
            &dataset,
            &format!(
                "PREFIX rs: <{RS_NS}> SELECT ?value WHERE {{ ?rs a rs:ResultSet ; ?p ?value . \
             FILTER(?p = rs:solution || ?p = rs:resultVariable) }}"
            ),
        )?;
        if !select_members.is_empty() {
            return Err("an ASK result may not also declare SELECT rows or variables".to_owned());
        }
        return Ok(RdfResult::Boolean(value));
    }

    parse_solutions(&dataset)
}

/// Decode SELECT headers, bindings and the explicitly declared row ordering.
fn parse_solutions(dataset: &Arc<purrdf_core::RdfDataset>) -> Result<RdfResult, String> {
    // The result variables (`rs:resultVariable` literals). Order does not
    // matter for solution-multiset equality (`compare::compare_solutions`
    // keys every cell by variable name), so a stable alphabetical order is
    // fine and keeps this deterministic.
    let var_query = format!(
        "PREFIX rs: <{RS_NS}>\n\
         SELECT ?var WHERE {{ ?rs a rs:ResultSet ; rs:resultVariable ?var }}"
    );
    let var_rows = query_rows(dataset, &var_query)?;
    let variables: Vec<String> = var_rows
        .iter()
        .map(|row| lexical(row.get("var")).ok_or("rs:resultVariable must be a literal"))
        .collect::<Result<BTreeSet<_>, _>>()?
        .into_iter()
        .collect();

    let mut groups = solution_groups(dataset)?;

    // Every (solution, variable-name, value) triple. `?sol` is a solution's
    // blank node; grouped below into one row per distinct solution.
    let binding_query = format!(
        "PREFIX rs: <{RS_NS}>\n\
         SELECT ?sol ?varName ?val WHERE {{\n\
         ?rs a rs:ResultSet ; rs:solution ?sol .\n\
         ?sol rs:binding ?b .\n\
         OPTIONAL {{ ?b rs:variable ?varName }} OPTIONAL {{ ?b rs:value ?val }}\n\
         }}"
    );
    let binding_rows = query_rows(dataset, &binding_query)?;

    // Group bindings by solution. The grouping key is a `Debug`-formatted
    // discriminator over the solution's (opaque, per-parse) blank node — it is
    // used ONLY to bucket rows together here and is never compared or emitted,
    // so any stable, distinct-per-solution string is correct.
    for row in &binding_rows {
        let sol = row
            .get("sol")
            .ok_or("rs:ResultSet solution row missing ?sol")?;
        let var_name =
            lexical(row.get("varName")).ok_or("rs:binding has no literal rs:variable")?;
        let val = row.get("val").ok_or("rs:binding has no rs:value")?.clone();
        if !variables.contains(&var_name) {
            return Err(format!("binding for undeclared result variable {var_name}"));
        }
        let (_, bindings) = groups
            .get_mut(&format!("{sol:?}"))
            .ok_or("binding has no solution")?;
        if bindings.insert(var_name, val).is_some() {
            return Err("a solution binds one variable more than once".to_owned());
        }
    }
    let mut groups: Vec<_> = groups.into_values().collect();
    let ordered = groups.iter().any(|(index, _)| index.is_some());
    if ordered {
        groups.sort_by_key(|(index, _)| *index);
        if groups
            .iter()
            .enumerate()
            .any(|(at, (index, _))| *index != Some(at + 1))
        {
            return Err(
                "rs:index values must cover every solution exactly once, starting at one"
                    .to_owned(),
            );
        }
    }
    let rows: Vec<Vec<Option<TermValue>>> = groups
        .into_iter()
        .map(|(_, mut binding)| variables.iter().map(|v| binding.remove(v)).collect())
        .collect();

    Ok(RdfResult::Solutions {
        solutions: ParsedSolutions { variables, rows },
        ordered,
    })
}

/// Bindings grouped by a stable opaque solution node, with its optional ordinal.
type SolutionGroups = BTreeMap<String, (Option<usize>, BTreeMap<String, TermValue>)>;

/// Read every declared solution, including rows with no bound cells.
fn solution_groups(dataset: &Arc<purrdf_core::RdfDataset>) -> Result<SolutionGroups, String> {
    // Seed every solution before reading bindings, preserving completely unbound
    // rows. An RDF graph's serialization order never defines solution order.
    let solution_rows = query_rows(
        dataset,
        &format!(
            "PREFIX rs: <{RS_NS}> SELECT ?sol ?index WHERE {{ \
         ?rs a rs:ResultSet ; rs:solution ?sol . OPTIONAL {{ ?sol rs:index ?index }} }}"
        ),
    )?;
    let mut groups = SolutionGroups::new();
    for row in solution_rows {
        let solution = row.get("sol").ok_or("solution has no node")?;
        let index = row
            .get("index")
            .map(|term| {
                if let TermValue::Literal {
                    lexical_form,
                    datatype,
                    ..
                } = term
                    && datatype == purrdf_core::datatype::XSD_INTEGER
                {
                    purrdf::xsd::numeric::parse_integer(lexical_form)
                        .map_err(|_| "rs:index must be an xsd:integer lexical form")
                        .and_then(|index| {
                            usize::try_from(index)
                                .map_err(|_| "rs:index must be a positive bounded integer")
                        })
                } else {
                    Err("rs:index must be an xsd:integer literal")
                }
            })
            .transpose()?;
        if groups
            .insert(format!("{solution:?}"), (index, BTreeMap::new()))
            .is_some()
        {
            return Err("a solution has more than one rs:index".to_owned());
        }
    }

    Ok(groups)
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

    #[test]
    fn indices_define_row_order_and_preserve_a_completely_unbound_solution() {
        let bytes = format!(
            "@prefix rs: <{RS_NS}> . [] a rs:ResultSet ; rs:resultVariable \"x\" ; \
             rs:solution [rs:index 2 ; rs:binding [rs:variable \"x\" ; rs:value \"second\"]], \
                         [rs:index 1] ."
        );
        let RdfResult::Solutions { solutions, ordered } =
            parse_result(crate::manifest::BASE_ROOT, "text/turtle", bytes.as_bytes())
                .expect("indexed rows")
        else {
            panic!("SELECT shape")
        };
        assert!(ordered);
        assert_eq!(solutions.rows.len(), 2);
        assert_eq!(solutions.rows[0], vec![None]);
        assert_eq!(
            solutions.rows[1],
            vec![Some(TermValue::simple_literal("second"))]
        );
    }

    #[test]
    fn incomplete_duplicate_and_invalid_indices_are_refused() {
        for indices in [
            "rs:index 0",
            "rs:index 2",
            "rs:index 1, 2",
            "rs:index \"1\"",
            "rs:index 1.0",
        ] {
            let bytes =
                format!("@prefix rs: <{RS_NS}> . [] a rs:ResultSet ; rs:solution [{indices}] .");
            assert!(
                parse_result(crate::manifest::BASE_ROOT, "text/turtle", bytes.as_bytes()).is_err(),
                "{indices}"
            );
        }
        let partial =
            format!("@prefix rs: <{RS_NS}> . [] a rs:ResultSet ; rs:solution [rs:index 1], [] .");
        assert!(
            parse_result(
                crate::manifest::BASE_ROOT,
                "text/turtle",
                partial.as_bytes()
            )
            .is_err()
        );
    }

    #[test]
    fn missing_bindings_undeclared_variables_and_mixed_result_forms_are_refused() {
        for body in [
            "rs:resultVariable \"x\" ; rs:solution [rs:binding [rs:variable \"x\"]]",
            "rs:resultVariable \"x\" ; rs:solution [rs:binding [rs:value \"value\"]]",
            "rs:resultVariable \"x\" ; rs:solution [rs:binding [rs:variable \"y\" ; rs:value \"value\"]]",
            "rs:boolean true ; rs:solution []",
            "rs:boolean \"true\"",
            "rs:boolean true, false",
        ] {
            let bytes = format!("@prefix rs: <{RS_NS}> . [] a rs:ResultSet ; {body} .");
            assert!(
                parse_result(crate::manifest::BASE_ROOT, "text/turtle", bytes.as_bytes()).is_err(),
                "{body}"
            );
        }
        assert!(
            parse_result(
                crate::manifest::BASE_ROOT,
                "text/turtle",
                b"<urn:s> <urn:p> <urn:o> ."
            )
            .is_err()
        );
    }
}
