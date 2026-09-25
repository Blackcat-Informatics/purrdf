// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Evaluating ONE node expression a shapes graph carries, outside any validation — the
//! SHACL 1.2 Node Expressions `evalExpr(expr, focusGraph, focusNode, scope)` as a
//! standalone call.
//!
//! The expression is a node of the shapes graph (an IRI, or a blank node the document
//! labels), parsed by the shapes parser itself
//! ([`crate::shapes::from_dataset_with_node_expressions`]) so its custom-function calls,
//! shape references and SPARQL prefixes bind exactly as they would inside a shape. The
//! data graph is the focus graph, exposed to SPARQL-based expressions through the same
//! projection validation uses, with the shapes graph as a named graph when the shapes
//! graph declares `sh:shapesGraph`. The shapes graph's declared SHACL-AF functions and
//! aggregates are in scope for the whole call.

use std::sync::Arc;

use ::purrdf::RdfDataset;

use crate::data::{GraphFilter, native_quads};
use crate::expression::{Binding, RecursionGuard, Scope, eval_node_expr_in_scope};
use crate::term::Term;

/// One standalone node-expression evaluation. See the [module docs](self).
#[derive(Debug, Clone, Copy)]
pub struct FreeExpression<'a> {
    /// The shapes graph that carries the expression.
    pub shapes: &'a Arc<RdfDataset>,
    /// The shapes document's own prefix map, the fallback prefix environment of its
    /// SPARQL-based expressions.
    pub prefixes: &'a [(String, String)],
    /// The node of `shapes` that IS the expression: an IRI, or a blank node the shapes
    /// document labels. An IRI that is the subject of no triple is a constant expression
    /// evaluating to itself, as SHACL 1.2 Node Expressions defines one.
    pub root: &'a Term,
    /// The focus graph.
    pub data: &'a RdfDataset,
    /// The focus node.
    pub focus: &'a Term,
    /// The scope's variable bindings, `(name, node)`, each readable through
    /// `shnex:var "name"`.
    pub scope: &'a [(String, Term)],
}

/// Evaluate `request`: the output nodes of its expression, in the order the expression's
/// sequence semantics define.
///
/// # Errors
///
/// A blank-node root the shapes graph never mentions (a mistyped label, which would
/// otherwise evaluate as the empty expression); a scope binding named `focusNode`, which
/// SHACL 1.2 Node Expressions §4.1.2 resolves against the focus node before the scope is
/// searched, so the binding could never be read; two bindings of one name, of which only
/// one could ever be read; anything the shapes parser refuses; and any evaluation error.
pub fn evaluate(request: &FreeExpression<'_>) -> Result<Vec<Term>, String> {
    let mut names: Vec<&str> = Vec::with_capacity(request.scope.len());
    for (name, _) in request.scope {
        if name == "focusNode" {
            return Err(
                "scope variable \"focusNode\" can never be read: SHACL 1.2 Node Expressions \
                 §4.1.2 resolves shnex:var \"focusNode\" to the focus node before the scope \
                 is searched. Pass the node as the focus node instead"
                    .to_owned(),
            );
        }
        if names.contains(&name.as_str()) {
            return Err(format!(
                "scope variable \"{name}\" is bound twice; only one binding could ever be \
                 read, so bind each name once"
            ));
        }
        names.push(name);
    }
    if let Term::BlankNode(label) = request.root {
        let mentioned = !native_quads(
            request.shapes.as_ref(),
            Some(request.root),
            None,
            None,
            GraphFilter::AnyGraph,
        )
        .is_empty()
            || !native_quads(
                request.shapes.as_ref(),
                None,
                None,
                Some(request.root),
                GraphFilter::AnyGraph,
            )
            .is_empty();
        if !mentioned {
            return Err(format!(
                "the shapes graph mentions no blank node _:{label}, so there is no \
                 expression to evaluate; name the expression node by the label the shapes \
                 document gives it, or by its IRI"
            ));
        }
    }
    let (shapes, mut exprs) = crate::shapes::from_dataset_with_node_expressions(
        request.shapes,
        request.prefixes,
        None,
        std::slice::from_ref(request.root),
    )?;
    let expr = exprs
        .pop()
        .ok_or("the shapes parser returned no expression for the one root it was given")?;
    let projected = crate::engine::project_dataset(request.data)?;
    let data = crate::engine::build_projected_data(projected, &shapes, None)?;
    let _function_scope =
        crate::sparql::enter_function_scope(crate::sparql::bind_in_current_env(&shapes.functions)?);
    let _aggregate_scope = crate::sparql::enter_aggregate_scope(Arc::clone(&shapes.aggregates));
    let mut guard = RecursionGuard::new();
    eval_bound(
        &data,
        request.focus,
        &expr,
        &mut guard,
        request.scope,
        Scope::EMPTY,
    )
}

/// Push `bindings` onto `scope`, first binding outermost, and evaluate.
fn eval_bound(
    data: &crate::data::ShaclData,
    focus: &Term,
    expr: &crate::expression::NodeExpr,
    guard: &mut RecursionGuard,
    bindings: &[(String, Term)],
    scope: Scope<'_>,
) -> Result<Vec<Term>, String> {
    match bindings.split_first() {
        None => eval_node_expr_in_scope(data, focus, expr, guard, scope),
        Some(((name, value), rest)) => {
            let binding = Binding::new(name, value, scope);
            eval_bound(data, focus, expr, guard, rest, Scope::bound(&binding))
        }
    }
}

/// Read one RDF term written on a command line or passed across a host boundary:
/// N-Triples 1.2 term syntax (`<iri>`, `"lexical"`, `"lexical"@lang`,
/// `"lexical"^^<datatype>`, `_:label`, `<<( s p o )>>`), or a bare absolute IRI.
///
/// A blank node keeps its label, so `_:b` names the node a document labels `_:b`.
///
/// # Errors
///
/// Text that is neither: not an absolute IRI, and not one well-formed N-Triples term.
pub fn parse_term(text: &str) -> Result<Term, String> {
    let trimmed = text.trim();
    let spelled =
        if trimmed.starts_with('<') || trimmed.starts_with('"') || trimmed.starts_with("_:") {
            trimmed.to_owned()
        } else {
            let absolute = purrdf_iri::is_absolute(trimmed).map_err(|e| {
                format!("`{text}` is neither an N-Triples term nor an absolute IRI: {e}")
            })?;
            if !absolute {
                return Err(format!(
                    "`{text}` is a relative IRI reference; write the absolute IRI, or an \
                 N-Triples term"
                ));
            }
            format!("<{trimmed}>")
        };
    // One N-Triples statement carrying the term as its object: the codec's own grammar,
    // not a second one. `rdf:value` is RDF's own predicate for "the value of".
    let document =
        format!("_:purrdf-term <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> {spelled} .\n");
    let dataset = crate::text_ingest::parse_ntriples_to_dataset(&document)
        .map_err(|errors| format!("`{text}` is not one N-Triples term: {}", errors.join("; ")))?;
    let quads = native_quads(dataset.as_ref(), None, None, None, GraphFilter::AnyGraph);
    match quads.as_slice() {
        [(_, _, object)] => Ok(object.clone()),
        _ => Err(format!("`{text}` is not one N-Triples term")),
    }
}
