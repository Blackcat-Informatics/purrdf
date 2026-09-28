// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A global SHACL SPARQL rule whose CONSTRUCT is a conjunctive pattern, read as rule
//! elements.
//!
//! A SHACL rule is otherwise a PRODUCER: its CONSTRUCT is run over the whole evaluation
//! graph on every iteration, so an iteration costs the size of the graph and a rule that
//! steps a value once per iteration costs the square of its steps. SHACL 1.2 Inference
//! Rules defines an iteration by what it infers, not by how: "Execute one iteration over
//! all iterating rules in the layer while the iteration has produced newly inferred
//! triples". For a query whose solutions only grow as the graph grows, the triples an
//! iteration infers anew are exactly those of the solutions that use a triple the
//! previous iteration added, which is what `purrdf-datalog`'s semi-naive rounds compute —
//! the same inference graph, iteration for iteration, at a cost proportional to what is
//! new. So such a query is lowered to the rule-set IR's element form
//! ([`ElementRule`]), and evaluated with the same schedule, layers and orders.
//!
//! # Exactly the queries whose element reading is the query
//!
//! [`elements`] reads a query only when every solution and every constructed triple of
//! the element rule is the query's, and declines — leaving the rule a producer — for
//! anything else:
//!
//! * The rule is GLOBAL (a shape rule is executed per focus node) and has no template
//!   parameters; its query names no `FROM` dataset and no graph.
//! * The `WHERE` pattern is triple patterns (no property path, no triple term), `FILTER`
//!   and `BIND`, joined — no `OPTIONAL`, `UNION`, `MINUS`, `VALUES`, sub-query, `GRAPH`
//!   or `SERVICE`, whose solutions do not only grow. A `FILTER` or `BIND` reads only
//!   variables its own group pattern binds, so flattening the groups changes no scope.
//! * No expression reads the graph (`EXISTS`), mints or reads the clock (`BNODE`,
//!   `RAND`, `NOW`, `UUID`, `STRUUID`), or calls an extension function (a SHACL function
//!   may query the data graph); XSD casts are built in.
//! * A `BIND` that errs leaves its variable unbound in SPARQL, which skips every
//!   template triple mentioning it, while an assignment element rejects the solution.
//!   The two agree when every template triple mentions every `BIND` variable and no
//!   triple pattern after it does.
//! * SPARQL CONSTRUCT skips an ill-formed triple, while an element rule's head is an RDF
//!   triple by refusal. The two agree when each template subject is an IRI or a variable
//!   some triple pattern binds as a subject, and each predicate an IRI or a variable one
//!   binds as a predicate.
//! * The template has no blank node: a CONSTRUCT mints fresh ones on every execution, so
//!   "some rules may produce fresh blank nodes with each execution and therefore cause
//!   infinite iterations", and reading it once per solution would terminate a rule the
//!   specification lets run on.

use purrdf_sparql_algebra::{
    Expression, Function, GraphPattern, NamedNodePattern, Query, SparqlParser,
    TermPattern as AlgebraTerm, TriplePattern as AlgebraTriple,
};

use super::ir::{Element, ElementRule, PatternTerm, TriplePattern, visit_expression};
use crate::term::{Literal, NamedNode, Term};

/// The XSD namespace, whose datatype IRIs are the built-in casts.
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

/// The element rule a global SPARQL rule's `construct` query is exactly, or `None` when
/// it is not one (see the [module docs](self)).
pub(crate) fn elements(construct: &str) -> Option<ElementRule> {
    let Ok(Query::Construct {
        template,
        pattern,
        dataset,
        ..
    }) = SparqlParser::new().parse_query(construct)
    else {
        return None;
    };
    if !dataset.default.is_empty() || !dataset.named.is_empty() {
        return None;
    }
    let mut body: Vec<Element> = Vec::new();
    let mut bound: Vec<String> = Vec::new();
    flatten(&pattern, &mut body, &mut bound)?;

    // The variables each element assigns, and no triple pattern after an assignment
    // mentions its variable.
    let mut assigned: Vec<String> = Vec::new();
    let mut subjects: Vec<String> = Vec::new();
    let mut predicates: Vec<String> = Vec::new();
    for element in &body {
        match element {
            Element::Assign { variable, .. } => assigned.push(variable.clone()),
            Element::Pattern(triple) => {
                if triple
                    .variable_names()
                    .iter()
                    .any(|name| assigned.contains(name))
                {
                    return None;
                }
                if let PatternTerm::Variable(name) = &triple.subject {
                    subjects.push(name.clone());
                }
                if let PatternTerm::Variable(name) = &triple.predicate {
                    predicates.push(name.clone());
                }
            }
            Element::Filter(_) | Element::Negation { .. } => {}
        }
    }

    let mut head: Vec<TriplePattern> = Vec::with_capacity(template.len());
    for quad in &template {
        if quad.graph.is_some() {
            return None;
        }
        let triple = pattern_triple(&quad.triple)?;
        if triple.has_blank_node() {
            return None;
        }
        let safe_subject = match &triple.subject {
            PatternTerm::Term(Term::NamedNode(_)) => true,
            PatternTerm::Variable(name) => subjects.contains(name),
            _ => false,
        };
        let safe_predicate = match &triple.predicate {
            PatternTerm::Term(Term::NamedNode(_)) => true,
            PatternTerm::Variable(name) => predicates.contains(name),
            _ => false,
        };
        let names = triple.variable_names();
        if !safe_subject
            || !safe_predicate
            || names.iter().any(|name| !bound.contains(name))
            || assigned.iter().any(|name| !names.contains(name))
        {
            return None;
        }
        head.push(triple);
    }
    if head.is_empty() {
        return None;
    }
    let rule = ElementRule {
        head,
        body,
        data: false,
    };
    rule.check_well_formed().ok()?;
    Some(rule)
}

/// Append the elements of `pattern` to `body`, and the variables it binds to `bound`;
/// `None` for a pattern outside the conjunctive fragment.
fn flatten(pattern: &GraphPattern, body: &mut Vec<Element>, bound: &mut Vec<String>) -> Option<()> {
    match pattern {
        GraphPattern::Bgp { patterns } => {
            for triple in patterns {
                let triple = pattern_triple(triple)?;
                for name in triple.variable_names() {
                    if !bound.contains(&name) {
                        bound.push(name);
                    }
                }
                body.push(Element::Pattern(triple));
            }
            Some(())
        }
        GraphPattern::Join { left, right } => {
            flatten(left, body, bound)?;
            flatten(right, body, bound)
        }
        // The expression reads only the variables its group binds: flattening the
        // group into its neighbours then binds nothing it could read.
        GraphPattern::Filter { expr, inner } => {
            let mut own: Vec<String> = Vec::new();
            flatten(inner, body, &mut own)?;
            if !expression_admitted(expr, &own) {
                return None;
            }
            body.push(Element::Filter(expr.clone()));
            merge(bound, own);
            Some(())
        }
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => {
            let mut own: Vec<String> = Vec::new();
            flatten(inner, body, &mut own)?;
            let name = variable.as_str().to_owned();
            if own.contains(&name)
                || bound.contains(&name)
                || !expression_admitted(expression, &own)
            {
                return None;
            }
            body.push(Element::Assign {
                variable: name.clone(),
                expression: expression.clone(),
            });
            own.push(name);
            merge(bound, own);
            Some(())
        }
        _ => None,
    }
}

/// Add `from`'s variables to `into`.
fn merge(into: &mut Vec<String>, from: Vec<String>) {
    for name in from {
        if !into.contains(&name) {
            into.push(name);
        }
    }
}

/// Whether `expression` reads only `bound` variables and nothing but its arguments.
fn expression_admitted(expression: &Expression, bound: &[String]) -> bool {
    let mut admitted = true;
    visit_expression(expression, &mut |node| match node {
        Expression::Variable(variable) | Expression::Bound(variable) => {
            admitted &= bound.iter().any(|name| name == variable.as_str());
        }
        Expression::Exists(_) => admitted = false,
        Expression::FunctionCall(function, _) => {
            admitted &= match function {
                Function::BNode
                | Function::Rand
                | Function::Now
                | Function::Uuid
                | Function::StrUuid
                | Function::Purrdf(_) => false,
                Function::Custom(iri) => iri.as_str().starts_with(XSD),
                _ => true,
            };
        }
        _ => {}
    });
    admitted
}

/// A WHERE or template triple pattern as an IR pattern; `None` for a triple term.
fn pattern_triple(triple: &AlgebraTriple) -> Option<TriplePattern> {
    let predicate = match &triple.predicate {
        NamedNodePattern::NamedNode(iri) => {
            PatternTerm::Term(Term::NamedNode(NamedNode::new_unchecked(iri.as_str())))
        }
        NamedNodePattern::Variable(variable) => PatternTerm::Variable(variable.as_str().to_owned()),
    };
    Some(TriplePattern::new(
        pattern_term(&triple.subject)?,
        predicate,
        pattern_term(&triple.object)?,
    ))
}

/// One position of a triple pattern; `None` for a triple term or a directional literal.
fn pattern_term(term: &AlgebraTerm) -> Option<PatternTerm> {
    Some(match term {
        AlgebraTerm::Variable(variable) => PatternTerm::Variable(variable.as_str().to_owned()),
        AlgebraTerm::BlankNode(blank) => PatternTerm::BlankNode(blank.as_str().to_owned()),
        AlgebraTerm::NamedNode(iri) => {
            PatternTerm::Term(Term::NamedNode(NamedNode::new_unchecked(iri.as_str())))
        }
        AlgebraTerm::Literal(literal) => {
            if literal.direction().is_some() {
                return None;
            }
            PatternTerm::Term(Term::Literal(match literal.language() {
                Some(language) => {
                    Literal::new_language_tagged_literal_unchecked(literal.value(), language)
                }
                None => Literal::new_typed_literal(
                    literal.value(),
                    NamedNode::new_unchecked(literal.datatype().as_str()),
                ),
            }))
        }
        AlgebraTerm::Triple(_) => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::elements;

    const EX: &str = "PREFIX ex: <http://example.org/ns#>\n";

    fn reads(query: &str) -> bool {
        elements(&format!("{EX}{query}")).is_some()
    }

    #[test]
    fn a_conjunctive_construct_is_read_as_elements() {
        assert!(reads(
            "CONSTRUCT { ?s ex:n ?m } WHERE { ?s ex:n ?n FILTER (?n > 0) BIND (?n - 1 AS ?m) }"
        ));
        assert!(reads(
            "CONSTRUCT { ?x ex:grand ?z } WHERE { ?x ex:parent ?y . ?y ex:parent ?z }"
        ));
        assert!(reads(
            "CONSTRUCT { ?y ?p ?x } WHERE { ?x ?p ?y . ?y a ex:Node . ?p a ex:Symmetric }"
        ));
        // A predicate variable bound only in subject position may be a blank node,
        // which CONSTRUCT skips and a rule head refuses: declined.
        assert!(!reads(
            "CONSTRUCT { ?x ?p ?y } WHERE { ?p ex:inverse ?q . ?y ?q ?x }"
        ));
        assert!(reads(
            "CONSTRUCT { ?x a ex:Adult } WHERE { ?x ex:age ?a FILTER (xsd:integer(?a) >= 18) }"
                .replace("xsd:integer", "<http://www.w3.org/2001/XMLSchema#integer>")
                .as_str()
        ));
    }

    /// Each pattern whose element reading would not be the query stays a producer.
    #[test]
    fn a_query_outside_the_fragment_is_not_read() {
        for query in [
            // Solutions that do not only grow.
            "CONSTRUCT { ?s ex:q ?o } WHERE { ?s ex:p ?o OPTIONAL { ?s ex:r ?z } }",
            "CONSTRUCT { ?s ex:q ?o } WHERE { { ?s ex:p ?o } UNION { ?s ex:r ?o } }",
            "CONSTRUCT { ?s ex:q ?o } WHERE { ?s ex:p ?o MINUS { ?s ex:r ?o } }",
            "CONSTRUCT { ?s ex:q ?o } WHERE { ?s ex:p ?o FILTER NOT EXISTS { ?s ex:r ?o } }",
            "CONSTRUCT { ?s ex:q ?o } WHERE { ?s ex:p ?o FILTER EXISTS { ?s ex:r ?o } }",
            "CONSTRUCT { ?s ex:q ?o } WHERE { VALUES ?o { 1 } ?s ex:p ?o }",
            "CONSTRUCT { ?s ex:q ?o } WHERE { ?s ex:p+ ?o }",
            "CONSTRUCT { ?s ex:q ?o } WHERE { GRAPH ex:g { ?s ex:p ?o } }",
            "CONSTRUCT { ?s ex:q ?o } FROM ex:g WHERE { ?s ex:p ?o }",
            // Minting or clock-reading expressions, and extension functions.
            "CONSTRUCT { ?s ex:q ?b } WHERE { ?s ex:p ?o BIND (BNODE() AS ?b) }",
            "CONSTRUCT { ?s ex:q ?t } WHERE { ?s ex:p ?o BIND (NOW() AS ?t) }",
            "CONSTRUCT { ?s ex:q ?t } WHERE { ?s ex:p ?o BIND (ex:fn(?o) AS ?t) }",
            // A template blank node is fresh per execution.
            "CONSTRUCT { ?s ex:q [ ex:r ?o ] } WHERE { ?s ex:p ?o }",
            // A BIND variable missing from a template triple, and one in a later pattern.
            "CONSTRUCT { ?s ex:q ?m . ?s ex:r ?o } WHERE { ?s ex:p ?o BIND (?o + 1 AS ?m) }",
            "CONSTRUCT { ?s ex:q ?m } WHERE { { ?s ex:p ?o BIND (?o + 1 AS ?m) } ?m ex:r ?s }",
            // A template subject or predicate no pattern binds as one.
            "CONSTRUCT { ?o ex:q ?s } WHERE { ?s ex:p ?o }",
            "CONSTRUCT { ?s ?o ?s } WHERE { ?s ex:p ?o }",
            // A FILTER reading a variable its own group does not bind.
            "CONSTRUCT { ?s ex:q ?o } WHERE { ?s ex:p ?o { ?s ex:r ?z FILTER (?o > 1) } }",
        ] {
            assert!(!reads(query), "{query}");
        }
    }
}
