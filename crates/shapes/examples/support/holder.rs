// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The machinery every `Holder` fixture of the emitter oracles runs on: one
//! shapes graph compiled to JSON Schema, and data variants of one `ex:h a
//! ex:Holder` node, each validated by SHACL and projected to its JSON-LD node.
//! The fixtures themselves (`shacl_lists.rs`, `shacl_temporal.rs`,
//! `shacl_value_shapes.rs`) are only their shapes, base values and variants.

// The module is included into more than one example binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::error::Error;
use std::fmt::Write as _;

use purrdf_lex::json::Value;
use purrdf_shapes::engine::{parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::json_schema::{CompiledSchema, Namespaces, compile};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

/// The namespace table every oracle compiles and projects with.
pub(crate) fn namespaces() -> Result<Namespaces, Box<dyn Error>> {
    Ok(Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/".to_owned())],
    )?)
}

/// One data variant: its label, projected `Holder` node, and SHACL verdict.
pub(crate) struct Case {
    pub(crate) label: &'static str,
    pub(crate) value: Value,
    pub(crate) conforms: bool,
}

/// A `Holder` fixture: its Turtle prefixes and shapes graph, the base value of
/// each property, and the `(label, property, replacement)` variants.
pub(crate) struct Fixture {
    pub(crate) prefixes: &'static str,
    pub(crate) shapes: &'static str,
    pub(crate) base: &'static [(&'static str, &'static str)],
    pub(crate) variants: &'static [(&'static str, &'static str, &'static str)],
}

impl Fixture {
    /// The shapes graph compiled to JSON Schema.
    pub(crate) fn compiled(&self) -> Result<CompiledSchema, Box<dyn Error>> {
        let shapes = parse_shapes(&format!("{}{}", self.prefixes, self.shapes), None)?;
        Ok(compile(&shapes, &namespaces()?)?)
    }

    /// Every variant, projected and validated.
    pub(crate) fn cases(&self) -> Result<Vec<Case>, Box<dyn Error>> {
        let shapes = parse_shapes(&format!("{}{}", self.prefixes, self.shapes), None)?;
        let namespaces = namespaces()?;
        self.variants
            .iter()
            .map(|&(label, property, replacement)| {
                let mut data = format!("{}\nex:h a ex:Holder", self.prefixes);
                for &(key, value) in self.base {
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
}
