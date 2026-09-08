// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native SHACL parameter cardinalities, independent of vocabulary imports.

use ::purrdf::{FastMap, IdSet, RdfDataset, TermId};

use crate::data::{GraphFilter, quads_for_pattern_ids};
use crate::model::{rdf, rdfs, sh};
use crate::shapes::Parser;
use crate::term::{Term, term_id_to_native};

/// Explicit Core max-one parameters, parameters of native components that have
/// multiple parameters, and single-valued shape metadata. SHACL 1.2 Core
/// §3.1.1 makes every parameter of a multi-parameter component single-valued,
/// including optional parameters. Single-parameter components such as
/// `sh:ClassConstraintComponent` remain repeatable unless Core explicitly limits
/// their parameter, as it does for `sh:datatype`.
const SINGLETON_PREDICATES: &[&str] = &[
    sh::DATATYPE,
    sh::NODE_KIND,
    sh::MIN_COUNT,
    sh::MAX_COUNT,
    sh::MIN_EXCLUSIVE,
    sh::MIN_INCLUSIVE,
    sh::MAX_EXCLUSIVE,
    sh::MAX_INCLUSIVE,
    sh::MIN_LENGTH,
    sh::MAX_LENGTH,
    sh::LANGUAGE_IN,
    sh::UNIQUE_LANG,
    sh::IN,
    sh::PATTERN,
    sh::FLAGS,
    sh::CLOSED,
    sh::IGNORED_PROPERTIES,
    sh::QUALIFIED_VALUE_SHAPE,
    sh::QUALIFIED_MIN_COUNT,
    sh::QUALIFIED_MAX_COUNT,
    sh::QUALIFIED_VALUE_SHAPES_DISJOINT,
    sh::REIFIER_SHAPE,
    sh::REIFICATION_REQUIRED,
];

/// These properties describe a SHACL node; arbitrary RDF annotations with the
/// same predicates do not make their subject a shape or declaration.
const METADATA_SINGLETONS: &[&str] = &[
    sh::PATH,
    sh::SEVERITY,
    sh::DEACTIVATED,
    sh::ORDER,
    sh::OPTIONAL,
    sh::SELECT,
    sh::ASK,
    sh::RETURN_TYPE,
];

/// Indexed role checks are needed only for metadata conflicts and `sh:optional`
/// values. Valid Core parameters require no extra graph traversal.
struct MetadataSubjects<'a> {
    data: &'a RdfDataset,
    markers: IdSet,
    references: IdSet,
    lists: IdSet,
    classes: IdSet,
    cache: FastMap<TermId, bool>,
}

impl<'a> MetadataSubjects<'a> {
    fn new(data: &'a RdfDataset) -> Self {
        let ids = |iris: &[&str]| {
            iris.iter()
                .filter_map(|iri| data.term_id_by_iri(iri))
                .collect()
        };
        let mut markers = ids(SINGLETON_PREDICATES);
        let repeatables: IdSet = ids(&[
            sh::CLASS,
            sh::HAS_VALUE,
            sh::NODE,
            sh::NOT,
            sh::AND,
            sh::OR,
            sh::XONE,
            sh::PROPERTY,
            sh::EQUALS,
            sh::DISJOINT,
            sh::LESS_THAN,
            sh::LESS_THAN_OR_EQUALS,
            sh::SPARQL,
            sh::EXPRESSION,
            sh::NODE_BY_EXPRESSION,
            sh::TARGET_CLASS,
            sh::TARGET_NODE,
            sh::TARGET_SUBJECTS_OF,
            sh::TARGET_OBJECTS_OF,
            sh::TARGET,
            sh::RULE,
            sh::PARAMETER_PROPERTY,
            sh::NODE_VALIDATOR,
            sh::PROPERTY_VALIDATOR,
            sh::VALIDATOR,
        ]);
        markers.extend(repeatables);
        Self {
            data,
            markers,
            references: ids(&[
                sh::PROPERTY,
                sh::NODE,
                sh::NOT,
                sh::QUALIFIED_VALUE_SHAPE,
                sh::REIFIER_SHAPE,
                sh::PARAMETER_PROPERTY,
                sh::NODE_VALIDATOR,
                sh::PROPERTY_VALIDATOR,
                sh::VALIDATOR,
                sh::SPARQL,
                sh::TARGET,
                sh::RULE,
                sh::CONDITION,
                sh::EXPRESSION,
                sh::NODE_BY_EXPRESSION,
                sh::BODY_EXPRESSION,
                sh::FILTER_SHAPE,
                sh::NODES,
                sh::IF,
                sh::THEN,
                sh::ELSE,
                sh::INVERSE_PATH,
                sh::ZERO_OR_MORE_PATH,
                sh::ONE_OR_MORE_PATH,
                sh::ZERO_OR_ONE_PATH,
                sh::SUBJECT,
                sh::PREDICATE,
                sh::OBJECT,
            ]),
            lists: ids(&[sh::AND, sh::OR, sh::XONE, sh::UNION, sh::INTERSECTION]),
            classes: ids(&[
                sh::NODE_SHAPE,
                sh::PROPERTY_SHAPE,
                sh::PARAMETER,
                sh::CONSTRAINT_COMPONENT,
                sh::SPARQL_CONSTRAINT,
                sh::SPARQL_ASK_VALIDATOR,
                sh::SPARQL_SELECT_VALIDATOR,
                sh::SPARQL_TARGET,
                sh::SPARQL_TARGET_TYPE,
                sh::SPARQL_FUNCTION,
                sh::FUNCTION,
                sh::TRIPLE_RULE,
                sh::SPARQL_RULE,
                sh::LIST_PARAMETER_EXPRESSION_FUNCTION,
                sh::NAMED_PARAMETER_EXPRESSION_FUNCTION,
            ]),
            cache: FastMap::default(),
        }
    }

    fn contains(&mut self, subject: TermId) -> bool {
        if let Some(found) = self.cache.get(&subject) {
            return *found;
        }
        let found = self.has_role(subject);
        self.cache.insert(subject, found);
        found
    }

    fn has_role(&self, subject: TermId) -> bool {
        let type_id = self.data.term_id_by_iri(rdf::TYPE);
        for quad in
            quads_for_pattern_ids(self.data, Some(subject), None, None, GraphFilter::AnyGraph)
        {
            if self.markers.contains(&quad.p)
                || (Some(quad.p) == type_id && self.has_shacl_class(quad.o))
            {
                return true;
            }
        }
        let first_id = self.data.term_id_by_iri(rdf::FIRST);
        for quad in
            quads_for_pattern_ids(self.data, None, None, Some(subject), GraphFilter::AnyGraph)
        {
            if self.references.contains(&quad.p)
                || (Some(quad.p) == first_id && self.is_shacl_list(quad.s))
            {
                return true;
            }
        }
        false
    }

    fn has_shacl_class(&self, class: TermId) -> bool {
        let subclass_id = self.data.term_id_by_iri(rdfs::SUB_CLASS_OF);
        let mut pending = vec![class];
        let mut seen = IdSet::default();
        while let Some(current) = pending.pop() {
            if !seen.insert(current) {
                continue;
            }
            if self.classes.contains(&current) {
                return true;
            }
            if let Some(predicate) = subclass_id {
                pending.extend(
                    quads_for_pattern_ids(
                        self.data,
                        Some(current),
                        Some(predicate),
                        None,
                        GraphFilter::AnyGraph,
                    )
                    .map(|quad| quad.o),
                );
            }
        }
        false
    }

    fn is_shacl_list(&self, cell: TermId) -> bool {
        let rest_id = self.data.term_id_by_iri(rdf::REST);
        let mut pending = vec![cell];
        let mut seen = IdSet::default();
        while let Some(current) = pending.pop() {
            if !seen.insert(current) {
                continue;
            }
            for quad in
                quads_for_pattern_ids(self.data, None, None, Some(current), GraphFilter::AnyGraph)
            {
                if self.lists.contains(&quad.p) {
                    return true;
                }
                if Some(quad.p) == rest_id {
                    pending.push(quad.s);
                }
            }
        }
        false
    }
}

impl Parser<'_> {
    /// Check the shapes dataset once before declarations or shapes are parsed.
    /// This also reaches untargeted shapes and parameter declarations. Keeping
    /// the first object as an interned ID makes cardinality count distinct RDF
    /// values in the union graph, not repetitions of a statement in named graphs.
    pub(crate) fn check_builtin_cardinalities(&self) -> Result<(), String> {
        let singleton_ids: IdSet = SINGLETON_PREDICATES
            .iter()
            .chain(METADATA_SINGLETONS)
            .filter_map(|iri| self.data.term_id_by_iri(iri))
            .collect();
        if singleton_ids.is_empty() {
            return Ok(());
        }

        let mut first_values: FastMap<(TermId, TermId), TermId> = FastMap::default();
        let metadata_ids: IdSet = METADATA_SINGLETONS
            .iter()
            .filter_map(|iri| self.data.term_id_by_iri(iri))
            .collect();
        let mut metadata_subjects = MetadataSubjects::new(self.data);
        let optional_id = self.data.term_id_by_iri(sh::OPTIONAL);
        let mut error: Option<String> = None;
        for quad in quads_for_pattern_ids(self.data, None, None, None, GraphFilter::AnyGraph) {
            if !singleton_ids.contains(&quad.p) {
                continue;
            }
            if Some(quad.p) == optional_id && metadata_subjects.contains(quad.s) {
                let value = term_id_to_native(self.data, quad.o);
                if !matches!(&value, Term::Literal(literal) if matches!(
                    purrdf_xsd::parse_by_iri(literal.value(), literal.datatype_str()),
                    Ok(Some(purrdf_xsd::XsdValue::Boolean(_)))
                )) {
                    let diagnostic = format!(
                        "node {} has invalid sh:optional value {value}; expected an xsd:boolean literal",
                        term_id_to_native(self.data, quad.s),
                    );
                    if error.as_ref().is_none_or(|previous| diagnostic < *previous) {
                        error = Some(diagnostic);
                    }
                }
            }
            let first = first_values.entry((quad.s, quad.p)).or_insert(quad.o);
            if *first != quad.o
                && (!metadata_ids.contains(&quad.p) || metadata_subjects.contains(quad.s))
            {
                let diagnostic = format!(
                    "node {} has more than one distinct value for {}; only one is allowed",
                    term_id_to_native(self.data, quad.s),
                    term_id_to_native(self.data, quad.p),
                );
                // Error selection must not depend on graph or interner order.
                if error.as_ref().is_none_or(|previous| diagnostic < *previous) {
                    error = Some(diagnostic);
                }
            }
        }
        error.map_or(Ok(()), Err)
    }
}
