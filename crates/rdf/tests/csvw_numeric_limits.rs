// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! CSVW numbers past `purrdf_xsd`'s bounded representation, through the public
//! [`read_csvw`] API: a 42-digit `xsd:integer` cell is a well-typed value, its
//! datatype facets compare it exactly, and a malformed facet is refused with the XPath
//! F&O code of the `purrdf_xsd` error behind it.

use std::collections::BTreeMap;

use purrdf_rdf::{
    CsvwAction, CsvwConfig, CsvwContext, CsvwInput, CsvwMode, CsvwVocabulary, ProjectionLimits,
    read_csvw,
};

const BASE: &str = "http://example.org/csvw/";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

fn config() -> CsvwConfig {
    CsvwConfig::new(
        BASE,
        CsvwContext::new(
            "http://www.w3.org/ns/csvw",
            BTreeMap::from([("xsd".to_owned(), XSD.to_owned())]),
        )
        .expect("context"),
        "http://example.org/csvw/group",
        CsvwVocabulary::new(
            "http://www.w3.org/ns/csvw#",
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
            "http://www.w3.org/2000/01/rdf-schema#",
            XSD,
        )
        .expect("vocabulary"),
        CsvwMode::Minimal,
        ProjectionLimits::new(16, 1_000_000, 8_000_000, 16_000_000, 16).expect("limits"),
        10_000,
    )
    .expect("config")
}

/// Read `csv` under metadata whose `n` column is `xsd:integer` with `facet`.
fn read(
    csv: &str,
    facet: &str,
) -> Result<purrdf_rdf::CsvwReadOutcome, purrdf_rdf::ProjectionError> {
    let metadata = format!(
        r#"{{"@context":"http://www.w3.org/ns/csvw","url":"data.csv",
            "tableSchema":{{"columns":[{{"name":"n","titles":"n",
            "datatype":{{"base":"integer"{facet}}}}}]}}}}"#
    );
    let resources = BTreeMap::from([
        (format!("{BASE}data.csv"), csv.as_bytes().to_vec()),
        (
            format!("{BASE}data.csv-metadata.json"),
            metadata.into_bytes(),
        ),
    ]);
    let config = config();
    let input = CsvwInput::new(
        CsvwAction::Metadata {
            metadata_iri: format!("{BASE}data.csv-metadata.json"),
        },
        resources,
        config.limits(),
    )?;
    read_csvw(&input, &config)
}

/// The read's dataset as N-Triples.
fn ntriples(outcome: &purrdf_rdf::CsvwReadOutcome) -> String {
    let bytes = purrdf_rdf::serialize_dataset(
        &*outcome.dataset,
        "application/n-triples",
        purrdf_rdf::SerializeGraph::Dataset,
    )
    .expect("serializes");
    String::from_utf8(bytes).expect("UTF-8")
}

/// Whether the read holds the typed literal `"lexical"^^<datatype>`.
fn holds(outcome: &purrdf_rdf::CsvwReadOutcome, lexical: &str, datatype: &str) -> bool {
    ntriples(outcome).contains(&format!("\"{lexical}\"^^<{XSD}{datatype}>"))
}

const BIG: &str = "100000000000000000000000000000000000000000";
const BIG_1: &str = "99999999999999999999999999999999999999999";

#[test]
fn a_42_digit_cell_is_a_well_typed_integer() {
    let outcome = read(&format!("n\n{BIG}\n"), "").expect("reads");
    assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);
    assert!(holds(&outcome, BIG, "integer"), "{}", ntriples(&outcome));
}

#[test]
fn datatype_facets_compare_a_42_digit_cell_exactly() {
    let facet = format!(r#","minimum":"{BIG_1}""#);
    let outcome = read(&format!("n\n{BIG}\n"), &facet).expect("reads");
    assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);
    assert!(holds(&outcome, BIG, "integer"), "{}", ntriples(&outcome));
    // One below the minimum is a validation warning, and the cell stays a string.
    let below = "99999999999999999999999999999999999999998";
    let outcome = read(&format!("n\n{below}\n"), &facet).expect("reads");
    assert_eq!(outcome.warnings.len(), 1, "{:?}", outcome.warnings);
    let text = ntriples(&outcome);
    assert!(
        text.contains(&format!("\"{below}\"")) && !holds(&outcome, below, "integer"),
        "{text}"
    );
}

#[test]
fn a_malformed_facet_names_its_fo_code() {
    let error = read("n\n1\n", r#","minimum":"1.5","maximum":"10""#).expect_err("a refused facet");
    assert!(error.to_string().contains("err:FORG0001"), "{error}");
    let presentation = error.presentation().expect("a typed presentation");
    assert_eq!(presentation.message_id(), "xsd-invalid-lexical");
    assert!(
        presentation
            .parameters()
            .iter()
            .any(|p| p.name() == "code" && format!("{:?}", p.value()).contains("FORG0001")),
        "{presentation:?}"
    );
}
