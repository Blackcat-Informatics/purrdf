// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parameter declarations and distinct constraint instances.

use super::{Component, Parameter, is_valid_varname, objects_of, sparql_local_name};
use crate::model::{sh, xsd};
use crate::term::{Term, sort_terms_canonical};

/// One constraint's parameter-to-RDF-term bindings.
pub(super) type ParameterBindings = Vec<(String, Term)>;

impl Component {
    /// SHACL Core §3.1.1: a single parameter declares one conjunct per value;
    /// every parameter of a multi-parameter component permits at most one value.
    /// All mandatory parameters must be present before a constraint is emitted.
    pub(crate) fn instantiate(
        &self,
        shape: &Term,
        mut values_of: impl FnMut(&str) -> Vec<Term>,
    ) -> Result<Vec<ParameterBindings>, String> {
        if let [parameter] = self.parameters.as_slice() {
            let mut values = values_of(parameter.path.as_str());
            sort_terms_canonical(&mut values);
            values.dedup();
            return Ok(values
                .into_iter()
                .map(|value| vec![(parameter.name.clone(), value)])
                .collect());
        }

        let mut bindings = Vec::with_capacity(self.parameters.len());
        let mut missing_required = false;
        for parameter in &self.parameters {
            let mut values = values_of(parameter.path.as_str());
            sort_terms_canonical(&mut values);
            values.dedup();
            if values.len() > 1 {
                return Err(format!(
                    "shape {shape} declares {} values for parameter {} of component {}, only one is allowed",
                    values.len(),
                    parameter.path,
                    self.id,
                ));
            }
            if let Some(value) = values.pop() {
                bindings.push((parameter.name.clone(), value));
            } else if !parameter.optional {
                // Inspect the other parameters as well: malformed cardinality
                // cannot depend on the lexical order of parameter predicates.
                missing_required = true;
            }
        }
        if missing_required {
            Ok(Vec::new())
        } else {
            Ok(vec![bindings])
        }
    }
}

/// Parse one declaration without filtering away malformed competing values.
pub(super) fn parse_parameter(
    data: &purrdf::RdfDataset,
    param_node: &Term,
    component_iri: &str,
) -> Result<Parameter, String> {
    let paths = objects_of(data, param_node, sh::PATH);
    let [Term::NamedNode(path)] = paths.as_slice() else {
        return Err(format!(
            "component {component_iri} parameter {param_node} must have exactly one sh:path IRI"
        ));
    };
    let name = sparql_local_name(path.as_str());
    if !is_valid_varname(&name) {
        return Err(format!(
            "component {component_iri} parameter path {path} yields invalid SPARQL variable name {name:?}"
        ));
    }
    let values = objects_of(data, param_node, sh::OPTIONAL);
    let optional = match values.as_slice() {
        [] => false,
        [Term::Literal(literal)] if literal.datatype_str() == xsd::BOOLEAN => {
            match literal.value() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => return Err(optional_error(component_iri, param_node)),
            }
        }
        _ => return Err(optional_error(component_iri, param_node)),
    };
    Ok(Parameter {
        path: path.clone(),
        name,
        optional,
    })
}

fn optional_error(component: &str, parameter: &Term) -> String {
    format!(
        "component {component} parameter {parameter} sh:optional must have at most one xsd:boolean value"
    )
}
