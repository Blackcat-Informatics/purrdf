// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Parsing for SHACL-AF `sh:SPARQLFunction` declarations.

use ::purrdf::FastSet;
use std::sync::Arc;

use ::purrdf::{RdfDataset, TermValue};
use purrdf_sparql_algebra::{Query, SparqlParser};
use purrdf_sparql_eval::{
    EvalError, ExprFnCall, NodeKind as EvalNodeKind, TypeConstraint, UserFnBody, UserFnParam,
    UserFunction, UserFunctionRegistry,
};

use crate::data::ShaclData;
use crate::expression::{CustomFunction, RecursionGuard, eval_custom_function_call};
use crate::model::{rdf, sh};
use crate::provenance::ParseProvenance;
use crate::sparql::enter_call_depth_scope;
use crate::term::{Term, term_value_to_native};

use crate::shapes::Parser;

/// Re-derive a shapes graph's `sh:SPARQLFunction` declarations from the shapes
/// DATASET, into `registry` — the restore-time twin of what
/// [`Parser::parse_sparql_functions`] does during a parse.
///
/// # Why a shapes graph's SPARQL functions need no carrier of their own
///
/// A `sh:SPARQLFunction` declaration is not an opaque host binding: it is an IRI, an
/// ordered `sh:parameter` list, a required-arity count, a `sh:select`/`sh:ask` body
/// and a `sh:returnType`, and every one of those is stated by the shapes graph
/// itself. Anything holding that graph can therefore rebuild the declaration with no
/// cooperation from the process that first parsed it — which is exactly what
/// [`purrdf_sparql_eval::user_fn::FnPopulation::Declared`] means by "rebuildable at
/// restore by re-parsing that graph".
///
/// A prepared shapes product carries that graph, authenticated, so the restore has
/// the input this needs. Reinstating the declarations here rather than transcribing
/// them into a second on-disk carrier is what keeps the restored function IDENTICAL
/// to the parsed one: there is one parser for these declarations, and a restore runs
/// it. The rejected alternative — a section of the product spelling each parameter,
/// body string and return type — is a second, silently divergent transcription of a
/// parse that already exists, and its first drift would be a restored function whose
/// arity or return constraint differed from the one the author wrote.
///
/// # The precedence rule is the parser's, unchanged
///
/// [`Parser::parse_sparql_functions`] SKIPS an IRI that the shapes graph also
/// declares as a custom node-expression function, because a node typed both has
/// already been parsed as the latter, body and all — registering it twice would put
/// two different functions under one IRI. That rule is a property of the graph's
/// content, so it is reproduced here by rediscovering the custom node-expression
/// declarations from the same dataset rather than by consulting whatever a caller
/// happens to have in hand. Rediscovery is what makes the skip set the parse's own.
///
/// # What the common case pays
///
/// Four pattern probes, each of which resolves its class IRI against the dataset's
/// term table and stops there when the graph never mentions it — two for the custom
/// node-expression classes, two for `sh:SPARQLFunction` and `sh:Function`. A shapes
/// graph that declares neither kind therefore walks no quads and builds no
/// declaration; what it does pay is one short-lived [`Parser`], whose construction
/// copies the document prefix map. That is the restore path's common case, and it is
/// bounded by the prefix map rather than by the graph.
///
/// # Errors
///
/// The parser's own message when a declaration in `dataset` is malformed. A dataset
/// that reaches here has already parsed once, so a failure means the dataset is not
/// the shapes graph it claims to be; refusing is the fail-closed answer.
pub(crate) fn register_declared_sparql_functions(
    dataset: &Arc<RdfDataset>,
    provenance: &ParseProvenance,
    registry: &mut UserFunctionRegistry,
) -> Result<(), String> {
    let mut parser = Parser::new(
        dataset.as_ref(),
        provenance.base().map(ToOwned::to_owned),
        provenance.doc_prefixes(),
        provenance.box_role_vocab().cloned(),
        Arc::clone(dataset),
        provenance.shapes_graph().map(ToOwned::to_owned),
    );
    parser.custom_fns = parser.discover_custom_functions()?;
    parser.parse_sparql_functions(registry)
}

impl Parser<'_> {
    /// Parse every `sh:SPARQLFunction` (or `sh:Function`) declaration in the shapes
    /// graph into `registry`: ordered `sh:parameter`s (pre-bound variable = the
    /// parameter predicate's local name), the required-arity count, the
    /// `sh:select`/`sh:ask` body, and the `sh:returnType` constraint.
    ///
    /// # Errors
    ///
    /// Hard-fails on a malformed declaration — a parameter without a predicate,
    /// two parameters whose derived variable names collide, a missing/ambiguous
    /// body, or an unparsable body query.
    ///
    /// What this adds to `registry` is INCOMPLETE on purpose: the custom
    /// node-expression functions SHACL 1.2 SPARQL Extensions §7.3 also asks for are
    /// added by [`crate::shapes::link::link_shapes`], which is the one place that
    /// can add them — registering a declaration is only sound once its body is
    /// installed, and installing bodies is that pass's own first step.
    ///
    /// It writes into a caller-supplied registry rather than returning a fresh one
    /// because the two callers start from different tables and must end at the same
    /// contents: a parse starts from an empty registry, and a prepared-product
    /// restore starts from the host's injected table (see
    /// [`register_declared_sparql_functions`]). Returning a registry would force the
    /// second caller to merge two of them, and a merge is a second place for the
    /// precedence between the kinds to be decided differently.
    pub(crate) fn parse_sparql_functions(
        &self,
        registry: &mut UserFunctionRegistry,
    ) -> Result<(), String> {
        let mut fn_ids: Vec<Term> = self
            .quads_with(None, Some(rdf::TYPE), Some(sh::SPARQL_FUNCTION))
            .into_iter()
            .chain(self.quads_with(None, Some(rdf::TYPE), Some(sh::FUNCTION)))
            .map(|(subject, _, _)| subject)
            .collect();
        crate::term::sort_terms_canonical(&mut fn_ids);
        fn_ids.dedup();

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
        // `parse_custom_function_bodies` has already parsed the body — so reaching
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
        // A GRAMMAR check, and deliberately only that.
        //
        // This parse runs under default options — no registered relation IRIs —
        // which is exactly why its result is thrown away. Whether a predicate IRI
        // in this body is a data edge or a call to a registered relation is decided
        // by the extension environment in force at VALIDATION time, and a shapes
        // graph is loaded once and validated many times under different ones. There
        // is no parse here that would be right for all of them.
        //
        // What is knowable here is whether the text is SPARQL at all and whether
        // its form matches the `sh:select`/`sh:ask` the author declared. Both are
        // properties of the text alone, both are author errors, and both are worth
        // failing at load rather than at first validation. So the text is kept and
        // the algebra is discarded — the same division
        // `purrdf-retrieval`'s `hoistable_clauses` documents: the blind parse is
        // about grammar, and the registry-aware parse stays the authority.
        let form = SparqlParser::new()
            .parse_query(&body_text)
            .map_err(|e| format!("sh:SPARQLFunction <{id}> has an unparsable body query: {e}"))?;
        match (&form, kind) {
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
            body: Arc::from(body_text),
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
/// closure [`crate::shapes::link::link_shapes`] installs.
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
pub(crate) fn invoke_expression_function(
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
