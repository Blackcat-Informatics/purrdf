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
//! [`NodeExpr::Intersection`], [`NodeExpr::If`], and the native set operators
//! [`NodeExpr::Distinct`], [`NodeExpr::Count`], [`NodeExpr::Offset`], and
//! [`NodeExpr::Limit`]. [`NodeExpr::OrderBy`] and the numeric aggregates
//! [`NodeExpr::Min`] / [`NodeExpr::Max`] / [`NodeExpr::Sum`] delegate to the
//! SPARQL engine ([`crate::sparql::eval_order`] /
//! [`crate::sparql::eval_aggregate`]) so value/numeric ordering and
//! type-promotion match the engine exactly. Builtin function calls ([`FnCall::Builtin`]) and
//! the `sh:if` effective-boolean-value route through the SPARQL seam
//! ([`crate::sparql::eval_scalar_expr`]); user-defined functions
//! ([`FnCall::UserDefined`]) are a hard capability error. The shape-bearing kind
//! [`NodeExpr::Filter`] (`sh:filterShape` / `sh:nodes`) re-enters the constraint
//! engine ([`crate::constraints::conforms`]) under a depth-bounded
//! [`RecursionGuard`] so a cyclic filter reference fails closed with a hard
//! error rather than overflowing the stack. [`NodeExpr::Exists`] (`sh:exists`)
//! is a node-expression predicate: true iff its inner expression yields at least
//! one node for the focus.
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
    /// `sh:exists` — true iff the inner node expression yields at least one node
    /// for the focus. Adopted semantics: `sh:exists` takes a NODE EXPRESSION (a
    /// shape does not "produce nodes"), evaluated for existence of any result.
    Exists(Box<Self>),
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
/// The guard has two layers. Within a single expression tree,
/// [`enter`](Self::enter) records an `(shape id, focus)` pair and errors on a
/// repeat; the caller must [`exit`](Self::exit) the same pair on every path once
/// its sub-evaluation completes. Across the constraint boundary — a
/// [`NodeExpr::Filter`] re-enters [`crate::constraints::conforms`], which builds
/// a FRESH guard per value node, so the in-flight set does not carry over — the
/// guard also tracks a monotone [`depth`](Self::depth). The constraint engine
/// seeds each nested evaluation with the caller's depth and hard-fails past
/// [`MAX_RECURSION_DEPTH`], so a mutually-recursive filter/exists cycle
/// fails closed instead of overflowing the stack.
#[derive(Debug, Default)]
pub struct RecursionGuard {
    stack: HashSet<(String, String)>,
    depth: u32,
}

/// Maximum nested `sh:filterShape` / `sh:exists` re-entry depth. Legitimate
/// SHACL shapes nest only a handful of filter layers; a mutually-recursive cycle
/// grows without bound and trips this ceiling, fail-closed, well before the
/// native stack is exhausted.
pub const MAX_RECURSION_DEPTH: u32 = 64;

impl RecursionGuard {
    /// A fresh guard with no in-flight pairs, at depth zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A fresh guard seeded at `depth` — used when the constraint engine
    /// re-enters expression evaluation across the `conforms` boundary so the
    /// filter/exists recursion depth is preserved across the fresh guard.
    #[must_use]
    pub fn with_depth(depth: u32) -> Self {
        Self {
            stack: HashSet::new(),
            depth,
        }
    }

    /// The current filter/exists re-entry depth carried by this guard.
    #[must_use]
    pub fn depth(&self) -> u32 {
        self.depth
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

/// Whether `terms` is exactly one `xsd:boolean` literal whose parsed VALUE is
/// true.
///
/// Both the canonical `"true"` and the alternative valid lexical `"1"` are
/// accepted (delegated to the XSD boolean value parser). A `"false"`/`"0"`
/// result, a non-boolean datatype (e.g. `"5"^^xsd:integer` — EBV-true but NOT a
/// boolean-true value, a genuine violation per SHACL-AF), an IRI, a blank node,
/// an empty result, or more than one term are all not-true. This is a value-true
/// check on `xsd:boolean`, deliberately narrower than full effective-boolean-value.
#[must_use]
pub fn is_true(terms: &[Term]) -> bool {
    let [Term::Literal(lit)] = terms else {
        return false;
    };
    matches!(
        purrdf_xsd::parse_by_iri(lit.value(), lit.datatype_str()),
        Ok(Some(purrdf_xsd::XsdValue::Boolean(true)))
    )
}

// ── Evaluator ───────────────────────────────────────────────────────────────────

/// Evaluate a node expression against `store`, from `focus`.
///
/// Returns the node set the expression maps `focus` to. [`NodeExpr::Filter`]
/// re-enters the constraint engine under `guard`; a cyclic filter reference is a
/// hard `Err` (see [`RecursionGuard`]).
///
/// # Errors
///
/// Returns `Err(String)` for an unsupported (user-defined function) kind, on a
/// recursion cycle or depth-limit breach, or when a sub-expression errors.
pub fn eval_node_expr<G: ShaclDataGraph>(
    store: &G,
    focus: &Term,
    expr: &NodeExpr,
    guard: &mut RecursionGuard,
) -> Result<Vec<Term>, String> {
    match expr {
        NodeExpr::Constant(t) => Ok(vec![t.clone()]),
        NodeExpr::This => Ok(vec![focus.clone()]),
        NodeExpr::Path(p) => {
            // Node-expression set outputs are canonicalized HERE (sort+dedup) so
            // sh:offset / sh:limit applied directly to a bare Path set are
            // deterministic. `path::eval`'s crate-wide first-seen iteration order
            // is left untouched (it is used elsewhere for path traversal).
            let mut v = path::eval(store, focus, p);
            v.sort_by_key(Term::to_string);
            v.dedup();
            Ok(v)
        }
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
        NodeExpr::Distinct(of) => {
            let mut out = eval_node_expr(store, focus, of, guard)?;
            out.sort_by_key(Term::to_string);
            out.dedup();
            Ok(out)
        }
        NodeExpr::Count { distinct, of } => {
            let mut out = eval_node_expr(store, focus, of, guard)?;
            if *distinct {
                out.sort_by_key(Term::to_string);
                out.dedup();
            }
            // Element count as a canonical `xsd:integer`. `usize::to_string`
            // avoids a lossy `as` cast.
            Ok(vec![Term::Literal(Literal::new_typed_literal(
                out.len().to_string(),
                NamedNode::new_unchecked(xsd::INTEGER),
            ))])
        }
        NodeExpr::OrderBy { of, descending } => {
            // Ordering follows SPARQL ORDER BY *value* semantics via the engine
            // (numeric/typed value order — e.g. "2"^^xsd:integer < "10"^^xsd:integer
            // — NOT N-Triples lexical order), and PRESERVES duplicates (SPARQL
            // sequence semantics — no dedup). Blank-node / quoted-triple operands
            // cannot appear in a VALUES block and are a hard error.
            let operands = eval_node_expr(store, focus, of, guard)?;
            crate::sparql::eval_order(&store.sparql_dataset(), &operands, *descending)
        }
        NodeExpr::Offset { of, n } => {
            let out = eval_node_expr(store, focus, of, guard)?;
            let skip = usize::try_from(*n).map_err(|e| format!("sh:offset value too large: {e}"))?;
            // Ordering is the caller's responsibility (an OrderBy wrapper) — apply
            // the offset to the already-produced sequence. The parser nests these
            // as `Limit(Offset(OrderBy(core)))`, so evaluation composes naturally:
            // OrderBy runs first, then Offset skips, then Limit truncates.
            Ok(out.into_iter().skip(skip).collect())
        }
        NodeExpr::Limit { of, n } => {
            let out = eval_node_expr(store, focus, of, guard)?;
            let take = usize::try_from(*n).map_err(|e| format!("sh:limit value too large: {e}"))?;
            Ok(out.into_iter().take(take).collect())
        }
        NodeExpr::Min(of) => aggregate(store, focus, of, "MIN", guard),
        NodeExpr::Max(of) => aggregate(store, focus, of, "MAX", guard),
        NodeExpr::Sum(of) => aggregate(store, focus, of, "SUM", guard),
        NodeExpr::Filter { nodes, shape } => {
            // Candidate nodes retained iff they conform to `shape`. The re-entry
            // into `conforms` is a fresh guard/subtree, so we (a) guard the
            // in-flight `(shape id, candidate)` pair against same-tree re-entry
            // and (b) thread the monotone depth across the constraint boundary so
            // a cross-shape filter cycle fails closed (depth ceiling) rather than
            // overflowing the stack.
            let candidates = eval_node_expr(store, focus, nodes, guard)?;
            let shape_id = shape.id.to_string();
            let next_depth = guard.depth().saturating_add(1);
            let mut kept: Vec<Term> = Vec::new();
            for value in candidates {
                let value_str = value.to_string();
                guard.enter(&shape_id, &value_str)?;
                // Capture the Result, exit the guard, THEN propagate — a clean
                // exit before the `?` avoids leaving stale in-flight state.
                let keep = crate::constraints::conforms_with_depth(
                    store, &value, shape, next_depth,
                );
                guard.exit(&shape_id, &value_str);
                if keep? {
                    kept.push(value);
                }
            }
            // Canonicalize the node-expression set output here (sort+dedup) so
            // sh:offset / sh:limit over a bare Filter set are deterministic
            // rather than store-iteration-order dependent.
            kept.sort_by_key(Term::to_string);
            kept.dedup();
            Ok(kept)
        }
        NodeExpr::Exists(inner) => {
            // `sh:exists` is a node-expression predicate: true iff `inner`
            // produces at least one node for the focus. A nested Filter inside
            // `inner` re-enters the guarded constraint engine itself.
            let out = eval_node_expr(store, focus, inner, guard)?;
            Ok(vec![bool_literal(!out.is_empty())])
        }
    }
}

/// Evaluate a set aggregate (`"MIN"`/`"MAX"`/`"SUM"`) over `of`'s result via the
/// single SPARQL path ([`crate::sparql::eval_aggregate`]).
///
/// The operands are evaluated first, then delegated to the SPARQL engine so
/// numeric type-promotion and ordering match the engine exactly (there is no
/// parallel Rust numeric fold). `SUM` of an empty set is `0`^^`xsd:integer`;
/// `MIN`/`MAX` of an empty set is unbound → an empty node set.
fn aggregate<G: ShaclDataGraph>(
    store: &G,
    focus: &Term,
    of: &NodeExpr,
    agg: &str,
    guard: &mut RecursionGuard,
) -> Result<Vec<Term>, String> {
    let operands = eval_node_expr(store, focus, of, guard)?;
    match crate::sparql::eval_aggregate(&store.sparql_dataset(), agg, &operands)? {
        Some(term) => Ok(vec![term]),
        None => Ok(Vec::new()),
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
        // The Path arm canonicalizes (sort+dedup) locally, so the result is
        // returned already sorted — no manual sort needed.
        let result = eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("path evals");
        assert_eq!(result, vec![ex("b"), ex("c")]);
        let sorted = {
            let mut v = result.clone();
            v.sort_by_key(Term::to_string);
            v
        };
        assert_eq!(result, sorted, "Path result must be returned sorted");
    }

    #[test]
    fn filter_result_is_returned_sorted() {
        use crate::report::Severity;

        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        // An empty (no-constraint) shape: every candidate conforms, so the Filter
        // output is exactly its candidate set — canonicalized (sort+dedup) locally.
        let shape = Shape {
            id: ex("leaf"),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
        };
        // Candidates supplied out of sorted order (c, a, b) to prove ordering.
        let expr = NodeExpr::Filter {
            nodes: Box::new(NodeExpr::Union(vec![
                NodeExpr::Constant(ex("c")),
                NodeExpr::Constant(ex("a")),
                NodeExpr::Constant(ex("b")),
            ])),
            shape: Box::new(shape),
        };
        let result = eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("filter evals");
        assert_eq!(
            result,
            vec![ex("a"), ex("b"), ex("c")],
            "Filter result must be returned sorted"
        );
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
        // A hard-erroring kind as the condition must surface its error.
        let expr = NodeExpr::If {
            cond: Box::new(NodeExpr::Call(FnCall::UserDefined {
                iri: NamedNode::new_unchecked("http://example.org/ns#myFn"),
                args: vec![],
            })),
            then: Box::new(NodeExpr::Constant(ex("yes"))),
            els: Box::new(NodeExpr::Constant(ex("no"))),
        };
        let err = eval_node_expr(&data, &ex("a"), &expr, &mut guard).unwrap_err();
        assert!(err.contains("user-defined function"), "got: {err}");
    }

    #[test]
    fn exists_true_when_inner_yields_nodes() {
        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        // ex:a has ex:p values → exists true.
        let expr = NodeExpr::Exists(Box::new(NodeExpr::Path(pred("p"))));
        let result = eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("exists evals");
        assert_eq!(result, vec![bool_literal(true)]);
    }

    #[test]
    fn exists_false_when_inner_empty() {
        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        // ex:a has no ex:missing edge → exists false.
        let expr = NodeExpr::Exists(Box::new(NodeExpr::Path(pred("missing"))));
        let result = eval_node_expr(&data, &ex("a"), &expr, &mut guard).expect("exists evals");
        assert_eq!(result, vec![bool_literal(false)]);
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
        // A value-true xsd:boolean written with the alternative lexical "1" is
        // still boolean-true (value semantics, not canonical-lexical matching).
        assert!(
            is_true(&[Term::Literal(Literal::new_typed_literal(
                "1",
                NamedNode::new_unchecked(xsd::BOOLEAN),
            ))]),
            "\"1\"^^xsd:boolean ⇒ true"
        );
        assert!(
            !is_true(&[Term::Literal(Literal::new_typed_literal(
                "0",
                NamedNode::new_unchecked(xsd::BOOLEAN),
            ))]),
            "\"0\"^^xsd:boolean ⇒ false"
        );
        // A non-boolean that is merely EBV-true (a genuine violation per
        // SHACL-AF: the expression must yield boolean true, not EBV-true).
        assert!(
            !is_true(&[Term::Literal(Literal::new_typed_literal(
                "5",
                NamedNode::new_unchecked(xsd::INTEGER),
            ))]),
            "\"5\"^^xsd:integer ⇒ false (not EBV-broadened)"
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

    // ── Aggregation / paging / ordering ──────────────────────────────────────

    /// A data graph with numeric values and orderable IRIs off one focus node.
    const AGG_DATA: &str = r"
        @prefix ex: <http://example.org/ns#> .
        ex:x ex:n 1, 2, 3 .
        ex:x ex:e ex:a, ex:b, ex:c .
    ";

    fn int_lit(n: &str) -> Term {
        Term::Literal(Literal::new_typed_literal(
            n,
            NamedNode::new_unchecked(xsd::INTEGER),
        ))
    }

    #[test]
    fn distinct_returns_sorted_unique_set() {
        let data = load_data(AGG_DATA);
        let mut guard = RecursionGuard::new();
        // NOTE: no node-expression kind emits a multiset (Path/Union/… all dedup),
        // so Distinct's observable behaviour over real operands is "sorted set".
        let expr = NodeExpr::Distinct(Box::new(NodeExpr::Path(pred("e"))));
        let result = eval_node_expr(&data, &ex("x"), &expr, &mut guard).expect("distinct evals");
        assert_eq!(result, vec![ex("a"), ex("b"), ex("c")]);
    }

    #[test]
    fn count_returns_cardinality_integer() {
        let data = load_data(AGG_DATA);
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Count {
            distinct: false,
            of: Box::new(NodeExpr::Path(pred("n"))),
        };
        let result = eval_node_expr(&data, &ex("x"), &expr, &mut guard).expect("count evals");
        assert_eq!(result, vec![int_lit("3")]);
    }

    #[test]
    fn count_distinct_returns_cardinality_integer() {
        let data = load_data(AGG_DATA);
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Count {
            distinct: true,
            of: Box::new(NodeExpr::Path(pred("e"))),
        };
        let result =
            eval_node_expr(&data, &ex("x"), &expr, &mut guard).expect("distinct count evals");
        assert_eq!(result, vec![int_lit("3")]);
    }

    #[test]
    fn count_of_empty_is_zero() {
        let data = load_data(AGG_DATA);
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Count {
            distinct: false,
            of: Box::new(NodeExpr::Path(pred("missing"))),
        };
        let result = eval_node_expr(&data, &ex("x"), &expr, &mut guard).expect("count evals");
        assert_eq!(result, vec![int_lit("0")]);
    }

    #[test]
    fn min_max_sum_over_integers() {
        let data = load_data(AGG_DATA);
        let mut guard = RecursionGuard::new();
        let path = || Box::new(NodeExpr::Path(pred("n")));

        let min =
            eval_node_expr(&data, &ex("x"), &NodeExpr::Min(path()), &mut guard).expect("min evals");
        assert_eq!(min, vec![int_lit("1")]);
        let max =
            eval_node_expr(&data, &ex("x"), &NodeExpr::Max(path()), &mut guard).expect("max evals");
        assert_eq!(max, vec![int_lit("3")]);
        let sum =
            eval_node_expr(&data, &ex("x"), &NodeExpr::Sum(path()), &mut guard).expect("sum evals");
        assert_eq!(sum, vec![int_lit("6")]);
    }

    #[test]
    fn sum_of_empty_is_zero_min_max_of_empty_is_unbound() {
        let data = load_data(AGG_DATA);
        let mut guard = RecursionGuard::new();
        let empty = || Box::new(NodeExpr::Path(pred("missing")));

        let sum = eval_node_expr(&data, &ex("x"), &NodeExpr::Sum(empty()), &mut guard)
            .expect("sum evals");
        assert_eq!(sum, vec![int_lit("0")]);
        let min = eval_node_expr(&data, &ex("x"), &NodeExpr::Min(empty()), &mut guard)
            .expect("min evals");
        assert!(min.is_empty(), "min of empty is unbound");
        let max = eval_node_expr(&data, &ex("x"), &NodeExpr::Max(empty()), &mut guard)
            .expect("max evals");
        assert!(max.is_empty(), "max of empty is unbound");
    }

    #[test]
    fn sum_promotes_int_and_decimal() {
        let data = load_data(
            r"
            @prefix ex: <http://example.org/ns#> .
            ex:x ex:v 1 .
            ex:x ex:v 2.5 .
        ",
        );
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Sum(Box::new(NodeExpr::Path(pred("v"))));
        let result = eval_node_expr(&data, &ex("x"), &expr, &mut guard).expect("sum evals");
        // 1 (int) + 2.5 (decimal) promotes to xsd:decimal 3.5.
        assert_eq!(
            result,
            vec![Term::Literal(Literal::new_typed_literal(
                "3.5",
                NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#decimal"),
            ))]
        );
    }

    #[test]
    fn orderby_ascending_and_descending() {
        let data = load_data(AGG_DATA);
        let mut guard = RecursionGuard::new();
        let asc = NodeExpr::OrderBy {
            of: Box::new(NodeExpr::Path(pred("e"))),
            descending: false,
        };
        let result = eval_node_expr(&data, &ex("x"), &asc, &mut guard).expect("orderby evals");
        assert_eq!(result, vec![ex("a"), ex("b"), ex("c")]);

        let desc = NodeExpr::OrderBy {
            of: Box::new(NodeExpr::Path(pred("e"))),
            descending: true,
        };
        let result = eval_node_expr(&data, &ex("x"), &desc, &mut guard).expect("orderby evals");
        assert_eq!(result, vec![ex("c"), ex("b"), ex("a")]);
    }

    #[test]
    fn offset_skips_leading_values() {
        let data = load_data(AGG_DATA);
        let mut guard = RecursionGuard::new();
        // OrderBy first so the sequence is deterministic before the offset.
        let expr = NodeExpr::Offset {
            of: Box::new(NodeExpr::OrderBy {
                of: Box::new(NodeExpr::Path(pred("e"))),
                descending: false,
            }),
            n: 1,
        };
        let result = eval_node_expr(&data, &ex("x"), &expr, &mut guard).expect("offset evals");
        assert_eq!(result, vec![ex("b"), ex("c")]);
    }

    #[test]
    fn limit_takes_leading_values() {
        let data = load_data(AGG_DATA);
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Limit {
            of: Box::new(NodeExpr::OrderBy {
                of: Box::new(NodeExpr::Path(pred("e"))),
                descending: false,
            }),
            n: 2,
        };
        let result = eval_node_expr(&data, &ex("x"), &expr, &mut guard).expect("limit evals");
        assert_eq!(result, vec![ex("a"), ex("b")]);
    }

    #[test]
    fn composed_limit_offset_orderby() {
        let data = load_data(AGG_DATA);
        let mut guard = RecursionGuard::new();
        // Parser nests as Limit(Offset(OrderBy(core))) — eval composes as
        // orderby → offset → limit.
        let expr = NodeExpr::Limit {
            of: Box::new(NodeExpr::Offset {
                of: Box::new(NodeExpr::OrderBy {
                    of: Box::new(NodeExpr::Path(pred("e"))),
                    descending: false,
                }),
                n: 1,
            }),
            n: 1,
        };
        // orderby → [a,b,c]; offset 1 → [b,c]; limit 1 → [b].
        let result = eval_node_expr(&data, &ex("x"), &expr, &mut guard).expect("composed evals");
        assert_eq!(result, vec![ex("b")]);
    }

    #[test]
    fn aggregate_over_blank_node_is_type_error() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Min(Box::new(NodeExpr::Constant(Term::blank("b0"))));
        let err = eval_node_expr(&data, &ex("x"), &expr, &mut guard).unwrap_err();
        assert!(
            err.contains("cannot appear in a SPARQL VALUES block"),
            "got: {err}"
        );
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

    /// A `sh:filterShape` chain deeper than [`MAX_RECURSION_DEPTH`] re-enters the
    /// constraint engine past the depth ceiling and must fail CLOSED — a hard
    /// `Err` naming the recursion depth — instead of overflowing the stack.
    ///
    /// The shapes graph parser flattens IRI-referenced shape cycles at load time
    /// (substituting an empty shape for an in-flight IRI), so an unbounded
    /// re-entry can only arise from a hand-built (or future non-parser) shape
    /// tree; this test builds that tree directly. Each level's `sh:expression`
    /// filters `sh:this` through the next shape, so validating the outermost
    /// shape re-enters `conforms` once per level.
    #[test]
    fn filter_chain_deeper_than_max_depth_fails_closed() {
        use crate::report::Severity;
        use crate::shapes::Constraint;

        // The validator's per-level frame (the large `eval_constraint` match) is
        // sizeable, so a `MAX_RECURSION_DEPTH`-deep chain needs more than a test
        // thread's default 2 MiB stack. Run on a generous stack so the DEPTH
        // GUARD — not a stack overflow — is what terminates the recursion; the
        // guard is what protects the (larger) production stack in the same way.
        let handle = std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                let data = load_data(DATA);

                let make_shape = |id: Term, constraints: Vec<Constraint>| Shape {
                    id,
                    targets: vec![],
                    constraints,
                    property_shapes: vec![],
                    severity: Severity::Violation,
                    message: None,
                    deactivated: false,
                    box_roles: vec![],
                };

                // Innermost shape: no constraints ⇒ every node trivially conforms.
                let mut shape = make_shape(ex("leaf"), vec![]);
                // Wrap one filter-through-inner layer per level, past the ceiling.
                let levels = MAX_RECURSION_DEPTH + 5;
                for i in 0..levels {
                    let expr = NodeExpr::Filter {
                        nodes: Box::new(NodeExpr::This),
                        shape: Box::new(shape),
                    };
                    shape = make_shape(
                        ex(&format!("s{i}")),
                        vec![Constraint::Expression {
                            expr,
                            message: None,
                            severity: None,
                        }],
                    );
                }

                crate::constraints::conforms(&data, &ex("a"), &shape)
            })
            .expect("spawn deep-stack thread");

        let err = handle
            .join()
            .expect("deep-stack thread must not overflow — the depth guard terminates it")
            .expect_err("a filter chain past the depth ceiling must be a hard error");
        assert!(
            err.contains("recursion depth"),
            "error should name the recursion depth, got: {err}"
        );
    }
}
