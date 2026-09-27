// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! * Pure `core::fmt::Write` into a `String` — **wasm-clean**, no std-only deps.
//!   The rendering is one loop over an explicit work list, which also renders
//!   [`crate::algebra::PropertyPathExpression`]'s `Display`: it needs no more
//!   machine stack for a taller tree.
//! * **Round-trips** with the parser: `parse(pattern_to_select_query(p))`
//!   reproduces `p` for every [`GraphPattern`]/[`Expression`] variant the parser
//!   emits, INCLUDING an aggregate/`GROUP BY` chain with its own outer `Project`
//!   already peeled away (the shape [`pattern_to_select_query`]'s own doctest
//!   takes, and the shape a whole aggregate query is once a caller — federation or
//!   otherwise — has stripped the query's top `SELECT` scaffold to reach the WHERE
//!   body underneath it).
//! * **Admitted in, admitted out.** A bracket or a brace is a level of the
//!   parser's recursion, which the stack bounds, and of the tree whose height the
//!   walks over it are measured against. So the rendering spends a bracket or a
//!   brace only where the grammar needs one to rebuild the same tree: expressions follow the grammar's precedence and
//!   associativity (`a + b + c` bare, `a - (b - c)` and `(a + b) * c`
//!   bracketted — see `Level`), property paths do the same
//!   ([`crate::algebra::PropertyPathExpression`]'s `Display`), a `UNION` chain
//!   or a run of `OPTIONAL`/`MINUS`/`BIND` clauses is written as the one flat
//!   sequence the parser rebuilds itself, and a sub-`SELECT` is never braced
//!   twice. Such a brace is one the source text needed too, so the text
//!   forwarded for a body the parser admitted is admitted again — bracketing
//!   every operator once turned a 440-operator `FILTER` into a refusal, which a
//!   `SERVICE SILENT` then answered as if the endpoint had failed.
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
    AggregateExpression, AggregateFunction, ArithmeticOperator, Expression, Function, GraphPattern,
    OrderExpression, PropertyFunctionCall, PropertyPathExpression,
};
use crate::ast::{
    BaseDirection, GroundTerm, Literal, NamedNodePattern, RDF_LANG_STRING, TermPattern,
    TriplePattern, Variable, XSD_STRING,
};
use crate::walk::{Flow, NodeRef, Visit, walk_pre_post};
use crate::worklist::WorkList;

/// The rendering's work list: a shallow tree's stays inline.
type Items<'a> = WorkList<Item<'a>, 32>;

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
/// The rendering is one loop over an explicit work list, so it needs no more machine
/// stack for a taller tree.
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
        // query once its own outer `Project` has already been peeled away.
        // Rendered as a sub-`SELECT` directly: that already reconstructs a
        // complete `SELECT … WHERE { … } [GROUP BY] …` string on its own, so
        // wrapping it in ANOTHER literal `SELECT * WHERE { … }` here would EITHER
        // double-nest a needless subquery whose outer `Project` re-projects the
        // identical variable set the inner one already declared OR — because a
        // `Group`-containing pattern has no valid `SELECT *` reading — render
        // `SELECT *` over an aggregate query, which SPARQL does not admit, and let
        // an aggregate call vanish behind a dangling reference to its synthetic
        // output variable.
        emit(&mut s, Item::Subselect(inner));
    } else {
        s.push_str("SELECT * WHERE ");
        emit(&mut s, Item::BracedGroup(inner));
    }
    s
}

/// Render `p` as the body of a `{ … }` group.
///
/// `pub(crate)`: this is also the WHERE-clause renderer `Display for
/// GraphUpdateOperation` (`algebra.rs`) reuses for `INSERT`/`DELETE … WHERE { … }`
/// — the exact same shape [`crate::parser`]'s `parse_group_graph_pattern` produces
/// for an UPDATE's WHERE clause (never the "bare aggregate chain" shape
/// [`needs_subselect_reconstruction`] exists to catch, which only arises once an
/// outer `Project` has been peeled from a top-level `SELECT`/subselect — an UPDATE
/// WHERE clause never carries one).
pub(crate) fn fmt_group_body(s: &mut String, p: &GraphPattern) {
    emit(s, Item::GroupBody(p));
}

/// Render a property path in its SPARQL surface syntax — the text of
/// [`PropertyPathExpression`]'s `Display`.
pub(crate) fn fmt_path(s: &mut String, path: &PropertyPathExpression) {
    emit(s, Item::Path(path));
}

/// Whether `p`, with no `Project` above it, is a bare modifier-chain shape the
/// sub-`SELECT` reconstruction must render directly rather than being wrapped in
/// another `SELECT * WHERE { … }` (see [`pattern_to_select_query`]'s doc for why
/// the wrap is wrong either way this predicate is true). `Slice`/`Distinct`/
/// `Reduced`/`OrderBy`/`Extend` are UNCONDITIONALLY this shape at the top — none of
/// them can legally appear as a bare (`Project`-less) element of an ORDINARY
/// `{ … }` group (SPARQL's grammar only ever produces them as part of a
/// `SELECT`/subselect's own solution-modifier chain, which always carries a
/// `Project`), so reaching one here with no `Project` above it is exactly the
/// "outer `Project` already peeled" shape, regardless of whether it goes on to reach
/// a `Group`. `Filter`/`Group` still route through [`extend_chain_reaches_group`],
/// which is the one case where "reaches a `Group`" (not "is one, unconditionally")
/// is the right test — an ordinary WHERE-body `FILTER` (never wrapping a `Group`)
/// must NOT trigger this.
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
/// (a) (b) …` lifts to one nested `Filter` per condition, so this follows the whole
/// chain rather than checking only the OUTERMOST `Filter`'s immediate `inner`)
/// nodes that terminates at a `Group` — the shape an aggregate query's own
/// SELECT-expression/`HAVING` chain takes once its outer `Project` has already been
/// peeled away.
///
/// Deliberately narrower than [`is_subselect_node`] and the sub-`SELECT` peel: a
/// `Project`/`Distinct`/`Reduced`/`Slice`/`OrderBy` node anywhere in the chain means
/// `p` is already one of [`is_subselect_node`]'s recognized shapes — a genuine,
/// self-contained sub-`SELECT` meant to be embedded as one element of a forwarded
/// WHERE body, which a group body wraps in a nested `{ SELECT … }`. Only the
/// narrower, `Project`-less shape here needs [`pattern_to_select_query`]'s different
/// (unwrapped) treatment — and it can ONLY arise there: an ordinary WHERE-body
/// `BIND`/`HAVING` never wraps a bare `Group` (a `Group` node is minted only by the
/// parser's aggregate-lifting, always immediately under the query's own modifier
/// chain), and any `{ SELECT … }` written in source text parses to a
/// `Project`-wrapped pattern, not a bare `Extend`/`Group` chain — so this predicate
/// cannot mistake an ordinary body element for this shape.
fn extend_chain_reaches_group(mut p: &GraphPattern) -> bool {
    loop {
        match p {
            GraphPattern::Group { .. } => return true,
            GraphPattern::Extend { inner, .. } | GraphPattern::Filter { inner, .. } => p = inner,
            _ => return false,
        }
    }
}

/// Does this pattern contain a [`GraphPattern::PropertyFunction`] anywhere in the
/// group structure it renders inline (it does not descend into `Expression`s,
/// which are always emitted inside their own braces)?
fn contains_property_function(p: &GraphPattern) -> bool {
    let mut found = false;
    walk_pre_post(NodeRef::Pattern(p), |visit, node| match (visit, node) {
        (Visit::Exit, _) => Flow::Descend,
        (Visit::Enter, NodeRef::Pattern(GraphPattern::PropertyFunction(_))) => {
            found = true;
            Flow::Stop
        }
        (Visit::Enter, NodeRef::Pattern(_)) => Flow::Descend,
        (Visit::Enter, _) => Flow::Skip,
    });
    found
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
/// PropertyFunction}` (the PF call itself) — so both continue down their own
/// `left`/`prior` operand.
fn is_pf_reabsorbable_left(mut p: &GraphPattern) -> bool {
    loop {
        match p {
            GraphPattern::Bgp { .. } => return true,
            GraphPattern::Join { left, right } if matches!(**right, GraphPattern::Bgp { .. }) => {
                p = left;
            }
            GraphPattern::Lateral { left, right }
                if matches!(**right, GraphPattern::PropertyFunction(_)) =>
            {
                p = left;
            }
            _ => return false,
        }
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
/// by the narrower [`left_operand_needs_bracing`] instead.
///
/// [`GraphPattern::Join`] is deliberately **excluded**: join is associative, so
/// re-associating a `Join` right operand into the running left-to-right chain
/// produces a semantically identical tree — the round-trip contract this module
/// promises is "semantics preserved", and tree-identity is asserted only where
/// semantics actually require it. Every modifier-rooted node
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
        | GraphPattern::Extend { .. }
        // `Unfold` renders its own `inner` inline before the `UNFOLD(…)`
        // clause, exactly as `Extend` renders its own before `BIND(…)`, so it
        // carries the identical right-operand splice hazard.
        | GraphPattern::Unfold { .. } => true,
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

/// `true` for the one [`GraphPattern`] shape that cannot be written inline
/// before a following group element: a [`GraphPattern::Filter`].
///
/// A `FILTER` constrains the whole group it is written in, wherever in the
/// group it stands — the parser collects a group's filters and wraps them
/// around everything else the group built. A `Filter` whose own group was
/// followed by another element can therefore only have come from a braced
/// sub-group, and written inline its condition would float out over the outer
/// element too (found by the corpus round-trip sweep: `service/service05.rq`'s
/// `FILTER`, scoped to a bracketed sub-group, re-associated onto the whole
/// outer group — including a `SERVICE ?g` lateral join written after it).
///
/// Every other node is rebuilt exactly by the group loop from its inline
/// rendering: `LeftJoin`/`Minus`/`Lateral` wrap the pattern before them in the
/// order written, and `Extend`/`Unfold` wrap it the same way, so a
/// `A OPTIONAL { B } BIND(e AS ?v) MINUS { C }` run reproduces its own
/// left-deep chain. A `Filter` nested inside one of those is braced by that
/// node's own rendering.
const fn left_operand_needs_bracing(p: &GraphPattern) -> bool {
    matches!(p, GraphPattern::Filter { .. })
}

/// The SPARQL expression grammar's binding strengths, loosest first
/// (§19.8 `[110]`–`[119]`): the level a rendered expression occupies, and the
/// level an operand position demands.
///
/// An operand whose own level is looser than its position demands is the one
/// shape that needs brackets; every other operand is written bare. That is what
/// keeps the forwarded text inside what the parser admitted: a bracket is one level
/// of the parser's recursion and one level of tree height, so bracketing every
/// operator turned a 440-operator chain the parser admitted into text it refused.
/// Bracketing only where the grammar would otherwise build a different tree writes
/// no bracket the source text did not also need, so the rendering of an admitted
/// tree is admitted again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    /// `ConditionalOrExpression`: `||`, left-associative.
    Or,
    /// `ConditionalAndExpression`: `&&`, left-associative.
    And,
    /// `RelationalExpression`: one optional `=`/`!=`/`<`/`>`/`<=`/`>=`/`IN`/
    /// `NOT IN` between two `NumericExpression`s — non-associative.
    Relational,
    /// `AdditiveExpression`: `+`/`-`, left-associative.
    Additive,
    /// `MultiplicativeExpression`: `*`/`/`, left-associative.
    Multiplicative,
    /// `UnaryExpression`: a prefix `!`/`+`/`-` over another unary expression.
    Unary,
    /// `PrimaryExpression`: terms, calls, `EXISTS`, and anything bracketted.
    Primary,
}

impl Level {
    /// The level the next-tighter operand of a left-associative operator at
    /// this level must reach: a right operand of equal level would re-parse
    /// left-nested, so it demands one level tighter.
    const fn tighter(self) -> Self {
        match self {
            Self::Or => Self::And,
            Self::And => Self::Relational,
            Self::Relational => Self::Additive,
            Self::Additive => Self::Multiplicative,
            Self::Multiplicative => Self::Unary,
            Self::Unary | Self::Primary => Self::Primary,
        }
    }
}

/// The level `e` renders at — which is decided by how it is SPELLED, not only by
/// its variant: `Not(Equal)` is written `a != b` and `Not(In)` `a NOT IN (…)` (both
/// relational, and both exactly the trees the parser builds for those spellings),
/// `Not(Exists)` is written `NOT EXISTS` (a primary), and any other `Not` is the
/// prefix `!`.
///
/// A variable standing for an aggregate's synthetic output renders as the
/// aggregate call, a primary — the same level the bare variable has.
fn expr_level(e: &Expression) -> Level {
    match e {
        Expression::Or(_) => Level::Or,
        Expression::And(_) => Level::And,
        Expression::Arithmetic(_, steps) => arithmetic_level(steps.last().0),
        Expression::Equal(..)
        | Expression::Greater(..)
        | Expression::GreaterOrEqual(..)
        | Expression::Less(..)
        | Expression::LessOrEqual(..)
        | Expression::In(..) => Level::Relational,
        Expression::Not(inner) => match **inner {
            Expression::Equal(..) | Expression::In(..) => Level::Relational,
            Expression::Exists(_) => Level::Primary,
            _ => Level::Unary,
        },
        Expression::UnaryPlus(_) | Expression::UnaryMinus(_) => Level::Unary,
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_)
        | Expression::SameTerm(..)
        | Expression::If(..)
        | Expression::Coalesce(_)
        | Expression::FunctionCall(..)
        | Expression::Exists(_) => Level::Primary,
    }
}

/// The level an arithmetic step's operator renders at.
const fn arithmetic_level(op: ArithmeticOperator) -> Level {
    if op.is_multiplicative() {
        Level::Multiplicative
    } else {
        Level::Additive
    }
}

/// One entry of the rendering's work list: text to write, or a node to render in a
/// given position. A node's rendering writes the text that leads it at once and
/// pushes the rest — its operands and the text between and after them — to be
/// written in order.
#[derive(Clone, Copy)]
enum Item<'a> {
    /// Literal text.
    Str(&'static str),
    /// Text borrowed from the tree, written verbatim: an IRI, a scalar-value name.
    Raw(&'a str),
    /// A literal.
    Literal(&'a Literal),
    /// A `GROUP_CONCAT` separator, escaped as a string literal's content.
    Escaped(&'a str),
    /// A bounded repetition's `{n}`, `{min,max}` or `{min,}`.
    Bounds(u32, Option<u32>),
    /// One character.
    Char(char),
    /// A variable, with its `?` sigil.
    Var(&'a Variable),
    /// An IRI or a variable in predicate, `GRAPH` or `SERVICE` position.
    Named(&'a NamedNodePattern),
    /// A `LIMIT`/`OFFSET` count, preceded by its keyword.
    Count(&'static str, usize),
    /// A pattern as the body of a `{ … }` group; a sub-`SELECT` renders as a braced
    /// `{ SELECT … }` block.
    GroupBody(&'a GraphPattern),
    /// A pattern as a braced group `{ … }` — the body of a `WHERE`, an arm of a
    /// `UNION`, or the group an `OPTIONAL`/`MINUS`/`LATERAL`/`GRAPH`/`SERVICE`/
    /// `EXISTS` keyword takes. A sub-`SELECT` already renders braced, and the grammar
    /// admits it directly in every one of these positions (`GroupGraphPattern ::= '{'
    /// ( SubSelect | GroupGraphPatternSub ) '}'`), so it is written once rather than
    /// wrapped in a second brace.
    BracedGroup(&'a GraphPattern),
    /// The operand a `Join`/`LeftJoin`/`Lateral`/`Minus` writes on its left, or the
    /// pattern a `Filter`/`Extend`/`Unfold` writes before its own clause, braced as
    /// its own group only when [`left_operand_needs_bracing`] says leaving it inline
    /// would change the tree the parser rebuilds. Every other shape is written
    /// inline, because the parser's group loop rebuilds it exactly: each element it
    /// reads wraps everything read before it, which is precisely the left-deep spine
    /// these nodes form. Bracing such an operand anyway would still round-trip, but
    /// it costs one nesting level per link of the chain, so a run of
    /// `OPTIONAL`/`MINUS`/`BIND` clauses the parser admitted would render into text
    /// it refuses.
    FlattenedLeft(&'a GraphPattern),
    /// The right operand of a `Join`, braced as its own group when leaving it
    /// unbraced would change the re-parsed tree's meaning: when it contains a
    /// property function (which renders as a plain triple the parser would fold into
    /// the left operand's triples block), or when
    /// [`rendering_starts_with_a_reabsorbable_left`]. A left operand needs no such
    /// brace: the parser's own left-deep assembly reproduces it.
    JoinRight(&'a GraphPattern),
    /// A solution-modifier chain, peeled and rendered as `SELECT [DISTINCT|REDUCED]
    /// <vars|*> WHERE { <body> } [GROUP BY] [HAVING] [ORDER BY] [LIMIT] [OFFSET]`.
    Subselect(&'a GraphPattern),
    /// A `FILTER`'s constraint: an `EXISTS` or `NOT EXISTS` bare, as the built-in
    /// call the grammar lets a constraint be, and anything else bracketted — the
    /// source text of a nest of `FILTER NOT EXISTS { … }` has no bracket at any
    /// level, and a bracket is a level of the parser's recursion.
    Constraint(&'a Expression),
    /// An expression in an operand position that demands at least this level:
    /// bare when it renders at that level or tighter, bracketted otherwise. With a
    /// `Group` in scope, a variable naming one of its aggregates' outputs renders as
    /// that aggregate's call (see [`Item::Aggregate`]).
    Expr(&'a Expression, Level, Option<GroupSpec<'a>>),
    /// An expression at its own level ([`expr_level`]), with no bracket around it.
    ExprBare(&'a Expression, Option<GroupSpec<'a>>),
    /// An aggregate call.
    Aggregate(&'a AggregateExpression),
    /// A term in a pattern.
    Term(&'a TermPattern),
    /// A `VALUES` cell.
    Ground(&'a GroundTerm),
    /// A property path at its own level.
    Path(&'a PropertyPathExpression),
    /// A property path as `^`'s operand: a sequence, an alternative or another
    /// inverse is bracketted (`^^p` needs `^(^p)` to stay two `Reverse`s). A
    /// quantified path is not: the postfix quantifiers bind tighter than `^`, so
    /// `^<p>*` re-parses as `Reverse(ZeroOrMore(<p>))` with no bracket.
    PathElt(&'a PropertyPathExpression),
    /// A property path as a postfix quantifier's operand: bracketted as `^`'s
    /// operand is, and also when it is itself quantified — `PathMod` attaches to
    /// exactly one `PathPrimary`, and an already-quantified path is not one, so
    /// `p**` is not surface syntax and `(p*)*` needs its bracket.
    QuantifierOperand(&'a PropertyPathExpression),
}

/// Render `first` onto `s`, and everything it pushes, until the work list is empty.
fn emit(s: &mut String, first: Item<'_>) {
    let mut stack = Items::with(first);
    while let Some(item) = stack.pop() {
        let queued = stack.len();
        render(s, item, &mut stack);
        stack.reverse_top(stack.len() - queued);
    }
}

/// Write what `item` leads with onto `s`, and queue what follows on `next`, in order;
/// the caller reverses what was queued, so it pops in that order.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per work-list entry, each a transcription of its grammar"
)]
fn render<'a>(s: &mut String, item: Item<'a>, next: &mut Items<'a>) {
    match item {
        Item::Str(text) | Item::Raw(text) => s.push_str(text),
        Item::Literal(l) => fmt_literal(s, l),
        Item::Escaped(text) => push_escaped(s, text),
        Item::Bounds(min, max) => {
            let _ = match max {
                Some(m) if m == min => write!(s, "{{{min}}}"),
                Some(m) => write!(s, "{{{min},{m}}}"),
                None => write!(s, "{{{min},}}"),
            };
        }
        Item::Char(c) => s.push(c),
        Item::Var(v) => {
            s.push('?');
            s.push_str(v.as_str());
        }
        Item::Named(n) => fmt_named_node_pattern(s, n),
        Item::Count(keyword, n) => {
            let _ = write!(s, "{keyword}{n}");
        }
        Item::GroupBody(p) => group_body(s, p, next),
        Item::BracedGroup(p) => {
            if is_subselect_node(p) {
                next.push(Item::GroupBody(p));
            } else {
                next.extend([Item::Str("{ "), Item::GroupBody(p), Item::Str(" }")]);
            }
        }
        Item::FlattenedLeft(p) => {
            if left_operand_needs_bracing(p) {
                next.extend([Item::Str("{ "), Item::GroupBody(p), Item::Str(" }")]);
            } else {
                next.push(Item::GroupBody(p));
            }
        }
        Item::JoinRight(p) => {
            if !is_subselect_node(p)
                && (contains_property_function(p) || rendering_starts_with_a_reabsorbable_left(p))
            {
                next.extend([Item::Str("{ "), Item::GroupBody(p), Item::Str(" }")]);
            } else {
                next.push(Item::GroupBody(p));
            }
        }
        Item::Subselect(p) => subselect(s, p, next),
        Item::Constraint(expr) => {
            let bare = match expr {
                Expression::Exists(_) => true,
                Expression::Not(inner) => matches!(**inner, Expression::Exists(_)),
                _ => false,
            };
            if bare {
                next.extend([Item::Char(' '), Item::Expr(expr, Level::Or, None)]);
            } else {
                next.extend([
                    Item::Char('('),
                    Item::Expr(expr, Level::Or, None),
                    Item::Char(')'),
                ]);
            }
        }
        Item::Expr(e, min, group) => {
            if expr_level(e) < min {
                next.extend([Item::Char('('), Item::ExprBare(e, group), Item::Char(')')]);
            } else {
                next.push(Item::ExprBare(e, group));
            }
        }
        Item::ExprBare(e, group) => expr_bare(s, e, group, next),
        Item::Aggregate(agg) => aggregate(s, agg, next),
        Item::Term(term) => match term {
            TermPattern::Triple(t) => {
                s.push_str("<<( ");
                triple_items(t, next);
                next.push(Item::Str(" )>>"));
            }
            leaf => fmt_leaf_term(s, leaf),
        },
        Item::Ground(term) => match term {
            GroundTerm::NamedNode(n) => {
                let _ = write!(s, "<{}>", n.as_str());
            }
            GroundTerm::Literal(l) => fmt_literal(s, l),
            GroundTerm::Triple(t) => {
                s.push_str("<<( ");
                next.extend([
                    Item::Ground(&t.subject),
                    Item::Str(" <"),
                    Item::Raw(t.predicate.as_str()),
                    Item::Str("> "),
                    Item::Ground(&t.object),
                    Item::Str(" )>>"),
                ]);
            }
            // Injection-only: emitted as a blank-node label. The parser never
            // produces this variant, and `purrdf-sparql-eval`'s `SERVICE` forwarding
            // path (`sanitize_forwarded_body` in `crates/sparql-eval/src/remote.rs`)
            // strips every `Values` column carrying one before a substituted
            // `SERVICE` body is serialized — a blank-node `VALUES` cell is not legal
            // `DataBlockValue` syntax, so it must never reach the wire. This arm
            // therefore stays live only for a hand-built pattern serialized directly
            // through this crate's public API; the forwarding path never feeds it one.
            GroundTerm::BlankNode(b) => {
                let _ = write!(s, "_:{}", b.as_str());
            }
        },
        Item::Path(path) => path_items(s, path, next),
        Item::PathElt(path) => match path {
            PropertyPathExpression::Sequence(..)
            | PropertyPathExpression::Alternative(..)
            | PropertyPathExpression::Reverse(..) => {
                next.extend([Item::Char('('), Item::Path(path), Item::Char(')')]);
            }
            other => next.push(Item::Path(other)),
        },
        Item::QuantifierOperand(path) => match path {
            PropertyPathExpression::Sequence(..)
            | PropertyPathExpression::Alternative(..)
            | PropertyPathExpression::Reverse(..)
            | PropertyPathExpression::ZeroOrMore(..)
            | PropertyPathExpression::OneOrMore(..)
            | PropertyPathExpression::ZeroOrOne(..)
            | PropertyPathExpression::Range { .. } => {
                next.extend([Item::Char('('), Item::Path(path), Item::Char(')')]);
            }
            other => next.push(Item::Path(other)),
        },
    }
}

/// Queue `s p o` of a pattern triple term's triple (between `<<( ` and ` )>>`).
fn triple_items<'a>(t: &'a TriplePattern, next: &mut Items<'a>) {
    next.extend([
        Item::Term(&t.subject),
        Item::Char(' '),
        Item::Named(&t.predicate),
        Item::Char(' '),
        Item::Term(&t.object),
    ]);
}

/// Render a pattern as the body of a group (see [`Item::GroupBody`]).
fn group_body<'a>(s: &mut String, p: &'a GraphPattern, next: &mut Items<'a>) {
    if is_subselect_node(p) {
        next.extend([Item::Str("{ "), Item::Subselect(p), Item::Str(" }")]);
        return;
    }
    match p {
        GraphPattern::Bgp { patterns } => {
            for (i, tp) in patterns.iter().enumerate() {
                if i > 0 {
                    next.push(Item::Char(' '));
                }
                triple_items(tp, next);
                next.push(Item::Str(" ."));
            }
        }
        GraphPattern::Path {
            subject,
            path,
            object,
        } => next.extend([
            Item::Term(subject),
            Item::Char(' '),
            Item::Path(path),
            Item::Char(' '),
            Item::Term(object),
            Item::Str(" ."),
        ]),
        GraphPattern::Join { left, right } => next.extend([
            Item::FlattenedLeft(left),
            Item::Char(' '),
            Item::JoinRight(right),
        ]),
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            next.extend([Item::FlattenedLeft(left), Item::Str(" OPTIONAL ")]);
            if let Some(expr) = expression {
                // The trailing `FILTER` the parser splits back out into this
                // node's expression, so the right operand stays a group of its
                // own even when it is a sub-`SELECT`.
                next.extend([
                    Item::Str("{ "),
                    Item::GroupBody(right),
                    Item::Str(" FILTER"),
                    Item::Constraint(expr),
                    Item::Str(" }"),
                ]);
            } else {
                next.push(Item::BracedGroup(right));
            }
        }
        GraphPattern::Lateral { left, right } => {
            next.push(Item::FlattenedLeft(left));
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
                    next.push(Item::Char(' '));
                }
                next.push(Item::GroupBody(right));
            } else {
                next.extend([Item::Str(" LATERAL "), Item::BracedGroup(right)]);
            }
        }
        GraphPattern::PropertyFunction(call) => property_function(call, next),
        GraphPattern::Filter { expr, inner } => {
            // A `FILTER` constrains its whole group wherever it is written, so
            // a chain of them over one inner pattern is exactly what the parser
            // builds from them written in a row; only a filter-free inner
            // pattern — or another `Filter` — is written inline.
            if matches!(**inner, GraphPattern::Filter { .. }) {
                next.push(Item::GroupBody(inner));
            } else {
                next.push(Item::FlattenedLeft(inner));
            }
            next.extend([Item::Str(" FILTER"), Item::Constraint(expr)]);
        }
        GraphPattern::Union { arms } => {
            // Written as the one flat `{ a } UNION { b } UNION …` chain the parser
            // reads back into one node with the same arms. A leading arm that is
            // itself a chain is written as the leading arms of this chain rather
            // than as a braced arm of its own: the parser folds it into the same
            // node, and bag union is associative, so the solutions are the same
            // sequence either way. Every later arm is braced, as it must have been
            // written.
            let (first, rest) = arms.split_first();
            match first {
                GraphPattern::Union { .. } => next.push(Item::GroupBody(first)),
                _ => next.push(Item::BracedGroup(first)),
            }
            for arm in rest {
                next.extend([Item::Str(" UNION "), Item::BracedGroup(arm)]);
            }
        }
        GraphPattern::Graph { name, inner } => {
            s.push_str("GRAPH ");
            fmt_named_node_pattern(s, name);
            s.push(' ');
            next.push(Item::BracedGroup(inner));
        }
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => next.extend([
            Item::FlattenedLeft(inner),
            Item::Str(" BIND("),
            Item::Expr(expression, Level::Or, None),
            Item::Str(" AS "),
            Item::Var(variable),
            Item::Char(')'),
        ]),
        // `[174] Unfold ::= 'UNFOLD' '(' Expression 'AS' Var ( ',' Var )? ')'`.
        // Renders exactly like `BIND` — the inner pattern inline, then the
        // clause — because the parser reads it the same way: as one more
        // element of the running group, stacked above everything before it.
        GraphPattern::Unfold {
            inner,
            expression,
            element,
            companion,
        } => {
            next.extend([
                Item::FlattenedLeft(inner),
                Item::Str(" UNFOLD("),
                Item::Expr(expression, Level::Or, None),
                Item::Str(" AS "),
                Item::Var(element),
            ]);
            if let Some(companion) = companion {
                next.extend([Item::Str(", "), Item::Var(companion)]);
            }
            next.push(Item::Char(')'));
        }
        GraphPattern::Minus { left, right } => next.extend([
            Item::FlattenedLeft(left),
            Item::Str(" MINUS "),
            Item::BracedGroup(right),
        ]),
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
            s.push(' ');
            next.push(Item::BracedGroup(inner));
        }
        GraphPattern::Values {
            variables,
            bindings,
        } => values(s, variables, bindings, next),
        // Sub-select nodes are rendered by the `is_subselect_node` branch above.
        GraphPattern::Project { .. }
        | GraphPattern::Distinct { .. }
        | GraphPattern::Reduced { .. }
        | GraphPattern::Slice { .. }
        | GraphPattern::OrderBy { .. }
        | GraphPattern::Group { .. } => unreachable!("handled by is_subselect_node"),
    }
}

/// A property-function call as the triple `subjectArgs <iri> objectArgs .`
///
/// The IRI is re-emitted byte-exact (PurRDF fabricates no namespace on output).
/// A ONE-element argument vector renders as the bare term; a zero- or
/// multi-element vector renders as collection syntax `( … )`, which the parser
/// reads back as an argument list — never as an `rdf:first`/`rdf:rest` chain —
/// because the predicate names a property function.
fn property_function<'a>(call: &'a PropertyFunctionCall, next: &mut Items<'a>) {
    property_function_args(&call.subject_args, next);
    next.extend([Item::Str(" <"), Item::Raw(&call.iri), Item::Str("> ")]);
    property_function_args(&call.object_args, next);
    next.push(Item::Str(" ."));
}

/// One side of a property-function call (see [`property_function`]).
fn property_function_args<'a>(args: &'a [TermPattern], next: &mut Items<'a>) {
    if let [single] = args {
        next.push(Item::Term(single));
        return;
    }
    if args.is_empty() {
        next.push(Item::Str("()"));
        return;
    }
    next.push(Item::Char('('));
    for arg in args {
        next.extend([Item::Char(' '), Item::Term(arg)]);
    }
    next.push(Item::Str(" )"));
}

/// A `VALUES (?v …) { (term …) … }` block (always the parenthesized form).
fn values<'a>(
    s: &mut String,
    variables: &'a [Variable],
    bindings: &'a [Vec<Option<GroundTerm>>],
    next: &mut Items<'a>,
) {
    s.push_str("VALUES (");
    for (i, v) in variables.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push('?');
        s.push_str(v.as_str());
    }
    s.push_str(") {");
    for row in bindings {
        next.push(Item::Str(" ("));
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                next.push(Item::Char(' '));
            }
            match cell {
                None => next.push(Item::Str("UNDEF")),
                Some(gt) => next.push(Item::Ground(gt)),
            }
        }
        next.push(Item::Char(')'));
    }
    next.push(Item::Str(" }"));
}

/// Peel the solution-modifier chain (outermost → innermost) and render
/// `SELECT [DISTINCT|REDUCED] <vars|*> WHERE { <body> } [GROUP BY] [HAVING]
/// [ORDER BY] [LIMIT] [OFFSET]`.
fn subselect<'a>(s: &mut String, p: &'a GraphPattern, next: &mut Items<'a>) {
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
    //   a plain `GROUP BY` key projection) must not re-emit as `SELECT * …
    //   GROUP BY ?s`, which the parser then correctly refuses on re-parse.
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
    //   non-aggregating body) must not render as an in-body `BIND`, which the
    //   parser's own "filters float to the group's end" rule then
    //   re-associates differently than the original tree.
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
            s.push('?');
            s.push_str(v.as_str());
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
    // `Item::ExprBare`'s rendering for why this is required for correctness, not
    // just cosmetics.
    for (i, (var, expr)) in select_exprs.iter().enumerate() {
        if plain_emitted || i > 0 {
            next.push(Item::Char(' '));
        }
        next.extend([
            Item::Char('('),
            Item::Expr(expr, Level::Or, group),
            Item::Str(" AS "),
            Item::Var(var),
            Item::Char(')'),
        ]);
    }

    next.extend([Item::Str(" WHERE "), Item::BracedGroup(cur)]);

    if let Some((vars, _)) = group {
        // An implicit single group (aggregates with no GROUP BY) has no clause.
        if !vars.is_empty() {
            next.push(Item::Str(" GROUP BY"));
            for v in vars {
                next.extend([Item::Char(' '), Item::Var(v)]);
            }
        }
    }
    if !having.is_empty() {
        next.push(Item::Str(" HAVING"));
        for expr in having {
            next.extend([
                Item::Char('('),
                Item::Expr(expr, Level::Or, group),
                Item::Char(')'),
            ]);
        }
    }
    if let Some(keys) = order {
        next.push(Item::Str(" ORDER BY"));
        order_keys(keys, group, next);
    }
    if let Some((start, length)) = slice {
        if let Some(len) = length {
            next.push(Item::Count(" LIMIT ", len));
        }
        if start > 0 {
            next.push(Item::Count(" OFFSET ", start));
        }
    }
}

/// Queue `ORDER BY` keys, each in its explicit `ASC(…)`/`DESC(…)` form.
fn order_keys<'a>(keys: &'a [OrderExpression], group: Option<GroupSpec<'a>>, next: &mut Items<'a>) {
    for key in keys {
        let (keyword, e) = match key {
            OrderExpression::Asc(e) => (" ASC(", e),
            OrderExpression::Desc(e) => (" DESC(", e),
        };
        next.extend([
            Item::Str(keyword),
            Item::Expr(e, Level::Or, group),
            Item::Char(')'),
        ]);
    }
}

/// Render `e` at its own level ([`expr_level`]), with no bracket around it.
///
/// With `group` in scope, a bare reference to one of its aggregates' synthetic
/// output variables renders as that aggregate's call instead of the bare variable
/// name. The WHERE body a sub-`SELECT` renders never binds such a variable
/// (`__purrdf_agg_N`, minted by the parser's aggregate-lifting; see
/// `Parser::fresh_agg_var`) — the aggregate FUNCTION CALL is what binds it, and
/// that call has no surface-syntax home inside the WHERE clause itself (§18.2.4's
/// algebra evaluates `Group` strictly after the WHERE body). So a projection
/// `(expr AS ?v)`, a `HAVING(expr)`, or an `ORDER BY` key that mentions one of
/// these synthetic variables — which is exactly how the parser represents
/// `(COUNT(?x) AS ?c)`, `HAVING(COUNT(?x) > 5)`, or `ORDER BY DESC(COUNT(?x))` —
/// must have that reference resolved back to the aggregate call on the way out, or
/// the serialized query would be syntactically valid but reference a variable
/// nothing binds.
///
/// `group` is `None` everywhere a WHERE-body expression is rendered, because an
/// aggregate's synthetic variable cannot escape into the WHERE body: it is
/// introduced by `Group`'s OWN aggregate list and consumed only by the
/// SELECT-expression/`HAVING`/`ORDER BY` layer sitting directly above that `Group`
/// in the modifier chain — never by a `FILTER`/`BIND` inside the body, and never by
/// a nested `EXISTS` pattern (a fresh, self-contained WHERE scope, which is why the
/// `Exists` arm below does not thread `group` through).
fn expr_bare<'a>(
    s: &mut String,
    e: &'a Expression,
    group: Option<GroupSpec<'a>>,
    next: &mut Items<'a>,
) {
    if let Expression::Variable(v) = e
        && let Some((_, aggs)) = group
        && let Some((_, agg)) = aggs.iter().find(|(ov, _)| ov == v)
    {
        next.push(Item::Aggregate(agg));
        return;
    }
    let operand = |e: &'a Expression, level: Level| Item::Expr(e, level, group);
    match e {
        Expression::NamedNode(n) => {
            let _ = write!(s, "<{}>", n.as_str());
        }
        Expression::Literal(l) => fmt_literal(s, l),
        Expression::Variable(v) => {
            s.push('?');
            s.push_str(v.as_str());
        }
        Expression::Bound(v) => {
            let _ = write!(s, "BOUND(?{})", v.as_str());
        }
        // Written as the flat `a OP b OP c …` the parser folds back into one node
        // with the same operands. The first operand may be any expression of the
        // chain's level (a leading chain of the same operator is folded into this
        // one on re-parse, which the three-valued fold makes the same value); every
        // later operand must bind tighter.
        Expression::Or(operands) | Expression::And(operands) => {
            let (op, level) = if matches!(e, Expression::Or(_)) {
                (" || ", Level::Or)
            } else {
                (" && ", Level::And)
            };
            let (first, rest) = operands.split_first();
            next.push(operand(first, level));
            for x in rest {
                next.extend([Item::Str(op), operand(x, level.tighter())]);
            }
        }
        // The left spine of a binary operator tree, written as the text the parser
        // folds back into the same chain. Each step's operand must bind tighter
        // than its operator, as the right operand of a left-associative operator
        // does. The value before a step is written bare when its last operator
        // binds at least as tightly as the step's, and bracketed otherwise: `a + b`
        // followed by `* c` is `(a + b) * c`. All those brackets open at the start
        // of the chain, one per additive-to-multiplicative change, and each closes
        // before the step that needs it — the brackets the source text needed too,
        // and no others, so a chain of any length the parser admitted renders into
        // text it admits again.
        Expression::Arithmetic(first, steps) => {
            let brackets = steps
                .windows(2)
                .filter(|pair| !pair[0].0.is_multiplicative() && pair[1].0.is_multiplicative())
                .count();
            for _ in 0..brackets {
                s.push('(');
            }
            next.push(operand(first, arithmetic_level(steps.first().0)));
            let mut previous: Option<ArithmeticOperator> = None;
            for (op, x) in steps {
                if previous
                    .is_some_and(|before| !before.is_multiplicative() && op.is_multiplicative())
                {
                    next.push(Item::Char(')'));
                }
                next.extend([
                    Item::Char(' '),
                    Item::Str(op.symbol()),
                    Item::Char(' '),
                    operand(x, arithmetic_level(*op).tighter()),
                ]);
                previous = Some(*op);
            }
        }
        Expression::Equal(a, b) => relational(a, "=", b, group, next),
        Expression::SameTerm(a, b) => {
            s.push_str("sameTerm(");
            next.extend([
                operand(a, Level::Or),
                Item::Str(", "),
                operand(b, Level::Or),
                Item::Char(')'),
            ]);
        }
        Expression::Greater(a, b) => relational(a, ">", b, group, next),
        Expression::GreaterOrEqual(a, b) => relational(a, ">=", b, group, next),
        Expression::Less(a, b) => relational(a, "<", b, group, next),
        Expression::LessOrEqual(a, b) => relational(a, "<=", b, group, next),
        // A prefix operator is a token of its own however the operand begins (the
        // lexer never merges a sign into what follows it, and every literal renders
        // quoted), so no space is needed.
        Expression::UnaryPlus(a) => {
            s.push('+');
            next.push(operand(a, Level::Unary));
        }
        Expression::UnaryMinus(a) => {
            s.push('-');
            next.push(operand(a, Level::Unary));
        }
        Expression::Not(a) => match &**a {
            // The spellings the parser itself builds these two trees from.
            Expression::Equal(l, r) => relational(l, "!=", r, group, next),
            Expression::In(l, list) => in_list(l, " NOT IN (", list, group, next),
            Expression::Exists(p) => {
                s.push_str("NOT EXISTS ");
                next.push(Item::BracedGroup(p));
            }
            other => {
                s.push('!');
                next.push(operand(other, Level::Unary));
            }
        },
        Expression::In(a, list) => in_list(a, " IN (", list, group, next),
        Expression::If(c, t, e2) => {
            s.push_str("IF(");
            next.extend([
                operand(c, Level::Or),
                Item::Str(", "),
                operand(t, Level::Or),
                Item::Str(", "),
                operand(e2, Level::Or),
                Item::Char(')'),
            ]);
        }
        Expression::Coalesce(list) => {
            s.push_str("COALESCE(");
            expr_list(list, group, next);
            next.push(Item::Char(')'));
        }
        Expression::FunctionCall(func, args) => {
            fmt_function_name(s, func);
            s.push('(');
            expr_list(args, group, next);
            next.push(Item::Char(')'));
        }
        Expression::Exists(p) => {
            s.push_str("EXISTS ");
            next.push(Item::BracedGroup(p));
        }
    }
}

/// Queue `a OP b` for a relational operator: both operands are
/// `NumericExpression`s, so each must reach [`Level::Additive`].
fn relational<'a>(
    a: &'a Expression,
    op: &'static str,
    b: &'a Expression,
    group: Option<GroupSpec<'a>>,
    next: &mut Items<'a>,
) {
    next.extend([
        Item::Expr(a, Level::Additive, group),
        Item::Char(' '),
        Item::Str(op),
        Item::Char(' '),
        Item::Expr(b, Level::Additive, group),
    ]);
}

/// Queue `a IN (list)` / `a NOT IN (list)` (`keyword` carries the spaces and the
/// opening bracket): the tested operand is a `NumericExpression`, each list entry
/// any expression.
fn in_list<'a>(
    a: &'a Expression,
    keyword: &'static str,
    list: &'a [Expression],
    group: Option<GroupSpec<'a>>,
    next: &mut Items<'a>,
) {
    next.extend([Item::Expr(a, Level::Additive, group), Item::Str(keyword)]);
    expr_list(list, group, next);
    next.push(Item::Char(')'));
}

/// Queue a comma-separated expression list.
fn expr_list<'a>(list: &'a [Expression], group: Option<GroupSpec<'a>>, next: &mut Items<'a>) {
    for (i, e) in list.iter().enumerate() {
        if i > 0 {
            next.push(Item::Str(", "));
        }
        next.push(Item::Expr(e, Level::Or, group));
    }
}

/// A SPARQL aggregate expression: `COUNT(*)`, `FUNC([DISTINCT] expr [;
/// SEPARATOR="…"])`, or `AGG(<iri>, [DISTINCT] arg, arg, … [; NAME=value]*)` for a
/// [`AggregateFunction::Custom`] aggregate.
///
/// Rendered wherever an aggregate's synthetic output variable is referenced in the
/// SELECT projection, `HAVING`, or `ORDER BY` of a sub-`SELECT` — the production
/// SERVICE-federation path this module exists for. An aggregate's own `args` are
/// plain WHERE-body expressions (nested aggregates are not legal SPARQL), so they
/// render with no `Group` in scope.
fn aggregate<'a>(s: &mut String, agg: &'a AggregateExpression, next: &mut Items<'a>) {
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
        AggregateFunction::Fold => "FOLD",
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
            expr_list(args, None, next);
            for (name, value) in &agg.scalarvals {
                next.extend([
                    Item::Str("; "),
                    Item::Raw(name),
                    Item::Char('='),
                    Item::Literal(value),
                ]);
            }
            next.push(Item::Char(')'));
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
        expr_list(args, None, next);
    }
    if let Some(sep) = agg.separator() {
        next.extend([
            Item::Str("; SEPARATOR=\""),
            Item::Escaped(sep),
            Item::Char('"'),
        ]);
    }
    // `FOLD`'s own sort keys — the ONE aggregate that has any
    // (`AggregateExpression::new` refuses them anywhere else, which is what
    // keeps this tail from ever emitting `SUM(?v ORDER BY ?k)`). They follow
    // the LAST exprlist entry with NO separating comma, since a comma there
    // would name a third argument on re-parse. Every condition is written in
    // its explicit `ASC(…)`/`DESC(…)` form: `FOLD(?v ORDER BY ?a ?b)`'s two
    // bare keys must not run together into one expression when re-read.
    if !agg.order_by.is_empty() {
        next.push(Item::Str(" ORDER BY"));
        order_keys(&agg.order_by, None, next);
    }
    next.push(Item::Char(')'));
}

/// Queue a property path at its own level: the text of its `Display`.
///
/// The standard operators round-trip with the parser; the two PurRDF extensions
/// render as `path{min,max}` (bounded repetition — round-trips) and `<any>` /
/// `<any:ns>` (predicate wildcard — emit-only).
fn path_items<'a>(s: &mut String, path: &'a PropertyPathExpression, next: &mut Items<'a>) {
    use PropertyPathExpression as P;
    match path {
        P::NamedNode(n) => {
            let _ = write!(s, "<{}>", n.as_str());
        }
        P::Reverse(a) => {
            s.push('^');
            next.push(Item::PathElt(a));
        }
        // `/` binds TIGHTER than `|`, and both are left-associative, so a chain is
        // written flat, `a/b/c`, and re-parses into one node with the same
        // elements. An `Alternative` element needs parens anywhere in a sequence
        // (bare, `(p1|p2)/(p3|p4)` would mis-group as `p1|(p2/p3)|p4`). A nested
        // `Sequence` needs them everywhere but first: as the first element it is
        // written as the leading elements of this chain, which the parser folds
        // into the same node — composition is associative, so the relation is the
        // same — while a later one bare would join this chain instead of staying
        // one element.
        P::Sequence(elements) => path_chain(elements, '/', next, |i, e| {
            matches!(e, P::Alternative(_)) || (i > 0 && matches!(e, P::Sequence(_)))
        }),
        // `|` has the lowest precedence, so only a nested `Alternative` needs parens,
        // and only after the first element, for the same reason as a `Sequence`'s.
        P::Alternative(elements) => path_chain(elements, '|', next, |i, e| {
            i > 0 && matches!(e, P::Alternative(_))
        }),
        P::ZeroOrMore(a) => next.extend([Item::QuantifierOperand(a), Item::Char('*')]),
        P::OneOrMore(a) => next.extend([Item::QuantifierOperand(a), Item::Char('+')]),
        P::ZeroOrOne(a) => next.extend([Item::QuantifierOperand(a), Item::Char('?')]),
        P::Range { inner, min, max } => {
            next.push(Item::QuantifierOperand(inner));
            next.push(Item::Bounds(*min, *max));
        }
        P::NegatedPropertySet(elems) => {
            s.push_str("!(");
            for (i, e) in elems.iter().enumerate() {
                if i > 0 {
                    s.push('|');
                }
                if e.inverse {
                    s.push('^');
                }
                let _ = write!(s, "<{}>", e.predicate.as_str());
            }
            s.push(')');
        }
        P::Wildcard { namespace } => match namespace {
            Some(ns) => {
                let _ = write!(s, "<any:{}>", ns.as_str());
            }
            None => s.push_str("<any>"),
        },
    }
}

/// Queue a [`PropertyPathExpression::Sequence`] or `Alternative` chain as its
/// elements joined by `op`, bracketing the element at index `i` when
/// `parens(i, element)` says the bare text would re-parse differently.
fn path_chain<'a>(
    elements: &'a [PropertyPathExpression],
    op: char,
    next: &mut Items<'a>,
    parens: impl Fn(usize, &PropertyPathExpression) -> bool,
) {
    for (i, element) in elements.iter().enumerate() {
        if i > 0 {
            next.push(Item::Char(op));
        }
        if parens(i, element) {
            next.extend([Item::Char('('), Item::Path(element), Item::Char(')')]);
        } else {
            next.push(Item::Path(element));
        }
    }
}

/// Emit a leaf query-pattern term (any but a quoted triple term).
fn fmt_leaf_term(s: &mut String, t: &TermPattern) {
    match t {
        TermPattern::NamedNode(n) => {
            let _ = write!(s, "<{}>", n.as_str());
        }
        TermPattern::BlankNode(b) => {
            let _ = write!(s, "_:{}", b.as_str());
        }
        TermPattern::Literal(l) => fmt_literal(s, l),
        TermPattern::Variable(v) => {
            s.push('?');
            s.push_str(v.as_str());
        }
        TermPattern::Triple(_) => unreachable!("a quoted triple term is queued, not a leaf"),
    }
}

/// Emit an IRI-or-variable (predicate / `GRAPH`/`SERVICE` name position).
fn fmt_named_node_pattern(s: &mut String, n: &NamedNodePattern) {
    match n {
        NamedNodePattern::NamedNode(node) => {
            let _ = write!(s, "<{}>", node.as_str());
        }
        NamedNodePattern::Variable(v) => {
            s.push('?');
            s.push_str(v.as_str());
        }
    }
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

/// Emit a SPARQL built-in or custom function name.
fn fmt_function_name(s: &mut String, f: &Function) {
    match function_keyword(f) {
        Some(name) => s.push_str(name),
        // A PurRDF extension call, a SEP-0009 composite-datatype call and a host
        // `Function::Custom` are all spelled by their ORIGINAL IRI (recorded in
        // the AST node), never by a keyword. PurRDF mints no vocabulary of its
        // own, so no namespace is ever fabricated on output; re-parsing with the
        // same `ParserOptions` re-dispatches to the same function.
        None => match f {
            Function::Purrdf(call) => {
                let _ = write!(s, "<{}>", call.iri);
            }
            // SEP-0009 fixes the IRI, so this is also `call.fn_kind.iri()` —
            // writing the recorded string rather than re-deriving it keeps the
            // "emit exactly what was read" rule uniform across every IRI-named
            // function seam.
            Function::Cdt(call) => {
                let _ = write!(s, "<{}>", call.iri);
            }
            Function::Custom(n) => {
                let _ = write!(s, "<{}>", n.as_str());
            }
            // `function_keyword` answers `Some` for every other variant.
            _ => s.push_str("<>"),
        },
    }
}

/// The canonical SPARQL grammar keyword for a built-in function, or `None` for
/// the three IRI-spelled variants ([`Function::Purrdf`], [`Function::Cdt`],
/// [`Function::Custom`]).
///
/// The single source of truth for a built-in's spelling: the serializer above
/// emits it, and `crate::parser::builtin_function_keyword` answers a
/// name-to-function lookup from it, so a resolver can never name a spelling the
/// serializer would not write.
pub(crate) fn function_keyword(f: &Function) -> Option<&'static str> {
    Some(match f {
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
        Function::Purrdf(_) | Function::Cdt(_) | Function::Custom(_) => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Query;
    use crate::tree::{Chain, Child};

    /// Render one aggregate call on its own.
    fn fmt_aggregate(s: &mut String, agg: &AggregateExpression) {
        emit(s, Item::Aggregate(agg));
    }
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
            Vec::new(),
            false,
        )
        .unwrap();
        let mut s = String::new();
        fmt_aggregate(&mut s, &agg);
        assert_eq!(s, "GROUP_CONCAT(?x; SEPARATOR=\"|\")");
    }

    #[test]
    fn aggregate_renders_count_star() {
        let agg = AggregateExpression::new(
            AggregateFunction::Count,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            true,
        )
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
            Vec::new(),
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
            let err = AggregateExpression::new(
                function.clone(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                false,
            )
            .unwrap_err();
            assert_eq!(err.function(), &function);
        }
        assert!(
            AggregateExpression::new(
                AggregateFunction::Count,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                false,
            )
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
                Vec::new(),
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
            Vec::new(),
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
                Vec::new(),
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
                Vec::new(),
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
                        left: Child::new(left),
                        right: Child::new(right),
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
                left: Child::new(normalize(left)),
                right: Child::new(normalize(right)),
            },
            GraphPattern::LeftJoin {
                left,
                right,
                expression,
            } => GraphPattern::LeftJoin {
                left: Child::new(normalize(left)),
                right: Child::new(normalize(right)),
                expression: expression.clone(),
            },
            GraphPattern::Filter { expr, inner } => GraphPattern::Filter {
                expr: expr.clone(),
                inner: Child::new(normalize(inner)),
            },
            GraphPattern::Union { arms } => GraphPattern::Union {
                arms: Chain::try_from(arms.iter().map(normalize).collect::<Vec<_>>())
                    .expect("two or more arms"),
            },
            GraphPattern::Graph { name, inner } => GraphPattern::Graph {
                name: name.clone(),
                inner: Child::new(normalize(inner)),
            },
            GraphPattern::Extend {
                inner,
                variable,
                expression,
            } => GraphPattern::Extend {
                inner: Child::new(normalize(inner)),
                variable: variable.clone(),
                expression: expression.clone(),
            },
            GraphPattern::Unfold {
                inner,
                expression,
                element,
                companion,
            } => GraphPattern::Unfold {
                inner: Child::new(normalize(inner)),
                expression: expression.clone(),
                element: element.clone(),
                companion: companion.clone(),
            },
            GraphPattern::Minus { left, right } => GraphPattern::Minus {
                left: Child::new(normalize(left)),
                right: Child::new(normalize(right)),
            },
            GraphPattern::Service {
                name,
                inner,
                silent,
            } => GraphPattern::Service {
                name: name.clone(),
                inner: Child::new(normalize(inner)),
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
                inner: Child::new(normalize(inner)),
                expression: expression.clone(),
            },
            GraphPattern::Project { inner, variables } => GraphPattern::Project {
                inner: Child::new(normalize(inner)),
                variables: variables.clone(),
            },
            GraphPattern::Distinct { inner } => GraphPattern::Distinct {
                inner: Child::new(normalize(inner)),
            },
            GraphPattern::Reduced { inner } => GraphPattern::Reduced {
                inner: Child::new(normalize(inner)),
            },
            GraphPattern::Slice {
                inner,
                start,
                length,
            } => GraphPattern::Slice {
                inner: Child::new(normalize(inner)),
                start: *start,
                length: *length,
            },
            GraphPattern::Group {
                inner,
                variables,
                aggregates,
            } => GraphPattern::Group {
                inner: Child::new(normalize(inner)),
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
        let union = GraphPattern::union(
            GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: TermPattern::Variable(Variable::new("s")),
                    predicate: NamedNodePattern::NamedNode(crate::ast::NamedNode::new_unchecked(
                        "https://example.org/d/p",
                    )),
                    object: TermPattern::Variable(Variable::new("o")),
                }],
            },
            GraphPattern::Bgp {
                patterns: vec![TriplePattern {
                    subject: TermPattern::Variable(Variable::new("s")),
                    predicate: NamedNodePattern::NamedNode(crate::ast::NamedNode::new_unchecked(
                        "https://example.org/d/q",
                    )),
                    object: TermPattern::Variable(Variable::new("o")),
                }],
            },
        );
        let body = GraphPattern::Lateral {
            left: Child::new(union),
            right: Child::new(GraphPattern::PropertyFunction(call)),
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
