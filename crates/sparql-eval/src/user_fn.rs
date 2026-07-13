// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Dynamic, host-injected SHACL-AF SPARQL-based functions (`sh:SPARQLFunction`).
//!
//! A shapes graph may declare its own functions: an IRI typed `sh:SPARQLFunction`
//! with ordered `sh:parameter`s, an optional `sh:returnType`, and a `sh:select` or
//! `sh:ask` body. Those calls appear in constraint/target queries and in SHACL-AF
//! node expressions as an ordinary call-position IRI, which the parser lowers to
//! [`Function::Custom`](purrdf_sparql_algebra::Function::Custom) (it is under no
//! configured extension-function namespace, so it is not the closed `PurrdfFn`
//! set). The evaluator resolves that IRI against a caller-injected
//! [`UserFunctionRegistry`] at eval time — the open counterpart to the closed,
//! parse-time-resolved `PurrdfFn` dispatch.
//!
//! The registry is pure data (parsed bodies + parameter metadata); executing a
//! call binds the arguments to the parameter variables as a pre-binding rewrite
//! (the same `crate::substitute` path `$this` injection uses) and evaluates the
//! body in a recursion-bounded child context. This keeps SPARQL execution inside
//! the evaluator and the registry free of any engine coupling.

use std::sync::Arc;

use purrdf_core::{DatasetView, TermId, TermValue};
use purrdf_sparql_algebra::Query;

use crate::DetHashMap;
use crate::error::EvalError;
use crate::eval::{EvalCtx, Outcome, evaluate_query, materialize_solutions};

/// The result form of a function body: a `sh:select` returns the first projected
/// value of the first solution; a `sh:ask` returns an `xsd:boolean`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserFnBody {
    /// A `sh:select` body: the return value is the first projected variable of the
    /// first solution row (empty result ⇒ no value).
    Select,
    /// A `sh:ask` body: the return value is the `xsd:boolean` of the ASK.
    Ask,
}

/// The `sh:nodeKind` of a parameter or return value, when constrained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    /// `sh:IRI`.
    Iri,
    /// `sh:BlankNode`.
    BlankNode,
    /// `sh:Literal`.
    Literal,
    /// `sh:BlankNodeOrIRI`.
    BlankNodeOrIri,
    /// `sh:BlankNodeOrLiteral`.
    BlankNodeOrLiteral,
    /// `sh:IRIOrLiteral`.
    IriOrLiteral,
}

/// The optional `sh:datatype`/`sh:nodeKind` type constraint on a parameter or the
/// return value. An empty constraint (`None`/`None`) accepts any term.
#[derive(Debug, Clone, Default)]
pub struct TypeConstraint {
    /// The required literal datatype IRI (`sh:datatype`), if any.
    pub datatype: Option<String>,
    /// The required node kind (`sh:nodeKind`), if any.
    pub node_kind: Option<NodeKind>,
}

impl TypeConstraint {
    /// Whether this constraint imposes any requirement.
    fn is_any(&self) -> bool {
        self.datatype.is_none() && self.node_kind.is_none()
    }

    /// Validate `value` against this constraint. `role` names the position
    /// (`parameter ?var` / `return value`) for the error message.
    fn check(&self, iri: &str, role: &str, value: &TermValue) -> Result<(), EvalError> {
        if self.is_any() {
            return Ok(());
        }
        if let Some(nk) = self.node_kind
            && !matches_node_kind(value, nk)
        {
            return Err(EvalError::function(format!(
                "SHACL-AF function <{iri}> {role} violates its sh:nodeKind constraint"
            )));
        }
        if let Some(dt) = &self.datatype {
            let ok = matches!(value, TermValue::Literal { datatype, .. } if datatype == dt);
            if !ok {
                return Err(EvalError::function(format!(
                    "SHACL-AF function <{iri}> {role} is not a literal of datatype <{dt}>"
                )));
            }
        }
        Ok(())
    }
}

/// A parameter of a [`UserFunction`]: the pre-bound variable name plus its type
/// constraint. Parameters are stored in call order (ascending `sh:order`, IRI as a
/// deterministic tiebreak).
#[derive(Debug, Clone)]
pub struct UserFnParam {
    /// The pre-bound SPARQL variable name (the local name of the parameter's
    /// `sh:path`/`sh:predicate` predicate).
    pub var: String,
    /// The parameter's `sh:datatype`/`sh:nodeKind` constraint.
    pub constraint: TypeConstraint,
}

/// A declared SHACL-AF SPARQL-based function: its ordered parameters, the count of
/// leading required (non-`sh:optional`) parameters, the parsed body, and the
/// return-value constraint.
#[derive(Debug, Clone)]
pub struct UserFunction {
    /// The parameters in call order.
    pub params: Vec<UserFnParam>,
    /// The number of leading required parameters (arity is `[required, params.len()]`).
    pub required: usize,
    /// The parsed `sh:select`/`sh:ask` body.
    pub body: Arc<Query>,
    /// Whether the body is a SELECT or an ASK.
    pub kind: UserFnBody,
    /// The `sh:returnType` constraint on the produced value, if declared.
    pub return_constraint: TypeConstraint,
}

/// A caller-injected table of SHACL-AF functions, keyed by function IRI. Built once
/// per shapes graph by the shapes crate and borrowed into evaluation via
/// [`NativeSparqlEngine::query_with_user_functions`](crate::NativeSparqlEngine::query_with_user_functions).
#[derive(Debug, Default, Clone)]
pub struct UserFunctionRegistry {
    fns: DetHashMap<String, UserFunction>,
}

impl UserFunctionRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `func` under its `iri`. A later registration of the same IRI
    /// replaces the earlier one.
    pub fn insert(&mut self, iri: impl Into<String>, func: UserFunction) {
        self.fns.insert(iri.into(), func);
    }

    /// Resolve a call-position IRI to its declared function, if any.
    #[must_use]
    pub fn resolve(&self, iri: &str) -> Option<&UserFunction> {
        self.fns.get(iri)
    }

    /// Whether the registry holds no functions (the common case: no
    /// `sh:SPARQLFunction` declared, so evaluation carries no registry at all).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fns.is_empty()
    }

    /// The number of declared functions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fns.len()
    }
}

/// Whether `value`'s node kind satisfies `nk`.
fn matches_node_kind(value: &TermValue, nk: NodeKind) -> bool {
    let (is_iri, is_blank, is_literal) = match value {
        TermValue::Iri(_) => (true, false, false),
        TermValue::Blank { .. } => (false, true, false),
        TermValue::Literal { .. } => (false, false, true),
        // A triple term is none of the three simple kinds.
        TermValue::Triple { .. } => (false, false, false),
    };
    match nk {
        NodeKind::Iri => is_iri,
        NodeKind::BlankNode => is_blank,
        NodeKind::Literal => is_literal,
        NodeKind::BlankNodeOrIri => is_blank || is_iri,
        NodeKind::BlankNodeOrLiteral => is_blank || is_literal,
        NodeKind::IriOrLiteral => is_iri || is_literal,
    }
}

/// Execute a resolved SHACL-AF function call: arity- and type-check the arguments,
/// bind them to the parameter variables, evaluate the body in a recursion-bounded
/// child context, and extract the single return value (`Ok(None)` = no value).
///
/// `args` are the already-evaluated argument values in call order (a `None` cell is
/// an unbound argument, which leaves that parameter variable unbound). The result is
/// a dataset-independent [`TermValue`]; the caller interns it into the parent
/// context.
///
/// # Errors
///
/// [`EvalError::Function`] on an arity or type-constraint violation or on exceeding
/// the user-function recursion bound; propagates body evaluation errors.
pub(crate) fn eval_user_function<D: DatasetView<Id = TermId> + Sync>(
    func: &UserFunction,
    iri: &str,
    args: &[Option<TermValue>],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<TermValue>, EvalError> {
    if args.len() < func.required || args.len() > func.params.len() {
        return Err(EvalError::function(format!(
            "SHACL-AF function <{iri}> expects {}..={} argument(s), got {}",
            func.required,
            func.params.len(),
            args.len()
        )));
    }

    // Bind each supplied argument to its parameter variable, type-checking as we go.
    // A mandatory parameter with an unbound (`None`) argument yields no result node
    // (SHACL-AF §5.2/§9.5): the function is not evaluated at all. An unbound OPTIONAL
    // argument simply leaves that parameter variable unbound (pre-binding semantics).
    let mut substitutions: Vec<(String, TermValue)> = Vec::with_capacity(args.len());
    for (idx, (arg, param)) in args.iter().zip(&func.params).enumerate() {
        match arg {
            Some(value) => {
                param
                    .constraint
                    .check(iri, &format!("parameter ?{}", param.var), value)?;
                substitutions.push((param.var.clone(), value.clone()));
            }
            None if idx < func.required => return Ok(None),
            None => {}
        }
    }

    // Recursion-bounded child context (guards mutually-recursive functions).
    let mut child = ctx.child_for_user_fn()?;
    let substituted = crate::substitute::apply_substitutions((*func.body).clone(), &substitutions)
        .map_err(|d| EvalError::function(d.to_string()))?;
    let outcome = evaluate_query(&substituted, &mut child)?;

    let result: Option<TermValue> = match (func.kind, outcome) {
        (UserFnBody::Ask, Outcome::Boolean(value)) => Some(TermValue::typed_literal(
            if value { "true" } else { "false" },
            "http://www.w3.org/2001/XMLSchema#boolean",
        )),
        (UserFnBody::Select, Outcome::Solutions(seq)) => {
            let (variables, rows) = materialize_solutions(&seq, &child);
            // A SHACL-AF function SELECT body yields a single result variable; a
            // multi-projection body has no well-defined return value.
            if variables.len() != 1 {
                return Err(EvalError::function(format!(
                    "SHACL-AF function <{iri}> SELECT body must project exactly one variable, got {}",
                    variables.len()
                )));
            }
            // The single projected value of the first solution row; an empty
            // result set is "no value".
            rows.into_iter()
                .next()
                .and_then(|row| row.into_iter().next().flatten())
        }
        // The declaration parser pairs `kind` with the matching body form, so a
        // mismatch is an internal invariant violation, not user input.
        (kind, outcome) => {
            return Err(EvalError::internal(format!(
                "SHACL-AF function <{iri}> body kind {kind:?} produced {outcome:?}"
            )));
        }
    };

    // Merge the child's minted identity / entropy / constructed state back into
    // the parent so it survives the return boundary: body-minted blanks stay
    // globally unique across calls, RAND()/UUID()/STRUUID() advance the stream
    // rather than replay it, and rdf:List quads constructed by listSlice/
    // listConcat remain reachable in the enclosing query's results.
    ctx.bnode_counter = child.bnode_counter;
    ctx.rng_state = child.rng_state;
    ctx.constructed.append(&mut child.constructed);

    // `sh:returnType` is informational (SHACL-AF §5.3): it documents/casts the
    // return and MAY be a class IRI, not a literal datatype. Enforcing it as a
    // runtime datatype constraint would spuriously reject IRI/blank-node returns,
    // so it is retained on `UserFunction` for callers but NOT enforced here.
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};
    use purrdf_sparql_algebra::SparqlParser;

    use crate::NativeSparqlEngine;

    const EX_INC: &str = "http://example.org/ns#inc";
    const EX_EVEN: &str = "http://example.org/ns#isEven";
    const EX_LOOP: &str = "http://example.org/ns#loop";

    fn empty_dataset() -> Arc<RdfDataset> {
        RdfDatasetBuilder::new().freeze().expect("freeze")
    }

    fn parse(body: &str) -> Arc<Query> {
        Arc::new(
            SparqlParser::new()
                .parse_query(body)
                .expect("parse function body"),
        )
    }

    fn int_param(var: &str) -> UserFnParam {
        UserFnParam {
            var: var.to_owned(),
            constraint: TypeConstraint::default(),
        }
    }

    /// A SELECT-bodied function `inc(?n) = ?n + 1` returns the projected value.
    #[test]
    fn select_body_returns_projected_value() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_INC,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse("SELECT ((?n + 1) AS ?result) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_INC}>(41)) AS ?v) WHERE {{}}");
        let result = NativeSparqlEngine::new()
            .query_with_user_functions(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                &registry,
            )
            .expect("query");
        match result {
            SparqlResult::Solutions { rows, .. } => {
                let cell = rows[0][0].as_ref().expect("bound result");
                assert!(
                    matches!(cell, TermValue::Literal { lexical_form, .. } if lexical_form == "42"),
                    "expected 42, got {cell:?}"
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// An ASK-bodied function returns an `xsd:boolean`.
    #[test]
    fn ask_body_returns_boolean() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_EVEN,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse("ASK { FILTER(?n / 2 = FLOOR(?n / 2)) }"),
                kind: UserFnBody::Ask,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        let run = |arg: i32| -> String {
            let query = format!("SELECT ((<{EX_EVEN}>({arg})) AS ?v) WHERE {{}}");
            match NativeSparqlEngine::new()
                .query_with_user_functions(
                    &ds,
                    SparqlRequest {
                        query: &query,
                        base_iri: None,
                        substitutions: &[],
                    },
                    &registry,
                )
                .expect("query")
            {
                SparqlResult::Solutions { rows, .. } => match rows[0][0].as_ref().expect("bound") {
                    TermValue::Literal { lexical_form, .. } => lexical_form.clone(),
                    other => panic!("expected literal, got {other:?}"),
                },
                other => panic!("expected solutions, got {other:?}"),
            }
        };
        assert_eq!(run(4), "true");
        assert_eq!(run(5), "false");
    }

    /// SHACL-AF §5.2/§9.5: a call missing a mandatory argument yields no result
    /// node. The body here ignores its parameter and always succeeds, so only the
    /// mandatory-argument guard (not an unbound `?n`) can suppress the value.
    #[test]
    fn unbound_mandatory_parameter_yields_no_value() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_EVEN,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse("ASK {}"),
                kind: UserFnBody::Ask,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        // `?missing` is never bound, so the sole mandatory argument is unbound.
        let query = format!("SELECT ((<{EX_EVEN}>(?missing)) AS ?v) WHERE {{}}");
        let result = NativeSparqlEngine::new()
            .query_with_user_functions(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                &registry,
            )
            .expect("query");
        match result {
            SparqlResult::Solutions { rows, .. } => {
                assert!(
                    rows[0][0].is_none(),
                    "unbound mandatory argument must yield no value, got {:?}",
                    rows[0][0]
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// A call with the wrong argument count is a hard [`EvalError::Function`].
    #[test]
    fn wrong_arity_is_a_hard_error() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_INC,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse("SELECT ((?n + 1) AS ?result) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        // Two arguments to a one-parameter function.
        let query = format!("SELECT ((<{EX_INC}>(1, 2)) AS ?v) WHERE {{}}");
        let err = NativeSparqlEngine::new()
            .query_with_user_functions(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                &registry,
            )
            .expect_err("arity mismatch must fail");
        assert!(
            err.to_string().contains("expects"),
            "expected arity error, got {err}"
        );
    }

    /// State minted in a function body is merged back into the parent context. Two
    /// calls of a RAND()-bodied function within ONE expression share the same
    /// `&mut EvalCtx` sequentially, so the merged-back rng_state advances and the
    /// two results differ (`= ` is false); without the merge-back they would
    /// replay the identical value and compare equal.
    #[test]
    fn function_body_state_is_merged_back() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_INC,
            UserFunction {
                params: vec![],
                required: 0,
                body: parse("SELECT (RAND() AS ?result) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_INC}>() = <{EX_INC}>()) AS ?eq) WHERE {{}}");
        let result = NativeSparqlEngine::new()
            .query_with_user_functions(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                &registry,
            )
            .expect("query");
        match result {
            SparqlResult::Solutions { rows, .. } => {
                let eq = rows[0][0].as_ref().expect("eq bound");
                assert!(
                    matches!(eq, TermValue::Literal { lexical_form, .. } if lexical_form == "false"),
                    "two RAND() calls sharing a merged context must differ (eq=false), got {eq:?}"
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// Calling a function IRI that is neither a registered `sh:SPARQLFunction` nor
    /// an XSD constructor is a hard error, not a silent unbound.
    #[test]
    fn undefined_function_call_is_a_hard_error() {
        let registry = UserFunctionRegistry::new();
        let ds = empty_dataset();
        let query = "SELECT ((<http://example.org/ns#nope>(1)) AS ?v) WHERE {}".to_owned();
        let err = NativeSparqlEngine::new()
            .query_with_user_functions(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                &registry,
            )
            .expect_err("undefined function must fail");
        assert!(
            err.to_string().contains("custom SPARQL function"),
            "expected undefined-function error, got {err}"
        );
    }

    /// An argument violating a parameter's `sh:datatype` is a hard error.
    #[test]
    fn parameter_datatype_violation_is_a_hard_error() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_INC,
            UserFunction {
                params: vec![UserFnParam {
                    var: "n".to_owned(),
                    constraint: TypeConstraint {
                        datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_owned()),
                        node_kind: None,
                    },
                }],
                required: 1,
                body: parse("SELECT ((?n) AS ?result) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        // The sole parameter requires xsd:integer; pass a string.
        let query = format!("SELECT ((<{EX_INC}>(\"hello\")) AS ?v) WHERE {{}}");
        let err = NativeSparqlEngine::new()
            .query_with_user_functions(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                &registry,
            )
            .expect_err("parameter datatype violation must fail");
        assert!(
            err.to_string().contains("datatype") || err.to_string().contains("parameter"),
            "expected parameter type error, got {err}"
        );
    }

    /// A SELECT body projecting more than one variable has no well-defined return
    /// value and is a hard error.
    #[test]
    fn multi_projection_select_body_is_a_hard_error() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_INC,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse("SELECT ((?n + 1) AS ?a) ((?n + 2) AS ?b) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_INC}>(1)) AS ?v) WHERE {{}}");
        let err = NativeSparqlEngine::new()
            .query_with_user_functions(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                &registry,
            )
            .expect_err("multi-projection body must fail");
        assert!(
            err.to_string().contains("exactly one variable"),
            "expected projection-arity error, got {err}"
        );
    }

    /// A self-recursive function fails closed at the depth bound rather than
    /// overflowing the stack.
    #[test]
    fn unbounded_recursion_fails_closed() {
        let mut registry = UserFunctionRegistry::new();
        // loop(?n) calls loop(?n) — a non-terminating self-recursion.
        registry.insert(
            EX_LOOP,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                body: parse(&format!("SELECT ((<{EX_LOOP}>(?n)) AS ?result) WHERE {{}}")),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_LOOP}>(1)) AS ?v) WHERE {{}}");
        let err = NativeSparqlEngine::new()
            .query_with_user_functions(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                &registry,
            )
            .expect_err("runaway recursion must fail");
        assert!(
            err.to_string().contains("recursion"),
            "expected recursion-bound error, got {err}"
        );
    }

    /// `sh:returnType` is informational (SHACL-AF §5.3) and MAY be a class IRI, so
    /// it is NOT enforced at runtime: a function returning an IRI is accepted even
    /// when its declared return type is a class rather than a literal datatype.
    /// (The pre-fix code enforced it as a datatype and wrongly hard-failed this.)
    #[test]
    fn return_type_is_informational_not_enforced() {
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            EX_INC,
            UserFunction {
                params: vec![int_param("n")],
                required: 1,
                // Returns an IRI; declared return type is a class (rdfs:Resource).
                body: parse("SELECT (<http://example.org/ns#thing> AS ?result) WHERE {}"),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint {
                    datatype: Some("http://www.w3.org/2000/01/rdf-schema#Resource".to_owned()),
                    node_kind: None,
                },
            },
        );
        let ds = empty_dataset();
        let query = format!("SELECT ((<{EX_INC}>(1)) AS ?v) WHERE {{}}");
        let result = NativeSparqlEngine::new()
            .query_with_user_functions(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                &registry,
            )
            .expect("class-typed return must be accepted, not rejected");
        match result {
            SparqlResult::Solutions { rows, .. } => {
                assert!(
                    rows[0][0].is_some(),
                    "class-typed IRI return must be accepted and returned, got {:?}",
                    rows[0][0]
                );
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }
}
