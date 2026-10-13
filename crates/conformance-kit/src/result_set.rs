// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Structural decoding of the DAWG RDF result-set carrier.

use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::TermValue;
use purrdf_iri::vocab::{rdf, rs};
use purrdf_sparql_results::ParsedSolutions;

use crate::GradeError;
use crate::graph::Reader;

/// Decode all solution nodes, including rows without any bindings.
///
/// # Errors
/// Refuses malformed headers, bindings, unknown variables, duplicate binding
/// positions, and incomplete or conflicting row indices.
pub fn decode(reader: Reader<'_>) -> Result<ParsedSolutions, GradeError> {
    let class = reader
        .dataset
        .term_id_by_iri(rs::RESULT_SET)
        .ok_or_else(|| GradeError::Malformed("no rs:ResultSet class".to_owned()))?;
    let roots = reader.subjects(rdf::TYPE, class);
    let [root] = roots.as_slice() else {
        return Err(GradeError::Malformed(format!(
            "{} result-set roots; exactly one required",
            roots.len()
        )));
    };
    let mut names = BTreeSet::new();
    for variable in reader.objects(*root, rs::RESULT_VARIABLE) {
        let variable = reader.lexical(variable)?;
        if !names.insert(variable) {
            return Err(GradeError::Malformed(
                "duplicate rs:resultVariable".to_owned(),
            ));
        }
    }
    let variables: Vec<_> = names.into_iter().collect();
    let solutions = reader.objects(*root, rs::SOLUTION);
    let mut rows = Vec::with_capacity(solutions.len());
    let mut indices = BTreeSet::new();
    let mut indexed = 0;
    for solution in solutions {
        if !matches!(
            reader.dataset.term_value(solution),
            TermValue::Iri(_) | TermValue::Blank { .. }
        ) {
            return Err(GradeError::Malformed(
                "rs:solution is not an RDF resource".to_owned(),
            ));
        }
        let mut cells = BTreeMap::new();
        for binding in reader.objects(solution, rs::BINDING) {
            if !matches!(
                reader.dataset.term_value(binding),
                TermValue::Iri(_) | TermValue::Blank { .. }
            ) {
                return Err(GradeError::Malformed(
                    "rs:binding is not an RDF resource".to_owned(),
                ));
            }
            let variable = reader.lexical(reader.one(binding, rs::VARIABLE)?)?;
            let value = reader.dataset.term_value(reader.one(binding, rs::VALUE)?);
            if !variables.contains(&variable) {
                return Err(GradeError::Malformed(format!(
                    "binding for undeclared variable {variable}"
                )));
            }
            if cells.insert(variable, value).is_some() {
                return Err(GradeError::Malformed(
                    "duplicate variable binding in one solution".to_owned(),
                ));
            }
        }
        let index = reader
            .optional(solution, rs::INDEX)?
            .map(|id| {
                reader.lexical(id)?.parse::<usize>().map_err(|_| {
                    GradeError::Malformed("rs:index is not a nonnegative integer".to_owned())
                })
            })
            .transpose()?;
        if let Some(index) = index {
            indexed += 1;
            if !indices.insert(index) {
                return Err(GradeError::Malformed("duplicate rs:index".to_owned()));
            }
        }
        let row = variables
            .iter()
            .map(|variable| cells.remove(variable))
            .collect();
        rows.push((index, row));
    }
    if indexed != 0 && indexed != rows.len() {
        return Err(GradeError::Malformed(
            "only some solution rows have rs:index".to_owned(),
        ));
    }
    rows.sort_by_key(|(index, _)| *index);
    Ok(ParsedSolutions {
        variables,
        rows: rows.into_iter().map(|(_, row)| row).collect(),
    })
}
