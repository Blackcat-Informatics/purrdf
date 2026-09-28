// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The value-position shape-constraint fixture every emitter oracle runs.
//!
//! One shapes graph whose property shapes judge their values against shapes —
//! `sh:someValue`, `sh:node`, `sh:and`, `sh:or`, `sh:xone` and `sh:not` — compiled
//! to JSON Schema; and data variants, each validated by SHACL and projected to its
//! JSON-LD node, so an oracle's probes are real projected instances whose source
//! verdict is SHACL validation's (checked here to be the compiled schema's too).

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

/// One shape-based constraint on each property.
const SHAPES: &str = r#"
    ex:HolderShape a sh:NodeShape ; sh:targetClass ex:Holder ;
        sh:property [ sh:path ex:some ; sh:someValue [ sh:datatype xsd:integer ] ] ;
        sh:property [ sh:path ex:node ; sh:node [ sh:datatype xsd:integer ] ] ;
        sh:property [ sh:path ex:and ; sh:maxCount 1 ;
                      sh:and ( [ sh:datatype xsd:integer ] [ sh:in ( 1 2 ) ] ) ] ;
        sh:property [ sh:path ex:or ; sh:maxCount 1 ;
                      sh:or ( [ sh:datatype xsd:integer ] [ sh:nodeKind sh:IRI ] ) ] ;
        sh:property [ sh:path ex:xone ; sh:maxCount 1 ;
                      sh:xone ( [ sh:datatype xsd:integer ] [ sh:in ( 1 "a" ) ] ) ] ;
        sh:property [ sh:path ex:not ; sh:maxCount 1 ; sh:not [ sh:datatype xsd:integer ] ] .
"#;

/// The conforming data every variant replaces one property of.
const BASE: [(&str, &str); 6] = [
    ("ex:some", r#""a", 2"#),
    ("ex:node", "1, 2"),
    ("ex:and", "1"),
    ("ex:or", "ex:x"),
    ("ex:xone", "2"),
    ("ex:not", r#""a""#),
];

/// `(label, property, replacement value)`; the first row keeps the base.
pub(crate) const VARIANTS: [(&str, &str, &str); 9] = [
    ("conforming", "ex:some", r#""a", 2"#),
    ("some-lone-member", "ex:some", "7"),
    ("some-none", "ex:some", r#""a", "b""#),
    ("node-one-not-integer", "ex:node", r#"1, "a""#),
    ("and-outside-in", "ex:and", "3"),
    ("or-neither", "ex:or", r#""a""#),
    ("xone-both", "ex:xone", "1"),
    ("xone-other", "ex:xone", r#""a""#),
    ("not-integer", "ex:not", "5"),
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

/// Every variant, projected and validated, its verdict checked to be the
/// compiled JSON Schema's.
pub(crate) fn cases() -> Result<Vec<Case>, Box<dyn Error>> {
    let shapes = parse_shapes(&format!("{PREFIXES}{SHAPES}"), None)?;
    let namespaces = namespaces()?;
    let schema: Value = serde_json::from_str(&compiled()?.schema_json)?;
    let location = "mem:///value-shapes.schema.json";
    let mut registry = purrdf_jsonschema::Registry::with_metaschemas(metaschemas());
    registry.add_resource(location, schema)?;
    let holder = registry.compile(&format!("{location}#/$defs/Holder"))?;
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
            if holder.is_valid(&value)? != report.conforms {
                return Err(format!(
                    "value-shape variant {label:?}: the compiled schema disagrees with SHACL"
                )
                .into());
            }
            Ok(Case {
                label,
                value,
                conforms: report.conforms,
            })
        })
        .collect()
}

fn metaschemas() -> &'static purrdf_jsonschema::Metaschemas {
    static SET: std::sync::OnceLock<purrdf_jsonschema::Metaschemas> = std::sync::OnceLock::new();
    SET.get_or_init(|| {
        purrdf_jsonschema::Metaschemas::new(
            purrdf_testkit::jsonschema_metaschemas::DRAFT_2020_12
                .iter()
                .map(|&(uri, text)| {
                    let document: Value = serde_json::from_str(text).expect("meta-schema JSON");
                    (uri, document)
                }),
        )
        .expect("the draft 2020-12 meta-schemas")
    })
}
