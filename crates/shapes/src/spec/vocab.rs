// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! What the vendored W3C SHACL 1.2 vocabularies DECLARE: every constraint
//! component and every node-expression function, with its parameters.
//!
//! The files are the crate-local byte-exact copies under `crates/shapes/spec/`
//! (written by the vendoring script and pinned by the frozen-corpus manifest), so
//! they ship inside the published package. `shacl-shacl.ttl` is not read here: it
//! is a shapes graph for validating shapes graphs, not a vocabulary declaration.

use std::collections::BTreeMap;

use ::purrdf::RdfDataset;

use super::FunctionClass;
use crate::data::{GraphFilter, native_quads};
use crate::model::{rdf, sh};
use crate::term::{NamedNode, Term};

/// The three vocabulary files, as `(file name, text)`.
const VOCABULARIES: [(&str, &str); 3] = [
    ("shacl.ttl", include_str!("../../spec/shacl.ttl")),
    ("shnex.ttl", include_str!("../../spec/shnex.ttl")),
    (
        "shnex-sparql.ttl",
        include_str!("../../spec/shnex-sparql.ttl"),
    ),
];

/// One declared parameter, as the vocabulary states it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeclaredParam {
    /// The parameter's `sh:path`.
    pub path: String,
    /// Whether the declaration states `sh:keyParameter true`.
    pub key: bool,
    /// Whether the declaration states `sh:optional true`.
    pub optional: bool,
}

/// One declared node-expression function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredFunction {
    /// The declaring class.
    pub class: FunctionClass,
    /// The declared parameters, sorted by path.
    pub params: Vec<DeclaredParam>,
}

/// Everything the vendored vocabularies declare, keyed by IRI.
#[derive(Clone, Debug, Default)]
pub struct Vocabulary {
    /// Every `sh:ConstraintComponent`, with its declared parameters sorted by path.
    pub components: BTreeMap<String, Vec<DeclaredParam>>,
    /// Every `sh:NamedParameterExpressionFunction`,
    /// `sh:ListParameterExpressionFunction` and `sh:NodeExpressionFunction`.
    pub functions: BTreeMap<String, DeclaredFunction>,
}

/// Read the vendored SHACL 1.2 vocabularies (`shacl.ttl`, `shnex.ttl`,
/// `shnex-sparql.ttl`) into their declarations.
///
/// # Errors
///
/// When a vendored file does not parse, or states a declaration this reader cannot
/// read (a non-IRI declaration, a parameter without an IRI `sh:path`, one IRI
/// declared under two classes). The files are frozen, so an error here means the
/// vendored bytes changed.
pub fn declared() -> Result<Vocabulary, String> {
    let mut out = Vocabulary::default();
    for (name, text) in VOCABULARIES {
        let dataset = crate::text_ingest::parse_turtle_to_dataset(text, None)
            .map_err(|errors| format!("{name} does not parse: {}", errors.join("; ")))?;
        read_components(&dataset, name, &mut out)?;
        read_functions(&dataset, name, &mut out)?;
    }
    Ok(out)
}

/// Every `sh:` and `shnex:` TERM the vendored `shacl.ttl` and `shnex.ttl` define:
/// each IRI subject in those two namespaces, other than the ontology IRIs
/// themselves and the `sh:Parameter` declarations (which name a component's or a
/// function's parameter, not a term of the vocabulary).
///
/// # Errors
///
/// When a vendored file does not parse.
pub fn declared_terms() -> Result<std::collections::BTreeSet<String>, String> {
    let rdf_type = Term::NamedNode(NamedNode::from(rdf::TYPE));
    let parameter = Term::NamedNode(NamedNode::from(sh::PARAMETER));
    let mut out = std::collections::BTreeSet::new();
    for (name, text) in &VOCABULARIES[..2] {
        let dataset = crate::text_ingest::parse_turtle_to_dataset(text, None)
            .map_err(|errors| format!("{name} does not parse: {}", errors.join("; ")))?;
        for (subject, _, _) in native_quads(&dataset, None, None, None, GraphFilter::AnyGraph) {
            let Term::NamedNode(iri) = &subject else {
                continue;
            };
            let iri = iri.as_str();
            let in_namespace = [sh::NS, crate::model::shnex::NS]
                .into_iter()
                .any(|ns| iri.len() > ns.len() && iri.starts_with(ns));
            if !in_namespace {
                continue;
            }
            let is_parameter = !native_quads(
                &dataset,
                Some(&subject),
                Some(&rdf_type),
                Some(&parameter),
                GraphFilter::AnyGraph,
            )
            .is_empty();
            if !is_parameter {
                out.insert(iri.to_owned());
            }
        }
    }
    Ok(out)
}

/// The IRI subjects typed `class` in `dataset`.
fn instances(dataset: &RdfDataset, class: &str, name: &str) -> Result<Vec<String>, String> {
    let rdf_type = Term::NamedNode(NamedNode::from(rdf::TYPE));
    let class = Term::NamedNode(NamedNode::from(class));
    let mut out = Vec::new();
    for (subject, _, _) in native_quads(
        dataset,
        None,
        Some(&rdf_type),
        Some(&class),
        GraphFilter::AnyGraph,
    ) {
        let Term::NamedNode(iri) = subject else {
            return Err(format!("{name} declares a non-IRI instance of {class}"));
        };
        out.push(iri.as_str().to_owned());
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn read_components(dataset: &RdfDataset, name: &str, out: &mut Vocabulary) -> Result<(), String> {
    for iri in instances(dataset, sh::CONSTRAINT_COMPONENT, name)? {
        let params = params_of(dataset, &iri, name)?;
        if out.components.insert(iri.clone(), params).is_some() {
            return Err(format!("<{iri}> is declared as a component twice"));
        }
    }
    Ok(())
}

fn read_functions(dataset: &RdfDataset, name: &str, out: &mut Vocabulary) -> Result<(), String> {
    for class in [
        FunctionClass::NamedParameter,
        FunctionClass::ListParameter,
        FunctionClass::Plain,
    ] {
        for iri in instances(dataset, class.iri(), name)? {
            let params = params_of(dataset, &iri, name)?;
            if out
                .functions
                .insert(iri.clone(), DeclaredFunction { class, params })
                .is_some()
            {
                return Err(format!(
                    "<{iri}> is declared under two node-expression function classes"
                ));
            }
        }
    }
    Ok(())
}

/// The declared `sh:parameter`s of `iri`, sorted by path.
fn params_of(dataset: &RdfDataset, iri: &str, name: &str) -> Result<Vec<DeclaredParam>, String> {
    let subject = Term::NamedNode(NamedNode::from(iri));
    let mut params = Vec::new();
    for param in objects(dataset, &subject, sh::PARAMETER_PROPERTY) {
        let paths = objects(dataset, &param, sh::PATH);
        let [Term::NamedNode(path)] = paths.as_slice() else {
            return Err(format!(
                "{name}: a sh:parameter of <{iri}> has no single IRI sh:path"
            ));
        };
        params.push(DeclaredParam {
            path: path.as_str().to_owned(),
            key: flag(dataset, &param, sh::KEY_PARAMETER),
            optional: flag(dataset, &param, sh::OPTIONAL),
        });
    }
    params.sort();
    Ok(params)
}

fn objects(dataset: &RdfDataset, subject: &Term, predicate: &str) -> Vec<Term> {
    let predicate = Term::NamedNode(NamedNode::from(predicate));
    native_quads(
        dataset,
        Some(subject),
        Some(&predicate),
        None,
        GraphFilter::AnyGraph,
    )
    .into_iter()
    .map(|(_, _, object)| object)
    .collect()
}

/// Whether `node` states `predicate true`.
fn flag(dataset: &RdfDataset, node: &Term, predicate: &str) -> bool {
    objects(dataset, node, predicate).iter().any(|value| {
        matches!(value, Term::Literal(lit) if matches!(
            purrdf_xsd::parse_by_iri(lit.value(), lit.datatype_str()),
            Ok(Some(purrdf_xsd::XsdValue::Boolean(true)))
        ))
    })
}
