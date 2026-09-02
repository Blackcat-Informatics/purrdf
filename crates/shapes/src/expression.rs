// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL node-expression evaluation.
//!
//! A *node expression* maps a focus node to a sequence of nodes (a
//! [`Vec<Term>`]). This module defines the intermediate representation
//! ([`NodeExpr`]) and a deterministic evaluator ([`eval_node_expr_in_scope`], with
//! [`eval_node_expr`] as its empty-scope entry point).
//!
//! # Two spec surfaces, one implementation
//!
//! Two W3C specifications define this language: SHACL Advanced Features, which
//! spells its kinds in the `sh:` namespace, and SHACL 1.2 Node Expressions, which
//! spells them in `shnex:` (`http://www.w3.org/ns/shacl-node-expr#`) and adds
//! kinds the older document has no term for. PurRDF accepts BOTH spellings and
//! maps them onto the SAME [`NodeExpr`] arms — there is exactly one evaluation
//! path per arm, nothing here is conditional or absent, and a node carrying both
//! spellings of one kind is an ambiguity that hard-fails at parse time. It is the
//! same arrangement as supporting two RDF syntaxes over one graph model.
//!
//! The kinds contributed by SHACL 1.2 Node Expressions are [`NodeExpr::Empty`]
//! (§4.1.1), [`NodeExpr::Var`] (§4.1.2), [`NodeExpr::List`] (§4.1.3),
//! [`NodeExpr::PathValues`] (§4.1.4), [`NodeExpr::Concat`] (§4.2.3),
//! [`NodeExpr::Remove`] (§4.2.4), [`NodeExpr::FlatMap`] (§4.3.1),
//! [`NodeExpr::FindFirst`] (§4.3.2), [`NodeExpr::MatchAll`] (§4.3.3),
//! [`NodeExpr::InstancesOf`] (§4.5.1), [`NodeExpr::NodesMatching`] (§4.5.2) and
//! [`NodeExpr::ConformsToShape`] (§4.5.3). An RDF 1.2 triple term is a triple term
//! expression (§3.1.3) — a constant that evaluates to itself — and is a first-class
//! value everywhere else in the language, focus nodes and value nodes alike.
//!
//! # Scope
//!
//! Evaluation carries a [`Scope`]: the `scope` argument of the specification's
//! `evalExpr(expr, focusGraph, focusNode, scope)`. It is a borrowed linked list of
//! [`Binding`] frames, so the empty case (the overwhelming majority of the tree)
//! costs one word and no allocation. `sh:expression` binds `value` to the value
//! node under test (§7.1); `sh:nodeByExpression` evaluates in the empty scope
//! (§7.2); `shnex:flatMap` and `shnex:orderBy` rebind the FOCUS NODE per element
//! and thread the caller's scope through unchanged.
//!
//! The wiring-free expression kinds are implemented directly: [`NodeExpr::Constant`],
//! [`NodeExpr::This`], [`NodeExpr::Path`], [`NodeExpr::Union`],
//! [`NodeExpr::Intersection`], [`NodeExpr::If`], and the native set operators
//! [`NodeExpr::Distinct`], [`NodeExpr::Count`], [`NodeExpr::Offset`], and
//! [`NodeExpr::Limit`]. [`NodeExpr::OrderBy`] and the numeric aggregates
//! [`NodeExpr::Min`] / [`NodeExpr::Max`] / [`NodeExpr::Sum`] delegate to the
//! SPARQL engine ([`crate::sparql::eval_order`] /
//! [`crate::sparql::eval_aggregate`]) so value/numeric ordering and
//! type-promotion match the engine exactly. Builtin function calls
//! ([`FnCall::Builtin`]) and the `sh:if` effective-boolean-value route through the
//! SPARQL seam ([`crate::sparql::eval_scalar_expr`]). The reachable builtin set is:
//! XSD constructor/cast IRIs (e.g. `xsd:boolean`, `xsd:integer`) and any purrdf
//! custom function IRI the SPARQL engine registers, both dispatched via the
//! `<iri>(…)` call form; PLUS the XPath/XQuery-functions-namespace
//! (`http://www.w3.org/2005/xpath-functions#…`) IRIs that `builtin_keyword`
//! lowers to their SPARQL 1.1 keyword (e.g. `fn:string-length` → `STRLEN`,
//! `fn:contains` → `CONTAINS`, `fn:numeric-abs` → `ABS`, `fn:matches` → `REGEX`),
//! rendered in keyword form because those builtins are keyword-only in SPARQL and
//! have no IRI call form. User-defined `sh:SPARQLFunction` calls
//! ([`FnCall::UserDefined`]) lower the same way (an `<iri>(…)` call) and resolve
//! against the shapes graph's function registry installed for the validation (see
//! [`crate::sparql::enter_function_scope`]); an unresolved call IRI is a hard error,
//! never a silent empty result. The shape-bearing kind
//! [`NodeExpr::Filter`] (`sh:filterShape` / `sh:nodes`) re-enters the constraint
//! engine ([`crate::constraints::conforms`]) under a depth-bounded
//! [`RecursionGuard`] so a cyclic filter reference fails closed with a hard
//! error rather than overflowing the stack. [`NodeExpr::Exists`] (`sh:exists`)
//! is a node-expression predicate: true iff its inner expression yields at least
//! one node for the focus.
//!
//! # Determinism, and which kinds are sequences
//!
//! [`Term`] is intentionally not `Ord`, so this module orders any SET-shaped
//! output with `crate::term::sort_terms_canonical(&mut v); v.dedup();`, using the
//! allocation-free canonical term comparator. The sibling
//! [`crate::sparql::eval_target`] uses the same ordering. The evaluator is
//! wasm32-clean: no clocks, threads, RNG, or filesystem.
//!
//! Several SHACL 1.2 kinds are SEQUENCE-valued and order-significant, and those
//! are deliberately NOT sorted and NOT deduplicated — canonicalizing them would
//! destroy the very thing the spec defines them to produce.
//! [`NodeExpr::List`] returns its members in list order; [`NodeExpr::Concat`]
//! concatenates its operands left to right, duplicates included;
//! [`NodeExpr::FlatMap`] concatenates its per-node results in input order;
//! [`NodeExpr::Remove`] preserves the order of its input; [`NodeExpr::OrderBy`]
//! produces the order it was asked for. Their determinism comes from their inputs
//! being deterministic, not from a final sort.

use std::sync::{Arc, OnceLock};

use ::purrdf::{FastMap, FastSet};

use crate::data::ShaclData;
use crate::model::xsd;
use crate::path;
use crate::shapes::{Path, Shape};
use crate::term::{Literal, NamedNode, Term};

/// The reserved `shnex:var` name that denotes the current focus node.
///
/// SHACL 1.2 Node Expressions §4.1.2 resolves this name BEFORE consulting the
/// scope, so a scope binding of the same name can never shadow the focus node.
pub const FOCUS_NODE_VAR: &str = "focusNode";

/// The `sh:expression` scope variable bound to the value node under test.
///
/// SHACL 1.2 Node Expressions §7.1 evaluates an expression constraint as
/// `evalExpr(expr, data graph, focusNode, {value: v})`, which is what makes
/// `[ shnex:var "value" ]` resolve inside an expression constraint.
pub const VALUE_VAR: &str = "value";

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
    /// `sh:orderby` — the operand's result sorted by a per-element sort key.
    ///
    /// Authority-grounded (W3C/DASH) semantics: `sh:orderby` names a node
    /// expression whose per-element values are the SORT KEY. The key is
    /// evaluated once per input element WITH THAT ELEMENT AS FOCUS, and elements
    /// are ordered by SPARQL value order over their keys. Ordering defaults to
    /// ASCENDING; direction is a separate flag (`sh:desc`).
    OrderBy {
        /// The input sequence expression (the operand to sort).
        of: Box<Self>,
        /// The sort-key node expression, evaluated per element (element-as-focus).
        key: Box<Self>,
        /// Whether to sort in descending order (default ascending).
        descending: bool,
    },
    /// `sh:exists` — true iff the inner node expression yields at least one node
    /// for the focus. Adopted semantics: `sh:exists` takes a NODE EXPRESSION (a
    /// shape does not "produce nodes"), evaluated for existence of any result.
    Exists(Box<Self>),
    /// A builtin or user-defined function call.
    Call(FnCall),

    // ── SHACL 1.2 Node Expressions §6 "Custom Node Expressions" ────────────────
    /// `shnex:arg` (§6.3) — an ARGUMENT reference, resolved against the argument
    /// scope of the innermost enclosing custom-function body.
    ///
    /// The sibling of [`Self::Var`], and deliberately a separate arm because the
    /// specification gives the two different value spaces: `shnex:var`'s scope
    /// values "are individual nodes", while `shnex:arg`'s "are node expressions",
    /// evaluated at the point of use in the EMPTY scope. An unbound key yields the
    /// empty list — §6.3's own second case, an absence rather than an error.
    Arg(ArgKey),
    /// A call of a custom node-expression function declared in the shapes graph
    /// (SHACL 1.2 Node Expressions §6.1 named-parameter / §6.2 list-parameter).
    ///
    /// The body is held BY REFERENCE, never inlined: a function whose body calls
    /// itself is legal (and bounded at evaluation by [`MAX_RECURSION_DEPTH`]),
    /// whereas inlining it at parse time would not terminate.
    CustomCall {
        /// The declared function.
        func: Arc<CustomFunction>,
        /// The arguments as authored, keyed the way the function's kind keys them:
        /// by zero-based index for a list-parameter function, by parameter `sh:path`
        /// IRI for a named-parameter one. They are node EXPRESSIONS, not values —
        /// §6.3 evaluates them where `shnex:arg` reads them.
        args: Vec<(ArgKey, Self)>,
    },

    // ── SHACL 1.2 Node Expressions (W3C WD, `shnex:` namespace) ─────────────────
    /// `shnex:EmptyExpression` (§4.1.1) — a blank node that is the subject of no
    /// triple. Its output nodes are the empty list.
    Empty,
    /// `shnex:var` (§4.1.2) — a variable reference resolved against the evaluation
    /// [`Scope`]. The name `"focusNode"` always denotes the current focus node; any
    /// other name resolves against the scope, and an unbound name yields the empty
    /// list (the spec's third case — an absence, not an error).
    Var(String),
    /// `shnex:ListExpression` (§4.1.3) — an RDF collection whose members (each an
    /// IRI or a literal) ARE the output nodes, in list order.
    ///
    /// SEQUENCE-valued and order-significant: the members are returned exactly as
    /// authored, never sorted and never deduplicated.
    List(Vec<Term>),
    /// `shnex:pathValues` with an explicit `shnex:focusNode` (§4.1.4) — the value
    /// nodes of `path` starting from the single node `focus` produces.
    ///
    /// A `shnex:pathValues` WITHOUT `shnex:focusNode` parses to [`Self::Path`]
    /// instead: the spec defines the omitted case as "the focus node from the
    /// evaluation context", which is exactly what [`Self::Path`] already walks.
    /// The explicit form needs its own arm because the spec makes a focus
    /// expression yielding MORE than one node an evaluation failure, where
    /// [`Self::FlatMap`] would concatenate.
    PathValues {
        /// The SHACL property path to walk.
        path: Path,
        /// The focus-node expression; must yield at most one node.
        focus: Box<Self>,
    },
    /// `shnex:concat` (§4.2.3) — the concatenation of the operands' output nodes.
    ///
    /// SEQUENCE-valued and order-significant: operand order is preserved and
    /// duplicates are kept (this is what distinguishes it from `sh:union`, which
    /// is the SHACL-AF set union and canonicalizes).
    Concat(Vec<Self>),
    /// `shnex:remove` / `shnex:nodes` (§4.2.4) — the nodes of `nodes` that are not
    /// also nodes of `remove`, preserving the order of `nodes`.
    ///
    /// Membership is TERM equality per the spec, so `"01"^^xsd:integer` does not
    /// remove `"1"^^xsd:integer`.
    Remove {
        /// The input-nodes expression.
        nodes: Box<Self>,
        /// The expression naming the nodes to drop.
        remove: Box<Self>,
    },
    /// `shnex:flatMap` / `shnex:nodes` (§4.3.1) — `map` evaluated once per input
    /// node WITH THAT NODE AS FOCUS, all results concatenated in input order.
    ///
    /// SEQUENCE-valued and order-significant; duplicates are preserved.
    FlatMap {
        /// The input-nodes expression (`shnex:nodes`, defaulting to the focus node).
        nodes: Box<Self>,
        /// The per-node expression (`shnex:flatMap`).
        map: Box<Self>,
    },
    /// `shnex:findFirst` / `shnex:nodes` (§4.3.2) — the first input node that
    /// conforms to `shape`, or the empty list when none does.
    FindFirst {
        /// The input-nodes expression (`shnex:nodes`, defaulting to the focus node).
        nodes: Box<Self>,
        /// The shape the first matching node must conform to.
        shape: Box<Shape>,
    },
    /// `shnex:matchAll` / `shnex:nodes` (§4.3.3) — `true` iff EVERY input node
    /// conforms to `shape`, `false` otherwise (an empty input is vacuously true).
    MatchAll {
        /// The input-nodes expression (`shnex:nodes`, defaulting to the focus node).
        nodes: Box<Self>,
        /// The shape every input node must conform to.
        shape: Box<Shape>,
    },
    /// `shnex:instancesOf` (§4.5.1) — every SHACL instance of the class in the
    /// focus graph, including instances of its subclasses.
    InstancesOf(NamedNode),
    /// `shnex:nodesMatching` (§4.5.2) — every node of the focus graph that conforms
    /// to the shape.
    NodesMatching(Box<Shape>),
    /// `shnex:conformsToShape` (§4.5.3) — `true` iff the single node the operand
    /// produces conforms to `shape`, `false` otherwise, and the empty list when the
    /// operand produces no node.
    ConformsToShape {
        /// The node expression producing the node under test (at most one node).
        node: Box<Self>,
        /// The shape the node is validated against.
        shape: ShapeArg,
    },

    // ── SHACL 1.2 SPARQL Extensions (`sh:select` / `sh:sparqlExpr`) ────────────
    /// A SPARQL-based node expression: `sh:select` (SHACL 1.2 SPARQL Extensions
    /// §6.1, function name `sh:SelectExpression`) or `sh:sparqlExpr` (§6.2,
    /// function name `sh:SPARQLExprExpression`).
    ///
    /// Both spellings reduce to the SAME thing, because §6.2 defines itself that
    /// way: a SPARQL expr expression is the expression embedded into the template
    /// `$PREFIXES$ SELECT ($EXPR$ AS ?result) WHERE {}`, which the specification
    /// spells out as the "equivalent expanded form" of the corresponding
    /// `sh:select`. The parser performs that expansion once, at shapes-load, so
    /// there is one arm, one evaluator and one query text.
    Select {
        /// The complete SELECT query, prefix header included.
        query: String,
        /// The single projected variable whose bindings are the output nodes
        /// (`result` for the `sh:sparqlExpr` spelling).
        variable: String,
        /// The authored key (`sh:select` / `sh:sparqlExpr`), quoted in diagnostics
        /// so a writer is told about the property they actually wrote.
        key: &'static str,
    },
}

/// The shape argument of `shnex:conformsToShape` (SHACL 1.2 Node Expressions
/// §4.5.3).
///
/// §4.5.3 makes that argument a NODE EXPRESSION — the spec constrains it with
/// `sh:nodeKind sh:IRI` and the words "Must produce the IRI of a well-formed
/// shape", where *produce* is the node-expression verb. So both of these are
/// legal, and the second is why this is an enum rather than a `Shape`:
///
/// ```turtle
/// # named: the shape IRI is written into the shapes graph
/// [ shnex:conformsToShape ( [ shnex:var "focusNode" ] ex:HasDirectorShape ) ]
/// # computed: the shape IRI comes out of the DATA graph
/// [ shnex:conformsToShape ( [ shnex:var "focusNode" ] [ shnex:pathValues ex:kind ] ) ]
/// ```
///
/// The two are not two features. A named argument is resolved ONCE, at load,
/// which is what lets an undefined shape IRI be refused there rather than holding
/// vacuously; a computed one cannot be, because its answer is in the data, so it
/// resolves per evaluation against the same shape index `sh:nodeByExpression`
/// (§7.2) uses — the shapes graph's own top-level shapes.
#[derive(Debug, Clone)]
pub enum ShapeArg {
    /// The argument NAMED the shape (an IRI, or an inline anonymous shape), so it
    /// was resolved and parsed at shapes-load time.
    Named(Box<Shape>),
    /// The argument COMPUTES the shape IRI, so it is resolved per evaluation.
    Computed {
        /// The node expression producing the shape IRI (exactly one node).
        expr: Box<NodeExpr>,
        /// The shapes graph's shape index, filled at the end of the shapes parse.
        /// The same handle `Constraint::NodeByExpression` carries, for the same
        /// reason: a shape that names another shape cannot resolve it while it is
        /// itself still being parsed.
        shapes: Arc<OnceLock<FastMap<String, Shape>>>,
    },
}

/// The key an argument is bound and looked up under inside a custom function's
/// body — the key space of `shnex:arg` (SHACL 1.2 Node Expressions §6.3).
///
/// §6.3 constrains the `shnex:arg` value to `sh:or ( [ sh:nodeKind sh:IRI ]
/// [ sh:datatype xsd:integer ] )`, and the two spellings mean different things:
/// an integer is a LIST parameter function's zero-based argument index (§6.2), an
/// IRI is a NAMED parameter function's parameter `sh:path` (§6.1). Keeping them as
/// two variants rather than one string means `[ shnex:arg 0 ]` can never
/// accidentally resolve a parameter whose IRI happens to render as `"0"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgKey {
    /// A zero-based argument index — `[ shnex:arg 0 ]` (§6.2).
    Index(u64),
    /// A parameter `sh:path` IRI — `[ shnex:arg ex:average ]` (§6.1).
    Named(String),
}

impl ArgKey {
    /// The SPARQL variable name this argument is pre-bound under inside a
    /// SPARQL-based function body.
    ///
    /// SHACL 1.2 SPARQL Extensions §7.2 spells an indexed argument `$arg0`, `$arg1`,
    /// … in a `sh:select` / `sh:sparqlExpr` body, matching the `shnex:arg0` parameter
    /// path §6.2 declares it under. A named argument has no spelling of its own in
    /// that document, so it takes the parameter IRI's local name — the same rule
    /// `sh:SPARQLFunction` already uses to turn a parameter predicate into a
    /// pre-bound variable.
    #[must_use]
    pub fn variable_name(&self) -> String {
        match self {
            Self::Index(index) => format!("arg{index}"),
            Self::Named(iri) => crate::shapes::local_name(iri).to_owned(),
        }
    }
}

impl core::fmt::Display for ArgKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Index(n) => write!(f, "{n}"),
            Self::Named(iri) => write!(f, "<{iri}>"),
        }
    }
}

/// Which way a custom node-expression function keys its arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomFnKind {
    /// `sh:ListParameterExpressionFunction` (SHACL 1.2 Node Expressions §6.2): the
    /// function's own IRI is its list parameter property, and its body reads
    /// arguments by zero-based index. This is the kind SHACL 1.2 SPARQL Extensions
    /// §7.3 asks a SPARQL engine to register as a callable function.
    ListParameter,
    /// `sh:NamedParameterExpressionFunction` (SHACL 1.2 Node Expressions §6.1):
    /// arguments are supplied under the parameters' own `sh:path` IRIs, and the
    /// call site is recognised by a parameter marked `sh:keyParameter true`. It has
    /// no positional call form, so it is deliberately NOT registered as a SPARQL
    /// function — §7.3 names only the list-parameter class.
    NamedParameter,
}

/// A custom node-expression function declared in the shapes graph.
///
/// # Why the body is a `OnceLock`
///
/// A function's body is a node expression that may call any declared function,
/// including itself. Declarations are therefore discovered and interned FIRST (so a
/// call site can resolve to the same `Arc` no matter which order the graph is read
/// in), and every body is parsed and installed afterwards. A body that is never
/// installed is an internal inconsistency, and evaluation reports it as a hard error
/// rather than treating the missing body as an empty result.
#[derive(Debug)]
pub struct CustomFunction {
    /// The function's own IRI. It is also the `focusNode` its body is evaluated
    /// with when the function is invoked from SPARQL (SHACL 1.2 SPARQL Extensions
    /// §7.3: "the `focusNode` passed into a custom SPARQL function based on a node
    /// expression is the IRI of the function itself").
    pub iri: NamedNode,
    /// Which way the function keys its arguments.
    pub kind: CustomFnKind,
    /// The declared parameter keys, in call order for a list-parameter function and
    /// in ascending IRI order for a named-parameter one.
    pub params: Vec<ArgKey>,
    /// The number of leading REQUIRED parameters (arity is `[required,
    /// params.len()]`), from `sh:optional`.
    pub required: usize,
    /// The `sh:bodyExpression` node expression, installed once every declaration has
    /// been interned.
    pub body: OnceLock<NodeExpr>,
}

impl CustomFunction {
    /// The installed body, or a hard error naming the function.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` when no body was ever installed — an internal
    /// inconsistency, reported rather than silently evaluated as the empty list.
    pub fn body(&self) -> Result<&NodeExpr, String> {
        self.body.get().ok_or_else(|| {
            format!(
                "internal error: custom node-expression function <{}> has no installed \
                 sh:bodyExpression",
                self.iri.as_str()
            )
        })
    }
}

/// The SPARQL surface form a `sparql:<NAME>` node-expression call lowers to.
///
/// SHACL 1.2 Node Expressions §5 makes every IRI of the W3C SPARQL 1.2 term
/// vocabulary callable "with the corresponding SPARQL function name". SPARQL
/// spells those names in four different syntactic shapes, and this enum is the
/// whole of the difference between them — the argument evaluation, the cartesian
/// product over multi-valued arguments and the result handling are shared by
/// every name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparqlCallForm {
    /// A keyword function call — `NAME(a0, a1, …)`.
    Call(&'static str),
    /// A binary infix operator — `(a0 OP a1)`. Exactly two arguments.
    Infix(&'static str),
    /// A unary prefix operator — `(OP a0)`. Exactly one argument.
    Prefix(&'static str),
    /// The `IN` / `NOT IN` membership forms — `(a0 KEYWORD (a1, a2, …))`. At
    /// least one argument (an empty candidate list is legal SPARQL).
    Membership(&'static str),
    /// `sparql:ebv` — the effective boolean value of the single argument,
    /// rendered `(!(! a0))`. SPARQL has no `EBV` keyword; `!` is the operator
    /// defined to coerce its operand through EBV, so applying it twice IS the
    /// call form, and it raises the same type error on a value that has no
    /// effective boolean value.
    Ebv,
}

impl SparqlCallForm {
    /// The SPARQL expression text for this form over `arity` placeholder
    /// variables `?a0 … ?a{arity-1}`.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` when `arity` cannot satisfy the form (an operator
    /// applied to the wrong number of operands). A keyword call's arity is the
    /// SPARQL grammar's business, and the parse of the rendered text — which the
    /// shapes parser performs at load — is what reports it.
    pub fn render(self, iri: &str, arity: usize) -> Result<String, String> {
        let arg = |i: usize| format!("?a{i}");
        match self {
            Self::Call(keyword) => {
                let args: Vec<String> = (0..arity).map(arg).collect();
                Ok(format!("{keyword}({})", args.join(", ")))
            }
            Self::Infix(op) => {
                if arity != 2 {
                    return Err(format!(
                        "<{iri}> is the SPARQL `{op}` operator and takes exactly 2 arguments, got {arity}"
                    ));
                }
                Ok(format!("({} {op} {})", arg(0), arg(1)))
            }
            Self::Prefix(op) => {
                if arity != 1 {
                    return Err(format!(
                        "<{iri}> is the SPARQL unary `{op}` operator and takes exactly 1 argument, got {arity}"
                    ));
                }
                Ok(format!("({op} {})", arg(0)))
            }
            Self::Membership(keyword) => {
                let Some(rest) = arity.checked_sub(1) else {
                    return Err(format!(
                        "<{iri}> is the SPARQL `{keyword}` form and takes at least 1 argument, got 0"
                    ));
                };
                let candidates: Vec<String> = (1..=rest).map(arg).collect();
                Ok(format!(
                    "({} {keyword} ({}))",
                    arg(0),
                    candidates.join(", ")
                ))
            }
            Self::Ebv => {
                if arity != 1 {
                    return Err(format!(
                        "<{iri}> is the SPARQL effective-boolean-value coercion and takes exactly \
                         1 argument, got {arity}"
                    ));
                }
                Ok(format!("(!(! {}))", arg(0)))
            }
        }
    }
}

/// Resolve a local name of the SPARQL 1.2 term vocabulary
/// ([`crate::model::sparql_ns`]) to the SPARQL surface form it is called
/// through — SHACL 1.2 Node Expressions §5's `sparql:<NAME>` dispatch.
///
/// The mechanism is a lookup, not a per-name branch. Every name SPARQL spells as
/// an ordinary *function call* is answered by the parser's own keyword table
/// through [`purrdf_sparql_algebra::builtin_function_keyword`], so this
/// implementation cannot claim a function the query parser would then reject,
/// and a name the parser gains is callable here the same day. The explicit table
/// below covers only the names SPARQL does NOT spell as a function call — the
/// operators, the functional forms, and the one name whose keyword is not its
/// own uppercasing (`encodeForUri` → `ENCODE_FOR_URI`).
///
/// # Errors
///
/// Returns `Err(String)` naming the IRI when the local name is not a callable
/// SPARQL function: the SPARQL aggregates (`sparql:agg-*`), which are not scalar
/// functions; the two `EXISTS` functional forms, which take a graph pattern
/// rather than argument values; and anything the vocabulary does not define. A
/// refusal, never a silent empty result.
pub fn sparql_ns_lowering(local: &str) -> Result<SparqlCallForm, String> {
    /// The SPARQL 1.2 names that are not spelled as a function call, each with
    /// the surface form it is spelled as instead.
    static NON_CALL_FORMS: &[(&str, SparqlCallForm)] = &[
        // Operators (SPARQL 1.2 Query §17.4.1 / the operator mapping table).
        ("add", SparqlCallForm::Infix("+")),
        ("subtract", SparqlCallForm::Infix("-")),
        ("multiply", SparqlCallForm::Infix("*")),
        ("divide", SparqlCallForm::Infix("/")),
        ("unary-plus", SparqlCallForm::Prefix("+")),
        ("unary-minus", SparqlCallForm::Prefix("-")),
        ("equals", SparqlCallForm::Infix("=")),
        ("not-equals", SparqlCallForm::Infix("!=")),
        ("greater-than", SparqlCallForm::Infix(">")),
        ("less-than", SparqlCallForm::Infix("<")),
        ("greater-than-or-equal", SparqlCallForm::Infix(">=")),
        ("less-than-or-equal", SparqlCallForm::Infix("<=")),
        // `sameValue` "replaces RDFterm-equal from SPARQL 1.1" and, in the
        // specification's own words, "cannot be used directly in a query": `=`
        // IS its call form, and `RDFterm-equal` is the 1.1 name of the same
        // operator. Both therefore lower to `=`, which is what the evaluator
        // already implements for them.
        ("sameValue", SparqlCallForm::Infix("=")),
        ("RDFterm-equal", SparqlCallForm::Infix("=")),
        // Functional forms.
        ("logical-and", SparqlCallForm::Infix("&&")),
        ("logical-or", SparqlCallForm::Infix("||")),
        ("logical-not", SparqlCallForm::Prefix("!")),
        // The effective boolean value of a term is what `!` applied twice
        // computes, and SPARQL has no other spelling for it: `EBV(x)` is
        // `!(!x)`, which raises the same type error on a non-EBV-able value that
        // `sparql:ebv` is defined to raise.
        ("ebv", SparqlCallForm::Ebv),
        ("in", SparqlCallForm::Membership("IN")),
        ("not-in", SparqlCallForm::Membership("NOT IN")),
        ("bound", SparqlCallForm::Call("BOUND")),
        ("if", SparqlCallForm::Call("IF")),
        ("coalesce", SparqlCallForm::Call("COALESCE")),
        ("sameTerm", SparqlCallForm::Call("sameTerm")),
        // The one function-call name whose keyword is not its own uppercasing.
        ("encodeForUri", SparqlCallForm::Call("ENCODE_FOR_URI")),
    ];

    if let Some(&(_, form)) = NON_CALL_FORMS.iter().find(|&&(name, _)| name == local) {
        return Ok(form);
    }
    if let Some(keyword) = purrdf_sparql_algebra::builtin_function_keyword(local) {
        return Ok(SparqlCallForm::Call(keyword));
    }
    let iri = format!("{}{local}", crate::model::sparql_ns::NS);
    if local.starts_with("agg-") {
        return Err(format!(
            "<{iri}> names a SPARQL AGGREGATE, not a scalar function, and cannot be called from a \
             node expression; the SHACL aggregate node expressions are shnex:count, shnex:min, \
             shnex:max and shnex:sum"
        ));
    }
    if local == "filter-exists" || local == "filter-not-exists" {
        return Err(format!(
            "<{iri}> is a SPARQL functional form over a GRAPH PATTERN, not over argument values, \
             and cannot be called from a node expression; the SHACL existence node expression is \
             shnex:exists"
        ));
    }
    Err(format!(
        "<{iri}> is not a callable SPARQL 1.2 function name"
    ))
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
    /// A `sparql:<NAME>` call (SHACL 1.2 Node Expressions §5) — an IRI of the
    /// W3C SPARQL 1.2 term vocabulary invoked over an `rdf:List` of argument
    /// node expressions.
    ///
    /// The SPARQL surface text is resolved and rendered ONCE, at shapes-load, by
    /// [`sparql_ns_lowering`] plus [`SparqlCallForm::render`], and the rendered
    /// text is parse-checked there too — so an unknown name, an operator applied
    /// to the wrong number of operands, and a keyword call of the wrong arity are
    /// all shapes-load failures rather than per-focus evaluation surprises.
    Sparql {
        /// The `sparql:<NAME>` IRI as authored, quoted in diagnostics.
        iri: NamedNode,
        /// The rendered SPARQL expression over the `?a0 … ?aN` placeholders.
        expr: String,
        /// The argument expressions, positionally matching the placeholders.
        args: Vec<NodeExpr>,
    },
}

// ── Evaluation scope ────────────────────────────────────────────────────────────

/// One `name → node` binding frame of an evaluation [`Scope`].
///
/// A binding is a BORROWED stack frame: it holds references to a name and a term
/// the caller already owns, plus the enclosing scope. Building one is a pointer
/// write, not an allocation, so pushing a binding on the hot path costs nothing
/// beyond a stack slot.
#[derive(Debug, Clone, Copy)]
pub struct Binding<'a> {
    name: &'a str,
    value: &'a Term,
    outer: Scope<'a>,
}

impl<'a> Binding<'a> {
    /// Bind `name` to `value` on top of `outer`.
    #[must_use]
    pub const fn new(name: &'a str, value: &'a Term, outer: Scope<'a>) -> Self {
        Self { name, value, outer }
    }
}

/// The variable-binding environment a node expression is evaluated in — the
/// `scope` argument of the SHACL 1.2 Node Expressions `evalExpr(expr, focusGraph,
/// focusNode, scope)` signature.
///
/// The scope is a borrowed singly-linked list of [`Binding`] frames rather than a
/// map, because the shape of real SHACL evaluation is "empty almost everywhere,
/// one or two bindings at a constraint boundary". [`Scope::EMPTY`] is a null
/// pointer, so the overwhelmingly common empty case costs one word and no
/// allocation, and [`lookup`](Self::lookup) walks at most as many links as the
/// caller actually pushed. The type is `Copy`, so passing it down the evaluator is
/// a register move.
///
/// # Who binds what
///
/// * `sh:expression` (Node Expressions §7.1) binds `value` to the value node under
///   test, which is what makes `[ shnex:var "value" ]` resolve inside an
///   expression constraint.
/// * `sh:nodeByExpression` (§7.2) evaluates in the EMPTY scope, by the spec's own
///   `evalExpr(expr, data graph, v, {})`.
/// * `shnex:var "focusNode"` never consults the scope at all: §4.1.2 resolves that
///   name against the focus node directly, before the scope is searched.
///
/// `shnex:flatMap` (§4.3.1) and `shnex:orderBy` (§4.2.8) rebind the FOCUS NODE per
/// element rather than a scope variable, so they thread the caller's scope through
/// unchanged — the spec writes them as `evalExpr(inner, focusGraph, n, scope)`.
/// # Two key spaces, one scope
///
/// The specification writes a single `scope`, but it gives its two kinds of entry
/// different value spaces: a `shnex:var` entry's value "is an individual node"
/// (§4.1.2) while a `shnex:arg` entry's value is a NODE EXPRESSION evaluated where
/// it is read (§6.3). They are therefore carried in two fields of this one type
/// rather than one map — [`Self::lookup`] answers the first, [`Self::lookup_arg`]
/// the second, and neither can ever answer the other's question by accident.
#[derive(Debug, Clone, Copy, Default)]
pub struct Scope<'a> {
    frame: Option<&'a Binding<'a>>,
    /// The argument bindings of the innermost enclosing custom-function body
    /// (SHACL 1.2 Node Expressions §6.1/§6.2), keyed as that function keys them.
    ///
    /// A flat borrowed slice rather than a linked list, because a custom function
    /// call REPLACES the argument environment outright — `evalExpr(expr, focusGraph,
    /// focusNode, scope) -> evalExpr(body, focusGraph, focusNode, argScope)` — so
    /// there is never an outer frame to chain to.
    args: &'a [(ArgKey, NodeExpr)],
}

impl<'a> Scope<'a> {
    /// The empty scope — no variable and no argument is bound.
    pub const EMPTY: Self = Self {
        frame: None,
        args: &[],
    };

    /// The scope whose innermost frame is `binding`, inheriting `binding`'s
    /// enclosing argument environment.
    #[must_use]
    pub const fn bound(binding: &'a Binding<'a>) -> Self {
        Self {
            frame: Some(binding),
            args: binding.outer.args,
        }
    }

    /// The scope a custom function's body is evaluated in: `args` bound, and no
    /// variable bound at all.
    ///
    /// The variable frame is dropped deliberately — SHACL 1.2 Node Expressions
    /// §6.1/§6.2 evaluate the body under `argScope`, not under the caller's scope
    /// extended by it, so a `shnex:var` inside a body sees nothing the caller had.
    #[must_use]
    pub const fn with_args(args: &'a [(ArgKey, NodeExpr)]) -> Self {
        Self { frame: None, args }
    }

    /// The node expression bound to argument key `key`, or `None` when the key is
    /// not in the argument scope (§6.3's second case).
    #[must_use]
    pub fn lookup_arg(&self, key: &ArgKey) -> Option<&'a NodeExpr> {
        self.args
            .iter()
            .find(|(bound, _)| bound == key)
            .map(|(_, expr)| expr)
    }

    /// Every argument binding in force, in call order.
    ///
    /// Read by the SPARQL-based body arm, which pre-binds them as query variables —
    /// SHACL 1.2 SPARQL Extensions §7.2 writes a `sh:select` body that references
    /// `$arg0` directly. Ordinary evaluation never calls this: `shnex:arg` resolves
    /// one key through [`lookup_arg`](Self::lookup_arg).
    #[must_use]
    pub const fn args(&self) -> &'a [(ArgKey, NodeExpr)] {
        self.args
    }

    /// The node bound to `name`, innermost binding first, or `None` when the name
    /// is not in scope.
    #[must_use]
    pub fn lookup(&self, name: &str) -> Option<&'a Term> {
        let mut cursor = self.frame;
        while let Some(binding) = cursor {
            if binding.name == name {
                return Some(binding.value);
            }
            cursor = binding.outer.frame;
        }
        None
    }

    /// Whether no variable is bound.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.frame.is_none()
    }

    /// Every binding in force, innermost first, each NAME appearing exactly once.
    ///
    /// A shadowed outer binding is dropped rather than listed twice, so the result
    /// is the scope as [`lookup`](Self::lookup) sees it — which is what a caller
    /// that must materialize the whole environment (a SPARQL-based node
    /// expression pre-binding its scope variables, SHACL 1.2 SPARQL Extensions
    /// §6.1) needs. Ordinary evaluation never calls this: the linked list exists
    /// precisely so the common path resolves one name without building a map.
    #[must_use]
    pub fn bindings(&self) -> Vec<(&'a str, &'a Term)> {
        let mut out: Vec<(&'a str, &'a Term)> = Vec::new();
        let mut cursor = self.frame;
        while let Some(binding) = cursor {
            if !out.iter().any(|(name, _)| *name == binding.name) {
                out.push((binding.name, binding.value));
            }
            cursor = binding.outer.frame;
        }
        out
    }
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
///
/// A third, independent counter bounds the STRUCTURAL nesting of one expression
/// tree (every `NodeExpr` node on the path from the root), capped at
/// [`MAX_NODE_EXPR_DEPTH`]. A shape-bearing cycle is not the only way to nest
/// without bound — an authored tree of `sh:union` / `sh:if` / paging wrappers
/// reaches the native stack directly, and a Rust stack overflow ABORTS the
/// process rather than returning an error a caller could handle.
#[derive(Debug, Default)]
pub struct RecursionGuard {
    stack: FastSet<(String, String)>,
    depth: u32,
    structural: u32,
}

/// Maximum nested `sh:filterShape` / `sh:exists` re-entry depth. Legitimate
/// SHACL shapes nest only a handful of filter layers; a mutually-recursive cycle
/// grows without bound and trips this ceiling, fail-closed, well before the
/// native stack is exhausted.
pub const MAX_RECURSION_DEPTH: u32 = 64;

/// Maximum STRUCTURAL nesting depth of a single node-expression tree — one unit
/// per [`NodeExpr`] node on the path from the root to the node being evaluated.
///
/// This is a DIFFERENT quantity from [`MAX_RECURSION_DEPTH`], which counts full
/// re-entries into the constraint engine (one unit = one `sh:filterShape` /
/// `sh:exists` round trip through [`crate::constraints::conforms`]), and it
/// deliberately does not share that ceiling. The shapes parser wraps EVERY
/// authored node expression in `Limit(Offset(OrderBy(core)))` when it carries
/// paging keys — three structural levels per authored node — and set combinators
/// (`sh:union`, `sh:intersection`, `sh:if`, function-call arguments) add one
/// level each. A ceiling of 64 would therefore reject a legitimate ~16-node
/// authored expression, turning "fail closed on a cycle" into "reject a valid
/// shape". At 256 the bound admits roughly 85 fully-paged authored levels — far
/// past anything hand-authored — while still refusing an expression tree deep
/// enough to threaten the native stack, which a Rust stack overflow would end by
/// ABORTING the process rather than by returning an error.
pub const MAX_NODE_EXPR_DEPTH: u32 = 256;

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
            stack: FastSet::default(),
            depth,
            structural: 0,
        }
    }

    /// The current filter/exists re-entry depth carried by this guard.
    #[must_use]
    pub fn depth(&self) -> u32 {
        self.depth
    }

    /// Descend one structural level of the node-expression tree.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` naming [`MAX_NODE_EXPR_DEPTH`] once the tree nests
    /// past the ceiling — a hard, catchable refusal in place of the uncatchable
    /// process abort a native stack overflow would be.
    fn enter_node_expr(&mut self) -> Result<(), String> {
        if self.structural >= MAX_NODE_EXPR_DEPTH {
            return Err(format!(
                "node expression nesting depth exceeded ({} > {MAX_NODE_EXPR_DEPTH}): the \
                 expression tree is nested past the structural limit",
                self.structural.saturating_add(1)
            ));
        }
        self.structural += 1;
        Ok(())
    }

    /// Ascend one structural level, undoing an [`enter_node_expr`](Self::enter_node_expr).
    fn exit_node_expr(&mut self) {
        self.structural = self.structural.saturating_sub(1);
    }

    /// Descend into a custom node-expression function's body (SHACL 1.2 Node
    /// Expressions §6.1/§6.2), charging one unit of the SAME re-entry counter
    /// `sh:filterShape` / `sh:exists` charge.
    ///
    /// A function body is a re-entry into evaluation exactly as a filter shape is:
    /// one unit is one call boundary crossed, and a self- or mutually-recursive
    /// function grows it without bound. It deliberately does NOT get a third,
    /// separate ceiling — the quantity being bounded is the same quantity, and the
    /// existing [`MAX_RECURSION_DEPTH`] is what bounds it. The counter is also what
    /// [`crate::sparql::current_call_depth`] publishes, so a cycle that leaves this
    /// evaluator through a `sh:select` body and re-enters through a registered
    /// SPARQL function keeps counting instead of restarting at zero.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` naming [`MAX_RECURSION_DEPTH`] once the ceiling is
    /// reached — a hard, catchable refusal in place of the uncatchable process abort
    /// a native stack overflow would be.
    pub fn enter_call(&mut self, iri: &str) -> Result<(), String> {
        if self.depth >= MAX_RECURSION_DEPTH {
            return Err(format!(
                "custom node-expression function <{iri}> re-entry depth exceeded \
                 ({MAX_RECURSION_DEPTH}): the call chain is recursive and has been refused rather \
                 than allowed to exhaust the native stack"
            ));
        }
        self.depth += 1;
        Ok(())
    }

    /// Ascend out of a custom function body, undoing an [`enter_call`](Self::enter_call).
    pub fn exit_call(&mut self) {
        self.depth = self.depth.saturating_sub(1);
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

/// Lower a known XPath/XQuery-functions-namespace function IRI to its SPARQL 1.1
/// keyword.
///
/// Several SPARQL 1.1 builtins (STRLEN, CONTAINS, ABS, REGEX, …) are keyword-only:
/// they have NO IRI call form, so the `<iri>(…)` call position never resolves them
/// and a SHACL-AF `sh:expression` naming the XPath IRI would silently fall out of
/// reach. This table maps the `http://www.w3.org/2005/xpath-functions#…` IRIs to
/// the keyword the engine dispatches. Returns `None` for anything else (XSD casts
/// and purrdf custom functions keep the `<iri>(…)` form).
///
/// A static `match` over `&'static str` — wasm-clean, no runtime allocation.
fn builtin_keyword(iri: &str) -> Option<&'static str> {
    const FN: &str = "http://www.w3.org/2005/xpath-functions#";
    let local = iri.strip_prefix(FN)?;
    Some(match local {
        "string-length" => "STRLEN",
        "contains" => "CONTAINS",
        "starts-with" => "STRSTARTS",
        "ends-with" => "STRENDS",
        "substring" => "SUBSTR",
        "upper-case" => "UCASE",
        "lower-case" => "LCASE",
        // `fn:concat` and `fn:string-join` both map to SPARQL CONCAT; CONCAT is
        // variadic, so the differing arity is not a mismatch here.
        "concat" | "string-join" => "CONCAT",
        "matches" => "REGEX",
        "replace" => "REPLACE",
        "numeric-abs" => "ABS",
        "numeric-ceil" => "CEIL",
        "numeric-floor" => "FLOOR",
        "numeric-round" => "ROUND",
        "year-from-dateTime" => "YEAR",
        "month-from-dateTime" => "MONTH",
        "day-from-dateTime" => "DAY",
        "hours-from-dateTime" => "HOURS",
        "minutes-from-dateTime" => "MINUTES",
        "seconds-from-dateTime" => "SECONDS",
        _ => return None,
    })
}

// ── Evaluator ───────────────────────────────────────────────────────────────────

/// Evaluate a node expression against `store`, from `focus`.
///
/// Returns the node set the expression maps `focus` to. [`NodeExpr::Filter`]
/// re-enters the constraint engine under `guard`; a cyclic filter reference is a
/// hard `Err` (see [`RecursionGuard`]).
///
/// EVERY arm of the evaluator descends through this entry point, so the
/// structural nesting of the expression tree is bounded by
/// [`MAX_NODE_EXPR_DEPTH`] uniformly — there is no arm (`sh:union`,
/// `sh:intersection`, `sh:if`, the paging wrappers, a function call's arguments)
/// through which an over-deep tree can reach the native stack.
///
/// # Errors
///
/// Returns `Err(String)` on a recursion cycle, a filter/exists depth-limit
/// breach, a structural-depth breach, or when a sub-expression errors (e.g. an
/// unresolved function IRI). A function call over multi-valued arguments is the
/// cartesian product of the argument value-sets, not an error.
pub fn eval_node_expr(
    store: &ShaclData,
    focus: &Term,
    expr: &NodeExpr,
    guard: &mut RecursionGuard,
) -> Result<Vec<Term>, String> {
    eval_node_expr_in_scope(store, focus, expr, guard, Scope::EMPTY)
}

/// Evaluate a node expression against `store`, from `focus`, with `scope` in force.
///
/// This is the full `evalExpr(expr, focusGraph, focusNode, scope)` of the SHACL 1.2
/// Node Expressions specification. [`eval_node_expr`] is exactly this function
/// under [`Scope::EMPTY`] — the spec's own top-level entry, not a convenience
/// variant with different semantics.
///
/// # Errors
///
/// As [`eval_node_expr`].
pub fn eval_node_expr_in_scope(
    store: &ShaclData,
    focus: &Term,
    expr: &NodeExpr,
    guard: &mut RecursionGuard,
    scope: Scope<'_>,
) -> Result<Vec<Term>, String> {
    guard.enter_node_expr()?;
    // Capture, unwind one level, THEN propagate: an early `?` here would leave
    // the structural counter permanently raised for the rest of the evaluation.
    let result = eval_node_expr_at_depth(store, focus, expr, guard, scope);
    guard.exit_node_expr();
    result
}

/// One structural level of [`eval_node_expr_in_scope`], with the depth already charged.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per SHACL node-expression kind; splitting the dispatch would \
              hide the exhaustive spec-to-arm correspondence this match exists to show"
)]
fn eval_node_expr_at_depth(
    store: &ShaclData,
    focus: &Term,
    expr: &NodeExpr,
    guard: &mut RecursionGuard,
    scope: Scope<'_>,
) -> Result<Vec<Term>, String> {
    match expr {
        NodeExpr::Constant(t) => Ok(vec![t.clone()]),
        NodeExpr::This => Ok(vec![focus.clone()]),
        NodeExpr::Path(p) => {
            // Node-expression set outputs are canonicalized HERE (sort+dedup) so
            // sh:offset / sh:limit applied directly to a bare Path set are
            // deterministic. `path::eval`'s crate-wide first-seen iteration order
            // is left untouched (it is used elsewhere for path traversal).
            let mut v = path::eval(store.core(), focus, p);
            crate::term::sort_terms_canonical(&mut v);
            v.dedup();
            Ok(v)
        }
        NodeExpr::Union(exprs) => {
            let mut out: Vec<Term> = Vec::new();
            for sub in exprs {
                out.extend(eval_node_expr_in_scope(store, focus, sub, guard, scope)?);
            }
            crate::term::sort_terms_canonical(&mut out);
            out.dedup();
            Ok(out)
        }
        NodeExpr::Intersection(exprs) => {
            let mut iter = exprs.iter();
            let Some(first) = iter.next() else {
                return Ok(Vec::new());
            };
            let mut acc: FastSet<Term> =
                eval_node_expr_in_scope(store, focus, first, guard, scope)?
                    .into_iter()
                    .collect();
            // Reuse a single scratch set across operands (clear + refill) rather
            // than allocating a fresh set per iteration.
            let mut next: FastSet<Term> = FastSet::default();
            for sub in iter {
                next.clear();
                next.extend(eval_node_expr_in_scope(store, focus, sub, guard, scope)?);
                acc.retain(|t| next.contains(t));
            }
            let mut out: Vec<Term> = acc.into_iter().collect();
            crate::term::sort_terms_canonical(&mut out);
            out.dedup();
            Ok(out)
        }
        NodeExpr::If { cond, then, els } => {
            // Propagate a condition error rather than swallowing it.
            let cond_nodes = eval_node_expr_in_scope(store, focus, cond, guard, scope)?;
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
                    let ebv = crate::sparql::eval_scalar_expr_view(
                        store.sparql_view(),
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
            eval_node_expr_in_scope(store, focus, branch, guard, scope)
        }
        // Builtin and user-defined (`sh:SPARQLFunction`) calls lower identically:
        // both render an `<iri>(…)` call and route through the SPARQL seam. The only
        // difference is resolution inside the engine — a builtin IRI resolves to its
        // SPARQL function, a user-defined IRI resolves against the in-scope function
        // registry (`enter_function_scope`). A builtin whose IRI is a keyword-only
        // SPARQL 1.1 function (STRLEN, CONTAINS, ABS, REGEX, …) is lowered to that
        // keyword; a user function's IRI is never a keyword, so it keeps the call form.
        NodeExpr::Call(
            FnCall::Builtin { iri, args }
            | FnCall::UserDefined { iri, args }
            | FnCall::Sparql { iri, args, .. },
        ) => {
            // Each argument is a node expression yielding a SET of values. Per the
            // reference implementations (TopBraid / DASH / pySHACL) the function is
            // invoked once for every tuple in the CARTESIAN PRODUCT of the argument
            // value-sets, and the results are unioned. A single-valued argument is
            // the 1-element special case, so existing sh:expression validation (one
            // value per argument) is a 1×1×…×1 product — exactly one invocation,
            // unchanged. An empty argument value-set makes the product empty
            // (SHACL/SPARQL set semantics): no invocations, empty result. A zero-arg
            // call is a single empty tuple → one invocation.
            let arg_values: Vec<Vec<Term>> = args
                .iter()
                .map(|arg| eval_node_expr_in_scope(store, focus, arg, guard, scope))
                .collect::<Result<_, _>>()?;
            // A `sparql:<NAME>` call carries its rendered SPARQL text from
            // shapes-load (see `FnCall::Sparql`); the other two kinds render an
            // `<iri>(…)` call (or the keyword form, for the keyword-only builtins)
            // here.
            let expr_string = match expr {
                NodeExpr::Call(FnCall::Sparql { expr, .. }) => expr.clone(),
                _ => {
                    let placeholders: Vec<String> =
                        (0..arg_values.len()).map(|i| format!("?a{i}")).collect();
                    match builtin_keyword(iri.as_str()) {
                        Some(kw) => format!("{kw}({})", placeholders.join(", ")),
                        None => format!("<{}>({})", iri.as_str(), placeholders.join(", ")),
                    }
                }
            };
            // The product size is the product of the per-argument value-set sizes;
            // any empty set collapses it to zero (no invocations).
            let combinations = arg_values.iter().map(Vec::len).product::<usize>();
            let mut out: Vec<Term> = Vec::new();
            if combinations > 0 {
                // A non-zero product guarantees every value-set is non-empty. The
                // argument keys — and their (reverse) positions in `bindings` — are
                // invariant across combinations, so the buffer is built once and only
                // the bound `Term` is overwritten each pass: no per-combination key
                // formatting and no per-combination `Vec` allocation.
                let mut bindings: Vec<(String, Term)> = arg_values
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(i, values)| (format!("a{i}"), values[0].clone()))
                    .collect();
                for k in 0..combinations {
                    // Decode the linear index `k` into a mixed-radix tuple: the digit
                    // for argument `i` is `(k / stride) % len`, iterating the last
                    // argument fastest (row-major over the value-sets). `bindings` and
                    // `arg_values.iter().rev()` share the same last-argument-first
                    // order, so the slots line up with the reversed value-sets.
                    let mut rem = k;
                    for (slot, values) in bindings.iter_mut().zip(arg_values.iter().rev()) {
                        let digit = rem % values.len();
                        rem /= values.len();
                        slot.1 = values[digit].clone();
                    }
                    // A SPARQL error/unbound result is the correct SHACL-AF "no value"
                    // signal for that tuple — it contributes nothing, not a violation.
                    if let Some(term) = crate::sparql::eval_scalar_expr_view(
                        store.sparql_view(),
                        &expr_string,
                        &bindings,
                    )? {
                        out.push(term);
                    }
                }
            }
            // Canonicalize the unioned result (sort+dedup) like every other
            // set-shaped node-expression output.
            crate::term::sort_terms_canonical(&mut out);
            out.dedup();
            Ok(out)
        }
        NodeExpr::Distinct(of) => {
            let mut out = eval_node_expr_in_scope(store, focus, of, guard, scope)?;
            crate::term::sort_terms_canonical(&mut out);
            out.dedup();
            Ok(out)
        }
        NodeExpr::Count { distinct, of } => {
            let mut out = eval_node_expr_in_scope(store, focus, of, guard, scope)?;
            if *distinct {
                crate::term::sort_terms_canonical(&mut out);
                out.dedup();
            }
            // Element count as a canonical `xsd:integer`. `usize::to_string`
            // avoids a lossy `as` cast.
            Ok(vec![Term::Literal(Literal::new_typed_literal(
                out.len().to_string(),
                NamedNode::new_unchecked(xsd::INTEGER),
            ))])
        }
        NodeExpr::OrderBy {
            of,
            key,
            descending,
        } => {
            // Authority-grounded (W3C/DASH) semantics: `sh:orderby` names a
            // sort-key node expression, evaluated PER ELEMENT with that element
            // as focus. Elements are ordered by SPARQL ORDER BY *value* semantics
            // over their keys (numeric/typed value order — e.g.
            // "2"^^xsd:integer < "10"^^xsd:integer — NOT N-Triples lexical
            // order). Direction defaults to ascending (`descending` flips it).
            let elements = eval_node_expr_in_scope(store, focus, of, guard, scope)?;
            if elements.is_empty() {
                return Ok(Vec::new());
            }
            // Sort key per element, element-as-focus. Exactly one key term per
            // element (0 or >1 is a hard error — no optionality).
            let mut keyed: Vec<(Term, Term)> = Vec::with_capacity(elements.len());
            for e in elements {
                let ks = eval_node_expr_in_scope(store, &e, key, guard, scope)?;
                let [k] = ks.as_slice() else {
                    return Err(format!(
                        "sh:orderby key must yield exactly one value per node, got {} for {e}",
                        ks.len()
                    ));
                };
                keyed.push((e, k.clone()));
            }
            // Value-order the DISTINCT keys via the SPARQL engine (reuse
            // `eval_order`), build a rank map, then sort elements by (rank,
            // canonical term string) so value-equal keys still yield a
            // byte-stable total order (tie-break).
            let mut distinct: Vec<Term> = keyed.iter().map(|(_, k)| k.clone()).collect();
            crate::term::sort_terms_canonical(&mut distinct);
            distinct.dedup();
            let ranked = crate::sparql::eval_order_view(store.sparql_view(), &distinct, false)?;
            let mut rank: FastMap<String, usize> = FastMap::default();
            for (i, k) in ranked.iter().enumerate() {
                rank.insert(k.to_string(), i);
            }
            // Precompute the sort keys once per element (rank + canonical element
            // string) so the comparator does no per-comparison allocation/lookup.
            let mut out: Vec<(Term, usize, String)> = keyed
                .into_iter()
                .map(|(e, k)| {
                    let r = rank.get(&k.to_string()).copied().unwrap_or(usize::MAX);
                    let es = e.to_string();
                    (e, r, es)
                })
                .collect();
            out.sort_by(|a, b| {
                let primary = if *descending {
                    b.1.cmp(&a.1)
                } else {
                    a.1.cmp(&b.1)
                };
                // Total-order tie-break, always ascending by canonical term string.
                primary.then_with(|| a.2.cmp(&b.2))
            });
            Ok(out.into_iter().map(|(e, _, _)| e).collect())
        }
        NodeExpr::Offset { of, n } => {
            let out = eval_node_expr_in_scope(store, focus, of, guard, scope)?;
            let skip =
                usize::try_from(*n).map_err(|e| format!("sh:offset value too large: {e}"))?;
            // Ordering is the caller's responsibility (an OrderBy wrapper) — apply
            // the offset to the already-produced sequence. The parser nests these
            // as `Limit(Offset(OrderBy(core)))`, so evaluation composes naturally:
            // OrderBy runs first, then Offset skips, then Limit truncates.
            Ok(out.into_iter().skip(skip).collect())
        }
        NodeExpr::Limit { of, n } => {
            let out = eval_node_expr_in_scope(store, focus, of, guard, scope)?;
            let take = usize::try_from(*n).map_err(|e| format!("sh:limit value too large: {e}"))?;
            Ok(out.into_iter().take(take).collect())
        }
        // ── SHACL 1.2 Node Expressions ─────────────────────────────────────────
        // §4.1.1 Empty expression: the empty list, always.
        NodeExpr::Empty => Ok(Vec::new()),
        // §4.1.2 Var expression, in the spec's stated order: `"focusNode"` names
        // the focus node; any other name resolves against the scope; an unbound
        // name yields the empty list (an absence, not an error).
        NodeExpr::Var(name) => Ok(match name.as_str() {
            FOCUS_NODE_VAR => vec![focus.clone()],
            other => scope.lookup(other).cloned().into_iter().collect(),
        }),
        // §4.1.3 List expression: the members, IN LIST ORDER. Sequence-valued —
        // deliberately NOT sorted and NOT deduplicated.
        NodeExpr::List(members) => Ok(members.clone()),
        // §4.1.4 Path values expression WITH an explicit `shnex:focusNode`: the
        // focus expression must yield at most one node (0 ⇒ empty, >1 ⇒ a hard
        // evaluation failure), and the path is walked from that node.
        NodeExpr::PathValues {
            path: walk,
            focus: from,
        } => {
            let starts = eval_node_expr_in_scope(store, focus, from, guard, scope)?;
            match starts.as_slice() {
                [] => Ok(Vec::new()),
                [start] => {
                    let mut v = path::eval(store.core(), start, walk);
                    crate::term::sort_terms_canonical(&mut v);
                    v.dedup();
                    Ok(v)
                }
                more => Err(format!(
                    "shnex:focusNode must yield at most one node, got {}",
                    more.len()
                )),
            }
        }
        // §4.2.3 Concat expression: operand outputs concatenated left to right.
        // Sequence-valued — order preserved, duplicates preserved.
        NodeExpr::Concat(operands) => {
            let mut out: Vec<Term> = Vec::new();
            for operand in operands {
                out.extend(eval_node_expr_in_scope(
                    store, focus, operand, guard, scope,
                )?);
            }
            Ok(out)
        }
        // §4.2.4 Remove expression: N minus M by TERM equality, preserving N's
        // order. `"01"^^xsd:integer` therefore does not remove `"1"^^xsd:integer`.
        NodeExpr::Remove { nodes, remove } => {
            let keep = eval_node_expr_in_scope(store, focus, nodes, guard, scope)?;
            let drop: FastSet<Term> = eval_node_expr_in_scope(store, focus, remove, guard, scope)?
                .into_iter()
                .collect();
            Ok(keep.into_iter().filter(|t| !drop.contains(t)).collect())
        }
        // §4.3.1 FlatMap expression: `map` evaluated once per input node WITH THAT
        // NODE AS FOCUS, results concatenated in input order. The scope threads
        // through unchanged — the spec rebinds the focus node here, not a variable.
        NodeExpr::FlatMap { nodes, map } => {
            let inputs = eval_node_expr_in_scope(store, focus, nodes, guard, scope)?;
            let mut out: Vec<Term> = Vec::new();
            for node in &inputs {
                out.extend(eval_node_expr_in_scope(store, node, map, guard, scope)?);
            }
            Ok(out)
        }
        // §4.3.2 FindFirst expression: the FIRST input node conforming to `shape`.
        // Short-circuits, so a long candidate list costs only as many conformance
        // checks as it takes to hit one.
        NodeExpr::FindFirst { nodes, shape } => {
            let inputs = eval_node_expr_in_scope(store, focus, nodes, guard, scope)?;
            for node in inputs {
                if conforms_guarded(store, &node, shape, guard)? {
                    return Ok(vec![node]);
                }
            }
            Ok(Vec::new())
        }
        // §4.3.3 MatchAll expression: true iff EVERY input node conforms. An empty
        // input is vacuously true ("every node in N conforms" over an empty N).
        NodeExpr::MatchAll { nodes, shape } => {
            let inputs = eval_node_expr_in_scope(store, focus, nodes, guard, scope)?;
            for node in inputs {
                if !conforms_guarded(store, &node, shape, guard)? {
                    return Ok(vec![bool_literal(false)]);
                }
            }
            Ok(vec![bool_literal(true)])
        }
        // §4.5.1 InstancesOf expression: the SHACL instances of the class in the
        // focus graph — which, per the spec's own note, INCLUDES instances of its
        // subclasses. The shared class-membership view already answers exactly that
        // question (it is what `sh:class` and `sh:targetClass` consult), so this
        // reuses it rather than walking `rdfs:subClassOf` a second time.
        NodeExpr::InstancesOf(class) => {
            store.prepare_class_membership();
            let class_term = Term::NamedNode(class.clone());
            let Some(class_id) = crate::data::resolve_id(store.core(), &class_term) else {
                // A class IRI absent from the data graph has no instances. That is
                // an empty answer, not a malformed expression.
                return Ok(Vec::new());
            };
            let mut out: Vec<Term> = store
                .class_view()
                .instances_of(class_id)
                .map(|id| crate::term::term_id_to_native(store.core(), id))
                .collect();
            crate::term::sort_terms_canonical(&mut out);
            out.dedup();
            Ok(out)
        }
        // §4.5.2 NodesMatching expression: every node of the focus graph that
        // conforms to `shape`. The spec itself warns this output "may be very
        // large"; the candidate set is every subject and object of the graph,
        // canonicalized before the conformance sweep so the result is deterministic
        // and each distinct node is checked exactly once.
        NodeExpr::NodesMatching(shape) => {
            let mut candidates: Vec<Term> = Vec::new();
            for (subject, _, object) in crate::data::native_quads(
                store.core(),
                None,
                None,
                None,
                crate::data::GraphFilter::AnyGraph,
            ) {
                candidates.push(subject);
                candidates.push(object);
            }
            crate::term::sort_terms_canonical(&mut candidates);
            candidates.dedup();
            let mut out: Vec<Term> = Vec::new();
            for node in candidates {
                if conforms_guarded(store, &node, shape, guard)? {
                    out.push(node);
                }
            }
            Ok(out)
        }
        // §4.5.3 ConformsToShape expression: a list parameter function, so the node
        // argument must produce at most one node. No node ⇒ the empty list (the
        // spec's stated "no value" case); otherwise the boolean conformance answer.
        NodeExpr::ConformsToShape { node, shape } => {
            let candidates = eval_node_expr_in_scope(store, focus, node, guard, scope)?;
            match candidates.as_slice() {
                [] => Ok(Vec::new()),
                [only] => {
                    // The shape argument is resolved the way it was written: a
                    // NAMED one was parsed at load, a COMPUTED one is evaluated
                    // here against the shapes graph's own shape index.
                    match shape {
                        ShapeArg::Named(shape) => Ok(vec![bool_literal(conforms_guarded(
                            store, only, shape, guard,
                        )?)]),
                        ShapeArg::Computed { expr, shapes } => {
                            let produced =
                                eval_node_expr_in_scope(store, focus, expr, guard, scope)?;
                            let [shape_iri] = produced.as_slice() else {
                                return Err(format!(
                                    "shnex:conformsToShape shape argument must produce exactly \
                                     one shape IRI, got {}",
                                    produced.len()
                                ));
                            };
                            let index = shapes.get().ok_or_else(|| {
                                "shnex:conformsToShape: the shapes graph's shape index was never \
                                 filled"
                                    .to_owned()
                            })?;
                            let shape = index.get(&shape_iri.to_string()).ok_or_else(|| {
                                format!(
                                    "shnex:conformsToShape shape argument produced {shape_iri}, \
                                     which is not a shape of this shapes graph"
                                )
                            })?;
                            Ok(vec![bool_literal(conforms_guarded(
                                store, only, shape, guard,
                            )?)])
                        }
                    }
                }
                more => Err(format!(
                    "shnex:conformsToShape node argument must yield at most one node, got {}",
                    more.len()
                )),
            }
        }
        // SHACL 1.2 SPARQL Extensions §6.1 (`sh:select`) / §6.2 (`sh:sparqlExpr`),
        // whose evaluation clauses are word-for-word the same: run the query
        // against the focus graph "with focusNode pre-bound to variable $this and
        // scope variables pre-bound with matching names", and take the bindings of
        // the single projected variable as the output nodes.
        NodeExpr::Select {
            query,
            variable,
            key,
        } => {
            // "Failure produced if scope contains variable named `this`" — the
            // pre-bound focus node would otherwise be silently clobbered by a
            // scope binding, so this is a hard refusal, not a precedence rule.
            let mut bindings: Vec<(String, Term)> = Vec::with_capacity(1 + scope.bindings().len());
            bindings.push(("this".to_owned(), focus.clone()));
            for (name, value) in scope.bindings() {
                if name == "this" {
                    return Err(format!(
                        "{key} node expression cannot be evaluated: the scope binds a variable \
                         named ?this, which would clobber the pre-bound focus node"
                    ));
                }
                bindings.push((name.to_owned(), value.clone()));
            }
            // SHACL 1.2 SPARQL Extensions §7.2 writes a custom function's `sh:select`
            // / `sh:sparqlExpr` body that references its arguments as `$arg0`,
            // `$arg1`, … directly, so the argument scope is pre-bound here under
            // those names (and, for a named-parameter function, under each
            // parameter's local name). The argument is a node EXPRESSION, evaluated
            // where it is read exactly as `shnex:arg` evaluates it.
            //
            // A variable can carry one term: an argument that produces none is left
            // UNBOUND (an absence the query itself decides what to do with), and one
            // that produces several is a hard refusal naming the variable — binding
            // an arbitrary member, or dropping the binding silently, would both hand
            // the query a different answer than the author wrote.
            for (arg_key, arg_expr) in scope.args() {
                let name = arg_key.variable_name();
                let values = eval_node_expr_in_scope(store, focus, arg_expr, guard, Scope::EMPTY)?;
                match values.as_slice() {
                    [] => {}
                    [only] => bindings.push((name, only.clone())),
                    more => {
                        return Err(format!(
                            "{key} node expression cannot pre-bind ?{name}: the argument produces \
                             {} nodes and a query variable carries one",
                            more.len()
                        ));
                    }
                }
            }
            crate::sparql::eval_select_nodes_view(store.sparql_view(), query, variable, &bindings)
                .map_err(|e| format!("{key} node expression: {e}"))
        }
        // §6.3 Arg expression: look the key up in the argument scope and evaluate
        // the bound NODE EXPRESSION there, in the empty scope —
        // `evalExpr(a, focusGraph, focusNode, {})`. An unbound key is the spec's
        // own second case and yields the empty list.
        NodeExpr::Arg(key) => match scope.lookup_arg(key) {
            None => Ok(Vec::new()),
            Some(arg) => {
                let out = eval_node_expr_in_scope(store, focus, arg, guard, Scope::EMPTY)?;
                // §6.2 says of a custom LIST parameter function that "each argument
                // produces at most one output node", and that "an evaluation failure
                // occurs if any output produces more than one node". That restriction
                // belongs to the indexed key space alone: §6.1's own example passes a
                // multi-valued `shnex:pathValues` to a NAMED parameter and sums it, so
                // a named argument is deliberately unrestricted here.
                if matches!(key, ArgKey::Index(_)) && out.len() > 1 {
                    return Err(format!(
                        "shnex:arg {key} names a list-parameter argument, which must produce at \
                         most one node, got {}",
                        out.len()
                    ));
                }
                Ok(out)
            }
        },
        // §6.1 / §6.2 Custom node-expression function call: evaluate the declared
        // body with the argument scope in force —
        // `evalExpr(expr, focusGraph, focusNode, scope) ->
        //  evalExpr(body, focusGraph, focusNode, argScope)`.
        //
        // The focus node and the focus graph both carry through unchanged; only the
        // scope is replaced. The re-entry counter is charged and released on EVERY
        // path, so an erroring body never leaves the guard permanently raised.
        NodeExpr::CustomCall { func, args } => {
            let body = func.body()?;
            guard.enter_call(func.iri.as_str())?;
            // Publish the depth for the duration of the body, so a `sh:select` inside
            // it starts its query at this depth: a cycle that leaves this evaluator
            // through SPARQL and re-enters through the §7.3 registration would
            // otherwise restart the count at zero and never terminate.
            let result = {
                let _depth = crate::sparql::enter_call_depth_scope(guard.depth());
                eval_node_expr_in_scope(store, focus, body, guard, Scope::with_args(args))
            };
            guard.exit_call();
            result
        }
        NodeExpr::Min(of) => aggregate(store, focus, of, "MIN", guard, scope),
        NodeExpr::Max(of) => aggregate(store, focus, of, "MAX", guard, scope),
        NodeExpr::Sum(of) => aggregate(store, focus, of, "SUM", guard, scope),
        NodeExpr::Filter { nodes, shape } => {
            // Candidate nodes retained iff they conform to `shape`. The re-entry
            // into `conforms` is a fresh guard/subtree, so we (a) guard the
            // in-flight `(shape id, candidate)` pair against same-tree re-entry
            // and (b) thread the monotone depth across the constraint boundary so
            // a cross-shape filter cycle fails closed (depth ceiling) rather than
            // overflowing the stack.
            let candidates = eval_node_expr_in_scope(store, focus, nodes, guard, scope)?;
            let mut kept: Vec<Term> = Vec::new();
            for value in candidates {
                if conforms_guarded(store, &value, shape, guard)? {
                    kept.push(value);
                }
            }
            // Canonicalize the node-expression set output here (sort+dedup) so
            // sh:offset / sh:limit over a bare Filter set are deterministic
            // rather than store-iteration-order dependent.
            //
            // This is the SHACL-AF set reading of the kind, and it is what both
            // spellings get: `sh:filterShape` and `shnex:filterShape` share one
            // arm and one evaluator, and the frozen `sh:`-written corpus pins the
            // canonicalized answer. SHACL 1.2 Node Expressions §4.2.5 instead says
            // "preserving the order in the list", so a filter placed directly over
            // a sequence-valued input (`shnex:concat`, `shnex:flatMap`) is sorted
            // and deduplicated here where that section would have kept the
            // sequence. Every other sequence-valued kind does preserve its order;
            // only this one is set-shaped, and deliberately so.
            crate::term::sort_terms_canonical(&mut kept);
            kept.dedup();
            Ok(kept)
        }
        NodeExpr::Exists(inner) => {
            // `sh:exists` is a node-expression predicate: true iff `inner`
            // produces at least one node for the focus. A nested Filter inside
            // `inner` re-enters the guarded constraint engine itself.
            let out = eval_node_expr_in_scope(store, focus, inner, guard, scope)?;
            Ok(vec![bool_literal(!out.is_empty())])
        }
    }
}

/// Invoke a custom LIST parameter function from a SPARQL query — SHACL 1.2 SPARQL
/// Extensions §7.3 "Evaluation of Custom SPARQL Functions".
///
/// `args` are the ALREADY-EVALUATED argument values the SPARQL engine computed, in
/// call order. §7.3 keys the scope by argument index, so they are bound to
/// [`ArgKey::Index`] `0…n-1`; and it fixes the focus node explicitly — "During
/// SPARQL query evaluation there is no dedicated focus node. Instead, the
/// `focusNode` passed into a custom SPARQL function based on a node expression is
/// the IRI of the function itself" — so the body is evaluated with the function's
/// own IRI as focus.
///
/// The return follows §7.3's own two-case rule: "If the list of output nodes `rs`
/// has exactly one member, then return that node." A body producing NO node has no
/// value to give, which is `Ok(None)` — SPARQL's unbound expression result, the same
/// signal every other value-less call in this crate returns. A body producing MORE
/// than one node is a hard error: the specification refuses to pick one, and so does
/// this, rather than returning a value it was not told to return.
///
/// # Errors
///
/// Returns `Err(String)` when the body was never installed, when the re-entry depth
/// bound is reached, when the body's evaluation fails, or when the body produces
/// more than one output node.
pub fn eval_custom_function_call(
    store: &ShaclData,
    func: &Arc<CustomFunction>,
    args: &[Term],
    guard: &mut RecursionGuard,
) -> Result<Option<Term>, String> {
    let mut bound: Vec<(ArgKey, NodeExpr)> = Vec::with_capacity(args.len());
    for (index, value) in args.iter().enumerate() {
        let key = u64::try_from(index)
            .map_err(|e| format!("argument index {index} is not representable: {e}"))?;
        bound.push((ArgKey::Index(key), NodeExpr::Constant(value.clone())));
    }
    let call = NodeExpr::CustomCall {
        func: Arc::clone(func),
        args: bound,
    };
    let focus = Term::NamedNode(func.iri.clone());
    let out = eval_node_expr(store, &focus, &call, guard)?;
    match out.as_slice() {
        [] => Ok(None),
        [only] => Ok(Some(only.clone())),
        more => Err(format!(
            "custom SPARQL function <{}> produced {} output nodes; SHACL 1.2 SPARQL Extensions \
             §7.3 returns a value only when the body produces exactly one",
            func.iri.as_str(),
            more.len()
        )),
    }
}

/// Conformance-check `node` against `shape`, under the shared recursion guard.
///
/// Every shape-bearing node-expression kind — `sh:filterShape` /
/// `shnex:filterShape` (§4.2.5), `shnex:findFirst` (§4.3.2), `shnex:matchAll`
/// (§4.3.3), `shnex:nodesMatching` (§4.5.2) and `shnex:conformsToShape` (§4.5.3) —
/// re-enters the constraint engine, and each such re-entry builds a FRESH guard
/// inside [`crate::constraints::conforms_with_depth`]. So the two layers are
/// applied here, once, for all of them: the in-flight `(shape id, node)` pair
/// catches a cycle within one expression tree, and the monotone depth carries the
/// re-entry count across the constraint boundary so a mutually-recursive
/// shape/expression cycle fails closed at [`MAX_RECURSION_DEPTH`] instead of
/// overflowing the native stack (which would ABORT rather than return).
///
/// The guard is exited BEFORE the verdict is propagated, so an erroring
/// sub-validation never leaves a stale in-flight entry behind.
fn conforms_guarded(
    store: &ShaclData,
    node: &Term,
    shape: &Shape,
    guard: &mut RecursionGuard,
) -> Result<bool, String> {
    let shape_id = shape.id.to_string();
    let node_key = node.to_string();
    let next_depth = guard.depth().saturating_add(1);
    guard.enter(&shape_id, &node_key)?;
    let verdict = crate::constraints::conforms_with_depth(store, node, shape, next_depth);
    guard.exit(&shape_id, &node_key);
    verdict
}

/// Evaluate a set aggregate (`"MIN"`/`"MAX"`/`"SUM"`) over `of`'s result via the
/// single SPARQL path ([`crate::sparql::eval_aggregate`]).
///
/// The operands are evaluated first, then delegated to the SPARQL engine so
/// numeric type-promotion and ordering match the engine exactly (there is no
/// parallel Rust numeric fold). `SUM` of an empty set is `0`^^`xsd:integer`;
/// `MIN`/`MAX` of an empty set is unbound → an empty node set.
fn aggregate(
    store: &ShaclData,
    focus: &Term,
    of: &NodeExpr,
    agg: &str,
    guard: &mut RecursionGuard,
    scope: Scope<'_>,
) -> Result<Vec<Term>, String> {
    let operands = eval_node_expr_in_scope(store, focus, of, guard, scope)?;
    match crate::sparql::eval_aggregate_view(store.sparql_view(), agg, &operands)? {
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

    /// A frozen test data graph plus a [`ShaclData`] view over it.
    struct TestData {
        ds: Arc<RdfDataset>,
    }

    impl TestData {
        fn data(&self) -> ShaclData {
            ShaclData::new(Arc::clone(&self.ds), Arc::clone(&self.ds), None)
        }
    }

    /// Load a tiny data graph from Turtle.
    fn load_data(ttl: &str) -> TestData {
        let ds: Arc<RdfDataset> =
            crate::text_ingest::parse_turtle_to_dataset(ttl, None).expect("turtle parse");
        TestData { ds }
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
        let result =
            eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("constant evals");
        assert_eq!(result, vec![ex("z")]);
    }

    #[test]
    fn this_returns_the_focus() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let result = eval_node_expr(&data.data(), &ex("a"), &NodeExpr::This, &mut guard)
            .expect("this evals");
        assert_eq!(result, vec![ex("a")]);
    }

    #[test]
    fn path_returns_value_nodes() {
        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Path(pred("p"));
        // The Path arm canonicalizes (sort+dedup) locally, so the result is
        // returned already sorted — no manual sort needed.
        let result = eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("path evals");
        assert_eq!(result, vec![ex("b"), ex("c")]);
        let sorted = {
            let mut v = result.clone();
            crate::term::sort_terms_canonical(&mut v);
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
            rules: vec![],
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
        let result =
            eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("filter evals");
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
        let result =
            eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("union evals");
        assert_eq!(result, vec![ex("b"), ex("c")]);
        // Explicitly assert deterministic (sorted) order.
        let sorted = {
            let mut v = result.clone();
            crate::term::sort_terms_canonical(&mut v);
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
            eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("intersection evals");
        assert_eq!(result, vec![ex("b")]);
    }

    #[test]
    fn intersection_empty_operands_is_empty() {
        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Intersection(vec![]);
        let result = eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard)
            .expect("empty intersection evals");
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
        let result = eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("if evals");
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
        let result = eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("if evals");
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
        let result = eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("if evals");
        assert_eq!(result, vec![ex("no")]);
    }

    #[test]
    fn if_propagates_condition_error() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        // A hard-erroring condition (an unresolved user-defined function, with no
        // registry in scope) must surface its error rather than being swallowed.
        let expr = NodeExpr::If {
            cond: Box::new(NodeExpr::Call(FnCall::UserDefined {
                iri: NamedNode::new_unchecked("http://example.org/ns#myFn"),
                args: vec![],
            })),
            then: Box::new(NodeExpr::Constant(ex("yes"))),
            els: Box::new(NodeExpr::Constant(ex("no"))),
        };
        let err = eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).unwrap_err();
        assert!(err.contains("myFn"), "got: {err}");
    }

    #[test]
    fn exists_true_when_inner_yields_nodes() {
        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        // ex:a has ex:p values → exists true.
        let expr = NodeExpr::Exists(Box::new(NodeExpr::Path(pred("p"))));
        let result =
            eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("exists evals");
        assert_eq!(result, vec![bool_literal(true)]);
    }

    #[test]
    fn exists_false_when_inner_empty() {
        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        // ex:a has no ex:missing edge → exists false.
        let expr = NodeExpr::Exists(Box::new(NodeExpr::Path(pred("missing"))));
        let result =
            eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("exists evals");
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
        let result =
            eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("builtin evals");
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
        let err = eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).unwrap_err();
        assert!(err.contains("custom SPARQL function"), "got: {err}");
    }

    /// A multi-valued function-call argument is the CARTESIAN PRODUCT of the
    /// argument value-sets (TopBraid / DASH / pySHACL semantics), not an error:
    /// `xsd:string(?x)` over `?x ∈ {ex:b, ex:c}` yields both string casts, unioned.
    #[test]
    fn builtin_call_multi_valued_arg_is_cartesian_product() {
        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        // ex:a ex:p reaches {b, c} — two terms — so the single-arg cast is invoked
        // once per value and the results are unioned (sorted, deduped).
        let expr = NodeExpr::Call(FnCall::Builtin {
            iri: NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#string"),
            args: vec![NodeExpr::Path(pred("p"))],
        });
        let result = eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard)
            .expect("multi-valued arg yields a product");
        let str_lit = |v: &str| {
            Term::Literal(Literal::new_typed_literal(
                v,
                NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#string"),
            ))
        };
        assert_eq!(
            result,
            vec![
                str_lit("http://example.org/ns#b"),
                str_lit("http://example.org/ns#c"),
            ]
        );
    }

    /// Two multi-valued arguments produce the full |A|×|B| product of invocations,
    /// unioned: `CONCAT` over {"1","2"} × {"a","b"} yields all four combinations.
    #[test]
    fn builtin_call_two_multi_valued_args_product() {
        let data = load_data(
            r#"
            @prefix ex: <http://example.org/ns#> .
            ex:x ex:l "1", "2" .
            ex:x ex:r "a", "b" .
        "#,
        );
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Call(FnCall::Builtin {
            iri: NamedNode::new_unchecked("http://www.w3.org/2005/xpath-functions#concat"),
            args: vec![NodeExpr::Path(pred("l")), NodeExpr::Path(pred("r"))],
        });
        let result = eval_node_expr(&data.data(), &ex("x"), &expr, &mut guard)
            .expect("two-arg product evals");
        let s = |v: &str| Term::Literal(Literal::new_simple_literal(v));
        assert_eq!(result, vec![s("1a"), s("1b"), s("2a"), s("2b")]);
    }

    /// An empty argument value-set collapses the product to empty (SHACL/SPARQL
    /// set semantics): no invocations, empty result — never an error.
    #[test]
    fn builtin_call_empty_arg_yields_empty_product() {
        let data = load_data(DATA);
        let mut guard = RecursionGuard::new();
        // ex:a has no ex:missing edge → empty value-set → empty product.
        let expr = NodeExpr::Call(FnCall::Builtin {
            iri: NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#string"),
            args: vec![NodeExpr::Path(pred("missing"))],
        });
        let result =
            eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("empty product evals");
        assert!(result.is_empty(), "empty arg ⇒ empty product");
    }

    /// A keyword-only SPARQL builtin named by its XPath-functions-namespace IRI
    /// must now dispatch end-to-end: `fn:string-length("hello")` → `5`^^xsd:integer.
    #[test]
    fn builtin_keyword_string_length_dispatches() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Call(FnCall::Builtin {
            iri: NamedNode::new_unchecked("http://www.w3.org/2005/xpath-functions#string-length"),
            args: vec![NodeExpr::Constant(Term::Literal(
                Literal::new_simple_literal("hello"),
            ))],
        });
        let result =
            eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("STRLEN evals");
        assert_eq!(result, vec![int_lit("5")]);
    }

    /// `fn:contains(str, substr)` lowers to SPARQL CONTAINS and yields a boolean.
    #[test]
    fn builtin_keyword_contains_dispatches() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let call = |s: &str, sub: &str| {
            NodeExpr::Call(FnCall::Builtin {
                iri: NamedNode::new_unchecked("http://www.w3.org/2005/xpath-functions#contains"),
                args: vec![
                    NodeExpr::Constant(Term::Literal(Literal::new_simple_literal(s))),
                    NodeExpr::Constant(Term::Literal(Literal::new_simple_literal(sub))),
                ],
            })
        };
        let yes = eval_node_expr(&data.data(), &ex("a"), &call("banana", "a"), &mut guard)
            .expect("CONTAINS evals");
        assert_eq!(yes, vec![bool_literal(true)]);
        let no = eval_node_expr(&data.data(), &ex("a"), &call("banana", "z"), &mut guard)
            .expect("CONTAINS evals");
        assert_eq!(no, vec![bool_literal(false)]);
    }

    /// Every mapped keyword must lower to a form the SPARQL engine accepts (no
    /// hard seam error). Each is exercised with well-typed arguments; the guard is
    /// that `eval_node_expr` returns `Ok` (a value or an empty set), never `Err`.
    #[test]
    fn builtin_keyword_table_all_dispatch() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let s = |v: &str| NodeExpr::Constant(Term::Literal(Literal::new_simple_literal(v)));
        let i = |v: &str| NodeExpr::Constant(int_lit(v));
        let dt = |v: &str| {
            NodeExpr::Constant(Term::Literal(Literal::new_typed_literal(
                v,
                NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#dateTime"),
            )))
        };
        let fnn = |local: &str| {
            NamedNode::new_unchecked(format!("http://www.w3.org/2005/xpath-functions#{local}"))
        };
        let dtv = "2026-07-03T12:34:56";
        let cases: Vec<(&str, Vec<NodeExpr>)> = vec![
            ("string-length", vec![s("hello")]),
            ("contains", vec![s("banana"), s("a")]),
            ("starts-with", vec![s("banana"), s("ba")]),
            ("ends-with", vec![s("banana"), s("na")]),
            ("substring", vec![s("banana"), i("2")]),
            ("upper-case", vec![s("abc")]),
            ("lower-case", vec![s("ABC")]),
            ("concat", vec![s("a"), s("b")]),
            ("string-join", vec![s("a"), s("b")]),
            ("matches", vec![s("banana"), s("an+")]),
            ("replace", vec![s("banana"), s("a"), s("o")]),
            ("numeric-abs", vec![i("-5")]),
            ("numeric-ceil", vec![i("5")]),
            ("numeric-floor", vec![i("5")]),
            ("numeric-round", vec![i("5")]),
            ("year-from-dateTime", vec![dt(dtv)]),
            ("month-from-dateTime", vec![dt(dtv)]),
            ("day-from-dateTime", vec![dt(dtv)]),
            ("hours-from-dateTime", vec![dt(dtv)]),
            ("minutes-from-dateTime", vec![dt(dtv)]),
            ("seconds-from-dateTime", vec![dt(dtv)]),
        ];
        for (local, args) in cases {
            let expr = NodeExpr::Call(FnCall::Builtin {
                iri: fnn(local),
                args,
            });
            let out = eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard);
            assert!(out.is_ok(), "fn:{local} must dispatch, got: {out:?}");
            assert_eq!(
                out.unwrap().len(),
                1,
                "fn:{local} must yield exactly one value"
            );
        }
    }

    #[test]
    fn user_defined_call_dispatches_against_the_registry() {
        use purrdf_sparql_eval::{
            TypeConstraint, UserFnBody, UserFnParam, UserFunction, UserFunctionRegistry,
        };

        // A `double(?x) = ?x * 2` SPARQL function declared in the (in-scope) registry.
        let mut registry = UserFunctionRegistry::new();
        registry.insert(
            "http://example.org/ns#double",
            UserFunction {
                params: vec![UserFnParam {
                    var: "x".to_owned(),
                    constraint: TypeConstraint::default(),
                }],
                required: 1,
                body: Arc::new(
                    purrdf_sparql_algebra::SparqlParser::new()
                        .parse_query("SELECT ((?x * 2) AS ?result) WHERE {}")
                        .expect("body parses"),
                ),
                kind: UserFnBody::Select,
                return_constraint: TypeConstraint::default(),
            },
        );
        let _scope = crate::sparql::enter_function_scope(Arc::new(registry));

        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Call(FnCall::UserDefined {
            iri: NamedNode::new_unchecked("http://example.org/ns#double"),
            args: vec![NodeExpr::Constant(Term::Literal(
                Literal::new_typed_literal(
                    "21",
                    NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#integer"),
                ),
            ))],
        });
        let out =
            eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("user fn dispatches");
        assert_eq!(
            out,
            vec![Term::Literal(Literal::new_typed_literal(
                "42",
                NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#integer"),
            ))]
        );
    }

    #[test]
    fn user_defined_call_to_unknown_function_errors() {
        // No function scope installed → an unknown call-position IRI is a hard error,
        // not a silent empty result.
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Call(FnCall::UserDefined {
            iri: NamedNode::new_unchecked("http://example.org/ns#missing"),
            args: vec![],
        });
        let err = eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).unwrap_err();
        assert!(err.contains("missing"), "got: {err}");
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
        let result = eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("if evals");
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
        let result = eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).expect("if evals");
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
        let err = eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard).unwrap_err();
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
        let result =
            eval_node_expr(&data.data(), &ex("x"), &expr, &mut guard).expect("distinct evals");
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
        let result =
            eval_node_expr(&data.data(), &ex("x"), &expr, &mut guard).expect("count evals");
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
        let result = eval_node_expr(&data.data(), &ex("x"), &expr, &mut guard)
            .expect("distinct count evals");
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
        let result =
            eval_node_expr(&data.data(), &ex("x"), &expr, &mut guard).expect("count evals");
        assert_eq!(result, vec![int_lit("0")]);
    }

    #[test]
    fn min_max_sum_over_integers() {
        let data = load_data(AGG_DATA);
        let mut guard = RecursionGuard::new();
        let path = || Box::new(NodeExpr::Path(pred("n")));

        let min = eval_node_expr(&data.data(), &ex("x"), &NodeExpr::Min(path()), &mut guard)
            .expect("min evals");
        assert_eq!(min, vec![int_lit("1")]);
        let max = eval_node_expr(&data.data(), &ex("x"), &NodeExpr::Max(path()), &mut guard)
            .expect("max evals");
        assert_eq!(max, vec![int_lit("3")]);
        let sum = eval_node_expr(&data.data(), &ex("x"), &NodeExpr::Sum(path()), &mut guard)
            .expect("sum evals");
        assert_eq!(sum, vec![int_lit("6")]);
    }

    #[test]
    fn sum_of_empty_is_zero_min_max_of_empty_is_unbound() {
        let data = load_data(AGG_DATA);
        let mut guard = RecursionGuard::new();
        let empty = || Box::new(NodeExpr::Path(pred("missing")));

        let sum = eval_node_expr(&data.data(), &ex("x"), &NodeExpr::Sum(empty()), &mut guard)
            .expect("sum evals");
        assert_eq!(sum, vec![int_lit("0")]);
        let min = eval_node_expr(&data.data(), &ex("x"), &NodeExpr::Min(empty()), &mut guard)
            .expect("min evals");
        assert!(min.is_empty(), "min of empty is unbound");
        let max = eval_node_expr(&data.data(), &ex("x"), &NodeExpr::Max(empty()), &mut guard)
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
        let result = eval_node_expr(&data.data(), &ex("x"), &expr, &mut guard).expect("sum evals");
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
            key: Box::new(NodeExpr::This),
            descending: false,
        };
        let result =
            eval_node_expr(&data.data(), &ex("x"), &asc, &mut guard).expect("orderby evals");
        assert_eq!(result, vec![ex("a"), ex("b"), ex("c")]);

        let desc = NodeExpr::OrderBy {
            of: Box::new(NodeExpr::Path(pred("e"))),
            key: Box::new(NodeExpr::This),
            descending: true,
        };
        let result =
            eval_node_expr(&data.data(), &ex("x"), &desc, &mut guard).expect("orderby evals");
        assert_eq!(result, vec![ex("c"), ex("b"), ex("a")]);
    }

    #[test]
    fn orderby_ties_break_by_canonical_term_string() {
        // Two DISTINCT elements (ex:a, ex:b) share the SAME sort-key value (1),
        // so the value-order engine cannot distinguish them. The output must be
        // deterministically tie-broken by canonical term string (ascending),
        // independent of input order — proving byte-stability does not rely on
        // the SPARQL engine's tie-break.
        let data = load_data(
            r"
            @prefix ex: <http://example.org/ns#> .
            ex:a ex:k 1 .
            ex:b ex:k 1 .
        ",
        );
        let mut guard = RecursionGuard::new();
        // Feed the input in reversed order (ex:b before ex:a) via a Union so the
        // engine's natural order can't accidentally produce the expected answer.
        let expr = NodeExpr::OrderBy {
            of: Box::new(NodeExpr::Union(vec![
                NodeExpr::Constant(ex("b")),
                NodeExpr::Constant(ex("a")),
            ])),
            key: Box::new(NodeExpr::Path(pred("k"))),
            descending: false,
        };
        let result =
            eval_node_expr(&data.data(), &ex("x"), &expr, &mut guard).expect("orderby evals");
        assert_eq!(
            result,
            vec![ex("a"), ex("b")],
            "ties break ascending by term"
        );

        // Descending flips the primary key, but the tie-break stays ascending.
        let expr_desc = NodeExpr::OrderBy {
            of: Box::new(NodeExpr::Union(vec![
                NodeExpr::Constant(ex("b")),
                NodeExpr::Constant(ex("a")),
            ])),
            key: Box::new(NodeExpr::Path(pred("k"))),
            descending: true,
        };
        let result =
            eval_node_expr(&data.data(), &ex("x"), &expr_desc, &mut guard).expect("orderby evals");
        assert_eq!(
            result,
            vec![ex("a"), ex("b")],
            "tie-break is always ascending even when descending"
        );
    }

    #[test]
    fn offset_skips_leading_values() {
        let data = load_data(AGG_DATA);
        let mut guard = RecursionGuard::new();
        // OrderBy first so the sequence is deterministic before the offset.
        let expr = NodeExpr::Offset {
            of: Box::new(NodeExpr::OrderBy {
                of: Box::new(NodeExpr::Path(pred("e"))),
                key: Box::new(NodeExpr::This),
                descending: false,
            }),
            n: 1,
        };
        let result =
            eval_node_expr(&data.data(), &ex("x"), &expr, &mut guard).expect("offset evals");
        assert_eq!(result, vec![ex("b"), ex("c")]);
    }

    #[test]
    fn limit_takes_leading_values() {
        let data = load_data(AGG_DATA);
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Limit {
            of: Box::new(NodeExpr::OrderBy {
                of: Box::new(NodeExpr::Path(pred("e"))),
                key: Box::new(NodeExpr::This),
                descending: false,
            }),
            n: 2,
        };
        let result =
            eval_node_expr(&data.data(), &ex("x"), &expr, &mut guard).expect("limit evals");
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
                    key: Box::new(NodeExpr::This),
                    descending: false,
                }),
                n: 1,
            }),
            n: 1,
        };
        // orderby → [a,b,c]; offset 1 → [b,c]; limit 1 → [b].
        let result =
            eval_node_expr(&data.data(), &ex("x"), &expr, &mut guard).expect("composed evals");
        assert_eq!(result, vec![ex("b")]);
    }

    /// `sh:min` over a blank node answers the blank node.
    ///
    /// A blank node is an ordinary term of the SPARQL total order (the LEAST kind
    /// of all), so a one-element bag containing one has a minimum, and it is that
    /// element. This used to be a hard type error — not because the comparator
    /// could not rank it, but because the operands were serialized into a `VALUES`
    /// block, which cannot spell a blank node. The refusal was the string bridge's,
    /// and it left with it.
    #[test]
    fn aggregate_over_a_blank_node_answers_the_blank_node() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let expr = NodeExpr::Min(Box::new(NodeExpr::Constant(Term::blank("b0"))));
        let result =
            eval_node_expr(&data.data(), &ex("x"), &expr, &mut guard).expect("min over a blank");
        assert_eq!(result, vec![Term::blank("b0")]);
    }

    /// `sh:max` over a set mixing every RDF 1.2 term kind answers the triple term,
    /// because a triple term is the greatest kind in the SPARQL total order.
    #[test]
    fn aggregate_over_mixed_rdf12_kinds_answers_the_triple_term() {
        let data = load_data("");
        let mut guard = RecursionGuard::new();
        let triple = Term::Triple(Box::new(crate::term::Triple::new(
            ex("s"),
            NamedNode::new_unchecked("http://example.org/ns#p"),
            ex("o"),
        )));
        let expr = NodeExpr::Max(Box::new(NodeExpr::Union(vec![
            NodeExpr::Constant(Term::blank("b0")),
            NodeExpr::Constant(ex("a")),
            NodeExpr::Constant(triple.clone()),
        ])));
        let result = eval_node_expr(&data.data(), &ex("x"), &expr, &mut guard)
            .expect("max over mixed kinds");
        assert_eq!(result, vec![triple]);
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

    /// A node-expression tree nested past [`MAX_NODE_EXPR_DEPTH`] must fail
    /// CLOSED — a hard `Err` naming the structural limit — instead of recursing
    /// into the native stack, whose exhaustion ABORTS the process uncatchably.
    ///
    /// The tree is built PROGRAMMATICALLY rather than authored as Turtle: a
    /// deeply nested blank-node Turtle document would exercise the Turtle parser
    /// in another crate, which is a different bound in a different place. The
    /// wrapper alternates the paging and set combinators so the assertion covers
    /// more than one arm of the evaluator.
    #[test]
    fn node_expression_deeper_than_the_structural_limit_fails_closed() {
        // The evaluator's per-level frame (the wide `eval_node_expr_at_depth`
        // match) is sizeable, so run on a generous stack: the point of the test
        // is that the DEPTH GUARD — not a stack overflow — is what terminates
        // the walk. Dropping the tree is also recursive, hence the same thread.
        let handle = std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                let data = load_data(DATA);
                let mut expr = NodeExpr::This;
                for i in 0..MAX_NODE_EXPR_DEPTH + 8 {
                    expr = if i % 2 == 0 {
                        NodeExpr::Union(vec![expr])
                    } else {
                        NodeExpr::Limit {
                            of: Box::new(expr),
                            n: 10,
                        }
                    };
                }
                let mut guard = RecursionGuard::new();
                eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard)
            })
            .expect("spawn deep-stack thread");

        let err = handle
            .join()
            .expect("deep-stack thread must not overflow — the depth guard terminates it")
            .expect_err("an expression nested past the structural ceiling must be a hard error");
        assert!(
            err.contains("node expression nesting depth exceeded"),
            "error should name the structural limit, got: {err}"
        );
        assert!(
            err.contains(&MAX_NODE_EXPR_DEPTH.to_string()),
            "error should name the ceiling value, got: {err}"
        );
    }

    /// The anti-regression guard for the ceiling: a DEEP BUT VALID expression
    /// still evaluates.
    ///
    /// `parse_node_expr_wrapped` always builds `Limit(Offset(OrderBy(core)))`
    /// around an authored node carrying paging keys — three structural levels per
    /// authored level — so 30 authored paging levels are 90+ structural levels.
    /// Reusing the 64-deep filter/exists ceiling for structural nesting would
    /// reject this perfectly legal shape, trading an abort for a conformance
    /// regression.
    #[test]
    fn deep_but_valid_paged_expression_still_evaluates() {
        const AUTHORED_LEVELS: usize = 30;
        let data = load_data(DATA);
        let mut expr = NodeExpr::This;
        for _ in 0..AUTHORED_LEVELS {
            expr = NodeExpr::Limit {
                of: Box::new(NodeExpr::Offset {
                    of: Box::new(NodeExpr::OrderBy {
                        of: Box::new(expr),
                        key: Box::new(NodeExpr::This),
                        descending: false,
                    }),
                    n: 0,
                }),
                n: 10,
            };
        }
        // 3 wrapper levels per authored level, plus the innermost `sh:this`:
        // comfortably past 64, comfortably inside the structural ceiling.
        assert!(AUTHORED_LEVELS * 3 + 1 > MAX_RECURSION_DEPTH as usize);
        assert!(AUTHORED_LEVELS * 3 + 1 < MAX_NODE_EXPR_DEPTH as usize);

        let mut guard = RecursionGuard::new();
        let result = eval_node_expr(&data.data(), &ex("a"), &expr, &mut guard)
            .expect("a deep but legal paged expression must evaluate");
        assert_eq!(result, vec![ex("a")]);
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
                    rules: vec![],
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

                crate::constraints::conforms(&data.data(), &ex("a"), &shape)
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
