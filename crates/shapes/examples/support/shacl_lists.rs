// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The SHACL 1.2 list-component fixture every emitter oracle runs.
//!
//! One shapes graph using `sh:minListLength`, `sh:maxListLength`,
//! `sh:uniqueMembers` and `sh:memberShape`, compiled to JSON Schema; and data
//! variants, each validated by SHACL and projected to its JSON-LD node, so an
//! oracle's probes are real projected instances whose source verdict is SHACL
//! validation's (checked here to be the compiled schema's too).

use std::error::Error;
use std::fmt::Write as _;

use purrdf_shapes::engine::{parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::json_schema::{CompiledSchema, Namespaces, compile};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;
use serde_json::Value;

const PREFIXES: &str = r"
    @prefix sh:  <http://www.w3.org/ns/shacl#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    @prefix ex:  <https://example.org/> .
";

/// Each list component on its own property; members of `ex:bounded` and
/// `ex:unique` are strings, `ex:members` holds its members to non-negative
/// integers.
const SHAPES: &str = r"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
        sh:property [ sh:path ex:bounded ; sh:maxCount 1 ; sh:minListLength 2 ; sh:maxListLength 3 ] ;
        sh:property [ sh:path ex:unique ; sh:maxCount 1 ; sh:uniqueMembers true ] ;
        sh:property [ sh:path ex:members ; sh:maxCount 1 ;
                      sh:memberShape [ sh:datatype xsd:integer ; sh:minInclusive 0 ] ] .
";

/// The conforming data every variant replaces one property of.
const BASE: [(&str, &str); 3] = [
    ("ex:bounded", r#"( "a" "b" )"#),
    ("ex:unique", r#"( "a" "b" )"#),
    ("ex:members", "( 0 5 )"),
];

/// `(label, property, replacement value)`; the first row keeps the base.
pub(crate) const VARIANTS: [(&str, &str, &str); 8] = [
    ("conforming", "ex:bounded", r#"( "a" "b" )"#),
    ("bounded-at-maximum", "ex:bounded", r#"( "a" "b" "c" )"#),
    ("bounded-too-short", "ex:bounded", r#"( "a" )"#),
    ("bounded-too-long", "ex:bounded", r#"( "a" "b" "c" "d" )"#),
    ("unique-repeated", "ex:unique", r#"( "a" "a" )"#),
    ("member-negative", "ex:members", "( 0 -1 )"),
    ("member-not-integer", "ex:members", r#"( 0 "x" )"#),
    ("bounded-not-a-list", "ex:bounded", r#""a""#),
];

/// The namespace table every oracle compiles and projects with.
pub(crate) fn namespaces() -> Result<Namespaces, Box<dyn Error>> {
    Ok(Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/".to_owned())],
    )?)
}

/// The shapes graph compiled to JSON Schema.
pub(crate) fn compiled() -> Result<CompiledSchema, Box<dyn Error>> {
    let shapes = parse_shapes(&format!("{PREFIXES}{SHAPES}"), None)?;
    Ok(compile(&shapes, &namespaces()?)?)
}

/// One data variant: its label, projected `Holder` node, and SHACL verdict.
pub(crate) struct Case {
    pub(crate) label: &'static str,
    pub(crate) value: Value,
    pub(crate) conforms: bool,
}

/// Every variant, projected and validated.
pub(crate) fn cases() -> Result<Vec<Case>, Box<dyn Error>> {
    let shapes = parse_shapes(&format!("{PREFIXES}{SHAPES}"), None)?;
    let namespaces = namespaces()?;
    VARIANTS
        .iter()
        .map(|&(label, property, replacement)| {
            let mut data = format!("{PREFIXES}\nex:h a ex:Holder");
            for (key, value) in BASE {
                let value = if key == property { replacement } else { value };
                write!(data, " ; {key} {value}")?;
            }
            data.push_str(" .\n");
            let dataset =
                parse_turtle_to_dataset(&data, None).map_err(|errors| format!("{errors:?}"))?;
            let report = validate_dataset_with_shapes_graph(&dataset, &shapes, None)?;
            let projected = purrdf_shapes::instance::project_graph(&dataset, &namespaces);
            let value = projected["@graph"]
                .as_array()
                .and_then(|nodes| {
                    nodes
                        .iter()
                        .find(|node| node["@id"] == "https://example.org/h")
                })
                .cloned()
                .ok_or("the Holder node is projected")?;
            Ok(Case {
                label,
                value,
                conforms: report.conforms,
            })
        })
        .collect()
}
