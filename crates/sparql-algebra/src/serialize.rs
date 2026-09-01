// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Algebra → SPARQL **surface-text** serialization.
//!
//! The inverse of [`crate::parser`]: [`pattern_to_select_query`] renders a
//! [`GraphPattern`] back to a complete, standalone `SELECT` query string. The
//! driving use case is SPARQL `SERVICE` federation: the evaluator forwards a
//! federated sub-pattern to a remote endpoint as a complete query, and that
//! requires re-materializing the algebra as text.
//!
//! # Design
//!
//! * Pure `core::fmt::Write` into a `String` — **wasm-clean**, no std-only deps,
//!   reusing the existing [`crate::algebra::PropertyPathExpression`] `Display`
//!   for paths.
//! * **Round-trips** with the parser: `parse(pattern_to_select_query(p))`
//!   reproduces `p` for every [`GraphPattern`]/[`Expression`] variant the parser
//!   emits, INCLUDING an aggregate/`GROUP BY` chain with its own outer `Project`
//!   already peeled away (the shape [`pattern_to_select_query`]'s own doctest
//!   takes, and the shape a whole aggregate query is once a caller — federation or
//!   otherwise — has stripped the query's top `SELECT` scaffold to reach the WHERE
//!   body underneath it). Expressions are conservatively fully parenthesized —
//!   over-parenthesization is a no-op on re-parse, so correctness never depends on
//!   reproducing the exact precedence.
//! * Solution-modifier nodes (`Project`/`Distinct`/`Reduced`/`Slice`/`OrderBy`/
//!   `Group`) re-materialize as a braced **sub-`SELECT`** `{ SELECT ... }`, the
//!   shape the parser produces for an inline subquery; an aggregate's own
//!   `SELECT`-expression/`HAVING` chain reaching `Group` with NO such node above it
//!   re-materializes as a complete top-level `SELECT` instead — see
//!   [`pattern_to_select_query`]'s own docs for why the distinction matters.
//!
//! The PurRDF predicate-wildcard path extension (`<any>`) is emit-only (no parse),
//! exactly as documented on [`crate::algebra::PropertyPathExpression`]'s
//! `Display`; a path carrying it does not round-trip, which is the established
//! contract.

use core::fmt::Write as _;

use crate::algebra::{
    AggregateExpression, AggregateFunction, Expression, Function, GraphPattern, OrderExpression,
    PropertyFunctionCall,
};
use crate::ast::{
    BaseDirection, GroundTerm, GroundTriple, Literal, NamedNodePattern, RDF_LANG_STRING,
    TermPattern, TriplePattern, Variable, XSD_STRING,
};

/// A `GROUP BY` key + its `(output var, aggregate)` pairs, borrowed from a
/// [`GraphPattern::Group`] node during sub-`SELECT` reconstruction.
type GroupSpec<'a> = (&'a [Variable], &'a [(Variable, AggregateExpression)]);

/// Render `inner` as a complete, standalone `SELECT` query string.
///
/// This is the entry point SERVICE federation uses to forward a sub-pattern to a
/// remote endpoint. The result is a syntactically complete, re-parseable query.
///
/// For an ORDINARY graph pattern (the common case: `inner` is a WHERE-body element
/// with no `SELECT`-expression/aggregate machinery of its own — a BGP, a join, a
/// filter, …) the rendering is the literal `SELECT * WHERE { … }` template the
/// module docs describe.
///
/// For `inner` reaching a [`GraphPattern::Group`] through a leading chain of
/// `SELECT`-expression (`Extend`) / `HAVING` (`Filter`) nodes with **no** `Project`
/// anywhere above it — the shape an aggregate query's own modifier chain takes once
/// its outer `Project` has already been peeled away, exactly as the doctest below
/// does — the rendering is that aggregate query's own complete `SELECT … WHERE { … }
/// [GROUP BY …] [HAVING …]` text instead, rather than `SELECT * WHERE { … }` wrapped
/// around it: SPARQL admits no `SELECT *` reading over a `GROUP BY`, and wrapping a
/// literal `SELECT *` around this shape would either be rejected as invalid syntax on
/// re-parse or — for the implicit whole-table group, which has no `GROUP BY` clause
/// to make the shape visible — silently drop the aggregate behind a `BIND`
/// referencing a variable nothing in the rendered text binds.
///
/// # Examples
///
/// ```
/// use purrdf_sparql_algebra::{GraphPattern, Query, SparqlParser};
/// use purrdf_sparql_algebra::pattern_to_select_query;
///
/// let parser = SparqlParser::new();
/// let Query::Select { pattern, .. } = parser
///     .parse_query("SELECT * WHERE { ?s <http://example.org/p> ?o }")
///     .expect("a well-formed query parses")
/// else {
///     panic!("a SELECT query parses to `Query::Select`");
/// };
/// let GraphPattern::Project { inner, .. } = pattern else {
///     panic!("the projection wraps the root pattern");
/// };
///
/// let rendered = pattern_to_select_query(&inner);
/// assert!(rendered.starts_with("SELECT * WHERE {"));
/// // The rendering is complete and re-parseable.
/// assert!(parser.parse_query(&rendered).is_ok());
/// ```
#[must_use]
pub fn pattern_to_select_query(inner: &GraphPattern) -> String {
    let mut s = String::new();
    if needs_subselect_reconstruction(inner) {
        // `inner` IS a bare (no `Project` anywhere above it) modifier chain —
        // a `SELECT`-expression/`HAVING` chain reaching a `Group`, OR a
        // `Slice`/`Distinct`/`Reduced`/`OrderBy`/`Extend` sitting at the very
        // top with nothing above it — the shape the parser's algebra takes
        // for a `GROUP`/`ORDER BY`/`LIMIT`/`DISTINCT`/`REDUCED`/SELECT-expression
        // query once its own outer `Project` has already been peeled away
        // (exactly what this function's doctest above does before calling
        // it — and what a caller of THIS function that itself stripped an
        // outer `Project`, e.g. the corpus round-trip sweep, does too).
        // Render it through [`fmt_subselect`] DIRECTLY: that already
        // reconstructs a complete `SELECT … WHERE { … } [GROUP BY] …` string
        // on its own, so wrapping it in ANOTHER literal `SELECT * WHERE { … }`
        // here would EITHER double-nest a needless subquery whose outer
        // `Project` re-projects the identical variable set the inner one
        // already declared (a real, if harmless, structural mismatch on
        // re-parse — see `crates/sparql-algebra/tests/serializer_roundtrip_sweep.rs`'s
        // `negation/full-minuend.rq` finding) OR — because a `Group`-containing
        // pattern has no valid `SELECT *` reading — render `SELECT *` over an
        // aggregate query, which SPARQL does not admit. Un-fixed, that second
        // failure mode is exactly how an aggregate call could vanish behind a
        // dangling reference to its synthetic output variable: seeing this
        // shape and choosing NOT to reach for `fmt_subselect` is the bug, not
        // a detail of how to call it.
        fmt_subselect(&mut s, inner);
    } else {
        s.push_str("SELECT * WHERE { ");
        fmt_group_body(&mut s, inner);
        s.push_str(" }");
    }
    s
}

/// Whether `p`, with no `Project` above it, is a bare modifier-chain shape
/// [`fmt_subselect`] must reconstruct directly rather than being wrapped in
/// another `SELECT * WHERE { … }` (see [`pattern_to_select_query`]'s doc for
/// why the wrap is wrong either way this predicate is true). `Slice`/
/// `Distinct`/`Reduced`/`OrderBy`/`Extend` are UNCONDITIONALLY this shape at
/// the top — none of them can legally appear as a bare (`Project`-less)
/// element of an ORDINARY `{ … }` group (SPARQL's grammar only ever produces
/// them as part of a `SELECT`/subselect's own solution-modifier chain, which
/// always carries a `Project`), so reaching one here with no `Project` above
/// it is exactly the "outer `Project` already peeled" shape, regardless of
/// whether it goes on to reach a `Group`. `Filter`/`Group` still route through
/// [`extend_chain_reaches_group`], which is the one case where "reaches a
/// `Group`" (not "is one, unconditionally") is the right test — an ordinary
/// WHERE-body `FILTER` (never wrapping a `Group`) must NOT trigger this.
fn needs_subselect_reconstruction(p: &GraphPattern) -> bool {
    matches!(
        p,
        GraphPattern::Extend { .. }
            | GraphPattern::Slice { .. }
            | GraphPattern::Distinct { .. }
            | GraphPattern::Reduced { .. }
            | GraphPattern::OrderBy { .. }
    ) || extend_chain_reaches_group(p)
}

/// `true` for the solution-modifier nodes that re-materialize as a sub-`SELECT`
/// rather than as a bare group-graph-pattern element.
fn is_subselect_node(p: &GraphPattern) -> bool {
    matches!(
        p,
        GraphPattern::Project { .. }
            | GraphPattern::Distinct { .. }
            | GraphPattern::Reduced { .. }
            | GraphPattern::Slice { .. }
            | GraphPattern::OrderBy { .. }
            | GraphPattern::Group { .. }
    )
}

/// Whether `p` is a leading chain of ONLY `Extend` (a SELECT-expression `(expr AS
/// ?v)` bind) / `Filter` (`HAVING` — however many conditions are chained; `HAVING
/// (a) (b) …` lifts to one nested `Filter` per condition, so this recurses through
/// the whole chain rather than checking only the OUTERMOST `Filter`'s immediate
/// `inner`) nodes that terminates at a `Group` — the shape an aggregate query's own
/// SELECT-expression/`HAVING` chain takes once its outer `Project` has already been
/// peeled away.
///
/// Deliberately narrower than [`is_subselect_node`]/[`fmt_subselect`]'s full peel
/// loop: a `Project`/`Distinct`/`Reduced`/`Slice`/`OrderBy` node anywhere in the chain
/// means `p` is already one of [`is_subselect_node`]'s recognized shapes — a genuine,
/// self-contained sub-`SELECT` meant to be embedded as one element of a forwarded
/// WHERE body, which [`fmt_group_body`] continues to wrap in a nested `{ SELECT … }`
/// exactly as it always has. Only the narrower, `Project`-less shape here needs
/// [`pattern_to_select_query`]'s different (unwrapped) treatment — and it can ONLY
/// arise there: an ordinary WHERE-body `BIND`/`HAVING` never wraps a bare `Group` (a
/// `Group` node is minted only by the parser's aggregate-lifting, always immediately
/// under the query's own modifier chain), and any `{ SELECT … }` written in source
/// text parses to a `Project`-wrapped pattern, not a bare `Extend`/`Group` chain — so
/// this predicate cannot mistake an ordinary body element for this shape.
fn extend_chain_reaches_group(p: &GraphPattern) -> bool {
    match p {
        GraphPattern::Group { .. } => true,
        // `HAVING (a) (b) …` lifts to a CHAIN of `Filter`s, one per condition
        // (`parser.rs`'s `for expr in modifiers.having { p = Filter{expr,
        // inner: p} }`), so recursing here — not just checking whether THIS
        // Filter's immediate `inner` is the `Group` — is what makes a query
        // with more than one `HAVING` condition recognized at all. Un-fixed,
        // a second `HAVING` condition made this predicate return `false`,
        // which sent `pattern_to_select_query` down the ordinary
        // `SELECT * WHERE { … }` path over a `Group`-containing pattern — the
        // exact "SPARQL has no `SELECT *` reading over `GROUP BY`" failure
        // this function's own doc says choosing that path is the bug.
        GraphPattern::Extend { inner, .. } | GraphPattern::Filter { inner, .. } => {
            extend_chain_reaches_group(inner)
        }
        _ => false,
    }
}

/// Emit a graph pattern as the body of a `{ … }` group. Modifier-wrapped patterns
/// (subqueries) are emitted as a braced `{ SELECT … }` block.
///
/// `pub(crate)`: this is also the WHERE-clause renderer `Display for
/// GraphUpdateOperation` (`algebra.rs`) reuses for `INSERT`/`DELETE … WHERE { … }`
/// — the exact same shape [`crate::parser`]'s `parse_group_graph_pattern` produces
/// for an UPDATE's WHERE clause (never the "bare aggregate chain" shape
/// [`needs_subselect_reconstruction`] exists to catch, which only arises once an
/// outer `Project` has been peeled from a top-level `SELECT`/subselect — an UPDATE
/// WHERE clause never carries one). Reusing this function rather than writing a
/// second pattern-to-text renderer is deliberate: see `algebra.rs`'s `Display for
/// GraphUpdateOperation` doc.
pub(crate) fn fmt_group_body(s: &mut String, p: &GraphPattern) {
    if is_subselect_node(p) {
        s.push_str("{ ");
        fmt_subselect(s, p);
        s.push_str(" }");
        return;
    }
    match p {
        GraphPattern::Bgp { patterns } => fmt_bgp(s, patterns),
        GraphPattern::Path {
            subject,
            path,
            object,
        } => {
            fmt_term(s, subject);
            let _ = write!(s, " {path} ");
            fmt_term(s, object);
            s.push_str(" .");
        }
        GraphPattern::Join { left, right } => {
            fmt_flattened_left(s, left);
            s.push(' ');
            fmt_join_right_operand(s, right);
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            fmt_flattened_left(s, left);
            s.push_str(" OPTIONAL { ");
            fmt_group_body(s, right);
            if let Some(expr) = expression {
                s.push_str(" FILTER(");
                fmt_expr(s, expr);
                s.push(')');
            }
            s.push_str(" }");
        }
        GraphPattern::Lateral { left, right } => {
            fmt_flattened_left(s, left);
            if parser_rebuilds_the_lateral(left, right) {
                // Two right-operand shapes are surface forms the parser's OWN
                // dispatch arms re-wrap into exactly this `Lateral` node without
                // any `LATERAL` keyword in the text: a property function (written
                // as a triple; the PF-triple-folding loop builds the chain) and a
                // variable-endpoint `SERVICE ?g { … }` (the SERVICE dispatch arm
                // auto-wraps a variable endpoint into a `Lateral` because it is
                // correlated with the enclosing pattern). Emitting an explicit
                // `LATERAL { … }` around either would double-nest on re-parse:
                // the braced RHS parses to its OWN `Lateral` node first (rooted at
                // the unit-table left), and the outer keyword would wrap that
                // again. So both render unwrapped here, exactly like the
                // fixed-IRI `SERVICE` case does for a plain `Join`.
                if !is_empty_group_body(left) {
                    s.push(' ');
                }
                fmt_group_body(s, right);
                return;
            }
            s.push_str(" LATERAL { ");
            fmt_group_body(s, right);
            s.push_str(" }");
        }
        GraphPattern::PropertyFunction(call) => fmt_property_function(s, call),
        GraphPattern::Filter { expr, inner } => {
            fmt_group_body(s, inner);
            s.push_str(" FILTER(");
            fmt_expr(s, expr);
            s.push(')');
        }
        GraphPattern::Union { left, right } => {
            s.push_str("{ ");
            fmt_group_body(s, left);
            s.push_str(" } UNION { ");
            fmt_group_body(s, right);
            s.push_str(" }");
        }
        GraphPattern::Graph { name, inner } => {
            s.push_str("GRAPH ");
            fmt_named_node_pattern(s, name);
            s.push_str(" { ");
            fmt_group_body(s, inner);
            s.push_str(" }");
        }
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => {
            fmt_group_body(s, inner);
            s.push_str(" BIND(");
            fmt_expr(s, expression);
            let _ = write!(s, " AS {})", VarRef(variable));
        }
        GraphPattern::Minus { left, right } => {
            fmt_flattened_left(s, left);
            s.push_str(" MINUS { ");
            fmt_group_body(s, right);
            s.push_str(" }");
        }
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => {
            s.push_str("SERVICE ");
            if *silent {
                s.push_str("SILENT ");
            }
            fmt_named_node_pattern(s, name);
            s.push_str(" { ");
            fmt_group_body(s, inner);
            s.push_str(" }");
        }
        GraphPattern::Values {
            variables,
            bindings,
        } => fmt_values(s, variables, bindings),
        // Subselect nodes are handled by the `is_subselect_node` branch above.
        GraphPattern::Project { .. }
        | GraphPattern::Distinct { .. }
        | GraphPattern::Reduced { .. }
        | GraphPattern::Slice { .. }
        | GraphPattern::OrderBy { .. }
        | GraphPattern::Group { .. } => unreachable!("handled by is_subselect_node"),
    }
}

/// Emit the RIGHT operand of a `Join`, braced as its own group when leaving it
/// unbraced would change the re-parsed tree's meaning.
///
/// Two independent reasons force a brace, both restated here rather than
/// inferred from the flag names alone:
///
/// * **Contains a property function.** A property function renders as a plain
///   triple, and the parser folds every property-function triple of ONE
///   triples block into a single left-deep `Lateral` chain rooted at the
///   triples written before it. A right operand that was a separate group
///   (`{ … }`, a `GRAPH`, a nested join) would therefore be absorbed into the
///   left operand's chain on re-parse; the explicit `{ … }` keeps it its own
///   group and the round-trip exact.
/// * **[`rendering_starts_with_a_reabsorbable_left`].** `LeftJoin` / `Lateral`
///   / `Minus` / `Filter` / `Extend` all render by emitting their OWN left
///   operand inline first; unbraced, that inline left operand splices onto the
///   outer `Join`'s left operand in the running left-to-right parse, changing
///   which elements the modifier applies over — a WIRE-FORMAT correctness bug,
///   since [`pattern_to_select_query`] is what SERVICE federation forwards to
///   a remote endpoint.
///
/// A LEFT operand needs no such brace in either case: the parser's own
/// left-deep assembly reproduces it. `Join` itself is deliberately excluded
/// from the second condition (see that function's docs); a plain `Join` right
/// operand containing no property function keeps the historical brace-free
/// rendering, which re-associates on re-parse into a semantically identical
/// (not tree-identical) `Join` chain.
fn fmt_join_right_operand(s: &mut String, p: &GraphPattern) {
    if is_subselect_node(p) {
        fmt_group_body(s, p);
        return;
    }
    if contains_property_function(p) || rendering_starts_with_a_reabsorbable_left(p) {
        s.push_str("{ ");
        fmt_group_body(s, p);
        s.push_str(" }");
    } else {
        fmt_group_body(s, p);
    }
}

/// Does this pattern contain a [`GraphPattern::PropertyFunction`] anywhere in the
/// group structure it renders inline (it does not descend into `Expression`s,
/// which are always emitted inside their own braces)?
fn contains_property_function(p: &GraphPattern) -> bool {
    match p {
        GraphPattern::PropertyFunction(_) => true,
        GraphPattern::Join { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right }
        | GraphPattern::LeftJoin { left, right, .. } => {
            contains_property_function(left) || contains_property_function(right)
        }
        GraphPattern::Filter { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Service { inner, .. }
        | GraphPattern::Extend { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Group { inner, .. } => contains_property_function(inner),
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => false,
    }
}

/// `true` for a group body that renders as the empty string (the identity table
/// `Z`), so a caller can skip the separating space before the next element.
fn is_empty_group_body(p: &GraphPattern) -> bool {
    matches!(p, GraphPattern::Bgp { patterns } if patterns.is_empty())
}

/// Does a [`GraphPattern::Lateral`] node render as a surface form the parser's
/// OWN dispatch arms — not the `LATERAL` keyword — re-wrap into exactly this
/// `Lateral` node on re-parse?
///
/// Two right-operand shapes qualify, under different conditions on `left`:
///
/// * A variable-endpoint [`GraphPattern::Service`] (`SERVICE ?g { … }`): the
///   parser's `SERVICE` dispatch arm auto-wraps a variable endpoint into a
///   `Lateral` USING WHATEVER `left` IT ALREADY HAS ACCUMULATED, unconditionally
///   — so this qualifies for every `left` shape. A FIXED-IRI `Service` does
///   **not** qualify at all: the parser's `SERVICE` arm folds it with a plain
///   [`crate::algebra::GraphPattern::Join`] instead, so without an explicit
///   `LATERAL { … }` keyword here the laterality would be lost on re-parse (the
///   round-tripped tree would be a `Join`, not a `Lateral`).
/// * A [`GraphPattern::PropertyFunction`] (written as a plain triple): the
///   parser's PF-triple-folding loop builds this `Lateral` node INSIDE ONE
///   triples block, independently of whatever the enclosing group's `left` is —
///   it only reproduces `left` exactly when `left` is ITSELF a shape that same
///   fold can build ([`is_pf_reabsorbable_left`]). When `left` is some other
///   group element (`Union`/`Graph`/`Values`/…), the fold instead starts a
///   fresh unit-table `Lateral` for the PF call and the group loop's own `join`
///   attaches `left` to it as a plain `Join` — dropping the laterality (the PF
///   call would no longer see `left`'s bindings). This qualifies ONLY when
///   [`is_pf_reabsorbable_left`] says so.
fn parser_rebuilds_the_lateral(left: &GraphPattern, right: &GraphPattern) -> bool {
    match right {
        GraphPattern::PropertyFunction(_) => is_pf_reabsorbable_left(left),
        GraphPattern::Service {
            name: NamedNodePattern::Variable(_),
            ..
        } => true,
        _ => false,
    }
}

/// Is `left` a shape the parser's PF-triple-folding loop (`BlockSink::into_pattern`,
/// in `parser.rs`) can itself produce as the LEFT operand of a
/// `Lateral{left, right: PropertyFunction}` node — i.e. would rendering `left`
/// unbraced immediately before a property-function triple re-parse as ONE
/// triples block that folds back to exactly this `left`?
///
/// The fold's only leaves are `Bgp`s (property paths are recorded separately
/// and always joined on AFTER every property-function fold, never inside one —
/// so a `Path` leaf is deliberately EXCLUDED here even though it renders
/// unbraced too: reparsing would relocate it to the far end of the block
/// instead of keeping it as this node's left, changing the tree). The two
/// combinators the fold builds around those leaves are `Join{prior, Bgp}`
/// (residual triples appended after a PF call) and `Lateral{prior,
/// PropertyFunction}` (the PF call itself) — so both recurse on their own
/// `left`/`prior` operand.
fn is_pf_reabsorbable_left(p: &GraphPattern) -> bool {
    match p {
        GraphPattern::Bgp { .. } => true,
        GraphPattern::Join { left, right } => {
            matches!(**right, GraphPattern::Bgp { .. }) && is_pf_reabsorbable_left(left)
        }
        GraphPattern::Lateral { left, right } => {
            matches!(**right, GraphPattern::PropertyFunction(_)) && is_pf_reabsorbable_left(left)
        }
        _ => false,
    }
}

/// `true` for the [`GraphPattern`] variants whose OWN rendering starts by
/// emitting their `left`/`inner` operand inline, with no group boundary in
/// front of it — the set that MUST be braced when placed as a [`Join`]'s right
/// operand, because gluing that rendering directly after the outer left
/// operand would splice the two `left` operands into one running left-to-right
/// accumulation on re-parse, re-associating the tree to a different meaning
/// (`(A JOIN B) OPTIONAL C` instead of `A JOIN (B OPTIONAL C)`, for example).
///
/// A SIMILAR (not identical — see its own doc) hazard applies to a `Join`/
/// `LeftJoin`/`Lateral`/`Minus` node's OWN LEFT operand; that side is decided
/// by the narrower [`left_operand_needs_bracing`] instead, via
/// [`fmt_flattened_left`].
///
/// [`GraphPattern::Join`] is deliberately **excluded**: join is associative, so
/// re-associating a `Join` right operand into the running left-to-right chain
/// produces a semantically identical tree — the round-trip contract this module
/// promises is "semantics preserved", and tree-identity is asserted only where
/// semantics actually require it (see `roundtrip_lateral_chain_shapes` beside
/// this function's call site). Every modifier-rooted node
/// (`Project`/`Distinct`/`Reduced`/`Slice`/`OrderBy`/`Group`) is also excluded:
/// [`is_subselect_node`] already renders those as a self-contained braced
/// sub-`SELECT`, so they never glue onto a preceding element in the first
/// place.
///
/// Exhaustive, wildcard-free: a new [`GraphPattern`] variant must be triaged
/// here explicitly rather than silently inheriting the unbraced default.
///
/// [`Join`]: crate::algebra::GraphPattern::Join
fn rendering_starts_with_a_reabsorbable_left(p: &GraphPattern) -> bool {
    match p {
        GraphPattern::LeftJoin { .. }
        | GraphPattern::Lateral { .. }
        | GraphPattern::Minus { .. }
        | GraphPattern::Filter { .. }
        | GraphPattern::Extend { .. } => true,
        GraphPattern::Bgp { .. }
        | GraphPattern::Path { .. }
        | GraphPattern::Join { .. }
        | GraphPattern::Union { .. }
        | GraphPattern::Graph { .. }
        | GraphPattern::Service { .. }
        | GraphPattern::Values { .. }
        | GraphPattern::OrderBy { .. }
        | GraphPattern::Project { .. }
        | GraphPattern::Distinct { .. }
        | GraphPattern::Reduced { .. }
        | GraphPattern::Slice { .. }
        | GraphPattern::Group { .. }
        | GraphPattern::PropertyFunction(_) => false,
    }
}

/// Emit the LEFT operand of a `Join`/`LeftJoin`/`Lateral`/`Minus` node,
/// bracing it as its own group when [`left_operand_needs_bracing`] says
/// leaving it unbraced would splice its own scope onto the outer group.
/// Every other LEFT shape (an ordinary `Bgp`/`Path`/`Join`/`Lateral`/`Union`/
/// `Graph`/`Service`/`Values`/property-function/…) keeps the historical
/// unbraced rendering: the parser's own left-deep assembly reproduces those
/// exactly.
fn fmt_flattened_left(s: &mut String, left: &GraphPattern) {
    if left_operand_needs_bracing(left) {
        s.push_str("{ ");
        fmt_group_body(s, left);
        s.push_str(" }");
    } else {
        fmt_group_body(s, left);
    }
}

/// `true` for the [`GraphPattern`] variants that need bracing when they sit
/// as the LEFT operand of a `Join`/`LeftJoin`/`Lateral`/`Minus` node — a
/// NARROWER set than [`rendering_starts_with_a_reabsorbable_left`]'s (that
/// predicate governs the RIGHT-operand hazard, a different question): a
/// `Filter`/`Extend`/`LeftJoin`/`Minus` LEFT operand renders by emitting ITS
/// OWN left operand inline first, so flattening it splices its own
/// filter/bind/optional/minus scope onto the OUTER node's own group instead
/// of keeping it as its own nested scope (found by the corpus round-trip
/// sweep: `service/service05.rq`'s `FILTER`, scoped to a bracketed
/// sub-group, re-associated onto the whole outer group — including a
/// `SERVICE ?g` lateral join written after it — once flattened unbraced).
///
/// `Lateral` is DELIBERATELY EXCLUDED, unlike on the right-operand
/// predicate: it chains left-deep the SAME way `Join` does (SEP-0006
/// laterality is written left-to-right; both a user-written
/// `A LATERAL { B } LATERAL { C }` chain and the parser's OWN
/// property-function-triple folding build exactly this shape as `g`
/// accumulates), so bracing it here would be WRONG for a PF-folded chain
/// specifically: the parser's triples-block loop expects to read a
/// continuous run of triples/PF-calls as ONE triples block, and a brace
/// splits it, breaking the fold (`roundtrip_property_functions_mixed_with_data_triples`
/// is what catches this — it round-trips a PF chain immediately followed by
/// an ordinary triple sharing the same block).
fn left_operand_needs_bracing(p: &GraphPattern) -> bool {
    match p {
        GraphPattern::LeftJoin { .. }
        | GraphPattern::Minus { .. }
        | GraphPattern::Filter { .. }
        | GraphPattern::Extend { .. } => true,
        GraphPattern::Bgp { .. }
        | GraphPattern::Path { .. }
        | GraphPattern::Join { .. }
        | GraphPattern::Lateral { .. }
        | GraphPattern::Union { .. }
        | GraphPattern::Graph { .. }
        | GraphPattern::Service { .. }
        | GraphPattern::Values { .. }
        | GraphPattern::OrderBy { .. }
        | GraphPattern::Project { .. }
        | GraphPattern::Distinct { .. }
        | GraphPattern::Reduced { .. }
        | GraphPattern::Slice { .. }
        | GraphPattern::Group { .. }
        | GraphPattern::PropertyFunction(_) => false,
    }
}

/// Emit a property-function call as the triple `subjectArgs <iri> objectArgs .`
///
/// The IRI is re-emitted byte-exact (PurRDF fabricates no namespace on output).
/// A ONE-element argument vector renders as the bare term; a zero- or
/// multi-element vector renders as collection syntax `( … )`, which the parser
/// reads back as an argument list — never as an `rdf:first`/`rdf:rest` chain —
/// because the predicate names a property function.
fn fmt_property_function(s: &mut String, call: &PropertyFunctionCall) {
    fmt_property_function_args(s, &call.subject_args);
    let _ = write!(s, " <{}> ", call.iri);
    fmt_property_function_args(s, &call.object_args);
    s.push_str(" .");
}

/// Emit one side of a property-function call (see [`fmt_property_function`]).
fn fmt_property_function_args(s: &mut String, args: &[TermPattern]) {
    if let [single] = args {
        fmt_term(s, single);
        return;
    }
    if args.is_empty() {
        s.push_str("()");
        return;
    }
    s.push('(');
    for arg in args {
        s.push(' ');
        fmt_term(s, arg);
    }
    s.push_str(" )");
}

/// Emit a basic graph pattern (a conjunction of triple patterns).
fn fmt_bgp(s: &mut String, patterns: &[TriplePattern]) {
    for (i, tp) in patterns.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        fmt_term(s, &tp.subject);
        s.push(' ');
        fmt_named_node_pattern(s, &tp.predicate);
        s.push(' ');
        fmt_term(s, &tp.object);
        s.push_str(" .");
    }
}

/// Peel the solution-modifier chain (outermost → innermost) and emit a
/// `SELECT [DISTINCT|REDUCED] <vars|*> WHERE { <body> } [GROUP BY] [HAVING]
/// [ORDER BY] [LIMIT] [OFFSET]`.
fn fmt_subselect(s: &mut String, p: &GraphPattern) {
    // Peel outer modifiers, recording each, until we reach the WHERE body.
    let mut cur = p;
    let mut distinct = false;
    let mut reduced = false;
    let mut slice: Option<(usize, Option<usize>)> = None;
    let mut project: Option<&[Variable]> = None;
    let mut order: Option<&[OrderExpression]> = None;
    // SELECT-expression binds (Extend nodes sitting above the Group/body).
    let mut select_exprs: Vec<(&Variable, &Expression)> = Vec::new();
    let mut group: Option<GroupSpec<'_>> = None;
    let mut having: Vec<&Expression> = Vec::new();

    loop {
        match cur {
            GraphPattern::Slice {
                inner,
                start,
                length,
            } => {
                slice = Some((*start, *length));
                cur = inner;
            }
            GraphPattern::Distinct { inner } => {
                distinct = true;
                cur = inner;
            }
            GraphPattern::Reduced { inner } => {
                reduced = true;
                cur = inner;
            }
            GraphPattern::Project { inner, variables } => {
                project = Some(variables);
                cur = inner;
            }
            GraphPattern::OrderBy { inner, expression } => {
                order = Some(expression);
                cur = inner;
            }
            GraphPattern::Extend {
                inner,
                variable,
                expression,
            } => {
                // A SELECT-expression bind only when it sits in the modifier
                // chain above the WHERE body (i.e. above a Group, or directly
                // above the body with no remaining group/where structure). We
                // greedily treat Extends encountered during the peel as SELECT
                // expressions; a BIND inside the WHERE body is reached only after
                // we stop peeling (it stays part of the body).
                select_exprs.push((variable, expression));
                cur = inner;
            }
            GraphPattern::Filter { expr, inner } if extend_chain_reaches_group(inner) => {
                // HAVING: this Filter's `inner` is either the `Group` directly
                // (one `HAVING` condition) or another `Extend`/`Filter` whose
                // OWN chain reaches it (`HAVING (a) (b) …`'s multi-condition
                // lift — see `extend_chain_reaches_group`'s doc). The
                // look-ahead through the WHOLE remaining chain, not just this
                // Filter's immediate `inner`, is what a second `HAVING`
                // condition needs: without it this arm stops peeling at the
                // FIRST (outermost) `HAVING` Filter, leaving `group` unset
                // and silently dropping the `GROUP BY`/aggregate rendering
                // entirely. An ordinary WHERE-body `FILTER` (no `Group`
                // anywhere below it) fails this guard and correctly falls
                // through to `_ => break`, staying part of the body.
                having.push(expr);
                cur = inner;
            }
            GraphPattern::Group {
                inner,
                variables,
                aggregates,
            } => {
                group = Some((variables, aggregates));
                cur = inner;
                break;
            }
            _ => break,
        }
    }
    // `select_exprs` was collected outermost-first; restore source order.
    select_exprs.reverse();
    having.reverse();

    s.push_str("SELECT ");
    if distinct {
        s.push_str("DISTINCT ");
    } else if reduced {
        s.push_str("REDUCED ");
    }
    // `SELECT *` has NO valid reading over an aggregating query (`GROUP BY`
    // present, or one or more aggregates) — SPARQL flatly bans it, the same
    // rule the parser itself enforces (`"SELECT * is not allowed in an
    // aggregate query"`). With no `Project` peeled here (this function's
    // top-level caller, `pattern_to_select_query`, is invoked on a
    // `Project`-STRIPPED body — see its own doc) there is no explicit
    // variable list to fall back to either, so one is reconstructed:
    // * Reaching a `Group`: its OWN key variables — the only ones guaranteed
    //   meaningful past a grouping boundary (an ordinary WHERE-body variable
    //   is NOT visible above a `Group`). Caught by the corpus round-trip
    //   sweep: `SELECT ?s { … } GROUP BY ?s` (no aggregate function at all —
    //   a plain `GROUP BY` key projection) used to re-emit `SELECT * …
    //   GROUP BY ?s`, which the parser then correctly refused on re-parse.
    // * Reaching a `Group` with an EMPTY key list (the implicit whole-table
    //   group an aggregate with no explicit `GROUP BY` gets): `None` —
    //   `cur` here sits BELOW the `Group`, and an ordinary WHERE-body
    //   variable is not legally project-able alongside an aggregate at all
    //   (`"SELECT projects ?s, which is neither a GROUP BY key nor confined
    //   to an aggregate"` — the very check that catches a wrong answer
    //   here), so there is nothing to add beyond the AS-targets already
    //   covering what the query needs.
    // * NOT reaching a `Group` at all (an `Extend` chain over an ordinary
    //   body — `needs_subselect_reconstruction`'s other case): every
    //   variable [`crate::parser::visible_variables`] still finds in the
    //   remaining body (`cur`, which here is genuinely still in scope — no
    //   grouping boundary was crossed), so a variable a real caller needs —
    //   chiefly `SERVICE` federation, whose whole contract with the remote
    //   endpoint is "return everything visible" — is never silently dropped
    //   from the projection just because it also carries a `(expr AS ?v)`
    //   bind. Also caught by the sweep: `SELECT (BNODE(?s1) AS ?b1) … WHERE
    //   { … FILTER (…) }` (a SELECT-expression bind over a filtered,
    //   non-aggregating body) used to render as an in-body `BIND`, which the
    //   parser's own "filters float to the group's end" rule then
    //   re-associated the `Filter`/`Extend` nesting differently than the
    //   original tree.
    let no_project_vars: Option<Vec<Variable>> = match &group {
        Some((vars, _)) if !vars.is_empty() => Some(vars.to_vec()),
        Some(_) => None,
        None if !select_exprs.is_empty() => Some(crate::parser::visible_variables(cur)),
        None => None,
    };
    // Skip any var whose binding will be emitted via `(expr AS ?v)`; emitting
    // it here too would produce an invalid duplicate projection. Shared by
    // every branch below that has a real variable LIST to filter (a
    // reconstructed `no_project_vars` list, or a genuine `Project`'s own
    // `variables`) — `*` needs no filtering, it names nothing to duplicate.
    let as_targets: std::collections::HashSet<&Variable> =
        select_exprs.iter().map(|(v, _)| *v).collect();
    let emit_filtered_vars = |s: &mut String, vars: &[Variable]| -> bool {
        let mut emitted = false;
        for v in vars {
            if as_targets.contains(v) {
                continue;
            }
            if emitted {
                s.push(' ');
            }
            let _ = write!(s, "{}", VarRef(v));
            emitted = true;
        }
        emitted
    };
    let plain_emitted = match project {
        None => match &no_project_vars {
            Some(vars) => emit_filtered_vars(s, vars),
            // `no_project_vars` is `None` in two shapes: an ordinary,
            // non-aggregating body with no `Extend` chain at all (`*` is
            // exactly right), OR an implicit whole-table group with nothing
            // legally project-able beyond its aggregate `select_exprs` (`*`
            // combined with a `(expr AS ?v)` target is ALSO illegal SPARQL —
            // `*` must be the entire select list — so it is pushed only when
            // there is no such target to combine it with).
            None if select_exprs.is_empty() => {
                s.push('*');
                false
            }
            None => false,
        },
        Some(vars) if vars.is_empty() && select_exprs.is_empty() => {
            s.push('*');
            false
        }
        Some(vars) => emit_filtered_vars(s, vars),
    };
    // Every select-expression/`HAVING`/`ORDER BY` render below sits ABOVE the
    // `Group` in the modifier chain, so each resolves an aggregate's synthetic
    // output variable to its rendered call form via `group` — see
    // `fmt_expr_agg`'s docs for why this is required for correctness, not just
    // cosmetics.
    for (i, (var, expr)) in select_exprs.iter().enumerate() {
        if plain_emitted || i > 0 {
            s.push(' ');
        }
        s.push('(');
        fmt_expr_agg(s, expr, group);
        let _ = write!(s, " AS {})", VarRef(var));
    }

    s.push_str(" WHERE { ");
    fmt_group_body(s, cur);
    s.push_str(" }");

    if let Some((vars, aggs)) = group {
        if !vars.is_empty() {
            s.push_str(" GROUP BY");
            for v in vars {
                let _ = write!(s, " {}", VarRef(v));
            }
        } else if !aggs.is_empty() {
            // Implicit single group (aggregates with no GROUP BY): no clause.
        }
    }
    if !having.is_empty() {
        s.push_str(" HAVING");
        for expr in &having {
            s.push('(');
            fmt_expr_agg(s, expr, group);
            s.push(')');
        }
    }
    if let Some(exprs) = order {
        s.push_str(" ORDER BY");
        for oe in exprs {
            match oe {
                OrderExpression::Asc(e) => {
                    s.push_str(" ASC(");
                    fmt_expr_agg(s, e, group);
                    s.push(')');
                }
                OrderExpression::Desc(e) => {
                    s.push_str(" DESC(");
                    fmt_expr_agg(s, e, group);
                    s.push(')');
                }
            }
        }
    }
    if let Some((start, length)) = slice {
        if let Some(len) = length {
            let _ = write!(s, " LIMIT {len}");
        }
        if start > 0 {
            let _ = write!(s, " OFFSET {start}");
        }
    }
}

/// Emit a `VALUES (?v …) { (term …) … }` block (always the parenthesized form).
fn fmt_values(s: &mut String, variables: &[Variable], bindings: &[Vec<Option<GroundTerm>>]) {
    s.push_str("VALUES (");
    for (i, v) in variables.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        let _ = write!(s, "{}", VarRef(v));
    }
    s.push_str(") {");
    for row in bindings {
        s.push_str(" (");
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            match cell {
                None => s.push_str("UNDEF"),
                Some(gt) => fmt_ground_term(s, gt),
            }
        }
        s.push(')');
    }
    s.push_str(" }");
}

/// Emit a query-pattern term.
fn fmt_term(s: &mut String, t: &TermPattern) {
    match t {
        TermPattern::NamedNode(n) => {
            let _ = write!(s, "<{}>", n.as_str());
        }
        TermPattern::BlankNode(b) => {
            let _ = write!(s, "_:{}", b.as_str());
        }
        TermPattern::Literal(l) => fmt_literal(s, l),
        TermPattern::Variable(v) => {
            let _ = write!(s, "{}", VarRef(v));
        }
        TermPattern::Triple(t) => fmt_triple_pattern(s, t),
    }
}

/// Emit an RDF 1.2 quoted triple term `<<( s p o )>>`.
fn fmt_triple_pattern(s: &mut String, t: &TriplePattern) {
    s.push_str("<<( ");
    fmt_term(s, &t.subject);
    s.push(' ');
    fmt_named_node_pattern(s, &t.predicate);
    s.push(' ');
    fmt_term(s, &t.object);
    s.push_str(" )>>");
}

/// Emit an IRI-or-variable (predicate / `GRAPH`/`SERVICE` name position).
fn fmt_named_node_pattern(s: &mut String, n: &NamedNodePattern) {
    match n {
        NamedNodePattern::NamedNode(node) => {
            let _ = write!(s, "<{}>", node.as_str());
        }
        NamedNodePattern::Variable(v) => {
            let _ = write!(s, "{}", VarRef(v));
        }
    }
}

/// Emit a ground term (VALUES cell).
fn fmt_ground_term(s: &mut String, gt: &GroundTerm) {
    match gt {
        GroundTerm::NamedNode(n) => {
            let _ = write!(s, "<{}>", n.as_str());
        }
        GroundTerm::Literal(l) => fmt_literal(s, l),
        GroundTerm::Triple(t) => fmt_ground_triple(s, t),
        // Injection-only (GAP-A): emitted as a blank-node label. The parser never
        // produces this variant, and `purrdf-sparql-eval`'s `SERVICE` forwarding path
        // (`sanitize_forwarded_body` in `crates/sparql-eval/src/remote.rs`) strips every
        // `Values` column carrying one before a substituted `SERVICE` body is
        // serialized — a blank-node `VALUES` cell is not legal `DataBlockValue` syntax,
        // so it must never reach the wire. This arm therefore stays live only for a
        // hand-built pattern serialized directly through this crate's public API (e.g. a
        // caller constructing a `GroundTerm::BlankNode` itself, or the round-trip tests
        // below); the forwarding path never feeds it one.
        GroundTerm::BlankNode(b) => {
            let _ = write!(s, "_:{}", b.as_str());
        }
    }
}

/// Emit a ground RDF 1.2 quoted triple term.
fn fmt_ground_triple(s: &mut String, t: &GroundTriple) {
    s.push_str("<<( ");
    fmt_ground_term(s, &t.subject);
    let _ = write!(s, " <{}> ", t.predicate.as_str());
    fmt_ground_term(s, &t.object);
    s.push_str(" )>>");
}

/// Emit a literal, escaping the lexical form to mirror the lexer's string rules.
fn fmt_literal(s: &mut String, l: &Literal) {
    s.push('"');
    push_escaped(s, l.value());
    s.push('"');
    match (l.language(), l.direction()) {
        (Some(lang), Some(dir)) => {
            let d = match dir {
                BaseDirection::Ltr => "ltr",
                BaseDirection::Rtl => "rtl",
            };
            let _ = write!(s, "@{lang}--{d}");
        }
        (Some(lang), None) => {
            let _ = write!(s, "@{lang}");
        }
        (None, _) => {
            let dt = l.datatype().as_str();
            // `xsd:string` and `rdf:langString` are implied; everything else is
            // explicit `^^<datatype>`.
            if dt != XSD_STRING && dt != RDF_LANG_STRING {
                let _ = write!(s, "^^<{dt}>");
            }
        }
    }
}

/// Escape a string literal's lexical content for a short `"…"` form, mirroring
/// the lexer's `lex_string` escape table (`\`, `"`, `\n`, `\r`, `\t`).
fn push_escaped(s: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '\\' => s.push_str("\\\\"),
            '"' => s.push_str("\\\""),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            '\u{0008}' => s.push_str("\\b"),
            '\u{000C}' => s.push_str("\\f"),
            other => s.push(other),
        }
    }
}

/// A `Display` shim that renders a [`Variable`] with its `?` sigil.
struct VarRef<'a>(&'a Variable);

impl core::fmt::Display for VarRef<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "?{}", self.0.as_str())
    }
}

/// Emit an expression. Binary and unary operators are conservatively
/// parenthesized so re-parse never depends on reproducing exact precedence.
///
/// This is a thin wrapper over [`fmt_expr_agg`] with no enclosing `Group` in
/// scope; every call site outside [`fmt_subselect`]'s SELECT-expression/
/// `HAVING`/`ORDER BY` rendering reaches only WHERE-body expressions, which can
/// never reference an aggregate's synthetic output variable (see
/// [`fmt_expr_agg`]'s docs).
fn fmt_expr(s: &mut String, e: &Expression) {
    fmt_expr_agg(s, e, None);
}

/// As [`fmt_expr`], but resolving any bare reference to a `Group` aggregate's
/// synthetic output variable — recursively, wherever it appears in the
/// expression tree — to that aggregate's rendered call form instead of the
/// bare variable name.
///
/// # Why this exists
///
/// The WHERE body [`fmt_subselect`] emits never binds an aggregate's synthetic
/// output variable (`__purrdf_agg_N`, minted by the parser's aggregate-lifting;
/// see `Parser::fresh_agg_var`) — the aggregate FUNCTION CALL is what binds it,
/// and that call has no surface-syntax home inside the WHERE clause itself
/// (§18.2.4's algebra evaluates `Group` strictly after the WHERE body). So a
/// projection `(expr AS ?v)`, a `HAVING(expr)`, or an `ORDER BY` key that
/// mentions one of these synthetic variables — which is exactly how the parser
/// represents `(COUNT(?x) AS ?c)`, `HAVING(COUNT(?x) > 5)`, or
/// `ORDER BY DESC(COUNT(?x))` — must have that reference resolved back to the
/// aggregate call on the way out, or the serialized query would be
/// syntactically valid but reference a variable nothing binds (the exact
/// "aggregates dropped" class of bug this module fixes, generalized to every
/// expression position an aggregate can reach, not just a bare `(AGG AS ?v)`
/// projection item).
///
/// `group` is `None` everywhere else in this module (plain `fmt_expr`) because
/// an aggregate's synthetic variable cannot escape into the WHERE body: it is
/// introduced by `Group`'s OWN aggregate list and consumed only by the
/// SELECT-expression/`HAVING`/`ORDER BY` layer sitting directly above that
/// `Group` in the modifier chain — never by a `FILTER`/`BIND` inside the body,
/// and never by a nested `EXISTS` pattern (a fresh, self-contained WHERE scope
/// reached via `fmt_group_body`, which is why the `Exists` arm below does not
/// thread `group` through).
fn fmt_expr_agg(s: &mut String, e: &Expression, group: Option<GroupSpec<'_>>) {
    if let Expression::Variable(v) = e
        && let Some((_, aggs)) = group
        && let Some((_, agg)) = aggs.iter().find(|(ov, _)| ov == v)
    {
        fmt_aggregate(s, agg);
        return;
    }
    match e {
        Expression::NamedNode(n) => {
            let _ = write!(s, "<{}>", n.as_str());
        }
        Expression::Literal(l) => fmt_literal(s, l),
        Expression::Variable(v) => {
            let _ = write!(s, "{}", VarRef(v));
        }
        Expression::Bound(v) => {
            let _ = write!(s, "BOUND({})", VarRef(v));
        }
        Expression::Or(a, b) => fmt_binop(s, a, "||", b, group),
        Expression::And(a, b) => fmt_binop(s, a, "&&", b, group),
        Expression::Equal(a, b) => fmt_binop(s, a, "=", b, group),
        Expression::SameTerm(a, b) => {
            s.push_str("sameTerm(");
            fmt_expr_agg(s, a, group);
            s.push_str(", ");
            fmt_expr_agg(s, b, group);
            s.push(')');
        }
        Expression::Greater(a, b) => fmt_binop(s, a, ">", b, group),
        Expression::GreaterOrEqual(a, b) => fmt_binop(s, a, ">=", b, group),
        Expression::Less(a, b) => fmt_binop(s, a, "<", b, group),
        Expression::LessOrEqual(a, b) => fmt_binop(s, a, "<=", b, group),
        Expression::Add(a, b) => fmt_binop(s, a, "+", b, group),
        Expression::Subtract(a, b) => fmt_binop(s, a, "-", b, group),
        Expression::Multiply(a, b) => fmt_binop(s, a, "*", b, group),
        Expression::Divide(a, b) => fmt_binop(s, a, "/", b, group),
        Expression::UnaryPlus(a) => {
            s.push_str("(+");
            fmt_expr_agg(s, a, group);
            s.push(')');
        }
        Expression::UnaryMinus(a) => {
            s.push_str("(-");
            fmt_expr_agg(s, a, group);
            s.push(')');
        }
        Expression::Not(a) => {
            s.push_str("(!");
            fmt_expr_agg(s, a, group);
            s.push(')');
        }
        Expression::In(a, list) => {
            s.push('(');
            fmt_expr_agg(s, a, group);
            s.push_str(" IN (");
            fmt_expr_list_agg(s, list, group);
            s.push_str("))");
        }
        Expression::If(c, t, e2) => {
            s.push_str("IF(");
            fmt_expr_agg(s, c, group);
            s.push_str(", ");
            fmt_expr_agg(s, t, group);
            s.push_str(", ");
            fmt_expr_agg(s, e2, group);
            s.push(')');
        }
        Expression::Coalesce(list) => {
            s.push_str("COALESCE(");
            fmt_expr_list_agg(s, list, group);
            s.push(')');
        }
        Expression::FunctionCall(func, args) => {
            fmt_function_name(s, func);
            s.push('(');
            fmt_expr_list_agg(s, args, group);
            s.push(')');
        }
        Expression::Exists(p) => {
            s.push_str("EXISTS { ");
            fmt_group_body(s, p);
            s.push_str(" }");
        }
    }
}

/// Emit `(a OP b)` with conservative parentheses.
fn fmt_binop(
    s: &mut String,
    a: &Expression,
    op: &str,
    b: &Expression,
    group: Option<GroupSpec<'_>>,
) {
    s.push('(');
    fmt_expr_agg(s, a, group);
    let _ = write!(s, " {op} ");
    fmt_expr_agg(s, b, group);
    s.push(')');
}

/// Emit a comma-separated expression list.
fn fmt_expr_list(s: &mut String, list: &[Expression]) {
    fmt_expr_list_agg(s, list, None);
}

/// As [`fmt_expr_list`], but resolving aggregate synthetic variables; see
/// [`fmt_expr_agg`].
fn fmt_expr_list_agg(s: &mut String, list: &[Expression], group: Option<GroupSpec<'_>>) {
    for (i, e) in list.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        fmt_expr_agg(s, e, group);
    }
}

/// Emit a SPARQL built-in or custom function name.
fn fmt_function_name(s: &mut String, f: &Function) {
    let name = match f {
        Function::Str => "STR",
        Function::Lang => "LANG",
        Function::LangMatches => "LANGMATCHES",
        Function::Datatype => "DATATYPE",
        Function::Iri => "IRI",
        Function::Uri => "URI",
        Function::BNode => "BNODE",
        Function::Rand => "RAND",
        Function::Abs => "ABS",
        Function::Ceil => "CEIL",
        Function::Floor => "FLOOR",
        Function::Round => "ROUND",
        Function::Concat => "CONCAT",
        Function::SubStr => "SUBSTR",
        Function::StrLen => "STRLEN",
        Function::Replace => "REPLACE",
        Function::UCase => "UCASE",
        Function::LCase => "LCASE",
        Function::EncodeForUri => "ENCODE_FOR_URI",
        Function::Contains => "CONTAINS",
        Function::StrStarts => "STRSTARTS",
        Function::StrEnds => "STRENDS",
        Function::StrBefore => "STRBEFORE",
        Function::StrAfter => "STRAFTER",
        Function::Year => "YEAR",
        Function::Month => "MONTH",
        Function::Day => "DAY",
        Function::Hours => "HOURS",
        Function::Minutes => "MINUTES",
        Function::Seconds => "SECONDS",
        Function::Timezone => "TIMEZONE",
        Function::Tz => "TZ",
        Function::Adjust => "ADJUST",
        Function::Now => "NOW",
        Function::Uuid => "UUID",
        Function::StrUuid => "STRUUID",
        Function::Md5 => "MD5",
        Function::Sha1 => "SHA1",
        Function::Sha256 => "SHA256",
        Function::Sha384 => "SHA384",
        Function::Sha512 => "SHA512",
        Function::Sha3_224 => "SHA3-224",
        Function::Sha3_256 => "SHA3-256",
        Function::Sha3_384 => "SHA3-384",
        Function::Sha3_512 => "SHA3-512",
        Function::StrLang => "STRLANG",
        Function::StrDt => "STRDT",
        Function::IsIri => "isIRI",
        Function::IsUri => "isURI",
        Function::IsBlank => "isBLANK",
        Function::IsLiteral => "isLITERAL",
        Function::IsNumeric => "isNUMERIC",
        Function::Regex => "REGEX",
        Function::Triple => "TRIPLE",
        Function::Subject => "SUBJECT",
        Function::Predicate => "PREDICATE",
        Function::Object => "OBJECT",
        Function::IsTriple => "isTRIPLE",
        Function::LangDir => "LANGDIR",
        Function::StrLangDir => "STRLANGDIR",
        Function::HasLang => "hasLANG",
        Function::HasLangDir => "hasLANGDIR",
        Function::Purrdf(call) => {
            // Emit the ORIGINAL IRI the call was parsed from (recorded in the AST
            // node). PurRDF mints no vocabulary of its own, so no namespace is ever
            // fabricated on output; re-parsing with the same ParserOptions
            // re-dispatches to the same PurrdfFn.
            let _ = write!(s, "<{}>", call.iri);
            return;
        }
        Function::Cdt(call) => {
            // The ORIGINAL IRI the call was parsed from. SEP-0009 fixes it, so this
            // is also `call.fn_kind.iri()` — writing the recorded string rather than
            // re-deriving it keeps the "emit exactly what was read" rule uniform
            // across both IRI-named function seams.
            let _ = write!(s, "<{}>", call.iri);
            return;
        }
        Function::Custom(n) => {
            let _ = write!(s, "<{}>", n.as_str());
            return;
        }
    };
    s.push_str(name);
}

/// Emit a SPARQL aggregate expression: `COUNT(*)`, `FUNC([DISTINCT] expr
/// [; SEPARATOR="…"])`, or `AGG(<iri>, [DISTINCT] arg, arg, … [; NAME=value]*)`
/// for a [`AggregateFunction::Custom`] aggregate.
///
/// Used by [`fmt_subselect`] (via [`fmt_expr_agg`]) wherever an aggregate's
/// synthetic output variable is referenced in the SELECT projection, `HAVING`,
/// or `ORDER BY` — the production SERVICE-federation path this module exists
/// for. An aggregate's own `args` are plain WHERE-body expressions (nested
/// aggregates are not legal SPARQL), so they render through the group-free
/// [`fmt_expr`].
fn fmt_aggregate(s: &mut String, agg: &AggregateExpression) {
    let AggregateExpression {
        function,
        args,
        distinct,
        ..
    } = agg;
    let name = match function {
        AggregateFunction::Count => "COUNT",
        AggregateFunction::Sum => "SUM",
        AggregateFunction::Avg => "AVG",
        AggregateFunction::Min => "MIN",
        AggregateFunction::Max => "MAX",
        AggregateFunction::Sample => "SAMPLE",
        AggregateFunction::GroupConcat => "GROUP_CONCAT",
        AggregateFunction::Custom(n) => {
            // `AGG(<iri>, [DISTINCT] arg, arg, … [; NAME=value]*)` — the
            // custom-aggregate surface (see `AggregateFunction::Custom`'s
            // docs); the IRI is the FIRST positional argument, not a call
            // prefix. `scalarvals` — populated ONLY by `parse_agg_scalarvals`
            // for a `Custom` aggregate (a built-in's own scalarvals, e.g.
            // GROUP_CONCAT's SEPARATOR, are rendered by the shared tail below,
            // never here) — round-trips through `; NAME=value` clauses in the
            // SAME order they were parsed, so a query that never wrote one
            // never emits one either.
            let _ = write!(s, "AGG(<{}>, ", n.as_str());
            if *distinct {
                s.push_str("DISTINCT ");
            }
            fmt_expr_list(s, args);
            for (name, value) in &agg.scalarvals {
                s.push_str("; ");
                s.push_str(name);
                s.push('=');
                fmt_literal(s, value);
            }
            s.push(')');
            return;
        }
    };
    s.push_str(name);
    s.push('(');
    if *distinct {
        s.push_str("DISTINCT ");
    }
    if args.is_empty() {
        // The spec's empty exprlist: COUNT(*) / COUNT(DISTINCT *).
        s.push('*');
    } else {
        fmt_expr_list(s, args);
    }
    if let Some(sep) = agg.separator() {
        s.push_str("; SEPARATOR=\"");
        push_escaped(s, sep);
        s.push('"');
    }
    s.push(')');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Query;
    use crate::algebra::AggregateExpressionError;
    use crate::parser::{ParserOptions, SparqlParser};

    /// Parse a full query and return its root pattern.
    fn pattern_of(query: &str) -> GraphPattern {
        let gm = "PREFIX purrdf: <https://example.org/ext/>\n".to_owned();
        let gm = gm.as_str();
        match SparqlParser::new()
            .parse_query(&format!("{gm}{query}"))
            .unwrap_or_else(|e| panic!("parse `{query}`: {e:?}"))
        {
            Query::Select { pattern, .. } => pattern,
            other => panic!("expected SELECT, got {other:?}"),
        }
    }

    /// Strip exactly one outer `Project` (the `SELECT …` scaffold) to recover the
    /// WHERE body — the shape a SERVICE node forwards and
    /// `pattern_to_select_query` consumes. The parser expands `SELECT *` to an
    /// explicit variable list, so the strip is unconditional.
    fn where_body(p: &GraphPattern) -> GraphPattern {
        match p {
            GraphPattern::Project { inner, .. } => (**inner).clone(),
            other => other.clone(),
        }
    }

    /// Assert that serializing the WHERE body then re-parsing reproduces the same
    /// algebra (round-trip stability) — exactly the SERVICE forward path.
    /// Returns the serialized text for callers that also want to inspect it.
    fn assert_roundtrip(query: &str) -> String {
        let body = where_body(&pattern_of(query));
        let text = pattern_to_select_query(&body);
        let reparsed = match SparqlParser::new()
            .parse_query(&text)
            .unwrap_or_else(|e| panic!("re-parse `{text}`: {e:?}"))
        {
            Query::Select { pattern, .. } => pattern,
            other => panic!("expected SELECT, got {other:?}"),
        };
        let reparsed_body = where_body(&reparsed);
        assert_eq!(
            reparsed_body, body,
            "round-trip mismatch for `{query}`\n serialized: {text}"
        );
        text
    }

    #[test]
    fn roundtrip_bgp() {
        assert_roundtrip("SELECT * WHERE { ?s <http://ex/p> ?o }");
    }

    #[test]
    fn roundtrip_multi_triple_bgp() {
        assert_roundtrip("SELECT * WHERE { ?s <http://ex/p> ?o . ?o <http://ex/q> ?z }");
    }

    #[test]
    fn roundtrip_optional() {
        assert_roundtrip("SELECT * WHERE { ?s <http://ex/p> ?o OPTIONAL { ?o <http://ex/q> ?z } }");
    }

    #[test]
    fn roundtrip_union() {
        assert_roundtrip(
            "SELECT * WHERE { { ?s <http://ex/p> ?o } UNION { ?s <http://ex/q> ?o } }",
        );
    }

    #[test]
    fn roundtrip_filter_and_bind() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/age> ?a FILTER(?a > 18) BIND((?a + 1) AS ?b) }",
        );
    }

    #[test]
    fn roundtrip_minus() {
        assert_roundtrip("SELECT * WHERE { ?s <http://ex/p> ?o MINUS { ?s <http://ex/q> ?o } }");
    }

    #[test]
    fn roundtrip_graph() {
        assert_roundtrip("SELECT * WHERE { GRAPH ?g { ?s <http://ex/p> ?o } }");
    }

    #[test]
    fn roundtrip_path() {
        assert_roundtrip("SELECT * WHERE { ?s <http://ex/p>+ ?o }");
    }

    #[test]
    fn roundtrip_values() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> ?o VALUES (?o) { (<http://ex/a>) (UNDEF) } }",
        );
    }

    #[test]
    fn roundtrip_typed_literal() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> }",
        );
    }

    #[test]
    fn roundtrip_lang_literal() {
        assert_roundtrip("SELECT * WHERE { ?s <http://ex/p> \"hi\"@en }");
    }

    #[test]
    fn roundtrip_quoted_triple() {
        assert_roundtrip(
            "SELECT ?r WHERE { ?r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( ?s ?p ?o )>> }",
        );
    }

    #[test]
    fn roundtrip_exists() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> ?o FILTER EXISTS { ?o <http://ex/q> ?z } }",
        );
    }

    #[test]
    fn roundtrip_nested_service() {
        assert_roundtrip("SELECT * WHERE { SERVICE <http://ep/sparql> { ?s <http://ex/p> ?o } }");
    }

    #[test]
    fn roundtrip_service_silent() {
        assert_roundtrip(
            "SELECT * WHERE { SERVICE SILENT <http://ep/sparql> { ?s <http://ex/p> ?o } }",
        );
    }

    #[test]
    fn roundtrip_adjust() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> ?dt \
             FILTER(ADJUST(?dt, \"PT1H\"^^<http://www.w3.org/2001/XMLSchema#dayTimeDuration>) = ?dt) }",
        );
    }

    #[test]
    fn roundtrip_subselect_distinct_limit() {
        assert_roundtrip(
            "SELECT * WHERE { { SELECT DISTINCT ?s WHERE { ?s <http://ex/p> ?o } ORDER BY ?s LIMIT 5 OFFSET 2 } }",
        );
    }

    // ── aggregate round-trip pins (the production SERVICE-federation path) ───
    //
    // `pattern_to_select_query` is called in production ONLY on a `SERVICE`
    // node's `inner` pattern (`sparql-eval`'s `remote.rs`), and a `SERVICE`
    // body that contains `GROUP BY`/an aggregate can only get there by being a
    // nested `{ SELECT ... }` sub-select (the SPARQL grammar has no way to write
    // `GROUP BY` directly inside a bare `{ GroupGraphPatternSub }`) — so each
    // aggregate form here is pinned the same way `roundtrip_service_grouped_
    // aggregate` below pins the production path itself: wrapped as a nested
    // sub-select, exactly mirroring `roundtrip_subselect_distinct_limit`'s
    // existing convention for non-aggregate solution modifiers. Each parses,
    // renders through `pattern_to_select_query`, re-parses, and asserts the
    // re-parsed algebra equals the original — proving the aggregate survives
    // the production serializer rather than being silently dropped.
    fn assert_subselect_roundtrip(inner_select_query: &str) {
        assert_roundtrip(&format!("SELECT * WHERE {{ {{ {inner_select_query} }} }}"));
    }

    #[test]
    fn roundtrip_count_star() {
        assert_subselect_roundtrip("SELECT ?t (COUNT(*) AS ?c) WHERE { ?x a ?t } GROUP BY ?t");
    }

    #[test]
    fn roundtrip_count_distinct() {
        assert_subselect_roundtrip(
            "SELECT ?t (COUNT(DISTINCT ?x) AS ?c) WHERE { ?x a ?t } GROUP BY ?t",
        );
    }

    #[test]
    fn roundtrip_sum() {
        assert_subselect_roundtrip(
            "SELECT ?t (SUM(?n) AS ?s) WHERE { ?x a ?t ; <http://ex/n> ?n } GROUP BY ?t",
        );
    }

    #[test]
    fn roundtrip_min_max_avg_sample() {
        assert_subselect_roundtrip(
            "SELECT ?t (MIN(?n) AS ?mn) (MAX(?n) AS ?mx) (AVG(?n) AS ?a) (SAMPLE(?n) AS ?sm) \
             WHERE { ?x a ?t ; <http://ex/n> ?n } GROUP BY ?t",
        );
    }

    #[test]
    fn roundtrip_group_concat_no_separator() {
        assert_subselect_roundtrip(
            "SELECT ?t (GROUP_CONCAT(?n) AS ?g) WHERE { ?x a ?t ; <http://ex/n> ?n } GROUP BY ?t",
        );
    }

    #[test]
    fn roundtrip_group_concat_with_separator() {
        assert_subselect_roundtrip(
            "SELECT ?t (GROUP_CONCAT(?n; SEPARATOR=\"|\") AS ?g) WHERE { ?x a ?t ; <http://ex/n> ?n } GROUP BY ?t",
        );
    }

    #[test]
    fn roundtrip_agg_custom_single_arg() {
        assert_subselect_roundtrip(
            "SELECT ?t (AGG(<http://ex/myAgg>, ?n) AS ?a) WHERE { ?x a ?t ; <http://ex/n> ?n } GROUP BY ?t",
        );
    }

    #[test]
    fn roundtrip_agg_custom_distinct_multi_arg() {
        assert_subselect_roundtrip(
            "SELECT ?t (AGG(<http://ex/myAgg>, DISTINCT ?n, ?m) AS ?a) \
             WHERE { ?x a ?t ; <http://ex/n> ?n ; <http://ex/m> ?m } GROUP BY ?t",
        );
    }

    /// The gap this increment closes: a NAMED scalarval on a custom aggregate
    /// (`AGG(<iri>, …; NAME=value)`) must survive parse → serialize → parse —
    /// the production SERVICE-federation path — with an EQUAL algebra, not just
    /// equal text. Before this fix `fmt_aggregate`'s `Custom` branch dropped
    /// `scalarvals` entirely, so this exact case silently lost data across a
    /// SERVICE forward.
    #[test]
    fn roundtrip_agg_custom_single_scalarval() {
        assert_subselect_roundtrip(
            "SELECT ?t (AGG(<http://ex/myAgg>, ?n; P=0.95) AS ?a) \
             WHERE { ?x a ?t ; <http://ex/n> ?n } GROUP BY ?t",
        );
    }

    #[test]
    fn roundtrip_agg_custom_multiple_scalarvals() {
        assert_subselect_roundtrip(
            "SELECT ?t (AGG(<http://ex/myAgg>, DISTINCT ?n; K=3; LABEL=\"top\") AS ?a) \
             WHERE { ?x a ?t ; <http://ex/n> ?n } GROUP BY ?t",
        );
    }

    /// `NAME` is matched case-insensitively and normalized upper-case, so a
    /// lower-case spelling in the query text still round-trips to an EQUAL
    /// algebra (the re-parsed key is the same upper-cased form either way).
    #[test]
    fn roundtrip_agg_custom_scalarval_name_is_case_normalized() {
        assert_subselect_roundtrip(
            "SELECT ?t (AGG(<http://ex/myAgg>, ?n; p=0.5) AS ?a) \
             WHERE { ?x a ?t ; <http://ex/n> ?n } GROUP BY ?t",
        );
    }

    #[test]
    fn roundtrip_having_referencing_aggregate() {
        // The HAVING clause references the aggregate's synthetic output variable
        // (COUNT(?x)), not just the SELECT projection — exercising the same
        // resolve-on-the-way-out fix for a second expression position.
        assert_subselect_roundtrip(
            "SELECT ?t (COUNT(?x) AS ?c) WHERE { ?x a ?t } GROUP BY ?t HAVING(COUNT(?x) > 1)",
        );
    }

    #[test]
    fn roundtrip_order_by_referencing_aggregate() {
        assert_subselect_roundtrip(
            "SELECT ?t (COUNT(?x) AS ?c) WHERE { ?x a ?t } GROUP BY ?t ORDER BY DESC(COUNT(?x))",
        );
    }

    #[test]
    fn roundtrip_service_grouped_aggregate() {
        // The exact hole this module's `fmt_subselect` fix closes: a grouped +
        // aggregated pattern forwarded to a federated SPARQL endpoint via SERVICE.
        // Before the fix, the aggregate function itself was dropped, leaving a
        // `(?__purrdf_agg_N AS ?c)` reference to a variable nothing in the
        // forwarded query text binds.
        assert_roundtrip(
            "SELECT * WHERE { SERVICE <http://ep/sparql> { \
             SELECT ?t (COUNT(?x) AS ?c) WHERE { ?x a ?t } GROUP BY ?t } }",
        );
    }

    // ── the DIRECT (un-doubled) aggregate shape `pattern_to_select_query`'s own
    //    doctest exercises: a caller peels exactly ONE outer `Project`, same as
    //    every `assert_roundtrip` case above, with NO extra nested-subselect wrapper.
    //    An aggregate query with no further modifier (`Distinct`/`Slice`/`OrderBy`/
    //    an explicit outer `Project` of its own) around it lands here as an
    //    `Extend`-over-`Group` chain, which used to reach `fmt_group_body`'s ordinary
    //    (aggregate-unaware) `Extend` handling instead of `fmt_subselect` — silently
    //    dropping the aggregate behind a `BIND` referencing a variable nothing
    //    upstream binds, or — for an explicit `GROUP BY` — producing `SELECT *` over
    //    an aggregate query, which SPARQL does not admit at all. ──────────────────

    /// The implicit whole-table group (no `GROUP BY` at all): the exact repro this
    /// fix closes. Before it, this rendered as
    /// `SELECT * WHERE { { SELECT * WHERE { … } } BIND(?__purrdf_agg_0 AS ?s) }` —
    /// the `SUM` silently dropped.
    #[test]
    fn direct_roundtrip_implicit_group_no_group_by() {
        assert_roundtrip("SELECT (SUM(?v) AS ?s) WHERE { ?x <http://ex/v> ?v }");
    }

    /// The explicit `GROUP BY` variant: before this fix, the SAME `Extend`-over-
    /// `Group` shape rendered `SELECT *` over an aggregate query, which the parser
    /// then refused to re-parse (`SELECT * is not allowed in an aggregate query`).
    #[test]
    fn direct_roundtrip_explicit_group_by() {
        assert_roundtrip(
            "SELECT ?t (SUM(?v) AS ?s) WHERE { ?x a ?t ; <http://ex/v> ?v } GROUP BY ?t",
        );
    }

    /// The `HAVING` clause referencing the aggregate's synthetic output variable,
    /// rendered directly (no double-nesting) — the `Filter`-directly-over-`Group`
    /// half of the shape this fix recognizes.
    #[test]
    fn direct_roundtrip_having() {
        assert_roundtrip(
            "SELECT ?t (COUNT(?x) AS ?c) WHERE { ?x a ?t } GROUP BY ?t HAVING(COUNT(?x) > 1)",
        );
    }

    /// `HAVING (a) (b)` — TWO conditions — lifts to a `Filter`-over-`Filter`-over-
    /// `Group` chain, not a single `Filter` directly over `Group`. The corpus
    /// round-trip sweep (`crates/sparql-algebra/tests/serializer_roundtrip_sweep.rs`)
    /// caught this: `extend_chain_reaches_group`/`fmt_subselect`'s peel loop each
    /// only checked the OUTERMOST `Filter`'s immediate `inner`, so a second
    /// `HAVING` condition made both silently mis-detect the whole shape — the
    /// SAME failure mode `direct_roundtrip_explicit_group_by`'s doc names,
    /// reachable through one more `HAVING` clause than that fix's own tests
    /// exercised.
    #[test]
    fn direct_roundtrip_multiple_having() {
        assert_roundtrip(
            "SELECT ?t (COUNT(?x) AS ?c) WHERE { ?x a ?t } GROUP BY ?t \
             HAVING(COUNT(?x) > 1)(COUNT(?x) < 100)",
        );
    }

    // NOTE: an `ORDER BY` above the aggregate chain is `is_subselect_node`-true
    // ALREADY (`OrderBy` is one of that predicate's original members, untouched by
    // this fix) and takes the PRE-EXISTING nested-subselect path
    // `roundtrip_order_by_referencing_aggregate` above pins via
    // `assert_subselect_roundtrip` — that path was never the "aggregate silently
    // dropped" defect this fix closes (it reparses to a semantically identical,
    // merely one-level-more-nested query), so it is not repeated here.

    /// Multiple aggregates, and no `GROUP BY` key, rendered directly — proving the
    /// fix is not a one-aggregate special case.
    #[test]
    fn direct_roundtrip_multiple_aggregates_no_group_by() {
        assert_roundtrip(
            "SELECT (COUNT(?x) AS ?c) (SUM(?v) AS ?s) \
             WHERE { ?x <http://ex/v> ?v }",
        );
    }

    /// `pattern_to_select_query`'s rendering of the implicit-group repro is a
    /// syntactically complete, standalone query with the aggregate call itself
    /// present in the text (not a dangling reference to its synthetic variable).
    #[test]
    fn direct_render_of_implicit_group_names_the_aggregate_call() {
        let body = where_body(&pattern_of(
            "SELECT (SUM(?v) AS ?s) WHERE { ?x <http://ex/v> ?v }",
        ));
        let text = pattern_to_select_query(&body);
        assert!(
            text.contains("SUM(?v)"),
            "the aggregate call must appear in the rendered text: {text}"
        );
        assert!(
            !text.contains("__purrdf_agg"),
            "no synthetic aggregate variable may leak into the rendered text: {text}"
        );
    }

    #[test]
    fn produces_complete_select() {
        let p = pattern_of("SELECT * WHERE { ?s <http://ex/p> ?o }");
        let text = pattern_to_select_query(&p);
        assert!(text.starts_with("SELECT * WHERE {"), "got: {text}");
        assert!(text.contains("<http://ex/p>"), "got: {text}");
    }

    /// A subselect that mixes a plain projected variable and a SELECT expression
    /// (`(expr AS ?v)`) must not duplicate the AS-target var in the projection
    /// list. Before the fix, parsing `SELECT ?s (?o + 1 AS ?x) WHERE { … }`
    /// pushed `?x` into both `projected` and `select_exprs`, so the serializer
    /// emitted `SELECT ?s ?x (?o + 1 AS ?x)` — invalid SPARQL 1.1 (double projection).
    #[test]
    fn subselect_select_expr_no_duplicate_projection() {
        // Build a subselect that has a plain var (?s) and an AS-expression (?x).
        // The subselect is embedded so `fmt_subselect` is exercised.
        let query = "SELECT * WHERE { { SELECT ?s (?o + 1 AS ?x) WHERE { ?s <http://ex/p> ?o } } }";
        let body = where_body(&pattern_of(query));
        let text = pattern_to_select_query(&body);

        // The AS-target ?x must appear exactly once, only inside `(… AS ?x)`.
        let count_bare_x = text.split_whitespace().filter(|tok| *tok == "?x").count();
        assert_eq!(
            count_bare_x, 0,
            "?x must not appear as a bare projected var; got: {text}"
        );
        assert!(
            text.contains("AS ?x)"),
            "?x must still appear in AS-expression form; got: {text}"
        );
        // The plain projected var ?s must still appear.
        assert!(
            text.split_whitespace().any(|t| t == "?s"),
            "?s must appear as a plain projected var; got: {text}"
        );
        // Round-trip: the serialized text must parse without error.
        SparqlParser::new()
            .parse_query(&text)
            .unwrap_or_else(|e| panic!("re-parse of `{text}` failed: {e:?}"));
    }

    #[test]
    fn aggregate_renders_group_concat_separator() {
        let agg = AggregateExpression::new(
            AggregateFunction::GroupConcat,
            vec![Expression::Variable(Variable::new("x"))],
            vec![("separator".to_owned(), Literal::new_simple("|"))],
            false,
        )
        .unwrap();
        let mut s = String::new();
        fmt_aggregate(&mut s, &agg);
        assert_eq!(s, "GROUP_CONCAT(?x; SEPARATOR=\"|\")");
    }

    #[test]
    fn aggregate_renders_count_star() {
        let agg = AggregateExpression::new(AggregateFunction::Count, Vec::new(), Vec::new(), true)
            .unwrap();
        let mut s = String::new();
        fmt_aggregate(&mut s, &agg);
        assert_eq!(s, "COUNT(DISTINCT *)");
    }

    #[test]
    fn aggregate_renders_custom_agg_call() {
        let agg = AggregateExpression::new(
            AggregateFunction::Custom(crate::ast::NamedNode::new_unchecked("http://ex/myAgg")),
            vec![
                Expression::Variable(Variable::new("a")),
                Expression::Variable(Variable::new("b")),
            ],
            Vec::new(),
            true,
        )
        .unwrap();
        let mut s = String::new();
        fmt_aggregate(&mut s, &agg);
        assert_eq!(s, "AGG(<http://ex/myAgg>, DISTINCT ?a, ?b)");
    }

    #[test]
    fn aggregate_renders_custom_agg_scalarval() {
        let agg = AggregateExpression::new(
            AggregateFunction::Custom(crate::ast::NamedNode::new_unchecked("http://ex/myAgg")),
            vec![Expression::Variable(Variable::new("v"))],
            vec![(
                "P".to_owned(),
                Literal::new_typed(
                    "0.95",
                    crate::ast::NamedNode::new_unchecked(
                        "http://www.w3.org/2001/XMLSchema#decimal",
                    ),
                ),
            )],
            false,
        )
        .unwrap();
        let mut s = String::new();
        fmt_aggregate(&mut s, &agg);
        assert_eq!(
            s,
            "AGG(<http://ex/myAgg>, ?v; P=\"0.95\"^^<http://www.w3.org/2001/XMLSchema#decimal>)"
        );
    }

    #[test]
    fn aggregate_expression_new_rejects_empty_args_for_non_count() {
        // Defense in depth for the type-level invariant this module's
        // `fmt_aggregate` (and `sparql-eval`'s dispatch) both rely on: only
        // `COUNT` may ever carry an empty `args`.
        for function in [
            AggregateFunction::Sum,
            AggregateFunction::Avg,
            AggregateFunction::Min,
            AggregateFunction::Max,
            AggregateFunction::Sample,
            AggregateFunction::GroupConcat,
            AggregateFunction::Custom(crate::ast::NamedNode::new_unchecked("http://ex/myAgg")),
        ] {
            let err = AggregateExpression::new(function.clone(), Vec::new(), Vec::new(), false)
                .unwrap_err();
            assert_eq!(err.function(), &function);
        }
        assert!(
            AggregateExpression::new(AggregateFunction::Count, Vec::new(), Vec::new(), false)
                .is_ok()
        );
    }

    /// The other half of the checked constructor: a `scalarvals` key the function
    /// does not admit is refused too, not just an empty `args`. Every built-in but
    /// `GroupConcat` admits NO key at all — this is what makes handing
    /// `fmt_aggregate`'s built-in tail a `SUM`/`AVG`/`MIN`/`MAX`/`SAMPLE` carrying a
    /// `"separator"` entry (which would render `SUM(?v; SEPARATOR="…")`, not SPARQL
    /// grammar for anything) unrepresentable.
    #[test]
    fn aggregate_expression_new_rejects_a_scalarval_key_the_function_does_not_admit() {
        let bogus_separator = vec![("separator".to_owned(), Literal::new_simple("|"))];
        for function in [
            AggregateFunction::Sum,
            AggregateFunction::Avg,
            AggregateFunction::Min,
            AggregateFunction::Max,
            AggregateFunction::Sample,
        ] {
            let err = AggregateExpression::new(
                function.clone(),
                vec![Expression::Variable(Variable::new("v"))],
                bogus_separator.clone(),
                false,
            )
            .unwrap_err();
            assert_eq!(err.function(), &function);
            assert!(
                matches!(err, AggregateExpressionError::Scalarval(_)),
                "must be the Scalarval arm, not Arity: {err:?}"
            );
        }
        // `COUNT` admits no scalarval key either, but it is the one function whose
        // `args` MAY also be empty — cover it with a non-empty `args` so this test
        // isolates the scalarval check from the arity check.
        let err = AggregateExpression::new(
            AggregateFunction::Count,
            vec![Expression::Variable(Variable::new("v"))],
            bogus_separator,
            false,
        )
        .unwrap_err();
        assert!(matches!(err, AggregateExpressionError::Scalarval(_)));
    }

    /// `GroupConcat` is the one built-in that DOES admit a scalarval key
    /// (`"separator"`) — the positive half of the check the previous test pins the
    /// negative half of.
    #[test]
    fn aggregate_expression_new_accepts_group_concats_separator() {
        assert!(
            AggregateExpression::new(
                AggregateFunction::GroupConcat,
                vec![Expression::Variable(Variable::new("v"))],
                vec![("separator".to_owned(), Literal::new_simple("|"))],
                false,
            )
            .is_ok()
        );
    }

    /// `AggregateFunction::Custom` admits ANY scalarval key structurally — the
    /// closed check against a specific registered aggregate's own declaration is a
    /// `sparql-eval` prepare-time concern, not this crate's (see the struct docs).
    #[test]
    fn aggregate_expression_new_accepts_any_scalarval_key_for_a_custom_aggregate() {
        assert!(
            AggregateExpression::new(
                AggregateFunction::Custom(crate::ast::NamedNode::new_unchecked("http://ex/myAgg")),
                vec![Expression::Variable(Variable::new("v"))],
                vec![("ANYTHING_AT_ALL".to_owned(), Literal::new_simple("x"))],
                false,
            )
            .is_ok()
        );
    }

    // ── property functions ────────────────────────────────────────────────────

    /// The caller-configured property-function namespace these tests use.
    const PF_NS: &str = "https://example.org/pf/";

    /// A prologue binding `pf:` to [`PF_NS`] and `ex:` to a data namespace.
    const PF_PROLOGUE: &str =
        "PREFIX pf: <https://example.org/pf/>\nPREFIX ex: <https://example.org/d/>\n";

    /// Options with [`PF_NS`] configured as a property-function namespace.
    fn pf_options() -> ParserOptions {
        ParserOptions {
            extension_fn_namespaces: Vec::new(),
            property_fn_namespaces: vec![PF_NS.to_owned()],
            property_fn_iris: Vec::new(),
        }
    }

    /// Parse a SELECT under [`pf_options`] and return its WHERE body.
    fn pf_body(query: &str) -> GraphPattern {
        let text = format!("{PF_PROLOGUE}{query}");
        match SparqlParser::new()
            .parse_query_with(&text, &pf_options())
            .unwrap_or_else(|e| panic!("parse `{text}`: {e:?}"))
        {
            Query::Select { pattern, .. } => where_body(&pattern),
            other => panic!("expected SELECT, got {other:?}"),
        }
    }

    /// Serialize the WHERE body of `query` and re-parse it under the SAME
    /// options: the algebra must come back identical, and the emitted text must
    /// carry the byte-exact predicate IRI. Returns the serialized text.
    fn assert_pf_roundtrip(query: &str) -> String {
        let body = pf_body(query);
        let text = pattern_to_select_query(&body);
        let reparsed = match SparqlParser::new()
            .parse_query_with(&text, &pf_options())
            .unwrap_or_else(|e| panic!("re-parse `{text}`: {e:?}"))
        {
            Query::Select { pattern, .. } => where_body(&pattern),
            other => panic!("expected SELECT, got {other:?}"),
        };
        assert_eq!(
            reparsed, body,
            "round-trip mismatch for `{query}`\n serialized: {text}"
        );
        text
    }

    #[test]
    fn roundtrip_property_function_unary() {
        let text = assert_pf_roundtrip("SELECT * WHERE { ?s pf:related ?o }");
        assert!(
            text.contains(&format!("?s <{PF_NS}related> ?o .")),
            "a 1-ary side emits the bare term; got: {text}"
        );
    }

    #[test]
    fn roundtrip_property_function_n_ary_object() {
        let text = assert_pf_roundtrip("SELECT * WHERE { ?s pf:solve ( ?a ?b ?c ) }");
        assert!(
            text.contains("( ?a ?b ?c )"),
            "a multi-element side emits collection syntax; got: {text}"
        );
    }

    #[test]
    fn roundtrip_property_function_n_ary_subject() {
        let text = assert_pf_roundtrip("SELECT * WHERE { ( ?a ?b ) pf:solve ?o }");
        assert!(text.contains("( ?a ?b )"), "got: {text}");
    }

    #[test]
    fn roundtrip_property_function_empty_side() {
        let text = assert_pf_roundtrip("SELECT * WHERE { () pf:solve ( ?o ) }");
        assert!(
            text.contains("() <"),
            "a zero-length side emits `()`; got: {text}"
        );
    }

    #[test]
    fn roundtrip_property_function_literal_args() {
        assert_pf_roundtrip(
            "SELECT * WHERE { ?s pf:solve ( \"purr\" \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> \"hi\"@en ) }",
        );
    }

    #[test]
    fn roundtrip_property_function_quoted_triple_arg() {
        assert_pf_roundtrip("SELECT * WHERE { ?s pf:solve <<( ?a ex:p ?b )>> }");
    }

    #[test]
    fn roundtrip_property_function_blank_arg() {
        assert_pf_roundtrip("SELECT * WHERE { _:b pf:solve ( _:c ?o ) }");
    }

    #[test]
    fn roundtrip_property_function_repeated_vars() {
        assert_pf_roundtrip("SELECT * WHERE { ( ?x ?x ) pf:solve ( ?x ?y ) }");
    }

    #[test]
    fn roundtrip_property_function_iri_and_nil_args() {
        // A bare rdf:nil argument stays a one-element vector across the trip
        // (it must not collapse into the empty `()` spelling).
        let text = assert_pf_roundtrip(
            "SELECT * WHERE { <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> pf:solve ex:o }",
        );
        assert!(
            !text.contains("() <"),
            "an explicit rdf:nil must not be re-emitted as the empty list; got: {text}"
        );
    }

    #[test]
    fn roundtrip_property_functions_mixed_with_data_triples() {
        let text = assert_pf_roundtrip(
            "SELECT * WHERE { ?s ex:name ?n . ?s pf:first ?a . ?a ex:p ?q . ( ?a ?q ) pf:second ( ) }",
        );
        assert!(text.contains(&format!("<{PF_NS}first>")), "got: {text}");
        assert!(text.contains(&format!("<{PF_NS}second>")), "got: {text}");
    }

    #[test]
    fn property_function_serializes_without_a_lateral_keyword() {
        // The node is written as a triple; the LATERAL scaffold the parser
        // builds around it is implicit in that surface form. `LATERAL` DOES
        // have a parser production now (SEP-0006) — so this is no longer a
        // "would fail to parse" guard: emitting the keyword here would PARSE
        // successfully, just into a double-nested `Lateral` (the braced RHS
        // parses to its own PF-triple `Lateral` first, rooted at the unit-table
        // left, and the explicit keyword would wrap that again), silently
        // producing the WRONG tree rather than failing loudly. This guard pins
        // the unwrapped rendering that keeps the round-trip tree-identical.
        let text = assert_pf_roundtrip("SELECT * WHERE { ?s ex:p ?o . ?s pf:related ?o }");
        assert!(!text.contains("LATERAL"), "got: {text}");
    }

    #[test]
    fn roundtrip_property_function_in_a_nested_group() {
        // A call reached through a braced sub-group is joined onto the outer
        // triples rather than folded into their lateral chain; the serializer
        // must keep that grouping so the re-parse does not re-associate.
        let text = assert_pf_roundtrip("SELECT * WHERE { ?s ex:p ?o . { ?x pf:solve ?y } }");
        assert!(
            text.contains('{'),
            "the nested group must survive; got: {text}"
        );
        assert_pf_roundtrip("SELECT * WHERE { ?s ex:p ?o . { ?x pf:a ?y } ?m pf:b ?n }");
        assert_pf_roundtrip(
            "SELECT * WHERE { ?s pf:a ?o OPTIONAL { ?o pf:b ?z } MINUS { ?s ex:q ?o } }",
        );
        assert_pf_roundtrip(
            "SELECT * WHERE { { ?s pf:a ?o } UNION { ?s pf:b ?o } FILTER(?o > 1) }",
        );
        assert_pf_roundtrip("SELECT * WHERE { GRAPH ?g { ?s pf:a ?o } ?s ex:p ?o }");
    }

    // ── LATERAL ──────────────────────────────────────────────────────────────

    #[test]
    fn roundtrip_lateral() {
        assert_roundtrip("SELECT * WHERE { ?s <http://ex/p> ?o LATERAL { ?o <http://ex/q> ?z } }");
    }

    #[test]
    fn roundtrip_lateral_chain_shapes() {
        // Left-deep: `A LATERAL {B} LATERAL {C}` parses as
        // `Lateral{Lateral{A,B},C}`.
        let left_deep = "SELECT * WHERE { ?s <http://ex/p> ?o LATERAL { ?o <http://ex/q> ?z } \
             LATERAL { ?z <http://ex/r> ?w } }";
        assert_roundtrip(left_deep);
        // Right-nested: `A LATERAL { B LATERAL {C} }` parses as
        // `Lateral{A,Lateral{B,C}}`.
        let right_nested = "SELECT * WHERE { ?s <http://ex/p> ?o \
             LATERAL { ?o <http://ex/q> ?z LATERAL { ?z <http://ex/r> ?w } } }";
        assert_roundtrip(right_nested);
        // LATERAL is not associative like JOIN: the two shapes must stay
        // DISTINCT trees across the round-trip rather than re-associating into
        // one another.
        assert_ne!(
            where_body(&pattern_of(left_deep)),
            where_body(&pattern_of(right_nested)),
            "the two chain shapes must remain distinct"
        );
    }

    #[test]
    fn roundtrip_lateral_as_a_join_right_operand() {
        // `A . { B LATERAL { C } }` parses to `Join{A, Lateral{B, C}}` — a
        // `Lateral` sitting as a `Join`'s right operand, reached through a
        // nested group rather than the top-level LATERAL-attaches-to-the-
        // preceding-pattern rule.
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> ?o . \
             { ?a <http://ex/x> ?b LATERAL { ?b <http://ex/y> ?c } } }",
        );
    }

    #[test]
    fn roundtrip_variable_endpoint_service() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> ?g SERVICE ?g { ?x <http://ex/q> ?y } }",
        );
    }

    #[test]
    fn variable_endpoint_service_serializes_without_a_lateral_keyword() {
        // The parser's own `SERVICE` dispatch arm auto-wraps a variable
        // endpoint into a `Lateral` node without any `LATERAL` keyword in the
        // text; emitting the keyword here would double-nest on re-parse.
        let body = where_body(&pattern_of(
            "SELECT * WHERE { ?s <http://ex/p> ?g SERVICE ?g { ?x <http://ex/q> ?y } }",
        ));
        let text = pattern_to_select_query(&body);
        assert!(!text.contains("LATERAL"), "got: {text}");
    }

    #[test]
    fn roundtrip_lateral_with_a_subselect_right_hand_side() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> ?o \
             LATERAL { SELECT ?z WHERE { ?o <http://ex/q> ?z } ORDER BY ?z LIMIT 1 } }",
        );
    }

    #[test]
    fn roundtrip_lateral_with_an_empty_left() {
        assert_roundtrip("SELECT * WHERE { LATERAL { ?s <http://ex/p> ?o } }");
    }

    #[test]
    fn roundtrip_lateral_with_a_fixed_iri_service_right() {
        // A fixed-IRI `SERVICE` under an explicit `LATERAL {}` is a shape the
        // parser's `SERVICE` arm does NOT auto-wrap (only a variable endpoint
        // does) — so the `LATERAL` keyword MUST be emitted here, or the
        // laterality is lost on re-parse as a plain `Join`.
        let text = assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> ?o \
             LATERAL { SERVICE <http://ep/sparql> { ?o <http://ex/q> ?z } } }",
        );
        assert!(text.contains("LATERAL"), "got: {text}");
    }

    #[test]
    fn roundtrip_lateral_with_a_property_function_right() {
        // The natural PF-triple fold (`A . A2 pf:call ?x`) builds a `Lateral`
        // whose right operand is the bare `PropertyFunction` node directly;
        // the parser's own PF-triple-folding loop rebuilds exactly this shape
        // from the unwrapped triple, so the `LATERAL` keyword must NOT be
        // emitted.
        let text = assert_pf_roundtrip("SELECT * WHERE { ?s ex:p ?o . ?o pf:related ?z }");
        assert!(!text.contains("LATERAL"), "got: {text}");
    }

    /// Collapse a `Lateral` node whose LEFT is the identity/unit table
    /// (`Bgp { patterns: [] }`) into its right operand alone — a lateral join
    /// against a table holding exactly one, empty solution evaluates its right
    /// operand exactly once with no extra bindings, i.e. is the right operand
    /// — recursively, so it also cancels a unit-table `Lateral` reached
    /// through other combinators. This is what an explicit `LATERAL { … }`
    /// keyword necessarily reintroduces around a bare property-function
    /// triple (the braced body's OWN triples-block fold roots the call at a
    /// fresh unit table before the outer keyword wraps it again), so
    /// tree-identity across that keyword is only available up to this
    /// cancellation. Every other `GraphPattern` variant is reconstructed with
    /// its children normalized the same way and every non-pattern field
    /// carried through unchanged; the match is exhaustive so a future algebra
    /// variant is a compile error here, not a silent blind spot.
    fn normalize(p: &GraphPattern) -> GraphPattern {
        match p {
            GraphPattern::Lateral { left, right } => {
                let left = normalize(left);
                let right = normalize(right);
                if matches!(&left, GraphPattern::Bgp { patterns } if patterns.is_empty()) {
                    right
                } else {
                    GraphPattern::Lateral {
                        left: Box::new(left),
                        right: Box::new(right),
                    }
                }
            }
            GraphPattern::Bgp { patterns } => GraphPattern::Bgp {
                patterns: patterns.clone(),
            },
            GraphPattern::Path {
                subject,
                path,
                object,
            } => GraphPattern::Path {
                subject: subject.clone(),
                path: path.clone(),
                object: object.clone(),
            },
            GraphPattern::PropertyFunction(call) => GraphPattern::PropertyFunction(call.clone()),
            GraphPattern::Join { left, right } => GraphPattern::Join {
                left: Box::new(normalize(left)),
                right: Box::new(normalize(right)),
            },
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => GraphPattern::LeftJoin {
                left: Box::new(normalize(left)),
                right: Box::new(normalize(right)),
                expression: expression.clone(),
            },
            GraphPattern::Filter { expr, inner } => GraphPattern::Filter {
                expr: expr.clone(),
                inner: Box::new(normalize(inner)),
            },
            GraphPattern::Union { left, right } => GraphPattern::Union {
                left: Box::new(normalize(left)),
                right: Box::new(normalize(right)),
            },
            GraphPattern::Graph { name, inner } => GraphPattern::Graph {
                name: name.clone(),
                inner: Box::new(normalize(inner)),
            },
            GraphPattern::Extend {
                inner,
                variable,
                expression,
            } => GraphPattern::Extend {
                inner: Box::new(normalize(inner)),
                variable: variable.clone(),
                expression: expression.clone(),
            },
            GraphPattern::Minus { left, right } => GraphPattern::Minus {
                left: Box::new(normalize(left)),
                right: Box::new(normalize(right)),
            },
            GraphPattern::Service {
                name,
                inner,
                silent,
            } => GraphPattern::Service {
                name: name.clone(),
                inner: Box::new(normalize(inner)),
                silent: *silent,
            },
            GraphPattern::Values {
                variables,
                bindings,
            } => GraphPattern::Values {
                variables: variables.clone(),
                bindings: bindings.clone(),
            },
            GraphPattern::OrderBy { inner, expression } => GraphPattern::OrderBy {
                inner: Box::new(normalize(inner)),
                expression: expression.clone(),
            },
            GraphPattern::Project { inner, variables } => GraphPattern::Project {
                inner: Box::new(normalize(inner)),
                variables: variables.clone(),
            },
            GraphPattern::Distinct { inner } => GraphPattern::Distinct {
                inner: Box::new(normalize(inner)),
            },
            GraphPattern::Reduced { inner } => GraphPattern::Reduced {
                inner: Box::new(normalize(inner)),
            },
            GraphPattern::Slice {
                inner,
                start,
                length,
            } => GraphPattern::Slice {
                inner: Box::new(normalize(inner)),
                start: *start,
                length: *length,
            },
            GraphPattern::Group {
                inner,
                variables,
                aggregates,
            } => GraphPattern::Group {
                inner: Box::new(normalize(inner)),
                variables: variables.clone(),
                aggregates: aggregates.clone(),
            },
        }
    }

    #[test]
    fn roundtrip_lateral_with_a_union_left_and_property_function_right() {
        // Only reachable by constructing the algebra directly — `GraphPattern`
        // and `pattern_to_select_query` are public API, but the parser itself
        // never builds this tree: its PF-triple fold only ever re-absorbs a
        // preceding TRIPLES BLOCK as a PF-`Lateral`'s left
        // (`is_pf_reabsorbable_left`), never a `Union`. Rendering the right
        // operand unwrapped here (the way a reabsorbable left does) would
        // re-parse as `Join(Union{..}, Lateral(unit-table, PF))` instead — the
        // PF call would stop seeing the union's bindings, a semantic change.
        // The `LATERAL { … }` keyword this fix DOES emit re-parses as
        // `Lateral{left: Union, right: Lateral{Bgp{[]}, PF}}` — laterality is
        // preserved (the union's bindings are visible to the call again), but
        // the braced body's own fold adds a unit-table `Lateral` around the
        // call that a bare, API-constructed `right: PropertyFunction` never
        // had; `normalize` cancels exactly that harmless wrapper before the
        // comparison.
        let call = PropertyFunctionCall {
            iri: format!("{PF_NS}related"),
            subject_args: vec![TermPattern::Variable(Variable::new("o"))],
            object_args: vec![TermPattern::Variable(Variable::new("z"))],
        };
        let union = GraphPattern::Union {
            left: Box::new(GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: TermPattern::Variable(Variable::new("s")),
                    predicate: NamedNodePattern::NamedNode(crate::ast::NamedNode::new_unchecked(
                        "https://example.org/d/p",
                    )),
                    object: TermPattern::Variable(Variable::new("o")),
                }],
            }),
            right: Box::new(GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: TermPattern::Variable(Variable::new("s")),
                    predicate: NamedNodePattern::NamedNode(crate::ast::NamedNode::new_unchecked(
                        "https://example.org/d/q",
                    )),
                    object: TermPattern::Variable(Variable::new("o")),
                }],
            }),
        };
        let body = GraphPattern::Lateral {
            left: Box::new(union),
            right: Box::new(GraphPattern::PropertyFunction(call)),
        };
        let text = pattern_to_select_query(&body);
        let reparsed = match SparqlParser::new()
            .parse_query_with(&text, &pf_options())
            .unwrap_or_else(|e| panic!("re-parse `{text}`: {e:?}"))
        {
            Query::Select { pattern, .. } => where_body(&pattern),
            other => panic!("expected SELECT, got {other:?}"),
        };
        assert_eq!(
            normalize(&reparsed),
            normalize(&body),
            "round-trip mismatch\n serialized: {text}"
        );
    }

    #[test]
    fn roundtrip_optional_as_a_join_right_operand() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> ?o . \
             { ?a <http://ex/x> ?b OPTIONAL { ?b <http://ex/y> ?c } } }",
        );
    }

    #[test]
    fn roundtrip_minus_as_a_join_right_operand() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> ?o . \
             { ?a <http://ex/x> ?b MINUS { ?b <http://ex/y> ?c } } }",
        );
    }

    #[test]
    fn roundtrip_filter_as_a_join_right_operand() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> ?o . \
             { ?a <http://ex/x> ?b FILTER(?b > 1) } }",
        );
    }

    #[test]
    fn roundtrip_extend_as_a_join_right_operand() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> ?o . \
             { ?a <http://ex/x> ?b BIND((?b + 1) AS ?c) } }",
        );
    }
}
