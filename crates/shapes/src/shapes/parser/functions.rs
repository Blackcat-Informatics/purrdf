// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parsing for SHACL-AF `sh:SPARQLFunction` declarations.

use std::collections::HashSet;
use std::sync::Arc;

use purrdf_sparql_algebra::{Query, SparqlParser};
use purrdf_sparql_eval::{
    NodeKind as EvalNodeKind, TypeConstraint, UserFnBody, UserFnParam, UserFunction,
    UserFunctionRegistry,
};

use crate::model::{rdf, sh};
use crate::term::Term;

use crate::shapes::Parser;

impl Parser<'_> {
    /// Parse every `sh:SPARQLFunction` (or `sh:Function`) declaration in the shapes
    /// graph into a [`UserFunctionRegistry`]: ordered `sh:parameter`s (pre-bound
    /// variable = the parameter predicate's local name), the required-arity count,
    /// the `sh:select`/`sh:ask` body, and the `sh:returnType` constraint.
    ///
    /// # Errors
    ///
    /// Hard-fails on a malformed declaration — a parameter without a predicate,
    /// two parameters whose derived variable names collide, a missing/ambiguous
    /// body, or an unparsable body query.
    pub(crate) fn parse_sparql_functions(&self) -> Result<UserFunctionRegistry, String> {
        let mut fn_ids: Vec<Term> = self
            .quads_with(None, Some(rdf::TYPE), Some(sh::SPARQL_FUNCTION))
            .into_iter()
            .chain(self.quads_with(None, Some(rdf::TYPE), Some(sh::FUNCTION)))
            .map(|q| q.subject)
            .collect();
        fn_ids.sort_by_key(ToString::to_string);
        fn_ids.dedup();

        let mut registry = UserFunctionRegistry::new();
        for id in fn_ids {
            // Only IRI-named functions are callable (the call site is an IRI).
            let Term::NamedNode(iri) = &id else {
                continue;
            };
            let func = self.parse_one_sparql_function(&id)?;
            registry.insert(iri.as_str().to_owned(), func);
        }
        Ok(registry)
    }

    /// Parse a single `sh:SPARQLFunction` declaration node into a [`UserFunction`].
    fn parse_one_sparql_function(&self, id: &Term) -> Result<UserFunction, String> {
        // ── Parameters, ordered by (sh:order, predicate IRI) ──────────────────
        struct RawParam {
            order: f64,
            predicate: String,
            var: String,
            optional: bool,
            constraint: TypeConstraint,
        }
        let mut raw: Vec<RawParam> = Vec::new();
        for p_node in self.objects_of(id, sh::PARAMETER_PROPERTY) {
            // The parameter predicate: sh:path (a predicate IRI) or sh:predicate.
            let predicate = self
                .first_object_of(&p_node, sh::PATH)
                .or_else(|| self.first_object_of(&p_node, sh::PREDICATE))
                .and_then(|t| match t {
                    Term::NamedNode(n) => Some(n.as_str().to_owned()),
                    _ => None,
                })
                .ok_or_else(|| {
                    format!("sh:SPARQLFunction <{id}> has a sh:parameter without an IRI sh:path/sh:predicate")
                })?;
            let var = crate::shapes::local_name(&predicate).to_owned();
            if var.is_empty() {
                return Err(format!(
                    "sh:SPARQLFunction <{id}> has a sh:parameter whose predicate <{predicate}> has an empty local name and yields no usable variable"
                ));
            }
            // A parameter must not shadow a SHACL/SHACL-AF pre-bound or reserved
            // variable (SHACL §3.2.1, SHACL-AF §5.2) — e.g. `this` would clobber the
            // injected focus-node binding during evaluation.
            const RESERVED_VARS: [&str; 6] = [
                "this",
                "path",
                "PATH",
                "value",
                "shapesGraph",
                "currentShape",
            ];
            if RESERVED_VARS.contains(&var.as_str()) {
                return Err(format!(
                    "sh:SPARQLFunction <{id}> parameter variable ?{var} is a SHACL/SHACL-AF reserved name"
                ));
            }
            let order = match self.first_object_of(&p_node, sh::ORDER) {
                None => f64::INFINITY,
                Some(Term::Literal(lit)) => lit.value().parse::<f64>().map_err(|_| {
                    format!(
                        "sh:SPARQLFunction <{id}> parameter ?{var} has a non-numeric sh:order '{}'",
                        lit.value()
                    )
                })?,
                Some(other) => {
                    return Err(format!(
                        "sh:SPARQLFunction <{id}> parameter ?{var} has a non-literal sh:order {other}"
                    ));
                }
            };
            let optional = match self.first_object_of(&p_node, sh::OPTIONAL) {
                None => false,
                Some(Term::Literal(lit)) => match lit.value() {
                    "true" | "1" => true,
                    "false" | "0" => false,
                    other => {
                        return Err(format!(
                            "sh:SPARQLFunction <{id}> parameter ?{var} has a non-boolean sh:optional '{other}'"
                        ));
                    }
                },
                Some(other) => {
                    return Err(format!(
                        "sh:SPARQLFunction <{id}> parameter ?{var} has a non-literal sh:optional {other}"
                    ));
                }
            };
            let constraint = self.type_constraint_of(&p_node);
            raw.push(RawParam {
                order,
                predicate,
                var,
                optional,
                constraint,
            });
        }
        // Deterministic order: ascending sh:order, IRI as tiebreak (unspecified
        // orders — INFINITY — sort last, still by IRI).
        raw.sort_by(|a, b| {
            a.order
                .partial_cmp(&b.order)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.predicate.cmp(&b.predicate))
        });

        // Reject colliding derived variable names — silent shadowing would bind the
        // wrong argument.
        let mut seen: HashSet<&str> = HashSet::new();
        for p in &raw {
            if !seen.insert(p.var.as_str()) {
                return Err(format!(
                    "sh:SPARQLFunction <{id}> has two parameters whose variable name ?{} collides",
                    p.var
                ));
            }
        }

        // A required parameter after an optional one is ill-formed (arity would be
        // ambiguous). Enforce the "optionals are trailing" rule.
        let mut seen_optional = false;
        for p in &raw {
            if p.optional {
                seen_optional = true;
            } else if seen_optional {
                return Err(format!(
                    "sh:SPARQLFunction <{id}> declares a required parameter ?{} after an optional one",
                    p.var
                ));
            }
        }
        let required = raw.iter().filter(|p| !p.optional).count();
        let params: Vec<UserFnParam> = raw
            .into_iter()
            .map(|p| UserFnParam {
                var: p.var,
                constraint: p.constraint,
            })
            .collect();

        // ── Body: exactly one of sh:select / sh:ask ───────────────────────────
        let select = self.first_string_object(id, sh::SELECT);
        let ask = self.first_string_object(id, sh::ASK);
        let (raw_body, kind) = match (select, ask) {
            (Some(s), None) => (s, UserFnBody::Select),
            (None, Some(a)) => (a, UserFnBody::Ask),
            (Some(_), Some(_)) => {
                return Err(format!(
                    "sh:SPARQLFunction <{id}> declares both sh:select and sh:ask (exactly one is required)"
                ));
            }
            (None, None) => {
                return Err(format!(
                    "sh:SPARQLFunction <{id}> is missing its sh:select/sh:ask body"
                ));
            }
        };
        let body_text = format!("{}{raw_body}", self.prefix_header(&[id]));
        let query = SparqlParser::new()
            .parse_query(&body_text)
            .map_err(|e| format!("sh:SPARQLFunction <{id}> has an unparsable body query: {e}"))?;
        match (&query, kind) {
            (Query::Select { .. }, UserFnBody::Select) | (Query::Ask { .. }, UserFnBody::Ask) => {}
            _ => {
                return Err(format!(
                    "sh:SPARQLFunction <{id}> body form does not match its sh:select/sh:ask declaration"
                ));
            }
        }

        let return_constraint = TypeConstraint {
            datatype: self.first_iri_object(id, sh::RETURN_TYPE),
            node_kind: None,
        };

        Ok(UserFunction {
            params,
            required,
            body: Arc::new(query),
            kind,
            return_constraint,
        })
    }

    /// The `sh:datatype`/`sh:nodeKind` type constraint declared on a parameter node.
    fn type_constraint_of(&self, p_node: &Term) -> TypeConstraint {
        let datatype = self.first_iri_object(p_node, sh::DATATYPE);
        let node_kind = self
            .first_object_of(p_node, sh::NODE_KIND)
            .and_then(|t| match t {
                Term::NamedNode(n) => node_kind_from_iri(n.as_str()),
                _ => None,
            });
        TypeConstraint {
            datatype,
            node_kind,
        }
    }
}

/// Map a `sh:nodeKind` object IRI to the evaluator's [`EvalNodeKind`] for a
/// function parameter/return type constraint.
fn node_kind_from_iri(iri: &str) -> Option<EvalNodeKind> {
    match iri {
        sh::IRI => Some(EvalNodeKind::Iri),
        sh::BLANK_NODE => Some(EvalNodeKind::BlankNode),
        sh::LITERAL => Some(EvalNodeKind::Literal),
        sh::BLANK_NODE_OR_IRI => Some(EvalNodeKind::BlankNodeOrIri),
        sh::BLANK_NODE_OR_LITERAL => Some(EvalNodeKind::BlankNodeOrLiteral),
        sh::IRI_OR_LITERAL => Some(EvalNodeKind::IriOrLiteral),
        _ => None,
    }
}
