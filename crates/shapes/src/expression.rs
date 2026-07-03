// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL Advanced Features node-expression evaluation.
//!
//! A SHACL-AF *node expression* maps a focus node to a set of nodes (a
//! [`Vec<Term>`]). This module defines the intermediate representation
//! ([`NodeExpr`]) and a deterministic evaluator ([`eval_node_expr`]).
//!
//! The wiring-free expression kinds are implemented directly: [`NodeExpr::Constant`],
//! [`NodeExpr::This`], [`NodeExpr::Path`], [`NodeExpr::Union`],
//! [`NodeExpr::Intersection`], and [`NodeExpr::If`]. Builtin function calls
//! ([`FnCall::Builtin`]) and the `sh:if` effective-boolean-value route through
//! the SPARQL seam ([`crate::sparql::eval_scalar_expr`]); user-defined
//! functions ([`FnCall::UserDefined`]) are a hard capability error. The
//! remaining kinds (filters, aggregates, `EXISTS`) need the constraint engine
//! wired in and return a hard error until later tasks land them.
//!
//! # Determinism
//!
//! [`Term`] is intentionally not `Ord`, so any set-shaped output is ordered with
//! the repo idiom `v.sort_by_key(Term::to_string); v.dedup();` (matching
//! [`crate::sparql::eval_target`]). The evaluator is wasm32-clean: no clocks,
//! threads, RNG, or filesystem.

use std::collections::HashSet;

use crate::data::ShaclDataGraph;
use crate::model::xsd;
use crate::path;
use crate::shapes::{Path, Shape};
use crate::term::{Literal, NamedNode, Term};

// ── Intermediate representation ─────────────────────────────────────────────────

/// A SHACL-AF node expression: a mapping from a focus node to a set of nodes.
///
/// Not `PartialEq`: it embeds [`Shape`] / [`Path`], which are not comparable
/// (a `Shape`'s `sh:pattern` constraint holds a compiled `regex::Regex`).
#[derive(Debug, Clone)]
pub enum NodeExpr {
    /// A constant term (`sh:this` aside — an RDF term used literally).
    Constant(Term),
    /// `sh:this` — the current focus node.
    This,
    /// A path expression — the value nodes of a [`Path`] from the focus node.
    Path(Path),
    /// `sh:filterShape` / `sh:nodes` — the nodes of `nodes` that conform to `shape`.
    Filter {
        /// The node expression producing the candidate nodes.
        nodes: Box<Self>,
        /// The shape each candidate must conform to.
        shape: Box<Shape>,
    },
    /// `sh:union` — the set-union of the operand expressions' results.
    Union(Vec<Self>),
    /// `sh:intersection` — the set-intersection of the operand expressions' results.
    Intersection(Vec<Self>),
    /// `sh:if` / `sh:then` / `sh:else` — a conditional expression.
    If {
        /// The condition expression (evaluated for its effective boolean value).
        cond: Box<Self>,
        /// The branch taken when `cond` is true.
        then: Box<Self>,
        /// The branch taken when `cond` is false (or empty).
        els: Box<Self>,
    },
    /// `sh:count` — the cardinality of `of`'s result (optionally after `DISTINCT`).
    Count {
        /// Whether to count distinct values.
        distinct: bool,
        /// The operand expression.
        of: Box<Self>,
    },
    /// `sh:distinct` — the operand's result with duplicates removed.
    Distinct(Box<Self>),
    /// `sh:min` — the minimum value of the operand's result.
    Min(Box<Self>),
    /// `sh:max` — the maximum value of the operand's result.
    Max(Box<Self>),
    /// `sh:sum` — the numeric sum of the operand's result.
    Sum(Box<Self>),
    /// `sh:limit` — the first `n` values of the operand's result.
    Limit {
        /// The operand expression.
        of: Box<Self>,
        /// The maximum number of values to keep.
        n: u64,
    },
    /// `sh:offset` — the operand's result with the first `n` values dropped.
    Offset {
        /// The operand expression.
        of: Box<Self>,
        /// The number of leading values to drop.
        n: u64,
    },
    /// `sh:orderby` — the operand's result sorted ascending or descending.
    OrderBy {
        /// The operand expression.
        of: Box<Self>,
        /// Whether to sort in descending order.
        descending: bool,
    },
    /// `sh:exists` — whether any node conforms to the given shape.
    Exists(Box<Shape>),
    /// A builtin or user-defined function call.
    Call(FnCall),
}

/// A function-call node expression (`sh:SPARQLFunction` / builtin).
#[derive(Debug, Clone)]
pub enum FnCall {
    /// A builtin (SPARQL / XPath) function identified by its IRI.
    Builtin {
        /// The function IRI.
        iri: NamedNode,
        /// The argument expressions.
        args: Vec<NodeExpr>,
    },
    /// A user-defined `sh:SPARQLFunction` identified by its IRI.
    UserDefined {
        /// The function IRI.
        iri: NamedNode,
        /// The argument expressions.
        args: Vec<NodeExpr>,
    },
}

// ── Recursion guard ─────────────────────────────────────────────────────────────

/// Detects cyclic re-entry into an in-flight `(shape id, focus)` pair while
/// evaluating shape-bearing node expressions (filters / `sh:exists`).
///
/// The guard is explicit: [`enter`](Self::enter) records a pair and errors on a
/// repeat; the caller must [`exit`](Self::exit) the same pair on every path once
/// its sub-evaluation completes.
#[derive(Debug, Default)]
pub struct RecursionGuard {
    stack: HashSet<(String, String)>,
}

impl RecursionGuard {
    /// A fresh guard with no in-flight pairs.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record `(shape_id, focus)` as in-flight.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` if the pair is already in flight — a recursion
    /// cycle through `sh:filterShape` / `sh:exists`.
    pub fn enter(&mut self, shape_id: &str, focus: &str) -> Result<(), String> {
        let key = (shape_id.to_owned(), focus.to_owned());
        if self.stack.contains(&key) {
            return Err(format!(
                "recursive node expression detected: shape {shape_id} re-entered for focus {focus}"
            ));
        }
        self.stack.insert(key);
        Ok(())
    }

    /// Clear `(shape_id, focus)` from the in-flight set.
    pub fn exit(&mut self, shape_id: &str, focus: &str) {
        self.stack.remove(&(shape_id.to_owned(), focus.to_owned()));
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────────

/// Build the canonical `xsd:boolean` term for `b` (`"true"`/`"false"`).
#[must_use]
pub fn bool_literal(b: bool) -> Term {
    let lexical = if b { "true" } else { "false" };
    Term::Literal(Literal::new_typed_literal(
        lexical,
        NamedNode::new_unchecked(xsd::BOOLEAN),
    ))
}

/// Whether `terms` is exactly the single canonical `"true"^^xsd:boolean` term.
///
/// An empty result, a `"false"` result, a non-boolean literal, or more than one
/// term are all not-true.
#[must_use]
pub fn is_true(terms: &[Term]) -> bool {
    matches!(terms, [only] if *only == bool_literal(true))
}

// ── Evaluator ───────────────────────────────────────────────────────────────────

/// Evaluate a node expression against `store`, from `focus`.
///
/// Returns the node set the expression maps `focus` to. Only the wiring-free
/// kinds are implemented; every other kind returns a hard `Err` until its
/// dependency (constraint engine / SPARQL evaluator) is wired in a later task.
///
/// # Errors
///
/// Returns `Err(String)` for a not-yet-implemented expression kind, on a
/// recursion cycle, or when a sub-expression errors.
// `guard` is currently only threaded through recursion — the shape-bearing kinds
// (`Filter` / `Exists`) that enter/exit it are wired in a later task.
#[allow(clippy::only_used_in_recursion)]
pub fn eval_node_expr<G: ShaclDataGraph>(
    store: &G,
    focus: &Term,
    expr: &NodeExpr,
    guard: &mut RecursionGuard,
) -> Result<Vec<Term>, String> {
    match expr {
        NodeExpr::Constant(t) => Ok(vec![t.clone()]),
        NodeExpr::This => Ok(vec![focus.clone()]),
        NodeExpr::Path(p) => Ok(path::eval(store, focus, p)),
        NodeExpr::Union(exprs) => {
            let mut out: Vec<Term> = Vec::new();
            for sub in exprs {
                out.extend(eval_node_expr(store, focus, sub, guard)?);
            }
            out.sort_by_key(Term::to_string);
            out.dedup();
            Ok(out)
        }
        NodeExpr::Intersection(exprs) => {
            let mut iter = exprs.iter();
            let Some(first) = iter.next() else {
                return Ok(Vec::new());
            };
            let mut acc: HashSet<Term> = eval_node_expr(store, focus, first, guard)?
                .into_iter()
                .collect();
            for sub in iter {
                let next: HashSet<Term> = eval_node_expr(store, focus, sub, guard)?
                    .into_iter()
                    .collect();
                acc.retain(|t| next.contains(t));
            }
            let mut out: Vec<Term> = acc.into_iter().collect();
            out.sort_by_key(Term::to_string);
            out.dedup();
            Ok(out)
        }
        NodeExpr::If { cond, then, els } => {
            // Propagate a condition error rather than swallowing it.
            let cond_nodes = eval_node_expr(store, focus, cond, guard)?;
            // Per SHACL-AF the condition is a single value routed through SPARQL
            // effective-boolean-value. `IF(?c, true, false)` applies EBV to its
            // first argument, so a bound `?result` of `true`^^xsd:boolean means
            // EBV-true, `false` means EBV-false. An unbound result (`Ok(None)`)
            // is a genuine SPARQL type error (EBV of a non-EBV-able value).
            //
            // NOTE: a legitimately empty condition result (0 terms) selects
            // `els` — an absent value is not an error. A type error on a present
            // value, however, is a malformed condition and we propagate it as a
            // hard `Err` (the no-swallowed-errors rule) rather than silently
            // selecting a branch.
            let branch = match cond_nodes.as_slice() {
                [] => els,
                [t] => {
                    let ebv = crate::sparql::eval_scalar_expr(
                        &store.sparql_dataset(),
                        "IF(?c, true, false)",
                        &[("c".to_owned(), t.clone())],
                    )?;
                    match ebv {
                        Some(term) if term == bool_literal(true) => then,
                        Some(term) if term == bool_literal(false) => els,
                        _ => {
                            return Err(format!(
                                "sh:if condition value {t} has no effective boolean value"
                            ));
                        }
                    }
                }
                more => {
                    return Err(format!(
                        "sh:if condition must yield at most one value, got {}",
                        more.len()
                    ));
                }
            };
            eval_node_expr(store, focus, branch, guard)
        }
        NodeExpr::Call(FnCall::Builtin { iri, args }) => {
            // Each argument must collapse to exactly one scalar term (SHACL-AF
            // function-call arg semantics).
            let mut arg_terms: Vec<(String, Term)> = Vec::with_capacity(args.len());
            for (idx, arg) in args.iter().enumerate() {
                let values = eval_node_expr(store, focus, arg, guard)?;
                let [only] = values.as_slice() else {
                    return Err(format!(
                        "builtin function <{}> argument {idx} must yield exactly one value, got {}",
                        iri.as_str(),
                        values.len()
                    ));
                };
                arg_terms.push((format!("a{idx}"), only.clone()));
            }
            // Render `<iri>(?a0, ?a1, ...)` and route through the SPARQL seam.
            let placeholders: Vec<String> = (0..arg_terms.len()).map(|i| format!("?a{i}")).collect();
            let expr_string = format!("<{}>({})", iri.as_str(), placeholders.join(", "));
            match crate::sparql::eval_scalar_expr(&store.sparql_dataset(), &expr_string, &arg_terms)?
            {
                // A SPARQL error/unbound result is the correct SHACL-AF "no
                // value" signal — an empty node set, not a forced violation.
                Some(term) => Ok(vec![term]),
                None => Ok(Vec::new()),
            }
        }
        NodeExpr::Call(FnCall::UserDefined { iri, .. }) => Err(format!(
            "SHACL-AF user-defined function <{iri}> requires the dynamic SPARQL function registry capability, which is not yet available",
            iri = iri.as_str()
        )),
        NodeExpr::Filter { .. }
        | NodeExpr::Count { .. }
        | NodeExpr::Distinct(_)
        | NodeExpr::Min(_)
        | NodeExpr::Max(_)
        | NodeExpr::Sum(_)
        | NodeExpr::Limit { .. }
        | NodeExpr::Offset { .. }
        | NodeExpr::OrderBy { .. }
        | NodeExpr::Exists(_) => Err(format!(
            "node expression kind not yet implemented: {expr:?}"
        )),
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ::purrdf::RdfDataset;

    use super::*;
    use crate::data::IrDataGraph;

    /// Load a tiny data graph from Turtle.
    fn load_data(ttl: &str) -> IrDataGraph {
        let dataset: Arc<RdfDataset> =
            crate::text_ingest::parse_turtle_to_dataset(ttl).expect("turtle parse");
        IrDataGraph::new(dataset)
    }

    const DATA: &str = r"
        @prefix ex: <http://example.org/ns#> .
        ex:a ex:p ex:b .
        ex:a ex:p ex:c .
        ex:d ex:q ex:a .
    ";

    fn nn(iri: &str) -> Term {
        Term::NamedNode(NamedNode::new_unchecked(iri))
    }

    fn ex(local: &str) -> Term {
        nn(&format!("http://example.org/ns#{local}"))
    }

    fn pred(local: &str) -> Path {
        Path::Predicate(NamedNode::new_unchecked(format!(
            "http://example.org/ns#{local}"
        )))
    }

    #[test]
    fn constant_returns_the_term() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Constant(ex("z"));
        let result = eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("constant evals");
        assert_eq!(result, vec![ex("z")]);
    }

    #[test]
    fn this_returns_the_focus() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let result =
            eval_node_expr(&data, &ex("a"), &NodeExpr::This, &mut guard).expect("this evals");
        assert_eq!(result, vec![ex("a")]);
    }

    #[test]
    fn path_returns_value_nodes() {
        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Path(pred("p"));
        let mut result = eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("path evals");
        result.sort_by_key(Term::to_string);
        assert_eq!(result, vec![ex("b"), ex("c")]);
    }

    #[test]
    fn union_dedups_and_sorts() {
        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        // ex:a's ex:p reaches {b, c}; add ex:b explicitly → dedup keeps one b.
        let expr = NodeExpr::Union(vec![NodeExpr::Path(pred("p")), NodeExpr::Constant(ex("b"))]);
        let result = eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("union evals");
        assert_eq!(result, vec![ex("b"), ex("c")]);
        // Explicitly assert deterministic (sorted) order.
        let sorted = {
            let mut v = result.clone();
            v.sort_by_key(Term::to_string);
            v
        };
        assert_eq!(result, sorted);
    }

    #[test]
    fn intersection_keeps_common_nodes() {
        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        // {b, c} ∩ {b, z} = {b}
        let expr = NodeExpr::Intersection(vec![
            NodeExpr::Path(pred("p")),
            NodeExpr::Union(vec![
                NodeExpr::Constant(ex("b")),
                NodeExpr::Constant(ex("z")),
            ]),
        ]);
        let result =
            eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("intersection evals");
        assert_eq!(result, vec![ex("b")]);
    }

    #[test]
    fn intersection_empty_operands_is_empty() {
        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Intersection(vec![]);
        let result =
            eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("empty intersection evals");
        assert!(result.is_empty());
    }

    #[test]
    fn if_true_selects_then() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::If {
            cond: Box::new(NodeExpr::Constant(bool_literal(true))),
            then: Box::new(NodeExpr::Constant(ex("yes"))),
            els: Box::new(NodeExpr::Constant(ex("no"))),
        };
        let result = eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("if evals");
        assert_eq!(result, vec![ex("yes")]);
    }

    #[test]
    fn if_false_selects_els() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::If {
            cond: Box::new(NodeExpr::Constant(bool_literal(false))),
            then: Box::new(NodeExpr::Constant(ex("yes"))),
            els: Box::new(NodeExpr::Constant(ex("no"))),
        };
        let result = eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("if evals");
        assert_eq!(result, vec![ex("no")]);
    }

    #[test]
    fn if_empty_condition_selects_els() {
        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        // ex:a has no ex:missing edge → empty condition → els branch.
        let expr = NodeExpr::If {
            cond: Box::new(NodeExpr::Path(pred("missing"))),
            then: Box::new(NodeExpr::Constant(ex("yes"))),
            els: Box::new(NodeExpr::Constant(ex("no"))),
        };
        let result = eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("if evals");
        assert_eq!(result, vec![ex("no")]);
    }

    #[test]
    fn if_propagates_condition_error() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        // A not-yet-implemented kind as the condition must surface its error.
        let expr = NodeExpr::If {
            cond: Box::new(NodeExpr::Distinct(Box::new(NodeExpr::This))),
            then: Box::new(NodeExpr::Constant(ex("yes"))),
            els: Box::new(NodeExpr::Constant(ex("no"))),
        };
        let err = eval_node_expr(&data, &ex("a"), &expr, &mut guard).unwrap_err();
        assert!(err.contains("not yet implemented"), "got: {err}");
    }

    #[test]
    fn unimplemented_kind_is_hard_error() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Exists(Box::new(Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![],
            severity: crate::report::Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
        }));
        let err = eval_node_expr(&data, &ex("a"), &expr, &mut guard).unwrap_err();
        assert!(err.contains("not yet implemented"), "got: {err}");
    }

    #[test]
    fn is_true_boundaries() {
        assert!(!is_true(&[]), "empty ⇒ false");
        assert!(!is_true(&[bool_literal(false)]), "single false ⇒ false");
        assert!(
            !is_true(&[Term::Literal(Literal::new_simple_literal("true"))]),
            "non-boolean literal ⇒ false"
        );
        assert!(
            !is_true(&[bool_literal(true), bool_literal(true)]),
            "two trues ⇒ false"
        );
        assert!(
            is_true(&[bool_literal(true)]),
            "single canonical true ⇒ true"
        );
    }

    #[test]
    fn builtin_call_evaluates_through_sparql_seam() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        // The `xsd:boolean` constructor is a call-position builtin the SPARQL
        // engine resolves (an XSD cast): xsd:boolean("true") → true.
        let expr = NodeExpr::Call(FnCall::Builtin {
            iri: NamedNode::new_unchecked(xsd::BOOLEAN),
            args: vec![NodeExpr::Constant(Term::Literal(
                Literal::new_simple_literal("true"),
            ))],
        });
        let result = eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("builtin evals");
        assert_eq!(result, vec![bool_literal(true)]);
    }

    #[test]
    fn builtin_call_unsupported_fn_is_hard_error() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        // An IRI the SPARQL engine does not resolve as a builtin cast is a hard
        // seam error (an unsupported custom function), not a swallowed empty set.
        let expr = NodeExpr::Call(FnCall::Builtin {
            iri: NamedNode::new_unchecked("http://example.org/ns#nope"),
            args: vec![NodeExpr::Constant(Term::Literal(
                Literal::new_simple_literal("x"),
            ))],
        });
        let err = eval_node_expr(&data, &ex("a"), &expr, &mut guard).unwrap_err();
        assert!(err.contains("custom SPARQL function"), "got: {err}");
    }

    #[test]
    fn builtin_call_arg_arity_error() {
        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        // ex:a ex:p reaches {b, c} — two terms — so the single-arg builtin must
        // reject the >1-term argument.
        let expr = NodeExpr::Call(FnCall::Builtin {
            iri: NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#string"),
            args: vec![NodeExpr::Path(pred("p"))],
        });
        let err = eval_node_expr(&data, &ex("a"), &expr, &mut guard).unwrap_err();
        assert!(
            err.contains("argument 0 must yield exactly one value, got 2"),
            "got: {err}"
        );
    }

    #[test]
    fn user_defined_call_is_capability_error() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Call(FnCall::UserDefined {
            iri: NamedNode::new_unchecked("http://example.org/ns#myFn"),
            args: vec![],
        });
        let err = eval_node_expr(&data, &ex("a"), &expr, &mut guard).unwrap_err();
        assert!(
            err.contains(
                "requires the dynamic SPARQL function registry capability, which is not yet available"
            ),
            "got: {err}"
        );
    }

    #[test]
    fn if_numeric_condition_ebv_true_selects_then() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        // A non-zero xsd:integer has EBV true.
        let expr = NodeExpr::If {
            cond: Box::new(NodeExpr::Constant(Term::Literal(
                Literal::new_typed_literal("5", NamedNode::new_unchecked(xsd::INTEGER)),
            ))),
            then: Box::new(NodeExpr::Constant(ex("yes"))),
            els: Box::new(NodeExpr::Constant(ex("no"))),
        };
        let result = eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("if evals");
        assert_eq!(result, vec![ex("yes")]);
    }

    #[test]
    fn if_numeric_condition_ebv_false_selects_els() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        // Zero has EBV false.
        let expr = NodeExpr::If {
            cond: Box::new(NodeExpr::Constant(Term::Literal(
                Literal::new_typed_literal("0", NamedNode::new_unchecked(xsd::INTEGER)),
            ))),
            then: Box::new(NodeExpr::Constant(ex("yes"))),
            els: Box::new(NodeExpr::Constant(ex("no"))),
        };
        let result = eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("if evals");
        assert_eq!(result, vec![ex("no")]);
    }

    #[test]
    fn if_non_ebv_condition_is_hard_error() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        // An IRI has no effective boolean value → a genuine type error → Err.
        let expr = NodeExpr::If {
            cond: Box::new(NodeExpr::Constant(ex("iri"))),
            then: Box::new(NodeExpr::Constant(ex("yes"))),
            els: Box::new(NodeExpr::Constant(ex("no"))),
        };
        let err = eval_node_expr(&data, &ex("a"), &expr, &mut guard).unwrap_err();
        assert!(err.contains("no effective boolean value"), "got: {err}");
    }

    #[test]
    fn recursion_guard_detects_reentry() {
        let mut guard = RecursionGuard::new();
        guard.enter("shapeA", "focusX").expect("first enter ok");
        let err = guard.enter("shapeA", "focusX").unwrap_err();
        assert!(err.contains("recursive"), "got: {err}");
        guard.exit("shapeA", "focusX");
        guard
            .enter("shapeA", "focusX")
            .expect("re-enter after exit ok");
    }
}
