// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The linker's per-declaration checks, shared by the function side
//! (`shapes::parser::custom_fn`) and the component side (`components`).
//!
//! Every check here answers one question about ONE declaration of a spec IRI the
//! table knows, and each refusal names the declaration and what to change.

use ::purrdf::RdfDataset;

use super::{ComponentRow, FunctionClass, NativeFunction, census};
use crate::data::{GraphFilter, native_quads};
use crate::model::sh;
use crate::term::{NamedNode, Term};

/// The predicates that GIVE a function declaration an implementation. A built-in
/// function already has one, so any of these on its declaration is a second definition.
const FUNCTION_IMPLEMENTATION_MATERIAL: [&str; 6] = [
    sh::BODY_EXPRESSION,
    sh::VALIDATOR,
    sh::NODE_VALIDATOR,
    sh::PROPERTY_VALIDATOR,
    sh::ASK,
    sh::SELECT,
];

/// The predicates that would make a built-in COMPONENT's declaration a second
/// definition of it: a function body, or a query stated on the component itself
/// rather than on a validator.
///
/// Validators are not here. SHACL 1.2 SPARQL Extensions, "Validators", selects "one of
/// the values" of `sh:nodeValidator` / `sh:propertyValidator` / `sh:validator` as a
/// constraint's validator, so a component with several validators is a component with
/// several implementations of one semantics — and "SHACL processors may choose
/// alternative approaches as long as the outcome is equivalent" ("Validation with
/// SPARQL-based Constraint Components"). For a built-in, the native implementation is
/// the approach this engine chooses; the declared validators are alternatives it never
/// runs, and the component registry lists each one
/// ([`crate::validator_alternatives`]), refusing only a validator that is not a
/// well-formed SPARQL validator of its attachment.
const COMPONENT_IMPLEMENTATION_MATERIAL: [&str; 3] = [sh::BODY_EXPRESSION, sh::ASK, sh::SELECT];

/// The `sh:` predicates a built-in component's declaration may carry besides its
/// signature (`sh:parameter`) and its alternative validators, none of which changes
/// what the component checks:
///
/// * `sh:labelTemplate` — SHACL 1.2 SPARQL Extensions, "Label Templates": it "can be
///   used at any constraint component to suggest how constraints could be rendered to
///   humans".
/// * `sh:message` — SHACL 1.2 SPARQL Extensions, "Mapping of Solution Bindings to
///   Result Properties": a component's `sh:message` supplies `sh:resultMessage` "For
///   SPARQL-based constraint components", after "the values of sh:message of the
///   validator"; it is part of the SPARQL validation protocol, which the native
///   implementation supersedes. A built-in's result messages are SHACL 1.2 Core's:
///   those of the shape's own `sh:message`.
/// * every term the census classifies [`census::TermClass::NonValidating`] — SHACL 1.2
///   Core, "Non-Validating Shape Characteristics": "properties that are ignored by SHACL
///   processors" (`sh:name`, `sh:description`, `sh:order`, `sh:group`, …).
///
/// Predicates outside the `sh:` and `shnex:` namespaces (`rdfs:label`, `rdfs:comment`,
/// a vocabulary's own annotations such as `dash:localConstraint`) are not SHACL terms
/// and change nothing a SHACL processor evaluates, so they are not examined. Every
/// other `sh:` predicate — `sh:severity`, `sh:deactivated`, `sh:property`, … — would
/// state something about the component that its native implementation does not honour,
/// so it is refused.
const BUILTIN_COMPONENT_ANNOTATIONS: [&str; 2] = [sh::LABEL_TEMPLATE, sh::MESSAGE];

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
fn refuse_material(
    data: &RdfDataset,
    iri: &str,
    what: &str,
    material: &[&str],
) -> Result<(), String> {
    let id = Term::NamedNode(NamedNode::from(iri));
    for &predicate in material {
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
    refuse_material(
        data,
        iri,
        "node-expression function",
        &FUNCTION_IMPLEMENTATION_MATERIAL,
    )?;
    let signature: Vec<(&'static str, bool)> =
        native.params().iter().map(|p| (p.path, p.key)).collect();
    check_signature(data, iri, "node-expression function", &signature)
}

/// Link one `sh:ConstraintComponent` declaration of spec component `row`.
///
/// Every spec component row is evaluated natively, so the declaration is a
/// SIGNATURE: it must state the native parameter set; one carrying a body or a query
/// of its own is a duplicate definition; and any other `sh:` statement on it must be
/// one of the annotations [`BUILTIN_COMPONENT_ANNOTATIONS`] lists. Its validators are
/// alternatives the caller records, never refused here.
pub(crate) fn bind_spec_component(data: &RdfDataset, row: &ComponentRow) -> Result<(), String> {
    let signature: Vec<(&'static str, bool)> = row.params.iter().map(|p| (p.path, false)).collect();
    check_signature(data, row.iri, "constraint component", &signature)?;
    refuse_material(
        data,
        row.iri,
        "constraint component",
        &COMPONENT_IMPLEMENTATION_MATERIAL,
    )?;
    refuse_builtin_component_statements(data, row.iri)
}

/// Refuse a `sh:`/`shnex:` statement on a built-in component's declaration that is
/// neither its signature, an alternative validator, nor an annotation
/// [`BUILTIN_COMPONENT_ANNOTATIONS`] accepts.
fn refuse_builtin_component_statements(data: &RdfDataset, iri: &str) -> Result<(), String> {
    let id = Term::NamedNode(NamedNode::from(iri));
    let mut predicates: Vec<NamedNode> =
        native_quads(data, Some(&id), None, None, GraphFilter::AnyGraph)
            .into_iter()
            .map(|(_, predicate, _)| predicate)
            .collect();
    predicates.sort();
    predicates.dedup();
    for predicate in &predicates {
        let p = predicate.as_str();
        if !census::is_census_namespace(p)
            || matches!(
                p,
                sh::PARAMETER_PROPERTY
                    | sh::VALIDATOR
                    | sh::NODE_VALIDATOR
                    | sh::PROPERTY_VALIDATOR
            )
            || BUILTIN_COMPONENT_ANNOTATIONS.contains(&p)
            || census::classify(p).is_some_and(|row| row.class == census::TermClass::NonValidating)
        {
            continue;
        }
        return Err(format!(
            "the declaration of the built-in <{iri}> carries <{p}>, which is not an annotation \
             of a constraint component: this engine implements <{iri}> natively, with the \
             specification's semantics, so a statement that would change what it checks or \
             reports is refused rather than ignored; remove it, or declare a component under \
             an IRI of your own"
        ));
    }
    Ok(())
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
