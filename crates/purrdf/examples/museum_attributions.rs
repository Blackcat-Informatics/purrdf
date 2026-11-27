// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Query two competing dates and their sources in a fictional collection.

use purrdf::sparql::{NativeSparqlEngine, from_json};
use purrdf::{SparqlEngine, SparqlRequest, parse_dataset};

const COLLECTION: &str = include_str!("museum/collection.ttl");
const QUERY: &str = include_str!("museum/attributions.rq");
const EXPECTED: &str = include_str!("museum/expected.srj");

fn check_attributions(collection: &str, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dataset = parse_dataset(collection.as_bytes(), "text/turtle", None)?;
    let result = NativeSparqlEngine::new().query(
        &dataset,
        SparqlRequest {
            query: QUERY,
            base_iri: None,
            substitutions: &[],
        },
    )?;
    let (variables, rows) = result
        .solutions()
        .ok_or_else(|| std::io::Error::other("the attribution query must return SELECT rows"))?;
    let expected = from_json(expected.as_bytes())?;
    assert_eq!(variables, expected.variables);
    assert_eq!(rows, expected.rows);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    check_attributions(COLLECTION, EXPECTED)?;

    // The notebook date changes independently of the register's date.
    let collection = COLLECTION.replacen("1723-09-08", "1724-09-08", 1);
    let expected = EXPECTED.replacen("1723-09-08", "1724-09-08", 1);
    assert_ne!(collection, COLLECTION);
    assert_ne!(expected, EXPECTED);
    check_attributions(&collection, &expected)
}

#[cfg(test)]
mod tests {
    #[test]
    fn museum_sources_and_dates_stay_paired() {
        super::main().expect("both original attributions and the documented edit match the golden");
    }
}
