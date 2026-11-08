// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What the emitter oracle fixtures share beyond the `Holder` fixtures: the
//! schema-import configuration, a hand-written schema wrapped as a compiled one,
//! and the loss-ledger lookup. Every including binary declares `json_model`.

// The module is included into more than one example binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::error::Error;

use purrdf_rdf::loss::LossLedger;
use purrdf_shapes::json_schema::{CompiledSchema, Namespaces};
use purrdf_shapes::{SchemaDatatypeMap, SchemaImportConfig};

use crate::json_model::{self, Value};

/// The XSD namespace the datatype map names its datatypes in.
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

/// The schema-import configuration every oracle reverses its schemas with: the
/// `ex:` namespace and the XSD datatypes.
pub(crate) fn import_config() -> Result<SchemaImportConfig, Box<dyn Error>> {
    let namespaces = Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/".to_owned())],
    )?;
    let datatypes = SchemaDatatypeMap::new(
        format!("{XSD}string"),
        format!("{XSD}boolean"),
        format!("{XSD}integer"),
        format!("{XSD}decimal"),
        format!("{XSD}dateTime"),
        format!("{XSD}date"),
        format!("{XSD}time"),
        format!("{XSD}anyURI"),
    )?;
    Ok(SchemaImportConfig::new(namespaces, datatypes))
}

/// `schema`, written by hand, as a compiled schema with no losses.
pub(crate) fn compiled(schema: &Value) -> Result<CompiledSchema, purrdf_lex::json::Error> {
    Ok(CompiledSchema {
        schema_json: format!("{}\n", json_model::write_pretty(schema)),
        openapi_json: "{}\n".to_owned(),
        losses: LossLedger::new(),
    })
}

/// Whether `losses` records `code` at the subject `location`.
pub(crate) fn has_loss(losses: &LossLedger, code: &str, location: &str) -> bool {
    losses.entries().iter().any(|entry| {
        entry.code == code
            && entry
                .location
                .as_ref()
                .and_then(|value| value.subject.as_deref())
                == Some(location)
    })
}

/// Locate a fixture assertion through the emitted, shared schema references.
/// A sole allOf reference is the value emitter's transparent conjunction.
pub(crate) fn referenced_location(schema: &Value, pointer: &str) -> Result<String, Box<dyn Error>> {
    let mut location = pointer.to_owned();
    let mut seen = std::collections::BTreeSet::new();
    loop {
        if !seen.insert(location.clone()) {
            return Err(format!("cyclic fixture schema reference at {location}").into());
        }
        let body = schema
            .pointer(
                location
                    .strip_prefix('#')
                    .ok_or("fixture pointer is not local")?,
            )
            .ok_or_else(|| format!("missing fixture schema at {location}"))?;
        let reference = body
            .get("$ref")
            .or_else(|| {
                let object = body.as_object()?;
                let [only] = object.get("allOf")?.as_array()?.as_slice() else {
                    return None;
                };
                (object.len() == 1).then(|| only.get("$ref")).flatten()
            })
            .and_then(Value::as_str);
        match reference {
            Some(reference) => reference.clone_into(&mut location),
            None => return Ok(location),
        }
    }
}
