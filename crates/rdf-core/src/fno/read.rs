// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A strict graph profile, with every accepted row accounted for.

use super::read_error::{FnoFieldKind as Field, FnoReadError, FnoStructureKind as Shape};
use super::{
    DCTERMS_FORMAT, DCTERMS_IS_PART_OF, FNO, FNOM, FnBlank, FnFunction, FnImpl, FnMapping,
    FnOutput, FnParam, FnParamMapping, FnReturnMapping, FnoCatalog, FnoCatalogInput, OWL_ONTOLOGY,
    RDF_TYPE, RDFS_COMMENT, RDFS_LABEL, RDFS_SEE_ALSO, SKOS_DEFINITION, TermValue,
};
use crate::collections::walk_rdf_list;
use crate::{DatasetView, GraphMatch, QuadIds, QuadValues};
use purrdf_iri::vocab::rdf::{FIRST, NIL, REST};
use std::collections::{BTreeMap, BTreeSet};

type ReadResult<T, E> = Result<T, Box<FnoReadError<E>>>;
type Index<'a> = BTreeMap<&'a TermValue, BTreeMap<&'a str, BTreeMap<&'a TermValue, Vec<usize>>>>;

impl FnoCatalog {
    /// Read a complete default-graph FnO document for the explicit ontology IRI.
    ///
    /// Native blank identities and literal facets survive the read. Every row
    /// must belong to the catalog or one of its disjoint, anonymous RDF lists.
    /// Named graphs, reifiers, annotations and unrepresented facts are refused.
    ///
    /// # Errors
    /// Returns original source failures, native list causes and owned original
    /// field operands. A failed or truncated source never publishes a catalog.
    pub fn from_dataset<D: DatasetView>(
        view: &D,
        ontology_iri: &str,
    ) -> ReadResult<Self, D::ReadError> {
        view.checked_read(|view| read(view, ontology_iri))
            .map_err(|cause| Box::new(FnoReadError::Source(cause)))?
    }
}

fn read<D: DatasetView>(view: &D, ontology: &str) -> ReadResult<FnoCatalog, D::ReadError> {
    let rows: Vec<_> = view.quads().collect();
    let reifiers: Vec<_> = view.reifier_quads().collect();
    let annotations: Vec<_> = view.annotation_quads().collect();
    let graphs: Vec<_> = if view.supports_named_graphs() {
        view.named_graphs().collect()
    } else {
        Vec::new()
    };
    if let Some(cause) = view.read_error() {
        return Err(Box::new(FnoReadError::Source(cause)));
    }
    let mut terms = BTreeMap::new();
    for row in rows.iter().chain(&reifiers).chain(&annotations) {
        for id in [Some(row.s), Some(row.p), Some(row.o), row.g]
            .into_iter()
            .flatten()
        {
            if let std::collections::btree_map::Entry::Vacant(slot) = terms.entry(id) {
                slot.insert(
                    view.term_value(id)
                        .map_err(|cause| Box::new(FnoReadError::Term(cause)))?,
                );
            }
        }
    }
    for &id in &graphs {
        if let std::collections::btree_map::Entry::Vacant(slot) = terms.entry(id) {
            slot.insert(
                view.term_value(id)
                    .map_err(|cause| Box::new(FnoReadError::Term(cause)))?,
            );
        }
    }
    let mut reader = Reader {
        view,
        rows: &rows,
        terms: &terms,
        index: BTreeMap::new(),
        claimed: vec![false; rows.len()],
        cells: BTreeSet::new(),
    };
    if let Some(row) = reifiers.first() {
        return Err(reader.statement(*row, Shape::Reifier));
    }
    if let Some(row) = annotations.first() {
        return Err(reader.statement(*row, Shape::Annotation));
    }
    for (at, row) in rows.iter().enumerate() {
        if row.g.is_some() {
            return Err(reader.statement(*row, Shape::NamedGraph));
        }
        let TermValue::Iri(predicate) = &terms[&row.p] else {
            return Err(reader.statement(*row, Shape::Unclaimed));
        };
        reader
            .index
            .entry(&terms[&row.s])
            .or_default()
            .entry(predicate)
            .or_default()
            .entry(&terms[&row.o])
            .or_default()
            .push(at);
    }
    if let Some(graph) = graphs.first() {
        return Err(Box::new(FnoReadError::Graph {
            graph: terms[graph].clone(),
        }));
    }
    reader.catalog(ontology)
}

struct Reader<'a, D: DatasetView> {
    view: &'a D,
    rows: &'a [QuadIds<D::Id>],
    terms: &'a BTreeMap<D::Id, TermValue>,
    index: Index<'a>,
    claimed: Vec<bool>,
    cells: BTreeSet<TermValue>,
}

impl<D: DatasetView> Reader<'_, D> {
    fn statement(&self, row: QuadIds<D::Id>, reason: Shape) -> Box<FnoReadError<D::ReadError>> {
        Box::new(FnoReadError::Statement {
            row: Box::new(QuadValues {
                s: self.terms[&row.s].clone(),
                p: self.terms[&row.p].clone(),
                o: self.terms[&row.o].clone(),
                g: row.g.map(|id| self.terms[&id].clone()),
            }),
            reason,
        })
    }

    fn field(
        &self,
        subject: &TermValue,
        predicate: &str,
        expected: Field,
    ) -> Box<FnoReadError<D::ReadError>> {
        Box::new(FnoReadError::Field {
            subject: subject.clone(),
            predicate: predicate.to_owned(),
            objects: self.objects(subject, predicate),
            expected,
        })
    }

    fn objects(&self, subject: &TermValue, predicate: &str) -> Vec<TermValue> {
        self.index
            .get(subject)
            .and_then(|props| props.get(predicate))
            .map_or_default(|objects| objects.keys().map(|value| (*value).clone()).collect())
    }

    fn claim(&mut self, subject: &TermValue, predicate: &str, object: &TermValue) {
        if let Some(rows) = self
            .index
            .get(subject)
            .and_then(|props| props.get(predicate))
            .and_then(|objects| objects.get(object))
        {
            for &row in rows {
                self.claimed[row] = true;
            }
        }
    }

    fn scalar(
        &mut self,
        subject: &TermValue,
        predicate: &str,
        required: bool,
    ) -> ReadResult<Option<TermValue>, D::ReadError> {
        let mut values = self.objects(subject, predicate);
        if values.len() > 1 || (required && values.is_empty()) {
            return Err(self.field(
                subject,
                predicate,
                if required {
                    Field::ExactlyOne
                } else {
                    Field::AtMostOne
                },
            ));
        }
        let value = values.pop();
        if let Some(value) = &value {
            self.claim(subject, predicate, value);
        }
        Ok(value)
    }

    fn iri(&mut self, subject: &TermValue, predicate: &str) -> ReadResult<String, D::ReadError> {
        match self.scalar(subject, predicate, true)? {
            Some(TermValue::Iri(value)) => Ok(value),
            _ => Err(self.field(subject, predicate, Field::Iri)),
        }
    }

    fn optional_iri(
        &mut self,
        subject: &TermValue,
        predicate: &str,
    ) -> ReadResult<Option<String>, D::ReadError> {
        match self.scalar(subject, predicate, false)? {
            None => Ok(None),
            Some(TermValue::Iri(value)) => Ok(Some(value)),
            _ => Err(self.field(subject, predicate, Field::Iri)),
        }
    }

    fn literal(
        &mut self,
        subject: &TermValue,
        predicate: &str,
        required: bool,
    ) -> ReadResult<Option<TermValue>, D::ReadError> {
        let value = self.scalar(subject, predicate, required)?;
        if value
            .as_ref()
            .is_some_and(|value| !matches!(value, TermValue::Literal { .. }))
        {
            return Err(self.field(subject, predicate, Field::Literal));
        }
        Ok(value)
    }

    fn text(
        &mut self,
        subject: &TermValue,
        predicate: &str,
    ) -> ReadResult<TermValue, D::ReadError> {
        Ok(self
            .literal(subject, predicate, true)?
            .expect("required scalar was checked"))
    }

    fn plain(&mut self, subject: &TermValue, predicate: &str) -> ReadResult<String, D::ReadError> {
        match self.scalar(subject, predicate, true)? {
            Some(TermValue::Literal {
                lexical_form,
                datatype,
                language: None,
                direction: None,
            }) if datatype == purrdf_xsd::datatype::XSD_STRING => Ok(lexical_form),
            _ => Err(self.field(subject, predicate, Field::PlainString)),
        }
    }

    fn boolean(&mut self, subject: &TermValue, predicate: &str) -> ReadResult<bool, D::ReadError> {
        match self.scalar(subject, predicate, true)? {
            Some(TermValue::Literal {
                lexical_form,
                datatype,
                language: None,
                direction: None,
            }) if datatype == purrdf_xsd::datatype::XSD_BOOLEAN => match lexical_form.as_str() {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => Err(self.field(subject, predicate, Field::Boolean)),
            },
            _ => Err(self.field(subject, predicate, Field::Boolean)),
        }
    }

    fn subjects(&self, class: &str) -> Vec<TermValue> {
        let class = TermValue::iri(class);
        self.index
            .iter()
            .filter(|(_, props)| {
                props
                    .get(RDF_TYPE)
                    .is_some_and(|types| types.contains_key(&class))
            })
            .map(|(subject, _)| (*subject).clone())
            .collect()
    }

    fn typed(&mut self, subject: &TermValue, class: &str) -> ReadResult<(), D::ReadError> {
        let value = TermValue::iri(class);
        if !self
            .index
            .get(subject)
            .and_then(|props| props.get(RDF_TYPE))
            .is_some_and(|types| types.contains_key(&value))
        {
            return Err(self.field(
                subject,
                RDF_TYPE,
                Field::Role {
                    class: class.to_owned(),
                },
            ));
        }
        self.claim(subject, RDF_TYPE, &value);
        Ok(())
    }

    fn subject_iri(&self, subject: &TermValue) -> ReadResult<String, D::ReadError> {
        if let TermValue::Iri(value) = subject {
            Ok(value.clone())
        } else {
            Err(self.field(subject, RDF_TYPE, Field::Iri))
        }
    }

    fn blank(&self, subject: &TermValue) -> ReadResult<FnBlank, D::ReadError> {
        if let TermValue::Blank { label, scope } = subject {
            Ok(FnBlank {
                label: label.clone(),
                scope: *scope,
            })
        } else {
            Err(self.field(subject, RDF_TYPE, Field::Blank))
        }
    }

    fn list(
        &mut self,
        subject: &TermValue,
        predicate: &str,
    ) -> ReadResult<Vec<TermValue>, D::ReadError> {
        let head = self
            .scalar(subject, predicate, true)?
            .expect("required head was checked");
        let id = self
            .view
            .term_id_by_value(&head)
            .map_err(|cause| Box::new(FnoReadError::Source(cause)))?
            .expect("the head was read from this view");
        let nil = self
            .view
            .term_id_by_value(&TermValue::iri(NIL))
            .map_err(|cause| Box::new(FnoReadError::Source(cause)))?;
        let first = self
            .view
            .term_id_by_value(&TermValue::iri(FIRST))
            .map_err(|cause| Box::new(FnoReadError::Source(cause)))?;
        let rest = self
            .view
            .term_id_by_value(&TermValue::iri(REST))
            .map_err(|cause| Box::new(FnoReadError::Source(cause)))?;
        let mut cells = Vec::new();
        let members = walk_rdf_list(
            id,
            nil,
            |cell| {
                if Some(cell) != nil {
                    cells.push(cell);
                }
                self.view.sole_object(cell, first, GraphMatch::Default)
            },
            |cell| self.view.sole_object(cell, rest, GraphMatch::Default),
        )
        .map_err(|cause| {
            Box::new(FnoReadError::List {
                subject: subject.clone(),
                predicate: predicate.to_owned(),
                cause: cause.map(|id| self.terms[&id].clone()),
            })
        })?;
        for (&cell, &member) in cells.iter().zip(&members) {
            if !matches!(self.terms[&member], TermValue::Iri(_)) {
                return Err(self.field(&self.terms[&cell], FIRST, Field::Iri));
            }
        }
        for id in cells {
            let cell = &self.terms[&id];
            let at = self.index[cell][FIRST]
                .values()
                .next()
                .and_then(|rows| rows.first())
                .expect("the strict walker found a first edge");
            let row = self.rows[*at];
            if !matches!(cell, TermValue::Blank { .. }) {
                return Err(self.statement(row, Shape::NonblankCell));
            }
            if !self.cells.insert(cell.clone())
                || self
                    .index
                    .get(cell)
                    .is_some_and(|props| props.contains_key(RDF_TYPE))
            {
                return Err(self.statement(row, Shape::SharedCell));
            }
            for predicate in [FIRST, REST] {
                for object in self.objects(cell, predicate) {
                    self.claim(cell, predicate, &object);
                }
            }
        }
        Ok(members
            .into_iter()
            .map(|id| self.terms[&id].clone())
            .collect())
    }

    fn catalog(&mut self, ontology: &str) -> ReadResult<FnoCatalog, D::ReadError> {
        let selector = TermValue::iri(ontology);
        let documents: Vec<_> = self
            .index
            .iter()
            .filter(|(_, props)| {
                props
                    .get(DCTERMS_IS_PART_OF)
                    .is_some_and(|values| values.contains_key(&selector))
            })
            .map(|(subject, _)| (*subject).clone())
            .collect();
        if documents.len() != 1 {
            return Err(Box::new(FnoReadError::Document {
                ontology_iri: ontology.to_owned(),
                candidates: documents,
            }));
        }
        let doc = &documents[0];
        self.typed(doc, OWL_ONTOLOGY)?;
        let mut input = FnoCatalogInput {
            document_iri: self.subject_iri(doc)?,
            ontology_iri: self.iri(doc, DCTERMS_IS_PART_OF)?,
            doc_label: self.text(doc, RDFS_LABEL)?,
            banner: self.text(doc, RDFS_COMMENT)?,
            functions: Vec::new(),
            params: Vec::new(),
            implementations: Vec::new(),
            mappings: Vec::new(),
        };
        for subject in self.subjects(&format!("{FNO}Parameter")) {
            self.typed(&subject, &format!("{FNO}Parameter"))?;
            input.params.push(FnParam {
                iri: self.subject_iri(&subject)?,
                predicate: self.optional_iri(&subject, &format!("{FNO}predicate"))?,
                r#type: self.iri(&subject, &format!("{FNO}type"))?,
                required: self.boolean(&subject, &format!("{FNO}required"))?,
                label: self.literal(&subject, RDFS_LABEL, false)?,
                description: self.literal(&subject, SKOS_DEFINITION, false)?,
            });
        }
        for subject in self.subjects(&format!("{FNO}Implementation")) {
            self.typed(&subject, &format!("{FNO}Implementation"))?;
            input.implementations.push(FnImpl {
                iri: self.subject_iri(&subject)?,
                format: self.plain(&subject, DCTERMS_FORMAT)?,
                see_also: self.iri(&subject, RDFS_SEE_ALSO)?,
            });
        }
        for subject in self.subjects(&format!("{FNO}Function")) {
            self.typed(&subject, &format!("{FNO}Function"))?;
            let returns = format!("{FNO}returns");
            let mut outputs = self.list(&subject, &returns)?;
            if outputs.len() != 1 {
                let head = self
                    .objects(&subject, &returns)
                    .pop()
                    .expect("the list head was checked");
                return Err(Box::new(FnoReadError::Returns {
                    subject,
                    predicate: returns,
                    head,
                    members: outputs,
                }));
            }
            let output = outputs.pop().expect("one output was checked");
            self.typed(&output, &format!("{FNO}Output"))?;
            let output = FnOutput {
                iri: self.subject_iri(&output)?,
                predicate: self.optional_iri(&output, &format!("{FNO}predicate"))?,
                r#type: self.iri(&output, &format!("{FNO}type"))?,
                label: self.literal(&output, RDFS_LABEL, false)?,
                description: self.literal(&output, SKOS_DEFINITION, false)?,
            };
            let expects_predicate = format!("{FNO}expects");
            let values = self.list(&subject, &expects_predicate)?;
            let mut expects = Vec::new();
            for value in values {
                if let TermValue::Iri(iri) = value {
                    expects.push(iri);
                } else {
                    return Err(self.field(&subject, &expects_predicate, Field::Iri));
                }
            }
            let mut kind_types = Vec::new();
            for kind in self.objects(&subject, RDF_TYPE) {
                let TermValue::Iri(iri) = &kind else {
                    return Err(self.field(&subject, RDF_TYPE, Field::Iri));
                };
                if !(iri.starts_with(FNO)
                    || iri.starts_with(FNOM)
                    || subject == *doc && iri == OWL_ONTOLOGY)
                {
                    self.claim(&subject, RDF_TYPE, &kind);
                    kind_types.push(iri.clone());
                }
            }
            input.functions.push(FnFunction {
                iri: self.subject_iri(&subject)?,
                label: self.text(&subject, RDFS_LABEL)?,
                description: self.literal(&subject, SKOS_DEFINITION, false)?,
                kind_types,
                see_also: self.optional_iri(&subject, RDFS_SEE_ALSO)?,
                expects,
                output,
            });
        }
        for subject in self.subjects(&format!("{FNO}Mapping")) {
            self.typed(&subject, &format!("{FNO}Mapping"))?;
            let mut mapping = FnMapping {
                node: self.blank(&subject)?,
                label: self.text(&subject, RDFS_LABEL)?,
                function: self.iri(&subject, &format!("{FNO}function"))?,
                implementation: self.iri(&subject, &format!("{FNO}implementation"))?,
                parameter_mappings: Vec::new(),
                return_mappings: Vec::new(),
            };
            let predicate = format!("{FNO}parameterMapping");
            for child in self.objects(&subject, &predicate) {
                self.claim(&subject, &predicate, &child);
                self.typed(&child, &format!("{FNOM}PropertyParameterMapping"))?;
                mapping.parameter_mappings.push(FnParamMapping {
                    node: self.blank(&child)?,
                    label: self.text(&child, RDFS_LABEL)?,
                    function_parameter: self.iri(&child, &format!("{FNOM}functionParameter"))?,
                    implementation_property: self
                        .plain(&child, &format!("{FNOM}implementationProperty"))?,
                });
            }
            let predicate = format!("{FNO}returnMapping");
            for child in self.objects(&subject, &predicate) {
                self.claim(&subject, &predicate, &child);
                self.typed(&child, &format!("{FNOM}DefaultReturnMapping"))?;
                mapping.return_mappings.push(FnReturnMapping {
                    node: self.blank(&child)?,
                    label: self.text(&child, RDFS_LABEL)?,
                    function_output: self.iri(&child, &format!("{FNOM}functionOutput"))?,
                    implementation_property: self
                        .plain(&child, &format!("{FNOM}implementationProperty"))?,
                });
            }
            input.mappings.push(mapping);
        }
        if let Some(at) = self.claimed.iter().position(|claimed| !claimed) {
            return Err(self.statement(self.rows[at], Shape::Unclaimed));
        }
        FnoCatalog::try_new(input)
            .map_err(|cause| Box::new(FnoReadError::Admission(Box::new(cause))))
    }
}
