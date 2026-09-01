// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parsing for SHACL-AF `sh:SPARQLFunction` declarations.

use ::purrdf::FastSet;
use std::sync::Arc;

use ::purrdf::TermValue;
use purrdf_sparql_algebra::{Query, SparqlParser};
use purrdf_sparql_eval::{
    Arity, EvalError, ExprFnCall, NodeKind as EvalNodeKind, TypeConstraint, UserFnBody,
    UserFnParam, UserFunction, UserFunctionRegistry,
};

use crate::data::ShaclData;
use crate::expression::{CustomFnKind, CustomFunction, RecursionGuard, eval_custom_function_call};
use crate::model::{rdf, sh};
use crate::sparql::enter_call_depth_scope;
use crate::term::{Term, term_value_to_native};

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
            .map(|(subject, _, _)| subject)
            .collect();
        crate::term::sort_terms_canonical(&mut fn_ids);
        fn_ids.dedup();

        let mut registry = UserFunctionRegistry::new();
        for id in fn_ids {
            // Only IRI-named functions are callable (the call site is an IRI).
            let Term::NamedNode(iri) = &id else {
                continue;
            };
            // A node typed both `sh:SPARQLFunction` and one of the custom
            // node-expression classes has already been parsed as the latter, body and
            // all; registering it twice would be two different functions under one
            // IRI. The declaring class it carries decides, once.
            if self.custom_fns.get(iri.as_str()).is_some() {
                continue;
            }
            let func = self.parse_one_sparql_function(&id)?;
            registry.insert(iri.as_str().to_owned(), func);
        }
        self.register_expression_bodied_functions(&mut registry)?;
        Ok(registry)
    }

    /// Register every custom LIST parameter function as a callable SPARQL function,
    /// into the SAME [`UserFunctionRegistry`] the `sh:select`/`sh:ask` bodies go
    /// into.
    ///
    /// SHACL 1.2 SPARQL Extensions §7.3: "SPARQL engines SHOULD register a function
    /// for any SHACL instance of `sh:ListParameterExpressionFunction` from any
    /// provided shapes graph." Only that class is registered — a
    /// `sh:NamedParameterExpressionFunction` keys its arguments by parameter IRI and
    /// has no positional call form, so §7.3 does not name it and there is no
    /// well-defined `ex:f(?x, ?y)` for it to answer.
    ///
    /// Each registration is a dataset-aware closure
    /// ([`purrdf_sparql_eval::ExprFnBody`]): the body is a node expression, so it is
    /// evaluated over the graph the CALLING QUERY supplies rather than over anything
    /// captured here. That is what makes a call inside a rules fixpoint read the
    /// current round's facts.
    ///
    /// # Errors
    ///
    /// Hard-fails when a declaration's body was never installed — an internal
    /// inconsistency that would otherwise register a function which silently answers
    /// nothing.
    fn register_expression_bodied_functions(
        &self,
        registry: &mut UserFunctionRegistry,
    ) -> Result<(), String> {
        for func in self.custom_fns.iter() {
            if !matches!(func.kind, CustomFnKind::ListParameter) {
                continue;
            }
            // Registering a function whose body never arrived would put a call site
            // in reach of a function that answers nothing; refuse at load instead.
            func.body()?;
            let arity = if func.required == func.params.len() {
                Arity::Exact(func.required)
            } else {
                Arity::Range {
                    min: func.required,
                    max: func.params.len(),
                }
            };
            let declared = Arc::clone(func);
            let iri = declared.iri.as_str().to_owned();
            registry.register_expr(
                iri,
                arity,
                Arc::new(move |call: &ExprFnCall<'_>| invoke_expression_function(&declared, call)),
            );
        }
        Ok(())
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
        let mut seen: FastSet<&str> = FastSet::default();
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

        // ── Body: exactly one of sh:select / sh:ask / sh:bodyExpression ───────
        //
        // `sh:bodyExpression` is the third body form: a NODE EXPRESSION rather than
        // query text (SHACL 1.2 SPARQL Extensions §7; SHACL 1.2 Node Expressions
        // §6.1/§6.2). It is not parsed here — a node-expression body belongs to the
        // declaring class that carries it, and `Parser::discover_custom_functions`
        // has already interned that declaration and
        // `install_custom_function_bodies` has already parsed the body — so reaching
        // this point with one means the node declared an expression body WITHOUT one
        // of the two classes that give it meaning. That is a body nothing would ever
        // evaluate, so it is refused rather than loaded green.
        let select = self.first_string_object(id, sh::SELECT);
        let ask = self.first_string_object(id, sh::ASK);
        let body_expression = self.first_object_of(id, sh::BODY_EXPRESSION);
        if let Some(body) = &body_expression {
            if select.is_some() || ask.is_some() {
                return Err(format!(
                    "sh:SPARQLFunction <{id}> declares a sh:bodyExpression alongside a \
                     sh:select/sh:ask body; exactly one body is required"
                ));
            }
            return Err(format!(
                "<{id}> declares the sh:bodyExpression {body} but is not typed \
                 sh:ListParameterExpressionFunction or sh:NamedParameterExpressionFunction, so \
                 nothing would ever evaluate that body"
            ));
        }
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
                    "sh:SPARQLFunction <{id}> is missing its sh:select/sh:ask/sh:bodyExpression body"
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

/// Evaluate one SPARQL call of a custom LIST parameter function — the body of the
/// closure `register_expression_bodied_functions` installs.
///
/// SHACL 1.2 SPARQL Extensions §7.3 in three moves:
///
/// 1. The already-evaluated arguments become the argument scope, keyed by INDEX.
///    An unbound argument in a required position leaves the call with no value
///    (`Ok(None)` — SPARQL's own expression-error result), which is what §7.3's
///    "otherwise the argument remains unbound" reduces to at a call boundary.
/// 2. The body is evaluated over `call.focus_graph`, the graph the calling query is
///    reading, with the FUNCTION'S OWN IRI as focus node — §7.3: "there is no
///    dedicated focus node. Instead, the `focusNode` passed into a custom SPARQL
///    function based on a node expression is the IRI of the function itself."
/// 3. Exactly one output node is returned; none is no value; more than one is a hard
///    error, because the specification returns a node only in the one-member case
///    and picking one would be inventing an answer.
///
/// The recursion guard is SEEDED from `call.depth`, so a cycle that passed through
/// SPARQL to get here keeps counting rather than restarting.
fn invoke_expression_function(
    func: &Arc<CustomFunction>,
    call: &ExprFnCall<'_>,
) -> Result<Option<TermValue>, EvalError> {
    let mut args: Vec<Term> = Vec::with_capacity(call.args.len());
    for (index, value) in call.args.iter().enumerate() {
        match value {
            Some(bound) => args.push(term_value_to_native(bound)),
            // A trailing unbound OPTIONAL argument is simply not supplied; an unbound
            // REQUIRED one leaves the call with no value at all.
            None if index >= func.required => break,
            None => return Ok(None),
        }
    }
    // A fresh view over the calling query's own graph. Both halves are the same
    // frozen dataset, so the node expression reads exactly the graph the query is
    // reading — the CURRENT one, never a capture from shapes-load.
    let store = ShaclData::new(
        Arc::clone(call.focus_graph),
        Arc::clone(call.focus_graph),
        None,
    );
    let mut guard = RecursionGuard::with_depth(call.depth);
    let _depth = enter_call_depth_scope(call.depth);
    let result = eval_custom_function_call(&store, func, &args, &mut guard)
        .map_err(|e| EvalError::function(format!("custom SPARQL function: {e}")))?;
    Ok(result.as_ref().map(Term::to_term_value))
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
