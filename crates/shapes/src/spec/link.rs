// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The linker's per-declaration checks, shared by the function side
//! (`shapes::parser::custom_fn`) and the component side (`components`).
//!
//! Every check here answers one question about ONE declaration of a spec IRI the
//! table knows, and each refusal names the declaration and what to change.

use ::purrdf::RdfDataset;

use super::{ComponentRow, FunctionClass, NativeFunction};
use crate::data::{GraphFilter, native_quads};
use crate::model::sh;
use crate::term::{NamedNode, Term};

/// The predicates that GIVE a declaration an implementation. A built-in already
/// has one, so any of these on a built-in's declaration is a second definition.
const IMPLEMENTATION_MATERIAL: [&str; 6] = [
    sh::BODY_EXPRESSION,
    sh::VALIDATOR,
    sh::NODE_VALIDATOR,
    sh::PROPERTY_VALIDATOR,
    sh::ASK,
    sh::SELECT,
];

/// A parameter a declaration states: its `sh:path` and every `sh:keyParameter`
/// value it states (none, usually one).
struct StatedParam {
    path: String,
    keys: Vec<bool>,
}

fn objects(data: &RdfDataset, subject: &Term, predicate: &str) -> Vec<Term> {
    let predicate = Term::NamedNode(NamedNode::from(predicate));
    native_quads(
        data,
        Some(subject),
        Some(&predicate),
        None,
        GraphFilter::AnyGraph,
    )
    .into_iter()
    .map(|(_, _, object)| object)
    .collect()
}

/// Refuse a declaration of a built-in that carries implementation material.
fn refuse_material(data: &RdfDataset, iri: &str, what: &str) -> Result<(), String> {
    let id = Term::NamedNode(NamedNode::from(iri));
    for predicate in IMPLEMENTATION_MATERIAL {
        if !objects(data, &id, predicate).is_empty() {
            return Err(format!(
                "duplicate definition of <{iri}>: it is a {what} this engine implements \
                 natively, and the shapes graph also gives it an implementation through \
                 <{predicate}>; a built-in cannot be redefined, so remove that statement or \
                 declare the implementation under an IRI of your own"
            ));
        }
    }
    Ok(())
}

/// The parameters `iri`'s declaration states, each with its key flag.
fn stated_params(data: &RdfDataset, iri: &str) -> Result<Vec<StatedParam>, String> {
    let id = Term::NamedNode(NamedNode::from(iri));
    let mut out = Vec::new();
    for param in objects(data, &id, sh::PARAMETER_PROPERTY) {
        let paths = objects(data, &param, sh::PATH);
        let [Term::NamedNode(path)] = paths.as_slice() else {
            return Err(format!(
                "the declaration of the built-in <{iri}> has a sh:parameter without exactly one \
                 IRI sh:path"
            ));
        };
        let mut keys: Vec<bool> = Vec::new();
        for value in objects(data, &param, sh::KEY_PARAMETER) {
            match &value {
                Term::Literal(lit) => {
                    match purrdf_xsd::parse_by_iri(lit.value(), lit.datatype_str()) {
                        Ok(Some(purrdf_xsd::XsdValue::Boolean(flag))) => keys.push(flag),
                        _ => {
                            return Err(format!(
                                "the declaration of the built-in <{iri}> has a sh:parameter whose \
                                 sh:keyParameter is not an xsd:boolean: {value}"
                            ));
                        }
                    }
                }
                other => {
                    return Err(format!(
                        "the declaration of the built-in <{iri}> has a sh:parameter whose \
                         sh:keyParameter is not a literal: {other}"
                    ));
                }
            }
        }
        out.push(StatedParam {
            path: path.as_str().to_owned(),
            keys,
        });
    }
    Ok(out)
}

/// Refuse a declaration that states a parameter the built-in's signature does not
/// have, or states one with a different key flag.
///
/// Open-world: a declaration may state FEWER facts than the signature — fewer
/// parameters, or a parameter without its `sh:keyParameter` — and is then an
/// incomplete description, not a contradicting one. What it may not do is state a
/// parameter the built-in does not have, or state a `sh:keyParameter` value the
/// built-in's parameter does not have. Optionality is not compared — the
/// specification text decides it (see [`super::SPEC_TEXT_OPTIONALITY`]).
fn check_signature(
    data: &RdfDataset,
    iri: &str,
    what: &str,
    native: &[(&'static str, bool)],
) -> Result<(), String> {
    for stated in stated_params(data, iri)? {
        match native.iter().find(|(path, _)| *path == stated.path) {
            None => {
                let expected: Vec<&str> = native.iter().map(|(path, _)| *path).collect();
                return Err(format!(
                    "signature mismatch for <{iri}>: the declaration states a sh:parameter with \
                     sh:path <{}>, which the built-in {what} does not have (its parameters are \
                     {expected:?}); a built-in's signature is the specification's, so drop the \
                     parameter or declare your own IRI",
                    stated.path
                ));
            }
            Some(&(_, key)) if stated.keys.iter().any(|&stated_key| stated_key != key) => {
                return Err(format!(
                    "signature mismatch for <{iri}>: the declaration states sh:parameter <{}> \
                     with sh:keyParameter {}, but it is {}a key parameter of the built-in {what}",
                    stated.path,
                    !key,
                    if key { "" } else { "not " }
                ));
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// Link one declaration of built-in function `iri` under `class`: refuse a second
/// definition, a declaring-class mismatch, and a signature mismatch. `Ok` means the
/// declaration binds natively and adds nothing to any custom index.
pub(crate) fn bind_native_function(
    data: &RdfDataset,
    iri: &str,
    class: FunctionClass,
    native: NativeFunction,
) -> Result<(), String> {
    if native.class() != class {
        return Err(format!(
            "kind mismatch for <{iri}>: it is the built-in {} and is declared here as a {}; \
             a built-in's declaring class is the specification's",
            describe_class(native.class()),
            describe_class(class)
        ));
    }
    refuse_material(data, iri, "node-expression function")?;
    let signature: Vec<(&'static str, bool)> =
        native.params().iter().map(|p| (p.path, p.key)).collect();
    check_signature(data, iri, "node-expression function", &signature)
}

/// Link one `sh:ConstraintComponent` declaration of spec component `row`.
///
/// Returns whether the declaration is a USER implementation of a component the
/// engine does not evaluate — a validator-bearing declaration of an unimplemented
/// row, which the custom-component registry then evaluates like any other.
pub(crate) fn bind_spec_component(data: &RdfDataset, row: &ComponentRow) -> Result<bool, String> {
    let signature: Vec<(&'static str, bool)> = row.params.iter().map(|p| (p.path, false)).collect();
    check_signature(data, row.iri, "constraint component", &signature)?;
    match row.status {
        super::ComponentStatus::Native => {
            refuse_material(data, row.iri, "constraint component")?;
            Ok(false)
        }
        super::ComponentStatus::Unimplemented => {
            let id = Term::NamedNode(NamedNode::from(row.iri));
            Ok([sh::VALIDATOR, sh::NODE_VALIDATOR, sh::PROPERTY_VALIDATOR]
                .into_iter()
                .any(|predicate| !objects(data, &id, predicate).is_empty()))
        }
    }
}

/// Refuse a component declaration of an IRI the table knows as a FUNCTION.
pub(crate) fn refuse_component_declared_function(iri: &str) -> Result<(), String> {
    match super::native_function(iri) {
        Some(native) => Err(format!(
            "kind mismatch for <{iri}>: it is the built-in {} and is declared here as a \
             sh:ConstraintComponent",
            describe_class(native.class())
        )),
        None => Ok(()),
    }
}

/// Refuse a function declaration of an IRI the table knows as a COMPONENT.
pub(crate) fn refuse_function_declared_component(
    iri: &str,
    class: FunctionClass,
) -> Result<(), String> {
    match super::component(iri) {
        Some(_) => Err(format!(
            "kind mismatch for <{iri}>: it is a built-in constraint component and is declared \
             here as a {}",
            describe_class(class)
        )),
        None => Ok(()),
    }
}

fn describe_class(class: FunctionClass) -> &'static str {
    match class {
        FunctionClass::NamedParameter => "sh:NamedParameterExpressionFunction",
        FunctionClass::ListParameter => "sh:ListParameterExpressionFunction",
        FunctionClass::Plain => "sh:NodeExpressionFunction",
    }
}
