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

use std::collections::HashMap;

use crate::term::NamedNode;

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
    /// Declared parameters, indexed in SHACL-specified order.
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
