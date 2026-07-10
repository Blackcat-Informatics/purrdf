// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Entailment-aware SPARQL orchestration over the native PurRDF engines.

use std::sync::Arc;

use purrdf_entail::{EntailError, QNode, QTriple, Regime, RuleSet};
use purrdf_rdf::{
    RdfDataset, RdfDiagnostic, RdfTextDirection, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_algebra::{
    BaseDirection, GraphPattern, Literal, NamedNodePattern, Query, TermPattern,
};
use purrdf_sparql_eval::NativeSparqlEngine;

/// Entailment behavior applied before evaluating one SPARQL query.
#[derive(Debug, Clone, Copy)]
pub enum QueryEntailment<'a> {
    /// Query asserted data directly.
    Simple,
    /// Materialize RDF entailment.
    Rdf,
    /// Materialize RDFS entailment.
    Rdfs,
    /// Materialize OWL 2 RL entailment.
    OwlRl,
    /// Perform query-directed OWL Direct-Semantics augmentation.
    OwlDirect,
    /// Materialize the supplied RIF-Core rule set.
    Rif(&'a RuleSet),
}

/// Failure from entailment-aware query preparation or evaluation.
#[derive(Debug)]
pub enum ReasoningError {
    /// SPARQL parsing or evaluation failed.
    Query(RdfDiagnostic),
    /// Entailment or rule materialization failed.
    Entailment(EntailError),
}

impl std::fmt::Display for ReasoningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query(error) => write!(f, "SPARQL query failed: {error}"),
            Self::Entailment(error) => write!(f, "entailment failed: {error}"),
        }
    }
}

impl std::error::Error for ReasoningError {}

impl From<RdfDiagnostic> for ReasoningError {
    fn from(value: RdfDiagnostic) -> Self {
        Self::Query(value)
    }
}

impl From<EntailError> for ReasoningError {
    fn from(value: EntailError) -> Self {
        Self::Entailment(value)
    }
}

/// Evaluate SPARQL under an explicit native entailment regime.
///
/// # Errors
///
/// Returns [`ReasoningError::Query`] for SPARQL failures and
/// [`ReasoningError::Entailment`] for malformed or inconsistent knowledge bases.
pub fn query_with_entailment(
    engine: &NativeSparqlEngine,
    dataset: &Arc<RdfDataset>,
    request: SparqlRequest<'_>,
    entailment: QueryEntailment<'_>,
) -> Result<SparqlResult, ReasoningError> {
    // Parse first so invalid queries fail before potentially expensive closure work.
    // OWL Direct also inspects this same cached plan, avoiding a second parse/cache lookup.
    let prepared_query = engine.prepare_query(request.query, request.base_iri)?;
    let prepared = match entailment {
        QueryEntailment::Simple => Arc::clone(dataset),
        QueryEntailment::Rdf => purrdf_entail::materialize(dataset, Regime::Rdf)?,
        QueryEntailment::Rdfs => purrdf_entail::materialize(dataset, Regime::Rdfs)?,
        QueryEntailment::OwlRl => purrdf_entail::materialize(dataset, Regime::OwlRl)?,
        QueryEntailment::OwlDirect => {
            let pattern = collect_query_bgp(&prepared_query.query);
            purrdf_entail::materialize_dl(dataset, &pattern)?
        }
        QueryEntailment::Rif(ruleset) => purrdf_entail::materialize_rif(dataset, ruleset)?,
    };
    engine
        .query_prepared(&prepared, &prepared_query, request.substitutions)
        .map_err(Into::into)
}

fn collect_query_bgp(query: &Query) -> Vec<QTriple> {
    let pattern = match query {
        Query::Select { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. }
        | Query::Ask { pattern, .. } => pattern,
    };
    let mut triples = Vec::new();
    collect_bgp(pattern, &mut triples);
    triples
}

fn collect_bgp(pattern: &GraphPattern, output: &mut Vec<QTriple>) {
    match pattern {
        GraphPattern::Bgp { patterns } => output.extend(patterns.iter().filter_map(|pattern| {
            Some(QTriple {
                s: term_to_qnode(&pattern.subject)?,
                p: named_node_pattern_to_qnode(&pattern.predicate),
                o: term_to_qnode(&pattern.object)?,
            })
        })),
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::LeftJoin { left, right, .. } => {
            collect_bgp(left, output);
            collect_bgp(right, output);
        }
        GraphPattern::Filter { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Extend { inner, .. }
        | GraphPattern::Service { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Group { inner, .. } => collect_bgp(inner, output),
        GraphPattern::Path { .. } | GraphPattern::Values { .. } => {}
    }
}

fn term_to_qnode(term: &TermPattern) -> Option<QNode> {
    Some(match term {
        TermPattern::Variable(variable) => QNode::Var(variable.as_str().to_owned()),
        TermPattern::NamedNode(node) => QNode::Term(TermValue::iri(node.as_str())),
        TermPattern::BlankNode(node) => QNode::Term(TermValue::blank(node.as_str())),
        TermPattern::Literal(literal) => QNode::Term(literal_to_term_value(literal)),
        TermPattern::Triple(_) => return None,
    })
}

fn named_node_pattern_to_qnode(pattern: &NamedNodePattern) -> QNode {
    match pattern {
        NamedNodePattern::NamedNode(node) => QNode::Term(TermValue::iri(node.as_str())),
        NamedNodePattern::Variable(variable) => QNode::Var(variable.as_str().to_owned()),
    }
}

fn literal_to_term_value(literal: &Literal) -> TermValue {
    match literal.language() {
        Some(language) => TermValue::Literal {
            lexical_form: literal.value().to_owned(),
            datatype: literal.datatype().as_str().to_owned(),
            language: Some(language.to_ascii_lowercase()),
            direction: literal.direction().map(|direction| match direction {
                BaseDirection::Ltr => RdfTextDirection::Ltr,
                BaseDirection::Rtl => RdfTextDirection::Rtl,
            }),
        },
        None => TermValue::typed_literal(literal.value(), literal.datatype().as_str()),
    }
}

#[cfg(test)]
mod tests {
    use purrdf_entail::{Atom, RifTerm, Rule, RuleSet};
    use purrdf_rdf::{RdfDatasetBuilder, TermValue};

    use super::*;

    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const RDFS_SUBCLASS: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

    fn hierarchy() -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let cat = builder.intern_iri("https://example.org/Cat");
        let animal = builder.intern_iri("https://example.org/Animal");
        let lillith = builder.intern_iri("https://example.org/lillith");
        let rdf_type = builder.intern_iri(RDF_TYPE);
        let subclass = builder.intern_iri(RDFS_SUBCLASS);
        builder.push_quad(cat, subclass, animal, None);
        builder.push_quad(lillith, rdf_type, cat, None);
        builder.freeze().unwrap()
    }

    fn ask(mode: QueryEntailment<'_>) -> SparqlResult {
        let query = "ASK { <https://example.org/lillith> a <https://example.org/Animal> }";
        query_with_entailment(
            &NativeSparqlEngine::new(),
            &hierarchy(),
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            mode,
        )
        .unwrap()
    }

    #[test]
    fn rdfs_query_sees_derived_type() {
        assert!(matches!(
            ask(QueryEntailment::Rdfs),
            SparqlResult::Boolean(true)
        ));
    }

    #[test]
    fn owl_rl_query_sees_derived_type() {
        assert!(matches!(
            ask(QueryEntailment::OwlRl),
            SparqlResult::Boolean(true)
        ));
    }

    #[test]
    fn owl_direct_query_uses_the_query_bgp() {
        assert!(matches!(
            ask(QueryEntailment::OwlDirect),
            SparqlResult::Boolean(true)
        ));
    }

    #[test]
    fn rdf_query_types_predicates_as_properties() {
        let query = format!(
            "ASK {{ <{RDFS_SUBCLASS}> a <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property> }}"
        );
        let result = query_with_entailment(
            &NativeSparqlEngine::new(),
            &hierarchy(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryEntailment::Rdf,
        )
        .unwrap();
        assert!(matches!(result, SparqlResult::Boolean(true)));
    }

    #[test]
    fn simple_query_does_not_invent_closure() {
        assert!(matches!(
            ask(QueryEntailment::Simple),
            SparqlResult::Boolean(false)
        ));
    }

    #[test]
    fn rif_query_sees_rule_derived_fact() {
        let mut rules = RuleSet::new();
        rules.push_rule(Rule {
            body: vec![Atom {
                s: RifTerm::Var("subject".to_owned()),
                p: RifTerm::Const(TermValue::iri(RDF_TYPE)),
                o: RifTerm::Const(TermValue::iri("https://example.org/Cat")),
            }],
            head: vec![Atom {
                s: RifTerm::Var("subject".to_owned()),
                p: RifTerm::Const(TermValue::iri(RDF_TYPE)),
                o: RifTerm::Const(TermValue::iri("https://example.org/Animal")),
            }],
        });
        assert!(matches!(
            ask(QueryEntailment::Rif(&rules)),
            SparqlResult::Boolean(true)
        ));
    }
}
