// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The **substituted** plan, retained across runs of one prepared execution.
//!
//! [`PreparedExecution`](crate::PreparedExecution) caches the parse and the
//! admission. It did not cache the *rewrite*: every run cloned the admitted algebra
//! and walked it again to push each pre-bound constant into the patterns that can
//! carry it and to plant the `VALUES` seed. For a SHACL validation that is once per
//! focus node, for a tree that is the same tree every time except for the cells the
//! caller's values sit in. `docs/design/purrdf-change-path-allocations.md` names the
//! artifact that closes it: *"the substituted shape decided once per query and per
//! pre-bound variable name, with only the values written per focus node."*
//!
//! This module is that artifact. A [`PrebindMemo`] holds the rewritten [`Query`] and
//! a list of [`Target`]s — the positions in it that hold a caller-supplied value —
//! so a run writes cells into a retained tree instead of building a new one.
//!
//! # Why the memo is keyed on value SHAPES and not on names alone
//!
//! The design note states the soundness premise: the pushdown's boundary is *"a
//! function of the query and the names of the pre-bound variables, both constant
//! across focus nodes."* That is true of the boundary — [`crate::substitute`]'s
//! descent stops at an `OPTIONAL`'s and a `MINUS`'s right arm by matching on the
//! `GraphPattern` variant, never on a value — but it is NOT the whole story about
//! which cells exist, and the difference is already live in the crate:
//! `term_pattern_from_ground` refuses a [`GroundTerm::BlankNode`], because a blank in
//! a pattern is an anonymous variable rather than a request to match that blank. So a
//! blank-node focus node is bound through `VALUES` rows alone — the seed, and the
//! one-row driver a property-function call naming it is given — while an IRI one is
//! written into the leaves themselves. The pushed set and the seed set are different
//! sets, and which positions exist depends on which of a small, closed set of
//! *shapes* each value has.
//!
//! [`ValueShape`] is that closed set, and it is the memo's key alongside the lane.
//! A run whose values have the memo's shapes writes into the retained tree; a run
//! whose values do not takes the ordinary per-run rewrite, unchanged. That is how a
//! focus set mixing IRI and blank-node nodes cannot corrupt the memo: the blank ones
//! never reach it.
//!
//! # Why a memo is never believed, only checked
//!
//! A memo that answers *differently* from the rewrite it stands in for would be a
//! silent wrong answer — the worst failure this path has. So the target list is not
//! derived by a second reading of [`crate::substitute`]'s rules, which would be a
//! reimplementation free to drift from them. It is **observed**: [`PrebindMemo::build`]
//! runs the real rewrite several times over, varying one parameter at a time, and
//! takes the cells that MOVED. A cell the rewrite does not write cannot move, and a
//! cell it writes must. The memo is then accepted only if replaying a different value
//! set into it reproduces, node for node, the tree the real rewrite produces for that
//! same value set. A memo that fails that check is discarded and the run takes the
//! ordinary path.

use purrdf_sparql_algebra::{
    AggregateExpression, AggregateFunction, Expression, GraphPattern, GroundTerm, GroundTriple,
    Literal, NamedNode, NamedNodePattern, OrderExpression, Query, TermPattern, Variable,
};

use crate::engine::ShaclPrebinding;
use crate::substitute::{
    apply_probes, apply_shacl_probes, expression_from_ground, has_repeated_variable,
    named_node_from_ground, term_pattern_from_ground,
};

/// The already-grounded pre-binding list both halves of the rewrite consume.
type Probes = [(Variable, GroundTerm)];

/// Everything about a pre-bound VALUE that the rewrite branches on.
///
/// The rewrite reads a value in exactly four places, and every one of them asks a
/// question this enum answers:
///
/// * `term_pattern_from_ground` — may the constant be written into a triple-pattern
///   position at all? No for a blank node, and no for a quoted triple with a blank
///   anywhere inside it.
/// * `expression_from_ground` — has the constant an expression form? Only an IRI or a
///   literal has one; a blank node and a quoted triple ride the `VALUES` seed.
/// * `substitute_in_term_pattern` — a property function's argument takes an IRI or a
///   literal as a written constant; a blank node there is bound by the one-row
///   `VALUES` that drives the call instead, which is a `VALUES` cell like the seed's.
/// * `substitute_in_named_node_pattern` — a `GRAPH`/`SERVICE` name takes an IRI and
///   nothing else.
///
/// Two values with the same shape therefore produce the same TREE, differing only in
/// the cells they occupy, which is exactly the premise a memo needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValueShape {
    /// An IRI: pushed into patterns, substituted in expressions, and the one shape a
    /// `GRAPH` name accepts.
    Iri,
    /// A literal: pushed into patterns and substituted in expressions, but never a
    /// graph name.
    Literal,
    /// Bound through `VALUES` rows and written into no pattern — a blank node, or a
    /// quoted triple with a blank node somewhere inside it. The seed carries it, and so
    /// does the one-row driver of any property-function call that names it.
    SeedOnly,
    /// A quoted triple every position of which is writable into a pattern.
    ///
    /// Carried as its own shape rather than folded into [`Self::Iri`] because it is
    /// the one shape a memo REFUSES, and refusing it needs a name. Writing a quoted
    /// triple into a term position changes how many nested term positions that
    /// position contains, and the memo's cell numbering counts term positions — so
    /// the numbering would become a function of the VALUE rather than of the query
    /// and the parameter names, which is the property the whole artifact rests on.
    /// Such a run takes the ordinary rewrite and answers identically; it is a memo
    /// that is declined, not a query that is refused.
    NestedTriple,
}

impl ValueShape {
    /// The shape of one grounded pre-binding.
    fn of(ground: &GroundTerm) -> Self {
        match ground {
            GroundTerm::NamedNode(_) => Self::Iri,
            GroundTerm::Literal(_) => Self::Literal,
            GroundTerm::BlankNode(_) => Self::SeedOnly,
            // Asked of the real predicate rather than re-derived: a quoted triple is
            // writable exactly when `term_pattern_from_ground` says it is, and that
            // function recurses through every nested position looking for a blank.
            GroundTerm::Triple(_) => {
                if term_pattern_from_ground(ground).is_some() {
                    Self::NestedTriple
                } else {
                    Self::SeedOnly
                }
            }
        }
    }
}

/// One position in the substituted algebra that holds a caller-supplied value.
#[derive(Clone, Copy, Debug)]
struct Target {
    /// The position's ordinal in [`walk_query`]'s fixed traversal order.
    cell: u32,
    /// Which pre-binding's value belongs here.
    parameter: u32,
}

/// A position the rewrite can write a value into, borrowed for writing.
///
/// Four variants because the rewrite writes four different things: a term into a
/// pattern position, a cell of a `VALUES` row, a constant expression, and a
/// `GRAPH`/`SERVICE` name.
enum Cell<'a> {
    /// A triple pattern's, property path's or property-function argument's term.
    Term(&'a mut TermPattern),
    /// One cell of one `VALUES` row.
    Ground(&'a mut Option<GroundTerm>),
    /// An expression node, which the SHACL lane replaces wholesale.
    Expr(&'a mut Expression),
    /// A `GRAPH` or `SERVICE` name.
    GraphName(&'a mut NamedNodePattern),
}

impl Cell<'_> {
    /// An owned copy of what this position currently holds, for differencing two
    /// rewrites of the same query.
    fn snapshot(&self) -> CellValue {
        match self {
            Self::Term(term) => CellValue::Term((*term).clone()),
            Self::Ground(slot) => CellValue::Ground((*slot).clone()),
            Self::Expr(expr) => CellValue::Expr((*expr).clone()),
            Self::GraphName(name) => CellValue::GraphName((*name).clone()),
        }
    }
}

/// [`Cell`]'s contents, owned. Build-time only: differencing two trees needs both
/// readings at once, and only one of them can be a live borrow.
#[derive(Clone, Debug, PartialEq)]
enum CellValue {
    /// See [`Cell::Term`].
    Term(TermPattern),
    /// See [`Cell::Ground`].
    Ground(Option<GroundTerm>),
    /// See [`Cell::Expr`].
    Expr(Expression),
    /// See [`Cell::GraphName`].
    GraphName(NamedNodePattern),
}

impl CellValue {
    /// Whether a real rewrite could ever write THIS WHOLE position as one atomic
    /// replacement, rather than only ever recursing through it to reach positions
    /// further in.
    ///
    /// Wildcard-free, and it has to mirror [`walk_term`]'s and
    /// [`walk_expression`]'s own recursion conditions exactly: a position this
    /// says is a leaf is exactly a position those functions do not descend past,
    /// and a position this says is not is exactly one they do.
    ///
    /// This exists because [`Cell::snapshot`] clones a position's WHOLE subtree,
    /// not just its own immediate content — so when a leaf changes, every
    /// ancestor's snapshot changes too, purely because the leaf sits inside it.
    /// Without this check, [`PrebindMemo::build`]'s diff would record every one of
    /// those ancestors as ITS OWN [`Target`] — a `TermPattern::Triple` whose
    /// nested `$this` moved, or the `Equal`/`Bound` wrapping a pre-bound variable
    /// in a `FILTER` — even though the real rewrite (`probe_term_pattern`,
    /// `substitute_in_expression`) never overwrites any of those wholesale; it
    /// only ever recurses through them to the `TermPattern::Variable` or
    /// `Expression::Variable`/`Expression::Bound` leaf inside. Writing a replay
    /// value into such an ancestor would replace that whole subtree — deleting
    /// the very leaf position the recursion exists to reach, which is exactly
    /// what made the traversal in [`write_values`] find fewer positions than were
    /// recorded for it.
    fn is_leaf(&self) -> bool {
        match self {
            Self::Term(TermPattern::Triple(_)) => false,
            Self::Term(_) | Self::Ground(_) | Self::GraphName(_) => true,
            Self::Expr(expr) => matches!(
                expr,
                Expression::Variable(_)
                    | Expression::Bound(_)
                    | Expression::NamedNode(_)
                    | Expression::Literal(_)
            ),
        }
    }
}

/// What a caller's value becomes at `cell`.
///
/// Every arm is the SAME constructor [`crate::substitute`] writes with, so a replay
/// cannot spell a constant differently from the rewrite it stands in for.
///
/// # Panics
///
/// If `ground`'s [`ValueShape`] is not the shape the target was recorded under. A
/// target for a term position exists only because `term_pattern_from_ground`
/// answered `Some` when the memo was built, and [`PrebindMemo::matches`] refuses a
/// run whose shapes differ from the build's — so this cannot fire without one of
/// those two having gone wrong, and quietly leaving the previous run's value in the
/// cell is precisely the silent wrong answer the memo must not be able to produce.
fn write_cell(cell: Cell<'_>, ground: &GroundTerm) {
    match cell {
        Cell::Term(term) => {
            *term = term_pattern_from_ground(ground)
                .expect("a term target is only recorded for a value shape a pattern can carry");
        }
        Cell::Ground(slot) => *slot = Some(ground.clone()),
        Cell::Expr(expr) => {
            *expr = expression_from_ground(ground)
                .expect("an expression target is only recorded for an IRI or a literal");
        }
        Cell::GraphName(name) => {
            *name = NamedNodePattern::NamedNode(
                named_node_from_ground(ground)
                    .expect("a graph-name target is only recorded for an IRI"),
            );
        }
    }
}

/// Run the pre-binding rewrite `lane` names, over already-grounded probes.
///
/// The memo calls THIS rather than a copy of it, so "what the memo stands in for" and
/// "what the engine would otherwise have run" are the same two function calls.
pub(crate) fn rewrite(
    query: Query,
    lane: ShaclPrebinding,
    probes: Vec<(Variable, GroundTerm)>,
) -> Query {
    match lane {
        ShaclPrebinding::Applied => apply_shacl_probes(query, probes),
        ShaclPrebinding::None => apply_probes(query, probes),
    }
}

/// Visit every position in `query` a pre-binding rewrite can write a value into, in
/// one fixed order, handing each a running ordinal.
///
/// **This is the only enumeration of those positions.** Recording a target and
/// replaying into it both go through here, so the two cannot disagree about which
/// position ordinal `n` is: they are literally the same traversal. A `GraphPattern`
/// or `Expression` variant added later fails to compile here — both matches below are
/// wildcard-free — rather than silently shifting every ordinal after it.
///
/// # What makes the ordinals stable across runs
///
/// An ordinal is assigned to a position, never to a node reached THROUGH one. A
/// written value is a leaf in every case the memo admits ([`ValueShape::NestedTriple`]
/// is declined for exactly this reason), and the position it replaces — a
/// `TermPattern::Variable`, an `Expression::Variable`, an `Expression::Bound`, a
/// `VALUES` cell, a `NamedNodePattern::Variable` — is a leaf too. So writing a value
/// changes what a position holds and never how many positions there are.
fn walk_query(query: &mut Query, visit: &mut dyn FnMut(u32, Cell<'_>)) -> u32 {
    let mut index = 0;
    match query {
        Query::Select { pattern, .. }
        | Query::Construct { pattern, .. }
        | Query::Describe { pattern, .. }
        | Query::Ask { pattern, .. } => walk_pattern(pattern, &mut index, visit),
    }
    index
}

/// [`walk_query`]'s graph-pattern recursion.
///
/// The arms mirror `crate::substitute::substitute_in_graph_pattern`'s arms one for
/// one, plus the term positions of the three leaves that walk does not enter because
/// the PUSHDOWN half of the rewrite owns them (`Bgp`, `Path`) and the cells of a
/// `VALUES` block, which is where the seed's row lives.
fn walk_pattern(pattern: &mut GraphPattern, index: &mut u32, visit: &mut dyn FnMut(u32, Cell<'_>)) {
    match pattern {
        GraphPattern::Bgp { patterns } => {
            for triple in patterns.iter_mut() {
                walk_term(&mut triple.subject, index, visit);
                // The predicate is a `NamedNodePattern` that neither half of the
                // rewrite writes: the pushdown visits subject and object only, and
                // the expression walk does not enter a `Bgp` at all.
                walk_term(&mut triple.object, index, visit);
            }
        }
        GraphPattern::Path {
            subject, object, ..
        } => {
            walk_term(subject, index, visit);
            walk_term(object, index, visit);
        }
        GraphPattern::Values { bindings, .. } => {
            for row in bindings.iter_mut() {
                for slot in row.iter_mut() {
                    let here = *index;
                    *index += 1;
                    visit(here, Cell::Ground(slot));
                }
            }
        }
        GraphPattern::Join { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right } => {
            walk_pattern(left, index, visit);
            walk_pattern(right, index, visit);
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            walk_pattern(left, index, visit);
            walk_pattern(right, index, visit);
            if let Some(expression) = expression {
                walk_expression(expression, index, visit);
            }
        }
        GraphPattern::Filter { expr, inner } => {
            walk_expression(expr, index, visit);
            walk_pattern(inner, index, visit);
        }
        GraphPattern::Graph { name, inner } | GraphPattern::Service { name, inner, .. } => {
            let here = *index;
            *index += 1;
            visit(here, Cell::GraphName(name));
            walk_pattern(inner, index, visit);
        }
        GraphPattern::Extend {
            inner, expression, ..
        }
        | GraphPattern::Unfold {
            inner, expression, ..
        } => {
            walk_pattern(inner, index, visit);
            walk_expression(expression, index, visit);
        }
        GraphPattern::OrderBy { inner, expression } => {
            walk_pattern(inner, index, visit);
            for order in expression.iter_mut() {
                walk_order(order, index, visit);
            }
        }
        GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => walk_pattern(inner, index, visit),
        GraphPattern::PropertyFunction(call) => {
            for term in call
                .subject_args
                .iter_mut()
                .chain(call.object_args.iter_mut())
            {
                walk_term(term, index, visit);
            }
        }
        GraphPattern::Group {
            inner, aggregates, ..
        } => {
            walk_pattern(inner, index, visit);
            walk_aggregates(aggregates, index, visit);
        }
    }
}

/// Visit the expressions inside a `GROUP BY`'s aggregates.
///
/// [`AggregateExpression`]'s parts are reachable only through its consuming
/// `into_parts`, because the checked constructor is the only way one comes into
/// existence — so each entry is taken out, walked and rebuilt. Taking the vector and
/// putting it back moves it rather than copying it, and the placeholder is a
/// `COUNT(*)`, whose empty argument list allocates nothing; the whole function is
/// allocation-free.
fn walk_aggregates(
    aggregates: &mut Vec<(Variable, AggregateExpression)>,
    index: &mut u32,
    visit: &mut dyn FnMut(u32, Cell<'_>),
) {
    let mut taken = std::mem::take(aggregates);
    for entry in &mut taken {
        let placeholder = count_star();
        let (function, mut args, scalarvals, mut order_by, distinct) =
            std::mem::replace(&mut entry.1, placeholder).into_parts();
        for arg in &mut args {
            walk_expression(arg, index, visit);
        }
        for order in &mut order_by {
            walk_order(order, index, visit);
        }
        entry.1 = AggregateExpression::new(function, args, scalarvals, order_by, distinct)
            .expect("visiting an argument changes no argument count, so arity stays valid");
    }
    *aggregates = taken;
}

/// `COUNT(*)`: the one aggregate whose argument list may be empty, so the one that
/// can stand in a slot for the length of a `mem::replace` without allocating.
fn count_star() -> AggregateExpression {
    AggregateExpression::new(
        AggregateFunction::Count,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        false,
    )
    .expect("COUNT(*) is the spec's one empty-exprlist aggregate")
}

/// [`walk_query`]'s sort-key recursion.
fn walk_order(order: &mut OrderExpression, index: &mut u32, visit: &mut dyn FnMut(u32, Cell<'_>)) {
    match order {
        OrderExpression::Asc(expr) | OrderExpression::Desc(expr) => {
            walk_expression(expr, index, visit);
        }
    }
}

/// [`walk_query`]'s term-position recursion.
///
/// A quoted triple's own subject and object are positions too — the pushdown recurses
/// into them — so they are numbered as well. Its predicate is not: neither half of the
/// rewrite writes there.
fn walk_term(term: &mut TermPattern, index: &mut u32, visit: &mut dyn FnMut(u32, Cell<'_>)) {
    let here = *index;
    *index += 1;
    visit(here, Cell::Term(term));
    if let TermPattern::Triple(triple) = term {
        walk_term(&mut triple.subject, index, visit);
        walk_term(&mut triple.object, index, visit);
    }
}

/// [`walk_query`]'s expression recursion.
///
/// The node itself is numbered BEFORE its children, because the SHACL lane replaces a
/// whole `Expression::Variable` or `Expression::Bound` node rather than editing one.
fn walk_expression(expr: &mut Expression, index: &mut u32, visit: &mut dyn FnMut(u32, Cell<'_>)) {
    let here = *index;
    *index += 1;
    visit(here, Cell::Expr(expr));
    match expr {
        Expression::Variable(_)
        | Expression::Bound(_)
        | Expression::NamedNode(_)
        | Expression::Literal(_) => {}
        Expression::Or(left, right)
        | Expression::And(left, right)
        | Expression::Equal(left, right)
        | Expression::SameTerm(left, right)
        | Expression::Greater(left, right)
        | Expression::GreaterOrEqual(left, right)
        | Expression::Less(left, right)
        | Expression::LessOrEqual(left, right)
        | Expression::Add(left, right)
        | Expression::Subtract(left, right)
        | Expression::Multiply(left, right)
        | Expression::Divide(left, right) => {
            walk_expression(left, index, visit);
            walk_expression(right, index, visit);
        }
        Expression::UnaryPlus(inner) | Expression::UnaryMinus(inner) | Expression::Not(inner) => {
            walk_expression(inner, index, visit);
        }
        Expression::In(target, list) => {
            walk_expression(target, index, visit);
            for item in list.iter_mut() {
                walk_expression(item, index, visit);
            }
        }
        Expression::If(cond, then_expr, else_expr) => {
            walk_expression(cond, index, visit);
            walk_expression(then_expr, index, visit);
            walk_expression(else_expr, index, visit);
        }
        Expression::Coalesce(list) | Expression::FunctionCall(_, list) => {
            for item in list.iter_mut() {
                walk_expression(item, index, visit);
            }
        }
        Expression::Exists(inner) => walk_pattern(inner, index, visit),
    }
}

/// Every position's contents, in [`walk_query`]'s order.
fn cell_snapshots(query: &mut Query) -> Vec<CellValue> {
    let mut cells = Vec::new();
    walk_query(query, &mut |_, cell| cells.push(cell.snapshot()));
    cells
}

/// Write each target's parameter value into the tree, and report how many positions
/// the traversal found.
///
/// Allocation-free: the traversal builds nothing, and every constant it writes is
/// `Arc<str>`-backed, so a write is a refcount bump.
fn write_values(query: &mut Query, targets: &[Target], probes: &Probes) -> u32 {
    let mut next = 0;
    let count = walk_query(query, &mut |index, cell| {
        // `targets` is sorted by `cell` and holds each ordinal at most once, and
        // `walk_query` yields ordinals in increasing order, so one forward cursor
        // matches them all with no search.
        if let Some(target) = targets.get(next)
            && target.cell == index
        {
            write_cell(cell, &probes[target.parameter as usize].1);
            next += 1;
        }
    });
    assert!(
        next == targets.len(),
        "the substituted plan retained by a prepared execution reached {next} of its {} value \
         positions: the traversal that records a position and the traversal that writes one \
         have diverged, so cells still carrying an earlier run's values would be evaluated as \
         if they carried this one's",
        targets.len()
    );
    count
}

/// A term of the same [`ValueShape`] as `ground` and never equal to it.
///
/// This is how [`PrebindMemo::build`] finds the cells a parameter occupies: rewrite
/// once with the caller's values, rewrite again with ONE parameter moved, and take
/// the positions that moved with it. Derived from the caller's own term rather than
/// minted, so the probe is a term the caller already brought and no vocabulary is
/// invented for it; it is longer than what it derives from, so it is always distinct;
/// and it keeps the term KIND, so the tree it produces has the same shape.
///
/// It never escapes: the trees it appears in are read for their cell positions and
/// dropped inside `build`.
fn moved(ground: &GroundTerm) -> GroundTerm {
    match ground {
        GroundTerm::NamedNode(node) => GroundTerm::NamedNode(moved_node(node)),
        GroundTerm::Literal(literal) => GroundTerm::Literal(match literal.language() {
            Some(language) => Literal::new_lang(
                format!("{}a", literal.value()),
                language,
                literal.direction(),
            ),
            None => Literal::new_typed(format!("{}a", literal.value()), literal.datatype().clone()),
        }),
        GroundTerm::BlankNode(blank) => GroundTerm::BlankNode(
            purrdf_sparql_algebra::BlankNode::new(format!("{}a", blank.as_str())),
        ),
        // Every position moves, so a quoted triple whose writability came from a
        // nested blank keeps it and a fully-writable one stays fully writable.
        GroundTerm::Triple(triple) => GroundTerm::Triple(Box::new(GroundTriple {
            subject: moved(&triple.subject),
            predicate: moved_node(&triple.predicate),
            object: moved(&triple.object),
        })),
    }
}

/// [`moved`], for an IRI.
fn moved_node(node: &NamedNode) -> NamedNode {
    NamedNode::new_unchecked(format!("{}a", node.as_str()))
}

/// The substituted algebra of one prepared execution, retained across its runs.
#[derive(Debug)]
pub(crate) struct PrebindMemo {
    /// Which of the two rewrites produced [`Self::query`]. The SHACL lane and the
    /// plain lane build DIFFERENT trees from the same query, so a run on the other
    /// lane must not read this one.
    lane: ShaclPrebinding,
    /// The [`ValueShape`] each parameter had when the tree was built, positionally.
    shapes: Box<[ValueShape]>,
    /// The rewritten query, holding the most recently bound values.
    query: Query,
    /// Where those values live, sorted by [`Target::cell`] and each ordinal at most
    /// once.
    targets: Box<[Target]>,
    /// How many positions [`walk_query`] found when the targets were recorded.
    ///
    /// Re-checked on every replay. If a written value ever changed the number of
    /// positions — the hazard [`ValueShape::NestedTriple`] is declined for — every
    /// ordinal after it would shift and the replay would write into the wrong cells.
    /// This turns that into a panic instead of a wrong answer.
    cells: u32,
}

impl PrebindMemo {
    /// The [`ValueShape`] of each probe, as a memo key.
    pub(crate) fn shapes_of(probes: &Probes) -> Box<[ValueShape]> {
        probes
            .iter()
            .map(|(_, ground)| ValueShape::of(ground))
            .collect()
    }

    /// Whether `shapes` is the shape list `probes` would produce, without building
    /// one to find out. Called on every run, so it allocates nothing.
    pub(crate) fn shapes_match(shapes: &[ValueShape], probes: &Probes) -> bool {
        shapes.len() == probes.len()
            && shapes
                .iter()
                .zip(probes)
                .all(|(shape, (_, ground))| *shape == ValueShape::of(ground))
    }

    /// Whether this memo answers for `lane` and `probes`.
    pub(crate) fn matches(&self, lane: ShaclPrebinding, probes: &Probes) -> bool {
        self.lane == lane && Self::shapes_match(&self.shapes, probes)
    }

    /// The retained tree, with `probes`' values written into it.
    ///
    /// # Panics
    ///
    /// If the traversal no longer reaches every recorded position, or reaches a
    /// different number of positions than it did when they were recorded. Both mean
    /// the retained tree holds cells from an earlier run that this run's values were
    /// meant to replace, which is a wrong answer rather than a slow one.
    pub(crate) fn bind(&mut self, probes: &Probes) -> &Query {
        debug_assert!(
            Self::shapes_match(&self.shapes, probes),
            "a memo is only ever bound after `matches` agreed to it"
        );
        let cells = write_values(&mut self.query, &self.targets, probes);
        assert!(
            cells == self.cells,
            "the substituted plan retained by a prepared execution now has {cells} value \
             positions where it had {}: a bound value has changed the SHAPE of the tree rather \
             than only its cells, so every position after it is misnumbered",
            self.cells
        );
        &self.query
    }

    /// The tree as it stands, for the run that built it.
    pub(crate) fn query(&self) -> &Query {
        &self.query
    }

    /// Build a memo for `prepared` under `lane` and the shapes of `probes`, or `None`
    /// if this query, lane and shape list are outside what a memo can represent.
    ///
    /// # How the targets are found
    ///
    /// By **observation of the real rewrite**, never by a second reading of its rules.
    /// One tree is built from `probes`; then, for each parameter in turn, another is
    /// built from `probes` with that one parameter's value [`moved`]. The positions
    /// whose contents differ are that parameter's, because a position the rewrite
    /// does not write cannot differ and a position it writes with that value must.
    /// Varying one parameter at a time is what makes the attribution unambiguous —
    /// with every value moved at once, two parameters bound to the SAME term (a
    /// SHACL `$this` and `$value` on a self-referential path, say) could not be told
    /// apart.
    ///
    /// # What makes it safe to use afterwards
    ///
    /// The last step replays the all-moved value set into a copy of the first tree
    /// and compares it against the tree the real rewrite builds for that same value
    /// set. They must be equal node for node. That is the check that closes the gap
    /// the differencing cannot close on its own: if this module's traversal failed to
    /// VISIT some position the rewrite writes, no amount of differencing would ever
    /// have noticed, but the replayed tree would carry the first value set's term
    /// there and the comparison fails. A memo that fails it is discarded.
    pub(crate) fn build(prepared: &Query, lane: ShaclPrebinding, probes: &Probes) -> Option<Self> {
        // A repeated pre-bound name keeps `apply_probes`' own per-variable path, which
        // builds a different tree; nothing on the prepared door can produce one
        // (`prepare_execution` refuses a repeated parameter), and a memo that assumed
        // the combined-seed shape for it would be describing the wrong rewrite.
        if has_repeated_variable(probes) {
            return None;
        }
        let shapes = Self::shapes_of(probes);
        if shapes.contains(&ValueShape::NestedTriple) {
            return None;
        }

        let mut base = rewrite(prepared.clone(), lane, probes.to_vec());
        let base_cells = cell_snapshots(&mut base);

        let moved_probes: Vec<(Variable, GroundTerm)> = probes
            .iter()
            .map(|(variable, ground)| (variable.clone(), moved(ground)))
            .collect();

        let mut targets = Vec::new();
        for (parameter, (_, ground)) in moved_probes.iter().enumerate() {
            let mut one_moved = probes.to_vec();
            one_moved[parameter].1 = ground.clone();
            let mut tree = rewrite(prepared.clone(), lane, one_moved);
            let cells = cell_snapshots(&mut tree);
            if cells.len() != base_cells.len() {
                return None;
            }
            for (cell, (here, there)) in base_cells.iter().zip(&cells).enumerate() {
                // A changed ANCESTOR of the real write site changes too — its
                // snapshot is the whole subtree — but it is never itself a
                // position the rewrite overwrites, only one it recurses through.
                // See [`CellValue::is_leaf`].
                if here != there && here.is_leaf() {
                    targets.push(Target {
                        cell: u32::try_from(cell).ok()?,
                        parameter: u32::try_from(parameter).ok()?,
                    });
                }
            }
        }
        targets.sort_unstable_by_key(|target| target.cell);
        // One position cannot hold two parameters' values. If two parameters both
        // claim one, the attribution is ambiguous and the memo would write whichever
        // of them the cursor reached first.
        if targets.windows(2).any(|pair| pair[0].cell == pair[1].cell) {
            return None;
        }

        let mut replayed = base.clone();
        let cells = write_values(&mut replayed, &targets, &moved_probes);
        if replayed != rewrite(prepared.clone(), lane, moved_probes) {
            return None;
        }
        Some(Self {
            lane,
            shapes,
            query: base,
            targets: targets.into_boxed_slice(),
            cells,
        })
    }
}

#[cfg(test)]
mod tests;
