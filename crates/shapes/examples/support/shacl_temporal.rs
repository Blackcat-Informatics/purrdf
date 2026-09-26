// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The temporal range-bound fixture every emitter oracle runs.
//!
//! One shapes graph bounding an `xsd:date`, an `xsd:dateTime` and an
//! `xsd:time` value, compiled to JSON Schema; and data variants, each
//! validated by SHACL and projected to its JSON-LD node, so an oracle's probes
//! are real projected instances whose source verdict is SHACL validation's.
//! The variants cross the XSD timeline's edges: a timezone against a local
//! bound and the reverse (comparable only beyond ±14:00), `24:00:00`, a leap
//! day, and values of another datatype.

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

/// A local lower and a zoned upper date bound, a zoned exclusive instant
/// bound, and a local upper time bound.
const SHAPES: &str = r#"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
        sh:property [ sh:path ex:day ; sh:maxCount 1 ;
                      sh:minInclusive "2020-03-01"^^xsd:date ;
                      sh:maxExclusive "2021-01-01Z"^^xsd:date ] ;
        sh:property [ sh:path ex:at ; sh:maxCount 1 ;
                      sh:minExclusive "2020-03-01T12:00:00Z"^^xsd:dateTime ] ;
        sh:property [ sh:path ex:clock ; sh:maxCount 1 ;
                      sh:maxInclusive "12:00:00"^^xsd:time ] .
"#;

/// The conforming data every variant replaces one property of.
const BASE: [(&str, &str); 3] = [
    ("ex:day", r#""2020-06-01"^^xsd:date"#),
    ("ex:at", r#""2020-03-01T12:00:01Z"^^xsd:dateTime"#),
    ("ex:clock", r#""11:00:00"^^xsd:time"#),
];

/// `(label, property, replacement value)`; the first row keeps the base.
pub(crate) const VARIANTS: [(&str, &str, &str); 12] = [
    ("conforming", "ex:day", r#""2020-06-01"^^xsd:date"#),
    (
        "day-zoned-inside",
        "ex:day",
        r#""2020-12-31+14:00"^^xsd:date"#,
    ),
    ("day-too-early", "ex:day", r#""2020-02-29"^^xsd:date"#),
    (
        "day-zone-incomparable",
        "ex:day",
        r#""2020-03-01Z"^^xsd:date"#,
    ),
    ("day-not-a-leap-day", "ex:day", r#""2021-02-29"^^xsd:date"#),
    (
        "at-equal",
        "ex:at",
        r#""2020-03-01T12:00:00Z"^^xsd:dateTime"#,
    ),
    (
        "at-offset-later",
        "ex:at",
        r#""2020-03-01T17:30:01+05:30"^^xsd:dateTime"#,
    ),
    (
        "at-local-in-window",
        "ex:at",
        r#""2020-03-02T02:00:00"^^xsd:dateTime"#,
    ),
    (
        "at-end-of-day",
        "ex:at",
        r#""2020-03-01T24:00:00Z"^^xsd:dateTime"#,
    ),
    ("clock-late", "ex:clock", r#""12:00:00.5"^^xsd:time"#),
    (
        "clock-zoned-early",
        "ex:clock",
        r#""01:59:59+14:00"^^xsd:time"#,
    ),
    ("clock-a-string", "ex:clock", r#""noon""#),
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
