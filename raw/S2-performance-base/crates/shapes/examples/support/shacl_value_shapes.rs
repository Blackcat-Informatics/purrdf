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

use crate::holder::{Case, Fixture};
use crate::metaschemas::metaschemas;

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

/// The fixture: its shapes graph, base values and variants.
pub(crate) const FIXTURE: Fixture = Fixture {
    prefixes: PREFIXES,
    shapes: SHAPES,
    base: &BASE,
    variants: &VARIANTS,
};

/// Every variant, projected and validated, each checked to be judged by the
/// compiled schema exactly as SHACL judged it.
pub(crate) fn cases() -> Result<Vec<Case>, Box<dyn Error>> {
    let schema = purrdf_lex::json::read(&FIXTURE.compiled()?.schema_json)?;
    let location = "mem:///value-shapes.schema.json";
    let mut registry = purrdf_jsonschema::Registry::with_metaschemas(metaschemas());
    registry.add_resource(location, schema)?;
    let holder = registry.compile(&format!("{location}#/$defs/Holder"))?;
    let cases = FIXTURE.cases()?;
    for case in &cases {
        if holder.is_valid(&case.value)? != case.conforms {
            return Err(format!(
                "value-shape variant {:?}: the compiled schema disagrees with SHACL",
                case.label
            )
            .into());
        }
    }
    Ok(cases)
}
