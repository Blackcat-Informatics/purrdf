// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed structural RDF readers, independent of any query engine.

use purrdf_core::{DatasetView as _, GraphMatch, RdfDataset, TermId, TermValue};

use crate::GradeError;

/// A parsed document and its explicitly selected graph.
#[derive(Debug, Clone, Copy)]
pub struct Reader<'a> {
    /// The immutable input; term handles stay local to it.
    pub dataset: &'a RdfDataset,
    /// Lists and relations are read from this graph selection together.
    pub graph: GraphMatch,
}

impl Reader<'_> {
    /// Objects of one relation, without executing SPARQL.
    #[must_use]
    pub fn objects(self, subject: TermId, predicate: &str) -> Vec<TermId> {
        self.dataset
            .term_id_by_iri(predicate)
            .map_or_else(Vec::new, |predicate| {
                self.dataset.objects(subject, predicate, self.graph)
            })
    }

    /// Require exactly one object of a relation.
    ///
    /// # Errors
    /// Refuses absent or multiply defined fields rather than choosing one.
    pub fn one(self, subject: TermId, predicate: &str) -> Result<TermId, GradeError> {
        let objects = self.objects(subject, predicate);
        match objects.as_slice() {
            [object] => Ok(*object),
            _ => Err(GradeError::Malformed(format!(
                "{predicate} has {} values; exactly one required",
                objects.len()
            ))),
        }
    }

    /// Read an optional single-valued field.
    ///
    /// # Errors
    /// Refuses multiple values.
    pub fn optional(self, subject: TermId, predicate: &str) -> Result<Option<TermId>, GradeError> {
        let objects = self.objects(subject, predicate);
        match objects.as_slice() {
            [] => Ok(None),
            [object] => Ok(Some(*object)),
            _ => Err(GradeError::Malformed(format!(
                "{predicate} has {} values; at most one allowed",
                objects.len()
            ))),
        }
    }

    /// A literal field's exact lexical bytes.
    ///
    /// # Errors
    /// Refuses a non-literal term.
    pub fn lexical(self, id: TermId) -> Result<String, GradeError> {
        match self.dataset.term_value(id) {
            TermValue::Literal { lexical_form, .. } => Ok(lexical_form),
            _ => Err(GradeError::Malformed("expected literal field".to_owned())),
        }
    }

    /// An IRI field's exact expanded value.
    ///
    /// # Errors
    /// Refuses a non-IRI term.
    pub fn iri(self, id: TermId) -> Result<String, GradeError> {
        match self.dataset.term_value(id) {
            TermValue::Iri(iri) => Ok(iri),
            _ => Err(GradeError::Malformed("expected IRI field".to_owned())),
        }
    }

    /// Read a strict RDF collection through the kernel's single list authority.
    ///
    /// # Errors
    /// Refuses cycles, forks, missing links and malformed cells.
    pub fn list(self, head: TermId) -> Result<Vec<TermId>, GradeError> {
        self.dataset
            .rdf_list_strict(head, self.graph)
            .map_err(|error| GradeError::Malformed(error.to_string()))
    }

    /// Subjects of a relation to one object, in deterministic term-id order.
    #[must_use]
    pub fn subjects(self, predicate: &str, object: TermId) -> Vec<TermId> {
        let Some(predicate) = self.dataset.term_id_by_iri(predicate) else {
            return Vec::new();
        };
        let mut subjects: Vec<_> = self
            .dataset
            .quads_for_pattern(None, Some(predicate), Some(object), self.graph)
            .map(|quad| quad.s)
            .collect();
        subjects.sort_unstable();
        subjects.dedup();
        subjects
    }
}
