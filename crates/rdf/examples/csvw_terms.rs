// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Project caller-defined classes, properties, and individuals into curated CSVW tables.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use purrdf_rdf::{
    CsvwConfig, CsvwContext, CsvwDatatype, CsvwMode, CsvwNaturalLanguage, CsvwTermsCardinality,
    CsvwTermsColumn, CsvwTermsConfig, CsvwTermsGraphSelection, CsvwTermsIdentityColumn,
    CsvwTermsLimits, CsvwTermsSelector, CsvwTermsTable, CsvwTermsValueMode, CsvwVocabulary,
    ProjectionLimits, parse_dataset, project_csvw_terms,
};

const EX: &str = "https://example.org/schema/";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const TYPE: &str = "https://example.org/schema/type";
const CLASS: &str = "https://example.org/schema/Class";
const PROPERTY: &str = "https://example.org/schema/Property";
const INDIVIDUAL: &str = "https://example.org/schema/Individual";
const LABEL: &str = "https://example.org/schema/label";

fn datatype(base: impl Into<String>) -> CsvwDatatype {
    CsvwDatatype {
        id: None,
        base: base.into(),
        format: None,
        length: None,
        min_length: None,
        max_length: None,
        minimum: None,
        maximum: None,
        min_inclusive: None,
        max_inclusive: None,
        min_exclusive: None,
        max_exclusive: None,
    }
}

fn titles(title: &str) -> CsvwNaturalLanguage {
    BTreeMap::from([(String::new(), vec![title.to_owned()])])
}

fn iri_column(
    name: &str,
    title: &str,
    predicate: &str,
    cardinality: CsvwTermsCardinality,
    required: bool,
) -> Result<CsvwTermsColumn, Box<dyn Error>> {
    Ok(CsvwTermsColumn::new(
        name,
        titles(title),
        predicate,
        CsvwTermsValueMode::iri(datatype(format!("{XSD}anyURI")))?,
        cardinality,
        required,
    )?)
}

fn literal_column(
    name: &str,
    title: &str,
    predicate: &str,
) -> Result<CsvwTermsColumn, Box<dyn Error>> {
    Ok(CsvwTermsColumn::new(
        name,
        titles(title),
        predicate,
        CsvwTermsValueMode::literal(datatype(format!("{XSD}string")), None, None)?,
        CsvwTermsCardinality::One,
        false,
    )?)
}

fn table(
    name: &str,
    kind: &str,
    columns: Vec<CsvwTermsColumn>,
) -> Result<CsvwTermsTable, Box<dyn Error>> {
    let path = format!("{name}.csv");
    Ok(CsvwTermsTable::new(
        name,
        format!("https://example.org/catalog/{path}"),
        path,
        CsvwTermsSelector::new(
            Some(TYPE.to_owned()),
            BTreeSet::from([kind.to_owned()]),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::from([EX.to_owned()]),
        )?,
        CsvwTermsIdentityColumn::new("iri", titles("IRI"), datatype(format!("{XSD}anyURI")))?,
        columns,
    )?)
}

fn terms_config() -> Result<CsvwTermsConfig, Box<dyn Error>> {
    let kind = || iri_column("kind", "Kind", TYPE, CsvwTermsCardinality::One, true);
    let many = || -> Result<CsvwTermsCardinality, Box<dyn Error>> {
        Ok(CsvwTermsCardinality::many(" | ")?)
    };
    let classes = table(
        "classes",
        CLASS,
        vec![
            kind()?,
            literal_column("label", "Label", LABEL)?,
            literal_column("definition", "Definition", &format!("{EX}definition"))?,
            iri_column("parents", "Parents", &format!("{EX}parent"), many()?, false)?,
        ],
    )?;
    let properties = table(
        "properties",
        PROPERTY,
        vec![
            kind()?,
            literal_column("label", "Label", LABEL)?,
            iri_column("domains", "Domains", &format!("{EX}domain"), many()?, false)?,
            iri_column("ranges", "Ranges", &format!("{EX}range"), many()?, false)?,
        ],
    )?;
    let individuals = table(
        "individuals",
        INDIVIDUAL,
        vec![kind()?, literal_column("label", "Label", LABEL)?],
    )?;
    let csvw = CsvwConfig::new(
        "https://example.org/catalog/csvw-metadata.json",
        CsvwContext::new(
            "http://www.w3.org/ns/csvw",
            BTreeMap::from([("xsd".to_owned(), XSD.to_owned())]),
        )?,
        "https://example.org/catalog",
        CsvwVocabulary::new(
            "http://www.w3.org/ns/csvw#",
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
            "http://www.w3.org/2000/01/rdf-schema#",
            XSD,
        )?,
        CsvwMode::Minimal,
        ProjectionLimits::new(8, 1_000_000, 4_000_000, 5_000_000, 16)?,
        1_000,
    )?;
    Ok(CsvwTermsConfig::new(
        csvw,
        "csvw-metadata.json",
        CsvwTermsGraphSelection::include(true, BTreeSet::new())?,
        vec![classes, properties, individuals],
        CsvwTermsLimits::new(1_000, 10_000, 32)?,
    )?)
}

fn main() -> Result<(), Box<dyn Error>> {
    let output = PathBuf::from(
        env::args_os()
            .nth(1)
            .ok_or("usage: csvw_terms OUTPUT_USTAR")?,
    );
    let dataset = parse_dataset(
        br#"@prefix ex: <https://example.org/schema/> .
ex:Agent ex:type ex:Class ; ex:label "Agent" ; ex:definition "An acting resource" .
ex:Person ex:type ex:Class ; ex:label "Person" ; ex:definition "A human agent" ; ex:parent ex:Agent .
ex:knows ex:type ex:Property ; ex:label "knows" ; ex:domain ex:Agent, ex:Person ; ex:range ex:Person .
ex:alice ex:type ex:Individual ; ex:label "Alice" .
"#,
        "text/turtle",
        None,
    )?;
    let projection = project_csvw_terms(dataset.as_ref(), &terms_config()?)?;
    if !projection.loss_ledger.entries().is_empty() {
        return Err("example configuration failed to represent every source statement".into());
    }
    let archive = projection.package.to_ustar()?;
    fs::write(&output, &archive)?;
    println!(
        "wrote {} tables, {} rows, and {} values as {} deterministic bytes to {}",
        projection.report.tables,
        projection.report.rows,
        projection.report.values,
        archive.len(),
        output.display()
    );
    Ok(())
}
