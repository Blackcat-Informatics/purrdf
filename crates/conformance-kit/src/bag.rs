// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Global scoped-blank equality for complete solution bags and sequences.

use std::collections::BTreeSet;

use purrdf_core::{BlankScope, RdfDatasetBuilder, RdfLiteral, TermFactory, TermValue};

use crate::GradeError;
use crate::identity::Identities;

const NS: &str = "urn:purrdf:conformance:solution:";

/// Canonical encoding with explicit header, row presence and binding position.
///
/// Every row remains present even when it has no bound cells or the header is
/// empty. Width and duplicate headers are checked before encoding. Blank
/// identities, including nested triple/CDT references, retain their original
/// `(label, scope)` partition under one global bijection.
///
/// # Errors
/// Returns a typed malformed-input or kernel canonicalization refusal.
pub fn canonical_solutions(
    variables: &[String],
    rows: &[Vec<Option<TermValue>>],
    ordered: bool,
) -> Result<String, GradeError> {
    let names: BTreeSet<&str> = variables.iter().map(String::as_str).collect();
    if names.len() != variables.len() {
        return Err(GradeError::Malformed(
            "duplicate result variable".to_owned(),
        ));
    }
    let mut builder = RdfDatasetBuilder::new();
    let mut identities = Identities::default();
    let set = builder.intern_blank("set", BlankScope(1));
    let header = builder.intern_iri(&format!("{NS}header"));
    let row_predicate = builder.intern_iri(&format!("{NS}row"));
    let binding = builder.intern_iri(&format!("{NS}binding"));
    let variable = builder.intern_iri(&format!("{NS}variable"));
    let value_predicate = builder.intern_iri(&format!("{NS}value"));
    let index = builder.intern_iri(&format!("{NS}index"));
    let width = builder.intern_iri(&format!("{NS}width"));
    let literal = |text: String| RdfLiteral {
        lexical_form: text,
        datatype: None,
        language: None,
        direction: None,
    };
    let width_value = builder.intern_literal(literal(variables.len().to_string()));
    builder.push_quad(set, width, width_value, None);
    for name in variables {
        let name = builder.intern_literal(literal(name.clone()));
        builder.push_quad(set, header, name, None);
    }
    for (row_index, row) in rows.iter().enumerate() {
        if row.len() != variables.len() {
            return Err(GradeError::Malformed(format!(
                "row {row_index} has {} cells for {} variables",
                row.len(),
                variables.len()
            )));
        }
        let row_node = builder.intern_blank(&format!("row{row_index}"), BlankScope(1));
        builder.push_quad(set, row_predicate, row_node, None);
        for (column, cell) in row.iter().enumerate() {
            if let Some(value) = cell {
                let binding_node =
                    builder.intern_blank(&format!("binding{row_index}_{column}"), BlankScope(1));
                let name = builder.intern_literal(literal(variables[column].clone()));
                let mapped = identities.map("result", value);
                let value = builder.intern_value(&mapped);
                builder.push_quad(row_node, binding, binding_node, None);
                builder.push_quad(binding_node, variable, name, None);
                builder.push_quad(binding_node, value_predicate, value, None);
            }
        }
        if ordered {
            let ordinal = builder.intern_literal(literal(row_index.to_string()));
            builder.push_quad(row_node, index, ordinal, None);
        }
    }
    let dataset = builder
        .freeze()
        .map_err(|error| GradeError::Malformed(error.to_string()))?;
    purrdf_core::try_canonicalize(&dataset)
        .map(|canonical| canonical.nquads)
        .map_err(|error| GradeError::Canonicalization(error.to_string()))
}

/// Compare complete result sets by variable name, retaining row multiplicity.
///
/// Header order is transport metadata; the exact set of unique names must agree.
/// For an ordered result, row positions remain observable.
///
/// # Errors
/// Returns a mismatch or a typed refusal for either malformed input.
pub fn compare_solutions(
    expected_variables: &[String],
    expected_rows: &[Vec<Option<TermValue>>],
    actual_variables: &[String],
    actual_rows: &[Vec<Option<TermValue>>],
    ordered: bool,
) -> Result<(), GradeError> {
    let expected = canonical_solutions(expected_variables, expected_rows, ordered)?;
    let actual = canonical_solutions(actual_variables, actual_rows, ordered)?;
    if expected == actual {
        Ok(())
    } else {
        Err(GradeError::Mismatch(format!(
            "solution {}: {} expected rows, {} actual rows",
            if ordered { "sequence" } else { "bag" },
            expected_rows.len(),
            actual_rows.len()
        )))
    }
}
