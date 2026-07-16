// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Emit a deterministic CSVW package for the independent development oracle.

use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use purrdf_rdf::{
    CsvwAction, CsvwConfig, CsvwContext, CsvwInput, CsvwMode, CsvwVocabulary, CsvwWritePlan,
    ProjectionLimits, read_csvw, write_csvw,
};

fn main() -> Result<(), Box<dyn Error>> {
    let output = PathBuf::from(
        env::args_os()
            .nth(1)
            .ok_or("usage: write_csvw_oracle_fixture OUTPUT_DIRECTORY")?,
    );
    fs::create_dir_all(&output)?;
    let output = fs::canonicalize(output)?;
    let output_text = output
        .to_str()
        .ok_or("oracle output directory must be valid UTF-8")?;
    if output_text.contains([' ', '#', '?', '%']) {
        return Err("oracle output directory contains characters requiring IRI escaping".into());
    }

    let root = format!("file://{output_text}");
    let metadata_iri = format!("{root}/csvw-metadata.json");
    let parents_iri = format!("{root}/tables/parents.csv");
    let children_iri = format!("{root}/tables/children.csv");
    let metadata = serde_json::to_vec(&serde_json::json!({
        "@context": "http://www.w3.org/ns/csvw",
        "@type": "TableGroup",
        "tables": [
            {
                "url": parents_iri,
                "tableSchema": {
                    "columns": [
                        {"name": "id", "titles": "id", "datatype": "integer", "required": true},
                        {"name": "label", "titles": "label", "datatype": "string", "required": true}
                    ],
                    "primaryKey": "id"
                }
            },
            {
                "url": children_iri,
                "tableSchema": {
                    "columns": [
                        {"name": "id", "titles": "id", "datatype": "integer", "required": true},
                        {"name": "parent", "titles": "parent", "datatype": "integer", "required": true},
                        {"name": "amount", "titles": "amount", "datatype": "decimal", "required": true}
                    ],
                    "primaryKey": "id",
                    "foreignKeys": [{
                        "columnReference": "parent",
                        "reference": {
                            "resource": parents_iri,
                            "columnReference": "id"
                        }
                    }]
                }
            }
        ]
    }))?;
    let limits = ProjectionLimits::new(16, 1_000_000, 2_000_000, 4_000_000, 16)?;
    let config = CsvwConfig::new(
        &metadata_iri,
        CsvwContext::new("http://www.w3.org/ns/csvw", BTreeMap::new())?,
        "http://example.org/purrdf/csvw-oracle-group",
        CsvwVocabulary::new(
            "http://www.w3.org/ns/csvw#",
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
            "http://www.w3.org/2000/01/rdf-schema#",
            "http://www.w3.org/2001/XMLSchema#",
        )?,
        CsvwMode::Standard,
        limits,
        1_000,
    )?;
    let input = CsvwInput::new(
        CsvwAction::Metadata {
            metadata_iri: metadata_iri.clone(),
        },
        BTreeMap::from([
            (metadata_iri, metadata),
            (parents_iri.clone(), b"id,label\n1,Alpha\n2,Beta\n".to_vec()),
            (
                children_iri.clone(),
                b"id,parent,amount\n10,1,12.50\n11,2,7.25\n".to_vec(),
            ),
        ]),
        limits,
    )?;
    let read = read_csvw(&input, &config)?;
    if !read.is_valid() || !read.loss_ledger.is_empty() {
        return Err("oracle source fixture did not read losslessly and validly".into());
    }
    let plan = CsvwWritePlan::new(
        "csvw-metadata.json",
        BTreeMap::from([
            (parents_iri, "tables/parents.csv".to_owned()),
            (children_iri, "tables/children.csv".to_owned()),
        ]),
    )?;
    let written = write_csvw(&read.group, &plan, &config)?;
    if !written.loss_ledger.is_empty() {
        return Err("canonical CSVW writer unexpectedly reported a loss".into());
    }
    for (path, bytes) in written.package.artifacts() {
        let target = output.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, bytes)?;
    }
    Ok(())
}
