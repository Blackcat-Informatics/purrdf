// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Data model for SHACL custom constraint components and their validator registry.
//!
//! A [`ComponentRegistry`] holds constraint components declared in a shapes graph
//! (instances of `sh:ConstraintComponent`) together with their `sh:Parameter`
//! declarations and SPARQL-based validators. The registry is populated at shape
//! parse time and consulted by the engine when it encounters a predicate that is
//! a declared component parameter path.
//!
//! These types are scaffolding for the custom component parser/evaluator added in
//! later tasks; they are exported from a `pub(crate)` module now and will be
//! constructed by the component loader once it lands.
#![allow(dead_code, unreachable_pub)]

use std::collections::{HashMap, HashSet};

use crate::data::{GraphFilter, IrDataGraph, ShaclDataGraph};
use crate::model::{rdf, rdfs, sh};
use crate::shapes::build_prefix_header;
use crate::term::{NamedNode, Term};

/// Discriminator for a SPARQL validator's query form.
#[derive(Debug, Clone)]
pub enum ValidatorKind {
    /// An `ASK` query: a result of `true` means the focus node/value is valid.
    Ask,
    /// A `SELECT` query: each result row denotes a validation violation.
    Select,
}

/// A SPARQL validator attached to a constraint component.
#[derive(Debug, Clone)]
pub struct Validator {
    /// Whether the validator is an ASK or SELECT query.
    pub kind: ValidatorKind,
    /// Full query text with any `PREFIX` header already prepended.
    pub query_text: String,
}

/// Declaration of a single `sh:Parameter` for a constraint component.
#[derive(Debug, Clone)]
pub struct Parameter {
    /// The parameter predicate (`sh:path` of the parameter declaration).
    pub path: NamedNode,
    /// The SPARQL local name used to bind the parameter value in the validator.
    pub name: String,
    /// Whether the parameter is optional (`sh:optional true`).
    pub optional: bool,
}

/// A SHACL custom constraint component.
#[derive(Debug, Clone)]
pub struct Component {
    /// The component IRI (the `sh:ConstraintComponent` instance).
    pub id: NamedNode,
    /// Declared parameters, sorted by path IRI string for determinism.
    pub parameters: Vec<Parameter>,
    /// Optional node-scope validator (`sh:nodeValidator`).
    pub node_validator: Option<Validator>,
    /// Optional property-scope validator (`sh:propertyValidator`).
    pub property_validator: Option<Validator>,
    /// Optional generic validator (`sh:validator`).
    pub validator: Option<Validator>,
}

/// Registry of custom constraint components keyed by component IRI string.
///
/// `by_parameter_path` maps a declared parameter predicate IRI string to the
/// IRI string of the component it belongs to, allowing the engine to recognize
/// custom constraint predicates while parsing shapes.
#[derive(Debug, Default, Clone)]
pub struct ComponentRegistry {
    /// Parameter predicate IRI string → owning component IRI string.
    pub by_parameter_path: HashMap<String, String>,
    /// Component IRI string → component definition.
    pub components: HashMap<String, Component>,
}

impl ComponentRegistry {
    /// Parse all `sh:ConstraintComponent` (and subclass) declarations from the
    /// shapes graph into a registry.
    ///
    /// Discovery walks every `rdf:type` triple and keeps the subject when its
    /// class is `sh:ConstraintComponent` or a subclass thereof. Each discovered
    /// component's `sh:parameter` declarations and `sh:nodeValidator` /
    /// `sh:propertyValidator` / `sh:validator` SPARQL validators are parsed and
    /// validated (query must parse to the declared form and satisfy SHACL-SPARQL
    /// pre-binding restrictions). Any malformed component is a hard error.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` when a component, parameter, or validator is
    /// malformed or when a validator query violates the pre-binding restrictions.
    pub fn parse(data: &IrDataGraph, doc_prefixes: &[(String, String)]) -> Result<Self, String> {
        let rdf_type = Term::NamedNode(NamedNode::from(rdf::TYPE));
        let mut component_iris: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut subclass_memo: HashMap<(String, String), bool> = HashMap::new();

        for q in data.quads_for_pattern(None, Some(&rdf_type), None, GraphFilter::AnyGraph) {
            let Term::NamedNode(class) = q.object else {
                continue;
            };
            if !is_subclass_of(
                data,
                class.as_str(),
                sh::CONSTRAINT_COMPONENT,
                &mut subclass_memo,
            ) {
                continue;
            }
            let Term::NamedNode(component) = q.subject else {
                continue;
            };
            if seen.insert(component.as_str().to_owned()) {
                component_iris.push(component.as_str().to_owned());
            }
        }
        component_iris.sort();

        let mut registry = Self::default();
        for component_iri in component_iris {
            let component_term = Term::NamedNode(NamedNode::from(component_iri.as_str()));
            let component = parse_component(
                data,
                doc_prefixes,
                &component_term,
                &component_iri,
                &mut subclass_memo,
            )?;
            let id = component.id.as_str().to_owned();
            for param in &component.parameters {
                registry
                    .by_parameter_path
                    .insert(param.path.as_str().to_owned(), id.clone());
            }
            registry.components.insert(id, component);
        }
        Ok(registry)
    }
}

/// Extract the SPARQL local name of an IRI: the longest suffix after the last
/// `/`, `#`, or `:`.
///
/// ```ignore
/// use purrdf_shapes::components::sparql_local_name;
///
/// assert_eq!(sparql_local_name("http://example.org/ns#requiredParam"), "requiredParam");
/// assert_eq!(sparql_local_name("http://example.org/ns/requiredParam"), "requiredParam");
/// assert_eq!(sparql_local_name("ex:requiredParam"), "requiredParam");
/// ```
#[must_use]
pub fn sparql_local_name(iri: &str) -> String {
    let mut idx = iri.rfind('/').map_or(0, |i| i + 1);
    if let Some(i) = iri.rfind('#') {
        idx = idx.max(i + 1);
    }
    if let Some(i) = iri.rfind(':') {
        idx = idx.max(i + 1);
    }
    iri[idx..].to_owned()
}

// ── Internal helpers ─────────────────────────────────────────────────────────

/// Return all objects for `(subject, predicate, ?)`.
fn objects_of(data: &IrDataGraph, subject: &Term, predicate: &str) -> Vec<Term> {
    if !subject.is_subject() {
        return vec![];
    }
    let pred = Term::NamedNode(NamedNode::from(predicate));
    data.quads_for_pattern(Some(subject), Some(&pred), None, GraphFilter::AnyGraph)
        .into_iter()
        .map(|q| q.object)
        .collect()
}

/// Return the first object for `(subject, predicate, ?)`, if any.
fn first_object_of(data: &IrDataGraph, subject: &Term, predicate: &str) -> Option<Term> {
    objects_of(data, subject, predicate).into_iter().next()
}

/// Whether `class_iri` is `target_iri` or a subclass thereof under
/// `rdfs:subClassOf` (reflexive transitive closure).
///
/// A memo table avoids repeated superclass walks; `false` is inserted before
/// recursion to break `rdfs:subClassOf` cycles.
fn is_subclass_of(
    data: &IrDataGraph,
    class_iri: &str,
    target_iri: &str,
    memo: &mut HashMap<(String, String), bool>,
) -> bool {
    if class_iri == target_iri {
        return true;
    }
    let key = (class_iri.to_owned(), target_iri.to_owned());
    if let Some(&result) = memo.get(&key) {
        return result;
    }
    memo.insert(key.clone(), false);
    let class_term = Term::NamedNode(NamedNode::from(class_iri));
    let sub_class_of = Term::NamedNode(NamedNode::from(rdfs::SUB_CLASS_OF));
    let mut result = false;
    for q in data.quads_for_pattern(
        Some(&class_term),
        Some(&sub_class_of),
        None,
        GraphFilter::AnyGraph,
    ) {
        let Term::NamedNode(super_class) = q.object else {
            continue;
        };
        if is_subclass_of(data, super_class.as_str(), target_iri, memo) {
            result = true;
            break;
        }
    }
    memo.insert(key, result);
    result
}

/// Determine the validator kind from its `rdf:type` declarations, respecting
/// subclasses of `sh:SPARQLAskValidator` and `sh:SPARQLSelectValidator`.
fn validator_kind(
    data: &IrDataGraph,
    validator: &Term,
    memo: &mut HashMap<(String, String), bool>,
) -> Result<ValidatorKind, String> {
    let mut is_ask = false;
    let mut is_select = false;
    for q in objects_of(data, validator, rdf::TYPE) {
        let Term::NamedNode(class) = q else {
            continue;
        };
        if is_subclass_of(data, class.as_str(), sh::SPARQL_ASK_VALIDATOR, memo) {
            is_ask = true;
        }
        if is_subclass_of(data, class.as_str(), sh::SPARQL_SELECT_VALIDATOR, memo) {
            is_select = true;
        }
    }
    if is_ask && is_select {
        return Err(format!(
            "validator {validator} is typed as both ASK and SELECT"
        ));
    }
    if is_ask {
        return Ok(ValidatorKind::Ask);
    }
    if is_select {
        return Ok(ValidatorKind::Select);
    }
    Err(format!(
        "validator {validator} must be typed as sh:SPARQLAskValidator or \
         sh:SPARQLSelectValidator (or a subclass)"
    ))
}

/// Whether `name` is a legal SPARQL VARNAME and not one of the reserved names
/// banned for SHACL-SPARQL parameter bindings.
fn is_valid_varname(name: &str) -> bool {
    const BANNED: &[&str] = &["this", "path", "PATH", "value"];
    if BANNED.contains(&name) {
        return false;
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parse a single `sh:parameter` declaration for a component.
fn parse_parameter(
    data: &IrDataGraph,
    param_node: &Term,
    component_iri: &str,
) -> Result<Parameter, String> {
    let paths: Vec<NamedNode> = objects_of(data, param_node, sh::PATH)
        .into_iter()
        .filter_map(|t| match t {
            Term::NamedNode(n) => Some(n),
            _ => None,
        })
        .collect();
    if paths.len() != 1 {
        return Err(format!(
            "component {component_iri} parameter {param_node} must have exactly one sh:path IRI"
        ));
    }
    let path = paths.into_iter().next().expect("paths length checked");
    let name = sparql_local_name(path.as_str());
    if !is_valid_varname(&name) {
        return Err(format!(
            "component {component_iri} parameter path <{}> yields invalid SPARQL variable name \
             {name:?}",
            path.as_str()
        ));
    }
    let optional = objects_of(data, param_node, sh::OPTIONAL)
        .iter()
        .any(|t| matches!(t, Term::Literal(lit) if lit.value() == "true"));
    Ok(Parameter {
        path,
        name,
        optional,
    })
}

/// Parse a single SPARQL validator node attached to a component.
fn parse_validator(
    data: &IrDataGraph,
    doc_prefixes: &[(String, String)],
    component: &Term,
    validator: &Term,
    param_names: &[String],
    subclass_memo: &mut HashMap<(String, String), bool>,
) -> Result<Validator, String> {
    let component_iri = match component {
        Term::NamedNode(n) => n.as_str(),
        _ => return Err(format!("component {component} is not a named node")),
    };
    let kind = validator_kind(data, validator, subclass_memo)
        .map_err(|e| format!("component {component_iri} validator {validator}: {e}"))?;

    let query_pred = match kind {
        ValidatorKind::Ask => sh::ASK,
        ValidatorKind::Select => sh::SELECT,
    };
    let raw_queries: Vec<String> = objects_of(data, validator, query_pred)
        .into_iter()
        .filter_map(|t| match t {
            Term::Literal(lit) => Some(lit.value().to_owned()),
            _ => None,
        })
        .collect();
    if raw_queries.len() != 1 {
        return Err(format!(
            "component {component_iri} validator {validator} must have exactly one {} literal",
            match kind {
                ValidatorKind::Ask => "sh:ask",
                ValidatorKind::Select => "sh:select",
            }
        ));
    }
    let raw_query = raw_queries.into_iter().next().expect("length checked");
    let query_text = format!(
        "{}{raw_query}",
        build_prefix_header(data, doc_prefixes, &[component, validator])
    );

    let query = match purrdf_sparql_algebra::SparqlParser::new().parse_query(&query_text) {
        Ok(q) => q,
        Err(e) => {
            return Err(format!(
                "component {component_iri} validator {validator} has an unparsable query: {e}"
            ))
        }
    };

    let got_form = match &query {
        purrdf_sparql_algebra::Query::Ask { .. } => "ASK",
        purrdf_sparql_algebra::Query::Select { .. } => "SELECT",
        _ => "non-ASK/SELECT",
    };
    let expected_form = match kind {
        ValidatorKind::Ask => "ASK",
        ValidatorKind::Select => "SELECT",
    };
    if got_form != expected_form {
        return Err(format!(
            "component {component_iri} validator {validator} is declared as {kind:?} but the \
             query text parses to a {got_form} query"
        ));
    }

    let mut prebound: Vec<&str> = vec!["this"];
    if matches!(kind, ValidatorKind::Ask) {
        prebound.push("value");
    }
    prebound.extend(param_names.iter().map(String::as_str));
    let prebinding_result = match kind {
        ValidatorKind::Ask => crate::prebinding::check_ask(&query, &prebound),
        ValidatorKind::Select => crate::prebinding::check_select(&query, &prebound),
    };
    prebinding_result.map_err(|e| {
        format!(
            "component {component_iri} validator {validator} violates pre-binding restrictions: \
             {e}"
        )
    })?;

    Ok(Validator { kind, query_text })
}

/// Parse a single constraint component node and its parameters / validators.
fn parse_component(
    data: &IrDataGraph,
    doc_prefixes: &[(String, String)],
    component: &Term,
    component_iri: &str,
    subclass_memo: &mut HashMap<(String, String), bool>,
) -> Result<Component, String> {
    let param_nodes: Vec<Term> = objects_of(data, component, sh::PARAMETER_PROPERTY);
    let mut parameters = Vec::with_capacity(param_nodes.len());
    for param_node in param_nodes {
        parameters.push(parse_parameter(data, &param_node, component_iri)?);
    }
    parameters.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));
    let param_names: Vec<String> = parameters.iter().map(|p| p.name.clone()).collect();

    let node_validator = objects_of(data, component, sh::NODE_VALIDATOR)
        .into_iter()
        .next()
        .map(|v| {
            parse_validator(
                data,
                doc_prefixes,
                component,
                &v,
                &param_names,
                subclass_memo,
            )
        })
        .transpose()?;
    let property_validator = objects_of(data, component, sh::PROPERTY_VALIDATOR)
        .into_iter()
        .next()
        .map(|v| {
            parse_validator(
                data,
                doc_prefixes,
                component,
                &v,
                &param_names,
                subclass_memo,
            )
        })
        .transpose()?;
    let validator = objects_of(data, component, sh::VALIDATOR)
        .into_iter()
        .next()
        .map(|v| {
            parse_validator(
                data,
                doc_prefixes,
                component,
                &v,
                &param_names,
                subclass_memo,
            )
        })
        .transpose()?;

    Ok(Component {
        id: NamedNode::from(component_iri),
        parameters,
        node_validator,
        property_validator,
        validator,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::data::IrDataGraph;
    use crate::text_ingest::extract_prefixes;

    fn load_registry(ttl: &str, base_iri: &str) -> ComponentRegistry {
        let prefixes = extract_prefixes(ttl);
        let dataset: Arc<::purrdf::RdfDataset> =
            ::purrdf::parse_dataset(ttl.as_bytes(), "text/turtle", Some(base_iri))
                .expect("fixture parses");
        let data = IrDataGraph::new(dataset);
        ComponentRegistry::parse(&data, &prefixes).expect("registry parses")
    }

    #[test]
    fn sparql_local_name_extracts_suffix() {
        assert_eq!(
            sparql_local_name("http://example.org/ns#requiredParam"),
            "requiredParam"
        );
        assert_eq!(
            sparql_local_name("http://example.org/ns/requiredParam"),
            "requiredParam"
        );
        assert_eq!(sparql_local_name("ex:requiredParam"), "requiredParam");
        assert_eq!(sparql_local_name("requiredParam"), "requiredParam");
    }

    #[test]
    fn component_registry_construction_round_trips() {
        let mut registry = ComponentRegistry::default();
        let component = Component {
            id: NamedNode::from("http://example.org/ns#ExampleComponent"),
            parameters: vec![Parameter {
                path: NamedNode::from("http://example.org/ns#param"),
                name: "param".to_owned(),
                optional: false,
            }],
            node_validator: Some(Validator {
                kind: ValidatorKind::Ask,
                query_text: "ASK { ?this a ex:Thing }".to_owned(),
            }),
            property_validator: None,
            validator: None,
        };
        let id = component.id.as_str().to_owned();
        let param_path = component.parameters[0].path.as_str().to_owned();
        registry.components.insert(id.clone(), component);
        registry.by_parameter_path.insert(param_path, id);
        assert_eq!(registry.components.len(), 1);
        assert_eq!(registry.by_parameter_path.len(), 1);
    }

    #[test]
    fn parses_optional_001_component() {
        let ttl = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vectors/shacl/sparql/component/optional-001.ttl"
        ))
        .expect("fixture exists");
        let registry = load_registry(
            &ttl,
            "http://datashapes.org/sh/tests/sparql/component/optional-001.test",
        );

        let component = registry
            .components
            .get("http://datashapes.org/sh/tests/sparql/component/optional-001.test#TestConstraintComponent")
            .expect("TestConstraintComponent present");
        assert_eq!(component.parameters.len(), 2);
        assert_eq!(component.parameters[0].name, "optionalParam");
        assert!(component.parameters[0].optional);
        assert_eq!(component.parameters[1].name, "requiredParam");
        assert!(!component.parameters[1].optional);
        let validator = component.validator.as_ref().expect("validator present");
        assert!(matches!(validator.kind, ValidatorKind::Ask));
        assert!(validator.query_text.contains("PREFIX ex:"));
    }

    #[test]
    fn parses_property_validator_select_001_component() {
        let ttl = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vectors/shacl/sparql/component/propertyValidator-select-001.ttl"
        ))
        .expect("fixture exists");
        let registry = load_registry(
            &ttl,
            "http://datashapes.org/sh/tests/sparql/component/propertyValidator-select-001.test",
        );

        let component = registry
            .components
            .get("http://datashapes.org/sh/tests/sparql/component/propertyValidator-select-001.test#LanguageConstraintComponentUsingSELECT")
            .expect("LanguageConstraintComponentUsingSELECT present");
        assert_eq!(component.parameters.len(), 1);
        assert_eq!(component.parameters[0].name, "lang");
        assert!(!component.parameters[0].optional);
        let validator = component
            .property_validator
            .as_ref()
            .expect("propertyValidator present");
        assert!(matches!(validator.kind, ValidatorKind::Select));
        assert!(validator.query_text.contains("PREFIX ex:"));
    }

    #[test]
    fn parses_validator_001_component() {
        let ttl = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vectors/shacl/sparql/component/validator-001.ttl"
        ))
        .expect("fixture exists");
        let registry = load_registry(
            &ttl,
            "http://datashapes.org/sh/tests/sparql/component/validator-001.test",
        );

        let component = registry
            .components
            .get("http://datashapes.org/sh/tests/sparql/component/validator-001.test#TestConstraintComponent")
            .expect("TestConstraintComponent present");
        assert_eq!(component.parameters.len(), 2);
        assert_eq!(component.parameters[0].name, "test1");
        assert_eq!(component.parameters[1].name, "test2");
        let validator = component.validator.as_ref().expect("validator present");
        assert!(matches!(validator.kind, ValidatorKind::Ask));
        assert!(validator.query_text.contains("CONCAT"));
    }
}
