// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL-SPARQL result annotations: read at load, applied per solution.
//!
//! SHACL 1.2 SPARQL Extensions, "Annotation Properties": "Implementations that
//! support this feature make it possible to inject annotation properties into the
//! validation result nodes created for each solution produced by the SELECT queries
//! of a SPARQL-based constraint or constraint component. Any such annotation
//! property needs to be declared via a value of sh:resultAnnotation at the subject
//! of the sh:select or sh:ask triple. The values of sh:resultAnnotation are called
//! result annotations and are either IRIs or blank nodes."
//!
//! Its syntax rules, each enforced at load:
//!
//! * `sh:annotationProperty` — "Each result annotation has exactly one value for
//!   the property sh:annotationProperty and this value is an IRI."
//! * `sh:annotationVarName` — "Each result annotation has at most 1 value for the
//!   property sh:annotationVarName and this value is literal with datatype
//!   xsd:string." The name must also be a SPARQL `VARNAME`: a name no query can
//!   bind would make the annotation silently fall back to its defaults for every
//!   solution, which is the author's mistake reported as nothing.
//! * `sh:annotationValue` — "Constant RDF terms that shall be used as default
//!   values."
//!
//! Beyond the specification's text, an annotation property that is itself a term
//! of the SHACL validation-report vocabulary (`sh:focusNode`, `sh:resultMessage`,
//! …) is refused: injecting it would give a validation result a second focus node
//! or a message no constraint declared, so the report would state something the
//! validation did not find.
//!
//! The mapping, per solution ([`annotate`]): "Use the value of the property
//! sh:annotationVarName. If no such value exists, use the local name of the value
//! of sh:annotationProperty as the variable name. If a variable name could be
//! determined, then the SHACL processor copies the binding for the given variable
//! as a value for the property specified using sh:annotationProperty into the
//! validation result that is being produced for the current solution. If the
//! variable has no binding in the result set solution, then the values of
//! sh:annotationValue are used, if present."

use ::purrdf::RdfDataset;

use crate::data::{GraphFilter, native_quads};
use crate::model::{sh, xsd};
use crate::shapes::ResultAnnotation;
use crate::spec::census::{self, Role, TermClass};
use crate::term::{NamedNode, Term, canonical_cmp};

/// The predicates a result-annotation node may carry besides `rdf:type`.
const ANNOTATION_TERMS: [&str; 3] = [
    sh::ANNOTATION_PROPERTY,
    sh::ANNOTATION_VAR_NAME,
    sh::ANNOTATION_VALUE,
];

/// Every result annotation `executable` (the subject of an `sh:select` or `sh:ask`
/// triple) declares, in canonical order.
///
/// # Errors
///
/// A value of `sh:resultAnnotation` that is a literal or a triple term, an
/// annotation without exactly one IRI `sh:annotationProperty`, with more than one
/// `sh:annotationVarName` or one that is not an `xsd:string` naming a SPARQL
/// variable, an annotation property of the SHACL report vocabulary, or a SHACL term
/// on the annotation node that is none of the three annotation properties.
pub(crate) fn parse(data: &RdfDataset, executable: &Term) -> Result<Vec<ResultAnnotation>, String> {
    let mut out = Vec::new();
    for (_, _, node) in objects(data, executable, sh::RESULT_ANNOTATION) {
        if !matches!(node, Term::NamedNode(_) | Term::BlankNode(_)) {
            return Err(format!(
                "sh:resultAnnotation on {executable} is {node}; SHACL 1.2 SPARQL Extensions: \
                 result annotations \"are either IRIs or blank nodes\""
            ));
        }
        out.push(parse_one(data, &node)?);
    }
    sort(&mut out);
    Ok(out)
}

/// The canonical order of a list of result annotations — by property, then
/// variable, then default values in canonical term order — without repeats. The
/// parser keeps this order and the prepared-product reader refuses any other.
pub(crate) fn sort(annotations: &mut Vec<ResultAnnotation>) {
    annotations.sort_by(|a, b| {
        a.property
            .cmp(&b.property)
            .then_with(|| a.variable.cmp(&b.variable))
            .then_with(|| {
                a.default_values
                    .iter()
                    .zip(&b.default_values)
                    .map(|(x, y)| canonical_cmp(x, y))
                    .find(|order| order.is_ne())
                    .unwrap_or_else(|| a.default_values.len().cmp(&b.default_values.len()))
            })
    });
    annotations.dedup();
}

/// One result-annotation node.
fn parse_one(data: &RdfDataset, node: &Term) -> Result<ResultAnnotation, String> {
    for (_, predicate, _) in native_quads(data, Some(node), None, None, GraphFilter::AnyGraph) {
        let p = predicate.as_str();
        if !census::is_census_namespace(p) || ANNOTATION_TERMS.contains(&p) {
            continue;
        }
        if census::classify(p).is_some_and(|row| row.class == TermClass::NonValidating) {
            continue;
        }
        return Err(format!(
            "result annotation {node} carries <{p}>, which is not sh:annotationProperty, \
             sh:annotationVarName or sh:annotationValue; it is refused rather than silently \
             ignored"
        ));
    }
    let properties: Vec<Term> = objects(data, node, sh::ANNOTATION_PROPERTY)
        .into_iter()
        .map(|(_, _, o)| o)
        .collect();
    let property = match properties.as_slice() {
        [Term::NamedNode(property)] => property.clone(),
        _ => {
            return Err(format!(
                "result annotation {node} has {} sh:annotationProperty value(s); SHACL 1.2 SPARQL \
                 Extensions: \"Each result annotation has exactly one value for the property \
                 sh:annotationProperty and this value is an IRI\"",
                properties.len()
            ));
        }
    };
    if let Some(row) = census::classify(property.as_str())
        && row.class == TermClass::Structural(Role::Report)
    {
        return Err(format!(
            "result annotation {node} names <{}> as its sh:annotationProperty, a property of the \
             SHACL validation-report vocabulary; injecting it would make every result state a \
             report fact no constraint produced",
            property.as_str()
        ));
    }
    let names: Vec<Term> = objects(data, node, sh::ANNOTATION_VAR_NAME)
        .into_iter()
        .map(|(_, _, o)| o)
        .collect();
    let variable = match names.as_slice() {
        [] => {
            // "If no such value exists, use the local name of the value of
            // sh:annotationProperty as the variable name." A local name that is not a
            // SPARQL variable name determines no variable, and then only the defaults
            // apply ("If a variable name could be determined, …").
            let local = crate::components::sparql_local_name(property.as_str());
            purrdf_sparql_algebra::lexer::is_varname(&local).then_some(local)
        }
        [Term::Literal(name)]
            if name.datatype_str() == xsd::STRING
                && purrdf_sparql_algebra::lexer::is_varname(name.value()) =>
        {
            Some(name.value().to_owned())
        }
        [single] => {
            return Err(format!(
                "sh:annotationVarName on result annotation {node} is {single}; it must be an \
                 xsd:string literal that is a SPARQL variable name (without its ? or $ sigil), \
                 or no solution could ever bind it"
            ));
        }
        _ => {
            return Err(format!(
                "result annotation {node} has {} sh:annotationVarName values; SHACL 1.2 SPARQL \
                 Extensions: \"Each result annotation has at most 1 value for the property \
                 sh:annotationVarName\"",
                names.len()
            ));
        }
    };
    let mut default_values: Vec<Term> = objects(data, node, sh::ANNOTATION_VALUE)
        .into_iter()
        .map(|(_, _, o)| o)
        .collect();
    crate::term::sort_terms_canonical(&mut default_values);
    default_values.dedup();
    Ok(ResultAnnotation {
        property,
        variable,
        default_values,
    })
}

/// Every `(subject, predicate, object)` with `subject` and `predicate`.
fn objects(data: &RdfDataset, subject: &Term, predicate: &str) -> Vec<(Term, NamedNode, Term)> {
    native_quads(
        data,
        Some(subject),
        Some(&Term::NamedNode(NamedNode::from(predicate))),
        None,
        GraphFilter::AnyGraph,
    )
}

/// The annotation pairs one solution gives a validation result: for each
/// annotation, the solution's binding of its variable (looked up with `binding`),
/// or else its default values. Sorted by property, then canonical term order,
/// without duplicates. Allocates nothing when `annotations` is empty.
pub(crate) fn annotate(
    annotations: &[ResultAnnotation],
    mut binding: impl FnMut(&str) -> Option<Term>,
) -> Vec<(NamedNode, Term)> {
    if annotations.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<(NamedNode, Term)> = Vec::new();
    for annotation in annotations {
        match annotation.variable.as_deref().and_then(&mut binding) {
            Some(value) => out.push((annotation.property.clone(), value)),
            None => out.extend(
                annotation
                    .default_values
                    .iter()
                    .map(|value| (annotation.property.clone(), value.clone())),
            ),
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| canonical_cmp(&a.1, &b.1)));
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::annotate;
    use crate::shapes::ResultAnnotation;
    use crate::term::{Literal, NamedNode, Term};

    fn iri(local: &str) -> NamedNode {
        NamedNode::from(format!("http://example.org/ns#{local}").as_str())
    }

    #[test]
    fn a_bound_variable_wins_and_an_unbound_one_takes_the_defaults() {
        let annotations = [
            ResultAnnotation {
                property: iri("time"),
                variable: Some("time".to_owned()),
                default_values: vec![Term::Literal(Literal::new_simple_literal("never"))],
            },
            ResultAnnotation {
                property: iri("who"),
                variable: Some("who".to_owned()),
                default_values: vec![Term::Literal(Literal::new_simple_literal("nobody"))],
            },
        ];
        let now = Term::Literal(Literal::new_simple_literal("now"));
        let pairs = annotate(&annotations, |name| (name == "time").then(|| now.clone()));
        assert_eq!(
            pairs,
            vec![
                (iri("time"), now),
                (
                    iri("who"),
                    Term::Literal(Literal::new_simple_literal("nobody"))
                ),
            ]
        );
    }

    #[test]
    fn no_variable_means_only_the_defaults() {
        let annotations = [ResultAnnotation {
            property: iri("created-at"),
            variable: None,
            default_values: vec![],
        }];
        assert_eq!(
            annotate(&annotations, |_| panic!("no variable to look up")),
            Vec::<(NamedNode, Term)>::new()
        );
    }
}
