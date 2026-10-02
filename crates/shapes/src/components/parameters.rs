// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Parameter declarations and distinct constraint instances.

use super::{Component, Parameter, is_reserved_parameter_name, is_valid_varname, objects_of};
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
///
/// # Errors
///
/// The violated syntax rule of SHACL 1.2 SPARQL Extensions ("Summary of Syntax Rules")
/// and the diagnostic: `Parameter-predicate-count` / `Parameter` for the `sh:path`,
/// `parameter-name-not-in` / `parameter-name-VARNAME` for the name it yields,
/// `optional-maxCount` / `optional-datatype` for `sh:optional`.
pub(super) fn parse_parameter(
    data: &purrdf_rdf::RdfDataset,
    param_node: &Term,
    component_iri: &str,
) -> Result<Parameter, (&'static str, String)> {
    let paths = objects_of(data, param_node, sh::PATH);
    let [Term::NamedNode(path)] = paths.as_slice() else {
        return Err((
            if paths.len() == 1 {
                "Parameter"
            } else {
                "Parameter-predicate-count"
            },
            format!(
                "component {component_iri} parameter {param_node} must have exactly one sh:path IRI"
            ),
        ));
    };
    let name = purrdf_iri::local_name(path.as_str()).to_owned();
    if !is_valid_varname(&name) {
        return Err((
            if is_reserved_parameter_name(&name) {
                "parameter-name-not-in"
            } else {
                "parameter-name-VARNAME"
            },
            format!(
                "component {component_iri} parameter path {path} yields invalid SPARQL variable \
                 name {name:?}"
            ),
        ));
    }
    let values = objects_of(data, param_node, sh::OPTIONAL);
    let optional = match values.as_slice() {
        [] => false,
        [Term::Literal(literal)] if literal.datatype_str() == xsd::BOOLEAN => {
            match literal.value() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => {
                    return Err((
                        "optional-datatype",
                        optional_error(component_iri, param_node),
                    ));
                }
            }
        }
        [_] => {
            return Err((
                "optional-datatype",
                optional_error(component_iri, param_node),
            ));
        }
        _ => {
            return Err((
                "optional-maxCount",
                optional_error(component_iri, param_node),
            ));
        }
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
