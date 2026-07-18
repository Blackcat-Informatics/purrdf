// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic OWL/RDFS property-surface derivation for developer schemas.
//!
//! This is deliberately a bounded schema theory, not an instance reasoner. It
//! computes one sparse class/property relation with source-axiom provenance;
//! every schema emitter and the public coverage manifest project from that
//! relation. Unsupported or malformed structures fail with a typed error.

use std::collections::{BTreeMap, BTreeSet};

use ::purrdf::RdfDataset;

use crate::data::{GraphFilter, native_quads};
use crate::json_schema::{
    MAX_OWL_EXPRESSION_DEPTH, MAX_SCHEMA_CLASSES, MAX_SCHEMA_PROPERTIES, MAX_SCHEMA_RELATIONS,
    SchemaClassPropertyCoverage, SchemaCompileError, SchemaCompileRequest, SchemaCoveragePrecision,
    SchemaCoverageProvenance, SchemaCoverageReport, SchemaCoverageStatus, SchemaPropertyCoverage,
    SchemaSurfaceMode,
};
use crate::model::{rdf, rdfs};
use crate::shapes::{Constraint, Path, Shape, Target};
use crate::term::Term;

const RDF_PROPERTY: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
const RDFS_SUB_PROPERTY_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
const RDFS_DATATYPE: &str = "http://www.w3.org/2000/01/rdf-schema#Datatype";
const RDFS_LITERAL: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";
const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
const OWL_DATA_RANGE: &str = "http://www.w3.org/2002/07/owl#DataRange";
const OWL_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
const OWL_DATATYPE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
const OWL_ANNOTATION_PROPERTY: &str = "http://www.w3.org/2002/07/owl#AnnotationProperty";
const OWL_FUNCTIONAL_PROPERTY: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";
const OWL_INVERSE_FUNCTIONAL_PROPERTY: &str =
    "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";
const OWL_EQUIVALENT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
const OWL_INVERSE_OF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
const OWL_EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
const OWL_UNION_OF: &str = "http://www.w3.org/2002/07/owl#unionOf";
const OWL_INTERSECTION_OF: &str = "http://www.w3.org/2002/07/owl#intersectionOf";

/// Exact bounded class expression supported for ontology domains and ranges.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum OntologyExpression {
    Named(String),
    Union(Vec<Self>),
    Intersection(Vec<Self>),
}

impl OntologyExpression {
    fn canonical(&self) -> String {
        match self {
            Self::Named(iri) => format!("<{iri}>"),
            Self::Union(members) => format!(
                "union({})",
                members
                    .iter()
                    .map(Self::canonical)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Intersection(members) => format!(
                "intersection({})",
                members
                    .iter()
                    .map(Self::canonical)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }

    fn named_members(&self, out: &mut BTreeSet<String>) {
        match self {
            Self::Named(iri) => {
                out.insert(iri.clone());
            }
            Self::Union(members) | Self::Intersection(members) => {
                for member in members {
                    member.named_members(out);
                }
            }
        }
    }

    fn matches_class(&self, supertypes: &BTreeSet<String>) -> bool {
        match self {
            Self::Named(iri) => supertypes.contains(iri),
            Self::Union(members) => members
                .iter()
                .any(|member| member.matches_class(supertypes)),
            Self::Intersection(members) => members
                .iter()
                .all(|member| member.matches_class(supertypes)),
        }
    }

    fn all_named_members_match(&self, predicate: &impl Fn(&str) -> bool) -> bool {
        match self {
            Self::Named(iri) => predicate(iri),
            Self::Union(members) | Self::Intersection(members) => members
                .iter()
                .all(|member| member.all_named_members_match(predicate)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OntologyPropertyKind {
    Generic,
    Object,
    Datatype,
    Annotation,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SourcedExpression {
    expression: OntologyExpression,
    provenance: SchemaCoverageProvenance,
}

#[derive(Debug, Clone, Default)]
struct PropertyFacts {
    declarations: BTreeSet<String>,
    provenance: BTreeSet<SchemaCoverageProvenance>,
    object_property: bool,
    datatype_property: bool,
    annotation_property: bool,
    domains: BTreeSet<SourcedExpression>,
    ranges: BTreeSet<SourcedExpression>,
    functional: BTreeSet<SchemaCoverageProvenance>,
}

impl PropertyFacts {
    fn kind(&self, property_iri: &str) -> Result<OntologyPropertyKind, SchemaCompileError> {
        let specialized = usize::from(self.object_property)
            + usize::from(self.datatype_property)
            + usize::from(self.annotation_property);
        if specialized > 1 {
            return Err(SchemaCompileError::InvalidOntology {
                subject: property_iri.to_owned(),
                reason: "property has incompatible object, datatype, or annotation declarations"
                    .to_owned(),
            });
        }
        Ok(if self.object_property {
            OntologyPropertyKind::Object
        } else if self.datatype_property {
            OntologyPropertyKind::Datatype
        } else if self.annotation_property {
            OntologyPropertyKind::Annotation
        } else {
            OntologyPropertyKind::Generic
        })
    }
}

/// One unshaped property admitted into a class definition.
#[derive(Debug, Clone)]
pub(crate) struct SurfaceProperty {
    pub(crate) iri: String,
    pub(crate) kind: OntologyPropertyKind,
    pub(crate) ranges: Vec<OntologyExpression>,
    pub(crate) datatype_iris: BTreeSet<String>,
    pub(crate) functional: bool,
    pub(crate) provenance: Vec<SchemaCoverageProvenance>,
}

/// One existing named class represented by a schema `$def`.
#[derive(Debug, Clone, Default)]
pub(crate) struct SurfaceClass {
    pub(crate) synthesized_open: bool,
    pub(crate) properties: BTreeMap<String, SurfaceProperty>,
}

/// Single source of truth for ontology-aware schema definitions and coverage.
#[derive(Debug, Clone)]
pub(crate) struct SchemaSurface {
    pub(crate) classes: BTreeMap<String, SurfaceClass>,
    pub(crate) report: SchemaCoverageReport,
}

impl SchemaSurface {
    fn assert_conservation(&self) {
        let report_properties: BTreeSet<&str> = self
            .report
            .properties
            .iter()
            .map(|property| property.property_iri.as_str())
            .collect();
        debug_assert_eq!(report_properties.len(), self.report.properties.len());
        let mut emitted = 0_usize;
        let mut range_expressions = 0_usize;
        let mut provenance_records = 0_usize;
        for class in self.classes.values() {
            for (property_iri, property) in &class.properties {
                debug_assert_eq!(property_iri, &property.iri);
                debug_assert!(report_properties.contains(property_iri.as_str()));
                match property.kind {
                    OntologyPropertyKind::Generic
                    | OntologyPropertyKind::Object
                    | OntologyPropertyKind::Datatype
                    | OntologyPropertyKind::Annotation => {}
                }
                if property.functional {
                    debug_assert!(!property.provenance.is_empty());
                }
                emitted += 1;
                range_expressions += property.ranges.len();
                provenance_records += property.provenance.len();
            }
        }
        debug_assert!(emitted <= MAX_SCHEMA_RELATIONS);
        debug_assert!(range_expressions <= MAX_SCHEMA_RELATIONS);
        debug_assert!(provenance_records <= MAX_SCHEMA_RELATIONS * 8);
    }
}

#[derive(Debug, Clone, Default)]
struct ShapeClassInfo {
    direct_properties: BTreeSet<String>,
    closed_surfaces: Vec<ClosedSurface>,
}

#[derive(Debug, Clone, Default)]
struct ClosedSurface {
    direct_properties: BTreeSet<String>,
    ignored_properties: BTreeSet<String>,
}

impl ShapeClassInfo {
    fn closed_allows(&self, property: &str) -> bool {
        self.closed_surfaces.iter().all(|closed| {
            closed.direct_properties.contains(property)
                || closed.ignored_properties.contains(property)
        })
    }
}

type ObjectIndex = BTreeMap<String, BTreeMap<String, Vec<Term>>>;

#[derive(Debug, Clone)]
struct TripleRow {
    subject: Term,
    subject_key: String,
    predicate: String,
    object: Term,
    object_key: String,
}

#[derive(Debug, Clone, Copy)]
enum PropertyRelationKind {
    SubProperty,
    Equivalent,
    Inverse,
}

#[derive(Debug, Clone)]
struct PropertyRelation {
    left: String,
    right: String,
    kind: PropertyRelationKind,
}

/// Build the deterministic class/property relation for one request.
pub(crate) fn build(
    request: &SchemaCompileRequest<'_>,
) -> Result<SchemaSurface, SchemaCompileError> {
    let union = RdfDataset::union(&[request.ontology(), request.shapes().shapes_dataset.as_ref()]);
    let mut rows = dataset_rows(&union);
    rows.sort_by(|left, right| {
        left.subject_key
            .cmp(&right.subject_key)
            .then_with(|| left.predicate.cmp(&right.predicate))
            .then_with(|| left.object_key.cmp(&right.object_key))
    });
    rows.dedup_by(|left, right| {
        left.subject == right.subject
            && left.predicate == right.predicate
            && left.object == right.object
    });
    let objects = object_index(&rows);

    let shape_classes = shape_class_info(request.shapes());
    let mut properties: BTreeMap<String, PropertyFacts> = BTreeMap::new();
    let mut explicit_classes = BTreeSet::new();
    let mut datatypes = BTreeSet::new();
    let mut subclass_relations = Vec::new();
    let mut equivalent_class_relations = Vec::new();
    let mut property_relations = Vec::new();

    for (class, info) in &shape_classes {
        explicit_classes.insert(class.clone());
        for property in &info.direct_properties {
            let facts = property_entry(&mut properties, property)?;
            facts.declarations.insert("sh:path".to_owned());
            facts.provenance.insert(SchemaCoverageProvenance {
                subject: class.clone(),
                predicate: crate::model::sh::PATH.to_owned(),
                object: format!("<{property}>"),
            });
        }
    }

    for row in &rows {
        let Some(subject_iri) = named_iri(&row.subject) else {
            continue;
        };
        match row.predicate.as_str() {
            rdf::TYPE => {
                let Some(type_iri) = named_iri(&row.object) else {
                    continue;
                };
                match type_iri {
                    RDF_PROPERTY
                    | OWL_OBJECT_PROPERTY
                    | OWL_DATATYPE_PROPERTY
                    | OWL_ANNOTATION_PROPERTY
                    | OWL_FUNCTIONAL_PROPERTY
                    | OWL_INVERSE_FUNCTIONAL_PROPERTY => {
                        let facts = property_entry(&mut properties, subject_iri)?;
                        facts.declarations.insert(type_iri.to_owned());
                        let provenance = axiom_provenance(subject_iri, rdf::TYPE, type_iri);
                        facts.provenance.insert(provenance.clone());
                        match type_iri {
                            OWL_OBJECT_PROPERTY => facts.object_property = true,
                            OWL_DATATYPE_PROPERTY => facts.datatype_property = true,
                            OWL_ANNOTATION_PROPERTY => facts.annotation_property = true,
                            OWL_FUNCTIONAL_PROPERTY => {
                                facts.functional.insert(provenance);
                            }
                            _ => {}
                        }
                    }
                    rdfs::CLASS | OWL_CLASS => {
                        explicit_classes.insert(subject_iri.to_owned());
                    }
                    RDFS_DATATYPE | OWL_DATA_RANGE => {
                        datatypes.insert(subject_iri.to_owned());
                    }
                    _ => {}
                }
            }
            rdfs::SUB_CLASS_OF => {
                let Some(parent) = named_iri(&row.object) else {
                    return Err(invalid_relation(subject_iri, rdfs::SUB_CLASS_OF));
                };
                explicit_classes.insert(subject_iri.to_owned());
                explicit_classes.insert(parent.to_owned());
                subclass_relations.push((subject_iri.to_owned(), parent.to_owned()));
            }
            OWL_EQUIVALENT_CLASS => {
                let Some(equivalent) = named_iri(&row.object) else {
                    return Err(invalid_relation(subject_iri, OWL_EQUIVALENT_CLASS));
                };
                explicit_classes.insert(subject_iri.to_owned());
                explicit_classes.insert(equivalent.to_owned());
                equivalent_class_relations.push((subject_iri.to_owned(), equivalent.to_owned()));
            }
            RDFS_SUB_PROPERTY_OF | OWL_EQUIVALENT_PROPERTY | OWL_INVERSE_OF => {
                let Some(right) = named_iri(&row.object) else {
                    return Err(invalid_relation(subject_iri, &row.predicate));
                };
                let left_facts = property_entry(&mut properties, subject_iri)?;
                left_facts
                    .declarations
                    .insert(format!("{}:subject", row.predicate));
                left_facts
                    .provenance
                    .insert(axiom_provenance(subject_iri, &row.predicate, right));
                let right_facts = property_entry(&mut properties, right)?;
                right_facts
                    .declarations
                    .insert(format!("{}:object", row.predicate));
                right_facts
                    .provenance
                    .insert(axiom_provenance(subject_iri, &row.predicate, right));
                property_relations.push(PropertyRelation {
                    left: subject_iri.to_owned(),
                    right: right.to_owned(),
                    kind: match row.predicate.as_str() {
                        RDFS_SUB_PROPERTY_OF => PropertyRelationKind::SubProperty,
                        OWL_EQUIVALENT_PROPERTY => PropertyRelationKind::Equivalent,
                        OWL_INVERSE_OF => PropertyRelationKind::Inverse,
                        _ => unreachable!("matched relation predicate"),
                    },
                });
            }
            RDFS_DOMAIN | rdfs::RANGE => {
                let expression = parse_expression(&row.object, &objects, 0)?;
                let predicate = row.predicate.as_str();
                let provenance = SchemaCoverageProvenance {
                    subject: subject_iri.to_owned(),
                    predicate: predicate.to_owned(),
                    object: expression.canonical(),
                };
                let sourced = SourcedExpression {
                    expression,
                    provenance: provenance.clone(),
                };
                let facts = property_entry(&mut properties, subject_iri)?;
                facts.declarations.insert(predicate.to_owned());
                facts.provenance.insert(provenance);
                if predicate == RDFS_DOMAIN {
                    facts.domains.insert(sourced);
                } else {
                    facts.ranges.insert(sourced);
                }
            }
            _ => {}
        }
    }

    enforce_limit("properties", properties.len(), MAX_SCHEMA_PROPERTIES)?;
    propagate_property_facts(&mut properties, &property_relations)?;
    validate_property_ranges(&properties, &datatypes)?;

    for facts in properties.values() {
        for domain in &facts.domains {
            domain.expression.named_members(&mut explicit_classes);
        }
        let kind = facts.kind("range class discovery")?;
        if matches!(
            kind,
            OntologyPropertyKind::Object | OntologyPropertyKind::Generic
        ) {
            for range in &facts.ranges {
                let mut named = BTreeSet::new();
                range.expression.named_members(&mut named);
                for iri in named {
                    if !is_builtin_datatype(&iri) && !datatypes.contains(&iri) {
                        explicit_classes.insert(iri);
                    }
                }
            }
        }
    }
    explicit_classes.extend(shape_classes.keys().cloned());
    enforce_limit("classes", explicit_classes.len(), MAX_SCHEMA_CLASSES)?;
    let supertypes = class_supertypes(
        &explicit_classes,
        &subclass_relations,
        &equivalent_class_relations,
    )?;

    assemble_surface(
        request,
        properties,
        &shape_classes,
        explicit_classes,
        &datatypes,
        &supertypes,
    )
}

fn dataset_rows(dataset: &RdfDataset) -> Vec<TripleRow> {
    native_quads(dataset, None, None, None, GraphFilter::AnyGraph)
        .into_iter()
        .map(|(subject, predicate, object)| {
            let subject_key = subject.to_string();
            let object_key = object.to_string();
            TripleRow {
                subject,
                subject_key,
                predicate: predicate.into_string(),
                object,
                object_key,
            }
        })
        .collect()
}

fn object_index(rows: &[TripleRow]) -> ObjectIndex {
    let mut index: ObjectIndex = BTreeMap::new();
    for row in rows {
        index
            .entry(row.subject_key.clone())
            .or_default()
            .entry(row.predicate.clone())
            .or_default()
            .push(row.object.clone());
    }
    for predicates in index.values_mut() {
        for values in predicates.values_mut() {
            values.sort_by_cached_key(ToString::to_string);
            values.dedup();
        }
    }
    index
}

fn named_iri(term: &Term) -> Option<&str> {
    match term {
        Term::NamedNode(node) => Some(node.as_str()),
        _ => None,
    }
}

fn property_entry<'a>(
    properties: &'a mut BTreeMap<String, PropertyFacts>,
    iri: &str,
) -> Result<&'a mut PropertyFacts, SchemaCompileError> {
    if !properties.contains_key(iri) && properties.len() == MAX_SCHEMA_PROPERTIES {
        return Err(SchemaCompileError::LimitExceeded {
            resource: "properties",
            limit: MAX_SCHEMA_PROPERTIES,
            observed: MAX_SCHEMA_PROPERTIES + 1,
        });
    }
    Ok(properties.entry(iri.to_owned()).or_default())
}

fn axiom_provenance(subject: &str, predicate: &str, object_iri: &str) -> SchemaCoverageProvenance {
    SchemaCoverageProvenance {
        subject: subject.to_owned(),
        predicate: predicate.to_owned(),
        object: format!("<{object_iri}>"),
    }
}

fn invalid_relation(subject: &str, predicate: &str) -> SchemaCompileError {
    SchemaCompileError::InvalidOntology {
        subject: subject.to_owned(),
        reason: format!("{predicate} requires a named IRI object in the schema fragment"),
    }
}

fn enforce_limit(
    resource: &'static str,
    observed: usize,
    limit: usize,
) -> Result<(), SchemaCompileError> {
    if observed > limit {
        Err(SchemaCompileError::LimitExceeded {
            resource,
            limit,
            observed,
        })
    } else {
        Ok(())
    }
}

fn coverage_cell_count(
    property_count: usize,
    eligible_class_count: usize,
    external_shaped_cells: usize,
    limit: usize,
) -> Result<usize, SchemaCompileError> {
    let observed = property_count
        .checked_mul(eligible_class_count)
        .and_then(|cells| cells.checked_add(external_shaped_cells))
        .unwrap_or(usize::MAX);
    enforce_limit("class/property coverage cells", observed, limit)?;
    Ok(observed)
}

fn parse_expression(
    term: &Term,
    objects: &ObjectIndex,
    depth: usize,
) -> Result<OntologyExpression, SchemaCompileError> {
    if depth > MAX_OWL_EXPRESSION_DEPTH {
        return Err(SchemaCompileError::LimitExceeded {
            resource: "OWL expression depth",
            limit: MAX_OWL_EXPRESSION_DEPTH,
            observed: depth,
        });
    }
    match term {
        Term::NamedNode(node) => Ok(OntologyExpression::Named(node.as_str().to_owned())),
        Term::BlankNode(_) => {
            let key = term.to_string();
            let unions = indexed_objects(objects, &key, OWL_UNION_OF);
            let intersections = indexed_objects(objects, &key, OWL_INTERSECTION_OF);
            match (unions.len(), intersections.len()) {
                (1, 0) => Ok(OntologyExpression::Union(parse_expression_list(
                    &unions[0],
                    objects,
                    depth + 1,
                    &key,
                )?)),
                (0, 1) => Ok(OntologyExpression::Intersection(parse_expression_list(
                    &intersections[0],
                    objects,
                    depth + 1,
                    &key,
                )?)),
                (0, 0) => Err(SchemaCompileError::InvalidOntology {
                    subject: key,
                    reason: "anonymous domain/range expression must declare exactly one owl:unionOf or owl:intersectionOf"
                        .to_owned(),
                }),
                _ => Err(SchemaCompileError::InvalidOntology {
                    subject: key,
                    reason: "anonymous domain/range expression has ambiguous union/intersection declarations"
                        .to_owned(),
                }),
            }
        }
        _ => Err(SchemaCompileError::InvalidOntology {
            subject: term.to_string(),
            reason: "domain/range expression must be a named class/datatype or an anonymous union/intersection"
                .to_owned(),
        }),
    }
}

fn parse_expression_list(
    head: &Term,
    objects: &ObjectIndex,
    depth: usize,
    owner: &str,
) -> Result<Vec<OntologyExpression>, SchemaCompileError> {
    let mut cursor = head.clone();
    let mut visited = BTreeSet::new();
    let mut members = Vec::new();
    loop {
        if matches!(&cursor, Term::NamedNode(node) if node.as_str() == rdf::NIL) {
            break;
        }
        let Term::BlankNode(_) = &cursor else {
            return Err(SchemaCompileError::InvalidOntology {
                subject: owner.to_owned(),
                reason: format!("OWL expression list must terminate at rdf:nil; found {cursor}"),
            });
        };
        let key = cursor.to_string();
        if !visited.insert(key.clone()) {
            return Err(SchemaCompileError::InvalidOntology {
                subject: owner.to_owned(),
                reason: format!("OWL expression list contains a cycle at {key}"),
            });
        }
        enforce_limit(
            "OWL expression list members",
            visited.len(),
            MAX_SCHEMA_CLASSES,
        )?;
        let first = indexed_objects(objects, &key, rdf::FIRST);
        let rest = indexed_objects(objects, &key, rdf::REST);
        if first.len() != 1 || rest.len() != 1 {
            return Err(SchemaCompileError::InvalidOntology {
                subject: key,
                reason: format!(
                    "RDF list cell requires exactly one rdf:first and rdf:rest; found {} and {}",
                    first.len(),
                    rest.len()
                ),
            });
        }
        members.push(parse_expression(&first[0], objects, depth)?);
        cursor = rest[0].clone();
    }
    if members.len() < 2 {
        return Err(SchemaCompileError::InvalidOntology {
            subject: owner.to_owned(),
            reason: "owl:unionOf/owl:intersectionOf requires at least two members".to_owned(),
        });
    }
    members.sort();
    members.dedup();
    if members.len() < 2 {
        return Err(SchemaCompileError::InvalidOntology {
            subject: owner.to_owned(),
            reason: "owl:unionOf/owl:intersectionOf must contain two distinct members".to_owned(),
        });
    }
    Ok(members)
}

fn indexed_objects<'a>(objects: &'a ObjectIndex, subject: &str, predicate: &str) -> &'a [Term] {
    objects
        .get(subject)
        .and_then(|predicates| predicates.get(predicate))
        .map_or(&[], Vec::as_slice)
}

fn shape_class_info(shapes: &crate::shapes::Shapes) -> BTreeMap<String, ShapeClassInfo> {
    let mut classes = BTreeMap::new();
    for shape in &shapes.node_shapes {
        if shape.deactivated {
            continue;
        }
        for target in &shape.targets {
            let class = match target {
                Target::Class(node) => Some(node.as_str()),
                Target::ImplicitClass(Term::NamedNode(node)) => Some(node.as_str()),
                _ => None,
            };
            if let Some(class) = class {
                merge_shape_info(classes.entry(class.to_owned()).or_default(), shape);
            }
        }
    }
    classes
}

fn merge_shape_info(info: &mut ShapeClassInfo, shape: &Shape) {
    let direct = direct_shape_properties(shape);
    info.direct_properties.extend(direct.iter().cloned());
    for constraint in &shape.constraints {
        if let Constraint::Closed { ignored } = constraint {
            info.closed_surfaces.push(ClosedSurface {
                direct_properties: direct.clone(),
                ignored_properties: ignored
                    .iter()
                    .map(|node| node.as_str().to_owned())
                    .collect(),
            });
        }
    }
}

fn direct_shape_properties(shape: &Shape) -> BTreeSet<String> {
    shape
        .property_shapes
        .iter()
        .filter(|property| !property.deactivated)
        .filter_map(|property| match &property.path {
            Path::Predicate(node) => Some(node.as_str().to_owned()),
            _ => None,
        })
        .collect()
}

fn propagate_property_facts(
    properties: &mut BTreeMap<String, PropertyFacts>,
    relations: &[PropertyRelation],
) -> Result<(), SchemaCompileError> {
    let names: Vec<String> = properties.keys().cloned().collect();
    let indices: BTreeMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect();
    let mut expression_graph = vec![BTreeSet::new(); names.len() * 2];
    let mut functional_graph = vec![BTreeSet::new(); names.len()];

    for relation in relations {
        let left = indices[relation.left.as_str()];
        let right = indices[relation.right.as_str()];
        match relation.kind {
            PropertyRelationKind::SubProperty => {
                add_edge(&mut expression_graph, right * 2, left * 2);
                add_edge(&mut expression_graph, right * 2 + 1, left * 2 + 1);
                add_edge(&mut functional_graph, right, left);
            }
            PropertyRelationKind::Equivalent => {
                add_bidirectional_edge(&mut expression_graph, left * 2, right * 2);
                add_bidirectional_edge(&mut expression_graph, left * 2 + 1, right * 2 + 1);
                add_bidirectional_edge(&mut functional_graph, left, right);
            }
            PropertyRelationKind::Inverse => {
                add_bidirectional_edge(&mut expression_graph, left * 2, right * 2 + 1);
                add_bidirectional_edge(&mut expression_graph, left * 2 + 1, right * 2);
            }
        }
    }
    let edge_count = expression_graph.iter().map(BTreeSet::len).sum::<usize>()
        + functional_graph.iter().map(BTreeSet::len).sum::<usize>();
    enforce_limit("ontology relation edges", edge_count, MAX_SCHEMA_RELATIONS)?;

    let mut expression_seeds = vec![BTreeSet::new(); names.len() * 2];
    let mut functional_seeds = vec![BTreeSet::new(); names.len()];
    for (index, name) in names.iter().enumerate() {
        let facts = &properties[name];
        expression_seeds[index * 2].clone_from(&facts.domains);
        expression_seeds[index * 2 + 1].clone_from(&facts.ranges);
        functional_seeds[index].clone_from(&facts.functional);
    }
    let effective_expressions = propagate_sets(
        &expression_graph,
        expression_seeds,
        "propagated ontology expressions",
    )?;
    let effective_functionality = propagate_sets(
        &functional_graph,
        functional_seeds,
        "propagated functionality facts",
    )?;
    let mut effective_expressions = effective_expressions.into_iter();
    let mut effective_functionality = effective_functionality.into_iter();
    for name in &names {
        let facts = properties
            .get_mut(name)
            .expect("property index was built from this map");
        facts.domains = effective_expressions
            .next()
            .expect("each property has one propagated domain set");
        facts.ranges = effective_expressions
            .next()
            .expect("each property has one propagated range set");
        facts.functional = effective_functionality
            .next()
            .expect("each property has one propagated functionality set");
    }
    Ok(())
}

fn add_edge(graph: &mut [BTreeSet<usize>], source: usize, destination: usize) {
    graph[source].insert(destination);
}

fn add_bidirectional_edge(graph: &mut [BTreeSet<usize>], left: usize, right: usize) {
    add_edge(graph, left, right);
    add_edge(graph, right, left);
}

fn validate_property_ranges(
    properties: &BTreeMap<String, PropertyFacts>,
    datatypes: &BTreeSet<String>,
) -> Result<(), SchemaCompileError> {
    for (property, facts) in properties {
        let kind = facts.kind(property)?;
        for range in &facts.ranges {
            match kind {
                OntologyPropertyKind::Datatype
                    if !range.expression.all_named_members_match(&|iri| {
                        is_builtin_datatype(iri) || datatypes.contains(iri)
                    }) =>
                {
                    return Err(SchemaCompileError::InvalidOntology {
                        subject: property.clone(),
                        reason: format!(
                            "owl:DatatypeProperty has non-datatype range {}",
                            range.expression.canonical()
                        ),
                    });
                }
                OntologyPropertyKind::Object
                    if !range.expression.all_named_members_match(&|iri| {
                        !is_builtin_datatype(iri) && !datatypes.contains(iri)
                    }) =>
                {
                    return Err(SchemaCompileError::InvalidOntology {
                        subject: property.clone(),
                        reason: format!(
                            "owl:ObjectProperty has datatype range {}",
                            range.expression.canonical()
                        ),
                    });
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn is_builtin_datatype(iri: &str) -> bool {
    iri.starts_with(crate::model::xsd::BASE) || iri == RDFS_LITERAL || iri == RDF_LANG_STRING
}

fn class_supertypes(
    classes: &BTreeSet<String>,
    subclasses: &[(String, String)],
    equivalents: &[(String, String)],
) -> Result<BTreeMap<String, BTreeSet<String>>, SchemaCompileError> {
    let names: Vec<String> = classes.iter().cloned().collect();
    let indices: BTreeMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect();
    let mut graph = vec![BTreeSet::new(); names.len()];
    for (child, parent) in subclasses {
        if let (Some(&child), Some(&parent)) =
            (indices.get(child.as_str()), indices.get(parent.as_str()))
        {
            add_edge(&mut graph, parent, child);
        }
    }
    for (left, right) in equivalents {
        if let (Some(&left), Some(&right)) =
            (indices.get(left.as_str()), indices.get(right.as_str()))
        {
            add_bidirectional_edge(&mut graph, left, right);
        }
    }
    enforce_limit(
        "class hierarchy edges",
        graph.iter().map(BTreeSet::len).sum(),
        MAX_SCHEMA_RELATIONS,
    )?;
    let seeds: Vec<BTreeSet<String>> = names
        .iter()
        .map(|name| BTreeSet::from([name.clone()]))
        .collect();
    let effective = propagate_sets(&graph, seeds, "propagated class memberships")?;
    Ok(names.into_iter().zip(effective).collect())
}

/// Propagate ordered fact sets through a directed graph after condensing every
/// legal cycle into one strongly connected component.
fn propagate_sets<T: Clone + Ord>(
    graph: &[BTreeSet<usize>],
    seeds: Vec<BTreeSet<T>>,
    resource: &'static str,
) -> Result<Vec<BTreeSet<T>>, SchemaCompileError> {
    propagate_sets_with_limit(graph, seeds, resource, MAX_SCHEMA_RELATIONS)
}

fn propagate_sets_with_limit<T: Clone + Ord>(
    graph: &[BTreeSet<usize>],
    seeds: Vec<BTreeSet<T>>,
    resource: &'static str,
    limit: usize,
) -> Result<Vec<BTreeSet<T>>, SchemaCompileError> {
    debug_assert_eq!(graph.len(), seeds.len());
    if graph.is_empty() {
        return Ok(Vec::new());
    }
    let components = strongly_connected_components(graph);
    let component_count = components.iter().copied().max().map_or(0, |max| max + 1);
    let mut values = vec![BTreeSet::new(); component_count];
    let mut materialized = 0_usize;
    for (node, facts) in seeds.into_iter().enumerate() {
        for fact in facts {
            insert_propagated_fact(
                &mut values[components[node]],
                fact,
                &mut materialized,
                resource,
                limit,
            )?;
        }
    }

    let mut dag = vec![BTreeSet::new(); component_count];
    let mut indegree = vec![0_usize; component_count];
    for (source, destinations) in graph.iter().enumerate() {
        let source_component = components[source];
        for &destination in destinations {
            let destination_component = components[destination];
            if source_component != destination_component
                && dag[source_component].insert(destination_component)
            {
                indegree[destination_component] += 1;
            }
        }
    }

    let mut ready: BTreeSet<usize> = indegree
        .iter()
        .enumerate()
        .filter_map(|(component, &degree)| (degree == 0).then_some(component))
        .collect();
    let mut visited = 0_usize;
    while let Some(component) = ready.pop_first() {
        visited += 1;
        let inherited: Vec<T> = values[component].iter().cloned().collect();
        for &destination in &dag[component] {
            for fact in &inherited {
                insert_propagated_fact(
                    &mut values[destination],
                    fact.clone(),
                    &mut materialized,
                    resource,
                    limit,
                )?;
            }
            indegree[destination] -= 1;
            if indegree[destination] == 0 {
                ready.insert(destination);
            }
        }
    }
    debug_assert_eq!(visited, component_count, "component graph is acyclic");
    let output_entries = components.iter().fold(0_usize, |total, &component| {
        total.saturating_add(values[component].len())
    });
    enforce_limit(resource, output_entries, limit)?;
    Ok(components
        .into_iter()
        .map(|component| values[component].clone())
        .collect())
}

fn insert_propagated_fact<T: Ord>(
    destination: &mut BTreeSet<T>,
    fact: T,
    materialized: &mut usize,
    resource: &'static str,
    limit: usize,
) -> Result<(), SchemaCompileError> {
    if destination.get(&fact).is_none() {
        let observed = materialized.saturating_add(1);
        enforce_limit(resource, observed, limit)?;
        destination.insert(fact);
        *materialized = observed;
    }
    Ok(())
}

/// Iterative Kosaraju decomposition. Iteration avoids recursion depth depending
/// on caller graph shape; lexical node/edge order keeps component assignment
/// deterministic even though only membership affects semantics.
fn strongly_connected_components(graph: &[BTreeSet<usize>]) -> Vec<usize> {
    let mut seen = vec![false; graph.len()];
    let mut finish = Vec::with_capacity(graph.len());
    for root in 0..graph.len() {
        if seen[root] {
            continue;
        }
        seen[root] = true;
        let mut stack = vec![(root, graph[root].iter())];
        while let Some((node, edges)) = stack.last_mut() {
            if let Some(&next) = edges.next() {
                if !seen[next] {
                    seen[next] = true;
                    stack.push((next, graph[next].iter()));
                }
            } else {
                finish.push(*node);
                stack.pop();
            }
        }
    }

    let mut reverse = vec![BTreeSet::new(); graph.len()];
    for (source, destinations) in graph.iter().enumerate() {
        for &destination in destinations {
            reverse[destination].insert(source);
        }
    }
    let mut components = vec![usize::MAX; graph.len()];
    let mut component = 0_usize;
    while let Some(root) = finish.pop() {
        if components[root] != usize::MAX {
            continue;
        }
        components[root] = component;
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            for &next in reverse[node].iter().rev() {
                if components[next] == usize::MAX {
                    components[next] = component;
                    stack.push(next);
                }
            }
        }
        component += 1;
    }
    components
}

fn assemble_surface(
    request: &SchemaCompileRequest<'_>,
    properties: BTreeMap<String, PropertyFacts>,
    shape_classes: &BTreeMap<String, ShapeClassInfo>,
    explicit_classes: BTreeSet<String>,
    datatypes: &BTreeSet<String>,
    supertypes: &BTreeMap<String, BTreeSet<String>>,
) -> Result<SchemaSurface, SchemaCompileError> {
    let eligible_classes: Vec<String> = explicit_classes
        .into_iter()
        .filter(|class| request.namespaces().is_caller_owned(class))
        .collect();
    let shaped_classes: BTreeSet<String> = shape_classes.keys().cloned().collect();
    let eligible_class_set: BTreeSet<&str> = eligible_classes.iter().map(String::as_str).collect();
    let external_shaped_classes: Vec<&str> = shaped_classes
        .iter()
        .map(String::as_str)
        .filter(|class| !eligible_class_set.contains(class))
        .collect();
    let represented_classes: BTreeSet<String> = match request.mode() {
        SchemaSurfaceMode::ShapedOnly => shaped_classes.clone(),
        SchemaSurfaceMode::OntologyComplete => eligible_classes
            .iter()
            .cloned()
            .chain(shaped_classes.iter().cloned())
            .collect(),
    };
    enforce_limit("classes", represented_classes.len(), MAX_SCHEMA_CLASSES)?;

    let mut classes: BTreeMap<String, SurfaceClass> = represented_classes
        .iter()
        .map(|class| {
            (
                class.clone(),
                SurfaceClass {
                    synthesized_open: !shaped_classes.contains(class),
                    properties: BTreeMap::new(),
                },
            )
        })
        .collect();
    let mut report_properties = Vec::with_capacity(properties.len());
    let external_shaped_cells = external_shaped_classes
        .iter()
        .map(|class| shape_classes[*class].direct_properties.len())
        .fold(0_usize, usize::saturating_add);
    coverage_cell_count(
        properties.len(),
        eligible_classes.len(),
        external_shaped_cells,
        MAX_SCHEMA_RELATIONS,
    )?;

    for (property_iri, facts) in properties {
        let kind = facts.kind(&property_iri)?;
        let mut datatype_iris = BTreeSet::new();
        for range in &facts.ranges {
            range.expression.named_members(&mut datatype_iris);
        }
        datatype_iris.retain(|iri| datatypes.contains(iri));
        let mut class_rows = Vec::new();
        let mut outcomes = BTreeSet::new();
        let base_provenance: Vec<SchemaCoverageProvenance> = facts
            .provenance
            .iter()
            .chain(facts.domains.iter().map(|domain| &domain.provenance))
            .chain(facts.ranges.iter().map(|range| &range.provenance))
            .chain(facts.functional.iter())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        for class_iri in &eligible_classes {
            let shape_info = shape_classes.get(class_iri);
            let has_shape = shape_info
                .is_some_and(|info| info.direct_properties.contains(property_iri.as_str()));
            let (status, precision) = if has_shape {
                (
                    SchemaCoverageStatus::HasShape,
                    SchemaCoveragePrecision::Exact,
                )
            } else if request.mode() == SchemaSurfaceMode::ShapedOnly {
                (
                    SchemaCoverageStatus::ExcludedShapedOnly,
                    SchemaCoveragePrecision::Exact,
                )
            } else if !request.namespaces().is_caller_owned(&property_iri) {
                (
                    SchemaCoverageStatus::ExcludedNamespace,
                    SchemaCoveragePrecision::Exact,
                )
            } else if !facts.domains.iter().all(|domain| {
                supertypes
                    .get(class_iri)
                    .is_some_and(|types| domain.expression.matches_class(types))
            }) {
                (
                    SchemaCoverageStatus::ExcludedDomain,
                    SchemaCoveragePrecision::Exact,
                )
            } else if shape_info.is_some_and(|info| !info.closed_allows(&property_iri)) {
                (
                    SchemaCoverageStatus::ExcludedClosedShape,
                    SchemaCoveragePrecision::Exact,
                )
            } else {
                let precision = if facts.functional.is_empty() {
                    SchemaCoveragePrecision::Exact
                } else {
                    SchemaCoveragePrecision::RepresentationApproximation
                };
                let class = classes
                    .get_mut(class_iri)
                    .expect("eligible complete-mode class has a surface entry");
                class.properties.insert(
                    property_iri.clone(),
                    SurfaceProperty {
                        iri: property_iri.clone(),
                        kind,
                        ranges: facts
                            .ranges
                            .iter()
                            .map(|range| range.expression.clone())
                            .collect(),
                        datatype_iris: datatype_iris.clone(),
                        functional: !facts.functional.is_empty(),
                        provenance: base_provenance.clone(),
                    },
                );
                (SchemaCoverageStatus::IncludedUnshaped, precision)
            };
            outcomes.insert(status);
            class_rows.push(SchemaClassPropertyCoverage {
                class_iri: class_iri.clone(),
                synthesized_open_class: classes
                    .get(class_iri)
                    .is_some_and(|class| class.synthesized_open),
                status,
                precision,
                provenance: base_provenance.clone(),
            });
        }

        // A shaped class may intentionally live outside the caller-owned
        // ontology boundary; retain its direct-shape audit row because legacy
        // compilation still emits it.
        for &class_iri in &external_shaped_classes {
            if shape_classes[class_iri]
                .direct_properties
                .contains(&property_iri)
            {
                outcomes.insert(SchemaCoverageStatus::HasShape);
                class_rows.push(SchemaClassPropertyCoverage {
                    class_iri: class_iri.to_owned(),
                    synthesized_open_class: false,
                    status: SchemaCoverageStatus::HasShape,
                    precision: SchemaCoveragePrecision::Exact,
                    provenance: base_provenance.clone(),
                });
            }
        }
        if outcomes.is_empty() {
            outcomes.insert(if !request.namespaces().is_caller_owned(&property_iri) {
                SchemaCoverageStatus::ExcludedNamespace
            } else if request.mode() == SchemaSurfaceMode::ShapedOnly {
                SchemaCoverageStatus::ExcludedShapedOnly
            } else {
                SchemaCoverageStatus::ExcludedDomain
            });
        }
        class_rows.sort_by(|left, right| left.class_iri.cmp(&right.class_iri));
        report_properties.push(SchemaPropertyCoverage {
            property_iri,
            declarations: facts.declarations.into_iter().collect(),
            outcomes: outcomes.into_iter().collect(),
            classes: class_rows,
        });
    }

    let surface = SchemaSurface {
        classes,
        report: SchemaCoverageReport {
            mode: request.mode(),
            properties: report_properties,
        },
    };
    surface.assert_conservation();
    Ok(surface)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_schema::Namespaces;
    use crate::shapes::from_dataset;

    const PREFIXES: &str = r"
        @prefix ex: <https://example.org/schema/> .
        @prefix ext: <https://external.example/vocab/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    ";

    fn namespaces() -> Namespaces {
        Namespaces::new(
            "ex",
            &[("ex".to_owned(), "https://example.org/schema/".to_owned())],
        )
        .expect("test namespace")
    }

    fn surface(
        shapes_body: &str,
        ontology_body: &str,
        mode: SchemaSurfaceMode,
    ) -> Result<SchemaSurface, SchemaCompileError> {
        let shape_dataset =
            crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}{shapes_body}"))
                .expect("shape Turtle");
        let shapes = from_dataset(&shape_dataset).expect("shape graph");
        let ontology =
            crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}{ontology_body}"))
                .expect("ontology Turtle");
        build(&SchemaCompileRequest::new(
            &shapes,
            &namespaces(),
            ontology.as_ref(),
            mode,
        ))
    }

    fn property<'a>(surface: &'a SchemaSurface, iri: &str) -> &'a SchemaPropertyCoverage {
        surface
            .report
            .properties
            .iter()
            .find(|property| property.property_iri == iri)
            .expect("catalogued property")
    }

    fn class_status(
        surface: &SchemaSurface,
        property_iri: &str,
        class_iri: &str,
    ) -> SchemaCoverageStatus {
        property(surface, property_iri)
            .classes
            .iter()
            .find(|row| row.class_iri == class_iri)
            .expect("class coverage row")
            .status
    }

    #[test]
    fn full_surface_catalogs_only_schema_properties_and_applies_domains() {
        let surface = surface(
            r"
                ex:PersonShape a sh:NodeShape ;
                    sh:targetClass ex:Person ;
                    sh:property [ sh:path ex:name ; sh:datatype xsd:string ] .
            ",
            r#"
                ex:Agent a owl:Class .
                ex:Person a owl:Class ; rdfs:subClassOf ex:Agent .
                ex:Message a owl:Class .

                ex:resentDate a owl:DatatypeProperty ;
                    rdfs:domain ex:Message ; rdfs:range xsd:dateTime .
                ex:identifier a owl:DatatypeProperty, owl:FunctionalProperty ;
                    rdfs:domain ex:Agent ; rdfs:range rdfs:Literal .
                ex:tag a rdf:Property .
                ext:externalOnly a owl:DatatypeProperty ; rdfs:range xsd:string .

                ex:instance ex:incidental "not a declaration" .
            "#,
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("complete surface");

        let catalog: BTreeSet<&str> = surface
            .report
            .properties
            .iter()
            .map(|property| property.property_iri.as_str())
            .collect();
        assert!(catalog.contains("https://example.org/schema/resentDate"));
        assert!(catalog.contains("https://example.org/schema/identifier"));
        assert!(catalog.contains("https://example.org/schema/tag"));
        assert!(catalog.contains("https://example.org/schema/name"));
        assert!(catalog.contains("https://external.example/vocab/externalOnly"));
        assert!(!catalog.contains("https://example.org/schema/incidental"));

        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/resentDate",
                "https://example.org/schema/Message"
            ),
            SchemaCoverageStatus::IncludedUnshaped
        );
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/resentDate",
                "https://example.org/schema/Person"
            ),
            SchemaCoverageStatus::ExcludedDomain
        );
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/identifier",
                "https://example.org/schema/Person"
            ),
            SchemaCoverageStatus::IncludedUnshaped,
            "subclasses satisfy inherited domain membership"
        );
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/name",
                "https://example.org/schema/Person"
            ),
            SchemaCoverageStatus::HasShape
        );
        assert_eq!(
            property(&surface, "https://external.example/vocab/externalOnly").outcomes,
            vec![SchemaCoverageStatus::ExcludedNamespace]
        );

        assert!(!surface.classes["https://example.org/schema/Person"].synthesized_open);
        assert!(surface.classes["https://example.org/schema/Agent"].synthesized_open);
        assert!(surface.classes["https://example.org/schema/Message"].synthesized_open);
        let identifier = &surface.classes["https://example.org/schema/Person"].properties["https://example.org/schema/identifier"];
        assert!(identifier.functional);
        assert_eq!(identifier.kind, OntologyPropertyKind::Datatype);
    }

    #[test]
    fn shaped_only_reports_exclusion_and_creates_no_carrier_classes() {
        let surface = surface(
            r"
                ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;
                    sh:property [ sh:path ex:name ] .
            ",
            r"
                ex:Person a owl:Class .
                ex:Message a owl:Class .
                ex:resentMessageId a owl:DatatypeProperty ;
                    rdfs:domain ex:Message ; rdfs:range rdfs:Literal .
            ",
            SchemaSurfaceMode::ShapedOnly,
        )
        .expect("shaped surface");
        assert_eq!(
            surface.classes.keys().cloned().collect::<Vec<_>>(),
            vec!["https://example.org/schema/Person"]
        );
        assert_eq!(
            property(&surface, "https://example.org/schema/resentMessageId").outcomes,
            vec![SchemaCoverageStatus::ExcludedShapedOnly]
        );
    }

    #[test]
    fn closed_shape_excludes_unshaped_property_but_ignored_property_is_admitted() {
        let surface = surface(
            r"
                ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;
                    sh:closed true ; sh:ignoredProperties ( ex:allowed ) ;
                    sh:property [ sh:path ex:name ] .
            ",
            r"
                ex:Person a owl:Class .
                ex:blocked a owl:DatatypeProperty ; rdfs:domain ex:Person .
                ex:allowed a owl:DatatypeProperty ; rdfs:domain ex:Person .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("closed surface");
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/blocked",
                "https://example.org/schema/Person"
            ),
            SchemaCoverageStatus::ExcludedClosedShape
        );
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/allowed",
                "https://example.org/schema/Person"
            ),
            SchemaCoverageStatus::IncludedUnshaped
        );
    }

    #[test]
    fn subproperty_equivalence_and_inverse_propagate_in_defined_directions() {
        let surface = surface(
            "",
            r"
                ex:Agent a owl:Class .
                ex:Document a owl:Class .
                ex:parent a owl:ObjectProperty, owl:FunctionalProperty ;
                    rdfs:domain ex:Agent ; rdfs:range ex:Document .
                ex:child a owl:ObjectProperty ; rdfs:subPropertyOf ex:parent .
                ex:alias a owl:ObjectProperty ; owl:equivalentProperty ex:child .
                ex:inverse a owl:ObjectProperty ; owl:inverseOf ex:parent .
                ex:reverseUnique a owl:InverseFunctionalProperty ;
                    rdfs:domain ex:Agent ; rdfs:range ex:Document .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("relation surface");
        for property_iri in [
            "https://example.org/schema/parent",
            "https://example.org/schema/child",
            "https://example.org/schema/alias",
        ] {
            assert!(
                surface.classes["https://example.org/schema/Agent"].properties[property_iri]
                    .functional
            );
        }
        assert!(
            surface.classes["https://example.org/schema/Document"]
                .properties
                .contains_key("https://example.org/schema/inverse"),
            "inverse swaps the parent's range into its domain"
        );
        assert!(
            !surface.classes["https://example.org/schema/Agent"].properties
                ["https://example.org/schema/reverseUnique"]
                .functional,
            "inverse-functional does not select a scalar value representation"
        );
    }

    #[test]
    fn multiple_domains_are_conjunctive_and_union_domain_is_disjunctive() {
        let surface = surface(
            "",
            r"
                ex:A a owl:Class .
                ex:B a owl:Class .
                ex:AB a owl:Class ; rdfs:subClassOf ex:A, ex:B .
                ex:both a rdf:Property ; rdfs:domain ex:A, ex:B .
                ex:either a rdf:Property ; rdfs:domain [
                    owl:unionOf ( ex:A ex:B )
                ] .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("expression surface");
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/both",
                "https://example.org/schema/A"
            ),
            SchemaCoverageStatus::ExcludedDomain
        );
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/both",
                "https://example.org/schema/AB"
            ),
            SchemaCoverageStatus::IncludedUnshaped
        );
        for class in ["A", "B", "AB"] {
            assert_eq!(
                class_status(
                    &surface,
                    "https://example.org/schema/either",
                    &format!("https://example.org/schema/{class}")
                ),
                SchemaCoverageStatus::IncludedUnshaped
            );
        }
    }

    #[test]
    fn hierarchy_cycles_are_legal_and_preserve_domain_membership() {
        let surface = surface(
            "",
            r"
                ex:A a owl:Class ; rdfs:subClassOf ex:B .
                ex:B a owl:Class ; rdfs:subClassOf ex:A .
                ex:p a rdf:Property ; rdfs:domain ex:A .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("cyclic hierarchy");
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/p",
                "https://example.org/schema/B"
            ),
            SchemaCoverageStatus::IncludedUnshaped
        );
    }

    #[test]
    fn malformed_and_cyclic_expression_lists_fail_with_typed_errors() {
        let malformed = surface(
            "",
            r"
                ex:p a rdf:Property ; rdfs:domain [ owl:unionOf _:list ] .
                _:list rdf:first ex:A ; rdf:first ex:B ; rdf:rest rdf:nil .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect_err("multiple rdf:first values must fail");
        assert!(matches!(
            malformed,
            SchemaCompileError::InvalidOntology { .. }
        ));

        let cyclic = surface(
            "",
            r"
                ex:p a rdf:Property ; rdfs:domain [ owl:intersectionOf _:list ] .
                _:list rdf:first ex:A ; rdf:rest _:tail .
                _:tail rdf:first ex:B ; rdf:rest _:list .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect_err("cyclic RDF list must fail");
        assert!(matches!(cyclic, SchemaCompileError::InvalidOntology { .. }));
    }

    #[test]
    fn ontology_and_shapes_blank_nodes_are_standardized_apart() {
        let surface = surface(
            r"
                _:shared rdf:first ex:ShapeOnly ; rdf:rest rdf:nil .
            ",
            r"
                ex:A a owl:Class .
                ex:B a owl:Class .
                ex:Holder a owl:Class .
                ex:choice a owl:ObjectProperty ;
                    rdfs:domain ex:Holder ;
                    rdfs:range [ owl:unionOf _:shared ] .
                _:shared rdf:first ex:A ; rdf:rest _:tail .
                _:tail rdf:first ex:B ; rdf:rest rdf:nil .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("same-label blanks from separate datasets remain independent");

        assert_eq!(
            surface.classes["https://example.org/schema/Holder"].properties
                ["https://example.org/schema/choice"]
                .ranges,
            vec![OntologyExpression::Union(vec![
                OntologyExpression::Named("https://example.org/schema/A".to_owned()),
                OntologyExpression::Named("https://example.org/schema/B".to_owned()),
            ])]
        );
    }

    #[test]
    fn propagation_and_coverage_preflights_enforce_exact_small_limits() {
        let chain = vec![
            BTreeSet::from([1_usize]),
            BTreeSet::from([2_usize]),
            BTreeSet::new(),
        ];
        let seeds = vec![
            BTreeSet::from([0_u8]),
            BTreeSet::from([1_u8]),
            BTreeSet::from([2_u8]),
        ];
        let transitive = propagate_sets_with_limit(&chain, seeds, "test facts", 5)
            .expect_err("transitive extension must stop at the limit");
        assert!(matches!(
            transitive,
            SchemaCompileError::LimitExceeded {
                resource: "test facts",
                limit: 5,
                observed: 6
            }
        ));

        let cycle = vec![
            BTreeSet::from([1_usize]),
            BTreeSet::from([2_usize]),
            BTreeSet::from([0_usize]),
        ];
        let seeds = vec![
            BTreeSet::from([0_u8]),
            BTreeSet::from([1_u8]),
            BTreeSet::from([2_u8]),
        ];
        let output = propagate_sets_with_limit(&cycle, seeds, "test facts", 8)
            .expect_err("per-node output cloning must stop at the limit");
        assert!(matches!(
            output,
            SchemaCompileError::LimitExceeded {
                resource: "test facts",
                limit: 8,
                observed: 9
            }
        ));

        let coverage = coverage_cell_count(2, 3, 1, 6)
            .expect_err("external shaped rows count toward coverage");
        assert!(matches!(
            coverage,
            SchemaCompileError::LimitExceeded {
                resource: "class/property coverage cells",
                limit: 6,
                observed: 7
            }
        ));
    }

    #[test]
    fn incompatible_property_kind_and_range_fail_with_typed_error() {
        let error = surface(
            "",
            r"
                ex:Person a owl:Class .
                ex:p a owl:DatatypeProperty ; rdfs:range ex:Person .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect_err("datatype property with class range must fail");
        assert!(matches!(
            error,
            SchemaCompileError::InvalidOntology { subject, .. }
                if subject == "https://example.org/schema/p"
        ));
    }

    #[test]
    fn report_and_surface_are_permutation_deterministic_and_conservative() {
        let first = surface(
            "",
            r"
                ex:C a owl:Class .
                ex:b a owl:DatatypeProperty ; rdfs:domain ex:C ; rdfs:range xsd:string .
                ex:a a owl:DatatypeProperty ; rdfs:domain ex:C ; rdfs:range xsd:dateTime .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("first surface");
        let second = surface(
            "",
            r"
                ex:a rdfs:range xsd:dateTime ; rdfs:domain ex:C ; a owl:DatatypeProperty .
                ex:b rdfs:range xsd:string ; a owl:DatatypeProperty ; rdfs:domain ex:C .
                ex:C a owl:Class .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("permuted surface");
        assert_eq!(first.report.to_json(), second.report.to_json());
        let catalog: Vec<&str> = first
            .report
            .properties
            .iter()
            .map(|property| property.property_iri.as_str())
            .collect();
        assert_eq!(
            catalog.len(),
            catalog.iter().copied().collect::<BTreeSet<_>>().len()
        );
        assert_eq!(first.classes.len(), second.classes.len());
    }
}
